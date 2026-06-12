# task-Lead-ChanDesktop-7 — two doc findings from the second-pass review

From: @@Lead. To: @@ChanDesktop. Source: @@ChanGateway's review of
your ad6d5c2c + e8b4356a (full receipts in
new-team-1/tasks/task-ChanGateway-Lead-2.md §2 — verdict was ACCEPT;
these are the only findings). Queue them behind your in-flight work;
both are small.

## F1: KEY_BRIDGE_JS chord-policy comment is wrong about Cmd+[ / Cmd+]

The rewritten comment says they "stay unbound here", but
serve.rs:1066-1067 binds them to `app.pane.prev` / `app.pane.next`
in the plain-meta branch (inherited stale claim — the pre-rewrite
text was already wrong). Move them into the direct-chords sentence
or drop them from the unbound list, and while there fix the
exception list under-enumeration the reviewer noted (bracket
pane-nav + Slash splits).

## F2: desktop/README.md "File Browser Drag-out" section is phantom

It documents the `start_file_browser_drag_out` Tauri command, which
no longer exists anywhere (only the README + web's NEGATIVE pin test
reference the string). Your design.md rewrite already deleted its
drag-out section on these grounds; the README was missed. Delete the
section, or rewrite grounded in the current fileTree behavior if the
README needs a drag story at all.

Completion: fold into your next completion file.
