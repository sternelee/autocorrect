//! LanguageTool grammar engine integration.
//!
//! Talks to a LanguageTool server (local/self-hosted by default — the
//! engine itself is Java, so it runs outside this process; see
//! docker-compose.languagetool.yml). The public languagetool.org API is
//! deliberately NOT a default: text must never leave the machine without
//! an explicit user choice.
//!
//! The overlay poll loop calls [`request_check`] after each text change;
//! results are cached per text hash and merged into the marker pipeline
//! on the next cycle (see [`cached`]). Unreachable/timeout servers are
//! treated as "no issues" — local spell checking keeps working offline.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// One grammar/style issue reported by LanguageTool.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrammarIssue {
    pub message: String,
    pub short_message: String,
    /// Match offset in the checked text (UTF-16 code units per LT API).
    pub offset: usize,
    pub length: usize,
    pub replacements: Vec<String>,
    pub rule_id: String,
    pub category: String,
}

impl GrammarIssue {
    /// The substring of `text` this issue refers to (empty when out of
    /// range — the marker pipeline skips empties).
    pub fn text_slice<'t>(&self, text: &'t str) -> &'t str {
        let units: Vec<u16> = text.encode_utf16().collect();
        let end = self.offset.saturating_add(self.length);
        if end <= units.len() {
            match (
                utf16_offset_to_byte(text, self.offset),
                utf16_offset_to_byte(text, end),
            ) {
                (Some(start), Some(end)) => &text[start..end],
                // Offset/length lands mid-surrogate or out of bounds — the
                // marker pipeline skips empty slices.
                _ => "",
            }
        } else {
            ""
        }
    }
}

/// Map a UTF-16 code-unit offset to a byte offset in `s` (None when the
/// offset falls inside a surrogate pair or past the end).
pub fn utf16_offset_to_byte(s: &str, target: usize) -> Option<usize> {
    let mut units = 0usize;
    for (byte, ch) in s.char_indices() {
        if units == target {
            return Some(byte);
        }
        units += ch.len_utf16();
        if units > target {
            return None; // inside a surrogate pair
        }
    }
    if units == target {
        Some(s.len())
    } else {
        None
    }
}

/// True when the text is Latin-dominant and worth sending to LanguageTool.
/// LT has no CJK grammar rules, and Chinese-dominant drafts would just pay
/// a roundtrip for "language not supported" noise.
pub fn is_latin_dominant(text: &str) -> bool {
    let mut letters = 0usize;
    let mut cjk = 0usize;
    for ch in text.chars() {
        if ch.is_ascii_alphabetic() {
            letters += 1;
        } else if ('\u{4E00}'..='\u{9FFF}').contains(&ch)
            || ('\u{3040}'..='\u{30FF}').contains(&ch)
            || ('\u{AC00}'..='\u{D7AF}').contains(&ch)
        {
            cjk += 1;
        }
    }
    letters > 0 && letters >= cjk
}

/// Cap per-request size so huge documents don't hog the server.
const MAX_CHECK_CHARS: usize = 4000;
const CHECK_TIMEOUT: Duration = Duration::from_secs(3);
/// Cached results older than this are dropped even when the text hash
/// matches (rules/dictionary on the server may have changed).
const CACHE_TTL: Duration = Duration::from_secs(300);

static IN_FLIGHT: AtomicBool = AtomicBool::new(false);
static LAST_RESULT: Mutex<Option<(u64, Vec<GrammarIssue>, Instant)>> = Mutex::new(None);

/// Cached grammar issues for `text_hash`, if any.
pub fn cached(text_hash: u64) -> Option<Vec<GrammarIssue>> {
    let guard = LAST_RESULT.lock().ok()?;
    let (hash, issues, at) = guard.as_ref()?;
    if *hash == text_hash && at.elapsed() < CACHE_TTL {
        Some(issues.clone())
    } else {
        None
    }
}

/// Spawn a background check for `text` unless one is already running.
/// Results land in the cache and are picked up by the next poll cycle.
pub fn request_check(text_hash: u64, text: String, endpoint: String) {
    if text.trim().is_empty() || text.chars().count() > MAX_CHECK_CHARS {
        return;
    }
    if !is_latin_dominant(&text) {
        return;
    }
    if IN_FLIGHT
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return; // a check is already running; its result will cover us
    }

    tauri::async_runtime::spawn(async move {
        let result =
            tokio::time::timeout(CHECK_TIMEOUT, check_with_server(&text, &endpoint, "auto")).await;
        let issues = match result {
            Ok(Ok(issues)) => issues,
            Ok(Err(e)) => {
                log::debug!("[GRAMMAR] LanguageTool unavailable: {e}");
                Vec::new()
            }
            Err(_) => {
                log::debug!("[GRAMMAR] LanguageTool timed out after {CHECK_TIMEOUT:?}");
                Vec::new()
            }
        };
        if let Ok(mut guard) = LAST_RESULT.lock() {
            *guard = Some((text_hash, issues, Instant::now()));
        }
        IN_FLIGHT.store(false, Ordering::SeqCst);
    });
}

