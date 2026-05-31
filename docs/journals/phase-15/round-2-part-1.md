# Phase 15 round 2, part 1: finish the dropped Lane-A items

Author: @@Architect (for @@Alex)
2026-05-31

## Context

Phase 15 round 1 shipped as v0.20.0, but several Lane A tasks from
`roadmap-round-1.md` never landed. Lane A's progress log in `lane-a-tasks.md`
stops at A2 (carousel + per-slot Dashboard backs); A3 and A4 were dropped and
not journaled. A second, line-by-line re-read of the roadmap's Dashboard
"About" slot (prompted by @@Alex) then turned up two more drops inside an
otherwise finished-looking slot: A6 and A7.

- **A4 / BUG-2** (the reported bug, confirmed by screenshot): in the Dashboard
  "Search" carousel slot, clicking a directory node opens an inspector that
  shows [Upload] [Download] but is missing "Show Directory", "Graph from
  here", and "New Terminal". Per the roadmap it must drop Upload and add those.
- **A3**: the Dashboard tab has no right-click menu to toggle which carousel
  slots participate in auto-rotation, plus a Settings (Cmd+,) entry.
- **A6**: the About slot front never moved the chan license next to the version
  (roadmap front layout `chan version {version} {license}`); it still sits in a
  separate attributions block below "Fund the work".
- **A7**: the About slot back theme dropdown does not change the screensaver
  preview (roadmap "changing the value changes the preview"); the preview is
  hardcoded to Matrix, so selecting "Default" leaves the Matrix frame on
  screen.

Scope for this part: A4 + A3 + A6 + A7. All frontend-only (Svelte 5); no Rust
is touched. The session that produced the A4/A3 plan had an unstable shell, so
static gate + browser smoke are listed as a single clean pass to run at
implementation time.

What DID land on the About back, for the record (verified, no action): the
theme relabel Plain->Default (`AboutSlotConfig.svelte:311`) and the PIN text
s/yet/set, "No PIN set" (`:279`).

## A4: Search-slot directory inspector actions

### Root cause (verified in source)

The live Dashboard front carousel is `web/src/components/EmptyPaneCarousel.svelte`
(mounted by `DashboardTab.svelte` via `<EmptyPaneCarousel>`). There is no
`dashboard/SearchSlot.svelte`; only the back-side `*SlotConfig.svelte` files
exist. Slide index 2 ("Search") renders the index graph (`<GraphCanvas>`) plus
its own inspector. That `<InspectorBody>` mount (around lines 592-599) passes
only:

```
<InspectorBody
  selection={{ kind: "directory", path: selectedIndexPath, label: selectedIndexLabel }}
  showRefs={false}
/>
```

No host action handlers are passed. In `FileInfoBody.svelte`'s
`actionsSection`, the directory branch renders "Show Directory" only
`{#if onReveal}` and "Graph from here" only `{#if onSetAsScope}` (both
undefined here), while the Upload + Download transfer block is unconditional,
and no New-Terminal affordance exists. That fully explains the screenshot.

### Changes

1. `web/src/components/FileInfoBody.svelte` (shared body; props approx 82-130,
   `actionsSection` approx 657-752):
   - Add two optional props: `onNewTerminal?: () => void` and
     `allowUpload?: boolean` (default true, so File Browser / editor / full
     Graph-tab inspectors are unaffected).
   - Gate the Upload button (approx 707-712) behind `{#if allowUpload}`; keep
     Download always.
   - Add a "New Terminal" button in the directory actions, after the "Graph
     from here" block (approx 735), shown `{#if onNewTerminal && isDir}`.
2. `web/src/components/InspectorBody.svelte` (dispatcher): add `onNewTerminal`
   + `allowUpload` to props and forward them to `FileInfoBody` in the
   `directory` arm (approx 101-108); forward `allowUpload` in the `file` arm
   too.
