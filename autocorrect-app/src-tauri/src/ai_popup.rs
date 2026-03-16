#![allow(unexpected_cfgs)]

use crate::commands::errors::Error;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};

#[cfg(target_os = "macos")]
mod geom {
    use objc2::Encode;

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct CGPoint {
        pub x: f64,
        pub y: f64,
    }

    impl CGPoint {
        pub fn new(x: f64, y: f64) -> Self {
            Self { x, y }
        }
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct CGSize {
        pub width: f64,
        pub height: f64,
    }

    impl CGSize {
        pub fn new(width: f64, height: f64) -> Self {
            Self { width, height }
        }
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct CGRect {
        pub origin: CGPoint,
        pub size: CGSize,
    }

    impl CGRect {
        pub fn new(origin: &CGPoint, size: &CGSize) -> Self {
            Self {
                origin: *origin,
                size: *size,
            }
        }
    }

    unsafe impl Encode for CGPoint {
        const ENCODING: objc2::Encoding = objc2::Encoding::Struct("CGPoint", &[
            objc2::Encoding::Double,
            objc2::Encoding::Double,
        ]);
    }

    unsafe impl Encode for CGSize {
        const ENCODING: objc2::Encoding = objc2::Encoding::Struct("CGSize", &[
            objc2::Encoding::Double,
            objc2::Encoding::Double,
        ]);
    }

    unsafe impl Encode for CGRect {
        const ENCODING: objc2::Encoding = objc2::Encoding::Struct("CGRect", &[
            <CGPoint>::ENCODING,
            <CGSize>::ENCODING,
        ]);
    }
}

// ── Shared state ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct AiPopupState {
    pub icon_visible: bool,
    pub popup_visible: bool,
    /// Screen position (top-left coords) of the native icon panel.
    pub icon_position: (i32, i32),
    /// Size of the native icon in logical pixels (used for hit-testing).
    pub icon_size: (i32, i32),
    pub selected_text: String,
    pub source_app_name: Option<String>,
}

impl AiPopupState {
    pub fn new() -> Self {
        Self {
            icon_visible: false,
            popup_visible: false,
            icon_position: (0, 0),
            icon_size: (36, 36),
            selected_text: String::new(),
            source_app_name: None,
        }
    }
}

pub struct SharedAiPopupState(pub Arc<Mutex<AiPopupState>>);

impl SharedAiPopupState {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(AiPopupState::new())))
    }
}

/// Native icon window handle — kept alive for the lifetime of the app.
#[cfg(target_os = "macos")]
#[derive(Default)]
struct NativeIconWindow {
    window: usize, // NSPanel*
}

#[cfg(target_os = "macos")]
pub struct SharedNativeIconWindow(pub Arc<Mutex<NativeIconWindow>>);

#[cfg(target_os = "macos")]
impl SharedNativeIconWindow {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(NativeIconWindow::default())))
    }
}

// ── Native icon rendering ─────────────────────────────────────────────────────

/// Show the native 💡 icon at the given screen position (top-left coords).
/// Safe to call from any thread; dispatches to main thread internally.
pub fn show_ai_icon(app: &AppHandle, x: i32, y: i32, selected_text: String) {
    // Suppress redundant updates. Once the icon is visible for a given selection,
    // do NOT reposition it — the cursor will have moved to hover over the icon,
    // which would otherwise cause it to chase the mouse cursor in a loop.
    if let Some(state) = app.try_state::<SharedAiPopupState>() {
        if let Ok(mut s) = state.0.lock() {
            if s.icon_visible && s.selected_text == selected_text {
                return; // same selection, keep existing position
            }
            s.icon_visible = true;
            s.icon_position = (x, y);
            s.selected_text = selected_text.clone();
            #[cfg(target_os = "macos")]
            {
                s.source_app_name = crate::popup::get_frontmost_app_name_macos();
            }
        }
    }

    #[cfg(target_os = "macos")]
    if let Some(native) = app.try_state::<SharedNativeIconWindow>() {
        let native = Arc::clone(&native.0);
        let _ = app.run_on_main_thread(move || unsafe {
            if let Ok(mut guard) = native.lock() {
                render_native_icon(&mut guard, x, y);
            }
        });
    }
}

/// Hide the native icon (and the popup if visible).
pub fn hide_ai_icon(app: &AppHandle) {
    let was_visible = app
        .try_state::<SharedAiPopupState>()
        .and_then(|s| s.0.lock().ok().map(|g| g.icon_visible))
        .unwrap_or(false);

    if !was_visible {
        return;
    }

    if let Some(state) = app.try_state::<SharedAiPopupState>() {
        if let Ok(mut s) = state.0.lock() {
            s.icon_visible = false;
        }
    }

    #[cfg(target_os = "macos")]
    if let Some(native) = app.try_state::<SharedNativeIconWindow>() {
        let native = Arc::clone(&native.0);
        let _ = app.run_on_main_thread(move || unsafe {
            if let Ok(guard) = native.lock() {
                hide_native_icon(&guard);
            }
        });
    }

    hide_ai_popup_internal(app);
}

