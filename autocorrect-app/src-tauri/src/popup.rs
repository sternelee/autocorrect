#![allow(unexpected_cfgs)]

use crate::commands::errors::Error;
use crate::commands::spellcheck::TypoSuggestion;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};

/// Popup state shared across the application
#[derive(Debug, Clone)]
pub struct PopupState {
    pub is_visible: bool,
    pub position: (i32, i32),
    original_text: String,
    suggestion: String,
    source_app_name: Option<String>,
}

impl PopupState {
    pub fn new() -> Self {
        Self {
            is_visible: false,
            position: (0, 0),
            original_text: String::new(),
            suggestion: String::new(),
            source_app_name: None,
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
    typos: Option<Vec<TypoSuggestion>>,
    offset: Option<usize>,
    char_length: Option<usize>,
) -> Result<(), Error> {
    log::info!("show_popup called with position: ({}, {})", x, y);

    // Get or create the popup window
    if let Some(popup_window) = app.get_webview_window("popup") {
        // Update state
        if let Some(state) = app.try_state::<SharedPopupState>() {
            let mut state = state.0.lock().map_err(|_| {
                Error::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Failed to lock popup state",
                ))
            })?;
            state.is_visible = true;
            state.position = (x, y);
            state.original_text = original_text.clone();
            state.suggestion = suggestion.clone();
            #[cfg(target_os = "macos")]
            {
                state.source_app_name = get_frontmost_app_name_macos();
            }
        }

        // Position the window
        let position = tauri::Position::Logical(tauri::LogicalPosition {
            x: x as f64,
            y: y as f64,
        });
        log::info!("Setting popup position to {:?}", position);
        let _ = popup_window.set_position(position);

        // Show the popup and make it the key window so it receives keyboard events.
        // We hide the main window first so that when the app activates (a side-effect
        // of makeKeyAndOrderFront) it doesn't appear on top of the source app.
        // All NSWindow calls must run on the main thread.
        #[cfg(target_os = "macos")]
        {
            let popup_window_mt = popup_window.clone();
            let app_mt = app.clone();
            let _ = popup_window.run_on_main_thread(move || {
                use objc2::msg_send;
                use objc2::runtime::AnyClass;
                type Id = *mut objc2::runtime::AnyObject;
                const NIL: Id = std::ptr::null_mut();

                // Hide the main window using NSWindow directly (synchronous, no
                // Tauri dispatch queuing) so it is gone before makeKeyAndOrderFront
                // activates the app.
                if let Some(main) = app_mt.get_webview_window("main") {
                    if let Ok(main_ptr) = main.ns_window() {
                        unsafe {
                            let main_ns = main_ptr as Id;
                            let _: () = msg_send![main_ns, orderOut: NIL];
                        }
                    }
                }

                if let Ok(ptr) = popup_window_mt.ns_window() {
                    unsafe {
                        let ns_window = ptr as Id;
                        let _: () = msg_send![ns_window, setLevel: 2001_i64];
                        let _: () = msg_send![ns_window, setHidesOnDeactivate: false];
                        let _: () = msg_send![ns_window, setAcceptsMouseMovedEvents: true];
                        // Activate the app first so makeKeyAndOrderFront actually
                        // grants key-window status.  Without this the popup appears
                        // but remains a non-key window (AutoCorrect is not the
                        // frontmost app), causing WKWebView to skip hover tracking
                        // until the user clicks once.
                        let app_class = AnyClass::get("NSApplication").expect("NSApplication not found");
                        let app_ns: Id = msg_send![app_class, sharedApplication];
                        let _: () = msg_send![app_ns, activateIgnoringOtherApps: true];
                        let _: () = msg_send![ns_window, makeKeyAndOrderFront: NIL];
                        let content_view: Id = msg_send![ns_window, contentView];
                        let _: bool = msg_send![ns_window, makeFirstResponder: content_view];
                    }
                }
            });
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = popup_window.show();
            let _ = popup_window.set_always_on_top(true);
        }

        // Emit event to frontend with the data including typos
        let _ = app.emit(
            "popup-show",
            &serde_json::json!({
                "originalText": original_text,
                "suggestion": suggestion,
                "x": x,
                "y": y,
                "typos": typos.unwrap_or_default(),
                "offset": offset,
                "charLength": char_length
            }),
        );

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
            let mut state = state.0.lock().map_err(|_| {
                Error::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Failed to lock popup state",
                ))
            })?;
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
        let _ = popup_window.set_position(tauri::Position::Logical(tauri::LogicalPosition {
            x: x as f64,
            y: y as f64,
        }));

        // Update state
        if let Some(state) = app.try_state::<SharedPopupState>() {
            let mut state = state.0.lock().map_err(|_| {
                Error::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Failed to lock popup state",
                ))
            })?;
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
    let state = state.0.lock().map_err(|_| {
        Error::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to lock popup state",
        ))
    })?;

    Ok(serde_json::json!({
        "isVisible": state.is_visible,
        "x": state.position.0,
        "y": state.position.1,
        "originalText": state.original_text,
        "suggestion": state.suggestion,
        "sourceAppName": state.source_app_name
    }))
}

