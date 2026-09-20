//! AXObserver event bridge (macOS).
//!
//! Watches the frontmost app's focused element for `AXValueChanged` /
//! `AXSelectedTextChanged` / focus changes and signals the typo-sync thread
//! immediately, replacing pure polling (first-keystroke latency drops from
//! "up to 800ms" to ~notification + debounce).
//!
//! Threading model: AXObserver must be created and added to a run loop on
//! the same thread that pumps that run loop. A single dedicated thread owns
//! a CFRunLoop with a repeating timer (500ms) that (a) tracks frontmost app
//! switches and (b) re-targets observers. Callbacks fire on that thread and
//! push `()` events into an mpsc channel consumed by the sync loop.
//!
//! Everything is defensive: any AX error downgrades to "no events" and the
//! poll loop keeps working as the fallback.

#![cfg(target_os = "macos")]

use std::sync::mpsc::Sender;
use std::sync::{Mutex, OnceLock};

type AXObserverRef = *mut std::ffi::c_void;
type AXUIElementRef = *mut std::ffi::c_void;
type CFRunLoopRef = *mut std::ffi::c_void;
type CFRunLoopSourceRef = *mut std::ffi::c_void;
type CFRunLoopTimerRef = *mut std::ffi::c_void;
type CFStringRef = *const std::ffi::c_void;
#[allow(non_camel_case_types)]
type pid_t = i32;

type AXObserverCallback =
    extern "C-unwind" fn(AXObserverRef, AXUIElementRef, CFStringRef, *mut std::ffi::c_void);
type CFRunLoopTimerCallback = extern "C-unwind" fn(CFRunLoopTimerRef, *mut std::ffi::c_void);

/// Raw AX handles confined to the bridge thread; Send so the state can live
/// in a static Mutex (every touch happens on the run-loop thread).
struct BridgeState {
    pid: i32,
    observer: AXObserverRef,
    observer_source: CFRunLoopSourceRef,
    app_element: AXUIElementRef,
    focused: AXUIElementRef,
}
unsafe impl Send for BridgeState {}

extern "C" {
    fn AXObserverCreate(pid: pid_t, callback: AXObserverCallback, out: *mut AXObserverRef) -> i32;
    fn AXObserverAddToRunLoop(observer: AXObserverRef, run_loop: CFRunLoopRef) -> i32;
    fn AXObserverGetRunLoopSource(observer: AXObserverRef) -> CFRunLoopSourceRef;
    fn AXObserverAddNotification(
        observer: AXObserverRef,
        element: AXUIElementRef,
        notification: CFStringRef,
        refcon: *mut std::ffi::c_void,
    ) -> i32;
    fn AXUIElementCreateApplication(pid: pid_t) -> AXUIElementRef;
    fn AXUIElementSetMessagingTimeout(element: AXUIElementRef, timeout: f64) -> i32;
    fn CFRelease(cf: *mut std::ffi::c_void);
    fn CFRunLoopGetCurrent() -> CFRunLoopRef;
    fn CFRunLoopRemoveSource(run_loop: CFRunLoopRef, source: CFRunLoopSourceRef, mode: CFStringRef);
    fn CFRunLoopSourceInvalidate(source: CFRunLoopSourceRef);
    fn CFRunLoopTimerCreate(
        allocator: *mut std::ffi::c_void,
        fire_date: f64,
        interval: f64,
        flags: u32,
        order: i64,
        callback: CFRunLoopTimerCallback,
        context: *mut std::ffi::c_void,
    ) -> CFRunLoopTimerRef;
    fn CFRunLoopAddTimer(run_loop: CFRunLoopRef, timer: CFRunLoopTimerRef, mode: CFStringRef);
}

/// CFRunLoop timer callback signature context (unused refcon).
static EVENT_SENDER: OnceLock<Sender<()>> = OnceLock::new();
static STATE: Mutex<Option<BridgeState>> = Mutex::new(None);
const NOTIF_FOCUS_CHANGED: &str = "AXFocusedUIElementChanged";
const NOTIF_VALUE_CHANGED: &str = "AXValueChanged";
const NOTIF_SELECTION_CHANGED: &str = "AXSelectedTextChanged";

