use std::{
    fs,
    io::Write,
    path::PathBuf,
    time::{Duration, SystemTime},
};

use reqwest::header::CONTENT_DISPOSITION;
use serde::Serialize;
use tauri::WebviewWindow;
use tokio::sync::mpsc;
use uuid::Uuid;

const EXPORT_ROOT_DIR: &str = "chan-desktop-drag-out";
const CLEANUP_AFTER: Duration = Duration::from_secs(30 * 60);
const STALE_EXPORT_AFTER: Duration = Duration::from_secs(60 * 60);

#[derive(Debug, Serialize)]
pub struct DragOutResponse {
    started: bool,
    cleanup_after_secs: u64,
}

#[tauri::command]
pub async fn start_file_browser_drag_out(
    window: WebviewWindow,
    path: String,
    is_dir: bool,
    download_url: String,
    filename: Option<String>,
    client_x: Option<f64>,
    client_y: Option<f64>,
) -> Result<DragOutResponse, String> {
    if !native_drag_supported() {
        return Err("native File Browser drag-out is not supported on this platform".to_string());
    }

    cleanup_old_exports();

    let request_url = valid_download_url(&window, &download_url)?;
    let response = reqwest::Client::new()
        .get(request_url)
        .send()
        .await
        .map_err(|e| format!("download export request failed: {e}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "download export request failed with HTTP {}",
            response.status()
        ));
    }

    let header_filename = response
        .headers()
        .get(CONTENT_DISPOSITION)
        .and_then(|value| value.to_str().ok())
        .and_then(content_disposition_filename);
    let export_filename = export_filename(header_filename.or(filename), &path, is_dir);
    let staged = stage_export_response(export_filename, response).await?;

    match start_native_drag(window, staged.file_path.clone(), client_x, client_y).await {
        Ok(true) => {
            schedule_cleanup(staged.dir_path, CLEANUP_AFTER);
            Ok(DragOutResponse {
                started: true,
                cleanup_after_secs: CLEANUP_AFTER.as_secs(),
            })
        }
        Ok(false) => {
            remove_export_dir(staged.dir_path).await;
            Err("native File Browser drag-out was cancelled".to_string())
        }
        Err(err) => {
            remove_export_dir(staged.dir_path).await;
            Err(err)
        }
    }
}

fn valid_download_url(window: &WebviewWindow, raw: &str) -> Result<String, String> {
    let current_url = window
        .url()
        .map_err(|e| format!("reading webview URL failed: {e}"))?;
    validate_download_url(raw, &current_url)
}

fn validate_download_url(raw: &str, current_url: &url::Url) -> Result<String, String> {
    let url = url::Url::parse(raw).map_err(|e| format!("invalid download URL: {e}"))?;
    match url.scheme() {
        "http" | "https" => {}
        scheme => return Err(format!("unsupported download URL scheme: {scheme}")),
    }
    if !same_origin(&url, current_url) {
        return Err("download URL must match the current drive origin".to_string());
    }
    if !url.path().contains("/api/files/") {
        return Err("download URL must target /api/files".to_string());
    }
    let has_download = url
        .query_pairs()
        .any(|(key, value)| key == "download" && value == "1");
    if !has_download {
        return Err("download URL must include download=1".to_string());
    }
    Ok(url.to_string())
}

fn same_origin(left: &url::Url, right: &url::Url) -> bool {
    left.scheme() == right.scheme()
        && left.host_str() == right.host_str()
        && left.port_or_known_default() == right.port_or_known_default()
}

fn content_disposition_filename(raw: &str) -> Option<String> {
    for part in raw.split(';').map(str::trim) {
        let Some(value) = part.strip_prefix("filename=") else {
            continue;
        };
        let value = value.trim();
        if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
            return Some(value[1..value.len() - 1].replace("\\\"", "\""));
        }
        if !value.is_empty() {
            return Some(value.to_string());
        }
    }
    None
}

