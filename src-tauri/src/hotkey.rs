//! Registering the global hotkey that opens the picker.

use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

use crate::state::AppState;

pub fn parse(accel: &str) -> Result<Shortcut, String> {
    accel
        .parse::<Shortcut>()
        .map_err(|e| format!("\"{accel}\" isn't a valid shortcut: {e}"))
}

/// Registers the hotkey at startup, recording any failure for the UI to show.
pub fn register_initial(app: &AppHandle, accel: &str) {
    let result = parse(accel).and_then(|sc| {
        app.global_shortcut()
            .register(sc)
            .map_err(|e| taken_message(accel, e))
    });
    *app.state::<AppState>().hotkey_error.lock().unwrap() = result.err();
}

/// Swaps the old hotkey for a new one. If the new one can't be registered,
/// the old one is put back so the user is never left without a hotkey.
pub fn replace(app: &AppHandle, old: &str, new: &str) -> Result<(), String> {
    let new_sc = parse(new)?;
    let shortcuts = app.global_shortcut();
    if let Ok(old_sc) = parse(old) {
        let _ = shortcuts.unregister(old_sc);
    }
    match shortcuts.register(new_sc) {
        Ok(()) => Ok(()),
        Err(e) => {
            if let Ok(old_sc) = parse(old) {
                let _ = shortcuts.register(old_sc);
            }
            Err(taken_message(new, e))
        }
    }
}

fn taken_message(accel: &str, e: impl std::fmt::Display) -> String {
    format!("Couldn't use {accel}; another app may already use it. ({e})")
}
