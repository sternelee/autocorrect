mod clipboard;
mod commands;
mod cspell;
mod hotkey;
mod macos_text;
mod popup;
mod text_selection;
mod typocheck;

use commands::config::{get_config, get_default_config, get_rules, update_config};
use commands::custom_corrections::{
    add_custom_correction, delete_custom_correction, get_custom_corrections,
    get_custom_corrections_path_cmd, update_custom_correction,
};
use commands::default::{read, write};
use commands::hotkey_config::{
    get_available_keys, get_hotkey_config, reset_hotkey_config, update_hotkey_config,
};
use commands::spellcheck::{
    get_clipboard_text, load_config, save_config, set_clipboard_text, spell_check,
};
use hotkey::HotkeyEvent;
use popup::SharedPopupState;
use std::sync::mpsc::TryRecvError;
use std::thread;
use tauri::{Emitter, Manager};
use text_selection::get_cursor_position;

// Import popup commands for the invoke handler
use popup::{
    accept_suggestion, get_popup_state, hide_popup, position_popup, reject_suggestion, show_popup,
    trigger_spell_check_workflow,
};

#[allow(clippy::missing_panics_doc)]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Create channel for clipboard events (hotkey channel is created below)
    let (_clipboard_tx, clipboard_rx) = std::sync::mpsc::channel();

    tauri::Builder::default()
        .setup(move |app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            // Initialize popup state
            app.manage(SharedPopupState::new());

            // Initialize hotkey listener with saved config (or default)
            let hotkey_config = commands::hotkey_config::load_hotkey_config();
            log::info!(
                "Loading hotkey config: {}",
                hotkey_config.to_display_string()
            );
            let (hotkey_rx, hotkey_handle) = hotkey::create_hotkey_channel(hotkey_config);

            log::info!("Global hotkey listener started");

            // Store the hotkey handle in the app state for cleanup
            app.manage(hotkey_handle);

            // Initialize clipboard monitor (disabled by default)
            // User can enable it via Tauri command
            log::info!("Clipboard monitor initialized (paused)");

            // Spawn thread to handle hotkey events and trigger spell check workflow
            let app_handle = app.handle().clone();
            thread::spawn(move || {
                loop {
                    match hotkey_rx.try_recv() {
                        Ok(HotkeyEvent::SpellCheckTriggered) => {
                            log::info!("Hotkey triggered, starting spell check workflow");

                            // Catch any panics to prevent app crashes
                            let result =
                                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                    // Get cursor position
                                    let (x, y) = get_cursor_position();
                                    log::info!("Got cursor position: ({}, {})", x, y);

                                    // Trigger the full spell check workflow
                                    let app_handle_clone = app_handle.clone();
                                    popup::trigger_spell_check_workflow(app_handle_clone, x, y)
                                }));

                            match result {
                                Ok(Ok(())) => {}
                                Ok(Err(e)) => {
                                    log::error!("Spell check workflow failed: {}", e);
                                }
                                Err(panic_info) => {
                                    let panic_msg =
                                        if let Some(s) = panic_info.downcast_ref::<String>() {
                                            s.clone()
                                        } else if let Some(s) = panic_info.downcast_ref::<&str>() {
                                            s.to_string()
                                        } else {
                                            "Unknown panic".to_string()
                                        };
                                    log::error!("Spell check workflow panicked: {}", panic_msg);
                                }
                            }
                        }
                        Err(TryRecvError::Empty) => {
                            thread::sleep(std::time::Duration::from_millis(100));
                        }
                        Err(TryRecvError::Disconnected) => {
                            log::warn!("Hotkey channel disconnected");
                            break;
                        }
                    }
                }
            });

            // Spawn thread to handle clipboard events
            let app_handle_for_clipboard = app.handle().clone();
            thread::spawn(move || {
                loop {
                    match clipboard_rx.try_recv() {
                        Ok(event) => {
                            match event {
                                clipboard::ClipboardEvent::NewText { text, has_cjk } => {
                                    log::info!(
                                        "Clipboard changed: {} chars, CJK: {}, emitting event",
                                        text.chars().count(),
                                        has_cjk
                                    );
                                    // Emit event to frontend with the text data
                                    if let Err(e) = app_handle_for_clipboard.emit(
                                        "clipboard-changed",
                                        serde_json::json!({
                                            "text": text,
                                            "has_cjk": has_cjk
                                        }),
                                    ) {
                                        log::error!("Failed to emit clipboard event: {}", e);
                                    }
                                }
                            }
                        }
                        Err(TryRecvError::Empty) => {
                            thread::sleep(std::time::Duration::from_millis(100));
                        }
                        Err(TryRecvError::Disconnected) => {
                            log::warn!("Clipboard channel disconnected");
                            break;
                        }
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            read,
            write,
            spell_check,
            get_clipboard_text,
            set_clipboard_text,
            load_config,
            save_config,
            get_config,
            get_default_config,
            get_rules,
            update_config,
            start_clipboard_monitor,
            stop_clipboard_monitor,
            get_cursor_pos_cmd,
            // Accessibility permission commands
            check_accessibility_permission,
            request_accessibility_permission,
            // Hotkey config commands
            get_hotkey_config,
            update_hotkey_config,
            reset_hotkey_config,
            get_available_keys,
            // Custom corrections commands
            get_custom_corrections,
            add_custom_correction,
            update_custom_correction,
            delete_custom_correction,
            get_custom_corrections_path_cmd,
            // Popup commands
            show_popup,
            hide_popup,
            position_popup,
            get_popup_state,
            accept_suggestion,
            reject_suggestion,
            trigger_spell_check_workflow,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Tauri command to get the current cursor position
#[tauri::command]
fn get_cursor_pos_cmd() -> Result<(i32, i32), String> {
    Ok(get_cursor_position())
}

/// Tauri command to start clipboard monitoring
#[tauri::command]
fn start_clipboard_monitor(
    window: tauri::Window,
    cjk_only: Option<bool>,
    poll_interval_ms: Option<u64>,
) -> Result<(), String> {
    let config = clipboard::ClipboardMonitorConfig {
        cjk_only: cjk_only.unwrap_or(true),
        poll_interval_ms: poll_interval_ms.unwrap_or(500),
        ..Default::default()
    };

    log::info!("Starting clipboard monitor with config: {:?}", config);

    // Note: In a full implementation, we would store the monitor handle
    // in a global state manager to allow stopping it later
    // For now, this is a placeholder for the functionality

    let _ = window.emit("clipboard-monitor-started", ());

    Ok(())
}

/// Tauri command to stop clipboard monitoring
#[tauri::command]
fn stop_clipboard_monitor(window: tauri::Window) -> Result<(), String> {
    log::info!("Stopping clipboard monitor");

    // Note: In a full implementation, we would stop the stored monitor
    // For now, this is a placeholder for the functionality

    let _ = window.emit("clipboard-monitor-stopped", ());

    Ok(())
}

/// Tauri command to check Accessibility permission status
#[tauri::command]
fn check_accessibility_permission() -> bool {
    #[cfg(target_os = "macos")]
    {
        use macos_text::check_accessibility_permission;
        check_accessibility_permission()
    }

    #[cfg(not(target_os = "macos"))]
    {
        true // Always true on other platforms
    }
}

/// Tauri command to request Accessibility permissions (shows system prompt)
#[tauri::command]
fn request_accessibility_permission() -> bool {
    #[cfg(target_os = "macos")]
    {
        use macos_text::request_accessibility_permission;
        request_accessibility_permission()
    }

    #[cfg(not(target_os = "macos"))]
    {
        true // Always true on other platforms
    }
}
