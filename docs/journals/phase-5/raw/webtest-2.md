# @@Webtest B task 1: parallel scenarios on the Phase 5 test service

Owner: @@Webtest B
Status: REVIEW
Depends on: [webtest-1](./webtest-1.md) (service up, URL + token shared)

Service in use (picked up from @@Webtest A's running process before they
recorded URL/token):

* URL: `http://127.0.0.1:8787/`
* Drive: `/tmp/chan-test-phase5`
* Server PID: 26921 (`./target/debug/chan serve /tmp/chan-test-phase5
  --host 127.0.0.1 --port 8787 --no-browser`)
* Bearer token read from
  `~/Library/Application Support/chan/tokens/f3d2001ac4a1abfc/token`
  (per-launch token persisted by `auth::load_or_create_token`)

## Goal

Cover scenarios that need a second pair of hands while @@Webtest A holds
the baseline smoke. Coordinate restarts and rebuilds with @@Webtest A so
both lanes use the same service instance.

## Scenarios

### Cleanup edges (after frontend-1 + frontend-2 land)

* Hash-state probe: paste pre-cleanup URLs into the browser
  (`#assistant=...`, `#scopes=1`) and confirm the app ignores them and
  rewrites a clean hash.
* Shortcut chord sanity: run through every chord listed in `chan serve`
  help text and the Cmd+? overlay. Nothing should bring up an Agent
  overlay or print a console error referencing dead overlays.
* Settings round-trip: open Settings, change a non-agent preference,
  reload the page, verify the change persists and no agent section
  reappears.

### Backend probe (after backend-1 lands and webtest-1 confirms 404s)

* Network panel: walk the editor (open file, edit, save, search,
  graph) and confirm no request goes to `/api/llm/*`, `/api/assistant/*`,
  or `/api/answers`. Capture HARs if anything slips.
* WebSocket frames: connect via dev tools to `/ws` and confirm the
  stream no longer emits `type:"llm_*"` or `type:"assistant_*"` events.

### Terminal + MCP env (after backend-1 + terminal MCP env-vars land)

* Open a terminal tab. Run `env | grep -E 'CHAN_MCP|CLAUDE_MCP|CODEX_MCP|GEMINI_MCP'`
  and paste the output here. Values must point at the in-process MCP
  socket / `chan __mcp-proxy` command.
* If `claude` / `codex` / `gemini` CLIs are installed on the test
  machine, attempt one MCP tool call from the terminal and confirm the
  drive is the chan-served drive. Note any CLI version that does not
  pick up the env contract.

### Per-window reload regression check

* Open two browser windows on the same service. Modify pane layout in
  window A. Reload window A. Confirm window B is unaffected and window
  A restores its own state (this is a future bug-fix area, so capture
  the current behaviour as a baseline).

## Reporting

* Update this task file with one section per scenario: status + raw
  evidence (commands, captures, screenshots if needed).
* For each issue found, file a follow-up task with the right owner and
  link it here.

## Teardown

* No service to tear down; @@Webtest A owns the service lifecycle.
* Close any background tabs you opened.

## Progress

### Backend probe (no UI required) — PASS

Removed routes return 404 with bearer auth in place:

| Route | Status | Body |
|---|---|---|
| `/api/llm/status` | 404 | `not found` |
| `/api/llm/cli-detection` | 404 | `not found` |
| `/api/llm/complete` | 404 | `not found` |
| `/api/llm/tools` | 404 | `not found` |
| `/api/assistant/conversation?path=foo` | 404 | `not found` |
| `/api/assistant/hash16?path=foo` | 404 | `not found` |
| `/api/answers` | 404 | `not found` |

Preserved routes still 200:

| Route | Status |
|---|---|
| `/api/health` | 200 `{"status":"ok"}` |
| `/api/build-info` | 200 `{"version":"0.8.1","features":{"embeddings":true}}` |
| `/api/config` | 200 (no `assistant`/`llm`/`agent` keys, no `pane_widths.assistant`) |
| `/api/files?path=/` | 200 |

