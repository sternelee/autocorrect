use crate::commands::config::{load_app_settings, save_app_settings};
use crate::commands::errors::Error;
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

/// Information about an ignored app
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IgnoredApp {
    /// Human-readable display name, e.g. "Xcode"
    pub name: String,
    /// Unique key, e.g. "com.apple.dt.Xcode"
    pub bundle_id: String,
    /// Suppress 💡 icon + popup + ai-popup
    pub ignore_popup: bool,
    /// Suppress overlay underline rendering
    pub ignore_overlay: bool,
}

/// App info returned by get_frontmost_app_info
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: String,
    pub bundle_id: String,
}

/// Get the list of ignored apps
#[tauri::command]
pub fn get_ignored_apps(app: AppHandle) -> Result<Vec<IgnoredApp>, Error> {
    let settings = load_app_settings(&app)?;
    Ok(settings.ignored_apps)
}

/// Add or update an ignored app (upsert by bundle_id)
#[tauri::command]
pub fn add_ignored_app(
    app: AppHandle,
    name: String,
    bundle_id: String,
    ignore_popup: bool,
    ignore_overlay: bool,
) -> Result<(), Error> {
    let mut settings = load_app_settings(&app)?;

    // Check if bundle_id already exists
    if let Some(existing) = settings
        .ignored_apps
        .iter_mut()
        .find(|a| a.bundle_id == bundle_id)
    {
        // Update existing entry
        existing.name = name;
        existing.ignore_popup = ignore_popup;
        existing.ignore_overlay = ignore_overlay;
    } else {
        // Add new entry
        settings.ignored_apps.push(IgnoredApp {
            name,
            bundle_id,
            ignore_popup,
            ignore_overlay,
        });
    }

    save_app_settings(&app, &settings)
}

/// Update an ignored app's flags
#[tauri::command]
pub fn update_ignored_app(
    app: AppHandle,
    bundle_id: String,
    ignore_popup: bool,
    ignore_overlay: bool,
) -> Result<(), Error> {
    let mut settings = load_app_settings(&app)?;

    if let Some(app) = settings
        .ignored_apps
        .iter_mut()
        .find(|a| a.bundle_id == bundle_id)
    {
        app.ignore_popup = ignore_popup;
        app.ignore_overlay = ignore_overlay;
    }

    save_app_settings(&app, &settings)
}

/// Remove an ignored app by bundle_id
#[tauri::command]
pub fn remove_ignored_app(app: AppHandle, bundle_id: String) -> Result<(), Error> {
    let mut settings = load_app_settings(&app)?;
    settings.ignored_apps.retain(|a| a.bundle_id != bundle_id);
    save_app_settings(&app, &settings)
}

/// Get information about the frontmost app (for Ignore button in popup)
#[tauri::command]
pub fn get_frontmost_app_info(_app: AppHandle) -> Result<Option<AppInfo>, Error> {
    #[cfg(target_os = "macos")]
    {
        if let Some((name, bundle_id)) = get_frontmost_app_info_macos() {
            return Ok(Some(AppInfo { name, bundle_id }));
        }
    }
    Ok(None)
}

/// Check if an app is ignored for popup or overlay
pub fn is_app_ignored(
    app: &AppHandle,
    bundle_id: &str,
    check_popup: bool,
    check_overlay: bool,
) -> bool {
    if let Ok(settings) = load_app_settings(app) {
        if let Some(ignored) = settings
            .ignored_apps
            .iter()
            .find(|a| a.bundle_id == bundle_id)
        {
            if check_popup && ignored.ignore_popup {
                return true;
            }
            if check_overlay && ignored.ignore_overlay {
                return true;
            }
        }
    }
    false
}

/// Get the frontmost app's bundle ID (macOS only, uses osascript)
#[cfg(target_os = "macos")]
pub fn get_frontmost_bundle_id_macos() -> Option<String> {
    let output = std::process::Command::new("osascript")
        .arg("-e")
        .arg("tell application \"System Events\" to get bundle identifier of first application process whose frontmost is true")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let bundle_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if bundle_id.is_empty() {
        None
    } else {
        Some(bundle_id)
    }
}

/// Get the frontmost app's name and bundle ID (macOS only, uses osascript)
#[cfg(target_os = "macos")]
pub fn get_frontmost_app_info_macos() -> Option<(String, String)> {
    // Get app name
    let name_output = std::process::Command::new("osascript")
        .arg("-e")
        .arg("tell application \"System Events\" to get name of first application process whose frontmost is true")
        .output()
        .ok()?;
    if !name_output.status.success() {
        return None;
    }
    let name = String::from_utf8_lossy(&name_output.stdout)
        .trim()
        .to_string();
    if name.is_empty() {
        return None;
    }

    // Get bundle ID
    let bundle_output = std::process::Command::new("osascript")
        .arg("-e")
        .arg("tell application \"System Events\" to get bundle identifier of first application process whose frontmost is true")
        .output()
        .ok()?;
    if !bundle_output.status.success() {
        return None;
    }
    let bundle_id = String::from_utf8_lossy(&bundle_output.stdout)
        .trim()
        .to_string();

    Some((name, bundle_id))
}