3. `web/src/components/EmptyPaneCarousel.svelte` (the screenshot mount, approx
   592-599): bind the handlers, each guarded on `selectedIndexPath !== null`
   ("" is the workspace root and is handled by all helpers):
   - `onReveal` calls `revealPathInBrowser(selectedIndexPath, { enter: true, inspectorOpen: true })`
   - `onSetAsScope` calls `openFsGraphForDirectory(selectedIndexPath)` (opens a new fs-graph tab)
   - `onNewTerminal` calls `openTerminalInPane(layout.activePaneId, terminalFromHereTarget(selectedIndexPath, true))`
   - `allowUpload={false}`

### Reuse (no new primitives)

- `revealPathInBrowser`        `web/src/state/store.svelte.ts:1804`
- `openFsGraphForDirectory`    `web/src/state/store.svelte.ts:1687`
- `terminalFromHereTarget`     `web/src/terminal/fromHere.ts`
- `openTerminalInPane` / `openTerminalInActivePane` / `layout`
                               `web/src/state/tabs.svelte.ts:1071 / 1067 / 743`

These are the same helpers the File Browser tree context menu uses
(`web/src/components/FileTree.svelte` approx 647-657).

### Decisions

- Keep Download; the roadmap only calls for dropping Upload.
- New Terminal is directory-only (terminal in the selected dir's cwd). The
  index-graph inspector is effectively directory-only anyway: non-directory
  node ids yield a null `selectedIndexPath`.
- Upload is suppressed only for this host via the default-true `allowUpload`
  flag, so no other inspector regresses.

## A3: Dashboard tab slot on/off menu + Settings

### Requirements (roadmap "Dashboard")

Right-click the Dashboard tab to get a vertical list of the carousel slots
(About / Workspace / Search), each with an on/off checkbox. At least one stays
checked; default all-on; per tab; unchecked slots are skipped in auto-rotation.
A separator, then "Settings (Cmd+,)" which flips the tab to its back.

### Changes

1. `web/src/state/tabs.svelte.ts`:
   - `DashboardTab` type (approx 434-443): add `disabledSlots?: number[]`
     (slide indices; absent/empty means all enabled).
   - Serialize (`SerTab` dashboard arm approx 3713-3723): emit `ds?: number[]`
     only when non-empty. Restore arm (approx 3946-3960): read it back; if the
     restored `carouselSlide` points at a disabled slot, clamp to the first
     enabled slot.
   - Small helpers: `dashboardSlotEnabled(tab, i)`, `toggleDashboardSlot(tab,
     i)` (refuses to disable the last enabled slot), `firstEnabledSlot(tab)`,
     `nextEnabledSlot(tab, from)`.
2. `web/src/components/DashboardTab.svelte`: enrich the existing
   `HamburgerMenu` `menuItems` (today just "Reload") with a checkbox row per
   slot, a separator, and a "Settings (Cmd+,)" button calling
   `flipHybrid(paneId)` (`tabs.svelte.ts:3064`). Reuse the checkbox-row markup
   pattern from `TerminalTab.svelte` (approx 1604-1642). Keep Reload.
   - Menu surface: the roadmap says "right-click the tab's title". The tab
     strip's context menu lives in `Pane.svelte` (approx 1063-1088, via
     `openTabMenu` + `tabMenu` state in `state/tabMenu.svelte.ts`), separate
     from `DashboardTab`'s own body menu. Decide at implementation whether to
     add a dashboard arm to the tab-strip menu render or to open
     `DashboardTab`'s rich menu from the tab-title right-click. Either way the
     slot checkboxes + Settings must be reachable from the tab title for
     parity with other tabs.
3. `web/src/components/EmptyPaneCarousel.svelte` auto-rotation (advance
   `$effect` approx 328-335, `slideCount = 3`): advance to the next enabled
   slot instead of `(slideIndex + 1) % slideCount`, and clamp the active slide
   to an enabled one. Pass the per-tab disabled set down via a new prop so the
   carousel knows what to skip.

### Decisions

- Slot identity is the numeric slide index 0/1/2 (consistent with
  `carouselSlide` and `slideCount`); persist the disabled set, omit the key
  when empty (pre-release, no migration path).
- Min-one-enabled is enforced in `toggleDashboardSlot`.
- Per tab: the field lives on `DashboardTab`; new tabs start all-on.
- Settings reuses `flipHybrid` (same path as the global Cmd+, in `App.svelte`
  approx 870).

