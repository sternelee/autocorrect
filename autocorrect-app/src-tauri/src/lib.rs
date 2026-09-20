mod ai_popup;
mod ax_observer;
mod clipboard;
mod commands;
mod grammar;
mod hotkey;
mod macos_text;
mod objc2_compat;
mod overlay;
mod popup;
mod system_typo_sync;
mod text_selection;
mod text_utils;
mod theme;
mod theme_errors;
mod translation;
mod typocheck;

// Timing constants (ms)
const SYNC_INTERVAL_MS: u64 = 800;
const HOVER_POLL_INTERVAL_MS: u64 = 100;
const HOVER_DELAY_MS: u128 = 300;
const CLIPBOARD_POLL_DEFAULT_MS: u64 = 500;

// Popup dimensions
const POPUP_WIDTH: f64 = 300.0;
const POPUP_HEIGHT: f64 = 120.0;
const POPUP_OFFSET: f64 = 10.0;

#[cfg(target_os = "macos")]
mod macos_geom;
#[cfg(target_os = "macos")]
use macos_geom::*;

use ai_popup::{SharedAiPopupState, SharedNativeIconWindow};
use commands::ai_grammar::{
    ai_assist, ai_clarity_check, ai_clarity_check_stream, ai_grammar_check, ai_polish_batch,
    ai_text_transform, ai_text_transform_stream, ai_tone_detect, ai_vocabulary_enhance,
};
use commands::config::{
    ensure_app_settings_initialized, get_config, get_default_config, get_polish_styles, get_rules,
    update_config,
};
use commands::custom_corrections::{
    add_custom_correction, delete_custom_correction, get_custom_corrections,
    get_custom_corrections_path_cmd, update_custom_correction,
};
use commands::default::{read, write};
use commands::hotkey_config::{
    get_available_keys, get_hotkey_config, reset_hotkey_config, update_hotkey_config,
};
use commands::ignored_apps::{
    add_ignored_app, get_frontmost_app_info, get_ignored_apps, is_app_ignored, remove_ignored_app,
    update_ignored_app,
};
use commands::spellcheck::{
    get_clipboard_text, load_config, save_config, set_clipboard_text, spell_check,
};
use commands::translate::{
    download_translation_model, get_translation_model_status, list_downloadable_translation_models,
    list_translation_providers, translate_text,
};
use hotkey::{HotkeyEvent, HotkeyHandle};
use overlay::OverlayManager;
use popup::SharedPopupState;
use serde::Serialize;
use std::sync::mpsc::TryRecvError;
use std::thread;
use tauri::{Emitter, Listener, Manager};
use text_selection::get_cursor_position;
use theme::{get_theme, set_theme};

// Import popup commands for the invoke handler
use popup::{
    accept_suggestion, get_popup_state, hide_popup, position_popup, reject_suggestion, show_popup,
    trigger_spell_check_workflow, undo_last_replacement,
};

/// Shared state for the clipboard monitor so start/stop commands can manage
/// its lifecycle without creating dangling threads.
#[derive(Default)]
pub struct ClipboardMonitorState(pub std::sync::Mutex<Option<clipboard::ClipboardHandle>>);