/// Create an NSString for an AX attribute/notification name and view it as
/// CFString. NSString toll-free bridges to CFString — the same pattern used
/// for every AX attribute name in macos_text.rs.
fn to_cf_string(s: &str) -> CFStringRef {
    unsafe {
        use objc2::msg_send;
        use objc2::runtime::AnyClass;
        type Id = *mut objc2::runtime::AnyObject;
        let ns_string_class = AnyClass::get("NSString").expect("NSString not found");
        match std::ffi::CString::new(s) {
            Ok(c) => {
                let ns: Id = msg_send![ns_string_class, stringWithUTF8String: c.as_ptr()];
                std::mem::transmute::<Id, CFStringRef>(ns)
            }
            Err(_) => std::ptr::null(),
        }
    }
}

fn ax_err_log(context: &str, err: i32) {
    if err != 0 {
        log::debug!("[AXOBS] {context} failed with AXError={err}");
    }
}

extern "C-unwind" fn bridge_timer_callback(
    _timer: CFRunLoopTimerRef,
    _info: *mut std::ffi::c_void,
) {
    let pid = match frontmost_pid() {
        Some(pid) => pid,
        None => return,
    };

    let mut state = match STATE.lock() {
        Ok(g) => g,
        Err(_) => return,
    };

    let same_app = state.as_ref().is_some_and(|s| s.pid == pid);
    if same_app {
        // Same app: just make sure we observe its focused element.
        if let Some(existing) = state.as_mut() {
            if existing.focused.is_null() {
                existing.focused = copy_focused_element(existing.app_element);
                observe_text_notifications(existing);
            }
        }
        return;
    }

    // App switch: rebuild the observer for the new app.
    if let Some(old) = state.take() {
        teardown(old);
    }
    match BridgeState::create(pid) {
        Ok(mut new_state) => {
            new_state.focused = copy_focused_element(new_state.app_element);
            observe_text_notifications(&new_state);
            log::info!("[AXOBS] observing pid={pid}");
            *state = Some(new_state);
        }
        Err(e) => {
            log::debug!("[AXOBS] bridge create failed for pid={pid}: {e}");
        }
    }
}

impl BridgeState {
    fn create(pid: pid_t) -> Result<Self, String> {
        unsafe {
            let mut observer: AXObserverRef = std::ptr::null_mut();
            let err = AXObserverCreate(pid, text_or_focus_callback, &mut observer);
            if err != 0 {
                return Err(format!("AXObserverCreate err={err}"));
            }
            let run_loop = CFRunLoopGetCurrent();
            let err = AXObserverAddToRunLoop(observer, run_loop);
            if err != 0 {
                CFRelease(observer);
                return Err(format!("AXObserverAddToRunLoop err={err}"));
            }
            let source = AXObserverGetRunLoopSource(observer);
            let app_element = AXUIElementCreateApplication(pid);
            AXUIElementSetMessagingTimeout(app_element, 1.5);
            Ok(Self {
                pid,
                observer,
                observer_source: source,
                app_element,
                focused: std::ptr::null_mut(),
            })
        }
    }
}

fn teardown(state: BridgeState) {
    unsafe {
        CFRunLoopSourceInvalidate(state.observer_source);
        CFRunLoopRemoveSource(
            CFRunLoopGetCurrent(),
            state.observer_source,
            core_foundation::runloop::kCFRunLoopDefaultMode as CFStringRef,
        );
        if !state.focused.is_null() {
            CFRelease(state.focused);
        }
        CFRelease(state.app_element);
        CFRelease(state.observer);
    }
}

