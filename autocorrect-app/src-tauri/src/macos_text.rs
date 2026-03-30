#![allow(unexpected_cfgs)]

use core_graphics::display::CGRect;
use objc2::msg_send;
use objc2::runtime::{AnyClass, AnyObject};

// Type alias for Objective-C object pointers (Accessibility API uses raw pointers)
type Id = *mut AnyObject;
const NIL: Id = std::ptr::null_mut();

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
    fn AXUIElementCreateSystemWide() -> Id;
    fn AXIsProcessTrusted() -> bool;
}

/// Returns whether the process currently has Accessibility permission.
pub fn check_accessibility_trusted() -> bool {
    unsafe { AXIsProcessTrusted() }
}

/// Open the Accessibility pane of System Settings so the user can enable
/// the app manually, then restart.  Works on macOS 13 Ventura and later
/// (x-apple.systempreferences URL) and falls back to the legacy URL for
/// older versions.
pub fn open_accessibility_settings() {
    // macOS 13+ uses "System Settings"; older versions use "System Preferences".
    let _ = std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
        .spawn();
}

/// 获取当前焦点输入框的文本及特定范围的坐标
#[cfg(target_os = "macos")]
pub fn get_focused_element_data(range_start: usize, range_len: usize) -> Result<(String, CGRect)> {
    if !unsafe { AXIsProcessTrusted() } {
        return Err(AccessibilityError::PermissionDenied);
    }

    unsafe {
        let system_element = AXUIElementCreateSystemWide();
        let mut focused_element: Id = NIL;

        // 1. 获取焦点元素
        let k_ax_focused_ui_element_attribute = "AXFocusedUIElement";
        let err = AXUIElementCopyAttributeValue(
            system_element,
            to_ax_string(k_ax_focused_ui_element_attribute),
            &mut focused_element,
        );

        if err != 0 || focused_element.is_null() {
            return Err(AccessibilityError::NoFocusedElement);
        }

        // 2. 获取全文 (用于校验 offset)
        let mut text_value: Id = NIL;
        AXUIElementCopyAttributeValue(focused_element, to_ax_string("AXValue"), &mut text_value);
        let full_text = if !text_value.is_null() {
            from_ax_string(text_value)
        } else {
            String::new()
        };

        // 3. 获取指定范围的屏幕坐标 (NSRect)
        let mut bounds_value: Id = NIL;
        let range = CFRange {
            location: range_start as i64,
            length: range_len as i64,
        };
        let ax_range = AXValueCreate(
            K_AXVALUE_CFRANGE_TYPE,
            &range as *const _ as *const std::ffi::c_void,
        );

        let err_bounds = AXUIElementCopyParameterizedAttributeValue(
            focused_element,
            to_ax_string("AXBoundsForRange"),
            ax_range,
            &mut bounds_value,
        );

        if err_bounds != 0 {
            log::info!(
                "[DIAG] AXBoundsForRange error: {} for range {}-{}",
                err_bounds,
                range_start,
                range_len
            );
        }

        if err_bounds == 0 && !bounds_value.is_null() {
            let mut rect = CGRect::default();
            if AXValueGetValue(
                bounds_value,
                K_AXVALUE_CGRECT_TYPE,
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
        let mut focused_element: Id = NIL;

        let err = AXUIElementCopyAttributeValue(
            system_element,
            to_ax_string("AXFocusedUIElement"),
            &mut focused_element,
        );

        if err != 0 || focused_element.is_null() {
            return Err(AccessibilityError::NoFocusedElement);
        }

        // Get PID and Bundle ID safely
        let mut pid: i32 = 0;
        AXUIElementGetPid(focused_element, &mut pid);

        let mut bundle_id = String::new();
        if pid > 0 {
            let app_class =
                AnyClass::get("NSRunningApplication").expect("NSRunningApplication not found");
            let app: Id = msg_send![app_class, runningApplicationWithProcessIdentifier: pid];
            if !app.is_null() {
                let ns_bundle_id: Id = msg_send![app, bundleIdentifier];
                if !ns_bundle_id.is_null() {
                    bundle_id = from_ax_string(ns_bundle_id);
                }
            }
        }

        let mut role_value: Id = NIL;
        AXUIElementCopyAttributeValue(focused_element, to_ax_string("AXRole"), &mut role_value);
        let role = if !role_value.is_null() {
            from_ax_string(role_value)
        } else {
            String::new()
        };

        let mut editable_value: Id = NIL;
        AXUIElementCopyAttributeValue(
            focused_element,
            to_ax_string("AXEditable"),
            &mut editable_value,
        );
        let editable = from_ax_bool(editable_value);

        // 优先全文 (AXValue)
        let mut text_value: Id = NIL;
        AXUIElementCopyAttributeValue(focused_element, to_ax_string("AXValue"), &mut text_value);

        // Slack/Electron fallback (AXSelectedText)
        if text_value.is_null() {
            AXUIElementCopyAttributeValue(
                focused_element,
                to_ax_string("AXSelectedText"),
                &mut text_value,
            );
        }

        if !text_value.is_null() {
            // Verify if the value is actually an NSString
            let ns_string_class = AnyClass::get("NSString").expect("NSString not found");
            let is_string: bool = msg_send![text_value, isKindOfClass: ns_string_class];

            if is_string {
                let full_text = from_ax_string(text_value);
                if !full_text.is_empty() {
                    let mut selected_range_value: Id = NIL;
                    AXUIElementCopyAttributeValue(
                        focused_element,
                        to_ax_string("AXSelectedTextRange"),
                        &mut selected_range_value,
                    );

                    let mut selected_range = CFRange {
                        location: 0,
                        length: 0,
                    };
                    if !selected_range_value.is_null() {
                        let _ = AXValueGetValue(
                            selected_range_value,
                            K_AXVALUE_CFRANGE_TYPE,
                            &mut selected_range as *mut _ as *mut std::ffi::c_void,
                        );
                    }

                    let total_u16 = full_text.encode_utf16().count();
                    if total_u16 > 20000 {
                        let caret_u16 = if selected_range.location >= 0 {
                            selected_range.location as usize
                        } else {
                            total_u16
                        };
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
        let mut focused_element: Id = NIL;
        let err = AXUIElementCopyAttributeValue(
            system_element,
            to_ax_string("AXFocusedUIElement"),
            &mut focused_element,
        );

        if err != 0 || focused_element.is_null() {
            return Err(AccessibilityError::NoFocusedElement);
        }

        let mut bounds_value: Id = NIL;
        let range = CFRange {
            location: range_start as i64,
            length: range_len as i64,
        };
        let ax_range = AXValueCreate(
            K_AXVALUE_CFRANGE_TYPE,
            &range as *const _ as *const std::ffi::c_void,
        );

        let err_bounds = AXUIElementCopyParameterizedAttributeValue(
            focused_element,
            to_ax_string("AXBoundsForRange"),
            ax_range,
            &mut bounds_value,
        );

        if err_bounds != 0 {
            log::info!(
                "[DIAG] AXBoundsForRange error: {} for range {}-{}",
                err_bounds,
                range_start,
                range_len
            );
        }

        if err_bounds == 0 && !bounds_value.is_null() {
            let mut rect = CGRect::default();
            if AXValueGetValue(
                bounds_value,
                K_AXVALUE_CGRECT_TYPE,
                &mut rect as *mut _ as *mut std::ffi::c_void,
            ) {
                log::info!("[DIAG] AXBoundsForRange raw: {:?}", rect);

                // 检测并修正 Electron 应用的窗口相对坐标
                let mut final_rect = rect;
                if let Ok((win_x, win_y)) = get_focused_window_position() {
                    if is_window_relative_coords(rect, (win_x, win_y)) {
                        log::info!(
                            "[DIAG] Detected window-relative coords in get_focused_range_bounds, adding window offset ({}, {})",
                            win_x,
                            win_y
                        );
                        final_rect.origin.x += win_x;
                        final_rect.origin.y += win_y;
                    }
                }

                log::info!("[DIAG] AXBoundsForRange returning: {:?}", final_rect);
                return Ok(final_rect);
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
        let mut focused_element: Id = NIL;
        let err = AXUIElementCopyAttributeValue(
            system_element,
            to_ax_string("AXFocusedUIElement"),
            &mut focused_element,
        );

        if err != 0 || focused_element.is_null() {
            log::warn!("[DIAG] get_focused_caret_bounds: no focused element");
            return Err(AccessibilityError::NoFocusedElement);
        }

        let mut selected_range_value: Id = NIL;
        AXUIElementCopyAttributeValue(
            focused_element,
            to_ax_string("AXSelectedTextRange"),
            &mut selected_range_value,
        );

        let mut selected_range = CFRange {
            location: 0,
            length: 0,
        };
        if !selected_range_value.is_null() {
            let _ = AXValueGetValue(
                selected_range_value,
                K_AXVALUE_CFRANGE_TYPE,
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
        let mut focused_element: Id = NIL;
        let err = AXUIElementCopyAttributeValue(
            system_element,
            to_ax_string("AXFocusedUIElement"),
            &mut focused_element,
        );

        if err != 0 || focused_element.is_null() {
            log::warn!("[DIAG] get_focused_element_bounds: no focused element");
            return Err(AccessibilityError::NoFocusedElement);
        }

        let mut frame_value: Id = NIL;
        AXUIElementCopyAttributeValue(focused_element, to_ax_string("AXFrame"), &mut frame_value);
        log::info!(
            "[DIAG] get_focused_element_bounds: AXFrame available={}",
            !frame_value.is_null()
        );
        if !frame_value.is_null() {
            let mut rect = CGRect::default();
            if AXValueGetValue(
                frame_value,
                K_AXVALUE_CGRECT_TYPE,
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
        let mut focused_element: Id = NIL;

        let err = AXUIElementCopyAttributeValue(
            system_element,
            to_ax_string("AXFocusedUIElement"),
            &mut focused_element,
        );

        if err != 0 || focused_element.is_null() {
            return Err(AccessibilityError::NoFocusedElement);
        }

        let mut selected_text: Id = NIL;
        let err_selected = AXUIElementCopyAttributeValue(
            focused_element,
            to_ax_string("AXSelectedText"),
            &mut selected_text,
        );

        if err_selected != 0 || selected_text.is_null() {
            return Err(AccessibilityError::NoTextSelected);
        }

        let text = from_ax_string(selected_text);
        if text.trim().is_empty() {
            return Err(AccessibilityError::NoTextSelected);
        }

        Ok(text)
    }
}

/// 获取选中文字的屏幕区域坐标（用于显示 AI 图标在选中文本右上角）
#[cfg(target_os = "macos")]
pub fn get_selected_text_bounds() -> Result<(i32, i32, i32, i32)> {
    // 返回 (x, y, width, height) - x,y 为左上角坐标
    if !unsafe { AXIsProcessTrusted() } {
        return Err(AccessibilityError::PermissionDenied);
    }

    unsafe {
        let system_element = AXUIElementCreateSystemWide();
        let mut focused_element: Id = NIL;

        let err = AXUIElementCopyAttributeValue(
            system_element,
            to_ax_string("AXFocusedUIElement"),
            &mut focused_element,
        );

        if err != 0 || focused_element.is_null() {
            return Err(AccessibilityError::NoFocusedElement);
        }

        // 获取选区范围
        let mut selected_range_value: Id = NIL;
        let err_range = AXUIElementCopyAttributeValue(
            focused_element,
            to_ax_string("AXSelectedTextRange"),
            &mut selected_range_value,
        );

        if err_range != 0 || selected_range_value.is_null() {
            return Err(AccessibilityError::NoTextSelected);
        }

        let mut selected_range = CFRange {
            location: 0,
            length: 0,
        };
        let range_size = std::mem::size_of::<CFRange>();
        if AXValueGetValue(
            selected_range_value,
            K_AXVALUE_CFRANGE_TYPE,
            &mut selected_range as *mut _ as *mut std::ffi::c_void,
        ) {
            let range_start = selected_range.location.max(0) as usize;
            let range_len = selected_range.length.max(0) as usize;

            // 获取 AXBoundsForRange
            let ax_range = AXValueCreate(
                K_AXVALUE_CFRANGE_TYPE,
                &selected_range as *const _ as *const std::ffi::c_void,
            );

            let mut bounds_value: Id = NIL;
            let err_bounds = AXUIElementCopyParameterizedAttributeValue(
                focused_element,
                to_ax_string("AXBoundsForRange"),
                ax_range,
                &mut bounds_value,
            );

            if err_bounds == 0 && !bounds_value.is_null() {
                let mut rect = CGRect::default();
                if AXValueGetValue(
                    bounds_value,
                    K_AXVALUE_CGRECT_TYPE,
                    &mut rect as *mut _ as *mut std::ffi::c_void,
                ) {
                    // 添加调试日志查看原始值
                    log::info!(
                        "[DIAG] get_selected_text_bounds: raw rect origin=({},{}) size=({},{})",
                        rect.origin.x,
                        rect.origin.y,
                        rect.size.width,
                        rect.size.height
                    );

                    // 检测 bounds 是否有效 (某些 Electron 应用返回 0,0 大小)
                    let is_valid = rect.size.width > 0.0 && rect.size.height > 0.0;

                    if is_valid {
                        // 检测并修正 Electron 应用的窗口相对坐标
                        let mut final_rect = rect;
                        if let Ok((win_x, win_y)) = get_focused_window_position() {
                            if is_window_relative_coords(rect, (win_x, win_y)) {
                                log::info!(
                                    "[DIAG] Detected window-relative coords, adding window offset ({}, {})",
                                    win_x,
                                    win_y
                                );
                                final_rect.origin.x += win_x;
                                final_rect.origin.y += win_y;
                            }
                        }

                        // 返回修正后的值
                        let sx = final_rect.origin.x as i32;
                        let sy = final_rect.origin.y as i32;
                        let sw = final_rect.size.width as i32;
                        let sh = final_rect.size.height as i32;

                        log::info!(
                            "[DIAG] get_selected_text_bounds returning: ({},{},{},{})",
                            sx,
                            sy,
                            sw,
                            sh
                        );
                        return Ok((sx, sy, sw, sh));
                    } else {
                        // Bounds 无效，回退到使用鼠标位置
                        log::info!("[DIAG] AXBoundsForRange returned invalid bounds, using mouse position fallback");
                        let (mouse_x, mouse_y) = get_cursor_position_nsevent();
                        // 返回鼠标位置作为近似选区位置，使用固定的小宽度
                        return Ok((mouse_x as i32, mouse_y as i32, 100, 20));
                    }
                }
            }
        }

        Err(AccessibilityError::NoTextSelected)
    }
}

/// Get the window's screen position for the currently focused element.
/// This is used to detect if AXBoundsForRange returns window-relative coordinates.
#[cfg(target_os = "macos")]
pub fn get_focused_window_position() -> Result<(f64, f64)> {
    if !unsafe { AXIsProcessTrusted() } {
        return Err(AccessibilityError::PermissionDenied);
    }

    unsafe {
        let system_element = AXUIElementCreateSystemWide();
        let mut focused_element: Id = NIL;

        let err = AXUIElementCopyAttributeValue(
            system_element,
            to_ax_string("AXFocusedUIElement"),
            &mut focused_element,
        );

        if err != 0 || focused_element.is_null() {
            return Err(AccessibilityError::NoFocusedElement);
        }

        // Walk up the hierarchy to find the window
        let mut current: Id = focused_element;
        let mut window_position: Option<(f64, f64)> = None;

        for _ in 0..10 {
            // Check if this element is a window
            let mut role_value: Id = NIL;
            AXUIElementCopyAttributeValue(current, to_ax_string("AXRole"), &mut role_value);
            let role = if !role_value.is_null() {
                from_ax_string(role_value)
            } else {
                String::new()
            };

            if role == "AXWindow" {
                // Get window position
                let mut position_value: Id = NIL;
                let err_pos = AXUIElementCopyAttributeValue(
                    current,
                    to_ax_string("AXPosition"),
                    &mut position_value,
                );

                if err_pos == 0 && !position_value.is_null() {
                    let mut point = core_graphics::geometry::CGPoint::new(0.0, 0.0);
                    if AXValueGetValue(
                        position_value,
                        2, // kAXValueTypeCGPoint
                        &mut point as *mut _ as *mut std::ffi::c_void,
                    ) {
                        window_position = Some((point.x, point.y));
                        log::info!(
                            "[DIAG] get_focused_window_position: found window at ({}, {})",
                            point.x,
                            point.y
                        );
                        break;
                    }
                }
            }

            // Move to parent
            let mut parent: Id = NIL;
            let err_parent =
                AXUIElementCopyAttributeValue(current, to_ax_string("AXParent"), &mut parent);

            if err_parent != 0 || parent.is_null() {
                break;
            }
            current = parent;
        }

        window_position.ok_or(AccessibilityError::NoFocusedElement)
    }
}

/// Check if the given bounds appear to be window-relative (inside an Electron app)
/// rather than screen coordinates. Electron apps often return coordinates relative
/// to the window origin instead of screen coordinates.
fn is_window_relative_coords(bounds: CGRect, window_pos: (f64, f64)) -> bool {
    // If bounds origin is very close to window position, it's likely window-relative
    let dx = (bounds.origin.x - window_pos.0).abs();
    let dy = (bounds.origin.y - window_pos.1).abs();

    // Window-relative coords typically have small delta (< 50 pixels)
    // Screen coords have larger delta (window position + text position)
    let threshold = 50.0;

    // Also check if bounds are at a typical "inside window" position
    // (small positive coordinates, not screen-size coordinates)
    let looks_like_window_coords = bounds.origin.x < 2000.0
        && bounds.origin.y < 2000.0
        && window_pos.0 > 0.0
        && window_pos.1 > 0.0;

    (dx < threshold && dy < threshold) || looks_like_window_coords
}

// --- macOS 底层辅助函数 ---

#[repr(C)]
struct CFRange {
    location: i64,
    length: i64,
}

const K_AXVALUE_CFRANGE_TYPE: i32 = 4;
const K_AXVALUE_CGRECT_TYPE: i32 = 3;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXUIElementCopyAttributeValue(element: Id, attribute: Id, value: *mut Id) -> i32;
    fn AXUIElementSetAttributeValue(element: Id, attribute: Id, value: Id) -> i32;
    fn AXUIElementGetPid(element: Id, pid: *mut i32) -> i32;
    fn AXUIElementCopyParameterizedAttributeValue(
        element: Id,
        attribute: Id,
        parameter: Id,
        value: *mut Id,
    ) -> i32;
    fn AXValueCreate(the_type: i32, value_ptr: *const std::ffi::c_void) -> Id;
    fn AXValueGetValue(value: Id, the_type: i32, value_ptr: *mut std::ffi::c_void) -> bool;
}

fn to_ax_string(s: &str) -> Id {
    unsafe {
        let c_string = match std::ffi::CString::new(s) {
            Ok(v) => v,
            Err(_) => return NIL,
        };
        let ns_string_class = AnyClass::get("NSString").expect("NSString not found");
        let ns_string: Id = msg_send![ns_string_class, stringWithUTF8String: c_string.as_ptr()];
        ns_string
    }
}

fn from_ax_string(ns_string: Id) -> String {
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

fn from_ax_bool(ns_value: Id) -> bool {
    unsafe {
        if ns_value.is_null() {
            return false;
        }
        // Use isKindOfClass: to verify it's a number/boolean before calling charValue
        let ns_number_class = AnyClass::get("NSNumber").expect("NSNumber not found");
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
        let mut focused_element: Id = NIL;
        let err = AXUIElementCopyAttributeValue(
            system_element,
            to_ax_string("AXFocusedUIElement"),
            &mut focused_element,
        );

        if err != 0 || focused_element.is_null() {
            return Err(AccessibilityError::NoFocusedElement);
        }

        let range = CFRange {
            location: start as i64,
            length: length as i64,
        };
        let ax_range = AXValueCreate(
            K_AXVALUE_CFRANGE_TYPE,
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
