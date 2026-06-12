# task-ChanDesktop-Lead-3 — file-drop desktop half: DONE (pending Alex smoke)

From: @@ChanDesktop. To: @@Lead. Re: task-Lead-ChanDesktop-3 +
task-Lead-ChanDesktop-6 (both amendments implemented). Contract was
frozen by @@Chan's ack (task-Chan-ChanDesktop-1) before I landed.

## Commit

`79de0e95` — `read_dropped_paths` IPC + ACL + contract pins.

- `dropped_paths.rs`: reads the macOS drag pasteboard
  (`NSFilenamesPboardType` property list — the exact read wry's own
  collect_paths does, so we parse what the native layer would have
  reported). Returns raw absolute paths in pasteboard order; `[]` for
  no-file drags and unconditionally off macOS. Registered on every
  platform so the SPA needs no platform branching.
- AMENDMENT 2: the pasteboard read runs on the MAIN thread
  (`run_on_main_thread` + oneshot back to the async command).
- AMENDMENT 1: `allow-read-dropped-paths` lives in a new
  `capabilities/local-drop.json` scoped to `workspace-*` +
  `terminal-*` ONLY (with the loopback remote.urls block those
  windows need). It is NOT in the workspace-window set and NOT in the
  workspace capability, so tunnel-*/outbound-* never see it. The
  scoping is pinned by `drag_pasteboard_read_is_scoped_to_locally_
  served_windows`, including must-NOT-leak assertions on both broad
  surfaces; registration + main-thread discipline pinned by
  `invoke_handler_registers_read_dropped_paths`.
- `disable_drag_drop_handler` comment in serve.rs now documents the
  wry constraint (why it must stay disabled) and points at the guard
  + IPC split.
- Cargo: objc2-app-kit + NSPasteboard feature, objc2-foundation +
  NSArray (both already direct deps via pdf.rs).

## Gate

fmt --check clean, clippy -p chan-desktop --all-targets 0 warnings,
test -p chan-desktop 79+7 green (77 prior + 2 new contract pins).

## What remains on this bug (not mine / not yet)

1. @@Chan: SPA-global guard (his step 1, ships independently) +
   terminal drop handler wiring (his step 3, against my IPC) +
   vitest incl. the Files-type discriminator coverage @@Lead asked
   for.
2. @@Alex hand-smoke (WKWebView arc is hand-only): after @@Chan's
   halves land — Finder drop on terminal (path prints, escaped,
   trailing space), on editor (embed unchanged), on graph/search
   (inert), on a tunnel/outbound window (inert, no path), tab moves
   unaffected. I'll build the DMG/app for him on request once the
   web half is in.
3. Linux terminal path-print: no persistent drag pasteboard
   equivalent — flagged for round close as agreed.

Ready signal to @@Alex should wait for @@Chan's step 3; I'll poke you
when I see his completion.

---

# Addendum: task-7 review findings fixed (`7da761de`)

- F1: Cmd+[ / Cmd+] moved from the "unbound" examples into the
  direct-chords enumeration (they bind app.pane.prev/next); the
  enumeration also gains the other under-listed direct chords the
  reviewer flagged (pane nav, Cmd+S, Cmd+/ + Cmd+Shift+/ splits).
- F2: README's phantom "File Browser Drag-out" section deleted (the
  command exists nowhere; same grounds as the design.md deletion).
  Grounding the rest of the README while in it: "Makefile builds
  `chan`" claim fixed (it builds the web bundle; desktop ships no
  chan binary), the nonexistent "Forget all workspaces" Settings
  sentence dropped, Linux artifact list now matches release.yml
  (.AppImage/.deb/.rpm).
- Gate after last edit: fmt clean, clippy 0, 79+7 green.

---

# Addendum 2: gate-red HOT fix (`1f27b17d`, task-8)

- Option (a) taken: `#[allow(deprecated)]` on `drag_pasteboard_paths`
  with the justification inline — wry-parity is the design point (wry
  0.55.1 still reads NSFilenamesPboardType in its own drag handler,
  so this parse cannot drift from the native layer's), and option (b)
  would add a file-URL percent-decoding path to diverge on. Migrate
  together with wry if wry ever moves off the type. Contract + pins
  unchanged.
- Re-gated with the REAL flags per your discipline:
  `RUSTFLAGS="-D warnings" cargo clippy -p chan-desktop --all-targets
  -- -D warnings` clean; `RUSTFLAGS="-D warnings" cargo test -p
  chan-desktop` 79+7 green; fmt clean.
- Root cause acknowledged: my own-gate ran bare clippy (deny-less) —
  flags discipline adopted for every future report from this lane.

Ready for your isolated-gate re-run on the new HEAD.
