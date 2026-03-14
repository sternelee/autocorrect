# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

AutoCorrect Desktop App is a Tauri 2 + Svelte 5 desktop client for system-wide text correction on macOS. It combines:

- local formatting/linting from the Rust `autocorrect` crate,
- typo detection via `typos`,
- optional AI transforms via OpenAI-compatible chat-completions APIs,
- macOS Accessibility-based selection/replacement,
- always-on-top popup/overlay windows for inline UX.

## Core Development Commands

```bash
# Install JS dependencies
npm install

# Run app in development (starts Vite + Tauri)
npm run tauri:dev

# Frontend-only dev server
npm run dev

# Typecheck Svelte + TS
npm run check

# Lint (Prettier check + ESLint)
npm run lint

# Auto-format
npm run format

# Production and debug builds
npm run tauri:build
npm run tauri:build:debug
```

## Rust Commands (backend only)

```bash
# Build backend crate only
cargo build --manifest-path src-tauri/Cargo.toml

# Run backend tests
cargo test --manifest-path src-tauri/Cargo.toml

# Run a single Rust test (by name filter)
cargo test --manifest-path src-tauri/Cargo.toml <test_name>

# Run a specific Rust integration/example target
cargo test --manifest-path src-tauri/Cargo.toml --test <test_target>
cargo run --manifest-path src-tauri/Cargo.toml --example <example_name>
```

## Architecture (Big Picture)

### 1) Multi-window Tauri app with Svelte entrypoints

Configured in `src-tauri/tauri.conf.json` and mounted from:

- `src/main.ts` → main settings/checker window (`src/App.svelte`)
- `src/pages/popup/main.ts` → suggestion popup UI
- `src/pages/overlay/main.ts` → transparent typo underline overlay
- `src/pages/ai-popup/main.ts` → AI tools popup

The backend orchestrates all windows and emits events; Svelte pages are thin reactive views over Tauri commands/events.

### 2) Backend as orchestration layer (`src-tauri/src/lib.rs`)

`run()` wires up the app lifecycle:

- initializes shared popup/AI/overlay state,
- registers Tauri commands,
- starts background threads for:
  - system typo synchronization loop (~800ms),
  - hover/click detection for marker interactions,
  - hotkey event handling,
  - clipboard event forwarding.

This file is the control plane: if behavior spans selection → checking → overlays/popups, start here.

### 3) Spellcheck pipeline and config split

Primary spellcheck path is in `src-tauri/src/commands/spellcheck.rs`:

1. `autocorrect::format_for` (correction output)
2. `autocorrect::lint_for` (structured line-level diffs)
3. `typos` detection (optional via settings)
4. optional AI post-processing when enabled.

Config is split intentionally:

- `~/.autocorrectrc` for autocorrect rules/word lists (YAML)
- `~/.autocorrect-app.json` for app-only settings (AI, typo toggle, underline style/color, etc.)

Config read/write and projection to frontend happen in `src-tauri/src/commands/config.rs`.

### 4) System integration boundary (macOS)

macOS-specific behavior lives in:

- `macos_text.rs` (Accessibility APIs, focused context, range bounds, selection operations)
- `text_selection.rs` (selection retrieval + clipboard fallback)
- `popup.rs` (focus return + replacement flow via clipboard/paste)
- `overlay.rs` (marker rendering coordination)

When fixing issues like “wrong range replaced”, “popup focus oddities”, or “marker position drift”, trace through these modules together; they form one runtime flow.

### 5) Event-driven frontend/backend contract

Backend emits events (e.g. `popup-show`, `popup-hide`, `update-markers`, `underline-config-update`), and frontend pages subscribe via `@tauri-apps/api/event`.

Backend commands are invoked from Svelte via `invoke()` for mutations and requests. Most UI behavior is therefore a command + event roundtrip rather than shared state.

## Key Frontend Surfaces

- `src/App.svelte`: main window tab shell (spell check, settings, about)
- `src/lib/components/SpellChecker.svelte`: manual checking + AI transform tools
- `src/lib/components/SettingsPanel.svelte`: rules, AI settings, hotkey, underline appearance, import/export
- `src/pages/popup/Popup.svelte`: lightweight correction popup with suggestion chips and custom dictionary action
- `src/pages/overlay/Overlay.svelte`: purely visual typo underline renderer
- `src/pages/ai-popup/AiPopup.svelte`: contextual AI actions for selected text

## Hotkey and popup workflow

High-level runtime path:

1. Global hotkey triggers spell-check workflow.
2. Selected text is fetched (AX API first, clipboard fallback path).
3. Spellcheck pipeline returns corrected text + typo suggestions.
4. Popup is shown near cursor or hovered marker.
5. Accept/reject routes through `popup.rs` to apply replacement and restore focus to source app.

If this chain breaks, inspect `hotkey.rs` + `popup.rs` + `text_selection.rs` + `macos_text.rs` together.

## Notes for future edits

- Keep command names and payload shapes aligned between Rust `#[tauri::command]` functions and Svelte `invoke()` calls.
- Underline style/color settings are persisted in app settings and pushed live via `underline-config-update`; update both persistence and event emission when changing appearance behavior.
- This repo currently has no dedicated frontend test runner script in `package.json`; validation is primarily `npm run check`, `npm run lint`, and Rust tests/builds.