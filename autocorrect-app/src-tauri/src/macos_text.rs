//! macOS-specific text selection and utilities
//!
//! This module provides macOS-specific functionality using:
//! - CGEvent for keyboard simulation (avoiding rdev conflicts)
//! - Clipboard for text retrieval
//! - AppleScript for getting selected text position

use std::process::Command;
use std::sync::atomic::{AtomicI32, Ordering};
use std::thread;
use std::time::Duration;

/// Global mouse position tracked by rdev
#[cfg(target_os = "macos")]
static MOUSE_X: AtomicI32 = AtomicI32::new(800);
#[cfg(target_os = "macos")]
static MOUSE_Y: AtomicI32 = AtomicI32::new(400);

/// Update the tracked mouse position (called by rdev listener)
#[cfg(target_os = "macos")]
pub fn update_mouse_position(x: i32, y: i32) {
    MOUSE_X.store(x, Ordering::Relaxed);
    MOUSE_Y.store(y, Ordering::Relaxed);
    log::debug!("Updated mouse position to: ({}, {})", x, y);
}

/// Get the last tracked mouse position
#[cfg(target_os = "macos")]
pub fn get_tracked_mouse_position() -> (i32, i32) {
    // Try to get current mouse position directly via CGEvent
    match get_current_mouse_position_cg() {
        Ok(pos) => {
            log::debug!("Got mouse position from CGEvent: ({}, {})", pos.0, pos.1);
            // Update the tracked position
            MOUSE_X.store(pos.0, Ordering::Relaxed);
            MOUSE_Y.store(pos.1, Ordering::Relaxed);
            pos
        }
        Err(_) => {
            // Fall back to tracked position
            let x = MOUSE_X.load(Ordering::Relaxed);
            let y = MOUSE_Y.load(Ordering::Relaxed);
            log::debug!("Retrieved tracked mouse position: ({}, {})", x, y);
            (x, y)
        }
    }
}

/// Get current mouse position using CGEvent (fallback)
#[cfg(target_os = "macos")]
fn get_current_mouse_position_cg() -> std::result::Result<(i32, i32), ()> {
    use core_graphics::event::{CGEvent, CGEventTapLocation};
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState).map_err(|_| ())?;

    // Query the current mouse location
    let event = CGEvent::new(source).map_err(|_| ())?;

    let point = event.location();
    Ok((point.x as i32, point.y as i32))
}

/// Error type for macOS operations
#[derive(Debug, thiserror::Error)]
pub enum AccessibilityError {
    #[error("No text selected")]
    NoTextSelected,

    #[error("Selected text is empty")]
    EmptySelection,

    #[error("Keyboard simulation failed: {0}")]
    KeyboardError(String),

    #[error("Clipboard error: {0}")]
    ClipboardError(String),

    #[error("AppleScript execution failed: {0}")]
    AppleScriptError(String),

    #[error("Timeout waiting for clipboard")]
    Timeout,

    #[error("Unsupported operation")]
    Unsupported,
}

/// Result type for operations
pub type Result<T> = std::result::Result<T, AccessibilityError>;

/// Simulate Cmd+C to copy selected text using CGEvent
///
/// This uses Core Graphics CGEvent which posts events at the HID level,
/// avoiding conflicts with rdev's event tap.
#[cfg(target_os = "macos")]
fn simulate_copy() -> Result<()> {
    use core_graphics::event::{CGEvent, CGEventTapLocation};
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

    // Create event source with private state ID
    let source = CGEventSource::new(CGEventSourceStateID::Private).map_err(|e| {
        AccessibilityError::KeyboardError(format!("Failed to create event source: {:?}", e))
    })?;

    // Key codes: Command = 0x37 (55), C = 0x08 (8)
    // Use raw values since KeyCode constants may not be accessible
    const CMD_KEYCODE: u16 = 0x37;
    const C_KEYCODE: u16 = 0x08;

    // Command key down
    let cmd_down = CGEvent::new_keyboard_event(source.clone(), CMD_KEYCODE, true).map_err(|e| {
        AccessibilityError::KeyboardError(format!("Failed to create Cmd down: {:?}", e))
    })?;
    cmd_down.set_flags(core_graphics::event::CGEventFlags::CGEventFlagCommand);
    cmd_down.post(CGEventTapLocation::HID);

    thread::sleep(Duration::from_millis(15));

    // C key down
    let c_down = CGEvent::new_keyboard_event(source.clone(), C_KEYCODE, true).map_err(|e| {
        AccessibilityError::KeyboardError(format!("Failed to create C down: {:?}", e))
    })?;
    c_down.set_flags(core_graphics::event::CGEventFlags::CGEventFlagCommand);
    c_down.post(CGEventTapLocation::HID);

    thread::sleep(Duration::from_millis(15));

    // C key up
    let c_up = CGEvent::new_keyboard_event(source.clone(), C_KEYCODE, false).map_err(|e| {
        AccessibilityError::KeyboardError(format!("Failed to create C up: {:?}", e))
    })?;
    c_up.set_flags(core_graphics::event::CGEventFlags::CGEventFlagCommand);
    c_up.post(CGEventTapLocation::HID);

    thread::sleep(Duration::from_millis(15));

    // Command key up
    let cmd_up = CGEvent::new_keyboard_event(source, CMD_KEYCODE, false).map_err(|e| {
        AccessibilityError::KeyboardError(format!("Failed to create Cmd up: {:?}", e))
    })?;
    cmd_up.post(CGEventTapLocation::HID);

    Ok(())
}

