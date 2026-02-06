# Typos Detection Limitations

## What the typos library detects

The `typos` library is designed to detect **common English spelling mistakes**, not all possible misspellings. It focuses on:

✅ **Well-known spelling errors**:
- `recieve` → `receive`
- `definately` → `definitely`
- `seperate` → `separate`
- `occured` → `occurred`
- `acommodate` → `accommodate`
- `neccessary` → `necessary`

## What it might NOT detect

❌ **Uncommon abbreviations or very short misspellings**:
- `whts` (too uncommon, might be interpreted as abbreviation)
- `wat` (too short, could be slang)
- `u` instead of `you` (intentional abbreviation)
- `r` instead of `are` (intentional abbreviation)

❌ **Context-dependent errors**:
- `there` vs `their` vs `they're` (all valid words)
- `to` vs `too` vs `two` (all valid words)
- `your` vs `you're` (all valid words)

❌ **Technical jargon or domain-specific terms**:
- Programming terms like `async`, `const`, `func`
- Product names like `iPhone`, `GitHub`, `PostgreSQL`
- Company-specific terminology

## How to extend typos detection

### Option 1: Add custom corrections to `.typos.toml`

Create `~/.typos.toml` with custom correction rules:

```toml
[type.typos]
# Add custom typo corrections
whts = "what's"
waht = "what"
teh = "the"
fo = "for"

[default.extend-words]
# Words that should NOT be flagged as typos
asyncio = "asyncio"
kubernetes = "kubernetes"
postgresql = "postgresql"
```

### Option 2: Use AutoCorrect's custom dictionary

In the AutoCorrect app Settings:
1. Go to "Spell Check Configuration"
2. Add words to "Custom Dictionary" (one per line)
3. These words won't be flagged by AutoCorrect's built-in checker

### Option 3: Disable typo checking

If typo detection isn't meeting your needs:
1. Open AutoCorrect Settings
2. Toggle OFF "Enable Advanced Typo Detection"
3. Save changes

This will keep AutoCorrect's CJK formatting but disable the typos library.

## Why "whts" isn't detected

The `typos` library uses a curated dictionary of common misspellings. "whts" is:
1. Too uncommon to be in the default dictionary
2. Only 4 letters (very short words are often abbreviations)
3. Doesn't match common transposition or letter-swap patterns

If you frequently encounter this specific typo, you can add it to your `.typos.toml`:

```toml
[type.typos]
whts = "what's"
```

## Testing typo detection

You can test which typos are detected:

```bash
# In AutoCorrect app directory
cd autocorrect-app/src-tauri
cargo test --lib typocheck::tests::test_whts -- --nocapture
```

This will show which test cases pass/fail.

## Alternative spell checkers

If you need more comprehensive spell checking, consider:

1. **Language Tool** - Grammar and advanced spell checking
2. **Hunspell** - Used by LibreOffice, Firefox
3. **Aspell** - Traditional spell checker
4. **OS built-in** - macOS Spell Check, Windows Spell Check

AutoCorrect focuses on CJK text formatting with basic English spell checking as a bonus feature.