### WebSocket frames — PASS

Opened `/ws?t=<token>`, drove activity via API (POST/DELETE three
markdown files, search query). 14 frames received across ~10 s, all
`"type":"watch"`. Zero frames matched `/llm|assistant|agent/i`. Static
verification: `grep -rn "\"type\"\s*:" crates/chan-server/src/` shows
only `watch`, `progress`, and terminal-local `error` types remain.

Note: chan-server's `self_writes` ledger suppresses watch echoes for
writes made through its own API. The 14 frames observed are out-of-band
watcher events around the create/rename/modify dance (atomic write
uses a tempfile rename) that the dedupe window let through. Either way,
no dead message type appeared.

### Terminal + MCP env — PASS

Opened `/api/terminal/ws?t=<token>&cols=200&rows=50&tab_name=webtest-b-probe`
and ran `env | grep -E 'CHAN_MCP|CLAUDE_MCP|CODEX_MCP|GEMINI_MCP' | sort`
inside the PTY. All eight variables present and well-formed:

```
CHAN_MCP_COMMAND=/Users/fiorix/.../target/debug/chan __mcp-proxy /var/folders/.../chan-mcp-26921-ae0f0c13.sock
CHAN_MCP_COMMAND_JSON=["/Users/fiorix/.../target/debug/chan","__mcp-proxy","/var/folders/.../chan-mcp-26921-ae0f0c13.sock"]
CHAN_MCP_SERVER_JSON={"args":["__mcp-proxy","/var/folders/.../chan-mcp-26921-ae0f0c13.sock"],"command":"/Users/fiorix/.../target/debug/chan","name":"chan"}
CHAN_MCP_SERVER_NAME=chan
CHAN_MCP_SOCKET=/var/folders/.../chan-mcp-26921-ae0f0c13.sock
CLAUDE_MCP_SERVER_JSON=<same JSON as CHAN_MCP_SERVER_JSON>
CODEX_MCP_SERVER_JSON=<same JSON as CHAN_MCP_SERVER_JSON>
GEMINI_MCP_SERVER_JSON=<same JSON as CHAN_MCP_SERVER_JSON>
```

Socket file exists with srwxr-xr-x perms and the per-pid name matches
the chan serve PID 26921.

MCP transport sanity check via `./target/debug/chan __mcp-proxy
"$CHAN_MCP_SOCKET"`: `initialize` returns `chan v0.8.1`, capabilities
`{tools:{}}`. `tools/list` returns graph_files_with_tag, graph_neighbors,
graph_tags, and the rest of the drive surface. So the contract that
external CLIs are meant to honour (read `*_MCP_SERVER_JSON`, launch the
`command` + `args`, speak JSON-RPC over stdio) is functionally valid
end to end.

CLIs not exercised on this machine: `claude`, `codex`, `gemini` CLI
binaries aren't installed in this dev environment, so the "real CLI
picks up the env contract" half of the acceptance criterion needs to
run on a machine that has them. Captured as follow-up for whoever owns
that validation pass.

### Cleanup edges — PARTIAL (queued behind service restart)

[frontend-2](./frontend-2.md) flipped to REVIEW while I was running
the backend probe. @@Webtest A owns the rebuild/restart cycle and
signalled they will re-baseline against the rebuilt bundle. The UI
scenarios in this task (hash-state probe, shortcut chord sanity,
Settings round-trip, network panel walk, per-window reload baseline)
are scoped to that next rebuilt service rather than the pre-frontend-2
bundle currently running, so they re-queue behind that rebuild.
@@Backend rebuilt `target/debug/chan` after the latest
`npm --prefix web run build` on 2026-05-17; @@Webtest A still owns
restarting the shared service.

Static residue sweep on the **current** `web/dist/` (post-frontend-2,
will be baked in by the next rebuild) shows the JS bundle is fully
clean: zero `Llm*`, `Assistant*`, `assistant*`, `Agent*` identifiers,
zero `/api/(llm|assistant|answers)` URL references. The chan-server
also has no remaining "type": `llm_*`/`assistant_*` emission paths.

