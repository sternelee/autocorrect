//! System-wide text selection using macOS Accessibility API
//!
//! This module provides functionality to retrieve the currently selected text
//! from the focused application using macOS Accessibility services.
//!
//! # Implementation
//!
//! Uses macOS Accessibility API (AXUIElement) to directly access the selected
//! text from the focused application without requiring manual copy.
//!
//! Falls back to clipboard reading if Accessibility API is unavailable or
//! permission is not granted.

/// Error type for text selection operations
#[derive(Debug, thiserror::Error)]
pub enum TextSelectionError {
    #[error("No text selected. Please select some text first, then press the hotkey.")]
    NoTextSelected,

    #[error("Selected text is empty")]
    EmptySelection,

    #[error("Accessibility permission denied. Please grant Accessibility permissions in System Settings > Privacy & Security > Accessibility")]
    PermissionDenied,

    #[error("Accessibility API error: {0}")]
    ApiError(String),

    #[error("Clipboard error: {0}")]
    ClipboardError(String),
}

/// Result type for text selection operations
pub type Result<T> = std::result::Result<T, TextSelectionError>;

/// Get the currently selected text from the system's focused element
///
/// This function:
/// 1. Simulates Cmd+C to copy selected text (uses CGEvent, avoids rdev conflicts)
/// 2. Reads the clipboard to get the text
/// 3. Returns appropriate errors if no text is selected
pub fn get_selected_text() -> Result<String> {
    #[cfg(target_os = "macos")]
    {
        // Use CGEvent to simulate copy (more reliable than AppleScript)
        use crate::macos_text::{get_selected_text_via_accessibility, AccessibilityError};

        match get_selected_text_via_accessibility() {
            Ok(text) => {
                if !text.trim().is_empty() {
                    log::info!("Got selected text: {} chars", text.chars().count());
                    return Ok(text);
                } else {
                    log::info!("Selected text is empty");
                    return Err(TextSelectionError::NoTextSelected);
                }
            }
            Err(AccessibilityError::NoTextSelected) => {
                log::info!("No text selected to copy");
                return Err(TextSelectionError::NoTextSelected);
            }
            Err(e) => {
                log::warn!("Failed to get selected text: {}", e);
                return Err(TextSelectionError::NoTextSelected);
            }
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        // Fall back to clipboard for other platforms
        get_selected_text_via_clipboard()
    }
}

/// Get text from clipboard (fallback method)
///
/// This is a fallback when Accessibility API is unavailable.
/// Users should copy text (⌘+C) before pressing the hotkey.
fn get_selected_text_via_clipboard() -> Result<String> {
    use arboard::Clipboard;

    let text = Clipboard::new()
        .and_then(|mut clipboard| clipboard.get_text())
        .map_err(|e| TextSelectionError::ClipboardError(e.to_string()))?;

    if text.trim().is_empty() {
        return Err(TextSelectionError::NoTextSelected);
    }

    log::info!("Got text from clipboard: {} chars", text.chars().count());
    Ok(text)
}

/// Get the cursor position for popup placement
///
/// Returns the (x, y) coordinates of the mouse cursor.
///
/// On macOS, this uses NSEvent to get the current mouse location.
/// Note: NSEvent must be called from the main thread, but hotkey
/// events come from a background thread, so we use a safe default.
pub fn get_cursor_position() -> (i32, i32) {
    #[cfg(target_os = "macos")]
    {
        // Use the cursor position with smart offset (appears above text when possible)
        crate::macos_text::get_cursor_position_nsevent()
    }

    #[cfg(not(target_os = "macos"))]
    {
        // Default position for other platforms
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
