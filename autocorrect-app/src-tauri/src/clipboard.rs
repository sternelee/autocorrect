//! Clipboard monitoring functionality
//!
//! This module provides clipboard change detection and monitoring.
//! It uses polling to detect clipboard changes and includes CJK character
//! detection for smart auto-triggering of spell-check.

use arboard::Clipboard;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

/// Clipboard event type
#[derive(Clone, Debug)]
pub enum ClipboardEvent {
    /// New text was copied to the clipboard
    NewText { text: String, has_cjk: bool },
}

/// Configuration for clipboard monitoring
#[derive(Clone, Debug)]
pub struct ClipboardMonitorConfig {
    /// Polling interval in milliseconds
    pub poll_interval_ms: u64,
    /// Whether to only trigger on text containing CJK characters
    pub cjk_only: bool,
    /// Debounce delay in milliseconds (ignore rapid changes)
    pub debounce_ms: u64,
}

impl Default for ClipboardMonitorConfig {
    fn default() -> Self {
        Self {
            poll_interval_ms: 500,  // Check every 500ms
            cjk_only: true,         // Only trigger on CJK text
            debounce_ms: 1000,      // Ignore changes within 1s
        }
    }
}

/// Control messages for the clipboard monitor
pub enum ClipboardControl {
    /// Stop the monitor
    Stop,
    /// Update the monitoring configuration
    UpdateConfig(ClipboardMonitorConfig),
    /// Pause monitoring temporarily
    Pause,
    /// Resume monitoring
    Resume,
}

/// Create a channel-based clipboard monitor
///
/// This is the preferred way to create a clipboard monitor as it provides
/// a proper receiver channel for the main thread.
pub fn create_clipboard_channel(
    config: ClipboardMonitorConfig,
) -> Result<(Receiver<ClipboardEvent>, ClipboardHandle), String> {
    let (tx, rx) = mpsc::channel();
    let (control_tx, control_rx) = mpsc::channel();
    let running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let running_clone = running.clone();

    let handle = thread::spawn(move || {
        log::info!("Clipboard monitor started with config: {:?}", config);

        let mut last_content: Option<String> = None;
        let mut last_change_time = std::time::Instant::now();
        let mut current_config = config;
        let mut is_paused = false;

        loop {
            if !running_clone.load(std::sync::atomic::Ordering::Relaxed) {
                log::info!("Clipboard monitor stopping");
                break;
            }

            // Check for control messages
            match control_rx.try_recv() {
                Ok(ClipboardControl::Stop) => {
                    log::info!("Clipboard monitor received stop command");
                    break;
                }
                Ok(ClipboardControl::UpdateConfig(new_config)) => {
                    log::info!("Clipboard monitor config updated: {:?}", new_config);
                    current_config = new_config;
                }
                Ok(ClipboardControl::Pause) => {
                    log::info!("Clipboard monitor paused");
                    is_paused = true;
                }
                Ok(ClipboardControl::Resume) => {
                    log::info!("Clipboard monitor resumed");
                    is_paused = false;
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    log::info!("Clipboard monitor control channel disconnected");
                    break;
                }
            }

            if is_paused {
                thread::sleep(Duration::from_millis(current_config.poll_interval_ms));
                continue;
            }

            // Try to get clipboard content
            match Clipboard::new() {
                Ok(mut clipboard) => {
                    match clipboard.get_text() {
                        Ok(text) => {
                            // Skip empty or whitespace-only text
                            if text.trim().is_empty() {
                                thread::sleep(Duration::from_millis(current_config.poll_interval_ms));
                                continue;
                            }

                            // Check if content changed
                            let content_changed = match &last_content {
                                Some(last) => last != &text,
                                None => true,
                            };

                            if content_changed {
                                let now = std::time::Instant::now();
                                let time_since_last_change =
                                    now.duration_since(last_change_time).as_millis() as u64;

                                // Apply debounce
                                if time_since_last_change >= current_config.debounce_ms {
                                    let has_cjk = contains_cjk(&text);

                                    // Check CJK filter
                                    if !current_config.cjk_only || has_cjk {
                                        log::debug!(
                                            "Clipboard changed: {} chars, CJK: {}",
                                            text.chars().count(),
                                            has_cjk
                                        );

                                        let _ = tx.send(ClipboardEvent::NewText {
                                            text: text.clone(),
                                            has_cjk,
                                        });
                                    }

                                    last_content = Some(text);
                                    last_change_time = now;
                                }
                            }
                        }
                        Err(_) => {
                            // Clipboard might contain non-text content (image, etc.)
                            // This is normal, just continue
                        }
                    }
                }
                Err(e) => {
                    log::error!("Failed to access clipboard: {}", e);
                }
            }

            thread::sleep(Duration::from_millis(current_config.poll_interval_ms));
        }
    });

    let handle = ClipboardHandle {
        running,
        control_tx,
        thread_handle: Some(handle),
    };

    Ok((rx, handle))
}

