use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, RwLock};
use symspell::{AsciiStringStrategy, SymSpell, Verbosity};
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
// custom dictionary in the Settings panel, or to ~/.autocorrect-cspell.json
// under `words` / `ignoreWords`.

/// CSpell-style user configuration. Only the fields we consume at runtime are
/// declared; unknown fields are ignored so the file stays compatible with the
/// upstream cspell CLI.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CSpellConfig {
    #[serde(default)]
    words: Vec<String>,
    #[serde(default)]
    ignore_words: Vec<String>,
    #[serde(default)]
    flag_words: Vec<String>,
}

/// Runtime state for CSpell integration. Kept in a single struct so we can
/// reload it atomically from the settings panel.
struct CSpellState {
    enabled: bool,
    /// When enabled, flag any English-looking token that is not in the bundled
    /// dictionary or in the cspell allow-list.
    unknown_word_detection_enabled: bool,
    config_path: PathBuf,
    config: CSpellConfig,
    /// Lowercased words that should never be flagged (`words` + `ignoreWords`).
    allowed_words: HashSet<String>,
    /// Lowercased words that should not be flagged as typos by any source.
    ignored_words: HashSet<String>,
    /// Lowercased typo -> suggested replacements from `flagWords`.
    flag_corrections: HashMap<String, Vec<String>>,
}

impl CSpellState {
    fn load(path: PathBuf, enabled: bool, unknown_word_detection_enabled: bool) -> Self {
        let config = load_cspell_config(&path);
        let mut state = Self {
            enabled,
            unknown_word_detection_enabled,
            config_path: path,
            config,
            allowed_words: HashSet::new(),
            ignored_words: HashSet::new(),
            flag_corrections: HashMap::new(),
        };
        state.rebuild();
        state
    }

    fn rebuild(&mut self) {
        self.allowed_words.clear();
        self.ignored_words.clear();
        self.flag_corrections.clear();

        for word in &self.config.words {
            self.allowed_words.insert(word.trim().to_lowercase());
        }
        for word in &self.config.ignore_words {
            let key = word.trim().to_lowercase();
            self.ignored_words.insert(key.clone());
            self.allowed_words.insert(key);
        }

        for raw in &self.config.flag_words {
            if let Some((typo, suggestions)) = parse_flag_word(raw) {
                self.flag_corrections.insert(typo, suggestions);
            }
        }
    }

    fn reload(&mut self) {
        self.config = load_cspell_config(&self.config_path);
        self.rebuild();
    }

    fn set_path(&mut self, path: PathBuf) {
        self.config_path = path;
        self.reload();
    }
}

fn default_cspell_config_path() -> Option<PathBuf> {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()?;
    Some(PathBuf::from(home).join(".autocorrect-cspell.json"))
}

fn load_cspell_config(path: &Path) -> CSpellConfig {
    if !path.exists() {
        return CSpellConfig::default();
    }
    match std::fs::read_to_string(path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_else(|e| {
            log::warn!("Failed to parse cspell config at {:?}: {}", path, e);
            CSpellConfig::default()
        }),
        Err(e) => {
            log::warn!("Failed to read cspell config at {:?}: {}", path, e);
            CSpellConfig::default()
        }
    }
}

/// Parse a cspell `flagWords` entry.
/// Supported forms: `word`, `word:suggestion`, `word->suggestion`.
/// Multiple suggestions can be separated by commas.
fn parse_flag_word(raw: &str) -> Option<(String, Vec<String>)> {
    let line = raw.trim();
    if line.is_empty() {
        return None;
    }

    let (typo, suggestion_str) = if let Some(idx) = line.find("->") {
        (&line[..idx], &line[idx + 2..])
    } else if let Some(idx) = line.find(':') {
        (&line[..idx], &line[idx + 1..])
    } else {
        (line, "")
    };

    let typo = typo.trim().to_lowercase();
    if typo.is_empty() {
        return None;
    }

    let suggestions: Vec<String> = if suggestion_str.is_empty() {
        Vec::new()
    } else {
        suggestion_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    };

    Some((typo, suggestions))
}

