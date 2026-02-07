//! System-wide text selection using macOS Accessibility API

use crate::macos_text::{self, AccessibilityError};

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
    // 现在的版本优先尝试直接从 UI 树获取，或者引导用户
    get_active_text()
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
