# Unified Theme Management Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement unified theme management using Tauri's `app.setTheme()` API with store persistence and automatic synchronization across all windows (App.svelte, SettingsPanel.svelte, Popup.svelte, AiPopup.svelte).

**Architecture:** App.svelte manages theme state and broadcasts to all windows via Tauri IPC. SettingsPanel triggers theme changes via Tauri command. All windows listen for theme-changed events and apply theme to DOM. Theme persists in Tauri store (app-settings.json) with localStorage fallback.

**Tech Stack:** Rust (Tauri 2.10, tauri-plugin-store), Svelte 5 runes, TypeScript, Tailwind CSS 4, window.matchMedia API

---

## File Structure

- **Create:** `src-tauri/src/commands/theme.rs`, `src-tauri/src/commands/theme_errors.rs`, `src/lib/types/theme.ts`
- **Modify:**
  - `src-tauri/Cargo.toml` - Add tauri-plugin-store dependency
  - `src-tauri/src/lib.rs` - Register theme commands and store plugin
  - `src/App.svelte` - Add theme management logic and event handling
  - `src/lib/components/SettingsPanel.svelte` - Refactor to UI-only theme selector
  - `src/pages/popup/Popup.svelte` - Add theme support
  - `src/pages/ai-popup/AiPopup.svelte` - Add theme support

---

## Chunk 1: Create Shared Type Definitions

### Task 1: Create theme type definitions file

**Files:**

- Create: `src/lib/types/theme.ts`

- [ ] **Step 1: Create ThemeMode type**

```typescript
export type ThemeMode = "light" | "dark" | "auto";
```

- [ ] **Step 2: Run TypeScript check**

```bash
npm run check
```

Expected: No type errors

- [ ] **Step 3: Commit**

```bash
git add src/lib/types/theme.ts
git commit -m "feat(types): add ThemeMode type definition"
```

---

## Chunk 2: Backend - Add Tauri Store Dependency

### Task 2: Update Cargo.toml

**Files:**

- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: Add tauri-plugin-store dependency**

```toml
[dependencies]
tauri-plugin-store = "2"
```

- [ ] **Step 2: Run cargo check**

```bash
cd src-tauri && cargo check
```

Expected: No errors or warnings

- [ ] **Step 3: Commit**

```bash
git add src-tauri/Cargo.toml
git commit -m "chore(rust): add tauri-plugin-store dependency for theme management"
```

---

## Chunk 3: Backend - Create Theme Commands Module

### Task 3: Create theme.rs module with get_theme command

**Files:**

- Create: `src-tauri/src/commands/theme.rs`

- [ ] **Step 1: Create module skeleton**

```rust
use tauri::Manager;
use crate::types::theme::ThemeMode;

/// Get current theme from Tauri store
#[tauri::command]
pub async fn get_theme(app: tauri::AppHandle) -> Result<String, String> {
    let store = app.store()?;

    // Try to read from Tauri store first
    if let Ok(Some(stored_theme)) = store.get("theme") {
        if let Ok(theme_str) = stored_theme.as_string() {
            return Ok(theme_str);
        }
    }

    Ok("auto".to_string())
}
```

- [ ] **Step 2: Run cargo check**

```bash
cd src-tauri && cargo check --message="Check theme.rs module"
```

Expected: No errors

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/commands/theme.rs
git commit -m "feat(rust): add theme.rs module with get_theme command"
```

---

## Chunk 4: Backend - Add set_theme Command

### Task 4: Implement set_theme command with validation

**Files:**

- Modify: `src-tauri/src/commands/theme.rs`
- Create: `src-tauri/src/commands/theme_errors.rs`

- [ ] **Step 1: Add ThemeError type**

```rust
use thiserror::Error;

#[derive(Debug, thiserror::Error)]
pub enum ThemeError {
    #[error("Theme '{0}' is not valid")]
    InvalidValue(String),
}
```

- [ ] **Step 2: Add set_theme function**

```rust
use serde::{Deserialize, Serialize};
use super::theme_errors::ThemeError;
use crate::types::theme::ThemeMode;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeChangePayload {
    pub theme: String,
}