static C_SPELL_STATE: LazyLock<RwLock<CSpellState>> = LazyLock::new(|| {
    let path =
        default_cspell_config_path().unwrap_or_else(|| PathBuf::from(".autocorrect-cspell.json"));
    RwLock::new(CSpellState::load(path, true, false))
});

/// Enable/disable CSpell integration, toggle unknown-word detection, and/or
/// switch the config file path.
pub fn set_cspell_state(enabled: bool, unknown_word_detection_enabled: bool, path: Option<String>) {
    if let Ok(mut state) = C_SPELL_STATE.write() {
        state.enabled = enabled;
        state.unknown_word_detection_enabled = unknown_word_detection_enabled;
        if let Some(p) = path {
            let new_path = PathBuf::from(p);
            if new_path != state.config_path {
                state.set_path(new_path);
                log::info!("CSpell config path updated");
            }
        }
        log::info!(
            "CSpell state updated: enabled={}, unknown_words={}, allowed_words={}, flag_words={}",
            state.enabled,
            state.unknown_word_detection_enabled,
            state.allowed_words.len(),
            state.flag_corrections.len()
        );
    }
}

/// Reload the CSpell configuration from the currently configured path.
pub fn reload_cspell_config() {
    if let Ok(mut state) = C_SPELL_STATE.write() {
        state.reload();
        log::info!(
            "Reloaded CSpell config: allowed_words={}, flag_words={}",
            state.allowed_words.len(),
            state.flag_corrections.len()
        );
    }
}

/// Warm up the SymSpell dictionary in the background so the first real spell
/// check doesn't block on building the deletion index.
pub fn warmup_symspell() {
    log::info!("Warming up SymSpell dictionary...");
    let start = std::time::Instant::now();
    let _ = SYM_SPELL.as_ref();
    log::info!("SymSpell warm-up finished in {:?}", start.elapsed());
}

fn is_cspell_allowed(word: &str) -> bool {
    let Ok(state) = C_SPELL_STATE.read() else {
        return false;
    };
    if !state.enabled {
        return false;
    }
    state.allowed_words.contains(&word.to_lowercase())
}

// Bundled dictionary from CSpell dictionaries (en_us, software-terms, companies,
// etc.) merged at build time by scripts/merge-cspell-dicts.mjs.
static BUNDLED_DICT: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    let dict_content = include_str!("../dictionaries/cspell-merged.txt");
    let mut words = HashSet::new();

    for line in dict_content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        words.insert(line);
    }

    // Add common terms manually if not in the bundled list.
    words.insert("tech");
    words.insert("tauri");
    words.insert("pnpm");

    log::info!("Loaded {} words from bundled dictionary", words.len());
    words
});

// SymSpell instance built from the bundled dictionary, used for high-quality
// suggestions on unknown words.
static SYM_SPELL: LazyLock<Option<SymSpell<AsciiStringStrategy>>> = LazyLock::new(|| {
    let mut symspell: SymSpell<AsciiStringStrategy> = SymSpell::default();
    let dict = include_str!("../dictionaries/cspell-merged.txt");
    let mut loaded = 0usize;

    for line in dict.lines() {
        let word = line.trim();
        if word.is_empty() || word.starts_with('#') {
            continue;
        }
        let entry = format!("{}\t1", word);
        if symspell.load_dictionary_line(&entry, 0, 1, "\t") {
            loaded += 1;
        }
    }

    log::info!("Loaded {} entries into SymSpell", loaded);
    Some(symspell)
});

/// Check if a word is in our bundled dictionary (case-insensitive)
fn is_bundled_word(word: &str) -> bool {
    if BUNDLED_DICT.contains(word) {
        return true;
    }
    let lower = word.to_lowercase();
    BUNDLED_DICT.iter().any(|&w| w.eq_ignore_ascii_case(&lower))
}

/// Return true if the word should not be flagged as an unknown word.
fn is_known_word(word: &str, state: &CSpellState) -> bool {
    // Skip short tokens and all-uppercase acronyms.
    if word.len() < 3 || word.chars().all(|c| c.is_ascii_uppercase()) {
        return true;
    }
    let lower = word.to_lowercase();
    if state.ignored_words.contains(&lower) || state.allowed_words.contains(&lower) {
        return true;
    }
    is_bundled_word(word)
}

