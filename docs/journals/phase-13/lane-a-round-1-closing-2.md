# @@LaneA round-1 closing-2 - Phase 13

You are @@LaneA picking up the SECOND round of round-1 closing
work. The previous @@LaneA + @@LaneB drained the original 12 + 5
items + landed Lane A's A1/A3/A4 round-1-closing slices; main is at
`e30f73ef chore(release): 0.17.0` with the version bump committed
but the v0.17.0 git tag NOT yet cut. @@Alex's empirical walk over
that tree turned up 9 more bugs; this file IS your Lane A task
list (2 inspector-side items: A5 + A6 in this round). @@LaneB owns
the other 7 + the merge-gate + the v0.17.0 release cut.

## Recover context (read in order)

- `/Users/fiorix/dev/github.com/fiorix/chan/CLAUDE.md`
- `/Users/fiorix/dev/github.com/fiorix/chan/design.md`
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/roadmap-round-1.md`
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/bootstrap.md`
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/retrospective-round-1.md` (the round-1 retrospective the previous @@LaneB landed at `a57c259f`)
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/lane-a-request.md` (original Lane A brief; channel + worktree convention)
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/lane-a-round-1-closing.md` (the prior closing brief @@LaneA worked from)
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/lane-a/journal.md` (your predecessor's full self-documentation)
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/coordination/event-alex-lane-a.md` (your inbox)
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/coordination/event-lane-b-lane-a.md` (cross-lane from @@LaneB)
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/coordination/event-lane-b-alex.md` (Lane B merge queue; watch for the new closing-2 merge cycle pings)

## Worktree + branch

Reuse `../chan-lane-a` on branch `phase-13-lane-a`. First action:
rebase on the current main tip.

```
git -C /Users/fiorix/dev/github.com/fiorix/chan-lane-a fetch . main
git -C /Users/fiorix/dev/github.com/fiorix/chan-lane-a rebase main
```

Source-only worktree. Coordination docs / channels / journals
stay in the MAIN checkout edited by ABSOLUTE PATH.

## Scope

User words quoted verbatim where available. File pointers are from
the Explore-agent triage that opened this round.

### A5. Workspace inspector in Dashboard slide 1 doesn't link Languages

User words: "the workspace inspector in the dashboard tab does not
present links for the languages (Markdown, Rust, etc) like it does
on File Browser and Graph".

Diagnosis: `web/src/components/WorkspaceInfoBody.svelte` ~lines
445-454 renders each language as a plain `<span class="lang-name">`,
while the sibling `web/src/components/FileInfoBody.svelte` ~lines
843-854 renders them as `<button onclick={() =>
openGraphForLanguage(lang.name)}>`. The A1 parity slice
(`3c9f57bd`) gated the Notes-directories config behind
`variant="dashboard"` but did NOT carry the language-row onClick
wiring across, so the workspace inspector now reads "almost like a
folder inspector" everywhere EXCEPT for clickable languages.

Files:
- `web/src/components/WorkspaceInfoBody.svelte` (~445-454, props
  destructure ~48-56)
- `web/src/components/EmptyPaneCarousel.svelte` (mount at ~428,
  `<WorkspaceInfoBody variant="dashboard" />`)
- `web/src/components/GraphPanel.svelte` (the variant=inspector
  mount for the graph-side workspace inspector — same fix applies)
- `web/src/components/FileBrowserSurface.svelte` (the
  variant=inspector mount for the FB-side workspace inspector)
- `web/src/components/workspaceInfoBodyParity.test.ts` (Lane A's
  own test from A1; extend with a new pin)

Fix shape:
1. Add `onLanguageClick?: (language: string) => void` to
   `WorkspaceInfoBody`'s `$props()` destructure.
2. Swap the `<span class="lang-name" ...>` for
   `<button type="button" class="lang-name" title="open in graph
   (scoped to this language)" onclick={() => onLanguageClick?.(lang.name)}>`.
3. Pass `onLanguageClick={openGraphForLanguage}` from the three
   mount sites (Dashboard / Graph / FB).
4. Vitest pin in `workspaceInfoBodyParity.test.ts` asserting the
   button + onClick wiring.

Acceptance: clicking the "Markdown" / "Rust" / etc. row in the
Dashboard's slide-1 Workspace info OR in the workspace-root
inspector inside FB or Graph opens a `lang=<name>` graph tab. Same
behaviour FileInfoBody already has for a non-root folder.

### A6. Contact chip doesn't graph-from-here in workspace inspector

User words: "Could not click on a Contact to graph from there,
like I can do with languages and hashtags".

Diagnosis: `WorkspaceInfoBody.svelte` doesn't render a Contacts
section at all today. Lane A's slice 4b wired contact pills inside
`FileInfoBody.svelte` (~1013-1030) calling `openGraphForContact`,
but the WORKSPACE-root inspector body has no equivalent surface.
@@Alex's expectation per the round-1 spec ("the workspace root
inspector ... should become like the inspector of any other
folder") implies contacts also need to appear at the workspace
root.

Files:
- `web/src/components/WorkspaceInfoBody.svelte` (no Contacts
  section to extend; mirror the `FileInfoBody.svelte` block)
- `web/src/components/FileInfoBody.svelte`: read the
  `contactPills` derivation (~lines 262-313) + the section render
  (~lines 1013-1030) and the `navigateContact()` helper.
- Three mount sites again (EmptyPaneCarousel, GraphPanel,
  FileBrowserSurface) — pass `onContactNavigate={openGraphForContact}`.
- `web/src/components/workspaceInfoBodyParity.test.ts` (pin a
  Contacts section render expectation).

Fix shape:
1. Add a `contactPills` derivation to WorkspaceInfoBody mirroring
   FileInfoBody's: read mention/link refs from
   `prefixReport` / `directReport` and surface unique contact
   entries.
2. Add an `onContactNavigate?: (path: string) => void` prop.
3. Render a Contacts section with the same pill markup
   FileInfoBody uses; each pill onClick fires
   `onContactNavigate?.(p)`.
4. Wire `onContactNavigate={openGraphForContact}` at the three
   mount sites.
5. Vitest pin for the Contacts section.

Acceptance: clicking a contact pill in the Dashboard workspace
inspector (or in FB / Graph workspace-root inspector) opens a
`contact=<name>.md` graph tab. Same shape A4b gave folder/file
inspectors.

## Cross-lane

- A5 + A6 are file-disjoint from every Lane B item this round
  (Lane B owns Pane / Graph / Dashboard back-of-card / Welcome /
  serialization). The only shared file is `EmptyPaneCarousel.svelte`
  (Lane B's bug 4 = QR fix at line 411; your A5 + A6 pass props at
  line 428). Cleanly separable; declare on
  `event-lane-a-lane-b.md` before editing per phase-12-onwards
  convention.
- A5's `openGraphForLanguage` and A6's `openGraphForContact`
  helpers already live in `web/src/state/store.svelte.ts` from
  slice 2a. Reuse, don't reimplement.

## Per-slice gate (mandatory before any "ready to merge")

```
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --no-default-features
(in web/)   npm run check
(in web/)   npm run build
(in web/)   npm test
```

Per `feedback_svelte_static_gate_misses_runtime`: A5 + A6 are
reactive Svelte changes; chan-desktop empirical walk lives with
@@LaneB's merge-gate cycle. Hand a clean static gate over.

Append your merge-ready entries to `event-lane-a-alex.md`:

```
ready to merge: phase-13-lane-a@<sha>  -  <one-line slice summary>
```

## Coordination rules

- Append-only channels.
- Each turn, before acting, read:
  - `event-alex-lane-a.md` (your inbox)
  - `event-lane-b-lane-a.md` (cross-lane from @@LaneB)
  - `event-lane-b-alex.md` (Lane B merge queue + release-cut
    progress)
- Self-document in `lane-a/journal.md`.
- DO NOT push to origin without explicit @@Alex ask
  (`feedback_merge_is_not_push`).
- Pre-release per `feedback_pre_release_no_backcompat`: drop
  legacy fields/formats outright; don't add migration shims.

## Out of scope

- Lane B's seven items in `lane-b-round-1-closing-2.md`.
- Anything not in the 9-bug round-2 list. Escalate scope creep to
  @@Alex on `event-lane-a-alex.md`.
- v0.17.0 release cut + the git tag. That's @@LaneB.

## First-turn checklist

1. Rebase `../chan-lane-a` on `main`.
2. Read the recovery files.
3. Append a turn-1 opening entry to `lane-a/journal.md`.
4. Pick whichever of A5 / A6 you can ship cleanest first. Work it
   to the per-slice gate; commit; ping `event-lane-a-alex.md`.
5. Idle on `event-lane-b-alex.md` between turns to catch the
   merge-gate cycles + the eventual "v0.17.0 cut" confirmation.
