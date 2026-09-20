//! System-wide focused-text typo synchronization.
//!
//! This module owns the macOS polling path that reads the focused AX text,
//! computes typo markers, updates the overlay, and manages the AI selection icon.

use crate::ai_popup::{self, SharedAiPopupState};
use crate::commands;
use crate::commands::ignored_apps::is_app_ignored;
use crate::grammar;
use crate::macos_text;
use crate::overlay::{OverlayManager, TypoMarker};
use crate::text_selection::get_cursor_position;
use crate::text_utils::*;
use crate::typocheck::{self, TypoError};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use tauri::Manager;

/// Hash of the last text seen by `sync_system_typos`.
/// When the text is unchanged, skip typo checking and overlay re-render.
static LAST_TEXT_HASH: AtomicU64 = AtomicU64::new(0);
/// Hash of the last bundle_id, so we reset the text hash on focus change.
static LAST_BUNDLE_HASH: AtomicU64 = AtomicU64::new(0);
/// Hash of the text whose grammar results have already been applied to
/// markers. When LanguageTool results land for the current text, one extra
/// marker rebuild merges them into the overlay.
static LAST_GRAMMAR_APPLIED: AtomicU64 = AtomicU64::new(0);

#[inline]
fn hash_str(s: &str) -> u64 {
    let mut h = DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

/// Build overlay markers for the currently focused text context.
///
/// Prefers exact AX range bounds and falls back to line/column layout
/// estimates for applications that do not expose reliable text ranges.
#[cfg(target_os = "macos")]
fn build_typo_markers(
    ax_session: &macos_text::AXPollSession,
    ctx: &macos_text::FocusedTextContext,
    grammar_issues: &[grammar::GrammarIssue],
) -> Vec<TypoMarker> {
    // 2. 检查拼写错误
    let mut typos = typocheck::check_typos(&ctx.text);

    // Merge LanguageTool grammar/style issues into the same marker stream.
    // Offsets arrive as UTF-16 code units; skip anything that cannot be
    // mapped back to a clean substring.
    for issue in grammar_issues {
        let text_slice = issue.text_slice(&ctx.text);
        if text_slice.is_empty() {
            continue;
        }
        let Some(byte_offset) = grammar::utf16_offset_to_byte(&ctx.text, issue.offset) else {
            continue;
        };
        if typos.iter().any(|e| e.byte_offset == byte_offset) {
            continue; // local checker already flags this range
        }
        let char_offset = utf16_offset_to_char_offset(&ctx.text, issue.offset);
        let (line, col) = char_offset_to_line_col(&ctx.text, char_offset);
        typos.push(TypoError {
            typo: text_slice.to_string(),
            suggestions: issue.replacements.clone(),
            byte_offset,
            line,
            col,
        });
    }
    typos.sort_by_key(|e| e.byte_offset);
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

    // Pre-compute all UTF-16 offsets in one O(text_len) pass
    // instead of N separate O(text_len) scans.
    let capped_typos: Vec<_> = typos.iter().take(10).collect();
    let utf16_offsets =
        batch_byte_to_utf16_offsets(&ctx.text, capped_typos.iter().map(|t| t.byte_offset));

    // 3. 为每个错误获取屏幕坐标（复用会话内的 focused element + window_pos）
    for (typo, typo_u16_offset) in capped_typos.iter().zip(utf16_offsets.iter()) {
        let absolute_offset = ctx.base_offset.saturating_add(*typo_u16_offset);
        let typo_u16_len = typo.typo.encode_utf16().count();

        if let Ok(rect) = ax_session.get_range_bounds(absolute_offset, typo_u16_len) {
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
        // Pre-compute UTF-16 offsets for fallback path too
        let fallback_utf16_offsets =
            batch_byte_to_utf16_offsets(&ctx.text, typos.iter().take(10).map(|t| t.byte_offset));

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
        let _line_count = visible_lines.len().max(1);
        let _max_line_chars = visible_lines
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
            (
                caret.origin.x - (caret_col.saturating_sub(1) as f64 * 8.0),
                caret_y - caret_line * lh,
                lh,
            )
        } else {
            (cursor_x as f64, cursor_y as f64 - 20.0, 22.0)
        };

        for (i, typo) in typos.iter().enumerate().take(10) {
            let typo_u16_offset = fallback_utf16_offsets[i];
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
                    'i' | 'l' | 'I' | 'j' | 't' | 'f' | 'r' | '1' | '\'' | '`' | '|' | '.'
                    | ',' | ':' | ';' => 0.4,
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
                        'i' | 'l' | 'I' | 'j' | 't' | 'f' | 'r' | '1' | '\'' | '`' | '|' | '.'
                        | ',' | ':' | ';' => 0.4,
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

            let absolute_offset = ctx.base_offset.saturating_add(typo_u16_offset);
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

    markers
}

/// Show or hide the floating AI action icon for the current text selection.
fn sync_ai_icon(app: &tauri::AppHandle) {
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
                    ai_popup::show_ai_icon(app, icon_x, icon_y, sel);
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

/// 系统级错误同步逻辑：先更新 typo overlay，再同步 AI selection icon。
pub fn sync_system_typos(app: &tauri::AppHandle) {
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

                // Grammar engine: fire an async LanguageTool check when this
                // text hasn't been checked yet, and rebuild markers once its
                // results land (the rebuild is triggered by the extra
                // grammar-hash term in the change detection below).
                let (grammar_enabled, endpoint) = grammar_settings(app);
                let mut grammar_issues: Vec<grammar::GrammarIssue> = Vec::new();
                let grammar_applied = if grammar_enabled {
                    match grammar::cached(text_hash) {
                        Some(issues) => {
                            grammar_issues = issues;
                            text_hash
                        }
                        None => {
                            grammar::request_check(text_hash, ctx.text.clone(), endpoint);
                            0
                        }
                    }
                } else {
                    0
                };
                let prev_grammar = LAST_GRAMMAR_APPLIED.swap(grammar_applied, Ordering::Relaxed);

                if text_hash == prev_text
                    && bundle_hash == prev_bundle
                    && grammar_applied == prev_grammar
                {
                    // Text unchanged and no fresh grammar results — skip.
                    return;
                }

                let markers = build_typo_markers(&ax_session, &ctx, &grammar_issues);
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

    sync_ai_icon(app);
}

/// Grammar engine settings for this cycle: (enabled, endpoint).
/// Defaults mirror `config.rs` when settings cannot be loaded.
fn grammar_settings(app: &tauri::AppHandle) -> (bool, String) {
    commands::config::load_app_settings(app).map_or_else(
        |_| (true, "http://127.0.0.1:8081".to_string()),
        |s| (s.grammar_engine_enabled, s.languagetool_endpoint),
    )
}
