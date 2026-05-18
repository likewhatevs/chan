# frontend-7: investigate markdown editor trailing buffer

Owner: @@Frontend
Status: REVIEW

## Goal

Investigate and (if reproducible) fix a rendering glitch Alex saw
in the markdown WYSIWYG editor where extra trailing content
appears below the actual document end. The plain-text / source-
code view of the same file renders correctly, so the issue is
specific to the WYSIWYG path.

## Symptom

* WYSIWYG view shows a strange trailing buffer below the document
  content (bullet markers and text continue past the document
  end).
* Source-code view of the same file renders correctly with a
  clean end.
* Not reliably reproducible at report time. Alex saw it once and
  could not pin a repeat.

## Suspects to check

* Tiptap / ProseMirror node-view caching: a previously displayed
  document tail that did not get torn down when the editor swapped
  documents (tab switch, file reload, pane-restore from hash).
* Markdown-to-HTML conversion writing a trailing block that
  ProseMirror retains after a partial replacement.
* CSS / layout: a fixed-min-height container leaving phantom rows
  visible.
* `web/src/editor/` (the editor component tree) and any
  document-swap path: `setContent` vs `replaceWith` ordering, or
  a missed `editor.commands.clearContent` before swap.
* Restore-from-hash code path that re-applies the same document
  on tab focus.

## Investigation prompts

1. Try to reproduce: open a markdown file, switch tabs to another
   markdown file, switch back, switch view to source, switch back
   to WYSIWYG. Try on a long document and a short one.
2. Diff editor state before and after a suspected trigger; capture
   ProseMirror's `state.doc` JSON.
3. If reproducible, narrow to the smallest trigger sequence and
   add a regression test in `web/src/editor/`.
4. If not reproducible after a focused session, record the
   investigation outcome in this task with the steps tried.
   Either land a defensive `clearContent` before document swap if
   the cost is low and the fix is plausible, or close as
   "unreproducible, will revisit on report".

## Relevant links

* Journal: [journal.md](./journal.md).
* Editor source: `web/src/editor/`.

## Acceptance criteria

* Repro recipe documented in this task, or
  "unreproducible after N minutes of attempts" with the attempts
  listed.
* If reproducible: fix lands with a regression test.
* If not reproducible: defensive `clearContent` (or equivalent) on
  document swap optional; record decision either way.

## Tests

* If a fix lands: vitest covering the swap path or a focused
  Tiptap fixture.
* `npm --prefix web run check`, `npm --prefix web test -- --run`,
  `npm --prefix web run build` green.

## Progress notes

* 2026-05-18 Alex reported the symptom with screenshots; not
  reproducible at report time.
* Code inspection note: the production WYSIWYG surface is CodeMirror
  6, not Tiptap/ProseMirror. Decoration plugins recompute on
  `docChanged`, so stale syntax decorations are unlikely after a
  normal external content replace.
* Plausible stale-state path: `FileEditorTab` can be reused across
  active file-tab switches, which also reuses nested editor-local
  state. Added a conservative `{#key tab.id}` around the editor body
  so WYSIWYG/source widgets, bubbles, and local editor refs are torn
  down when switching file tabs.
* Could not reproduce from code-only inspection; this is a defensive
  lifecycle fix. Webtest should try the long-doc/short-doc WYSIWYG
  tab-switch sequence from the investigation prompts.

## Completion notes

Verification:
* `npm --prefix web run check` passed.
* `npm --prefix web test -- --run` passed: 19 files, 185 tests.
* `npm --prefix web run build` passed with existing Vite warnings.
* Webtest round 4 passed the long/short markdown tab-switch probe:
  no trailing WYSIWYG buffer observed.
