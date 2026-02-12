use cocoa::base::{id, nil};
use core_graphics::display::CGRect;
use objc::{msg_send, sel, sel_impl};
use std::process::Command;
use std::sync::atomic::{AtomicI32, Ordering};
use std::thread;
use std::time::Duration;

/// Accessibility API Error
#[derive(Debug, thiserror::Error)]
pub enum AccessibilityError {
    #[error("No text selected")]
    NoTextSelected,
    #[error("Accessibility permission denied")]
    PermissionDenied,
    #[error("Failed to get focused element")]
    NoFocusedElement,
    #[error("API error: {0}")]
    ApiError(String),
    #[error("Unsupported operation")]
    Unsupported,
    #[error("Keyboard simulation failed: {0}")]
    KeyboardError(String),
    #[error("Clipboard error: {0}")]
    ClipboardError(String),
    #[error("AppleScript execution failed: {0}")]
    AppleScriptError(String),
}

pub type Result<T> = std::result::Result<T, AccessibilityError>;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXUIElementCreateSystemWide() -> id;
    fn AXIsProcessTrusted() -> bool;
}

/// 检查并请求辅助功能权限
pub fn check_and_request_accessibility() -> bool {
    unsafe {
        if AXIsProcessTrusted() {
            return true;
        }
        // 如果没有权限，可以通过脚本触发系统弹窗
        let _ = Command::new("osascript")
            .arg("-e")
            .arg("tell application \"System Preferences\" to activate")
            .spawn();
        false
    }
}

/// 获取当前焦点输入框的文本及特定范围的坐标
#[cfg(target_os = "macos")]
pub fn get_focused_element_data(range_start: usize, range_len: usize) -> Result<(String, CGRect)> {
    if !unsafe { AXIsProcessTrusted() } {
        return Err(AccessibilityError::PermissionDenied);
    }

    unsafe {
        let system_element = AXUIElementCreateSystemWide();
        let mut focused_element: id = nil;

        // 1. 获取焦点元素
        let k_ax_focused_ui_element_attribute = "AXFocusedUIElement";
        let err = AXUIElementCopyAttributeValue(
            system_element,
            to_ax_string(k_ax_focused_ui_element_attribute),
            &mut focused_element,
        );

        if err != 0 || focused_element == nil {
            return Err(AccessibilityError::NoFocusedElement);
        }

        // 2. 获取全文 (用于校验 offset)
        let mut text_value: id = nil;
        AXUIElementCopyAttributeValue(focused_element, to_ax_string("AXValue"), &mut text_value);
        let full_text = if text_value != nil {
            from_ax_string(text_value)
        } else {
            String::new()
        };

        // 3. 获取指定范围的屏幕坐标 (NSRect)
        let mut bounds_value: id = nil;
        let range = CFRange {
            location: range_start as i64,
            length: range_len as i64,
        };
        let ax_range = AXValueCreate(
            kAXValueCFRangeType,
            &range as *const _ as *const std::ffi::c_void,
        );

        let err_bounds = AXUIElementCopyParameterizedAttributeValue(
            focused_element,
            to_ax_string("AXBoundsForRange"),
            ax_range,
            &mut bounds_value,
        );

        if err_bounds == 0 && bounds_value != nil {
            let mut rect = CGRect::default();
            if AXValueGetValue(
                bounds_value,
                kAXValueCGRectType,
                &mut rect as *mut _ as *mut std::ffi::c_void,
            ) {
                return Ok((full_text, rect));
            }
        }

        Ok((full_text, CGRect::default()))
    }
}

/// 获取当前选中的文本（优先 AXSelectedText）
#[cfg(target_os = "macos")]
pub fn get_selected_text() -> Result<String> {
    if !unsafe { AXIsProcessTrusted() } {
        return Err(AccessibilityError::PermissionDenied);
    }

    unsafe {
        let system_element = AXUIElementCreateSystemWide();
        let mut focused_element: id = nil;

        let err = AXUIElementCopyAttributeValue(
            system_element,
            to_ax_string("AXFocusedUIElement"),
            &mut focused_element,
        );

        if err != 0 || focused_element == nil {
            return Err(AccessibilityError::NoFocusedElement);
        }

        let mut selected_text: id = nil;
        let err_selected = AXUIElementCopyAttributeValue(
            focused_element,
            to_ax_string("AXSelectedText"),
            &mut selected_text,
        );

        if err_selected != 0 || selected_text == nil {
            return Err(AccessibilityError::NoTextSelected);
        }

        let text = from_ax_string(selected_text);
        if text.trim().is_empty() {
            return Err(AccessibilityError::NoTextSelected);
        }

        Ok(text)
    }
}

// --- macOS 底层辅助函数 ---

#[repr(C)]
struct CFRange {
    location: i64,
    length: i64,
}

const kAXValueCFRangeType: i32 = 4;
const kAXValueCGRectType: i32 = 3;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXUIElementCopyAttributeValue(element: id, attribute: id, value: *mut id) -> i32;
    fn AXUIElementCopyParameterizedAttributeValue(
        element: id,
        attribute: id,
        parameter: id,
        value: *mut id,
    ) -> i32;
    fn AXValueCreate(the_type: i32, value_ptr: *const std::ffi::c_void) -> id;
    fn AXValueGetValue(value: id, the_type: i32, value_ptr: *mut std::ffi::c_void) -> bool;
}

fn to_ax_string(s: &str) -> id {
    unsafe {
        let c_string = match std::ffi::CString::new(s) {
            Ok(v) => v,
            Err(_) => return nil,
        };
        let ns_string: id =
            msg_send![objc::class!(NSString), stringWithUTF8String: c_string.as_ptr()];
        ns_string
    }
}

fn from_ax_string(ns_string: id) -> String {
    unsafe {
        let c_str: *const std::os::raw::c_char = msg_send![ns_string, UTF8String];
        if c_str.is_null() {
            String::new()
        } else {
            std::ffi::CStr::from_ptr(c_str)
                .to_string_lossy()
                .into_owned()
        }
    }
}

// 保留原有的 MOUSE 追踪逻辑以便兼容
static MOUSE_X: AtomicI32 = AtomicI32::new(800);
static MOUSE_Y: AtomicI32 = AtomicI32::new(400);

pub fn update_mouse_position(x: i32, y: i32) {
    MOUSE_X.store(x, Ordering::Relaxed);
    MOUSE_Y.store(y, Ordering::Relaxed);
}

pub fn get_cursor_position_nsevent() -> (i32, i32) {
    let x = MOUSE_X.load(Ordering::Relaxed);
    let y = MOUSE_Y.load(Ordering::Relaxed);
    (x, y)
}
