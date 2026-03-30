//! Global hotkey management using rdev
//!
//! This module provides cross-platform global hotkey listening functionality.
//! It runs in a separate thread and communicates with the main thread via channels.

use rdev::{Event, EventType, Key};
use serde::{Deserialize, Deserializer, Serialize};
use std::sync::mpsc::{self, Receiver};
use std::sync::Mutex;
use std::thread;

/// Hotkey event type
#[derive(Clone, Debug)]
pub enum HotkeyEvent {
    /// The spell-check hotkey was triggered
    SpellCheckTriggered,
}

/// State tracking for modifier keys
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub meta: bool, // Command on macOS, Windows key on Windows
    pub alt: bool,
}

impl Modifiers {
    /// Check if all required modifiers are pressed
    fn has_required(&self, required: &Modifiers) -> bool {
        self.shift == required.shift
            && self.ctrl == required.ctrl
            && self.meta == required.meta
            && self.alt == required.alt
    }
}

/// Hotkey configuration
#[derive(Clone, Debug, Serialize)]
pub struct HotkeyConfig {
    /// The key to listen for (e.g., Key::KeyA)
    #[serde(skip)]
    pub key: Key,
    /// Key name for serialization (e.g., "KeyA", "Space", "Return")
    #[serde(rename = "key")]
    pub key_name: String,
    /// Required modifier states
    pub modifiers: Modifiers,
}

// Custom deserialization for HotkeyConfig
impl<'de> Deserialize<'de> for HotkeyConfig {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct HotkeyConfigRaw {
            key: String,
            modifiers: Modifiers,
        }

        let raw = HotkeyConfigRaw::deserialize(deserializer)?;
        let key = Self::key_from_name(&raw.key);

        Ok(Self {
            key,
            key_name: raw.key,
            modifiers: raw.modifiers,
        })
    }
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        #[cfg(target_os = "macos")]
        let modifiers = Modifiers {
            shift: true,
            ctrl: false,
            meta: true, // Cmd
            alt: false,
        };

        #[cfg(not(target_os = "macos"))]
        let modifiers = Modifiers {
            shift: true,
            ctrl: true, // Ctrl
            meta: false,
            alt: false,
        };

        Self {
            key: Key::KeyA,
            key_name: "KeyA".to_string(),
            modifiers,
        }
    }
}

impl HotkeyConfig {
    /// Create a new HotkeyConfig from key name and modifiers
    pub fn new(key_name: String, modifiers: Modifiers) -> Self {
        let key = Self::key_from_name(&key_name);
        Self {
            key,
            key_name,
            modifiers,
        }
    }

    /// Convert rdev Key to string representation
    pub fn key_to_name(key: Key) -> String {
        match key {
            Key::KeyA => "KeyA".to_string(),
            Key::KeyB => "KeyB".to_string(),
            Key::KeyC => "KeyC".to_string(),
            Key::KeyD => "KeyD".to_string(),
            Key::KeyE => "KeyE".to_string(),
            Key::KeyF => "KeyF".to_string(),
            Key::KeyG => "KeyG".to_string(),
            Key::KeyH => "KeyH".to_string(),
            Key::KeyI => "KeyI".to_string(),
            Key::KeyJ => "KeyJ".to_string(),
            Key::KeyK => "KeyK".to_string(),
            Key::KeyL => "KeyL".to_string(),
            Key::KeyM => "KeyM".to_string(),
            Key::KeyN => "KeyN".to_string(),
            Key::KeyO => "KeyO".to_string(),
            Key::KeyP => "KeyP".to_string(),
            Key::KeyQ => "KeyQ".to_string(),
            Key::KeyR => "KeyR".to_string(),
            Key::KeyS => "KeyS".to_string(),
            Key::KeyT => "KeyT".to_string(),
            Key::KeyU => "KeyU".to_string(),
            Key::KeyV => "KeyV".to_string(),
            Key::KeyW => "KeyW".to_string(),
            Key::KeyX => "KeyX".to_string(),
            Key::KeyY => "KeyY".to_string(),
            Key::KeyZ => "KeyZ".to_string(),
            Key::Space => "Space".to_string(),
            Key::Return => "Return".to_string(),
            Key::Tab => "Tab".to_string(),
            Key::Backspace => "Backspace".to_string(),
            Key::Escape => "Escape".to_string(),
            _ => "Unknown".to_string(),
        }
    }

