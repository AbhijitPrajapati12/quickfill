// Hide the console window in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod crypto;
mod expander;
mod hotkey;
mod paste;
mod picker;
mod state;
mod store;

use std::sync::atomic::AtomicIsize;
use std::sync::Mutex;

use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
use tauri_plugin_global_shortcut::ShortcutState;

use state::AppState;

/// Passed by the Windows Run-key entry so we know to start quietly in the tray.
const AUTOSTART_FLAG: &str = "--autostart";

fn main() {
    tauri::Builder::default()
        // Must be first: a second launch just shows the running instance.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main(app);
        }))
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![AUTOSTART_FLAG]),
        ))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        picker::toggle(app);
                    }
                })
                .build(),
        )
        .setup(|app| {
            let handle = app.handle().clone();
            let path = app.path().app_data_dir()?.join("data.bin");
            let (mut data, first_run) = store::load(&path);

            // The Run key is the source of truth for start-with-Windows.
            data.settings.autostart = handle.autolaunch().is_enabled().unwrap_or(false);

            expander::set_codes(&data.items);
            expander::set_enabled(data.settings.codes_enabled);
            let hotkey = data.settings.hotkey.clone();
            let codes_enabled = data.settings.codes_enabled;

            if first_run {
                let _ = store::save(&path, &data);
            }
            app.manage(AppState {
                data: Mutex::new(data),
                path,
                prev_hwnd: AtomicIsize::new(0),
                hotkey_error: Mutex::new(None),
                pause_item: Mutex::new(None),
            });

            // Windows are created only now (`"create": false` in tauri.conf.json):
            // a window created earlier can call get_data before AppState exists.
            for config in &app.config().app.windows {
                WebviewWindowBuilder::from_config(&handle, config)?.build()?;
            }

            hotkey::register_initial(&handle, &hotkey);
            build_tray(&handle, codes_enabled)?;

            let autostarted = std::env::args().any(|a| a == AUTOSTART_FLAG);
            let hotkey_failed = app.state::<AppState>().hotkey_error.lock().unwrap().is_some();
            if !autostarted || first_run || hotkey_failed {
                show_main(&handle);
            }
            Ok(())
        })
        .on_window_event(|window, event| match (window.label(), event) {
            // Closing the manager just hides it; the app keeps running in the tray.
            ("main", WindowEvent::CloseRequested { api, .. }) => {
                api.prevent_close();
                let _ = window.hide();
            }
            ("picker", WindowEvent::Focused(false)) => {
                let _ = window.hide();
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_data,
            commands::save_items,
            commands::set_hotkey,
            commands::set_autostart,
            commands::set_codes_enabled,
            commands::paste_item,
            commands::hide_picker,
        ])
        .run(tauri::generate_context!())
        .expect("error while running QuickFill");
}

fn show_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn build_tray(app: &AppHandle, codes_enabled: bool) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open QuickFill", true, None::<&str>)?;
    let pause = CheckMenuItem::with_id(
        app,
        "pause",
        "Pause short codes",
        true,
        !codes_enabled,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&open, &pause, &separator, &quit])?;
    *app.state::<AppState>().pause_item.lock().unwrap() = Some(pause);

    TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().cloned().expect("app icon missing"))
        .tooltip("QuickFill")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_main(app),
            "pause" => {
                let enabled = !app.state::<AppState>().data.lock().unwrap().settings.codes_enabled;
                if let Err(e) = commands::apply_codes_enabled(app, enabled) {
                    eprintln!("QuickFill: {e}");
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}
