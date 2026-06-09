use super::config::load_app_settings;
use super::errors::Error;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::Emitter;

// ── AI Result Cache ──────────────────────────────────────────────────────────

const CACHE_TTL: Duration = Duration::from_secs(300); // 5 minutes
const CACHE_MAX_ENTRIES: usize = 256;

/// In-memory LRU-ish cache for AI command results so repeated identical
/// requests (same text + operation + model + endpoint) are served instantly.
static AI_CACHE: std::sync::OnceLock<Mutex<HashMap<String, (serde_json::Value, Instant)>>> =
    std::sync::OnceLock::new();

fn ai_cache() -> &'static Mutex<HashMap<String, (serde_json::Value, Instant)>> {
    AI_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn build_cache_key(operation: &str, text: &str, model: &str, api_base: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    operation.hash(&mut hasher);
    text.hash(&mut hasher);
    model.hash(&mut hasher);
    api_base.hash(&mut hasher);
    format!("{}-{}", operation, hasher.finish())
}

/// Retrieve a cached result if it exists and hasn't expired.
fn get_cached<T: serde::de::DeserializeOwned>(key: &str) -> Option<T> {
    let mut cache = ai_cache().lock().ok()?;
    if let Some((value, timestamp)) = cache.get(key) {
        if timestamp.elapsed() < CACHE_TTL {
            return serde_json::from_value(value.clone()).ok();
        }
        // Expired — remove it.
        cache.remove(key);
    }
    None
}

/// Store a result in the cache, evicting the oldest entry if at capacity.
fn set_cache<T: serde::Serialize>(key: &str, value: &T) {
    let Ok(mut cache) = ai_cache().lock() else { return };
    let Ok(json_value) = serde_json::to_value(value) else { return };

    if cache.len() >= CACHE_MAX_ENTRIES {
        let oldest = cache
            .iter()
            .min_by_key(|(_, (_, t))| *t)
            .map(|(k, _)| k.clone());
        if let Some(k) = oldest {
            cache.remove(&k);
        }
    }
    cache.insert(key.to_string(), (json_value, Instant::now()));
}

// Predefined polish styles
pub const POLISH_STYLES: &[&str] = &["formal", "conversational", "academic", "business"];

