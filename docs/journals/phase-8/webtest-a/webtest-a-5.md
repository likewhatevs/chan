# webtest-a-5 — Hybrid back-side correction wave walkthrough (-a-47 + -a-48 + -a-53 + -a-54)

Owner: @@WebtestA
Cut: 2026-05-21 by @@Architect
Status: dispatched

## Goal

Bundled walkthrough covering the Hybrid back-side
correction wave + the design-correction follow-ups that
shipped post `webtest-a-4`. Four slices, one verdict.

* **`fullstack-a-47`** (`dd586fc`) — Drop front/back
  independent theme; single per-Hybrid value.
* **`fullstack-a-48`** (`0391eae`) — Migrate
  Search/Indexing/Reports settings to Hybrid FB back-side
  (option B: SPA wiring + default ON; backend gating
  deferred). chan-reports toggle restored to user-visible
  state per the bug-list regression entry.
* **`fullstack-a-53`** (`8c65296`) — Hybrid back-side
  theme architecture correction (Appearance reverted to
  SettingsPanel + per-Hybrid `inherit | light | dark`
  override toggle in both Editor + Terminal backs) +
  custom-TERM PARTIAL fix bundled.
* **`fullstack-a-54`** (`714ec48`) — Hybrid flip UX
  redesign (tab strip preserved + mirrored tabs +
  hamburger swap + family-name title in tab area).

This walk closes the design-correction follow-up loop
from `webtest-a-4`'s 1 PARTIAL + the @@Alex design
corrections that surfaced after `-a-46` shipped.

## Background

### Design-correction context

