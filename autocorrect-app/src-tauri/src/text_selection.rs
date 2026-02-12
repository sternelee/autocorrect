//! System-wide text selection using macOS Accessibility API

use crate::macos_text::{self, AccessibilityError};
use std::thread;
use std::time::Duration;

/// Error type for text selection operations
#[derive(Debug, thiserror::Error)]
pub enum TextSelectionError {
    #[error("No text found")]
    NoTextFound,
    #[error("No text selected")]
    NoTextSelected,
    #[error("Permission denied")]
    PermissionDenied,
    #[error("API error: {0}")]
    ApiError(String),
}

pub type Result<T> = std::result::Result<T, TextSelectionError>;

/// 获取当前焦点输入框的全文（实时）
pub fn get_active_text() -> Result<String> {
    #[cfg(target_os = "macos")]
    {
        // 尝试获取焦点元素的全文
        // 我们传入 0,0 只是为了获取全文，暂时不关心坐标
        match macos_text::get_focused_element_data(0, 0) {
            Ok((text, _)) => Ok(text),
            Err(AccessibilityError::PermissionDenied) => Err(TextSelectionError::PermissionDenied),
            Err(_) => Err(TextSelectionError::NoTextFound),
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(String::new())
    }
}

/// 获取选中的文字（保持兼容性）
pub fn get_selected_text() -> Result<String> {
    #[cfg(target_os = "macos")]
    {
        match macos_text::get_selected_text() {
            Ok(text) => {
                log::info!("Selected text fetched via AXSelectedText");
                Ok(text)
            }
            Err(AccessibilityError::PermissionDenied) => Err(TextSelectionError::PermissionDenied),
            Err(AccessibilityError::NoTextSelected | AccessibilityError::NoFocusedElement) => {
                log::info!("AX selected text unavailable, trying clipboard fallback");
                get_selected_text_via_clipboard_fallback()
            }
            Err(e) => Err(TextSelectionError::ApiError(e.to_string())),
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err(TextSelectionError::NoTextFound)
    }
}

#[cfg(target_os = "macos")]
fn get_selected_text_via_clipboard_fallback() -> Result<String> {
    let mut clipboard = arboard::Clipboard::new()
        .map_err(|e| TextSelectionError::ApiError(format!("Clipboard init failed: {e}")))?;

    let old_clipboard = clipboard.get_text().ok();
    let sentinel = format!("__autocorrect_sentinel_{}__", std::process::id());

    // Write a sentinel before simulating copy so we can detect whether selection was copied.
    clipboard
        .set_text(sentinel.clone())
        .map_err(|e| TextSelectionError::ApiError(format!("Clipboard write failed: {e}")))?;

    let status = std::process::Command::new("osascript")
        .arg("-e")
        .arg("tell application \"System Events\" to keystroke \"c\" using command down")
        .status()
        .map_err(|e| TextSelectionError::ApiError(format!("Copy simulation failed: {e}")))?;

    if !status.success() {
        restore_clipboard(&mut clipboard, old_clipboard);
        return Err(TextSelectionError::ApiError(
            "Copy simulation command failed".to_string(),
        ));
    }

    thread::sleep(Duration::from_millis(150));

    let copied = clipboard.get_text().unwrap_or_default();
    restore_clipboard(&mut clipboard, old_clipboard);

    if copied.trim().is_empty() || copied == sentinel {
        log::warn!("Clipboard fallback did not capture selected text");
        return Err(TextSelectionError::NoTextSelected);
    }

    log::info!("Selected text fetched via clipboard fallback");
    Ok(copied)
}

#[cfg(target_os = "macos")]
fn restore_clipboard(clipboard: &mut arboard::Clipboard, previous: Option<String>) {
    if let Some(text) = previous {
        let _ = clipboard.set_text(text);
    }
}

pub fn get_cursor_position() -> (i32, i32) {
    #[cfg(target_os = "macos")]
    {
        macos_text::get_cursor_position_nsevent()
    }
    #[cfg(not(target_os = "macos"))]
    {
        (800, 400)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_cursor_position() {
        let (x, y) = get_cursor_position();
        println!("Cursor position: ({}, {})", x, y);
        assert!(x >= 0);
        assert!(y >= 0);
    }

    #[test]
    fn test_get_selected_text() {
        match get_selected_text() {
            Ok(text) => println!("Selected text: {}", text),
            Err(e) => println!("Error: {}", e),
        }
    }
}
