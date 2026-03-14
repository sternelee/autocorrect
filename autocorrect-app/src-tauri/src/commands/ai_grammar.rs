use super::config::load_app_settings;
use super::errors::Error;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AiGrammarRequest {
    pub text: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AiGrammarResponse {
    pub corrected_text: String,
    pub model: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AiTextTransformRequest {
    pub text: String,
    pub operation: String, // grammar | translate | polish
    pub target_language: Option<String>,
    pub polish_style: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AiTextTransformResponse {
    pub output_text: Option<String>,
    pub typos: Vec<AiTypo>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AiTypo {
    pub typo: String,
    pub suggestions: Vec<String>,
    pub line: usize,
    pub col: usize,
}

fn default_model() -> &'static str {
    "openai/gpt-5-nano"
}

fn default_chat_endpoint() -> &'static str {
    "https://openrouter.ai/api/v1/chat/completions"
}

fn normalize_model(model: Option<String>) -> String {
    let value = model.unwrap_or_else(|| default_model().to_string());
    let trimmed = value.trim();
    if trimmed.is_empty() {
        default_model().to_string()
    } else {
        trimmed.to_string()
    }
}

fn normalize_endpoint(endpoint: Option<String>) -> String {
    let value = endpoint.unwrap_or_else(|| default_chat_endpoint().to_string());
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return default_chat_endpoint().to_string();
    }
    if trimmed == "https://openrouter.ai/api/v1" {
        return default_chat_endpoint().to_string();
    }
    trimmed.to_string()
}

fn extract_content(value: &serde_json::Value) -> String {
    if let Some(content) = value
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
    {
        if let Some(text) = content.as_str() {
            return text.trim().to_string();
        }
        if let Some(arr) = content.as_array() {
            let merged = arr
                .iter()
                .filter_map(|part| part.get("text").and_then(|t| t.as_str()))
                .collect::<Vec<_>>()
                .join("");
            return merged.trim().to_string();
        }
    }
    String::new()
}

fn extract_json_object(content: &str) -> String {
    let trimmed = content.trim();
    if let Some(stripped) = trimmed.strip_prefix("```json") {
        return stripped.trim().trim_end_matches("```").trim().to_string();
    }
    if let Some(stripped) = trimmed.strip_prefix("```") {
        return stripped.trim().trim_end_matches("```").trim().to_string();
    }
    trimmed.to_string()
}

/// Call OpenAI chat completions and return corrected text only.
pub async fn correct_text_with_openai(
    api_base_url: &str,
    api_key: &str,
    model: &str,
    text: &str,
    timeout_ms: u64,
) -> Result<String, Error> {
    let system_prompt = "You are a precise grammar corrector. Rewrite text to fix grammar, punctuation, and phrasing while preserving original meaning, tone, and language. Return corrected text only, no markdown, no explanation.";

    let payload = json!({
        "model": model,
        "temperature": 0,
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": text }
        ]
    });

    let client = tauri_plugin_http::reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(timeout_ms))
        .build()
        .map_err(|e| Error::Api(format!("Failed to build HTTP client: {}", e)))?;

    let response = client
        .post(api_base_url)
        .header("Content-Type", "application/json")
        .bearer_auth(api_key)
        .body(payload.to_string())
        .send()
        .await
        .map_err(|e| Error::Api(format!("HTTP request failed: {}", e)))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| Error::Api(format!("Failed to read HTTP response body: {}", e)))?;

    if !status.is_success() {
        return Err(Error::Api(format!(
            "OpenAI request failed with status {}: {}",
            status, body
        )));
    }
    let value: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| Error::Api(format!("Invalid OpenAI response JSON: {}", e)))?;

    if let Some(err) = value.get("error") {
        return Err(Error::Api(format!("OpenAI returned error: {}", err)));
    }

    let content = extract_content(&value);

    if content.trim().is_empty() {
        return Err(Error::Api("OpenAI returned empty content".to_string()));
    }

    Ok(content)
}

