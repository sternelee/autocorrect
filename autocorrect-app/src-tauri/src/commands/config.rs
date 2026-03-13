use super::errors::Error;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use tauri::Emitter;

pub const DEFAULT_UNDERLINE_STYLE: &str = "wavy";
pub const DEFAULT_UNDERLINE_COLOR: &str = "#ff3b30";

/// Custom app-specific settings stored separately from autocorrect config
#[derive(Clone, Serialize, Deserialize)]
struct AppSettings {
    #[serde(default = "default_typo_checking")]
    typo_checking_enabled: bool,
    #[serde(default)]
    ai_grammar_enabled: bool,
    #[serde(default)]
    openai_api_key: String,
    #[serde(default = "default_openai_model")]
    openai_model: String,
    #[serde(default = "default_ai_max_input_chars")]
    ai_max_input_chars: usize,
    #[serde(default = "default_ai_timeout_ms")]
    ai_timeout_ms: u64,
    #[serde(default = "default_ai_api_base_url")]
    ai_api_base_url: String,
    #[serde(default = "default_ai_translate_target_language")]
    ai_translate_target_language: String,
    #[serde(default = "default_ai_polish_style")]
    ai_polish_style: String,
    #[serde(default = "default_underline_style")]
    underline_style: String,
    #[serde(default = "default_underline_color")]
    underline_color: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            typo_checking_enabled: default_typo_checking(),
            ai_grammar_enabled: false,
            openai_api_key: String::new(),
            openai_model: default_openai_model(),
            ai_max_input_chars: default_ai_max_input_chars(),
            ai_timeout_ms: default_ai_timeout_ms(),
            ai_api_base_url: default_ai_api_base_url(),
            ai_translate_target_language: default_ai_translate_target_language(),
            ai_polish_style: default_ai_polish_style(),
            underline_style: default_underline_style(),
            underline_color: default_underline_color(),
        }
    }
}

fn default_typo_checking() -> bool {
    true
}

fn default_openai_model() -> String {
    "gpt-4.1-mini".to_string()
}

fn default_ai_max_input_chars() -> usize {
    2000
}

fn default_ai_timeout_ms() -> u64 {
    12000
}

fn default_ai_api_base_url() -> String {
    "https://openrouter.ai/api/v1/chat/completions".to_string()
}

fn default_ai_translate_target_language() -> String {
    "English".to_string()
}

fn default_ai_polish_style() -> String {
    "professional".to_string()
}

fn default_underline_style() -> String {
    DEFAULT_UNDERLINE_STYLE.to_string()
}

fn default_underline_color() -> String {
    DEFAULT_UNDERLINE_COLOR.to_string()
}

/// Convert u8 to SeverityMode
fn u8_to_severity_mode(value: u8) -> autocorrect::config::SeverityMode {
    match value {
        0 => autocorrect::config::SeverityMode::Off,
        1 => autocorrect::config::SeverityMode::Error,
        2 => autocorrect::config::SeverityMode::Warning,
        _ => autocorrect::config::SeverityMode::Error,
    }
}

/// Current merged configuration from default + user config
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    /// All rules with their current severity
    pub rules: HashMap<String, u8>,
    /// Text-specific rules
    pub text_rules: HashMap<String, u8>,
    /// Spellcheck words
    pub spellcheck_words: Vec<String>,
    /// File type mappings
    pub file_types: HashMap<String, String>,
    /// Context-specific settings
    pub context: HashMap<String, u8>,
    /// Path to user config file
    pub config_path: String,
    /// Enable/disable typo checking
    pub typo_checking_enabled: bool,
    /// Enable/disable AI grammar check
    pub ai_grammar_enabled: bool,
    /// OpenAI API key
    pub openai_api_key: String,
    /// OpenAI model name
    pub openai_model: String,
    /// Max input chars for AI check
    pub ai_max_input_chars: usize,
    /// Timeout in milliseconds for AI check
    pub ai_timeout_ms: u64,
    /// AI provider base URL
    pub ai_api_base_url: String,
    /// Default translation target language
    pub ai_translate_target_language: String,
    /// Default polish style
    pub ai_polish_style: String,
    /// Underline style: "wavy" | "solid" | "dashed" | "dotted"
    pub underline_style: String,
    /// Underline color hex (e.g. "#ff3b30")
    pub underline_color: String,
}

/// Information about a single rule
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleInfo {
    /// Rule name (e.g., "space-word", "fullwidth")
    pub name: String,
    /// Current severity: 0=off, 1=error, 2=warning
    pub severity: u8,
    /// Human-readable description
    pub description: String,
    /// Default severity value
    pub default_severity: u8,
}

