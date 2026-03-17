<script lang="ts">
  $locale;
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { Home, Info, Settings } from "lucide-svelte";
  import { onDestroy, onMount } from "svelte";
  import SettingsPanel from "$lib/components/SettingsPanel.svelte";
  import SpellChecker from "$lib/components/SpellChecker.svelte";
  import StatusIndicator from "$lib/components/StatusIndicator.svelte";
  import { Button } from "$lib/components/ui/button";
  import { locale, t } from "$lib/i18n";
  import type { ThemeMode } from "$lib/types/theme";
  import {
    applyThemeToDom,
    isThemeMode,
    loadThemeFromLocalStorage,
    saveThemeToLocalStorage,
  } from "$lib/theme";
  $locale;

  const tr = $derived(
    (key: string, params?: Record<string, string | number>) => {
      const _ = $locale;
      return t(key, params);
    },
  );

  let currentTab: "spellchecker" | "settings" | "about" =
    $state("spellchecker");
  let isEnabled = $state(true);
  let correctionCount = $state(0);
  let theme: ThemeMode = $state("auto");

  let mediaQuery: MediaQueryList | null = null;
  let unlistenThemeChanged: (() => void) | null = null;
  let unlistenAccepted: (() => void) | null = null;
  let unlistenNoChanges: (() => void) | null = null;

  async function loadThemeFromStore(): Promise<ThemeMode> {
    try {
      const stored = await invoke<string>("get_theme");
      if (isThemeMode(stored)) {
        return stored;
      }
    } catch (error) {
      console.warn(
        "Failed to load theme from store, fallback to localStorage:",
        error,
      );
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

  function handleToggleEnabled(enabled: boolean) {
    isEnabled = enabled;
  }

  onMount(async () => {
    applyTheme(await loadThemeFromStore());
    setupSystemThemeListener();

    try {
      unlistenThemeChanged = await listen<ThemeMode>(
        "theme-changed",
        (event) => {
          const mode = event.payload;
          if (isThemeMode(mode)) {
            applyTheme(mode);
          }
        },
      );
    } catch (error) {
      console.error("Failed to listen for theme changes:", error);
    }

    unlistenAccepted = await listen("suggestion-accepted", () => {
      correctionCount++;
    });

    unlistenNoChanges = await listen("no-changes-needed", () => {
      console.log(tr("spell.noSuggestions"));
    });
  });

  onDestroy(() => {
    cleanupThemeListener();
    unlistenThemeChanged?.();
    unlistenThemeChanged = null;
    unlistenAccepted?.();
    unlistenAccepted = null;
    unlistenNoChanges?.();
    unlistenNoChanges = null;
  });
</script>

<div class="flex h-screen flex-col bg-background" data-locale={$locale}>
  <!-- Top Status Bar -->
  <header class="border-b bg-card/50 backdrop-blur-sm">
    <div class="flex items-center justify-between px-4 py-2">
      <div class="flex items-center gap-2">
        <div
          class="flex h-8 w-8 items-center justify-center rounded-lg bg-primary"
        >
          <span class="text-lg font-bold text-primary-foreground">A</span>
        </div>
        <h1 class="text-lg font-semibold hidden">AutoCorrect</h1>
      </div>

      <nav class="flex gap-1">
        <Button
          onclick={() => (currentTab = "spellchecker")}
          variant={currentTab === "spellchecker" ? "default" : "ghost"}
          size="sm"
        >
          <Home class="mr-1 h-4 w-4" />
          {tr("app.tab.spellchecker")}
        </Button>
        <Button
          onclick={() => (currentTab = "settings")}
          variant={currentTab === "settings" ? "default" : "ghost"}
          size="sm"
        >
          <Settings class="mr-1 h-4 w-4" />
          {tr("app.tab.settings")}
        </Button>
        <Button
          onclick={() => (currentTab = "about")}
          variant={currentTab === "about" ? "default" : "ghost"}
          size="sm"
        >
          <Info class="mr-1 h-4 w-4" />
          {tr("app.tab.about")}
        </Button>
      </nav>

      <StatusIndicator
        bind:isEnabled
        bind:correctionCount
        onToggle={handleToggleEnabled}
        compact={true}
      />
    </div>
  </header>

  <!-- Main Content Area -->
  <main class="flex-1 overflow-auto">
    {#if currentTab === "spellchecker"}
      <SpellChecker />
    {:else if currentTab === "settings"}
      <SettingsPanel {theme} />
    {:else if currentTab === "about"}
      <div class="flex h-full items-center justify-center p-6">
        <div class="max-w-md space-y-4 text-center">
          <div
            class="mx-auto flex h-16 w-16 items-center justify-center rounded-2xl bg-primary"
          >
            <span class="text-3xl font-bold text-primary-foreground">A</span>
          </div>
          <h2 class="text-2xl font-bold">{tr("app.about.title")}</h2>
          <p class="text-muted-foreground">{tr("app.about.desc")}</p>
          <div class="space-y-2 rounded-lg border bg-card p-4 text-left">
            <h3 class="text-sm font-semibold">{tr("app.about.features")}</h3>
            <ul class="space-y-1 text-sm text-muted-foreground">
              <li>{tr("app.about.f1")}</li>
              <li>{tr("app.about.f2")}</li>
              <li>{tr("app.about.f3")}</li>
              <li>{tr("app.about.f4")}</li>
              <li>{tr("app.about.f5")}</li>
            </ul>
          </div>
          <div class="text-xs text-muted-foreground">
            {tr("app.about.version")}
          </div>
        </div>
      </div>
    {/if}
  </main>
</div>
