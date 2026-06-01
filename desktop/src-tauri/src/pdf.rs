// Native paginating PDF export for the desktop shell (macOS only).
//
// In the browser, "Export to PDF" builds a self-contained themed HTML
// document and calls `window.print()`, which hands off to the browser's
// print-to-PDF pipeline. That pipeline honors `@page`, CSS auto-
// pagination, and explicit `.chan-page-break` rules, so a long note spans
// multiple pages and the editor's `@pagebreak` feature lands a real page
// break. `window.print()` is a no-op inside Tauri's WKWebView, so the same
// path silently does nothing on desktop.
//
// WKWebView exposes two capture routes. `createPDF` is a SCREEN capture:
// it rasterizes the laid-out page and does NOT run the print pipeline, so
// it ignores `@page`, auto-pagination, and page breaks. A long note clips
// to roughly one page. `printOperationWithPrintInfo:` instead returns an
// `NSPrintOperation` that drives the real macOS print pipeline; run
// silently to a PDF file it produces the same paginated output the browser
// would, page breaks included. This module uses the print route so the
// native export matches the browser export.
//
// The frontend gates the call: only macOS desktop reaches here. Linux /
// Windows hide the button entirely (no native equivalent wired), so this
// module is compiled in only under `cfg(target_os = "macos")` and the
// command is registered only on macOS.

#![cfg(target_os = "macos")]

use std::sync::mpsc;
use std::time::{Duration, Instant};

use objc2::runtime::{AnyObject, ProtocolObject};
use objc2::{MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSBackingStoreType, NSPrintInfo, NSPrintJobSavingURL, NSPrintSaveJob, NSWindow,
    NSWindowStyleMask,
};
use objc2_core_foundation::{kCFRunLoopDefaultMode, CFRunLoop, CFRunLoopRunResult};
use objc2_foundation::{NSCopying, NSPoint, NSRect, NSSize, NSString, NSURL};
use objc2_web_kit::{WKWebView, WKWebViewConfiguration};
use tauri::AppHandle;

/// Upper bound on how long the offscreen webview may take to finish
/// loading the print HTML before we give up. Notes-scale documents render
/// near-instantly; this is a safety valve against a wedged navigation, not
/// a normal-path delay.
const LOAD_TIMEOUT: Duration = Duration::from_secs(10);

/// How long to wait for the worker thread to hear back from the main
/// thread. The main-thread work itself is bounded by the load timeout
/// above plus a synchronous print run, so this only needs headroom over
/// their sum.
const DISPATCH_TIMEOUT: Duration = Duration::from_secs(30);

/// One slice of the main run loop, in seconds. We pump the loop in short
/// slices so WebKit's internal navigation sources can progress while we
/// wait synchronously on the main thread for the load to settle.
const RUNLOOP_SLICE: f64 = 0.02;

/// Standard letter width in points (8.5in * 72) used as the offscreen page
/// width when the SPA does not narrow the page. The page-width ratio the
/// editor uses to shrink content maps to a fraction of this.
const LETTER_WIDTH_PT: f64 = 612.0;

/// Standard letter height in points (11in * 72). Used as the offscreen
/// window height and the print paper height.
const LETTER_HEIGHT_PT: f64 = 792.0;

/// Render `html` to a paginated PDF via WKWebView's print pipeline and
/// return the bytes.
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
        // WKWebView / NSWindow / NSPrintOperation constructors below would
        // be unsound.
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
/// the load to settle, run the print pipeline to a PDF, and return the
/// bytes.
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
        NSSize::new(page_width, LETTER_HEIGHT_PT),
    );

    // SAFETY: every call here is a main-thread-only WebKit / AppKit API
    // and we hold a valid `MainThreadMarker`. The objects live until the
    // end of this function (and the run-loop pumping below keeps the
    // navigation serviced), so nothing outlives its captured state.
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

        capture_pdf(&webview)
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

