#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;

use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{Manager, State};

use config::{Config, ConfigStore, Drive};

struct AppState {
    store: Mutex<ConfigStore>,
}

#[derive(Serialize, Deserialize)]
struct DriveUpdate {
    path: String,
    name: Option<String>,
    on: Option<bool>,
    private: Option<bool>,
}

#[tauri::command]
fn get_config(state: State<AppState>) -> Result<Config, String> {
    state.store.lock().unwrap().get().map_err(err)
}

#[tauri::command]
fn get_config_path(state: State<AppState>) -> String {
    state
        .store
        .lock()
        .unwrap()
        .path()
        .display()
        .to_string()
}

#[tauri::command]
fn add_drive(state: State<AppState>, path: String) -> Result<Config, String> {
    let mut store = state.store.lock().unwrap();
    let mut cfg = store.get().map_err(err)?;
    if cfg.drives.iter().any(|d| d.path == path) {
        return Ok(cfg);
    }
    let name = std::path::Path::new(&path)
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.clone());
    cfg.drives.push(Drive {
        path,
        name,
        on: false,
        private: true,
        url: String::new(),
    });
    store.save(&cfg).map_err(err)?;
    Ok(cfg)
}

#[tauri::command]
fn remove_drive(state: State<AppState>, path: String) -> Result<Config, String> {
    let mut store = state.store.lock().unwrap();
    let mut cfg = store.get().map_err(err)?;
    cfg.drives.retain(|d| d.path != path);
    store.save(&cfg).map_err(err)?;
    Ok(cfg)
}

#[tauri::command]
fn update_drive(state: State<AppState>, update: DriveUpdate) -> Result<Config, String> {
    let mut store = state.store.lock().unwrap();
    let mut cfg = store.get().map_err(err)?;
    if let Some(d) = cfg.drives.iter_mut().find(|d| d.path == update.path) {
        if let Some(n) = update.name {
            d.name = n;
        }
        if let Some(o) = update.on {
            d.on = o;
        }
        if let Some(p) = update.private {
            d.private = p;
        }
    }
    store.save(&cfg).map_err(err)?;
    Ok(cfg)
}

#[tauri::command]
fn forget_all(state: State<AppState>) -> Result<Config, String> {
    let mut store = state.store.lock().unwrap();
    store.delete().map_err(err)?;
    store.get().map_err(err)
}

#[tauri::command]
fn show_settings(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("settings") {
        w.show().map_err(err)?;
        w.set_focus().map_err(err)?;
    }
    Ok(())
}

fn err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

fn main() {
    let store = ConfigStore::new().expect("failed to init config store");

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            store: Mutex::new(store),
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            get_config_path,
            add_drive,
            remove_drive,
            update_drive,
            forget_all,
            show_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
