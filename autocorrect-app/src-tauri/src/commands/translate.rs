use super::config::load_app_settings;
use super::errors::Error;
use crate::translation::local_mt::model_files_ready;
use crate::translation::translator::{list_providers, TranslationProviderInfo, Translator};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::Emitter;

const HF_HUB_BASE: &str = "https://huggingface.co";

/// Files required for a local Marian/NLLB ONNX model.
const REQUIRED_MODEL_FILES: &[&str] = &[
    "encoder_model.onnx",
    "decoder_model_merged.onnx",
    "decoder_model.onnx",
    "tokenizer.json",
];

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslateRequest {
    pub text: String,
    pub source_lang: Option<String>,
    pub target_lang: String,
    pub force_provider: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslateResponse {
    pub translated_text: String,
    pub provider_used: String,
    pub fallback_used: bool,
}

/// List translation providers available on this system.
#[tauri::command]
pub fn list_translation_providers(
    app: tauri::AppHandle,
) -> Result<Vec<TranslationProviderInfo>, Error> {
    let settings = load_app_settings(&app)?;
    Ok(list_providers(&settings))
}

/// Translate text using the configured provider.
/// Falls back between providers when the primary is unavailable.
#[tauri::command]
pub async fn translate_text(
    app: tauri::AppHandle,
    request: TranslateRequest,
) -> Result<TranslateResponse, Error> {
    let settings = load_app_settings(&app)?;
    let source_lang = request
        .source_lang
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or(&settings.ai_translate_source_language);
    let target_lang = &request.target_lang;

    let translator = Translator::from_settings(&settings);
    let effective_provider = request
        .force_provider
        .as_deref()
        .filter(|p| !p.is_empty())
        .map_or(translator.provider(), |p| {
            crate::translation::translator::parse_provider(p)
        });

    log::info!(
        "[translation] translate request provider={:?} source={source_lang} target={target_lang} text_len={}",
        effective_provider,
        request.text.len()
    );

    match effective_provider {
        crate::translation::translator::TranslationProvider::OpenAi => {
            let api_key = settings.openai_api_key.trim();
            if api_key.is_empty() {
                // Fallback to Apple or local if configured
                return fallback_translate(&settings, &request.text, source_lang, target_lang)
                    .await;
            }

            let model = settings.openai_model.clone();
            let api_base_url = settings.ai_api_base_url.clone();
            let timeout_ms = settings.ai_timeout_ms;
            let operation = "translate";

            let system_prompt =
                super::ai_grammar::build_system_prompt_for_translation(target_lang)?;

            let payload = serde_json::json!({
                "model": model,
                "temperature": 0,
                "messages": [
                    { "role": "system", "content": system_prompt },
                    { "role": "user", "content": &request.text }
                ]
            });

            let client = tauri_plugin_http::reqwest::Client::builder()
                .timeout(std::time::Duration::from_millis(timeout_ms))
                .build()
                .map_err(|e| Error::Api(format!("Failed to build HTTP client: {e}")))?;

            let response = client
                .post(&api_base_url)
                .header("Content-Type", "application/json")
                .bearer_auth(api_key)
                .body(payload.to_string())
                .send()
                .await
                .map_err(|e| Error::Api(format!("HTTP request failed: {e}")))?;

            let status = response.status();
            let body = response
                .text()
                .await
                .map_err(|e| Error::Api(format!("Failed to read HTTP response body: {e}")))?;

            if !status.is_success() {
                return Err(Error::Api(format!(
                    "AI request failed with status {}: {}",
                    status, body
                )));
            }

            let value: serde_json::Value = serde_json::from_str(&body)
                .map_err(|e| Error::Api(format!("Invalid AI response JSON: {}", e)))?;

            let content = super::ai_grammar::extract_content(&value);
            if content.trim().is_empty() {
                return Err(Error::Api("AI returned empty content".to_string()));
            }

            Ok(TranslateResponse {
                translated_text: content.trim().to_string(),
                provider_used: "openai".to_string(),
                fallback_used: false,
            })
        }
        _ => {
            match translator
                .translate(&request.text, source_lang, target_lang)
                .await
            {
                Ok(translated) => Ok(TranslateResponse {
                    translated_text: translated,
                    provider_used: match effective_provider {
                        crate::translation::translator::TranslationProvider::Apple => {
                            "apple".to_string()
                        }
                        crate::translation::translator::TranslationProvider::Local => {
                            "local".to_string()
                        }
                        _ => "unknown".to_string(),
                    },
                    fallback_used: false,
                }),
                Err(e) => Err(e),
            }
        }
    }
}

/// Request to download a translation model from Hugging Face.
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadTranslationModelRequest {
    /// Hugging Face model id, e.g. `Helsinki-NLP/opus-mt-zh-en`.
    pub model_id: String,
    /// Directory where the model files will be saved.
    pub target_dir: String,
}

/// Status of a local translation model directory.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationModelStatus {
    pub ready: bool,
    pub path: String,
    pub files_present: Vec<String>,
    pub files_missing: Vec<String>,
}

