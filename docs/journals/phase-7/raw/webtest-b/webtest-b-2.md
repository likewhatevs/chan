# webtest-b-2: wave-1.5 walkthrough Lane B

Owner: @@WebtestB
Cut by: @@Architect
Date: 2026-05-18

## Goal

Once wave-1.5 lands (the new `fullstack-6` pane cluster,
`fullstack-7` light-mode contrast, plus the `fullstack-2`
external-link revision once it lands), walk through the Lane
B-relevant pieces on the running 8810 server.

Until those land you have one task you can run NOW: confirm
B14 (terminal sessions silent after reload) really is closed
on current main with the latest commits in (`systacean-1` /
`fullstack-1` / `fullstack-5` / `systacean-2` all in). Your
earlier pass said NOT REPRO — re-verify after a fresh
rebuild.

## Relevant links

* [./webtest-b-1.md](./webtest-b-1.md) — your prior Lane B
  baseline + adjacent passes.
* [../fullstack-a/fullstack-6.md](../fullstack-a/fullstack-6.md)
  — pane cluster (B15 click semantics, pane menu reorg,
  color, next/prev, doc tab menu).
* [../fullstack-a/fullstack-7.md](../fullstack-a/fullstack-7.md)
  — light-mode terminal contrast.
* [../fullstack-a/fullstack-2.md](../fullstack-a/fullstack-2.md)
  — external-link revision (tunnel-aware Tauri shell.open).

## Acceptance criteria

### Immediate (do now)

* B14 confirmation: rebuild `cargo build -p chan`, restart
  the 8810 chan serve, repro your prior B14 test (output in
  background terminal, reload the page). Confirm session
  re-attaches, input enabled, scrollback retained. If
  scrollback retention still missing: file as a clean
  follow-up.

### After wave-1.5 lands

When @@FullStack pings each of fullstack-6 / fullstack-7
ready (and after my architect-side clearance), run these:

* **fullstack-6 Lane B coverage**: B15 click handlers, pane
  right-click menu shape, doc tab right-click menu
  (terminal already had one — verify the new doc-tab menu
  works), focus border color toggle (try all three on the
  same pane), Next/Prev pane shortcut + menu entries.
* **fullstack-7**: spot-check legibility of a 16-color test
  in light-mode terminal. Quick visual.
* **systacean-3 follow-up** (if it lands): re-repro the
  drift bug; verify whatever fix @@Systacean ships actually
  prevents the hop.

For each, append a dated section with verdicts in this
task file.

## Out of scope

* `fullstack-2` external-link walkthrough — that's
  @@WebtestA's `webtest-a-3` lane.

## Hand-off

Fire `alex/event-webtest-b-architect.md` (type `poke`) on
completion of each batch.

## Permission scope

Your earlier permission grant covers cargo build + chan
serve + browser automation. Wave-1.5 testing reuses the
same shell scope; no fresh permission event needed unless
you're testing a tunnel-loop variant.

## 2026-05-18 18:30 BST - systacean-3 pre-commit verification (partial)

Noticed the post-recycle `cargo build -p chan` baked the
uncommitted `systacean-3` patch (still shows as
`M crates/chan-server/src/static_assets.rs` in
`git status`) into my dev binary. That makes the Lane A +
Lane B re-repro you queued runnable NOW against the
patched binary, no commit required.

### Pre-fix vs post-fix server header probe (via curl)

```
GET / on 8810 (chan-webtest-b-1, patched binary):
  HTTP/1.1 200 OK
  content-type: text/html; charset=utf-8
  cache-control: no-store
  vary: Host
  content-length: 1097

GET /assets/index-<hash>.js on 8810:
  HTTP/1.1 200 OK
  content-type: application/javascript; charset=utf-8
  cache-control: public, max-age=31536000, immutable
  vary: Host
  content-length: 1134896

GET / on 8811 (chan-webtest-b-drift, patched binary):
  identical shape; same SPA shell length (1097).
```

