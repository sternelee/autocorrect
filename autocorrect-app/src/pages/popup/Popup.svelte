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
  let isEditing = $state(false);
  let editValue = $state('');
  let notification = $state<{ message: string; isError: boolean } | null>(null);

  let editTextarea = $state<HTMLTextAreaElement | undefined>(undefined);
  let notificationTimer: ReturnType<typeof setTimeout> | null = null;

  const showOriginal = $derived(originalText !== suggestion && originalText !== '');

  onMount(() => {
    const unlistenShow = listen<PopupData>('popup-show', (event) => {
      const data = event.payload;
      originalText = data.originalText;
      suggestion = data.suggestion;
      typos = data.typos || [];
      offset = data.offset ?? null;
      charLength = data.charLength ?? null;
      isEditing = false;
      editValue = '';
    });

    const unlistenHide = listen('popup-hide', () => {
      hidePopup();
    });

    return () => {
      unlistenShow.then((fn) => fn());
      unlistenHide.then((fn) => fn());
    };
  });

  async function acceptSuggestion() {
    const textToUse = isEditing ? editValue : suggestion;
    if (!textToUse.trim()) return;
    try {
      await invoke('accept_suggestion', { text: textToUse, offset, charLength });
    } catch (error) {
      console.error('Failed to accept suggestion:', error);
    }
  }

  async function rejectSuggestion() {
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
    isEditing = false;
    editValue = '';
  }

  function startEdit() {
    editValue = suggestion;
    isEditing = true;
    setTimeout(() => editTextarea?.focus(), 0);
  }

  function cancelEdit() {
    isEditing = false;
    editValue = '';
  }

  function applyTypoSuggestion(typo: string, sugg: string) {
    const regex = new RegExp(`\\b${escapeRegex(typo)}\\b`, 'gi');
    suggestion = suggestion.replace(regex, sugg);
    typos = typos.filter((t) => t.typo !== typo);
  }

  async function addToCustomCorrections(typo: string, correction: string) {
    try {
      await invoke('add_custom_correction', { typo, correction });
      typos = typos.filter((t) => t.typo !== typo);
      showNotificationMsg(`Added "${typo} → ${correction}" to custom corrections.`);
    } catch (error) {
      showNotificationMsg(`Failed to add custom correction: ${error}`, true);
    }
  }

  function showNotificationMsg(message: string, isError = false) {
    if (notificationTimer) clearTimeout(notificationTimer);
    notification = { message, isError };
    notificationTimer = setTimeout(() => {
      notification = null;
    }, 3000);
  }

  function escapeRegex(text: string): string {
    return text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey && !isEditing) {
      e.preventDefault();
      acceptSuggestion();
    } else if (e.key === 'Escape') {
      e.preventDefault();
      if (isEditing) cancelEdit();
      else rejectSuggestion();
    }
  }

  function onTextareaInput(e: Event) {
    const el = e.target as HTMLTextAreaElement;
    el.style.height = 'auto';
    el.style.height = Math.max(60, el.scrollHeight) + 'px';
  }
</script>

<svelte:window onkeydown={onKeydown} />

