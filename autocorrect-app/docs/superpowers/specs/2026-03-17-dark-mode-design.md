# Dark Mode Support Design Spec

**Date:** 2026-03-17
**Author:** OpenCode Agent
**Status:** Draft

## Overview

Add dark mode support to the AutoCorrect desktop application with configurable theme options in the Settings panel. The app will support three theme modes: light, dark, and auto (system preference).

## Requirements

### Theme Modes

- `light` - Force light theme
- `dark` - Force dark theme
- `auto` - Follow system preference (default: `auto`)

### User Interface

- Theme selector in Settings panel > Appearance section
- Located above "Underline Style" setting
- Immediate application (no save button required)

### Storage & Persistence

- Store theme preference in localStorage with key `autocorrect-theme`
- Default to `auto` on fresh install
- Fallback to `auto` if no stored value

### System Theme Detection

- Detect system preference via `matchMedia('(prefers-color-scheme: dark)')`
- Listen for system theme changes in real-time when in `auto` mode
- Update DOM immediately on system theme change

## Technical Design

### Theme Application Mechanism

- Toggle `.dark` CSS class on `<html>` element
- Dark theme styles already defined in `src/app.css` (lines 37-64)
- Light theme styles in `:root` (lines 7-35)

### Component Changes

#### SettingsPanel.svelte

Add state for theme selection:

```typescript
type ThemeMode = "light" | "dark" | "auto";
let theme: ThemeMode = $state("auto");
```

Add theme selector UI in Appearance section (above underline style):

```svelte
<div class="space-y-2">
  <p class="text-sm font-medium">{tr("settings.theme")}</p>
  <select bind:value={theme} onchange={applyTheme} class="...">
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

#### Theme Management Logic

```typescript
const THEME_STORAGE_KEY = "autocorrect-theme";

function loadTheme(): ThemeMode {
  const stored = localStorage.getItem(THEME_STORAGE_KEY);
  if (stored === "light" || stored === "dark" || stored === "auto") {
    return stored;
  }
  return "auto"; // default
}

function applyTheme(mode: ThemeMode) {
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
  localStorage.setItem(THEME_STORAGE_KEY, mode);
}

let mediaQuery: MediaQueryList | null = null;

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

// On mount
onMount(() => {
  theme = loadTheme();
  applyTheme(theme);
  setupSystemThemeListener();
});

// Cleanup
onDestroy(() => {
  cleanupThemeListener();
});
```

### i18n Keys

Add to `src/lib/i18n/messages.ts`:

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

## Implementation Plan

1. Add i18n translation keys for theme options
2. Create theme management logic in SettingsPanel.svelte
3. Add theme selector UI in Appearance section
4. Implement localStorage persistence
5. Implement system theme listener with cleanup
6. Test theme switching (light, dark, auto)
7. Test system theme changes in auto mode
8. Test localStorage persistence across app restarts

## Testing Checklist

- [ ] Theme selector appears in Settings > Appearance
- [ ] Light mode displays correctly
- [ ] Dark mode displays correctly
- [ ] Auto mode follows system preference
- [ ] Theme persists across app restarts
- [ ] System theme change updates UI in auto mode (real-time)
- [ ] MediaQuery listener cleaned up on component unmount
- [ ] No console errors related to theme switching

## Edge Cases

- localStorage unavailable (rare): fallback to default `auto`
- System preference detection fails: fallback to `auto`
- Multiple components trying to manage theme: single source of truth in SettingsPanel

## Future Considerations

- Migrate to backend persistence (app-settings.json) if cloud sync is added
- Add manual override in auto mode (e.g., "Auto (currently dark)")
- Custom accent colors for dark mode
