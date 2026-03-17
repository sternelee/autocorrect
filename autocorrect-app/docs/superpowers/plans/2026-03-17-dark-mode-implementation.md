# Dark Mode Support Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add dark mode support with configurable theme options (light/dark/auto) in Settings panel

**Architecture:** Frontend-only localStorage persistence with system theme detection via Match Media API. Toggle `.dark` CSS class on `<html>` element.

**Tech Stack:** Svelte 5 runes, localStorage, window.matchMedia, Tailwind CSS 4

---

## File Structure

- **Create:** None (all modifications to existing files)
- **Modify:**
  - `src/lib/i18n/messages.ts` - Add theme translation keys
  - `src/lib/components/SettingsPanel.svelte` - Add theme management logic and UI selector

---

## Chunk 1: Add i18n Translation Keys

### Task 1: Add Theme Translations to i18n Messages

**Files:**

- Modify: `src/lib/i18n/messages.ts`

- [ ] **Step 1: Add theme translations to English locale**

```typescript
// In the `en` object, add these keys after existing settings keys:
"settings.theme": "Theme",
"settings.theme.light": "Light",
"settings.theme.dark": "Dark",
"settings.theme.auto": "Auto",
"settings.theme.autoDesc": "Follows system preference"
```

- [ ] **Step 2: Add theme translations to Chinese locale**

```typescript
// In the `"zh-CN"` object, add these keys after existing settings keys:
"settings.theme": "主题",
"settings.theme.light": "浅色",
"settings.theme.dark": "深色",
"settings.theme.auto": "自动",
"settings.theme.autoDesc": "跟随系统偏好"
```

- [ ] **Step 3: Verify translations are valid TypeScript**

Run: `npm run check`
Expected: No type errors related to i18n keys

- [ ] **Step 4: Commit**

```bash
git add src/lib/i18n/messages.ts
git commit -m "feat(app): add theme translation keys"
```

---

## Chunk 2: Add Theme State and Type Definitions

### Task 2: Define Theme Type and State in SettingsPanel

**Files:**

- Modify: `src/lib/components/SettingsPanel.svelte`

- [ ] **Step 1: Add ThemeMode type definition**

```typescript
// Add near line 60, after other interface definitions, before state declarations
type ThemeMode = "light" | "dark" | "auto";
```

- [ ] **Step 2: Add theme state variable**

```typescript
// Add near line 103, after `uiLanguage` state declaration
let theme: ThemeMode = $state("auto");
```

- [ ] **Step 3: Verify TypeScript compilation**

Run: `npm run check`
Expected: No type errors

- [ ] **Step 4: Commit**

```bash
git add src/lib/components/SettingsPanel.svelte
git commit -m "feat(app): add theme type and state"
```

---

## Chunk 3: Add Theme Management Functions

### Task 3: Implement Theme Loading and Application Logic

**Files:**

- Modify: `src/lib/components/SettingsPanel.svelte`

- [ ] **Step 1: Add theme storage constant**

```typescript
// Add near line 103, before state declarations
const THEME_STORAGE_KEY = "autocorrect-theme";
```

- [ ] **Step 2: Add loadTheme function**

```typescript
// Add near line 103, after THEME_STORAGE_KEY constant
function loadTheme(): ThemeMode {
  const stored = localStorage.getItem(THEME_STORAGE_KEY);
  if (stored === "light" || stored === "dark" || stored === "auto") {
    return stored;
  }
  return "auto"; // default
}
```

- [ ] **Step 3: Add applyTheme function**

```typescript
// Add after loadTheme function
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
```

- [ ] **Step 4: Add media query state**

```typescript
// Add after applyTheme function, before existing hotkey state (around line 166)
let mediaQuery: MediaQueryList | null = null;
```

- [ ] **Step 5: Add setupSystemThemeListener function**

```typescript
// Add after mediaQuery state
function setupSystemThemeListener() {
  if (mediaQuery) {
    mediaQuery.removeEventListener("change", handleSystemThemeChange);
  }
  mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
  mediaQuery.addEventListener("change", handleSystemThemeChange);
}
```

- [ ] **Step 6: Add handleSystemThemeChange function**

```typescript
// Add after setupSystemThemeListener function
function handleSystemThemeChange(e: MediaQueryListEvent) {
  if (theme === "auto") {
    const html = document.documentElement;
    html.classList.toggle("dark", e.matches);
  }
}
```

- [ ] **Step 7: Add cleanupThemeListener function**

