#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod registry;
mod watcher;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

use serde::Serialize;
use tauri::{Manager, State};

use config::{Config, ConfigStore};

/// Process-wide state. The mutex is held only for synchronous map
/// updates, never across `.await` (this binary is sync-only at the
/// command layer); shelling out to `chan` happens on Tauri's worker
/// threads via `tauri::command`.
struct AppState {
    store: Mutex<ConfigStore>,
    /// Live serve URLs keyed by canonical drive path. Not persisted:
    /// chan rotates the bearer token on every `chan serve`.
    urls: Mutex<HashMap<String, String>>,
}

/// Merged drive view returned to the frontend. Combines a chan
/// registry entry with desktop sidecar state.
#[derive(Debug, Clone, Serialize)]
struct Drive {
    /// Display path (the canonical form when canonicalisation
    /// succeeded, otherwise the registry's literal path).
    path: String,
    /// Display name. Sourced from chan's registry; the desktop UI
    /// is read-only on names. Falls back to the basename when chan
    /// has not assigned one.
    name: String,
    /// On-toggle state from the desktop sidecar.
    on: bool,
    /// Live serve URL. Empty when no serve is running.
    url: String,
}

#[tauri::command]
fn list_drives(state: State<AppState>) -> Result<Vec<Drive>, String> {
    let cfg = state.store.lock().unwrap().get().map_err(err)?;
    let urls = state.urls.lock().unwrap();
    let entries = registry::read().map_err(err)?;

    let merged = entries
        .into_iter()
        .map(|e| {
            let key = canonical_key(&e.path);
            let display_path = key.clone();
            let name = e
                .name
                .or_else(|| basename(&e.path))
                .unwrap_or_else(|| display_path.clone());
            let on = cfg.sidecar.get(&key).map(|s| s.on).unwrap_or(false);
            let url = urls.get(&key).cloned().unwrap_or_default();
            Drive {
                path: display_path,
                name,
                on,
                url,
            }
        })
        .collect();
    Ok(merged)
}

#[tauri::command]
fn add_drive(path: String) -> Result<(), String> {
    // Defer to the chan binary as the registry's only writer. The
    // watcher will pick up the registry change and refresh the UI.
    let path = canonical_key(Path::new(&path));
    let out = Command::new(chan_bin())
        .args(["add", &path])
        .output()
        .map_err(|e| format!("running `chan add`: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "`chan add` failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(())
}

#[tauri::command]
fn remove_drive(state: State<AppState>, path: String) -> Result<(), String> {
    let key = canonical_key(Path::new(&path));
    let out = Command::new(chan_bin())
        .args(["remove", &key])
        .output()
        .map_err(|e| format!("running `chan remove`: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "`chan remove` failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }

    let mut store = state.store.lock().unwrap();
    let mut cfg = store.get().map_err(err)?;
    cfg.sidecar.remove(&key);
    store.save(&cfg).map_err(err)?;
    state.urls.lock().unwrap().remove(&key);
    Ok(())
}

#[tauri::command]
fn set_drive_on(state: State<AppState>, path: String, on: bool) -> Result<(), String> {
    // Sidecar-only for now. Wiring this to spawn / stop `chan serve`
    // (and capture the URL into AppState.urls) is a separate task.
    let key = canonical_key(Path::new(&path));
    let mut store = state.store.lock().unwrap();
    let mut cfg = store.get().map_err(err)?;
    cfg.sidecar.entry(key).or_default().on = on;
    store.save(&cfg).map_err(err)?;
    Ok(())
}

#[tauri::command]
fn forget_all(state: State<AppState>) -> Result<(), String> {
    // Best-effort: ask chan to forget every registered drive, then
    // wipe our sidecar. Per-drive failures are surfaced together at
    // the end so a single bad entry doesn't block the rest.
    let entries = registry::read().map_err(err)?;
    let mut errors = Vec::new();
    for e in &entries {
        let key = canonical_key(&e.path);
        let out = Command::new(chan_bin()).args(["remove", &key]).output();
        match out {
            Ok(o) if o.status.success() => {}
            Ok(o) => errors.push(format!(
                "{key}: {}",
                String::from_utf8_lossy(&o.stderr).trim()
            )),
            Err(err) => errors.push(format!("{key}: {err}")),
        }
    }
    let mut store = state.store.lock().unwrap();
    store.delete().map_err(err)?;
    state.urls.lock().unwrap().clear();

    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!("partial: {}", errors.join("; ")))
    }
}

#[tauri::command]
fn get_config(state: State<AppState>) -> Result<Config, String> {
    state.store.lock().unwrap().get().map_err(err)
}

#[tauri::command]
fn get_config_path(state: State<AppState>) -> String {
    state.store.lock().unwrap().path().display().to_string()
}

#[tauri::command]
fn get_registry_path() -> String {
    registry::path().display().to_string()
}

#[tauri::command]
fn show_settings(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("settings") {
        w.show().map_err(err)?;
        w.set_focus().map_err(err)?;
    }
    Ok(())
}

#[tauri::command]
fn set_dev_mode(
    app: tauri::AppHandle,
    state: State<AppState>,
    enabled: bool,
) -> Result<Config, String> {
    let mut store = state.store.lock().unwrap();
    let mut cfg = store.get().map_err(err)?;
    cfg.dev_mode = enabled;
    store.save(&cfg).map_err(err)?;
    apply_dev_mode(&app, enabled);
    Ok(cfg)
}

fn apply_dev_mode(app: &tauri::AppHandle, enabled: bool) {
    for (_, win) in app.webview_windows() {
        if enabled {
            win.open_devtools();
        } else {
            win.close_devtools();
        }
    }
}

/// Canonical-path key used for sidecar lookups and as the displayed
/// path. `canonicalize` falls back to the input on error so we still
/// produce a stable key for not-yet-existing or asleep paths.
fn canonical_key(p: &Path) -> String {
    p.canonicalize()
        .unwrap_or_else(|_| PathBuf::from(p))
        .display()
        .to_string()
}

fn basename(p: &Path) -> Option<String> {
    p.file_name().map(|s| s.to_string_lossy().into_owned())
}

/// Resolve the chan binary. Prototype: trust `$PATH`. The bundled
/// binary path (production) is documented in design.md.
fn chan_bin() -> &'static str {
    "chan"
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
            urls: Mutex::new(HashMap::new()),
        })
        .setup(|app| {
            let state: State<AppState> = app.state();
            let cfg = state.store.lock().unwrap().get().unwrap_or_default();
            if cfg.dev_mode {
                apply_dev_mode(app.handle(), true);
            }

            // Spawn the registry watcher. The debouncer owns a
            // background thread we want alive for the rest of the
            // process; we leak it rather than thread the unnameable
            // generic type through Tauri state. Failure is non-fatal:
            // the UI just won't auto-refresh on external changes.
            match watcher::spawn(app.handle().clone(), &registry::path()) {
                Ok(d) => {
                    Box::leak(Box::new(d));
                }
                Err(e) => eprintln!("chan-desktop: registry watcher disabled: {e}"),
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_drives,
            add_drive,
            remove_drive,
            set_drive_on,
            forget_all,
            get_config,
            get_config_path,
            get_registry_path,
            show_settings,
            set_dev_mode,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