/// Return a curated list of local translation models that can be downloaded.
#[tauri::command]
pub fn list_downloadable_translation_models() -> Vec<DownloadableTranslationModel> {
    vec![
        DownloadableTranslationModel {
            id: "Helsinki-NLP/opus-mt-zh-en".to_string(),
            name: "Marian: Chinese → English".to_string(),
            source_lang: "zh".to_string(),
            target_lang: "en".to_string(),
        },
        DownloadableTranslationModel {
            id: "Helsinki-NLP/opus-mt-en-zh".to_string(),
            name: "Marian: English → Chinese".to_string(),
            source_lang: "en".to_string(),
            target_lang: "zh".to_string(),
        },
        DownloadableTranslationModel {
            id: "facebook/nllb-200-distilled-600M".to_string(),
            name: "NLLB-200 600M ( multilingual )".to_string(),
            source_lang: "auto".to_string(),
            target_lang: "en".to_string(),
        },
    ]
}

/// Metadata about a downloadable translation model.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadableTranslationModel {
    pub id: String,
    pub name: String,
    pub source_lang: String,
    pub target_lang: String,
}

/// Check whether a local model directory has the files needed to load.
#[tauri::command]
pub fn get_translation_model_status(model_path: String) -> Result<TranslationModelStatus, Error> {
    let path = PathBuf::from(&model_path);
    let mut files_present = Vec::new();
    let mut files_missing = Vec::new();

    for file in REQUIRED_MODEL_FILES {
        if path.join(file).exists() {
            files_present.push(file.to_string());
        } else {
            files_missing.push(file.to_string());
        }
    }

    // `decoder_model_merged.onnx` OR `decoder_model.onnx` is sufficient.
    let has_decoder = files_present
        .iter()
        .any(|f| f == "decoder_model_merged.onnx" || f == "decoder_model.onnx");

    let ready = model_files_ready(&path)
        || (path.exists()
            && files_present.contains(&"encoder_model.onnx".to_string())
            && has_decoder
            && files_present.contains(&"tokenizer.json".to_string()));

    Ok(TranslationModelStatus {
        ready,
        path: model_path,
        files_present,
        files_missing,
    })
}