#[cfg(target_os = "macos")]
unsafe fn render_native_icon(state: &mut NativeIconWindow, x: i32, y: i32) {
    use objc2::msg_send;
    use objc2::runtime::AnyClass;
    use geom::{CGRect, CGPoint, CGSize};

    type Id = *mut objc2::runtime::AnyObject;
    const NIL: Id = std::ptr::null_mut();

    const ICON_SIZE: f64 = 36.0;

    // y is in Quartz top-left coords (from CGEvent); NSWindow uses bottom-left.
    let screen_height: f64 = {
        let screen: Id = msg_send![AnyClass::get("NSScreen").expect("NSScreen not found"), mainScreen];
        if screen.is_null() {
            return;
        }
        let frame: CGRect = msg_send![screen, frame];
        frame.size.height
    };
    let mac_y = screen_height - y as f64 - ICON_SIZE;

    // Create the panel once; reuse on subsequent calls.
    if state.window == 0 {
        let style_mask = 128_u64; // NSBorderlessWindowMask | NSNonactivatingPanelMask
        let frame = CGRect::new(
            &CGPoint::new(x as f64, mac_y),
            &CGSize::new(ICON_SIZE, ICON_SIZE),
        );
        let panel_class = AnyClass::get("NSPanel").expect("NSPanel not found");
        let window: Id = msg_send![panel_class, alloc];
        let window: Id = msg_send![
            window,
            initWithContentRect: frame
            styleMask: style_mask
            backing: 2_u64 // NSBackingStoreBuffered
            defer: false
        ];
        if window.is_null() {
            return;
        }

        let _: () = msg_send![window, setOpaque: false];
        let clear: Id = msg_send![AnyClass::get("NSColor").expect("NSColor not found"), clearColor];
        let _: () = msg_send![window, setBackgroundColor: clear];
        let _: () = msg_send![window, setIgnoresMouseEvents: false];
        let _: () = msg_send![window, setReleasedWhenClosed: false];
        let _: () = msg_send![window, setHasShadow: false];
        let _: () = msg_send![window, setHidesOnDeactivate: false];
        let _: () = msg_send![window, setCollectionBehavior: (1_u64 << 0) | (1_u64 << 7)];
        let _: () = msg_send![window, setLevel: 2002_i64];
        let _: () = msg_send![window, setAcceptsMouseMovedEvents: true];

        // Clear background view - no background, just emoji
        let content_frame = CGRect::new(
            &CGPoint::new(0.0, 0.0),
            &CGSize::new(ICON_SIZE, ICON_SIZE),
        );
        let view_class = AnyClass::get("NSView").expect("NSView not found");
        let bg_view: Id = msg_send![view_class, alloc];
        let bg_view: Id = msg_send![bg_view, initWithFrame: content_frame];
        let _: () = msg_send![bg_view, setWantsLayer: true];
        let bg_layer: Id = msg_send![bg_view, layer];
        // Transparent background
        let clear_color: Id = msg_send![AnyClass::get("NSColor").expect("NSColor not found"), clearColor];
        // CGColor returns a Core Foundation pointer, not an Objective-C object
        let cg_clear: *mut std::ffi::c_void = msg_send![clear_color, CGColor];
        let _: () = msg_send![bg_layer, setBackgroundColor: cg_clear];

        // 💡 label - centered vertically
        let text_field_class = AnyClass::get("NSTextField").expect("NSTextField not found");
        let label: Id = msg_send![text_field_class, alloc];
        let label_frame = CGRect::new(
            &CGPoint::new(0.0, (ICON_SIZE - 20.0) / 2.0),
            &CGSize::new(ICON_SIZE, 20.0),
        );
        let label: Id = msg_send![label, initWithFrame: label_frame];
        let emoji = crate::objc2_compat::ns_string("💡");
        let _: () = msg_send![label, setStringValue: emoji];
        let _: () = msg_send![label, setBezeled: false];
        let _: () = msg_send![label, setDrawsBackground: false];
        let _: () = msg_send![label, setEditable: false];
        let _: () = msg_send![label, setSelectable: false];
        // Center the emoji horizontally and vertically
        let _: () = msg_send![label, setAlignment: 1_i64]; // NSTextAlignmentCenter
                                                           // Font size
        let font_class = AnyClass::get("NSFont").expect("NSFont not found");
        let font: Id = msg_send![font_class, systemFontOfSize: 20.0_f64];
        let _: () = msg_send![label, setFont: font];

        let _: () = msg_send![bg_view, addSubview: label];
        let _: () = msg_send![window, setContentView: bg_view];

        state.window = window as usize;
    }

    // Reposition.
    let window = state.window as Id;
    let new_frame = CGRect::new(
        &CGPoint::new(x as f64, mac_y),
        &CGSize::new(ICON_SIZE, ICON_SIZE),
    );
    let _: () = msg_send![window, setFrame: new_frame display: true];
    let _: () = msg_send![window, orderFrontRegardless];
}