Matches the `systacean-3` proposal exactly: SPA shell is
`no-store`, hashed assets are `immutable` for a year, both
`Vary: Host`. The diff also adds two unit tests
(`static_cache_headers_do_not_store_spa_shell`,
`static_cache_headers_allow_immutable_assets`).

### Service-worker state (was a candidate hypothesis)

```js
navigator.serviceWorker.getRegistrations()  => []
navigator.serviceWorker.controller          => null
```

No SW registered, no SW controlling. The SW hypothesis was
already weak per @@Systacean's source grep; this confirms
the runtime story matches.

### Drift recipe under the patched binary (in-progress)

Setup:

* 8810: my round-1 Lane B drive `/tmp/chan-webtest-b-1`
  (patched binary).
* 8811: new throwaway drive
  `/tmp/chan-webtest-b-drift` seeded with `index.md`
  (marker `DRIFT-DRIVE-ROOT`) + `drift.md` (patched
  binary).
* 8801: @@WebtestA's Lane A chan serve came up partway
  through; I did NOT touch their drive.

Recipe runs:

1. Two-Lane-B variant (8810 + 8811 only): navigated
   8810 -> 8811 -> 8810 with a multi-tab fragment URL,
   then reversed. **NO drift hop observed.** Every nav
   stayed on the originating port; tree contents matched
   the originating drive.
2. Welcome-state pane menu Files-action variant (8810
   only): URL fragment-driven into welcome state on
   8810. Page rendered welcome content for
   `chan-webtest-b-1`. **No port hop.** This rules out
   the post-recycle Files-action hypothesis as a sole
   trigger on the patched binary.
3. Three-server variant (8810 + 8811 + 8801 coexistence):
   I started 8810 -> 8801 -> 8811 -> 8810 with multi-tab
   fragment URL, and **before I could finalize the read,
   both my background `chan serve` processes
   (8810 + 8811) received `SIGTERM`** and exited. Lane A
   on 8801 stayed alive. The Lane-A-coexistence variant
   is the historically reliable trigger; I did not get a
   clean verdict on this variant before the kill.

### Interim verdict

**Strong:** under the patched binary, two simultaneously
running Lane-B style drives on different ports + same-
host alternation do NOT trigger the silent cross-port
hop. Headers are exactly what the proposal targets. SW
hypothesis is cleared.

**Outstanding:** the Lane-A-coexistence variant (the
original drift trigger) was not completed before my
servers were SIGTERM'd. Re-running this requires:

1. Coordinate with @@WebtestA so their 8801 Lane A
   server stays up.
2. Restart my 8810 + 8811 servers (the dev binary still
   includes the patch).
3. Re-run the multi-tab fragment recipe and capture
   port + drive name fields.

### Why the kill is worth flagging

The SIGTERM hit both my background servers but spared
@@WebtestA's 8801. So this wasn't a blanket
`pkill chan`; the kill was specific to my two PIDs.
Could be:

* @@Alex / @@Architect coordinating around Round 2 setup
  (e.g., wanting only one Lane-B server at a time so the
  drift conditions stay clean).
* A teardown tool acting on my session's process tree.
* @@Systacean iterating on the static-assets patch and
  needing to restart the bound port; less likely since
  8811 wouldn't conflict.

Either way, I'm parking the re-repro rather than
silently relaunching. Architect can poke me back to
continue when conditions are right.

### Acceptance status update

* B14 immediate (Section "Immediate (do now)"): done in
  the 17:30 BST appendix on `webtest-b-1.md`. Verdict:
  B14 NOT REPRO; B19 narrowed to scrollback-only.
* `systacean-3` re-repro (queued under "After wave-1.5
  lands"): partial verdict above. Two-Lane-B variant
  shows no drift on the patched binary. Lane-A-
  coexistence variant outstanding.
* `fullstack-6`, `fullstack-7` walkthroughs: still
  blocked on @@FullStack landing those.

Firing a partial poke on
[../alex/event-webtest-b-architect.md](../alex/event-webtest-b-architect.md).
