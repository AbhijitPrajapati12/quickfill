//! Short codes: watches typing through a low-level keyboard hook and, when the
//! last characters typed match an item's code (e.g. ":gh"), erases the code and
//! pastes the item's value.
//!
//! Privacy: only the last `BUFFER_LEN` characters are kept, in memory, and the
//! buffer is cleared on Enter/Tab/navigation, shortcuts, mouse clicks and
//! window switches.
//! Nothing typed is ever logged or written to disk.

use std::cell::RefCell;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::{OnceLock, RwLock};
use std::thread::{self, sleep};
use std::time::Duration;

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, GetKeyState, GetKeyboardLayout, ToUnicodeEx, VK_BACK, VK_CAPITAL,
    VK_CONTROL, VK_DELETE, VK_DOWN, VK_END, VK_ESCAPE, VK_HOME, VK_LCONTROL, VK_LEFT, VK_LMENU,
    VK_LSHIFT, VK_LWIN, VK_MENU, VK_NEXT, VK_PRIOR, VK_RCONTROL, VK_RETURN, VK_RIGHT, VK_RMENU,
    VK_RSHIFT, VK_RWIN, VK_SHIFT, VK_TAB, VK_UP,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetForegroundWindow, GetMessageW, GetWindowThreadProcessId,
    PostThreadMessageW, SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx, HC_ACTION,
    KBDLLHOOKSTRUCT, LLKHF_INJECTED, MSG, WH_KEYBOARD_LL, WH_MOUSE_LL, WM_KEYDOWN, WM_LBUTTONDOWN,
    WM_MBUTTONDOWN, WM_QUIT, WM_RBUTTONDOWN, WM_SYSKEYDOWN,
};

use crate::paste;
use crate::store::Item;

const BUFFER_LEN: usize = 32;

/// (code, value) pairs the hook matches against.
static CODES: RwLock<Vec<(String, String)>> = RwLock::new(Vec::new());
/// Hands matches from the hook thread to the worker that does the typing.
static WORKER: OnceLock<Sender<Expansion>> = OnceLock::new();
/// Thread id of the running hook thread (0 = not running).
static HOOK_THREAD: AtomicU32 = AtomicU32::new(0);

struct Expansion {
    erase: usize,
    value: String,
}

thread_local! {
    static BUFFER: RefCell<String> = const { RefCell::new(String::new()) };
    static LAST_WINDOW: RefCell<isize> = const { RefCell::new(0) };
}

pub fn set_codes(items: &[Item]) {
    let codes = items
        .iter()
        .filter_map(|i| i.code.clone().map(|c| (c, i.value.clone())))
        .filter(|(_, value)| !value.is_empty())
        .collect();
    *CODES.write().unwrap() = codes;
}

/// Turns short codes on or off. Off fully uninstalls the keyboard hook.
pub fn set_enabled(enabled: bool) {
    if enabled {
        start();
    } else {
        stop();
    }
}

fn start() {
    if HOOK_THREAD.load(Ordering::SeqCst) != 0 {
        return;
    }
    WORKER.get_or_init(spawn_worker);
    let (ready_tx, ready_rx) = channel();
    thread::spawn(move || unsafe {
        let module = GetModuleHandleW(None).ok().map(|m| m.into());
        let hook = match SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), module, 0) {
            Ok(hook) => hook,
            Err(e) => {
                eprintln!("QuickFill: couldn't install keyboard hook: {e}");
                let _ = ready_tx.send(());
                return;
            }
        };
        // A click may move the text cursor to another field, so clicks clear
        // the buffer too. Without this hook, codes still work, just less strictly.
        let mouse_hook = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), module, 0).ok();
        HOOK_THREAD.store(GetCurrentThreadId(), Ordering::SeqCst);
        let _ = ready_tx.send(());

        // Low-level hooks are called through this thread's message loop.
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).0 > 0 {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        let _ = UnhookWindowsHookEx(hook);
        if let Some(mouse_hook) = mouse_hook {
            let _ = UnhookWindowsHookEx(mouse_hook);
        }
    });
    let _ = ready_rx.recv();
}

fn stop() {
    let id = HOOK_THREAD.swap(0, Ordering::SeqCst);
    if id != 0 {
        unsafe {
            let _ = PostThreadMessageW(id, WM_QUIT, WPARAM(0), LPARAM(0));
        }
    }
}

fn spawn_worker() -> Sender<Expansion> {
    let (tx, rx) = channel::<Expansion>();
    thread::spawn(move || {
        for job in rx {
            // Let the target app process the code's last keystroke first.
            sleep(Duration::from_millis(40));
            paste::send_backspaces(job.erase);
            sleep(Duration::from_millis(20));
            if let Err(e) = paste::paste_text(&job.value) {
                eprintln!("QuickFill: short code paste failed: {e}");
            }
        }
    });
    tx
}

unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let msg = wparam.0 as u32;
        if msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN {
            let kb = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
            // Skip synthetic input, including our own backspaces and Ctrl+V.
            if kb.flags.0 & LLKHF_INJECTED.0 == 0 {
                on_key_down(kb.vkCode, kb.scanCode);
            }
        }
    }
    CallNextHookEx(None, code, wparam, lparam)
}

unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32
        && matches!(wparam.0 as u32, WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN)
    {
        clear();
    }
    CallNextHookEx(None, code, wparam, lparam)
}

fn on_key_down(vk: u32, scan: u32) {
    let foreground = unsafe { GetForegroundWindow() };

    // Typing in a different window starts fresh.
    let switched = LAST_WINDOW.with(|last| {
        let changed = *last.borrow() != foreground.0 as isize;
        *last.borrow_mut() = foreground.0 as isize;
        changed
    });
    if switched {
        clear();
    }

    if is_modifier(vk) {
        return;
    }
    // Never expand inside QuickFill itself (e.g. while editing a code).
    if is_own_window(foreground) || shortcut_held() {
        clear();
        return;
    }
    if vk == VK_BACK.0 as u32 {
        BUFFER.with(|b| {
            b.borrow_mut().pop();
        });
        return;
    }
    if is_reset_key(vk) {
        clear();
        return;
    }

    let Some(typed) = key_to_text(vk, scan, foreground) else { return };
    if typed.chars().any(char::is_control) {
        clear();
        return;
    }

    let matched = BUFFER.with(|b| {
        let mut buffer = b.borrow_mut();
        buffer.push_str(&typed);
        let excess = buffer.chars().count().saturating_sub(BUFFER_LEN);
        if excess > 0 {
            let cut = buffer.char_indices().nth(excess).map_or(0, |(i, _)| i);
            buffer.drain(..cut);
        }
        let codes = CODES.read().unwrap();
        codes
            .iter()
            .find(|(code, _)| buffer.ends_with(code.as_str()))
            .map(|(code, value)| Expansion { erase: code.chars().count(), value: value.clone() })
    });

    if let Some(job) = matched {
        clear();
        if let Some(worker) = WORKER.get() {
            let _ = worker.send(job);
        }
    }
}

fn clear() {
    BUFFER.with(|b| b.borrow_mut().clear());
}

/// Converts a key press to the character(s) it types, using the keyboard layout
/// of the app being typed into.
fn key_to_text(vk: u32, scan: u32, foreground: HWND) -> Option<String> {
    unsafe {
        let mut state = [0u8; 256];
        if is_down(VK_SHIFT.0) {
            state[VK_SHIFT.0 as usize] = 0x80;
        }
        if GetKeyState(VK_CAPITAL.0 as i32) & 1 != 0 {
            state[VK_CAPITAL.0 as usize] = 0x01;
        }
        let thread = GetWindowThreadProcessId(foreground, None);
        let layout = GetKeyboardLayout(thread);
        let mut out = [0u16; 8];
        // Flag 0x4: don't change keyboard state, so dead keys (accents) keep
        // working in the app being typed into.
        let n = ToUnicodeEx(vk, scan, &state, &mut out, 0x4, Some(layout));
        (n > 0).then(|| String::from_utf16_lossy(&out[..n as usize]))
    }
}

fn is_own_window(hwnd: HWND) -> bool {
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    pid == std::process::id()
}

fn is_down(vk: u16) -> bool {
    unsafe { GetAsyncKeyState(vk as i32) as u16 & 0x8000 != 0 }
}

/// Ctrl/Alt/Win combos are shortcuts, not typing.
fn shortcut_held() -> bool {
    [VK_CONTROL, VK_MENU, VK_LWIN, VK_RWIN].iter().any(|vk| is_down(vk.0))
}

fn is_modifier(vk: u32) -> bool {
    [
        VK_SHIFT, VK_LSHIFT, VK_RSHIFT, VK_CONTROL, VK_LCONTROL, VK_RCONTROL, VK_MENU, VK_LMENU,
        VK_RMENU, VK_LWIN, VK_RWIN, VK_CAPITAL,
    ]
    .iter()
    .any(|m| m.0 as u32 == vk)
}

fn is_reset_key(vk: u32) -> bool {
    [
        VK_RETURN, VK_TAB, VK_ESCAPE, VK_LEFT, VK_RIGHT, VK_UP, VK_DOWN, VK_HOME, VK_END,
        VK_PRIOR, VK_NEXT, VK_DELETE,
    ]
    .iter()
    .any(|k| k.0 as u32 == vk)
}