#[cfg(target_os = "macos")]
unsafe fn hide_native_icon(state: &NativeIconWindow) {
    use objc2::msg_send;
    type Id = *mut objc2::runtime::AnyObject;
    const NIL: Id = std::ptr::null_mut();
    if state.window != 0 {
        let window = state.window as Id;
        let _: () = msg_send![window, orderOut: NIL];
    }
}

// ── Popup show / hide (WebView window) ───────────────────────────────────────

fn hide_ai_popup_internal(app: &AppHandle) {
    if let Some(state) = app.try_state::<SharedAiPopupState>() {
        if let Ok(mut s) = state.0.lock() {
            s.popup_visible = false;
        }
    }
    if let Some(w) = app.get_webview_window("ai-popup") {
        let _ = w.hide();
    }
}

/// Called from the hover thread when the cursor enters the icon area.
pub fn show_ai_popup_from_hover(app: &AppHandle) {
    let (selected_text, icon_pos) = {
        let state = match app.try_state::<SharedAiPopupState>() {
            Some(s) => s,
            None => return,
        };
        let s = match state.0.lock() {
            Ok(g) => g,
            Err(_) => return,
        };
        if s.popup_visible {
            return; // already showing
        }
        (s.selected_text.clone(), s.icon_position)
    };

    // Place popup to the right of the icon, similar to popup positioning
    let popup_x = icon_pos.0 + 46; // icon size 36 + 10px gap
    let popup_y = icon_pos.1 - 100; // vertically centered on icon
    let _ = show_ai_popup_at(app, popup_x, popup_y, selected_text);
}