fn export_filename(candidate: Option<String>, path: &str, is_dir: bool) -> String {
    let raw = candidate
        .as_deref()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| basename_or_download(path));
    let mut out = String::with_capacity(raw.len());
    let base = raw
        .rsplit(['/', '\\'])
        .find(|segment| !segment.is_empty())
        .unwrap_or("download");
    for ch in base.chars() {
        if ch == ':' || ch == '/' || ch == '\\' || ch.is_control() {
            out.push('_');
        } else {
            out.push(ch);
        }
    }
    let trimmed = out.trim();
    let safe = if trimmed.is_empty() || trimmed == "." || trimmed == ".." {
        "download".to_string()
    } else {
        trimmed.to_string()
    };
    if is_dir && !safe.to_ascii_lowercase().ends_with(".tar") {
        format!("{safe}.tar")
    } else {
        safe
    }
}

fn basename_or_download(path: &str) -> &str {
    path.rsplit('/')
        .find(|segment| !segment.is_empty())
        .unwrap_or("download")
}

struct StagedExport {
    dir_path: PathBuf,
    file_path: PathBuf,
}

async fn stage_export_response(
    filename: String,
    mut response: reqwest::Response,
) -> Result<StagedExport, String> {
    let staged = tauri::async_runtime::spawn_blocking(move || create_staged_export_sync(&filename))
        .await
        .map_err(|e| format!("staging export task failed: {e}"))??;
    let (tx, rx) = mpsc::channel::<Vec<u8>>(4);
    let file_path = staged.file_path.clone();
    let writer =
        tauri::async_runtime::spawn_blocking(move || write_export_stream_sync(file_path, rx));

    while let Some(chunk) = match response.chunk().await {
        Ok(chunk) => chunk,
        Err(e) => {
            drop(tx);
            let _ = writer.await;
            remove_export_dir(staged.dir_path).await;
            return Err(format!("reading export bytes failed: {e}"));
        }
    } {
        if tx.send(chunk.to_vec()).await.is_err() {
            drop(tx);
            let result = writer
                .await
                .map_err(|e| format!("export writer task failed: {e}"))?;
            remove_export_dir(staged.dir_path).await;
            return match result {
                Ok(()) => Err("export writer stopped before download completed".to_string()),
                Err(e) => Err(e),
            };
        }
    }
    drop(tx);
    match writer.await {
        Ok(Ok(())) => Ok(staged),
        Ok(Err(e)) => {
            remove_export_dir(staged.dir_path).await;
            Err(e)
        }
        Err(e) => {
            remove_export_dir(staged.dir_path).await;
            Err(format!("export writer task failed: {e}"))
        }
    }
}

fn create_staged_export_sync(filename: &str) -> Result<StagedExport, String> {
    let dir_path = export_root().join(Uuid::new_v4().to_string());
    fs::create_dir_all(&dir_path)
        .map_err(|e| format!("creating export staging directory failed: {e}"))?;
    let file_path = dir_path.join(filename);
    fs::File::create(&file_path)
        .map_err(|e| format!("creating export staging file failed: {e}"))?;
    Ok(StagedExport {
        dir_path,
        file_path,
    })
}

fn write_export_stream_sync(
    file_path: PathBuf,
    mut rx: mpsc::Receiver<Vec<u8>>,
) -> Result<(), String> {
    let mut file = fs::File::create(file_path)
        .map_err(|e| format!("opening export staging file failed: {e}"))?;
    while let Some(chunk) = rx.blocking_recv() {
        file.write_all(&chunk)
            .map_err(|e| format!("writing export staging file failed: {e}"))?;
    }
    file.flush()
        .map_err(|e| format!("flushing export staging file failed: {e}"))
}

fn export_root() -> PathBuf {
    std::env::temp_dir().join(EXPORT_ROOT_DIR)
}

fn cleanup_old_exports() {
    std::mem::drop(tauri::async_runtime::spawn_blocking(|| {
        let root = export_root();
        let now = SystemTime::now();
        let Ok(entries) = fs::read_dir(root) else {
            return;
        };
        for entry in entries.flatten() {
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            if !metadata.is_dir() {
                continue;
            }
            let stale = metadata
                .modified()
                .ok()
                .and_then(|modified| now.duration_since(modified).ok())
                .is_some_and(|age| age > STALE_EXPORT_AFTER);
            if stale {
                let _ = fs::remove_dir_all(entry.path());
            }
        }
    }));
}

fn schedule_cleanup(dir_path: PathBuf, delay: Duration) {
    std::mem::drop(tauri::async_runtime::spawn(async move {
        tokio::time::sleep(delay).await;
        remove_export_dir(dir_path).await;
    }));
}

