#![allow(unexpected_cfgs)]

use crate::commands::errors::Error;
use crate::commands::spellcheck::TypoSuggestion;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};

/// Guard so overlapping accept/undo choreographies cannot interleave
/// (e.g. Enter + click firing twice, or undo racing an accept).
static REPLACEMENT_IN_FLIGHT: AtomicBool = AtomicBool::new(false);

/// Guard so a slow spell-check workflow drops later hotkey triggers
/// instead of queueing ghost popups that fire after the first one.
static WORKFLOW_IN_FLIGHT: AtomicBool = AtomicBool::new(false);

/// Popup state shared across the application
#[derive(Debug, Clone)]
pub struct PopupState {
    pub is_visible: bool,
    pub position: (i32, i32),
    pub original_text: String,
    pub suggestion: String,
    pub source_app_name: Option<String>,
    pub source_bundle_id: Option<String>,
}

impl PopupState {
    pub fn new() -> Self {
        Self {
            is_visible: false,
            position: (0, 0),
            original_text: String::new(),
            suggestion: String::new(),
            source_app_name: None,
            source_bundle_id: None,
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

/// Snapshot of the last auto-applied replacement so the user can undo one step.
#[derive(Clone, Debug)]
pub struct LastReplacement {
    pub original_word: String,
    pub replacement: String,
    pub offset: usize,
    pub char_length: usize,
    pub source_app_name: Option<String>,
    pub source_bundle_id: Option<String>,
}

static LAST_REPLACEMENT: std::sync::Mutex<Option<LastReplacement>> = std::sync::Mutex::new(None);

/// Push a replacement snapshot (only the most recent is kept).
fn push_last_replacement(entry: LastReplacement) {
    if let Ok(mut guard) = LAST_REPLACEMENT.lock() {
        *guard = Some(entry);
    }
}

/// Peek the last replacement without removing it.
fn peek_last_replacement() -> Option<LastReplacement> {
    LAST_REPLACEMENT.lock().ok().and_then(|g| g.clone())
}

/// Clear the undo slot (e.g. after a successful undo).
fn clear_last_replacement() {
    if let Ok(mut guard) = LAST_REPLACEMENT.lock() {
        *guard = None;
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
            let mut state = state
                .0
                .lock()
                .map_err(|_| Error::Io(std::io::Error::other("Failed to lock popup state")))?;
            state.is_visible = true;
            state.position = (x, y);
            state.original_text = original_text.clone();
            state.suggestion = suggestion.clone();
            #[cfg(target_os = "macos")]
            {
                // Single fast NSWorkspace call (no osascript subprocess, which
                // added 100-300ms latency to every popup show).
                if let Some((name, bundle_id)) = frontmost_app_info_nsworkspace() {
                    state.source_app_name = Some(name);
                    state.source_bundle_id = Some(bundle_id);
                }
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
                        let app_class =
                            AnyClass::get("NSApplication").expect("NSApplication not found");
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
            let mut state = state
                .0
                .lock()
                .map_err(|_| Error::Io(std::io::Error::other("Failed to lock popup state")))?;
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
            let mut state = state
                .0
                .lock()
                .map_err(|_| Error::Io(std::io::Error::other("Failed to lock popup state")))?;
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
    let state = state
        .0
        .lock()
        .map_err(|_| Error::Io(std::io::Error::other("Failed to lock popup state")))?;

    Ok(serde_json::json!({
        "isVisible": state.is_visible,
        "x": state.position.0,
        "y": state.position.1,
        "originalText": state.original_text,
        "suggestion": state.suggestion,
        "sourceAppName": state.source_app_name,
        "sourceBundleId": state.source_bundle_id
    }))
}

/// Accept the suggestion and apply to the currently selected text.
///
/// The macOS focus-return + paste choreography involves sleeps and blocking
/// subprocess calls that can take over a second. This command is `async` and
/// runs that work on a blocking worker thread so the main thread (NSWindow
/// event processing, other commands) stays responsive.
#[tauri::command]
pub async fn accept_suggestion(
    app: AppHandle,
    text: String,
    offset: Option<usize>,
    char_length: Option<usize>,
) -> Result<(), Error> {
    // Drop duplicate accepts (Enter + click firing twice) instead of running
    // two paste choreographies back to back.
    if REPLACEMENT_IN_FLIGHT
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        log::warn!("accept_suggestion already in progress, ignoring duplicate");
        return Ok(());
    }

    let result = match tauri::async_runtime::spawn_blocking(move || {
        accept_suggestion_blocking(app, text, offset, char_length)
    })
    .await
    {
        Ok(result) => result,
        Err(e) => Err(Error::InputSimulation(format!(
            "accept task join failed: {e}"
        ))),
    };

    REPLACEMENT_IN_FLIGHT.store(false, Ordering::SeqCst);
    result
}

fn accept_suggestion_blocking(
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
    let (source_app_name, source_bundle_id) = app
        .try_state::<SharedPopupState>()
        .and_then(|state| {
            state
                .0
                .lock()
                .ok()
                .map(|s| (s.source_app_name.clone(), s.source_bundle_id.clone()))
        })
        .unwrap_or((None, None));

    // Hide popup so focus can return to the source app.
    hide_popup(app.clone())?;
    thread::sleep(Duration::from_millis(80));

    if !source_is_self_macos(&app, &source_app_name, &source_bundle_id) {
        // Bring the source app back to the front. Prefer the exact bundle id
        // (single NSRunningApplication call); fall back to the name-based
        // osascript for state captured before bundle ids were tracked.
        activate_source_app_macos(source_bundle_id.as_deref(), source_app_name.as_deref())?;

        // Wait until the source app is actually frontmost (instant ObjC call,
        // no subprocess overhead). On timeout we ABORT instead of pasting:
        // pasting into whatever app happens to be frontmost would corrupt
        // the wrong app's text.
        wait_source_frontmost_macos(
            source_bundle_id.as_deref(),
            source_app_name.as_deref(),
            600,
            "[accept]",
        )?;
        // Continue once the AX focused-element state is actually ready
        // instead of guessing a fixed settle delay.
        crate::macos_text::wait_focused_element_ready(600);
    }

    // Now that the source app has focus, select the exact typo range so the
    // replacement covers the word rather than inserting at the cursor.
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
                            "[accept] select_text_range timed out, replacement will insert at caret"
                        );
                        break;
                    }
                    thread::sleep(Duration::from_millis(60));
                }
            }
        }
    }

    // Capture original word before replacing (selection is already active).
    let original_word = crate::macos_text::get_selected_text().unwrap_or_default();

    // Primary replacement path: write AXSelectedText directly. No clipboard
    // roundtrip, no simulated keystrokes, no clipboard to restore.
    if let Err(e) = crate::macos_text::set_selected_text(text) {
        // Fallback: clipboard + simulated ⌘V paste for apps whose text views
        // do not accept AXSelectedText writes.
        log::warn!(
            "[accept] AXSelectedText write failed ({}), falling back to clipboard paste",
            e
        );

        let mut clipboard = arboard::Clipboard::new()
            .map_err(|e| Error::Clipboard(format!("Failed to access clipboard: {e}")))?;
        let previous_clipboard = clipboard.get_text().ok();

        clipboard
            .set_text(text.to_string())
            .map_err(|e| Error::Clipboard(format!("Failed to set clipboard text: {e}")))?;
        // Snapshot the changeCount AFTER our write; restore only if the user
        // did not copy something else in the meantime.
        let our_change_count = crate::macos_text::pasteboard_change_count();

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
        if crate::macos_text::pasteboard_change_count() == our_change_count {
            restore_clipboard(&mut clipboard, previous_clipboard);
        } else {
            log::info!("[accept] pasteboard changed by someone else, keeping current clipboard");
        }
    } else {
        log::info!("[accept] replaced selection via AXSelectedText");
    }

    // Save undo snapshot.
    if let (Some(start), Some(len)) = (offset, char_length) {
        push_last_replacement(LastReplacement {
            original_word,
            replacement: text.to_string(),
            offset: start,
            char_length: len,
            source_app_name: source_app_name.clone(),
            source_bundle_id: source_bundle_id.clone(),
        });
        log::info!(
            "[accept] saved undo: offset={} len={} replacement='{}'",
            start,
            len,
            text
        );

        // Restore caret to end of replaced text so the user can keep typing.
        let replacement_utf16_len = text.encode_utf16().count();
        let caret_pos = start + replacement_utf16_len;
        if let Err(e) = crate::macos_text::select_text_range(caret_pos, 0) {
            log::warn!("[accept] failed to restore caret position: {}", e);
        }
    }

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

