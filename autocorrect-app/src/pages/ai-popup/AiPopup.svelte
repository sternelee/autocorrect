<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";
  import { locale, t } from "$lib/i18n";
  import {
    Tooltip,
    TooltipTrigger,
    TooltipContent,
    TooltipProvider,
  } from "$lib/components/ui/tooltip";
  import { Button } from "$lib/components/ui/button";
  import { Input } from "$lib/components/ui/input";
  import { Textarea } from "$lib/components/ui/textarea";
  $locale;

  // Reactive translation helper
  const tr = $derived((key: string, params?: Record<string, string | number>) => {
    const _ = $locale;
    return t(key, params);
  });

  type Tool = "translate" | "polish" | "improve" | "summarize";

  // Mirror the AppConfig shape used by SpellChecker / SettingsPanel.
  interface AppConfig {
    aiGrammarEnabled?: boolean;
    aiTranslateTargetLanguage?: string;
    aiPolishStyle?: string;
  }

  let selectedText = $state("");
  let result = $state("");
  let loading = $state(false);
  let error = $state("");
  let activeTool = $state<Tool | null>(null);
  let translateLang = $state("English");
  let ignoreMessage = $state("");
  let ignoreError = $state(false);

  // Source app info (captured when popup shows)
  let sourceAppName = $state("");
  let sourceBundleId = $state("");

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

  async function runTool(tool: Tool) {
    if (!selectedText.trim()) return;

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

      const res = await invoke<{ outputText?: string }>("ai_text_transform", {
        request: {
          text: selectedText,
          operation:
            tool === "improve" || tool === "summarize" ? "polish" : tool,
          targetLanguage: tool === "translate" ? translateLang : null,
          polishStyle: polishStyleMap[tool] ?? config.aiPolishStyle ?? null,
        },
      });
      result = res.outputText ?? "";
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

  onMount(() => {
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

    invoke<{ selectedText: string }>("get_ai_popup_state")
      .then((state) => {
        if (state.selectedText) selectedText = state.selectedText;
      })
      .catch(() => {});

    return () => {
      unlistenPromise.then((fn) => fn());
    };
  });

  const preview = $derived(
    selectedText.length > 120 ? selectedText.slice(0, 120) + "…" : selectedText,
  );
</script>

<svelte:window onkeydown={(e) => {
  if (e.key === "Escape") close();
}} />

<div class="popup" data-locale={$locale}>
  <!-- Header -->
  <div class="header">
    <span class="header-icon">
      <!-- magic wand icon -->
      <svg
        width="14"
        height="14"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
      >
        <path d="M21.5 2v6h-6M21.34 2.5a8 8 0 1 1-.58 4.9" />
      </svg>
    </span>
    <span class="title">{tr("aipopup.tools")}</span>
    <div class="header-actions">
      {#if ignoreMessage}
        <span class="ignore-message" class:error={ignoreError}>{ignoreMessage}</span>
      {/if}
      <TooltipProvider>
        <Tooltip>
          <TooltipTrigger>
            <button class="icon-btn ignore" onclick={ignoreApp}>
              <svg
                width="13"
                height="13"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
              >
                <circle cx="12" cy="12" r="10" />
                <line x1="4.93" y1="4.93" x2="19.07" y2="19.07" />
              </svg>
            </button>
          </TooltipTrigger>
          <TooltipContent side="top">{tr("popup.ignoreTooltip")}</TooltipContent>
        </Tooltip>
      </TooltipProvider>
      <button class="icon-btn close" title={tr("aipopup.close")} onclick={close}>
        <svg
          width="13"
          height="13"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2.5"
          stroke-linecap="round"
        >
          <line x1="18" y1="6" x2="6" y2="18" /><line x1="6" y1="6" x2="18" y2="18" />
        </svg>
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
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    background: transparent;
    overflow: hidden;
  }

  .popup {
    display: inline-flex;
    flex-direction: column;
    gap: 8px;
    background: rgba(255, 255, 255, 0.97);
    backdrop-filter: blur(16px);
    -webkit-backdrop-filter: blur(16px);
    border-radius: 10px;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.14), 0 0 0 1px rgba(0, 0, 0, 0.06);
    padding: 8px 10px 10px;
    min-width: 280px;
    max-width: 100vw;
    border-bottom: 3px solid #7c3aed;
  }

  .header {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .header-icon {
    color: #7c3aed;
    display: flex;
    align-items: center;
    flex-shrink: 0;
  }

  .title {
    font-size: 13px;
    font-weight: 600;
    color: #111827;
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
    color: #059669;
    padding: 2px 8px;
    border-radius: 4px;
    background: rgba(5, 150, 105, 0.1);
    white-space: nowrap;
    animation: fadeIn 0.2s ease;
  }

  .ignore-message.error {
    color: #dc2626;
    background: rgba(220, 38, 38, 0.1);
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
    color: #6b7280;
    transition: background 0.12s, color 0.12s;
    padding: 0;
  }

  .icon-btn.ignore:hover {
    background: #fef3c7;
    color: #f59e0b;
  }

  .icon-btn.close:hover {
    background: #fee2e2;
    color: #dc2626;
  }

  .preview {
    padding: 8px 10px;
    background: #f9fafb;
    border-radius: 6px;
  }

  .preview-label {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: #6b7280;
    display: block;
    margin-bottom: 3px;
  }

  .preview-text {
    margin: 0;
    font-size: 12px;
    color: #374151;
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
    background: #f9fafb;
    border: 1px solid #e5e7eb;
    border-radius: 6px;
    color: #374151;
    cursor: pointer;
    font-size: 11px;
    transition: all 0.12s ease;
  }

  .tool-btn:hover:not(:disabled) {
    background: #f3f4f6;
    border-color: #7c3aed;
    color: #7c3aed;
  }

  .tool-btn.active {
    background: #ede9fe;
    border-color: #7c3aed;
    color: #7c3aed;
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
    color: #6b7280;
  }

  .lang-btn {
    font-size: 11px;
    padding: 2px 8px;
    border-radius: 10px;
    background: #f9fafb;
    border: 1px solid #e5e7eb;
    color: #6b7280;
    cursor: pointer;
  }

  .lang-btn.selected {
    background: #ede9fe;
    border-color: #7c3aed;
    color: #7c3aed;
  }

  .loading {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 14px;
    color: #6b7280;
    font-size: 12px;
  }

  .spinner {
    width: 14px;
    height: 14px;
    border: 2px solid #e5e7eb;
    border-top-color: #7c3aed;
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
    background: #fef2f2;
    border: 1px solid #fecaca;
    border-radius: 6px;
    color: #dc2626;
    font-size: 12px;
    line-height: 1.4;
  }

  .result-wrap {
    padding: 0 4px;
  }

  :global(.result-area) {
    width: 100%;
    box-sizing: border-box;
    background: #f9fafb;
    border: 1px solid #e5e7eb;
    border-radius: 6px;
    color: #111827;
    font-size: 12px;
    line-height: 1.5;
    padding: 8px 10px;
    resize: vertical;
    font-family: inherit;
    outline: none;
  }

  :global(.result-area:focus) {
    border-color: #7c3aed;
  }

  .result-actions {
    display: flex;
    gap: 8px;
    margin-top: 8px;
  }
</style>