/// Download a translation model from Hugging Face.
#[tauri::command]
pub async fn download_translation_model(
    app: tauri::AppHandle,
    request: DownloadTranslationModelRequest,
) -> Result<String, Error> {
    let model_id = request.model_id.trim();
    if model_id.is_empty() {
        return Err(Error::Config("model id is required".to_string()));
    }

    let target_dir = PathBuf::from(request.target_dir.trim());
    if target_dir.as_os_str().is_empty() {
        return Err(Error::Config("target directory is required".to_string()));
    }

    std::fs::create_dir_all(&target_dir).map_err(Error::Io)?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(600))
        .build()
        .map_err(|e| Error::Api(format!("failed to build HTTP client: {e}")))?;

    // Determine which decoder variant is available.
    let mut files_to_download = vec!["encoder_model.onnx", "tokenizer.json"];
    let decoder_file = if file_exists_on_hf(&client, model_id, "decoder_model_merged.onnx").await? {
        "decoder_model_merged.onnx"
    } else if file_exists_on_hf(&client, model_id, "decoder_model.onnx").await? {
        "decoder_model.onnx"
    } else {
        return Err(Error::Api(format!(
            "model {model_id} does not provide decoder_model_merged.onnx or decoder_model.onnx on Hugging Face"
        )));
    };
    files_to_download.push(decoder_file);

    let _ = app.emit(
        "translation-model-download-started",
        serde_json::json!({ "modelId": model_id, "targetDir": target_dir }),
    );

    for file in &files_to_download {
        let url = format!("{}/{}/resolve/main/{}", HF_HUB_BASE, model_id, file);
        let dest = target_dir.join(file);

        let _ = app.emit(
            "translation-model-download-file-started",
            serde_json::json!({ "file": file }),
        );

        download_file_with_progress(&client, &url, &dest, &app, file).await?;

        let _ = app.emit(
            "translation-model-download-file-done",
            serde_json::json!({ "file": file }),
        );
    }

    let _ = app.emit(
        "translation-model-download-done",
        serde_json::json!({ "modelId": model_id, "targetDir": target_dir }),
    );

    Ok(target_dir.to_string_lossy().to_string())
}

async fn file_exists_on_hf(
    client: &reqwest::Client,
    model_id: &str,
    file: &str,
) -> Result<bool, Error> {
    let url = format!("{}/{}/resolve/main/{}", HF_HUB_BASE, model_id, file);
    let response = client
        .head(&url)
        .send()
        .await
        .map_err(|e| Error::Api(format!("HEAD request failed for {url}: {e}")))?;
    Ok(response.status().is_success())
}

async fn download_file_with_progress(
    client: &reqwest::Client,
    url: &str,
    dest: &std::path::Path,
    app: &tauri::AppHandle,
    file_name: &str,
) -> Result<(), Error> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| Error::Api(format!("download failed for {url}: {e}")))?;

    let status = response.status();
    if !status.is_success() {
        return Err(Error::Api(format!(
            "download failed for {url} with status {status}"
        )));
    }

    let total = response.content_length();
    let mut stream = response.bytes_stream();
    let mut file = tokio::fs::File::create(dest).await.map_err(Error::Io)?;
    let mut downloaded: u64 = 0;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| Error::Api(format!("stream error: {e}")))?;
        tokio::io::AsyncWriteExt::write_all(&mut file, &chunk)
            .await
            .map_err(Error::Io)?;
        downloaded += chunk.len() as u64;

        if let Some(total) = total {
            let percent = (downloaded as f64 / total as f64 * 100.0) as u32;
            let _ = app.emit(
                "translation-model-download-progress",
                serde_json::json!({
                    "file": file_name,
                    "downloaded": downloaded,
                    "total": total,
                    "percent": percent,
                }),
            );
        }
    }

    Ok(())
}

/// Attempt to translate using Apple or local fallback when OpenAI is unavailable.
async fn fallback_translate(
    settings: &super::config::AppSettings,
    text: &str,
    source_lang: &str,
    target_lang: &str,
) -> Result<TranslateResponse, Error> {
    // Try Apple Translation first
    #[cfg(target_os = "macos")]
    {
        if crate::translation::apple::is_available() {
            match crate::translation::apple::translate(text, source_lang, target_lang) {
                Ok(translated) => {
                    return Ok(TranslateResponse {
                        translated_text: translated,
                        provider_used: "apple".to_string(),
                        fallback_used: true,
                    });
                }
                Err(e) => {
                    log::warn!("[translation] Apple fallback failed: {e}");
                }
            }
        }
    }

    // Try local Marian/NLLB
    {
        let translator = Translator::from_settings(settings);
        match translator.translate(text, source_lang, target_lang).await {
            Ok(translated) => {
                return Ok(TranslateResponse {
                    translated_text: translated,
                    provider_used: "local".to_string(),
                    fallback_used: true,
                });
            }
            Err(e) => {
                log::warn!("[translation] local fallback failed: {e}");
            }
        }
    }

    Err(Error::Api(
        "No translation provider is available. Configure an API key, enable Apple Translation, or set a local model path."
            .to_string(),
    ))
}