fn build_system_prompt(
    operation: &str,
    target_language: Option<&str>,
    polish_style: Option<&str>,
) -> Result<String, Error> {
    match operation {
        "grammar" => Ok("You are a grammar checker. Return ONLY compact JSON in this exact schema: {\"typos\":[{\"typo\":\"string\",\"suggestions\":[\"string\"],\"line\":1,\"col\":1}]}. Rules: 1) Include only grammar/usage issues; 2) line/col are 1-based positions in original text; 3) If no issues, return {\"typos\":[]}; 4) No extra keys, no markdown, no prose.".to_string()),
        "translate" => {
            let lang = target_language.unwrap_or("English").trim();
            Ok(format!("You are a professional translator. Translate the user text into {}. Return translated text only. No markdown, no explanations, no prefixes.", lang))
        }
        "polish" => {
            let style = polish_style.unwrap_or("professional").trim();
            Ok(format!("You are an expert copy editor. Polish the text in a {} style. Return polished text only. No markdown, no explanations, no prefixes.", style))
        }
        _ => Err(Error::Api("Unsupported operation, expected grammar|translate|polish".to_string())),
    }
}

#[tauri::command]
pub async fn ai_grammar_check(
    app: tauri::AppHandle,
    request: AiGrammarRequest,
) -> Result<AiGrammarResponse, Error> {
    let settings = load_app_settings(&app)?;
    let api_key = settings.openai_api_key.trim().to_string();
    if api_key.is_empty() {
        return Err(Error::Api(
            "OpenAI API key is required for AI grammar check".to_string(),
        ));
    }

    let model = normalize_model(Some(settings.openai_model));
    let timeout_ms = settings.ai_timeout_ms;
    let api_base_url = normalize_endpoint(Some(settings.ai_api_base_url));

    let corrected_text =
        correct_text_with_openai(&api_base_url, &api_key, &model, &request.text, timeout_ms)
            .await?;
    Ok(AiGrammarResponse {
        corrected_text,
        model,
    })
}

#[tauri::command]
pub async fn ai_text_transform(
    app: tauri::AppHandle,
    request: AiTextTransformRequest,
) -> Result<AiTextTransformResponse, Error> {
    let settings = load_app_settings(&app)?;
    let api_key = settings.openai_api_key.trim().to_string();
    if api_key.is_empty() {
        return Err(Error::Api("API key is required".to_string()));
    }

    let model = normalize_model(Some(settings.openai_model));
    let timeout_ms = settings.ai_timeout_ms;
    let api_base_url = normalize_endpoint(Some(settings.ai_api_base_url));
    let operation = request.operation.trim().to_lowercase();
    let system_prompt = build_system_prompt(
        &operation,
        request.target_language.as_deref(),
        request.polish_style.as_deref(),
    )?;

    let payload = json!({
        "model": model,
        "temperature": 0,
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": request.text }
        ]
    });

    let client = tauri_plugin_http::reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(timeout_ms))
        .build()
        .map_err(|e| Error::Api(format!("Failed to build HTTP client: {}", e)))?;
    let response = client
        .post(&api_base_url)
        .header("Content-Type", "application/json")
        .bearer_auth(&api_key)
        .body(payload.to_string())
        .send()
        .await
        .map_err(|e| Error::Api(format!("HTTP request failed: {}", e)))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| Error::Api(format!("Failed to read HTTP response body: {}", e)))?;

    if !status.is_success() {
        return Err(Error::Api(format!(
            "AI request failed with status {}: {}",
            status, body
        )));
    }
    let value: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| Error::Api(format!("Invalid AI response JSON: {}", e)))?;

    if let Some(err) = value.get("error") {
        return Err(Error::Api(format!("AI returned error: {}", err)));
    }

    let content = extract_content(&value);
    if content.trim().is_empty() {
        return Err(Error::Api("AI returned empty content".to_string()));
    }

    if operation == "grammar" {
        let json_text = extract_json_object(&content);
        let parsed: serde_json::Value = serde_json::from_str(&json_text).map_err(|e| {
            Error::Api(format!(
                "Grammar response is not valid JSON typos format: {}",
                e
            ))
        })?;
        let typos = parsed
            .get("typos")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([]));
        let typos: Vec<AiTypo> = serde_json::from_value(typos)
            .map_err(|e| Error::Api(format!("Invalid typos payload format: {}", e)))?;
        return Ok(AiTextTransformResponse {
            output_text: None,
            typos,
        });
    }

    Ok(AiTextTransformResponse {
        output_text: Some(content),
        typos: Vec::new(),
    })
}
