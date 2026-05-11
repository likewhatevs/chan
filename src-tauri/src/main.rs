#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod registry;
mod serve;
mod watcher;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::menu::{Menu, MenuItemBuilder, PredefinedMenuItem, WINDOW_SUBMENU_ID};
use tauri::{Manager, RunEvent, State, WindowEvent};

use config::{Config, ConfigStore};
use serve::ServeHandle;

/// Process-wide state. Shared via `Arc` because the serve supervisor
/// hands clones to per-drive reader threads.
pub struct AppState {
    store: Mutex<ConfigStore>,
    /// Live `chan serve` children keyed by canonical drive path.
    /// Holds the captured URL once chan prints it.
    serves: Mutex<HashMap<String, ServeHandle>>,
}

impl AppState {
    /// Set the URL on a running serve handle. Returns `true` on a
    /// real change so the caller can decide whether to emit an
    /// event. Caller must NOT hold `serves` lock.
    pub fn set_serve_url(&self, key: &str, url: &str) -> bool {
        let mut serves = self.serves.lock().unwrap();
        let Some(h) = serves.get_mut(key) else {
            return false;
        };
        if h.url.as_deref() == Some(url) {
            return false;
        }
        h.url = Some(url.to_string());
        true
    }

    /// Last port this drive's `chan serve` bound to, if any. Used
    /// by the supervisor to prefer the same port across restarts so
    /// open browser tabs don't permanently dead-end on reconnect.
    pub fn drive_port(&self, key: &str) -> Option<u16> {
        self.store
            .lock()
            .unwrap()
            .get()
            .ok()?
            .sidecar
            .get(key)
            .and_then(|s| s.last_port)
    }

    /// Persist the port chosen for this drive's serve.
    pub fn set_drive_port(&self, key: &str, port: u16) -> std::io::Result<()> {
        let mut store = self.store.lock().unwrap();
        let mut cfg = store.get()?;
        cfg.sidecar.entry(key.to_string()).or_default().last_port = Some(port);
        store.save(&cfg)
    }
}

/// Merged drive view returned to the frontend. Combines a chan
/// registry entry with desktop sidecar state and the live serve URL.
#[derive(Debug, Clone, Serialize)]
struct Drive {
    path: String,
    name: String,
    on: bool,
    url: String,
}