### Caveat: an existing test locks the *opposite* in

Lane A deliberately shipped the no-menu state, so A3 must *reverse* a decision,
not just add to it:

- `DashboardTab.svelte` carries a design comment ("No Settings entry here:
  Cmd+, is the canonical flip") and its body menu is Reload-only.
- `web/src/components/dashboardTabAndCarousel.test.ts:282` is a passing vitest
  test titled "DashboardTab right-click menu carries only Reload (no Settings
  entry)" asserting the Settings row, `settingsOpen` state, `openSettings` /
  `closeSettings`, and a `HybridSurfaceConfigShell` import are all absent.

Implementing A3 means updating/removing that lock-out test (and the design
comment), not only extending the suite. Otherwise the new Settings entry fails
the existing assertion and the gate goes red.

## A6: About-front license placement

### Why it was missed

A4 + A3 were the obvious Lane-A drops (an unreachable bug + a whole menu). This
one is subtler: the About slot *looks* finished, but the roadmap front layout
(`roadmap-round-1.md:38`) is `chan version {version} {license}` and the license
never moved next to the version. It surfaced only on a line-by-line re-read of
the About spec.

### Root cause (verified in source)

`web/src/components/EmptyPaneCarousel.svelte` slide 0 (About, approx 417-508)
renders the version alone on its grid row (approx 425-426):

```
<span class="k">chan version</span>
<span class="v mono">{buildInfo?.version ?? "n/a"}</span>
```

Chan's own license link ("Apache 2.0",
`https://github.com/fiorix/chan/blob/main/LICENSE`) lives in the
`about-licenses` block (approx 488-507), *below* "Fund the work" and a
separator, grouped with the third-party attributions. The roadmap wants the
chan license up on the version row.

### Changes (move ONLY chan's Apache 2.0)

1. In the `about-grid` version row (approx 425-426), render the license next to
   the version value: the `{version}` string followed by the existing
   `Apache 2.0` anchor (same LICENSE URL). Per the layout @@Alex confirmed:

   ```
   chan version  0.20.0  Apache 2.0
   embeddings    on (hybrid search)
   ```

2. Remove the `chan` / `Apache 2.0` pair from `about-licenses` (approx 489-492);
   leave the `terminal font` (SIL OFL 1.1) and `matrix screen lock` (MIT)
   attributions in that below-the-fold block untouched.
3. Update the block comment (approx 481-486): it currently claims "Chan's own
   Apache 2 license joins the section so the three runtime licenses sit
   together" - no longer true once chan's license moves up.

### Reuse / no backend change

- The license string is hardcoded in the template today and stays hardcoded.
  `BuildInfo` (`crates/chan-server/src/routes/build_info.rs:7-11`) exposes
  `version` + `features` only; do NOT add a license field for this.

### Decisions

- Move only chan's Apache 2.0 link to the version row (confirmed by @@Alex).
  The third-party font + screensaver attributions stay in the block below
  "Fund the work".
- Keep it a link (not plain text), same URL as today.

## A7: About-back screensaver preview reacts to theme

### Why it was missed

The About *back* mostly landed (theme relabel Plain->Default at
`AboutSlotConfig.svelte:311`; PIN text s/yet/set at `:279`). The one piece
dropped is the roadmap line "changing the value changes the preview"
(`roadmap-round-1.md:48`): the preview is hardcoded to Matrix, so selecting
"Default" leaves the Matrix frame on screen.

### Root cause (verified in source)

`web/src/components/dashboard/AboutSlotConfig.svelte:344-354` hardcodes the
preview and its comment admits the punt:

```
<!-- ... The Default (plain) theme preview is
     DashboardSlotBack's job; this slot only previews Matrix. -->
<section class="screensaver-preview">
  <h3>Screensaver preview</h3>
  <div class="preview-box">
    <MatrixRainPreview width={320} height={180} />
  </div>
  <p class="hint">Static preview of the Matrix lock theme.</p>
</section>
```

The theme `<select>` (approx 304-314, `value="plain"` labeled "Default" /
`value="matrix"` labeled "Matrix") already binds to `screensaverTheme` and
updates on change; the preview just ignores it.

