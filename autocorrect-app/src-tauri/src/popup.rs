use crate::commands::spellcheck::SpellCheckResult;
use crate::commands::errors::Error;
use tauri::{AppHandle, Emitter, Manager, State, Window};
use std::sync::{Arc, Mutex};

/// Popup state shared across the application
#[derive(Debug, Clone)]
pub struct PopupState {
    is_visible: bool,
    position: (i32, i32),
    original_text: String,
    suggestion: String,
}

impl PopupState {
    pub fn new() -> Self {
        Self {
            is_visible: false,
            position: (0, 0),
            original_text: String::new(),
            suggestion: String::new(),
        }
    }
}

/// Shared popup state wrapper
pub struct SharedPopupState(pub Arc<Mutex<PopupState>>);

impl SharedPopupState {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(PopupState::new())))
    }
}

/// Show the popup window with spell check results
#[tauri::command]
pub fn show_popup(
    app: AppHandle,
    x: i32,
    y: i32,
    original_text: String,
    suggestion: String,
) -> Result<(), Error> {
    // Get or create the popup window
    if let Some(popup_window) = app.get_webview_window("popup") {
        // Update state
        if let Some(state) = app.try_state::<SharedPopupState>() {
            let mut state = state.0.lock().map_err(|_| Error::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to lock popup state"
            )))?;
            state.is_visible = true;
            state.position = (x, y);
            state.original_text = original_text.clone();
            state.suggestion = suggestion.clone();
        }

        // Position the window
        let _ = popup_window.set_position(tauri::Position::Physical(tauri::PhysicalPosition {
            x,
            y,
        }));

        // Show and focus the popup
        let _ = popup_window.show();
        let _ = popup_window.set_focus();
        let _ = popup_window.set_always_on_top(true);

        // Emit event to frontend with the data
        let _ = app.emit("popup-show", &serde_json::json!({
            "originalText": original_text,
            "suggestion": suggestion,
            "x": x,
            "y": y
        }));

        Ok(())
    } else {
        Err(Error::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Popup window not found",
        )))
    }
}

/// Hide the popup window
#[tauri::command]
pub fn hide_popup(app: AppHandle) -> Result<(), Error> {
    if let Some(popup_window) = app.get_webview_window("popup") {
        // Update state
        if let Some(state) = app.try_state::<SharedPopupState>() {
            let mut state = state.0.lock().map_err(|_| Error::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to lock popup state"
            )))?;
            state.is_visible = false;
        }

        let _ = popup_window.hide();
        let _ = app.emit("popup-hide", ());

        Ok(())
    } else {
        Err(Error::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Popup window not found",
        )))
    }
}

/// Position the popup at specific coordinates
#[tauri::command]
pub fn position_popup(app: AppHandle, x: i32, y: i32) -> Result<(), Error> {
    if let Some(popup_window) = app.get_webview_window("popup") {
        let _ = popup_window.set_position(tauri::Position::Physical(tauri::PhysicalPosition {
            x,
            y,
        }));

        // Update state
        if let Some(state) = app.try_state::<SharedPopupState>() {
            let mut state = state.0.lock().map_err(|_| Error::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to lock popup state"
            )))?;
            state.position = (x, y);
        }

        Ok(())
    } else {
        Err(Error::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Popup window not found",
        )))
    }
}

/// Get the current popup state
#[tauri::command]
pub fn get_popup_state(state: State<SharedPopupState>) -> Result<serde_json::Value, Error> {
    let state = state.0.lock().map_err(|_| Error::Io(std::io::Error::new(
        std::io::ErrorKind::Other,
        "Failed to lock popup state"
    )))?;

    Ok(serde_json::json!({
        "isVisible": state.is_visible,
        "x": state.position.0,
        "y": state.position.1,
        "originalText": state.original_text,
        "suggestion": state.suggestion
    }))
}

/// Accept the suggestion - set clipboard to corrected text
///
/// Note: Due to conflicts between rdev (global hotkey listener) and enigo (keyboard simulation),
/// automatic paste simulation has been disabled. The corrected text will be copied to clipboard
/// and the user should manually paste (⌘+V).
#[tauri::command]
pub fn accept_suggestion(app: AppHandle, text: String) -> Result<(), Error> {
    // Set clipboard to the corrected text
    use crate::commands::spellcheck::set_clipboard_text;
    set_clipboard_text(text.clone())?;

    // Hide popup
    hide_popup(app.clone())?;

    // Emit accepted event with the corrected text
    let _ = app.emit("suggestion-accepted", serde_json::json!({
        "text": text,
        "message": "Corrected text copied to clipboard. Press ⌘+V to paste."
    }));

    Ok(())
}

/// Reject the suggestion - just hide popup
#[tauri::command]
pub fn reject_suggestion(app: AppHandle) -> Result<(), Error> {
    // Clone before hiding so we can still use app for emit
    hide_popup(app.clone())?;
    let _ = app.emit("suggestion-rejected", ());
    Ok(())
}

/// Trigger spell check workflow - get selected text, check, show popup
///
/// This function handles the complete workflow:
/// 1. Gets the currently selected text from the system (via Accessibility API or copy simulation)
/// 2. Runs spell check on the text
/// 3. Shows popup with suggestions if corrections are needed
#[tauri::command]
pub fn trigger_spell_check_workflow(
    app: AppHandle,
    x: i32,
    y: i32,
) -> Result<(), Error> {
    use crate::commands::spellcheck::spell_check;
    use crate::text_selection::{get_selected_text, get_selected_text_via_copy, TextSelectionError};

    // Try to get selected text using Accessibility API first
    let text = match get_selected_text() {
        Ok(text) => {
            log::info!("Got selected text via Accessibility API: {} chars", text.chars().count());
            text
        }
        Err(TextSelectionError::PermissionDenied) => {
            log::warn!("Accessibility permission denied, falling back to copy simulation");
            // Fall back to copy simulation
            match get_selected_text_via_copy() {
                Ok(text) => {
                    log::info!("Got selected text via copy simulation: {} chars", text.chars().count());
                    text
                }
                Err(e) => {
                    log::warn!("Failed to get selected text: {}", e);
                    // Emit event indicating no text was selected
                    let _ = app.emit("no-text-selected", ());
                    return Ok(());
                }
            }
        }
        Err(TextSelectionError::EmptySelection) => {
            log::info!("No text selected");
            let _ = app.emit("no-text-selected", ());
            return Ok(());
        }
        Err(e) => {
            log::warn!("Failed to get selected text via API: {}, trying copy simulation", e);
            // Fall back to copy simulation
            match get_selected_text_via_copy() {
                Ok(text) => {
                    log::info!("Got selected text via copy simulation: {} chars", text.chars().count());
                    text
                }
                Err(e) => {
                    log::warn!("Failed to get selected text: {}", e);
                    let _ = app.emit("no-text-selected", ());
                    return Ok(());
                }
            }
        }
    };

    if text.trim().is_empty() {
        log::info!("Selected text is empty");
        let _ = app.emit("no-text-selected", ());
        return Ok(());
    }

    // Run spell check
    let result = spell_check(text)?;

    // If there are changes, show popup
    if result.has_changes && !result.corrected.is_empty() {
        log::info!("Spell check found corrections needed");
        show_popup(
            app,
            x,
            y,
            result.original,
            result.corrected,
        )?;
    } else {
        // No changes needed, emit a notification
        log::info!("Spell check: no changes needed");
        let _ = app.emit("no-changes-needed", serde_json::json!({
            "message": "Text is already correct",
            "original": result.original
        }));
    }

    Ok(())
}
