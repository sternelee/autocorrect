//! Hotkey configuration commands
//!
//! This module provides Tauri commands for managing the global hotkey configuration.

use crate::commands::errors::Error;
use crate::hotkey::{HotkeyConfig, HotkeyConfigCell};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::Manager;

/// Push a new binding into the shared config cell so the running rdev
/// listener picks it up on the next event without a restart.
fn apply_to_live_listener(
    app: &tauri::AppHandle,
    config: &HotkeyConfig,
) {
    if let Some(cell) = app.try_state::<HotkeyConfigCell>() {
        if let Ok(mut guard) = cell.0.lock() {
            *guard = config.clone();
            log::info!(
                "Hotkey config swapped live: {}",
                guard.to_display_string()
            );
        } else {
            log::error!("Failed to lock HotkeyConfigCell; live update skipped.");
        }
    } else {
        log::warn!(
            "HotkeyConfigCell not managed yet; live update skipped (startup race?"
        );
    }
}

/// Hotkey configuration file name
const CONFIG_FILE: &str = "hotkey-config.json";

/// Get the configuration directory path
fn get_config_dir() -> Result<PathBuf, Error> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| {
            Error::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Configuration directory not found",
            ))
        })?
        .join("autocorrect-app");

    // Create directory if it doesn't exist
    fs::create_dir_all(&config_dir).map_err(|e| {
        Error::Io(std::io::Error::other(
            format!("Failed to create config directory: {}", e),
        ))
    })?;

    Ok(config_dir)
}

/// Get the hotkey configuration file path
fn get_config_path() -> Result<PathBuf, Error> {
    Ok(get_config_dir()?.join(CONFIG_FILE))
}

/// Load hotkey configuration from file
fn load_config_from_file() -> Result<HotkeyConfig, Error> {
    let config_path = get_config_path()?;

    if !config_path.exists() {
        // Return default config if file doesn't exist
        return Ok(HotkeyConfig::default());
    }

    let content = fs::read_to_string(&config_path).map_err(|e| {
        Error::Io(std::io::Error::other(
            format!("Failed to read config file: {}", e),
        ))
    })?;

    let mut config: HotkeyConfig = serde_json::from_str(&content).map_err(|e| {
        Error::Io(std::io::Error::other(
            format!("Failed to parse config file: {}", e),
        ))
    })?;

    // Sync the key field from key_name
    config.sync_key();

    Ok(config)
}

/// Save hotkey configuration to file
fn save_config_to_file(config: &HotkeyConfig) -> Result<(), Error> {
    let config_path = get_config_path()?;

    let content = serde_json::to_string_pretty(config).map_err(|e| {
        Error::Io(std::io::Error::other(
            format!("Failed to serialize config: {}", e),
        ))
    })?;

    fs::write(&config_path, content).map_err(|e| {
        Error::Io(std::io::Error::other(
            format!("Failed to write config file: {}", e),
        ))
    })?;

    log::info!("Hotkey configuration saved to {:?}", config_path);

    Ok(())
}

/// Get the current hotkey configuration
#[tauri::command]
pub fn get_hotkey_config() -> Result<HotkeyConfigResponse, Error> {
    let config = load_config_from_file()?;
    Ok(HotkeyConfigResponse {
        key: config.key_name.clone(),
        modifiers: config.modifiers.clone(),
        display_string: config.to_display_string(),
    })
}

/// Hotkey configuration response (without the internal Key field)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotkeyConfigResponse {
    pub key: String,
    pub modifiers: crate::hotkey::Modifiers,
    pub display_string: String,
}

/// Update hotkey configuration request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateHotkeyConfigRequest {
    pub key: String,
    pub modifiers: crate::hotkey::Modifiers,
}

/// Update the hotkey configuration
#[tauri::command]
pub fn update_hotkey_config(
    app: tauri::AppHandle,
    request: UpdateHotkeyConfigRequest,
) -> Result<HotkeyConfigResponse, Error> {
    let config = HotkeyConfig::new(request.key.clone(), request.modifiers);

    // Validate the configuration
    if config.key_name.is_empty() {
        return Err(Error::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Key name cannot be empty",
        )));
    }

    // Save the configuration
    save_config_to_file(&config)?;

    // Push the new binding into the running rdev listener so the change
    // takes effect immediately, without restarting the app.
    apply_to_live_listener(&app, &config);

    log::info!("Hotkey configuration updated: {:?}", config);

    Ok(HotkeyConfigResponse {
        key: config.key_name.clone(),
        modifiers: config.modifiers.clone(),
        display_string: config.to_display_string(),
    })
}

