use crate::commands::errors::Error;
use std::ffi::{c_char, CStr, CString};

#[cfg(target_os = "macos")]
extern "C" {
    fn autocorrect_apple_translation_is_available() -> i32;
    fn autocorrect_apple_translation_translate(
        source_text: *const c_char,
        source_lang: *const c_char,
        target_lang: *const c_char,
    ) -> *mut c_char;
    fn autocorrect_apple_translation_free_string(ptr: *mut c_char);
}

pub fn is_available() -> bool {
    #[cfg(target_os = "macos")]
    {
        unsafe { autocorrect_apple_translation_is_available() != 0 }
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

/// Translate `text` using the Apple Translation framework.
///
/// `source_lang` may be `"auto"` or an empty string to let the system detect it.
/// `target_lang` must be an ISO language code or a natural-language name
/// (e.g. "English", "Chinese", "ja").
pub fn translate(text: &str, source_lang: &str, target_lang: &str) -> Result<String, Error> {
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (text, source_lang, target_lang);
        return Err(Error::Api(
            "Apple Translation is only available on macOS".to_string(),
        ));
    }

    #[cfg(target_os = "macos")]
    {
        if unsafe { autocorrect_apple_translation_is_available() } == 0 {
            return Err(Error::Api(
                "Apple Translation requires macOS 26.0 or later".to_string(),
            ));
        }

        let normalized_target =
            normalize_language_for_apple(target_lang).ok_or_else(|| {
                Error::Api(format!(
                    "Apple Translation requires an ISO language code (e.g. en, zh-Hans, ja); got '{target_lang}'"
                ))
            })?;
        let normalized_source = normalize_language_for_apple(source_lang);

        let source_text = CString::new(text)
            .map_err(|_| Error::Api("translation input contains NUL byte".to_string()))?;
        let source_lang_cstring = normalized_source
            .map(|s| {
                CString::new(s)
                    .map_err(|_| Error::Api("source language contains NUL byte".to_string()))
            })
            .transpose()?;
        let target_lang_cstring = CString::new(normalized_target)
            .map_err(|_| Error::Api("target language contains NUL byte".to_string()))?;

        let ptr = unsafe {
            autocorrect_apple_translation_translate(
                source_text.as_ptr(),
                source_lang_cstring
                    .as_ref()
                    .map_or(std::ptr::null(), |lang| lang.as_ptr()),
                target_lang_cstring.as_ptr(),
            )
        };

        if ptr.is_null() {
            return Err(Error::Api(
                "Apple Translation returned an empty response".to_string(),
            ));
        }

        let translated = unsafe {
            let value = CStr::from_ptr(ptr).to_string_lossy().to_string();
            autocorrect_apple_translation_free_string(ptr);
            value
        };

        if let Some(message) = translated.strip_prefix("[error]") {
            return Err(Error::Api(message.trim().to_string()));
        }

        Ok(translated.trim().to_string())
    }
}

/// Convert a user-facing language string into an ISO-style tag suitable for
/// Apple Translation (`Locale.Language`). Returns `None` for empty or "auto"
/// inputs so callers can fall back to system language detection.
fn normalize_language_for_apple(lang: &str) -> Option<String> {
    let trimmed = lang.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("auto") {
        return None;
    }

    let lower = trimmed.to_lowercase();
    if let Some(tag) = match lower.as_str() {
        "english" => Some("en"),
        "chinese" | "simplified chinese" | "mandarin" => Some("zh-Hans"),
        "traditional chinese" => Some("zh-Hant"),
        "japanese" => Some("ja"),
        "korean" => Some("ko"),
        "spanish" => Some("es"),
        "french" => Some("fr"),
        "german" => Some("de"),
        "russian" => Some("ru"),
        "portuguese" => Some("pt"),
        "italian" => Some("it"),
        "arabic" => Some("ar"),
        "hindi" => Some("hi"),
        "vietnamese" => Some("vi"),
        "thai" => Some("th"),
        _ => None,
    } {
        return Some(tag.to_string());
    }

    if trimmed.len() <= 8
        && trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Some(trimmed.replace('_', "-"));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_language_for_apple_maps_common_names() {
        let cases = vec![
            ("English", Some("en")),
            ("english", Some("en")),
            ("  English  ", Some("en")),
            ("Chinese", Some("zh-Hans")),
            ("Traditional Chinese", Some("zh-Hant")),
            ("Japanese", Some("ja")),
            ("Korean", Some("ko")),
        ];

        for (input, expected) in cases {
            assert_eq!(
                normalize_language_for_apple(input).as_deref(),
                expected,
                "normalize_language_for_apple({:?})",
                input
            );
        }
    }

    #[test]
    fn normalize_language_for_apple_returns_none_for_auto_or_empty() {
        assert_eq!(normalize_language_for_apple("auto"), None);
        assert_eq!(normalize_language_for_apple("AUTO"), None);
        assert_eq!(normalize_language_for_apple(""), None);
        assert_eq!(normalize_language_for_apple("   "), None);
    }
}
