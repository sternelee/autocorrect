# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

AutoCorrect Desktop App is a system-wide text correction and spell checking application built with Tauri 2 (Rust backend) and Svelte 5 (frontend). It provides Grammarly-like real-time text checking with overlay markers and a suggestion popup system for macOS.

## Build and Development Commands

```bash
# Install dependencies
npm install

# Development (hot reload)
npm run tauri:dev

# Build for release
npm run tauri:build

# Debug build (faster compilation)
npm run tauri:build:debug

# Frontend-only (no Rust backend)
npm run dev

# Type checking
npm run check

# Linting and formatting
npm run lint
npm run format
```

## Architecture

### Tauri Backend (src-tauri/)

The Rust backend handles system integration, spell checking, and UI coordination:

**Entry Point** (`lib.rs`):
- Initializes popup state, overlay manager, and hotkey listener
- Spawns background thread for system typo sync (800ms polling)
- Routes hotkey events to spell check workflow

**Core Modules**:

| Module | Purpose |
|--------|---------|
| `popup.rs` | Popup window state management and suggestion acceptance/rejection |
| `overlay.rs` | Transparent fullscreen overlay for typo markers (Grammarly-style underlines) |
| `text_selection.rs` | System-wide text selection via Accessibility API with clipboard fallback |
| `macos_text.rs` | macOS Accessibility API bindings (AXUIElement, focused element data) |
| `hotkey.rs` | Global hotkey listener using rdev |
| `clipboard.rs` | Clipboard monitoring and text operations |
| `typocheck.rs` | Typo detection using the `typos` crate |

**Commands** (`commands/`):

| Command File | Functions |
|--------------|-----------|
| `spellcheck.rs` | `spell_check`, `get_clipboard_text`, `set_clipboard_text`, `simulate_paste` |
| `config.rs` | Configuration management for AutoCorrect rules |
| `hotkey_config.rs` | Hotkey customization and persistence |
| `custom_corrections.rs` | User-defined corrections dictionary |
| `ai_grammar.rs` | OpenAI/OpenRouter integration for AI-powered grammar checking |
| `errors.rs` | Unified error types for Tauri commands |

**Spell Check Pipeline** (`commands/spellcheck.rs`):
1. Local formatting via `autocorrect::format_for()` (CJK spacing, punctuation)
2. Lint via `autocorrect::lint_for()` for line-by-line changes
3. Typo detection via `typos` crate (configurable)
4. CSpell integration (optional)
5. AI grammar enhancement via OpenAI-compatible API (optional)

### Frontend (src/)

**Main Components**:

| Component | Purpose |
|-----------|---------|
| `SpellChecker.svelte` | Main text input and correction interface |
| `SettingsPanel.svelte` | Configuration UI for rules, hotkeys, AI settings |
| `SuggestionPopup.svelte` | Floating suggestion window |
| `StatusIndicator.svelte` | Enable/disable toggle and correction count |
| `CustomCorrectionsManager.svelte` | Custom dictionary management |

**UI Library**: Tailwind CSS 4 with shadcn-svelte components (`src/lib/components/ui/`)

### Windows (tauri.conf.json + programmatic)

1. **main** (800x600): Primary application window (`index.html`)
2. **popup** (320x240): Transparent, always-on-top suggestion popup (`popup.html`)
3. **overlay**: Fullscreen transparent window created programmatically for typo markers (click-through, `overlay.html`)

### Event Flow

```
Hotkey (Cmd+Shift+A) → hotkey.rs → trigger_spell_check_workflow()
    ↓
get_selected_text() → text_selection.rs
    ↓ (Accessibility API or clipboard fallback)
spell_check() → commands/spellcheck.rs
    ↓ (autocorrect + typos + optional AI)
show_popup() → popup.rs → emit "popup-show" event
    ↓
Frontend renders suggestions → accept_suggestion() / reject_suggestion()
    ↓ (accept)
apply_suggestion_to_selection_macos() → clipboard + paste simulation
```

### Configuration

- **App settings**: `~/.autocorrect-app.json` (AI, CSpell, typo checking toggles)
- **AutoCorrect rules**: `~/.autocorrectrc` (formatting rules)
- **Hotkey config**: Stored via `hotkey_config.rs` in app data directory

### Frontend-Backend Communication

- **Tauri commands**: Frontend invokes Rust functions via `@tauri-apps/api` (`invoke()`)
- **Tauri events**: Backend emits events like `popup-show`, `popup-hide`, `overlay-update` for reactive UI
- **Capabilities**: Permissions defined in `src-tauri/capabilities/default.json`

## macOS-Specific Notes

- Requires Accessibility permissions (`check_and_request_accessibility()`)
- Uses private APIs (`macos-private-api` feature in tauri.conf.json)
- Overlay uses `setIgnoresMouseEvents: true` for click-through
- Text replacement uses AppleScript for paste simulation

## Key Dependencies

- `autocorrect` (path dependency): Core formatting engine
- `typos` / `typos-cli`: Spell checking
- `enigo`: Keyboard simulation
- `rdev`: Global hotkey listening (forked version for macOS stability)
- `arboard`: Clipboard operations
- `cocoa`, `objc`, `core-foundation`: macOS Accessibility bindings
- `tauri-plugin-log`: Logging
- `tauri-plugin-http`: HTTP requests (for AI grammar API)
