<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';

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

  let originalText = $state('');
  let suggestion = $state('');
  let typos = $state<TypoSuggestion[]>([]);
  let offset = $state<number | null>(null);
  let charLength = $state<number | null>(null);

  // For a typo-only popup triggered by hover, we surface the first typo's suggestions as chips.
  // For a full spell-check popup, we show the whole corrected suggestion.
  const chips = $derived(
    typos.length > 0
      ? typos[0].suggestions.slice(0, 4)
      : suggestion && suggestion !== originalText
        ? [suggestion]
        : []
  );

  const title = $derived(typos.length > 0 ? 'Correct spelling' : 'AutoCorrect');

  onMount(() => {
    const unlistenShow = listen<PopupData>('popup-show', (event) => {
      const data = event.payload;
      originalText = data.originalText;
      suggestion = data.suggestion;
      typos = data.typos || [];
      offset = data.offset ?? null;
      charLength = data.charLength ?? null;
    });

    const unlistenHide = listen('popup-hide', () => {
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
      await invoke('accept_suggestion', { text: textToUse, offset, charLength });
    } catch (error) {
      console.error('Failed to accept suggestion:', error);
    }
  }

  async function reject() {
    try {
      await invoke('reject_suggestion');
    } catch (error) {
      console.error('Failed to reject suggestion:', error);
      hidePopup();
    }
  }

  function hidePopup() {
    getCurrentWindow().hide();
    originalText = '';
    suggestion = '';
    typos = [];
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      accept();
    } else if (e.key === 'Escape') {
      e.preventDefault();
      reject();
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

<div class="popup">
  <div class="header">
    <span class="header-icon">
      <!-- pencil icon -->
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
        <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/>
        <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"/>
      </svg>
    </span>
    <span class="title">{title}</span>
    <div class="header-actions">
      <button class="icon-btn thumb" title="Accept (Enter)" onclick={() => accept()}>
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M14 9V5a3 3 0 0 0-3-3l-4 9v11h11.28a2 2 0 0 0 2-1.7l1.38-9a2 2 0 0 0-2-2.3H14z"/>
          <path d="M7 22H4a2 2 0 0 1-2-2v-7a2 2 0 0 1 2-2h3"/>
        </svg>
      </button>
      <button class="icon-btn thumb" title="Reject (Esc)" onclick={reject}>
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M10 15v4a3 3 0 0 0 3 3l4-9V2H5.72a2 2 0 0 0-2 1.7l-1.38 9a2 2 0 0 0 2 2.3H10z"/>
          <path d="M17 2h2.67A2.31 2.31 0 0 1 22 4v7a2.31 2.31 0 0 1-2.33 2H17"/>
        </svg>
      </button>
      <button class="icon-btn close" title="Close" onclick={reject}>
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">
          <line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>
        </svg>
      </button>
    </div>
  </div>

  {#if chips.length > 0}
    <div class="chips">
      {#each chips as chip}
        <button class="chip" onclick={() => accept(chip)}>{chip}</button>
      {/each}
    </div>
  {/if}
</div>

<style>
  :global(*) {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
  }

  :global(body) {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    background: transparent;
    overflow: hidden;
    -webkit-app-region: drag;
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
    transition: background 0.12s, color 0.12s;
    padding: 0;
    -webkit-app-region: no-drag;
  }

  .icon-btn.thumb:hover {
    background: #f3f4f6;
    color: #374151;
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
</style>