/// Updates to apply to the configuration
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigUpdates {
    /// Rule severities to update (only includes changed rules)
    pub rules: Option<HashMap<String, Option<u8>>>,
    /// Text rule severities to update
    pub text_rules: Option<HashMap<String, Option<u8>>>,
    /// Spellcheck words to set (replaces entire list)
    pub spellcheck_words: Option<Vec<String>>,
    /// Enable/disable typo checking
    pub typo_checking_enabled: Option<bool>,
    /// Enable/disable AI grammar check
    pub ai_grammar_enabled: Option<bool>,
    /// OpenAI API key
    pub openai_api_key: Option<String>,
    /// OpenAI model name
    pub openai_model: Option<String>,
    /// Max input chars for AI check
    pub ai_max_input_chars: Option<usize>,
    /// Timeout in milliseconds for AI check
    pub ai_timeout_ms: Option<u64>,
    /// AI provider base URL
    pub ai_api_base_url: Option<String>,
    /// Default translation target language
    pub ai_translate_target_language: Option<String>,
    /// Default polish style
    pub ai_polish_style: Option<String>,
    /// Underline style
    pub underline_style: Option<String>,
    /// Underline color hex
    pub underline_color: Option<String>,
}

/// Get the current merged configuration (default + user config)
#[tauri::command]
pub fn get_config() -> Result<AppConfig, Error> {
    let config_path = get_user_config_path();
    let user_config_content = if config_path.exists() {
        fs::read_to_string(&config_path).unwrap_or_default()
    } else {
        String::new()
    };

    // Get current config from autocorrect crate (default + merged user config)
    let current_config = autocorrect::config::Config::current();

    // Load app-specific settings
    let app_settings = load_app_settings();

    // Extract spellcheck words from user config if present
    let spellcheck_words = if user_config_content.is_empty() {
        Vec::new()
    } else {
        // Parse user config to extract spellcheck words
        if let Ok(user_config) = autocorrect::config::Config::from_str(&user_config_content) {
            user_config.spellcheck.words
        } else {
            Vec::new()
        }
    };

    // Convert rules to serializable format
    let mut rules = HashMap::new();
    for (name, mode) in current_config.rules.iter() {
        rules.insert(name.clone(), *mode as u8);
    }

    // Convert text_rules
    let mut text_rules = HashMap::new();
    for (name, mode) in current_config.text_rules.iter() {
        text_rules.insert(name.clone(), *mode as u8);
    }

    // Convert context
    let mut context = HashMap::new();
    for (name, mode) in current_config.context.iter() {
        context.insert(name.clone(), *mode as u8);
    }

    Ok(AppConfig {
        rules,
        text_rules,
        spellcheck_words,
        file_types: current_config.file_types.clone(),
        context,
        config_path: config_path.to_string_lossy().to_string(),
        typo_checking_enabled: app_settings.typo_checking_enabled,
        ai_grammar_enabled: app_settings.ai_grammar_enabled,
        openai_api_key: app_settings.openai_api_key,
        openai_model: app_settings.openai_model,
        ai_max_input_chars: app_settings.ai_max_input_chars,
        ai_timeout_ms: app_settings.ai_timeout_ms,
        ai_api_base_url: app_settings.ai_api_base_url,
        ai_translate_target_language: app_settings.ai_translate_target_language,
        ai_polish_style: app_settings.ai_polish_style,
        underline_style: app_settings.underline_style,
        underline_color: app_settings.underline_color,
    })
}

/// Get the default configuration as YAML string
#[tauri::command]
pub fn get_default_config() -> Result<String, Error> {
    // Read from the default config embedded in autocorrect crate
    let default_config_str = include_str!("../../../../autocorrect/.autocorrectrc.default");
    Ok(default_config_str.to_string())
}

/// Get all available rules with their current severity and descriptions
#[tauri::command]
pub fn get_rules() -> Result<Vec<RuleInfo>, Error> {
    let current_config = autocorrect::config::Config::current();
    let default_config_str = include_str!("../../../../autocorrect/.autocorrectrc.default");
    let default_config = autocorrect::config::Config::from_str(default_config_str)
        .unwrap_or_else(|_| autocorrect::config::Config::default());

    let mut rules = Vec::new();

    // Get all rule names from the current config (includes all default rules)
    for name in current_config.rules.keys() {
        let severity = current_config
            .rules
            .get(name)
            .map(|m| *m as u8)
            .unwrap_or(1); // Default to error

        let description = get_rule_description(name);

        // Get default severity from the default config
        let default_severity = default_config
            .rules
            .get(name)
            .map(|m| *m as u8)
            .unwrap_or(1);

        rules.push(RuleInfo {
            name: name.clone(),
            severity,
            description,
            default_severity,
        });
    }

    // Sort rules by name for consistent display
    rules.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(rules)
}

