use serde::Serialize;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::AppHandle;

#[derive(Debug, Serialize, Clone)]
pub struct TypoMarker {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub text: String,
    pub suggestions: Vec<String>,
    pub offset: usize,
    pub char_length: usize,
}

pub struct OverlayManager {
    handle: AppHandle,
    #[cfg(target_os = "macos")]
    state: Arc<Mutex<NativeOverlayState>>,
    pub current_markers: Arc<Mutex<Vec<TypoMarker>>>,
}

static LAST_MARKER_COUNT: AtomicUsize = AtomicUsize::new(usize::MAX);

#[cfg(target_os = "macos")]
#[derive(Default)]
struct NativeOverlayState {
    window: usize,
    container: usize,
    marker_views: Vec<usize>,
    visible: bool,
    empty_streak: usize,
    last_non_empty_at: Option<Instant>,
    screen_height: f64,
    frame_origin_x: f64,
    frame_origin_y: f64,
}

impl OverlayManager {
    pub fn new(handle: AppHandle) -> Self {
        Self {
            handle,
            #[cfg(target_os = "macos")]
            state: Arc::new(Mutex::new(NativeOverlayState::default())),
            current_markers: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn get_or_create_overlay(&self) -> Result<(), String> {
        #[cfg(target_os = "macos")]
        {
            let state = Arc::clone(&self.state);
            self.handle
                .run_on_main_thread(move || {
                    if let Ok(mut guard) = state.lock() {
                        if let Err(err) = unsafe { ensure_native_overlay(&mut guard) } {
                            log::warn!("Failed to create native overlay: {}", err);
                        }
                    }
                })
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn update_markers(&self, markers: Vec<TypoMarker>) {
        if let Ok(mut lock) = self.current_markers.lock() {
            *lock = markers.clone();
        }

        let marker_count = markers.len();
        let previous = LAST_MARKER_COUNT.swap(marker_count, Ordering::Relaxed);
        if previous != marker_count {
            log::info!("Overlay markers updated: {}", marker_count);
        }

        // Log marker positions
        for (i, m) in markers.iter().enumerate() {
            log::debug!(
                "[DIAG] overlay marker[{}]: id={}, x={:.1}, y={:.1}, w={:.1}, h={:.1}",
                i,
                m.id,
                m.x,
                m.y,
                m.width,
                m.height
            );
        }

        #[cfg(target_os = "macos")]
        {
            let state = Arc::clone(&self.state);
            let _ = self.handle.run_on_main_thread(move || {
                if let Ok(mut guard) = state.lock() {
                    if let Err(err) = unsafe { render_native_markers(&mut guard, &markers) } {
                        log::warn!("Native overlay render error: {}", err);
                    }
                }
            });
        }
    }
}

#[cfg(target_os = "macos")]
unsafe fn ensure_native_overlay(state: &mut NativeOverlayState) -> Result<(), String> {
    use cocoa::appkit::{
        NSApp, NSApplication, NSBackingStoreType, NSColor, NSScreen, NSView, NSWindowStyleMask,
    };
    use cocoa::base::{id, nil, NO, YES};
    use cocoa::foundation::{NSPoint, NSRect, NSSize};
    use objc::{class, msg_send, sel, sel_impl};

    if state.window != 0 && state.container != 0 {
        return Ok(());
    }

    let app = NSApp();
    if app == nil {
        return Err("NSApp is nil".to_string());
    }

    let screens: id = NSScreen::screens(nil);
    if screens == nil {
        return Err("screen list not found".to_string());
    }
    let count: usize = msg_send![screens, count];
    if count == 0 {
        return Err("no screens found".to_string());
    }

    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;
    for idx in 0..count {
        let screen: id = msg_send![screens, objectAtIndex: idx];
        if screen == nil {
            continue;
        }
        let frame: NSRect = msg_send![screen, frame];
        min_x = min_x.min(frame.origin.x);
        min_y = min_y.min(frame.origin.y);
        max_x = max_x.max(frame.origin.x + frame.size.width);
        max_y = max_y.max(frame.origin.y + frame.size.height);
    }
    if !min_x.is_finite() || !min_y.is_finite() || !max_x.is_finite() || !max_y.is_finite() {
        return Err("invalid screen bounds".to_string());
    }
    let frame = NSRect::new(
        NSPoint::new(min_x, min_y),
        NSSize::new((max_x - min_x).max(1.0), (max_y - min_y).max(1.0)),
    );
    state.screen_height = max_y - min_y;
    state.frame_origin_x = frame.origin.x;
    state.frame_origin_y = frame.origin.y;
    log::info!("[OVERLAY] Overlay frame: origin=({},{}), size=({},{}) screen_h={}", 
        min_x, min_y, frame.size.width, frame.size.height, state.screen_height);

    // NSNonactivatingPanelMask (1 << 7) allows the panel to stay on top without taking focus.
    // We use the raw value because it's sometimes missing from the cocoa-rs enums.
    let style_mask = 128_u64; // NSBorderlessWindowMask (0) | NSNonactivatingPanelMask (128)
    let window: id = msg_send![class!(NSPanel), alloc];
    let window: id = msg_send![
        window,
        initWithContentRect: frame
        styleMask: style_mask
        backing: NSBackingStoreType::NSBackingStoreBuffered
        defer: NO
    ];

    if window == nil {
        return Err("failed to create NSWindow".to_string());
    }

    let _: () = msg_send![window, setOpaque: NO];
    let clear: id = NSColor::clearColor(nil);
    let _: () = msg_send![window, setBackgroundColor: clear];
    let _: () = msg_send![window, setIgnoresMouseEvents: YES];
    let _: () = msg_send![window, setReleasedWhenClosed: NO];
    let _: () = msg_send![window, setHasShadow: NO];
    let _: () = msg_send![window, setHidesOnDeactivate: NO];
    // canJoinAllSpaces (1 << 0) | fullScreenAuxiliary (1 << 7)
    let _: () = msg_send![window, setCollectionBehavior: (1 << 0) | (1 << 7)];
    let _: () = msg_send![window, setLevel: 2000_i64];

    let content = NSView::alloc(nil).initWithFrame_(frame);
    if content == nil {
        return Err("failed to create overlay content view".to_string());
    }
    let _: () = msg_send![content, setWantsLayer: YES];
    let _: () = msg_send![window, setContentView: content];
    let _: () = msg_send![window, orderFrontRegardless];

    state.window = window as usize;
    state.container = content as usize;
    state.visible = true;
    state.empty_streak = 0;
    state.last_non_empty_at = None;

    Ok(())
}

#[cfg(target_os = "macos")]
unsafe fn render_native_markers(
    state: &mut NativeOverlayState,
    markers: &[TypoMarker],
) -> Result<(), String> {
    use cocoa::appkit::{NSColor, NSView};
    use cocoa::base::{id, nil, NO, YES};
    use cocoa::foundation::{NSPoint, NSRect, NSSize};
    use objc::{class, msg_send, sel, sel_impl};

    ensure_native_overlay(state)?;
    let window = state.window as id;
    let container = state.container as id;

    // Clear old views
    for marker_view in state.marker_views.drain(..) {
        let _: () = msg_send![marker_view as id, removeFromSuperview];
    }

    if markers.is_empty() {
        return Ok(());
    }

    let _: () = msg_send![window, orderFrontRegardless];
    state.last_non_empty_at = Some(Instant::now());

    // Helper to add a line
    // Standard Coordinate System: Top-Left (0,0 is top-left of primary monitor)
    let mut add_line = |x: f64, y_top_left: f64, w: f64, color: (f64, f64, f64, f64), _name: &str| {
        let desktop_top_y = state.frame_origin_y + state.screen_height;
        let screen_y_bl = desktop_top_y - y_top_left;
        let local_y = screen_y_bl - state.frame_origin_y;
        let local_x = x - state.frame_origin_x;

        let rect = NSRect::new(NSPoint::new(local_x, local_y), NSSize::new(w, 2.0));
        let view = NSView::alloc(nil).initWithFrame_(rect);
        let _: () = msg_send![view, setWantsLayer: YES];
        let layer: id = msg_send![view, layer];
        let nscolor: id = msg_send![class!(NSColor), colorWithCalibratedRed: color.0 green: color.1 blue: color.2 alpha: color.3];
        let cg_color: id = msg_send![nscolor, CGColor];
        let _: () = msg_send![layer, setBackgroundColor: cg_color];
        let _: () = msg_send![layer, setCornerRadius: 1.0_f64];
        let _: () = msg_send![container, addSubview: view];
        state.marker_views.push(view as usize);
    };

    for (i, marker) in markers.iter().enumerate() {
        // Both native (AXBoundsForRange) and fallback markers store Y in the
        // top-left coordinate system (y=0 at top of primary screen, increases
        // downward — same as Core Graphics / CGDisplay).
        //
        // For native markers, marker.y is the TOP of the character bounding
        // rect; the underline should sit at the BOTTOM edge, so we add height.
        // For fallback markers, lib.rs already stores final_y at the bottom
        // of the text line, so we use it directly.
        let is_fallback = marker.id.contains("fallback");
        let y_top_left = if is_fallback {
            marker.y
        } else {
            // Bottom of character bounding box (top-left coords, y increases down)
            marker.y + marker.height
        };

        add_line(marker.x, y_top_left, marker.width, (1.0, 0.1, 0.1, 0.95), &format!("MARKER_{}", i));
    }

    Ok(())
}
