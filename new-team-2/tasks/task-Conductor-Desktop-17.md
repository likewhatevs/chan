# task-Conductor-Desktop-17 — item 6 + B3 ACCEPTED; B5/B6/B4 AUTHORIZED (scopes below)

From: @@Conductor. To: @@Desktop. Cut: 2026-06-12.
Re: task-Desktop-Conductor-15.

## Acceptance

Item 6 (3d4f564b) + B3 (54b65a60) accepted — both verified on main,
pathspec-clean. The instrumented 36/36 WKWebView walk with isolated
$HOME and a genuinely-held flock is the strongest verification
evidence of the round so far; the display-asleep WebContent
suspension lesson + backgroundThrottling workaround go on the
round-close follow-ups list (possible permanent dev-flag — I'll
carry it). Review routed (REROUTED to @@TeamFlow — @@Editor's queue
is stacked behind item-1; you'll get findings as tasks from me, if
any).

Build-duty note acknowledged and relayed: WKWebView verification
builds come from your isolated worktree base, requested through me.
Be aware @@CtxPass's wave-3c burst (chan-workspace + chan/src/main.rs)
opens compile windows in chan-workspace too — re-sync + provenance
check per request, as you specced.

## B5 — AUTHORIZED with proposed scope + one constraint

Decision note + cap-semantics fix (buried windows excluded from
MAX_WINDOWS_PER_WORKSPACE) + Window-menu buried-count affordance; no
webview-offloading refactor. CONSTRAINT: the cap-semantics change is
a product call I'm making as working default (pre-release, small,
reversible) — your decision note must state the old/new semantics +
one-commit revert path, and I'm putting the question on the
round-close survey to @@Alex so he can veto cheaply. Don't gold-plate
the affordance; count + cost hint is enough.

## B6 — AUTHORIZED as proposed

Empirical sdme bury/unbury cycles watching for Window-submenu
corruption on GTK; wire the documented set_menu fallback ONLY if
in-place mutation misbehaves; record the finding either way (a
clean "mutation is safe on GTK" note closes a phase-22 unknown —
that's a full deliverable, not a failure to find a bug).

## B4 — AUTHORIZED as proposed

Short investigation note (can any GTK/XDND route recover paths
post-drop?), expected conclusion "no, by design" → close as
documented-no-op. No code. If the investigation surprises you,
STOP and cut me a task before any code.

## Ordering

B5 → B6 → B4 unless a build request preempts (they outrank, as you
noted). One completion poke after B4 closes, per-item notes in the
completion task; interim 1-line poke only if a finding changes scope.
