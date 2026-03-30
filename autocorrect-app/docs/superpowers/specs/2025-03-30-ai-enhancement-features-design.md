# AI Enhancement Features Design

**Date:** 2025-03-30  
**Status:** Approved  
**Related:** AI_ROADMAP.md

## Overview

Implement three AI-powered writing enhancement features based on Grammarly functionality comparison: tone detection, clarity analysis, and vocabulary enhancement.

## Goals

- Detect text tone and provide tone adjustment capabilities
- Analyze text clarity and readability with improvement suggestions
- Enhance vocabulary with context-aware word alternatives
- Seamlessly integrate with existing AI popup system

## Features

### 1. Tone Detection & Adjustment (`ai_tone_detect`)

**Function:** Analyze text to detect tone characteristics and provide adjustment suggestions.

**Tone Types Detected:**

- Formal / 正式
- Informal / 随意
- Friendly / 友好
- Serious / 严肃
- Professional / 专业
- Confident / 自信
- Academic / 学术
- Business / 商务

**API Response:**

```rust
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AiToneDetectResponse {
    pub overall: String,      // Primary detected tone
    pub score: f32,           // Confidence score 0-100
    pub tones: Vec<AiTone>,   // All detected tones with scores
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AiTone {
    pub name: String,
    pub score: f32,           // 0-100
}
```

**Non-streaming response** (structured JSON from LLM)

### 2. Clarity & Conciseness (`ai_clarity_check`)

**Function:** Analyze text for clarity issues, redundancy, and readability.

**Issues Detected:**

- Redundant phrases
- Complex sentence structures
- Passive voice usage
- Readability score

**API Response:**

```rust
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AiClarityCheckResponse {
    pub score: f32,                    // Overall clarity score 0-100
    pub issues: Vec<AiClarityIssue>,
    pub stats: AiClarityStats,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AiClarityIssue {
    pub issue_type: String,  // "redundancy" | "complexity" | "passive" | "vague"
    pub text: String,        // Problematic text
    pub suggestion: String,  // Improvement suggestion
    pub line: usize,
    pub col: usize,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AiClarityStats {
    pub readability_grade: String,  // e.g., "Grade 8"
    pub avg_sentence_length: f32,
    pub passive_voice_count: usize,
}
```

**Streaming response** for real-time issue display

### 3. Vocabulary Enhancement (`ai_vocabulary_enhance`)

**Function:** Provide context-aware word alternatives for repetitive or weak vocabulary.

**API Response:**

```rust
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
    pub reason: String,  // "more precise" | "more formal" | "less repetitive"
}
```

**Non-streaming response** (batch processing)

## UI Design

### Layout

Two-column layout within the existing AI popup:

```
┌─────────────────────────────────────────────────────────────┐
│  [翻译] [润色▼] [语气] [清晰度] [词汇增强] [语法]        │
├───────────────────────┬─────────────────────────────────────┤
│                       │                                     │
│   Original Text       │   Analysis Panel                    │
│   (readonly)          │   (scrollable)                      │
│                       │                                     │
├───────────────────────┴─────────────────────────────────────┤
│  [取消]                                    [一键应用]       │
└─────────────────────────────────────────────────────────────┘
```

### Analysis Panel Content

**Tone Detection View:**

- Primary tone badge with confidence score
- List of all detected tones with visual bars
- Suggested tone adjustment options

**Clarity View:**

- Overall clarity score (0-100)
- Issue cards with:
  - Issue type icon
  - Original text (highlighted)
  - Suggested improvement
  - Line/column position
- Readability statistics

**Vocabulary View:**

- Word suggestions list
- Original word + alternatives
- Reason for each alternative
- "Use this" button per suggestion

### Color Scheme

Uses existing Tailwind CSS color system:

- Issues: `text-amber-500` / `bg-amber-500/10`
- Improvements: `text-emerald-500` / `bg-emerald-500/10`
- Neutral: `text-muted-foreground`

## Implementation

### Backend (Rust)

**New Commands in `src-tauri/src/commands/ai_grammar.rs`:**

```rust
#[tauri::command]
pub async fn ai_tone_detect(
    app: tauri::AppHandle,
    request: AiToneDetectRequest,
) -> Result<AiToneDetectResponse, Error>

#[tauri::command]
pub async fn ai_clarity_check(
    app: tauri::AppHandle,
    request: AiClarityCheckRequest,
) -> Result<AiClarityCheckResponse, Error>

#[tauri::command]
pub async fn ai_vocabulary_enhance(
    app: tauri::AppHandle,
    request: AiVocabularyEnhanceRequest,
) -> Result<AiVocabularyEnhanceResponse, Error>
```

**Prompt Design:**

1. **Tone Detection:**

```
You are a tone analyzer. Analyze the text and identify the tone.
Return ONLY compact JSON: {"overall":"string","score":85,"tones":[{"name":"string","score":85},...]}.
Tone options: formal, informal, friendly, serious, professional, confident, academic, business.
No markdown, no explanations.
```

2. **Clarity Check:**

```
You are a clarity editor. Identify redundancy, complexity issues, and passive voice.
Return ONLY JSON: {"score":85,"issues":[{"type":"redundancy|complexity|passive","text":"...","suggestion":"...","line":1,"col":1}],"stats":{"readabilityGrade":"Grade 8","avgSentenceLength":15.5,"passiveVoiceCount":2}}.
```

3. **Vocabulary Enhancement:**

```
You are a vocabulary enhancer. Suggest better words for repetitive or weak vocabulary.
Return ONLY JSON: {"suggestions":[{"original":"word","line":1,"col":1,"alternatives":[{"word":"better","reason":"more precise"}]}]}.
```

### Frontend (Svelte)

**Components:**

1. **AiEnhancementPanel.svelte** - Main container with tab switching
2. **ToneDetectionView.svelte** - Tone analysis display
3. **ClarityCheckView.svelte** - Clarity issues list
4. **VocabularyEnhanceView.svelte** - Word suggestions
5. **AnalysisResultCard.svelte** - Reusable issue/suggestion card

**State Management:**

```typescript
interface AiEnhancementState {
  mode: "tone" | "clarity" | "vocabulary" | null;
  loading: boolean;
  originalText: string;
  toneResult: ToneDetectResponse | null;
  clarityResult: ClarityCheckResponse | null;
  vocabResult: VocabularyEnhanceResponse | null;
}
```

### Integration Points

1. **AI Popup:** Add three new buttons next to existing translate/polish buttons
2. **Commands:** Register new commands in `lib.rs` invoke handler
3. **TypeScript Types:** Add to frontend type definitions
4. **i18n:** Add translation keys for new UI elements

## Error Handling

- Empty API key: Show settings prompt
- API errors: Display user-friendly error message
- Timeout: Retry with exponential backoff (max 3 attempts)
- Empty results: Show "No issues found" state

## Performance

- Reuse existing HTTP client configuration
- Cache results for same text (5 minute TTL)
- Stream clarity check for large texts
- Debounce button clicks (300ms)

## Testing

1. **Unit Tests:** Mock LLM responses, test JSON parsing
2. **Integration Tests:** End-to-end flow with test API key
3. **UI Tests:** Component rendering and interaction
4. **Manual QA:** Test with various text types (technical, casual, academic)

## Future Enhancements

- Tone adjustment (convert formal to casual, etc.)
- Custom tone profiles
- Readability comparison with benchmarks
- Vocabulary diversity score
- Export analysis report

## Security

- API keys stored in Tauri secure storage (existing mechanism)
- No text logging to external services
- Local-only processing for suggestions application
