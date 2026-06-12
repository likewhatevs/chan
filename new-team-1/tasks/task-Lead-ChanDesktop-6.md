# task-Lead-ChanDesktop-6 — design C review: ACCEPTED with two amendments

From: @@Lead. To: @@ChanDesktop. Re: task-ChanDesktop-Chan-1.md.

The wry-source verification is exactly the right depth — it kills my
decision tree's premise cleanly (handler returns `true`
unconditionally → WebKit never sees any drag, in-page DnD included)
and explains why disable_drag_drop_handler exists. Design C
(DOM guard + pasteboard-read IPC) is APPROVED as the mechanism, with
two amendments before you implement:

## Amendment 1 (security, contract change): scope the ACL to
locally-served windows only

`allow-read-dropped-paths` must NOT extend to `tunnel-*` /
`outbound-*` windows. Those webviews render an SPA served by the
REMOTE host; the macOS `.drag` pasteboard is system-wide and holds
the user's most recent drag from ANY app, persisting after the drag
ends. A compromised or malicious remote could poll the command from
its SPA and harvest local file paths the user drags around in other
applications. Scope the permission to the locally-served kinds
(`workspace-*`, `terminal-*`). Consequence on remote windows: the
guard still prevents the takeover; no path is printed — which is
correct anyway, since a local path is meaningless to the remote PTY.
Pin the scoping in the serve.rs include_str! contract tests like the
rest of the capability shape.

## Amendment 2 (implementation): read the pasteboard on the main
thread

NSPasteboard is AppKit; do the read via the app's main-thread
runner rather than the command's worker thread.

Everything else in the contract stands as proposed: raw unescaped
paths from the IPC, escaping web-side, `[]` for no-file drags and on
Linux (no-op accepted — flag stays in the round close for a future
Linux equivalent).

@@Chan is being told to ack WITH these amendments plus a
guard-discriminator requirement on their side (only intercept drags
whose DataTransfer carries Files, so tab-move DnD is untouched).
Proceed on their ack.
