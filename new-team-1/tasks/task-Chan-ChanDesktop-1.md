# task-Chan-ChanDesktop-1 — file-drop joint spec: ACK with one refinement

From: @@Chan. To: @@ChanDesktop. Re: task-ChanDesktop-Chan-1.

## ACK — Design C is right

Your wry/tauri-runtime-wry source pin settles it: with the native
handler enabled WebKit never sees ANY drag (the closure returns true
unconditionally), so Option A's premise is false and Option B breaks
in-page tab moves. `.disable_drag_drop_handler()` stays; DOM guard +
`read_dropped_paths` IPC. I accept the contract as written:

- `read_dropped_paths() -> Vec<String>`, absolute paths, pasteboard
  order, raw (no file:// scheme, unescaped), `[]` on no file items,
  `[]` unconditionally off macOS, only meaningful inside a `drop`
  handler, ACL `allow-read-dropped-paths` on the workspace-window set.
- Shell-escaping is mine, web-side, vitest-covered: POSIX single-quote
  wrap, embedded `'` -> `'\''`, space-separated, single trailing
  space. `[]` -> silent no-op.
- Plain browser: terminal drop no-op (window.__TAURI__ detection).
- Linux terminal drop no-op for now — agreed, your flag to @@Lead.

## The refinement (answers your dropEffect question)

The global guard will fire ONLY for OS file drags:

```
if (!e.dataTransfer?.types?.includes("Files")) return;
```

window-level CAPTURE-phase `dragover`/`drop`, preventDefault outside
the allowlisted zones, no stopPropagation inside them. In-page HTML5
drags (pane tab moves) never carry the `Files` type, so the guard
never preventDefaults their dragover and their dropEffect/cursor
semantics are byte-identical to today — the interaction you asked me
to check cannot occur. This also keeps the guard inert for text
selections dragged within the page.

(Without the type gate, a global dragover preventDefault would mark
the whole window a valid drop target for tab drags — cursor feedback
lies, drops fire where nothing consumes them. With it, no overlap.)

## Sequencing

1. I land the SPA-global guard first (ships independently; kills the
   takeover data-loss in browser AND desktop).
2. You implement `read_dropped_paths` + ACL whenever ready — the
   contract above is frozen from my side as of this file.
3. I wire the terminal-pane drop handler against the contract
   (preventDefault -> invoke -> escape -> existing terminal write
   path; insert at cursor, multiple files space-separated).
4. My verification: vitest (guard gating incl. the Files-type gate,
   zone allowlist, escaping) + Chrome smoke (drop on graph/search
   inert, editor embed + FB upload + tab moves unaffected). Yours:
   desktop gate + ACL pins; @@Alex hand-smokes the WKWebView arc.

I am closing out the round-1 tidy task first (own-gate running);
guard work starts right after. No further blockers from my side.

## Addendum (per task-Lead-Chan-4 — @@Lead's approval amendments)

@@Lead approved design C with two amendments; my ack incorporates
both. Supersedes one line above:

1. Files discriminator: already in this ack (the refinement section).
   Per @@Lead it also gets explicit vitest coverage: a synthetic
   dragover with types ['text/plain'] must NOT be prevented; one with
   ['Files'] outside an allowlisted zone must.
2. ACL scope CHANGE: `allow-read-dropped-paths` goes on the
   locally-served window kinds only (workspace-*, terminal-*), NOT
   tunnel-*/outbound-* — a remote-served SPA must not be able to
   harvest the system drag pasteboard (task-Lead-ChanDesktop-6.md
   amendment 1). My earlier "workspace-window set / every SPA window
   kind" sentence is superseded. My terminal drop handler degrades
   gracefully: invoke succeeds -> escape + insert; invoke rejected
   (ACL) or plain browser -> silent no-op with the guard still
   preventing the takeover. No window-kind special-casing in the SPA;
   the ACL is the source of truth.

Contract otherwise frozen as acked. Building order unchanged: guard
first, then the terminal path-print against your IPC.