/// Suggest two-word compound splits like "mycompany" -> "my company".
fn suggest_compound_splits(word: &str, limit: usize) -> Vec<String> {
    let lower = word.to_lowercase();
    let mut suggestions = Vec::new();
    for i in 2..lower.len().saturating_sub(1) {
        let left = &lower[..i];
        let right = &lower[i..];
        if BUNDLED_DICT.contains(left) && BUNDLED_DICT.contains(right) {
            suggestions.push(format!("{} {}", left, right));
            if suggestions.len() >= limit {
                break;
            }
        }
    }
    suggestions
}

/// Generate adjacent-letter transposition candidates for a word.
fn transposition_candidates(word: &str) -> Vec<String> {
    let chars: Vec<char> = word.chars().collect();
    let mut candidates = Vec::new();
    for i in 0..chars.len().saturating_sub(1) {
        let mut s = chars.clone();
        s.swap(i, i + 1);
        candidates.push(s.iter().collect::<String>());
    }
    candidates
}

/// Suggest corrections for an unknown word using transposition fixes, SymSpell,
/// and compound-word splits.
fn suggest_for_unknown(word: &str, limit: usize) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut suggestions = Vec::new();

    let push = |suggestions: &mut Vec<String>, seen: &mut HashSet<String>, term: String| {
        if seen.insert(term.clone()) {
            suggestions.push(term);
        }
    };

    // Adjacent transpositions are the most likely real-word fixes (e.g. teh -> the).
    for candidate in transposition_candidates(word) {
        if BUNDLED_DICT.contains(candidate.as_str()) {
            push(&mut suggestions, &mut seen, candidate);
            if suggestions.len() >= limit {
                return suggestions;
            }
        }
    }

    if let Some(symspell) = SYM_SPELL.as_ref() {
        for s in symspell.lookup(word, Verbosity::Closest, 2) {
            if s.distance > 0 {
                push(&mut suggestions, &mut seen, s.term);
                if suggestions.len() >= limit {
                    return suggestions;
                }
            }
        }
    }

    for split in suggest_compound_splits(word, limit - suggestions.len()) {
        push(&mut suggestions, &mut seen, split);
        if suggestions.len() >= limit {
            break;
        }
    }

    suggestions
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

// Use RwLock to allow reloading custom corrections at runtime.
static CUSTOM_CORRECTIONS: LazyLock<RwLock<HashMap<String, String>>> =
    LazyLock::new(|| RwLock::new(load_custom_corrections()));

/// Force reload custom corrections from file.
/// This should be called after updating the corrections file.
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

