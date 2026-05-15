# bob-5: Per-CLI readiness reporting + binary-path override

Owner: Bob. Depends on: nothing structural; sits alongside bob-4
cleanup (bob-4 deletes dead surface, bob-5 adds new surface). May
be tackled before, during, or after bob-4; pick what flows best.

## Why

User feedback after manual testing:

1. Settings only ever shows "ready" for whichever CLI is the
   currently-active backend. The other two rows have no readiness
   signal at all because `/api/llm/status` reports for the active
   backend only.
2. Non-ready backends need a clear, per-backend explanation of
   what's wrong (binary missing, wrong PATH, etc.).
3. Users want an explicit per-CLI override for the command path
   (or a PATH override). Today the CLI lookup goes through the
   default `PATH` from the shell environment.

The downstream consumer is martin-7's redesigned Settings panel
(left-side CLI dropdown with per-backend readiness + right-side
override input). bob-5 ships the server contracts that panel
consumes.

## Files to touch (expected)

- `crates/chan-server/src/routes/llm.rs` (new handler + route).
- `crates/chan-server/src/routes/preferences.rs` (extend the
  `CliPrefsView` shape to round-trip the override).
- `crates/chan-server/src/lib.rs` (route registration).
- `../chan-core/crates/chan-llm/src/config.rs` IF (and only if) the
  config schema needs a new field for `cmd_override`. Check the
  existing `claude_cli` / `gemini_cli` / `codex_cli` config structs
  first; chan-llm 0.11 may already accept a `cmd` field per
  backend. If yes, route the override through that and skip the
  chan-core edit. If no, decline the chan-core edit and surface
  the gap; we'll cut a separate chan-core task. Do NOT extend
  chan-core silently.

## Required changes

### 1. Multi-CLI readiness endpoint

Add `GET /api/llm/cli_detection` returning an array of detection
records, one per CLI backend:

```json
{
  "detections": [
    {
      "backend": "claude_cli",
      "ready": true,
      "command": ["/usr/local/bin/claude"],
      "reason": null
    },
    {
      "backend": "gemini_cli",
      "ready": false,
      "command": ["gemini"],
      "reason": "`gemini` not found or rejected. Install the Gemini CLI, or set its cmd in llm.toml."
    },
    {
      "backend": "codex_cli",
      "ready": false,
      "command": ["codex"],
      "reason": "..."
    }
  ]
}
```

Use the existing `chan_llm::detect_backend_cli` and `detect_all`
helpers (whichever fits). The `reason` field should match the
formatting `routes/llm.rs::api_llm_status` already produces (line
129-132 today) so the frontend can render the same text in both
places. Each detection should account for the current
`cmd_override` if one is set in prefs.

### 2. Per-CLI cmd override persistence

Extend `CliPrefsView` in `routes/preferences.rs` (today: `enabled`
+ `model`) with a `cmd_override: Option<String>` field. Wire it
through:
- The unified Preferences GET response (returned alongside model).
- The Preferences PUT path (accepts an updated override).

On the chan-llm side, check whether `ClaudeCli` / `GeminiCli` /
`CodexCli` config structs already have a `cmd` or `command` field.
If yes, the override populates that field. If no, see the
"chan-core gap" note in the Files section above.

### 3. Override validation

When a client PUTs a `cmd_override` value, validate it before
persisting. Two cases:

- Empty / None: clear the override. Always valid.
- Non-empty string: validate it as one of:
  - An absolute path: must exist on disk, must be a file (not a
    directory / symlink to nowhere / device), must be executable
    by the current process. Use `std::fs::metadata` +
    permissions check. Do NOT execute the binary to validate it.
  - A bare command name (`claude`, `gemini`, etc.): must resolve
    via PATH. Use `which::which` if `which` is already a dep, or
    a hand-rolled PATH walk. Returning the resolved absolute
    path in the response body is a nice bonus but not required.

On validation failure, return 400 with a structured error
explaining the rejection (`{"error": "binary not found: ..."}`).
Do not persist the bad value.

Note: the user feedback said "verified with chan-core's hardened
path checking code." chan-drive's `resolve_safe_strict` is
drive-scoped (sandboxed to the drive root) and is the WRONG tool
here. The binary may be `/usr/local/bin/claude` which is outside
any drive root. Use chan-llm's existing CLI detection logic
(`detect_backend_cli` with a config that points at the override)
since it's the same code path that runs at chat time. If
chan-llm doesn't expose a "validate this candidate without
running it" entry point, the simple OS file-existence /
permission check above is sufficient for v1.

### 4. Existing endpoint compatibility

`/api/llm/status` keeps reporting the active backend only.
Don't change its response shape. The new endpoint is additive.

## Acceptance criteria

1. `cargo build` passes.
2. `cargo test` passes; new endpoint has a focused test in the
   same style as bob-3's lifecycle tests (assert response JSON
   shape, mock chan-llm detection if needed).
3. `cargo clippy --all-targets -- -D warnings` and `cargo fmt
   --all -- --check` pass.
4. `GET /api/llm/cli_detection` returns three detection records.
5. `PUT /api/preferences` with a new `assistant.claude_cli.cmd_override`
   field round-trips: GET after PUT returns the same value, and
   the detection endpoint reflects the override on next call.
6. Invalid override values return 400 with a useful error message.

## Out of scope

- Changing the meaning of `default_backend`. martin-7 is removing
  the default-backend selector from Settings, but the underlying
  field still drives chat. Don't touch it.
- Model picking. That stays in the assistant inspector
  (martin-8). The `models.{claude_cli,...}` config field is
  unchanged.
- Removing the dead anthropic/gemini surface. That's bob-4.

## Done means

Post an update to `tasks/journal.md` (status DONE for bob-5, plus
one-line log entry).