/// AXObserver callback: fires for focus changes AND text changes of the
/// observed element(s). Focus changes re-target text notifications; text
/// changes emit an event.
extern "C-unwind" fn text_or_focus_callback(
    _observer: AXObserverRef,
    _element: AXUIElementRef,
    notification: CFStringRef,
    _refcon: *mut std::ffi::c_void,
) {
    let name = unsafe {
        use core_foundation::base::TCFType;
        let cf = notification as core_foundation::string::CFStringRef;
        core_foundation::string::CFString::wrap_under_get_rule(cf).to_string()
    };

    if name == NOTIF_FOCUS_CHANGED {
        let mut state = match STATE.lock() {
            Ok(g) => g,
            Err(_) => return,
        };
        if let Some(existing) = state.as_mut() {
            if !existing.focused.is_null() {
                unsafe { CFRelease(existing.focused) };
                existing.focused = std::ptr::null_mut();
            }
            existing.focused = copy_focused_element(existing.app_element);
            observe_text_notifications(existing);
        }
        return;
    }

    // Text/selection changed on the observed element.
    if let Some(sender) = EVENT_SENDER.get() {
        let _ = sender.send(());
    }
}

fn copy_focused_element(app_element: AXUIElementRef) -> AXUIElementRef {
    unsafe {
        extern "C" {
            fn AXUIElementCopyAttributeValue(
                element: AXUIElementRef,
                attr: CFStringRef,
                out: *mut AXUIElementRef,
            ) -> i32;
        }
        let attr = to_cf_string("AXFocusedUIElement");
        if attr.is_null() {
            return std::ptr::null_mut();
        }
        let mut focused: AXUIElementRef = std::ptr::null_mut();
        let err = AXUIElementCopyAttributeValue(app_element, attr, &mut focused);
        if err != 0 {
            return std::ptr::null_mut();
        }
        if !focused.is_null() {
            AXUIElementSetMessagingTimeout(focused, 1.5);
        }
        focused
    }
}

fn observe_text_notifications(state: &BridgeState) {
    if state.focused.is_null() {
        return;
    }
    unsafe {
        ax_err_log(
            "observe AXValueChanged",
            AXObserverAddNotification(
                state.observer,
                state.focused,
                to_cf_string(NOTIF_VALUE_CHANGED),
                std::ptr::null_mut(),
            ),
        );
        ax_err_log(
            "observe AXSelectedTextChanged",
            AXObserverAddNotification(
                state.observer,
                state.focused,
                to_cf_string(NOTIF_SELECTION_CHANGED),
                std::ptr::null_mut(),
            ),
        );
    }
}

/// Frontmost app pid via NSWorkspace (fast ObjC call, same pattern as
/// popup.rs).
fn frontmost_pid() -> Option<pid_t> {
    unsafe {
        use objc2::msg_send;
        use objc2::runtime::AnyClass;
        type Id = *mut objc2::runtime::AnyObject;
        let workspace_class = AnyClass::get("NSWorkspace")?;
        let workspace: Id = msg_send![workspace_class, sharedWorkspace];
        if workspace.is_null() {
            return None;
        }
        let app: Id = msg_send![workspace, frontmostApplication];
        if app.is_null() {
            return None;
        }
        let pid: pid_t = msg_send![app, processIdentifier];
        if pid <= 0 {
            None
        } else {
            Some(pid)
        }
    }
}

/// Spawn the bridge thread. `event_tx` receives `()` whenever the focused
/// text (or selection) of the frontmost app likely changed.
pub fn spawn_text_event_bridge(event_tx: Sender<()>) {
    let _ = EVENT_SENDER.set(event_tx);
    std::thread::Builder::new()
        .name("ax-observer-bridge".to_string())
        .spawn(|| unsafe {
            // SAFETY: AXObserver + CFRunLoop calls; see module docs for the
            // threading contract. All failures downgrade to no-events.
            let run_loop = CFRunLoopGetCurrent();
            let timer = CFRunLoopTimerCreate(
                std::ptr::null_mut(),
                0.0, // fire immediately
                0.5, // re-check frontmost app every 500ms
                0,
                0,
                bridge_timer_callback,
                std::ptr::null_mut(),
            );
            if !timer.is_null() {
                CFRunLoopAddTimer(
                    run_loop,
                    timer,
                    core_foundation::runloop::kCFRunLoopDefaultMode as CFStringRef,
                );
            }
            core_foundation::runloop::CFRunLoop::run_current();
        })
        .map(|_| ())
        .unwrap_or_else(|e| log::warn!("[AXOBS] failed to spawn bridge thread: {e}"));
}