/// Accept the suggestion and apply to the currently selected text.
#[tauri::command]
pub fn accept_suggestion(
    app: AppHandle,
    text: String,
    offset: Option<usize>,
    char_length: Option<usize>,
) -> Result<(), Error> {
    #[cfg(target_os = "macos")]
    {
        // Pass offset/char_length into the macOS handler so the typo range
        // is selected AFTER focus returns to the source app (not while popup
        // still has focus, which would select into the wrong window).
        match apply_suggestion_to_selection_macos(app.clone(), &text, offset, char_length) {
            Ok(()) => {
                let _ = app.emit(
                    "suggestion-accepted",
                    serde_json::json!({
                        "text": text,
                        "message": "Suggestion applied to selected text."
                    }),
                );
                return Ok(());
            }
            Err(e) => {
                log::warn!(
                    "Auto-apply suggestion failed, fallback to clipboard-only mode: {}",
                    e
                );
            }
        }
    }

    // Fallback: keep clipboard-only behavior.
    use crate::commands::spellcheck::set_clipboard_text;
    set_clipboard_text(text.clone())?;

    // Hide popup
    hide_popup(app.clone())?;

    let _ = app.emit(
        "suggestion-accepted",
        serde_json::json!({
            "text": text,
            "message": "Corrected text copied to clipboard. Press ⌘+V to paste."
        }),
    );

    Ok(())
}

#[cfg(target_os = "macos")]
fn apply_suggestion_to_selection_macos(
    app: AppHandle,
    text: &str,
    offset: Option<usize>,
    char_length: Option<usize>,
) -> Result<(), Error> {
    let source_app_name = app
        .try_state::<SharedPopupState>()
        .and_then(|state| state.0.lock().ok().and_then(|s| s.source_app_name.clone()));

    let mut clipboard = arboard::Clipboard::new()
        .map_err(|e| Error::Clipboard(format!("Failed to access clipboard: {e}")))?;
    let previous_clipboard = clipboard.get_text().ok();

    clipboard
        .set_text(text.to_string())
        .map_err(|e| Error::Clipboard(format!("Failed to set clipboard text: {e}")))?;

    // Hide popup so focus can return to the source app.
    hide_popup(app)?;
    thread::sleep(Duration::from_millis(80));

    if let Some(ref app_name) = source_app_name {
        if app_name != "autocorrect-app" && app_name != "AutoCorrect" {
            activate_app_macos(app_name)?;

            // Wait until the source app is actually frontmost using NSWorkspace
            // (instant ObjC call, no subprocess overhead).
            let deadline = std::time::Instant::now();
            loop {
                thread::sleep(Duration::from_millis(30));
                if is_app_frontmost_macos_pub(app_name) {
                    break;
                }
                if deadline.elapsed().as_millis() > 600 {
                    log::warn!(
                        "[accept] activate timeout: {} still not frontmost",
                        app_name
                    );
                    break;
                }
            }
            // Extra settle time so the AX focused-element state catches up.
            thread::sleep(Duration::from_millis(80));
        }
    }

    // Now that the source app has focus, select the exact typo range so the
    // paste replaces the word rather than inserting at the cursor.
    if let (Some(start), Some(len)) = (offset, char_length) {
        log::info!("[accept] select_text_range: offset={} len={}", start, len);
        // Retry for up to 600 ms in case the AX focus is still settling.
        let sel_deadline = std::time::Instant::now();
        loop {
            match crate::macos_text::select_text_range(start, len) {
                Ok(()) => {
                    log::info!("[accept] select_text_range succeeded");
                    thread::sleep(Duration::from_millis(50));
                    break;
                }
                Err(e) => {
                    log::warn!("[accept] select_text_range failed: {}", e);
                    if sel_deadline.elapsed().as_millis() > 600 {
                        log::warn!(
                            "[accept] select_text_range timed out, paste will insert at caret"
                        );
                        break;
                    }
                    thread::sleep(Duration::from_millis(60));
                }
            }
        }
    }

    let status = std::process::Command::new("osascript")
        .arg("-e")
        .arg("tell application \"System Events\" to keystroke \"v\" using command down")
        .status()
        .map_err(|e| Error::InputSimulation(format!("Failed to trigger paste: {e}")))?;

    if !status.success() {
        restore_clipboard(&mut clipboard, previous_clipboard);
        return Err(Error::InputSimulation(
            "Paste simulation command failed".to_string(),
        ));
    }

    thread::sleep(Duration::from_millis(80));
    restore_clipboard(&mut clipboard, previous_clipboard);
    Ok(())
}

#[cfg(target_os = "macos")]
fn restore_clipboard(clipboard: &mut arboard::Clipboard, previous_clipboard: Option<String>) {
    if let Some(old_text) = previous_clipboard {
        let _ = clipboard.set_text(old_text);
    }
}