/// Get the currently selected text by simulating copy
///
/// This function:
/// 1. Simulates Cmd+C to copy selected text
/// 2. Reads the clipboard
/// 3. Returns the copied text
///
/// Note: This modifies the clipboard.
#[cfg(target_os = "macos")]
pub fn get_selected_text_via_accessibility() -> Result<String> {
    use arboard::Clipboard;

    // Simulate Cmd+C
    simulate_copy()?;

    // Wait for clipboard to update
    thread::sleep(Duration::from_millis(50));

    // Try multiple times to get the clipboard content
    for attempt in 0..5 {
        match Clipboard::new().and_then(|mut cb| cb.get_text()) {
            Ok(text) => {
                if !text.trim().is_empty() {
                    log::info!(
                        "Got selected text via copy simulation: {} chars",
                        text.chars().count()
                    );
                    return Ok(text);
                }
            }
            Err(_) => {
                if attempt == 4 {
                    return Err(AccessibilityError::ClipboardError(
                        "Failed to read clipboard".to_string(),
                    ));
                }
            }
        }
        thread::sleep(Duration::from_millis(20));
    }

    Err(AccessibilityError::NoTextSelected)
}

/// Check if the app has Accessibility permissions (not used with CGEvent method)
#[cfg(target_os = "macos")]
pub fn check_accessibility_permission() -> bool {
    true // CGEvent doesn't require Accessibility permission
}

/// Request Accessibility permissions (not needed with CGEvent method)
#[cfg(target_os = "macos")]
pub fn request_accessibility_permission() -> bool {
    true // CGEvent doesn't require Accessibility permission
}

/// Get the position of selected text using AppleScript
///
/// This uses AppleScript to get the mouse location at the current moment.
#[cfg(target_os = "macos")]
pub fn get_selected_text_position() -> Option<(i32, i32)> {
    let script = r#"
    tell application "System Events"
        set mouseLoc to mouse location
        set xPos to item 1 of mouseLoc
        set yPos to item 2 of mouseLoc
        return (xPos as string) & "," & (yPos as string)
    end tell
    "#;

    let output = Command::new("osascript").arg("-e").arg(script).output();

    match output {
        Ok(result) if result.status.success() => {
            let stdout = String::from_utf8_lossy(&result.stdout).trim().to_string();
            if !stdout.is_empty() {
                let parts: Vec<&str> = stdout.split(',').collect();
                if parts.len() == 2 {
                    if let (Ok(x), Ok(y)) = (parts[0].parse::<f64>(), parts[1].parse::<f64>()) {
                        log::info!("Got mouse position from AppleScript: ({}, {})", x, y);
                        // Position popup slightly to the right and below the mouse
                        return Some((x as i32 + 20, y as i32 + 30));
                    }
                }
            }
        }
        Ok(result) => {
            let stderr = String::from_utf8_lossy(&result.stderr);
            log::debug!("AppleScript error: {}", stderr);
        }
        Err(e) => {
            log::debug!("Failed to run AppleScript: {}", e);
        }
    }

    None
}

/// Get cursor position using selected text position or mouse position
///
/// Returns the (x, y) coordinates for the popup.
/// Tries to get the selected text position via AppleScript first,
/// falls back to current mouse position via CGEvent.
#[cfg(target_os = "macos")]
pub fn get_cursor_position_nsevent() -> (i32, i32) {
    // Try to get the selected text position using AppleScript
    if let Some(pos) = get_selected_text_position() {
        log::info!("Using AppleScript position: {:?}", pos);
        return pos;
    }

    // Fallback to current mouse position (real-time via CGEvent)
    match get_current_mouse_position_cg() {
        Ok((x, y)) => {
            log::info!("Using current mouse position from CGEvent: ({}, {})", x, y);
            // Position popup slightly to the right and below
            (x + 20, y + 30)
        }
        Err(_) => {
            // Final fallback to tracked position
            let (x, y) = get_tracked_mouse_position();
            log::info!("Using tracked mouse position: ({}, {})", x, y);
            (x + 20, y + 30)
        }
    }
}

/// Placeholder implementations for non-macOS platforms
#[cfg(not(target_os = "macos"))]
pub fn get_selected_text_via_accessibility() -> Result<String> {
    Err(AccessibilityError::Unsupported)
}

#[cfg(not(target_os = "macos"))]
pub fn check_accessibility_permission() -> bool {
    false
}

#[cfg(not(target_os = "macos"))]
pub fn request_accessibility_permission() -> bool {
    false
}

#[cfg(not(target_os = "macos"))]
pub fn get_cursor_position_nsevent() -> (i32, i32) {
    (0, 0)
}
