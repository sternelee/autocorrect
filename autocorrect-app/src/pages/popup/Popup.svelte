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
  import type { ThemeMode } from "$lib/types/theme";
  import {
    applyThemeToDom,
    isThemeMode,
    loadThemeFromLocalStorage,
    saveThemeToLocalStorage,
  } from "$lib/theme";
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
  let ignoreMessage = $state("");
  let ignoreError = $state(false);

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
      console.warn("Failed to load popup theme from store, fallback to localStorage:", error);
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

    // Get initial popup state (contains source app info)
    (async () => {
      try {
        const state = await invoke<{
          sourceAppName?: string;
          sourceBundleId?: string;
        }>("get_popup_state");
        sourceAppName = state.sourceAppName || "";
        sourceBundleId = state.sourceBundleId || "";
      } catch (e) {
        console.error("Failed to get popup state:", e);
      }
    })();

    const unlistenShowPromise = listen<PopupData>("popup-show", async (event) => {
      const data = event.payload;
      originalText = data.originalText;
      suggestion = data.suggestion;
      typos = data.typos || [];
      offset = data.offset ?? null;
      charLength = data.charLength ?? null;

      // Refresh source app info when popup shows
      try {
        const state = await invoke<{
          sourceAppName?: string;
          sourceBundleId?: string;
        }>("get_popup_state");
        sourceAppName = state.sourceAppName || "";
        sourceBundleId = state.sourceBundleId || "";
      } catch (e) {
        console.error("Failed to get popup state:", e);
      }
    });

    const unlistenHidePromise = listen("popup-hide", () => {
      hidePopup();
    });

    return () => {
      unlistenThemePromise.then((fn) => fn());
      unlistenShowPromise.then((fn) => fn());
      unlistenHidePromise.then((fn) => fn());
      cleanupThemeListener();
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
    ignoreMessage = "";
    ignoreError = false;
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
        reject();
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

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      accept();
    } else if (e.key === "Escape") {
      e.preventDefault();
      reject();
    }
  }

  function isInteractiveTarget(target: EventTarget | null): target is HTMLElement {
    return (
      target instanceof HTMLElement &&
      Boolean(target.closest("button, input, textarea, select, a, [role='button']"))
    );
  }

  async function startWindowDrag(event: MouseEvent) {
    if (event.button !== 0 || isInteractiveTarget(event.target)) return;
    try {
      await getCurrentWindow().startDragging();
    } catch (error) {
      console.error("Failed to start popup drag:", error);
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

<div class="popup" data-locale={$locale}>
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="header" onmousedown={startWindowDrag}>
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
      {#if ignoreMessage}
        <span class="ignore-message" class:error={ignoreError}>{ignoreMessage}</span>
      {/if}
      <TooltipProvider>
        <Tooltip>
          <TooltipTrigger>
            <button
              class="icon-btn ignore"
              aria-label={tr("popup.ignoreTooltip")}
              onclick={ignoreApp}
            >
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
    background: var(--popup-surface);
    backdrop-filter: blur(16px);
    -webkit-backdrop-filter: blur(16px);
    border-radius: 10px;
    box-shadow: var(--popup-shadow);
    padding: 8px 10px 10px;
    min-width: 180px;
    max-width: 100vw;
    border-bottom: 3px solid var(--popup-spell-border-accent);
  }

  .header {
    display: flex;
    align-items: center;
    gap: 6px;
    -webkit-app-region: drag;
    cursor: move;
    cursor: all-scroll;
  }

  .header-icon {
    color: var(--popup-spell-icon-accent);
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
    -webkit-app-region: no-drag;
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
    -webkit-app-region: no-drag;
  }

  .icon-btn.ignore:hover {
    background: var(--popup-hover-ignore-bg);
    color: var(--popup-hover-ignore-fg);
  }

  .icon-btn.close:hover {
    background: var(--popup-hover-close-bg);
    color: var(--popup-hover-close-fg);
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    -webkit-app-region: no-drag;
  }

  .chip {
    padding: 3px 10px;
    border: 1.5px solid var(--popup-chip-border);
    background: var(--popup-chip-bg);
    border-radius: 6px;
    font-size: 13px;
    font-weight: 500;
    color: var(--popup-chip-fg);
    cursor: pointer;
    transition: all 0.12s ease;
    -webkit-app-region: no-drag;
  }

  .chip:hover {
    border-color: var(--popup-chip-hover-border);
    background: var(--popup-chip-hover-bg);
    color: var(--popup-chip-hover-fg);
  }

  .chip:active {
    background: var(--popup-chip-active-bg);
  }

  .add-custom {
    color: var(--popup-muted-icon);
    flex-shrink: 0;
    align-self: center;
  }

  .add-custom.added {
    color: var(--popup-added-fg);
    cursor: default;
  }

  .add-custom:hover:not(:disabled) {
    background: var(--popup-chip-bg);
    color: var(--popup-chip-fg);
  }
</style>
