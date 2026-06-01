// Native vector PDF export for the desktop shell (macOS only).
//
// In the browser, "Export to PDF" builds a self-contained themed HTML
// document and calls `window.print()`, which hands off to the browser's
// print-to-PDF pipeline. `window.print()` is a no-op inside Tauri's
// WKWebView, so the same path silently does nothing on desktop. WKWebView
// exposes `createPDF(configuration:completionHandler:)`, which renders the
// page to a real vector PDF; this module drives that API from an offscreen
// webview so chan-desktop produces the same themed PDF the browser would.
//
// The frontend gates the call: only macOS desktop reaches here. Linux /
// Windows hide the button entirely (no native equivalent wired), so this
// module is compiled in only under `cfg(target_os = "macos")` and the
// command is registered only on macOS.

#![cfg(target_os = "macos")]

use std::sync::mpsc;
use std::time::{Duration, Instant};

use block2::RcBlock;
use objc2::{MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{NSBackingStoreType, NSWindow, NSWindowStyleMask};
use objc2_core_foundation::{kCFRunLoopDefaultMode, CFRunLoop, CFRunLoopRunResult};
use objc2_foundation::{NSData, NSError, NSPoint, NSRect, NSSize, NSString};
use objc2_web_kit::{WKPDFConfiguration, WKWebView, WKWebViewConfiguration};
use tauri::AppHandle;

/// Upper bound on how long the offscreen webview may take to finish
/// loading the print HTML before we give up. Notes-scale documents render
/// near-instantly; this is a safety valve against a wedged navigation, not
/// a normal-path delay.
const LOAD_TIMEOUT: Duration = Duration::from_secs(10);

/// Upper bound on the `createPDF` completion callback. Same rationale:
/// the capture is fast, this only guards against a callback that never
/// fires.
const CAPTURE_TIMEOUT: Duration = Duration::from_secs(15);

/// How long to wait for the worker thread to hear back from the main
/// thread. The main-thread work itself is bounded by the two timeouts
/// above, so this only needs headroom over their sum.
const DISPATCH_TIMEOUT: Duration = Duration::from_secs(30);

/// One slice of the main run loop, in seconds. We pump the loop in short
/// slices so WebKit's internal sources (navigation, the PDF completion
/// block) can progress while we wait synchronously on the main thread.
const RUNLOOP_SLICE: f64 = 0.02;

/// Standard letter width in points (8.5in * 72) used as the offscreen page
/// width when the SPA does not narrow the page. The page-width ratio the
/// editor uses to shrink content maps to a fraction of this.
const LETTER_WIDTH_PT: f64 = 612.0;

/// A generous offscreen page height. WKWebView lays out the full document
/// height regardless of the window height, and `createPDF` paginates the
/// captured content, so this only needs to be a plausible viewport, not
/// the true document height.
const OFFSCREEN_HEIGHT_PT: f64 = 1056.0;

/// Render `html` to a vector PDF via WKWebView and return the bytes.
///
/// `page_width_px` is the CSS pixel width the SPA used when it built the
/// print document (the editor's page-width control). We size the offscreen
/// webview to that width so the captured layout matches what the user sees
/// in the editor; a zero / non-positive value falls back to letter width.
///
/// All WebKit work runs on the main thread (WKWebView is main-thread-only).
/// Tauri command threads run off the main thread, so we hop onto it via
/// `AppHandle::run_on_main_thread` and bridge the result back over a
/// channel.
#[tauri::command]
pub fn export_pdf_macos(
    app: AppHandle,
    html: String,
    page_width_px: f64,
) -> Result<Vec<u8>, String> {
    let (tx, rx) = mpsc::channel::<Result<Vec<u8>, String>>();
    app.run_on_main_thread(move || {
        // `run_on_main_thread` guarantees this closure runs on the main
        // thread, so the marker is sound. Without a valid marker the
        // WKWebView / NSWindow constructors below would be unsound.
        let mtm = MainThreadMarker::new()
            .expect("run_on_main_thread closure must execute on the main thread");
        let result = render_pdf_on_main(mtm, &html, page_width_px);
        // The receiver may have already timed out and dropped; a failed
        // send just means nobody is listening, which is fine.
        let _ = tx.send(result);
    })
    .map_err(|e| format!("scheduling PDF export on the main thread failed: {e}"))?;

    match rx.recv_timeout(DISPATCH_TIMEOUT) {
        Ok(result) => result,
        Err(mpsc::RecvTimeoutError::Timeout) => {
            Err("PDF export timed out waiting for the main thread".to_string())
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            Err("PDF export main-thread task ended without a result".to_string())
        }
    }
}

/// Main-thread body: build an offscreen webview, load the HTML, wait for
/// the load to settle, capture the PDF, and return the bytes.
fn render_pdf_on_main(
    mtm: MainThreadMarker,
    html: &str,
    page_width_px: f64,
) -> Result<Vec<u8>, String> {
    let page_width = if page_width_px.is_finite() && page_width_px > 0.0 {
        page_width_px
    } else {
        LETTER_WIDTH_PT
    };
    let frame = NSRect::new(
        NSPoint::new(0.0, 0.0),
        NSSize::new(page_width, OFFSCREEN_HEIGHT_PT),
    );

    // SAFETY: every call here is a main-thread-only WebKit / AppKit API
    // and we hold a valid `MainThreadMarker`. The objects live until the
    // end of this function (and the run-loop pumping below keeps their
    // pending work serviced), so no callback outlives its captured state.
    unsafe {
        let config = WKWebViewConfiguration::new(mtm);
        let webview = WKWebView::initWithFrame_configuration(WKWebView::alloc(mtm), frame, &config);

        // An unattached WKWebView may skip layout and render a blank page,
        // because off-window views are not guaranteed to lay out. Parent it
        // in an offscreen, never-shown NSWindow sized to the page so the
        // content lays out before capture. The window is borderless and
        // never ordered front, so nothing flashes on screen.
        let window = NSWindow::initWithContentRect_styleMask_backing_defer(
            NSWindow::alloc(mtm),
            frame,
            NSWindowStyleMask::Borderless,
            NSBackingStoreType::Buffered,
            false,
        );
        window.setContentView(Some(&webview));

        let nav = NSString::from_str(html);
        let _ = webview.loadHTMLString_baseURL(&nav, None);

        if !wait_until_loaded(&webview) {
            return Err("PDF export timed out loading the document".to_string());
        }

        capture_pdf(mtm, &webview, page_width)
    }
}

/// Pump the main run loop in short slices until the webview reports it has
/// finished loading, or `LOAD_TIMEOUT` elapses. Returns whether the load
/// settled in time.
///
/// We poll `isLoading` rather than installing a `WKNavigationDelegate`:
/// `setNavigationDelegate` holds the delegate weakly, so a delegate would
/// need explicit lifetime management for a callback that fires once. For a
/// single bounded wait, pumping the loop and polling the documented
/// `isLoading` flag is simpler and equally correct, and the run-loop slices
/// are what let the navigation actually progress.
unsafe fn wait_until_loaded(webview: &WKWebView) -> bool {
    let deadline = Instant::now() + LOAD_TIMEOUT;
    // `loadHTMLString` schedules the navigation but `isLoading` may not flip
    // to true until the run loop turns, so pump at least once before the
    // first check.
    loop {
        pump_run_loop_once();
        if !webview.isLoading() {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
    }
}

/// Capture the currently-loaded page to a PDF sized to `page_width` points.
/// Bridges the async completion handler to this thread over a channel and
/// pumps the run loop until it fires or `CAPTURE_TIMEOUT` elapses.
unsafe fn capture_pdf(
    mtm: MainThreadMarker,
    webview: &WKWebView,
    page_width: f64,
) -> Result<Vec<u8>, String> {
    let pdf_config = WKPDFConfiguration::new(mtm);
    // The PDF rect is the capture region in the page's coordinate space.
    // Width matches the offscreen layout width; the tall height lets WebKit
    // capture the full document, which it then paginates.
    pdf_config.setRect(NSRect::new(
        NSPoint::new(0.0, 0.0),
        NSSize::new(page_width, OFFSCREEN_HEIGHT_PT),
    ));

    let (tx, rx) = mpsc::channel::<Result<Vec<u8>, String>>();
    // The completion handler is an `Fn` block (it may, in principle, be
    // invoked more than once by the framework), so it captures a cloneable
    // `Sender` rather than moving an owned value. A second send after the
    // receiver is gone is harmless.
    let handler = RcBlock::new(move |data: *mut NSData, error: *mut NSError| {
        let result = if !data.is_null() {
            // SAFETY: WebKit hands us a +0 autoreleased NSData that is valid
            // for the duration of the callback; copying it out immediately
            // is sound.
            let bytes = unsafe { (*data).to_vec() };
            Ok(bytes)
        } else if !error.is_null() {
            let message = unsafe { (*error).localizedDescription() };
            Err(format!("WKWebView createPDF failed: {message}"))
        } else {
            Err("WKWebView createPDF returned no data".to_string())
        };
        let _ = tx.send(result);
    });

    webview.createPDFWithConfiguration_completionHandler(Some(&pdf_config), &handler);

    let deadline = Instant::now() + CAPTURE_TIMEOUT;
    loop {
        match rx.try_recv() {
            Ok(result) => return result,
            Err(mpsc::TryRecvError::Disconnected) => {
                return Err("PDF export capture channel closed unexpectedly".to_string());
            }
            Err(mpsc::TryRecvError::Empty) => {}
        }
        if Instant::now() >= deadline {
            return Err("PDF export timed out capturing the document".to_string());
        }
        pump_run_loop_once();
    }
}

/// Run the current (main) run loop for one short slice so WebKit's pending
/// sources can advance. `return_after_source_handled = false` lets the loop
/// process whatever is ready within the slice rather than bailing after the
/// first source.
fn pump_run_loop_once() {
    // SAFETY: reading the `kCFRunLoopDefaultMode` extern static is sound;
    // it is a process-lifetime constant CFString set up by CoreFoundation.
    let mode = unsafe { kCFRunLoopDefaultMode };
    let _ret: CFRunLoopRunResult = CFRunLoop::run_in_mode(mode, RUNLOOP_SLICE, false);
}
