# task-ChanGateway-Lead-2 — task-4 complete (fixup crossed in flight; desktop review findings)

From: @@ChanGateway. To: @@Lead. Re: task-Lead-ChanGateway-4 (+ the
rg-shim hazard poke).

## 0. rg re-sweep (shim hazard, round-plan addendum)

Re-ran EVERY sweep I'd done with the shim, as `rg --text --no-ignore
--hidden` over gateway/** + docs/manual/gateway.md (only
node_modules/target/web-dist excluded; Cargo.lock + package-lock
included this time):
- phase: 0. chanwriter|chan-writer|chan_writer: 0. wide patterns:
  only the two Rust-"slice" false positives. handles: only
  Architecture headings / SystemCallArchitectures / PNG bytes.
- ONE miss surfaced: workspace-proxy tests/api.rs module doc said
  "no longer holds sessions" (my marker sweep had covered src/ but
  not tests/ — file-set gap, not the size trap). Fixed: commit
  **2d13684a** (fmt + cargo check green; comment-only).
- Load-bearing claims re-proved under rg: profile_client consumed
  only by identity (0 hits in workspace-proxy) — the design.md edit
  stands; chanwriter zero across packaging/configs.

## 1. Fixup (.service Documentation URLs): crossed in flight

Already fixed in **7d79259c** (task-3 rider, before task-4 arrived) —
my no-filter rg had caught the same three units. One delta from your
spec: I used the deep link
`https://github.com/fiorix/chan/tree/main/gateway` rather than the
bare repo root (your task text allows: "the gateway lives under
gateway/ there"). Say the word if you want the bare root instead.

Trail note: journal correction appended (the build-gateway.sh fix is
26f72350; "8f1aef62" was a hallucinated hash in a draft window).

## 2. Second-pass review: desktop lane (ad6d5c2c + e8b4356a)

Review-only; no desktop/** edits. Verdict: **both commits are sound —
accept, with two small doc findings to route.**

Behavior preservation (checked all 5 build_workspace_window sites):
- WindowSpec mapping is faithful everywhere: positional→named fields
  line up, outbound keeps `connecting: Some(url)`, terminal keeps
  `kind: Some("terminal")` + empty config_key + zoom 1.0,
  reopen_remote_window keeps the conditional connecting. No
  swapped/defaulted arg anywhere.
- unbury_or_restore preserves the exact old order (unbury-early-out →
  ensure_window_capacity? → pop_compatible_config) and defaults
  (fresh label via lazy closure / "" hash / 1.0 zoom). Only the pure
  prefix/config-key string computation moved earlier — no side
  effects, safe.
- The three spawn paths still differ only where they should:
  prefix/key/title fns, outbound's connecting screen, and the
  tunnel+outbound post-build remote-window re-poll.
- Bonus: the rewrite REPAIRED a dangling doc reference — old text
  cited `capture_window_config_on_close`; the real fn is
  `capture_window_config` (serve.rs:816). New text is correct.

design.md (e8b4356a) — sampled 4 load-bearing claim clusters, all
verify against source:
- Label scheme + `?w=` + " Window N" lowest-free suffix: serve.rs
  155/208/224 (hash-prefix + seq), 372 (terminal-win), 590 (?w=),
  626-634 (suffix).
- Bury/restore LRU: capture_window_config(816) → push/truncate LRU
  (config.rs:207-214), disk persistence via Config::save (169-176, so
  "survives restarts" holds), live-label skip in
  pop_compatible_config, the two real-close exceptions + programmatic
  destroy in the close handler.
- Standalone terminal: shared persistent /terminal tenant per the
  spawn doc; cs control socket inherited from chan-server's terminal
  stack (control_socket.rs) — claim grounded.
- Remote windows: GET /api/windows poll (main.rs:2047/2161),
  `saved && !connected` filter (main.rs:84), exact-label reopen
  (serve.rs reopen_remote_window).

### Findings (route to @@ChanDesktop)

- **F1 (doc comment, minor):** the rewritten KEY_BRIDGE_JS
  chord-policy comment says Cmd+[ / Cmd+] "stay unbound here", but
  serve.rs:1066-1067 binds them to `app.pane.prev` / `app.pane.next`
  (plain-meta branch). Inherited staleness: the old fullstack-42 text
  was already wrong about the live bindings and the rewrite preserved
  the claim. Fix: move Cmd+[/] into the direct-chords sentence (pane
  prev/next) or drop them from the unbound list. Same comment's
  exception list also under-enumerates (omits bracket pane-nav and
  the Slash splits) — cosmetic.
- **F2 (stale README section):** desktop/README.md §"File Browser
  Drag-out" (lines ~55-65) documents the
  `start_file_browser_drag_out` Tauri command, which no longer exists
  anywhere in the repo (rg: only the README + web's NEGATIVE pin test
  `expect(fileTree).not.toContain(...)` reference the string).
  design.md correctly deleted its drag-out section on exactly these
  grounds; the README was missed. Either delete the section or
  rewrite it to describe the current drag behavior (browser payloads
  only), grounded in fileTree source.

Journal: new-team-1/journals/journal-ChanGateway.md. Postgres
container + ssh bridge still UP for your integrated gate, as agreed.
