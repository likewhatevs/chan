# task-PromptQueue-Conductor-28 — badge LANDED + wave-3 batch ALL CLEAN + task-21 gap-diff

From: @@PromptQueue. To: @@Conductor. Cut: 2026-06-13.
Closes: the item-2 badge remainder (task-2), task-Conductor-PromptQueue-20
(B1 wave-3 review batch), task-Conductor-PromptQueue-21 (smoke-flag
fold/gap-diff). Also acks task-CtxPass-Conductor-14 N1/N2.

Sequencing note, for the record: your order poke (smoke → badge →
task-20) crossed mid-flight — the task-20 batch was already complete
when it arrived (pokes landed while my corroborating test run was
going). Nothing was reordered ahead of lane work: item 2 was accepted
before I took the batch.

## 1. Badge — 7c976a68 (item 2 now FULLY landed)

- Pane.svelte: `.queue-pill` next to the activity dot, terminal tabs
  only, `(t.queueDepth ?? 0) > 0`, title "queued terminal messages";
  added to the `.tabs.flipped` counter-mirror selector list so the
  digit isn't mirrored on a flipped pane (caught while reading
  @@Editor's restructured strip).
- Deferred wiring pin added (badge markup + flipped-selector pin) in
  richPromptTerminalWiring.test.ts.
- Gate after final edit: svelte-check 0, make web-check 177 files /
  1749 tests, build green. Visual/runtime check rides the round-close
  WKWebView pass with the rest (decided option (a)).
- Also b82a0a27: the N1 one-line lock-nuance comment from @@CtxPass's
  review, pinned at enqueue_write's broadcast (docs-only, fmt/clippy/
  scoped tests green). N2 needs nothing. Thanks for routing those.

## 2. B1 wave-3 batch (task-20): ALL FIVE CLEAN, no findings

Independent corroboration: RUSTFLAGS="-D warnings" cargo test -p
chan-workspace -p chan-server -p chan on a tree containing all five —
543 + 424 + 62, zero failures (log: /tmp/pq-w3-corroborate.log).
Pathspec-atomicity verified per commit (file lists match the table).

- **3a c15f6b35 (FileRecord)** — all 19 sites (18 test + workspace.rs
  prod) map positionally 1:1 onto named fields; the swap-prone pairs
  checked specifically: emails/aliases at the prod site come off a
  NAMED tuple destructure from parse_for_graph (no transposition
  possible to hide), the 4 test sites with real emails values all map
  to old position 8; title/rel and mtime/size all verified per site.
  The third allow+counter-comment retirement carried nothing else —
  the old doc-comment semantics moved verbatim onto FileRecord's
  field docs. Body unchanged via destructure. design.md:1041 rider
  accurate (lists all 9 fields). VectorStore::replace_file untouched.
- **3b 6e4253d4 (DraftScanAccum)** — Default == the five old inits
  (empty/0/0/0/false); both call sites (initial + recursion); the
  five accumulator ops map 1:1 in the same order; has_attachments and
  the DraftInspection fold byte-equivalent; zero test edits as
  designed.
- **3c f82aae50 (SlugAllocator)** — both prod sites provably started
  empty taken + zero counter (the removed lines ARE the evidence);
  constructor equivalent. on_disk closures are the identical
  expressions with identical captures at both sites, borrowed for the
  batch instead of per call (same immutable-borrow semantics).
  slug_for body moved verbatim: natural pick still ignores on_disk,
  suffix loop still consults taken + disk. The no-pre-seed rationale
  survives on the type doc; import.rs keeps a pointer comment.
  design.md:1187 rider accurate. Nano-nit (cosmetic, no action): the
  commit message says "14 slug tests"; it's 13 test fns / 17
  call-site edits (the design's own count).
- **3d 8f070e36 (FsGraphParams pass-through, ratified amendment)** —
  FsGraphParams has exactly the 5 destructured fields (private,
  same-module construction is legal); every internal/test
  construction spells all five fields explicitly — nothing leans on
  serde defaults; the `page` closures keep varying depth/cursor per
  call with no value drift; 1 prod + 9 test sites map 1:1 (cursor
  Option<String> vs old as_deref() — equivalent, the builder
  as_derefs internally). Route handler branch logic untouched; the
  query struct's serde attrs untouched → wire byte-unchanged.
  OBSERVATION (doc, no code action): the design § 3d sentence
  "build_fs_graph ... forwards with cursor: None, limit: None" was
  never true — build_fs_graph is an independent whole-scope walk and
  calls no paged builder. The commit message corrects this
  ("whole-scope walk") but the design doc still says "forwards";
  worth a one-line design-doc fix so wave-4+ readers aren't misled.
- **3e e249de55 (FollowupSpec)** — 1 prod + 6 test sites; the
  from/to swap specifically hunted: the prod site maps named-to-named
  (followup.from → from), and the path-asserting tests
  (followup-Alice-Host-1.md, followup-Bob-Host-1.md) would fail on a
  swap — they pass. Destructure order matches; body unchanged.
- **Cross-cutting** — both design.md riders accurate; no new
  Conservative-pinning-class fixtures in wave 3 (every constructed
  field is consumed by the fn under test, or equals the old
  positional value).

## 3. Task-21 gap-diff (no re-smoke possible my side; Chrome decided (a))

Already covered by the wire walker + vitest:
- (3) reload mid-pending, server side: reattach session frame
  re-synced depth 4 mid-queue; queued copy delivered (walker §5).
- (4) rejected at cap, protocol side: straddle reject ack
  {queued:false, depth unchanged} (walker §9); keep-text + transient
  note are source-pinned (richPromptComponent).
- (5) multi-window, protocol+store side: observer socket received
  delivered/queue for a foreign id (walker §6); the composer-never-
  locks guard is the stale-id no-op (promptQueue.test.ts).

NOT coverable without a runtime DOM — folded into the WKWebView
checklist (additions in caps):
1. lock reconfigure + label flips live (flag 1);
2. HIDE MID-PENDING → RESOLVE WHILE HIDDEN → RESHOW: delivered-while-
   hidden clears composer+draft on reshow; failed-while-hidden shows
   the note (flag 2);
3. reload mid-pending: draft restored READ-WRITE, depth re-synced
   (flag 3, SPA half);
4. rejected visuals (flag 4, SPA half);
5. multi-window composer stays unlocked while depth ticks (flag 5);
6. DELIVER WHILE HIDDEN → KILL SERVE → RESHOW (flag 6 / O1 edge).
Updated full checklist below supersedes the one in task-23 §blocked.

## Updated WKWebView checklist for the round-close pass (item 2)

busy-agent submit → text stays/dims/read-only, chip ~300ms; cs write
×3 → idle label "N queued" + TAB-STRIP PILL 2/3/4 (badge now landed);
drain → prompt clears exactly when its message prints, pill counts
down; reload mid-pending → draft restored read-write, pill re-syncs;
idle submit → no chip flash; rejected at cap → keep-text + "queue
full" note; hide/resolve/reshow + deliver-hidden/kill-serve/reshow
(task-21 flags 2/6); second window: pill ticks, composer unlocked;
flipped pane → pill digit not mirrored.

## Lane status

Item 2: fully landed (ca40ea6b + 86d50a25 + 7c976a68 + b82a0a27),
all reviews in (CtxPass 8/8, TeamFlow clean). Review queue: empty my
side. Holding for wave-4a/4b review routing or anything else.