/// Fast NSWorkspace-based frontmost app info: `(localizedName, bundleIdentifier)`.
/// No subprocess — replaces the two osascript calls that added 100-300ms to
/// every popup show.
#[cfg(target_os = "macos")]
pub fn frontmost_app_info_nsworkspace() -> Option<(String, String)> {
    use objc2::msg_send;
    use objc2::runtime::AnyClass;

    type Id = *mut objc2::runtime::AnyObject;

    unsafe {
        let workspace_class = AnyClass::get("NSWorkspace").expect("NSWorkspace not found");
        let workspace: Id = msg_send![workspace_class, sharedWorkspace];
        let front_app: Id = msg_send![workspace, frontmostApplication];
        if front_app.is_null() {
            return None;
        }

        let ns_to_string = |obj: Id| -> Option<String> {
            if obj.is_null() {
                return None;
            }
            let utf8: *const std::os::raw::c_char = msg_send![obj, UTF8String];
            if utf8.is_null() {
                return None;
            }
            Some(
                std::ffi::CStr::from_ptr(utf8)
                    .to_string_lossy()
                    .into_owned(),
            )
        };

        let name: Id = msg_send![front_app, localizedName];
        let bundle: Id = msg_send![front_app, bundleIdentifier];
        let name = ns_to_string(name)?;
        let bundle = ns_to_string(bundle)?;
        Some((name, bundle))
    }
}