/// Fast NSWorkspace-based check — no subprocess, returns in microseconds.
#[cfg(target_os = "macos")]
pub fn is_app_frontmost_macos_pub(app_name: &str) -> bool {
    use objc2::msg_send;
    use objc2::runtime::AnyClass;
    

    type Id = *mut objc2::runtime::AnyObject;

    unsafe {
        let workspace_class = AnyClass::get("NSWorkspace").expect("NSWorkspace not found");
        let workspace: Id = msg_send![workspace_class, sharedWorkspace];
        let front_app: Id = msg_send![workspace, frontmostApplication];
        if front_app.is_null() {
            return false;
        }
        let name: Id = msg_send![front_app, localizedName];
        if name.is_null() {
            return false;
        }
        let ns_str: *const std::os::raw::c_char = msg_send![name, UTF8String];
        if ns_str.is_null() {
            return false;
        }
        let rust_str = std::ffi::CStr::from_ptr(ns_str).to_string_lossy();
        rust_str.contains(app_name) || app_name.contains(rust_str.as_ref())
    }
}

#[cfg(target_os = "macos")]
pub fn get_frontmost_app_name_macos() -> Option<String> {
    let output = std::process::Command::new("osascript")
        .arg("-e")
        .arg("tell application \"System Events\" to get name of first application process whose frontmost is true")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

#[cfg(target_os = "macos")]
fn activate_app_macos(app_name: &str) -> Result<(), Error> {
    let escaped = app_name.replace('\\', "\\\\").replace('"', "\\\"");
    let script = format!("tell application \"{}\" to activate", escaped);
    let status = std::process::Command::new("osascript")
        .arg("-e")
        .arg(script)
        .status()
        .map_err(|e| Error::InputSimulation(format!("Failed to activate source app: {e}")))?;

    if status.success() {
        Ok(())
    } else {
        Err(Error::InputSimulation(
            "Failed to activate source app".to_string(),
        ))
    }
}

/// Reject the suggestion - just hide popup
#[tauri::command]
pub fn reject_suggestion(app: AppHandle) -> Result<(), Error> {
    #[cfg(target_os = "macos")]
    let source_app_name = app
        .try_state::<SharedPopupState>()
        .and_then(|state| state.0.lock().ok().and_then(|s| s.source_app_name.clone()));

    // Clone before hiding so we can still use app for emit
    hide_popup(app.clone())?;

    #[cfg(target_os = "macos")]
    if let Some(app_name) = source_app_name {
        if app_name != "autocorrect-app" && app_name != "AutoCorrect" {
            let _ = activate_app_macos(&app_name);
        }
    }

    let _ = app.emit("suggestion-rejected", ());
    Ok(())
}

/// Trigger spell check workflow - get selected text, check, show popup
///
/// This function handles the complete workflow:
/// 1. Gets the currently selected text from the system via Accessibility API
/// 2. Falls back to clipboard if Accessibility is unavailable
/// 3. Runs spell check on the text
/// 4. Shows popup with suggestions if corrections are needed
#[tauri::command]
pub fn trigger_spell_check_workflow(app: AppHandle, x: i32, y: i32) -> Result<(), Error> {
    use crate::commands::spellcheck::spell_check_sync;
    use crate::text_selection::{get_selected_text, TextSelectionError};

    // Get selected text (Accessibility API with clipboard fallback)
    let text = match get_selected_text() {
        Ok(text) => text,
        Err(TextSelectionError::PermissionDenied) => {
            log::warn!("Accessibility permission denied");
            let _ = app.emit("permission-denied", serde_json::json!({
                "message": "Please grant Accessibility permissions in System Settings > Privacy & Security > Accessibility"
            }));
            return Ok(());
        }
        Err(TextSelectionError::NoTextSelected) => {
            log::info!("No text selected");
            let _ = app.emit(
                "no-text-selected",
                serde_json::json!({
                    "message": "Please select some text first, then press the hotkey"
                }),
            );
            return Ok(());
        }
        Err(e) => {
            log::warn!("Failed to get selected text: {}", e);
            let _ = app.emit(
                "error-getting-text",
                serde_json::json!({
                    "message": format!("Error: {}", e)
                }),
            );
            return Ok(());
        }
    };

    if text.trim().is_empty() {
        log::info!("Selected text is empty");
        let _ = app.emit(
            "no-text-selected",
            serde_json::json!({
                "message": "Selected text is empty"
            }),
        );
        return Ok(());
    }

    // Run spell check
    let result = spell_check_sync(app.clone(), text.clone(), Some(true))?;

    // If there are changes or typos, show popup
    if (result.has_changes || !result.typos.is_empty()) && !result.corrected.is_empty() {
        log::info!("Spell check found corrections needed");
        show_popup(
            app,
            x,
            y,
            text,
            result.corrected,
            Some(result.typos),
            None,
            None,
        )?;
    } else {
        // No changes needed, emit a notification
        log::info!("Spell check: no changes needed");
        let _ = app.emit(
            "no-changes-needed",
            serde_json::json!({
                "message": "Text is already correct",
                "original": result.original
            }),
        );
    }

    Ok(())
}
