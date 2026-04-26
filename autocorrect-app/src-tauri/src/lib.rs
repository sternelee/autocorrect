mod ai_popup;
mod clipboard;
mod commands;
mod hotkey;
mod macos_text;
mod objc2_compat;
mod overlay;
mod popup;
mod text_selection;
mod theme;
mod theme_errors;
mod typocheck;

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};

/// Hash of the last text seen by `sync_system_typos`.
/// When the text is unchanged, skip typo checking and overlay re-render.
static LAST_TEXT_HASH: AtomicU64 = AtomicU64::new(0);
/// Hash of the last bundle_id, so we reset the text hash on focus change.
static LAST_BUNDLE_HASH: AtomicU64 = AtomicU64::new(0);

#[inline]
fn hash_str(s: &str) -> u64 {
    let mut h = DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

#[cfg(target_os = "macos")]
mod geom {
    use objc2::Encode;

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct CGPoint {
        pub x: f64,
        pub y: f64,
    }

    impl CGPoint {
        pub fn new(x: f64, y: f64) -> Self {
            Self { x, y }
        }
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct CGSize {
        pub width: f64,
        pub height: f64,
    }

    impl CGSize {
        pub fn new(width: f64, height: f64) -> Self {
            Self { width, height }
        }
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct CGRect {
        pub origin: CGPoint,
        pub size: CGSize,
    }

    unsafe impl Encode for CGPoint {
        const ENCODING: objc2::Encoding = objc2::Encoding::Struct(
            "CGPoint",
            &[objc2::Encoding::Double, objc2::Encoding::Double],
        );
    }

    unsafe impl Encode for CGSize {
        const ENCODING: objc2::Encoding = objc2::Encoding::Struct(
            "CGSize",
            &[objc2::Encoding::Double, objc2::Encoding::Double],
        );
    }

    unsafe impl Encode for CGRect {
        const ENCODING: objc2::Encoding =
            objc2::Encoding::Struct("CGRect", &[<CGPoint>::ENCODING, <CGSize>::ENCODING]);
    }
}

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
use hotkey::HotkeyEvent;
use overlay::{OverlayManager, TypoMarker};
use popup::SharedPopupState;
use std::sync::mpsc::TryRecvError;
use std::thread;
use tauri::{Emitter, Manager};
use text_selection::get_cursor_position;
use theme::{get_theme, set_theme};

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

            // Check Accessibility permission.  We do NOT trigger the system
            // dialog automatically (it shows every launch for unsigned apps
            // and confuses users).  Instead we open System Settings directly
            // so the user can add the app themselves, then restart.
            #[cfg(target_os = "macos")]
            {
                if !macos_text::check_accessibility_trusted() {
                    log::warn!(
                        "Accessibility permission not granted. \
                         Opening System Settings → Privacy & Security → Accessibility. \
                         Please enable AutoCorrect and restart the app."
                    );
                    macos_text::open_accessibility_settings();
                } else {
                    log::info!("Accessibility permission granted ✓");
                }
            }

            ensure_app_settings_initialized(&app.handle())?;

            // Initialize popup state
            app.manage(SharedPopupState::new());
            app.manage(SharedAiPopupState::new());
            #[cfg(target_os = "macos")]
            app.manage(SharedNativeIconWindow::new());

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
            let (hotkey_rx, hotkey_handle) = hotkey::create_hotkey_channel(hotkey_config);

            log::info!("Global hotkey listener started");

            // Store the hotkey handle in the app state for cleanup
            app.manage(hotkey_handle);

            // 核心：启动系统级下划线同步循环
            let app_handle_for_sync = app.handle().clone();
            thread::spawn(move || loop {
                let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    #[cfg(target_os = "macos")]
                    unsafe {
                        use objc2::msg_send;
                        use objc2::runtime::AnyClass;
                        type Id = *mut objc2::runtime::AnyObject;
                        let pool_class = AnyClass::get("NSAutoreleasePool").expect("NSAutoreleasePool not found");
                        let pool: Id = msg_send![pool_class, new];
                        sync_system_typos(&app_handle_for_sync);
                        let _: () = msg_send![pool, drain];
                    }
                    #[cfg(not(target_os = "macos"))]
                    sync_system_typos(&app_handle_for_sync);
                }));
                thread::sleep(std::time::Duration::from_millis(800));
            });

            // Spawn thread to monitor mouse hover over typo markers
            let app_handle_for_hover = app.handle().clone();
            thread::spawn(move || {
                let mut last_hovered_id = String::new();
                let mut hover_start: Option<std::time::Instant> = None;
                // Track previous left-button state to detect leading edge of a click.
                let mut prev_left_down = false;

                loop {
                    thread::sleep(std::time::Duration::from_millis(100));

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
                                        let popup_w = 300.0_f64;
                                        let popup_h = 120.0_f64;
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
                    let (desktop_min_x, desktop_top_y, desktop_width, desktop_height) = unsafe {
                        use geom::CGRect;
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
                            if start.elapsed().as_millis() > 300 {
                                // Prepare typo suggestion objects
                                let typo_suggestions = vec![crate::commands::spellcheck::TypoSuggestion {
                                    typo: marker.text.clone(),
                                    suggestions: marker.suggestions.clone(),
                                    line: 0,
                                    col: 0,
                                }];

                                let suggestion_text = marker.suggestions.first().cloned().unwrap_or_default();

                                // Position popup from mouse cursor - offset to place corner at cursor
                                let popup_width = 300.0;
                                let popup_height = 120.0;
                                let offset_x = 10.0;
                                let offset_y = 10.0;

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

                            // Check if frontmost app is ignored for popup
                            let should_trigger = match commands::ignored_apps::get_frontmost_bundle_id_macos()
                            {
                                Some(bundle_id) => !is_app_ignored(&app_handle, &bundle_id, true, false),
                                None => true, // Trigger if we can't get bundle ID
                            };

                            if !should_trigger {
                                log::info!("App is ignored for popup, skipping hotkey trigger");
                                continue;
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
            get_polish_styles,
            update_config,
            start_clipboard_monitor,
            stop_clipboard_monitor,
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
            trigger_spell_check_workflow,
            // AI popup commands
            ai_popup::show_ai_popup,
            ai_popup::hide_ai_popup,
            ai_popup::get_ai_popup_state,
            ai_popup::accept_ai_result,
            // Theme commands
            get_theme,
            set_theme,
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

/// 系统级错误同步逻辑
fn sync_system_typos(app: &tauri::AppHandle) {
    let overlay_manager = match app.try_state::<OverlayManager>() {
        Some(m) => m,
        None => return,
    };

    // When AutoCorrect windows are focused, disable system overlay updates to avoid focus flicker.
    if let Some(main_window) = app.get_webview_window("main") {
        if main_window.is_focused().unwrap_or(false) {
            overlay_manager.update_markers(vec![]);
            return;
        }
    }

    // 1. 获取 AX 会话（本次 poll 周期内复用同一 focused element，
    //    避免每个 typo 都重复 AXUIElementCreateSystemWide + AXFocusedUIElement）
    #[cfg(target_os = "macos")]
    {
        let ax_session = match macos_text::AXPollSession::new() {
            Some(s) => s,
            None => {
                overlay_manager.update_markers(vec![]);
                return;
            }
        };

        match ax_session.get_text_context() {
            Ok(ctx) => {
                let is_traditional_input = matches!(
                    ctx.role.as_str(),
                    "AXTextField" | "AXTextArea" | "AXSearchField" | "AXComboBox"
                );
                let is_web_input = ctx.role == "AXWebArea";
                let is_terminal = matches!(
                    ctx.bundle_id.as_str(),
                    "com.apple.Terminal"
                        | "com.googlecode.iterm2"
                        | "io.alacritty"
                        | "com.microsoft.VSCode"
                        | "com.mitchellh.ghostty"
                );
                let is_slack = ctx.bundle_id == "com.tinyspeck.slackmacgap";
                // Some native Apple apps (Notes, Mail) and some Electron apps
                // incorrectly report editable=false on their text areas.
                // Accept any traditional text role regardless of the editable flag —
                // we rely on the terminal/bundle filters above for exclusions.
                let should_process = if is_terminal {
                    false
                } else if is_slack {
                    matches!(
                        ctx.role.as_str(),
                        "AXTextArea" | "AXWebArea" | "AXGroup" | "AXTextField"
                    )
                } else {
                    is_traditional_input || is_web_input
                };

                if !should_process {
                    // log::info!(
                    //     "[DIAG] Filtered out: role={}, editable={}, bundle={}, terminal={}",
                    //     ctx.role,
                    //     ctx.editable,
                    //     ctx.bundle_id,
                    //     is_terminal
                    // );
                    overlay_manager.update_markers(vec![]);
                    return;
                }

                // Check if app is ignored for overlay
                if is_app_ignored(app, &ctx.bundle_id, false, true) {
                    log::info!(
                        "[DIAG] App {} is ignored for overlay, skipping",
                        ctx.bundle_id
                    );
                    overlay_manager.update_markers(vec![]);
                    return;
                }

                if ctx.text.is_empty() {
                    log::info!("[DIAG] Text is empty for bundle={}", ctx.bundle_id);
                    // Reset hash so we re-check when text becomes non-empty
                    LAST_TEXT_HASH.store(0, Ordering::Relaxed);
                    overlay_manager.update_markers(vec![]);
                    return;
                }

                // Skip typo check if text and focused app are unchanged since last cycle.
                // This avoids re-running the typos library on every 800ms tick when
                // the user is not typing.
                let bundle_hash = hash_str(&ctx.bundle_id);
                let text_hash = hash_str(&ctx.text);
                let prev_bundle = LAST_BUNDLE_HASH.swap(bundle_hash, Ordering::Relaxed);
                let prev_text = LAST_TEXT_HASH.swap(text_hash, Ordering::Relaxed);
                if text_hash == prev_text && bundle_hash == prev_bundle {
                    // Text unchanged — skip typo check and overlay re-render.
                    return;
                }

                // 2. 检查拼写错误
                let typos = typocheck::check_typos(&ctx.text);
                log::info!(
                    "[DIAG] Checked text (len={}) for typos: found {}",
                    ctx.text.len(),
                    typos.len()
                );
                if typos.is_empty() {
                    log::info!(
                        "[DIAG] No typos found in text: {:?}",
                        ctx.text.chars().take(50).collect::<String>()
                    );
                } else {
                    for t in &typos {
                        log::info!("[DIAG] Typo found: '{}' at byte {}", t.typo, t.byte_offset);
                    }
                }

                let mut markers = Vec::new();

                // 3. 为每个错误获取屏幕坐标（复用会话内的 focused element + window_pos）
                for typo in typos.iter().take(10) {
                    let typo_u16_offset = byte_offset_to_utf16_offset(&ctx.text, typo.byte_offset);
                    let absolute_offset = ctx.base_offset.saturating_add(typo_u16_offset);
                    let typo_u16_len = typo.typo.encode_utf16().count();

                    if let Ok(rect) =
                        ax_session.get_range_bounds(absolute_offset, typo_u16_len)
                    {
                        if rect.size.width > 0.0 {
                            markers.push(TypoMarker {
                                id: format!("{}-{}", absolute_offset, typo.typo),
                                x: rect.origin.x,
                                y: rect.origin.y,
                                width: rect.size.width,
                                height: rect.size.height,
                                text: typo.typo.clone(),
                                suggestions: typo.suggestions.clone(),
                                offset: absolute_offset,
                                char_length: typo_u16_len,
                            });
                        }
                    }
                }

                // Fallback mechanism if no markers found but typos exist
                if !typos.is_empty() && markers.is_empty() {
                    let frame_rect = ax_session.get_element_bounds().ok();
                    let caret_rect = ax_session.get_caret_bounds().ok();
                    let (cursor_x, cursor_y) = get_cursor_position();

                    log::info!(
                        "[DIAG] Fallback started. frame={:?} caret={:?}",
                        frame_rect.is_some(),
                        caret_rect.is_some()
                    );

                    let has_valid_frame = frame_rect
                        .as_ref()
                        .map(|r| r.size.width > 20.0 && r.size.height > 10.0)
                        .unwrap_or(false);
                    let has_valid_caret = caret_rect
                        .as_ref()
                        .map(|r| r.size.width > 0.0 || r.size.height > 0.0)
                        .unwrap_or(false);

                    let text_char_len = ctx.text.chars().count().max(1);
                    let visible_lines: Vec<&str> = ctx.text.lines().collect();
                    let line_count = visible_lines.len().max(1);
                    let max_line_chars = visible_lines
                        .iter()
                        .map(|line| line.chars().count())
                        .max()
                        .unwrap_or(text_char_len)
                        .max(1);
                    let caret_char_offset =
                        utf16_offset_to_char_offset(&ctx.text, ctx.caret_offset);
                    let (caret_line, caret_col) =
                        char_offset_to_line_col(&ctx.text, caret_char_offset);

                    let (base_x, base_y, line_height) = if has_valid_frame {
                        let frame = frame_rect.unwrap_or_default();
                        // Slack and Electron apps usually have padding inside the AXFrame.
                        // Adjusted padding_x to 12.0 to move the underline 2px to the right.
                        let padding_x = 12.0;
                        let padding_y = 12.0;

                        // Treat the frame's origin as the true top-left of the text area
                        (frame.origin.x + padding_x, frame.origin.y + padding_y, 22.0)
                    } else if has_valid_caret {
                        let caret = caret_rect.unwrap_or_default();
                        // Caret is at the current line, we can't easily find line 1 without full scroll info,
                        // but if we assume single line or we use caret Y as base for line 1 (risky but okay for fallback).
                        let caret_y = caret.origin.y;
                        let caret_line = caret_line.saturating_sub(1) as f64;
                        let lh = caret.size.height.clamp(16.0, 28.0);
                        (
                            caret.origin.x - (caret_col.saturating_sub(1) as f64 * 8.0),
                            caret_y - caret_line * lh,
                            lh,
                        )
                    } else {
                        (cursor_x as f64, cursor_y as f64 - 20.0, 22.0)
                    };

                    for (i, typo) in typos.iter().enumerate().take(10) {
                        let fallback_line = typo.line.saturating_sub(1) as f64;
                        let line_text = typo
                            .line
                            .checked_sub(1)
                            .and_then(|idx| visible_lines.get(idx))
                            .copied();

                        // Re-tuned character width. 8.1 still caused slight rightward drift at the end of long sentences.
                        let base_char_width = 7.95;

                        let prefix_chars = typo.col.saturating_sub(1);
                        let text_before = line_text
                            .map(|l| {
                                let end = char_index_to_byte_offset(l, prefix_chars);
                                &l[..end.min(l.len())]
                            })
                            .unwrap_or("");

                        // Calculate width of text before typo
                        let prefix_width = text_before.chars().fold(0.0_f64, |acc, ch| {
                            let factor = match ch {
                                'i' | 'l' | 'I' | 'j' | 't' | 'f' | 'r' | '1' | '\'' | '`'
                                | '|' | '.' | ',' | ':' | ';' => 0.4,
                                'm' | 'w' | 'M' | 'W' => 1.4,
                                'A'..='Z' => 1.05,
                                ' ' => 0.65,
                                _ if ch.is_ascii_punctuation() => 0.6,
                                _ => 1.0,
                            };
                            acc + (base_char_width * factor)
                        });

                        // Calculate width of the typo itself
                        let word_width = typo
                            .typo
                            .chars()
                            .fold(0.0_f64, |acc, ch| {
                                let factor = match ch {
                                    'i' | 'l' | 'I' | 'j' | 't' | 'f' | 'r' | '1' | '\'' | '`'
                                    | '|' | '.' | ',' | ':' | ';' => 0.4,
                                    'm' | 'w' | 'M' | 'W' => 1.4,
                                    'A'..='Z' => 1.05,
                                    ' ' => 0.65,
                                    _ if ch.is_ascii_punctuation() => 0.6,
                                    _ => 1.0,
                                };
                                acc + (base_char_width * factor)
                            })
                            .max(6.0_f64);

                        let final_x = base_x + prefix_width;
                        let final_y = base_y + (fallback_line * line_height) + line_height - 2.0;

                        let absolute_offset = ctx.base_offset.saturating_add(
                            byte_offset_to_utf16_offset(&ctx.text, typo.byte_offset),
                        );
                        let char_length = typo.typo.encode_utf16().count();

                        markers.push(TypoMarker {
                            id: format!("layout-fallback-{}-{}", i, typo.typo),
                            x: final_x,
                            y: final_y,
                            width: word_width,
                            height: 2.0,
                            text: typo.typo.clone(),
                            suggestions: typo.suggestions.clone(),
                            offset: absolute_offset,
                            char_length,
                        });
                    }
                    log::info!("Generated {} fallback markers", markers.len());
                }

                overlay_manager.update_markers(markers);
            }
            Err(e) => {
                log::info!("[DIAG] sync_system_typos: focus fetch failed: {:?}", e);
                // Important: still call update_markers([]) so ensure_native_overlay runs
                // and shows the pink diagnostic line.
                overlay_manager.update_markers(vec![]);
            }
        }
    }

    // Detect long text selections and show/hide the AI floating icon.
    // Reuse the AX session obtained above (shared focused element).
    #[cfg(target_os = "macos")]
    {
        const MIN_SELECTION_CHARS: usize = 6;
        let mut icon_triggered = false;

        // Reuse session if available; fall back to a fresh one for the AI path only.
        let ai_session = macos_text::AXPollSession::new();

        let sel_result = ai_session
            .as_ref()
            .map(|s| s.get_selected_text())
            .unwrap_or(Err(macos_text::AccessibilityError::NoFocusedElement));

        match sel_result {
            Ok(sel) if sel.chars().count() >= MIN_SELECTION_CHARS => {
                let should_show_icon = match commands::ignored_apps::get_frontmost_bundle_id_macos()
                {
                    Some(bundle_id) => !is_app_ignored(app, &bundle_id, true, false),
                    None => true,
                };

                if should_show_icon {
                    let (icon_x, icon_y) = match ai_session
                        .as_ref()
                        .and_then(|s| s.get_selected_text_bounds().ok())
                    {
                        Some((sx, sy, sw, _sh)) => {
                            log::info!("[AI] selection bounds: x={} y={} w={}", sx, sy, sw);
                            (sx + sw + 4, sy - 18)
                        }
                        None => {
                            log::warn!("[AI] get_selected_text_bounds failed, using cursor");
                            let (cx, cy) = get_cursor_position();
                            (cx + 10, cy - 18)
                        }
                    };
                    log::info!(
                        "[AI] selection={} chars, icon=({},{})",
                        sel.chars().count(),
                        icon_x,
                        icon_y
                    );
                    ai_popup::show_ai_icon(app, icon_x, icon_y as i32, sel);
                    icon_triggered = true;
                }
            }
            Ok(sel) => {
                log::debug!("[AI] selection too short: {} chars", sel.chars().count());
            }
            Err(e) => {
                log::debug!("[AI] get_selected_text err: {:?}", e);
            }
        }
        // Hide icon when clicking outside or when popup is shown
        let popup_open = app
            .try_state::<SharedAiPopupState>()
            .and_then(|s| s.0.lock().ok().map(|g| g.popup_visible))
            .unwrap_or(false);
        if !icon_triggered || popup_open {
            ai_popup::hide_ai_icon(app);
        }
    }
}

fn byte_offset_to_utf16_offset(text: &str, byte_offset: usize) -> usize {
    let clamped = byte_offset.min(text.len());
    text[..clamped].encode_utf16().count()
}

fn byte_offset_to_char_offset(text: &str, byte_offset: usize) -> usize {
    let clamped = byte_offset.min(text.len());
    text[..clamped].chars().count()
}

fn utf16_offset_to_char_offset(text: &str, utf16_offset: usize) -> usize {
    let mut chars = 0usize;
    let mut seen_u16 = 0usize;
    for ch in text.chars() {
        if seen_u16 >= utf16_offset {
            break;
        }
        seen_u16 += ch.len_utf16();
        chars += 1;
    }
    chars
}

fn char_offset_to_line_col(text: &str, char_offset: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut col = 1usize;

    for (idx, ch) in text.chars().enumerate() {
        if idx >= char_offset {
            break;
        }

        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }

    (line, col)
}

fn estimate_word_visual_width(word: &str, avg_char_width: f64) -> f64 {
    let units = visual_units(word);
    (units * avg_char_width * 0.9).max(avg_char_width * 0.7)
}

fn visual_width_for_prefix(text: &str, char_count: usize, avg_char_width: f64) -> f64 {
    visual_units_for_prefix(text, char_count) * avg_char_width * 0.9
}

fn char_index_to_byte_offset(text: &str, char_index: usize) -> usize {
    if char_index == 0 {
        return 0;
    }

    text.char_indices()
        .nth(char_index)
        .map(|(idx, _)| idx)
        .unwrap_or(text.len())
}

fn visual_units(text: &str) -> f64 {
    text.chars()
        .fold(0.0, |acc, ch| acc + glyph_width_factor(ch))
}

fn visual_units_for_prefix(text: &str, char_count: usize) -> f64 {
    text.chars()
        .take(char_count)
        .fold(0.0, |acc, ch| acc + glyph_width_factor(ch))
}

fn glyph_width_factor(ch: char) -> f64 {
    match ch {
        'i' | 'l' | 'I' | 'j' | 't' | 'f' | 'r' | '1' | '\'' | '`' | '|' => 0.55,
        'm' | 'w' | 'M' | 'W' | '@' | '%' | '&' | 'Q' | 'O' => 1.3,
        'A'..='Z' => 1.05,
        '0'..='9' => 0.9,
        _ if ch.is_ascii_punctuation() => 0.5,
        _ => 0.92,
    }
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
    auto_launch::AutoLaunch::new(&app_name, &exec_path, false, &[""] as &[&str])
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
