# Unified Theme Management Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement unified theme management using Tauri's `app.setTheme()` API with store persistence and automatic synchronization across all windows (App.svelte, SettingsPanel.svelte, Popup.svelte, AiPopup.svelte).

**Architecture:** App.svelte manages theme state and broadcasts to all windows via Tauri IPC. SettingsPanel triggers theme changes via Tauri command. All windows listen for theme-changed events and apply theme to DOM. Theme persists in Tauri store (app-settings.json) with localStorage fallback.

**Tech Stack:** Rust (Tauri 2.10, tauri-plugin-store), Svelte 5 runes, TypeScript, Tailwind CSS 4, window.matchMedia API

---

## File Structure

- **Modify:**
  - `src-tauri/src/lib.rs` - Register theme commands
  - `src/App.svelte` - Add theme management logic and event handling
  - `src/lib/components/SettingsPanel.svelte` - Refactor to UI-only theme selector
  - `src/lib/i18n/messages.ts` - Add theme translation keys
  - `src/pages/popup/Popup.svelte` - Add theme support
  - `src/pages/ai-popup/AiPopup.svelte` - Add theme support

---

## Chunk 1: Backend - Register Theme Commands

### Task 1: Register theme commands in lib.rs

**Files:**

- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add theme module declarations**

```rust
mod theme;
mod theme_errors;
```

- [ ] **Step 2: Register theme commands in invoke_handler**

Find the `invoke_handler` call (around line 516) and add theme commands:

```rust
.invoke_handler(tauri::generate_handler![
    // ... existing commands
    get_theme,
    set_theme,
])
```

- [ ] **Step 3: Run cargo check**

```bash
cd src-tauri && cargo check --message="Register theme commands"
```

Expected: No errors

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "chore(rust): register theme commands in lib.rs"
```

---

## Chunk 2: Frontend - Add i18n Translation Keys

### Task 2: Add theme translation keys

**Files:**

- Modify: `src/lib/i18n/messages.ts`

- [ ] **Step 1: Add theme translation keys**

```typescript
{
  en: {
    // ... existing keys
    "settings.theme": "Theme",
    "settings.theme.light": "Light",
    "settings.theme.dark": "Dark",
    "settings.theme.auto": "Auto",
    "settings.theme.autoDesc": "Follows system preference"
  },
  "zh-CN": {
    // ... existing keys
    "settings.theme": "主题",
    "settings.theme.light": "浅色",
    "settings.theme.dark": "深色",
    "settings.theme.auto": "自动",
    "settings.theme.autoDesc": "跟随系统偏好"
  }
}
```

- [ ] **Step 2: Verify TypeScript compilation**

```bash
npm run check
```

Expected: No type errors

- [ ] **Step 3: Commit**

```bash
git add src/lib/i18n/messages.ts
git commit -m "feat(i18n): add theme translation keys"
```

---

## Chunk 3: Frontend - Refactor App.svelte

### Task 3: Add theme management to App.svelte

**Files:**

- Modify: `src/App.svelte`

- [ ] **Step 1: Add theme type and state**

Add before onMount:

```typescript
// Theme type
type ThemeMode = "light" | "dark" | "auto";

// Theme state
let theme: ThemeMode = $state("auto");
```

- [ ] **Step 2: Add theme management functions**

Add after state declarations:

```typescript
  // System theme listener
  let mediaQuery: MediaQueryList | null = null;

  // Load theme from Tauri store
  async function loadThemeFromStore(): Promise<ThemeMode> {
    try {
      const stored = await invoke<ThemeMode>("get_theme");
      if (stored === "light" || stored === "dark" || stored === "auto") {
        return stored;
      }
    } catch (e) {
      console.warn("Failed to load theme from store:", e);
    }

    return "auto";
  }

  // Apply theme to DOM
  function applyThemeToDom(mode: ThemeMode) {
    const html = document.documentElement;

    if (mode === "dark") {
      html.classList.add("dark");
    } else if (mode === "auto") {
      const prefersDark = window.matchMedia(
        "(prefers-color-scheme: dark)",
      ).matches;
      html.classList.toggle("dark", prefersDark);
    } else {
      html.classList.remove("dark");
    }
  }

  // Setup system theme listener
  function setupSystemThemeListener() {
    if (mediaQuery) {
      mediaQuery.removeEventListener("change", handleSystemThemeChange);
    }

    mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
    mediaQuery.addEventListener("change", handleSystemThemeChange);
  }

  function handleSystemThemeChange(e: MediaQueryListEvent) {
    if (theme === "auto") {
      const html = document.documentElement;
      html.classList.toggle("dark", e.matches);
    }
  }

  function cleanupThemeListener() {
    if (mediaQuery) {
      mediaQuery.removeEventListener("change", handleSystemThemeChange);
      mediaQuery = null;
    }
  }

  // Tauri event listener
  let unlistenThemeChanged = (() => void) | null = null;

  onMount(async () => {
    // Initialize theme from store
    const savedTheme = await loadThemeFromStore();
    theme = savedTheme;
    applyThemeToDom(savedTheme);

    // Setup system listener
    setupSystemThemeListener();

    // Listen for theme changes
    try {
      unlistenThemeChanged = await listen<ThemeMode>("theme-changed", (event) => {
        const newTheme = event.payload;
        theme = newTheme;
        applyThemeToDom(newTheme);
      });
    } catch (e) {
      console.error("Failed to listen for theme changes:", e);
    }
  });

  onDestroy(() => {
    cleanupThemeListener();

    if (unlistenThemeChanged) {
      unlistenThemeChanged();
      unlistenThemeChanged = null;
    }
  });