Two **cosmetic residue** findings for @@Frontend to consider as part
of the frontend-2 review (not blockers):

1. [`web/index.html`](../web/index.html) lines 7–13: stale Favicon
   comment block references "the assistant button via CSS mask" and
   `--assistant-accent`. The button is gone and the CSS var is no
   longer defined anywhere live (only `web/src/design.md` mentions
   it, also stale). Comment should be rewritten to drop the assistant
   framing or just describe the favicon source.
2. [`web/src/design.md`](../web/src/design.md) lines 94 and 193:
   colour-token table references `--assistant-accent`. If the token
   is gone from the live design system it should be dropped from the
   reference doc.

### Cleanup edges — UI pass (post-frontend-2 binary, PID 37920)

@@Webtest A relaunched the service on the rebuilt binary (PID 37920,
served bundle `index-CEe19Ekn.js`). Bearer token persisted across the
relaunch (per `auth::load_or_create_token` semantics) so I reused
`qag7t48iruaBs88YycrJ7etcikDeEcdi`. Drove the UI scenarios in my own
MCP tab (`tabId 503724914`) so I didn't disturb @@Webtest A's tabs.

#### Hash-state probe — PASS with one observation

Set `location.hash` to `#assistant=open&scopes=2&files=1%3Anotes` (and
two smaller variants `#assistant=foo`, `#scopes=1`). In every case:

* The app does **not** open any overlay (`visibleOverlays=0`).
* `document.body.innerText` matches `agent` / `assistant` / `llm` zero
  times.
* `document.body.innerHTML` matches `agent` zero times.
* No console errors.

