export interface AiTone {
  name: string;
  score: number;
}

export interface AiToneDetectResponse {
  overall: string;
  score: number;
  tones: AiTone[];
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
