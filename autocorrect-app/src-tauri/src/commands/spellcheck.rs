use super::config::{load_app_settings, AppSettings};
use super::errors::Error;
use crate::commands::ai_grammar::check_grammar_issues_with_ai;
use crate::typocheck;
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use serde::{Deserialize, Serialize};
use std::env;

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

fn run_local_spellcheck(
    text: &str,
    app_settings: &AppSettings,
) -> (String, Vec<LineChange>, Vec<TypoSuggestion>) {
    let file_ext = "text"; // Default to text/plain mode

    // Get the corrected text using autocorrect
    let format_result = autocorrect::format_for(text, file_ext);
    let local_corrected = format_result.out;

    // Get diff/lint for line-by-line changes
    let lint_result = autocorrect::lint_for(text, file_ext);
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

    // Check for typos using typos library (based on settings)
    let mut typos = Vec::new();

    // Check with typos library if enabled
    if app_settings.typo_checking_enabled {
        let typo_errors = typocheck::check_typos(text);
        typos.extend(typo_errors.into_iter().map(|typo| TypoSuggestion {
            typo: typo.typo,
            suggestions: typo.suggestions,
            line: typo.line,
            col: typo.col,
        }));
    }

    (local_corrected, line_changes, typos)
}

#[tauri::command]
pub async fn spell_check(
    app: tauri::AppHandle,
    text: String,
    enable_ai: Option<bool>,
) -> Result<SpellCheckResult, Error> {
    let original = text.clone();
    let app_settings = load_app_settings(&app)?;

    let text_for_local = text.clone();
    let settings_for_local = app_settings.clone();
    let (local_corrected, mut line_changes, mut typos) =
        tauri::async_runtime::spawn_blocking(move || {
            run_local_spellcheck(&text_for_local, &settings_for_local)
        })
        .await
        .map_err(|e| Error::Api(format!("Spell check task join error: {}", e)))?;

    let mut corrected = local_corrected;
    let mut has_changes = original != corrected;

    // Optional AI grammar check: returns structured issues instead of rewriting.
    if enable_ai.unwrap_or(true) && app_settings.ai_grammar_enabled {
        let api_key = app_settings.openai_api_key.trim();
        let model = if app_settings.openai_model.trim().is_empty() {
            "gpt-4.1-mini"
        } else {
            app_settings.openai_model.trim()
        };
        if !api_key.is_empty() && original.chars().count() <= app_settings.ai_max_input_chars {
            match check_grammar_issues_with_ai(
                app_settings.ai_api_base_url.trim(),
                api_key,
                model,
                &original,
                app_settings.ai_timeout_ms,
            )
            .await
            {
                Ok(ai_typos) => {
                    for t in ai_typos {
                        let key = (t.typo.clone(), t.line, t.col);
                        if !typos.iter().any(|e| (e.typo.clone(), e.line, e.col) == key) {
                            typos.push(TypoSuggestion {
                                typo: t.typo,
                                suggestions: t.suggestions,
                                line: t.line,
                                col: t.col,
                            });
                        }
                    }
                }
                Err(e) => {
                    log::warn!("AI grammar check skipped due to API error: {}", e);
                }
            }
        }
    }

    if has_changes && line_changes.is_empty() {
        line_changes.push(LineChange {
            line: 1,
            col: 1,
            original: original.clone(),
            corrected: corrected.clone(),
            severity: 2,
        });
    }

    Ok(SpellCheckResult {
        original,
        corrected,
        has_changes,
        line_changes,
        typos,
    })
}

pub fn spell_check_sync(
    app: tauri::AppHandle,
    text: String,
    enable_ai: Option<bool>,
) -> Result<SpellCheckResult, Error> {
    tauri::async_runtime::block_on(spell_check(app, text, enable_ai))
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
