# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

AutoCorrect is a CJK (Chinese, Japanese, Korean) text formatter and linter. The core is written in Rust with bindings for Node.js, Ruby, Python, Java, and WebAssembly. It also includes an LSP server for IDE integration.

## Build and Test Commands

### Core Rust (CLI)

```bash
# Build release binary (macOS aarch64)
make build

# Run tests
make test
make test:lint        # Lint test fixtures
make test:lint-json   # JSON output test
make test:stdin       # Test stdin processing

# Run benchmarks
make bench
cargo criterion

# Run CLI for manual testing
make run              # Run with template config
make run:json         # Run with JSON output

# Start LSP server
make server
```

### Language Bindings

```bash
# Node.js
make test:node        # Build and test Node.js binding
make test:node:cli    # Test CLI integration

# Python
make test:python      # Build and test Python binding

# Ruby
make test:ruby        # Build and test Ruby binding

# Java
make test:java        # Build and test Java binding
```

### WebAssembly

```bash
# Install wasm dependencies
make install

# Build WASM package
make wasm

# Publish to NPM
make wasm:publish
```

### Release

```bash
# Bump version across all packages
FROM=2.16.3 TO=2.17.0 make version

# Publish Rust crate
make crate:publish
```

## Architecture

### Core Library (`autocorrect/`)

The core Rust library provides the formatting engine:

- **`lib.rs`** - Public API entry point, exports `format()`, `format_for()`, `lint_for()`
- **`format.rs`** - Main formatting logic
- **`rule/`** - Formatting rule implementations:
  - `mod.rs` - Rule orchestration (`RULES` run first, then `AFTER_RULES`)
  - `word.rs` - Space rules between CJK and English
  - `fullwidth.rs` - Fullwidth character conversion
  - `halfwidth.rs` - Halfwidth conversion (spellcheck integration)
  - `spellcheck.rs` - Dictionary-based word correction
- **`code/`** - File-type specific parsers (28+ languages):
  - Each file implements `lint_X()` and `format_X()` functions
  - Uses AST parsing to extract strings/comments from code
  - `mod.rs` routes files to appropriate parser via `lint_for()` / `format_for()`
- **`config/`** - Configuration management:
  - Supports YAML and JSON formats
  - Merges user config with default (`.autocorrectrc.default`)
  - Rule severity: 0 = off, 1 = error, 2 = warning

### CLI (`autocorrect-cli/`)

- Entry point for the `autocorrect` binary
- Commands: `--lint`, `--fix`, `--stdin`, `--format json|rdjson`
- Config loading from `.autocorrectrc`

### LSP Server (`autocorrect-lsp/`)

- Implements Language Server Protocol
- Uses `tower-lsp` framework
- Provides real-time diagnostics in editors
- Integrates `typos` crate for spellchecking (LSP-only feature)

### Language Bindings

All bindings expose the same core API:
- `format(text)` - Format text
- `format_for(text, filepath)` - Format with file type detection
- `lint_for(text, filepath)` - Lint and return diff

The bindings use Rust's FFI:
- **Node.js** - `napi-rs` for native Node addon
- **Ruby** - `rutie` for Ruby C extension
- **Python** - `pyo3` for Python native module
- **Java** - `jni` for JNI binding

### Rule System

Rules are applied in two phases:

**Phase 1 (RULES):**
1. `space-word` - Add space between CJK and English words
2. `space-punctuation` - Add space near punctuation
3. `space-bracket` - Add space around brackets `[]` `()`
4. `space-dash` - Add space around dashes `-`
5. `space-backticks` - Add space around backticks
6. `space-dollar` - Add space around dollar signs `$`
7. `fullwidth` - Convert to fullwidth punctuation

**Phase 2 (AFTER_RULES):**
1. `halfwidth-word` - Convert fullwidth alphanumeric to halfwidth
2. `halfwidth-punctuation` - Convert fullwidth punctuation to halfwidth in English
3. `no-space-fullwidth` - Remove spaces near fullwidth
4. `no-space-fullwidth-quote` - Remove spaces near fullwidth quotes
5. `spellcheck` - Apply dictionary corrections

### File Type Detection

File types are matched in `autocorrect/src/code/types.rs`. Custom file type associations can be configured via `fileTypes` in `.autocorrectrc`.

### Inline Control Comments

Files can temporarily disable AutoCorrect using comments:
- `autocorrect-disable` - Disable all rules
- `autocorrect-disable <rule>` - Disable specific rule
- `autocorrect-enable` - Re-enable rules

### Ignoring Files

Files matching `.gitignore` patterns are ignored. Additional patterns can be added to `.autocorrectignore`.

## Key Files

- `Cargo.toml` - Workspace configuration
- `Makefile` - Build and test commands
- `.autocorrectrc.template` - Example configuration
- `autocorrect/.autocorrectrc.default` - Default configuration (built-in)
- `autocorrect/src/code/mod.rs` - File type routing logic
- `autocorrect/src/rule/mod.rs` - Rule pipeline

## Testing

Test fixtures are in `tests/fixtures/` with naming pattern:
- `*.fixed.{ext}` - Expected output after formatting
- `*.raw.{ext}` - Input before formatting

When adding new test cases, add both `.raw` and `.fixed` files.