/// Exact bundle-id frontmost check (no substring matching).
#[cfg(target_os = "macos")]
pub fn is_bundle_frontmost_macos(bundle_id: &str) -> bool {
    frontmost_app_info_nsworkspace().is_some_and(|(_, b)| b == bundle_id)
}

/// Activate the running app with the given bundle id via NSRunningApplication.
/// Returns false when no running app matches (caller falls back to name-based
/// activation).
#[cfg(target_os = "macos")]
pub fn activate_app_bundle_macos(bundle_id: &str) -> bool {
    use objc2::msg_send;
    use objc2::runtime::AnyClass;

    type Id = *mut objc2::runtime::AnyObject;

    let c_bundle = match std::ffi::CString::new(bundle_id) {
        Ok(c) => c,
        Err(_) => return false,
    };

    unsafe {
        let nsstring_class = AnyClass::get("NSString").expect("NSString not found");
        let ns_bundle: Id = msg_send![nsstring_class, stringWithUTF8String: c_bundle.as_ptr()];
        let running_class =
            AnyClass::get("NSRunningApplication").expect("NSRunningApplication not found");
        let app: Id = msg_send![running_class, runningApplicationWithBundleIdentifier: ns_bundle];
        if app.is_null() {
            return false;
        }
        // NSApplicationActivateIgnoringOtherApps
        let ok: bool = msg_send![app, activateWithOptions: 4u64];
        ok
    }
}