/// Set theme in Tauri store and emit change event
#[tauri::command]
pub async fn set_theme(app: tauri::AppHandle, theme: ThemeMode) -> Result<(), ThemeError> {
    let store = app.store()?;

    // Validate theme value
    if !matches!(theme.as_str(), "light" | "dark" | "auto") {
        return Err(ThemeError::InvalidValue(format!(
            "Theme '{}' is not valid. Must be 'light', 'dark', or 'auto'",
            theme
        )));
    }

    // Save to Tauri store
    store.set("theme", &theme).map_err(|e| ThemeError::from(e))?;

    // Emit event to all windows
    app.emit("theme-changed", Some(&theme));

    Ok(())
}
```

- [ ] **Step 3: Run cargo check**

```bash
cd src-tauri && cargo check --message="Check theme.rs set_theme function"
```

Expected: No errors

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands/theme.rs src-tauri/src/commands/theme_errors.rs
git commit -m "feat(rust): add set_theme command with validation and error handling"
```

---

## Chunk 5: Backend - Initialize Tauri Store and Register Commands

### Task 5: Register theme commands and initialize store plugin

**Files:**

- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add theme module and types module declarations**

```rust
mod theme;
mod theme_errors;
mod types;
```

- [ ] **Step 2: Initialize Tauri store plugin in run() function**

Find the `.setup(move |app| {` section (around line 112) and add store plugin initialization:

```rust
tauri::Builder::default()
    .plugin(tauri_plugin_store::Builder::new().build())
    .setup(move |app| {
        // ... existing plugins
    })
```

- [ ] **Step 3: Register theme commands in invoke_handler**

Find the `invoke_handler` call (around line 516) and add theme commands:

```rust
.invoke_handler(tauri::generate_handler![
    // ... existing commands
    get_theme(app),
    set_theme(app, theme),
])
```

- [ ] **Step 4: Run cargo check**

```bash
cd src-tauri && cargo check --message="Register theme commands and store plugin"
```

