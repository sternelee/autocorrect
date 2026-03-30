# AI Enhancement Features Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement three AI-powered writing enhancement features (tone detection, clarity analysis, vocabulary enhancement) in the autocorrect-app desktop application.

**Architecture:**

- Backend: Three new Rust Tauri commands in `ai_grammar.rs` that call LLM APIs with structured JSON prompts
- Frontend: New Svelte components integrated into the existing AI popup system with a two-column layout (original text + analysis panel)
- Communication: Follow existing patterns for streaming (clarity) vs non-streaming (tone, vocabulary) responses

**Tech Stack:** Rust (Tauri), Svelte 5, TypeScript, Tailwind CSS, OpenAI API (via OpenRouter)

---

## File Structure

```
src-tauri/src/commands/ai_grammar.rs    # Add 3 new commands + structs
src-tauri/src/lib.rs                    # Register new commands in invoke_handler
src/lib/components/
  AiEnhancementPanel.svelte            # Main container with tab switching
  ToneDetectionView.svelte             # Tone analysis display
  ClarityCheckView.svelte              # Clarity issues list
  VocabularyEnhanceView.svelte         # Word suggestions
src/lib/types/
  ai.ts                                # TypeScript interfaces
src/routes/ai-popup/+page.svelte       # Modify to integrate new buttons
```

---

## Chunk 1: Rust Backend - Tone Detection Command

### Task 1.1: Add Request/Response Structs

**Files:**

- Modify: `src-tauri/src/commands/ai_grammar.rs`

- [ ] **Step 1: Add structs after existing AiGrammarResponse**

```rust
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
```

- [ ] **Step 2: Add helper function for tone detection prompt**

Add after `build_system_prompt` function:

