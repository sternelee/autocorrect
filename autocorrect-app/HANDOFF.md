# AutoCorrect App - Development Handoff

**Project Location:** `/Users/sternelee/www/github/autocorrect/autocorrect-app/`

**Date:** February 4, 2026

**Status:** Implementation Complete, Crash Bug Pending Resolution

---

## Overview

AutoCorrect App is a Tauri 2 + Svelte 5 desktop application providing system-wide text correction using the parent `autocorrect` Rust crate. The app features a global hotkey (Cmd+Shift+A) workflow that captures clipboard content, runs spell check, and displays a floating popup for accepting/rejecting corrections.

**Technology Stack:**

- Backend: Rust with Tauri 2.6.2
- Frontend: Svelte 5 (runes mode) + Tailwind CSS 4
- Template: `alysonhower/tauri2-svelte5-shadcn`
- Parent Dependency: `autocorrect` workspace crate at `../../autocorrect`

---

## Project Structure

```
autocorrect-app/
├── src/                          # Frontend source
│   ├── lib/
│   │   └── components/           # Svelte components
│   │       ├── SpellChecker.svelte      # Main spell-check interface
│   │       ├── SuggestionPopup.svelte   # In-app popup overlay
│   │       ├── SettingsPanel.svelte     # Configuration UI
│   │       └── StatusIndicator.svelte   # Status display
│   ├── App.svelte                # Main app component with routing
│   ├── popup.ts                  # Popup window TypeScript handler
│   └── main.ts                   # Frontend entry point
├── src-tauri/                    # Tauri backend
│   ├── src/
│   │   ├── commands/
│   │   │   ├── spellcheck.rs     # spell_check, clipboard, paste, config
│   │   │   ├── config.rs         # get_config, get_rules, update_config
│   │   │   ├── errors.rs         # Error types
│   │   │   └── default.rs        # Default file ops
│   │   ├── hotkey.rs             # Global hotkey listener (rdev)
│   │   ├── clipboard.rs          # Clipboard monitoring (CJK detection)
│   │   ├── popup.rs              # Popup window management
│   │   ├── commands.rs           # Command exports
│   │   └── lib.rs                # Main entry, setup, event loops
│   ├── Cargo.toml                # Rust dependencies
│   ├── tauri.conf.json           # App config (main + popup windows)
│   └── capabilities/
│       └── default.json          # Tauri permissions
├── popup.html                    # Standalone floating popup page
├── package.json                  # NPM scripts
├── README.md                     # Comprehensive docs
├── TESTING.md                    # Test cases
├── CHANGELOG.md                  # Version history
├── COMPLETION_SUMMARY.md         # Implementation summary
└── demo.sh                       # Interactive testing script
```

---

## Implemented Features

### 1. Tauri Backend Commands

**Spell Checking** (`commands/spellcheck.rs`):

- `spell_check(text: String)` - Returns corrected text with diff
- `get_clipboard_text()` - Read clipboard
- `set_clipboard_text(text: String)` - Write clipboard
- `simulate_paste()` - Send Cmd+V / Ctrl+V key combo
- `load_config()` - Load ~/.autocorrectrc
- `save_config(content: String)` - Save config file

**Configuration** (`commands/config.rs`):

- `get_config()` - Get current merged config (default + user)
- `get_default_config()` - Get default config as YAML
- `get_rules()` - Get all rules with severity and descriptions
- `update_config(updates: ConfigUpdates)` - Update rule severities, spellcheck words

**Popup Management** (`popup.rs`):

- `show_popup(x, y, original_text, suggestion)` - Show floating popup
- `hide_popup()` - Hide popup
- `position_popup(x, y)` - Move popup
- `get_popup_state()` - Get current popup state
- `accept_suggestion(text: String)` - Set clipboard + simulate paste
- `reject_suggestion()` - Just close popup
- `trigger_spell_check_workflow(x, y)` - Full workflow: clipboard → check → popup

**Clipboard Monitoring** (`clipboard.rs`):

- CJK (Chinese/Japanese/Korean) character detection
- Configurable poll interval
- Emits `clipboard-changed` events to frontend

**Global Hotkey** (`hotkey.rs`):

