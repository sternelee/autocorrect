# AutoCorrect Desktop App

A system-wide text correction and spell checking desktop application powered by [AutoCorrect](https://github.com/huacnlee/autocorrect). Built with Tauri 2 and Svelte 5 for a lightweight, cross-platform experience.

## Features

- **Real-time Spell Checking**: Instant text correction using AutoCorrect's powerful engine
- **Global Hotkey Support**: Trigger spell checking from anywhere with `Cmd+Shift+A` (macOS) or `Ctrl+Shift+A` (Windows/Linux)
- **Clipboard Monitoring**: Automatically detect and correct text copied to clipboard
- **Custom Rules**: Configure and customize correction rules to fit your needs
- **Multi-language Support**: Full support for English, Chinese, and other CJK languages
- **Lightweight**: Built with Rust backend and Svelte frontend for minimal resource usage
- **Cross-platform**: Works on macOS, Windows, and Linux

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
   - **macOS**: Open the `.dmg` file and drag AutoCorrect to Applications
   - **Windows**: Run the `.msi` installer
   - **Linux**: Install the `.deb` or `.AppImage` package

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

## Usage Guide

### Basic Spell Checking

1. **Using the App Window**:
   - Open AutoCorrect from your Applications menu
   - Type or paste text into the main spell checker area
   - Click "Check Spelling" to see corrections
   - Accept or reject individual suggestions

2. **Using Global Hotkey**:
   - Select text in any application
   - Press `Cmd+Shift+A` (macOS) or `Ctrl+Shift+A` (Windows/Linux)
   - AutoCorrect will show a popup with spelling suggestions
   - Press `Enter` to accept or `Esc` to dismiss

3. **Using Clipboard Monitoring**:
   - Enable "Clipboard Monitor" in Settings
   - Copy text to clipboard (`Cmd+C` or `Ctrl+C`)
   - AutoCorrect automatically detects and corrects the text
   - The corrected text replaces your clipboard contents

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

### Hotkey Not Working

- Check that no other application is using the same hotkey
- Verify AutoCorrect has necessary system permissions (macOS: Accessibility, Windows: Input monitoring)
- Restart the application after changing settings

### Clipboard Monitor Issues

- Some applications may block clipboard access
- Adjust the poll interval in Settings if monitoring is slow
- Disable "CJK Only" to monitor all clipboard content

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