### Changes

1. Make the preview switch on `screensaverTheme`:
   - `{#if screensaverTheme === "matrix"}` -> keep `<MatrixRainPreview>`.
   - `{:else}` -> render the plain/Default preview: the same look the real lock
     screen shows for non-matrix, i.e. the chan enso mark on a dark backdrop.
2. Update the hint to track the active theme ("Static preview of the Default
   lock theme" / "... Matrix lock theme").

### Reuse (no new primitive look)

- Plain preview = the `.screensaver-mark` treatment from
  `web/src/components/ScreensaverOverlay.svelte:155-160` + its CSS at `:221`
  (`/chan-mark.png` mask over `--text-secondary`, ~0.38 opacity, dark
  backdrop), scaled into the preview box. A small standalone
  `PlainScreensaverPreview.svelte` - mirroring how `MatrixRainPreview.svelte`
  encapsulates the static matrix frame - keeps `AboutSlotConfig` clean and the
  back face timer-free (no rAF for the static plain frame).

### Decisions

- Default preview mirrors the real lock screen (enso mark on dark backdrop),
  not a blank box (confirmed by @@Alex). The preview swaps live as the dropdown
  changes.

## Tests

Extend `web/src/components/dashboardTabAndCarousel.test.ts` (the existing
?raw source-pattern suite) plus a FileInfoBody assertion:

- A4: the slide-2 `InspectorBody` mount now passes `onReveal` /
  `onSetAsScope` / `onNewTerminal` + `allowUpload={false}`; `FileInfoBody`
  gates Upload on `allowUpload` and renders New Terminal with `onNewTerminal`.
- A3: the menu lists the slots with checkboxes; the last checked slot cannot be
  unchecked; `ds` round-trips and is omitted when empty; auto-rotation skips
  disabled slots; `carouselSlide` clamps off a disabled slot; the Settings
  entry calls `flipHybrid`. Remove/replace the existing
  "carries only Reload (no Settings entry)" assertion (`:282`) it now
  contradicts (see the A3 caveat).
- A6: the About front version row carries the `Apache 2.0` LICENSE anchor; the
  `about-licenses` block no longer contains the `chan` row but still has the
  font + screensaver attributions.
- A7: the About-back preview renders `<MatrixRainPreview>` only when
  `screensaverTheme === "matrix"` and the plain enso-mark frame otherwise; the
  hint string tracks the selected theme.

## Verification (one pass, on a stable shell)

1. Static gate in `web/`: `npx svelte-check` (0/0), `npm run test` (vitest),
   `npm run build`.
2. `cargo build -p chan`; serve a throwaway drive with nested directories (ask
   new-vs-reuse drive + seed first).
3. Browser smoke (required; static gates miss Svelte-5 runtime reactivity):
   - A4: Dashboard, Search slot, click a directory node. Inspector shows Show
     Directory + Graph from here + New Terminal, no Upload (Download present).
     Click each: a File Browser tab opens at the dir; a new fs-graph tab roots
     at the dir; a terminal opens with cwd equal to the dir. Confirm File
     Browser / editor / full Graph-tab directory inspectors still show Upload
     (no regression from the default-true flag).
   - A3: right-click the Dashboard tab title for slot checkboxes; uncheck a
     slot and confirm auto-rotation skips it; confirm the last enabled slot
     cannot be unchecked; Settings (Cmd+,) flips to the back; reload and
     confirm the disabled set + slide persist; a fresh Dashboard tab starts
     all-on.
   - A6: Dashboard, About slot front. The version row reads
     "chan version {version} Apache 2.0" with Apache 2.0 a working link; the
     font + screensaver attributions still sit in the block below "Fund the
     work"; no duplicate chan license.
   - A7: Dashboard, About slot back (Cmd+, or Settings). Toggle the Theme
     dropdown: "Matrix" shows the matrix frame, "Default" shows the enso-mark
     plain frame; the hint text follows the selection. The back face stays
     timer-free (no animation/rAF while flipped).

Frontend-only; no Rust expected. If any Rust is touched, also run the repo
pre-push gate (fmt, clippy -D warnings, test, build --no-default-features).
