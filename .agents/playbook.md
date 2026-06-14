# Agent playbook

The operational lessons a new agent needs before joining a chan multi-agent phase. Distilled from the phase retrospectives in [`../docs/phases/`](../docs/phases/); each lesson cites the phase that taught it so you can read the full story there. This is the "how we actually work and what went wrong when we did not" companion to the public process doc [`../docs/coordination.md`](../docs/coordination.md) and the per-role contact cards in [`roster/README.md`](roster/README.md).

## The coordination model (and how it evolved)

The scheme changed over the project; read the phase you are in to know which one is live. The trajectory:

```
phases  coordination scheme
------  ---------------------------------------------------------------
1-6     flat task files at the phase root + one shared journal.
7       the model later phases refined: one directory per author,
        append-only dated journals, typed event-channel files, and
        architect-orchestrated dispatch.
8       a second team (chan-desktop) runs the same model in parallel;
        @@Alex bridges the two architect leads.
11+     lanes work in per-lane git worktrees for CODE while the
        coordination documents stay in one shared, append-only bus.
15+     an isolated gate worktree gates the COMMITTED state; the team
        bus moves into the workspace and runs over cs-terminal tooling
        (pokes, surveys) instead of a fsnotify watcher.
```

The phase-13 r2 Team Work revamp REMOVED the fsnotify event-watcher / poke-dispatch / notification-bubble layer; the [`orchestration/`](orchestration/) contracts are retained as the blueprint for its planned return, not a description of current runtime.

## Coordination discipline

