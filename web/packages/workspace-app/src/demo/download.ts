// Every file download in the frontend-only demo returns this page instead of
// the real bytes: the demo has no backend to serve file contents, so a
// download hands back chan-demo.md, a plain-text About page. Metadata export
// is separate: it downloads the real in-memory archive.
//
// Edit the content in ./chan-demo.md (imported below at build time). Its QR is
// a UTF8 half-block code (no ANSI escapes) inside a fenced code block, so it
// renders as a scannable, SQUARE QR both under `cat` and in a Markdown editor
// (the fence keeps it monospace; a plain ASCII grid comes out 2:1 and will not
// scan). On a light background it reads normally; a dark background inverts it,
// which most scanners still accept. To point the QR elsewhere, replace the
// fenced block with the output of (this QR points at the donate link):
//   qrencode -t UTF8 "https://buymeacoffee.com/fiorix"
import chanDemoMd from "./chan-demo.md?raw";

export const CHAN_DEMO_MD = chanDemoMd;

/// Trigger a browser download of the canned About page as chan-demo.md. Used
/// by the demo download seam for every file/directory download.
export function demoDownload(): void {
  const blob = new Blob([CHAN_DEMO_MD], { type: "text/markdown" });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = "chan-demo.md";
  link.rel = "noopener";
  link.style.display = "none";
  document.body.appendChild(link);
  link.click();
  link.remove();
  // Let the click start before releasing the object URL.
  setTimeout(() => URL.revokeObjectURL(url), 0);
}
