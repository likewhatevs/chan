# Teach an Agent the chan Surface with One Command

> Status: shipped in [v0.72.0](../../release/release-v0.72.0.md).

> Status: implemented on `feat/dump-skill` (`81aca589`), pending merge. Grounded against `59acd07a` (main, immediately after the v0.71.0 GA close).

## Summary

Add `chan dump-skill`: a single command that prints an agent-facing manual of chan's whole surface, rendered live from the clap trees rather than checked in as a document. `--list` prints the topic index and `--topic <slug>` prints one page.

The manual is not a second corpus. Every section is the `render_long_help` output of a real command, and the concept prose an agent needs (the graph model, team rounds, devserver setup, the authoring pipeline) lives in the `long_about` / `after_long_help` of the command that owns that knowledge. Improving `cs export --help` and improving the manual is one edit.

## Problem

An agent launched inside a chan terminal has no way to discover what it is inside. It sees a shell, so it behaves like one: it never finds `cs`, never opens a workspace, never spawns a team, never exports a document, and cannot guide the user toward any of it. Everything chan adds over a plain terminal is invisible unless somebody tells the agent it exists.

The environment does carry the signal (`$CHAN`, `$CHAN_CONTROL_SOCKET`, `$CHAN_WINDOW_ID`), but nothing turns that signal into usable knowledge. `chan --help` before this item summarized commands in one line each and pointed nowhere; the detailed behavior lived in scattered doc comments that clap collapsed onto single lines, so even reading the help was a poor experience.

The obvious alternative, a checked-in `SKILL.md`, is the failure mode the repo already knows: a document that describes the CLI drifts from the CLI on the first commit that nobody remembers to mirror. `chan open --help` had already drifted from `shortcuts.ts`, carrying a renamed "Enter Hybrid Nav" row and omitting "Flip pane side" entirely.

## Evidence

- `crates/chan/src/lib.rs` and `crates/chan-shell/src/cli.rs` at `59acd07a`: command documentation is derive doc comments only. clap joins a doc comment's paragraphs into one line, so `chan close --help` emitted a single 318-column line.
- The workspace pins `clap` without `wrap_help` (`Cargo.toml`), so help text reaches the terminal verbatim and nothing rewraps an over-long line.
- `SERVE_LONG_ABOUT` in `crates/chan/src/lib.rs` embedded a chord table pasted by hand from `web/packages/workspace-app/src/state/shortcuts.ts` with no check that the two agreed. They did not.

## Desired Contract

1. `chan dump-skill` prints a complete manual on stdout and writes nothing. `--list` prints the topic index; `--topic <slug>` prints one page.
2. Every `##` section of the manual is the live long help of a real command. No section is authored separately from the command it documents.
3. Concept prose lives in the help of the command that owns it, not in the skill module. The skill module holds the spine, the frontmatter, and the closing index.
4. The emitted document carries the three-key frontmatter (`name`, `description`, `when_to_use`) that agent harnesses parse, so `chan dump-skill > ~/.claude/skills/chan/SKILL.md` is the whole install.
5. `--topic` accepts aliases, so an agent that guesses a noun (`teams`, `lima`, `pagebreak`) still lands on the command that covers it.
6. A topic page is a fragment: it carries no frontmatter, because a fragment is not a skill file.
7. Help text is hand-wrapped and ASCII, because clap prints it verbatim.
8. `chan open --help` and `shortcuts.ts` cannot disagree silently.

## Implementation Boundaries

- `crates/chan/src/skill.rs` owns the spine (`SPINE`), the frontmatter, the lead, and the closing index, and renders sections through `render_long_help` off two roots: `Cli::command()` and `chan_shell::CsCli::command()`. `cs` is its own parser, not a subtree of `chan`, and the module keeps them separate exactly as the command line does.
- Long-form help moves out of doc comments into consts: `crates/chan/src/help.rs` for `chan`, `crates/chan-shell/src/help.rs` for `cs`. Doc comments keep one short prose paragraph; anything with structure (examples, tables, unit files) is a const wired up as `long_about` / `after_long_help`. `chan-shell`'s help module is client-gated so chan-server does not link the manual text.
- Each `_AFTER` const follows one template: EXAMPLES, SIDE EFFECTS, CAUTIONS, CAVEATS, SEE ALSO, omitting any that would be empty.
- `chan dump-skill` is pure output, like `chan completions`: no workspace, no registry, no side effects.
- `renderTable` in `web/packages/workspace-app/src/state/shortcuts.ts` grows an optional `maxWidth`, which moves an oversized trailing note onto its own indented line. `chan open --help` passes a cap because clap does not rewrap; callers with no column budget omit it. `scripts/shortcuts-table.mjs --serve-long-about` emits the indented table alone, and the Rust side owns the framing around it.
- `scripts/check-shortcuts-help.py` regenerates the table and diffs it against `KEYBINDINGS_TABLE` in `crates/chan/src/lib.rs`. `make shortcuts-check` runs it and is part of `make pre-push`, hence of `make ci-linux`. It lives on the web side of the Makefile because the generator needs node.
- Out of scope: writing the skill file anywhere on disk, generating agent-owned config, and per-agent output formats. The document goes to stdout and the caller decides where it lands.

## Acceptance Checks

Automated, in `crates/chan/src/skill.rs`:

- every spine path resolves against the live clap tree;
- topic slugs and aliases are unique;
- every visible command in both trees is either a spine section or an explicit exemption, matched by exact path so a new leaf cannot hide behind a documented parent;
- every command `about`, `long_about` and `after_long_help` line is at most 76 columns, ASCII, tab-free, and cannot break the skill's code fence;
- every command summary is one line, because a summary is one row of its parent's command list;
- every `chan ...` / `cs ...` invocation an EXAMPLES block shows resolves against the live tree, honoring `infer_subcommands`;
- every `--topic X` cross-reference resolves, and no page links to its own topic;
- the rendered skill starts with frontmatter, contains every section heading, and leaks no generator markers.

Scoped commands:

```sh
cargo fmt --check
cargo clippy -p chan -p chan-shell --all-targets -- -D warnings
cargo test -p chan -p chan-shell
make shortcuts-check
cd web && npm run check -w @chan/workspace-app && npm run test -w @chan/workspace-app
```

Manual:

- `chan dump-skill --list` names every topic with its aliases;
- `chan dump-skill --topic teams` and `--topic cs-terminal-team` print the same page;
- `chan dump-skill --topic nope` fails and lists the known topics;
- `chan dump-skill > SKILL.md` installed into an agent's skill directory lets a fresh agent in a chan terminal find `cs` and drive a workspace.

## Known Gaps

Recorded from the adversarial review of `81aca589`, for whoever carries this forward.

- Each section's one-line subtitle (`Section::title`), the frontmatter, and the lead are authored in `skill.rs` and reach the manual, so the "every byte is reachable from some `--help`" claim holds for section bodies only. No test reads a title, so a title can contradict the command it labels.
- The width and ASCII checks read `about` / `long_about` / `after_long_help` only. Argument help is never inspected, and clap indents it, so `--help` output routinely exceeds 80 columns (`chan workspace search` reaches 161).
- No test inspects `render_long_help` output, so enabling clap's `wrap_help` (a dependency bump, or a future crate in the root workspace asking for it) would rewrap every help string with nothing to catch it.
- The EXAMPLES check validates the subcommand path only. A flag or value that does not exist passes.
- `UNDOCUMENTED` is one-directional: removing an entry reds the suite, but adding one is silent and a stale entry for a command that no longer exists is never reported.
