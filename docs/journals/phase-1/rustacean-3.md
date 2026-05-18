# rustacean-3: CLI parity for config, graph, and status

Owner: rustacean. Depends on: rustacean-1, rustacean-2. Unblocks:
syseng-1.

## Goal

Add CLI subcommands that match the web UI's public functionality without
duplicating server internals in the binary crate.

## Required commands

- `chan config get <key>`
- `chan config set <key>=<value>`
- `chan graph ...`
- `chan status ...`

## Product shape

- Config keys should match settings concepts exposed in the Settings
  overlay, starting with editor theme/layout/appearance. Use current key
  names, not misspelled public contracts.
- `chan graph` queries the graph per scope. Include folder/file scope once
  rustacean-2 freezes the API/core shape.
- `chan status` reports drive, search index, filesystem graph, and
  chan-report state.

## Architecture constraints

- Clap definitions and `cmd_*` dispatch live in `crates/chan/src/main.rs`.
- Server-owned logic stays in `chan-server`; drive/index logic stays in
  chan-drive.
- Config file writes go through existing store/config helpers.
- Output should be scriptable. Prefer stable text now only if JSON would
  be premature; if adding `--json`, test it.

## Acceptance criteria

1. Help text reflects exact behavior.
2. Config get/set can read and update at least editor theme/layout/
   appearance settings that exist today.
3. Graph command can query file/folder scope and print nodes/edges.
4. Status command reports index/report/graph freshness enough to match
   web dashboard expectations.
5. Existing subcommands continue to work.

## Verification

- CLI unit tests where practical.
- `cargo test -p chan`
- `cargo test -p chan-server`
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets -- -D warnings`

## Done means

Update this file with final command syntax, examples, changed paths,
tests run, and mark `rustacean-3` REVIEW in `journal.md`.

---

## 2026-05-16 Execution

Status: DONE.

### Inherited work

`chan graph <PATH>` and `chan status [PATH] [--json]` were already
drafted in the uncommitted `crates/chan/src/main.rs` (see
`backend-1.md`). This task kept `--scope all` on the semantic markdown
graph and switched `--scope file|folder` to the filesystem graph builder
shared with `/api/fs-graph`, so CLI `Graph this` parity now covers
files, folders, symlinks, hardlinks, ghosts, depth, and `truncated`.
One status cleanup was also applied:

- `chan status` no longer surfaces `contacts_need_email_backfill`
  (consumer removed by rustacean-1; the producer-side helper in
  chan-core gets purged by the `chan-core-purge-1` follow-up).

### New: `chan config get|set`

`chan config` reads and writes editor preferences, server path settings,
and assistant backend settings. Editor writes route through
`chan_server::EditorPrefs::save`; server writes through
`chan_server::ServerConfig::save`; assistant writes through
`chan_llm::LlmConfig::save`, keeping each namespace on its existing
atomic-write path. Keys covered today:

```
editor.theme               system|light|dark
editor.editor_theme        github|google_docs|word
editor.line_spacing        tight|standard
editor.date_format         string (matches dateFormats.ts ids)
editor.pane_widths.inspector   u32
editor.pane_widths.graph       u32
editor.pane_widths.browser     u32
editor.pane_widths.search      u32
editor.pane_widths.outline     u32
editor.pane_widths.assistant   u32
server.attachments_dir         string
server.answers_dir             string
assistant.effective_enabled    bool (read-only)
assistant.default_backend      claude_cli|gemini_cli|codex_cli|none
assistant.answers_dir          alias of server.answers_dir
assistant.{claude_cli|gemini_cli|codex_cli}.enabled       bool
assistant.{claude_cli|gemini_cli|codex_cli}.model         string|default
assistant.{claude_cli|gemini_cli|codex_cli}.cmd_override  string|none
```

Registry-backed drive settings are deliberately not in this slice.

### Syntax

```
chan config get                          # dump preferences.toml
chan config get --json                   # JSON dump
chan config get editor.theme             # scalar (e.g. "dark")
chan config get editor.theme --json      # JSON-quoted

chan config set editor.theme=dark        # equals form
chan config set editor.theme dark        # two-arg form
chan config set editor.pane_widths.search=320
```

Refusals (non-zero exit, stderr message, NO partial config write):

- empty value: `chan config set editor.theme=` -> `value must not be empty`
- missing value: `chan config set editor.theme` -> `missing value: use ...`
- unknown key: `chan config get editor.bogus` -> `unknown key ...`
- bad value: `chan config set editor.theme=neon` -> `expected system|light|dark, got neon`
- bad number: `chan config set editor.pane_widths.search=-1` -> `expected non-negative integer`
- read-only assistant state: `chan config set assistant.effective_enabled=true`
  -> `read-only`

### Files changed

- `crates/chan/src/main.rs` (cleanups + Config enum + cmd_config + helpers + 12 tests)
- `crates/chan/Cargo.toml` (add `toml` dep for the TOML dump path)
- `crates/chan-server/src/indexer.rs` (rustacean-1: removed
  contacts_need_email_backfill consumer; was already in main.rs's
  status path, which this task pruned too)

### Verification

```
cargo fmt --all -- --check                # clean
cargo build -p chan                       # ok
cargo clippy --all-targets -- -D warnings # clean
cargo test -p chan                        # 46 passed
cargo test -p chan-server                 # 92 passed (covers fs-graph
                                            and EditorPrefs paths)
target/debug/chan config get              # dumps TOML preferences
target/debug/chan config get editor.theme # prints `system`
target/debug/chan config set editor.theme=dark    # writes + echoes
target/debug/chan config set editor.theme system  # resets
target/debug/chan config set editor.theme=        # 400, exit 1
target/debug/chan config get bogus.key            # 400, exit 1
target/debug/chan graph /tmp/chan-cli-fsgraph \
  --scope folder --target notes --depth 1 --json # fs-graph JSON
target/debug/chan graph /tmp/chan-cli-fsgraph \
  --scope file --target notes/a.md --json        # fs-graph JSON
target/debug/chan graph /tmp/chan-cli-fsgraph \
  --scope folder --target notes/a.md             # 400, exit 1
```

Architect verification note: initial `cargo test -p chan` failed
because two config tests still referenced a removed `read_pref_key`
helper. Tests now call `read_config_key` with `ServerConfig::default`
and pass without warnings.

### Residual risks / follow-ups

- Registry-backed settings are not exposed through `chan config` yet.
  File as follow-up if the Settings overlay grows a stable key schema
  for drive registry values.
- syseng-1's hardening probe for `chan config set editor.theme=`
  expects non-zero exit + no partial write. Verified manually in
  this session; syseng can rerun against the fixture.
