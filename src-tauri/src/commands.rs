//! Commands invoked from the React UI.

use std::sync::atomic::Ordering;

use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_autostart::ManagerExt;

use crate::state::{AppState, Snapshot};
use crate::store::{self, Item};
use crate::{expander, hotkey, paste, picker};

#[tauri::command]
pub fn get_data(state: State<AppState>) -> Snapshot {
    state.snapshot()
}

#[tauri::command]
pub fn save_items(state: State<AppState>, items: Vec<Item>) -> Result<Snapshot, String> {
    let items = store::normalize(items);
    store::validate(&items)?;
    {
        let mut data = state.data.lock().unwrap();
        let mut next = data.clone();
        next.items = items;
        store::save(&state.path, &next)?;
        expander::set_codes(&next.items);
        *data = next;
    }
    Ok(state.snapshot())
}

#[tauri::command]
pub fn set_hotkey(app: AppHandle, state: State<AppState>, accel: String) -> Result<Snapshot, String> {
    let old = state.data.lock().unwrap().settings.hotkey.clone();
    hotkey::replace(&app, &old, &accel)?;
    *state.hotkey_error.lock().unwrap() = None;
    update_settings(&state, |s| s.hotkey = accel)?;
    Ok(state.snapshot())
}

#[tauri::command]
pub fn set_autostart(app: AppHandle, state: State<AppState>, enabled: bool) -> Result<Snapshot, String> {
    let launcher = app.autolaunch();
    let result = if enabled { launcher.enable() } else { launcher.disable() };
    result.map_err(|e| format!("Couldn't change start-with-Windows: {e}"))?;
    update_settings(&state, |s| s.autostart = enabled)?;
    Ok(state.snapshot())
}

#[tauri::command]
pub fn set_codes_enabled(app: AppHandle, enabled: bool) -> Result<Snapshot, String> {
    apply_codes_enabled(&app, enabled)?;
    Ok(app.state::<AppState>().snapshot())
}

/// Shared by the settings toggle and the tray's "Pause short codes" item.
pub fn apply_codes_enabled(app: &AppHandle, enabled: bool) -> Result<(), String> {
    let state = app.state::<AppState>();
    update_settings(&state, |s| s.codes_enabled = enabled)?;
    expander::set_enabled(enabled);
    if let Some(item) = state.pause_item.lock().unwrap().as_ref() {
        let _ = item.set_checked(!enabled);
    }
    let _ = app.emit_to("main", "data:changed", state.snapshot());
    Ok(())
}

#[tauri::command]
pub async fn paste_item(app: AppHandle, id: String) -> Result<(), String> {
    let state = app.state::<AppState>();
    let value = state
        .data
        .lock()
        .unwrap()
        .items
        .iter()
        .find(|i| i.id == id)
        .map(|i| i.value.clone())
        .ok_or("Item not found")?;
    let prev = state.prev_hwnd.load(Ordering::SeqCst);

    tauri::async_runtime::spawn_blocking(move || {
        // Refocus the original app before hiding, while we still own the foreground.
        paste::focus_window(prev);
        if let Some(picker) = app.get_webview_window("picker") {
            let _ = picker.hide();
        }
        paste::focus_window(prev);
        paste::paste_text(&value)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn hide_picker(app: AppHandle) {
    picker::close(&app);
}

fn update_settings(
    state: &AppState,
    change: impl FnOnce(&mut store::Settings),
) -> Result<(), String> {
    let mut data = state.data.lock().unwrap();
    let mut next = data.clone();
    change(&mut next.settings);
    store::save(&state.path, &next)?;
    *data = next;
    Ok(())
}
