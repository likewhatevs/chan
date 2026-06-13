# @@Alex hand-smoke — round 1 close (shrunk by the instrumented walks)

Binary: `/tmp/chan-desktop-gate-target/debug/chan-desktop`
(sha 8b64ec7d, clean — no instrumentation, pristine SPA). Built from
final HEAD b82a0a27. Your real HOME/workspaces are fine.

Already machine-proven — do NOT re-check: item-1 keep-alive incl.
deep-scroll + session-restore caret + undo boundary; item-2 wire
semantics (18/18), busy-submit visuals, queued chip, flipped pill;
runtime console clean (composited); B6 GTK menus; fit-loop = benign
(asleep-display-only); item-6 logic (36/36 instrumented).

Each item ~30s. Sources: @@Editor's specs
(round-1-walk-editor-assertion-specs.md), @@PromptQueue's recipes
(task-PromptQueue-Conductor-28), B5 note, task-Desktop-Conductor-38.

1. **Item 4 — the one human-click check** (synthetic events skip the
   buggy default action, so only a real click proves it): click a
   terminal tab → type immediately (keys land in the terminal);
   with a rich-prompt bubble open, click that terminal's tab →
   caret stays in the bubble.
2. **Item 2 — dynamic remainder** on a busy terminal
   (`while true; do date; sleep 0.3; done`):
   - 3 × `cs terminal write` → tab pill climbs 2/3/4;
   - Ctrl-C the loop → queue drains, your pending prompt clears
     exactly when its message prints;
   - reload mid-pending → draft text restored, pill re-syncs;
   - idle terminal submit → no chip flash;
   - (optional) hide the tab mid-pending, let it deliver, reshow →
     composer cleared.
3. **Tab DnD** — reorder within a pane + drag across panes (mouseup
   fix risk surface). **OS-file drop** — into the active editor OK,
   non-zones inert.
4. **Item 6 — pixel/hit pass**: Open-button click feel, failure
   dialog rendering (hold the flock via a second `chan serve` if you
   want the dialog), pill consistency.
   **B5 — 30s**: close (bury) a workspace window → Window menu reads
   "Hidden Windows (1, kept warm in memory)"; with one buried, the
   11th window still opens.
5. **Item 5A** — on a live survey: 1..N pick, F follow-up, X dismiss
   from the keyboard. **Item 3** — bootstrap a throwaway team →
   every broadcast toggle OFF.
6. **Optional observation** (pre-existing, not round scope, for
   @@Editor's judgment): Cmd+. pane-mode round-trip resets editor
   scroll to top (flip alone preserves it). Note only if it bothers
   you in practice.

Reply on the survey: pick "All clean" or "Issues found" (use F for
detail — it papers a followup file automatically).