- Cross-platform global hotkey using `rdev`
- Default: `Cmd+Shift+A` (macOS) or `Ctrl+Shift+A` (others)
- Runs in separate thread, communicates via channels

### 2. Frontend Components

**Main Interface** (`App.svelte`):

- Tab-based navigation (Spell Check, Settings, About)
- Status indicator (enabled/disabled with corrections counter)
- Route handling for tab switching

**Spell Checker** (`SpellChecker.svelte`):

- Text input area
- Spell check button
- Display of corrections (original vs suggested)
- Accept/reject functionality

**Settings Panel** (`SettingsPanel.svelte`):

- Enable/disable AutoCorrect toggle
- Hotkey configuration display
- Clipboard monitor toggle + settings
- Rule toggles (on/off/warning/error)

**Status Indicator** (`StatusIndicator.svelte`):

- Green/gray dot for enabled/disabled
- Corrections counter
- Click to toggle status

### 3. Floating Popup Window

**Independent Window** (configured in `tauri.conf.json`):

```json
{
  "label": "popup",
  "url": "popup.html",
  "decorations": false,
  "transparent": true,
  "alwaysOnTop": true,
  "skipTaskbar": true,
  "visible": false,
  "width": 320,
  "height": 240
}
```

**Popup Features** (`popup.html` + `popup.ts`):

- Displays original text (strikethrough) and suggestion
- Accept button (sets clipboard + simulates paste)
- Reject button (closes popup)
- Edit button (custom correction)
- Keyboard shortcuts: Enter=Accept, Esc=Reject
- Click outside to close

### 4. Configuration

**Tauri Config** (`tauri.conf.json`):

- Product: AutoCorrect
- Version: 0.1.0
- Identifier: dev.autocorrect.app
- macOS Private API enabled
- Two windows: main + popup

**Rust Dependencies** (`Cargo.toml`):

```toml
autocorrect = { path = "../../autocorrect" }  # Workspace dependency
tauri = { version = "2.6.2", features = ["macos-private-api"] }
enigo = "0.2"      # Keyboard simulation
rdev = "0.5"       # Global hotkeys
arboard = "3"      # Clipboard access
tokio = { version = "1", features = ["time"] }
```

**Frontend Dependencies** (`package.json`):

- Svelte 5.28.1 (runes mode)
- Tailwind CSS 4.1.7
- Tauri API 2.5.0
- lucide-svelte (icons)

### 5. Key Workflow

**Global Hotkey Workflow:**

1. User presses `Cmd+Shift+A` anywhere
2. `hotkey.rs` detects combo, sends `SpellCheckTriggered` event
3. `lib.rs` event loop triggers `trigger_spell_check_workflow()`
4. Get clipboard text
5. Run `autocorrect::format_for()`
6. If changes detected, show popup at cursor position
7. User accepts: set clipboard + simulate paste (Cmd+V)
8. User rejects: just hide popup

---

## Known Issues

### Critical: App Crashes on Startup

**Status:** UNCONFIRMED - Needs debugging

**Description:** The application crashes during startup. The crash likely occurs during hotkey listener initialization in `lib.rs` setup function.

**Potential Causes:**

1. Hotkey thread initialization issue
2. rdev::listen blocking behavior
3. Permissions issue on macOS (Accessibility)
4. Channel setup between threads

**Debugging Steps:**

1. Run with RUST_BACKTRACE=1: `RUST_BACKTRACE=1 npm run tauri:dev`
2. Check macOS System Settings → Privacy & Security → Accessibility
3. Add logging in hotkey.rs to trace initialization
4. Try disabling hotkey listener temporarily
5. Check if clipboard monitor channel is causing issues

**Location:** `/Users/sternelee/www/github/autocorrect/autocorrect-app/src-tauri/src/lib.rs` lines 43-83

---

## Development Commands

**Development:**

```bash
npm run tauri:dev        # Full dev environment (Vite + Tauri)
npm run dev              # Frontend only (port 1420)
```

**Building:**

```bash
npm run tauri:build          # Release build (slow, optimized)
npm run tauri:build:debug    # Debug build (faster)
npm run check                # Svelte type checking
```

**Testing:**

```bash
./demo.sh                # Interactive demo/testing script
```

---

## Configuration Files

**User Config Location:** `~/.autocorrectrc`

**Config Structure:**