#[allow(clippy::missing_panics_doc)]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(tauri_plugin_store::Builder::new().build())
        .setup(move |app| {
            app.handle().plugin(tauri_plugin_http::init())?;

            // Enable logging in both debug and release builds so issues in the
            // packaged app can be diagnosed.
            app.handle().plugin(
                tauri_plugin_log::Builder::default()
                    .level(log::LevelFilter::Info)
                    .build(),
            )?;

            // Check Accessibility permission on startup.
            // We only LOG here — we intentionally do NOT open System Settings
            // automatically because:
            //   1. AXIsProcessTrusted() can transiently return false on the
            //      first call even when permission is already granted (unsigned
            //      builds, sandbox init delay).
            //   2. Auto-opening System Settings on every launch is disruptive
            //      and confusing for users who have already granted access.
            // The frontend polls check_accessibility_permission() and surfaces
            // a guided prompt with a "Open Settings" button when needed.
            #[cfg(target_os = "macos")]
            {
                if !macos_text::check_accessibility_trusted() {
                    log::warn!(
                        "Accessibility permission not yet granted (or not yet visible \
                         to this process). Spell-check overlay will be inactive until \
                         access is enabled in System Settings → Privacy → Accessibility."
                    );
                } else {
                    log::info!("Accessibility permission granted ✓");
                }
            }

            ensure_app_settings_initialized(app.handle())?;

            // Initialize popup state
            app.manage(SharedPopupState::new());
            app.manage(SharedAiPopupState::new());
            #[cfg(target_os = "macos")]
            app.manage(SharedNativeIconWindow::new());

            // Diagnostic bridge: the ai-popup webview reports its adaptive
            // height-fit steps so sizing issues are visible in the app log.
            {
                let handle = app.handle().clone();
                let _ = handle.listen("ai-popup-fit-diag", move |event| {
                    log::info!("[AIPOP-FIT] {}", event.payload());
                });
            }

            // Initialize Overlay Manager
            let overlay_manager = OverlayManager::new(app.handle().clone());
            let _ = overlay_manager.get_or_create_overlay();
            app.manage(overlay_manager);

            // Initialize hotkey listener with saved config (or default)
            let hotkey_config = commands::hotkey_config::load_hotkey_config();
            log::info!(
                "Loading hotkey config: {}",
                hotkey_config.to_display_string()
            );
            // Wrap the active binding in a shared cell so update / reset
            // commands can swap it without restarting the rdev listener.
            let shared_hotkey_config = hotkey::shared_hotkey_config(hotkey_config);
            let (hotkey_rx, hotkey_handle) =
                hotkey::create_hotkey_channel(shared_hotkey_config.clone());

            log::info!("Global hotkey listener started");

            // Store the hotkey handle for cleanup and a writeable handle to
            // the current binding so commands can mutate it at runtime.
            app.manage(hotkey_handle);
            app.manage(hotkey::HotkeyConfigCell(shared_hotkey_config));

            // Initialize clipboard monitor state (empty until started by command)
            app.manage(ClipboardMonitorState::default());

            // 核心：启动系统级下划线同步循环
            // Event-driven: an AXObserver bridge signals text changes so the
            // sync runs within ~150ms of a keystroke; the adaptive interval
            // below is only the fallback ceiling for apps that emit no
            // notifications.
            let (ax_event_tx, ax_event_rx) = std::sync::mpsc::channel::<()>();
            #[cfg(target_os = "macos")]
            ax_observer::spawn_text_event_bridge(ax_event_tx);
            #[cfg(not(target_os = "macos"))]
            drop(ax_event_tx);

            let app_handle_for_sync = app.handle().clone();
            thread::spawn(move || {
                let mut poll_interval = std::time::Duration::from_millis(SYNC_INTERVAL_MS);
                loop {
                    let start = std::time::Instant::now();
                    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        #[cfg(target_os = "macos")]
                        unsafe {
                            use objc2::msg_send;
                            use objc2::runtime::AnyClass;
                            type Id = *mut objc2::runtime::AnyObject;
                            let pool_class = AnyClass::get("NSAutoreleasePool").expect("NSAutoreleasePool not found");
                            let pool: Id = msg_send![pool_class, new];
                            system_typo_sync::sync_system_typos(&app_handle_for_sync);
                            let _: () = msg_send![pool, drain];
                        }
                        #[cfg(not(target_os = "macos"))]
                        system_typo_sync::sync_system_typos(&app_handle_for_sync);
                    }));

                    // Adaptive fallback ceiling: shorten when the user is
                    // actively typing or clicking, so apps that emit no AX
                    // notifications still get near-real-time underlines.
                    #[cfg(target_os = "macos")]
                    {
                        let since_key = macos_text::seconds_since_last_keydown();
                        let since_mouse = macos_text::seconds_since_last_mouse_down();
                        poll_interval = if since_key < 0.5 || since_mouse < 0.5 {
                            std::time::Duration::from_millis(100)
                        } else {
                            std::time::Duration::from_millis(SYNC_INTERVAL_MS)
                        };
                    }
                    #[cfg(not(target_os = "macos"))]
                    {
                        poll_interval = std::time::Duration::from_millis(SYNC_INTERVAL_MS);
                    }

                    let elapsed = start.elapsed();
                    if elapsed < poll_interval {
                        // Wake early on AX text events (macOS); block for the
                        // fallback interval otherwise.
                        let _ = ax_event_rx.recv_timeout(poll_interval - elapsed);
                        // Debounce a rapid keystroke burst: events during the
                        // window coalesce into one sync.
                        thread::sleep(std::time::Duration::from_millis(120));
                        while ax_event_rx.try_recv().is_ok() {}
                    }
                }
            });

            // Spawn thread to monitor mouse hover over typo markers
            let app_handle_for_hover = app.handle().clone();
            thread::spawn(move || {
                let mut last_hovered_id = String::new();
                let mut hover_start: Option<std::time::Instant> = None;
                // Track previous left-button state to detect leading edge of a click.
                let mut prev_left_down = false;

                loop {
                    thread::sleep(std::time::Duration::from_millis(HOVER_POLL_INTERVAL_MS));

                    let overlay_manager = match app_handle_for_hover.try_state::<OverlayManager>() {
                        Some(m) => m,
                        None => continue,
                    };

                    let (mouse_x, mouse_y) = text_selection::get_cursor_position();
                    let mouse_x_f = mouse_x as f64;
                    let mouse_y_f = mouse_y as f64;

                    // Dismiss popup on click outside its bounds.
                    // Also detect hover over the native AI icon.
                    #[cfg(target_os = "macos")]
                    {
                        let left_down = is_left_button_down();
                        if left_down && !prev_left_down {
                            // New left-button press – check whether it lands outside the popup.
                            if let Some(popup_state) = app_handle_for_hover.try_state::<crate::popup::SharedPopupState>() {
                                if let Ok(state) = popup_state.0.lock() {
                                    if state.is_visible {
                                        let (px, py) = state.position;
                                        let popup_w = POPUP_WIDTH;
                                        let popup_h = POPUP_HEIGHT;
                                        let outside = mouse_x_f < px as f64
                                            || mouse_x_f > px as f64 + popup_w
                                            || mouse_y_f < py as f64
                                            || mouse_y_f > py as f64 + popup_h;
                                        if outside {
                                            drop(state);
                                            let _ = crate::popup::hide_popup(app_handle_for_hover.clone());
                                        }
                                    }
                                }
                            }
                        }
                        prev_left_down = left_down;

                        // Show AI popup when cursor enters the native icon bounds.
                        if let Some(ai_state) = app_handle_for_hover.try_state::<crate::ai_popup::SharedAiPopupState>() {
                            if let Ok(s) = ai_state.0.lock() {
                                if s.icon_visible && !s.popup_visible {
                                    let (ix, iy) = s.icon_position;
                                    let (iw, ih) = s.icon_size;
                                    let over_icon = mouse_x_f >= ix as f64
                                        && mouse_x_f <= ix as f64 + iw as f64
                                        && mouse_y_f >= iy as f64
                                        && mouse_y_f <= iy as f64 + ih as f64;
                                    if over_icon {
                                        log::info!("[AI] cursor over icon ({},{}) icon_pos=({},{}) size=({},{})", mouse_x, mouse_y, ix, iy, iw, ih);
                                        drop(s);
                                        crate::ai_popup::show_ai_popup_from_hover(&app_handle_for_hover);
                                    }
                                }
                            }
                        }
                    }

                    let markers = {
                        if let Ok(lock) = overlay_manager.current_markers.lock() {
                            lock.clone()
                        } else {
                            continue;
                        }
                    };

                    let mut hovered_marker = None;

                    // Get screen bounds for coordinate conversion (similar to overlay.rs)
                    #[cfg(target_os = "macos")]
                    let (desktop_min_x, _desktop_top_y, desktop_width, _desktop_height) = unsafe {
                        use objc2::msg_send;
                        use objc2::runtime::AnyClass;

                        type Id = *mut objc2::runtime::AnyObject;

                        let mut min_x = f64::MAX;
                        let mut min_y = f64::MAX;
                        let mut max_x = f64::MIN;
                        let mut max_y = f64::MIN;
                        let screens: Id = msg_send![AnyClass::get("NSScreen").expect("NSScreen not found"), screens];
                        // NSScreen.screens returns a Swift Array on modern macOS, which is toll-free bridged to CFArray
                        // Use CFArrayGetCount to safely get the count without type encoding issues
                        let count = core_foundation::array::CFArrayGetCount(screens as core_foundation::array::CFArrayRef);
                        for idx in 0..count {
                            let screen: Id = msg_send![screens, objectAtIndex: idx];
                            let frame: CGRect = msg_send![screen, frame];
                            min_x = min_x.min(frame.origin.x);
                            min_y = min_y.min(frame.origin.y);
                            max_x = max_x.max(frame.origin.x + frame.size.width);
                            max_y = max_y.max(frame.origin.y + frame.size.height);
                        }
                        (
                            min_x,
                            max_y,
                            max_x - min_x,
                            max_y - min_y,
                        )
                    };
                    #[cfg(not(target_os = "macos"))]
                    let (desktop_min_x, desktop_top_y, desktop_width, desktop_height) =
                        (0.0, 1080.0, 1920.0, 1080.0);

                    for marker in markers {
                        // All coordinates (marker and mouse) are in the top-left
                        // system: y=0 at top of primary screen, increases downward.
                        //
                        // Native markers: marker.y = top of char bounding box (AX).
                        //   Underline is at marker.y + marker.height (bottom edge).
                        // Fallback markers: marker.y = bottom of text line (lib.rs
                        //   final_y) = underline position; marker.height = 2.0.
                        let is_fallback = marker.id.contains("fallback");
                        let y_underline = if is_fallback {
                            marker.y
                        } else {
                            marker.y + marker.height
                        };
                        // Hit area: extend above the underline to cover the full
                        // character height, plus a small pad on each side.
                        let char_h = if is_fallback { 20.0_f64 } else { marker.height };
                        let hit_top    = y_underline - char_h - 4.0;
                        let hit_bottom = y_underline + 6.0;
                        let hit_left   = marker.x - 4.0;
                        let hit_right  = marker.x + marker.width + 4.0;

                        if mouse_x_f >= hit_left
                            && mouse_x_f <= hit_right
                            && mouse_y_f >= hit_top
                            && mouse_y_f <= hit_bottom
                        {
                            hovered_marker = Some(marker);
                            break;
                        }
                    }

                    if let Some(marker) = hovered_marker {
                        if last_hovered_id != marker.id {
                            last_hovered_id = marker.id.clone();
                            hover_start = Some(std::time::Instant::now());
                        } else if let Some(start) = hover_start {
                            // Show popup after 300ms of hovering
                            if start.elapsed().as_millis() > HOVER_DELAY_MS {
                                // Prepare typo suggestion objects
                                let typo_suggestions = vec![crate::commands::spellcheck::TypoSuggestion {
                                    typo: marker.text.clone(),
                                    suggestions: marker.suggestions.clone(),
                                    line: 0,
                                    col: 0,
                                }];

                                let suggestion_text = marker.suggestions.first().cloned().unwrap_or_default();

                                // Position popup from mouse cursor - offset to place corner at cursor
                                let popup_width = POPUP_WIDTH;
                                let popup_height = POPUP_HEIGHT;
                                let offset_x = POPUP_OFFSET;
                                let offset_y = POPUP_OFFSET;

                                // Use the PRIMARY display height in CG coordinates
                                // (y=0 at top, increases down — same system as mouse_x/y).
                                // desktop_height is the virtual desktop total which can be much
                                // larger than the primary screen on multi-monitor setups, making
                                // the clamp below ineffective. CGDisplayBounds gives us the
                                // real primary screen bounds.
                                #[cfg(target_os = "macos")]
                                let primary_h = unsafe {
                                    use core_graphics::display::{CGDisplayBounds, CGMainDisplayID};
                                    CGDisplayBounds(CGMainDisplayID()).size.height
                                };
                                #[cfg(not(target_os = "macos"))]
                                let primary_h = 1080.0_f64;

                                let mut popup_x = mouse_x_f + offset_x;
                                // Prefer below cursor; if it goes off-screen, flip above.
                                let mut popup_y = if mouse_y_f + offset_y + popup_height <= primary_h {
                                    mouse_y_f + offset_y
                                } else {
                                    mouse_y_f - popup_height - offset_y
                                };

                                // Final clamp so the popup stays within primary screen bounds.
                                let min_x = desktop_min_x;
                                let max_x = (desktop_min_x + desktop_width - popup_width).max(min_x);
                                popup_x = popup_x.clamp(min_x, max_x);
                                popup_y = popup_y.clamp(0.0, (primary_h - popup_height).max(0.0));

                                log::info!("[POPUP] hover triggered: mouse=({:.0},{:.0}) primary_h={:.0} -> popup=({:.0},{:.0})",
                                    mouse_x_f, mouse_y_f, primary_h, popup_x, popup_y);

                                let _ = popup::show_popup(
                                    app_handle_for_hover.clone(),
                                    popup_x as i32,
                                    popup_y as i32,
                                    marker.text.clone(),
                                    suggestion_text,
                                    Some(typo_suggestions),
                                    Some(marker.offset),
                                    Some(marker.char_length)
                                );

                                // Prevent re-triggering until we move away
                                hover_start = None;
                            }
                        }
                    } else {
                        // Mouse moved away from the marker
                        if !last_hovered_id.is_empty() {
                            last_hovered_id.clear();
                            hover_start = None;

                            // Let the frontend popup hide itself when mouse leaves its window,
                            // or we could hide it here. We'll let the user interact with the popup.
                        }
                    }
                }
            });

            // Spawn thread to handle hotkey events and trigger spell check workflow
            let app_handle = app.handle().clone();
            thread::spawn(move || {
                loop {
                    match hotkey_rx.try_recv() {
                        Ok(HotkeyEvent::SpellCheckTriggered) => {
                            log::info!("Hotkey triggered, starting spell check workflow");

                            // Check if frontmost app is ignored for popup.
                            // Fast NSWorkspace call — the previous osascript
                            // lookup added 100-300ms to every hotkey trigger.
                            #[cfg(target_os = "macos")]
                            let frontmost_bundle = popup::frontmost_app_info_nsworkspace()
                                .map(|(_, b)| b)
                                .filter(|b| !b.is_empty());
                            #[cfg(not(target_os = "macos"))]
                            let frontmost_bundle =
                                commands::ignored_apps::get_frontmost_bundle_id_macos();

                            let should_trigger = match frontmost_bundle {
                                Some(bundle_id) => !is_app_ignored(&app_handle, &bundle_id, true, false),
                                None => true, // Trigger if we can't get bundle ID
                            };

                            if !should_trigger {
                                log::info!("App is ignored for popup, skipping hotkey trigger");
                                continue;
                            }

                            // With a live text selection, the hotkey opens the
                            // AI tools popup (translate/polish/rewrite) instead
                            // of the spell-check suggestion popup. Uses the
                            // pure-AX selection read — no ⌘C clipboard
                            // fallback — so nothing here can clobber the
                            // user's clipboard.
                            #[cfg(target_os = "macos")]
                            {
                                let selection =
                                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                                        macos_text::get_selected_text,
                                    ))
                                    .ok()
                                    .and_then(|r| r.ok())
                                    .filter(|t| !t.trim().is_empty());

                                if let Some(text) = selection {
                                    let (icon_x, icon_y) =
                                        match macos_text::get_selected_text_bounds() {
                                            Ok((sx, sy, sw, _sh)) => (sx + sw + 4, sy - 18),
                                            Err(_) => {
                                                let (cx, cy) = get_cursor_position();
                                                (cx + 10, cy - 18)
                                            }
                                        };
                                    log::info!(
                                        "[HOTKEY] selection {} chars, AI popup at ({},{})",
                                        text.chars().count(),
                                        icon_x,
                                        icon_y
                                    );
                                    ai_popup::show_ai_icon(&app_handle, icon_x, icon_y, text);
                                    ai_popup::show_ai_popup_from_hover(&app_handle);
                                    continue;
                                }
                            }

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

            Ok(())
        })
        // On macOS, Tauri 2's default behavior when the user clicks the close
        // (red traffic-light) button is to *destroy* the window, which breaks
        // the Dock-click → Show flow. Match native macOS convention by
        // intercepting CloseRequested on the main window: prevent_close +
        // hide. The existing RunEvent::Reopen handler below then restores
        // it when the user clicks the Dock icon.
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    #[cfg(target_os = "macos")]
                    {
                        api.prevent_close();
                        let _ = window.hide();
                    }
                }
            }
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
            get_polish_styles,
            update_config,
            start_clipboard_monitor,
            stop_clipboard_monitor,
            is_clipboard_monitor_running,
            get_system_health,
            get_cursor_pos_cmd,
            // Accessibility permission commands
            check_accessibility_permission,
            request_accessibility_permission,
            // Autostart commands
            get_autostart_enabled,
            set_autostart_enabled,
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
            ai_grammar_check,
            ai_assist,
            ai_text_transform,
            ai_text_transform_stream,
            ai_polish_batch,
            ai_tone_detect,
            ai_clarity_check,
            ai_clarity_check_stream,
            ai_vocabulary_enhance,
            // Ignored apps commands
            get_ignored_apps,
            add_ignored_app,
            update_ignored_app,
            remove_ignored_app,
            get_frontmost_app_info,
            // Popup commands
            show_popup,
            hide_popup,
            position_popup,
            get_popup_state,
            accept_suggestion,
            reject_suggestion,
            undo_last_replacement,
            trigger_spell_check_workflow,
            // AI popup commands
            ai_popup::show_ai_popup,
            ai_popup::hide_ai_popup,
            ai_popup::get_ai_popup_state,
            ai_popup::accept_ai_result,
            // Theme commands
            get_theme,
            set_theme,
            // Translation commands
            list_translation_providers,
            translate_text,
            list_downloadable_translation_models,
            get_translation_model_status,
            download_translation_model,
            // Grammar engine commands
            grammar::grammar_check_text,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            // On macOS, clicking the Dock icon while all windows are hidden fires
            // RunEvent::Reopen. Bring the main window back to the front.
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen {
                has_visible_windows,
                ..
            } = event
            {
                if !has_visible_windows {
                    if let Some(main) = app_handle.get_webview_window("main") {
                        let _ = main.show();
                        let _ = main.set_focus();
                    }
                }
            }
        });
}