@@Alex flagged two corrections after `-a-46` landed (see
[`../architect/round-2-plan.md`](../architect/round-2-plan.md)
§"Theme architecture correction 2026-05-21" + §"Flip UX
correction 2026-05-21"):

1. Appearance system/dark/light is a GLOBAL DEFAULT
   (Settings overlay); per-Hybrid OVERRIDE is the
   user's way to say "this surface specifically renders
   different."
2. Flip UX keeps tab strip in same physical position;
   tabs mirror; hamburger swaps to opposite end; family-
   name title shows INSIDE the tab area.

`-a-53` + `-a-54` deliver both corrections. This walk
verifies them empirically.

### `-a-45` PARTIAL re-verification

`webtest-a-4` surfaced a custom-TERM PARTIAL on `-a-45`
check #3: `setTermSelection("__custom__")` seeded
`default_term=""` but `currentTerm` derivation collapsed
to `DEFAULT_TERM`. Root-caused in
`HybridTerminalConfig.svelte:104` + `:86-88`. `-a-53`
bundled the fix per the architect routing. This walk
re-verifies the PARTIAL is now HOLD.

## Coverage slice (lane A)

Pure SPA work; standing terminal + Chrome MCP perm
covers the walk. Single chan + test-server boot; walk
all four slices + the PARTIAL re-verification; commit a
single bundled verdict.

## Acceptance criteria

### `-a-47` — Drop front/back independent theme (4 checks)

1. **Per-Hybrid theme is single value** — open a Hybrid
   pane; set a theme via hamburger toggle; flip
   front/back; confirm BOTH sides render the same theme.
   No more front-vs-back theme split.
2. **Cross-Hybrid independence** — open a second
   Hybrid pane; confirm it does NOT inherit the first
   Hybrid's theme (per-pane independence preserved).
3. **Wire format `bm` marker** — round-trip a Hybrid
   pane (e.g. via tab serialize/restore); confirm
   back-existence survives without a per-side theme
   value.
4. **Legacy migration** — if your test drive has any
   stored Hybrid panes from pre-`-a-47`, confirm
   front-side wins on theme migration.

### `-a-48` — Search/Indexing/Reports settings in Hybrid FB back (5 checks)

1. **Open a Hybrid File Browser pane**; flip to back.
   `HybridFileBrowserConfig.svelte` should now show the
   three toggles (was empty stub pre-`-a-48`).
2. **Semantic search toggle**: matches the prior
   SettingsPanel `-a-21` behaviour (full state machine
   + polling + BuildInfo guard + formatModelSize). Persist
   + round-trip.
3. **Multi-model picker**: disabled `<select>` showing
   `BAAI/bge-small-en-v1.5` as default. Round-3 placeholder
   per spec; confirm it renders but doesn't accept input.
4. **chan-reports toggle**: visible + default ON. Toggle
   it OFF; flip back; confirm persistence. Help text
   should EXPLICITLY say backend gating + destructive-
   on-disable land in follow-up (i.e. "OFF" doesn't yet
   stop indexing — honest-toggle UX).
5. **Settings overlay (`Cmd+,`) shrunk**: no Semantic
   search / chan-reports sections any more. Should just
   be About + GlobalConfig autosave plumbing.

### `-a-53` — Theme architecture correction (5 checks)

1. **Appearance section BACK in Settings**: open `Cmd+,`;
   confirm Appearance (system/light/dark) is present
   again (regression-verify that `-a-46`'s migration was
   reverted).
2. **Per-Hybrid theme override toggle on Hybrid Editor
   back**: open a Hybrid Editor pane; flip; confirm the
   3-option toggle (Inherit / Light / Dark) is present.
   Default selected: Inherit.
3. **Per-Hybrid theme override toggle on Hybrid Terminal
   back**: same as above for Hybrid Terminal.
4. **Resolution order — override > global**: set
   Settings Appearance = Dark; on an Editor pane set
   override = Light. Confirm THAT Editor renders light;
   other panes stay dark.
5. **Resolution order — inherit > global**: with Editor
   override = Light, switch back to Inherit. Confirm the
   Editor pane reverts to dark (tracking the global).

### `-a-53` bundled — custom-TERM PARTIAL re-verification (1 check)

6. **Custom TERM input renders when selected** —
   re-walk `webtest-a-4` `-a-45` check #3. Open a Hybrid
   Terminal pane; flip; select "Custom..." in the Default
   TERM dropdown. The custom-TERM input field should now
   appear (was the PARTIAL fail in `webtest-a-4`).
   Set a value (e.g. `vt100`); persistence round-trips.

### `-a-54` — Flip UX redesign (6 checks)

**DESIGN-CORRECTION CONTEXT 2026-05-21**: @@Alex
clarified post-`-a-54` ship that the family-name title
should NOT appear in the tab strip chrome — only the
back-side config view (which already has it per
`-a-43`'s stubs) carries the title. Plus: in flipped
state, tabs should align RIGHT (not left), since flipped
means "looking from behind." `-a-55` is the corrective
follow-up (cut 2026-05-21; sits between this walk and
the next wave).

Walk the CURRENT state as below; grade checks #5 + #6
with the design-correction context in mind (don't FAIL
the walk for the family-name title's presence in the
strip — `-a-55` will remove it; also expect tabs to
align LEFT in this walk's snapshot since right-alignment
hasn't landed yet).

1. **Front state unchanged**: visual identity matches
   pre-`-a-54` (un-flipped panes look the same).
2. **Flipped state — tab strip preserved**: same physical
   position when flipped (no chrome rotate).
3. **Flipped state — tabs mirrored**: tab labels render
   mirrored (`scaleX(-1)` or equivalent). Visual sanity:
   reads as "viewed from behind."
4. **Flipped state — hamburger swapped**: hamburger on
   the OPPOSITE end of the tab strip when flipped
   (e.g. front: right end → back: left end). Click
   functional + menu anchors correctly.
5. **Flipped state — family-name title visible
   (CURRENT STATE; slated for REMOVAL via `-a-55`)**:
   "Hybrid Terminal" / "Hybrid Editor" / etc. shows
   INSIDE the tab area. Capture as HOLD or PARTIAL +
   note: "to be removed via `-a-55`; back-side config
   view's own title is the canonical surface."
6. **Tab switching from back**: with a Hybrid flipped,
   click a mirrored tab. Active tab swaps; back-side
   config component swaps. The family-name title in
   the back-side CONFIG VIEW (NOT the tab strip's
   title) updates to match — that's the canonical
   indicator.

### `-a-54` / `-a-55` — design-correction side observation

Capture two side observations in the verdict tail (NOT
failures, just design-history notes for the walk):

* Tab-strip family-name title (check #5) is the
  architect-side misinterpretation slated for removal
  in `-a-55`.
* Tab alignment in flipped state: currently LEFT-aligned
  per `-a-54` ship; should be RIGHT-aligned per
  @@Alex's "tabs aligned to the right.. because we
  flipped" correction. `-a-55` includes the
  right-alignment fix.

Don't grade either as a failure; the architect-side
misinterpretation produced `-a-54`'s current shape,
and `-a-55` is the corrective follow-up.

### Walkthrough audit trail

Append a fresh dated heading to
[`webtest-a-1.md`](webtest-a-1.md):
`## 2026-05-21 — fullstack-a-47 + -a-48 + -a-53 + -a-54
walkthroughs (Hybrid back-side correction wave + design
follow-ups)`. Capture:

* All four slices' acceptance subsections + the PARTIAL
  re-verification with HOLD / FAIL / PARTIAL verdict
  per check.
* Screenshots at each step (especially the flipped-state
  visual deltas + the per-Hybrid theme override
  examples).
* Side observations for the bug list.
* Tear-down evidence.

## How to start

1. `git status` confirm clean; `git log --oneline -15`
   confirms `dd586fc`, `0391eae`, `8c65296`, `714ec48`
   all in HEAD.
2. Spin up a fresh test server. Throwaway drive seed:
   chan-source default (matches `-3`/`-4` pattern) or
   ad-hoc.
3. `cargo build -p chan`; `web/npm run build`; restart
   server.
4. Walk `-a-47` four checks first (theme collapse;
   foundation for `-a-53`).
5. Walk `-a-48` five checks (FB-back migration).
6. Walk `-a-53` five checks + the custom-TERM PARTIAL
   re-verification (theme architecture correction).
7. Walk `-a-54` six checks (flip UX redesign).
8. Append the bundled verdict to `webtest-a-1.md`; fire
   poke to @@Architect via
   `event-webtest-a-architect.md`.
9. Tear down per the standing rule.

## Coordination

* @@WebtestA lane (reactive).
* Standing terminal + Chrome MCP perm covers the walk.
* If regression-class issues surface, file bug-list +
  flag for @@Architect routing.

### Pre-commit discipline carry-forward

Same shape as `-3` + `-4`:

* `git commit <path> -m "..."` path-limit OR explicit
  `git add` per path + pre/post-commit audits.
* Discipline catches stowaways when applied (proven by
  `56e6692` save during `-3` commit).

## Numbering

Highest committed `webtest-a-N` is `-4` (`06afe3f`
verdict + `c9fb768` close-out marker); this is `-5`.

## Out of scope

* Graph overhaul sub-wave (`-a-49..52`) — not yet
  landed; folds into `webtest-a-6` or later.
* `-a-42` About section build-out — not yet committed
  (gates on A+B+C+F all landed; now technically
  unblocked but parked behind graph overhaul).
* chan-reports backend gating + destructive-on-disable
  modal (deferred follow-up from `-a-48` option B;
  not yet cut as a task).