Expected: No errors

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "chore(rust): register theme commands and initialize Tauri store plugin"
```

---

## Chunk 6: Frontend - Add i18n Translation Keys

### Task 6: Add theme translation keys

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

## Chunk 7: Frontend - Refactor App.svelte

### Task 7: Add theme management to App.svelte

**Files:**

- Modify: `src/App.svelte`

- [ ] **Step 1: Replace script section with theme management**

Replace the entire `<script>` section with:

```typescript
<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { listen, invoke } from "@tauri-apps/api/event";
  import { locale, t } from "$lib/i18n";
  import { ThemeMode } from "$lib/types/theme";
  $locale;

  // Reactive translation helper
  const tr = $derived((key: string, params?: Record<string, string | number>) => {
    const _ = $locale;
    return t(key, params);
  });

  // Theme state
  let theme: ThemeMode = $state("auto");

  // System theme listener
  let mediaQuery: MediaQueryList | null = null;

  // Load theme from store
  async function loadThemeFromStore(): Promise<ThemeMode> {
    try {
      const stored = await invoke<ThemeMode>("get_theme");
      if (stored === "light" || stored === "dark" || stored === "auto") {
        return stored;
      }
    } catch (e) {
      console.warn("Failed to load theme from store:", e);
    }
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

    // Update state
    theme = mode;
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
  let unlistenThemeChanged: (() => void) | null = null;

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

- [ ] **Step 2: Pass theme prop to SettingsPanel**

Find the SettingsPanel invocation (around line 100) and add theme prop:

```typescript
{#if currentTab === 'settings'}
  <SettingsPanel theme={theme} />
{/if}
```

- [ ] **Step 3: Verify TypeScript compilation**

```bash
npm run check
```

Expected: No type errors

- [ ] **Step 4: Commit**

```bash
git add src/App.svelte
git commit -m "refactor(app): add unified theme management with Tauri store and IPC"
```

---

## Chunk 8: Frontend - Refactor SettingsPanel.svelte

### Task 8: Simplify SettingsPanel to UI-only component

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
  import { ThemeMode } from "$lib/types/theme";
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

## Chunk 9: Frontend - Update Popup.svelte

### Task 9: Add theme support to Popup.svelte

**Files:**

- Modify: `src/pages/popup/Popup.svelte`

- [ ] **Step 1: Add theme prop, import types, and apply logic**

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

  // Listen for theme changes
  let unlistenThemeChanged = null;

  onMount(() => {
    try {
      unlistenThemeChanged = await listen<ThemeMode>("theme-changed", (event) => {
        const newTheme = event.payload;
        applyThemeToDom();
      });
    } catch (e) {
      console.error("Failed to listen for theme changes:", e);
    }
  });

  onDestroy(() => {
    if (unlistenThemeChanged) {
      unlistenThemeChanged();
      unlistenThemeChanged = null;
    }
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
git commit -m "feat(app): add theme support and event listening to Popup.svelte"
```

---

## Chunk 10: Frontend - Update AiPopup.svelte

### Task 10: Add theme support to AiPopup.svelte

**Files:**

- Modify: `src/pages/ai-popup/AiPopup.svelte`

- [ ] **Step 1: Add theme prop, import types, and apply logic**

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

  // Listen for theme changes
  let unlistenThemeChanged = null;

  onMount(() => {
    try {
      unlistenThemeChanged = await listen<ThemeMode>("theme-changed", (event) => {
        const newTheme = event.payload;
        applyThemeToDom();
      });
    } catch (e) {
      console.error("Failed to listen for theme changes:", e);
    }
  });

  onDestroy(() => {
    if (unlistenThemeChanged) {
      unlistenThemeChanged();
      unlistenThemeChanged = null;
    }
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
git commit -m "feat(app): add theme support and event listening to AiPopup.svelte"
```

---

## Chunk 11: Testing

### Task 11: Test unified theme management

**Files:**

- No file changes (testing only)

- [ ] **Step 1: Run Rust tests**

```bash
cd src-tauri && cargo test --lib theme
```

Expected: All tests pass

- [ ] **Step 2: Test theme commands and store functionality**

````bash
# Start Tauri dev
npm run tauri:dev

# Manual testing checklist:
# [ ] Test get_theme command returns stored theme from Tauri store
# [ ] Test get_theme command falls back to "auto" when store is empty/unavailable
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

- [ ] Shared ThemeMode type created
- [ ] Tauri store dependency added to Cargo.toml
- [ ] Theme commands compile without errors
- [ ] Theme commands registered in lib.rs
- [ ] get_theme command receives app_handle parameter correctly
- [ ] get_theme reads from Tauri store correctly
- [ ] get_theme returns "auto" as default
- [ ] get_theme handles store unavailability gracefully
- [ ] set_theme command receives app_handle parameter correctly
- [ ] set_theme validates theme value using matches! macro
- [ ] set_theme returns ThemeError::InvalidValue for invalid theme
- [ ] set_theme saves to Tauri store correctly
- [ ] set_theme emits theme-changed event
- [ ] Tauri store plugin initialized in lib.rs run() function

Frontend:

- [ ] Shared ThemeMode type created and imported in App.svelte
- [ ] Shared ThemeMode type imported in SettingsPanel.svelte
- [ ] Shared ThemeMode type imported in Popup.svelte
- [ ] Shared ThemeMode type imported in AiPopup.svelte
- [ ] i18n translation keys added for theme
- [ ] App.svelte initializes theme from store on mount
- [ ] App.svelte uses invoke to call get_theme and set_theme
- [ ] App.svelte applies theme to DOM immediately on mount and changes
- [ ] App.svelte listens for theme-changed events
- [ ] App.svelte passes theme to SettingsPanel as prop
- [ ] SettingsPanel receives theme as prop
- [ ] SettingsPanel displays theme selector UI correctly
- [ ] SettingsPanel calls set_theme on theme change
- [ ] Popup.svelte receives theme as prop
- [ ] Popup.svelte applies theme on mount
- [ ] Popup.svelte listens for theme-changed events
- [ ] Popup.svelte applies theme on prop changes
- [ ] AiPopup.svelte receives theme as prop
- [ ] AiPopup.svelte applies theme on mount
- [ ] AiPopup.svelte listens for theme-changed events
- [ ] AiPopup.svelte applies theme on prop changes
- [ ] All windows (App, Settings, Popup, AiPopup) respect theme setting
- [ ] Theme persists across app restarts
- [ ] Theme changes sync to all windows via theme-changed event
- [ ] Auto mode follows system preference
- [ ] System theme changes update UI in real-time
- [ ] TypeScript compilation passes (npm run check)
- [ ] No console errors related to theme management

## Edge Cases Handled

- **Store unavailable:** get_theme returns "auto" as default, no manual fallback to localStorage
- **Invalid theme value:** set_theme returns ThemeError::InvalidValue
- **Race conditions:** Last write wins for store, applyThemeToDom handles concurrent calls
- **Memory leaks:** Proper cleanup of event listeners (MediaQuery and Tauri)
- **Popup/AiPopup open:** Windows listen for theme-changed events and apply immediately when props update from theme changes

## Data Migration Notes

No explicit data migration needed. New installations use Tauri store exclusively. Existing installations with localStorage theme will continue to work (get_theme checks store first, then localStorage). Both systems can coexist during transition.
