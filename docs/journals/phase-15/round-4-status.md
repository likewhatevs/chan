# Phase-15 round-4 status (architect-owned, live)

ACTIVE WAVE: Wave 4 (indexing/graph fixes; Wave 3 CODE-COMPLETE, smokes
pending). WAVE 2 COMPLETE: all four
lanes merged via clean pathspec commits + the merged core re-verified (cargo
check --workspace --exclude chan-desktop = exit 0; chan-desktop excluded only
because the PDF subagent is mid-edit). Wave-2 merge commits: A `7a27e191`
(fedora/arch sdme templates); B `06c371a6` (release.yml M1 multi-arch matrix)
+ `30a3347f` (gateway-linux sdme, VM-verified 4 .deb); C `626593e9`
(server-side team spawn); D `e747f1d2` (phase-8 raw/ deletion). Plus A's
Wave-3 crash fix already merged: `3c437f24`.

WAVE 3 SCOPE (NEW, @@Host-approved; v0.23.0 HOLDS for it): B = static musl
`chan` CLI binary (release.yml musl legs + zigbuild de-risk). A = the two
desktop bug fixes (crash DONE 3c437f24; PDF in flight via subagent) + the
`[[` editor smoke carryover + the release gate + the v0.23.0 cut. C + D have
NO Wave-3 scope (done for the round; standby for lending). See the cross-lane
notes for the musl assessment + the PDF decision.

Round-4 opened on the v0.22.0 base (HEAD at kickoff = 00e5e573). Wave-1 merge
commits: A `99ef3c8e` + `12160198`; B `bb1eed2f`; C `ede23ecf`; D `0a180ffd`
+ `f8c8edec`.

On (re)start, read this first to learn the active wave, then do your lane
doc's section for that wave. @@Architect updates this file at every barrier;
it is the single source of "where are we" after a refresh.

## Wave status

```
legend: -- todo  ~~ in progress  GG gated-green  MM merged
        VV verified (verification-only, no code to merge)
+------+----+----+----+----+
| Lane | W1 | W2 | W3 | W4 |
+------+----+----+----+----+
| A    | MM | MM | MM | MM |   (W3 crash+PDF DONE; W4 favicon d55439c1; A drives the cut)
| B    | MM | MM | MM | -- |   (W3 musl 101c0f66 DONE; done/standby)
| C    | MM | MM | -- | -- |   (W2 626593e9; done/standby)
| D    | MM | MM | -- | MM |   (W4 DONE: spine ce9c286e[live-smoked] + tokei fcf06679 + .txt a5c95545)
+------+----+----+----+----+

WAVE 3 + WAVE 4 CODE-COMPLETE + verified. W3: musl (B 101c0f66) + crash
(A 3c437f24) + PDF paginating (A 173bf693 + ccc117e7). W4 (D, all gated + 3
clean pathspec commits; A re-gated the merged tree green - cargo check + web
svelte-check 0 errors): spine pulse ce9c286e (D LIVE browser-smoked: 4 dirs
orange during a real 359/360 embed sweep), tokei spam fcf06679, .txt-not-a-
graph-document a5c95545 (matches the architect call - .md=document, .txt stays
editable+BM25-searchable; D verified live "document 360" on the all-.md
corpus). Favicon unified d55439c1. Magic-detection feature deferred to round-5.
PENDING for the cut: A's full release gate (make pre-push + gateway) + the
pre-cut release.yml workflow_dispatch dry-run; version bump 0.22.0->0.23.0;
docs(phase-15) round-close commit; @@Host desktop smokes (crash + PDF) + the
favicon visual confirm; then push + tag on @@Host's go.
```

## Wave-1 scope per lane (lanes re-orient from their own lane-doc Wave-1 sections)

- A (architect): write + maintain these coordination docs; coordinate; gate
  the Wave-1 merges + sequence them; run the editor browser-smokes IF @@Host
  re-allows `navigate` (else carry as empirically-unverified); lend subagents
  to @@LaneB. NO heavy coding this wave.
- B: de-risk the build long pole. Get ONE distro (ubuntu, matches CI) building
  chan-desktop from macOS via sdme + emit a valid AppImage + verify the `cs ->
  chan-desktop` symlink dispatches (`cs terminal list` against a server, no
  GUI). New `make` target(s). This is the riskiest unknown; prove it first.
