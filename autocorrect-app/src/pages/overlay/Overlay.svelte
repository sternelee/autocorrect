<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';

  interface TypoMarker {
    id: string;
    x: number;
    y: number;
    width: number;
    height: number;
    text: string;
  }

  interface MarkerView {
    key: string;
    left: number;
    top: number;
    width: number;
    opacity: number;
  }

  let markers = $state<MarkerView[]>([]);

  function clamp(value: number, min: number, max: number): number {
    return Math.min(Math.max(value, min), max);
  }

  function resolveMarkerTop(marker: TypoMarker): number {
    if (marker.id.includes('fallback')) {
      return clamp(window.innerHeight - marker.y - 2, 0, Math.max(0, window.innerHeight - 4));
    }
    const topFromTopLeft = marker.y + marker.height - 2;
    const topFromBottomLeft = window.innerHeight - marker.y - 2;
    const topA = clamp(topFromTopLeft, 0, Math.max(0, window.innerHeight - 4));
    const topB = clamp(topFromBottomLeft, 0, Math.max(0, window.innerHeight - 4));
    const aDelta = Math.abs(topFromTopLeft - topA);
    const bDelta = Math.abs(topFromBottomLeft - topB);
    return aDelta <= bDelta ? topA : topB;
  }

  function buildViews(incoming: TypoMarker[]): MarkerView[] {
    const views: MarkerView[] = [];
    for (const marker of incoming) {
      if (!marker.width || marker.width <= 0) continue;

      const isFallback = marker.id.includes('fallback');
      const topA = clamp(marker.y + marker.height - 2, 0, Math.max(0, window.innerHeight - 4));
      const topB = clamp(window.innerHeight - marker.y - 2, 0, Math.max(0, window.innerHeight - 4));

      views.push({
        key: marker.id,
        left: Math.max(0, marker.x),
        top: resolveMarkerTop(marker),
        width: Math.max(2, marker.width),
        opacity: 1,
      });

      if (!isFallback && Math.abs(topA - topB) > 6) {
        views.push({
          key: `${marker.id}-mirror`,
          left: Math.max(0, marker.x),
          top: topB,
          width: Math.max(2, marker.width),
          opacity: 0.55,
        });
      }
    }
    return views;
  }

  onMount(() => {
    const unlisten = listen<TypoMarker[]>('update-markers', (event) => {
      markers = buildViews(event.payload || []);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  });
</script>

<div id="markers-container">
  {#each markers as m (m.key)}
    <div
      class="typo-underline"
      style:left="{m.left}px"
      style:top="{m.top}px"
      style:width="{m.width}px"
      style:opacity={m.opacity}
    ></div>
  {/each}
</div>

<style>
  :global(html),
  :global(body) {
    width: 100%;
    height: 100%;
    margin: 0;
    padding: 0;
    overflow: hidden;
    background: transparent;
    pointer-events: none;
  }

  #markers-container {
    position: fixed;
    inset: 0;
    pointer-events: none;
  }

  .typo-underline {
    position: absolute;
    height: 6px;
    background: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='8' height='6' viewBox='0 0 8 6'%3E%3Cpath d='M0 4 Q 2 1, 4 4 T 8 4' fill='none' stroke='%23ff3b30' stroke-width='1.4' stroke-linecap='round'/%3E%3C/svg%3E")
      repeat-x;
    pointer-events: none;
    z-index: 9999;
    opacity: 0.96;
  }
</style>
