use crate::theme_errors::ThemeError;
use tauri::Emitter;
use tauri::Theme;
use tauri_plugin_store::StoreExt;

const APP_SETTINGS_STORE_FILE: &str = "app-settings.json";
const THEME_STORE_KEY: &str = "theme";
const DEFAULT_THEME: &str = "auto";
const THEME_CHANGED_EVENT: &str = "theme-changed";

fn parse_theme(theme: &str) -> Result<Option<Theme>, ThemeError> {
    match theme {
        "light" => Ok(Some(Theme::Light)),
        "dark" => Ok(Some(Theme::Dark)),
        "auto" => Ok(None),
        _ => Err(ThemeError::InvalidTheme(theme.to_string())),
    }
}

fn normalize_theme(value: Option<&str>) -> &'static str {
    match value {
        Some("light") => "light",
        Some("dark") => "dark",
        Some("auto") => "auto",
        _ => DEFAULT_THEME,
    }
}

#[tauri::command]
pub fn get_theme(app: tauri::AppHandle) -> Result<String, ThemeError> {
    let store = app
        .store(APP_SETTINGS_STORE_FILE)
        .map_err(|e| ThemeError::Store(format!("Failed to access theme store: {}", e)))?;

    let stored = store
        .get(THEME_STORE_KEY)
        .and_then(|value| value.as_str().map(ToString::to_string));

    Ok(normalize_theme(stored.as_deref()).to_string())
}

#[tauri::command]
pub fn set_theme(app: tauri::AppHandle, theme: String) -> Result<String, ThemeError> {
    parse_theme(&theme)?;
    let normalized = normalize_theme(Some(theme.as_str())).to_string();

    let store = app
        .store(APP_SETTINGS_STORE_FILE)
        .map_err(|e| ThemeError::Store(format!("Failed to access theme store: {}", e)))?;

    store.set(THEME_STORE_KEY, serde_json::Value::String(normalized.clone()));
    store
        .save()
        .map_err(|e| ThemeError::Store(format!("Failed to persist theme store: {}", e)))?;

    let tauri_theme = parse_theme(&normalized)?;
    app.set_theme(tauri_theme);

    app.emit(THEME_CHANGED_EVENT, normalized.clone())
        .map_err(|e| ThemeError::Sync(format!("Failed to emit theme change event: {}", e)))?;

    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::{normalize_theme, parse_theme};

    #[test]
    fn normalizes_invalid_theme_to_auto() {
        assert_eq!(normalize_theme(Some("unknown")), "auto");
        assert_eq!(normalize_theme(None), "auto");
    }

    #[test]
    fn accepts_valid_theme_values() {
        assert!(parse_theme("light").is_ok());
        assert!(parse_theme("dark").is_ok());
        assert!(parse_theme("auto").is_ok());
    }

    #[test]
    fn rejects_invalid_theme_values() {
        assert!(parse_theme("system").is_err());
    }
}