/// Run a synchronous check against `endpoint` (e.g. "http://127.0.0.1:8081").
pub async fn check_with_server(
    text: &str,
    endpoint: &str,
    language: &str,
) -> Result<Vec<GrammarIssue>, String> {
    let (hostname, port) = split_endpoint(endpoint);
    let client = languagetool_rust::api::server::ServerClient::new(&hostname, &port);
    let request = languagetool_rust::api::check::Request::default()
        .with_text(text)
        .with_language(language.to_string());
    let response = client
        .check(&request)
        .await
        .map_err(|e| format!("LanguageTool request failed: {e}"))?;

    Ok(response
        .matches
        .into_iter()
        .map(|m| GrammarIssue {
            message: m.message,
            short_message: m.short_message,
            offset: m.offset,
            length: m.length,
            replacements: m
                .replacements
                .into_iter()
                .map(|r| r.value)
                .take(5)
                .collect(),
            rule_id: m.rule.id,
            category: m.rule.category.id,
        })
        .collect())
}

/// Split a base URL like "http://127.0.0.1:8081" into the
/// (hostname, port) pair ServerClient::new expects. The hostname KEEPS its
/// scheme — ServerClient concatenates `{hostname}:{port}/v2` verbatim, so a
/// bare "127.0.0.1" would parse as a URL scheme and fail the request.
fn split_endpoint(endpoint: &str) -> (String, String) {
    let (scheme, rest) = match endpoint.split_once("://") {
        Some((scheme, rest)) => (format!("{scheme}://"), rest),
        None => ("http://".to_string(), endpoint),
    };
    let rest = rest.trim_end_matches('/');
    match rest.rsplit_once(':') {
        Some((host, port)) if !port.is_empty() && port.chars().all(|c| c.is_ascii_digit()) => {
            (format!("{scheme}{host}"), port.to_string())
        }
        _ => (format!("{scheme}{rest}"), String::new()),
    }
}

/// Tauri command: run a one-off grammar check (used by the settings test
/// UI and diagnostics).
#[tauri::command]
pub async fn grammar_check_text(text: String) -> Result<Vec<GrammarIssue>, String> {
    let endpoint = "http://127.0.0.1:8081".to_string();
    tokio::time::timeout(CHECK_TIMEOUT, check_with_server(&text, &endpoint, "auto"))
        .await
        .map_err(|_| "LanguageTool request timed out".to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_latin_dominant() {
        assert!(is_latin_dominant("hello world this is English"));
        assert!(is_latin_dominant("mixed 中文 and English text"));
        assert!(!is_latin_dominant("这是一段中文文本"));
        assert!(!is_latin_dominant(""));
        assert!(!is_latin_dominant("12345 !!!"));
    }

    #[test]
    fn test_text_slice_utf16() {
        let issue = GrammarIssue {
            message: "m".into(),
            short_message: String::new(),
            offset: 9,
            length: 3,
            replacements: vec![],
            rule_id: String::new(),
            category: String::new(),
        };
        assert_eq!(issue.text_slice("hello 👋 teh world"), "teh");
        // Offset inside a surrogate pair → empty slice, marker skipped.
        let bad = GrammarIssue {
            offset: 7,
            length: 2,
            ..issue.clone()
        };
        assert_eq!(bad.text_slice("hello 👋 x"), "");
        let oob = GrammarIssue {
            offset: 99,
            length: 2,
            ..issue
        };
        assert_eq!(oob.text_slice("short"), "");
    }

    #[test]
    fn test_split_endpoint() {
        assert_eq!(
            split_endpoint("http://127.0.0.1:8081"),
            ("http://127.0.0.1".to_string(), "8081".to_string())
        );
        assert_eq!(
            split_endpoint("https://languagetool.example.com"),
            (
                "https://languagetool.example.com".to_string(),
                String::new()
            )
        );
        // Scheme-less input defaults to http.
        assert_eq!(
            split_endpoint("127.0.0.1:8081"),
            ("http://127.0.0.1".to_string(), "8081".to_string())
        );
    }

    #[test]
    fn test_utf16_offset_to_byte() {
        let s = "a👋b";
        assert_eq!(utf16_offset_to_byte(s, 0), Some(0));
        assert_eq!(utf16_offset_to_byte(s, 1), Some(1));
        assert_eq!(utf16_offset_to_byte(s, 2), None); // mid-surrogate
        assert_eq!(utf16_offset_to_byte(s, 3), Some(5));
        assert_eq!(utf16_offset_to_byte(s, 4), Some(6));
        assert_eq!(utf16_offset_to_byte(s, 5), None);
    }

    /// End-to-end check against a running local server (start it with
    /// docker-compose.languagetool.yml or the official zip):
    /// cargo test -p autocorrect-app --lib test_languagetool_e2e -- --ignored --nocapture
    #[test]
    #[ignore]
    fn test_languagetool_e2e() {
        let issues = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(check_with_server(
                "This are a smal test with a individal issue.",
                "http://127.0.0.1:8081",
                "en-US",
            ))
            .expect("LanguageTool server reachable");
        println!("found {} grammar issues:", issues.len());
        for i in &issues {
            println!("  [{}] {} -> {:?}", i.rule_id, i.message, i.replacements);
        }
        assert!(
            issues
                .iter()
                .any(|i| i.rule_id == "IT_IS" || i.message.to_uppercase().contains("ARE")),
            "expected a subject-verb issue for 'This are'"
        );
    }
}
