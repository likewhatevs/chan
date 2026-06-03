# task-LaneA-LaneB-5: B4 - cs pane split/close (UNBLOCKED)

From: @@LaneA  To: @@LaneB  Wave: 2 (after the cs-surface; B4 now UNBLOCKED)

@@LaneD's B5 chan-server burst landed green and touched control_socket.rs
ONLY in the spawn_team region (~702) - the pane-exec region (~102) you need is
clear. The serialization held. Do B4 after you finish the cs-surface (B-4).

## B4 spec (from bootstrap; re-verify lines against HEAD)

1. `cs pane split` directions must be RIGHT and BOTTOM (match the hybrid pane
   hamburger, Pane.svelte ~484-500). chan-shell SplitDirArg is Left|Bottom today
   (cli.rs ~188-202) -> align to Right|Bottom.
2. One-shot `cs pane` commands must NOT enter hybrid-nav transaction mode
   (tabs.svelte.ts enterPaneMode vs enterPaneModeTransaction ~2353-2376) and must
   NOT steal focus from the SENDING terminal UNLESS the command's purpose is to
   change pane/tab focus (paneModeSplit hardcodes activePaneId to the new pane
   ~2618-2631). Fixes @@Alex's "stuck in transaction mode" + "lost focus after
   split/close" report.

## Files (your lane)

crates/chan-shell/src/cli.rs (SplitDirArg), web/src/state/tabs.svelte.ts (pane
region ~2353/2618 - yours), web/src/components/Pane.svelte,
crates/chan-server/src/control_socket.rs (pane-exec region ~102 - now clear).
Keep the pane region edits away from @@LaneC's saveDraft (~2085) + the
prompt-sink (~1428) you already touched.

## Gate

- cargo fmt --check + cargo clippy -p chan-shell -p chan-server --all-targets
  -D warnings + cargo test (-p chan-shell -p chan-server).
- make web-check + svelte-check + npm run build.
- Browser-smoke: `cs pane split` -> RIGHT + BOTTOM options; a one-shot
  `cs pane split`/`cs pane close` from a terminal does NOT enter transaction
  mode and does NOT move focus off the sending terminal (unless the command
  targets focus). rust-embed: npm build before cargo build; smoke the served
  bundle.

## Report

Cut task-LaneB-LaneA-N (summary + own-gate-green + pathspec shas) + poke. This is
your last round-1 item; round-2 R2-3 (per-terminal survey) comes after.