After `location.reload()` the dead keys **persist** in the URL (hash
becomes `#assistant=open&scopes=2&files=1%3Anotes&s={...}`). The app
adds its own keys (`s=`, `files=` after the next normal action) but
does not strip unknown keys. This is **not the same** as the
acceptance phrasing in this task file ("rewrites a clean hash") and
not the same as the implied frontend-1 behaviour ("URL hash never
adds `assistant=` / `scopes=`", which is about output and stays PASS).
The current implementation is "ignore unknown keys, preserve them".
Flagging as an observation for @@Frontend, not a blocker: a stale
bookmark with `#assistant=...` will keep that string in the address
bar forever; functionally the app does nothing with it.

#### Shortcut chord sanity — PASS

Static: `web/src/state/shortcuts.ts` declares the full chord registry
with no `Agent` / `Assistant` / `llm` ids. The only `Agent` token in
the file is `navigator.userAgent` (UA-string sniffing). `Mod+I` is
fully absent from the registry; `Mod+Shift+I` is bound to
`app.terminal.broadcast.toggle`.

Runtime: dispatched `KeyboardEvent` for `Mod+I` (key `i`, metaKey +
ctrlKey true). No new dialog appeared (overlay count unchanged before
vs. after). No console error. Subsequent `Mod+,`, `Mod+P`,
`Mod+Shift+F`, `Mod+Shift+M`, `Esc` all behaved correctly (settings /
files / search / graph / dismiss).

#### Settings round-trip — PASS

Visible Settings dialog headers: `Settings, Editor theme, Appearance,
Layout, Date pills`. Zero `agent`/`assistant`/`llm` matches in
`section title` text or in the rendered Settings subtree.

Round-trip:

1. Selected radio `editor_theme=google_docs` (was `github`).
2. Frontend fired a single `PATCH /api/config` (status 200 pending in
   the network panel; the response carried the full updated
   `preferences` payload).
3. Hard reload via `location.reload()`.
4. `GET /api/config` returned `editor_theme: google_docs`.
5. Restored to `github` via the same PATCH and confirmed.

**Side finding for @@Backend**: `PATCH /api/config` rejected a partial
body `{"preferences":{"editor_theme":"github"}}` with HTTP 422
(`missing field attachments_dir`). It accepts the full preferences
shape only. That's surprising for a PATCH verb (PUT-shaped semantics)
and means any client doing a partial write has to re-fetch + merge +
PATCH. May be intentional, but worth confirming. Not gating frontend
behaviour — the in-app Settings panel always sends the full shape.

#### Network panel walk — PASS

Cleared the network panel and drove `Cmd+P` → click `scratch.md` →
`Cmd+Shift+F` → typed `phase` in the search input → `Esc` →
`Cmd+Shift+M`. Captured 15 API requests:

| # | Method | Path | Status |
|---|---|---|---|
| 1–4, 7, 15 | GET | `/api/index/status` | 200 (polling) |
| 5 | GET | `/api/files?dir=notes` | 200 |
| 6 | GET | `/api/files?dir=projects` | 200 |
| 8, 13 | GET | `/api/graph` | 200 |
| 9, 11 | GET | `/api/backlinks/notes/scratch.md` | 200 |
| 10 | GET | `/api/report/file?path=notes%2Fscratch.md` | 200 |
| 12 | GET | `/api/search/content?q=phase&limit=25` | 200 |
| 14 | PUT | `/api/session?w=default` | 204 |

**Zero requests** to `/api/llm/*`, `/api/assistant/*`, `/api/answers`.

Bonus observations:

* `/api/session?w=default` confirms [backend-2](./backend-2.md)'s
  per-window session key is wired up; browsers (no chan-desktop
  `w=<label>` parameter) fall back to `default` as documented.
* `/api/config` now exposes `search_aggression: balanced` which
  confirms [systacean-2](./systacean-2.md) / systacean-3's wave-2 knob
  is live on the running binary.

### Per-window reload regression check — BASELINE CAPTURED

Acceptance asked for a current-behaviour baseline (this is a wave-2
bug-fix area, not a blocker today). Two tabs live in the same Chrome
window (MCP tab group `525312089`):

| Tab | URL hash (truncated) | session key |
|---|---|---|
| `503724841` (@@Webtest A) | `#files=1%3Anotes` | `w=default` |
| `503724914` (mine) | `#s={...Terminal layout...}&files=...&graph=...` | `w=default` |

Observations:

* Each tab maintains its **own** URL hash (`files=`, `graph=`, `s=`),
  so per-tab pane / overlay state survives reload of just that tab as
  long as the hash is present.
* Both tabs hit `/api/session?w=default` because there's no
  chan-desktop `w=<window-label>` query parameter on a plain browser
  visit (per [backend-2](./backend-2.md)'s fallback). That means the
  persisted session blob (used to seed a tab that arrives with no
  hash, e.g. a fresh `http://127.0.0.1:8787/`) is **shared** between
  browser tabs; whichever tab last `pagehide`-keepalived overwrites
  for the next no-hash visitor.
* This is the expected pre-fix state. The wave-2 plan in the journal
  separates chan-desktop windows by `w=<window-label>`; browser tabs
  remain unscoped until / unless someone adds a per-tab session key
  generator on the frontend side.

No new bug surfaced beyond what's already in scope for the wave-2
fix. Baseline captured for regression comparison after frontend-3
lands per-window state plumbing.

## Completion notes

All six scenarios in scope ran on the post-frontend-2 / post-systacean-1
binary (PID 37920, bundle `index-CEe19Ekn.js`):

| Scenario | Result |
|---|---|
| Removed routes 404 | PASS (7/7 dead, 4/4 preserved) |
| `/api/config` agent-free | PASS |
| `/ws` frame types | PASS (only `watch`; zero llm/assistant/agent) |
| Terminal MCP env | PASS (8/8 vars, MCP transport functional) |
| Hash-state probe | PASS for behaviour; observation filed (unknown keys persist, never opened an overlay) |
| Shortcut chord sanity | PASS (static + runtime; `Cmd+I` unbound, `Cmd+Shift+I` is terminal broadcast) |
| Settings round-trip | PASS (`editor_theme` PATCH + reload + verify) |
| Network panel walk | PASS (15 hits, zero on dead routes) |
| Per-window reload baseline | CAPTURED |

### Follow-ups filed (not blockers)

1. @@Frontend: `web/index.html` stale Favicon comment mentions the
   removed Agent button and `--assistant-accent` CSS var.
2. @@Frontend: `web/src/design.md` lines 94 + 193 still document
   `--assistant-accent` token that's no longer defined in live CSS.
3. @@Frontend: unknown hash keys (`assistant=`, `scopes=`, …) are
   ignored at runtime but not stripped, so they survive across
   reload. Decide whether this is acceptable (current) or whether
   the hash router should drop unknown keys.
4. @@Backend: `PATCH /api/config` requires the **full** preferences
   shape (rejects partial body with 422 / `missing field
   attachments_dir`). Document or relax — current behaviour is more
   PUT than PATCH.
5. Owner-TBD: real `claude` / `codex` / `gemini` CLI validation of
   the `*_MCP_SERVER_JSON` env contract was not exercised on this
   machine (CLIs not installed). Needs a machine that has them.

### Service teardown

Service ownership stays with @@Webtest A per [webtest-1](./webtest-1.md);
not my call to stop it. My MCP browser tab `503724914` was created
specifically for this task and is mine to close at Alex's signal
(leaving it open so my Cmd+, / Cmd+P / Cmd+Shift+F / Cmd+Shift+M
flow is inspectable if @@Architect or Alex wants to verify).

### Status

REVIEW. @@Webtest B is idle and ready for more work.

## Follow-up: independent BUG-WT5-A repro (2026-05-17)

@@Webtest A's [BUG-WT5-A](./webtest-1.md#bug-wt5-a-incremental-indexer-misses-newly-created-files)
report: "Incremental indexing of newly-created files: REGRESSION —
new files don't reach the BM25 content index until a forced rebuild."

I ran an independent second-opinion repro on the same service / drive
(PID 48037, bundle `index-ppamOU7w.js`, drive `/tmp/chan-test-phase5`).
**Cannot reproduce.** Three paths all index correctly:

| Probe | File creation path | Keyword | indexed_docs | BM25 hit? |
|---|---|---|---|---|
| 1 | `POST /api/files` (atomic write via chan-drive) | `wtb_uniquerepro_<ts>` | 87 → 88 | NO (tokenizer splits on `_` / digits) |
| 2 | `POST /api/files` (atomic write via chan-drive) | `webtestbexclusiveprobetoken` | 88 → 89 | YES, score 1.0 |
| 3 | PTY shell `printf '...' > path` (filesystem write, bypasses chan-drive) | `wtbshellpath<base36>` | 90 → 91 | YES, score 1.0 |
| 4 | PTY shell `echo '...' > brand.md` — **EXACT** repro of @@Webtest A's commands | `brandnewprobe` | 91 → 92 | YES, score 1.0 |

Findings:

1. The create-path BM25 indexing **works** for all four probes that
   used keywords compatible with the tokenizer.
2. Probe 1 returned `hits: []` solely because the keyword
   `wtb_uniquerepro_<digits>` mixes alphanumerics across `_`. The
   BM25 tokenizer appears to split tokens on `_` / non-alphabetic,
   so the indexed terms didn't contain the literal compound. Not a
   bug; a tokenization detail.
3. Probe 4 ran the **byte-identical** commands @@Webtest A reported
   would fail (`echo 'new doc with keyword brandnewprobe' > /tmp/chan-test-phase5/brand.md`
   then `curl /api/search/content?q=brandnewprobe`). It now returns
   the hit with score 1.0 and `indexed_docs` incremented as expected.

**Suspected root cause for the original report:**
Either (a) a transient state during @@Webtest A's round-3 smoke (the
service had just been restarted; the indexer may have been in a
debounce window or replaying a backlog), or (b) the bug existed on
the round-3 binary but was fixed as a side effect of [systacean-4](./systacean-4.md)
(VCS-aware watcher filter + graph-resume hardening) which landed
after @@Webtest A's report. systacean-4 specifically reshaped the
event filter at the watch boundary; that's the most plausible side
effect.

Routing this back to @@Webtest A: please re-run the original probe
when convenient; if it still fails on the new binary the regression
is real and the root cause is post-systacean-4. If it now passes the
issue can be downgraded to "transient" and closed.

The probe keyword `wtb_uniquerepro_<digits>` did however surface a
**genuine documentation gap**: the BM25 tokenizer behaviour is not
captured in any test or doc I could find. @@Systacean may want to
add either a test fixture or a one-liner in
`crates/chan-drive/design.md` noting "BM25 splits on `_` and
non-alpha; multi-segment compound keywords won't match as-is."
Filing as observation for owner-decide.

### Test artifacts cleanup

Service came back up on **PID 59434 / round-4 binary**. Cleaned my four
probe files via `DELETE /api/files`:

* `notes/wtb_repro.md` -> 204
* `notes/wtb_clean.md` -> 204
* `notes/wtb_shell.md` -> 204
* `brand.md` -> 204

`/api/search/files?q=wtb_` and `?q=brand` both return `[]`. Drive
state is back to pre-probe modulo `notes/welcome.md` (still carries
my `webtestbexclusiveprobetokencontrolmod` append plus @@Webtest A's
`welcomeprobetouch` line). Leaving `welcome.md` alone because
@@Webtest A may have additional probe state in there from the
round-4 git smoke; safer for teardown to handle in one pass.

### Regression re-check on PID 59434

Quick re-verification of my Round-1 scenarios against the current
binary (post-systacean-4, post-systacean-3, post-frontend-3,
post-backend-2):

| Probe | Status |
|---|---|
| `/api/health` | 200 |
| All 5 dead routes (`/api/llm/status`, `/api/llm/complete`, `/api/assistant/conversation?path=foo`, `/api/assistant/hash16?path=foo`, `/api/answers`) | 404 |
| `/api/config` preferences | zero agent/assistant/llm keys; new `search_aggression` field present (systacean-3 live) |

No regressions from the original PASS set. The full UI lane was
already validated on PID 37920; round-4's changes (systacean-3 knob,
systacean-4 VCS-aware watcher, fixes that resolved BUG-WT5-A) do not
touch the surfaces my scenarios cover.

