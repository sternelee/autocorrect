<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { Ban, RefreshCw, Sparkles, X } from "lucide-svelte";
  import { onMount } from "svelte";
  import { locale, t } from "$lib/i18n";
  import {
    Tooltip,
    TooltipTrigger,
    TooltipContent,
    TooltipProvider,
  } from "$lib/components/ui/tooltip";
  import { Button } from "$lib/components/ui/button";
  import { Textarea } from "$lib/components/ui/textarea";
  import type {
    AiAssistResponse,
    AiClarityCheckResponse,
    AiToneDetectResponse,
    AiVocabularyEnhanceResponse,
  } from "$lib/types/ai";
  import type { AppConfig } from "$lib/types/app";
  import type { ThemeMode } from "$lib/types/theme";
  import {
    applyThemeToDom,
    isThemeMode,
    loadThemeFromLocalStorage,
    saveThemeToLocalStorage,
  } from "$lib/theme";

  $locale;

  const tr = $derived(
    (key: string, params?: Record<string, string | number>) => {
      const _ = $locale;
      return t(key, params);
    },
  );

  type AssistAction = "translate" | "rewrite" | "paraphrase" | "concise";
  type AnalysisTool = "tone" | "clarity" | "vocabulary";
  type Tool = AssistAction | AnalysisTool;

  const TRANSLATION_CHAR_LIMIT = 4000;

  const assistTools: {
    id: AssistAction;
    icon: string;
    label: () => string;
    description: () => string;
  }[] = [
    {
      id: "translate",
      icon: "🌐",
      label: () => tr("aipopup.translate"),
      description: () => tr("aipopup.translateDesc"),
    },
    {
      id: "rewrite",
      icon: "✨",
      label: () => tr("aipopup.rewrite"),
      description: () => tr("aipopup.rewriteDesc"),
    },
    {
      id: "paraphrase",
      icon: "🪄",
      label: () => tr("aipopup.paraphrase"),
      description: () => tr("aipopup.paraphraseDesc"),
    },
    {
      id: "concise",
      icon: "✂️",
      label: () => tr("aipopup.concise"),
      description: () => tr("aipopup.conciseDesc"),
    },
  ];

  const analysisTools: {
    id: AnalysisTool;
    icon: string;
    label: () => string;
    description: () => string;
  }[] = [
    {
      id: "tone",
      icon: "🎯",
      label: () => tr("aipopup.tone"),
      description: () => tr("aipopup.toneDesc"),
    },
    {
      id: "clarity",
      icon: "🧭",
      label: () => tr("aipopup.clarity"),
      description: () => tr("aipopup.clarityDesc"),
    },
    {
      id: "vocabulary",
      icon: "📚",
      label: () => tr("aipopup.vocabulary"),
      description: () => tr("aipopup.vocabularyDesc"),
    },
  ];

  const languages = [
    "English",
    "English (US)",
    "English (UK)",
    "简体中文",
    "繁體中文",
    "Español",
    "Español (México)",
    "Français",
    "Deutsch",
    "Italiano",
    "Português",
    "Português (Brasil)",
    "Nederlands",
    "Svenska",
    "Polski",
    "Türkçe",
    "Українська",
    "日本語",
    "한국어",
    "Bahasa Indonesia",
  ];

  let selectedText = $state("");
  let result = $state("");
  let loading = $state(false);
  let error = $state("");
  let activeTool = $state<Tool | null>(null);
  let translateLang = $state("English");
  let ignoreMessage = $state("");
  let ignoreError = $state(false);
  let assistResult = $state<AiAssistResponse | null>(null);
  let lastAssistAction = $state<AssistAction | null>(null);
  let toneResult = $state<AiToneDetectResponse | null>(null);
  let clarityResult = $state<AiClarityCheckResponse | null>(null);
  let vocabResult = $state<AiVocabularyEnhanceResponse | null>(null);
  let clarityStreamBuffer = $state("");

  let sourceAppName = $state("");
  let sourceBundleId = $state("");
  let theme: ThemeMode = $state("auto");
  let mediaQuery: MediaQueryList | null = null;

  const selectedCharCount = $derived(Array.from(selectedText).length);
  const preview = $derived(
    selectedText.length > 140 ? selectedText.slice(0, 140) + "…" : selectedText,
  );

  function isAssistTool(tool: Tool | null): tool is AssistAction {
    return (
      tool === "translate" ||
      tool === "rewrite" ||
      tool === "paraphrase" ||
      tool === "concise"
    );
  }

  function clearAssistState() {
    assistResult = null;
    lastAssistAction = null;
    result = "";
    error = "";
    if (isAssistTool(activeTool)) {
      activeTool = null;
    }
  }

  function clearAnalysisState() {
    toneResult = null;
    clarityResult = null;
    vocabResult = null;
    clarityStreamBuffer = "";
  }

  function clearTransientState() {
    clearAssistState();
    clearAnalysisState();
    result = "";
    error = "";
    loading = false;
  }

  function tryParseClarityBuffer(raw: string): AiClarityCheckResponse | null {
    const trimmed = raw.trim();
    if (!trimmed) return null;

    try {
      return JSON.parse(trimmed) as AiClarityCheckResponse;
    } catch {
      const scoreMatch = trimmed.match(/"score"\s*:\s*([0-9]+(?:\.[0-9]+)?)/);
      const issuesMatch = trimmed.match(/"issues"\s*:\s*(\[[\s\S]*?\])/);
      const statsMatch = trimmed.match(/"stats"\s*:\s*(\{[\s\S]*\})/);

      let issues: AiClarityCheckResponse["issues"] = [];
      if (issuesMatch?.[1]) {
        try {
          issues = JSON.parse(
            issuesMatch[1],
          ) as AiClarityCheckResponse["issues"];
        } catch {
          issues = [];
        }
      }

      let stats: AiClarityCheckResponse["stats"] = {
        readabilityGrade: "-",
        avgSentenceLength: 0,
        passiveVoiceCount: 0,
      };

      if (statsMatch?.[1]) {
        try {
          stats = JSON.parse(statsMatch[1]) as AiClarityCheckResponse["stats"];
        } catch {
          // keep defaults until full payload arrives
        }
      }

      if (!scoreMatch && issues.length === 0) {
        return null;
      }

      return {
        score: scoreMatch ? Number(scoreMatch[1]) : 0,
        issues,
        stats,
      };
    }
  }

  async function loadThemeFromStore(): Promise<ThemeMode> {
    try {
      const stored = await invoke<string>("get_theme");
      if (isThemeMode(stored)) {
        return stored;
      }
    } catch (themeError) {
      console.warn(
        "Failed to load AI popup theme from store, fallback to localStorage:",
        themeError,
      );
    }
    return loadThemeFromLocalStorage();
  }

  async function loadAiDefaults() {
    try {
      const config = await invoke<AppConfig>("get_config");
      translateLang = config.aiTranslateTargetLanguage ?? "English";
    } catch (configError) {
      console.warn("Failed to load AI popup defaults:", configError);
    }
  }

  function applyTheme(mode: ThemeMode) {
    theme = mode;
    applyThemeToDom(mode);
    saveThemeToLocalStorage(mode);
  }

  function handleSystemThemeChange(event: MediaQueryListEvent) {
    if (theme === "auto") {
      document.documentElement.classList.toggle("dark", event.matches);
    }
  }

  function setupSystemThemeListener() {
    if (mediaQuery) {
      mediaQuery.removeEventListener("change", handleSystemThemeChange);
    }
    mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
    mediaQuery.addEventListener("change", handleSystemThemeChange);
  }

  function cleanupThemeListener() {
    if (mediaQuery) {
      mediaQuery.removeEventListener("change", handleSystemThemeChange);
      mediaQuery = null;
    }
  }

  async function ensureAiEnabled() {
    const config = await invoke<AppConfig>("get_config");
    if (!config.aiGrammarEnabled) {
      throw new Error(tr("aipopup.aiEnableHint"));
    }
  }

  async function runAssist(action: AssistAction) {
    if (!selectedText.trim() || loading) return;

    if (action === "translate" && selectedCharCount > TRANSLATION_CHAR_LIMIT) {
      activeTool = action;
      clearAnalysisState();
      assistResult = null;
      result = "";
      error = tr("aipopup.translateLimitError", {
        limit: TRANSLATION_CHAR_LIMIT,
      });
      return;
    }

    activeTool = action;
    loading = true;
    error = "";
    clearAnalysisState();
    assistResult = null;
    result = "";

    try {
      await ensureAiEnabled();
      const response = await invoke<AiAssistResponse>("ai_assist", {
        request: {
          text: selectedText,
          action,
          targetLanguage: action === "translate" ? translateLang : null,
        },
      });
      assistResult = response;
      result = response.primaryText;
      lastAssistAction = action;
    } catch (assistError) {
      error =
        assistError instanceof Error
          ? assistError.message
          : String(assistError);
    } finally {
      loading = false;
    }
  }

  async function regenerateAssist() {
    if (!lastAssistAction || loading) return;
    await runAssist(lastAssistAction);
  }

  async function runTool(tool: Tool) {
    if (!selectedText.trim() || loading) return;

    if (isAssistTool(tool)) {
      await runAssist(tool);
      return;
    }

    activeTool = tool;
    loading = true;
    error = "";
    clearAssistState();
    clearAnalysisState();
    result = "";

    try {
      await ensureAiEnabled();

      if (tool === "tone") {
        toneResult = await invoke<AiToneDetectResponse>("ai_tone_detect", {
          request: { text: selectedText },
        });
        return;
      }

      if (tool === "vocabulary") {
        vocabResult = await invoke<AiVocabularyEnhanceResponse>(
          "ai_vocabulary_enhance",
          {
            request: { text: selectedText },
          },
        );
        return;
      }

      const useStream = selectedText.length > 280;
      if (useStream) {
        await invoke("ai_clarity_check_stream", {
          request: { text: selectedText },
        });
        return;
      }

      clarityResult = await invoke<AiClarityCheckResponse>("ai_clarity_check", {
        request: { text: selectedText },
      });
      loading = false;
    } catch (toolError) {
      error =
        toolError instanceof Error ? toolError.message : String(toolError);
      loading = false;
    }
  }

  async function acceptResult() {
    if (!result.trim()) return;
    await invoke("accept_ai_result", { text: result });
  }

  function pickAssistText(text: string) {
    result = text;
  }

  function applyVocabularySuggestion(original: string, replacement: string) {
    if (!selectedText || !original) return;
    result = selectedText.split(original).join(replacement);
  }

  function getIssueTypeLabel(type: string): string {
    const map: Record<string, string> = {
      redundancy: tr("aipopup.analysis.issue.redundancy"),
      complexity: tr("aipopup.analysis.issue.complexity"),
      passive: tr("aipopup.analysis.issue.passive"),
      vague: tr("aipopup.analysis.issue.vague"),
    };
    return map[type] ?? type;
  }

  function buildClarityAppliedText(): string {
    if (!clarityResult) return selectedText;
    let next = selectedText;
    for (const issue of clarityResult.issues) {
      if (issue.text && issue.suggestion) {
        next = next.split(issue.text).join(issue.suggestion);
      }
    }
    return next;
  }

  function buildVocabularyAppliedText(): string {
    if (!vocabResult) return selectedText;
    let next = selectedText;
    for (const item of vocabResult.suggestions) {
      const first = item.alternatives[0];
      if (item.original && first?.word) {
        next = next.split(item.original).join(first.word);
      }
    }
    return next;
  }

  function acceptAnalysisResult() {
    if (activeTool === "vocabulary") {
      const next = result.trim() ? result : buildVocabularyAppliedText();
      if (!next.trim()) {
        return;
      }
      if (!window.confirm(tr("aipopup.analysis.confirmApplyAll"))) {
        return;
      }
      result = next;
      void acceptResult();
      return;
    }

    if (
      activeTool === "clarity" &&
      clarityResult &&
      clarityResult.issues.length > 0
    ) {
      if (!window.confirm(tr("aipopup.analysis.confirmApplyAll"))) {
        return;
      }
      result = buildClarityAppliedText();
      void acceptResult();
    }
  }

  async function close() {
    await invoke("hide_ai_popup");
  }

  async function ignoreApp() {
    if (!sourceAppName || !sourceBundleId) {
      ignoreError = true;
      ignoreMessage = tr("popup.ignoreError");
      setTimeout(() => {
        ignoreMessage = "";
        ignoreError = false;
      }, 2000);
      return;
    }

    try {
      await invoke("add_ignored_app", {
        name: sourceAppName,
        bundleId: sourceBundleId,
        ignorePopup: true,
        ignoreOverlay: true,
      });

      ignoreMessage = tr("popup.ignored", { name: sourceAppName });
      setTimeout(() => {
        close();
      }, 800);
    } catch (ignoreAppError) {
      console.error("Failed to ignore app:", ignoreAppError);
      ignoreError = true;
      ignoreMessage = tr("popup.ignoreError");
      setTimeout(() => {
        ignoreMessage = "";
        ignoreError = false;
      }, 2000);
    }
  }

  function isInteractiveTarget(
    target: EventTarget | null,
  ): target is HTMLElement {
    return (
      target instanceof HTMLElement &&
      Boolean(
        target.closest(
          "button, input, textarea, select, a, [role='button'], [data-no-drag]",
        ),
      )
    );
  }

  async function startWindowDrag(event: MouseEvent) {
    if (event.button !== 0 || isInteractiveTarget(event.target)) return;
    try {
      await getCurrentWindow().startDragging();
    } catch (dragError) {
      console.error("Failed to start AI popup drag:", dragError);
    }
  }

  onMount(() => {
    const unlistenThemePromise = listen<ThemeMode>("theme-changed", (event) => {
      const mode = event.payload;
      if (isThemeMode(mode)) {
        applyTheme(mode);
      }
    });

    loadThemeFromStore().then((mode) => {
      applyTheme(mode);
      setupSystemThemeListener();
    });
    void loadAiDefaults();

    (async () => {
      try {
        const state = await invoke<{
          sourceAppName?: string;
          sourceBundleId?: string;
        }>("get_ai_popup_state");
        sourceAppName = state.sourceAppName || "";
        sourceBundleId = state.sourceBundleId || "";
      } catch (stateError) {
        console.error("Failed to get ai popup state:", stateError);
      }
    })();

    const unlistenShowPromise = listen<{ selectedText: string }>(
      "ai-popup-show",
      async (event) => {
        selectedText = event.payload.selectedText;
        clearTransientState();

        try {
          const state = await invoke<{
            sourceAppName?: string;
            sourceBundleId?: string;
          }>("get_ai_popup_state");
          sourceAppName = state.sourceAppName || "";
          sourceBundleId = state.sourceBundleId || "";
        } catch (stateError) {
          console.error("Failed to get ai popup state:", stateError);
        }
      },
    );

    const unlistenClarityChunkPromise = listen<string>(
      "ai-clarity-chunk",
      (event) => {
        if (activeTool !== "clarity") {
          return;
        }
        clarityStreamBuffer += event.payload;
        const partial = tryParseClarityBuffer(clarityStreamBuffer);
        if (partial) {
          clarityResult = partial;
        }
      },
    );

    const unlistenClarityCompletePromise = listen("ai-clarity-complete", () => {
      if (activeTool !== "clarity") {
        return;
      }
      const parsed = tryParseClarityBuffer(clarityStreamBuffer);
      if (parsed) {
        clarityResult = parsed;
      } else if (clarityStreamBuffer.trim()) {
        error = tr("aipopup.clarityParseError");
      }
      loading = false;
    });

    const unlistenClarityErrorPromise = listen<string>(
      "ai-clarity-error",
      (event) => {
        if (activeTool !== "clarity") {
          return;
        }
        error = event.payload;
        loading = false;
      },
    );

    invoke<{ selectedText: string }>("get_ai_popup_state")
      .then((state) => {
        if (state.selectedText) selectedText = state.selectedText;
      })
      .catch(() => {});

    return () => {
      unlistenThemePromise.then((fn) => fn());
      unlistenShowPromise.then((fn) => fn());
      unlistenClarityChunkPromise.then((fn) => fn());
      unlistenClarityCompletePromise.then((fn) => fn());
      unlistenClarityErrorPromise.then((fn) => fn());
      cleanupThemeListener();
    };
  });