```typescript
// Add after handleSystemThemeChange function
function cleanupThemeListener() {
  if (mediaQuery) {
    mediaQuery.removeEventListener("change", handleSystemThemeChange);
    mediaQuery = null;
  }
}
```

- [ ] **Step 8: Verify TypeScript compilation**

Run: `npm run check`
Expected: No type errors

- [ ] **Step 9: Commit**

```bash
git add src/lib/components/SettingsPanel.svelte
git commit -m "feat(app): add theme management functions"
```

---

## Chunk 4: Initialize Theme on Mount and Cleanup

### Task 4: Integrate Theme Management with Component Lifecycle

**Files:**

- Modify: `src/lib/components/SettingsPanel.svelte`

- [ ] **Step 1: Add import for onDestroy**

```typescript
// Add to line 3, alongside other imports
import { onDestroy } from "svelte";
```

- [ ] **Step 2: Initialize theme on mount**

```typescript
// Find the existing onMount call (around line 398) and modify it:
onMount(() => {
  loadConfiguration();
  loadHotkeyConfiguration();

  // Initialize theme
  theme = loadTheme();
  applyTheme(theme);
  setupSystemThemeListener();
});
```

- [ ] **Step 3: Add cleanup on destroy**

```typescript
// Add after onMount call, before function definitions
onDestroy(() => {
  cleanupThemeListener();
});
```

- [ ] **Step 4: Verify TypeScript compilation**

Run: `npm run check`
Expected: No type errors

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/SettingsPanel.svelte
git commit -m "feat(app): initialize theme and add cleanup"
```

---

## Chunk 5: Add Theme Selector UI

### Task 5: Add Theme Selector to Appearance Section

**Files:**

- Modify: `src/lib/components/SettingsPanel.svelte`

- [ ] **Step 1: Add theme selector HTML**

Find the Appearance section (starts around line 746 with `<h3 class="text-sm font-semibold">{tr("settings.appearance")}</h3>`), and add this immediately after that line, before the `<!-- Underline Style -->` comment:

```svelte
        <!-- Theme Selector -->
        <div class="space-y-2">
          <p class="text-sm font-medium">{tr("settings.theme")}</p>
          <select
            bind:value={theme}
            onchange={() => applyTheme(theme)}
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

- [ ] **Step 2: Verify build succeeds**

Run: `npm run build`
Expected: Build completes successfully with no errors

- [ ] **Step 3: Commit**

```bash
git add src/lib/components/SettingsPanel.svelte
git commit -m "feat(app): add theme selector UI"
```

---

## Chunk 6: Testing and Validation

### Task 6: Test Theme Functionality

**Files:**

- No file changes (testing only)

- [ ] **Step 1: Run development server**

Run: `npm run tauri:dev`
Expected: App launches successfully

- [ ] **Step 2: Verify default theme is auto**

Action: Launch app fresh (no localStorage)
Expected: App follows system theme preference

- [ ] **Step 3: Test light mode**

Action: In Settings > Appearance, select "Light"
Expected: App switches to light theme immediately, no save button needed

- [ ] **Step 4: Test dark mode**

Action: In Settings > Appearance, select "Dark"
Expected: App switches to dark theme immediately

- [ ] **Step 5: Test auto mode**

Action: In Settings > Appearance, select "Auto"
Expected: App follows system theme, description text appears

- [ ] **Step 6: Test system theme change in auto mode**

Action: With "Auto" selected, change system dark/light mode
Expected: App updates theme in real-time without interaction

- [ ] **Step 7: Test persistence across restart**

Action: Set theme to "Dark", close app, reopen
Expected: App starts in dark mode

- [ ] **Step 8: Verify no console errors**

Action: Open browser DevTools console
Expected: No errors related to theme switching

- [ ] **Step 9: Commit**

```bash
git commit --allow-empty -m "test(app): verify theme functionality"
```

---

## Testing Checklist

- [ ] Theme selector appears in Settings > Appearance
- [ ] Light mode displays correctly
- [ ] Dark mode displays correctly
- [ ] Auto mode follows system preference
- [ ] Theme persists across app restarts
- [ ] System theme change updates UI in auto mode (real-time)
- [ ] MediaQuery listener cleaned up on component unmount
- [ ] No console errors related to theme switching

## Edge Cases Handled

- localStorage unavailable: fallback to `auto` default in loadTheme()
- System preference detection fails: fallback to light in applyTheme() when auto mode selected
- Multiple components: single source of truth via SettingsPanel state
