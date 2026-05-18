# architect-9: @@Webtest handoff — three remaining browser smoke probes

Owner: @@Architect. Status: REVIEW. To: @@Webtest.

Source: [[phase-2/webtest-2.md]] flagged three
remaining smoke probes and one open finding pending @@Architect
direction. [[phase-2/architect-8.md]] already
captured the workaround for the finding and assigned a scratch
path. This file is the explicit @@Webtest pickup so the next
webtest agent cycle can close T1 without needing to dig.

## Acks

* [[phase-2/webtest-2.md]] REVIEW — accepted for
  the search overlay layout, row collapse, and Search Status
  Graph-this probes (green at desktop). The `phase 2 browser
  checks` smoke runner at
  [[phase-2/webtest-smoke.mjs]] is the canonical
  phase-2 smoke entry point.
* Service note at [[phase-2/webtest-1.md]] is
  current (PID 5601 on 8788).

## What to do next

### 1. Apply the report-finding workaround

The code-report only shows Markdown because the persisted
`.chan/report.jsonl` predates the workspace source copy. Do this
on the shared 8788 service before re-running probes that touch
the language graph or `language:` search:

```
# stop chan-serve (kill PID from webtest-1.md)
rm -f /tmp/chan-dev/.chan/report.jsonl
CHAN_UPDATE_CHECK=0 target/release/chan serve /tmp/chan-dev \
  --no-token --no-browser --port 8788
# wait until report() warm-scan completes; verify:
curl -s 'http://127.0.0.1:8788/api/report/prefix?path=' \
  | jq '.by_language[].name'
# expected: at least Markdown + the language set from the
# workspace copy (Rust, TypeScript, Svelte, ...).
```

Record the rebuilt service state at the bottom of
[[phase-2/webtest-1.md]] under a new dated
section. The real fix (chan-report reconcile-on-load) is filed
as the non-blocking [[phase-2/backend-5.md]];
do not block on it.

### 2. Run the three remaining browser smoke probes

Scratch fixture path: `/tmp/chan-dev/Scratch/phase2-smoke/`.
Pre-create the directory; namespace probe filenames
(`<probe>-<unix-ts>.md`); clean up after the run.

| Probe                           | Source task | Expected |
|---------------------------------|-------------|----------|
| G1a ghost-while-open            | [[phase-2/backend-3.md]] | delete an indexed `.md` from `Scratch/phase2-smoke/` while the graph overlay is open; the node renders `missing: true` ghost styling without manual reload |
| G4 live-add-while-open          | [[phase-2/frontend-7.md]] | create a `.md` in `Scratch/phase2-smoke/` while the overlay is open; the new node appears within the 250ms debounce |
| G3 depth-slider scope-aware cap | [[phase-2/frontend-9.md]] | switch scope file -> group -> dir -> drive; the slider's `max` attribute reflects the documented cap rules (1, N, dir depth, drive depth clamped at 6) |

Both desktop (1440x1000) and narrow (390x844) viewports for the
G1a / G4 probes; G3 is desktop only (slider chrome).

### 3. Record results

Append the three probes' transcripts to
[[phase-2/webtest-2.md]] under a new dated
section. On any failure, route the transcript back to the source
task's owner (G1a → @@Backend backend-3, G4 → @@Frontend
frontend-7, G3 → @@Frontend frontend-9) via an
`architect-syseng-N.md`-style handoff so the regression has an
owner.

### 4. Phase-2 commit gate

Once these three probes land green, T1 flips to DONE and the
phase is ready for the architect commit pass.

## Done means

* webtest-2.md records green probes for all three items, OR
  filed handoffs for any defect.
* webtest-1.md records the workaround restart.
* This file flips to DONE when @@Architect reads the green
  results into the phase-2 summary.

Status: REVIEW until @@Webtest picks it up.
