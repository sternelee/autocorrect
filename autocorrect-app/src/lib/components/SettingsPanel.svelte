<script lang="ts">
  $locale;
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { Button } from "$lib/components/ui/button";
  import {
    Card,
    CardContent,
    CardDescription,
    CardHeader,
    CardTitle,
  } from "$lib/components/ui/card";
  import { Input } from "$lib/components/ui/input";
  import { Switch } from "$lib/components/ui/switch";
  import { Textarea } from "$lib/components/ui/textarea";
  import {
    Download,
    Upload,
    Save,
    RotateCcw,
    AlertCircle,
    Keyboard,
  } from "lucide-svelte";
  import CustomCorrectionsManager from "./CustomCorrectionsManager.svelte";
  import IgnoredAppsManager from "./IgnoredAppsManager.svelte";
  import type { ThemeMode } from "$lib/types/theme";
  import { locale, t, setLocale } from "$lib/i18n";

  let { theme }: { theme: ThemeMode } = $props();

  // Reactive translation helper - use in template with {tr("key")}
  const tr = $derived((key: string, params?: Record<string, string | number>) => {
    // Access $locale to establish reactive dependency
    const _ = $locale;
    return t(key, params);
  });

  // Rule info from backend
  interface RuleInfo {
    name: string;
    severity: number; // 0=off, 1=error, 2=warning
    description: string;
    defaultSeverity: number;
  }

  // App config from backend
  interface AppConfig {
    rules: Record<string, number>;
    textRules: Record<string, number>;
    spellcheckWords: string[];
    fileTypes: Record<string, string>;
    context: Record<string, number>;
    configPath: string;
    typoCheckingEnabled?: boolean;
    aiGrammarEnabled?: boolean;
    openaiApiKey?: string;
    openaiModel?: string;
    aiMaxInputChars?: number;
    aiTimeoutMs?: number;
    aiApiBaseUrl?: string;
    aiTranslateTargetLanguage?: string;
    aiPolishStyles?: string[];
    uiLanguage?: string;
    underlineStyle?: string;
    underlineColor?: string;
  }

  // Hotkey configuration
  interface Modifiers {
    shift: boolean;
    ctrl: boolean;
    meta: boolean;
    alt: boolean;
  }

  interface HotkeyConfig {
    key: string;
    modifiers: Modifiers;
    display_string: string;
  }

  let configPath = $state("");
  let isLoading = $state(false);
  let saveSuccess = $state(false);
  let loadError: string | null = $state(null);

  // All available rules with their info
  let rules: RuleInfo[] = $state([]);

  // Custom words for spellcheck
  let customWords = $state("");

  // Typo checking configuration
  let typoCheckingEnabled = $state(true);

  // Autostart configuration
  let autostartEnabled = $state(false);

  // AI grammar configuration
  let aiGrammarEnabled = $state(false);
  let openaiApiKey = $state("");
  let openaiModel = $state("gpt-4.1-mini");
  let aiMaxInputChars = $state(2000);
  let aiTimeoutMs = $state(12000);
  let aiApiBaseUrl = $state("https://openrouter.ai/api/v1/chat/completions");
  let aiTranslateTargetLanguage = $state("English");
  let aiPolishStyles = $state<string[]>([]);

  // Available polish styles
  const POLISH_STYLES = [
    { value: "formal", labelKey: "aipopup.styleFormal" },
    { value: "conversational", labelKey: "aipopup.styleConversational" },
    { value: "academic", labelKey: "aipopup.styleAcademic" },
    { value: "business", labelKey: "aipopup.styleBusiness" },
  ];
  let uiLanguage: "en" | "zh-CN" = $state("en");

  // Underline appearance
  let underlineStyle = $state("wavy");
  let underlineColor = $state("#ff3b30");

  const UNDERLINE_STYLES = [
    { value: "wavy", labelKey: "settings.underlineStyle.wavy" },
    { value: "solid", labelKey: "settings.underlineStyle.solid" },
    { value: "dashed", labelKey: "settings.underlineStyle.dashed" },
    { value: "dotted", labelKey: "settings.underlineStyle.dotted" },
  ];

  const UNDERLINE_COLORS = [
    {
      value: "#ff3b30",
      labelKey: "settings.underlineColor.red",
      tw: "bg-[#ff3b30]",
    },
    {
      value: "#ff9500",
      labelKey: "settings.underlineColor.orange",
      tw: "bg-[#ff9500]",
    },
    {
      value: "#ffcc00",
      labelKey: "settings.underlineColor.yellow",
      tw: "bg-[#ffcc00]",
    },
    {
      value: "#ff2d55",
      labelKey: "settings.underlineColor.pink",
      tw: "bg-[#ff2d55]",
    },
    {
      value: "#af52de",
      labelKey: "settings.underlineColor.purple",
      tw: "bg-[#af52de]",
    },
    {
      value: "#007aff",
      labelKey: "settings.underlineColor.blue",
      tw: "bg-[#007aff]",
    },
    {
      value: "#34c759",
      labelKey: "settings.underlineColor.green",
      tw: "bg-[#34c759]",
    },
  ];

  const translateLanguageOptions = [
    "简体中文",
    "English",
    "繁體中文",
    "日本語",
    "Русский",
    "한국어",
    "Français",
    "Deutsch",
    "Español",
    "Português",
  ];

  // Hotkey configuration state
  let hotkeyEnabled = $state(true);
  let hotkeyConfig: HotkeyConfig | null = $state(null);
  let showKeySelector = $state(false);
  let isRecording = $state(false);
  let recordedShortcut: HotkeyConfig | null = $state(null);
  let recordingError: string | null = $state(null);
  let recordingTimeout: ReturnType<typeof setTimeout> | null = $state(null);

  // Track unsaved changes
  let hasUnsavedChanges = $state(false);

  async function onThemeChange(selectedTheme: ThemeMode) {
    try {
      await invoke("set_theme", { theme: selectedTheme });
    } catch (e) {
      console.error("Failed to set theme:", e);
    }
  }

  async function loadConfiguration() {
    isLoading = true;
    loadError = null;
    try {
      // Load critical settings in parallel to reduce wait time.
      const [config, allRules, autostart] = await Promise.all([
        invoke<AppConfig>("get_config"),
        invoke<RuleInfo[]>("get_rules"),
        invoke<boolean>("get_autostart_enabled").catch((e) => {
          console.error("Failed to load autostart setting:", e);
          return null;
        }),
      ]);

      configPath = config.configPath;
      rules = allRules;

      // Load spellcheck words
      customWords = config.spellcheckWords.join("\n");

      // Load typo checking setting (default to true if not present)
      typoCheckingEnabled = config.typoCheckingEnabled ?? true;

      if (autostart !== null) {
        autostartEnabled = autostart;
      }

      // Load AI grammar settings
      aiGrammarEnabled = config.aiGrammarEnabled ?? false;
      openaiApiKey = config.openaiApiKey ?? "";
      openaiModel = config.openaiModel ?? "gpt-4.1-mini";
      aiMaxInputChars = config.aiMaxInputChars ?? 2000;
      aiTimeoutMs = config.aiTimeoutMs ?? 12000;
      aiApiBaseUrl =
        config.aiApiBaseUrl ?? "https://openrouter.ai/api/v1/chat/completions";
      aiTranslateTargetLanguage = config.aiTranslateTargetLanguage ?? "English";
      aiPolishStyles = config.aiPolishStyles?.length ? config.aiPolishStyles : ["formal"];
      uiLanguage = config.uiLanguage === "zh-CN" ? "zh-CN" : "en";
      setLocale(uiLanguage);
      underlineStyle = config.underlineStyle ?? "wavy";
      underlineColor = config.underlineColor ?? "#ff3b30";

      hasUnsavedChanges = false;
    } catch (error) {
      console.error("Failed to load config:", error);
      loadError =
        error instanceof Error ? error.message : tr("settings.configError");
    } finally {
      isLoading = false;
    }
  }

  // Save autostart setting when toggled
  async function toggleAutostart() {
    try {
      await invoke("set_autostart_enabled", { enabled: autostartEnabled });
    } catch (e) {
      console.error("Failed to toggle autostart:", e);
    }
  }

  async function saveConfiguration() {
    isLoading = true;
    loadError = null;
    try {
      // Build rule updates (only include changed rules)
      const rulesUpdate: Record<string, number | null> = {};

      for (const rule of rules) {
        // If severity differs from default, include it; otherwise set to null to reset
        if (rule.severity !== rule.defaultSeverity) {
          rulesUpdate[rule.name] = rule.severity;
        } else {
          rulesUpdate[rule.name] = null;
        }
      }

      // Parse custom words
      const wordsArray = customWords
        .split("\n")
        .map((w) => w.trim())
        .filter((w) => w.length > 0);

      // Send update to backend
      await invoke("update_config", {
        updates: {
          rules: rulesUpdate,
          spellcheckWords: wordsArray,
          typoCheckingEnabled: typoCheckingEnabled,
          aiGrammarEnabled: aiGrammarEnabled,
          openaiApiKey: openaiApiKey,
          openaiModel: openaiModel,
          aiMaxInputChars: aiMaxInputChars,
          aiTimeoutMs: aiTimeoutMs,
          aiApiBaseUrl: aiApiBaseUrl,
          aiTranslateTargetLanguage: aiTranslateTargetLanguage,
          aiPolishStyle: aiPolishStyles,
          uiLanguage: uiLanguage,
          underlineStyle: underlineStyle,
          underlineColor: underlineColor,
        },
      });

      // Reload config to reflect saved state
      await loadConfiguration();

      // Show success feedback
      saveSuccess = true;
      setTimeout(() => {
        saveSuccess = false;
      }, 2000);
    } catch (error) {
      console.error("Failed to save config:", error);
      loadError =
        error instanceof Error ? error.message : tr("settings.configError");
    } finally {
      isLoading = false;
    }
  }

  async function exportConfig() {
    try {
      const defaultConfig = await invoke<string>("get_default_config");
      const blob = new Blob([defaultConfig], { type: "text/yaml" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = ".autocorrectrc";
      a.click();
      URL.revokeObjectURL(url);
    } catch (error) {
      console.error("Failed to export config:", error);
      loadError = tr("settings.configError");
    }
  }

  async function importConfig() {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = ".autocorrectrc,.yaml,.yml,.txt";

    input.onchange = async (e) => {
      const file = (e.target as HTMLInputElement).files?.[0];
      if (file) {
        try {
          const content = await file.text();
          // Use the existing save_config command to import
          await invoke("save_config", { content });
          // Reload to show new state
          await loadConfiguration();
        } catch (error) {
          console.error("Failed to import config:", error);
          loadError = tr("settings.configError");
        }
      }
    };

    input.click();
  }

  async function resetToDefaults() {
    isLoading = true;
    try {
      // Reset all rules to their defaults
      for (const rule of rules) {
        rule.severity = rule.defaultSeverity;
      }
      customWords = "";
      aiGrammarEnabled = false;
      openaiApiKey = "";
      openaiModel = "gpt-4.1-mini";
      aiMaxInputChars = 2000;
      aiTimeoutMs = 12000;
      aiApiBaseUrl = "https://openrouter.ai/api/v1/chat/completions";
      aiTranslateTargetLanguage = "English";
      aiPolishStyles = ["formal"];
      underlineStyle = "wavy";
      underlineColor = "#ff3b30";
      await saveConfiguration();
    } catch (error) {
      console.error("Failed to reset config:", error);
      loadError = tr("settings.configError");
    } finally {
      isLoading = false;
    }
  }

  function getSeverityLabel(severity: number): string {
    switch (severity) {
      case 0:
        return tr("settings.off");
      case 1:
        return tr("settings.error");
      case 2:
        return tr("settings.warn");
      default:
        return tr("settings.off");
    }
  }

  function getSeverityColor(severity: number): string {
    switch (severity) {
      case 0:
        return "text-gray-500";
      case 1:
        return "text-red-600";
      case 2:
        return "text-yellow-600";
      default:
        return "text-gray-500";
    }
  }

  // Load configuration on mount
  onMount(() => {
    const frameId = requestAnimationFrame(() => {
      void loadConfiguration();
      void loadHotkeyConfiguration();
    });

    return () => {
      cancelAnimationFrame(frameId);
    };
  });

  async function loadHotkeyConfiguration() {
    try {
      const config = await invoke<HotkeyConfig>("get_hotkey_config");
      hotkeyConfig = config;
    } catch (error) {
      console.error("Failed to load hotkey config:", error);
    }
  }

  async function startRecording() {
    isRecording = true;
    recordingError = null;
    recordedShortcut = null;

    // Clear any existing timeout
    if (recordingTimeout) {
      clearTimeout(recordingTimeout);
    }

    // Set a timeout to auto-cancel recording after 10 seconds
    recordingTimeout = setTimeout(() => {
      if (isRecording) {
        isRecording = false;
        recordingError = tr("settings.recordTimeout");
        // Remove keyboard listener
        document.removeEventListener("keydown", handleKeyPress);
      }
    }, 10000);

    // Add keyboard event listener
    document.addEventListener("keydown", handleKeyPress);
  }

  function handleKeyPress(e: KeyboardEvent) {
    if (!isRecording) {
      document.removeEventListener("keydown", handleKeyPress);
      return;
    }

    // Ignore modifier keys by themselves - we only want the final key
    // The modifiers are captured from e.shiftKey, e.metaKey, etc.
    const isModifierKey =
      e.key === "Shift" ||
      e.key === "Control" ||
      e.key === "Meta" ||
      e.key === "Alt" ||
      e.code === "ShiftLeft" ||
      e.code === "ShiftRight" ||
      e.code === "ControlLeft" ||
      e.code === "ControlRight" ||
      e.code === "MetaLeft" ||
      e.code === "MetaRight" ||
      e.code === "AltLeft" ||
      e.code === "AltRight";

    if (isModifierKey) {
      // Don't prevent default for modifier keys, just ignore them
      return;
    }

    e.preventDefault();
    e.stopPropagation();

    // Clear timeout
    if (recordingTimeout) {
      clearTimeout(recordingTimeout);
    }

    // Build modifiers from the event
    const modifiers: Modifiers = {
      shift: e.shiftKey,
      ctrl: e.ctrlKey,
      meta: e.metaKey,
      alt: e.altKey,
    };

    // Map keyboard event key to our key names
    let keyName = mapEventKeyToKeyName(e.key, e.code);

    if (!keyName) {
      recordingError = tr("settings.unsupportedKey", {
        key: e.key,
        code: e.code,
      });
      isRecording = false;
      document.removeEventListener("keydown", handleKeyPress);
      return;
    }

    // Create the recorded shortcut
    recordedShortcut = {
      key: keyName,
      modifiers,
      display_string: formatShortcutDisplay(modifiers, keyName),
    };

    isRecording = false;
    document.removeEventListener("keydown", handleKeyPress);
  }

  function mapEventKeyToKeyName(key: string, code: string): string | null {
    // Map event key/code to our key names
    // Prefer code over key for letter keys as it's more consistent
    if (code === "Space" || key === " ") return "Space";
    if (code === "Enter" || key === "Enter") return "Return";
    if (code === "Tab" || key === "Tab") return "Tab";
    if (code === "Backspace" || key === "Backspace") return "Backspace";
    if (code === "Delete" || key === "Delete") return "Backspace";
    if (code === "Escape" || key === "Escape") return "Escape";

    // Function keys - use key as it's more reliable
    if (key.startsWith("F") && key.length <= 3) {
      const num = parseInt(key.substring(1));
      if (num >= 1 && num <= 12) return key;
    }

    // Letter keys - use code which is consistent (e.g., "KeyK" for 'k' key)
    if (code.startsWith("Key") && code.length === 4) {
      return code; // e.g., "KeyK" -> "KeyK"
    }

    // Fallback: try using key for letters
    if (/^[a-zA-Z]$/.test(key)) {
      return "Key" + key.toUpperCase();
    }

    // Number keys
    if (/^[0-9]$/.test(key)) {
      return "Num" + key;
    }
    if (code.startsWith("Digit")) {
      return "Num" + code.substring(5);
    }

    return null;
  }

  function formatShortcutDisplay(modifiers: Modifiers, key: string): string {
    const parts: string[] = [];

    if (modifiers.meta) parts.push("⌘");
    if (modifiers.shift) parts.push("⇧");
    if (modifiers.alt) parts.push("⌥");
    if (modifiers.ctrl) parts.push("⌃");

    // Format the key nicely
    let keyLabel = key;
    if (key === "Space") keyLabel = "Space";
    else if (key === "Return") keyLabel = "Return";
    else if (key === "Tab") keyLabel = "Tab";
    else if (key === "Backspace") keyLabel = "⌫";
    else if (key === "Escape") keyLabel = "Esc";
    else if (key.startsWith("Key")) keyLabel = key.substring(3);
    else if (key.startsWith("Num")) keyLabel = key.substring(3);
    else if (key.startsWith("F")) keyLabel = key;

    parts.push(keyLabel);
    return parts.join("+");
  }

  async function saveRecordedShortcut() {
    if (!recordedShortcut) return;

    try {
      const newConfig = await invoke<HotkeyConfig>("update_hotkey_config", {
        request: {
          key: recordedShortcut.key,
          modifiers: recordedShortcut.modifiers,
        },
      });
      hotkeyConfig = newConfig;
      showKeySelector = false;
      recordedShortcut = null;
    } catch (error) {
      console.error("Failed to save hotkey config:", error);
      loadError = tr("settings.configError");
    }
  }

  async function resetHotkeyToDefaults() {
    try {
      const defaultConfig = await invoke<HotkeyConfig>("reset_hotkey_config");
      hotkeyConfig = defaultConfig;
      showKeySelector = false;
    } catch (error) {
      console.error("Failed to reset hotkey config:", error);
      loadError = tr("settings.configError");
    }
  }

  function cancelRecording() {
    isRecording = false;
    recordedShortcut = null;
    recordingError = null;
    if (recordingTimeout) {
      clearTimeout(recordingTimeout);
      recordingTimeout = null;
    }
    // Remove keyboard listener
    document.removeEventListener("keydown", handleKeyPress);
  }
</script>

<div class="flex flex-col gap-4 p-6" data-locale={$locale}>
  {#if loadError}
    <div
      class="rounded-lg border border-red-200 bg-red-50 p-4 dark:border-red-800 dark:bg-red-950"
    >
      <div class="flex items-start gap-3">
        <AlertCircle
          class="h-5 w-5 text-red-600 dark:text-red-400 flex-shrink-0 mt-0.5"
        />
        <div class="flex-1">
          <h4 class="text-sm font-semibold text-red-900 dark:text-red-100">
            {tr("settings.configError")}
          </h4>
          <p class="mt-1 text-sm text-red-700 dark:text-red-300">{loadError}</p>
        </div>
        <button
          onclick={() => (loadError = null)}
          class="text-red-600 hover:text-red-800 dark:text-red-400 dark:hover:text-red-200"
        >
          &times;
        </button>
      </div>
    </div>
  {/if}

  <Card>
    <CardHeader>
      <CardTitle>{tr("settings.title")}</CardTitle>
      <CardDescription>
        {tr("settings.desc", { path: configPath || tr("settings.loading") })}
        {#if hasUnsavedChanges}
          <span class="ml-2 text-amber-600 dark:text-amber-400"
            >{tr("settings.unsaved")}</span
          >
        {/if}
      </CardDescription>
    </CardHeader>
    <CardContent class="space-y-6">
      <!-- Language -->
      <div class="space-y-2">
        <h3 class="text-sm font-semibold">{tr("settings.language")}</h3>
        <p class="text-xs text-muted-foreground">
          {tr("settings.languageDesc")}
        </p>
        <select
          bind:value={uiLanguage}
          onchange={() => {
            setLocale(uiLanguage);
            hasUnsavedChanges = true;
          }}
          class="border-input bg-background ring-offset-background focus-visible:border-ring focus-visible:ring-ring/50 flex h-9 w-full max-w-xs min-w-0 rounded-md border px-3 py-1 text-sm outline-none focus-visible:ring-[3px]"
        >
          <option value="en">{tr("settings.lang.en")}</option>
          <option value="zh-CN">{tr("settings.lang.zh")}</option>
        </select>
      </div>

      <!-- Rule Toggles with Severity -->
      <div class="space-y-4">
        <div class="flex items-center justify-between">
          <h3 class="text-sm font-semibold">{tr("settings.rules")}</h3>
          <div class="flex gap-2 text-xs">
            <span class="flex items-center gap-1">
              <span class="h-3 w-3 rounded-full bg-red-600"></span>
              {tr("settings.error")}
            </span>
            <span class="flex items-center gap-1">
              <span class="h-3 w-3 rounded-full bg-yellow-600"></span>
              {tr("settings.warn")}
            </span>
            <span class="flex items-center gap-1">
              <span class="h-3 w-3 rounded-full bg-gray-400"></span>
              {tr("settings.off")}
            </span>
          </div>
        </div>
        <div class="space-y-3">
          {#each rules as rule}
            <div class="flex items-start justify-between rounded-lg border p-3">
              <div class="space-y-0.5 flex-1">
                <div class="flex items-center gap-2">
                  <span class="text-sm font-medium">{rule.name}</span>
                  <span
                    class="text-xs font-mono {getSeverityColor(rule.severity)}"
                  >
                    {getSeverityLabel(rule.severity)}
                  </span>
                  {#if rule.severity !== rule.defaultSeverity}
                    <span class="text-xs text-muted-foreground"
                      >({tr("settings.default")}: {getSeverityLabel(
                        rule.defaultSeverity,
                      )})</span
                    >
                  {/if}
                </div>
                <p class="text-xs text-muted-foreground">{rule.description}</p>
              </div>
              <div class="flex gap-1">
                <button
                  onclick={() => {
                    rule.severity = 0;
                    hasUnsavedChanges = true;
                  }}
                  class="rounded px-2 py-1 text-xs font-medium transition-colors {rule.severity ===
                  0
                    ? 'bg-gray-600 text-white'
                    : 'bg-gray-200 text-gray-700 hover:bg-gray-300 dark:bg-gray-700 dark:text-gray-300 dark:hover:bg-gray-600'}"
                  title={tr("settings.off")}
                >
                  {tr("settings.off")}
                </button>
                <button
                  onclick={() => {
                    rule.severity = 1;
                    hasUnsavedChanges = true;
                  }}
                  class="rounded px-2 py-1 text-xs font-medium transition-colors {rule.severity ===
                  1
                    ? 'bg-red-600 text-white'
                    : 'bg-red-200 text-red-700 hover:bg-red-300 dark:bg-red-900 dark:text-red-300 dark:hover:bg-red-800'}"
                  title={tr("settings.error")}
                >
                  {tr("settings.error")}
                </button>
                <button
                  onclick={() => {
                    rule.severity = 2;
                    hasUnsavedChanges = true;
                  }}
                  class="rounded px-2 py-1 text-xs font-medium transition-colors {rule.severity ===
                  2
                    ? 'bg-yellow-600 text-white'
                    : 'bg-yellow-200 text-yellow-700 hover:bg-yellow-300 dark:bg-yellow-900 dark:text-yellow-300 dark:hover:bg-yellow-800'}"
                  title={tr("settings.warn")}
                >
                  {tr("settings.warn")}
                </button>
              </div>
            </div>
          {/each}
        </div>
      </div>

      <!-- Appearance -->
      <div class="space-y-3">
        <h3 class="text-sm font-semibold">{tr("settings.appearance")}</h3>

        <div class="rounded-lg border p-3 space-y-4">
          <!-- Theme Selector -->
          <div class="space-y-2">
            <p class="text-sm font-medium">{tr("settings.theme")}</p>
            <select
              value={theme}
              onchange={(e) => {
                const target = e.currentTarget;
                onThemeChange(target.value as ThemeMode);
              }}
              class="border-input bg-background ring-offset-background focus-visible:border-ring focus-visible:ring-ring/50 flex h-9 w-full max-w-xs min-w-0 rounded-md border px-3 py-1 text-sm outline-none focus-visible:ring-[3px]"
            >
              <option value="light">{tr("settings.theme.light")}</option>
              <option value="dark">{tr("settings.theme.dark")}</option>
              <option value="auto">{tr("settings.theme.auto")}</option>
            </select>
            {#if theme === 'auto'}
              <p class="text-xs text-muted-foreground">
                {tr("settings.theme.autoDesc")}
              </p>
            {/if}
          </div>

          <!-- Underline Style -->
          <div class="space-y-2">
            <p class="text-sm font-medium">{tr("settings.underlineStyle")}</p>
            <div class="flex gap-2">
              {#each UNDERLINE_STYLES as s}
                <button
                  class="px-3 py-1.5 rounded-md border text-xs font-medium transition-colors
										{underlineStyle === s.value
                    ? 'border-primary bg-primary text-primary-foreground'
                    : 'border-border bg-background hover:bg-muted'}"
                  onclick={() => {
                    underlineStyle = s.value;
                    hasUnsavedChanges = true;
                  }}>{tr(s.labelKey)}</button
                >
              {/each}
            </div>
          </div>

          <!-- Underline Color -->
          <div class="space-y-2">
            <p class="text-sm font-medium">{tr("settings.underlineColor")}</p>
            <div class="flex gap-2 flex-wrap">
              {#each UNDERLINE_COLORS as c}
                <button
                  class="w-7 h-7 rounded-full border-2 transition-all {c.tw}
										{underlineColor === c.value
                    ? 'border-foreground scale-110'
                    : 'border-transparent'}"
                  title={tr(c.labelKey)}
                  onclick={() => {
                    underlineColor = c.value;
                    hasUnsavedChanges = true;
                  }}
                ></button>
              {/each}
            </div>
          </div>

          <!-- Preview -->
          <div class="space-y-1">
            <p class="text-xs text-muted-foreground">{tr("settings.preview")}</p>
            <span
              class="text-sm"
              style="text-decoration: underline; text-decoration-style: {underlineStyle ===
              'wavy'
                ? 'wavy'
                : underlineStyle}; text-decoration-color: {underlineColor};"
            >
              {tr("settings.sampleTypo")}
            </span>
          </div>
        </div>
      </div>

      <!-- General Settings -->
      <div class="space-y-3">
        <h3 class="text-sm font-semibold">{tr("settings.general") || "General"}</h3>

        <!-- Launch at Login -->
        <div class="flex items-center justify-between rounded-lg border p-3">
          <div class="space-y-0.5">
            <label class="text-sm font-medium" for="autostart-enabled"
              >{tr("settings.autostart")}</label
            >
            <p class="text-xs text-muted-foreground">
              {tr("settings.autostartDesc")}
            </p>
          </div>
          <Switch
            bind:checked={autostartEnabled}
            id="autostart-enabled"
            onchange={() => toggleAutostart()}
          />
        </div>
      </div>

      <!-- Custom Words -->
      <div class="space-y-3">
        <h3 class="text-sm font-semibold">{tr("settings.spellConfig")}</h3>

        <!-- Enable/Disable Typo Checking -->
        <div class="flex items-center justify-between rounded-lg border p-3">
          <div class="space-y-0.5">
            <label class="text-sm font-medium" for="typo-checking-enabled"
              >{tr("settings.typoToggle")}</label
            >
            <p class="text-xs text-muted-foreground">
              {tr("settings.typoToggleDesc")}
            </p>
          </div>
          <Switch
            bind:checked={typoCheckingEnabled}
            id="typo-checking-enabled"
            onchange={() => (hasUnsavedChanges = true)}
          />
        </div>

        <!-- AI Grammar -->
        <div class="flex items-center justify-between rounded-lg border p-3">
          <div class="space-y-0.5">
            <label class="text-sm font-medium" for="ai-grammar-enabled"
              >{tr("settings.aiToggle")}</label
            >
            <p class="text-xs text-muted-foreground">
              {tr("settings.aiToggleDesc")}
            </p>
          </div>
          <Switch
            bind:checked={aiGrammarEnabled}
            id="ai-grammar-enabled"
            onchange={() => (hasUnsavedChanges = true)}
          />
        </div>

        {#if aiGrammarEnabled}
          <div class="rounded-lg border p-4 space-y-3">
            <div class="space-y-1">
              <label class="text-sm font-medium" for="openai-api-key"
                >{tr("settings.apiKey")}</label
              >
              <Input
                id="openai-api-key"
                type="password"
                bind:value={openaiApiKey}
                placeholder="sk-..."
                oninput={() => (hasUnsavedChanges = true)}
              />
            </div>
            <div class="grid grid-cols-1 md:grid-cols-3 gap-3">
              <div class="space-y-1">
                <label class="text-sm font-medium" for="openai-model"
                  >{tr("settings.model")}</label
                >
                <Input
                  id="openai-model"
                  bind:value={openaiModel}
                  placeholder="gpt-4.1-mini"
                  oninput={() => (hasUnsavedChanges = true)}
                />
              </div>
              <div class="space-y-1">
                <label class="text-sm font-medium" for="ai-max-input"
                  >{tr("settings.maxInput")}</label
                >
                <Input
                  id="ai-max-input"
                  type="number"
                  bind:value={aiMaxInputChars}
                  min="200"
                  max="20000"
                  oninput={() => (hasUnsavedChanges = true)}
                />
              </div>
              <div class="space-y-1">
                <label class="text-sm font-medium" for="ai-timeout-ms"
                  >{tr("settings.timeout")}</label
                >
                <Input
                  id="ai-timeout-ms"
                  type="number"
                  bind:value={aiTimeoutMs}
                  min="1000"
                  max="120000"
                  step="500"
                  oninput={() => (hasUnsavedChanges = true)}
                />
              </div>
            </div>
            <div class="grid grid-cols-1 md:grid-cols-3 gap-3">
              <div class="space-y-1 md:col-span-2">
                <label class="text-sm font-medium" for="ai-api-base-url"
                  >{tr("settings.endpoint")}</label
                >
                <Input
                  id="ai-api-base-url"
                  bind:value={aiApiBaseUrl}
                  placeholder="https://openrouter.ai/api/v1/chat/completions"
                  oninput={() => (hasUnsavedChanges = true)}
                />
              </div>
              <div class="space-y-1">
                <label class="text-sm font-medium" for="ai-target-language"
                  >{tr("settings.targetLanguage")}</label
                >
                <select
                  id="ai-target-language"
                  bind:value={aiTranslateTargetLanguage}
                  onchange={() => (hasUnsavedChanges = true)}
                  class="border-input bg-background ring-offset-background focus-visible:border-ring focus-visible:ring-ring/50 flex h-9 w-full min-w-0 rounded-md border px-3 py-1 text-sm outline-none focus-visible:ring-[3px]"
                >
                  {#each translateLanguageOptions as language}
                    <option value={language}>{language}</option>
                  {/each}
                </select>
              </div>
            </div>
            <div class="space-y-2">
              <span class="text-sm font-medium">
                {tr("settings.polishStyle")}
              </span>
              <div class="flex flex-wrap gap-2">
                {#each POLISH_STYLES as style}
                  <button
                    type="button"
                    class="inline-flex items-center rounded-md border border-input bg-background px-3 py-1.5 text-sm transition-colors hover:bg-accent hover:text-accent-foreground"
                    class:bg-primary={aiPolishStyles.includes(style.value)}
                    class:text-primary-foreground={aiPolishStyles.includes(style.value)}
                    class:border-primary={aiPolishStyles.includes(style.value)}
                    onclick={() => {
                      if (aiPolishStyles.includes(style.value)) {
                        aiPolishStyles = aiPolishStyles.filter((s) => s !== style.value);
                      } else {
                        aiPolishStyles = [...aiPolishStyles, style.value];
                      }
                      hasUnsavedChanges = true;
                    }}
                  >
                    {tr(style.labelKey)}
                  </button>
                {/each}
              </div>
            </div>
          </div>
        {/if}

        <!-- Custom Words Textarea -->
        <div class="rounded-lg border p-3">
          <label for="custom-words" class="mb-2 block text-sm font-medium">
            {tr("settings.customDict")}
          </label>
          <Textarea
            id="custom-words"
            bind:value={customWords}
            oninput={() => (hasUnsavedChanges = true)}
            placeholder={tr("settings.customDictPlaceholder")}
            class="min-h-[100px] font-mono text-sm"
          />
          <p class="mt-1 text-xs text-muted-foreground">
            {tr("settings.customDictDesc")}
          </p>
        </div>

        <!-- Custom Typo Corrections Manager -->
        <div class="rounded-lg border p-4">
          <CustomCorrectionsManager />
        </div>

        <!-- Ignored Apps Manager -->
        <div class="rounded-lg border p-4">
          <IgnoredAppsManager />
        </div>
      </div>

      <!-- Hotkey Configuration -->
      <div class="space-y-3">
        <h3 class="text-sm font-semibold">{tr("settings.hotkey")}</h3>
        <div class="flex items-center justify-between rounded-lg border p-3">
          <div class="space-y-0.5">
            <label class="text-sm font-medium" for="hotkey-enabled"
              >{tr("settings.hotkeyEnable")}</label
            >
            <p class="text-xs text-muted-foreground">
              {tr("settings.hotkeyDesc")}
            </p>
          </div>
          <Switch bind:checked={hotkeyEnabled} id="hotkey-enabled" />
        </div>

        {#if hotkeyEnabled}
          <!-- Current Hotkey Display -->
          <div class="rounded-lg border p-3">
            <div class="mb-3 flex items-center justify-between">
              <span class="text-sm font-medium"
                >{tr("settings.currentHotkey")}</span
              >
              {#if hotkeyConfig}
                <div
                  class="flex items-center gap-2 rounded-md bg-muted px-3 py-1.5 font-mono text-sm"
                >
                  <Keyboard class="h-4 w-4" />
                  <span>{hotkeyConfig.display_string}</span>
                </div>
              {:else}
                <div class="text-sm text-muted-foreground">
                  {tr("settings.loading")}
                </div>
              {/if}
            </div>

            <!-- Change Hotkey Button -->
            {#if !showKeySelector}
              <div class="flex gap-2">
                <button
                  onclick={() => (showKeySelector = true)}
                  class="rounded-md bg-primary px-3 py-1.5 text-sm font-medium text-primary-foreground hover:bg-primary/90"
                >
                  {tr("settings.changeHotkey")}
                </button>
                <button
                  onclick={resetHotkeyToDefaults}
                  class="rounded-md border px-3 py-1.5 text-sm font-medium hover:bg-muted"
                >
                  {tr("settings.resetDefault")}
                </button>
              </div>
            {/if}
          </div>

          <!-- Shortcut Recording UI -->
          {#if showKeySelector}
            <div class="space-y-3 rounded-lg border p-3">
              <h4 class="text-sm font-medium">
                {tr("settings.changeHotkeyTitle")}
              </h4>

              {#if !isRecording && !recordedShortcut}
                <!-- Recording Instructions -->
                <div class="rounded-md bg-muted p-4 text-center">
                  <p class="text-sm text-muted-foreground">
                    {tr("settings.recordHint")}
                    <br />
                    {tr("settings.recordHint2")}
                    <span class="font-mono">⌘+⇧+S</span>
                  </p>
                  <button
                    onclick={startRecording}
                    class="mt-3 rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90"
                  >
                    <Keyboard class="mr-2 h-4 w-4 inline" />
                    {tr("settings.startRecording")}
                  </button>
                </div>
              {/if}

              {#if isRecording}
                <!-- Recording State -->
                <div
                  class="rounded-md border-2 border-primary bg-primary/5 p-6 text-center"
                >
                  <div class="mb-3 flex justify-center">
                    <div
                      class="flex h-8 w-8 animate-pulse items-center justify-center rounded-full bg-primary"
                    >
                      <Keyboard class="h-4 w-4 text-primary-foreground" />
                    </div>
                  </div>
                  <p class="text-sm font-medium">{tr("settings.recording")}</p>
                  <p class="mt-1 text-xs text-muted-foreground">
                    {tr("settings.recordingDesc")}
                  </p>
                  <button
                    onclick={cancelRecording}
                    class="mt-3 rounded-md border px-3 py-1.5 text-sm font-medium hover:bg-muted"
                  >
                    {tr("settings.cancel")}
                  </button>
                </div>
              {/if}

              {#if recordedShortcut}
                <!-- Recorded Result -->
                <div class="rounded-md bg-muted p-4 text-center">
                  <p class="text-xs font-medium text-muted-foreground">
                    {tr("settings.recorded")}
                  </p>
                  <p class="mt-2 font-mono text-lg">
                    {recordedShortcut.display_string}
                  </p>
                  <div class="mt-4 flex justify-center gap-2">
                    <button
                      onclick={saveRecordedShortcut}
                      class="rounded-md bg-primary px-3 py-1.5 text-sm font-medium text-primary-foreground hover:bg-primary/90"
                    >
                      {tr("settings.saveHotkey")}
                    </button>
                    <button
                      onclick={() => (recordedShortcut = null)}
                      class="rounded-md border px-3 py-1.5 text-sm font-medium hover:bg-muted"
                    >
                      {tr("settings.tryAgain")}
                    </button>
                  </div>
                </div>
              {/if}

              {#if recordingError}
                <!-- Error State -->
                <div
                  class="rounded-md border border-red-200 bg-red-50 p-4 text-center dark:border-red-800 dark:bg-red-950"
                >
                  <p class="text-sm font-medium text-red-900 dark:text-red-100">
                    {recordingError}
                  </p>
                  <div class="mt-3 flex justify-center gap-2">
                    <button
                      onclick={() => {
                        recordingError = null;
                        startRecording();
                      }}
                      class="rounded-md bg-primary px-3 py-1.5 text-sm font-medium text-primary-foreground hover:bg-primary/90"
                    >
                      {tr("settings.tryAgain")}
                    </button>
                    <button
                      onclick={() => {
                        recordingError = null;
                        showKeySelector = false;
                      }}
                      class="rounded-md border px-3 py-1.5 text-sm font-medium hover:bg-muted"
                    >
                      {tr("settings.cancel")}
                    </button>
                  </div>
                </div>
              {/if}

              <!-- Cancel Button (bottom) -->
              {#if !isRecording && !recordedShortcut && !recordingError}
                <button
                  onclick={() => (showKeySelector = false)}
                  class="w-full rounded-md border px-3 py-1.5 text-sm font-medium hover:bg-muted"
                >
                  {tr("settings.cancel")}
                </button>
              {/if}
            </div>
          {/if}
        {/if}
      </div>

      <!-- Config Import/Export -->
      <div class="space-y-3">
        <h3 class="text-sm font-semibold">{tr("settings.configuration")}</h3>
        <div class="flex flex-wrap gap-2">
          <Button
            onclick={saveConfiguration}
            variant="default"
            disabled={isLoading || !hasUnsavedChanges}
            class={saveSuccess ? "bg-green-600 hover:bg-green-700" : ""}
          >
            {#if isLoading}
              <div
                class="mr-2 h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent"
              ></div>
            {:else}
              <Save class="mr-2 h-4 w-4" />
            {/if}
            {saveSuccess ? tr("settings.saved") : tr("settings.saveChanges")}
          </Button>
          <Button
            onclick={resetToDefaults}
            variant="outline"
            disabled={isLoading}
          >
            <RotateCcw class="mr-2 h-4 w-4" />
            {tr("settings.resetDefaults")}
          </Button>
          <Button onclick={exportConfig} variant="outline" disabled={isLoading}>
            <Download class="mr-2 h-4 w-4" />
            {tr("settings.exportDefault")}
          </Button>
          <Button onclick={importConfig} variant="outline" disabled={isLoading}>
            <Upload class="mr-2 h-4 w-4" />
            {tr("settings.importConfig")}
          </Button>
        </div>
      </div>
    </CardContent>
  </Card>
</div>
