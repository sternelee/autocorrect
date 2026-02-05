//! Shortcut recording functionality
//!
//! This module provides utilities for recording keyboard shortcuts.
//! Due to limitations with multiple rdev event taps, recording is now
//! handled on the frontend side, and this module provides helper functions
//! for formatting and validation.

use crate::hotkey::Modifiers;
use serde::{Deserialize, Serialize};

/// Recorded shortcut information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordedShortcut {
    pub key: String,
    pub modifiers: Modifiers,
    pub display_string: String,
}

impl RecordedShortcut {
    /// Create a new RecordedShortcut from key and modifiers
    pub fn new(key_name: String, modifiers: Modifiers) -> Self {
        let display_string = Self::format_display_string(&modifiers, &key_name);
        Self {
            key: key_name,
            modifiers,
            display_string,
        }
    }

    /// Format the display string for a shortcut
    fn format_display_string(modifiers: &Modifiers, key: &str) -> String {
        let mut parts = Vec::new();

        #[cfg(target_os = "macos")]
        {
            if modifiers.meta {
                parts.push("⌘".to_string());
            }
            if modifiers.shift {
                parts.push("⇧".to_string());
            }
            if modifiers.alt {
                parts.push("⌥".to_string());
            }
            if modifiers.ctrl {
                parts.push("⌃".to_string());
            }
        }

        #[cfg(not(target_os = "macos"))]
        {
            if modifiers.ctrl {
                parts.push("Ctrl".to_string());
            }
            if modifiers.shift {
                parts.push("Shift".to_string());
            }
            if modifiers.alt {
                parts.push("Alt".to_string());
            }
            if modifiers.meta {
                parts.push("Meta".to_string());
            }
        }

        // Format the key name nicely
        let key_label = if key == "Space" {
            "Space".to_string()
        } else if key == "Return" {
            "Return".to_string()
        } else if key == "Tab" {
            "Tab".to_string()
        } else if key == "Backspace" {
            "⌫".to_string()
        } else if key == "Escape" {
            "Esc".to_string()
        } else if key.starts_with("Key") {
            key[3..].to_string()
        } else if key.starts_with("Num") {
            key[3..].to_string()
        } else if key.starts_with("F") && key.len() > 1 {
            key.to_string()
        } else {
            key.to_string()
        };

        parts.push(key_label);
        parts.join("+")
    }
}

/// Validate if a key name is valid
pub fn is_valid_key(key: &str) -> bool {
    let valid_keys = [
        "KeyA", "KeyB", "KeyC", "KeyD", "KeyE", "KeyF", "KeyG", "KeyH", "KeyI", "KeyJ",
        "KeyK", "KeyL", "KeyM", "KeyN", "KeyO", "KeyP", "KeyQ", "KeyR", "KeyS", "KeyT",
        "KeyU", "KeyV", "KeyW", "KeyX", "KeyY", "KeyZ", "Space", "Return", "Tab",
        "Backspace", "Escape", "F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9",
        "F10", "F11", "F12", "Num0", "Num1", "Num2", "Num3", "Num4", "Num5", "Num6",
        "Num7", "Num8", "Num9"
    ];

    valid_keys.contains(&key)
}