#[tauri::command]
fn list_drives(state: State<Arc<AppState>>) -> Result<Vec<Drive>, String> {
    let serves = state.serves.lock().unwrap();
    let entries = registry::read().map_err(err)?;

    // `on` is derived from a live serve handle, never persisted.
    // That way a desktop restart comes up with everything off
    // (matching reality: nothing is actually running yet) and
    // there is no chance of a stale on=true sticking around after
    // chan died unexpectedly.
    let merged = entries
        .into_iter()
        .map(|e| {
            let key = canonical_key(&e.path);
            let display_path = key.clone();
            let name = e
                .name
                .or_else(|| basename(&e.path))
                .unwrap_or_else(|| display_path.clone());
            let handle = serves.get(&key);
            let on = handle.is_some();
            let url = handle.and_then(|h| h.url.clone()).unwrap_or_default();
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
fn add_drive(
    app: tauri::AppHandle,
    state: State<Arc<AppState>>,
    path: String,
) -> Result<(), String> {
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

    // Auto-start: opening a drive from the desktop is the user's
    // way of saying "make this drive usable now". Spinning up the
    // serve immediately is what they expect; otherwise the freshly
    // added row sits there with On=off and Launch disabled, which
    // looks broken.
    let verbose = state.store.lock().unwrap().get().map_err(err)?.dev_mode;
    serve::start(app, Arc::clone(&state), path, chan_bin(), verbose)?;
    Ok(())
}

#[tauri::command]
fn remove_drive(state: State<Arc<AppState>>, path: String) -> Result<(), String> {
    let key = canonical_key(Path::new(&path));
    serve::stop(&state, &key);

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
    Ok(())
}

#[tauri::command]
fn set_drive_on(
    app: tauri::AppHandle,
    state: State<Arc<AppState>>,
    path: String,
    on: bool,
) -> Result<(), String> {
    let key = canonical_key(Path::new(&path));
    if on {
        let verbose = state.store.lock().unwrap().get().map_err(err)?.dev_mode;
        serve::start(app, Arc::clone(&state), key, chan_bin(), verbose)?;
    } else {
        serve::stop(&state, &key);
    }
    Ok(())
}

#[tauri::command]
fn get_config(state: State<Arc<AppState>>) -> Result<Config, String> {
    state.store.lock().unwrap().get().map_err(err)
}

#[tauri::command]
fn show_settings(app: tauri::AppHandle) -> Result<(), String> {
    show_window(&app, "settings")
}

#[tauri::command]
fn set_dev_mode(
    app: tauri::AppHandle,
    state: State<Arc<AppState>>,
    enabled: bool,
) -> Result<Config, String> {
    let mut store = state.store.lock().unwrap();
    let mut cfg = store.get().map_err(err)?;
    cfg.dev_mode = enabled;
    store.save(&cfg).map_err(err)?;
    drop(store);
    apply_dev_mode(&app, enabled);
    Ok(cfg)
}

/// Show or hide the console window. Called whenever dev mode flips
/// and on startup if the persisted dev mode is on.
fn apply_dev_mode(app: &tauri::AppHandle, enabled: bool) {
    let Some(w) = app.get_webview_window("console") else {
        return;
    };
    if enabled {
        let _ = w.show();
        let _ = w.set_focus();
    } else {
        let _ = w.hide();
    }
}

fn show_window(app: &tauri::AppHandle, label: &str) -> Result<(), String> {
    if let Some(w) = app.get_webview_window(label) {
        w.show().map_err(err)?;
        w.set_focus().map_err(err)?;
    }
    Ok(())
}

/// Canonical-path key used for sidecar lookups, serve identity, and
/// the displayed path. `canonicalize` falls back to the input on
/// error so we still produce a stable key for not-yet-existing or
/// asleep paths.
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
/// binary path (production) is documented in design.md section 5.2.
fn chan_bin() -> &'static str {
    "chan"
}

fn err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

fn main() {
    let store = ConfigStore::new().expect("failed to init config store");
    let state = Arc::new(AppState {
        store: Mutex::new(store),
        serves: Mutex::new(HashMap::new()),
    });
    let state_for_exit = Arc::clone(&state);

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(state)
        .setup(|app| {
            let state: State<Arc<AppState>> = app.state();
            let cfg = state.store.lock().unwrap().get().unwrap_or_default();
            if cfg.dev_mode {
                apply_dev_mode(app.handle(), true);
            }

            install_app_menu(app.handle())?;

            // Closing the main window via the red traffic light or
            // Cmd+W should hide it, not destroy it: hidden serve
            // children, settings, and console windows can still keep
            // the process alive, and reopening via Dock click or the
            // Window > Drive Manager menu item should be instant.
            // Without this, a closed main window cannot be brought
            // back without quitting and relaunching.
            if let Some(main) = app.get_webview_window("main") {
                let main_for_event = main.clone();
                main.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = main_for_event.hide();
                    }
                });
                let _ = main.show();
                let _ = main.set_focus();
            }

            // Registry watcher. Leaked: we want it alive for the
            // process lifetime and the inner Watcher type is
            // unnameable through `manage`.
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
            get_config,
            show_settings,
            set_dev_mode,
        ])
        .build(tauri::generate_context!())
        .expect("error building tauri application");

    app.run(move |app, event| {
        match event {
            RunEvent::Exit => {
                // Best-effort: SIGKILL every running chan child so
                // they don't outlive the desktop. The OS reclaims
                // the ports within seconds.
                serve::stop_all(&state_for_exit);
            }
            // macOS: Dock click or `open -a` while the process is
            // still alive. If no windows are visible (main has been
            // hidden / closed and the user has no drive windows
            // open), bring the main window back.
            #[cfg(target_os = "macos")]
            RunEvent::Reopen {
                has_visible_windows,
                ..
            } => {
                if !has_visible_windows {
                    let _ = show_window(app, "main");
                }
            }
            _ => {}
        }
    });
}

/// Inject window-navigation items into the default Tauri menu.
/// Tauri's `Menu::default` produces the standard macOS menubar
/// (app / File / Edit / View / Window / Help) but its Window
/// submenu only has Minimize / Zoom / Close — a closed main
/// window has no menu path back. We prepend Drive Manager,
/// Settings, and Logs items to that submenu so each app window
/// is reachable by name.
fn install_app_menu(app: &tauri::AppHandle) -> tauri::Result<()> {
    let menu = Menu::default(app)?;

    let drive_manager = MenuItemBuilder::with_id("win-main", "Drive Manager")
        .accelerator("CmdOrCtrl+1")
        .build(app)?;
    let settings = MenuItemBuilder::with_id("win-settings", "Settings…")
        .accelerator("CmdOrCtrl+,")
        .build(app)?;
    let logs = MenuItemBuilder::with_id("win-console", "Logs")
        .accelerator("CmdOrCtrl+L")
        .build(app)?;

    if let Some(window_submenu) = menu.get(WINDOW_SUBMENU_ID).and_then(|k| k.as_submenu().cloned()) {
        let sep = PredefinedMenuItem::separator(app)?;
        window_submenu.prepend_items(&[&drive_manager, &settings, &logs, &sep])?;
    }

    app.set_menu(menu)?;
    app.on_menu_event(|app, event| match event.id().as_ref() {
        "win-main" => {
            let _ = show_window(app, "main");
        }
        "win-settings" => {
            let _ = show_window(app, "settings");
        }
        "win-console" => {
            let _ = show_window(app, "console");
        }
        _ => {}
    });
    Ok(())
}