- C: build the `cs terminal team` CLI surface. `--script` FIRST (the
  design-driver): `new --script` / `load --script` emit a runnable shell
  script of the whole bootstrap. Then the `new`/`load` control-socket handler
  (config write/read + server-side bootstrap.md regen via the refactored
  shared fn). Spawn orchestration is Wave 2.
- D: (1) land the semantic wiring (small, self-contained): the route + CLI
  request Mode::Hybrid when `semantic_enabled` (+ model present), else Bm25;
  fix the stale comment. (2) Start phase-8 docs: synthesize the essence
  README + resolve the docs/agents citation handling (do NOT delete raw yet).

## Wave-2 scope (ACTIVE; lanes re-orient here at the refresh)

SCOPE DIVISION (architect, to avoid A-subagent vs B duplication on the
distro matrix): the fedora + arch per-distro `.sdme` rootfs builds are
OWNED BY A's lent subagents (B's driver is DISTRO-parameterized; each distro
just needs `scripts/dev/sdme/chan-desktop-<distro>.sdme`). B does NOT redo
fedora/arch; B owns the gateway-linux build + the release.yml CI matrix.

- B: the gateway-linux build via sdme + extend `.github/workflows/release.yml`
  into the multi-distro CI matrix (the B<->A seam: land + gate BEFORE A cuts
  v0.23.0). Does NOT redo fedora/arch (A's subagents own those .sdme files;
  B consumes the validated templates A integrates into scripts/dev/sdme/).
- C: the lead-first terminal-spawn orchestration. ARCHITECT DECISION:
  orchestrate the spawn SERVER-SIDE via the terminal Registry (lead first,
  full command+env, submit chords) - robust, no SPA edit; keep `--script` as
  the auditable best-effort form. Do NOT half-wire `cs terminal new
  --command/--env` (the gate-blind SPA seam). + tests + a live smoke.
- D: delete phase-8 `raw/` (citation repoint already merged f8c8edec); then
  verify no ghost graph nodes + desktect.md links resolve.
- A: drive + integrate the lent fedora/arch `.sdme` templates; sequence B's
  release.yml matrix to land + gate BEFORE the cut; full release gate (incl.
  gateway workspace); the docs(phase-15) round-4 commit (whole tree +
  retrospective); cut v0.23.0 on @@Host's go (foreground push + ls-remote
  verify; tag fires release.yml). The `[[` Indexing-bubble carryover smoke
  is the last A verification item.
- A's lent subagents (background, worktree isolation): build + VALIDATE
  chan-desktop-fedora.sdme then chan-desktop-arch.sdme via sdme on the lima
  VM (8GiB, so SEQUENTIAL builds), inheriting B's gotchas (TMPDIR=/var/tmp,
  xdg-utils, REBUILD_ROOTFS/CONTAINER). Return the validated templates; A
  integrates + commits (single committer = race-proof).

## Touch points this wave (@@Architect-held)

- B<->A (release.yml): the ONLY cross-lane seam. @@LaneB edits
  `.github/workflows/release.yml` for the multi-distro matrix; the architect's
  release cut USES release.yml. Sequence B's release.yml change to land + gate
  BEFORE @@LaneA cuts v0.23.0 (Wave 2). No file collision within a wave (B owns
  release.yml; A only reads it for the cut).
- No other seams: C's chan-server files (control_socket/team_config) are
  disjoint from D's (routes/search); D's CLI edit (main.rs cmd_search) is
  disjoint from C's chan-shell.

## Carryover from round-3 (tracked for the architect)

