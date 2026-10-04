use std::path::PathBuf;
use std::sync::atomic::AtomicIsize;
use std::sync::Mutex;

use serde::Serialize;
use tauri::menu::CheckMenuItem;
use tauri::Wry;

use crate::store::{Data, Item, Settings};

pub struct AppState {
    pub data: Mutex<Data>,
    pub path: PathBuf,
    /// The window that had focus when the picker opened; we paste back into it.
    pub prev_hwnd: AtomicIsize,
    /// Set when the saved hotkey couldn't be registered (e.g. another app owns it).
    pub hotkey_error: Mutex<Option<String>>,
    pub pause_item: Mutex<Option<CheckMenuItem<Wry>>>,
}

/// What the UI receives from every command.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub items: Vec<Item>,
    pub settings: Settings,
    pub hotkey_error: Option<String>,
}

impl AppState {
    pub fn snapshot(&self) -> Snapshot {
        let data = self.data.lock().unwrap();
        Snapshot {
            items: data.items.clone(),
            settings: data.settings.clone(),
            hotkey_error: self.hotkey_error.lock().unwrap().clone(),
        }
    }
}