</script>

<svelte:window
  onkeydown={(event) => {
    if (event.key === "Escape") close();
  }}
/>

<div class="popup" data-locale={$locale}>
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="header" onmousedown={startWindowDrag}>
    <span class="header-icon">
      <Sparkles class="h-3.5 w-3.5" />
    </span>
    <span class="title">{tr("aipopup.tools")}</span>
    <div class="header-actions" data-no-drag>
      {#if ignoreMessage}
        <span class="ignore-message" class:error={ignoreError}>
          {ignoreMessage}
        </span>
      {/if}
      <TooltipProvider>
        <Tooltip>
          <TooltipTrigger>
            <button
              class="icon-btn ignore"
              aria-label={tr("popup.ignoreTooltip")}
              onclick={ignoreApp}
            >
              <Ban class="h-3.5 w-3.5" />
            </button>
          </TooltipTrigger>
          <TooltipContent side="top">{tr("popup.ignoreTooltip")}</TooltipContent
          >
        </Tooltip>
      </TooltipProvider>
      <button
        class="icon-btn close"
        title={tr("aipopup.close")}
        onclick={close}
      >
        <X class="h-3.5 w-3.5" />
      </button>
    </div>
  </div>

  {#if selectedText}
    <div class="preview">
      <div class="preview-meta">
        <span class="preview-label">{tr("aipopup.selected")}</span>
        <span class="char-count">
          {selectedCharCount}
          {tr("aipopup.characters")}
        </span>
      </div>
      <p class="preview-text">{preview}</p>
    </div>
  {/if}

  <div class="section">
    <div class="section-header">
      <span class="section-title">{tr("aipopup.rewriteSection")}</span>
      <span class="section-subtitle">{tr("aipopup.rewriteSectionDesc")}</span>
    </div>
    <div class="tool-grid tool-grid-assist">
      {#each assistTools as tool}
        <button
          class="tool-card"
          class:active={activeTool === tool.id}
          onclick={() => runTool(tool.id)}
          disabled={loading}
        >
          <span class="tool-icon">{tool.icon}</span>
          <span class="tool-title">{tool.label()}</span>
          <span class="tool-description">{tool.description()}</span>
        </button>
      {/each}
    </div>
  </div>

  <div class="section">
    <div class="section-header">
      <span class="section-title">{tr("aipopup.analysisSection")}</span>
      <span class="section-subtitle">{tr("aipopup.analysisSectionDesc")}</span>
    </div>
    <div class="tool-grid tool-grid-analysis">
      {#each analysisTools as tool}
        <button
          class="tool-card tool-card-compact"
          class:active={activeTool === tool.id}
          onclick={() => runTool(tool.id)}
          disabled={loading}
        >
          <span class="tool-icon">{tool.icon}</span>
          <span class="tool-title">{tool.label()}</span>
          <span class="tool-description">{tool.description()}</span>
        </button>
      {/each}
    </div>
  </div>

  {#if activeTool === "translate"}
    <div class="translate-panel">
      <div class="translate-meta">
        <span class="translate-label">{tr("aipopup.into")}</span>
        <span class="translate-hint">{tr("aipopup.translateHint")}</span>
      </div>
      <div class="lang-row">
        {#each languages as lang}
          <button
            class="lang-btn"
            class:selected={translateLang === lang}
            disabled={loading}
            onclick={() => {
              translateLang = lang;
              void runAssist("translate");
            }}
          >
            {lang}
          </button>
        {/each}
      </div>
      <div
        class="limit-note"
        class:limit-note-error={selectedCharCount > TRANSLATION_CHAR_LIMIT}
      >
        {tr("aipopup.translateLimit", {
          count: selectedCharCount,
          limit: TRANSLATION_CHAR_LIMIT,
        })}
      </div>
    </div>
  {/if}

  {#if loading}
    <div class="loading">
      <div class="spinner"></div>
      <span>{tr("aipopup.generating")}</span>
    </div>
  {/if}

  {#if error}
    <div class="error">{error}</div>
  {/if}

  {#if assistResult && isAssistTool(activeTool)}
    <div class="assist-panel">
      <div class="assist-header">
        <div>
          <div class="assist-headline">{assistResult.headline}</div>
          <div class="assist-summary">{assistResult.summary}</div>
        </div>
        <button
          class="ghost-action"
          onclick={regenerateAssist}
          disabled={loading}
        >
          <RefreshCw class="h-3.5 w-3.5" />
          {tr("aipopup.tryAnother")}
        </button>
      </div>

      {#if assistResult.focus.length > 0}
        <div class="focus-row">
          {#each assistResult.focus as focus}
            <span class="focus-chip">{focus}</span>
          {/each}
        </div>
      {/if}

      <div class="suggestion-card">
        <div class="suggestion-label">{tr("aipopup.bestVersion")}</div>
        <Textarea
          class="result-area"
          bind:value={result}
          rows={6}
          spellcheck={false}
        />
      </div>

      {#if assistResult.alternatives.length > 0}
        <div class="alternatives">
          <div class="alternatives-title">{tr("aipopup.alternatives")}</div>
          <div class="alternative-list">
            {#each assistResult.alternatives as alternative}
              <button
                class="alternative-card"
                class:selected={result === alternative.text}
                onclick={() => pickAssistText(alternative.text)}
              >
                <span class="alternative-label">{alternative.label}</span>
                <span class="alternative-text">{alternative.text}</span>
              </button>
            {/each}
          </div>
        </div>
      {/if}

      <div class="result-actions">
        <Button size="sm" onclick={acceptResult}>
          {tr("aipopup.accept")}
        </Button>
        <Button size="sm" variant="outline" onclick={clearAssistState}>
          {tr("aipopup.discard")}
        </Button>
      </div>
    </div>
  {/if}

  {#if (activeTool === "tone" && toneResult) || (activeTool === "clarity" && clarityResult) || (activeTool === "vocabulary" && vocabResult)}
    <div class="analysis-layout">
      <div class="analysis-original">
        <div class="analysis-title">{tr("aipopup.analysis.original")}</div>
        <p>{selectedText}</p>
      </div>
      <div class="analysis-panel">
        {#if activeTool === "tone" && toneResult}
          <div class="analysis-title">{tr("aipopup.analysis.toneTitle")}</div>
          <div class="tone-overall">
            {tr("aipopup.analysis.primaryTone")}: {toneResult.overall} ({toneResult.score}%)
          </div>
          <div class="tone-list">
            {#each toneResult.tones as tone}
              <div class="tone-item">
                <span>{tone.name}</span>
                <span>{tone.score}%</span>
              </div>
            {/each}
          </div>
        {/if}

        {#if activeTool === "clarity" && clarityResult}
          <div class="analysis-title">
            {tr("aipopup.analysis.clarityTitle")}
          </div>
          <div class="clarity-score">
            {tr("aipopup.analysis.score")}: {clarityResult.score}/100
          </div>
          <div class="clarity-stats">
            <span
              >{tr("aipopup.analysis.readability")}: {clarityResult.stats
                .readabilityGrade}</span
            >
            <span
              >{tr("aipopup.analysis.avgSentence")}
              {clarityResult.stats.avgSentenceLength}</span
            >
            <span
              >{tr("aipopup.analysis.passiveVoice")}
              {clarityResult.stats.passiveVoiceCount}</span
            >
          </div>
          {#if clarityResult.issues.length === 0}
            <div class="empty-hint">{tr("aipopup.analysis.noIssues")}</div>
          {:else}
            <div class="issue-list">
              {#each clarityResult.issues as issue}
                <div class="issue-item">
                  <div class="issue-type">
                    {getIssueTypeLabel(issue.issueType)}
                  </div>
                  <div class="issue-text">{issue.text}</div>
                  <div class="issue-suggestion">
                    {tr("aipopup.analysis.suggestion")}: {issue.suggestion}
                  </div>
                </div>
              {/each}
            </div>
          {/if}
        {/if}

        {#if activeTool === "vocabulary" && vocabResult}
          <div class="analysis-title">{tr("aipopup.analysis.vocabTitle")}</div>
          {#if vocabResult.suggestions.length === 0}
            <div class="empty-hint">
              {tr("aipopup.analysis.vocabNoSuggestions")}
            </div>
          {:else}
            <div class="issue-list">
              {#each vocabResult.suggestions as item}
                <div class="issue-item">
                  <div class="issue-text">
                    {tr("aipopup.analysis.sourceWord")}: {item.original}
                  </div>
                  {#each item.alternatives as alt}
                    <div class="vocab-option">
                      <span>{alt.word}</span>
                      <span class="vocab-reason">{alt.reason}</span>
                      <button
                        class="mini-apply"
                        onclick={() =>
                          applyVocabularySuggestion(item.original, alt.word)}
                      >
                        {tr("aipopup.analysis.apply")}
                      </button>
                    </div>
                  {/each}
                </div>
              {/each}
            </div>
          {/if}
        {/if}
      </div>
    </div>
    <div class="analysis-actions">
      <Button size="sm" onclick={acceptAnalysisResult}>
        {tr("aipopup.analysis.applyAll")}
      </Button>
    </div>
  {/if}

  {#if result && !loading && !assistResult}
    <div class="result-wrap">
      <Textarea
        class="result-area"
        bind:value={result}
        rows={6}
        spellcheck={false}
      />
      <div class="result-actions">
        <Button size="sm" onclick={acceptResult}>
          {tr("aipopup.accept")}
        </Button>
        <Button
          size="sm"
          variant="outline"
          onclick={() => {
            result = "";
            error = "";
          }}
        >
          {tr("aipopup.discard")}
        </Button>
      </div>
    </div>
  {/if}
</div>

<style>
  :global(body) {
    font-family:
      -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    background: transparent;
    overflow: hidden;
  }

  .popup {
    display: inline-flex;
    flex-direction: column;
    gap: 10px;
    background: var(--popup-surface);
    backdrop-filter: blur(16px);
    -webkit-backdrop-filter: blur(16px);
    border-radius: 12px;
    box-shadow: var(--popup-shadow);
    padding: 10px 12px 12px;
    min-width: 360px;
    max-width: min(720px, 95vw);
    border-bottom: 3px solid var(--popup-ai-border-accent);
  }

  .header {
    display: flex;
    align-items: center;
    gap: 6px;
    cursor: move;
    cursor: all-scroll;
  }

  .header-icon {
    color: var(--popup-ai-icon-accent);
    display: flex;
    align-items: center;
    flex-shrink: 0;
  }

  .title {
    font-size: 13px;
    font-weight: 600;
    color: var(--popup-title);
    flex: 1;
    white-space: nowrap;
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-left: auto;
  }

  .ignore-message {
    font-size: 11px;
    font-weight: 500;
    color: var(--popup-inline-success-fg);
    padding: 2px 8px;
    border-radius: 4px;
    background: var(--popup-inline-success-bg);
    white-space: nowrap;
    animation: fadeIn 0.2s ease;
  }

  .ignore-message.error {
    color: var(--popup-inline-error-fg);
    background: var(--popup-inline-error-bg);
  }

  @keyframes fadeIn {
    from {
      opacity: 0;
      transform: scale(0.95);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }

  .icon-btn {
    width: 26px;
    height: 26px;
    border: none;
    background: transparent;
    border-radius: 6px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--popup-icon);
    transition:
      background 0.12s,
      color 0.12s;
    padding: 0;
  }

  .icon-btn.ignore:hover {
    background: var(--popup-hover-ignore-bg);
    color: var(--popup-hover-ignore-fg);
  }

  .icon-btn.close:hover {
    background: var(--popup-hover-close-bg);
    color: var(--popup-hover-close-fg);
  }

  .preview {
    padding: 10px 12px;
    background: var(--popup-muted-surface);
    border-radius: 8px;
    border: 1px solid var(--popup-border);
  }

  .preview-meta {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin-bottom: 4px;
  }

  .preview-label,
  .char-count,
  .section-subtitle,
  .translate-hint,
  .limit-note {
    font-size: 11px;
    color: var(--popup-muted-label);
  }

  .preview-label {
    text-transform: uppercase;
    letter-spacing: 0.5px;
    font-weight: 600;
  }

  .preview-text {
    margin: 0;
    font-size: 12px;
    color: var(--popup-muted-text);
    line-height: 1.4;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .section-header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 8px;
  }

  .section-title {
    font-size: 12px;
    font-weight: 600;
    color: var(--popup-title);
  }

  .tool-grid {
    display: grid;
    gap: 8px;
  }

  .tool-grid-assist {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .tool-grid-analysis {
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }

  .tool-card {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 4px;
    padding: 10px 12px;
    background: var(--popup-muted-surface);
    border: 1px solid var(--popup-border);
    border-radius: 10px;
    color: var(--popup-muted-text);
    cursor: pointer;
    text-align: left;
    transition: all 0.12s ease;
  }

  .tool-card:hover:not(:disabled) {
    background: var(--popup-ai-hover-bg);
    border-color: var(--popup-ai-hover-border);
    color: var(--popup-ai-hover-fg);
  }

  .tool-card.active {
    background: var(--popup-ai-active-bg);
    border-color: var(--popup-ai-active-border);
    color: var(--popup-ai-active-fg);
  }

  .tool-card:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }

  .tool-card-compact {
    min-height: 96px;
  }

  .tool-icon {
    font-size: 16px;
    line-height: 1;
  }

  .tool-title {
    font-size: 12px;
    font-weight: 600;
  }

  .tool-description {
    font-size: 11px;
    line-height: 1.45;
    color: inherit;
    opacity: 0.82;
  }

  .translate-panel,
  .assist-panel,
  .result-wrap {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 12px;
    border: 1px solid var(--popup-border);
    background: var(--popup-muted-surface);
    border-radius: 10px;
  }

  .translate-meta,
  .assist-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }

  .translate-label,
  .assist-headline,
  .suggestion-label,
  .alternatives-title,
  .analysis-title {
    font-size: 12px;
    font-weight: 600;
    color: var(--popup-title);
  }

  .assist-summary {
    margin-top: 4px;
    font-size: 11px;
    line-height: 1.45;
    color: var(--popup-muted-label);
  }

  .ghost-action {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    border: 1px solid var(--popup-border);
    border-radius: 999px;
    background: var(--popup-surface);
    color: var(--popup-muted-text);
    font-size: 11px;
    padding: 6px 10px;
    cursor: pointer;
  }

  .ghost-action:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .lang-row {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .lang-btn,
  .focus-chip {
    font-size: 11px;
    padding: 4px 10px;
    border-radius: 999px;
    background: var(--popup-surface);
    border: 1px solid var(--popup-border);
    color: var(--popup-muted-label);
    cursor: pointer;
  }

  .lang-btn.selected {
    background: var(--popup-ai-active-bg);
    border-color: var(--popup-ai-active-border);
    color: var(--popup-ai-active-fg);
  }

  .limit-note-error {
    color: var(--popup-inline-error-fg);
  }

  .focus-row {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .focus-chip {
    cursor: default;
  }

  .suggestion-card {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .alternative-list {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px;
  }

  .alternative-card {
    display: flex;
    flex-direction: column;
    gap: 4px;
    border: 1px solid var(--popup-border);
    border-radius: 8px;
    background: var(--popup-surface);
    padding: 10px;
    text-align: left;
    cursor: pointer;
    transition: all 0.12s ease;
  }

  .alternative-card:hover,
  .alternative-card.selected {
    border-color: var(--popup-ai-active-border);
    background: var(--popup-ai-active-bg);
  }

  .alternative-label {
    font-size: 11px;
    font-weight: 600;
    color: var(--popup-title);
  }

  .alternative-text {
    font-size: 11px;
    line-height: 1.45;
    color: var(--popup-muted-text);
    line-clamp: 4;
    display: -webkit-box;
    -webkit-line-clamp: 4;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .loading {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 14px;
    color: var(--popup-muted-label);
    font-size: 12px;
  }

  .spinner {
    width: 14px;
    height: 14px;
    border: 2px solid var(--popup-border);
    border-top-color: var(--popup-ai-spinner-top);
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .error {
    padding: 10px 12px;
    background: var(--popup-error-bg);
    border: 1px solid var(--popup-error-border);
    border-radius: 8px;
    color: var(--popup-error-fg);
    font-size: 12px;
    line-height: 1.45;
  }

  :global(.result-area) {
    width: 100%;
    box-sizing: border-box;
    background: var(--popup-input-bg);
    border: 1px solid var(--popup-input-border);
    border-radius: 8px;
    color: var(--popup-input-fg);
    font-size: 12px;
    line-height: 1.5;
    padding: 10px 12px;
    resize: vertical;
    font-family: inherit;
    outline: none;
  }

  :global(.result-area:focus) {
    border-color: var(--popup-ai-focus-border);
  }

  .result-actions,
  .analysis-actions {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
  }

  .analysis-layout {
    display: grid;
    grid-template-columns: 1fr 1.15fr;
    gap: 10px;
    max-height: 300px;
  }

  .analysis-original,
  .analysis-panel {
    border: 1px solid var(--popup-border);
    background: var(--popup-muted-surface);
    border-radius: 10px;
    padding: 10px;
    overflow: auto;
  }

  .analysis-original p {
    margin: 0;
    white-space: pre-wrap;
    font-size: 12px;
    color: var(--popup-muted-text);
    line-height: 1.45;
  }

  .tone-overall,
  .clarity-score {
    font-size: 12px;
    color: var(--popup-title);
    margin-bottom: 6px;
  }

  .tone-list,
  .issue-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .tone-item,
  .vocab-option {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    font-size: 11px;
    padding: 6px;
    border: 1px solid var(--popup-border);
    border-radius: 5px;
    background: var(--popup-surface);
  }

  .issue-item {
    border: 1px solid var(--popup-border);
    border-radius: 5px;
    background: var(--popup-surface);
    padding: 6px;
    font-size: 11px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .issue-type {
    font-weight: 600;
    color: var(--popup-title);
  }

  .issue-text {
    color: var(--popup-muted-text);
  }

  .issue-suggestion {
    color: var(--popup-inline-success-fg);
  }

  .clarity-stats {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-bottom: 6px;
    font-size: 10px;
    color: var(--popup-muted-label);
  }

  .empty-hint {
    font-size: 11px;
    color: var(--popup-muted-label);
  }

  .vocab-reason {
    font-size: 10px;
    color: var(--popup-muted-label);
  }

  .mini-apply {
    border: 1px solid var(--popup-ai-active-border);
    border-radius: 5px;
    background: var(--popup-ai-active-bg);
    color: var(--popup-ai-active-fg);
    font-size: 10px;
    padding: 2px 6px;
    cursor: pointer;
  }

  @media (max-width: 680px) {
    .popup {
      min-width: 320px;
      max-width: 96vw;
    }

    .tool-grid-assist,
    .tool-grid-analysis,
    .alternative-list,
    .analysis-layout {
      grid-template-columns: 1fr;
    }
  }
</style>