- 2 editor browser-smokes (click-to-place-caret, [[ stuck-Indexing bubble):
  gated-green + source-tested, shipped empirically-unverified in v0.22.0
  because `navigate` was denied to ALL lanes. Re-run when @@Host re-allows
  navigate; A owns.
- The chip-clobber fix (round-3 41e7908e) was partially confirmed live (a
  server showed embedding:{done,total} during a real background embed); the
  full edit-during-embed transition is locked by the set_idle_reattaches unit
  test. No round-4 action.

## Cross-lane notes (latest at top)

- NEW SCOPE + @@Host BUG REPORTS (2026-06-01, post-crash, @@Host-directed):
  - WAVE 3 = STATIC MUSL `chan` BINARY (@@Host APPROVED; v0.23.0 HOLDS for it).
    Ask: ship a fully-static standalone `chan` Linux binary so a too-new build
    libc does not block old machines. Assessment (grounded): the CUDA blocker
    is already gone (embeddings default to pure-Rust candle CPU since 044c23ff;
    `cuda` is an opt-in feature, NOT default). TLS = rustls+ring, NO openssl-sys
    in the tree (the usual musl killer is absent). The C/C++ deps the musl build
    must cross-compile: ring, libsqlite3-sys (bundled), tokenizers' esaxx-rs
    (C++) + onig (C). Tool = cargo-zigbuild (zig as cross cc; both musl arches
    from one runner). SCOPE: standalone `chan` tarball -> musl static; `.deb`/
    `.rpm` stay gnu (distro has glibc); chan-desktop AppImage stays gnu (webkit
    can't be static; @@Host OK with lax desktop). Integration point found:
    packaging/linux/Makefile chan-tarball has an explicit gnu-only guard to
    lift. TOOLING ALREADY READY on the Mac: zig 0.15.2 + cargo-zigbuild
    installed, x86_64-unknown-linux-musl target added. The one unknown to
    retire (Wave-3 de-risk, B + a lent subagent, mirrors the AppImage de-risk):
    does the full embeddings+tokenizers+candle tree link FULLY static under
    zigbuild musl - one build proves it. OWNER: B's lane (release.yml
    linux-cli-artifacts job + a musl tarball make path). Sequences AFTER B's
    Wave-2 M1 release.yml lands (same file). A project memory captures the
    standing decision.
  - @@Host BUG 1 (HIGH - crash): chan-desktop, a URL-type remote workspace
    (connect-not-listen) whose remote server is DOWN. Open -> a new window comes
    up all-white/stuck; clicking that window's macOS red-dot close button
    CRASHES THE WHOLE app (all windows). ROOT CAUSE (subagent, grounded):
    closing the window fires the CloseRequested handler
    (desktop/src-tauri/src/serve.rs:405-413) -> capture_window_config_on_close
    (serve.rs:438) calls `window.url()` to snapshot the URL hash. For an
    OUTBOUND window whose remote navigation FAILED (server down), WKWebView's
    URL is nil/empty, so tauri-runtime-wry does `.parse().expect("invalid
    webview URL")` (lib.rs:3912) / wry does `.URL().unwrap()` (wkwebview
    mod.rs:1349) -> PANIC on the Tauri event-loop thread -> unwinds through
    tauri::App -> whole process aborts. chan's `match window.url()` can't catch
    it (panic is upstream of the Result). Desktop-only (Tauri/wry/WKWebView).
    ADJACENT RISK: the SAME handler runs for workspace-*/tunnel-* windows, so
    any window whose backend died before close hits the identical panic.
    FIXED -> `3c437f24` (A): capture_window_config_on_close skips url() for
    outbound-* windows (the reported repro, thread-independent) + catch_unwind
    around the read for local/tunnel windows (the adjacent backend-died case).
    Gated green (fmt+clippy+chan-desktop tests). EMPIRICALLY UNVERIFIED - needs
    a @@Host desktop smoke (WKWebView, not Chrome-automatable): open the
    outbound URL workspace with its server down -> close the stuck white window
    -> app must survive. Full investigation: tasks/a76f8e58a9ddbc916.output.
  - @@Host BUG 2 (PDF export): "Export as PDF" is a no-op in macOS desktop-
    native. ROOT CAUSE (subagent, grounded): the feature calls `win.print()`
    (web/src/editor/print.ts:314) via the hidden-iframe printer; WKWebView (wry)
    does NOT implement window.print(), so it silently no-ops (works in a real
    browser = desktop-only). The team already solved the analogous Download gap
    with a native Tauri command (desktop/src-tauri/src/download.rs +
    save_file_to_downloads). Fix options: (A) native Tauri save-PDF command
    mirroring Download [recommended pattern]; (B) server-side render [heavy,
    violates single-binary principle - reject]; (C) bundle a JS PDF lib
    (jsPDF+html2canvas) -> Blob -> reuse save_file_to_downloads [frontend-only,
    unifies browser+desktop].
    DECISION (@@Host): option A - NATIVE VECTOR PDF on macOS now (WKWebView
    createPDF of the themed print HTML -> bytes -> reuse save_file_to_downloads,
    NO new permission); Linux desktop-native = HIDE the Export-to-PDF button
    entirely (no Linux PDF code this round); web = unchanged (win.print()).
    Wave-3 item, OWNED BY A (@@Host: "I drive both in Wave-3"). A spawned an
    impl subagent (desktop/src-tauri pdf cmd via objc2-web-kit + web/ button-
    visibility + the macOS/web/linux branch); A gates + commits; @@Host smokes
    on macOS (WKWebView not automatable). Crash bug = DONE 3c437f24; PDF = DONE
    173bf693 (gated green) but used WKWebView createPDF = a SCREEN capture that
    does NOT run the print pipeline -> clips long notes AND ignores the editor's
    @pagebreak feature (page_break.ts -> <hr class="chan-page-break">; print.ts
    emits break-after:page + @page margins). @@Host requires pagination + user
    page breaks. FIX-FORWARD (A, subagent in flight): switch the native path to
    the macOS PRINT pipeline (WKWebView.printOperationWithPrintInfo -> silent
    save-to-PDF), which honors @page + auto page-breaks + .chan-page-break.
    PAGINATION REWORK DONE -> `ccc117e7` (print pipeline; gated green, re-verified
    by A). .app being REBUILT at target/release/bundle/macos/Chan.app (build-only,
    @@Host's running app untouched) so @@Host can smoke crash + the paginating PDF
    (multi-page note + an @pagebreak note) in one go. SAFETY: builds only
    compile/bundle to target/; NEVER launch a built .app from the agent side (no
    single-instance guard -> would collide with @@Host's session).
  - TEMPLATES INTEGRATED -> `7a27e191` (A, race-proof pathspec commit, only the
    2 .sdme files). @@LaneB IS UNBLOCKED: the .sdme paths its release.yml M1
    comment references now exist in main, and the VM is FREE (Wave-3 musl uses
    zigbuild on the MAC, not the VM, so no VM contention). B can land the M1
    release.yml + run the held gateway-sdme build.
  - @@LaneB CLEARANCE (A reviewed the release.yml M1 diff - CORRECT, matches the
    M1 decision: multi-arch matrix amd64 ubuntu-latest + arm64 ubuntu-24.04-arm,
    stages the .rpm, globs per format dir, suffixed upload name. GO):
    1. MERGE the release.yml M1 matrix now (race-proof pathspec; seam condition
       met - templates in main @ 7a27e191, before the cut).
    2. Run the gateway-sdme VM build (VM free) + commit the gateway static files;
       if it fights like arch's AppImage, carry it (dev-surface, not tag-time).
    3. DO NOT add the musl CLI legs to release.yml now - that is a SEPARATE
       Wave-3 edit (you own it; sequences after this Wave-2 barrier). release.yml
       is NOT "done" after M1.
    4. The authoritative release.yml validation is the pre-cut workflow_dispatch
       DRY-RUN (publish=false) A runs before tagging - actionlint-not-local is
       fine; the static YAML+structure gate is enough to merge.
    Once (1)+(2) land + you poke Wave-2 done, the Wave-2 barrier is COMPLETE
    (A templates + C spawn + D raw all done).
  - ARCH BUILD OUTCOME (DECIDED, A): arch .deb + .rpm VALIDATED (all pacman dep
    names resolve; cargo built). The AppImage step FAILS at linuxdeploy even
    WITH NO_STRIP=1 (added to arch.sdme as a known-required fix, but there is a
    SECOND undiagnosed cause; tauri swallows linuxdeploy's stderr). DECISION:
    stop chasing it - arch's AppImage is REDUNDANT (ubuntu+fedora emit the
    universal aarch64 AppImage), arch ships no native pacman package, and the
    .sdme is dev-surface-only (zero blast radius). Carried to round-5 backlog
    (capture linuxdeploy's real error via a direct verbose run). The arch.sdme
    is integrated as deb+rpm-validated / AppImage-unverified, documented in the
    template + the backlog. (NB: a trailing `; echo` masked make's real exit on
    the first arch run - read the build log, not the task-notification exit.)

- CRASH RECOVERY (2026-06-01, A re-oriented after a SESSION CRASH, not a
  refresh). The lent fedora/arch subagent's task handle did NOT survive the
  crash; A re-established the build state empirically on the VM. Findings:
  - FEDORA: VALIDATED. The crash KILLed `chan-desktop-build-fedora` at 13:11
    (`Container ... terminated by signal KILL`), but it had ALREADY emitted all
    three bundles before the kill. A restarted the stopped container (came up
    HEALTH=ok, warm 2.6G cargo target cache + bundles intact) and confirmed:
    `Chan_0.22.0_aarch64.AppImage` (ELF aarch64), `Chan_0.22.0_arm64.deb`
    (Debian pkg), `Chan-0.22.0-1.aarch64.rpm` (RPM v3.0). The fedora .sdme dep
    names are proven (the rootfs built + the container booted to
    graphical.target). A re-ran the driver end-to-end (warm container ->
    re-bundle -> copy-out to target/linux-desktop/fedora/) to land clean
    host-side evidence.
  - ARCH: base rootfs is GOOD (it has /usr/bin/pacman = genuine Arch Linux ARM;
    `sdme fs ls` MISLABELS it "Ubuntu 26.04 LTS", a cosmetic OS-detect quirk,
    not a wrong image). The arch build never ran (crash hit during fedora). A
    starts it after fedora copy-out (8GiB VM = one build at a time); the only
    arch-specific risk is the pacman dep names (webkit2gtk-4.1 / libayatana-
    appindicator / etc.) - the cargo + Tauri-bundler path is distro-agnostic,
    already proven on ubuntu AND fedora.
  - INTEGRATION PLAN: both validated .sdme templates land in main
    `scripts/dev/sdme/` in ONE commit once arch validates (single committer =
    race-proof); then A signals B "templates landed + VM free" so B's release.yml
    M1 matrix (which references the .sdme paths in a comment) can merge, and B's
    held gateway-sdme build can run.
  - B + C have UNCOMMITTED Wave-2 work in the tree (B: Makefile linux-gateway
    target + gateway/scripts/dev/sdme/*; C: control_socket.rs + team_config.rs +
    submit.rs). Those are their lanes; they recover in their own tabs and
    re-orient from these docs. A does NOT touch them.
  - VM build dependency note: NONE of the sdme builds (fedora/arch desktop,
    gateway-linux) are on the v0.23.0 TAG-TIME critical path - they are local
    dev/QA validations. release.yml (M1) builds everything natively in CI. So
    the cut does not block on the VM; the VM work strengthens confidence.

- WAVE-2 IN PROGRESS (2026-06-01, A re-oriented after a refresh). Ground
  truth as found on disk + VM:
  - D: DONE + merged + verified. `e747f1d2` docs(phase-8): drop raw/ (283
    files). Verified static: zero markdown/wikilink refs into raw/, so no
    ghost graph nodes possible; desktect.md links resolve (the one broken
    `skills/architect.md` is the pre-existing round-5 backlog item, not
    phase-8). D's Wave-2 needs nothing further. D poked wave-2-done.
  - A's lent fedora/arch .sdme subagent IS IN FLIGHT (spawned by the prior
    architect instance; the task handle did NOT survive the /clear, so A
    now tracks it EMPIRICALLY, not via TaskOutput). State:
    - Worktree: `.claude/worktrees/agent-a2dc714cce9bdd5ab` (locked, @
      bb1eed2f base). Both templates ALREADY WRITTEN there:
      `scripts/dev/sdme/chan-desktop-{fedora,arch}.sdme`.
    - VM: `chan-desktop-build-fedora` container building NOW (cargo done,
      in the linuxdeploy/AppImage bundle stage as of ~13:00). `arch` is
      sequential next (8GiB VM = one build at a time). ubuntu container
      from B's Wave-1 still booted (idle).
    - On completion A integrates the two validated .sdme files into the
      MAIN tree `scripts/dev/sdme/` (single committer = race-proof), then
      unblocks B's release.yml matrix.
  - VM SEQUENCING DECISION (A, to resolve the A-subagent<->B contention on
    the single 8GiB VM): A's fedora+arch builds OWN the VM until they
    finish. B HOLDS any local gateway-sdme build until A signals "VM free".
    B may meanwhile write the release.yml multi-distro matrix edit (pure
    text, no VM) referencing `chan-desktop-{fedora,arch}.sdme`; it lands +
    gates AFTER A integrates the templates and BEFORE the v0.23.0 cut.
  - RELEASE.YML MECHANISM (A DECIDED M1, the B<->A seam): B surfaced a
    fork (round-4-lane-b-release-matrix.md). GH-hosted runners are
    ubuntu-only, so literal fedora/arch CI = `container:` jobs that
    re-list distro deps in release.yml = the single-source DRIFT that
    killed the v0.19.0 cut, AND can't be validated locally (ships
    unverified INTO the release workflow = tag-time break risk), AND arch
    emits no native package. DECISION = M1: extend linux-desktop-artifacts
    to ubuntu multi-ARCH (amd64 + arm64-runner, matching the CLI/gateway
    jobs) and STAGE the .rpm the job currently drops. CI ships universal
    AppImage + .deb + .rpm on amd64 AND arm64 (arm64 desktop is a real gap
    today). Zero drift, fully gateable. The fedora/arch .sdme files own
    the LOCAL multi-distro dev/QA build path (the backlog's primary "build
    from macOS via sdme" deliverable) and are referenced in a release.yml
    COMMENT, not a CI container - so the subagent work is the dev surface,
    not wasted. CI fedora coverage (M3's one container build-smoke) is a
    round-5 option, not v0.23.0 (the local sdme fedora build already
    validates fedora-specific breakage; M3's drift-y dep list isn't worth
    it now). B writes + statically-gates the M1 release.yml NOW (no VM);
    it MERGES after A integrates the .sdme templates (so the comment
    points at real paths) and BEFORE the cut. Host informed; can override
    toward M2/M3 if literal CI fedora/arch was specifically wanted.
  - B: gateway-linux build + release.yml matrix not yet in the tree
    (release.yml untouched vs HEAD; no gateway chan-desktop .sdme). Gateway
    static files (gateway-build.sdme + build-gateway.sh + root Makefile
    `linux-gateway`) ARE written + statically gated; the VM gateway build
    is held until A signals VM-free.
  - C: server-side spawn orchestration not yet in the tree (no diff on
    control_socket/team_config/chan-shell vs the Wave-1 merge).

- @@LaneB Wave-1 is FUNCTIONALLY DONE (journal): ubuntu chan-desktop builds
  end-to-end headless via sdme (AppImage/.deb/.rpm emitted), gated, files
  disjoint - but still UNCOMMITTED in the tree. ARCHITECT CALL: B's Wave-1 is
  cleared to commit (gated-green + disjoint, no seam this wave). Land it +
  poke wave-1-done; that is the last thing the Wave-1 barrier waits on.
  - B ESCALATION (architect-decided): the `cs -> chan-desktop` argv0 dispatch
    is BROKEN on the AppImage. linuxdeploy's generated AppRun re-execs
    `AppRun.wrapped` WITHOUT `-a`, so argv0 resets to the wrapped path and
    `invoked_as_cs` fails -> the GUI launches (panics headless). Detection +
    the cs client + control-socket round-trip are SOUND on the real artifact;
    only the AppImage argv0 plumbing breaks. DECISION: round-5/phase-16
    BACKLOG (not v0.23.0) - the fix touches `cs_install.rs`/chan-shell
    (flagged DONE/don't-edit) and the macOS + CLI cs paths work; the AppImage
    GUI launch is itself out of scope this round. B's fix hook: a custom
    AppRun honoring the `ARGV0=cs` the type-2 runtime exports
    (`exec -a "${ARGV0:-$0}"`, AppRun.wrapped preserving it), or detection
    reading `ARGV0` / a `CHAN_INVOKED_AS_CS` env the wrapper sets.
  - Wave-2 lend: once B's driver is committed, A spawns worktree subagents
    for the fedora + arch .sdme rootfs templates (driver is already
    DISTRO-parameterized; each distro just needs its dep-name variants).
- A Wave-1 DONE for the merged code (carryover smokes below). Two injected
  @@Host nits, both confirmed LIVE (navigate, throwaway drive) + merged:
  - `99ef3c8e` fix(editor): relative-markdown link pills openable + `#`/`^`
    surfaced. Confirmed: plain-click that dead-ended on "No matches" now
    shows the linked file (basename search) -> Enter re-picks / Cmd+Enter
    opens; Cmd-click opens directly; `[[Welcome#` reaches headings; footer
    advertises `# heading - ^ block`; pill has a Cmd-click tooltip.
  - `12160198` fix(nav-help): Hybrid Nav `p` row relabeled "Stage Team Work
    Terminal" (was the stale "Stage Smart Prompt Terminal"). DOM-verified.
  Carryover smokes: click-to-place-caret VERIFIED (caret landed accurately
  across the whole editor-fix session); `[[` stuck-Indexing bubble exercised
  on an idle index (picker fetches + renders live; startIndexWatch wired) -
  the mid-build stuck-bubble race is hard to time on a tiny drive; will
  retry against a larger reindex before the cut, else carry as before.
- Wave-2 pending architect decisions (for the refresh, not now):
  - C spawn mechanism: non-`--script` `new` should orchestrate the lead-
    first spawn SERVER-SIDE via the terminal Registry (robust; no SPA edit),
    keeping `--script` as the auditable best-effort form. A leans this way
    (avoids the gate-blind half-wired `cs terminal new --command/--env` SPA
    seam C flagged). Confirm at the Wave-2 refresh.
  - D Wave-2: delete phase-8 `raw/` (citation repoint already merged
    f8c8edec); then verify no ghost nodes + desktect.md links resolve.
- Backlog (round-5/phase-16): pre-existing broken link `skills/architect.md`
  in BOTH desktect.md:27 and architect.md:15 (docs/agents/skills/ empty),
  surfaced by @@LaneD. Not a phase-8 citation; out of this round's scope.
  ALSO: (a) C's SPA-visibility item - a registry-spawned `cs terminal team new`
  team is listable/pokeable/broadcastable but its panes do NOT auto-surface in
  an SPA window (no server->SPA "attach existing session to a pane" window-
  command; adding one is an SPA edit). By design this round (headless/automatable
  contract; Cmd+P stays the SPA-visible path). (b) arch chan-desktop AppImage
  linuxdeploy failure (deb+rpm work). (c) cs-on-AppImage argv0 dispatch (B,
  round-3 carryover). (d) the PDF export native path if not done in Wave-3.
- Wave 1 running (2026-06-01). All four lanes bootstrapped + self-orienting.
  @@Host RE-ALLOWED browser `navigate` (2026-06-01). A can now run the 2
  carryover editor smokes live (throwaway drive, scoped pkill, tear down).
- @@Host bug report (2026-06-01, triaged by A): `[[` link from a draft to
  `new-team-1/bootstrap.md` -> picker inserts `[bootstrap](../../new-team-1/
  bootstrap.md)` (relative markdown is the default form) and the target
  RESOLVES (normalizeHref -> new-team-1/bootstrap.md). The "no match / can't
  click open" is a UX artifact, not a broken link: right after insert the
  caret sits on the link line so source is revealed (not a clickable pill);
  clicking the revealed URL lands the caret in the URL slot, which
  triggers.ts re-opens the wiki picker in "raw" mode with the literal
  `../../...` path as the query -> link-targets can't match it -> the "No
  matches in 2159 documents" bubble. `#` heading + `^` block link modes
  ALREADY SHIP (wiki.ts classifyQuery); they are just undiscoverable (footer
  shows only insert/open). Not in any worker's lane (editor).
  @@Host SCOPE CALL (2026-06-01): A takes it this round, targets v0.23.0.
  Scope = (1) fix the caret-in-URL-slot raw-mode picker so it searches a
  useful term (basename) / offers open instead of "No matches"; (2) add a
  discoverability hint for clickable links + the `#`/`^` modes. Confirm live
  first (navigate now allowed). Latent draft-promotion `../` break stays a
  round-5/phase-16 backlog item (not in v0.23.0 scope).
- Round-4 opened. Coupling is LOW (disjoint lanes), so expect fewer
  barrier stalls than round-3. The gated-push SIGPIPE rule
  (round-4-bootstrap.md) is standing: foreground + file-redirect + verify with
  git ls-remote.