fn show_ai_popup_at(app: &AppHandle, x: i32, y: i32, selected_text: String) -> Result<(), Error> {
    let popup_window = match app.get_webview_window("ai-popup") {
        Some(w) => w,
        None => return Ok(()),
    };

    // Hide the floating icon when popup shows
    hide_ai_icon(app);

    if let Some(state) = app.try_state::<SharedAiPopupState>() {
        if let Ok(mut s) = state.0.lock() {
            s.popup_visible = true;
        }
    }

    // All UI operations (set_position, show, NSWindow tweaks) MUST run on the
    // main thread. Calling them from a background thread silently fails on macOS.
    let win = popup_window.clone();
    let _ = popup_window.run_on_main_thread(move || {
        let _ = win.set_position(tauri::Position::Logical(tauri::LogicalPosition {
            x: x as f64,
            y: y as f64,
        }));
        let _ = win.show();

        #[cfg(target_os = "macos")]
        if let Ok(ptr) = win.ns_window() {
            use objc2::msg_send;
            use objc2::runtime::AnyClass;

            type Id = *mut objc2::runtime::AnyObject;
            const NIL: Id = std::ptr::null_mut();

            unsafe {
                let ns = ptr as Id;
                // Convert to NSNonactivatingPanel — floats above source app
                // without stealing focus or clearing the text selection.
                extern "C" {
                    fn object_setClass(obj: Id, cls: Id) -> Id;
                }
                let panel_class: Id = AnyClass::get("NSPanel").expect("NSPanel not found") as *const _ as Id;
                object_setClass(ns, panel_class);
                let cur_mask: usize = msg_send![ns, styleMask];
                let _: () = msg_send![ns, setStyleMask: cur_mask | 128_usize];
                let _: () = msg_send![ns, setFloatingPanel: true];
                // 移除 setBecomesKeyOnlyIfNeeded，确保窗口能正确获得焦点
                // let _: () = msg_send![ns, setBecomesKeyOnlyIfNeeded: true];
                let _: () = msg_send![ns, setLevel: 2001_i64]; // 改为 2001
                let _: () = msg_send![ns, setHidesOnDeactivate: false];
                let _: () = msg_send![ns, setAcceptsMouseMovedEvents: true];

                // Activate app so the popup receives mouse events
                let app_class = AnyClass::get("NSApplication").expect("NSApplication not found");
                let app_ns: Id = msg_send![app_class, sharedApplication];
                let _: () = msg_send![app_ns, activateIgnoringOtherApps: true];
                let _: () = msg_send![ns, makeKeyAndOrderFront: NIL];

                // Set first responder to enable hover/click events
                let content_view: Id = msg_send![ns, contentView];
                let _: bool = msg_send![ns, makeFirstResponder: content_view];
            }
        }
    });

    let _ = app.emit(
        "ai-popup-show",
        serde_json::json!({ "selectedText": selected_text }),
    );
    Ok(())
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[tauri::command]
pub fn show_ai_popup(app: AppHandle) -> Result<(), Error> {
    let (text, pos) = {
        let state = app
            .try_state::<SharedAiPopupState>()
            .ok_or_else(|| io_err("no state"))?;
        let s = state.0.lock().map_err(|_| io_err("lock failed"))?;
        (s.selected_text.clone(), s.icon_position)
    };
    show_ai_popup_at(&app, pos.0, pos.1 + 40, text)
}

#[tauri::command]
pub fn hide_ai_popup(app: AppHandle) -> Result<(), Error> {
    hide_ai_popup_internal(&app);
    let _ = app.emit("ai-popup-hide", ());
    Ok(())
}

#[tauri::command]
pub fn get_ai_popup_state(state: State<SharedAiPopupState>) -> Result<serde_json::Value, Error> {
    let s = state.0.lock().map_err(|_| io_err("lock failed"))?;
    Ok(serde_json::json!({
        "iconVisible": s.icon_visible,
        "popupVisible": s.popup_visible,
        "selectedText": s.selected_text,
        "sourceAppName": s.source_app_name,
    }))
}

#[tauri::command]
pub fn accept_ai_result(app: AppHandle, text: String) -> Result<(), Error> {
    #[cfg(target_os = "macos")]
    {
        let source_app = app
            .try_state::<SharedAiPopupState>()
            .and_then(|s| s.0.lock().ok().and_then(|g| g.source_app_name.clone()));

        let mut clipboard =
            arboard::Clipboard::new().map_err(|e| Error::Clipboard(format!("{e}")))?;
        let prev = clipboard.get_text().ok();
        clipboard
            .set_text(text.clone())
            .map_err(|e| Error::Clipboard(format!("{e}")))?;

        // Dismiss both native icon and WebView popup.
        hide_ai_popup_internal(&app);
        if let Some(state) = app.try_state::<SharedAiPopupState>() {
            if let Ok(mut s) = state.0.lock() {
                s.icon_visible = false;
                s.popup_visible = false;
            }
        }
        #[cfg(target_os = "macos")]
        if let Some(native) = app.try_state::<SharedNativeIconWindow>() {
            let native = Arc::clone(&native.0);
            let _ = app.run_on_main_thread(move || unsafe {
                if let Ok(guard) = native.lock() {
                    hide_native_icon(&guard);
                }
            });
        }

        thread::sleep(Duration::from_millis(80));

        if let Some(ref name) = source_app {
            if name != "autocorrect-app" && name != "AutoCorrect" {
                activate_app(name)?;
                let t = std::time::Instant::now();
                loop {
                    thread::sleep(Duration::from_millis(30));
                    if crate::popup::is_app_frontmost_macos_pub(name) {
                        break;
                    }
                    if t.elapsed().as_millis() > 600 {
                        break;
                    }
                }
                thread::sleep(Duration::from_millis(80));
            }
        }

        let status = std::process::Command::new("osascript")
            .arg("-e")
            .arg("tell application \"System Events\" to keystroke \"v\" using command down")
            .status()
            .map_err(|e| Error::InputSimulation(format!("{e}")))?;

        thread::sleep(Duration::from_millis(80));
        if let Some(old) = prev {
            let _ = clipboard.set_text(old);
        }
        if !status.success() {
            return Err(Error::InputSimulation("paste failed".into()));
        }

        let _ = app.emit("ai-result-accepted", serde_json::json!({ "text": text }));
        return Ok(());
    }

    #[cfg(not(target_os = "macos"))]
    {
        use crate::commands::spellcheck::set_clipboard_text;
        set_clipboard_text(text.clone())?;
        hide_ai_popup_internal(&app);
        let _ = app.emit("ai-result-accepted", serde_json::json!({ "text": text }));
        Ok(())
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn io_err(msg: &str) -> Error {
    Error::Io(std::io::Error::new(std::io::ErrorKind::Other, msg))
}

#[cfg(target_os = "macos")]
fn activate_app(app_name: &str) -> Result<(), Error> {
    let escaped = app_name.replace('\\', "\\\\").replace('"', "\\\"");
    let status = std::process::Command::new("osascript")
        .arg("-e")
        .arg(format!("tell application \"{}\" to activate", escaped))
        .status()
        .map_err(|e| Error::InputSimulation(format!("{e}")))?;
    if status.success() {
        Ok(())
    } else {
        Err(Error::InputSimulation("activate failed".into()))
    }
}