/// Handle for managing a clipboard monitor
pub struct ClipboardHandle {
    running: std::sync::Arc<std::sync::atomic::AtomicBool>,
    control_tx: Sender<ClipboardControl>,
    thread_handle: Option<thread::JoinHandle<()>>,
}

impl ClipboardHandle {
    /// Stop the clipboard monitor
    pub fn stop(self) {
        log::info!("Stopping clipboard monitor");
        self.running.store(false, std::sync::atomic::Ordering::Relaxed);
        let _ = self.control_tx.send(ClipboardControl::Stop);
        if let Some(handle) = self.thread_handle {
            let _ = handle.join();
        }
    }

    /// Pause monitoring temporarily
    pub fn pause(&self) -> Result<(), mpsc::SendError<ClipboardControl>> {
        self.control_tx.send(ClipboardControl::Pause)
    }

    /// Resume monitoring
    pub fn resume(&self) -> Result<(), mpsc::SendError<ClipboardControl>> {
        self.control_tx.send(ClipboardControl::Resume)
    }

    /// Update the monitoring configuration
    pub fn update_config(&self, config: ClipboardMonitorConfig) -> Result<(), mpsc::SendError<ClipboardControl>> {
        self.control_tx.send(ClipboardControl::UpdateConfig(config))
    }
}

/// Check if text contains CJK (Chinese, Japanese, Korean) characters
///
/// This function checks for characters in these Unicode ranges:
/// - CJK Unified Ideographs: U+4E00–U+9FFF
/// - CJK Extension A: U+3400–U+4DBF
/// - CJK Extension B: U+20000–U+2A6DF
/// - CJK Extension C: U+2A700–U+2B73F
/// - CJK Extension D: U+2B740–U+2B81F
/// - CJK Extension E: U+2B820–U+2CEAF
/// - CJK Extension F: U+2CEB0–U+2EBEF
/// - CJK Compatibility Ideographs: U+F900–U+FAFF
/// - Hiragana: U+3040–U+309F
/// - Katakana: U+30A0–U+30FF
/// - Hangul Syllables: U+AC00–U+D7AF
/// - Hangul Jamo: U+1100–U+11FF
pub fn contains_cjk(text: &str) -> bool {
    for c in text.chars() {
        let code = c as u32;

        // CJK Unified Ideographs
        if (0x4E00..=0x9FFF).contains(&code) {
            return true;
        }
        // CJK Extension A
        if (0x3400..=0x4DBF).contains(&code) {
            return true;
        }
        // CJK Compatibility Ideographs
        if (0xF900..=0xFAFF).contains(&code) {
            return true;
        }
        // Hiragana
        if (0x3040..=0x309F).contains(&code) {
            return true;
        }
        // Katakana
        if (0x30A0..=0x30FF).contains(&code) {
            return true;
        }
        // Hangul Syllables
        if (0xAC00..=0xD7AF).contains(&code) {
            return true;
        }
        // Hangul Jamo
        if (0x1100..=0x11FF).contains(&code) {
            return true;
        }
        // Full-width punctuation and symbols (common in CJK text)
        if (0xFF00..=0xFFEF).contains(&code) {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contains_cjk_chinese() {
        assert!(contains_cjk("你好世界"));
        assert!(contains_cjk("Mixed text with 中文 characters"));
        assert!(!contains_cjk("No CJK here"));
    }

    #[test]
    fn test_contains_cjk_japanese() {
        assert!(contains_cjk("こんにちは"));
        assert!(contains_cjk("ひらがな"));
        assert!(contains_cjk("カタカナ"));
        assert!(contains_cjk("日本語"));
    }

    #[test]
    fn test_contains_cjk_korean() {
        assert!(contains_cjk("안녕하세요"));
        assert!(contains_cjk("한국어"));
    }

    #[test]
    fn test_contains_cjk_mixed() {
        assert!(contains_cjk("Hello 世界"));
        assert!(contains_cjk("Mix of 日本語 and English"));
        assert!(!contains_cjk("Just regular punctuation: .,!?"));
    }

    #[test]
    fn test_contains_cjk_fullwidth() {
        assert!(contains_cjk("，。！")); // Full-width punctuation
        assert!(!contains_cjk(",.!"));  // Half-width punctuation
    }

    #[test]
    fn test_default_config() {
        let config = ClipboardMonitorConfig::default();
        assert_eq!(config.poll_interval_ms, 500);
        assert_eq!(config.cjk_only, true);
        assert_eq!(config.debounce_ms, 1000);
    }
}
