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
/// Shared, mutable `HotkeyConfig` used by the running rdev listener.
///
/// The listener clones the currently bound key/modifiers on every event
/// so that updates through `HotkeyConfigCell` (e.g. from
/// `update_hotkey_config`) take effect without restarting the listener
/// thread. Manage a `HotkeyConfigCell` in Tauri App state so commands
/// can write through to it.
pub type SharedHotkeyConfig = std::sync::Arc<std::sync::Mutex<HotkeyConfig>>;

pub struct HotkeyConfigCell(pub SharedHotkeyConfig);

/// Convenience constructor for the shared hotkey config.
pub fn shared_hotkey_config(config: HotkeyConfig) -> SharedHotkeyConfig {
    std::sync::Arc::new(std::sync::Mutex::new(config))
}

/// Calibrate the tracked modifier state from the actual CGEventSource flags
/// so hotkey decisions use the real current keyboard state.
///
/// Press/release counting desyncs when events are dropped (focus switches,
/// event-tap hiccups), leaving "sticky" modifiers that block or mis-trigger
/// the hotkey. Querying the HID flags on every event makes the state
/// self-healing.
#[cfg(target_os = "macos")]
fn calibrate_modifiers_from_cg(modifiers: &mut Modifiers) {
    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGEventSourceFlagsState(state_id: i32) -> u64;
    }

    // kCGEventSourceStateHIDSystemState
    const K_CG_EVENT_SOURCE_STATE_HID_SYSTEM_STATE: i32 = 1;
    // NX_* masks from IOKit hidsystem/IOLLEvent.h (match kCGEventFlagMask*).
    const NX_SHIFTMASK: u64 = 0x0002_0000;
    const NX_CONTROLMASK: u64 = 0x0004_0000;
    const NX_ALTERNATEMASK: u64 = 0x0008_0000;
    const NX_COMMANDMASK: u64 = 0x0010_0000;

    let flags = unsafe { CGEventSourceFlagsState(K_CG_EVENT_SOURCE_STATE_HID_SYSTEM_STATE) };
    modifiers.shift = flags & NX_SHIFTMASK != 0;
    modifiers.ctrl = flags & NX_CONTROLMASK != 0;
    modifiers.alt = flags & NX_ALTERNATEMASK != 0;
    modifiers.meta = flags & NX_COMMANDMASK != 0;
}

/// # Arguments
/// * `config_cell` - Shared, mutable hotkey configuration. The listener
///   reads from it on every key event so updates take effect without a
///   restart.
///
/// # Returns
/// A tuple containing:
/// - A receiver that yields hotkey events
/// - A handle that can be used to stop the listener
///
/// # Example
/// ```ignore
/// use autocorrect_app_lib::hotkey::{create_hotkey_channel, shared_hotkey_config, HotkeyConfig};
/// use rdev::Key;
///
/// let shared = shared_hotkey_config(HotkeyConfig {
///     key: Key::KeyA,
///     ..Default::default()
/// });
/// let (rx, handle) = create_hotkey_channel(shared);
/// ```
pub fn create_hotkey_channel(
    config_cell: SharedHotkeyConfig,
) -> (Receiver<HotkeyEvent>, HotkeyHandle) {
    let (tx, rx) = mpsc::channel();
    let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let running_clone = running.clone();

    let config_clone = config_cell.clone();
    let handle = thread::spawn(move || {
        log::info!(
            "Hotkey listener started with config: {}",
            config_clone
                .lock()
                .map(|c| c.to_display_string())
                .unwrap_or_else(|_| "<poisoned>".to_string())
        );

        #[cfg(target_os = "macos")]
        use crate::macos_text::update_mouse_position;

        // rdev::listen blocks; it returns when the CGEventTap is disabled
        // (e.g. a timeout, or a temporary Accessibility permission hiccup).
        // Rebuild the listener instead of letting the hotkey die silently.
        loop {
            if !running_clone.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }

            // Fresh state per (re)start; CG flag calibration below fixes any
            // drift, so resetting the tally on restart is harmless.
            // Wrap modifiers in a Mutex for safe access across callbacks.
            let modifiers = std::sync::Arc::new(Mutex::new(Modifiers::default()));
            let modifiers_clone = modifiers.clone();
            let tx = tx.clone();
            let running_cb = running_clone.clone();
            let config_cb = config_clone.clone();

            let callback = move |event: Event| {
                if !running_cb.load(std::sync::atomic::Ordering::Relaxed) {
                    return;
                }

                // Track mouse position on mouse move events
                #[cfg(target_os = "macos")]
                {
                    if let EventType::MouseMove { x, y, .. } = event.event_type {
                        update_mouse_position(x as i32, y as i32);
                    }
                }

                // Snapshot the currently active hotkey binding before matching.
                // This makes config swaps from `update_hotkey_config` visible
                // without restarting the listener thread.
                let (cfg_key, cfg_modifiers) = match config_cb.lock() {
                    Ok(guard) => (guard.key, guard.modifiers.clone()),
                    Err(poisoned) => {
                        log::error!("Hotkey config lock poisoned; recovering");
                        let guard = poisoned.into_inner();
                        (guard.key, guard.modifiers.clone())
                    }
                };

                // Lock the mutex to safely modify the modifiers state
                if let Ok(mut modifiers_guard) = modifiers_clone.lock() {
                    // Trust the real keyboard state over our press/release
                    // tally, which desyncs when events are dropped (focus
                    // switches, event-tap hiccups). This runs on every
                    // event, so stale "sticky modifier" states self-heal.
                    #[cfg(target_os = "macos")]
                    calibrate_modifiers_from_cg(&mut modifiers_guard);

                    match event.event_type {
                        EventType::KeyPress(key) => {
                            // Update modifier state on key press
                            match key {
                                Key::ShiftLeft | Key::ShiftRight => modifiers_guard.shift = true,
                                Key::ControlLeft | Key::ControlRight => modifiers_guard.ctrl = true,
                                Key::MetaLeft | Key::MetaRight => modifiers_guard.meta = true,
                                // On macOS the rdev fork (fufesou/rdev) maps the
                                // left Option to `Key::Alt` and the right Option
                                // to `Key::AltGr`. Treat both as the Alt modifier
                                // so users pressing either Option key trigger
                                // the configured hotkey.
                                Key::Alt | Key::AltGr => modifiers_guard.alt = true,
                                _ => {
                                    // Check if this is our hotkey combination
                                    if key == cfg_key
                                        && modifiers_guard.has_required(&cfg_modifiers)
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
                                Key::ControlLeft | Key::ControlRight => {
                                    modifiers_guard.ctrl = false
                                }
                                Key::MetaLeft | Key::MetaRight => modifiers_guard.meta = false,
                                Key::Alt | Key::AltGr => modifiers_guard.alt = false,
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }
            };

            // Start listening for events
            // Note: rdev::listen blocks until an error occurs or the process exits
            match rdev::listen(callback) {
                Err(e) => {
                    log::error!("Hotkey listener error: {:?}; restarting in 1s", e);
                    thread::sleep(std::time::Duration::from_secs(1));
                }
                Ok(()) => {
                    log::info!("Hotkey listener stopped cleanly");
                    break;
                }
            }
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