/// Reset hotkey configuration to default
#[tauri::command]
pub fn reset_hotkey_config(
    app: tauri::AppHandle,
) -> Result<HotkeyConfigResponse, Error> {
    let config = HotkeyConfig::default();

    // Save the default configuration
    save_config_to_file(&config)?;

    apply_to_live_listener(&app, &config);

    log::info!("Hotkey configuration reset to default");

    Ok(HotkeyConfigResponse {
        key: config.key_name.clone(),
        modifiers: config.modifiers.clone(),
        display_string: config.to_display_string(),
    })
}

/// Get available keys for hotkey configuration
#[tauri::command]
pub fn get_available_keys() -> Vec<KeyInfo> {
    vec![
        KeyInfo {
            name: "KeyA".to_string(),
            label: "A".to_string(),
        },
        KeyInfo {
            name: "KeyB".to_string(),
            label: "B".to_string(),
        },
        KeyInfo {
            name: "KeyC".to_string(),
            label: "C".to_string(),
        },
        KeyInfo {
            name: "KeyD".to_string(),
            label: "D".to_string(),
        },
        KeyInfo {
            name: "KeyE".to_string(),
            label: "E".to_string(),
        },
        KeyInfo {
            name: "KeyF".to_string(),
            label: "F".to_string(),
        },
        KeyInfo {
            name: "KeyG".to_string(),
            label: "G".to_string(),
        },
        KeyInfo {
            name: "KeyH".to_string(),
            label: "H".to_string(),
        },
        KeyInfo {
            name: "KeyI".to_string(),
            label: "I".to_string(),
        },
        KeyInfo {
            name: "KeyJ".to_string(),
            label: "J".to_string(),
        },
        KeyInfo {
            name: "KeyK".to_string(),
            label: "K".to_string(),
        },
        KeyInfo {
            name: "KeyL".to_string(),
            label: "L".to_string(),
        },
        KeyInfo {
            name: "KeyM".to_string(),
            label: "M".to_string(),
        },
        KeyInfo {
            name: "KeyN".to_string(),
            label: "N".to_string(),
        },
        KeyInfo {
            name: "KeyO".to_string(),
            label: "O".to_string(),
        },
        KeyInfo {
            name: "KeyP".to_string(),
            label: "P".to_string(),
        },
        KeyInfo {
            name: "KeyQ".to_string(),
            label: "Q".to_string(),
        },
        KeyInfo {
            name: "KeyR".to_string(),
            label: "R".to_string(),
        },
        KeyInfo {
            name: "KeyS".to_string(),
            label: "S".to_string(),
        },
        KeyInfo {
            name: "KeyT".to_string(),
            label: "T".to_string(),
        },
        KeyInfo {
            name: "KeyU".to_string(),
            label: "U".to_string(),
        },
        KeyInfo {
            name: "KeyV".to_string(),
            label: "V".to_string(),
        },
        KeyInfo {
            name: "KeyW".to_string(),
            label: "W".to_string(),
        },
        KeyInfo {
            name: "KeyX".to_string(),
            label: "X".to_string(),
        },
        KeyInfo {
            name: "KeyY".to_string(),
            label: "Y".to_string(),
        },
        KeyInfo {
            name: "KeyZ".to_string(),
            label: "Z".to_string(),
        },
        KeyInfo {
            name: "Space".to_string(),
            label: "Space".to_string(),
        },
        KeyInfo {
            name: "Return".to_string(),
            label: "Return".to_string(),
        },
        KeyInfo {
            name: "Tab".to_string(),
            label: "Tab".to_string(),
        },
    ]
}

/// Key information for UI display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyInfo {
    pub name: String,
    pub label: String,
}

/// Export the load_config function for use in lib.rs
pub fn load_hotkey_config() -> HotkeyConfig {
    load_config_from_file().unwrap_or_else(|e| {
        log::warn!("Failed to load hotkey config: {}, using default", e);
        HotkeyConfig::default()
    })
}
