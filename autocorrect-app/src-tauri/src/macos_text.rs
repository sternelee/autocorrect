use cocoa::base::{id, nil};
use core_graphics::display::CGRect;
use objc::{msg_send, sel, sel_impl};
use std::process::Command;
use std::sync::atomic::{AtomicI32, Ordering};

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

#[derive(Debug, Clone)]
pub struct FocusedTextContext {
    pub text: String,
    pub base_offset: usize,  // AX range units (UTF-16 code units)
    pub caret_offset: usize, // caret offset inside `text` (UTF-16 code units)
    pub role: String,
    pub editable: bool,
    pub bundle_id: String,
}

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

        if err_bounds != 0 {
            log::info!("[DIAG] AXBoundsForRange error: {} for range {}-{}", err_bounds, range_start, range_len);
        }

        if err_bounds == 0 && bounds_value != nil {
            let mut rect = CGRect::default();
            if AXValueGetValue(
                bounds_value,
                kAXValueCGRectType,
                &mut rect as *mut _ as *mut std::ffi::c_void,
            ) {
                log::info!("[DIAG] AXBoundsForRange success: {:?}", rect);
                return Ok((full_text, rect));
            }
        }

        Ok((full_text, CGRect::default()))
    }
}

/// 获取焦点输入框文本上下文:
/// 1) 优先 AXValue (全文, base_offset=0)
/// 2) 回退 AXSelectedText + AXSelectedTextRange (部分文本, base_offset=选区起点)
#[cfg(target_os = "macos")]
pub fn get_focused_text_context() -> Result<FocusedTextContext> {
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

        // Get PID and Bundle ID safely
        let mut pid: i32 = 0;
        AXUIElementGetPid(focused_element, &mut pid);
        
        let mut bundle_id = String::new();
        if pid > 0 {
            let app_class = objc::class!(NSRunningApplication);
            let app: id = msg_send![app_class, runningApplicationWithProcessIdentifier: pid];
            if app != nil {
                let ns_bundle_id: id = msg_send![app, bundleIdentifier];
                if ns_bundle_id != nil {
                    bundle_id = from_ax_string(ns_bundle_id);
                }
            }
        }

        let mut role_value: id = nil;
        AXUIElementCopyAttributeValue(focused_element, to_ax_string("AXRole"), &mut role_value);
        let role = if role_value != nil {
            from_ax_string(role_value)
        } else {
            String::new()
        };

        let mut editable_value: id = nil;
        AXUIElementCopyAttributeValue(
            focused_element,
            to_ax_string("AXEditable"),
            &mut editable_value,
        );
        let editable = from_ax_bool(editable_value);

        // 优先全文 (AXValue)
        let mut text_value: id = nil;
        AXUIElementCopyAttributeValue(focused_element, to_ax_string("AXValue"), &mut text_value);
        
        // Slack/Electron fallback (AXSelectedText)
        if text_value == nil {
            AXUIElementCopyAttributeValue(focused_element, to_ax_string("AXSelectedText"), &mut text_value);
        }

        if text_value != nil {
            // Verify if the value is actually an NSString
            let ns_string_class: id = msg_send![objc::class!(NSString), class];
            let is_string: bool = msg_send![text_value, isKindOfClass: ns_string_class];
            
            if is_string {
                let full_text = from_ax_string(text_value);
                if !full_text.is_empty() {
                let mut selected_range_value: id = nil;
                AXUIElementCopyAttributeValue(
                    focused_element,
                    to_ax_string("AXSelectedTextRange"),
                    &mut selected_range_value,
                );
                
                let mut selected_range = CFRange { location: 0, length: 0 };
                if selected_range_value != nil {
                    let _ = AXValueGetValue(
                        selected_range_value,
                        kAXValueCFRangeType,
                        &mut selected_range as *mut _ as *mut std::ffi::c_void,
                    );
                }

                let total_u16 = full_text.encode_utf16().count();
                if total_u16 > 20000 {
                    let caret_u16 = if selected_range.location >= 0 { selected_range.location as usize } else { total_u16 };
                    let start_u16 = caret_u16.saturating_sub(3000);
                    let end_u16 = (caret_u16 + 1000).min(total_u16);
                    let sliced = slice_by_utf16_range(&full_text, start_u16, end_u16);
                    return Ok(FocusedTextContext {
                        text: sliced,
                        base_offset: start_u16,
                        caret_offset: caret_u16.saturating_sub(start_u16),
                        role,
                        editable,
                        bundle_id,
                    });
                }

                return Ok(FocusedTextContext {
                    text: full_text,
                    base_offset: 0,
                    caret_offset: selected_range.location.max(0) as usize,
                    role,
                    editable,
                    bundle_id,
                });
            }
        }
    }

    Ok(FocusedTextContext {
        text: String::new(),
        base_offset: 0,
        caret_offset: 0,
        role,
        editable,
        bundle_id,
    })
}
}

/// 仅获取焦点输入框指定范围屏幕坐标
#[cfg(target_os = "macos")]
pub fn get_focused_range_bounds(range_start: usize, range_len: usize) -> Result<CGRect> {
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

        if err_bounds != 0 {
            log::info!("[DIAG] AXBoundsForRange error: {} for range {}-{}", err_bounds, range_start, range_len);
        }

        if err_bounds == 0 && bounds_value != nil {
            let mut rect = CGRect::default();
            if AXValueGetValue(
                bounds_value,
                kAXValueCGRectType,
                &mut rect as *mut _ as *mut std::ffi::c_void,
            ) {
                log::info!("[DIAG] AXBoundsForRange success: {:?}", rect);
                return Ok(rect);
            }
        }

        log::info!("[DIAG] AXBoundsForRange failed, returning default rect");
        Ok(CGRect::default())
    }
}