// Style descriptions for prompts
fn get_style_description(style: &str) -> &'static str {
    match style {
        "formal" => "formal and professional",
        "conversational" => "conversational and friendly",
        "academic" => "academic and scholarly",
        "business" => "business and corporate",
        _ => "professional",
    }
}

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
pub struct AiToneDetectRequest {
    pub text: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AiToneDetectResponse {
    pub overall: String,
    pub score: f32,
    pub tones: Vec<AiTone>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AiTone {
    pub name: String,
    pub score: f32,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AiClarityCheckRequest {
    pub text: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AiClarityCheckResponse {
    pub score: f32,
    pub issues: Vec<AiClarityIssue>,
    pub stats: AiClarityStats,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AiClarityIssue {
    pub issue_type: String,
    pub text: String,
    pub suggestion: String,
    pub line: usize,
    pub col: usize,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AiClarityStats {
    pub readability_grade: String,
    pub avg_sentence_length: f32,
    pub passive_voice_count: usize,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AiVocabularyEnhanceRequest {
    pub text: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AiVocabularyEnhanceResponse {
    pub suggestions: Vec<AiVocabSuggestion>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AiVocabSuggestion {
    pub original: String,
    pub line: usize,
    pub col: usize,
    pub alternatives: Vec<AiVocabAlternative>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AiVocabAlternative {
    pub word: String,
    pub reason: String,
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
pub struct AiAssistRequest {
    pub text: String,
    pub action: String,
    pub target_language: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AiAssistResponse {
    pub headline: String,
    pub summary: String,
    pub primary_text: String,
    pub alternatives: Vec<AiAssistAlternative>,
    pub focus: Vec<String>,
    pub target_language: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AiAssistAlternative {
    pub label: String,
    pub text: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AiPolishedResult {
    pub style: String,
    pub output_text: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AiPolishBatchRequest {
    pub text: String,
    pub styles: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AiPolishBatchResponse {
    pub results: Vec<AiPolishedResult>,
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

fn translation_char_limit() -> usize {
    4000
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

/// Call AI for structured grammar issues and return a list of `AiTypo`.
/// This is the preferred way to get grammar feedback inside the spell-check
/// flow because it preserves original text and returns per-issue positions
/// instead of rewriting the whole document.
pub async fn check_grammar_issues_with_ai(
    api_base_url: &str,
    api_key: &str,
    model: &str,
    text: &str,
    timeout_ms: u64,
) -> Result<Vec<AiTypo>, Error> {
    let cache_key = build_cache_key("grammar", text, model, api_base_url);
    if let Some(cached) = get_cached::<Vec<AiTypo>>(&cache_key) {
        log::info!("[AI_CACHE] grammar hit for key={}", &cache_key[..cache_key.len().min(20)]);
        return Ok(cached);
    }

    let system_prompt = build_system_prompt("grammar", None, None)?;

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
            "AI grammar request failed with status {}: {}",
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

    set_cache(&cache_key, &typos);
    Ok(typos)
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

fn build_tone_detect_prompt() -> String {
    "You are a tone analyzer. Analyze the text and identify tone.
Return ONLY compact JSON in this exact schema: {\"overall\":\"string\",\"score\":85,\"tones\":[{\"name\":\"string\",\"score\":85}] }.
Tone options: formal, informal, friendly, serious, professional, confident, academic, business.
Scores are 0-100. `overall` must be the highest-confidence tone.
No markdown, no explanations, no code block."
        .to_string()
}

fn build_clarity_check_prompt() -> String {
    "You are a clarity editor. Identify redundancy, complexity, passive voice, and vague expressions.
Return ONLY compact JSON in this exact schema: {\"score\":85,\"issues\":[{\"issueType\":\"redundancy|complexity|passive|vague\",\"text\":\"string\",\"suggestion\":\"string\",\"line\":1,\"col\":1}],\"stats\":{\"readabilityGrade\":\"Grade 8\",\"avgSentenceLength\":15.5,\"passiveVoiceCount\":2}}.
If there are no issues, return an empty issues array and keep stats valid.
No markdown, no explanations, no code block."
        .to_string()
}

fn build_vocabulary_prompt() -> String {
    "You are a vocabulary enhancer. Suggest better alternatives for weak or repetitive words.
Return ONLY compact JSON in this exact schema: {\"suggestions\":[{\"original\":\"string\",\"line\":1,\"col\":1,\"alternatives\":[{\"word\":\"string\",\"reason\":\"more precise|more formal|less repetitive|stronger|clearer\"}]}]}.
If there are no useful suggestions, return {\"suggestions\":[]}.
No markdown, no explanations, no code block."
        .to_string()
}

fn build_assist_prompt(action: &str, target_language: Option<&str>) -> Result<String, Error> {
    let schema = "{\"headline\":\"string\",\"summary\":\"string\",\"primaryText\":\"string\",\"alternatives\":[{\"label\":\"string\",\"text\":\"string\"}],\"focus\":[\"string\"],\"targetLanguage\":\"string|null\"}";

    match action {
        "translate" => {
            let lang = target_language.unwrap_or("English").trim();
            Ok(format!(
                "You are Grammarly-style translation assistance embedded inside a writing flow. Detect the source language automatically and translate the user's text into {lang}. Preserve meaning, register, intent, and natural voice. Return ONLY compact JSON in this exact schema: {schema}. Rules: 1) `headline` is 2-5 words describing the result; 2) `summary` is one short sentence about what improved; 3) `primaryText` is the best translation ready to insert; 4) include 1-2 alternatives when helpful, especially for short phrases where nuance matters; 5) `focus` should contain 2-4 short tags like accuracy, natural, concise, formal, friendly, clear; 6) set `targetLanguage` to \"{lang}\"; 7) no markdown, no prose outside JSON."
            ))
        }
        "rewrite" => Ok(format!(
            "You are Grammarly-style rewriting assistance. Rewrite the user's text so it is clearer, smoother, and more effective while preserving meaning, intent, and voice. Return ONLY compact JSON in this exact schema: {schema}. Rules: 1) `headline` is 2-5 words; 2) `summary` briefly explains the improvement; 3) `primaryText` is the best meaning-preserving rewrite; 4) provide 1-2 alternatives with distinct labels like \"More direct\" or \"More natural\"; 5) `focus` should contain 2-4 short tags like clarity, flow, concise, professional, natural; 6) `targetLanguage` must be null; 7) no markdown, no prose outside JSON."
        )),
        "paraphrase" => Ok(format!(
            "You are Grammarly-style paraphrasing assistance. Rephrase the user's text in fresh wording while keeping the original meaning intact and sounding natural. Return ONLY compact JSON in this exact schema: {schema}. Rules: 1) `headline` is 2-5 words; 2) `summary` briefly explains the change; 3) `primaryText` is the strongest paraphrase; 4) provide 1-2 alternatives with noticeably different wording while preserving meaning; 5) `focus` should contain 2-4 short tags like natural, expressive, concise, professional, engaging; 6) `targetLanguage` must be null; 7) no markdown, no prose outside JSON."
        )),
        "concise" => Ok(format!(
            "You are Grammarly-style revision assistance. Rewrite the user's text to be more concise and easier to scan while preserving meaning and essential detail. Return ONLY compact JSON in this exact schema: {schema}. Rules: 1) `headline` is 2-5 words; 2) `summary` briefly explains the improvement; 3) `primaryText` is the best concise rewrite; 4) provide 1-2 alternatives with different levels of brevity when useful; 5) `focus` should contain 2-4 short tags like concise, clear, direct, readable; 6) `targetLanguage` must be null; 7) no markdown, no prose outside JSON."
        )),
        _ => Err(Error::Api(
            "Unsupported assist action, expected translate|rewrite|paraphrase|concise"
                .to_string(),
        )),
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
        let typos = check_grammar_issues_with_ai(
            &api_base_url,
            &api_key,
            &model,
            &request.text,
            timeout_ms,
        )
        .await?;
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

#[tauri::command]
pub async fn ai_tone_detect(
    app: tauri::AppHandle,
    request: AiToneDetectRequest,
) -> Result<AiToneDetectResponse, Error> {
    let settings = load_app_settings(&app)?;
    let api_key = settings.openai_api_key.trim().to_string();
    if api_key.is_empty() {
        return Err(Error::Api("API key is required".to_string()));
    }

    let model = normalize_model(Some(settings.openai_model));
    let timeout_ms = settings.ai_timeout_ms;
    let api_base_url = normalize_endpoint(Some(settings.ai_api_base_url));

    let cache_key = build_cache_key("tone", &request.text, &model, &api_base_url);
    if let Some(cached) = get_cached::<AiToneDetectResponse>(&cache_key) {
        log::info!("[AI_CACHE] tone hit");
        return Ok(cached);
    }

    let payload = json!({
        "model": model,
        "temperature": 0,
        "messages": [
            { "role": "system", "content": build_tone_detect_prompt() },
            { "role": "user", "content": request.text }
        ]
    });

    let client = tauri_plugin_http::reqwest::Client::builder()
        .timeout(Duration::from_millis(timeout_ms))
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

    let json_text = extract_json_object(&content);
    let result = serde_json::from_str::<AiToneDetectResponse>(&json_text)
        .map_err(|e| Error::Api(format!("Failed to parse tone detection JSON: {}", e)))?;
    set_cache(&cache_key, &result);
    Ok(result)
}

#[tauri::command]
pub async fn ai_clarity_check(
    app: tauri::AppHandle,
    request: AiClarityCheckRequest,
) -> Result<AiClarityCheckResponse, Error> {
    let settings = load_app_settings(&app)?;
    let api_key = settings.openai_api_key.trim().to_string();
    if api_key.is_empty() {
        return Err(Error::Api("API key is required".to_string()));
    }

    let model = normalize_model(Some(settings.openai_model));
    let timeout_ms = settings.ai_timeout_ms;
    let api_base_url = normalize_endpoint(Some(settings.ai_api_base_url));

    let cache_key = build_cache_key("clarity", &request.text, &model, &api_base_url);
    if let Some(cached) = get_cached::<AiClarityCheckResponse>(&cache_key) {
        log::info!("[AI_CACHE] clarity hit");
        return Ok(cached);
    }

    let payload = json!({
        "model": model,
        "temperature": 0,
        "messages": [
            { "role": "system", "content": build_clarity_check_prompt() },
            { "role": "user", "content": request.text }
        ]
    });

    let client = tauri_plugin_http::reqwest::Client::builder()
        .timeout(Duration::from_millis(timeout_ms))
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

    let json_text = extract_json_object(&content);
    let result = serde_json::from_str::<AiClarityCheckResponse>(&json_text)
        .map_err(|e| Error::Api(format!("Failed to parse clarity check JSON: {}", e)))?;
    set_cache(&cache_key, &result);
    Ok(result)
}

#[tauri::command]
pub async fn ai_vocabulary_enhance(
    app: tauri::AppHandle,
    request: AiVocabularyEnhanceRequest,
) -> Result<AiVocabularyEnhanceResponse, Error> {
    let settings = load_app_settings(&app)?;
    let api_key = settings.openai_api_key.trim().to_string();
    if api_key.is_empty() {
        return Err(Error::Api("API key is required".to_string()));
    }

    let model = normalize_model(Some(settings.openai_model));
    let timeout_ms = settings.ai_timeout_ms;
    let api_base_url = normalize_endpoint(Some(settings.ai_api_base_url));

    let cache_key = build_cache_key("vocabulary", &request.text, &model, &api_base_url);
    if let Some(cached) = get_cached::<AiVocabularyEnhanceResponse>(&cache_key) {
        log::info!("[AI_CACHE] vocabulary hit");
        return Ok(cached);
    }

    let payload = json!({
        "model": model,
        "temperature": 0,
        "messages": [
            { "role": "system", "content": build_vocabulary_prompt() },
            { "role": "user", "content": request.text }
        ]
    });

    let client = tauri_plugin_http::reqwest::Client::builder()
        .timeout(Duration::from_millis(timeout_ms))
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

    let json_text = extract_json_object(&content);
    let result = serde_json::from_str::<AiVocabularyEnhanceResponse>(&json_text)
        .map_err(|e| Error::Api(format!("Failed to parse vocabulary JSON: {}", e)))?;
    set_cache(&cache_key, &result);
    Ok(result)
}

#[tauri::command]
pub async fn ai_assist(
    app: tauri::AppHandle,
    request: AiAssistRequest,
) -> Result<AiAssistResponse, Error> {
    let settings = load_app_settings(&app)?;
    let api_key = settings.openai_api_key.trim().to_string();
    if api_key.is_empty() {
        return Err(Error::Api("API key is required".to_string()));
    }

    let action = request.action.trim().to_lowercase();
    if action == "translate" && request.text.chars().count() > translation_char_limit() {
        return Err(Error::Api(format!(
            "Translation is limited to {} characters per request",
            translation_char_limit()
        )));
    }

    let model = normalize_model(Some(settings.openai_model));
    let timeout_ms = settings.ai_timeout_ms;
    let api_base_url = normalize_endpoint(Some(settings.ai_api_base_url));
    let system_prompt = build_assist_prompt(&action, request.target_language.as_deref())?;

    let payload = json!({
        "model": model,
        "temperature": 0.35,
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": request.text }
        ]
    });

    let client = tauri_plugin_http::reqwest::Client::builder()
        .timeout(Duration::from_millis(timeout_ms))
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

    let json_text = extract_json_object(&content);
    let mut parsed = serde_json::from_str::<AiAssistResponse>(&json_text)
        .map_err(|e| Error::Api(format!("Failed to parse AI assist JSON: {}", e)))?;

    parsed.headline = parsed.headline.trim().to_string();
    parsed.summary = parsed.summary.trim().to_string();
    parsed.primary_text = parsed.primary_text.trim().to_string();
    parsed
        .alternatives
        .retain(|item| !item.text.trim().is_empty());
    for item in &mut parsed.alternatives {
        item.label = item.label.trim().to_string();
        item.text = item.text.trim().to_string();
    }
    parsed.focus = parsed
        .focus
        .into_iter()
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect();

    if parsed.primary_text.is_empty() {
        return Err(Error::Api(
            "AI assist response is missing primaryText".to_string(),
        ));
    }

    Ok(parsed)
}

#[tauri::command]
pub async fn ai_clarity_check_stream(
    app: tauri::AppHandle,
    request: AiClarityCheckRequest,
) -> Result<(), Error> {
    let settings = load_app_settings(&app)?;
    let api_key = settings.openai_api_key.trim().to_string();
    if api_key.is_empty() {
        return Err(Error::Api("API key is required".to_string()));
    }

    let model = normalize_model(Some(settings.openai_model));
    let timeout_ms = settings.ai_timeout_ms.max(120_000);
    let api_base_url = normalize_endpoint(Some(settings.ai_api_base_url));

    let payload = json!({
        "model": model,
        "temperature": 0,
        "stream": true,
        "reasoning": { "exclude": true },
        "messages": [
            { "role": "system", "content": build_clarity_check_prompt() },
            { "role": "user", "content": request.text }
        ]
    });

    let client = tauri_plugin_http::reqwest::Client::builder()
        .timeout(Duration::from_millis(timeout_ms))
        .build()
        .map_err(|e| Error::Api(format!("Failed to build HTTP client: {}", e)))?;

    let mut response = client
        .post(&api_base_url)
        .header("Content-Type", "application/json")
        .bearer_auth(api_key)
        .body(payload.to_string())
        .send()
        .await
        .map_err(|e| Error::Api(format!("HTTP request failed: {}", e)))?;

    let status = response.status();
    if !status.is_success() {
        let body = response
            .text()
            .await
            .map_err(|e| Error::Api(format!("Failed to read HTTP response body: {}", e)))?;
        return Err(Error::Api(format!(
            "AI request failed with status {}: {}",
            status, body
        )));
    }

    let mut pending = String::new();
    loop {
        match response.chunk().await {
            Ok(Some(bytes)) => {
                pending.push_str(&String::from_utf8_lossy(&bytes));
                while let Some(newline_pos) = pending.find('\n') {
                    let line = pending[..newline_pos].trim_end_matches('\r').to_string();
                    pending.drain(..=newline_pos);

                    if !line.starts_with("data:") {
                        continue;
                    }

                    let data = line.trim_start_matches("data:").trim_start();
                    if data == "[DONE]" {
                        let _ = app.emit("ai-clarity-complete", ());
                        return Ok(());
                    }

                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(data) {
                        let choices = value.get("choices").and_then(|c| c.as_array());
                        if let Some(first_choice) = choices.and_then(|choices| choices.first()) {
                            for content in collect_stream_content(first_choice) {
                                let _ = app.emit("ai-clarity-chunk", content);
                            }
                        }
                    }
                }
            }
            Ok(None) => {
                let _ = app.emit("ai-clarity-complete", ());
                break;
            }
            Err(e) => {
                let message = format!("Clarity stream error: {}", e);
                let _ = app.emit("ai-clarity-error", message.clone());
                return Err(Error::Api(message));
            }
        }
    }

    Ok(())
}

fn collect_stream_content(choice: &serde_json::Value) -> Vec<String> {
    fn push_content_value(content: &serde_json::Value, out: &mut Vec<String>) {
        match content {
            serde_json::Value::String(s) => {
                if !s.is_empty() {
                    out.push(s.clone());
                }
            }
            serde_json::Value::Array(items) => {
                for item in items {
                    if let Some(text) = item.get("text").and_then(|v| v.as_str()) {
                        if !text.is_empty() {
                            out.push(text.to_string());
                        }
                        continue;
                    }
                    if let Some(text) = item.get("content").and_then(|v| v.as_str()) {
                        if !text.is_empty() {
                            out.push(text.to_string());
                        }
                    }
                }
            }
            serde_json::Value::Object(map) => {
                if let Some(text) = map.get("text").and_then(|v| v.as_str()) {
                    if !text.is_empty() {
                        out.push(text.to_string());
                    }
                }
            }
            _ => {}
        }
    }

    let mut out = Vec::new();
    if let Some(delta) = choice.get("delta") {
        if let Some(content) = delta.get("content") {
            push_content_value(content, &mut out);
        }
    }
    if let Some(message) = choice.get("message") {
        if let Some(content) = message.get("content") {
            push_content_value(content, &mut out);
        }
    }
    if let Some(text) = choice.get("text") {
        push_content_value(text, &mut out);
    }
    out
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
        "reasoning": { "exclude": true },
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
        request_id,
        status,
        content_type,
        transfer_encoding
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
    let mut reasoning_only_events: usize = 0;
    let mut parse_error_events: usize = 0;
    let mut sample_non_content_logs: usize = 0;
    let mut raw_sse_log_count: usize = 0;
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

                    if line.starts_with("data:") {
                        let data = line.trim_start_matches("data:").trim_start();
                        if raw_sse_log_count < 40 || raw_sse_log_count.is_multiple_of(200) {
                            log::info!("[AI_STREAM][{}] raw_sse_data={}", request_id, data);
                        }
                        raw_sse_log_count += 1;
                        if data == "[DONE]" {
                            continue;
                        }

                        if let Ok(value) = serde_json::from_str::<serde_json::Value>(data) {
                            if let Some(choices) = value.get("choices").and_then(|c| c.as_array()) {
                                if let Some(first_choice) = choices.first() {
                                    let mut has_finish_reason = false;
                                    if let Some(reason) =
                                        first_choice.get("finish_reason").and_then(|r| r.as_str())
                                    {
                                        if !reason.is_empty() && reason != "null" {
                                            has_finish_reason = true;
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
                                    let contents = collect_stream_content(first_choice);
                                    if contents.is_empty() {
                                        if !has_finish_reason {
                                            non_content_events += 1;
                                            let has_reasoning = first_choice
                                                .get("delta")
                                                .and_then(|d| d.get("reasoning"))
                                                .and_then(|r| r.as_str())
                                                .map(|s| !s.is_empty())
                                                .unwrap_or(false);
                                            if has_reasoning {
                                                reasoning_only_events += 1;
                                            }
                                            if sample_non_content_logs < 3 {
                                                log::warn!(
                                                    "[AI_STREAM][{}] non-content event sample={} payload_prefix={}",
                                                    request_id,
                                                    sample_non_content_logs + 1,
                                                    data.chars().take(400).collect::<String>()
                                                );
                                                sample_non_content_logs += 1;
                                            }
                                        }
                                    } else {
                                        for content in contents {
                                            emitted_chars += content.chars().count();
                                            let _ = app.emit("ai-stream-chunk", content);
                                        }
                                    }
                                }
                            }
                        } else {
                            parse_error_events += 1;
                            if parse_error_events <= 3 {
                                log::warn!(
                                    "[AI_STREAM][{}] stream json parse error sample={} data_prefix={}",
                                    request_id,
                                    parse_error_events,
                                    data.chars().take(300).collect::<String>()
                                );
                            }
                        }
                    }
                }

                if stream_finished {
                    break;
                }

                if emitted_chars == 0
                    && reasoning_only_events >= 40
                    && started_at.elapsed() > Duration::from_secs(3)
                {
                    log::warn!(
                        "[AI_STREAM][{}] reasoning-only stream detected chunk_count={} reasoning_only_events={} total_bytes={} elapsed_ms={}",
                        request_id,
                        chunk_count,
                        reasoning_only_events,
                        total_bytes,
                        started_at.elapsed().as_millis()
                    );
                }

                if emitted_chars == 0
                    && chunk_count >= 120
                    && started_at.elapsed() > Duration::from_secs(12)
                {
                    log::warn!(
                        "[AI_STREAM][{}] stream stalled without content chunk_count={} total_bytes={} non_content_events={} reasoning_only_events={} parse_error_events={} elapsed_ms={}",
                        request_id,
                        chunk_count,
                        total_bytes,
                        non_content_events,
                        reasoning_only_events,
                        parse_error_events,
                        started_at.elapsed().as_millis()
                    );
                }

                // Guardrail: if stream runs too long or too chatty with too little useful text, fail fast.
                if started_at.elapsed() > Duration::from_secs(35)
                    || chunk_count > 1500
                    || (total_bytes > 400_000 && emitted_chars < 120)
                {
                    let message = format!(
                        "Stream guard triggered (elapsed_ms={} chunk_count={} total_bytes={} emitted_chars={} non_content_events={} parse_error_events={})",
                        started_at.elapsed().as_millis(),
                        chunk_count,
                        total_bytes,
                        emitted_chars,
                        non_content_events,
                        parse_error_events
                    );
                    log::error!("[AI_STREAM][{}] {}", request_id, message);
                    let _ = app.emit("ai-stream-error", message.clone());
                    return Err(Error::Api(message));
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
                let message = format!(
                    "Stream error: {} (chunk_count={} total_bytes={} emitted_chars={} pending_len={} elapsed_ms={})",
                    e,
                    chunk_count,
                    total_bytes,
                    emitted_chars,
                    pending.len(),
                    started_at.elapsed().as_millis(),
                );
                let _ = app.emit("ai-stream-error", message.clone());
                return Err(Error::Api(message));
            }
        }
    }

    if emitted_chars == 0 {
        if operation == "grammar" {
            log::info!(
                "[AI_STREAM][{}] grammar stream completed with no issues (zero content): chunk_count={} total_bytes={} non_content_events={} reasoning_only_events={} parse_error_events={}",
                request_id,
                chunk_count,
                total_bytes,
                non_content_events,
                reasoning_only_events,
                parse_error_events
            );
            let _ = app.emit("ai-stream-complete", ());
            return Ok(());
        }
        let message = format!(
            "Stream ended without content (chunk_count={} total_bytes={} non_content_events={} reasoning_only_events={} parse_error_events={})",
            chunk_count, total_bytes, non_content_events, reasoning_only_events, parse_error_events
        );
        log::error!("[AI_STREAM][{}] {}", request_id, message);
        let _ = app.emit("ai-stream-error", message.clone());
        return Err(Error::Api(message));
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

/// Call AI API and return polished text for a single style.
async fn polish_text_with_style(
    api_base_url: &str,
    api_key: &str,
    model: &str,
    text: &str,
    style: &str,
    timeout_ms: u64,
) -> Result<String, Error> {
    let style_desc = get_style_description(style);
    let system_prompt = format!(
        "You are an expert copy editor. Polish the text in a {} style. Return polished text only. No markdown, no explanations, no prefixes.",
        style_desc
    );

    let payload = json!({
        "model": model,
        "temperature": 0,
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": text }
        ]
    });

    let client = tauri_plugin_http::reqwest::Client::builder()
        .timeout(Duration::from_millis(timeout_ms))
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

    Ok(content)
}

#[tauri::command]
pub async fn ai_polish_batch(
    app: tauri::AppHandle,
    request: AiPolishBatchRequest,
) -> Result<AiPolishBatchResponse, Error> {
    let settings = load_app_settings(&app)?;
    let api_key = settings.openai_api_key.trim().to_string();
    if api_key.is_empty() {
        return Err(Error::Api("API key is required".to_string()));
    }

    let model = normalize_model(Some(settings.openai_model));
    let timeout_ms = settings.ai_timeout_ms;
    let api_base_url = normalize_endpoint(Some(settings.ai_api_base_url));

    // Process each style in parallel using tokio::stream
    let mut handles = Vec::new();

    for style in &request.styles {
        let text = request.text.clone();
        let api_base_url = api_base_url.clone();
        let api_key = api_key.clone();
        let model = model.clone();
        let style = style.clone();

        handles.push(tokio::spawn(async move {
            polish_text_with_style(&api_base_url, &api_key, &model, &text, &style, timeout_ms).await
        }));
    }

    let mut results = Vec::new();
    for (style, handle) in request.styles.iter().zip(handles) {
        match handle.await {
            Ok(Ok(output_text)) => {
                results.push(AiPolishedResult {
                    style: style.clone(),
                    output_text,
                });
            }
            Ok(Err(e)) => {
                log::error!("Polish style '{}' failed: {}", style, e);
                // Add empty result for failed style
                results.push(AiPolishedResult {
                    style: style.clone(),
                    output_text: String::new(),
                });
            }
            Err(e) => {
                log::error!("Polish style '{}' task failed: {}", style, e);
                results.push(AiPolishedResult {
                    style: style.clone(),
                    output_text: String::new(),
                });
            }
        }
    }

    Ok(AiPolishBatchResponse { results })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_object_trims_fence() {
        let wrapped = "```json\n{\"a\":1}\n```";
        assert_eq!(extract_json_object(wrapped), "{\"a\":1}");

        let wrapped_plain = "```\n{\"b\":2}\n```";
        assert_eq!(extract_json_object(wrapped_plain), "{\"b\":2}");
    }

    #[test]
    fn test_normalize_endpoint_and_model() {
        assert_eq!(
            normalize_endpoint(Some("https://openrouter.ai/api/v1".to_string())),
            "https://openrouter.ai/api/v1/chat/completions"
        );
        assert_eq!(
            normalize_model(Some("   ".to_string())),
            "openai/gpt-5-nano"
        );
    }

    #[test]
    fn test_deserialize_tone_detect_response() {
        let payload = r#"{
            "overall":"professional",
            "score":91,
            "tones":[{"name":"professional","score":91},{"name":"friendly","score":35}]
        }"#;
        let parsed: AiToneDetectResponse = serde_json::from_str(payload).unwrap();
        assert_eq!(parsed.overall, "professional");
        assert_eq!(parsed.tones.len(), 2);
        assert_eq!(parsed.tones[0].name, "professional");
    }

    #[test]
    fn test_deserialize_clarity_response_camel_case() {
        let payload = r#"{
            "score": 84,
            "issues": [
                {"issueType":"redundancy","text":"非常非常好","suggestion":"非常好","line":1,"col":1}
            ],
            "stats": {"readabilityGrade":"Grade 8","avgSentenceLength":12.5,"passiveVoiceCount":1}
        }"#;
        let parsed: AiClarityCheckResponse = serde_json::from_str(payload).unwrap();
        assert_eq!(parsed.issues[0].issue_type, "redundancy");
        assert_eq!(parsed.stats.readability_grade, "Grade 8");
        assert_eq!(parsed.stats.passive_voice_count, 1);
    }

    #[test]
    fn test_deserialize_vocabulary_response() {
        let payload = r#"{
            "suggestions": [
                {
                    "original":"非常好",
                    "line":1,
                    "col":1,
                    "alternatives":[{"word":"出色","reason":"stronger"}]
                }
            ]
        }"#;
        let parsed: AiVocabularyEnhanceResponse = serde_json::from_str(payload).unwrap();
        assert_eq!(parsed.suggestions.len(), 1);
        assert_eq!(parsed.suggestions[0].alternatives[0].word, "出色");
    }

    #[test]
    fn test_collect_stream_content_from_delta_and_text() {
        let choice = serde_json::json!({
            "delta": { "content": [{"text":"你"}, {"text":"好"}] },
            "text": "!"
        });
        let out = collect_stream_content(&choice);
        assert_eq!(
            out,
            vec!["你".to_string(), "好".to_string(), "!".to_string()]
        );
    }
}
