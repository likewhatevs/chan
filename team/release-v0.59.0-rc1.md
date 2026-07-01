# v0.59.0-rc1: editor list hang-indent, smart list paste, and directory links

Prepared on a dedicated `editor-fixes` worktree off `main`, this RC collects the editor bug fixes from the v0.59.0 request. It is three commits, each gated green and verified in the browser, staged on the branch and not yet merged or tagged. The v0.59.0 feature request (mermaid to excalidraw) was held out of scope for this pass, on the explicit steer to fix the bugs without introducing new ones.

## Theme

Make list editing look and behave the way a writer expects, and make links to folders act like links to folders. The list work is presentation-only: the markdown document is never rewritten, so everything round-trips. The link work is a small additive wire flag plus a routing branch, with file and broken-link behavior left untouched.

## What was asked

The v0.59.0 request listed four editor bugs and one feature:

- Directory links rendered as broken, and clicking one rejected the folder with a "not a text file" toast instead of opening the file browser.
- A setext-heading bold flash. This one was dropped during the round: Alex judged it a non-issue and chose to leave setext headings on.
- Wrapped continuation lines of a list item fell back to the left margin instead of hanging under the item text.
- Ordered (enumerated) lists indented less than bullet and hyphen lists.
- A mermaid-to-excalidraw conversion feature, held for a later cut.

Two more asks surfaced while validating the list work:

- A smart paste so that copying a whole list row and pasting it into a freshly continued list item does not leave a double marker.
- A broad editing-behavior sweep: click to edit, text selection, copy and paste (including rows that contain an image), Enter continuation and exit, and Tab / Shift-Tab indent.

## What shipped

| Commit | Summary |
| --- | --- |
| `7e63c5e8` | Hang-indent wrapped list continuation lines (bugs 3 and 4, all list types, all depths, tasks included) |
| `8941deaa` | Merge a pasted list row into the current bullet (the double-marker fix) |
| `787f1ad6` | Open directory links in the file browser (bug 1) |

### List hang-indent

Wrapped continuation lines now align under the item text across hyphen, asterisk, plus, ordered-period, ordered-paren, and task lists, at every nesting depth. The source whitespace around a marker, meaning the leading indent and the gap before the item text, is hidden render-only so the text starts at a fixed marker column. A static CSS rule pads the line by that column and pulls the first line back with `text-indent`, so the wrapped lines hang under the text. Indentation is driven by the item's syntactic depth, one marker column per level, rather than by source indentation, so a nested marker sits under its parent's text and ordered lists step the same width as bullets. Task checkboxes share the marker column, and resetting `text-indent` on that column keeps the marker glyph and the checkbox inside it and keeps the checkbox clickable.

### Smart list paste

Copying a list row and pasting it into an existing list item, typically the empty one Enter just created, now flows the pasted content into that bullet instead of inserting a second marker. The paste handler already stripped a leading marker for the rich-HTML path through `dedentListPaste`; the same dedent now runs on the plain-text path, which is what a chan-to-chan copy uses (`navigator.clipboard.writeText`). It only fires when the caret sits on a list line and the pasted text starts with a marker, so every other paste defers to CodeMirror's default. The paste handler, `pasteHandler`, carries three responsibilities: image defer, rich-HTML conversion, and the plain-text list dedent.

### Directory links

`resolve_link` now detects a directory target after its file-candidate probe and returns it with a new `is_dir` wire flag. The link renders as a valid `directory` pill instead of the broken strikethrough, and the click opens the file browser at that folder through `openBrowserInActivePane` rather than handing a directory path to the text editor. File links and genuinely missing links keep their existing behavior. The `is_dir` field is additive with a serde default, so no `NodeKind` variant was needed and the route serializes it without any route change.

## Validation

- Rust gate on the changed crate and the whole workspace: `cargo fmt --check`, `cargo clippy` with `RUSTFLAGS="-D warnings"`, `cargo check --workspace` (covers chan-server and chan-desktop), `cargo test -p chan-workspace`. The resolve_link suite is 15 tests including two new ones for the directory and non-directory cases.
- Web gate: `npm run check` (svelte-check, zero errors), `npx vitest run` (2090 tests, including a new `openLinkTarget` directory-routing test), `npm run build`.
- Browser verification against a throwaway workspace seeded with every list type at long wrapping widths: measured text-column deltas of zero per depth; click-to-edit, undo, Enter continuation, checkbox toggle, Tab and Shift-Tab all correct; copy and paste round-trips preserved the source marker and whitespace; copying a row containing an inline image preserved both the image markdown and the text; the directory link rendered valid and opened the file browser at the folder, while the file link opened the file and the missing link stayed broken.

