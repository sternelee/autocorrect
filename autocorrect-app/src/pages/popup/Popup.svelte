<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import {
    Tooltip,
    TooltipTrigger,
    TooltipContent,
    TooltipProvider,
  } from "$lib/components/ui/tooltip";
  import { locale, t } from "$lib/i18n";
  $locale;

  // Reactive translation helper
  const tr = $derived((key: string, params?: Record<string, string | number>) => {
    const _ = $locale;
    return t(key, params);
  });

  interface TypoSuggestion {
    typo: string;
    suggestions: string[];
    line: number;
    col: number;
  }

  interface PopupData {
    originalText: string;
    suggestion: string;
    x: number;
    y: number;
    typos?: TypoSuggestion[];
    offset?: number;
    charLength?: number;
  }

  let originalText = $state("");
  let suggestion = $state("");
  let typos = $state<TypoSuggestion[]>([]);
  let offset = $state<number | null>(null);
  let charLength = $state<number | null>(null);
  let addedToCustom = $state(false);

  // For a typo-only popup triggered by hover, we surface the first typo's suggestions as chips.
  // For a full spell-check popup, we show the whole corrected suggestion.
  const chips = $derived(
    typos.length > 0
      ? typos[0].suggestions.slice(0, 4)
      : suggestion && suggestion !== originalText
        ? [suggestion]
        : [],
  );

  const title = $derived(
    typos.length > 0 ? tr("popup.correct") : tr("popup.title"),
  );

  // The word/phrase pair to save as a custom correction:
  // typo popup → typos[0].typo → first chip; general → originalText → suggestion.
  const customPair = $derived(
    typos.length > 0 && typos[0].suggestions.length > 0
      ? { from: typos[0].typo, to: typos[0].suggestions[0] }
      : originalText && suggestion && originalText !== suggestion
        ? { from: originalText, to: suggestion }
        : null,
  );

  onMount(() => {
    const unlistenShow = listen<PopupData>("popup-show", (event) => {
      const data = event.payload;
      originalText = data.originalText;
      suggestion = data.suggestion;
      typos = data.typos || [];
      offset = data.offset ?? null;
      charLength = data.charLength ?? null;
    });

    const unlistenHide = listen("popup-hide", () => {
      hidePopup();
    });

    return () => {
      unlistenShow.then((fn) => fn());
      unlistenHide.then((fn) => fn());
    };
  });

  async function accept(text?: string) {
    const textToUse = text ?? suggestion;
    if (!textToUse.trim()) return;
    try {
      await invoke("accept_suggestion", {
        text: textToUse,
        offset,
        charLength,
      });
    } catch (error) {
      console.error("Failed to accept suggestion:", error);
    }
  }

  async function reject() {
    try {
      await invoke("reject_suggestion");
    } catch (error) {
      console.error("Failed to reject suggestion:", error);
      hidePopup();
    }
  }

  function hidePopup() {
    getCurrentWindow().hide();
    originalText = "";
    suggestion = "";
    typos = [];
    addedToCustom = false;
  }

  async function addToCustom() {
    if (!customPair) return;
    try {
      await invoke("add_custom_correction", {
        typo: customPair.from,
        correction: customPair.to,
      });
      addedToCustom = true;
    } catch (error) {
      console.error("Failed to add custom correction:", error);
    }
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      accept();
    } else if (e.key === "Escape") {
      e.preventDefault();
      reject();
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

<div class="popup" data-locale={$locale}>
  <div class="header">
    <span class="header-icon">
      <!-- pencil icon -->
      <svg
        width="14"
        height="14"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2.2"
        stroke-linecap="round"
        stroke-linejoin="round"
      >
        <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7" />
        <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z" />
      </svg>
    </span>
    <span class="title">{title}</span>
    <div class="header-actions">
      <button class="icon-btn close" title={tr("popup.close")} onclick={reject}>
        <svg
          width="13"
          height="13"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2.5"
          stroke-linecap="round"
        >
          <line x1="18" y1="6" x2="6" y2="18" /><line
            x1="6"
            y1="6"
            x2="18"
            y2="18"
          />
        </svg>
      </button>
    </div>
  </div>

  {#if chips.length > 0}
    <div class="chips">
      {#each chips as chip}
        <button class="chip" onclick={() => accept(chip)}>{chip}</button>
      {/each}
      {#if customPair}
        <TooltipProvider>
          <Tooltip>
            <TooltipTrigger>
              <button
                class="icon-btn add-custom"
                class:added={addedToCustom}
                disabled={addedToCustom}
                onclick={addToCustom}
              >
                {#if addedToCustom}
                  <svg
                    width="13"
                    height="13"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2.5"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                  >
                    <polyline points="20 6 9 17 4 12" />
                  </svg>
                {:else}
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
                    <path
                      d="M19 21l-7-5-7 5V5a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2z"
                    />
                    <line x1="12" y1="8" x2="12" y2="14" />
                    <line x1="9" y1="11" x2="15" y2="11" />
                  </svg>
                {/if}
              </button>
            </TooltipTrigger>
            <TooltipContent side="top">
              {addedToCustom ? tr("popup.saved") : tr("popup.saveDict")}
            </TooltipContent>
          </Tooltip>
        </TooltipProvider>
      {/if}
    </div>
  {/if}
</div>

<style>
  :global(body) {
    font-family:
      -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    background: transparent;
    overflow: hidden;
    -webkit-app-region: drag;
  }

  .popup *,
  .popup *::before,
  .popup *::after {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
  }

  .popup {
    display: inline-flex;
    flex-direction: column;
    gap: 8px;
    background: rgba(255, 255, 255, 0.97);
    backdrop-filter: blur(16px);
    -webkit-backdrop-filter: blur(16px);
    border-radius: 10px;
    box-shadow:
      0 4px 20px rgba(0, 0, 0, 0.14),
      0 0 0 1px rgba(0, 0, 0, 0.06);
    padding: 8px 10px 10px;
    min-width: 180px;
    max-width: 100vw;
    border-bottom: 3px solid #f59e0b;
  }

  .header {
    display: flex;
    align-items: center;
    gap: 6px;
    -webkit-app-region: drag;
  }

  .header-icon {
    color: #dc2626;
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
    gap: 2px;
    -webkit-app-region: no-drag;
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
    transition:
      background 0.12s,
      color 0.12s;
    padding: 0;
    -webkit-app-region: no-drag;
  }

  .icon-btn.close:hover {
    background: #fee2e2;
    color: #dc2626;
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    -webkit-app-region: no-drag;
  }

  .chip {
    padding: 3px 10px;
    border: 1.5px solid #d1d5db;
    background: #f9fafb;
    border-radius: 6px;
    font-size: 13px;
    font-weight: 500;
    color: #374151;
    cursor: pointer;
    transition: all 0.12s ease;
    -webkit-app-region: no-drag;
  }

  .chip:hover {
    border-color: #16a34a;
    background: #f0fdf4;
    color: #16a34a;
  }

  .chip:active {
    background: #dcfce7;
  }

  .add-custom {
    color: #9ca3af;
    flex-shrink: 0;
    align-self: center;
  }

  .add-custom:hover:notr(:disabled) {
    background: #f3f4f6;
    color: #6b7280;
  }

  .add-custom.added {
    color: #16a34a;
    cursor: default;
  }
</style>
