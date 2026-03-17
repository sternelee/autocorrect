# Unified Theme Management Design Spec

**Date:** 2026-03-17
**Author:** OpenCode Agent
**Status:** Draft

## Overview

Implement unified theme management using Tauri's `app.setTheme()` API to ensure consistent theming across all application windows (App.svelte, SettingsPanel.svelte, Popup.svelte, AiPopup.svelte).

## Requirements

### Theme Modes

- `light` - Force light theme
- `dark` - Force dark theme
- `auto` - Follow system preference (default: `auto`)

### Unified Theme Management

- All windows share the same theme state via Tauri store
- Theme changes triggered from Settings Panel are immediately synchronized to all windows
- Theme changes persist across app restarts via Tauri store
- Popup and AI popup windows dynamically adapt to current theme

### User Interface

- Theme selector in Settings panel > Appearance section
- Immediate application (no save button required)
- All windows update theme in real-time when theme changes

## Technical Design

### Theme Storage Strategy

**Primary Storage:** Tauri Store (`app-settings.json`)

- Use Tauri's `app.setTheme(theme)` API
- Store persists across app restarts
- Automatically syncs to all windows

\*\*Fallback Storage (Optional): localStorage

- If Tauri store is unavailable (e.g., in development environment)
- Fall back to localStorage as secondary option
- Graceful degradation

### Architecture

**Component Roles:**

**1. App.svelte (Theme Manager)**

- Initialize theme on app startup from Tauri store
- Apply theme to DOM immediately
- Listen for `theme-changed` events from Tauri
- Broadcast theme state to all child windows
- Manage system preference listener for auto mode

**2. SettingsPanel.svelte (Theme Controller)**

- Receive theme from App.svelte as prop
- Display theme selector UI in Appearance section
- On theme change, call `app.setTheme(theme)` via Tauri API
- No local theme state or localStorage management

**3. Popup.svelte & AiPopup.svelte (Theme Consumers)**

- Receive theme from App.svelte as prop
- Listen for `theme-changed` events from Tauri
- Apply theme to DOM immediately on change
- No local theme management logic

### Data Flow

```
┌─────────────────────────────────────────────────┐
│  User changes theme in Settings Panel       │
└────────────────────┬─────────────────────────┘
                 │
                 │
                 ▼
        ┌────────────────────────┐
        │  App.setTheme(theme)  │
        │    (Tauri API)      │
        └───────────┬───────────┘
                    │
                    │
        ┌───────────┴─────────────────┐
        │   Tauri Store             │
        │   (app-settings.json)      │
        └───────────┬────────────┘
                    │
                    ▼
          ┌─────────────────────────┐
          │ Theme Changed Event    │
          │   (theme-changed)      │
          └───────────┬───────────────┘
                     │
                     ▼
         ┌─────────────────────────────────┐
         │  All Windows Update Theme  │
         │  (App.svelte, SettingsPanel,  │
         │   Popup.svelte, AiPopup.svelte)   │
         └─────────────────────────────────┘
```

## Implementation Details

### Frontend Components

#### 1. App.svelte - Theme Management

**Add theme management logic:**

```typescript
// Theme type
type ThemeMode = "light" | "dark" | "auto";

// Reactive theme state
let theme: ThemeMode = $state("auto");

// System theme listener
let mediaQuery: MediaQueryList | null = null;

function loadThemeFromStore(): Promise<ThemeMode> {
  // Try Tauri store first
  try {
    const stored = await invoke<ThemeMode>("get_theme");
    if (stored === "light" || stored === "dark" || stored === "auto") {
      return stored;
    }
  } catch (e) {
    console.warn(
      "Failed to load theme from store, using localStorage fallback:",
      e,
    );
  }

  // Fallback to localStorage
  const localTheme = localStorage.getItem("autocorrect-theme");
  if (
    localTheme === "light" ||
    localTheme === "dark" ||
    localTheme === "auto"
  ) {
    return localTheme;
  }

  return "auto"; // Default
}

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

  // Listen for theme changes from Tauri
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

// Emit theme changes to child windows
// Pass theme state as prop to child components
```

#### 2. SettingsPanel.svelte - Theme Selector

**Simplify to UI-only component:**

```svelte
<script lang="ts">
  // Import tauri API
  import { invoke } from "@tauri-apps/api/core";

  // Props received from App.svelte
  export let theme: ThemeMode;

  // Translation helper
  import { locale, t } from "$lib/i18n";
  $locale;
  const tr = $derived((key: string, params?: Record<string, string | number>) => {
    const _ = $locale;
    return t(key, params);
  });

  async function onThemeChange(selectedTheme: ThemeMode) {
    // Update Tauri store
    try {
      await invoke("set_theme", { theme: selectedTheme });
    } catch (e) {
      console.error("Failed to set theme:", e);
    }
  }
</script>

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

#### 3. Popup.svelte & AiPopup.svelte - Theme Consumers

**Receive theme from App.svelte and apply:**

```svelte
<script lang="ts">
  // Receive theme from parent
  export let theme: ThemeMode;

  function applyThemeToDom() {
    const html = document.documentElement;

    if (theme === "dark") {
      html.classList.add("dark");
    } else if (theme === "auto") {
      const prefersDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
      html.classList.toggle("dark", prefersDark);
    } else {
      html.classList.remove("dark");
    }
  }