</script>
```

- [ ] **Step 3: Pass theme prop to SettingsPanel**

Find the SettingsPanel invocation (around line 100) and add theme prop:

```typescript
{#if currentTab === 'settings'}
  <SettingsPanel theme={theme} />
{/if}
```

- [ ] **Step 4: Verify TypeScript compilation**

```bash
npm run check
```

Expected: No type errors

- [ ] **Step 5: Commit**

```bash
git add src/App.svelte
git commit -m "refactor(app): add unified theme management to App.svelte"
```

---

## Chunk 4: Frontend - Refactor SettingsPanel.svelte

### Task 4: Simplify SettingsPanel to UI-only component

**Files:**

- Modify: `src/lib/components/SettingsPanel.svelte`

- [ ] **Step 1: Remove all theme management code**

Remove theme-related code:

- ThemeMode type
- theme state variable
- THEME_STORAGE_KEY constant
- loadTheme, applyTheme functions
- mediaQuery state and listeners
- setupSystemThemeListener, handleSystemThemeChange, cleanupThemeListener functions

- [ ] **Step 2: Add theme prop and change handler**

Add at top of script section:

```svelte
<script lang="ts">
  // Keep existing imports
  import { invoke } from "@tauri-apps/api/core";
  import { locale, t } from "$lib/i18n";
  $locale;

  const tr = $derived((key: string, params?: Record<string, string | number>) => {
    const _ = $locale;
    return t(key, params);
  };

  // Props received from parent
  export let theme: ThemeMode;

  async function onThemeChange(selectedTheme: ThemeMode) {
    try {
      await invoke("set_theme", { theme: selectedTheme });
    } catch (e) {
      console.error("Failed to set theme:", e);
    }
  }
</script>
```

- [ ] **Step 2: Keep theme selector UI**

Ensure theme selector uses theme prop:

```svelte
<!-- Theme Selector UI -->
<div class="space-y-2">
  <p class="text-sm font-medium">{tr("settings.theme")}</p>
  <select
    bind:value={theme}
    onchange={() => onThemeChange(theme)}
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
```

- [ ] **Step 3: Verify TypeScript compilation**

```bash
npm run check
```

Expected: No type errors

- [ ] **Step 4: Commit**

```bash
git add src/lib/components/SettingsPanel.svelte
git commit -m "refactor(app): simplify SettingsPanel to theme UI component"
```

---

## Chunk 5: Frontend - Update Popup.svelte

### Task 5: Add theme support to Popup.svelte

**Files:**

- Modify: `src/pages/popup/Popup.svelte`

- [ ] **Step 1: Add theme prop and apply logic**

Add at top of script section:

```svelte
<script lang="ts">
  import { onMount, onDestroy, listen } from "svelte";
  // Keep existing imports
  import { locale, t } from "$lib/i18n";
  import { ThemeMode } from "$lib/types/theme";
  $locale;

  const tr = $derived((key: string, params?: Record<string, string | number>) => {
    const _ = $locale;
    return t(key, params);
  };

  // Theme prop received from App.svelte
  export let theme: ThemeMode;

  function applyThemeToDom() {
    const html = document.documentElement;

    if (theme === "dark") {
      html.classList.add("dark");
    } else if (theme === "auto") {
      const prefersDark = window.matchMedia(
        "(prefers-color-scheme: dark)",
      ).matches;
      html.classList.toggle("dark", prefersDark);
    } else {
      html.classList.remove("dark");
    }
  }

  // Apply theme on mount
  onMount(() => {
    applyThemeToDom();
  });
</script>
```

- [ ] **Step 2: Verify TypeScript compilation**

```bash
npm run check
```

Expected: No type errors

- [ ] **Step 3: Commit**

```bash
git add src/pages/popup/Popup.svelte
git commit -m "feat(app): add theme support to Popup.svelte"
```

---

## Chunk 6: Frontend - Update AiPopup.svelte

### Task 6: Add theme support to AiPopup.svelte

**Files:**

- Modify: `src/pages/ai-popup/AiPopup.svelte`

- [ ] **Step 1: Add theme prop and apply logic**

Same approach as Popup.svelte:

```svelte
<script lang="ts">
  import { onMount, onDestroy, listen } from "svelte";
  // Keep existing imports
  import { locale, t } from "$lib/i18n";
  import { ThemeMode } from "$lib/types/theme";
  $locale;

  const tr = $derived((key: string, params?: Record<string, string | number>) => {
    const _ = $locale;
    return t(key, params);
  };

  // Theme prop received from App.svelte
  export let theme: ThemeMode;

  function applyThemeToDom() {
    const html = document.documentElement;

    if (theme === "dark") {
      html.classList.add("dark");
    } else if (theme === "auto") {
      const prefersDark = window.matchMedia(
        "(prefers-color-scheme: dark)",
      ).matches;
      html.classList.toggle("dark", prefersDark);
    } else {
      html.classList.remove("dark");
    }
  }

  // Apply theme on mount
  onMount(() => {
    applyThemeToDom();
  });
</script>
```

- [ ] **Step 2: Verify TypeScript compilation**

```bash
npm run check
```

Expected: No type errors

- [ ] **Step 3: Commit**

```bash
git add src/pages/ai-popup/AiPopup.svelte
git commit -m "feat(app): add theme support to AiPopup.svelte"
```

---

## Chunk 7: Testing

### Task 7: Test unified theme management

**Files:**

- No file changes (testing only)

- [ ] **Step 1: Run Rust tests**

```bash
cd src-tauri && cargo test --lib theme
```

Expected: All tests pass

- [ ] **Step 2: Test theme commands manually**

````bash
# Start Tauri dev
npm run tauri:dev

# Manual testing checklist:
# [ ] Test get_theme command returns stored theme from Tauri store
# [ ] Test get_theme command falls back to "auto" when store is empty
# [ ] Test set_theme command saves to Tauri store
# [ ] Test set_theme emits theme-changed event
# [ ] Test App.svelte initializes theme from store on mount
# [ ] Test App.svelte applies theme to DOM correctly (light/dark/auto)
# [ ] Test App.svelte listens for theme-changed events
# [ ] Test SettingsPanel displays theme selector
# [ ] Test SettingsPanel theme selector triggers set_theme
# [ ] Test theme changes sync to all windows
# [ ] Test Popup.svelte receives theme from App.svelte
# [ ] Test Popup.svelte applies theme correctly
# [ ] Test AiPopup.svelte receives theme from App.svelte
# [ ] Test AiPopup.svelte applies theme correctly
# [ ] Test auto mode follows system preference
# [ ] Test system theme changes update UI in real-time
# [ ] Test theme persists across app restarts

- [ ] **Step 3: Commit test results**

```bash
git commit --allow-empty -m "test(app): verify unified theme management works end-to-end"
````

---

## Testing Checklist

Backend:

- [ ] Theme commands registered in lib.rs
- [ ] get_theme command receives app_handle parameter correctly
- [ ] get_theme reads from Tauri store correctly
- [ ] get_theme returns "auto" as default
- [ ] get_theme handles store unavailability gracefully
- [ ] set_theme command receives app_handle parameter correctly
- [ ] set_theme validates theme value
- [ ] set_theme saves to Tauri store correctly
- [ ] set_theme emits theme-changed event

Frontend:

- [ ] ThemeMode type defined in components
- [ ] App.svelte initializes theme from store on mount
- [ ] App.svelte uses invoke to call get_theme
- [ ] App.svelte applies theme to DOM immediately on mount and changes
- [ ] App.svelte listens for theme-changed events
- [ ] App.svelte passes theme to SettingsPanel as prop
- [ ] SettingsPanel receives theme as prop
- [ ] SettingsPanel displays theme selector UI correctly
- [ ] SettingsPanel calls set_theme on theme change
- [ ] Popup.svelte receives theme as prop
- [ ] Popup.svelte applies theme on mount
- [ ] Popup.svelte applies theme on prop changes
- [ ] AiPopup.svelte receives theme as prop
- [ ] AiPopup.svelte applies theme on mount
- [ ] All windows (App, Settings, Popup, AiPopup) respect theme setting
- [ ] Theme persists across app restarts
- [ ] Theme changes sync to all windows via theme-changed event
- [ ] Auto mode follows system preference
- [ ] System theme changes update UI in real-time
- [ ] TypeScript compilation passes (npm run check)
- [ ] No console errors related to theme management

## Edge Cases Handled

- **Store unavailable:** get_theme returns "auto" as default
- **Invalid theme value:** set_theme returns error string
- **Race conditions:** Last write wins for store, applyThemeToDom handles concurrent calls
- **Memory leaks:** Proper cleanup of event listeners (MediaQuery and Tauri)

## Data Migration Notes

No explicit data migration needed. New installations use Tauri store exclusively. Existing installations with localStorage theme will lose their theme preference (no automatic migration).
