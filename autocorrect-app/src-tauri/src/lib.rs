mod clipboard;
mod commands;
mod hotkey;
mod macos_text;
mod overlay;
mod popup;
mod text_selection;
mod typocheck;

use commands::ai_grammar::{ai_grammar_check, ai_text_transform};
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
use overlay::{OverlayManager, TypoMarker};
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

/// Show the widget popup at the specified position
#[tauri::command]
fn trigger_widget_popup(app: tauri::AppHandle, x: i32, y: i32) -> Result<(), String> {
    log::info!("trigger_widget_popup called at ({}, {})", x, y);

    // Get the text from the focused element
    #[cfg(target_os = "macos")]
    {
        if let Ok(ctx) = macos_text::get_focused_text_context() {
            if !ctx.text.is_empty() {
                // Trigger the spell check workflow with the current text
                let app_clone = app.clone();
                std::thread::spawn(move || {
                    popup::trigger_spell_check_workflow(app_clone, x, y + 40).ok();
                });
            }
        }
    }

    Ok(())
}

/// Update widget position and visibility based on typos
#[tauri::command]
fn update_widget(app: tauri::AppHandle, typo_count: i32, x: f64, y: f64) -> Result<(), String> {
    log::debug!("update_widget: count={}, x={}, y={}", typo_count, x, y);

    if let Some(widget_window) = app.get_webview_window("widget") {
        if typo_count > 0 {
            // Position widget at bottom-left of input field
            // x is left edge, y is top edge - so bottom-left is (x, y + height - widget_size)
            let widget_x = x as i32;
            let widget_y = (y as i32) + 20; // Slightly below the input field

            let position = tauri::Position::Physical(tauri::PhysicalPosition {
                x: widget_x,
                y: widget_y,
            });
            let _ = widget_window.set_position(position);
            let _ = widget_window.show();

            // Emit event to update widget UI
            let _ = app.emit(
                "widget-update",
                serde_json::json!({ "typoCount": typo_count }),
            );
        } else {
            let _ = widget_window.hide();
        }
    }

    Ok(())
}

