use super::config::load_app_settings;
use super::errors::Error;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;
use tauri::Emitter;

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

async fn run_stream_fallback(
    app: &tauri::AppHandle,
    fallback_request: &AiTextTransformRequest,
    request_id: u128,
    reason: &str,
    elapsed_ms: u128,
) -> Result<(), Error> {
    log::warn!(
        "[AI_STREAM][{}] {} ; trying non-stream fallback",
        request_id,
        reason
    );
    match ai_text_transform(app.clone(), fallback_request.clone()).await {
        Ok(resp) => {
            if let Some(text) = resp.output_text {
                let output_chars = text.chars().count();
                if !text.is_empty() {
                    let _ = app.emit("ai-stream-chunk", &text);
                }
                let _ = app.emit("ai-stream-complete", ());
                log::info!(
                    "[AI_STREAM][{}] fallback succeeded output_chars={} total_elapsed_ms={}",
                    request_id,
                    output_chars,
                    elapsed_ms
                );
                Ok(())
            } else {
                let message = "Stream fallback returned no text output".to_string();
                log::error!(
                    "[AI_STREAM][{}] fallback returned empty output total_elapsed_ms={}",
                    request_id,
                    elapsed_ms
                );
                let _ = app.emit("ai-stream-error", message.clone());
                Err(Error::Api(message))
            }
        }
        Err(fallback_error) => {
            let message = format!("{}; fallback error: {}", reason, fallback_error);
            log::error!(
                "[AI_STREAM][{}] fallback failed reason={} fallback_error={} total_elapsed_ms={}",
                request_id,
                reason,
                fallback_error,
                elapsed_ms
            );
            let _ = app.emit("ai-stream-error", message.clone());
            Err(Error::Api(message))
        }
    }
}

