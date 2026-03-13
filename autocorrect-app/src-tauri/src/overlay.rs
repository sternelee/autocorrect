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
            let (ul_style, ul_color) = crate::commands::config::get_underline_config();
            let _ = self.handle.run_on_main_thread(move || {
                if let Ok(mut guard) = state.lock() {
                    if let Err(err) =
                        unsafe { render_native_markers(&mut guard, &markers, &ul_style, &ul_color) }
                    {
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
    log::info!(
        "[OVERLAY] Overlay frame: origin=({},{}), size=({},{}) screen_h={}",
        min_x,
        min_y,
        frame.size.width,
        frame.size.height,
        state.screen_height
    );

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

/// Parse "#rrggbb" into (r, g, b, alpha) floats in [0.0, 1.0].
fn parse_hex_color(hex: &str) -> (f64, f64, f64, f64) {
    let h = hex.trim_start_matches('#');
    if h.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&h[0..2], 16),
            u8::from_str_radix(&h[2..4], 16),
            u8::from_str_radix(&h[4..6], 16),
        ) {
            return (r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0, 0.95);
        }
    }
    (1.0, 0.23, 0.19, 0.95) // fallback red
}

// CGPath C API — available on all macOS targets via CoreGraphics.framework.
#[cfg(target_os = "macos")]
extern "C" {
    fn CGPathCreateMutable() -> *mut std::ffi::c_void;
    fn CGPathMoveToPoint(path: *mut std::ffi::c_void, m: *const std::ffi::c_void, x: f64, y: f64);
    fn CGPathAddQuadCurveToPoint(
        path: *mut std::ffi::c_void,
        m: *const std::ffi::c_void,
        cpx: f64,
        cpy: f64,
        x: f64,
        y: f64,
    );
    fn CGPathAddLineToPoint(
        path: *mut std::ffi::c_void,
        m: *const std::ffi::c_void,
        x: f64,
        y: f64,
    );
    fn CGPathRelease(path: *mut std::ffi::c_void);
}

#[cfg(target_os = "macos")]
unsafe fn render_native_markers(
    state: &mut NativeOverlayState,
    markers: &[TypoMarker],
    ul_style: &str,
    ul_color: &str,
) -> Result<(), String> {
    use cocoa::appkit::{NSColor, NSView};
    use cocoa::base::{id, nil, YES};
    use cocoa::foundation::{NSPoint, NSRect, NSSize};
    use objc::{class, msg_send, sel, sel_impl};

    ensure_native_overlay(state)?;
    let window = state.window as id;
    let container = state.container as id;

    for marker_view in state.marker_views.drain(..) {
        let _: () = msg_send![marker_view as id, removeFromSuperview];
    }

    if markers.is_empty() {
        return Ok(());
    }

    let _: () = msg_send![window, orderFrontRegardless];
    state.last_non_empty_at = Some(Instant::now());

    let color = parse_hex_color(ul_color);

    // Pre-extract state fields to avoid borrow conflicts in closures.
    let desktop_top_y = state.frame_origin_y + state.screen_height;
    let origin_x = state.frame_origin_x;
    let origin_y = state.frame_origin_y;

    // Convert top-left screen coordinates to NSWindow local (bottom-left) coordinates.
    let to_local =
        |x: f64, y_tl: f64| -> (f64, f64) { (x - origin_x, desktop_top_y - y_tl - origin_y) };

    let make_cg_color = |r: f64, g: f64, b: f64, a: f64| -> id {
        let ns: id = msg_send![class!(NSColor),
            colorWithCalibratedRed: r green: g blue: b alpha: a];
        msg_send![ns, CGColor]
    };

    for marker in markers.iter() {
        let is_fallback = marker.id.contains("fallback");
        let y = if is_fallback {
            marker.y
        } else {
            marker.y + marker.height
        };
        let x = marker.x;
        let w = marker.width;

        match ul_style {
            "wavy" => {
                // Draw a smooth bezier wave using CAShapeLayer + CGPath.
                // The wave is centred on the underline position; the view is
                // tall enough to contain the full amplitude.
                let amp = 2.0_f64; // ±2 px vertical swing
                let period = 8.0_f64; // pixels per full wave cycle
                let view_h = amp * 2.0 + 2.0; // total view height

                // Position the view so its vertical centre lands on y.
                let (local_x, base_local_y) = to_local(x, y);
                let local_y = base_local_y - view_h / 2.0;

                let rect = NSRect::new(NSPoint::new(local_x, local_y), NSSize::new(w, view_h));
                let view: id = NSView::alloc(nil).initWithFrame_(rect);
                let _: () = msg_send![view, setWantsLayer: YES];

                // Build the bezier path in view-local coordinates.
                // y=0 is bottom of view; mid = view_h/2 is the wave baseline.
                let mid = view_h / 2.0;
                let path = CGPathCreateMutable();
                CGPathMoveToPoint(path, std::ptr::null(), 0.0, mid);
                let mut px = 0.0_f64;
                while px + period <= w {
                    // Up-hump: control point above baseline
                    CGPathAddQuadCurveToPoint(
                        path,
                        std::ptr::null(),
                        px + period * 0.25,
                        mid + amp,
                        px + period * 0.5,
                        mid,
                    );
                    // Down-hump: control point below baseline
                    CGPathAddQuadCurveToPoint(
                        path,
                        std::ptr::null(),
                        px + period * 0.75,
                        mid - amp,
                        px + period,
                        mid,
                    );
                    px += period;
                }
                // Close out any remaining width with a straight line.
                if px < w {
                    CGPathAddLineToPoint(path, std::ptr::null(), w, mid);
                }

                // Create CAShapeLayer and configure it.
                let shape: id = msg_send![class!(CAShapeLayer), layer];
                let _: () = msg_send![shape, setPath: path];
                CGPathRelease(path);

                let cg_color = make_cg_color(color.0, color.1, color.2, color.3);
                let _: () = msg_send![shape, setStrokeColor: cg_color];
                let clear_ns: id = NSColor::clearColor(nil);
                let clear_cg: id = msg_send![clear_ns, CGColor];
                let _: () = msg_send![shape, setFillColor: clear_cg];
                let _: () = msg_send![shape, setLineWidth: 1.5_f64];

                let view_layer: id = msg_send![view, layer];
                let _: () = msg_send![view_layer, addSublayer: shape];
                let _: () = msg_send![container, addSubview: view];
                state.marker_views.push(view as usize);
            }
            "dashed" => {
                let dash = 6.0_f64;
                let gap = 3.0_f64;
                let mut px = x;
                while px < x + w {
                    let seg = dash.min(x + w - px);
                    if seg > 0.0 {
                        let (lx, ly) = to_local(px, y);
                        let rect = NSRect::new(NSPoint::new(lx, ly), NSSize::new(seg, 2.0));
                        let view: id = NSView::alloc(nil).initWithFrame_(rect);
                        let _: () = msg_send![view, setWantsLayer: YES];
                        let layer: id = msg_send![view, layer];
                        let cg = make_cg_color(color.0, color.1, color.2, color.3);
                        let _: () = msg_send![layer, setBackgroundColor: cg];
                        let _: () = msg_send![layer, setCornerRadius: 1.0_f64];
                        let _: () = msg_send![container, addSubview: view];
                        state.marker_views.push(view as usize);
                    }
                    px += dash + gap;
                }
            }
            "dotted" => {
                let dot = 2.0_f64;
                let gap = 2.0_f64;
                let mut px = x;
                while px < x + w {
                    let seg = dot.min(x + w - px);
                    if seg > 0.0 {
                        let (lx, ly) = to_local(px, y);
                        let rect = NSRect::new(NSPoint::new(lx, ly), NSSize::new(seg, 2.0));
                        let view: id = NSView::alloc(nil).initWithFrame_(rect);
                        let _: () = msg_send![view, setWantsLayer: YES];
                        let layer: id = msg_send![view, layer];
                        let cg = make_cg_color(color.0, color.1, color.2, color.3);
                        let _: () = msg_send![layer, setBackgroundColor: cg];
                        let _: () = msg_send![layer, setCornerRadius: 1.0_f64];
                        let _: () = msg_send![container, addSubview: view];
                        state.marker_views.push(view as usize);
                    }
                    px += dot + gap;
                }
            }
            _ => {
                // solid
                let (lx, ly) = to_local(x, y);
                let rect = NSRect::new(NSPoint::new(lx, ly), NSSize::new(w, 2.0));
                let view: id = NSView::alloc(nil).initWithFrame_(rect);
                let _: () = msg_send![view, setWantsLayer: YES];
                let layer: id = msg_send![view, layer];
                let cg = make_cg_color(color.0, color.1, color.2, color.3);
                let _: () = msg_send![layer, setBackgroundColor: cg];
                let _: () = msg_send![layer, setCornerRadius: 1.0_f64];
                let _: () = msg_send![container, addSubview: view];
                state.marker_views.push(view as usize);
            }
        }
    }

    Ok(())
}