#[allow(clippy::missing_panics_doc)]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Create channel for clipboard events (hotkey channel is created below)
    let (_clipboard_tx, clipboard_rx) = std::sync::mpsc::channel();

    tauri::Builder::default()
        .setup(move |app| {
            app.handle().plugin(tauri_plugin_http::init())?;

            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            // Initialize popup state
            app.manage(SharedPopupState::new());

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
                        use objc::{msg_send, sel, sel_impl};
                        let pool: cocoa::base::id = msg_send![objc::class!(NSAutoreleasePool), new];
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
                
                loop {
                    thread::sleep(std::time::Duration::from_millis(100));
                    
                    let overlay_manager = match app_handle_for_hover.try_state::<OverlayManager>() {
                        Some(m) => m,
                        None => continue,
                    };

                    let (mouse_x, mouse_y) = text_selection::get_cursor_position();
                    let mouse_x_f = mouse_x as f64;
                    let mouse_y_f = mouse_y as f64;

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
                        use cocoa::appkit::NSScreen;
                        use cocoa::base::{id, nil};
                        use cocoa::foundation::NSRect;
                        use objc::{msg_send, sel, sel_impl};
                        
                        let mut min_x = f64::MAX;
                        let mut min_y = f64::MAX;
                        let mut max_x = f64::MIN;
                        let mut max_y = f64::MIN;
                        let screens: id = NSScreen::screens(nil);
                        let count: usize = msg_send![screens, count];
                        for idx in 0..count {
                            let screen: id = msg_send![screens, objectAtIndex: idx];
                            let frame: NSRect = msg_send![screen, frame];
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
                        let is_fallback = marker.id.contains("fallback");
                        let y_top_left = if is_fallback {
                            marker.y
                        } else {
                            desktop_top_y - marker.y
                        };

                        // Add a larger hit area for easier hovering (e.g., ±8px vertically)
                        let hit_rect_x = marker.x - 4.0;
                        let hit_rect_y = y_top_left - 8.0;
                        let hit_rect_w = marker.width + 8.0;
                        let hit_rect_h = marker.height + 16.0;

                        if mouse_x_f >= hit_rect_x 
                            && mouse_x_f <= hit_rect_x + hit_rect_w 
                            && mouse_y_f >= hit_rect_y 
                            && mouse_y_f <= hit_rect_y + hit_rect_h 
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
                                
                                // Position popup from mouse cursor with a small offset.
                                let popup_width = 320.0;
                                let popup_height = 240.0;
                                let offset_x = 12.0;
                                let offset_y = 18.0;

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
            ai_grammar_check,
            ai_text_transform,
            // Popup commands
            show_popup,
            hide_popup,
            position_popup,
            get_popup_state,
            accept_suggestion,
            reject_suggestion,
            trigger_spell_check_workflow,
            // Widget commands
            trigger_widget_popup,
            update_widget,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
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

    // 1. 检查焦点文本
    #[cfg(target_os = "macos")]
    {
        match macos_text::get_focused_text_context() {
            Ok(ctx) => {
                let is_traditional_input = matches!(
                    ctx.role.as_str(),
                    "AXTextField" | "AXTextArea" | "AXSearchField" | "AXComboBox"
                );
                let is_web_input = ctx.role == "AXWebArea";
                let is_terminal = matches!(
                    ctx.bundle_id.as_str(),
                    "com.apple.Terminal" | "com.googlecode.iterm2" | "io.alacritty" | "com.microsoft.VSCode" | "com.mitchellh.ghostty"
                );
                let is_slack = ctx.bundle_id == "com.tinyspeck.slackmacgap";

                // For traditional apps, we demand ctx.editable to be true to avoid reading static text.
                // However, Electron apps like Slack often incorrectly report AXTextArea as editable=false.
                let should_process = if is_terminal {
                    false
                } else if is_slack {
                    matches!(ctx.role.as_str(), "AXTextArea" | "AXWebArea" | "AXGroup" | "AXTextField")
                } else {
                    (is_traditional_input && ctx.editable) || is_web_input
                };

                if !should_process {
                    log::info!("[DIAG] Filtered out: role={}, editable={}, bundle={}, terminal={}", ctx.role, ctx.editable, ctx.bundle_id, is_terminal);
                    overlay_manager.update_markers(vec![]);
                    return;
                }

                if ctx.text.is_empty() {
                    log::info!("[DIAG] Text is empty for bundle={}", ctx.bundle_id);
                    overlay_manager.update_markers(vec![]);
                    return;
                }

                // 2. 检查拼写错误
                let typos = typocheck::check_typos(&ctx.text);
                log::info!("[DIAG] Checked text (len={}) for typos: found {}", ctx.text.len(), typos.len());
                if typos.is_empty() {
                    log::info!("[DIAG] No typos found in text: {:?}", ctx.text.chars().take(50).collect::<String>());
                } else {
                    for t in &typos {
                        log::info!("[DIAG] Typo found: '{}' at byte {}", t.typo, t.byte_offset);
                    }
                }

                let mut markers = Vec::new();

                // 3. 为每个错误获取屏幕坐标
                for typo in typos.iter().take(10) {
                    let typo_u16_offset = byte_offset_to_utf16_offset(&ctx.text, typo.byte_offset);
                    let absolute_offset = ctx.base_offset.saturating_add(typo_u16_offset);
                    let typo_u16_len = typo.typo.encode_utf16().count();
                    
                    if let Ok(rect) = macos_text::get_focused_range_bounds(absolute_offset, typo_u16_len) {
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
                    let frame_rect = macos_text::get_focused_element_bounds().ok();
                    let caret_rect = macos_text::get_focused_caret_bounds().ok();
                    let (cursor_x, cursor_y) = get_cursor_position();
                    
                    log::info!("[DIAG] Fallback started. frame={:?} caret={:?}", frame_rect.is_some(), caret_rect.is_some());

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
                    let caret_char_offset = utf16_offset_to_char_offset(&ctx.text, ctx.caret_offset);
                    let (caret_line, caret_col) = char_offset_to_line_col(&ctx.text, caret_char_offset);

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
                        (caret.origin.x - (caret_col.saturating_sub(1) as f64 * 8.0), caret_y - caret_line * lh, lh)
                    } else {
                        (cursor_x as f64, cursor_y as f64 - 20.0, 22.0)
                    };

                    for (i, typo) in typos.iter().enumerate().take(10) {
                        let fallback_line = typo.line.saturating_sub(1) as f64;
                        let line_text = typo.line.checked_sub(1).and_then(|idx| visible_lines.get(idx)).copied();
                        
                        // Re-tuned character width. 8.1 still caused slight rightward drift at the end of long sentences.
                        let base_char_width = 7.95; 
                        
                        let prefix_chars = typo.col.saturating_sub(1);
                        let text_before = line_text.map(|l| {
                            let end = char_index_to_byte_offset(l, prefix_chars);
                            &l[..end.min(l.len())]
                        }).unwrap_or("");
                        
                        // Calculate width of text before typo
                        let prefix_width = text_before.chars().fold(0.0_f64, |acc, ch| {
                            let factor = match ch {
                                'i' | 'l' | 'I' | 'j' | 't' | 'f' | 'r' | '1' | '\'' | '`' | '|' | '.' | ',' | ':' | ';' => 0.4,
                                'm' | 'w' | 'M' | 'W' => 1.4,
                                'A'..='Z' => 1.05,
                                ' ' => 0.65,
                                _ if ch.is_ascii_punctuation() => 0.6,
                                _ => 1.0,
                            };
                            acc + (base_char_width * factor)
                        });

                        // Calculate width of the typo itself
                        let word_width = typo.typo.chars().fold(0.0_f64, |acc, ch| {
                            let factor = match ch {
                                'i' | 'l' | 'I' | 'j' | 't' | 'f' | 'r' | '1' | '\'' | '`' | '|' | '.' | ',' | ':' | ';' => 0.4,
                                'm' | 'w' | 'M' | 'W' => 1.4,
                                'A'..='Z' => 1.05,
                                ' ' => 0.65,
                                _ if ch.is_ascii_punctuation() => 0.6,
                                _ => 1.0,
                            };
                            acc + (base_char_width * factor)
                        }).max(6.0_f64);

                        let final_x = base_x + prefix_width;
                        let final_y = base_y + (fallback_line * line_height) + line_height - 2.0;

                        let absolute_offset = ctx.base_offset.saturating_add(byte_offset_to_utf16_offset(&ctx.text, typo.byte_offset));
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
        use macos_text::check_and_request_accessibility;
        check_and_request_accessibility()
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
        use macos_text::check_and_request_accessibility;
        check_and_request_accessibility()
    }

    #[cfg(not(target_os = "macos"))]
    {
        true // Always true on other platforms
    }
}
