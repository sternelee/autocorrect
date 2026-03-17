# Custom Typo Detection - Solving the "whts" Problem

AutoCorrect desktop app uses the `typos` library to detect common English spelling errors. However, the `typos` library has limitations - it only detects **common spelling mistakes**, not all possible misspellings.

## Why "whts" is Not Detected by Default

The `typos` library uses a curated dictionary of common misspellings. By default, it does **NOT** detect:

❌ Uncommon abbreviations (like "whts")
❌ Very short words (like "wat")
❌ Context-dependent errors (like "there" vs "their")  
❌ Technical jargon or proprietary terms

✅ But it **DOES** detect common spelling errors:

- `recieve` → `receive`
- `definately` → `definitely`
- `seperate` → `separate`
- `occured` → `occurred`
- `acommodate` → `accommodate`
- `neccessary` → `necessary`

## Solution: Add Custom Typo Corrections

If you need to detect specific typos (like "whts"), you can create a custom configuration file.

### Step 1: Create Custom Configuration File

Create a file in your home directory: `~/.autocorrect-typos.txt`

```bash
# macOS/Linux
touch ~/.autocorrect-typos.txt

# Windows
type nul > %USERPROFILE%\.autocorrect-typos.txt
```

### Step 2: Add Custom Typo Mappings

Edit `~/.autocorrect-typos.txt` with format `typo=correction`:

```
# Custom typo corrections
whts=what's
teh=the
waht=what
fo=for
fro=for
```

### Step 3: Restart AutoCorrect App

After saving the file, restart the AutoCorrect app. The custom configuration will be automatically loaded.

### Step 4: Test It

Select text containing "whts", press the global hotkey (default `Cmd+Shift+K`), and you should now see:

```
Spelling Issues Found:
- whts → what's
```

## File Format

The `~/.autocorrect-typos.txt` file format:

```
# Lines starting with # are comments
# Format: typo=correction

whts=what's
teh=the
waht=what

# You can add as many mappings as you want
recieve=receive
definately=definitely
```

### Advanced Example

```
# Common typing mistakes
teh=the
fo=for
fro=for
waht=what
whts=what's

# Homophones you frequently mix up
their=they're
your=you're

# Domain-specific terms
recieve=receive
seperate=separate
```

## Testing Your Configuration

Run this test in terminal to verify your configuration works:

```bash
cd autocorrect-app/src-tauri
cargo test --lib typocheck::tests::test_whts_with_custom -- --nocapture
```

You should see:

```
Text: 'whts is your name'
Found 1 typos
  - 'whts' -> ["what's"]
✅ Custom correction for 'whts' is working!
```

## Troubleshooting

### Q: My custom configuration is not working

A: Make sure:

1. File path is correct: `~/.autocorrect-typos.txt` (with dot prefix)
2. File format is correct: `typo=correction` (one per line)
3. You restarted the AutoCorrect app
4. Typo checking is enabled (Settings → Enable Advanced Typo Detection)

### Q: Can I disable typo checking completely?

A: Yes. In AutoCorrect settings:

1. Open Settings panel
2. Find "Enable Advanced Typo Detection"
3. Toggle it OFF
4. Click "Save Changes"

This keeps AutoCorrect's CJK formatting but disables spell checking.

### Q: How do I ignore words that are incorrectly flagged?

A: Use AutoCorrect's Custom Dictionary feature:

1. Open Settings → Spell Check Configuration
2. Add words to "Custom Dictionary" text area (one per line)
3. Save changes

These words won't be flagged by AutoCorrect's built-in checker.

## Technical Details

- **Default detector**: `typos` library (detects common spelling errors)
- **Custom detector**: Reads `~/.autocorrect-typos.txt` file
- **Priority**: Custom corrections take precedence over default detection
- **Performance**: Configuration file loaded once at app startup (LazyLock)
- **Word boundaries**: Includes apostrophes (supports "what's", "don't", etc.)

## Example Use Cases

### 1. Personal Typing Habits

If you frequently type "whts" instead of "what's":

```
whts=what's
wat=what
bcz=because
```

### 2. Regional Spelling Differences

If you want to enforce American spelling:

```
colour=color
favour=favor
realise=realize
```

### 3. Technical Terms

Add corrections for technical jargon you frequently misspell:

```
kubernetes=kubernetes  # Just ignore it
asyncio=asyncio        # Python library
postgresql=PostgreSQL  # Capitalize correctly
```

## Alternative Spell Checkers

If you need more comprehensive spell checking:

1. **LanguageTool** - Grammar and advanced spell checking
2. **Hunspell** - Used by LibreOffice, Firefox
3. **Aspell** - Traditional spell checker
4. **OS built-in** - macOS Spell Check, Windows Spell Check

AutoCorrect focuses on CJK text formatting. English spell checking is a bonus feature, not the primary focus.

## File Location

| OS      | Location                               |
| ------- | -------------------------------------- |
| macOS   | `~/.autocorrect-typos.txt`             |
| Linux   | `~/.autocorrect-typos.txt`             |
| Windows | `%USERPROFILE%\.autocorrect-typos.txt` |

## Complete Example File

Create `~/.autocorrect-typos.txt`:

```
# ============================================
# AutoCorrect Custom Typo Corrections
# ============================================
#
# Format: typo=correction
# Lines starting with # are ignored
#

# Common typing mistakes
whts=what's
teh=the
waht=what
fo=for
fro=for

# Homophones
their=they're
your=you're
its=it's

# Common misspellings
recieve=receive
definately=definitely
seperate=separate
occured=occurred
acommodate=accommodate

# Technical terms (add as-is to prevent false positives)
# These won't be flagged
```

Then restart AutoCorrect app, and all these typos will be detected!