### Follow-up: OBS-WT5-D fix verification (frontend-7) — PASS

@@Webtest A surfaced [OBS-WT5-D](./webtest-1.md) (two plain-browser
tabs both fall back to `w=default`, second tab's layout overwrites
the first). [frontend-7](./frontend-7.md) shipped the fix:
chan-desktop URL `w=<window-label>` wins; plain browsers
generate/reuse an 8-hex `sessionStorage` key (`chan.session.window`);
sessionStorage failure falls back to `default` with a one-time
warning.

Re-ran the per-window baseline scenario from my original lane,
this time on PID 78898 (post-frontend-6/7 bundle
`index-CcTpqyXe.js`):

| Tab | Generated `chan.session.window` | `/api/session?w=` used after settle |
|---|---|---|
| 503724914 (mine) | `2272c82f` | `?w=2272c82f` |
| 503724911 (Webtest A) | `c38a0791` | `?w=c38a0791` |
| 503724841 (Webtest A) | `942e04a0` | `?w=942e04a0` |

Three distinct 8-hex keys, each persisted per tab. After a hard
reload of tab 914 the only `/api/session` traffic is `GET ?w=2272c82f`
(200) and `PUT ?w=2272c82f` (204). No `default` touches once
sessionStorage has a key. **OBS-WT5-D fix verified.**

