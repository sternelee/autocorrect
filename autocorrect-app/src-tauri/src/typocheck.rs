use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{LazyLock, RwLock};
use typos::Status;

// Note: The typos library detects common English spelling errors.
// To add custom typo corrections, create ~/.autocorrect-typos.txt file
// with lines in the format: typo=correction
//
// Example:
// whts=what's
// teh=the
// waht=what
//
// To ignore words (prevent false positives), add them to the AutoCorrect
// custom dictionary in the Settings panel.

// Bundled dictionary from CSpell software-terms and companies
// This contains common programming/software terms that should not be flagged as typos
static BUNDLED_DICT: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    let dict_content = include_str!("../dictionaries/cspell-bundled.txt");
    let mut words = HashSet::new();

    for line in dict_content.lines() {
        let line = line.trim();
        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // Add both lowercase and original case versions
        words.insert(line);
    }

    // Add common terms manually if not in the bundled list
    words.insert("tech");
    words.insert("tauri");
    words.insert("pnpm");

    log::info!("Loaded {} words from bundled dictionary", words.len());
    words
});

/// Check if a word is in our bundled dictionary (case-insensitive)
fn is_bundled_word(word: &str) -> bool {
    // Check exact match first
    if BUNDLED_DICT.contains(word) {
        return true;
    }
    // Check lowercase version
    let lower = word.to_lowercase();
    BUNDLED_DICT.iter().any(|&w| w.eq_ignore_ascii_case(&lower))
}

fn get_custom_corrections_path() -> Option<PathBuf> {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()?;
    let path = PathBuf::from(home).join(".autocorrect-typos.txt");
    if path.exists() {
        Some(path)
    } else {
        None
    }
}

fn load_custom_corrections() -> HashMap<String, String> {
    let mut corrections = HashMap::new();

    if let Some(path) = get_custom_corrections_path() {
        if let Ok(content) = std::fs::read_to_string(&path) {
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }

                if let Some((typo, correction)) = line.split_once('=') {
                    corrections.insert(typo.trim().to_lowercase(), correction.trim().to_string());
                }
            }
        }
    }

    corrections
}

static POLICY: LazyLock<typos_cli::policy::Policy> =
    LazyLock::new(typos_cli::policy::Policy::default);

// Common English words (google-10000-english) used to verify doubled-letter
// fold candidates. The typos dictionary only maps wrong→correct spellings,
// so it cannot tell a valid word from an unknown one at runtime.
static COMMON_ENGLISH_WORDS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    let content = include_str!("../dictionaries/english-common.txt");
    let words: HashSet<&'static str> = content
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();
    log::info!("Loaded {} common English words", words.len());
    words
});

// Use RwLock to allow reloading custom corrections at runtime
static CUSTOM_CORRECTIONS: LazyLock<RwLock<HashMap<String, String>>> =
    LazyLock::new(|| RwLock::new(load_custom_corrections()));

