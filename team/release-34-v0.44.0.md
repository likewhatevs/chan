# Phase 34 - launcher reflects reality (workspaces + devservers) + `chan open`/`close` + transfer bubble

Status: ready for release as `v0.44.0`. All build work landed + committed; the workspace/server/CLI
behaviors are covered by unit + HTTP-layer tests and each lane's own-gate, plus an Lead headless e2e
(local devserver + `chan open`/`close`/`--remove`) that caught and confirmed-fixed a real bug. The
full-tree integrated gate runs at round close; version pins bump at tag. The WKWebView-native bits — the
transfer bubble's live download/cancel, the clickable Open-windows dot, the close-guard prompt, the §4
Open/Turn-on window mint, the file-browser content-peek — are desktop-only and deferred to Alex's
end-to-end smoke (he will run the `cs download` 1-2-files-then-cancel pass). Span: 2026-06-22.
Tags: #web-launcher #chan-library #devserver #devserver-registry #chan-open-close #cs-upload-download
#transfer-bubble #window-close-guard #cs-open-plaintext #rich-prompt #4-agent-team

Phase 33 shipped the web-launcher unified across all three surfaces, but the launcher's registry CRUD
still ran against an in-memory mock — so the desktop loopback showed a hardcoded fake set
(`notes`/`Journal`/`prod`) instead of the user's real workspaces. Phase 34 closes that: the launcher
becomes a true view of the real library on the desktop, finishes the `chan serve`/`unserve` →
`chan open`/`close` verb migration, reshapes the devserver form to a single URL, and — folded in by
Alex during the round — makes `cs upload`/`cs download` a visible, cancellable, reload-surviving
surface tied to its window.

## What shipped

**Launcher reflects reality (§1/§2/§3/§4):**

- **§1 — registry flipped to live.** `web-launcher/src/api/backend.ts` collapsed to the live HTTP client
  (`liveApi`) for every method, dropping the `REGISTRY=mockApi`/`WINDOW_FEED=liveApi` split. The desktop
  loopback lists + mutates the real `~/.chan` workspaces and configured devservers. `mock.ts` stays a test
  double, pinned in the registry-driven vitest via `vi.mock("../api/backend")`.
- **§2 — devserver registry bridge (the shared seam).** A new `chan-library` `DevserverRegistry` trait +
  URL-shaped `DevserverEntry`/`DevserverInput` (token write-only), held by `WorkspaceHost` as
  `Option<Arc<dyn DevserverRegistry>>` (mirror of `workspace_overlay`); chan-desktop implements it over its
  config (`DevserverConfigRegistry`, persisted, installed next to the workspace overlay). The trait lives in
  chan-library (not chan-server, despite the plan's file note) because `WorkspaceHost` holds the handle and
  the dep flows chan-server → chan-library. Routes `GET/POST /api/library/devservers` + `PUT/DELETE /:id`
  through `host.devserver_registry()`, same loopback gate as workspaces (read-only/headless ⇒ empty list +
  403/404 mutation).
- **§3 — one devserver URL field.** `Devserver { host, port }` → `url: String` (scheme kept; port defaulted
  from the scheme at dial — the forward hook for the devserver-proxy/OAuth dial, marked as a follow-up, not
  built). Frontend dialog collapsed Host/Port to a single validated `Devserver URL`.
- **§4 — per-row Open / Turn on.** `WorkspaceList.svelte`: `ws.on` → **Open** (`openWorkspaceWindow` mints a
  new window via `createWindow`), `!ws.on` → **Turn on**; read-only surfaces keep the static pill.

**`chan open`/`close` (§5):**

- `chan serve`/`unserve` removed; `Command::Open`/`Command::Close` added. `chan open {path}` = workspace add
  (git-parent check) + serve/mount with the existing desktop/devserver handoff polymorphism; `chan open
  {url}` registers a devserver via the `OpenDevserver` handoff (register-only — the launcher's Connect drives
  the dial; no-desktop → clear error). `chan close {path}` is best-effort unserve (idempotent); `--remove`
  also forgets the workspace. `ControlRequest::Unserve` → `Close { remove }`.

**`cs upload`/`cs download` visible + cancellable + window-scoped (folded in by Alex):**

- A per-window `transfers` model + `TransferBubble.svelte` (progress + Cancel/Retry/Dismiss), opened from a
  status-bar launcher; the bubble is the single download surface (the inspector download bar was retired).
  State persists in sessionStorage (`chan.transfers:${sessionWindowId()}`) so it survives a window reload —
  an in-flight transfer restores as **interrupted** (download offers Retry; an upload's `File` can't survive
  a reload, so Dismiss-only), never a frozen bar.
- **Window close-guard:** a new chan-library `WindowTransfers` (per-window in-flight count, reported by the
  SPA over `/ws` as `{type:"transfers","active":n}`, RAII-cleared on socket disconnect — so reload-as-
  interrupted falls out for free) + `WorkspaceHost::tenant_has_active_transfer`. The desktop's
  `CloseRequested` handler mirrors the live-shells guard: closing a window mid-transfer prompts **Keep open**
  vs **Cancel transfer & close**.

**Other folded-in fixes:**

- **`cs open` + file browser open any plaintext file** via the existing `chan-workspace` content classifier
  (`looks_like_text`/`read_text_with_stat`), not the extension; `cs open` creates a nonexistent path as
  plaintext.
- **Rich-prompt ArrowUp recall** no longer sticks read-only (the un-grey is folded into the dispatch +
  focus deferred to a microtask, matching the delivered path).
- **`chan close --remove` unregisters from a running devserver** — root cause was the devserver's own stale
  in-memory `Library` + the `persist_state` map re-growing `workspaces.json`; the fix routes `--remove`
  through `host.remove_workspace_for_root` and makes `persist_state` reconcile against the host (also closing
  the same latent divergence in the launcher DELETE path). Plus a plain `chan close` now persists off-state.
- **Window-bury notice** simplified (no em dash).

## Team / process

4-agent round (Lead + Launcher + Desktop + CLI), seam-first. Lead pinned two mini-seams — the
`DevserverRegistry` (round open) and the `WindowTransfers` close-guard signal (mid-round, when Alex folded
in the transfer feature) — and the workers built their surfaces against the pinned contracts. Dispatch +
journals under `dev/v0.44.0/team/` (gitignored live bus).

## Retrospective

### Done

§1-§5 + every Alex fold-in (rich-prompt, notice copy, clickable dot, `cs open` plaintext + file-browser
peek, the full transfer feature, the inspector-bar retirement, the `chan close --remove` devserver fix). All
lanes own-gate-green; Lead headless e2e verified the devserver + `chan open`/`close`/`--remove` path and
caught the `--remove` bug before ship.

### Pending (deferred to Alex / next phase)

- **Alex's live desktop smoke** — the WKWebView bits (transfer bubble live download/cancel, the dot,
  close-guard prompt, §4 Open, file-browser peek). Ships gated-green + live-unverified per the pre-release
  norm.