    /// Convert string representation to rdev Key
    pub fn key_from_name(name: &str) -> Key {
        match name {
            "KeyA" => Key::KeyA,
            "KeyB" => Key::KeyB,
            "KeyC" => Key::KeyC,
            "KeyD" => Key::KeyD,
            "KeyE" => Key::KeyE,
            "KeyF" => Key::KeyF,
            "KeyG" => Key::KeyG,
            "KeyH" => Key::KeyH,
            "KeyI" => Key::KeyI,
            "KeyJ" => Key::KeyJ,
            "KeyK" => Key::KeyK,
            "KeyL" => Key::KeyL,
            "KeyM" => Key::KeyM,
            "KeyN" => Key::KeyN,
            "KeyO" => Key::KeyO,
            "KeyP" => Key::KeyP,
            "KeyQ" => Key::KeyQ,
            "KeyR" => Key::KeyR,
            "KeyS" => Key::KeyS,
            "KeyT" => Key::KeyT,
            "KeyU" => Key::KeyU,
            "KeyV" => Key::KeyV,
            "KeyW" => Key::KeyW,
            "KeyX" => Key::KeyX,
            "KeyY" => Key::KeyY,
            "KeyZ" => Key::KeyZ,
            "Space" => Key::Space,
            "Return" => Key::Return,
            "Tab" => Key::Tab,
            "Backspace" => Key::Backspace,
            "Escape" => Key::Escape,
            _ => Key::KeyA, // Default fallback
        }
    }

    /// Get a human-readable display string for this hotkey
    pub fn to_display_string(&self) -> String {
        let mut parts = Vec::new();

        #[cfg(target_os = "macos")]
        {
            if self.modifiers.meta {
                parts.push("⌘".to_string());
            }
            if self.modifiers.shift {
                parts.push("⇧".to_string());
            }
            if self.modifiers.alt {
                parts.push("⌥".to_string());
            }
            if self.modifiers.ctrl {
                parts.push("⌃".to_string());
            }
        }

        #[cfg(not(target_os = "macos"))]
        {
            if self.modifiers.ctrl {
                parts.push("Ctrl".to_string());
            }
            if self.modifiers.shift {
                parts.push("Shift".to_string());
            }
            if self.modifiers.alt {
                parts.push("Alt".to_string());
            }
            if self.modifiers.meta {
                parts.push("Meta".to_string());
            }
        }

        parts.push(self.key_name.clone());
        parts.join("+")
    }

    /// Update the key from key_name (call this after deserializing)
    pub fn sync_key(&mut self) {
        self.key = Self::key_from_name(&self.key_name);
    }
}

