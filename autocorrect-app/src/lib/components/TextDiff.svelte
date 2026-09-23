<script lang="ts">
  type DiffPart = {
    type: "equal" | "added" | "removed";
    value: string;
  };

  let { original, revised }: { original: string; revised: string } = $props();

  const MAX_DIFF_TOKENS = 600;

  function tokenize(text: string): string[] {
    return text.match(/\s+|\S+/g) ?? [];
  }

  function pushPart(parts: DiffPart[], type: DiffPart["type"], value: string) {
    if (!value) return;
    const last = parts[parts.length - 1];
    if (last?.type === type) {
      last.value += value;
    } else {
      parts.push({ type, value });
    }
  }

  function buildTextDiff(before: string, after: string): DiffPart[] {
    if (before === after) return [{ type: "equal", value: after }];

    const beforeTokens = tokenize(before);
    const afterTokens = tokenize(after);

    // Keep the popup responsive for very large selections. The editable full
    // text remains available below, while this fallback still signals a change.
    if (beforeTokens.length * afterTokens.length > MAX_DIFF_TOKENS * MAX_DIFF_TOKENS) {
      return [
        { type: "removed", value: before },
        { type: "added", value: after },
      ];
    }

    const rows = beforeTokens.length + 1;
    const cols = afterTokens.length + 1;
    const lcs = new Uint32Array(rows * cols);

    for (let i = beforeTokens.length - 1; i >= 0; i -= 1) {
      for (let j = afterTokens.length - 1; j >= 0; j -= 1) {
        lcs[i * cols + j] =
          beforeTokens[i] === afterTokens[j]
            ? lcs[(i + 1) * cols + j + 1] + 1
            : Math.max(lcs[(i + 1) * cols + j], lcs[i * cols + j + 1]);
      }
    }

    const parts: DiffPart[] = [];
    let i = 0;
    let j = 0;
    while (i < beforeTokens.length && j < afterTokens.length) {
      if (beforeTokens[i] === afterTokens[j]) {
        pushPart(parts, "equal", beforeTokens[i]);
        i += 1;
        j += 1;
      } else if (lcs[(i + 1) * cols + j] >= lcs[i * cols + j + 1]) {
        pushPart(parts, "removed", beforeTokens[i]);
        i += 1;
      } else {
        pushPart(parts, "added", afterTokens[j]);
        j += 1;
      }
    }
    while (i < beforeTokens.length) {
      pushPart(parts, "removed", beforeTokens[i]);
      i += 1;
    }
    while (j < afterTokens.length) {
      pushPart(parts, "added", afterTokens[j]);
      j += 1;
    }

    return parts;
  }

  const parts = $derived(buildTextDiff(original, revised));
</script>

<p class="text-diff">
  {#each parts as part}
    <span
      class:diff-added={part.type === "added"}
      class:diff-removed={part.type === "removed"}
      >{part.value}</span
    >
  {/each}
</p>

<style>
  .text-diff {
    margin: 0;
    font-size: 12px;
    line-height: 1.5;
    white-space: pre-wrap;
    word-break: break-word;
    color: var(--popup-muted-text, inherit);
  }

  .diff-added {
    background: color-mix(in srgb, #34c759 22%, transparent);
    color: var(--popup-title, inherit);
    border-radius: 3px;
    text-decoration: none;
  }

  .diff-removed {
    background: color-mix(in srgb, #ff3b30 18%, transparent);
    color: var(--popup-muted-text, inherit);
    border-radius: 3px;
    text-decoration: line-through;
    text-decoration-thickness: 1px;
  }
</style>
