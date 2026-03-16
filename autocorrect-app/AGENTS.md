# Repository Guidelines

This file provides authoritative guidance for agentic coding agents working in this repository.

## Project Overview

Desktop application built with **Tauri 2** + **Svelte 5** + **TypeScript** + **Tailwind CSS 4** (frontend) and a **Rust** backend providing system-wide CJK text correction, global hotkeys, macOS Accessibility integration, and clipboard monitoring.

## Project Structure

```
autocorrect-app/
├── src/                        # Svelte 5 + TypeScript frontend
│   ├── App.svelte              # Root component, tab routing
│   ├── lib/
│   │   ├── components/         # UI components (PascalCase.svelte)
│   │   │   ├── SpellChecker.svelte
│   │   │   ├── SettingsPanel.svelte
│   │   │   ├── StatusIndicator.svelte
│   │   │   ├── CustomCorrectionsManager.svelte
│   │   │   └── ui/             # Primitives: button, card, input, switch, textarea, tooltip
│   │   ├── commands.svelte.ts  # Tauri invoke wrappers + Svelte 5 $state
│   │   └── i18n/               # Locale strings and t() helper
│   └── main.ts                 # Entry point
├── src-tauri/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs              # App bootstrap, plugin setup, event loop
│       ├── main.rs             # Thin binary wrapper
│       ├── commands.rs         # Module re-exports
│       ├── commands/           # #[tauri::command] handlers grouped by feature
│       │   ├── errors.rs       # Shared Error enum + Serialize impl
│       │   ├── spellcheck.rs
│       │   ├── config.rs
│       │   ├── hotkey_config.rs
│       │   ├── shortcut_recorder.rs
│       │   ├── custom_corrections.rs
│       │   ├── ai_grammar.rs
│       │   └── default.rs
│       ├── hotkey.rs
│       ├── clipboard.rs
│       ├── popup.rs / ai_popup.rs / overlay.rs
│       ├── text_selection.rs
│       ├── macos_text.rs / objc2_compat.rs
│       └── typocheck.rs
└── static/                     # Static web assets (do not hand-edit dist/)
```

## Build, Lint, and Test Commands

### Frontend

```bash
npm install                    # Install all dependencies
npm run dev                    # Frontend-only Vite dev server
npm run build                  # Build frontend bundle (outputs to dist/)
npm run check                  # Svelte + TypeScript type checking (svelte-check + tsc)
npm run lint                   # Prettier check + ESLint (run before every PR)
npm run format                 # Auto-format with Prettier
```

### Full Desktop App

```bash
npm run tauri:dev              # Run full app with hot reload (frontend + Rust)
npm run tauri:build            # Release binary
npm run tauri:build:debug      # Debug binary
```

### Rust Backend

```bash
# Run all Rust tests
cargo test --manifest-path src-tauri/Cargo.toml

# Run a single test by path (preferred pattern)
cargo test --manifest-path src-tauri/Cargo.toml -p autocorrect-app -- <test_name>

# Example: run a specific function test
cargo test --manifest-path src-tauri/Cargo.toml -- commands::config::tests::test_load_settings

# Format Rust code
cd src-tauri && cargo fmt

# Check Rust without building
cd src-tauri && cargo check
```

### Required Quality Gates Before PR

1. `npm run format` — fix all formatting
2. `npm run lint` — must pass (Prettier + ESLint)
3. `npm run check` — must pass (Svelte + TS types)
4. `cargo test --manifest-path src-tauri/Cargo.toml` — all Rust tests

## Frontend Code Style (TypeScript / Svelte)

### Svelte 5 Runes

Always use Svelte 5 rune syntax — never the legacy Options API or `writable` stores for local state.

```ts
let count = $state(0); // reactive state
let doubled = $derived(count * 2); // derived value
let fn = $derived((key: string) => t(key)); // derived function (for i18n)
$effect(() => {
  /* side effect */
});
```

### Imports