/// 获取当前焦点输入框光标(caret)附近坐标，用于不支持 AXBoundsForRange 的降级渲染。
#[cfg(target_os = "macos")]
pub fn get_focused_caret_bounds() -> Result<CGRect> {
    log::info!("[DIAG] get_focused_caret_bounds called");
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
            log::warn!("[DIAG] get_focused_caret_bounds: no focused element");
            return Err(AccessibilityError::NoFocusedElement);
        }

        let mut selected_range_value: id = nil;
        AXUIElementCopyAttributeValue(
            focused_element,
            to_ax_string("AXSelectedTextRange"),
            &mut selected_range_value,
        );

        let mut selected_range = CFRange {
            location: 0,
            length: 0,
        };
        if selected_range_value != nil {
            let _ = AXValueGetValue(
                selected_range_value,
                kAXValueCFRangeType,
                &mut selected_range as *mut _ as *mut std::ffi::c_void,
            );
        }

        let caret_location = selected_range.location.max(0) as usize;
        log::info!(
            "[DIAG] get_focused_caret_bounds: caret_location={}",
            caret_location
        );
        // Ask 1-char range near caret; many apps return a caret-adjacent rect for this.
        let rect = get_focused_range_bounds(caret_location, 1)?;
        log::info!("[DIAG] get_focused_caret_bounds result: {:?}", rect);
        Ok(rect)
    }
}

/// 获取焦点输入控件整体边界，用于行级降级渲染。
#[cfg(target_os = "macos")]
pub fn get_focused_element_bounds() -> Result<CGRect> {
    log::info!("[DIAG] get_focused_element_bounds called");
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
            log::warn!("[DIAG] get_focused_element_bounds: no focused element");
            return Err(AccessibilityError::NoFocusedElement);
        }

        let mut frame_value: id = nil;
        AXUIElementCopyAttributeValue(focused_element, to_ax_string("AXFrame"), &mut frame_value);
        log::info!(
            "[DIAG] get_focused_element_bounds: AXFrame available={}",
            frame_value != nil
        );
        if frame_value != nil {
            let mut rect = CGRect::default();
            if AXValueGetValue(
                frame_value,
                kAXValueCGRectType,
                &mut rect as *mut _ as *mut std::ffi::c_void,
            ) {
                log::info!("[DIAG] get_focused_element_bounds result: {:?}", rect);
                return Ok(rect);
            }
        }

        log::info!("[DIAG] get_focused_element_bounds: AXFrame unavailable, returning default");
        Ok(CGRect::default())
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
    fn AXUIElementSetAttributeValue(element: id, attribute: id, value: id) -> i32;
    fn AXUIElementGetPid(element: id, pid: *mut i32) -> i32;
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

fn from_ax_bool(ns_value: id) -> bool {
    unsafe {
        if ns_value == nil {
            return false;
        }
        // Use isKindOfClass: to verify it's a number/boolean before calling charValue
        let ns_number_class: id = msg_send![objc::class!(NSNumber), class];
        let is_number: bool = msg_send![ns_value, isKindOfClass: ns_number_class];
        if is_number {
            let val: i8 = msg_send![ns_value, charValue];
            val != 0
        } else {
            false
        }
    }
}

fn slice_by_utf16_range(s: &str, start_u16: usize, end_u16: usize) -> String {
    if start_u16 >= end_u16 {
        return String::new();
    }

    let mut start_byte = 0usize;
    let mut end_byte = s.len();
    let mut u16_pos = 0usize;
    let mut found_start = false;
    let mut found_end = false;

    for (byte_idx, ch) in s.char_indices() {
        if !found_start && u16_pos >= start_u16 {
            start_byte = byte_idx;
            found_start = true;
        }
        if !found_end && u16_pos >= end_u16 {
            end_byte = byte_idx;
            found_end = true;
            break;
        }
        u16_pos += ch.len_utf16();
    }

    if !found_start {
        start_byte = s.len();
    }
    if !found_end && u16_pos < end_u16 {
        end_byte = s.len();
    }

    if start_byte >= end_byte {
        return String::new();
    }

    s[start_byte..end_byte].to_string()
}

/// 设置焦点输入框的选中文本范围
#[cfg(target_os = "macos")]
pub fn select_text_range(start: usize, length: usize) -> Result<()> {
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

        let range = CFRange {
            location: start as i64,
            length: length as i64,
        };
        let ax_range = AXValueCreate(
            kAXValueCFRangeType,
            &range as *const _ as *const std::ffi::c_void,
        );

        let err_set = AXUIElementSetAttributeValue(
            focused_element,
            to_ax_string("AXSelectedTextRange"),
            ax_range,
        );

        if err_set != 0 {
            log::warn!("[DIAG] select_text_range failed with err={}", err_set);
            return Err(AccessibilityError::ApiError(format!(
                "Failed to set selected text range: {}",
                err_set
            )));
        }

        Ok(())
    }
}

// Use CGEvent to get accurate global mouse position
pub fn update_mouse_position(_x: i32, _y: i32) {
    // No-op: we now fetch dynamically
}

pub fn get_cursor_position_nsevent() -> (i32, i32) {
    #[cfg(target_os = "macos")]
    {
        use core_graphics::event::CGEvent;
        use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
        if let Ok(source) = CGEventSource::new(CGEventSourceStateID::CombinedSessionState) {
            if let Ok(event) = CGEvent::new(source) {
                let point = event.location();
                return (point.x as i32, point.y as i32);
            }
        }
    }
    (0, 0)
}
