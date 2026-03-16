# Ignored Apps Whitelist — Design Spec

**Date:** 2026-03-16  
**Status:** Approved

---

## Overview

Add an "Ignore" button to the `popup` and `ai-popup` windows. Clicking it adds the current source app to a persistent whitelist. Whitelisted apps can independently suppress: (a) the 💡 icon + popup/ai-popup, and (b) the overlay underline rendering. The whitelist is managed in a new `IgnoredAppsManager` component in the Settings page.

---

## Goals

- Let users permanently suppress autocorrect UI for specific apps (e.g. Xcode, Terminal).
- Per-app granularity: disable popup only, overlay only, or both.
- Zero-friction UX: one click from popup, no manual bundle ID entry.
- Persist across restarts via existing `app-settings.json` store.

## Non-Goals

- No re-enable from popup (only from Settings).
- No manual add-by-name in Settings (bundle ID must come from live app detection).
- No per-rule granularity per app.

---

## Data Model

### `IgnoredApp` struct (Rust)

```rust
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IgnoredApp {
    pub name: String,         // Human-readable display name, e.g. "Xcode"
    pub bundle_id: String,    // Unique key, e.g. "com.apple.dt.Xcode"
    pub ignore_popup: bool,   // Suppress 💡 icon + popup + ai-popup
    pub ignore_overlay: bool, // Suppress overlay underline rendering
}
```

### `AppSettings` change

Add one field with `#[serde(default)]` for backward compatibility:

```rust
#[serde(default)]
pub ignored_apps: Vec<IgnoredApp>,
```

Stored in existing `app-settings.json` via `tauri-plugin-store`.

---

## Backend: New Module `commands/ignored_apps.rs`

### Helper: `get_frontmost_app_info_macos() -> Option<(String, String)>`

Pure ObjC via `objc2`, no subprocess. Uses `NSWorkspace.sharedWorkspace.frontmostApplication` to read both `localizedName` and `bundleIdentifier`. Returns `None` on non-macOS or if values are null.

### Tauri Commands

| Command                  | Signature                                                                   | Notes                   |
| ------------------------ | --------------------------------------------------------------------------- | ----------------------- |
| `get_ignored_apps`       | `(app: AppHandle) -> Result<Vec<IgnoredApp>, Error>`                        | Returns full list       |
| `add_ignored_app`        | `(app, name, bundle_id, ignore_popup, ignore_overlay) -> Result<(), Error>` | Upserts by bundle_id    |
| `update_ignored_app`     | `(app, bundle_id, ignore_popup, ignore_overlay) -> Result<(), Error>`       | Updates flags only      |
| `remove_ignored_app`     | `(app, bundle_id) -> Result<(), Error>`                                     | Removes by bundle_id    |
| `get_frontmost_app_info` | `(app: AppHandle) -> Result<Option<AppInfo>, Error>`                        | For popup Ignore button |

```rust
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: String,
    pub bundle_id: String,
}
```

All commands load/mutate/save `AppSettings` via existing `load_app_settings` / `save_app_settings` helpers in `config.rs`.

### Enforcement Helper: `is_app_ignored`

```rust
// In commands/ignored_apps.rs, pub
pub fn is_app_ignored(
    app: &AppHandle,
    bundle_id: &str,
    check_popup: bool,
    check_overlay: bool,
) -> bool
```

Loads `AppSettings`, checks `ignored_apps` for `bundle_id`. Returns `true` if the matching entry has the requested flag(s) set.

### `lib.rs` Injection Points

Three call sites, all gated on `#[cfg(target_os = "macos")]`:

1. **`show_ai_icon` call** (~line 854 of `lib.rs`): Before showing the 💡 icon, call `get_frontmost_bundle_id_macos()` and check `is_app_ignored(..., check_popup: true, check_overlay: false)`. If true, skip.

2. **Hotkey handler** (where `trigger_spell_check_workflow` is invoked from hotkey): Same check as above.

3. **Overlay drawing** (where overlay markers are applied per-app): Check `is_app_ignored(..., check_popup: false, check_overlay: true)`. If true, skip overlay rendering for that app.

A lightweight `get_frontmost_bundle_id_macos() -> Option<String>` helper (ObjC only, no subprocess) is added in `commands/ignored_apps.rs` and reused at all three sites.

---

## Frontend

### Ignore Button in `popup` and `ai-popup`

**Placement:** Next to the close button, right-aligned in the footer/toolbar area.

**Appearance:**

```
[⊘ Ignore]
```

Small secondary/ghost button. On hover: tooltip "忽略该应用 / Ignore this app — won't show corrections for this app again".

**Interaction flow:**

1. User clicks Ignore.
2. Frontend reads `sourceAppName` from existing popup state (already stored at show-time).
3. Calls `get_frontmost_app_info` Tauri command to get `{ name, bundleId }` for the source app.
4. Calls `add_ignored_app(name, bundleId, ignorePopup: true, ignoreOverlay: true)`.
5. Shows brief inline confirmation: "已忽略 Xcode / Xcode ignored".
6. After 800ms, auto-closes the popup.