/// Check for typos in the given text.
pub fn check_typos(text: &str) -> Vec<TypoError> {
    let mut typos_errors: Vec<TypoError> = Vec::new();

    // Pass 1: collect all typos from the typos library (byte offsets only, line/col deferred)
    let results = typos::check_str(text, POLICY.tokenizer, POLICY.dict);

    for typo in results {
        if let Status::Corrections(corrections) = typo.corrections {
            let typo_word = typo.typo.to_string();

            if is_bundled_word(typo_word.as_str()) || is_cspell_allowed(typo_word.as_str()) {
                log::debug!("Skipping '{}' - found in dictionary", typo_word);
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

    // Pass 2: apply custom corrections + CSpell flagWords + unknown-word detection.
    {
        let guard = CUSTOM_CORRECTIONS.read().ok();
        let custom_corrections = guard.as_ref();

        let mut extra_corrections: HashMap<String, Vec<String>> = HashMap::new();
        let state_guard = C_SPELL_STATE.read().ok();

        if let Some(state) = state_guard.as_ref() {
            // CSpell flagWords first; file-based custom corrections override them.
            if state.enabled {
                for (typo, suggestions) in &state.flag_corrections {
                    extra_corrections.insert(typo.clone(), suggestions.clone());
                }
            }
        }

        if let Some(corrections) = custom_corrections.filter(|c| !c.is_empty()) {
            for (typo, correction) in corrections.iter() {
                extra_corrections.insert(typo.clone(), vec![correction.clone()]);
            }
        }

        let should_scan = !extra_corrections.is_empty()
            || state_guard
                .as_ref()
                .map_or(false, |s| s.enabled && s.unknown_word_detection_enabled);

        if should_scan {
            let mut flagged_offsets: HashSet<usize> =
                typos_errors.iter().map(|e| e.byte_offset).collect();

            let mut word_start: Option<usize> = None;
            let mut word = String::new();
            for (i, ch) in text.char_indices() {
                if ch.is_alphabetic() || ch == '\'' {
                    if word_start.is_none() {
                        word_start = Some(i);
                    }
                    word.push(ch);
                } else if let Some(start) = word_start.take() {
                    apply_token_corrections(
                        &word,
                        start,
                        &extra_corrections,
                        state_guard.as_deref(),
                        &mut flagged_offsets,
                        &mut typos_errors,
                    );
                    word.clear();
                }
            }
            if let Some(start) = word_start {
                apply_token_corrections(
                    &word,
                    start,
                    &extra_corrections,
                    state_guard.as_deref(),
                    &mut flagged_offsets,
                    &mut typos_errors,
                );
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

/// Apply custom / CSpell flagWord corrections and/or unknown-word detection to a
/// single token (no line/col — deferred to batch_line_col).
fn apply_token_corrections(
    word: &str,
    word_start: usize,
    corrections: &HashMap<String, Vec<String>>,
    state: Option<&CSpellState>,
    flagged_offsets: &mut HashSet<usize>,
    typos_errors: &mut Vec<TypoError>,
) {
    if flagged_offsets.contains(&word_start) {
        return;
    }

    let word_lower = word.to_lowercase();
    let ignored = state.map_or(false, |s| s.ignored_words.contains(word_lower.as_str()));
    if ignored {
        return;
    }

    // Custom / flagWord correction takes precedence.
    if let Some(suggestions) = corrections.get(word_lower.as_str()) {
        typos_errors.push(TypoError {
            typo: word.to_string(),
            suggestions: suggestions.clone(),
            byte_offset: word_start,
            line: 0,
            col: 0,
        });
        flagged_offsets.insert(word_start);
        return;
    }

    // Unknown-word detection: flag any token not in the bundled dictionary or
    // cspell allow-list.
    let unknown_detection = state.map_or(false, |s| s.enabled && s.unknown_word_detection_enabled);
    if unknown_detection && !is_known_word(word, state.unwrap()) {
        let suggestions = suggest_for_unknown(word, 5);
        typos_errors.push(TypoError {
            typo: word.to_string(),
            suggestions,
            byte_offset: word_start,
            line: 0,
            col: 0,
        });
        flagged_offsets.insert(word_start);
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
    use std::sync::Mutex;

    /// Serialize tests that mutate the shared CSpell state to avoid flaky races.
    static C_SPELL_TEST_LOCK: Mutex<()> = Mutex::new(());

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

    #[test]
    fn test_parse_flag_word() {
        assert_eq!(
            parse_flag_word("color: colour"),
            Some(("color".to_string(), vec!["colour".to_string()]))
        );
        assert_eq!(
            parse_flag_word("canot->cannot"),
            Some(("canot".to_string(), vec!["cannot".to_string()]))
        );
        assert_eq!(
            parse_flag_word("incase: in case, encase"),
            Some((
                "incase".to_string(),
                vec!["in case".to_string(), "encase".to_string()]
            ))
        );
        assert_eq!(
            parse_flag_word("forbidden"),
            Some(("forbidden".to_string(), Vec::new()))
        );
        assert_eq!(parse_flag_word(""), None);
    }

    #[test]
    fn test_cspell_allowed_word_suppresses_typos() {
        let _guard = C_SPELL_TEST_LOCK.lock().unwrap();

        // "tauri" is in the bundled dictionary, so this tests the cspell path
        // with a word that is not in the bundled list.
        let rare_word = "zzzzzzzzzzzzzz";
        assert!(
            !is_bundled_word(rare_word),
            "Test word should not be bundled"
        );

        // Simulate a CSpell config with the rare word as allowed.
        {
            let mut state = C_SPELL_STATE.write().unwrap();
            state.config = CSpellConfig {
                words: vec![rare_word.to_string()],
                ..Default::default()
            };
            state.enabled = true;
            state.rebuild();
        }

        assert!(is_cspell_allowed(rare_word));

        // Restore default state.
        {
            let mut state = C_SPELL_STATE.write().unwrap();
            state.config = CSpellConfig::default();
            state.rebuild();
        }
    }

    #[test]
    fn test_cspell_flag_word_detected() {
        let _guard = C_SPELL_TEST_LOCK.lock().unwrap();

        let mut state = C_SPELL_STATE.write().unwrap();
        state.config = CSpellConfig {
            flag_words: vec!["calor->colour".to_string()],
            ..Default::default()
        };
        state.enabled = true;
        state.rebuild();
        drop(state);

        let typos = check_typos("The color is calor");
        let found = typos.iter().any(|t| t.typo.to_lowercase() == "calor");
        assert!(found, "Expected CSpell flagWord 'calor' to be detected");

        let mut state = C_SPELL_STATE.write().unwrap();
        state.config = CSpellConfig::default();
        state.rebuild();
    }

    #[test]
    fn test_cspell_flag_word_from_file() {
        let _guard = C_SPELL_TEST_LOCK.lock().unwrap();

        let config_path = std::env::temp_dir().join(format!(
            "autocorrect-cspell-test-{}.json",
            std::process::id()
        ));
        let config = serde_json::json!({
            "flagWords": ["mycompany"],
            "words": [],
            "ignoreWords": []
        });
        std::fs::write(&config_path, config.to_string()).unwrap();

        set_cspell_state(true, false, Some(config_path.to_string_lossy().to_string()));

        let typos = check_typos("mycompany");
        let found = typos.iter().any(|t| t.typo.to_lowercase() == "mycompany");
        assert!(
            found,
            "Expected CSpell flagWord 'mycompany' to be detected, got {:?}",
            typos
        );

        // Restore default state.
        set_cspell_state(true, false, None);
        let _ = std::fs::remove_file(&config_path);
    }

    #[test]
    fn test_unknown_word_detection() {
        let _guard = C_SPELL_TEST_LOCK.lock().unwrap();

        // "mycompany" is not in the bundled dictionary.
        assert!(!is_bundled_word("mycompany"));

        {
            let mut state = C_SPELL_STATE.write().unwrap();
            state.config = CSpellConfig::default();
            state.enabled = true;
            state.unknown_word_detection_enabled = true;
            state.rebuild();
        }

        let typos = check_typos("mycompany is here");
        let found = typos.iter().any(|t| t.typo.to_lowercase() == "mycompany");
        assert!(
            found,
            "Expected 'mycompany' to be flagged as unknown word, got {:?}",
            typos
        );

        // Adding it to words should suppress the flag.
        {
            let mut state = C_SPELL_STATE.write().unwrap();
            state.config = CSpellConfig {
                words: vec!["mycompany".to_string()],
                ..Default::default()
            };
            state.rebuild();
        }

        let typos = check_typos("mycompany is here");
        assert!(
            !typos.iter().any(|t| t.typo.to_lowercase() == "mycompany"),
            "Expected 'mycompany' to be allowed after adding to words"
        );

        // Restore default state.
        {
            let mut state = C_SPELL_STATE.write().unwrap();
            state.config = CSpellConfig::default();
            state.unknown_word_detection_enabled = false;
            state.rebuild();
        }
    }

    #[test]
    fn test_unknown_word_suggestions() {
        let mycompany_suggestions = suggest_for_unknown("mycompany", 5);
        println!("mycompany suggestions: {:?}", mycompany_suggestions);
        assert!(
            mycompany_suggestions.contains(&"my company".to_string()),
            "Expected compound split suggestion for 'mycompany'"
        );

        let teh_suggestions = suggest_for_unknown("teh", 5);
        println!("teh suggestions: {:?}", teh_suggestions);
        assert!(
            teh_suggestions.contains(&"the".to_string()),
            "Expected edit-distance suggestion for 'teh'"
        );
    }
}
