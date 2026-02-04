use super::errors::Error;
use enigo::{Keyboard, Direction, Enigo, Key, Settings};
use serde::Serialize;
use std::env;

#[derive(Clone, Serialize)]
pub struct SpellCheckResult {
    pub original: String,
    pub corrected: String,
    pub has_changes: bool,
    pub line_changes: Vec<LineChange>,
}

#[derive(Clone, Serialize)]
pub struct LineChange {
    pub line: u32,
    pub col: u32,
    pub original: String,
    pub corrected: String,
    pub severity: u8,
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

    Ok(SpellCheckResult {
        original,
        corrected,
        has_changes,
        line_changes,
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
    let mut enigo = Enigo::new(&Settings::default())
        .map_err(|e| Error::InputSimulation(format!("Failed to initialize input simulation: {}", e)))?;

    // Use Command on macOS, Control on other platforms
    #[cfg(target_os = "macos")]
    {
        enigo.key(Key::Meta, Direction::Press)
            .map_err(|e| Error::InputSimulation(format!("Failed to simulate key press: {}", e)))?;
    }

    #[cfg(not(target_os = "macos"))]
    {
        enigo.key(Key::Control, Direction::Press)
            .map_err(|e| Error::InputSimulation(format!("Failed to simulate key press: {}", e)))?;
    }

    enigo.key(Key::Unicode('v'), Direction::Click)
        .map_err(|e| Error::InputSimulation(format!("Failed to simulate key press: {}", e)))?;

    // Release the modifier key
    #[cfg(target_os = "macos")]
    {
        enigo.key(Key::Meta, Direction::Release)
            .map_err(|e| Error::InputSimulation(format!("Failed to simulate key press: {}", e)))?;
    }

    #[cfg(not(target_os = "macos"))]
    {
        enigo.key(Key::Control, Direction::Release)
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
