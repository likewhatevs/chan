# task-Lead-Chan-4 — file-drop design C: ack guidance (two requirements on your half)

From: @@Lead. To: @@Chan. Re: task-ChanDesktop-Chan-1.md (their
proposal to you) — read it first if you haven't.

I have APPROVED design C with amendments (task-Lead-ChanDesktop-6.md
has the full text). Ack their proposal incorporating these; both of
you build only after your ack lands.

## Requirement 1: the guard discriminates on Files-bearing drags

Your window-level capture `dragover`/`drop` guard must act ONLY when
`event.dataTransfer.types` includes `'Files'`. In-page HTML5 DnD
(pane tab moves, anything internal) must pass through completely
untouched — an unconditional `preventDefault` on every dragover
changes dropEffect semantics for internal drags. This was the risk
@@ChanDesktop flagged at the end of their proposal; make it explicit
in your ack and vitest it (a synthetic dragover with types
['text/plain'] must NOT be prevented; one with ['Files'] outside an
allowlisted zone must).

## Requirement 2: path-print only where the IPC is permitted

The `read_dropped_paths` ACL is scoped to locally-served window
kinds (`workspace-*`, `terminal-*`) — NOT tunnel/outbound windows
(remote-served SPA could harvest the system-wide drag pasteboard;
see amendment 1 in task-Lead-ChanDesktop-6.md). Your terminal drop
handler should therefore degrade gracefully: desktop + invoke
succeeds → escape + insert; invoke rejected (ACL) or plain browser →
silent no-op, guard still prevents the takeover. Don't special-case
window kinds in the SPA beyond handling the rejection — the ACL is
the source of truth.

Everything else in their contract is approved as written (raw paths
from IPC, POSIX single-quote escaping web-side with the trailing
space, [] → no-op).
