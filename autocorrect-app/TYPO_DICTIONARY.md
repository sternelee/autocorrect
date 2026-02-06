# Custom Typo Dictionary

AutoCorrect desktop app includes advanced English spell-checking powered by the `typos` library. You can customize the spell-checker to recognize your own words and technical terms, or add custom typo corrections.

## Quick Start: Add Custom Typo Corrections

If you want to detect specific typos (like "whts" → "what's"), create a `.typos.toml` file in your home directory:

```toml
[type.typos]
# Add your custom typo corrections
whts = "what's"
waht = "what"
teh = "the"
recieve = "receive"

[default.extend-words]
# Words that should NOT be flagged as typos
mycompany = "mycompany"
customterm = "customterm"
```

## Configuration File Location

- **macOS/Linux**: `~/.typos.toml`
- **Windows**: `%USERPROFILE%\.typos.toml`

## How to Add Custom Words (Not Typos)

If you have technical terms or proper nouns that are being flagged incorrectly:

```toml
[default.extend-words]
# Programming terms
dockerfile = "dockerfile"
kubernetes = "kubernetes"
postgresql = "postgresql"

# Company/product names
autocorrect = "autocorrect"
tauri = "tauri"

# Project-specific terms
myapp = "myapp"
myproject = "myproject"
```

## How to Add Custom Typo Corrections

If you want the spell checker to detect and correct specific mistakes:

```toml
[type.typos]
# Your custom typo → correction mappings
whts = "what's"
waht = "what"
teh = "the"
fo = "for"
fro = "for"
```

**Note**: By default, the `typos` library only detects common English spelling errors. If you encounter specific typos that aren't being caught, add them to this section.

## Example: Complete Configuration

```toml
# ~/.typos.toml

# Words that should NOT be flagged as errors
[default.extend-words]
dockerfile = "dockerfile"
kubernetes = "kubernetes"  
mycompany = "mycompany"
autocorrect = "autocorrect"

# Custom typo corrections
[type.typos]
whts = "what's"
waht = "what"
teh = "the"

# File checking options
[default]
check-file = true
check-filenames = false
```

## Additional Options

You can also configure other typos settings:

```toml
[default]
# Check file contents (default: true)
check-file = true

# Check file names (default: false)
check-filenames = false

# Extend dictionary with custom words
[default.extend-words]
myword = "myword"
anotherword = "anotherword"

# Override built-in corrections
[default.extend-corrections]
# If you want "teh" to be corrected to something other than "the"
teh = "teh"  # This will prevent correction
```

## Disabling Typo Checking

You can disable typo checking entirely from the AutoCorrect settings panel:

1. Open AutoCorrect app
2. Go to Settings
3. Find "Enable Advanced Typo Detection"
4. Toggle it off
5. Click "Save Changes"

## More Information

For more advanced configuration options, see the [typos documentation](https://github.com/crate-ci/typos).
