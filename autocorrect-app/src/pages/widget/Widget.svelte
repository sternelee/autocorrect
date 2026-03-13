<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';

  interface WidgetUpdate {
    typoCount: number;
  }

  let typoCount = $state(0);

  const hasErrors = $derived(typoCount > 0);
  const badgeText = $derived(typoCount > 9 ? '9+' : String(typoCount));

  onMount(() => {
    const unlisten = listen<WidgetUpdate>('widget-update', (event) => {
      typoCount = event.payload.typoCount;
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  });

  async function handleClick() {
    try {
      const pos = await invoke<[number, number]>('get_cursor_pos_cmd');
      await invoke('trigger_widget_popup', { x: pos[0], y: pos[1] });
    } catch (e) {
      console.error('Failed to show popup:', e);
    }
  }
</script>

<div
  id="widget"
  class:has-errors={hasErrors}
  title="Click to see suggestions"
  onclick={handleClick}
  role="button"
  tabindex="0"
  onkeydown={(e) => e.key === 'Enter' && handleClick()}
>
  <svg class="icon" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
    <path
      d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-1 17.93c-3.95-.49-7-3.85-7-7.93 0-.62.08-1.21.21-1.79L9 15v1c0 1.1.9 2 2 2v1.93zm6.9-2.54c-.26-.81-1-1.39-1.9-1.39h-1v-3c0-.55-.45-1-1-1H8v-2h2c.55 0 1-.45 1-1V7h2c1.1 0 2-.9 2-2v-.41c2.93 1.19 5 4.06 5 7.41 0 2.08-.8 3.97-2.1 5.39z"
    />
  </svg>
  {#if hasErrors}
    <span class="badge">{badgeText}</span>
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

  #widget {
    width: 36px;
    height: 36px;
    background: linear-gradient(135deg, #16a34a 0%, #15803d 100%);
    border-radius: 50%;
    box-shadow: 0 4px 16px rgba(22, 163, 74, 0.4);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: all 0.2s ease;
    -webkit-app-region: no-drag;
    position: relative;
  }

  #widget:hover {
    transform: scale(1.1);
    box-shadow: 0 6px 20px rgba(22, 163, 74, 0.5);
  }

  #widget:active {
    transform: scale(0.95);
  }

  #widget.has-errors {
    background: linear-gradient(135deg, #dc2626 0%, #b91c1c 100%);
    box-shadow: 0 4px 16px rgba(220, 38, 38, 0.4);
    animation: pulse 2s infinite;
  }

  #widget.has-errors:hover {
    box-shadow: 0 6px 20px rgba(220, 38, 38, 0.5);
  }

  @keyframes pulse {
    0%,
    100% {
      box-shadow: 0 4px 16px rgba(220, 38, 38, 0.4);
    }
    50% {
      box-shadow: 0 4px 24px rgba(220, 38, 38, 0.6);
    }
  }

  .icon {
    width: 20px;
    height: 20px;
    fill: white;
  }

  .badge {
    position: absolute;
    top: -4px;
    right: -4px;
    min-width: 18px;
    height: 18px;
    background: white;
    border-radius: 9px;
    color: #dc2626;
    font-size: 11px;
    font-weight: 700;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0 5px;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.2);
  }
</style>
