<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { Ban, Sparkles, X } from "lucide-svelte";
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
  import type { ThemeMode } from "$lib/types/theme";
  import {
    applyThemeToDom,
    isThemeMode,
    loadThemeFromLocalStorage,
    saveThemeToLocalStorage,
  } from "$lib/theme";
  $locale;

  // Reactive translation helper
  const tr = $derived(
    (key: string, params?: Record<string, string | number>) => {
      const _ = $locale;
      return t(key, params);
    },
  );

  type Tool = "translate" | "polish" | "improve" | "summarize";

  // Polish styles
  type PolishStyle = "formal" | "conversational" | "academic" | "business";

  // Mirror the AppConfig shape used by SpellChecker / SettingsPanel.
  interface AppConfig {
    aiGrammarEnabled?: boolean;
    aiTranslateTargetLanguage?: string;
    aiPolishStyle?: string;
  }

  // Result from batch polish
  interface PolishedResult {
    style: PolishStyle;
    output_text: string;
  }

  let selectedText = $state("");
  let result = $state("");
  let loading = $state(false);
  let error = $state("");
  let activeTool = $state<Tool | null>(null);
  let translateLang = $state("English");
  let ignoreMessage = $state("");
  let ignoreError = $state(false);

  // Polish styles
  let selectedStyles = $state<PolishStyle[]>([]);
  let batchResults = $state<PolishedResult[]>([]);
  let selectedResultIndex = $state<number | null>(null);

  // Source app info (captured when popup shows)
  let sourceAppName = $state("");
  let sourceBundleId = $state("");
  let theme: ThemeMode = $state("auto");
  let mediaQuery: MediaQueryList | null = null;

  async function loadThemeFromStore(): Promise<ThemeMode> {
    try {
      const stored = await invoke<string>("get_theme");
      if (isThemeMode(stored)) {
        return stored;
      }
    } catch (error) {
      console.warn(
        "Failed to load AI popup theme from store, fallback to localStorage:",
        error,
      );
    }
    return loadThemeFromLocalStorage();
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

  const tools: { id: Tool; label: () => string; icon: string }[] = [
    { id: "translate", label: () => tr("aipopup.translate"), icon: "🌐" },
    { id: "polish", label: () => tr("aipopup.polish"), icon: "✨" },
    { id: "improve", label: () => tr("aipopup.improve"), icon: "📝" },
    { id: "summarize", label: () => tr("aipopup.summarize"), icon: "📋" },
  ];

  const languages = [
    "English",
    "Chinese",
    "Japanese",
    "Spanish",
    "French",
    "German",
    "Korean",
  ];

  function getStyleLabel(style: PolishStyle): string {
    const styleLabelMap: Record<PolishStyle, () => string> = {
      formal: () => tr("aipopup.styleFormal"),
      conversational: () => tr("aipopup.styleConversational"),
      academic: () => tr("aipopup.styleAcademic"),
      business: () => tr("aipopup.styleBusiness"),
    };
    return styleLabelMap[style]();
  }
  const polishStyles: { id: PolishStyle; label: () => string }[] = [
    { id: "formal", label: () => tr("aipopup.styleFormal") },
    { id: "conversational", label: () => tr("aipopup.styleConversational") },
    { id: "academic", label: () => tr("aipopup.styleAcademic") },
    { id: "business", label: () => tr("aipopup.styleBusiness") },
  ];

  // All polish styles selected by default
  function selectAllStyles() {
    selectedStyles = polishStyles.map((s) => s.id);
  }

  function toggleStyle(style: PolishStyle) {
    if (selectedStyles.includes(style)) {
      selectedStyles = selectedStyles.filter((s) => s !== style);
    } else {
      selectedStyles = [...selectedStyles, style];
    }
  }

  async function runBatchPolish() {
    if (!selectedText.trim() || selectedStyles.length === 0) return;

    loading = true;
    error = "";
    batchResults = [];
    selectedResultIndex = null;

    try {
      const config = await invoke<AppConfig>("get_config");
      if (!config.aiGrammarEnabled) {
        throw new Error(tr("aipopup.aiEnableHint"));
      }

      const response = await invoke<{ results: PolishedResult[] }>(
        "ai_polish_batch",
        {
          request: {
            text: selectedText,
            styles: selectedStyles,
          },
        },
      );

      batchResults = response.results;
      // Auto-select first result if available
      if (batchResults.length > 0 && batchResults[0].output_text) {
        selectedResultIndex = 0;
        result = batchResults[0].output_text;
      }
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  function selectResult(index: number) {
    selectedResultIndex = index;
    if (batchResults[index]) {
      result = batchResults[index].output_text;
    }
  }

  async function runTool(tool: Tool) {
    if (!selectedText.trim()) return;

    // For polish tool, show style selector first
    if (tool === "polish") {
      activeTool = tool;
      // Reset batch state
      batchResults = [];
      selectedResultIndex = null;
      result = "";
      error = "";
      // Select all styles by default
      selectAllStyles();
      return;
    }

    // For other tools, run immediately
    activeTool = tool;
    loading = true;
    error = "";
    result = "";

    try {
      // Load config fresh each time — same pattern as SpellChecker.svelte.
      const config = await invoke<AppConfig>("get_config");
      if (!config.aiGrammarEnabled) {
        throw new Error(tr("aipopup.aiEnableHint"));
      }

      const polishStyleMap: Record<string, string> = {
        improve: "clear, natural, and engaging",
        summarize: "summarize concisely in the same language as the input",
      };

      await invoke("ai_text_transform_stream", {
        request: {
          text: selectedText,
          operation:
            tool === "improve" || tool === "summarize" ? "polish" : tool,
          targetLanguage: tool === "translate" ? translateLang : null,
          polishStyle: polishStyleMap[tool] ?? config.aiPolishStyle ?? null,
        },
      });
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  async function acceptResult() {
    if (!result.trim()) return;
    await invoke("accept_ai_result", { text: result });
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
    } catch (error) {
      console.error("Failed to ignore app:", error);
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
        target.closest("button, input, textarea, select, a, [role='button']"),
      )
    );
  }

  async function startWindowDrag(event: MouseEvent) {
    if (event.button !== 0 || isInteractiveTarget(event.target)) return;
    try {
      await getCurrentWindow().startDragging();
    } catch (error) {
      console.error("Failed to start AI popup drag:", error);
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

    // Get initial ai popup state (contains source app info)
    (async () => {
      try {
        const state = await invoke<{
          sourceAppName?: string;
          sourceBundleId?: string;
        }>("get_ai_popup_state");
        sourceAppName = state.sourceAppName || "";
        sourceBundleId = state.sourceBundleId || "";
      } catch (e) {
        console.error("Failed to get ai popup state:", e);
      }
    })();

    const unlistenPromise = listen<{ selectedText: string }>(
      "ai-popup-show",
      async (e) => {
        selectedText = e.payload.selectedText;
        result = "";
        error = "";
        activeTool = null;
        loading = false;

        // Refresh source app info when popup shows
        try {
          const state = await invoke<{
            sourceAppName?: string;
            sourceBundleId?: string;
          }>("get_ai_popup_state");
          sourceAppName = state.sourceAppName || "";
          sourceBundleId = state.sourceBundleId || "";
        } catch (e) {
          console.error("Failed to get ai popup state:", e);
        }
      },
    );

    const unlistenChunkPromise = listen<string>("ai-stream-chunk", (event) => {
      if (!loading || !activeTool) {
        return;
      }
      result += event.payload;
    });

    const unlistenCompletePromise = listen("ai-stream-complete", () => {
      if (!activeTool) {
        return;
      }
      loading = false;
    });

    const unlistenErrorPromise = listen<string>("ai-stream-error", (event) => {
      if (!activeTool) {
        return;
      }
      error = event.payload;
      loading = false;
    });

    invoke<{ selectedText: string }>("get_ai_popup_state")
      .then((state) => {
        if (state.selectedText) selectedText = state.selectedText;
      })
      .catch(() => {});

    return () => {
      unlistenThemePromise.then((fn) => fn());
      unlistenPromise.then((fn) => fn());
      unlistenChunkPromise.then((fn) => fn());
      unlistenCompletePromise.then((fn) => fn());
      unlistenErrorPromise.then((fn) => fn());
      cleanupThemeListener();
    };
  });

  const preview = $derived(
    selectedText.length > 120 ? selectedText.slice(0, 120) + "…" : selectedText,
  );
</script>

<svelte:window
  onkeydown={(e) => {
    if (e.key === "Escape") close();
  }}
/>

<div class="popup" data-locale={$locale}>
  <!-- Header -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="header" onmousedown={startWindowDrag}>
    <span class="header-icon">
      <Sparkles class="h-3.5 w-3.5" />
    </span>
    <span class="title">{tr("aipopup.tools")}</span>
    <div class="header-actions">
      {#if ignoreMessage}
        <span class="ignore-message" class:error={ignoreError}
          >{ignoreMessage}</span
        >
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

  <!-- Selected text preview -->
  {#if selectedText}
    <div class="preview">
      <span class="preview-label">{tr("aipopup.selected")}</span>
      <p class="preview-text">{preview}</p>
    </div>
  {/if}

  <!-- Tool buttons -->
  <div class="tools">
    {#each tools as tool}
      <button
        class="tool-btn"
        class:active={activeTool === tool.id}
        onclick={() => runTool(tool.id)}
        disabled={loading}
      >
        <span class="tool-icon">{tool.icon}</span>
        <span class="tool-label">{tool.label()}</span>
      </button>
    {/each}
  </div>

  <!-- Language selector (only for translate) -->
  {#if activeTool === "translate" || (!activeTool && false)}
    <div class="lang-row">
      <span class="lang-label">{tr("aipopup.into")}</span>
      {#each languages as lang}
        <button
          class="lang-btn"
          class:selected={translateLang === lang}
          onclick={() => {
            translateLang = lang;
            runTool("translate");
          }}
        >
          {lang}
        </button>
      {/each}
    </div>
  {/if}

  <!-- Style selector (only for polish) -->
  {#if activeTool === "polish" && !loading && batchResults.length === 0}
    <div class="style-selector">
      <div class="style-header">
        <span class="style-label">{tr("aipopup.styles")}</span>
      </div>
      <div class="style-buttons">
        {#each polishStyles as style}
          <button
            class="style-btn"
            class:selected={selectedStyles.includes(style.id)}
            onclick={() => toggleStyle(style.id)}
          >
            {style.label()}
          </button>
        {/each}
      </div>
      <Button
        size="sm"
        class="generate-btn"
        onclick={runBatchPolish}
        disabled={selectedStyles.length === 0}
      >
        {tr("aipopup.generateAll")}
      </Button>
    </div>
  {/if}

  <!-- Batch results (for polish) -->
  {#if batchResults.length > 0}
    <div class="batch-results">
      <div class="results-header">
        <span class="results-label">{tr("aipopup.selectResult")}</span>
      </div>
      <div class="results-list">
        {#each batchResults as res, i}
          <button
            class="result-card"
            class:selected={selectedResultIndex === i}
            class:empty={!res.output_text}
            onclick={() => selectResult(i)}
            disabled={!res.output_text}
          >
            <span class="result-style">{getStyleLabel(res.style)}</span>
            <span class="result-preview">{res.output_text.slice(0, 80)}{res.output_text.length > 80 ? "…" : ""}</span>
          </button>
        {/each}
      </div>
    </div>
  {/if}

  <!-- Loading -->
  {#if loading}
    <div class="loading">
      <div class="spinner"></div>
      <span>{tr("aipopup.generating")}</span>
    </div>
  {/if}

  <!-- Error -->
  {#if error}
    <div class="error">{error}</div>
  {/if}

  <!-- Result -->
  {#if result && !loading}
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
            activeTool = null;
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
    gap: 8px;
    background: var(--popup-surface);
    backdrop-filter: blur(16px);
    -webkit-backdrop-filter: blur(16px);
    border-radius: 10px;
    box-shadow: var(--popup-shadow);
    padding: 8px 10px 10px;
    min-width: 280px;
    max-width: 100vw;
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
    padding: 8px 10px;
    background: var(--popup-muted-surface);
    border-radius: 6px;
  }

  .preview-label {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--popup-muted-label);
    display: block;
    margin-bottom: 3px;
  }

  .preview-text {
    margin: 0;
    font-size: 12px;
    color: var(--popup-muted-text);
    line-height: 1.4;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .tools {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 6px;
  }

  .tool-btn {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 3px;
    padding: 8px 4px;
    background: var(--popup-muted-surface);
    border: 1px solid var(--popup-border);
    border-radius: 6px;
    color: var(--popup-muted-text);
    cursor: pointer;
    font-size: 11px;
    transition: all 0.12s ease;
  }

  .tool-btn:hover:not(:disabled) {
    background: var(--popup-ai-hover-bg);
    border-color: var(--popup-ai-hover-border);
    color: var(--popup-ai-hover-fg);
  }

  .tool-btn.active {
    background: var(--popup-ai-active-bg);
    border-color: var(--popup-ai-active-border);
    color: var(--popup-ai-active-fg);
  }

  .tool-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .tool-icon {
    font-size: 14px;
  }

  .tool-label {
    font-size: 10px;
    font-weight: 500;
  }

  .lang-row {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    padding: 0 4px;
    align-items: center;
  }

  .lang-label {
    font-size: 11px;
    color: var(--popup-muted-label);
  }

  .lang-btn {
    font-size: 11px;
    padding: 2px 8px;
    border-radius: 10px;
    background: var(--popup-muted-surface);
    border: 1px solid var(--popup-border);
    color: var(--popup-muted-label);
    cursor: pointer;
  }

  .lang-btn.selected {
    background: var(--popup-ai-active-bg);
    border-color: var(--popup-ai-active-border);
    color: var(--popup-ai-active-fg);
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
    margin: 0 4px;
    padding: 8px 10px;
    background: var(--popup-error-bg);
    border: 1px solid var(--popup-error-border);
    border-radius: 6px;
    color: var(--popup-error-fg);
    font-size: 12px;
    line-height: 1.4;
  }

  .result-wrap {
    padding: 0 4px;
  }

  :global(.result-area) {
    width: 100%;
    box-sizing: border-box;
    background: var(--popup-input-bg);
    border: 1px solid var(--popup-input-border);
    border-radius: 6px;
    color: var(--popup-input-fg);
    font-size: 12px;
    line-height: 1.5;
    padding: 8px 10px;
    resize: vertical;
    font-family: inherit;
    outline: none;
  }

  :global(.result-area:focus) {
    border-color: var(--popup-ai-focus-border);
  }

  .result-actions {
    display: flex;
    gap: 8px;
    margin-top: 8px;
  }

  /* Style selector styles */
  .style-selector {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 8px;
    background: var(--popup-muted-surface);
    border-radius: 6px;
  }

  .style-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .style-label {
    font-size: 11px;
    font-weight: 500;
    color: var(--popup-muted-label);
  }

  .style-buttons {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 6px;
  }

  .style-btn {
    padding: 6px 8px;
    background: var(--popup-surface);
    border: 1px solid var(--popup-border);
    border-radius: 6px;
    color: var(--popup-muted-text);
    font-size: 11px;
    cursor: pointer;
    transition: all 0.12s ease;
  }

  .style-btn:hover {
    background: var(--popup-ai-hover-bg);
    border-color: var(--popup-ai-hover-border);
  }

  .style-btn.selected {
    background: var(--popup-ai-active-bg);
    border-color: var(--popup-ai-active-border);
    color: var(--popup-ai-active-fg);
  }

  :global(.generate-btn) {
    width: 100%;
    margin-top: 4px;
  }

  /* Batch results styles */
  .batch-results {
    display: flex;
    flex-direction: column;
    gap: 6px;
    max-height: 200px;
    overflow-y: auto;
  }

  .results-header {
    font-size: 11px;
    font-weight: 500;
    color: var(--popup-muted-label);
  }

  .results-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .result-card {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 8px;
    background: var(--popup-surface);
    border: 1px solid var(--popup-border);
    border-radius: 6px;
    cursor: pointer;
    text-align: left;
    transition: all 0.12s ease;
  }

  .result-card:hover:not(:disabled) {
    background: var(--popup-ai-hover-bg);
    border-color: var(--popup-ai-hover-border);
  }

  .result-card.selected {
    background: var(--popup-ai-active-bg);
    border-color: var(--popup-ai-active-border);
  }

  .result-card.empty {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .result-style {
    font-size: 11px;
    font-weight: 600;
    color: var(--popup-title);
  }

  .result-preview {
    font-size: 11px;
    color: var(--popup-muted-text);
    line-height: 1.3;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