async fn remove_export_dir(dir_path: PathBuf) {
    let _ = tauri::async_runtime::spawn_blocking(move || fs::remove_dir_all(dir_path)).await;
}

#[cfg(target_os = "macos")]
fn native_drag_supported() -> bool {
    true
}

#[cfg(not(target_os = "macos"))]
fn native_drag_supported() -> bool {
    false
}

#[cfg(target_os = "macos")]
#[allow(deprecated)]
async fn start_native_drag(
    window: WebviewWindow,
    file_path: PathBuf,
    client_x: Option<f64>,
    client_y: Option<f64>,
) -> Result<bool, String> {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSView, NSWindow};
    use objc2_foundation::NSString;
    use tokio::sync::oneshot;

    let (tx, rx) = oneshot::channel();
    let drag_window = window.clone();
    window
        .run_on_main_thread(move || {
            let result = (|| {
                let mtm = MainThreadMarker::new()
                    .ok_or_else(|| "native drag did not run on the main thread".to_string())?;
                let ns_view_ptr = drag_window
                    .ns_view()
                    .map_err(|e| format!("reading native view failed: {e}"))?;
                if ns_view_ptr.is_null() {
                    return Err("native view was null".to_string());
                }
                let ns_window_ptr = drag_window
                    .ns_window()
                    .map_err(|e| format!("reading native window failed: {e}"))?;
                if ns_window_ptr.is_null() {
                    return Err("native window was null".to_string());
                }

                let ns_view = unsafe { &*(ns_view_ptr.cast::<NSView>()) };
                let ns_window = unsafe { &*(ns_window_ptr.cast::<NSWindow>()) };
                let app = NSApplication::sharedApplication(mtm);
                let event = drag_event(&app, ns_view, ns_window, client_x, client_y)
                    .ok_or_else(|| "building native drag event failed".to_string())?;
                let rect = drag_rect(ns_view, client_x, client_y);
                let file_name = NSString::from_str(&file_path_to_drag(&file_path));
                Ok(ns_view.dragFile_fromRect_slideBack_event(&file_name, rect, true, &event))
            })();
            let _ = tx.send(result);
        })
        .map_err(|e| format!("dispatching native drag failed: {e}"))?;
    rx.await
        .map_err(|e| format!("native drag result channel closed: {e}"))?
}

#[cfg(target_os = "macos")]
fn file_path_to_drag(path: &std::path::Path) -> String {
    path.as_os_str().to_string_lossy().into_owned()
}

#[cfg(target_os = "macos")]
#[allow(deprecated)]
fn drag_event(
    app: &objc2_app_kit::NSApplication,
    view: &objc2_app_kit::NSView,
    window: &objc2_app_kit::NSWindow,
    client_x: Option<f64>,
    client_y: Option<f64>,
) -> Option<objc2::rc::Retained<objc2_app_kit::NSEvent>> {
    use objc2_app_kit::{NSEvent, NSEventModifierFlags, NSEventType};

    let current = app.currentEvent();
    if let Some(event) = current.as_ref() {
        let event_type = event.r#type();
        if event_type == NSEventType::LeftMouseDown || event_type == NSEventType::LeftMouseDragged {
            return current;
        }
    }

    let point = drag_point(view, client_x, client_y);
    let flags = current
        .as_ref()
        .map(|event| event.modifierFlags())
        .unwrap_or_else(NSEventModifierFlags::empty);
    let timestamp = current
        .as_ref()
        .map(|event| event.timestamp())
        .unwrap_or_default();
    let event_number = current
        .as_ref()
        .map(|event| event.eventNumber())
        .unwrap_or_default();
    let click_count = current
        .as_ref()
        .map(|event| event.clickCount())
        .unwrap_or(1);
    let pressure = current
        .as_ref()
        .map(|event| event.pressure())
        .unwrap_or(1.0);
    NSEvent::mouseEventWithType_location_modifierFlags_timestamp_windowNumber_context_eventNumber_clickCount_pressure(
        NSEventType::LeftMouseDragged,
        point,
        flags,
        timestamp,
        window.windowNumber(),
        None,
        event_number,
        click_count,
        pressure,
    )
}