## Highlights

- The right model for the hang-indent was render-only whitespace hiding plus static CSS. Once we landed on it, every measured delta was zero and there was no timing or animation-frame fragility to babysit. Keeping the document untouched means copy, paste, and save all round-trip for free.
- The double-marker fix was small because the machinery already existed. The investigation found `dedentListPaste` already solving the case for rich paste, so the plain-text fix was about thirty lines of wiring plus a reused, already-tested helper, not a new subsystem.
- The directory-link fix stayed isolated. The relative-path resolution was already done frontend-side, the wire change was one optional flag, and the click routing was one branch in one function, so the blast radius was tiny and `cargo check --workspace` confirmed nothing downstream broke.
- Browser verification earned its keep. The static gate is genuinely blind to CodeMirror runtime layout, and hands-on checking caught the real regressions (checkbox position, double-digit marker wrap, the nested-depth gap) that svelte-check and the source-pinned vitest could never see.
- Working in small verifiable steps paid off. "Do one case before you do all" stopped a wrong approach from shipping wide, and "go back to the known-good state, then switch ideas" reset cleanly instead of layering patches on a failing design.

## Lowlights

- The hang-indent took two dead ends before the right model. The first attempt, a fixed five-character CSS hang with no whitespace hiding, left wrapped lines a few pixels short of the text and moved the hyphen lists off their column, and Alex rightly rejected it. The second attempt, a plugin that measured each line and set a per-line hang, failed for an environmental reason rather than a logic bug: `requestMeasure` is gated on an animation frame, the automation tab runs hidden so frames never fire, and a `setTimeout` fallback measured during layout churn and produced wrong sticky values, including a hyphen outlier off by more than forty pixels. That environmental failure mode looked like a code bug at first and cost real cycles before it was abandoned.
- The fix introduced its own regressions that only showed up on screen. Inherited `text-indent` leaked into the marker column and pushed the checkbox out as a stray bracket in the gutter; fixing that with `text-indent: 0` then let a two-character marker like "10." wrap inside its box until a `white-space: nowrap` pinned it. Each was invisible to the gate and surfaced only by looking.
- The first "done" was premature. The hang fix was reported working while it still only covered depth zero; the nested items had no hang, and that gap surfaced because Alex clicked around and noticed. A webdev-review pass then formalized the catch and also flagged that nested tasks were skipping the hang decoration entirely. The review should have come before the report, not after.
- Test hygiene added avoidable friction. The throwaway note literally contained the phrase "not a text file," which false-positived a naive toast check; the SPA re-persists its session and kept overriding deep-links until the stored key was cleared; and the hidden tab plus device-pixel scaling made click coordinates unreliable, so a synthetic Cmd-mousedown on the pill was the dependable way to exercise the open gesture. None of these were product bugs, but each one briefly read like one.

## What worked between us

The collaboration tightened as it went. Early on the loop was long: try a whole approach, show it, get a rejection, retry. It got much faster once the cadence became "one case, then all," "reset to known-good before switching," and "review before declaring done." The scope also grew in a healthy direction: what began as two list bugs turned into a validated pass over selection, copy and paste, image rows, and keyboard list behavior, and that sweep is what surfaced the double-marker annoyance, which then shipped as its own feature. Decisive direction from Alex (clear rejections with the actual reason, plus "proceed" once a step landed green) kept momentum without over-checkpointing.

## Still open

- Progressive outdent: pressing Enter on an empty nested item currently exits the list in one press rather than outdenting a level at a time. Flagged as an optional keymap tweak, not done.
- Bug 2 (setext flash) was dropped by decision, and the mermaid-to-excalidraw feature is deferred to a later v0.59.0 cut.
- The three commits are local on `editor-fixes`. Nothing has been pushed, merged, or tagged.
