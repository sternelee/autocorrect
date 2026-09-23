use crate::commands::config::AppSettings;
use crate::commands::errors::Error;
use crate::translation::local_mt::{self};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TranslationProvider {
    OpenAi,
    Apple,
    Local,
}

/// Runtime information about a translation provider.
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationProviderInfo {
    pub id: String,
    pub name: String,
    pub available: bool,
    pub error: Option<String>,
}

/// Translate text using a configured local provider.
///
/// This is intentionally a thin wrapper around the backend implementations
/// so that callers can decide whether to fall back to OpenAI-based translation.
pub struct Translator {
    provider: TranslationProvider,
    local_model_path: PathBuf,
}

impl Translator {
    pub fn from_settings(settings: &AppSettings) -> Self {
        Self {
            provider: parse_provider(&settings.ai_translation_provider),
            local_model_path: PathBuf::from(&settings.ai_translation_local_model_path),
        }
    }

    pub fn provider(&self) -> TranslationProvider {
        self.provider
    }

    pub async fn translate(
        &self,
        text: &str,
        source_lang: &str,
        target_lang: &str,
    ) -> Result<String, Error> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Ok(String::new());
        }

        match self.provider {
            TranslationProvider::OpenAi => Err(Error::Api(
                "OpenAI translation should be handled by the caller".to_string(),
            )),
            TranslationProvider::Apple => {
                let text = trimmed.to_string();
                let source = source_lang.to_string();
                let target = target_lang.to_string();
                tokio::task::spawn_blocking(move || {
                    crate::translation::apple::translate(&text, &source, &target)
                })
                .await
                .map_err(|e| Error::Api(format!("Apple Translation task failed: {e}")))?
            }
            TranslationProvider::Local => {
                if self.local_model_path.as_os_str().is_empty() {
                    return Err(Error::Config(
                        "Local translation model path is empty".to_string(),
                    ));
                }
                if !self.local_model_path.exists() {
                    return Err(Error::Config(format!(
                        "Local translation model path does not exist: {}",
                        self.local_model_path.display()
                    )));
                }
                let source =
                    if source_lang.eq_ignore_ascii_case("auto") || source_lang.trim().is_empty() {
                        None
                    } else {
                        Some(source_lang)
                    };
                let backend = local_mt::load_backend_cached(&self.local_model_path, source)?;
                let backend = Arc::clone(&backend);
                let text = trimmed.to_string();
                let target =
                    normalize_language_code(target_lang).unwrap_or_else(|| target_lang.to_string());
                tokio::task::spawn_blocking(move || backend.translate(&text, &target))
                    .await
                    .map_err(|e| Error::Api(format!("local MT task failed: {e}")))?
            }
        }
    }
}

pub fn parse_provider(value: &str) -> TranslationProvider {
    match value.trim().to_lowercase().as_str() {
        "apple" => TranslationProvider::Apple,
        "local" | "local_marian" | "marian" => TranslationProvider::Local,
        _ => TranslationProvider::OpenAi,
    }
}

/// List available translation providers and their runtime status.
pub fn list_providers(settings: &AppSettings) -> Vec<TranslationProviderInfo> {
    let mut providers = Vec::new();

    providers.push(TranslationProviderInfo {
        id: "openai".to_string(),
        name: "OpenAI / OpenRouter".to_string(),
        available: !settings.openai_api_key.is_empty(),
        error: if settings.openai_api_key.is_empty() {
            Some("API key is not configured".to_string())
        } else {
            None
        },
    });

    #[cfg(target_os = "macos")]
    {
        let available = crate::translation::apple::is_available();
        providers.push(TranslationProviderInfo {
            id: "apple".to_string(),
            name: "Apple Translation (on-device)".to_string(),
            available,
            error: if available {
                None
            } else {
                Some("Apple Translation requires macOS 26.0 or later".to_string())
            },
        });
    }

    #[cfg(not(target_os = "macos"))]
    {
        providers.push(TranslationProviderInfo {
            id: "apple".to_string(),
            name: "Apple Translation (macOS only)".to_string(),
            available: false,
            error: Some("Apple Translation is only available on macOS".to_string()),
        });
    }

    let local_path = PathBuf::from(&settings.ai_translation_local_model_path);
    let (available, error) = if local_path.as_os_str().is_empty() {
        (
            false,
            Some("Local model path is not configured".to_string()),
        )
    } else if !local_path.exists() {
        (
            false,
            Some(format!(
                "Local model path does not exist: {}",
                local_path.display()
            )),
        )
    } else {
        (local_mt::model_files_ready(&local_path), None)
    };
    providers.push(TranslationProviderInfo {
        id: "local".to_string(),
        name: "Local Marian / NLLB ONNX".to_string(),
        available,
        error: if available { None } else { error },
    });

    providers
}

/// Convert a natural-language or ISO language name into an ISO code that the
/// backends can consume. Returns `None` for "auto" or empty inputs.
pub fn normalize_language_code(lang: &str) -> Option<String> {
    let trimmed = lang.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("auto") {
        return None;
    }

    let lower = trimmed.to_lowercase();
    if let Some(tag) = match lower.as_str() {
        "english" => Some("en"),
        "chinese" | "simplified chinese" | "mandarin" | "简体中文" => Some("zh-Hans"),
        "traditional chinese" | "繁體中文" => Some("zh-Hant"),
        "japanese" | "日本語" => Some("ja"),
        "korean" | "한국어" => Some("ko"),
        "spanish" | "español" => Some("es"),
        "french" | "français" => Some("fr"),
        "german" | "deutsch" => Some("de"),
        "russian" | "русский" => Some("ru"),
        "portuguese" | "português" => Some("pt"),
        "italian" | "italiano" => Some("it"),
        "arabic" | "العربية" => Some("ar"),
        "hindi" | "हिन्दी" => Some("hi"),
        "vietnamese" | "tiếng việt" => Some("vi"),
        "thai" | "ไทย" => Some("th"),
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
    fn parse_provider_recognizes_ids() {
        assert!(matches!(
            parse_provider("apple"),
            TranslationProvider::Apple
        ));
        assert!(matches!(
            parse_provider("local"),
            TranslationProvider::Local
        ));
        assert!(matches!(
            parse_provider("OpenAI"),
            TranslationProvider::OpenAi
        ));
        assert!(matches!(parse_provider(""), TranslationProvider::OpenAi));
    }

    #[test]
    fn normalize_language_code_maps_names() {
        assert_eq!(normalize_language_code("English").as_deref(), Some("en"));
        assert_eq!(
            normalize_language_code("简体中文").as_deref(),
            Some("zh-Hans")
        );
        assert_eq!(normalize_language_code("auto"), None);
        assert_eq!(normalize_language_code(""), None);
    }
}
