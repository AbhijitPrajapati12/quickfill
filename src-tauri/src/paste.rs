//! Pasting text into whatever app has focus: swap the clipboard, send Ctrl+V,
//! then put the user's original clipboard text back.

use std::thread::sleep;
use std::time::{Duration, Instant};

use arboard::{Clipboard, SetExtWindows};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS,
    KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_BACK, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT, VK_V,
};
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, SetForegroundWindow};

/// How long the target app gets to read the clipboard before we restore it.
const RESTORE_DELAY: Duration = Duration::from_millis(300);

pub fn paste_text(text: &str) -> Result<(), String> {
    let mut clipboard = Clipboard::new().map_err(|e| format!("Clipboard unavailable: {e}"))?;
    // Only text can be restored; images or files on the clipboard are lost.
    let previous = clipboard.get_text().ok();
    clipboard
        .set()
        .exclude_from_history()
        .text(to_crlf(text))
        .map_err(|e| format!("Couldn't set clipboard: {e}"))?;

    wait_for_modifiers_released();
    send_keys(&[(VK_CONTROL, false), (VK_V, false), (VK_V, true), (VK_CONTROL, true)]);

    sleep(RESTORE_DELAY);
    if let Some(previous) = previous {
        let _ = clipboard.set().exclude_from_history().text(previous);
    }
    Ok(())
}

/// Windows apps expect CRLF line endings; some (older editors, many web
/// forms) collapse a bare LF into nothing.
fn to_crlf(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\n', "\r\n")
}

pub fn send_backspaces(count: usize) {
    let keys: Vec<_> = (0..count)
        .flat_map(|_| [(VK_BACK, false), (VK_BACK, true)])
        .collect();
    send_keys(&keys);
}

/// Brings `hwnd` to the front and waits (briefly) until Windows confirms it.
pub fn focus_window(hwnd: isize) {
    if hwnd == 0 {
        return;
    }
    let target = HWND(hwnd as _);
    unsafe {
        if GetForegroundWindow() == target {
            return;
        }
        let _ = SetForegroundWindow(target);
        if wait_for_foreground(target, Duration::from_millis(150)) {
            return;
        }
        // Windows sometimes refuses focus changes; a synthetic Alt tap lifts
        // the restriction (a well-known workaround).
        send_keys(&[(VK_MENU, false), (VK_MENU, true)]);
        let _ = SetForegroundWindow(target);
        wait_for_foreground(target, Duration::from_millis(300));
    }
}

fn wait_for_foreground(target: HWND, timeout: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if unsafe { GetForegroundWindow() } == target {
            return true;
        }
        sleep(Duration::from_millis(10));
    }
    false
}

/// If the user is still holding Shift/Ctrl/Alt/Win, our Ctrl+V would turn into
/// another shortcut, so wait (up to 1s) for them to let go.
fn wait_for_modifiers_released() {
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(1) {
        let held = [VK_SHIFT, VK_CONTROL, VK_MENU, VK_LWIN, VK_RWIN]
            .iter()
            .any(|vk| unsafe { GetAsyncKeyState(vk.0 as i32) } as u16 & 0x8000 != 0);
        if !held {
            return;
        }
        sleep(Duration::from_millis(15));
    }
}

fn send_keys(keys: &[(VIRTUAL_KEY, bool)]) {
    let inputs: Vec<INPUT> = keys
        .iter()
        .map(|&(vk, up)| INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    wScan: 0,
                    dwFlags: if up { KEYEVENTF_KEYUP } else { KEYBD_EVENT_FLAGS(0) },
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        })
        .collect();
    unsafe {
        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}
