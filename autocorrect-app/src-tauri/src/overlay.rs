use cocoa::base::id;
use core_graphics::display::CGRect;
use objc::{msg_send, sel, sel_impl};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, WebviewWindow, WebviewWindowBuilder};

#[derive(Debug, Serialize, Clone)]
pub struct TypoMarker {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub text: String,
}

/// 管理 Overlay 标记窗口
pub struct OverlayManager {
    handle: AppHandle,
}

impl OverlayManager {
    pub fn new(handle: AppHandle) -> Self {
        Self { handle }
    }

    /// 获取或创建全屏透明窗口
    pub fn get_or_create_overlay(&self) -> tauri::Result<WebviewWindow> {
        if let Some(window) = self.handle.get_webview_window("overlay") {
            Ok(window)
        } else {
            let win = WebviewWindowBuilder::new(
                &self.handle,
                "overlay",
                tauri::WebviewUrl::App("overlay.html".into()),
            )
            .transparent(true)
            // .fullscreen(true) // 暂时用大窗口代替，方便调试
            .inner_size(2000.0, 1200.0)
            .always_on_top(true)
            .decorations(false)
            .shadow(false)
            .visible(false)
            .build()?;

            // 设置点击穿透 (macOS 专用)
            #[cfg(target_os = "macos")]
            {
                let ns_win = win.ns_window().unwrap() as id;
                unsafe {
                    let _: () = msg_send![ns_win, setIgnoresMouseEvents: true];
                }
            }

            Ok(win)
        }
    }

    /// 更新屏幕上的错误下划线标记
    pub fn update_markers(&self, markers: Vec<TypoMarker>) {
        if let Some(window) = self.handle.get_webview_window("overlay") {
            if markers.is_empty() {
                let _ = window.hide();
            } else {
                let _ = window.show();
                let _ = window.emit("update-markers", markers);
            }
        }
    }
}
