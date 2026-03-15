<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";
  import { locale, t } from "$lib/i18n";
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

  async function acceptResultr() {
    if (!result.trim()) return;
    await invoke("accept_ai_result", { text: result });
  }

  async function close() {
    await invoke("hide_ai_popup");
  }

  onMount(() => {
    const unlistenPromise = listen<{ selectedText: string }>(
      "ai-popup-show",
      (e) => {
        selectedText = e.payload.selectedText;
        result = "";
        error = "";
        activeTool = null;
        loading = false;
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

<div class="popup" data-locale={$locale}>
  <!-- Header -->
  <div class="header">
    <div class="header-left">
      <span class="ai-badge">{tr("aipopup.tools")}</span>
    </div>
    <button class="close-btn" onclick={close} aria-label={tr("aipopup.close")}
      >✕</button
    >
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
        <span>{tool.icon}</span>
        <span>{tool.label()}</span>
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
      <textarea
        class="result-area"
        bind:value={result}
        rows={6}
        spellcheck={false}
      ></textarea>
      <div class="result-actions">
        <button class="accept-btn" onclick={acceptResult}>
          {tr("aipopup.accept")}
        </button>
        <button
          class="discard-btn"
          onclick={() => {
            result = "";
            activeTool = null;
          }}
        >
          {tr("aipopup.discard")}
        </button>
      </div>
    </div>
  {/if}
</div>

<style>
  :global(html, body) {
    margin: 0;
    padding: 0;
    background: transparent;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
    font-size: 13px;
  }

  .popup {
    width: 380px;
    background: rgba(28, 28, 32, 0.96);
    border-radius: 14px;
    border: 1px solid rgba(255, 255, 255, 0.1);
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
    color: #e5e7eb;
    overflow: hidden;
    box-shadow:
      0 8px 32px rgba(0, 0, 0, 0.5),
      0 0 0 0.5px rgba(255, 255, 255, 0.08);
    animation: slide-in 0.18s cubic-bezier(0.34, 1.56, 0.64, 1);
  }

  @keyframes slide-in {
    from {
      transform: translateY(-6px) scale(0.97);
      opacity: 0;
    }
    to {
      transform: translateY(0) scale(1);
      opacity: 1;
    }
  }

  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 14px 8px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.07);
  }

  .ai-badge {
    font-size: 12px;
    font-weight: 600;
    color: #a78bfa;
    letter-spacing: 0.3px;
  }

  .close-btn {
    background: none;
    border: none;
    color: #6b7280;
    font-size: 13px;
    cursor: pointer;
    padding: 2px 4px;
    border-radius: 4px;
    line-height: 1;
  }
  .close-btn:hover {
    color: #e5e7eb;
    background: rgba(255, 255, 255, 0.08);
  }

  .preview {
    padding: 8px 14px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
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
    color: #9ca3af;
    line-height: 1.4;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .tools {
    display: grid;
    grid-template-columns: repeatr(4, 1fr);
    gap: 6px;
    padding: 10px 14px;
  }

  .tool-btn {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 3px;
    padding: 8px 4px;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 8px;
    color: #d1d5db;
    cursor: pointer;
    font-size: 11px;
    transition: all 0.12s ease;
  }
  .tool-btn:hover:notr(:disabled) {
    background: rgba(167, 139, 250, 0.15);
    border-color: rgba(167, 139, 250, 0.3);
    color: #c4b5fd;
  }
  .tool-btn.active {
    background: rgba(167, 139, 250, 0.2);
    border-color: rgba(167, 139, 250, 0.5);
    color: #c4b5fd;
  }
  .tool-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .lang-row {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    padding: 0 14px 10px;
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
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.1);
    color: #9ca3af;
    cursor: pointer;
  }
  .lang-btn.selected {
    background: rgba(167, 139, 250, 0.2);
    border-color: rgba(167, 139, 250, 0.4);
    color: #c4b5fd;
  }

  .loading {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 14px;
    color: #9ca3af;
    font-size: 12px;
  }
  .spinner {
    width: 14px;
    height: 14px;
    border: 2px solid rgba(167, 139, 250, 0.2);
    border-top-color: #a78bfa;
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .error {
    margin: 0 14px 10px;
    padding: 8px 10px;
    background: rgba(239, 68, 68, 0.15);
    border: 1px solid rgba(239, 68, 68, 0.3);
    border-radius: 6px;
    color: #fca5a5;
    font-size: 12px;
    line-height: 1.4;
  }

  .result-wrap {
    padding: 0 14px 14px;
  }
  .result-area {
    width: 100%;
    box-sizing: border-box;
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 8px;
    color: #e5e7eb;
    font-size: 12px;
    line-height: 1.5;
    padding: 8px 10px;
    resize: vertical;
    font-family: inherit;
    outline: none;
  }
  .result-area:focus {
    border-color: rgba(167, 139, 250, 0.4);
  }

  .result-actions {
    display: flex;
    gap: 8px;
    margin-top: 8px;
  }
  .accept-btn {
    flex: 1;
    padding: 7px 0;
    background: #7c3aed;
    border: none;
    border-radius: 8px;
    color: white;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.12s ease;
  }
  .accept-btn:hover {
    background: #6d28d9;
  }
  .discard-btn {
    padding: 7px 14px;
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 8px;
    color: #9ca3af;
    font-size: 12px;
    cursor: pointer;
  }
  .discard-btn:hover {
    background: rgba(255, 255, 255, 0.1);
    color: #d1d5db;
  }
</style>
