# Repository Guidelines

## Project Structure & Module Organization
- `src/`: Svelte 5 + TypeScript frontend.
- `src/lib/components/`: UI components (`PascalCase.svelte`), including `ui/` primitives.
- `src/lib/commands.svelte.ts`: frontend wrappers for Tauri commands.
- `src-tauri/src/`: Rust backend (app entry, hotkey, clipboard, spellcheck, command handlers).
- `src-tauri/src/commands/`: Tauri `#[command]` implementations grouped by feature.
- `static/`: static assets for the web layer. `dist/` is build output (do not hand-edit).
- `README.md`, `TESTING.md`, and feature notes (`*_DEBUG.md`, `*_COMPLETE.md`) capture operational context.

## Build, Test, and Development Commands
- `npm install`: install frontend/tooling dependencies.
- `npm run tauri:dev`: run full desktop app in development mode (frontend + Rust backend).
- `npm run dev`: run frontend-only Vite dev server.
- `npm run build`: build frontend bundle.
- `npm run tauri:build` / `npm run tauri:build:debug`: build release/debug desktop binaries.
- `npm run check`: Svelte + TypeScript type checks.
- `npm run lint`: Prettier check + ESLint.
- `npm run format`: format repository with Prettier.
- `cargo test --manifest-path src-tauri/Cargo.toml`: run Rust tests.

## Coding Style & Naming Conventions
- Frontend style is enforced by Prettier + ESLint (`eslint.config.js`). Run `npm run format` and `npm run lint` before PRs.
- Use `PascalCase` for Svelte components, `camelCase` for TS variables/functions, and `kebab-case` for non-component filenames where applicable.
- Rust follows idiomatic conventions: modules/functions in `snake_case`, types/enums in `CamelCase`; keep code `rustfmt`-clean.

## Testing Guidelines
- No dedicated JS unit test suite is configured; use `npm run check` + `npm run lint` as required quality gates.
- Validate desktop behavior with `npm run tauri:dev` using scenarios in `TESTING.md`.
- Add/maintain Rust unit tests in `src-tauri` for backend logic changes and run `cargo test`.

## Commit & Pull Request Guidelines
- Prefer Conventional Commit style seen in history: `feat: ...`, `feat(app): ...`, `chore: ...`, `fix: ...`.
- Keep commits scoped and imperative (one change per commit when practical).
- PRs should include: concise summary, linked issue(s), test steps/results, and screenshots/GIFs for UI changes (e.g., `src/App.svelte`, popup/overlay behavior).
- Note platform-specific validation for hotkey/clipboard changes (especially macOS permissions/accessibility).