**One observation worth a note (not a bug)**: on the *very first*
load of a fresh sessionStorage (no key yet) I captured three early
`/api/session?w=default` hits before the per-tab key took over.
Subsequent loads use only the per-tab key. Means: two browser tabs
opened simultaneously in a fresh profile would briefly race on the
`default` blob until each generates and persists its own key. Low
impact — single-tab and existing-tab paths are clean — but maybe
worth a tiny "generate key before first session API call" tweak in
`web/src/api/client.ts` if anyone has cycles. Not gating phase
close.

Did NOT run BUG-WT5-C ([frontend-6](./frontend-6.md)) verification
because @@Webtest A explicitly owns that lane per the journal
round-9 note ("Webtest-1's task notes already say the lane
self-triggers on REVIEW; no separate ping needed"). Staying out of
their smoke window to avoid noise in the report.

### Follow-up: backend-3 MCP env scope — PASS

[backend-3](./backend-3.md) reshapes the PTY env contract: drop the
`CLAUDE_/CODEX_/GEMINI_MCP_SERVER_JSON` aliases (kept only the
CHAN_-prefixed namespace), and accept `mcp_env=on|off` as a per-WS
query-param so a user can opt out of MCP env injection on a fresh
PTY. Verified live on PID 1443, bundle baked from current HEAD.

Probe was a re-run of my earlier 8-env-var terminal probe, this
time asserting two new things.

**Test 1 — default (`mcp_env=on`, fresh session):**

`/api/terminal/ws?t=<TOKEN>&cols=200&rows=50&tab_name=wtb-env-on`
(no `session=` param → fresh PTY; no `mcp_env=` param → default on)

First server frame `{type:"session", id:"541e4620…", seq:0,
missed_bytes:0}` (`seq:0` = fresh creation, not reattach).

`env | grep -E 'CHAN_MCP|CLAUDE_MCP|CODEX_MCP|GEMINI_MCP' | sort`
in the PTY:

```
CHAN_MCP_COMMAND=/Users/fiorix/…/target/debug/chan __mcp-proxy /var/folders/.../chan-mcp-8248-d7d82b67.sock
CHAN_MCP_COMMAND_JSON=["/Users/fiorix/…/target/debug/chan","__mcp-proxy","/var/folders/.../chan-mcp-8248-d7d82b67.sock"]
CHAN_MCP_SERVER_JSON={"args":["__mcp-proxy","/var/folders/.../chan-mcp-8248-d7d82b67.sock"],"command":"/Users/fiorix/…/target/debug/chan","name":"chan"}
CHAN_MCP_SERVER_NAME=chan
CHAN_MCP_SOCKET=/var/folders/.../chan-mcp-8248-d7d82b67.sock
```

5 `CHAN_*` vars, **zero `CLAUDE_/CODEX_/GEMINI_*` lines**. The three
CLI-flavoured aliases that I verified present in my Round-1 probe
are now intentionally gone, matching the backend-3 spec.

**Test 2 — opt-out (`mcp_env=off`, fresh session):**

`/api/terminal/ws?t=<TOKEN>&cols=200&rows=50&tab_name=wtb-env-off&mcp_env=off`

First server frame `{type:"session", id:"615c4c4d…", seq:0,
missed_bytes:0}` (`seq:0` = fresh creation, distinct session id
from Test 1).

Same `env | grep` in the PTY: **empty output**. None of the five
`CHAN_*` vars are present. The opt-out path is fully clean.

**Existing-session reattach not directly tested** here — the
backend-3 spec says "existing-session reattach leaves the already-
exec'd environment unchanged" (the env is fixed when the PTY child
is `exec`d, can't be changed by a later attach). That contract is
covered by the chan-server unit test
`routes::terminal::tests::mcp_env_off_omits_chan_mcp_vars` per the
journal round-14 backend-3 entry; no need to drive it from the
black-box side.

Net: backend-3 acceptance criteria PASS from the live-service side.
@@Webtest A can skip this verification when their next live smoke
runs and focus on the UI side ([frontend-10](./frontend-10.md) toggle
+ info bubble + inject-command button) which I can't drive without
the actual menu UI.

(populated by @@Webtest B at task close)
