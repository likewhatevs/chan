# Gemini Verification and First-Class OpenCode Terminal Support

Status: implemented. Grounded against OpenCode 1.18.3 and Gemini CLI 0.51.0 on 2026-07-18.

Implementation branch: `feature/opencode-terminal-support`.

## Summary

OpenCode is a first-class terminal submit agent everywhere chan already recognizes Claude, Codex, and Gemini. Agent identity remains derived from the stored spawn command or `CHAN_AGENT`; the SPA has no agent selector.

The built-in OpenCode submit template is exactly:

```text
\x1b[200~{}\x1b[201~\r
```

After trimming trailing newlines from the prompt body, chan substitutes it for `{}` and sends the complete bracketed-paste-plus-CR payload in one PTY write. This form is proven for multiline and paste-sized input.

Gemini remains the only split-write submit agent. Chan sends its trimmed body first, then sends a bare `\r` as a later ordered PTY write. No queue timing, capacity, batching, drain, storage, idle-gate, or delivery-event behavior changed in this work.

## Public Contract

- Rust: `SubmitAgent::OpenCode`, with lower-case name `opencode`.
- CLI: `cs terminal write --submit=opencode`.
- Environment: `CHAN_AGENT=opencode` and `CHAN_SUBMIT_OPENCODE`.
- Configuration: `[opencode].template` in `~/.chan/submit.toml`.
- Terminal WebSocket: optional `session.submit_agent`.
- TypeScript: `"opencode"` in `SubmitAgent` and `AgentTarget`.

## Submit Behavior

| Agent | Built-in encoding | Ordered PTY writes |
|---|---|---:|
| Claude | body + modifyOtherKeys Cmd+Enter CSI | 1 |
| Codex | bracketed paste + CR | 1 |
| Gemini | body, then bare CR | 2 |
| OpenCode | bracketed paste + CR | 1 |

OpenCode's default is bracketed paste even though a small raw body plus CR also worked. The bracketed form has direct multiline and approximately 20 KiB coverage, and matches OpenCode's separate paste handling before Return submission.

Gemini CLI 0.51.0 intentionally treats Return received within 30 ms of inserted text as Shift+Return. Bracketed paste is emitted as an insertable event too, so combining either raw text or bracketed paste with an immediate CR does not satisfy Gemini's submit contract. The existing later-write behavior is therefore deliberate and unchanged.

Runtime template precedence is:

1. `CHAN_SUBMIT_<AGENT>`.
2. The matching table in `~/.chan/submit.toml`.
3. The built-in template.

Trailing newlines are trimmed before template substitution for all submitted prompts. `submit_writes` splits the resolved result only for Gemini; OpenCode has raw-write cost one.

## Agent Derivation

`SubmitAgent::derive` is the Rust source of truth and the Team dialog carries a byte-compatible TypeScript mirror.

- A recognized `CHAN_AGENT` value wins over command detection.
- `none` and `shell` explicitly disable agent submit encoding.
- Otherwise, command detection uses a case-insensitive whole-word match.
- `opencode`, `OPENCODE`, and wrappers such as `opencode-ai` match.
- Partial words such as `myopencode` and `opencoded` do not match.
- Shells and unknown commands remain unclassified.

Chan does not inspect terminal titles or child processes. An agent launched manually from an existing shell is intentionally outside spawn-command derivation.

## Server-Reported Identity

Each terminal `session` frame may include `submit_agent`. The server recomputes it from the current PTY incarnation's stored command and `CHAN_AGENT` whenever it sends an attach prelude:

- initial attach reports the original spawn identity;
- restart uses the replacement spawn options;
- browser reattach reports the current identity again;
- shells and unknown commands omit the field for wire compatibility.

`TerminalTab.submitAgent` is transient SPA state. Every `session` frame replaces it, including clearing it when the field is absent. Rich Prompt prefers this server value. Keyboard-protocol inference remains only as a fallback for older servers and agents launched manually from a shell. If that fallback classifies OpenCode as Codex, the default bytes are compatible.

## Team Work

Team member and lead identity still come from each member's command plus optional `CHAN_AGENT`; there is no stored agent field or visible selector. OpenCode members appear as `opencode` in generated bootstrap material, whose submit description is `bracketed-paste + \r`. An OpenCode lead receives its identity prompt as one bracketed-paste-plus-CR write.

## Automated Coverage

The implementation adds or extends tests for:

- four-agent parse/name round trips and clap acceptance/rejection;
- OpenCode command wrappers, case folding, partial-word rejection, and `CHAN_AGENT` precedence;
- exact singleton, multiline, trailing-newline, and approximately 20 KiB OpenCode payload bytes;
- OpenCode's one-part `submit_writes` result and Gemini's retained two-part result;
- `CHAN_SUBMIT_OPENCODE` and `[opencode]` precedence/parsing;
- session-frame identity presence, omission, restart replacement, and reattach resynchronization;
- Rust/TypeScript encoding parity, session-frame adoption and clearing, Rich Prompt preference, and protocol fallback;
- Team Work derivation, generated bootstrap text, and an OpenCode lead's exact one-write encoding.

Final repository verification commands:

```sh
make web-check
make pre-push
```

The existing queue tests remain unchanged and are exercised by the full gate.

## Live Evidence and Remaining External Check

The disposable OpenCode 1.18.3 baseline accepted small text plus CR in one write. It also accepted one bracketed paste containing approximately 20 KiB followed by CR in that same write and returned every head/tail sentinel exactly. The bracketed form is the pinned default.

Gemini CLI 0.51.0 reproduced the expected negative cases: combined text plus immediate CR and bracketed paste plus immediate CR did not submit. OAuth completed in the browser on this machine but returned to the authentication chooser, so the final model-backed positive matrix requires a free personal Google account or a free AI Studio key. A key must never be sent through `cs`, terminal logs, probe artifacts, or committed configuration.

Pinned CLI validation must use transient npm packages and temporary XDG/npm-cache directories. Do not use Snap, `sudo`, or a persistent global install.

## References

- [Gemini keypress timing behavior](https://github.com/google-gemini/gemini-cli/blob/v0.51.0/packages/cli/src/ui/contexts/KeypressContext.tsx#L230-L247)
- [Gemini authentication](https://geminicli.com/docs/get-started/authentication/)
- [Gemini installation](https://geminicli.com/docs/get-started/installation/)
- [OpenCode keybindings](https://opencode.ai/docs/keybinds)
- [OpenCode 1.18.3 prompt input](https://github.com/anomalyco/opencode/blob/v1.18.3/packages/tui/src/component/prompt/index.tsx#L1390-L1418)
- [OpenCode installation](https://opencode.ai/docs/)
