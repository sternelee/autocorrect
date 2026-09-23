<script lang="ts">
  $locale;
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { Button } from "$lib/components/ui/button";
  import { Textarea } from "$lib/components/ui/textarea";
  import { AlertCircle, RotateCcw, Save } from "lucide-svelte";
  import { locale, t } from "$lib/i18n";
  import type { CSpellConfig } from "$lib/types/app";

  interface Props {
    configPath: string;
  }

  let { configPath }: Props = $props();

  const tr = $derived(
    (key: string, params?: Record<string, string | number>) => {
      const _ = $locale;
      return t(key, params);
    },
  );

  let words = $state("");
  let ignoreWords = $state("");
  let flagWords = $state("");
  let isLoading = $state(false);
  let saveSuccess = $state(false);
  let errorMessage: string | null = $state(null);

  async function loadConfig() {
    isLoading = true;
    errorMessage = null;
    try {
      const cfg = await invoke<CSpellConfig>("get_cspell_config", {
        path: configPath,
      });
      words = (cfg.words ?? []).join("\n");
      ignoreWords = (cfg.ignoreWords ?? []).join("\n");
      flagWords = (cfg.flagWords ?? []).join("\n");
    } catch (error) {
      console.error("Failed to load CSpell config:", error);
      errorMessage =
        error instanceof Error ? error.message : tr("cspell.loadError");
    } finally {
      isLoading = false;
    }
  }

  async function saveConfig() {
    isLoading = true;
    errorMessage = null;
    saveSuccess = false;

    const cfg: CSpellConfig = {
      words: words
        .split("\n")
        .map((w) => w.trim())
        .filter((w) => w.length > 0),
      ignoreWords: ignoreWords
        .split("\n")
        .map((w) => w.trim())
        .filter((w) => w.length > 0),
      flagWords: flagWords
        .split("\n")
        .map((w) => w.trim())
        .filter((w) => w.length > 0),
    };

    try {
      await invoke("save_cspell_config", { path: configPath, config: cfg });
      saveSuccess = true;
      setTimeout(() => {
        saveSuccess = false;
      }, 2000);
    } catch (error) {
      console.error("Failed to save CSpell config:", error);
      errorMessage =
        error instanceof Error ? error.message : tr("cspell.saveError");
    } finally {
      isLoading = false;
    }
  }

  async function reloadConfig() {
    try {
      await invoke("reload_cspell_config_cmd");
      await loadConfig();
    } catch (error) {
      console.error("Failed to reload CSpell config:", error);
      errorMessage =
        error instanceof Error ? error.message : tr("cspell.loadError");
    }
  }

  $effect(() => {
    // Reload whenever the configured path changes.
    configPath;
    void loadConfig();
  });

  onMount(() => {
    void loadConfig();
  });
</script>

<div class="space-y-4">
  <div>
    <h3 class="text-sm font-semibold">{tr("cspell.title")}</h3>
    <p class="text-muted-foreground mt-1 text-xs">
      {tr("cspell.desc", { path: configPath || tr("settings.loading") })}
    </p>
  </div>

  {#if errorMessage}
    <div
      class="rounded-lg border border-red-200 bg-red-50 p-3 dark:border-red-800 dark:bg-red-950"
    >
      <div class="flex items-start gap-2">
        <AlertCircle
          class="h-4 w-4 flex-shrink-0 text-red-600 dark:text-red-400 mt-0.5"
        />
        <p class="text-sm text-red-900 dark:text-red-100">{errorMessage}</p>
      </div>
    </div>
  {/if}

  <div class="grid grid-cols-1 gap-4 md:grid-cols-3">
    <div class="space-y-2">
      <label for="cspell-words" class="text-xs font-medium">
        {tr("cspell.words")}
      </label>
      <Textarea
        id="cspell-words"
        bind:value={words}
        placeholder={tr("cspell.wordsPlaceholder")}
        class="min-h-[120px] font-mono text-sm"
        disabled={isLoading}
      />
      <p class="text-muted-foreground text-xs">{tr("cspell.wordsDesc")}</p>
    </div>

    <div class="space-y-2">
      <label for="cspell-ignore-words" class="text-xs font-medium">
        {tr("cspell.ignoreWords")}
      </label>
      <Textarea
        id="cspell-ignore-words"
        bind:value={ignoreWords}
        placeholder={tr("cspell.ignoreWordsPlaceholder")}
        class="min-h-[120px] font-mono text-sm"
        disabled={isLoading}
      />
      <p class="text-muted-foreground text-xs">{tr("cspell.ignoreWordsDesc")}</p>
    </div>

    <div class="space-y-2">
      <label for="cspell-flag-words" class="text-xs font-medium">
        {tr("cspell.flagWords")}
      </label>
      <Textarea
        id="cspell-flag-words"
        bind:value={flagWords}
        placeholder={tr("cspell.flagWordsPlaceholder")}
        class="min-h-[120px] font-mono text-sm"
        disabled={isLoading}
      />
      <p class="text-muted-foreground text-xs">{tr("cspell.flagWordsDesc")}</p>
    </div>
  </div>

  <div class="flex flex-wrap gap-2">
    <Button onclick={saveConfig} disabled={isLoading} variant="default">
      {#if isLoading}
        <div
          class="mr-2 h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent"
        ></div>
      {:else}
        <Save class="mr-2 h-4 w-4" />
      {/if}
      {saveSuccess ? tr("cspell.saved") : tr("cspell.save")}
    </Button>
    <Button onclick={reloadConfig} disabled={isLoading} variant="outline">
      <RotateCcw class="mr-2 h-4 w-4" />
      {tr("cspell.reload")}
    </Button>
  </div>
</div>
