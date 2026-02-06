# AutoCorrect Agent Guidelines

This repository contains the AutoCorrect CJK text formatter and linter. It is a multi-package Rust workspace with various language bindings.

## Build and Test Commands

### Core Rust Commands
- **Build CLI:** `make build` (builds and installs to path) or `cargo build -p autocorrect-cli --release`
- **Run All Tests:** `make test`
- **Run Single Test:** `cargo test -p <package> --lib <path::to::test>`
  - Example: `cargo test -p autocorrect --lib rule::tests::test_format_part`
  - Workspace test: `cargo test -p autocorrect --lib tests::test_format`
- **Test Stdin:** `make test:stdin` (verifies pipe support)
- **Run Benchmarks:** `make bench` (requires `criterion`)

### Linting & Fixtures
- **Lint All Fixtures:** `make test:lint` (runs on `tests/fixtures/*.raw.*`)
- **Run Lint on Single File:** `cargo run -- --lint tests/fixtures/example.raw.txt`
- **Add New Test Case:** Add `name.raw.ext` (input) and `name.fixed.ext` (expected) to `tests/fixtures/`.
- **JSON Output:** `cargo run -- --lint --format json`

### Language Bindings
- **Node.js:** `make test:node` (requires `yarn`, uses `napi-rs`)
- **Python:** `make test:python` (requires `pytest`, uses `pyo3`)
- **Ruby:** `make test:ruby` (requires `bundle`, uses `rutie/rb-sys`)
- **Java:** `make test:java` (requires `maven`, uses `jni`)
- **WASM:** `make wasm` (requires `wasm-pack` and `wasm-opt`)

### Desktop Application (autocorrect-app)
- **Framework:** Tauri 2.0 + Svelte 5 + Tailwind CSS 4.
- **Build/Dev:**
  - `cd autocorrect-app && pnpm install` (or `npm install`)
  - `pnpm tauri dev`: Run in development mode with hot reload.
  - `pnpm tauri build`: Build the production application.
- **Lint/Check:**
  - `pnpm lint`: Lint with Prettier and ESLint.
  - `pnpm check`: Svelte-check and TypeScript type checking.
- **Features:** System-wide text correction, global hotkeys, macOS accessibility integration, clipboard monitoring.

## Project Structure
- `autocorrect/`: Core library logic, rules, and file-type parsers.
- `autocorrect-cli/`: Command-line interface with `tokio`.
- `autocorrect-lsp/`: Language Server Protocol implementation using `tower-lsp`.
- `autocorrect-node/`, `autocorrect-py/`, `autocorrect-rb/`, `autocorrect-java/`, `autocorrect-wasm/`: Bindings.
- `autocorrect-app/`: Desktop application (Tauri + Svelte 5).
  - `src-tauri/`: Rust backend, system integration (hotkeys, accessibility).
  - `src/`: Frontend UI (Svelte components).
- `tests/fixtures/`: Regression test data for all supported file types.

## Code Style Guidelines

### Rust General
- **Formatting:** Adhere to standard `rustfmt` defaults (no custom config).
- **Edition:** Using Rust 2024 edition for most crates.
- **Naming:** `snake_case` for functions/variables, `PascalCase` for types/traits, `SCREAMING_SNAKE_CASE` for constants/statics.
- **Imports:** 
  1. `std` imports
  2. External crate imports
  3. `crate::` imports
  Example:
  ```rust
  use std::borrow::Cow;
  use regex::Regex;
  use crate::rule::Rule;
  ```

### Error Handling & Safety
- **Libraries:** Use `anyhow::Result` for application logic and flexible error handling.
- **Avoid Panics:** Generally avoid `.unwrap()` in library code (`autocorrect/src`). Use `?` or handle errors. 
- **Macros:** `unwrap()` is acceptable within the `regexp!` macro and tests where failure is expected to stop execution.
- **Safety:** Minimize `unsafe` blocks. Most binding crates use high-level abstractions (`napi-rs`, `pyo3`).

### Patterns & Architecture

#### CJK Rules Implementation
Rules are defined in `autocorrect/src/rule/` using `Strategery` and the `regexp!` macro.
- `Strategery::new(one, other)`: Adds space between two regex patterns.
- `.with_reverse()`: Also applies the rule in reverse order.
- `.with_remove_space()`: Removes space instead of adding.

Example of a rule definition:
```rust
Strategery::new(r"\p{CJK}", r"[a-zA-Z0-9]").with_reverse()
```

#### Macros
- `regexp!`: Custom macro for CJK regex shortcuts.
  - `\p{CJK}`: Matches Han, Hangul, Katakana, Hiragana, Bopomofo.
  - `\p{CJ}`: Matches Han, Katakana, Hiragana, Bopomofo.
- `map!`: Shorthand for `HashMap` creation.
- `cases!`: Shorthand for multi-line test cases in unit tests.

#### Ignorer Logic
- Supports `.autocorrectignore` and `.gitignore`.
- Implementation in `autocorrect/src/ignorer.rs` using the `ignore` crate.

#### LSP Features
- Real-time linting and formatting.
- Spellcheck integration via `typos` crate.
- Code actions for quick fixes.

## Development Workflow

### Adding a New Rule
1. Add rule logic in `autocorrect/src/rule/word.rs` or a new module.
2. Register the rule in `autocorrect/src/rule/mod.rs` within `RULES` or `AFTER_RULES`.
3. Add unit tests in the same file.
4. Add fixture tests in `tests/fixtures/`.

### Updating Bindings
When changing the core API:
1. Update `autocorrect/src/lib.rs` and its public exports.
2. Update all binding crates in `autocorrect-*/src/lib.rs`.
3. Run `make test:node`, `make test:python`, etc., to verify consistency.

## Note on External Rules
No `.cursorrules` or `.github/copilot-instructions.md` were found in this repository. Use these guidelines as the primary source of truth for agent behavior. If you find conflicts with project-specific documentation like `DEVELOPMENT.md` or `CLAUDE.md`, prioritize these guidelines for automation tasks.