#[cfg(target_os = "macos")]
fn drag_rect(
    view: &objc2_app_kit::NSView,
    client_x: Option<f64>,
    client_y: Option<f64>,
) -> objc2_foundation::NSRect {
    let point = drag_point(view, client_x, client_y);
    objc2_foundation::NSRect::new(
        objc2_foundation::NSPoint::new(point.x - 2.0, point.y - 2.0),
        objc2_foundation::NSSize::new(4.0, 4.0),
    )
}

#[cfg(target_os = "macos")]
fn drag_point(
    view: &objc2_app_kit::NSView,
    client_x: Option<f64>,
    client_y: Option<f64>,
) -> objc2_foundation::NSPoint {
    let bounds = view.bounds();
    let x = client_x.unwrap_or(bounds.size.width / 2.0);
    let y = match client_y {
        Some(y) if view.isFlipped() => y,
        Some(y) => bounds.size.height - y,
        None => bounds.size.height / 2.0,
    };
    objc2_foundation::NSPoint::new(x, y)
}

#[cfg(not(target_os = "macos"))]
async fn start_native_drag(
    _window: WebviewWindow,
    _file_path: PathBuf,
    _client_x: Option<f64>,
    _client_y: Option<f64>,
) -> Result<bool, String> {
    Err("native File Browser drag-out is not supported on this platform".to_string())
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use super::*;

    #[test]
    fn export_filename_preserves_file_basename() {
        assert_eq!(
            export_filename(Some("notes/readme.md".to_string()), "fallback.txt", false),
            "readme.md",
        );
        assert_eq!(
            export_filename(None, "nested/original name.txt", false),
            "original name.txt",
        );
    }

    #[test]
    fn export_filename_names_directory_archives_clearly() {
        assert_eq!(export_filename(None, "Projects/Alpha", true), "Alpha.tar");
        assert_eq!(
            export_filename(Some("bundle.tar".to_string()), "Projects/Alpha", true),
            "bundle.tar",
        );
    }

    #[test]
    fn export_filename_sanitizes_unsafe_names() {
        assert_eq!(
            export_filename(Some("bad:name\n.md".to_string()), "fallback", false),
            "bad_name_.md",
        );
        assert_eq!(
            export_filename(Some("..".to_string()), "fallback", false),
            "download"
        );
    }

    #[test]
    fn content_disposition_filename_reads_attachment_filename() {
        assert_eq!(
            content_disposition_filename("attachment; filename=\"readme.md\"").as_deref(),
            Some("readme.md"),
        );
        assert_eq!(
            content_disposition_filename("attachment; filename=notes.tar").as_deref(),
            Some("notes.tar"),
        );
    }

    #[test]
    fn valid_download_url_requires_http_scheme() {
        let current = url::Url::parse("http://127.0.0.1:9000/drive/index.html").unwrap();
        assert!(
            validate_download_url("http://127.0.0.1:9000/api/files/a?download=1", &current).is_ok()
        );
        assert!(validate_download_url("/api/files/a?download=1", &current).is_err());
        assert!(validate_download_url("file:///tmp/a", &current).is_err());
    }

    #[test]
    fn valid_download_url_requires_same_origin_api_download() {
        let current = url::Url::parse("http://127.0.0.1:9000/drive/index.html").unwrap();
        assert!(validate_download_url(
            "http://127.0.0.1:9000/drive/api/files/a?download=1",
            &current
        )
        .is_ok());
        assert!(validate_download_url(
            "http://127.0.0.1:9001/drive/api/files/a?download=1",
            &current
        )
        .is_err());
        assert!(
            validate_download_url("http://127.0.0.1:9000/drive/api/files/a", &current).is_err()
        );
        assert!(
            validate_download_url("http://127.0.0.1:9000/drive/api/search?q=a", &current).is_err()
        );
    }

    #[test]
    fn export_root_is_system_temp_scoped() {
        assert_eq!(export_root().file_name(), Some(OsStr::new(EXPORT_ROOT_DIR)));
    }

    #[test]
    fn write_export_stream_sync_writes_chunks() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("out.bin");
        let (tx, rx) = mpsc::channel(2);
        tx.blocking_send(b"ab".to_vec()).unwrap();
        tx.blocking_send(b"cd".to_vec()).unwrap();
        drop(tx);

        write_export_stream_sync(path.clone(), rx).unwrap();

        assert_eq!(std::fs::read(path).unwrap(), b"abcd");
    }
}
