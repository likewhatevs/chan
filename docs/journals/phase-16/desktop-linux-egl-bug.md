# chan-desktop backlog: Linux AppImage white-window / EGL_BAD_PARAMETER

Status: BACKLOG (filed by @@Host 2026-06-01, deferred out of round-1 scope).
Owner-to-be: chan-desktop lane (@@LaneC area / desktect domain).

## Symptom

chan-desktop windows render PLAIN WHITE with no content. Reproduced by
@@Host on a CachyOS (Arch-based) system running the released AppImage
`Chan_0.23.0_amd64.AppImage`. Console:

```
2026-06-01T23:15:56 INFO chan_desktop: installed cs wrapper into ~/.local/bin
Could not create default EGL display: EGL_BAD_PARAMETER. Aborting...
Could not create default EGL display: EGL_BAD_PARAMETER. Aborting...
2026-06-01T23:15:57 WARN chan_workspace::index::facade: Embedding model not
   downloaded; falling back to BM25-only keyword search. ...
Could not create default EGL display: EGL_BAD_PARAMETER. Aborting...
^C
```

The repeated EGL line = one per webview/WebProcess spawn (each white window).

## What is NOT the bug

- The `cs` wrapper install line means the chan-desktop binary itself booted
  fine (P1-style wrapper install works on Linux).
- The `Embedding model not downloaded -> BM25-only` WARN is expected/benign
  (no embed model bundled in that build), unrelated to the render failure.

## Likely cause

This is the WebKitGTK WebProcess failing to initialize its GPU/EGL render
path, so the web content never paints (white window). WebKitGTK 2.42+ ships
a DMABUF-based renderer that fails with exactly `EGL_BAD_PARAMETER` on a
range of Linux GPU/driver/compositor combinations (notably Nvidia
proprietary, some Mesa setups, and certain Wayland sessions). Tauri apps
inherit this because the webview IS WebKitGTK on Linux.

## Candidate fixes (verify on a real Linux GPU desktop)

1. `WEBKIT_DISABLE_DMABUF_RENDERER=1` -> most common, targeted fix for this
   exact error; disables only the DMABUF path, keeps accelerated compositing
   where possible.
2. `WEBKIT_DISABLE_COMPOSITING_MODE=1` -> heavier fallback (forces non-
   accelerated compositing); use only if (1) is insufficient.
3. `LIBGL_ALWAYS_SOFTWARE=1` -> last-resort full software GL.

Shipping shape (design call, not just a user env tweak):
- Do NOT blanket-disable DMABUF for all Linux users (it costs perf on working
  setups). Prefer: set `WEBKIT_DISABLE_DMABUF_RENDERER=1` from the launcher
  only when a probe/heuristic detects the failure (or on first-run EGL error,
  relaunch the webview with the var set), or document it as an escape hatch.
- The launcher already sets process env before spawning the webview, so a
  conditional default is feasible in `desktop/src-tauri`.

## Open questions to capture from @@Host when this is scheduled

- GPU vendor/driver (Nvidia proprietary vs Mesa/AMD/Intel)?
- Wayland or X11 session?
- Does launching with `WEBKIT_DISABLE_DMABUF_RENDERER=1 ./Chan_*.AppImage`
  fix it? (the single fastest signal; confirms the diagnosis)

## Verification constraint

This needs a REAL Linux desktop with a GPU + display/compositor to
reproduce. Headless containers (lima-vm + sdme, aarch64) cannot repro an
EGL/display init failure, and the AppImage is amd64. So fix-verification
requires @@Host's CachyOS box (or an equivalent Linux GPU desktop). A code
fix can be authored blind, but acceptance is gated on @@Host re-testing.
