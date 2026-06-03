# task-LaneA-LaneD-2: Verify-Lane - build, gate, stage for @@Alex smoke

From: @@LaneA (lead)  To: @@LaneD  Type: verify

## HOLD

Do NOT start until @@LaneA pokes you that @@LaneB (frontend) + @@LaneC (Rust
cleanup) have BOTH landed. Until then, hold.

## Source of truth

new-team-1/desktop-redesign-design.md (LOCKED block + §5 build + smoke).
MODAL path. The launcher redesign: single [New] button -> a 3-choice modal
(local / outbound / inbound); rows ON|WHERE with directional icons + a
connection dot for remote rows; no per-row gear; no header tagline.

## Steps (after the landed-poke)

1. Confirm the working tree has B's desktop/src/* changes + C's
   desktop/src-tauri/* changes (git status / git diff --stat).
2. Build the app: `cd desktop && make build`. Confirm the bundle builds
   clean. (frontendDist=../src, so no web/ npm build is needed.)
3. Full Rust gate: cargo fmt --check; cargo clippy --all-targets -- -D
   warnings; cargo build (workspace). All green.
4. You CANNOT drive the WKWebView click-through (Chrome MCP is Blink and
   cannot reach chan-desktop). So: STAGE the built app and produce a precise
   smoke checklist for @@Alex from design §5, adapted to the MODAL path:
   - Header: single [New]; no tagline; theme toggle flips.
   - [New] -> modal opens (overlay, list dimmed behind); segmented switch
     swaps Local / Outbound / Inbound bodies.
   - Local: Choose folder -> scan rows fill -> 2 toggles default off ->
     Open registers -> modal closes -> new ON|WHERE row, home/computer icon.
   - Outbound: URL + Name -> Attach URL -> outbound row w/ outbound icon.
   - Inbound: Start listening -> port + snippet; copy works; Local|Tunnel
     switches snippet; Stop -> form; Done closes modal, listener KEEPS
     running (reopen Inbound -> still listening).
   - Rows: ON|WHERE header; remote ON = connection dot (no url/tunnel text
     tag); no per-row gear; reveal-in-Finder + Open-split + Forget still work.
   - ESC / backdrop / [X] dismiss the modal; ESC does NOT stop a live
     listener. Empty-state + first-run open the modal on Local.
5. Report to @@LaneA: build + gate status, the built app path, and the
   checklist - so @@LaneA hands the smoke to @@Alex.

## [LaneA] GO - @@LaneB + @@LaneC have BOTH landed

Both lanes landed and the tree is GREEN per their reports (@@LaneB: modal
frontend, make build green; @@LaneC: gear removal + dead-code cascade +
serve.rs canary reconciliation, cargo test -p chan-desktop 79/0 + tunnel_e2e
7/0, clippy/build green). 7 files modified: desktop/src/{index.html,main.js,
styles.css} + desktop/src-tauri/{permissions/app.toml,src/main.rs,src/serve.rs,
src/embedded.rs}.

Proceed with your steps now. RE-RUN the full gate yourself (do not trust the
reports blindly - you are the gate): fmt --check, clippy --all-targets -D
warnings, build (workspace or -p chan-desktop), cargo test. Then build the
app + STAGE it + produce the smoke checklist for @@Alex. Report the built
app path + checklist to @@LaneA so I hand the WKWebView click-through to
@@Alex. Do not commit.

## On completion

Cut a completion task to @@LaneA at tasks/task-LaneD-LaneA-2.md
(append-only) + poke.
