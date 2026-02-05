//! System-wide text selection using macOS Accessibility API
//!
//! This module provides functionality to retrieve the currently selected text
//! from the focused application using macOS Accessibility services.
//!
//! # Current Implementation
//!
//! Due to conflicts between rdev (global hotkey listener) and enigo (keyboard simulation),
//! the automatic copy simulation has been disabled to prevent crashes.
//!
//! Users should manually copy text (⌘+C) before triggering the spell check workflow.
//!
//! # Alternative Approach
//!
//! For a production app, consider:
//! 1. Using macOS Accessibility API directly via CGEvent/AXUIElement
//! 2. Implementing a proper NSPasteboard monitoring system
//! 3. Using a separate process for keyboard simulation

/// Error type for text selection operations
#[derive(Debug, thiserror::Error)]
pub enum TextSelectionError {
    #[error("No text selected. Please select text and copy it first (⌘+C), then press the hotkey again.")]
    NoTextSelected,

    #[error("Selected text is empty")]
    EmptySelection,

    #[error("Accessibility API error: {0}")]
    AccessibilityError(String),

    #[error("Permission denied - app needs Accessibility permissions")]
    PermissionDenied,
}

/// Result type for text selection operations
pub type Result<T> = std::result::Result<T, TextSelectionError>;

/// Get the currently selected text from the system's focused element
///
/// This function currently returns an error, directing users to copy text manually.
/// The automatic copy simulation has been disabled due to conflicts with
/// the global hotkey listener.
pub fn get_selected_text() -> Result<String> {
    Err(TextSelectionError::NoTextSelected)
}

/// Alternative approach: Get text from clipboard
///
/// This function simply reads from the clipboard without any simulation.
/// Users should manually copy text (⌘+C) before pressing the hotkey.
pub fn get_selected_text_via_copy() -> Result<String> {
    use arboard::Clipboard;

    // Get the current clipboard content
    let text = Clipboard::new()
        .and_then(|mut clipboard| clipboard.get_text())
        .map_err(|_| TextSelectionError::NoTextSelected)?;

    if text.trim().is_empty() {
        return Err(TextSelectionError::EmptySelection);
    }

    Ok(text)
}

/// Get the cursor position for popup placement
///
/// Returns the (x, y) coordinates of the mouse cursor.
///
/// On macOS, this would ideally use NSEvent to get the current mouse location.
/// For now, we return a default position centered on a typical screen.
///
/// TODO: Implement proper mouse position tracking using:
/// - Core Graphics CGEvent
/// - NSEvent mouseLocation via objc bindings
/// - Or track mouse movement via rdev events
pub fn get_cursor_position() -> (i32, i32) {
    // For now, return a default position centered on a typical screen
    (800, 400)
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
}