Order: external packages → `@tauri-apps/*` → `$lib/components` → `$lib/...` → local.

```svelte
<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { Button } from "$lib/components/ui/button";
  import { Card, CardContent } from "$lib/components/ui/card";
  import { locale, t } from "$lib/i18n";
</script>
```

### Tauri Commands

Call Tauri commands via `invoke` directly in components, or wrap them in `src/lib/commands.svelte.ts`:

```ts
import { invoke } from "@tauri-apps/api/core";
const result = await invoke<SpellCheckResult>("spell_check", { text });
```

### Naming & Files

- Components: `PascalCase.svelte`
- Non-component files: `camelCase.ts` or `kebab-case.ts`
- Variables/functions: `camelCase`
- Types/interfaces: `PascalCase`
- Constants: `SCREAMING_SNAKE_CASE` for enums, `camelCase` for `const` values

### Types

Prefer explicit TypeScript types. Define interfaces for all Tauri command return shapes.

```ts
interface SpellCheckResult {
  original: string;
  corrected: string;
  hasChanges: boolean;
  lineChanges: LineChange[];
  typos: TypoSuggestion[];
}
```

### Tailwind CSS

Use Tailwind 4 utility classes. Prefer `clsx` + `tailwind-merge` via the `cn()` helper for conditional classes. Use `bits-ui` primitives for accessible components.

## Rust Backend Code Style

### Naming Conventions

- Modules and functions: `snake_case`
- Types, structs, enums, traits: `CamelCase`
- Constants/statics: `SCREAMING_SNAKE_CASE`

### Module Organization

Group `#[tauri::command]` functions by feature in `src-tauri/src/commands/`. Register all commands in the `invoke_handler` in `lib.rs`.

### Error Handling

Use the shared `commands::errors::Error` enum for all command errors. It derives `thiserror::Error` and implements `serde::Serialize` for Tauri IPC. Add new variants there rather than using ad-hoc strings.

```rust
use super::errors::Error;

#[tauri::command]
pub async fn my_command(input: String) -> Result<MyResult, Error> {
    let data = std::fs::read_to_string(&input).map_err(Error::Io)?;
    Ok(process(data))
}
```

- Avoid `.unwrap()` in library/command code; use `?` or explicit `map_err`.
- `unwrap()` is acceptable in tests and the `regexp!` macro context.

### Imports

```rust
// 1. std
use std::sync::{Arc, Mutex};
// 2. external crates
use serde::{Deserialize, Serialize};
use tauri::State;
// 3. crate-internal
use crate::commands::errors::Error;
```

### Serde / IPC

Use `#[serde(rename_all = "camelCase")]` on structs/enums that cross the Tauri IPC boundary to match TypeScript conventions.

```rust
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub ai_grammar_enabled: bool,
    pub hotkey_enabled: bool,
}
```

### Async & Threading

- Use `tokio` for async commands and background loops.
- Shared mutable state passed to Tauri: wrap in `Arc<Mutex<T>>` and register via `app.manage(...)`.

## Testing Guidelines

- **No JS unit test suite** — `npm run check` and `npm run lint` are the frontend quality gates.
- **Desktop smoke testing** — run `npm run tauri:dev` and follow scenarios in `TESTING.md`.
- **Rust unit tests** — add `#[cfg(test)] mod tests { ... }` blocks in each module for pure logic; run with `cargo test`.
- **Single Rust test** — use `cargo test --manifest-path src-tauri/Cargo.toml -- <test_path>` (substring match).

## Commit & Pull Request Guidelines

- Conventional Commits: `feat: ...`, `feat(app): ...`, `fix: ...`, `chore: ...`, `refactor: ...`
- One logical change per commit; imperative mood in subject line.
- PRs must include: summary, linked issue(s), test steps/results, screenshots or GIFs for any UI change.
- macOS-only features (hotkeys, Accessibility, clipboard): note permission requirements and test on macOS.
- Do not edit files in `dist/` — these are build artifacts.