/// Tauri command to get the current cursor position
#[tauri::command]
fn get_cursor_pos_cmd() -> Result<(i32, i32), String> {
    Ok(get_cursor_position())
}

/// Tauri command to start clipboard monitoring
#[tauri::command]
fn start_clipboard_monitor(
    app: tauri::AppHandle,
    window: tauri::Window,
    cjk_only: Option<bool>,
    poll_interval_ms: Option<u64>,
) -> Result<(), String> {
    let config = clipboard::ClipboardMonitorConfig {
        cjk_only: cjk_only.unwrap_or(true),
        poll_interval_ms: poll_interval_ms.unwrap_or(CLIPBOARD_POLL_DEFAULT_MS),
        ..Default::default()
    };

    log::info!("Starting clipboard monitor with config: {:?}", config);

    let (rx, handle) = clipboard::create_clipboard_channel(config)
        .map_err(|e| format!("Failed to create clipboard monitor: {}", e))?;

    // Forward clipboard events to the frontend
    let app_handle = app.clone();
    std::thread::spawn(move || loop {
        match rx.recv() {
            Ok(event) => match event {
                clipboard::ClipboardEvent::NewText { text, has_cjk } => {
                    log::info!(
                        "Clipboard changed: {} chars, CJK: {}, emitting event",
                        text.chars().count(),
                        has_cjk
                    );
                    let _ = app_handle.emit(
                        "clipboard-changed",
                        serde_json::json!({
                            "text": text,
                            "has_cjk": has_cjk
                        }),
                    );
                }
            },
            Err(_) => {
                log::info!("Clipboard monitor receiver disconnected");
                break;
            }
        }
    });

    // Atomically stop any existing monitor and store the new one.
    if let Some(state) = app.try_state::<ClipboardMonitorState>() {
        if let Ok(mut guard) = state.0.lock() {
            if let Some(old) = guard.take() {
                old.stop();
            }
            *guard = Some(handle);
        }
    }

    let _ = window.emit("clipboard-monitor-started", ());
    Ok(())
}

