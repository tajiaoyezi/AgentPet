//! AgentPet desktop shell (M0 skeleton).
//!
//! Wires up the three windows (pet-overlay / status-panel / settings), the
//! system tray, the authoritative [`AppState`], the `health` command/event
//! round-trip, and best-effort file logging. No business logic lives here yet
//! (events, pets, notifications, adapters arrive in M1–M6).

mod app_state;
mod commands;
mod config;
mod logging;
mod window;

// Reserved module placeholders for later milestones; empty in M0 (design D7).
mod adapters;
mod event_bus;
mod notify;
mod pet;
mod session;
mod store;

use app_state::AppState;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager, RunEvent, WindowEvent,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Resolve data paths, create the directory tree + settings placeholder, then
    // start logging. Everything here is best-effort: a failure must never crash
    // the app (app-data-paths / app-logging specs).
    let paths = config::paths::AppPaths::resolve();
    let _log_guard = match &paths {
        Ok(p) => {
            if let Err(e) = p.ensure_tree() {
                eprintln!("AgentPet: failed to create data dir tree: {e:#}");
            }
            if let Err(e) = config::settings::ensure_placeholder(p) {
                eprintln!("AgentPet: failed to write settings placeholder: {e:#}");
            }
            logging::init(&p.logs_dir())
        }
        Err(e) => {
            eprintln!("AgentPet: failed to resolve data dir: {e:#}");
            None
        }
    };

    tracing::info!("AgentPet starting (M0 skeleton)");

    let app = tauri::Builder::default()
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![commands::health])
        .setup(|app| {
            // System tray with three menu items.
            let open_panel =
                MenuItem::with_id(app, "open_panel", "打开状态面板", true, None::<&str>)?;
            let open_settings =
                MenuItem::with_id(app, "open_settings", "打开设置", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open_panel, &open_settings, &quit])?;

            let mut tray = TrayIconBuilder::new()
                .tooltip("AgentPet")
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open_panel" => show_window(app, "status-panel"),
                    "open_settings" => show_window(app, "settings"),
                    "quit" => app.exit(0),
                    _ => {}
                });
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            tray.build(app)?;

            // Re-assert overlay topmost after creation (M6 will add Win32 fallback).
            if let Some(overlay) = app.get_webview_window("pet-overlay") {
                window::overlay::reapply_topmost(&overlay);
            }
            for label in ["pet-overlay", "status-panel", "settings"] {
                if app.get_webview_window(label).is_some() {
                    tracing::info!("window created: {label}");
                }
            }
            Ok(())
        })
        .on_window_event(|win, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                // Close-to-tray: hide instead of quitting; only tray "quit" exits.
                api.prevent_close();
                let _ = win.hide();
                tracing::info!("window hidden to tray: {}", win.label());
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building AgentPet");

    app.run(|_app, event| {
        if let RunEvent::Exit = event {
            tracing::info!("AgentPet exiting");
        }
    });
}

/// Show, restore, and focus a window by label (used by the tray menu).
fn show_window(app: &tauri::AppHandle, label: &str) {
    if let Some(win) = app.get_webview_window(label) {
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
    }
}