/// True when the recorded source app is AutoCorrect itself, in which case we
/// skip the activate/frontmost dance (the popup was triggered from our own
/// window, e.g. the spell checker tab).
#[cfg(target_os = "macos")]
fn source_is_self_macos(
    app: &AppHandle,
    source_app_name: &Option<String>,
    source_bundle_id: &Option<String>,
) -> bool {
    let name_is_self = source_app_name
        .as_deref()
        .is_some_and(|n| n == "autocorrect-app" || n == "AutoCorrect");
    let self_bundle = app.config().identifier.clone();
    let bundle_is_self = source_bundle_id
        .as_deref()
        .is_some_and(|b| !b.is_empty() && b == self_bundle);
    name_is_self || bundle_is_self
}

/// Activate the source app, preferring the exact bundle id over the
/// name-based osascript fallback.
#[cfg(target_os = "macos")]
fn activate_source_app_macos(
    source_bundle_id: Option<&str>,
    source_app_name: Option<&str>,
) -> Result<(), Error> {
    if let Some(bundle) = source_bundle_id {
        if !bundle.is_empty() && activate_app_bundle_macos(bundle) {
            return Ok(());
        }
    }
    if let Some(name) = source_app_name {
        if !name.is_empty() {
            return activate_app_macos(name);
        }
    }
    Ok(())
}

