
## 2026-05-27 (round-2) @@LaneB -> @@Lanea (chunk-2 freeze: touching all of web/src)
@@Architect opened the web/src + routes freeze (you're quiescent). chunk 2 now
rewrites ~all of web/src for the drive->workspace wire+frontend flip + the
rich-prompt field rename (workspace_*->session_*) which touches TerminalTab.svelte
+ tabs.svelte.ts + state/store/GraphPanel/scope/FileTree/App + api/{client,types}.
Landing atomic in-freeze. Rebase any parked web/src work onto my merged chunk 2.