- See "Next-phase follow-ups" below.

### Highlights

- **Seam-first held up twice.** Both mini-seams (`DevserverRegistry`, `WindowTransfers`) let three lanes
  build in parallel against stable contracts with no wire churn — the second one was pinned mid-round under
  a feature Alex folded in late, and still landed clean.
- **Worker judgment.** Agents deviated correctly from Lead's suggested task shapes when those were wrong
  (e.g. CLI keying the close-guard on the `?w=` session id not the label; resolving the prefix from window
  records because `config_key` is empty for watcher windows; choosing the 8 KiB sniff over a full read).
- **The e2e earned its keep.** Alex's request for a pre-ship devserver smoke caught a real
  `chan close --remove` bug (lingering launcher entry + restart resurrection) that every per-lane unit gate
  had passed. CLI's deeper root-cause fix (persist_state reconciliation) also closed a latent bug in the
  launcher DELETE path.

### Lowlights

- **Poke-crossing churn.** Many stale/duplicate completion pokes crossed in flight (workers re-confirming
  done work, "blocked" reports that were already unblocked). Cost coordination cycles; mitigated by always
  verifying against HEAD before acting, never on a sha named in a poke.
- **The transfer feature grew a lot mid-round** (progress → cancel → reload-survival → window close-guard →
  inspector retirement → a found CLI bug), turning a "small fold-in" into the round's largest piece and a
  3-surface sub-project. It landed, but it stretched the close.
- **Journal hygiene.** `journal-Lead.md` accumulated duplicate "at close" lines from successive edits +
  bash appends (the file kept shifting under edits) — consolidated at close.

### Honest feedback

- **To the workers:** consistently strong — own-gates with `-D warnings` parity, atomic pathspec commits, and
  the good sense to flag gnarly findings (the flaky-trigger gotcha, the config_key emptiness) rather than
  paper over them. Keep it.
- **To Lead (me):** I mis-framed the transfer feature as workspace-level and conflated "library-level
  transfers" with the separate chan-library-metadata item — Alex had to correct me. I should have scoped
  the transfer feature's full shape (which surfaces, which level) up front instead of discovering it
  reactively across pokes. I also under-specified the close-guard in the first transfer task (Launcher's
  discovery didn't include it; I had to re-require it).
- **To Alex:** folding many asks in mid-round (each reasonable on its own) compounded into significant
  late scope on a round that was otherwise near-closed; it worked only because the lanes stayed disciplined
  and the seams absorbed it. The reorder (do docs+gate, smoke later) was the right call to stop the growth.

## Next-phase follow-ups

1. **Standalone-terminal `cs download`/`cs upload`** (library-level transfers): the terminal tenant has no
   workspace and no per-tenant control socket, so this needs a terminal-tenant control socket + a
   workspace-less transfer handler + a defined transfer target/scope (with sandbox implications). This round
   is workspace-only by Alex's decision.
2. **chan-library metadata.** Move the download/upload of workspace metadata to become chan-library metadata,
   with the functionality living in the chan-library SPA (web-launcher).
3. **Transfer close-guard for connected-devserver windows.** The guard is local-library only today; a
   connected devserver's window transfers would need a remote transfer signal over the feed.
4. **Full upload-surface unification.** Route the second upload flow (`replaceFileAt`) through the bubble and
   retire the upload status-bar text (this round retired the download bar only).
5. **Devserver-proxy / OAuth dial.** §3 kept the URL scheme + defaults the port for the eventual proxied
   `https://{user}.devserver.chan.app` dial; the OAuth branch is marked in `devserver.rs` and not built.
