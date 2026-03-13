# AutoCorrect Desktop App

A system-wide text correction and spell checking desktop application powered by [AutoCorrect](https://github.com/huacnlee/autocorrect). Built with Tauri 2 and Svelte 5 for a lightweight, cross-platform experience.

## Features

- **Real-time Overlay Markers**: Grammarly-style red underlines appear under typos as you type in any app
- **Hover Suggestion Popup**: Hover over a red underline to see a correction popup; press `Enter` to accept or `Esc` to dismiss
- **In-place Typo Replacement**: Accepting a suggestion replaces the exact typo word in the source application (Slack, Notes, etc.)
- **Global Hotkey Support**: Trigger spell checking from anywhere with `Cmd+Shift+A` (macOS)
- **Custom Rules**: Configure and customize correction rules to fit your needs
- **Multi-language Support**: Full support for English, Chinese, and other CJK languages
- **Lightweight**: Built with Rust backend and Svelte frontend for minimal resource usage

## Screenshots

<!-- Add screenshots here -->
- Main application window showing spell checker interface
- Settings panel with rule configuration
- Suggestion popup in action

## Installation

### Prerequisites

- **Node.js**: 18.x or later (install via [nvm](https://github.com/nvm-sh/nvm))
- **Rust**: 1.77.2 or later ([install instructions](https://www.rust-lang.org/tools/install))
- **System Dependencies**:
  - **macOS**: Xcode Command Line Tools (`xcode-select --install`)
  - **Linux**: WebView runtime (e.g., `webkit2gtk-4.1` on Ubuntu)
  - **Windows**: [Microsoft Visual C++ Redistributable](https://visualstudio.microsoft.com/vs/community/)

### Install from Release

1. Download the latest release from the [Releases](https://github.com/huacnlee/autocorrect/releases) page
2. Install the appropriate package for your platform:
   - **macOS**: Open the `.dmg` file and drag AutoCorrect to `/Applications`
   - **Windows**: Run the `.msi` installer
   - **Linux**: Install the `.deb` or `.AppImage` package

> **macOS users**: See [macOS Permissions Setup](#macos-permissions-setup) below — this is required for the overlay and hotkey features to work.

### Build from Source

```bash
# Clone the repository
git clone https://github.com/huacnlee/autocorrect.git
cd autocorrect/autocorrect-app

# Install Node dependencies
npm install

# Run in development mode
npm run tauri:dev

# Or build for release
npm run tauri:build
```

## Development Setup

### Quick Start

```bash
# Install dependencies
npm install

# Start development server with hot reload
npm run tauri:dev
```

### Available Scripts

| Command | Description |
|---------|-------------|
| `npm run dev` | Start Vite dev server (frontend only) |
| `npm run build` | Build frontend for production |
| `npm run tauri:dev` | Start full development environment (Tauri + Vite) |
| `npm run tauri:build` | Build release binaries |
| `npm run tauri:build:debug` | Build debug binaries (faster compilation) |
| `npm run check` | Run TypeScript and Svelte type checking |
| `npm run lint` | Run ESLint and Prettier checks |
| `npm run format` | Format code with Prettier |

### Project Structure

```
autocorrect-app/
├── src/                    # Frontend source (Svelte)
│   ├── lib/
│   │   ├── components/     # Svelte components
│   │   └── commands.svelte.ts  # Tauri command wrappers
│   ├── App.svelte          # Main application component
│   └── main.ts             # Application entry point
├── src-tauri/              # Backend source (Rust)
│   ├── src/
│   │   ├── commands/       # Tauri command handlers
│   │   ├── clipboard.rs    # Clipboard monitoring
│   │   ├── hotkey.rs       # Global hotkey handling
│   │   └── lib.rs          # Main application entry
│   ├── capabilities/       # Tauri permissions
│   ├── Cargo.toml          # Rust dependencies
│   └── tauri.conf.json     # Tauri configuration
└── package.json            # Node dependencies and scripts
```

## macOS Permissions Setup

AutoCorrect uses the macOS Accessibility API to read text from other apps, draw overlay markers, and simulate paste for typo replacement. These features require explicit user permission.

### First-time Setup (after installing to `/Applications`)

**Step 1 — Remove the quarantine flag** (critical: skipping this step causes permissions to reset on every launch)

```bash
sudo xattr -dr com.apple.quarantine /Applications/AutoCorrect.app
```

**Step 2 — Grant Accessibility permission**

Launch AutoCorrect. It will automatically open **System Settings → Privacy & Security → Accessibility**. Enable the toggle next to AutoCorrect.

**Step 3 — Restart AutoCorrect**

Quit and reopen the app. The red underline overlay will now appear as you type.

**Step 4 — Grant Input Monitoring permission** (for global hotkeys)

The first time you use `Cmd+Shift+A`, macOS will prompt for Input Monitoring. Alternatively, add AutoCorrect manually in **System Settings → Privacy & Security → Input Monitoring**.

### Why permissions reset after every rebuild

macOS ties Accessibility permissions to the app binary. When you rebuild (`pnpm tauri build`), the binary changes and the permission entry becomes invalid. Re-run Steps 1–4 after each build.

For permanent permission persistence across builds, [code-sign the app](https://tauri.app/distribute/sign/) with an Apple Developer certificate — permissions are then stored by code-signing identity, not binary hash.

### Supported Applications

The overlay works in apps that expose text via the macOS Accessibility API:

| Works | Does not work |
|-------|---------------|
| Slack, Discord | Terminal emulators (Ghostty, iTerm2, Terminal.app) |
| Notes, Mail, TextEdit | VS Code (terminal pane) |
| Chrome, Safari (input fields) | Sandboxed apps without AX support |
| Most native macOS apps | |

---

## Usage Guide

### Real-time Overlay (Hover Mode)

1. Type text with typos in any supported app (Slack, Notes, TextEdit, etc.)
2. Red underlines appear under detected typos within ~1 second
3. Hover the mouse over a red underline — a suggestion popup appears
4. Press `Enter` to accept and replace the typo, or `Esc` / click elsewhere to dismiss

### Global Hotkey (Selection Mode)

1. Select text containing errors in any application
2. Press `Cmd+Shift+A` (macOS)
3. A popup shows the corrected version of the entire selection
4. Press `Enter` to accept (replaces selection) or `Esc` to dismiss

### Settings Panel

Access the Settings panel to configure:

- **Enable/Disable AutoCorrect**: Toggle the entire correction system
- **Hotkey Configuration**: Change the global hotkey combination
- **Clipboard Monitor**: Enable automatic clipboard monitoring
  -CJK Only: Only monitor text containing CJK characters
  -Poll Interval: Adjust clipboard check frequency (ms)
- **Correction Rules**: View and customize correction rules
- **Language Settings**: Configure language-specific options

### System Tray

AutoCorrect runs in the background with a system tray icon:

- **Click**: Show/hide the main window
- **Right-click**: Quick actions menu
  - Enable/Disable AutoCorrect
  - Check spelling from clipboard
  - Open Settings
  - Quit application

## Configuration

AutoCorrect stores configuration in:

- **macOS**: `~/Library/Application Support/dev.autocorrect.app/`
- **Windows**: `%APPDATA%/autocorrect-app/`
- **Linux**: `~/.config/autocorrect-app/`

Configuration file (`config.json`):

```json
{
  "enabled": true,
  "hotkey": {
    "modifiers": ["command", "shift"],
    "key": "a"
  },
  "clipboard": {
    "enabled": false,
    "cjk_only": true,
    "poll_interval_ms": 500
  },
  "rules": {
    "enabled": ["spellcheck", "formatter"],
    "disabled": []
  }
}
```

## Building for Release

### Release Build

```bash
npm run tauri:build
```

Release binaries are created in `src-tauri/target/release/bundle/`:

- **macOS**: `.dmg` and `.app` bundles
- **Windows**: `.msi` installer
- **Linux**: `.deb` package and `.AppImage`

### Debug Build

For faster compilation during development:

```bash
npm run tauri:build:debug
```

### Code Signing

To code sign your builds (recommended for distribution):

1. **macOS**: Set up code signing certificates in `tauri.conf.json`
2. **Windows**: Install a code signing certificate and configure in `tauri.conf.json`
3. **Linux**: Package signing varies by distribution

See [Tauri's signing documentation](https://tauri.app/distribute/sign/) for details.

## Troubleshooting

### Red Underlines Not Appearing

1. **Check Accessibility permission**: Open System Settings → Privacy & Security → Accessibility. AutoCorrect must be listed and enabled.
2. **Remove quarantine flag**: Run `sudo xattr -dr com.apple.quarantine /Applications/AutoCorrect.app` then restart the app.
3. **Wrong app**: Overlay does not work in terminal emulators (Ghostty, iTerm2, Terminal.app) — this is intentional.
4. **Rebuilt the app?**: Re-grant Accessibility permission after every rebuild (see [macOS Permissions Setup](#macos-permissions-setup)).

### Permission Dialog Appears on Every Launch

This means the Accessibility permission is not persisting, almost always caused by the quarantine flag. Run:

```bash
sudo xattr -dr com.apple.quarantine /Applications/AutoCorrect.app
```

Then remove AutoCorrect from the Accessibility list and re-add it. For a permanent fix, [code-sign the app](https://tauri.app/distribute/sign/).

### Hotkey Not Working

- Ensure AutoCorrect is in System Settings → Privacy & Security → **Input Monitoring**
- Check that no other app uses `Cmd+Shift+A`
- Restart the application after granting Input Monitoring permission

### Typo Replacement Inserts Instead of Replacing

- Ensure the source app (Slack, Notes, etc.) has AX focus when you hover the popup
- The popup must receive focus (it should automatically) before pressing `Enter`
- If replacement still fails, try clicking the **Accept** button instead of pressing `Enter`

### Build Errors

- Ensure Rust and Node.js are up to date
- Clear the build cache: `rm -rf src-tauri/target`
- Reinstall dependencies: `rm -rf node_modules && npm install`

## Known Limitations

- Clipboard monitoring may not work with some applications that block clipboard access
- Global hotkey may conflict with other applications
- Text replacement in external applications is limited by platform accessibility APIs
- Some CJK language features require additional fonts or system support

## Roadmap

- [ ] Enhanced text replacement in external applications
- [ ] Custom dictionary management
- [ ] Language detection and automatic switching
- [ ] Cloud sync for settings and custom rules
- [ ] Browser extension integration
- [ ] Machine learning-powered suggestions
- [ ] Team/collaboration features

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](../CONTRIBUTING.md) for guidelines.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- [AutoCorrect](https://github.com/huacnlee/autocorrect) - Core spell checking engine
- [Tauri](https://tauri.app/) - Desktop application framework
- [Svelte](https://svelte.dev/) - Reactive UI framework
- [shadcn-svelte](https://next.shadcn-svelte.com/) - UI component library

## Support

- **Issues**: [GitHub Issues](https://github.com/huacnlee/autocorrect/issues)
- **Discussions**: [GitHub Discussions](https://github.com/huacnlee/autocorrect/discussions)
- **Documentation**: [AutoCorrect Docs](https://github.com/huacnlee/autocorrect)

## Version

Current version: **0.1.0**

See [CHANGELOG.md](CHANGELOG.md) for version history and updates.