/// Tauri command to check clipboard monitor status
#[tauri::command]
fn is_clipboard_monitor_running(app: tauri::AppHandle) -> bool {
    app.try_state::<ClipboardMonitorState>()
        .and_then(|state| state.0.lock().ok().map(|guard| guard.is_some()))
        .unwrap_or(false)
}

/// Runtime health snapshot for the settings dashboard.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SystemHealth {
    accessibility_granted: bool,
    hotkey_running: bool,
    hotkey_binding: String,
    overlay_ready: bool,
    overlay_marker_count: usize,
    clipboard_monitor_running: bool,
    platform: String,
}

/// Tauri command to inspect runtime integrations without reading focused text.
#[tauri::command]
fn get_system_health(app: tauri::AppHandle) -> SystemHealth {
    let accessibility_granted = check_accessibility_permission();
    let hotkey_running = app
        .try_state::<HotkeyHandle>()
        .map(|handle| handle.is_running())
        .unwrap_or(false);
    let hotkey_binding = app
        .try_state::<hotkey::HotkeyConfigCell>()
        .and_then(|cell| cell.0.lock().ok().map(|config| config.to_display_string()))
        .unwrap_or_else(|| "Unavailable".to_string());
    let overlay_state = app.try_state::<OverlayManager>();
    let overlay_ready = overlay_state.is_some();
    let overlay_marker_count = overlay_state
        .and_then(|manager| manager.current_markers.lock().ok().map(|m| m.len()))
        .unwrap_or(0);

    SystemHealth {
        accessibility_granted,
        hotkey_running,
        hotkey_binding,
        overlay_ready,
        overlay_marker_count,
        clipboard_monitor_running: is_clipboard_monitor_running(app),
        platform: std::env::consts::OS.to_string(),
    }
}