/// Update configuration with specific changes
#[tauri::command]
pub fn update_config(app: tauri::AppHandle, updates: ConfigUpdates) -> Result<(), Error> {
    let config_path = get_user_config_path();

    // Read existing user config or start with minimal structure
    let user_config_content = if config_path.exists() {
        fs::read_to_string(&config_path)?
    } else {
        String::from("# AutoCorrect Configuration\n")
    };

    // Parse existing user config
    let mut user_config = if user_config_content.trim().is_empty() {
        autocorrect::config::Config::default()
    } else {
        autocorrect::config::Config::from_str(&user_config_content)
            .unwrap_or_else(|_| autocorrect::config::Config::default())
    };

    // Apply rule updates
    if let Some(rule_updates) = updates.rules {
        for (rule_name, severity) in rule_updates {
            if let Some(sev) = severity {
                user_config
                    .rules
                    .insert(rule_name, u8_to_severity_mode(sev));
            } else {
                // None means remove the override (use default)
                user_config.rules.remove(&rule_name);
            }
        }
    }

    // Apply text rule updates
    if let Some(text_rule_updates) = updates.text_rules {
        for (rule_name, severity) in text_rule_updates {
            if let Some(sev) = severity {
                user_config
                    .text_rules
                    .insert(rule_name, u8_to_severity_mode(sev));
            } else {
                user_config.text_rules.remove(&rule_name);
            }
        }
    }

    // Apply spellcheck words update
    if let Some(words) = updates.spellcheck_words {
        user_config.spellcheck.words = words;
    }

    // Handle app-specific settings (typo checking)
    if let Some(typo_enabled) = updates.typo_checking_enabled {
        let mut app_settings = load_app_settings();
        app_settings.typo_checking_enabled = typo_enabled;
        save_app_settings(&app_settings)?;
    }

    if let Some(ai_enabled) = updates.ai_grammar_enabled {
        let mut app_settings = load_app_settings();
        app_settings.ai_grammar_enabled = ai_enabled;
        save_app_settings(&app_settings)?;
    }

    if let Some(openai_api_key) = updates.openai_api_key {
        let mut app_settings = load_app_settings();
        app_settings.openai_api_key = openai_api_key;
        save_app_settings(&app_settings)?;
    }

    if let Some(openai_model) = updates.openai_model {
        let mut app_settings = load_app_settings();
        app_settings.openai_model = openai_model;
        save_app_settings(&app_settings)?;
    }

    if let Some(ai_max_input_chars) = updates.ai_max_input_chars {
        let mut app_settings = load_app_settings();
        app_settings.ai_max_input_chars = ai_max_input_chars;
        save_app_settings(&app_settings)?;
    }

    if let Some(ai_timeout_ms) = updates.ai_timeout_ms {
        let mut app_settings = load_app_settings();
        app_settings.ai_timeout_ms = ai_timeout_ms;
        save_app_settings(&app_settings)?;
    }

    if let Some(ai_api_base_url) = updates.ai_api_base_url {
        let mut app_settings = load_app_settings();
        app_settings.ai_api_base_url = ai_api_base_url;
        save_app_settings(&app_settings)?;
    }

    if let Some(ai_translate_target_language) = updates.ai_translate_target_language {
        let mut app_settings = load_app_settings();
        app_settings.ai_translate_target_language = ai_translate_target_language;
        save_app_settings(&app_settings)?;
    }

    if let Some(ai_polish_style) = updates.ai_polish_style {
        let mut app_settings = load_app_settings();
        app_settings.ai_polish_style = ai_polish_style;
        save_app_settings(&app_settings)?;
    }

    let mut underline_changed = false;
    if let Some(style) = updates.underline_style {
        let mut app_settings = load_app_settings();
        app_settings.underline_style = style;
        save_app_settings(&app_settings)?;
        underline_changed = true;
    }
    if let Some(color) = updates.underline_color {
        let mut app_settings = load_app_settings();
        app_settings.underline_color = color;
        save_app_settings(&app_settings)?;
        underline_changed = true;
    }
    if underline_changed {
        let settings = load_app_settings();
        let _ = app.emit(
            "underline-config-update",
            serde_json::json!({
                "underlineStyle": settings.underline_style,
                "underlineColor": settings.underline_color,
            }),
        );
    }

    // Serialize back to YAML
    // For a cleaner YAML output, we'll manually construct it
    let yaml = serialize_config_to_yaml(&user_config)?;

    // Write to user config file
    fs::write(&config_path, yaml)?;

    // Reload config in the autocorrect crate
    let config_str = fs::read_to_string(&config_path)?;
    autocorrect::config::load(&config_str)
        .map_err(|e| Error::Config(format!("Failed to reload config: {}", e)))?;

    Ok(())
}

/// Get the path to the user's .autocorrectrc file
fn get_user_config_path() -> PathBuf {
    let home_dir = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());

    PathBuf::from(home_dir).join(".autocorrectrc")
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

