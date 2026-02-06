use super::errors::Error;
use crate::{cspell, typocheck};
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;

/// App-specific settings
#[derive(Clone, Serialize, Deserialize, Default)]
struct AppSettings {
    #[serde(default = "default_typo_checking")]
    typo_checking_enabled: bool,
    #[serde(default)]
    cspell_enabled: bool,
    #[serde(default)]
    cspell_dictionaries: cspell::CSpellDictionaries,
}

fn default_typo_checking() -> bool {
    true
}

/// Get the path to the app settings file
fn get_app_settings_path() -> PathBuf {
    let home_dir = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());

    PathBuf::from(home_dir).join(".autocorrect-app.json")
}

/// Load app-specific settings
fn load_app_settings() -> AppSettings {
    let settings_path = get_app_settings_path();
    if settings_path.exists() {
        if let Ok(content) = fs::read_to_string(&settings_path) {
            if let Ok(settings) = serde_json::from_str(&content) {
                return settings;
            }
        }
    }
    AppSettings::default()
}

#[derive(Clone, Serialize)]
pub struct SpellCheckResult {
    pub original: String,
    pub corrected: String,
    pub has_changes: bool,
    pub line_changes: Vec<LineChange>,
    pub typos: Vec<TypoSuggestion>,
}

#[derive(Clone, Serialize)]
pub struct LineChange {
    pub line: u32,
    pub col: u32,
    pub original: String,
    pub corrected: String,
    pub severity: u8,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TypoSuggestion {
    pub typo: String,
    pub suggestions: Vec<String>,
    pub line: usize,
    pub col: usize,
}

#[derive(Clone, Serialize)]
pub struct Config {
    pub content: String,
    pub path: String,
}

#[tauri::command]
pub fn spell_check(text: String) -> Result<SpellCheckResult, Error> {
    let original = text.clone();
    let file_ext = "text"; // Default to text/plain mode

    // Get the corrected text using autocorrect
    let format_result = autocorrect::format_for(&text, file_ext);
    let corrected = format_result.out;
    let has_changes = original != corrected;

    // Get diff/lint for line-by-line changes
    let lint_result = autocorrect::lint_for(&text, file_ext);
    let mut line_changes = Vec::new();

    // Parse the lint result to extract line changes
    for line_result in lint_result.lines.iter() {
        line_changes.push(LineChange {
            line: line_result.line as u32,
            col: line_result.col as u32,
            original: line_result.old.clone(),
            corrected: line_result.new.clone(),
            severity: line_result.severity as u8,
        });
    }

    // If we have changes but no line changes detected, add a summary change
    if has_changes && line_changes.is_empty() {
        line_changes.push(LineChange {
            line: 1,
            col: 1,
            original: original.clone(),
            corrected: corrected.clone(),
            severity: 2, // Warning
        });
    }

    // Check for typos using typos library and CSpell (based on settings)
    let app_settings = load_app_settings();
    let mut typos = Vec::new();

    // 1. Check with typos library if enabled
    if app_settings.typo_checking_enabled {
        let typo_errors = typocheck::check_typos(&text);
        typos.extend(typo_errors.into_iter().map(|typo| TypoSuggestion {
            typo: typo.typo,
            suggestions: typo.suggestions,
            line: typo.line,
            col: typo.col,
        }));
    }

    // 2. Check with CSpell if enabled
    if app_settings.cspell_enabled {
        let cspell_errors = cspell::check_with_cspell(&text, &app_settings.cspell_dictionaries);
        typos.extend(cspell_errors.into_iter().map(|typo| TypoSuggestion {
            typo: typo.typo,
            suggestions: typo.suggestions,
            line: typo.line,
            col: typo.col,
        }));
    }

    // Deduplicate typos (in case both checkers found the same issue)
    typos.sort_by(|a, b| {
        a.line
            .cmp(&b.line)
            .then_with(|| a.col.cmp(&b.col))
            .then_with(|| a.typo.cmp(&b.typo))
    });
    typos.dedup_by(|a, b| a.line == b.line && a.col == b.col && a.typo == b.typo);

    Ok(SpellCheckResult {
        original,
        corrected,
        has_changes,
        line_changes,
        typos,
    })
}

#[tauri::command]
pub fn get_clipboard_text() -> Result<String, Error> {
    let mut clipboard = arboard::Clipboard::new()
        .map_err(|e| Error::Clipboard(format!("Failed to access clipboard: {}", e)))?;

    let text = clipboard
        .get_text()
        .map_err(|e| Error::Clipboard(format!("Failed to get clipboard text: {}", e)))?;

    Ok(text)
}

#[tauri::command]
pub fn set_clipboard_text(text: String) -> Result<(), Error> {
    let mut clipboard = arboard::Clipboard::new()
        .map_err(|e| Error::Clipboard(format!("Failed to access clipboard: {}", e)))?;

    clipboard
        .set_text(&text)
        .map_err(|e| Error::Clipboard(format!("Failed to set clipboard text: {}", e)))?;

    Ok(())
}

#[tauri::command]
pub fn simulate_paste() -> Result<(), Error> {
    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| {
        Error::InputSimulation(format!("Failed to initialize input simulation: {}", e))
    })?;

    // Use Command on macOS, Control on other platforms
    #[cfg(target_os = "macos")]
    {
        enigo
            .key(Key::Meta, Direction::Press)
            .map_err(|e| Error::InputSimulation(format!("Failed to simulate key press: {}", e)))?;
    }

    #[cfg(not(target_os = "macos"))]
    {
        enigo
            .key(Key::Control, Direction::Press)
            .map_err(|e| Error::InputSimulation(format!("Failed to simulate key press: {}", e)))?;
    }

    enigo
        .key(Key::Unicode('v'), Direction::Click)
        .map_err(|e| Error::InputSimulation(format!("Failed to simulate key press: {}", e)))?;

    // Release the modifier key
    #[cfg(target_os = "macos")]
    {
        enigo
            .key(Key::Meta, Direction::Release)
            .map_err(|e| Error::InputSimulation(format!("Failed to simulate key press: {}", e)))?;
    }

    #[cfg(not(target_os = "macos"))]
    {
        enigo
            .key(Key::Control, Direction::Release)
            .map_err(|e| Error::InputSimulation(format!("Failed to simulate key press: {}", e)))?;
    }

    Ok(())
}

#[tauri::command]
pub fn load_config() -> Result<Config, Error> {
    let home_dir = env::var("HOME")
        .map_err(|_| Error::Config("Could not determine HOME directory".to_string()))?;

    let config_path = format!("{}/.autocorrectrc", home_dir);

    let content = if std::path::Path::new(&config_path).exists() {
        std::fs::read_to_string(&config_path)?
    } else {
        // Default config
        DEFAULT_CONFIG.to_string()
    };

    Ok(Config {
        content,
        path: config_path,
    })
}

#[tauri::command]
pub fn save_config(content: String) -> Result<(), Error> {
    let home_dir = env::var("HOME")
        .map_err(|_| Error::Config("Could not determine HOME directory".to_string()))?;

    let config_path = format!("{}/.autocorrectrc", home_dir);

    std::fs::write(&config_path, content)?;

    Ok(())
}

const DEFAULT_CONFIG: &str = r#"# AutoCorrect Configuration File
# See https://github.com/huacnlee/autocorrect for full options

# Enable/disable features
features:
  - linter        # Enable linter rules
  # - spellcheck   # Enable spell checking

# Language-specific rules
rules:
  # English rules
  - halfwidth_quotes: always
  - space_between_cjk_and_english_or_numbers: true

  # Chinese-specific
  # - fullwidth_punctuation: true
"#;
