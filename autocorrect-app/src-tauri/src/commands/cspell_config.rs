use super::errors::Error;
use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;

/// CSpell user configuration exposed to the frontend.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CSpellConfig {
    #[serde(default)]
    pub words: Vec<String>,
    #[serde(default)]
    pub ignore_words: Vec<String>,
    #[serde(default)]
    pub flag_words: Vec<String>,
}

fn default_cspell_config_path() -> Result<PathBuf, Error> {
    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .map_err(|_| Error::Config("Could not determine HOME directory".to_string()))?;
    Ok(PathBuf::from(home).join(".autocorrect-cspell.json"))
}

fn resolve_cspell_path(path: Option<String>) -> Result<PathBuf, Error> {
    match path {
        Some(p) if !p.trim().is_empty() => Ok(PathBuf::from(p)),
        _ => default_cspell_config_path(),
    }
}

/// Load the CSpell configuration from the given path (or the default path).
#[tauri::command]
pub fn get_cspell_config(path: Option<String>) -> Result<CSpellConfig, Error> {
    let path = resolve_cspell_path(path)?;
    if !path.exists() {
        return Ok(CSpellConfig::default());
    }
    let content = std::fs::read_to_string(&path)
        .map_err(|e| Error::Config(format!("Failed to read cspell config: {}", e)))?;
    serde_json::from_str(&content)
        .map_err(|e| Error::Config(format!("Invalid cspell config: {}", e)))
}

/// Save the CSpell configuration to the given path (or the default path) and
/// reload the in-memory state so changes take effect immediately.
#[tauri::command]
pub fn save_cspell_config(path: Option<String>, config: CSpellConfig) -> Result<(), Error> {
    let path = resolve_cspell_path(path)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| Error::Config(format!("Failed to create config directory: {}", e)))?;
    }
    let content = serde_json::to_string_pretty(&config)
        .map_err(|e| Error::Config(format!("Failed to serialize cspell config: {}", e)))?;
    std::fs::write(&path, content)
        .map_err(|e| Error::Config(format!("Failed to write cspell config: {}", e)))?;
    crate::typocheck::reload_cspell_config();
    Ok(())
}

/// Reload the CSpell configuration from the currently active config path.
#[tauri::command]
pub fn reload_cspell_config_cmd() -> Result<(), Error> {
    crate::typocheck::reload_cspell_config();
    Ok(())
}

/// Return the default CSpell config file path for display in the UI.
#[tauri::command]
pub fn get_default_cspell_config_path() -> Result<String, Error> {
    default_cspell_config_path().map(|p| p.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cspell_config_roundtrip() {
        let config = CSpellConfig {
            words: vec!["tauri".to_string()],
            ignore_words: vec!["ignored".to_string()],
            flag_words: vec!["whts: what's".to_string()],
        };
        let json = serde_json::to_string_pretty(&config).unwrap();
        let parsed: CSpellConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.words, config.words);
        assert_eq!(parsed.ignore_words, config.ignore_words);
        assert_eq!(parsed.flag_words, config.flag_words);
    }
}