/// Return (underline_style, underline_color) from persisted app settings.
/// Used by the overlay renderer without going through the full Tauri command layer.
pub fn get_underline_config() -> (String, String) {
    let s = load_app_settings();
    (s.underline_style, s.underline_color)
}

/// Save app-specific settings
fn save_app_settings(settings: &AppSettings) -> Result<(), Error> {
    let settings_path = get_app_settings_path();
    let json = serde_json::to_string_pretty(settings)
        .map_err(|e| Error::Config(format!("Failed to serialize settings: {}", e)))?;
    fs::write(&settings_path, json)?;
    Ok(())
}

/// Get a human-readable description for a rule
fn get_rule_description(name: &str) -> String {
    match name {
        "space-word" => {
            "Add space between CJK (Chinese, Japanese, Korean) and English words".to_string()
        }
        "space-punctuation" => "Add space between some punctuation marks and CJK text".to_string(),
        "space-bracket" => "Add space between brackets (), [] when near CJK text".to_string(),
        "space-backticks" => "Add space between backticks `` when near CJK text".to_string(),
        "space-dash" => "Add space around dash `-` when near CJK text".to_string(),
        "space-dollar" => "Add space between dollar sign $ when near CJK text".to_string(),
        "fullwidth" => {
            "Convert punctuation and symbols to fullwidth characters in CJK context".to_string()
        }
        "halfwidth-word" => "Convert fullwidth alphanumeric characters to halfwidth".to_string(),
        "halfwidth-punctuation" => {
            "Convert fullwidth punctuation to halfwidth in English text".to_string()
        }
        "no-space-fullwidth" => "Remove unnecessary spaces near fullwidth punctuation".to_string(),
        "no-space-fullwidth-quote" => "Remove spaces around fullwidth quotes".to_string(),
        "spellcheck" => "Check spelling against custom dictionary".to_string(),
        _ => format!("Rule: {}", name),
    }
}

/// Get the default severity for a rule from the embedded default config
fn get_default_rule_severity(rule_name: &str) -> u8 {
    let default_config_str = include_str!("../../../../autocorrect/.autocorrectrc.default");
    if let Ok(config) = autocorrect::config::Config::from_str(default_config_str) {
        config.rules.get(rule_name).map(|m| *m as u8).unwrap_or(1)
    } else {
        1 // Default to error if we can't parse
    }
}

/// Serialize a Config to a clean YAML format
fn serialize_config_to_yaml(config: &autocorrect::config::Config) -> Result<String, Error> {
    let mut yaml = String::from("# AutoCorrect Configuration\n# Generated by AutoCorrect App\n\n");

    // Add rules section
    if !config.rules.is_empty() {
        yaml.push_str("rules:\n");
        let mut rule_names: Vec<String> = config.rules.keys().cloned().collect();
        rule_names.sort();

        for name in rule_names {
            if let Some(mode) = config.rules.get(&name) {
                let severity = *mode as u8;
                // Only include non-default values
                if severity != get_default_rule_severity(&name) {
                    yaml.push_str(&format!("  {}: {}\n", name, severity));
                }
            }
        }
        yaml.push('\n');
    }

    // Add text rules section
    if !config.text_rules.is_empty() {
        yaml.push_str("textRules:\n");
        let mut rule_names: Vec<String> = config.text_rules.keys().cloned().collect();
        rule_names.sort();

        for name in rule_names {
            if let Some(mode) = config.text_rules.get(&name) {
                yaml.push_str(&format!("  {}: {}\n", name, *mode as u8));
            }
        }
        yaml.push('\n');
    }

    // Add spellcheck section
    if !config.spellcheck.words.is_empty() {
        yaml.push_str("spellcheck:\n");
        yaml.push_str("  words:\n");
        for word in &config.spellcheck.words {
            yaml.push_str(&format!("    - {}\n", word));
        }
        yaml.push('\n');
    }

    // Add file types section
    if !config.file_types.is_empty() {
        yaml.push_str("fileTypes:\n");
        let mut exts: Vec<String> = config.file_types.keys().cloned().collect();
        exts.sort();

        for ext in exts {
            if let Some(file_type) = config.file_types.get(&ext) {
                yaml.push_str(&format!("  {}: {}\n", ext, file_type));
            }
        }
        yaml.push('\n');
    }

    // Add context section
    if !config.context.is_empty() {
        yaml.push_str("context:\n");
        let mut contexts: Vec<String> = config.context.keys().cloned().collect();
        contexts.sort();

        for ctx in contexts {
            if let Some(mode) = config.context.get(&ctx) {
                yaml.push_str(&format!("  {}: {}\n", ctx, *mode as u8));
            }
        }
    }

    Ok(yaml)
}
