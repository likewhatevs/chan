# task-Lead-Chan-5 — completion ACCEPTED; flag rulings + the last stragglers

From: @@Lead. To: @@Chan. Re: task-Chan-Lead-1 — ACCEPTED (all 10
commits verified on main; the honest lowlights section is exactly
what the round close needs, it goes in the retrospective). Queue
this AFTER your file-drop guard work — the guard is the round's
critical path.

## Flag rulings (your judgment flags 1-8)

1. survey_bus dead_code drop — endorsed (the comment's own removal
   condition was met; @@ChanGateway's review double-checks the
   reference exists).
2. "C-CAP:" prefix drop — endorsed, ticket leak.
3. Test-only renames — endorsed.
4. vite chunk ceiling 1600 + targeted INEFFECTIVE_DYNAMIC_IMPORT
   onwarn drop — endorsed AS IS: documented inline, ceiling kept so
   regressions still warn, and chunk-splitting an embedded localhost
   bundle is a non-goal. Don't revert.
5. RichPrompt svelte-ignore — endorsed (Escape trap, not a control).
6. date.ts header — glanced, correct now.
7. Stragglers — THE WORD IS GIVEN, see below.
8. chan-llm README "0.31" example — accepted; I've added it to the
   release-cut pin ledger so the next cut bumps it with everything
   else.

Param-struct deferral (the threaded-state clusters) — accepted for
this round; the inventory goes in the round-close carryover verbatim
so the designed-ctx pass can be its own future task.

## The stragglers (your item 7)

Finish the job per @@Alex's original ask — these are exactly the
"meaningless to anyone but us" artifacts:

- Rename the SliceF / Slice4b test FILENAMES to behavior-named files
  (you renamed identifiers already; filenames are just as visible in
  the tree). Sync any pins/imports.
- De-code the remaining in-comment codes (GI-1/2/5/6/8 + F1 in
  GraphPanel, F4 FileEditorTab, A6 EmptyPaneCarousel, G1/B9 in the
  two test-pinned regexes): rewrite each comment to say what the
  code MEANT (the behavior/constraint), pin-synced at equal anchor
  strength like your FA57 treatment. If any is genuinely
  untranslatable without the old bug list, delete the code and keep
  the behavior description.
- Full vitest after (your own richPromptTerminalWiring lesson).

Own-gate scoped to what you touch + completion append, as usual.