/// Run the macOS print pipeline on the loaded page to a temp PDF file and
/// return its bytes.
///
/// Why the print pipeline (not `createPDF`): `printOperationWithPrintInfo:`
/// runs WebKit's real print path, which honors `@page`, CSS auto-
/// pagination, and `.chan-page-break`. `createPDF` is a screen capture that
/// ignores all three, so a long note would clip to one page.
///
/// Why silent: an `NSPrintOperation` defaults to showing the print panel
/// and a progress panel. We disable both on the operation, and set the
/// print info's job disposition to `NSPrintSaveJob` with a destination URL
/// in its attributes dictionary, so the operation writes straight to that
/// file with no UI.
///
/// Why a temp file (not in-memory data): the silent save-to-PDF path keys
/// off `NSPrintJobSavingURL`, a file URL. There is no public main-thread
/// API to save the print job's PDF directly into an `NSData`, so we round-
/// trip through a temp file and read it back.
unsafe fn capture_pdf(webview: &WKWebView) -> Result<Vec<u8>, String> {
    let dest = unique_temp_pdf_path();
    let dest_str = dest.to_string_lossy().into_owned();

    let info = NSPrintInfo::new();

    // US Letter paper. The CSS print document targets a letter-equivalent
    // page width, so the paper size matches the layout the SPA built.
    info.setPaperSize(NSSize::new(LETTER_WIDTH_PT, LETTER_HEIGHT_PT));

    // Zero the NSPrintInfo margins. The print HTML already supplies page
    // margins through `@page { margin: 0.65in }`, which the print pipeline
    // honors. Leaving the default NSPrintInfo margins in place would stack
    // on top of the CSS margin and double-inset the content, so we drive
    // the margins purely from CSS and keep the paper edge-to-edge here.
    info.setLeftMargin(0.0);
    info.setRightMargin(0.0);
    info.setTopMargin(0.0);
    info.setBottomMargin(0.0);

    // Save-to-PDF: write the job to a file instead of spooling to a
    // printer or showing the save panel.
    info.setJobDisposition(NSPrintSaveJob);

    // The destination file URL goes in the print info's attributes
    // dictionary under `NSPrintJobSavingURL`. This is the documented key
    // for a headless save-to-PDF job.
    let url = NSURL::fileURLWithPath(&NSString::from_str(&dest_str));
    // SAFETY: `dictionary()` returns the print info's mutable attributes
    // dictionary; inserting an NSURL value under the AppKit-defined
    // `NSPrintJobSavingURL` key is the documented, type-correct use. The
    // key is an NSString (which conforms to NSCopying); the value is an
    // NSURL erased to AnyObject as the dictionary's object type expects.
    let attrs = info.dictionary();
    let url_obj: &AnyObject = url.as_ref();
    let key: &ProtocolObject<dyn NSCopying> = ProtocolObject::from_ref(NSPrintJobSavingURL);
    attrs.setObject_forKey(url_obj, key);

    let print_op = webview.printOperationWithPrintInfo(&info);

    // Headless: never raise the print panel or the progress panel.
    print_op.setShowsPrintPanel(false);
    print_op.setShowsProgressPanel(false);

    // `runOperation` is synchronous: it returns only after the job has been
    // fully written (or failed), so unlike `createPDF` there is no async
    // completion handler to await and no run-loop pumping needed here. We
    // hold the main thread for the duration, which is fine for a notes-
    // scale document.
    let ok = print_op.runOperation();
    if !ok {
        let _ = std::fs::remove_file(&dest);
        return Err("WKWebView print operation failed".to_string());
    }

    let bytes = std::fs::read(&dest)
        .map_err(|e| format!("reading the exported PDF from {dest_str} failed: {e}"))?;
    // Best-effort cleanup; a leftover temp file is harmless but we remove
    // it on the success path so the temp dir does not accumulate exports.
    let _ = std::fs::remove_file(&dest);

    if bytes.is_empty() {
        return Err("the print operation produced an empty PDF".to_string());
    }
    Ok(bytes)
}

/// Build a unique temp file path for one export. We never reuse a fixed
/// name so two near-simultaneous exports cannot clobber each other's
/// output before the read-back.
fn unique_temp_pdf_path() -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("chan-export-{pid}-{nanos}-{n}.pdf"))
}

/// Run the current (main) run loop for one short slice so WebKit's pending
/// navigation sources can advance. `return_after_source_handled = false`
/// lets the loop process whatever is ready within the slice rather than
/// bailing after the first source.
fn pump_run_loop_once() {
    // SAFETY: reading the `kCFRunLoopDefaultMode` extern static is sound;
    // it is a process-lifetime constant CFString set up by CoreFoundation.
    let mode = unsafe { kCFRunLoopDefaultMode };
    let _ret: CFRunLoopRunResult = CFRunLoop::run_in_mode(mode, RUNLOOP_SLICE, false);
}
