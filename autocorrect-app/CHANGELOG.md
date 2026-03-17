# Changelog

All notable changes to the AutoCorrect Desktop App will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-02-04

### Added

- **Initial Release of AutoCorrect Desktop Application**
  - Real-time spell checking powered by AutoCorrect engine
  - Global hotkey support (Cmd+Shift+A / Ctrl+Shift+A) for system-wide corrections
  - Clipboard monitoring for automatic text correction
  - Cross-platform support (macOS, Windows, Linux)
  - Built with Tauri 2 and Svelte 5

### Features

- **Spell Checking Interface**
  - Main spell checker tab with text input area
  - Real-time correction suggestions
  - Accept/reject individual corrections
  - Support for English and CJK languages

- **Global Hotkey System**
  - System-wide hotkey for triggering corrections
  - Configurable hotkey combinations
  - Popup interface for quick corrections
  - Works with any application

- **Clipboard Monitoring**
  - Automatic detection of clipboard changes
  - Optional CJK-only filtering
  - Configurable poll interval
  - Background operation

- **Settings Panel**
  - Enable/disable AutoCorrect
  - Hotkey configuration
  - Clipboard monitor controls
  - Correction rules management
  - Configuration persistence

- **User Interface**
  - Clean, modern design with shadcn-svelte components
  - Status indicator showing enabled/disabled state
  - Navigation between Spell Check, Settings, and About tabs
  - Responsive layout

### Technology Stack

- **Backend**: Rust with Tauri 2
  - AutoCorrect integration for spell checking
  - rdev for global hotkey handling
  - arboard for clipboard access
  - enigo for text simulation

- **Frontend**: Svelte 5 with Tailwind CSS 4
  - Reactive UI with Svelte 5 runes
  - Modern component library (shadcn-svelte)
  - TypeScript for type safety

### Known Limitations

- Clipboard monitoring detects changes but automatic text replacement in external applications is limited
- Text replacement in external applications depends on platform accessibility APIs
- Global hotkey may conflict with other applications using the same combination
- Some applications may block clipboard access
- No custom dictionary management yet
- No cloud sync for settings

### Roadmap

#### Upcoming Features (0.2.0)

- [ ] Enhanced text replacement in external applications
- [ ] Custom dictionary management UI
- [ ] Language detection and automatic switching
- [ ] System tray integration
- [ ] Auto-start on system boot

#### Future Features (1.0.0)

- [ ] Cloud sync for settings and custom rules
- [ ] Browser extension integration
- [ ] Machine learning-powered suggestions
- [ ] Team/collaboration features
- [ ] Advanced rule editor
- [ ] Statistics and usage tracking

### Development Notes

- Minimum Node.js version: 18.x
- Minimum Rust version: 1.77.2
- Tested on macOS 14+, Windows 10+, Ubuntu 22.04+

### Documentation

- Comprehensive README with installation and usage instructions
- Testing guide with step-by-step test cases
- Inline code documentation
- Demo script for guided testing

### Contributors

- AutoCorrect Contributors

---

## Version History Format

Each version includes:

- **Added**: New features
- **Changed**: Changes to existing functionality
- **Deprecated**: Features to be removed in future releases
- **Removed**: Features removed in this release
- **Fixed**: Bug fixes
- **Security**: Security vulnerability fixes

---

[Unreleased]: https://github.com/huacnlee/autocorrect/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/huacnlee/autocorrect/releases/tag/v0.1.0