</script>

{#if theme}
  <svelte:window onkeydown={(e) => e.key === "Escape" && getCurrentWindow().hide()} />
{/if}

<!-- Apply theme on mount or prop change -->
{#key theme}
  {@const applyThemeToDom()}
{/key}
```

### Backend Commands (Rust)

#### 1. Get Theme Command

```rust
use tauri::Manager;

#[tauri::command]
pub async fn get_theme() -> Result<String, String> {
    let app = app_handle();
    let store = app.store()?;

    // Try to read from Tauri store first
    if let Ok(Some(stored_theme)) = store.get("theme") {
        if let Ok(theme_str) = stored_theme.as_string() {
            return Ok(theme_str);
        }
    }

    // Fallback to localStorage (optional)
    #[cfg(target_os = "macos")]
    {
        use std::os::unix::fs::OpenOptionsExt;
        use std::fs::read_to_string;
    use std::path::PathBuf;

        let config_path = dirs::config_localdata()
            .unwrap()
            .join("AutoCorrect")
            .join("app-settings.json");

        if let Ok(content) = read_to_string(&config_path, OpenOptions::read()) {
            if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(theme_obj) = config.get("theme") {
                    if let Ok(theme_str) = theme_obj.as_str() {
                        return Ok(theme_str);
                    }
                }
            }
        }
    }

    Ok("auto".to_string())
}
```

#### 2. Set Theme Command

```rust
use tauri::Manager;

#[tauri::command]
pub async fn set_theme(theme: String) -> Result<(), String> {
    let app = app_handle();
    let store = app.store()?;

    // Save to Tauri store
    store.set("theme", theme)?;

    // Emit event to all windows
    app.emit("theme-changed", Some(theme));

    Ok(())
}
```

### Register Commands in lib.rs

```rust
fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .setup(move |app| {
            app.handle().plugin(tauri_plugin_http::init())?;
            // ... existing plugins

            // Register theme commands
            app.handle().plugin(tauri_plugin_store::Builder::new().build())
                .invoke_handler(tauri::generate_handler![
                    get_theme,
                    set_theme,
                ])
                .build()
        })
        .invoke_handler(tauri::generate_handler![
            // ... existing handlers
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            // ... existing event handlers
        });
}
```

## Implementation Plan

### Phase 1: Backend (Rust)

1. Add `tauri-plugin-store` dependency to `Cargo.toml`
2. Create `src-tauri/src/commands/theme.rs` module
3. Implement `get_theme` command with Tauri store + localStorage fallback
4. Implement `set_theme` command with store + event emission
5. Register commands in `src-tauri/src/lib.rs`
6. Test theme commands with `cargo test`

### Phase 2: Frontend Refactor

1. Refactor App.svelte to use theme management logic
2. Simplify SettingsPanel.svelte to UI-only (pass theme as prop)
3. Update Popup.svelte to receive and apply theme prop
4. Update AiPopup.svelte to receive and apply theme prop
5. Remove all theme management from SettingsPanel, Popup, and AiPopup
6. Test all windows respect theme setting

### Phase 3: Integration Testing

1. Test theme persistence across app restarts
2. Test real-time theme synchronization
3. Test auto mode system preference detection
4. Test theme changes propagate to all windows
5. Test edge cases (store unavailable, corrupted data)

## Testing Checklist

- [ ] Theme commands registered in Rust
- [ ] App.svelte initializes theme from store on startup
- [ ] SettingsPanel theme selector works and calls set_theme
- [ ] Theme changes persist across app restarts
- [ ] Theme changes sync to all windows (App, Settings, Popup, AiPopup)
- [ ] Popup and AiPopup apply theme on mount and prop changes
- [ ] Auto mode follows system preference
- [ ] System theme changes update UI in real-time
- [ ] Event listeners properly cleaned up on component destroy
- [ ] No console errors related to theme management

## Edge Cases

- **Store unavailable:** Gracefully fallback to localStorage
- **Corrupted store data:** Return "auto" as safe default
- **Invalid theme value:** Ignore and keep current theme
- **Race conditions:** Last write wins for store, applyThemeToDom handles concurrent calls
- **Memory leaks:** Proper cleanup of event listeners and MediaQuery listeners

## Future Considerations

- Migrate to Tauri's new state management APIs when available
- Add theme transitions/animations for smoother UX
- Support custom accent colors
- Per-window theme preferences (advanced users may want different themes for popup vs main window)

## Migration Path

For existing implementations (if any):

1. Update `Cargo.toml` to add store plugin dependency
2. Run `cargo build` to verify compilation
3. Update all frontend components to remove theme management logic
4. Test that existing functionality still works
5. Deploy to users