/// Tauri command to stop clipboard monitoring
#[tauri::command]
fn stop_clipboard_monitor(app: tauri::AppHandle, window: tauri::Window) -> Result<(), String> {
    log::info!("Stopping clipboard monitor");

    if let Some(state) = app.try_state::<ClipboardMonitorState>() {
        if let Ok(mut guard) = state.0.lock() {
            if let Some(handle) = guard.take() {
                handle.stop();
            }
        }
    }

    let _ = window.emit("clipboard-monitor-stopped", ());
    Ok(())
}

/// Tauri command to check Accessibility permission status
#[tauri::command]
fn check_accessibility_permission() -> bool {
    #[cfg(target_os = "macos")]
    {
        macos_text::check_accessibility_trusted()
    }
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

/// Returns true if the left mouse button is currently held down.
/// Uses NSEvent.pressedMouseButtons (bit 0 = left button).
#[cfg(target_os = "macos")]
fn is_left_button_down() -> bool {
    use objc2::msg_send;
    use objc2::runtime::AnyClass;
    unsafe {
        let event_class = AnyClass::get("NSEvent").expect("NSEvent not found");
        let pressed: usize = msg_send![event_class, pressedMouseButtons];
        pressed & 1 != 0
    }
}

/// Open the Accessibility pane so the user can enable the app, then restart.
#[tauri::command]
fn request_accessibility_permission() -> bool {
    #[cfg(target_os = "macos")]
    {
        if macos_text::check_accessibility_trusted() {
            return true;
        }
        macos_text::open_accessibility_settings();
        false
    }
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn get_auto_launch() -> auto_launch::AutoLaunch {
    let app_name = "AutoCorrect";
    // Get the current executable path
    let exec_path = std::env::current_exe()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    // macOS signature: new(app_name, app_path, hidden, args)
    auto_launch::AutoLaunch::new(app_name, &exec_path, false, &[""] as &[&str])
}

/// Get the current autostart state
#[tauri::command]
fn get_autostart_enabled() -> bool {
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        get_auto_launch().is_enabled().unwrap_or(false)
    }
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        false
    }
}

/// Enable or disable autostart
#[tauri::command]
fn set_autostart_enabled(enabled: bool) -> Result<bool, String> {
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        let autolaunch = get_auto_launch();
        if enabled {
            autolaunch
                .enable()
                .map_err(|e| format!("Failed to enable autostart: {}", e))?;
            log::info!("Autostart enabled");
        } else {
            autolaunch
                .disable()
                .map_err(|e| format!("Failed to disable autostart: {}", e))?;
            log::info!("Autostart disabled");
        }
        Ok(enabled)
    }
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        let _ = enabled;
        Err("Autostart is not supported on this platform".to_string())
    }
}
