# task-Lead-Chan-2 — addendum: extended patterns, workspace correction, confirmed stale items

From: @@Lead. To: @@Chan. Extends task-Lead-Chan-1 (do both in one
pass; this corrects and widens it).

## Correction: desktop/src-tauri IS a root-workspace member

round-1-plan.md said desktop/ is a separate workspace — wrong, only
gateway/ is. `desktop/src-tauri` (crate `chan-desktop`) is a member
of the root workspace, so:

- Scope your warning sweep with
  `cargo clippy --workspace --exclude chan-desktop --all-targets`.
  Desktop warnings belong to @@ChanDesktop; do NOT fix them, or you
  two will collide.
- `cargo test --workspace` compiling chan-desktop is fine; just
  don't edit under desktop/ except mechanical call-site fixes riding
  your signature commits (unchanged from task 1).

## Extended archaeology patterns (recon under-counted)

My pattern list missed several handle/round vocabularies. Sweep
crates/ + web/ again with:

```
grep -rniE '(systacean|desktacean|desktest|@@(Host|CI|Architect|Lane|FullStack|Webtest)|round-[0-9]+|wave-[0-9]+|slice [a-z0-9]+|\b-a-[0-9]+\b|track [ab]\b)'
```

Known concentrations (all confirmed by my own reads, all yours):

- USER-VISIBLE `--help` text in crates/chan/src/main.rs: the `Add`,
  `Index`, and `Reports` variants' doc comments cite `systacean-27`,
  `systacean-7`, "Round-2's lean-workspace policy". Clap doc comments
  ARE the help output — rewrite them to state the behavior only.
- Route doc comments: routes/index.rs ("systacean-7:"),
  mentions.rs ("systacean-35:"), reports_toggle.rs, screensaver.rs
  ("systacean-39/40"), fonts.rs ("fullstack-b-30 slice b"),
  excluded_dirs.rs ("round-1 wave-3, @@Host"), survey.rs ("the
  @@LaneC side").
- chan-workspace/src/index/config.rs: IndexConfig field docs cite
  systacean-7/27/40, fullstack-a-21/-a-99, "per `-a-77`", "(Round-3
  may refactor...)".
- crates/chan/src/main.rs:2513: "Phase-3 renamed `tight` ->
  `compact`".

## Confirmed stale help text (fix with the scrub)

`chan add --reports` help claims reports are "Off by default", but
`IndexConfig::default()` sets `reports_enabled: true` for NEW
workspaces (legacy files omitting the field stay false — see the
test `reports_enabled_defaults_true_for_new_workspace_but_legacy_
file_stays_false`). Make the help text match reality. Check the
`--semantic-search` help and the `Reports` subcommand help for the
same claim while you're there (semantic IS off by default; reports
is not).

## FYI

I rewrote docs/config-reference.md tables against your structs
(ServerConfig terminal.font/mcp_env, EditorPrefs
hybrid_surface_themes, IndexConfig excluded_dirs + screensaver
block, TeamConfig tab_group/mcp_env, KnownWorkspace). If your
hygiene pass changes any persisted field, ping me so the reference
stays current.

Completion: fold into your task-1 completion file.