#[tauri::command]
pub async fn ai_text_transform_stream(
    app: tauri::AppHandle,
    request: AiTextTransformRequest,
) -> Result<(), Error> {
    let request_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let started_at = std::time::Instant::now();
    let fallback_request = request.clone();
    let settings = load_app_settings(&app)?;
    let api_key = settings.openai_api_key.trim().to_string();
    if api_key.is_empty() {
        return Err(Error::Api("API key is required".to_string()));
    }

    let model = normalize_model(Some(settings.openai_model));
    let timeout_ms = settings.ai_timeout_ms;
    // Streaming responses are long-lived; keep a larger ceiling than regular calls.
    let stream_timeout_ms = timeout_ms.max(120_000);
    let api_base_url = normalize_endpoint(Some(settings.ai_api_base_url));
    let operation = request.operation.trim().to_lowercase();
    let system_prompt = build_system_prompt(
        &operation,
        request.target_language.as_deref(),
        request.polish_style.as_deref(),
    )?;

    log::info!(
        "[AI_STREAM][{}] start operation={} text_len={} model={} endpoint={} timeout_ms={} target_lang={:?} polish_style={:?}",
        request_id,
        operation,
        request.text.chars().count(),
        model,
        api_base_url,
        stream_timeout_ms,
        request.target_language,
        request.polish_style
    );

    let payload = json!({
        "model": model,
        "temperature": 0,
        "stream": true,
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": request.text }
        ]
    });

    let client = tauri_plugin_http::reqwest::Client::builder()
        .timeout(Duration::from_millis(stream_timeout_ms))
        .build()
        .map_err(|e| Error::Api(format!("Failed to build HTTP client: {}", e)))?;
    
    let mut response = client
        .post(api_base_url)
        .header("Content-Type", "application/json")
        .bearer_auth(api_key)
        .body(payload.to_string())
        .send()
        .await
        .map_err(|e| Error::Api(format!("HTTP request failed: {}", e)))?;
    
    let status = response.status();
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("<unknown>")
        .to_string();
    let transfer_encoding = response
        .headers()
        .get("transfer-encoding")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("<unknown>")
        .to_string();
    log::info!(
        "[AI_STREAM][{}] response status={} content-type={} transfer-encoding={}",
        request_id, status, content_type, transfer_encoding
    );
    if !status.is_success() {
        let body = response
            .text()
            .await
            .map_err(|e| Error::Api(format!("Failed to read HTTP response body: {}", e)))?;
        log::error!(
            "[AI_STREAM][{}] non-success status={} body_prefix={}",
            request_id,
            status,
            body.chars().take(500).collect::<String>()
        );
        return Err(Error::Api(format!(
            "AI request failed with status {}: {}",
            status, body
        )));
    }

    let mut pending = String::new();
    let mut chunk_count: usize = 0;
    let mut total_bytes: usize = 0;
    let mut emitted_chars: usize = 0;
    let mut non_content_events: usize = 0;
    let mut stream_finished = false;
    
    loop {
        match response.chunk().await {
            Ok(Some(bytes)) => {
                chunk_count += 1;
                total_bytes += bytes.len();
                pending.push_str(&String::from_utf8_lossy(&bytes));
                if chunk_count == 1 || chunk_count % 20 == 0 {
                    log::info!(
                        "[AI_STREAM][{}] chunk_count={} total_bytes={} pending_len={} emitted_chars={}",
                        request_id,
                        chunk_count,
                        total_bytes,
                        pending.len(),
                        emitted_chars
                    );
                }

                while let Some(newline_pos) = pending.find('\n') {
                    let line = pending[..newline_pos].trim_end_matches('\r').to_string();
                    pending.drain(..=newline_pos);

                    if line.starts_with("data: ") {
                        let data = &line[6..];
                        if data == "[DONE]" {
                            continue;
                        }
                        
                        if let Ok(value) = serde_json::from_str::<serde_json::Value>(data) {
                            if let Some(choices) = value.get("choices").and_then(|c| c.as_array()) {
                                if let Some(first_choice) = choices.first() {
                                    if let Some(reason) = first_choice
                                        .get("finish_reason")
                                        .and_then(|r| r.as_str())
                                    {
                                        if !reason.is_empty() && reason != "null" {
                                            log::info!(
                                                "[AI_STREAM][{}] finish_reason={} chunk_count={} emitted_chars={}",
                                                request_id,
                                                reason,
                                                chunk_count,
                                                emitted_chars
                                            );
                                            stream_finished = true;
                                        }
                                    }
                                    if let Some(delta) = first_choice.get("delta") {
                                        if let Some(content) = delta.get("content").and_then(|c| c.as_str()) {
                                            if !content.is_empty() {
                                                emitted_chars += content.chars().count();
                                                let _ = app.emit("ai-stream-chunk", content);
                                            } else {
                                                non_content_events += 1;
                                            }
                                        } else {
                                            non_content_events += 1;
                                        }
                                    } else {
                                        non_content_events += 1;
                                    }
                                }
                            }
                        }
                    }
                }

                if stream_finished {
                    break;
                }

                if emitted_chars == 0
                    && chunk_count >= 120
                    && started_at.elapsed() > Duration::from_secs(12)
                {
                    let reason = format!(
                        "Stream stalled with no content (chunk_count={} total_bytes={} non_content_events={})",
                        chunk_count, total_bytes, non_content_events
                    );
                    return run_stream_fallback(
                        &app,
                        &fallback_request,
                        request_id,
                        &reason,
                        started_at.elapsed().as_millis(),
                    )
                    .await;
                }

                // Guardrail: if stream runs too long or too chatty with too little useful text,
                // stop waiting and fallback to one-shot response.
                if started_at.elapsed() > Duration::from_secs(35)
                    || chunk_count > 1500
                    || (total_bytes > 400_000 && emitted_chars < 120)
                {
                    let reason = format!(
                        "Stream guard triggered (elapsed_ms={} chunk_count={} total_bytes={} emitted_chars={} non_content_events={})",
                        started_at.elapsed().as_millis(),
                        chunk_count,
                        total_bytes,
                        emitted_chars,
                        non_content_events
                    );
                    return run_stream_fallback(
                        &app,
                        &fallback_request,
                        request_id,
                        &reason,
                        started_at.elapsed().as_millis(),
                    )
                    .await;
                }
            }
            Ok(None) => {
                log::info!(
                    "[AI_STREAM][{}] stream ended chunk_count={} total_bytes={} emitted_chars={} elapsed_ms={}",
                    request_id,
                    chunk_count,
                    total_bytes,
                    emitted_chars,
                    started_at.elapsed().as_millis()
                );
                break;
            }
            Err(e) => {
                let reason = format!(
                    "Stream error: {} (chunk_count={} total_bytes={} emitted_chars={} pending_len={} elapsed_ms={})",
                    e,
                    chunk_count,
                    total_bytes,
                    emitted_chars,
                    pending.len(),
                    started_at.elapsed().as_millis(),
                );
                return run_stream_fallback(
                    &app,
                    &fallback_request,
                    request_id,
                    &reason,
                    started_at.elapsed().as_millis(),
                )
                .await;
            }
        }
    }

    let _ = app.emit("ai-stream-complete", ());
    log::info!(
        "[AI_STREAM][{}] complete emitted_chars={} total_elapsed_ms={}",
        request_id,
        emitted_chars,
        started_at.elapsed().as_millis()
    );
    
    Ok(())
}
