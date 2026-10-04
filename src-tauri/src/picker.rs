//! Showing the popup picker next to the mouse pointer.

use std::sync::atomic::Ordering;

use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize};
use windows::Win32::Foundation::POINT;
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows::Win32::UI::WindowsAndMessaging::{GetCursorPos, GetForegroundWindow};

use crate::paste;
use crate::state::AppState;

/// Picker size in logical pixels (scaled by the monitor's DPI).
const WIDTH: f64 = 248.0;
const HEIGHT: f64 = 232.0;
/// Gap between the pointer and the popup.
const OFFSET: i32 = 6;

/// Hotkey handler: opens the picker, or closes it if it's already open.
pub fn toggle(app: &AppHandle) {
    let Some(picker) = app.get_webview_window("picker") else { return };
    if picker.is_visible().unwrap_or(false) {
        close(app);
        return;
    }

    let state = app.state::<AppState>();
    let foreground = unsafe { GetForegroundWindow() };
    state.prev_hwnd.store(foreground.0 as isize, Ordering::SeqCst);

    let (x, y, w, h) = placement();
    let _ = picker.set_position(PhysicalPosition::new(x, y));
    let _ = picker.set_size(PhysicalSize::new(w, h));
    let _ = app.emit_to("picker", "picker:open", state.snapshot());
    let _ = picker.show();
    let _ = picker.set_focus();
}

/// Hides the picker and gives focus back to the app the user was typing in.
pub fn close(app: &AppHandle) {
    let prev = app.state::<AppState>().prev_hwnd.load(Ordering::SeqCst);
    // Refocus first, while we still own the foreground; Windows is stricter
    // about focus changes once our window is hidden.
    paste::focus_window(prev);
    if let Some(picker) = app.get_webview_window("picker") {
        let _ = picker.hide();
    }
}

/// Position and size in physical pixels: below-right of the pointer, flipped
/// to the other side when it would run off the monitor's work area.
fn placement() -> (i32, i32, u32, u32) {
    unsafe {
        let mut pt = POINT::default();
        let _ = GetCursorPos(&mut pt);
        let monitor = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);

        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        let _ = GetMonitorInfoW(monitor, &mut info);
        let work = info.rcWork;

        let (mut dpi_x, mut dpi_y) = (96u32, 96u32);
        let _ = GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y);
        let scale = dpi_x as f64 / 96.0;
        let w = (WIDTH * scale).round() as i32;
        let h = (HEIGHT * scale).round() as i32;

        let mut x = pt.x + OFFSET;
        if x + w > work.right {
            x = pt.x - w - OFFSET;
        }
        let mut y = pt.y + OFFSET;
        if y + h > work.bottom {
            y = pt.y - h - OFFSET;
        }
        x = x.clamp(work.left, (work.right - w).max(work.left));
        y = y.clamp(work.top, (work.bottom - h).max(work.top));
        (x, y, w as u32, h as u32)
    }
}