<div id="popup">
  <div class="header">
    <h3>✨ Suggestion</h3>
    <button class="close-btn" onclick={rejectSuggestion} title="Close (Esc)">×</button>
  </div>

  <div class="content">
    {#if showOriginal}
      <div class="original">
        <span class="label">Original:</span>
        <span class="text">{originalText}</span>
      </div>
    {/if}
    <div class="suggestion">
      <span class="label">Suggested:</span>
      <span class="text">{suggestion}</span>
    </div>
  </div>

  {#if typos.length > 0}
    <div class="typos">
      <div class="typos-header">⚠️ Spelling Issues</div>
      <div class="typos-list">
        {#each typos as typo}
          <div class="typo-item">
            <div class="typo-error">
              <span class="typo-word">{typo.typo}</span>
              <span class="typo-location">Line {typo.line}, Col {typo.col}</span>
            </div>
            <div class="typo-suggestions">
              {#each typo.suggestions.slice(0, 3) as sugg}
                <button
                  class="typo-suggestion-btn"
                  onclick={() => applyTypoSuggestion(typo.typo, sugg)}
                  title={`Replace "${typo.typo}" with "${sugg}"`}
                >{sugg}</button>
              {/each}
              {#if typo.suggestions.length > 0}
                <button
                  class="typo-custom-btn"
                  onclick={() => addToCustomCorrections(typo.typo, typo.suggestions[0])}
                  title={`Add "${typo.typo} → ${typo.suggestions[0]}" to custom corrections`}
                >+ Add to Custom</button>
              {/if}
            </div>
          </div>
        {/each}
      </div>
    </div>
  {/if}

  {#if isEditing}
    <div class="edit-mode">
      <textarea
        class="edit-textarea"
        bind:this={editTextarea}
        bind:value={editValue}
        placeholder="Custom correction..."
        oninput={onTextareaInput}
      ></textarea>
    </div>
  {/if}

  {#if !isEditing}
    <div class="actions">
      <button class="btn btn-reject" onclick={rejectSuggestion}>✕ Reject</button>
      <button class="btn btn-edit" onclick={startEdit} title="Edit the suggestion">✎ Edit</button>
      <button class="btn btn-accept" onclick={acceptSuggestion}>✓ Accept</button>
    </div>
  {:else}
    <div class="edit-actions">
      <button class="btn btn-reject" onclick={cancelEdit}>✕ Cancel</button>
      <button class="btn btn-accept" onclick={acceptSuggestion}>✓ Use Custom</button>
    </div>
  {/if}

  <div class="hint">Enter=Accept, Esc=Reject</div>
</div>

{#if notification}
  <div
    class="notification"
    class:notification-success={!notification.isError}
    class:notification-error={notification.isError}
  >
    {notification.message}
  </div>
{/if}

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

  #popup {
    width: 100vw;
    height: 100vh;
    background: rgba(255, 255, 255, 0.98);
    backdrop-filter: blur(20px);
    border-radius: 12px;
    box-shadow: 0 10px 40px rgba(0, 0, 0, 0.2);
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .header h3 {
    font-size: 14px;
    font-weight: 600;
    color: #1a1a1a;
  }

  .close-btn {
    width: 28px;
    height: 28px;
    border: none;
    background: transparent;
    border-radius: 50%;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #dc2626;
    font-size: 18px;
    line-height: 1;
    -webkit-app-region: no-drag;
  }

  .close-btn:hover {
    background: rgba(220, 38, 38, 0.1);
  }

  .content {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .original {
    font-size: 13px;
    color: #6b7280;
  }

  .original .text {
    text-decoration: line-through;
    color: #dc2626;
  }

  .suggestion {
    font-size: 15px;
    font-weight: 500;
    color: #16a34a;
  }

  .actions,
  .edit-actions {
    display: flex;
    gap: 8px;
    margin-top: 4px;
  }

  .btn {
    flex: 1;
    padding: 8px 16px;
    border: none;
    border-radius: 8px;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    -webkit-app-region: no-drag;
    transition: all 0.15s ease;
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-accept {
    background: #16a34a;
    color: white;
  }

  .btn-accept:hover:not(:disabled) {
    background: #15803d;
  }

  .btn-reject {
    background: #e5e7eb;
    color: #374151;
  }

  .btn-reject:hover:not(:disabled) {
    background: #d1d5db;
  }

  .btn-edit {
    background: #f3f4f6;
    color: #374151;
  }

  .btn-edit:hover:not(:disabled) {
    background: #e5e7eb;
  }

  .edit-mode {
    display: flex;
    flex-direction: column;
  }

  .edit-textarea {
    width: 100%;
    min-height: 60px;
    padding: 8px;
    border: 1px solid #d1d5db;
    border-radius: 6px;
    font-size: 13px;
    font-family: inherit;
    resize: vertical;
    -webkit-app-region: no-drag;
  }

  .edit-textarea:focus {
    outline: none;
    border-color: #16a34a;
  }

  .hint {
    text-align: center;
    font-size: 10px;
    color: #9ca3af;
    padding-top: 8px;
    border-top: 1px solid #f3f4f6;
  }

  .typos {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 8px;
    background: #fef3c7;
    border-radius: 8px;
    border: 1px solid #fbbf24;
  }

  .typos-header {
    font-size: 12px;
    font-weight: 600;
    color: #92400e;
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .typos-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .typo-item {
    background: white;
    padding: 8px;
    border-radius: 6px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .typo-error {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .typo-word {
    font-weight: 600;
    color: #dc2626;
    text-decoration: underline wavy #dc2626;
  }

  .typo-location {
    font-size: 10px;
    color: #6b7280;
  }

  .typo-suggestions {
    display: flex;
    gap: 4px;
    flex-wrap: wrap;
  }

  .typo-suggestion-btn {
    padding: 4px 8px;
    border: 1px solid #d1d5db;
    background: #f9fafb;
    border-radius: 4px;
    font-size: 11px;
    color: #374151;
    cursor: pointer;
    -webkit-app-region: no-drag;
    transition: all 0.15s ease;
  }

  .typo-suggestion-btn:hover {
    background: #16a34a;
    color: white;
    border-color: #16a34a;
  }

  .typo-custom-btn {
    padding: 4px 8px;
    border: 1px solid #9333ea;
    background: #faf5ff;
    border-radius: 4px;
    font-size: 11px;
    color: #9333ea;
    cursor: pointer;
    -webkit-app-region: no-drag;
    transition: all 0.15s ease;
    font-weight: 500;
  }

  .typo-custom-btn:hover {
    background: #9333ea;
    color: white;
  }

  .notification {
    position: fixed;
    top: 8px;
    left: 50%;
    transform: translateX(-50%);
    padding: 8px 16px;
    border-radius: 6px;
    font-size: 11px;
    font-weight: 500;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
    z-index: 1000;
    animation: slideDown 0.3s ease;
    -webkit-app-region: no-drag;
  }

  .notification-success {
    background: #16a34a;
    color: white;
  }

  .notification-error {
    background: #dc2626;
    color: white;
  }

  @keyframes slideDown {
    from {
      opacity: 0;
      transform: translateX(-50%) translateY(-10px);
    }
    to {
      opacity: 1;
      transform: translateX(-50%) translateY(0);
    }
  }
</style>