- Append-only, even in coordination. Once an agent has started a task, new asks become NEW tasks, not amendments to the one in flight. Rewriting under someone loses the audit trail. (phase 7)
- Lean poke bus. A poke is a one-line pointer ("read <path>"); the context lives in the on-disk task file it points to. Fat pokes stack, truncate, and bury the substance. (phase 16)
- Every poke to an agent CLI must end with the agent's submit chord (`--submit=<agent>`), or it parks unsubmitted in the compose box and stalls the round. (phase 15)
- Workers route decisions to the lead/architect; the lead consolidates and surveys @@Alex with `cs terminal survey` (a blocking overlay in the host's window). Workers do not survey the host directly, and do not use a TUI survey. (phases 15, 16)
- Verify a write landed BEFORE you poke about it: grep or Read the file, then poke. Never bundle a heredoc-write plus a poke plus a grep into one shell command; truncation silently drops the later steps. (phase 8)
- Redistribute spillover from the queue TAIL, not the head: the lane is already working its next-up item. (phase 7)
- Status reports are curated highlights / lowlights / contention, not full tabular dumps; the detail stays in the task files. (phase 7+)

## Git and commit discipline in a shared worktree

- The only race-proof commit when peers stage concurrently is an explicit pathspec: `git commit -F msg -- <path> <path>` (flags BEFORE the `--`). A plain `git add` + `git commit`, even chained with `&&`, still lets a peer's concurrent staging contaminate your commit. (phase 8)
- `git add <single-path>` does NOT unstage other files. Run `git diff --staged --stat` before committing and `git show --stat HEAD` after, every time, in the multi-agent tree. (phase 8)
- Merge is not push. "Merge to main" means a local merge only; never push without an explicit ask from @@Alex. A standing commit clearance is not a standing push clearance. (phases 8, 12)
- A backgrounded gated push SIGPIPEs (exit 141) and silently fails to update the remote (the pre-push hook emits ~90KB over ~3 min). Push in the foreground, redirect to a file, and verify with `git ls-remote` before tagging. `--no-verify` is classifier-blocked. (phase 15)

## The gate and the quality bar

- Run a scoped own-gate before any "done" report. Rust: `cargo fmt --check` + `cargo clippy --all-targets -D warnings` + `cargo test` (scoped `-p <crate>`; re-check `--no-default-features` if you touched feature gates). Frontend: `make web-check` (vitest, catches stale `?raw` source-pins) + svelte-check + `npm run build`. Desktop: `cd desktop && make build`.
- The lead owns the full-tree `make pre-push` from an isolated gate worktree, which gates the COMMITTED state and is immune to peers' WIP. Lanes report scoped-green plus a pathspec sha; they do not block on the main-tree gate (it false-reds on concurrent WIP). (phase 15)
- The release gate must build EVERY workspace CI ships. `gateway/` is a SEPARATE Cargo workspace; a `crates/`-scoped check misses it, and so do `desktop/src-tauri` and gateway construction sites for a new required field on a shared struct. Grep the whole repo and build them. v0.19.0 died at tag time on a stale crate name. (phases 8, 14, 16)
- Do not pipe the command whose exit code you are verifying: `cargo ... | tail` reports tail's 0 and hides cargo's failure. Run bare and capture `$?`, or set `pipefail`. (phase 8)

## Verification discipline (smoke, not just gates)

- rust-embed bakes the frontend bundle at BUILD time. To smoke a frontend change you must `npm run build` then `cargo build -p chan` then restart; there is no hot reload. A stale `web/dist` (gitignored) gives a false "the flag is broken" negative. Grep the SERVED bundle, not the source, when a flag looks broken. (phases 8, 15)
- When re-walking a previously-failed test, `pkill chan serve` + rebuild
  + verify binary provenance + restart. Stale-binary false positives
  caused multi-round wild-goose chases. In multi-agent runs a broad `pkill` kills every lane's server; the orchestrator serves from a renamed binary copy and lanes scope their pkills to their own path. (phase 15)
- Static gates (svelte-check, `?raw` source-pattern vitest) MISS Svelte-5 runtime reactivity errors (for example `state_unsafe_mutation` from mutating `$state` inside a `$derived`). Browser-smoke any reactivity change. (phase 15)
- Under tooling flakiness, agents confabulate: they invent file content matching their hypothesis, or mistake a stale Edit echo for a landed change. Anchor on `git status` / sha / `curl`, read atomically, and sha-verify before reasoning. (phase 8)
- Terminal render glitches (focus-switch, paste) are WKWebView/desktop specific and do NOT reproduce in Chrome automation (Blink). Ask which client first; verify desktop fixes in chan-desktop, which only @@Alex can hand-smoke. (phases 13, 15)

## Wire, rename, and cross-crate discipline

- A green cargo gate does NOT prove a rename is complete. serde enum tags, Tauri permission strings, JS `invoke` names, and route strings are validated at RUNTIME. Audit and smoke them; pin wire strings with `serde(rename)`. Watch ambiguous words: "drive" meant three things (the chan directory, cloud products, the tunnel domain); rename only the one you mean. This class of bug hit five times in one phase. (phases 12, 14)
- Adding a REQUIRED field to a shared type: grep ALL literals (both casings) and run svelte-check; vitest strips types, so a scoped vitest passes with fixtures missing the field. (phase 15)
- A multi-file Rust signature change leaves a transient non-compiling window that blocks same-crate peers (file ownership does not isolate the build). Make the signature change and all call sites in one burst and re-`cargo check -p <crate>` green before pausing. `.ts` edits are interleave-safe. (phase 16)
- Cross-crate kinds (the Rust FileClass / indexer kind and its TS mirror) must stay in lockstep. (phases 2, 6)

## Pre-release norms

- chan is pre-release: no back-compat, no migration paths. Drop legacy fields / formats / ids outright; do not add graceful-degrade paths or escalate back-compat questions. (phases 12+)
- Quality bar: keep the full gate green and ship no KNOWN bug. @@Alex accepts a few small bugs as the solo user but NOT a lower gate. Give a final validation pass and a local DMG before any upstream tag. (phase
  16)
- Release cut mechanics: bump every version pin together (Cargo.toml `[workspace.package]`, tauri.conf.json, Cargo.lock, and web/). Tagging `vX.Y.Z` fires release.yml; self-upgrade is data-driven from the published `latest.json`, so cutting a release auto-supersedes prior versions. macOS sign/notarize only runs on Actions; dry-run via workflow_dispatch before tagging. (phases 14, 15)

## Working as an agent inside chan

chan doubles as the orchestration host. A terminal chan launches gets `CHAN_MCP_*` env vars to reach chan's in-process MCP server, and a `CHAN_TAB_NAME` it self-identifies from. The integration contracts (event files written atomically, the spawn protocol, MCP discovery) live in [`orchestration/`](orchestration/). Secrets never appear in journals, chat, or commits; signing-secret consumption is directed by NAME through GitHub Actions Secrets only. (phase 8+)