**Edge case:** If `get_frontmost_app_info` returns null (non-macOS or no frontmost app), show a brief error "无法获取应用信息 / Could not get app info" and do not close.

### `IgnoredAppsManager.svelte` (new component)

Placed in `src/lib/components/IgnoredAppsManager.svelte`. Added to `SettingsPanel.svelte` as a new card section below Custom Corrections.

**Layout:**

```
┌─ Ignored Apps ──────────────────────────────────────────┐
│ App              Disable Popup  Disable Overlay  Action │
│ Xcode            [✓ switch]     [✓ switch]       [Del]  │
│ Terminal         [✓ switch]     [✗ switch]       [Del]  │
│                                                         │
│ (empty state) No ignored apps yet. Use the Ignore       │
│ button in the correction popup to add apps here.        │
└─────────────────────────────────────────────────────────┘
```

- **Switch** for `ignorePopup` and `ignoreOverlay` — on toggle, calls `update_ignored_app` immediately.
- **Delete** button — opens `AlertDialog` confirmation (same pattern as `CustomCorrectionsManager`).
- **No manual add** — empty state explains how to add via popup.
- Loads on mount via `get_ignored_apps`.

### i18n Keys

New keys added to both `en` and `zh-CN` in `messages.ts`:

| Key                                   | en                                                                                     | zh-CN                                            |
| ------------------------------------- | -------------------------------------------------------------------------------------- | ------------------------------------------------ |
| `popup.ignore`                        | `Ignore`                                                                               | `忽略`                                           |
| `popup.ignoreTooltip`                 | `Ignore this app — won't show corrections here again`                                  | `忽略该应用，不再对其启用校验和 AI 功能`         |
| `popup.ignored`                       | `Ignored {name}`                                                                       | `已忽略 {name}`                                  |
| `popup.ignoreError`                   | `Could not get app info`                                                               | `无法获取应用信息`                               |
| `settings.ignoredApps`                | `Ignored Apps`                                                                         | `忽略的应用`                                     |
| `settings.ignoredAppsDesc`            | `Apps where corrections and AI features are suppressed.`                               | `不启用校验和 AI 功能的应用列表。`               |
| `settings.ignoredApps.disablePopup`   | `Disable Popup`                                                                        | `禁用弹窗`                                       |
| `settings.ignoredApps.disableOverlay` | `Disable Overlay`                                                                      | `禁用下划线`                                     |
| `settings.ignoredApps.empty`          | `No ignored apps yet. Use the Ignore button in the correction popup to add apps here.` | `暂无忽略的应用。通过弹窗中的「忽略」按钮添加。` |
| `settings.ignoredApps.deleteTitle`    | `Remove ignored app?`                                                                  | `移除忽略的应用？`                               |
| `settings.ignoredApps.deleteDesc`     | `"{name}" will show corrections again.`                                                | `"{name}" 将重新启用校验和 AI 功能。`            |

---

## File Changelist

| File                                           | Change                                                                                             |
| ---------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| `src-tauri/src/commands/ignored_apps.rs`       | **New** — `IgnoredApp`, `AppInfo`, all commands, `is_app_ignored`, `get_frontmost_bundle_id_macos` |
| `src-tauri/src/commands/mod.rs`                | Add `pub mod ignored_apps`                                                                         |
| `src-tauri/src/commands/config.rs`             | Add `ignored_apps: Vec<IgnoredApp>` to `AppSettings` + `Default`                                   |
| `src-tauri/src/lib.rs`                         | 3 enforcement call sites                                                                           |
| `src/lib/components/IgnoredAppsManager.svelte` | **New**                                                                                            |
| `src/lib/components/SettingsPanel.svelte`      | Import + render `IgnoredAppsManager`                                                               |
| `src/lib/i18n/messages.ts`                     | New i18n keys (en + zh-CN)                                                                         |
| Popup HTML/Svelte source files                 | Ignore button + handler                                                                            |
| `src-tauri/src/commands/errors.rs`             | No change needed (existing `Error` variants cover it)                                              |

---

## Error Handling

- All new commands return `Result<_, Error>` using the existing `Error` enum.
- `add_ignored_app` is an upsert: if `bundle_id` already exists, update flags without duplicating.
- `update_ignored_app` on unknown `bundle_id` returns `Ok(())` silently (idempotent).
- `remove_ignored_app` on unknown `bundle_id` returns `Ok(())` silently.

---

## Testing

- **Rust unit tests** in `commands/ignored_apps.rs`: test add/update/remove/is_ignored logic against an in-memory `Vec<IgnoredApp>`.
- **Manual smoke test:**
  1. Open any app (e.g. TextEdit), select text → 💡 icon appears → hover → popup opens → click Ignore.
  2. Confirm popup closes and app appears in Settings → Ignored Apps.
  3. Select text again in TextEdit → 💡 icon does NOT appear.
  4. Toggle `ignore_overlay` off in Settings → overlay underlines resume in TextEdit.
  5. Delete the entry → full behavior restored.
