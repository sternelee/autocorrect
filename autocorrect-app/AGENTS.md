# Repository Guidelines

## Project Overview

This is the **AutoCorrect** desktop application — a cross-platform system-wide text correction tool built with **Tauri 2** (Rust backend) and **Svelte 5** with **TypeScript** and **Tailwind CSS 4** (frontend). macOS-specific features include global hotkeys, Accessibility-based text selection, clipboard monitoring, and overlay popups.

## Project Structure & Module Organization

```
autocorrect-app/
├── src/                        # Frontend (Svelte 5 + TypeScript)
│   ├── lib/
│   │   ├── components/         # Svelte components (PascalCase.svelte)
│   │   │   └── ui/             # Primitive UI components
│   │   ├── commands.svelte.ts  # Tauri invoke wrappers
│   │   └── i18n/               # Localization helpers
│   ├── App.svelte              # Root component / tab routing
│   └── main.ts                 # Entry point
├── src-tauri/
│   ├── Cargo.toml              # Rust dependencies
│   ├── capabilities/           # Tauri permissions
│   ├── tauri.conf.json         # Tauri configuration
│   └── src/                    # Rust backend
│       ├── commands/           # #[tauri::command] handlers grouped by feature
│       ├── lib.rs              # App bootstrap and event loop
│       ├── hotkey.rs           # Global hotkey handling
│       ├── clipboard.rs        # Clipboard monitoring
│       └── popup.rs / overlay.rs / ai_popup.rs  # UI overlays
└── static/                     # Static web assets (favicon, etc.)
```

## Build, Test, and Development Commands

| Command                                           | Purpose                                 |
| ------------------------------------------------- | --------------------------------------- |
| `npm install`                                     | Install Node dependencies               |
| `npm run dev`                                     | Start the Vite frontend-only dev server |
| `npm run tauri:dev`                               | Run the full Tauri app with hot reload  |
| `npm run tauri:build`                             | Build release binaries                  |
| `npm run tauri:build:debug`                       | Build debug binaries (faster)           |
| `npm run check`                                   | Svelte + TypeScript type checking       |
| `npm run lint`                                    | Prettier check + ESLint                 |
| `npm run format`                                  | Auto-format with Prettier               |
| `cargo test --manifest-path src-tauri/Cargo.toml` | Run all Rust backend tests              |

## Coding Style & Naming Conventions

- **TypeScript / Svelte**: use Svelte 5 runes (`$state`, `$derived`, `$effect`).
  - Components: `PascalCase.svelte`
  - Variables / functions: `camelCase`
  - Types / interfaces: `PascalCase`
  - Use `cn()` (from `clsx` + `tailwind-merge`) for conditional Tailwind classes.
- **Rust**: follow standard Rust conventions.
  - Modules / functions: `snake_case`
  - Types / structs: `PascalCase`
  - Constants: `SCREAMING_SNAKE_CASE`
  - Avoid `.unwrap()` outside tests; use `?` or `map_err`.
- Use `#[serde(rename_all = "camelCase")]` on Rust structs that cross the Tauri IPC boundary.

## Testing Guidelines

- **Frontend quality gates**: `npm run check` and `npm run lint` (no JS unit test suite).
- **Rust unit tests**: add `#[cfg(test)]` modules in each backend file. Run:

  ```bash
  cargo test --manifest-path src-tauri/Cargo.toml
  ```

- Run a single Rust test with a substring match:

  ```bash
  cargo test --manifest-path src-tauri/Cargo.toml -- <test_name>
  ```

- **Desktop smoke testing**: run `npm run tauri:dev` and follow the scenarios in `TESTING.md`.

## Commit & Pull Request Guidelines

- Use **Conventional Commits**: `feat:`, `fix:`, `chore:`, `refactor:`, `feat(app):`, etc.
- Write concise, imperative subject lines; one logical change per commit.
- Pull requests should include:
  - A brief summary and linked issue(s)
  - Test steps and results
  - Screenshots or GIFs for any UI changes
  - Notes on macOS permission requirements for hotkey / Accessibility features

## Agent-Specific Instructions

- Do not edit files in `dist/` — they are build artifacts.
- Run the full frontend and Rust quality gates before finishing work.
- For macOS-only features, verify permissions and test on macOS when possible.