/// Create a channel-based hotkey listener
///
/// This is the preferred way to create a hotkey listener as it provides
/// a proper receiver channel for the main thread.
///
/// # Arguments
/// * `config` - Hotkey configuration specifying the key combination to listen for
///
/// # Returns
/// A tuple containing:
/// - A receiver that yields hotkey events
/// - A handle that can be used to stop the listener
///
/// # Example
/// ```ignore
/// use autocorrect_app_lib::hotkey::{create_hotkey_channel, HotkeyConfig};
/// use rdev::Key;
///
/// let config = HotkeyConfig {
///     key: Key::KeyA,
///     ..Default::default()
/// };
///
/// let (rx, handle) = create_hotkey_channel(config);
/// ```
pub fn create_hotkey_channel(config: HotkeyConfig) -> (Receiver<HotkeyEvent>, HotkeyHandle) {
    let (tx, rx) = mpsc::channel();
    let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let running_clone = running.clone();

    // Wrap modifiers in a Mutex to allow safe mutable access across callback invocations
    let modifiers = std::sync::Arc::new(Mutex::new(Modifiers::default()));
    let modifiers_clone = modifiers.clone();

    let handle = thread::spawn(move || {
        log::info!("Hotkey listener started with config: {:?}", config);

        #[cfg(target_os = "macos")]
        use crate::macos_text::update_mouse_position;

        let callback = move |event: Event| {
            if !running_clone.load(std::sync::atomic::Ordering::Relaxed) {
                return;
            }

            // Track mouse position on mouse move events
            #[cfg(target_os = "macos")]
            {
                if let EventType::MouseMove { x, y, .. } = event.event_type {
                    update_mouse_position(x as i32, y as i32);
                }
            }

            // Lock the mutex to safely modify the modifiers state
            if let Ok(mut modifiers_guard) = modifiers_clone.lock() {
                match event.event_type {
                    EventType::KeyPress(key) => {
                        // Update modifier state on key press
                        match key {
                            Key::ShiftLeft | Key::ShiftRight => modifiers_guard.shift = true,
                            Key::ControlLeft | Key::ControlRight => modifiers_guard.ctrl = true,
                            Key::MetaLeft | Key::MetaRight => modifiers_guard.meta = true,
                            Key::Alt => modifiers_guard.alt = true,
                            _ => {
                                // Check if this is our hotkey combination
                                if key == config.key
                                    && modifiers_guard.has_required(&config.modifiers)
                                {
                                    log::debug!("Hotkey triggered: {:?}", key);
                                    let _ = tx.send(HotkeyEvent::SpellCheckTriggered);
                                }
                            }
                        }
                    }
                    EventType::KeyRelease(key) => {
                        // Update modifier state on key release
                        match key {
                            Key::ShiftLeft | Key::ShiftRight => modifiers_guard.shift = false,
                            Key::ControlLeft | Key::ControlRight => modifiers_guard.ctrl = false,
                            Key::MetaLeft | Key::MetaRight => modifiers_guard.meta = false,
                            Key::Alt => modifiers_guard.alt = false,
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        };

        // Start listening for events
        // Note: rdev::listen blocks until an error occurs or the process exits
        if let Err(e) = rdev::listen(callback) {
            log::error!("Hotkey listener error: {:?}", e);
        }
    });

    let handle = HotkeyHandle {
        running,
        thread_handle: Some(handle),
    };

    (rx, handle)
}

/// Handle for managing a hotkey listener
pub struct HotkeyHandle {
    running: std::sync::Arc<std::sync::atomic::AtomicBool>,
    thread_handle: Option<thread::JoinHandle<()>>,
}

impl HotkeyHandle {
    /// Stop the hotkey listener
    ///
    /// This sets the running flag to false and waits for the listener thread to finish.
    /// Note that rdev::listen is a blocking call, so this may not immediately stop
    /// the listener. The listener will exit when rdev::listen returns.
    pub fn stop(self) {
        log::info!("Stopping hotkey listener");
        self.running
            .store(false, std::sync::atomic::Ordering::Relaxed);
        if let Some(handle) = self.thread_handle {
            let _ = handle.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = HotkeyConfig::default();
        assert_eq!(config.key, Key::KeyA);

        #[cfg(target_os = "macos")]
        assert!(config.modifiers.meta && config.modifiers.shift);

        #[cfg(not(target_os = "macos"))]
        assert!(config.modifiers.ctrl && config.modifiers.shift);
    }

    #[test]
    fn test_modifiers_check() {
        let required = Modifiers {
            shift: true,
            ctrl: true,
            meta: false,
            alt: false,
        };

        let current = Modifiers {
            shift: true,
            ctrl: true,
            meta: false,
            alt: false,
        };

        assert!(current.has_required(&required));

        let wrong = Modifiers {
            shift: false,
            ctrl: true,
            meta: false,
            alt: false,
        };

        assert!(!wrong.has_required(&required));
    }
}