```yaml
rules:
  space-word: 2 # 0=off, 1=error, 2=warning
  fullwidth: 1
textRules:
  halfwidth-quotes: 2
spellcheck:
  words:
    - customword
fileTypes:
  .md: markdown
```

**Default Config Embedded:** `../../../../autocorrect/.autocorrectrc.default`

---

## Platform-Specific Notes

### macOS

- Private API enabled for future enhancements
- Accessibility permissions required for global hotkey
- Cursor position detection is currently placeholder (returns 800, 400)
- Full implementation would use NSEvent or CGEvent

### Windows/Linux

- Uses Ctrl+Shift+A hotkey
- Similar permission requirements may apply

---

## File Locations (Absolute Paths)

**Key Implementation Files:**

- `/Users/sternelee/www/github/autocorrect/autocorrect-app/src-tauri/src/lib.rs` - Main entry, setup
- `/Users/sternelee/www/github/autocorrect/autocorrect-app/src-tauri/src/hotkey.rs` - Global hotkey
- `/Users/sternelee/www/github/autocorrect/autocorrect-app/src-tauri/src/popup.rs` - Popup management
- `/Users/sternelee/www/github/autocorrect/autocorrect-app/src-tauri/src/clipboard.rs` - Clipboard monitoring
- `/Users/sternelee/www/github/autocorrect/autocorrect-app/src-tauri/src/commands/spellcheck.rs` - Spell commands
- `/Users/sternelee/www/github/autocorrect/autocorrect-app/src-tauri/src/commands/config.rs` - Config commands
- `/Users/sternelee/www/github/autocorrect/autocorrect-app/src/popup.ts` - Popup frontend
- `/Users/sternelee/www/github/autocorrect/autocorrect-app/popup.html` - Popup page
- `/Users/sternelee/www/github/autocorrect/autocorrect-app/src-tauri/tauri.conf.json` - App config
- `/Users/sternelee/www/github/autocorrect/autocorrect-app/src-tauri/Cargo.toml` - Rust deps

**Documentation:**

- `/Users/sternelee/www/github/autocorrect/autocorrect-app/README.md`
- `/Users/sternelee/www/github/autocorrect/autocorrect-app/TESTING.md`
- `/Users/sternelee/www/github/autocorrect/autocorrect-app/CHANGELOG.md`
- `/Users/sternelee/www/github/autocorrect/autocorrect-app/COMPLETION_SUMMARY.md`

---

## Next Steps for Continuation

### 1. Debug Startup Crash (Priority: High)

- Investigate hotkey listener initialization
- Add error handling in lib.rs setup
- Verify macOS permissions
- Consider lazy initialization of hotkey listener

### 2. Complete Workflow Testing

- Once crash is fixed, test full workflow:
  - Press hotkey
  - Verify popup appears
  - Test accept/reject buttons
  - Verify clipboard + paste simulation

### 3. Cursor Position Detection

- Implement proper macOS cursor position using NSEvent/CGEvent
- Replace placeholder (800, 400) in `get_cursor_position()`

### 4. Clipboard Monitor Enhancement

- Currently detects changes but doesn't auto-replace
- Implement automatic text replacement workflow

### 5. Svelte 5 Compatibility Fixes (from COMPLETION_SUMMARY.md)

- Fix lucide-svelte import errors
- Update components to use `$props()` instead of `export let`
- Fix duplicate imports in switch.svelte
- Convert style objects to strings

---

## References

**Parent Project:** `/Users/sternelee/www/github/autocorrect/`

- AutoCorrect Rust crate at workspace path
- Used for all text correction logic

**Template:** `alysonhower/tauri2-svelte5-shadcn`

- Base template with shadcn/ui components
- Svelte 5 runes mode enabled

**Tauri Docs:** https://tauri.app/v2/
**Svelte 5 Docs:** https://svelte.dev/docs

---

## Contact / Context

This handoff was created after implementing the full Tauri 2 + Svelte 5 desktop application with global hotkey functionality and floating popup. The app is functionally complete but has a startup crash that needs resolution before testing can proceed.

**Branch:** feat/app
**Parent Repo:** https://github.com/huacnlee/autocorrect
**App Location:** `/Users/sternelee/www/github/autocorrect/autocorrect-app/`
