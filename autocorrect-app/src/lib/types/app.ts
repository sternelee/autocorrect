// Shared frontend types that mirror Tauri IPC return shapes.
// Keep this file in sync with the Rust command return types.

export interface AppConfig {
  rules?: Record<string, number>;
  textRules?: Record<string, number>;
  spellcheckWords?: string[];
  fileTypes?: Record<string, string>;
  context?: Record<string, number>;
  configPath?: string;
  typoCheckingEnabled?: boolean;
  aiGrammarEnabled?: boolean;
  openaiApiKey?: string;
  openaiModel?: string;
  aiMaxInputChars?: number;
  aiTimeoutMs?: number;
  aiApiBaseUrl?: string;
  aiTranslateTargetLanguage?: string;
  aiPolishStyle?: string[];
  aiPolishStyles?: string[];
  uiLanguage?: string;
  underlineStyle?: string;
  underlineColor?: string;
}

export interface RuleInfo {
  name: string;
  severity: number;
  description: string;
  defaultSeverity: number;
}

export interface Modifiers {
  shift: boolean;
  ctrl: boolean;
  meta: boolean;
  alt: boolean;
}

export interface HotkeyConfig {
  key: string;
  modifiers: Modifiers;
  display_string: string;
}

export interface LineChange {
  line: number;
  col: number;
  original: string;
  corrected: string;
  severity: number;
}

export interface TypoSuggestion {
  typo: string;
  suggestions: string[];
  line: number;
  col: number;
}

export interface SpellCheckResult {
  original: string;
  corrected: string;
  has_changes: boolean;
  line_changes: LineChange[];
  typos: TypoSuggestion[];
}

export interface AiTextTransformResponse {
  outputText?: string;
  typos?: TypoSuggestion[];
}

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
