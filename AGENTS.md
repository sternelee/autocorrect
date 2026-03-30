# AutoCorrect Agent Guidelines

This repository contains the AutoCorrect CJK text formatter and linter. It is a multi-package Rust workspace with various language bindings.

## Build and Test Commands

### Core Rust Commands

```bash
make build                    # Build release CLI binary (macOS aarch64)
make test                     # Run all Rust tests
make test:stdin               # Test stdin pipe support
make test:lint                # Lint all fixtures in tests/fixtures/
make bench                    # Run benchmarks (requires criterion)

# Run a single test
cargo test -p autocorrect --lib rule::tests::test_format_space_dash
cargo test -p autocorrect-cli --test integration_test

# Format and check
cargo fmt                     # Format all Rust code
cargo clippy                  # Run linter
```

### Language Bindings

```bash
make test:node                # Build and test Node.js binding (requires yarn)
make test:python              # Build and test Python binding (requires pytest)
make test:ruby                # Build and test Ruby binding (requires bundle)
make test:java                # Build and test Java binding (requires maven)
make wasm                     # Build WASM package (requires wasm-pack, wasm-opt)
```

### Desktop Application (autocorrect-app)

```bash
cd autocorrect-app && pnpm install
pnpm tauri dev                # Run with hot reload
pnpm tauri build              # Build production binary
pnpm lint                     # Prettier + ESLint
pnpm check                    # Svelte-check + TypeScript
```

### Required Quality Gates Before PR

1. `cargo test` — all Rust tests pass
2. `cargo fmt --check` — formatting correct
3. `cargo clippy` — no warnings
4. For desktop app: `pnpm lint && pnpm check`

## Project Structure

```
autocorrect/           # Core library: rules, parsers, config
autocorrect-cli/       # CLI binary (tokio-based)
autocorrect-lsp/       # Language Server Protocol (tower-lsp)
autocorrect-node/      # Node.js binding (napi-rs)
autocorrect-py/        # Python binding (pyo3)
autocorrect-rb/        # Ruby binding (rutie/rb-sys)
autocorrect-java/      # Java binding (jni)
autocorrect-wasm/      # WebAssembly binding
autocorrect-app/       # Desktop app (Tauri 2 + Svelte 5)
tests/fixtures/        # Regression test data (*.raw.ext, *.fixed.ext)
```

## Code Style Guidelines

### Rust Naming Conventions

- Functions/variables: `snake_case`
- Types/structs/enums/traits: `PascalCase`
- Constants/statics: `SCREAMING_SNAKE_CASE`
- Modules: `snake_case`

### Import Order

```rust
// 1. std imports
use std::borrow::Cow;
use std::collections::HashMap;

// 2. external crates
use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};

// 3. crate-internal imports
use crate::rule::Rule;
use crate::result::Severity;
```

### Error Handling

- Use `anyhow::Result` for library code and application logic
- Avoid `.unwrap()` in library code; use `?` or explicit error handling
- `.unwrap()` is acceptable in tests and within the `regexp!` macro

```rust
// Good: propagate errors
pub fn load_config(path: &str) -> Result<Config> {
    let data = std::fs::read_to_string(path)?;
    Ok(parse_config(&data)?)
}
```

### Key Macros

- `regexp!`: CJK regex shortcuts (`\p{CJK}`, `\p{CJ}`)
- `map!`: HashMap creation shorthand
- `cases!`: Multi-line test case helper
- `assert_json_eq!`: JSON comparison ignoring whitespace

```rust
// regexp! expands CJK character classes
static ref CJK_RE: Regex = regexp!(r"\p{CJK}");

// map! creates HashMap
let config = map! { "key" => "value" };
```

### Rule System

Rules are defined in `autocorrect/src/rule/` using `Strategery`:

```rust
// Add space between CJK and alphanumeric
Strategery::new(r"\p{CJK}", r"[a-zA-Z0-9]").with_reverse()

// Remove space near fullwidth punctuation
Strategery::new(r"\w|\p{CJK}", r"[，。！？]").with_remove_space()
```

Register rules in `autocorrect/src/rule/mod.rs`:

- `RULES`: First phase (space-word, fullwidth, etc.)
- `AFTER_RULES`: Second phase (halfwidth, spellcheck, etc.)

## Testing

### Unit Tests

Add `#[cfg(test)] mod tests { ... }` blocks in each module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_space_word() {
        assert_eq!(format_space_word("Hello世界"), "Hello 世界");
    }
}
```

### Fixture Tests

Add test cases to `tests/fixtures/`:

- `name.raw.ext` — input file
- `name.fixed.ext` — expected output
- `name.expect.json` — lint result (optional)

## Development Workflow

### Adding a New Rule

1. Add rule logic in `autocorrect/src/rule/word.rs` or new module
2. Register in `RULES` or `AFTER_RULES` in `autocorrect/src/rule/mod.rs`
3. Add unit tests in the same file
4. Add fixture tests in `tests/fixtures/`

### Updating Bindings

When changing core API:

1. Update `autocorrect/src/lib.rs` exports
2. Update all binding crates (`autocorrect-*/src/lib.rs`)
3. Run `make test:node test:python test:ruby test:java`

### Adding File Type Support

1. Create parser in `autocorrect/src/code/<lang>.rs`
2. Implement `lint_<lang>()` and `format_<lang>()` functions
3. Register in `autocorrect/src/code/mod.rs` and `types.rs`
4. Add test fixtures

## Configuration

- Default config: `autocorrect/.autocorrectrc.default`
- Template: `.autocorrectrc.template`
- Rule severity: `0` = off, `1` = error, `2` = warning
- Inline control: `autocorrect-disable`, `autocorrect-enable`

## Commit Guidelines

- Use Conventional Commits: `feat:`, `fix:`, `chore:`, `refactor:`
- One logical change per commit
- Imperative mood in subject line
