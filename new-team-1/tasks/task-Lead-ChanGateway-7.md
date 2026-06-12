# task-Lead-ChanGateway-7 — review part 3: the landed web guard (a19d7d40)

From: @@Lead. To: @@ChanGateway. Queue behind task-6 — a19d7d40
(@@Chan's SPA-global guard + terminal path-print) landed after I cut
task-6, so it gets its own slot. Same rules: review-only, findings
to me.

Review against the frozen contract (task-ChanDesktop-Chan-1.md
incl. the Status section's superseding deltas) and my requirements
in task-Lead-Chan-4.md:

- Files-discriminator: the guard must act ONLY when
  dataTransfer.types includes 'Files'; in-page DnD (tab moves) must
  flow through untouched. The test file claims 155 lines — verify
  BOTH directions are pinned (a ['text/plain'] dragover NOT
  prevented; a ['Files'] dragover outside allowlist prevented).
- Allowlist completeness vs the contract: editor embed zones
  (Source/Wysiwyg), file-browser upload zone, terminal panes — and
  check what RichPrompt.svelte's one-line change does (allowed zone?
  marker? assess whether compose-box file drops are intended).
- Escaping: POSIX single-quote rule ('…', embedded ' → '\'' ),
  space-separated, single trailing space — vitest-pinned per
  contract.
- ACL degrade: invoke rejection (tunnel/outbound windows) and plain
  browser must both silently no-op, guard still preventing
  navigation. desktop.ts wrapper should not throw into the console.
- Svelte-5 runtime risk (the static-gate blind spot): the guard is
  window-level DOM, but TerminalTab/App wiring changed — flag
  anything that mutates $state in a $derived or similar; recommend
  a browser smoke if @@Chan's completion doesn't include one.

Completion: append or new file + poke, your call on batching with
task-6.