```rust
fn build_tone_detect_prompt() -> String {
    "You are a tone analyzer. Analyze the text and identify the tone.
Return ONLY compact JSON in this exact schema: {\"overall\":\"string\",\"score\":85,\"tones\":[{\"name\":\"string\",\"score\":85},...]}.
Tone options: formal, informal, friendly, serious, professional, confident, academic, business.
Score is 0-100. Overall is the primary tone. Tones array includes all detected tones sorted by score.
No markdown, no explanations, no code blocks.".to_string()
}
```

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/commands/ai_grammar.rs
git commit -m "feat: add tone detection structs and prompt"
```

### Task 1.2: Implement ai_tone_detect Command

**Files:**

- Modify: `src-tauri/src/commands/ai_grammar.rs`

- [ ] **Step 1: Add the command function**

Add at end of file before tests:

```rust
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
    let system_prompt = build_tone_detect_prompt();

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

    let json_text = extract_json_object(&content);
    let parsed: AiToneDetectResponse = serde_json::from_str(&json_text)
        .map_err(|e| Error::Api(format!("Failed to parse tone detection response: {}", e)))?;

    Ok(parsed)
}
```

- [ ] **Step 2: Register command in lib.rs**

Modify: `src-tauri/src/lib.rs` around line 74

Find the imports and add `ai_tone_detect`:

```rust
use commands::ai_grammar::{ai_grammar_check, ai_polish_batch, ai_text_transform, ai_text_transform_stream, ai_tone_detect};
```

Then find the invoke_handler and add the command:

```rust
.invoke_handler(
    tauri::generate_handler![
        // ... existing commands ...
        ai_tone_detect,
        // ... rest of commands ...
    ]
)
```

- [ ] **Step 3: Run Rust tests**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -- ai_tone_detect
```

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands/ai_grammar.rs src-tauri/src/lib.rs
git commit -m "feat: implement ai_tone_detect command"
```

---

## Chunk 2: Rust Backend - Clarity Check Command

### Task 2.1: Add Request/Response Structs

**Files:**

- Modify: `src-tauri/src/commands/ai_grammar.rs`

- [ ] **Step 1: Add structs after tone detection structs**

```rust
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
```

- [ ] **Step 2: Add helper function for clarity check prompt**

```rust
fn build_clarity_check_prompt() -> String {
    "You are a clarity editor. Identify redundancy, complexity issues, and passive voice in the text.
Return ONLY compact JSON in this exact schema: {\"score\":85,\"issues\":[{\"issueType\":\"redundancy|complexity|passive|vague\",\"text\":\"original text\",\"suggestion\":\"improved text\",\"line\":1,\"col\":1}],\"stats\":{\"readabilityGrade\":\"Grade 8\",\"avgSentenceLength\":15.5,\"passiveVoiceCount\":2}}.
Score is overall clarity 0-100. Issues array contains specific problems with positions (1-based).
No markdown, no explanations, no code blocks.".to_string()
}
```

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/commands/ai_grammar.rs
git commit -m "feat: add clarity check structs and prompt"
```

### Task 2.2: Implement Streaming ai_clarity_check Command

**Files:**

- Modify: `src-tauri/src/commands/ai_grammar.rs`

- [ ] **Step 1: Add streaming response structs**

```rust
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AiClarityChunk {
    pub issue: Option<AiClarityIssue>,
    pub score: Option<f32>,
    pub stats: Option<AiClarityStats>,
    pub done: bool,
}
```

- [ ] **Step 2: Add the streaming command function**

```rust
#[tauri::command]
pub async fn ai_clarity_check_stream(
    app: tauri::AppHandle,
    request: AiClarityCheckRequest,
) -> Result<(), Error> {
    let request_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);

    let settings = load_app_settings(&app)?;
    let api_key = settings.openai_api_key.trim().to_string();
    if api_key.is_empty() {
        return Err(Error::Api("API key is required".to_string()));
    }

    let model = normalize_model(Some(settings.openai_model));
    let timeout_ms = settings.ai_timeout_ms.max(120_000); // Longer timeout for analysis
    let api_base_url = normalize_endpoint(Some(settings.ai_api_base_url));
    let system_prompt = build_clarity_check_prompt();

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
        return Err(Error::Api(format!("AI request failed with status {}: {}", status, body)));
    }

    let mut pending = String::new();
    let mut buffer = String::new();

    loop {
        match response.chunk().await {
            Ok(Some(bytes)) => {
                pending.push_str(&String::from_utf8_lossy(&bytes));

                while let Some(newline_pos) = pending.find('\n') {
                    let line = pending[..newline_pos].trim_end_matches('\r').to_string();
                    pending.drain(..=newline_pos);

                    if line.starts_with("data:") {
                        let data = line.trim_start_matches("data:").trim_start();
                        if data == "[DONE]" {
                            // Parse final buffered content
                            if !buffer.is_empty() {
                                if let Ok(result) = serde_json::from_str::<AiClarityCheckResponse>(&buffer) {
                                    let _ = app.emit("ai-clarity-complete", result);
                                }
                            }
                            return Ok(());
                        }

                        if let Ok(value) = serde_json::from_str::<serde_json::Value>(data) {
                            if let Some(choices) = value.get("choices").and_then(|c| c.as_array()) {
                                if let Some(first_choice) = choices.first() {
                                    let contents = collect_stream_content(first_choice);
                                    for content in contents {
                                        buffer.push_str(&content);
                                        // Try to emit partial results if valid JSON
                                        if let Ok(partial) = serde_json::from_str::<AiClarityCheckResponse>(&buffer) {
                                            let _ = app.emit("ai-clarity-partial", partial);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Ok(None) => {
                if !buffer.is_empty() {
                    if let Ok(result) = serde_json::from_str::<AiClarityCheckResponse>(&buffer) {
                        let _ = app.emit("ai-clarity-complete", result);
                    }
                }
                break;
            }
            Err(e) => {
                return Err(Error::Api(format!("Stream error: {}", e)));
            }
        }
    }

    Ok(())
}
```

- [ ] **Step 3: Register command in lib.rs**

Add to imports:

```rust
use commands::ai_grammar::{..., ai_clarity_check_stream};
```

Add to invoke_handler:

```rust
ai_clarity_check_stream,
```

- [ ] **Step 4: Run Rust tests**

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands/ai_grammar.rs src-tauri/src/lib.rs
git commit -m "feat: implement ai_clarity_check_stream command"
```

---

## Chunk 3: Rust Backend - Vocabulary Enhancement Command

### Task 3.1: Add Request/Response Structs

**Files:**

- Modify: `src-tauri/src/commands/ai_grammar.rs`

- [ ] **Step 1: Add structs**

```rust
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
```

- [ ] **Step 2: Add helper function for vocabulary prompt**

```rust
fn build_vocabulary_prompt() -> String {
    "You are a vocabulary enhancer. Suggest better words for repetitive or weak vocabulary in the text.
Return ONLY compact JSON in this exact schema: {\"suggestions\":[{\"original\":\"word\",\"line\":1,\"col\":1,\"alternatives\":[{\"word\":\"better\",\"reason\":\"more precise\"}]}]}.
Reason options: more precise, more formal, less repetitive, stronger, clearer.
Line and col are 1-based positions in original text.
No markdown, no explanations, no code blocks.".to_string()
}
```

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/commands/ai_grammar.rs
git commit -m "feat: add vocabulary enhancement structs and prompt"
```

### Task 3.2: Implement ai_vocabulary_enhance Command

**Files:**

- Modify: `src-tauri/src/commands/ai_grammar.rs`

- [ ] **Step 1: Add the command function**

```rust
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
    let system_prompt = build_vocabulary_prompt();

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

    let json_text = extract_json_object(&content);
    let parsed: AiVocabularyEnhanceResponse = serde_json::from_str(&json_text)
        .map_err(|e| Error::Api(format!("Failed to parse vocabulary enhancement response: {}", e)))?;

    Ok(parsed)
}
```

- [ ] **Step 2: Register command in lib.rs**

Add to imports:

```rust
use commands::ai_grammar::{..., ai_vocabulary_enhance};
```

Add to invoke_handler:

```rust
ai_vocabulary_enhance,
```

- [ ] **Step 3: Run Rust tests**

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands/ai_grammar.rs src-tauri/src/lib.rs
git commit -m "feat: implement ai_vocabulary_enhance command"
```

---

## Chunk 4: Frontend TypeScript Types

### Task 4.1: Create Type Definitions

**Files:**

- Create: `src/lib/types/ai.ts`

- [ ] **Step 1: Write type definitions**

```typescript
// Tone Detection Types
export interface AiTone {
  name: string;
  score: number;
}

export interface AiToneDetectResponse {
  overall: string;
  score: number;
  tones: AiTone[];
}

// Clarity Check Types
export interface AiClarityIssue {
  issueType: string;
  text: string;
  suggestion: string;
  line: number;
  col: number;
}

export interface AiClarityStats {
  readabilityGrade: string;
  avgSentenceLength: number;
  passiveVoiceCount: number;
}

export interface AiClarityCheckResponse {
  score: number;
  issues: AiClarityIssue[];
  stats: AiClarityStats;
}

// Vocabulary Enhancement Types
export interface AiVocabAlternative {
  word: string;
  reason: string;
}

export interface AiVocabSuggestion {
  original: string;
  line: number;
  col: number;
  alternatives: AiVocabAlternative[];
}

export interface AiVocabularyEnhanceResponse {
  suggestions: AiVocabSuggestion[];
}

// Enhancement State
export type EnhancementMode = "tone" | "clarity" | "vocabulary" | null;

export interface AiEnhancementState {
  mode: EnhancementMode;
  loading: boolean;
  originalText: string;
  toneResult: AiToneDetectResponse | null;
  clarityResult: AiClarityCheckResponse | null;
  vocabResult: AiVocabularyEnhanceResponse | null;
}
```

- [ ] **Step 2: Commit**

```bash
git add src/lib/types/ai.ts
git commit -m "feat: add TypeScript types for AI enhancement features"
```

---

## Chunk 5: Frontend Components

### Task 5.1: Create ToneDetectionView Component

**Files:**

- Create: `src/lib/components/ToneDetectionView.svelte`

- [ ] **Step 1: Create component file**

```svelte
<script lang="ts">
  import type { AiToneDetectResponse } from "$lib/types/ai";
  import { Badge } from "$lib/components/ui/badge";
  import { Progress } from "$lib/components/ui/progress";
  import { Sparkles } from "lucide-svelte";

  interface Props {
    result: AiToneDetectResponse;
  }

  let { result }: Props = $props();

  function getToneColor(name: string): string {
    const colors: Record<string, string> = {
      formal: "bg-blue-500",
      informal: "bg-green-500",
      friendly: "bg-yellow-500",
      serious: "bg-slate-500",
      professional: "bg-indigo-500",
      confident: "bg-emerald-500",
      academic: "bg-purple-500",
      business: "bg-cyan-500",
    };
    return colors[name.toLowerCase()] || "bg-gray-500";
  }

  function getToneIcon(name: string): string {
    const icons: Record<string, string> = {
      formal: "🎩",
      informal: "😊",
      friendly: "👋",
      serious: "🎯",
      professional: "💼",
      confident: "💪",
      academic: "📚",
      business: "📊",
    };
    return icons[name.toLowerCase()] || "✨";
  }
</script>

<div class="space-y-4 p-4">
  <!-- Primary Tone -->
  <div class="rounded-lg border bg-card p-4">
    <div class="flex items-center gap-2 text-sm text-muted-foreground mb-2">
      <Sparkles class="h-4 w-4" />
      <span>主要语气</span>
    </div>
    <div class="flex items-center gap-3">
      <span class="text-2xl">{getToneIcon(result.overall)}</span>
      <div class="flex-1">
        <div class="text-lg font-semibold capitalize">{result.overall}</div>
        <div class="flex items-center gap-2 mt-1">
          <Progress value={result.score} max={100} class="h-2 flex-1" />
          <span class="text-sm text-muted-foreground">{result.score}%</span>
        </div>
      </div>
    </div>
  </div>

  <!-- All Tones -->
  <div class="space-y-2">
    <h3 class="text-sm font-medium text-muted-foreground">检测到的语气</h3>
    <div class="space-y-2">
      {#each result.tones as tone}
        <div class="flex items-center gap-3 rounded-md border p-2">
          <span class="text-lg">{getToneIcon(tone.name)}</span>
          <div class="flex-1">
            <div class="flex items-center justify-between">
              <span class="capitalize font-medium">{tone.name}</span>
              <Badge variant="secondary">{tone.score}%</Badge>
            </div>
            <Progress value={tone.score} max={100} class="h-1.5 mt-1" />
          </div>
        </div>
      {/each}
    </div>
  </div>
</div>
```

- [ ] **Step 2: Commit**

```bash
git add src/lib/components/ToneDetectionView.svelte
git commit -m "feat: add ToneDetectionView component"
```

### Task 5.2: Create ClarityCheckView Component

**Files:**

- Create: `src/lib/components/ClarityCheckView.svelte`

- [ ] **Step 1: Create component file**

```svelte
<script lang="ts">
  import type { AiClarityCheckResponse, AiClarityIssue } from "$lib/types/ai";
  import { Badge } from "$lib/components/ui/badge";
  import { Card, CardContent } from "$lib/components/ui/card";
  import { Progress } from "$lib/components/ui/progress";
  import { AlertTriangle, CheckCircle, FileText, Type } from "lucide-svelte";

  interface Props {
    result: AiClarityCheckResponse;
  }

  let { result }: Props = $props();

  function getIssueIcon(type: string) {
    switch (type) {
      case "redundancy":
        return AlertTriangle;
      case "complexity":
        return Type;
      case "passive":
        return FileText;
      default:
        return AlertTriangle;
    }
  }

  function getIssueColor(type: string): string {
    switch (type) {
      case "redundancy":
        return "border-amber-500/50 bg-amber-500/10";
      case "complexity":
        return "border-blue-500/50 bg-blue-500/10";
      case "passive":
        return "border-purple-500/50 bg-purple-500/10";
      default:
        return "border-gray-500/50 bg-gray-500/10";
    }
  }

  function getIssueLabel(type: string): string {
    const labels: Record<string, string> = {
      redundancy: "冗余",
      complexity: "复杂",
      passive: "被动语态",
      vague: "模糊",
    };
    return labels[type] || type;
  }
</script>

<div class="space-y-4 p-4">
  <!-- Overall Score -->
  <div class="rounded-lg border bg-card p-4">
    <div class="flex items-center justify-between mb-2">
      <span class="text-sm text-muted-foreground">清晰度评分</span>
      <Badge variant={result.score >= 80 ? "default" : result.score >= 60 ? "secondary" : "destructive"}>
        {result.score}/100
      </Badge>
    </div>
    <Progress value={result.score} max={100} class="h-2" />
  </div>

  <!-- Stats -->
  <div class="grid grid-cols-3 gap-2">
    <Card>
      <CardContent class="p-3 text-center">
        <div class="text-xs text-muted-foreground">可读性</div>
        <div class="text-sm font-medium">{result.stats.readabilityGrade}</div>
      </CardContent>
    </Card>
    <Card>
      <CardContent class="p-3 text-center">
        <div class="text-xs text-muted-foreground">平均句长</div>
        <div class="text-sm font-medium">{result.stats.avgSentenceLength.toFixed(1)}</div>
      </CardContent>
    </Card>
    <Card>
      <CardContent class="p-3 text-center">
        <div class="text-xs text-muted-foreground">被动语态</div>
        <div class="text-sm font-medium">{result.stats.passiveVoiceCount}</div>
      </CardContent>
    </Card>
  </div>

  <!-- Issues -->
  <div class="space-y-2">
    <h3 class="text-sm font-medium text-muted-foreground">
      发现问题 ({result.issues.length})
    </h3>
    {#if result.issues.length === 0}
      <div class="flex items-center gap-2 rounded-md border border-emerald-500/50 bg-emerald-500/10 p-3">
        <CheckCircle class="h-4 w-4 text-emerald-500" />
        <span class="text-sm">未发现明显问题</span>
      </div>
    {:else}
      <div class="space-y-2">
        {#each result.issues as issue}
          <div class="rounded-md border p-3 {getIssueColor(issue.issueType)}">
            <div class="flex items-center gap-2 mb-2">
              <svelte:component this={getIssueIcon(issue.issueType)} class="h-4 w-4" />
              <Badge variant="outline" class="text-xs">{getIssueLabel(issue.issueType)}</Badge>
              <span class="text-xs text-muted-foreground ml-auto">第{issue.line}行</span>
            </div>
            <div class="space-y-1">
              <div class="text-sm line-through text-muted-foreground">{issue.text}</div>
              <div class="text-sm font-medium text-emerald-600">→ {issue.suggestion}</div>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>
```

- [ ] **Step 2: Commit**

```bash
git add src/lib/components/ClarityCheckView.svelte
git commit -m "feat: add ClarityCheckView component"
```

### Task 5.3: Create VocabularyEnhanceView Component

**Files:**

- Create: `src/lib/components/VocabularyEnhanceView.svelte`

- [ ] **Step 1: Create component file**

```svelte
<script lang="ts">
  import type { AiVocabularyEnhanceResponse, AiVocabSuggestion } from "$lib/types/ai";
  import { Badge } from "$lib/components/ui/badge";
  import { Button } from "$lib/components/ui/button";
  import { Card, CardContent } from "$lib/components/ui/card";
  import { Lightbulb, CheckCircle } from "lucide-svelte";

  interface Props {
    result: AiVocabularyEnhanceResponse;
    onApply?: (original: string, replacement: string) => void;
  }

  let { result, onApply }: Props = $props();

  function getReasonLabel(reason: string): string {
    const labels: Record<string, string> = {
      "more precise": "更精准",
      "more formal": "更正式",
      "less repetitive": "减少重复",
      stronger: "更有力",
      clearer: "更清晰",
    };
    return labels[reason] || reason;
  }

  function getReasonColor(reason: string): string {
    if (reason.includes("precise")) return "bg-blue-500/10 text-blue-600";
    if (reason.includes("formal")) return "bg-purple-500/10 text-purple-600";
    if (reason.includes("repetitive")) return "bg-amber-500/10 text-amber-600";
    if (reason.includes("stronger")) return "bg-emerald-500/10 text-emerald-600";
    return "bg-gray-500/10 text-gray-600";
  }
</script>

<div class="space-y-4 p-4">
  <!-- Header -->
  <div class="flex items-center gap-2 text-sm text-muted-foreground">
    <Lightbulb class="h-4 w-4" />
    <span>词汇增强建议 ({result.suggestions.length})</span>
  </div>

  <!-- Suggestions -->
  {#if result.suggestions.length === 0}
    <div class="flex items-center gap-2 rounded-md border border-emerald-500/50 bg-emerald-500/10 p-3">
      <CheckCircle class="h-4 w-4 text-emerald-500" />
      <span class="text-sm">词汇使用良好，暂无建议</span>
    </div>
  {:else}
    <div class="space-y-3">
      {#each result.suggestions as suggestion}
        <Card>
          <CardContent class="p-3 space-y-3">
            <!-- Original Word -->
            <div class="flex items-center justify-between">
              <div>
                <div class="text-xs text-muted-foreground">原文</div>
                <div class="text-sm font-medium">"{suggestion.original}"</div>
              </div>
              <div class="text-xs text-muted-foreground">
                第{suggestion.line}行
              </div>
            </div>

            <!-- Alternatives -->
            <div class="space-y-2">
              <div class="text-xs text-muted-foreground">建议替换</div>
              {#each suggestion.alternatives as alt}
                <div class="flex items-center gap-2">
                  <div class="flex-1 rounded-md border p-2">
                    <div class="flex items-center justify-between mb-1">
                      <span class="font-medium text-emerald-600">{alt.word}</span>
                      <Badge class="text-xs {getReasonColor(alt.reason)}">
                        {getReasonLabel(alt.reason)}
                      </Badge>
                    </div>
                  </div>
                  {#if onApply}
                    <Button
                      variant="outline"
                      size="sm"
                      onclick={() => onApply(suggestion.original, alt.word)}
                    >
                      使用
                    </Button>
                  {/if}
                </div>
              {/each}
            </div>
          </CardContent>
        </Card>
      {/each}
    </div>
  {/if}
</div>
```

- [ ] **Step 2: Commit**

```bash
git add src/lib/components/VocabularyEnhanceView.svelte
git commit -m "feat: add VocabularyEnhanceView component"
```

---

## Chunk 6: AI Popup Integration

### Task 6.1: Modify AI Popup Page

**Files:**

- Modify: `src/routes/ai-popup/+page.svelte`

- [ ] **Step 1: Add imports and types**

At the top of the file, add:

```typescript
import type {
  AiToneDetectResponse,
  AiClarityCheckResponse,
  AiVocabularyEnhanceResponse,
  EnhancementMode,
} from "$lib/types/ai";
import ToneDetectionView from "$lib/components/ToneDetectionView.svelte";
import ClarityCheckView from "$lib/components/ClarityCheckView.svelte";
import VocabularyEnhanceView from "$lib/components/VocabularyEnhanceView.svelte";
```

- [ ] **Step 2: Add state variables**

After existing state declarations, add:

```typescript
let enhancementMode: EnhancementMode = $state(null);
let toneResult: AiToneDetectResponse | null = $state(null);
let clarityResult: AiClarityCheckResponse | null = $state(null);
let vocabResult: AiVocabularyEnhanceResponse | null = $state(null);
let enhancementLoading = $state(false);
```

- [ ] **Step 3: Add enhancement functions**

Add after existing functions:

```typescript
async function runToneDetection() {
  if (!selectedText) return;
  enhancementLoading = true;
  enhancementMode = "tone";

  try {
    const result = await invoke<AiToneDetectResponse>("ai_tone_detect", {
      request: { text: selectedText },
    });
    toneResult = result;
  } catch (error) {
    console.error("Tone detection failed:", error);
    // Show error toast
  } finally {
    enhancementLoading = false;
  }
}

async function runClarityCheck() {
  if (!selectedText) return;
  enhancementLoading = true;
  enhancementMode = "clarity";

  try {
    // Setup event listeners for streaming
    const unlistenPartial = await listen<AiClarityCheckResponse>(
      "ai-clarity-partial",
      (event) => {
        clarityResult = event.payload;
      },
    );

    const unlistenComplete = await listen<AiClarityCheckResponse>(
      "ai-clarity-complete",
      (event) => {
        clarityResult = event.payload;
        unlistenPartial();
        unlistenComplete();
      },
    );

    await invoke("ai_clarity_check_stream", {
      request: { text: selectedText },
    });
  } catch (error) {
    console.error("Clarity check failed:", error);
  } finally {
    enhancementLoading = false;
  }
}

async function runVocabularyEnhance() {
  if (!selectedText) return;
  enhancementLoading = true;
  enhancementMode = "vocabulary";

  try {
    const result = await invoke<AiVocabularyEnhanceResponse>(
      "ai_vocabulary_enhance",
      {
        request: { text: selectedText },
      },
    );
    vocabResult = result;
  } catch (error) {
    console.error("Vocabulary enhancement failed:", error);
  } finally {
    enhancementLoading = false;
  }
}
```

- [ ] **Step 4: Add UI buttons**

Find the existing buttons (translate, polish) and add three new buttons:

```svelte
<Button
  variant={enhancementMode === 'tone' ? 'default' : 'outline'}
  size="sm"
  onclick={runToneDetection}
  disabled={enhancementLoading}
>
  {#if enhancementLoading && enhancementMode === 'tone'}
    <Loader2 class="mr-1 h-3 w-3 animate-spin" />
  {:else}
    <Sparkles class="mr-1 h-3 w-3" />
  {/if}
  语气
</Button>

<Button
  variant={enhancementMode === 'clarity' ? 'default' : 'outline'}
  size="sm"
  onclick={runClarityCheck}
  disabled={enhancementLoading}
>
  {#if enhancementLoading && enhancementMode === 'clarity'}
    <Loader2 class="mr-1 h-3 w-3 animate-spin" />
  {:else}
    <FileText class="mr-1 h-3 w-3" />
  {/if}
  清晰度
</Button>

<Button
  variant={enhancementMode === 'vocabulary' ? 'default' : 'outline'}
  size="sm"
  onclick={runVocabularyEnhance}
  disabled={enhancementLoading}
>
  {#if enhancementLoading && enhancementMode === 'vocabulary'}
    <Loader2 class="mr-1 h-3 w-3 animate-spin" />
  {:else}
    <Type class="mr-1 h-3 w-3" />
  {/if}
  词汇
</Button>
```

- [ ] **Step 5: Add results display area**

In the two-column layout, add the enhancement results panel:

```svelte
<!-- Right column: Results -->
<div class="flex-1 flex flex-col">
  <!-- ... existing tabs ... -->

  <!-- Enhancement Results -->
  {#if enhancementMode}
    <div class="flex-1 overflow-auto border rounded-md">
      {#if enhancementLoading && !toneResult && !clarityResult && !vocabResult}
        <div class="flex items-center justify-center h-full">
          <Loader2 class="h-6 w-6 animate-spin text-muted-foreground" />
        </div>
      {:else if enhancementMode === 'tone' && toneResult}
        <ToneDetectionView result={toneResult} />
      {:else if enhancementMode === 'clarity' && clarityResult}
        <ClarityCheckView result={clarityResult} />
      {:else if enhancementMode === 'vocabulary' && vocabResult}
        <VocabularyEnhanceView result={vocabResult} />
      {/if}
    </div>
  {/if}
</div>
```

- [ ] **Step 6: Commit**

```bash
git add src/routes/ai-popup/+page.svelte
git commit -m "feat: integrate AI enhancement features into popup"
```

---

## Chunk 7: Testing & Quality Gates

### Task 7.1: Run TypeScript Check

```bash
cd /Users/sternelee/www/github/autocorrect/autocorrect-app
npm run check
```

Expected: No TypeScript errors

### Task 7.2: Run Frontend Lint

```bash
npm run lint
```

Expected: No linting errors

### Task 7.3: Run Rust Tests

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: All tests pass

### Task 7.4: Build Test

```bash
npm run build
```

Expected: Build succeeds without errors

### Task 7.5: Final Commit

```bash
git add .
git commit -m "feat: complete AI enhancement features (tone, clarity, vocabulary)"
```

---

## Summary

This plan implements three AI writing enhancement features:

1. **Tone Detection** (`ai_tone_detect`): Non-streaming, returns overall tone + all detected tones with scores
2. **Clarity Check** (`ai_clarity_check_stream`): Streaming, analyzes redundancy, complexity, passive voice with real-time updates
3. **Vocabulary Enhancement** (`ai_vocabulary_enhance`): Non-streaming, provides context-aware word alternatives

All features integrate seamlessly with the existing AI popup system, follow established patterns, and use the same API configuration as translate/polish features.