/// Poll until the source app is frontmost (exact bundle-id match when
/// available, name-based otherwise). Returns `Err` on timeout so the caller
/// can abort instead of pasting into the wrong app.
#[cfg(target_os = "macos")]
fn wait_source_frontmost_macos(
    source_bundle_id: Option<&str>,
    source_app_name: Option<&str>,
    timeout_ms: u128,
    context: &str,
) -> Result<(), Error> {
    let is_frontmost = |bundle: Option<&str>, name: Option<&str>| -> bool {
        if let Some(b) = bundle.filter(|b| !b.is_empty()) {
            return is_bundle_frontmost_macos(b);
        }
        if let Some(n) = name {
            return is_app_frontmost_macos_pub(n);
        }
        // Nothing known about the source app; assume we can proceed.
        true
    };

    let deadline = std::time::Instant::now();
    loop {
        thread::sleep(Duration::from_millis(30));
        if is_frontmost(source_bundle_id, source_app_name) {
            return Ok(());
        }
        if deadline.elapsed().as_millis() > timeout_ms {
            log::warn!(
                "{context}: source app still not frontmost after {}ms, aborting",
                timeout_ms
            );
            return Err(Error::InputSimulation(
                "Source app is not frontmost; paste aborted".to_string(),
            ));
        }
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
pub fn get_frontmost_app_bundle_id_macos() -> Option<String> {
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

/// Undo the last auto-applied replacement by reverting the text at the
/// recorded offset. Only the most recent replacement is remembered.
///
/// Like `accept_suggestion`, the choreography runs on a blocking worker
/// thread so the main thread stays responsive.
#[tauri::command]
pub async fn undo_last_replacement(app: AppHandle) -> Result<(), Error> {
    if REPLACEMENT_IN_FLIGHT
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        log::warn!("A replacement operation is already in progress, ignoring undo");
        return Ok(());
    }

    let result =
        match tauri::async_runtime::spawn_blocking(move || undo_last_replacement_blocking(app))
            .await
        {
            Ok(result) => result,
            Err(e) => Err(Error::InputSimulation(format!(
                "undo task join failed: {e}"
            ))),
        };

    REPLACEMENT_IN_FLIGHT.store(false, Ordering::SeqCst);
    result
}

fn undo_last_replacement_blocking(app: AppHandle) -> Result<(), Error> {
    let entry = peek_last_replacement()
        .ok_or_else(|| Error::InputSimulation("No recent replacement to undo".to_string()))?;

    #[cfg(target_os = "macos")]
    {
        if !source_is_self_macos(&app, &entry.source_app_name, &entry.source_bundle_id) {
            activate_source_app_macos(
                entry.source_bundle_id.as_deref(),
                entry.source_app_name.as_deref(),
            )?;
            // Abort on timeout: reverting text into the wrong app would
            // silently corrupt it.
            wait_source_frontmost_macos(
                entry.source_bundle_id.as_deref(),
                entry.source_app_name.as_deref(),
                600,
                "[undo]",
            )?;
            // Continue once the AX focused-element state is actually ready.
            crate::macos_text::wait_focused_element_ready(600);
        }

        let replacement_utf16_len = entry.replacement.encode_utf16().count();
        let undo_offset = entry.offset;
        let undo_len = replacement_utf16_len;
        let sel_deadline = std::time::Instant::now();
        loop {
            match crate::macos_text::select_text_range(undo_offset, undo_len) {
                Ok(()) => {
                    thread::sleep(Duration::from_millis(50));
                    break;
                }
                Err(e) => {
                    log::warn!("[undo] select_text_range failed: {}", e);
                    if sel_deadline.elapsed().as_millis() > 600 {
                        log::warn!("[undo] select_text_range timed out");
                        break;
                    }
                    thread::sleep(Duration::from_millis(60));
                }
            }
        }

        // Primary path: write AXSelectedText directly, no clipboard involved.
        if let Err(e) = crate::macos_text::set_selected_text(&entry.original_word) {
            log::warn!(
                "[undo] AXSelectedText write failed ({}), falling back to clipboard paste",
                e
            );

            let mut clipboard = arboard::Clipboard::new()
                .map_err(|e| Error::Clipboard(format!("Clipboard init failed: {e}")))?;
            let previous_clipboard = clipboard.get_text().ok();

            clipboard
                .set_text(entry.original_word.clone())
                .map_err(|e| Error::Clipboard(format!("Clipboard write failed: {e}")))?;
            let our_change_count = crate::macos_text::pasteboard_change_count();

            let status = std::process::Command::new("osascript")
                .arg("-e")
                .arg("tell application \"System Events\" to keystroke \"v\" using command down")
                .status()
                .map_err(|e| Error::InputSimulation(format!("Paste simulation failed: {e}")))?;

            if !status.success() {
                restore_clipboard(&mut clipboard, previous_clipboard);
                return Err(Error::InputSimulation("Undo paste failed".to_string()));
            }

            thread::sleep(Duration::from_millis(80));
            if crate::macos_text::pasteboard_change_count() == our_change_count {
                restore_clipboard(&mut clipboard, previous_clipboard);
            } else {
                log::info!("[undo] pasteboard changed by someone else, keeping it");
            }
        } else {
            log::info!("[undo] reverted text via AXSelectedText");
        }

        // Restore caret to end of reverted text.
        let original_utf16_len = entry.original_word.encode_utf16().count();
        let caret_pos = undo_offset + original_utf16_len;
        if let Err(e) = crate::macos_text::select_text_range(caret_pos, 0) {
            log::warn!("[undo] failed to restore caret: {}", e);
        }
    }

    clear_last_replacement();
    let _ = app.emit(
        "suggestion-undone",
        serde_json::json!({
            "original": entry.original_word,
            "replacement": entry.replacement,
        }),
    );
    Ok(())
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
///
/// Re-entrant triggers (hotkey pressed while a previous run is still in
/// flight, e.g. a slow AI-enabled check) are dropped instead of queueing
/// ghost popups that would appear after the first one completes.
#[tauri::command]
pub fn trigger_spell_check_workflow(app: AppHandle, x: i32, y: i32) -> Result<(), Error> {
    if WORKFLOW_IN_FLIGHT
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        log::info!("Spell check workflow already in progress, dropping trigger");
        return Ok(());
    }
    let result = trigger_spell_check_workflow_inner(app, x, y);
    WORKFLOW_IN_FLIGHT.store(false, Ordering::SeqCst);
    result
}

fn trigger_spell_check_workflow_inner(app: AppHandle, x: i32, y: i32) -> Result<(), Error> {
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
        // Small offset from cursor so popup doesn't cover the selection
        let offset_x = 5;
        let offset_y = 5;
        show_popup(
            app,
            x + offset_x,
            y + offset_y,
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
