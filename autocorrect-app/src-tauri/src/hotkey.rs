//! Global hotkey management using rdev
//!
//! This module provides cross-platform global hotkey listening functionality.
//! It runs in a separate thread and communicates with the main thread via channels.

use rdev::{Event, EventType, Key};
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
#[derive(Clone, Default, Debug)]
struct Modifiers {
    shift: bool,
    ctrl: bool,
    meta: bool, // Command on macOS, Windows key on Windows
    alt: bool,
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
#[derive(Clone, Debug)]
pub struct HotkeyConfig {
    /// The key to listen for (e.g., Key::KeyA)
    pub key: Key,
    /// Required modifier states
    pub modifiers: Modifiers,
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
            modifiers,
        }
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
/// ```no_run
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

        let callback = move |event: Event| {
            if !running_clone.load(std::sync::atomic::Ordering::Relaxed) {
                return;
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