/// Force reload custom corrections from file
/// This should be called after updating the corrections file
pub fn reload_custom_corrections() {
    let new_corrections = load_custom_corrections();
    if let Ok(mut corrections) = CUSTOM_CORRECTIONS.write() {
        *corrections = new_corrections;
        log::info!("Reloaded {} custom corrections", corrections.len());
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct TypoError {
    pub typo: String,
    pub suggestions: Vec<String>,
    pub byte_offset: usize,
    pub line: usize,
    pub col: usize,
}

/// Check for typos in the given text
pub fn check_typos(text: &str) -> Vec<TypoError> {
    let mut typos_errors: Vec<TypoError> = Vec::new();

    // Pass 1: collect all typos from the typos library (byte offsets only, line/col deferred)
    let results = typos::check_str(text, POLICY.tokenizer, POLICY.dict);

    for typo in results {
        if let Status::Corrections(corrections) = typo.corrections {
            let typo_word = typo.typo.to_string();

            if is_bundled_word(&typo_word) {
                log::debug!("Skipping '{}' - found in bundled dictionary", typo_word);
                continue;
            }

            let suggestions = corrections
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>();

            typos_errors.push(TypoError {
                typo: typo_word,
                suggestions,
                byte_offset: typo.byte_offset,
                line: 0, // filled in batch below
                col: 0,
            });
        }
    }

    // Pass 1.5: the pre-generated typos dictionary has no letter-repetition
    // variants, so doubled-letter typos like "helloo"/"worldd" pass through
    // as unknown words. Fold repeated runs ourselves and keep the fold that
    // yields exactly one common English word.
    for ident in POLICY.tokenizer.parse_str(text) {
        for word in ident.split() {
            let token = word.token();
            if !token.bytes().all(|b| b.is_ascii_alphabetic()) {
                continue;
            }
            match POLICY.dict.correct_word(word) {
                Some(Status::Valid) | Some(Status::Corrections(_)) => continue,
                _ => {}
            }
            if is_bundled_word(token)
                || COMMON_ENGLISH_WORDS.contains(token.to_lowercase().as_str())
            {
                continue;
            }
            if let Some(correction) = fold_repeats_to_valid(token) {
                if !typos_errors.iter().any(|e| e.byte_offset == word.offset()) {
                    typos_errors.push(TypoError {
                        typo: token.to_string(),
                        suggestions: vec![correction],
                        byte_offset: word.offset(),
                        line: 0, // filled in batch below
                        col: 0,
                    });
                }
            }
        }
    }

    // Pass 2: apply custom corrections (byte offsets only, no line/col yet)
    {
        let guard = CUSTOM_CORRECTIONS.read().ok();
        if let Some(corrections) = guard.as_ref().filter(|c| !c.is_empty()) {
            let mut word_start: Option<usize> = None;
            let mut word = String::new();
            for (i, ch) in text.char_indices() {
                if ch.is_alphabetic() || ch == '\'' {
                    if word_start.is_none() {
                        word_start = Some(i);
                    }
                    word.push(ch);
                } else if let Some(start) = word_start.take() {
                    apply_custom_correction(&word, start, corrections, &mut typos_errors);
                    word.clear();
                }
            }
            if let Some(start) = word_start {
                apply_custom_correction(&word, start, corrections, &mut typos_errors);
            }
        }
    }

    // Sort by byte offset
    typos_errors.sort_by_key(|e| e.byte_offset);

    // Batch-compute line/col in ONE O(text_len) scan rather than O(N * text_len)
    if !typos_errors.is_empty() {
        let positions = batch_line_col(text, typos_errors.iter().map(|e| e.byte_offset));
        for (err, (line, col)) in typos_errors.iter_mut().zip(positions) {
            err.line = line;
            err.col = col;
        }
    }

    typos_errors
}

/// Try to correct a doubled-letter typo (e.g. "helloo" → "hello").
///
/// The upstream dictionary has no variants for inserted repeated letters, so
/// we fold each run of a repeated character down to 1 or 2 copies and keep
/// the fold only when it maps to exactly one common English word. Returns
/// None when the word has no runs, or the fold is ambiguous.
fn fold_repeats_to_valid(word: &str) -> Option<String> {
    let bytes = word.as_bytes();

    // Byte ranges [start, end) of runs holding the same char 2+ times.
    let runs: Vec<(usize, usize)> = {
        let mut runs = Vec::new();
        let mut i = 0;
        while i < bytes.len() {
            let mut j = i + 1;
            while j < bytes.len() && bytes[j] == bytes[i] {
                j += 1;
            }
            if j - i >= 2 {
                runs.push((i, j));
            }
            i = j;
        }
        runs
    };
    if runs.is_empty() {
        return None;
    }

    // Candidate folded lengths per run: 1, or 2 (which also covers "keep
    // as-is" for a run of exactly 2, e.g. the "ll" in "helloo").
    let choices: Vec<Vec<usize>> = runs.iter().map(|_| vec![1usize, 2usize]).collect();

    let mut valid: Option<String> = None;
    let mut counters = vec![0usize; choices.len()];
    loop {
        let mut folded = String::with_capacity(word.len());
        let mut pos = 0usize;
        for (i, &(start, end)) in runs.iter().enumerate() {
            folded.push_str(&word[pos..start]);
            let ch = word[start..]
                .chars()
                .next()
                .expect("run start is char boundary");
            for _ in 0..choices[i][counters[i]] {
                folded.push(ch);
            }
            pos = end;
        }
        folded.push_str(&word[pos..]);

        if COMMON_ENGLISH_WORDS.contains(folded.to_lowercase().as_str()) {
            if valid.as_deref().is_some_and(|v| v != folded) {
                return None; // ambiguous fold — don't guess
            }
            valid = Some(folded);
        }

        // Mixed-radix increment over the choice counters; all wrapped = done.
        for idx in (0..counters.len()).rev() {
            counters[idx] += 1;
            if counters[idx] < choices[idx].len() {
                break;
            }
            counters[idx] = 0;
            if idx == 0 {
                return valid;
            }
        }
    }
}

/// Apply a custom correction entry (no line/col — deferred to batch_line_col).
fn apply_custom_correction(
    word: &str,
    word_start: usize,
    corrections: &std::collections::HashMap<String, String>,
    typos_errors: &mut Vec<TypoError>,
) {
    let word_lower = word.to_lowercase();
    if let Some(correction) = corrections.get(&word_lower) {
        if !typos_errors.iter().any(|e| e.byte_offset == word_start) {
            typos_errors.push(TypoError {
                typo: word.to_string(),
                suggestions: vec![correction.clone()],
                byte_offset: word_start,
                line: 0,
                col: 0,
            });
        }
    }
}

/// Compute (line, col) for a sorted sequence of byte offsets in a single O(text_len) scan.
/// Offsets must be sorted ascending (guaranteed by caller's sort_by_key above).
fn batch_line_col(text: &str, offsets: impl Iterator<Item = usize>) -> Vec<(usize, usize)> {
    let offsets: Vec<usize> = offsets.collect();
    let mut result = vec![(1usize, 1usize); offsets.len()];
    if offsets.is_empty() {
        return result;
    }
    let mut line = 1usize;
    let mut col = 1usize;
    let mut byte_pos = 0usize;
    let mut idx = 0usize;
    for ch in text.chars() {
        while idx < offsets.len() && offsets[idx] <= byte_pos {
            result[idx] = (line, col);
            idx += 1;
        }
        if idx >= offsets.len() {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
        byte_pos += ch.len_utf8();
    }
    while idx < offsets.len() {
        result[idx] = (line, col);
        idx += 1;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_typos() {
        let text = "This is a smaple text with a typo.\nAnother line has a soure.";
        let typos = check_typos(text);

        assert_eq!(typos.len(), 2);
        assert_eq!(typos[0].typo, "smaple");
        assert!(typos[0].suggestions.contains(&"sample".to_string()));
        assert_eq!(typos[0].line, 1);

        assert_eq!(typos[1].typo, "soure");
        assert_eq!(typos[1].line, 2);
    }

    #[test]
    fn test_doubled_letter_typos() {
        // The upstream typos dictionary has no letter-repetition variants;
        // these are caught by our fold_repeats_to_valid pass.
        let cases = vec![
            ("helloo world", "helloo", "hello"),
            ("helloo", "helloo", "hello"),
            ("hellooo", "hellooo", "hello"),
            ("helllo", "helllo", "hello"),
            ("worldd", "worldd", "world"),
            ("Helloo world", "Helloo", "Hello"),
        ];
        for (text, expected_typo, expected_fix) in cases {
            let typos = check_typos(text);
            let found = typos
                .iter()
                .find(|t| t.typo.eq_ignore_ascii_case(expected_typo));
            assert!(
                found.is_some(),
                "Expected typo '{expected_typo}' in '{text}', got {:?}",
                typos
            );
            assert_eq!(
                found.unwrap().suggestions,
                vec![expected_fix.to_string()],
                "case '{text}'"
            );
        }
    }

    #[test]
    fn test_doubled_letter_no_false_positives() {
        // Valid words with double letters must not be flagged.
        let text = "book keep took shall success feeling";
        assert_eq!(check_typos(text).len(), 0);
    }

    #[test]
    fn test_no_typos() {
        let text = "This is a correct sentence.";
        let typos = check_typos(text);
        assert_eq!(typos.len(), 0);
    }

    #[test]
    fn test_common_typos() {
        // Test various common typos that SHOULD be detected by typos library
        let test_cases = vec![
            ("recieve the message", "recieve"),
            ("definately yes", "definately"),
            ("seperate items", "seperate"),
            ("occured yesterday", "occured"),
            ("acommodate guests", "acommodate"),
            ("neccessary items", "neccessary"),
        ];

        for (text, expected_typo) in test_cases {
            let typos = check_typos(text);

            println!("\nText: '{}'", text);
            println!("Found {} typos", typos.len());
            for typo in &typos {
                println!("  - '{}' -> {:?}", typo.typo, typo.suggestions);
            }

            let found = typos.iter().any(|t| t.typo == expected_typo);
            assert!(
                found,
                "Expected to find typo '{}' in '{}'",
                expected_typo, text
            );
        }
    }

    #[test]
    fn test_custom_corrections_loading() {
        // Test that custom corrections can be loaded
        let corrections = load_custom_corrections();
        println!("Loaded {} custom corrections", corrections.len());

        // If ~/.autocorrect-typos.txt exists, it should have loaded some
        if get_custom_corrections_path().is_some() {
            println!("Custom corrections file found");
            for (typo, correction) in corrections.iter() {
                println!("  {} -> {}", typo, correction);
            }
        } else {
            println!("No custom corrections file found (this is OK for testing)");
        }
    }

    #[test]
    fn test_whts_with_custom() {
        let text = "whts is your name";
        let typos = check_typos(text);

        println!("\nText: '{}'", text);
        println!("Found {} typos", typos.len());
        for typo in &typos {
            println!("  - '{}' -> {:?}", typo.typo, typo.suggestions);
        }

        // If custom corrections are loaded, whts should be detected
        if let Ok(corrections) = CUSTOM_CORRECTIONS.read() {
            if !corrections.is_empty() && corrections.contains_key("whts") {
                let found = typos.iter().any(|t| t.typo.to_lowercase() == "whts");
                assert!(
                    found,
                    "Expected to find 'whts' typo with custom corrections"
                );
                println!("✅ Custom correction for 'whts' is working!");
            } else {
                println!("Note: No custom corrections loaded. Add ~/.autocorrect-typos.txt with 'whts=what's'");
            }
        }
    }
}
