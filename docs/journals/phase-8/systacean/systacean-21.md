# systacean-21 — enrich poke event echo with timestamp + path + heading (cache-bust for rate-limit mitigation)

Owner: @@Systacean
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Replace the bare `b"poke"` literal that
`dispatch_agent_event` writes to the receiving agent's PTY
with a richer string carrying a wall-clock timestamp + the
task-file path + the heading anchor:

```
Poke, it's {Fri, 22 May at 05:31}. Check your task at {path}#{heading} and execute.\n
```

Every poke becomes a unique input → guaranteed cache-miss
on Anthropic's prompt-cache layer → reduces the
rate-limit / HTTP 500 surface @@Alex has been hitting
daily.

## Background

**Primary motivation** (operational): @@Alex 2026-05-22
identified that bare `poke` repeats appear to trigger
Anthropic's prompt-cache hit pattern + land on
capacity-constrained paths (rate limits + HTTP 500s)
during peak hours. Cache-bust via input uniqueness is the
immediate mitigation. See
[`../phase-8-bugs.md`](../phase-8-bugs.md) "Enrich poke
event echo with timestamp + task path + heading anchor"
for the full framing.

**STRONG OBSERVATIONAL EVIDENCE 2026-05-22**: @@Alex
tested informally. All four agents (FullStackA,
FullStackB, Systacean, CI) were INSTA-rate-limited on
bare `poke`. The same agents, prompted with non-bare
alternatives, woke up cleanly:

* "aloha amigo, it's time.. check your tasks and execute"
* "oi, it's 5:35, check your tasks and execute"
* "hey it's 5:35, check your tasks and execute"

NOT CONFIRMED — the bare-poke + non-bare attempts ran at
slightly different times; time-of-day capacity variance
isn't ruled out. Only Anthropic could confirm via their
telemetry. But evidence is strong enough to act on +
enriching the poke text is a strict improvement
regardless (better agent context, less identical-input
repetition). Until `-21` ships, @@Alex bootstraps each
agent via non-bare prompts manually.

**Secondary benefit**: gives the agent immediate context
about what to look at without polling / grepping.

## Today's behaviour

`crates/chan-server/src/terminal_sessions.rs:525-549`
(`dispatch_agent_event`):

```rust
let mut bytes = Vec::with_capacity(4 + mode.submit_chord().len());
bytes.extend_from_slice(b"poke");
bytes.extend_from_slice(mode.submit_chord());
session.send_input(&bytes);
```

The literal `b"poke"` is the cache-key collision risk.
Every agent gets the same bytes.

`crates/chan-server/src/event_watcher.rs:47-61`
(`AgentEvent`):

```rust
pub(crate) struct AgentEvent {
    pub id: String,
    pub event_type: AgentEventType,
    pub from: String,
    pub to: String,
    pub topic: Option<String>,
    // ... survey fields ...
}
```

No `path` or `heading` fields today.

## Decision: fix shape

### Schema extension (event_watcher.rs)

Add two optional fields to `AgentEvent`:

```rust
pub path: Option<String>,
pub heading: Option<String>,
```

Both `Option<String>` + serde-skip-when-None for
backward-compat (pre-`-21` event files load cleanly with
None for both).

### Content templating (terminal_sessions.rs)

In `dispatch_agent_event`, format the rich template when
BOTH `path` + `heading` are present:

```
Poke, it's <weekday>, <day> <month> at <HH:MM>. Check your task at <path>#<heading> and execute.
```

(Trailing newline / submit chord handled by the existing
`mode.submit_chord()` append per `-b-13`.)

Format spec:
* `<weekday>`: short form — `Mon`, `Tue`, `Wed`, `Thu`,
  `Fri`, `Sat`, `Sun`.
* `<day>`: integer, no leading zero (`1`-`31`).
* `<month>`: short form — `Jan`, `Feb`, ..., `Dec`.
* `<HH:MM>`: 24-hour, zero-padded — `05:31`, `14:08`,
  `23:59`.
* TZ: server-side wall-clock. **Recommendation**:
  system-local time (more meaningful to the user who
  receives it). If chrono/time crate dep adds too much
  weight, default to UTC + label as such in the
  template. Implementer picks.

Fallback path: if `path` OR `heading` is missing,
fall back to bare `b"poke"` per today's behaviour. Covers:
* In-flight legacy events from pre-`-21` writers.
* Survey / survey-reply event types where the
  path-context doesn't apply.

### Architect-side workflow note

Going forward, the architect (and any other lane firing
pokes) populates `path` + `heading` in the JSON payload.
Backward-compat means existing event-file infrastructure
keeps working without changes. The architect-side workflow
tooling update is a separate concern; flag it in the
task tail for follow-up if you spot specific places that
need updates.

## Acceptance criteria

### Schema

1. `AgentEvent` carries `path: Option<String>` +
   `heading: Option<String>`.
2. Both fields serde-skip-when-None (use
   `#[serde(skip_serializing_if = "Option::is_none")]`
   or equivalent for the Serialize side if it exists;
   Deserialize handles missing-field as None natively).
3. Existing event files (pre-`-21` shape) parse cleanly
   via the existing JSON parser + load with `path = None`
   + `heading = None`.

### Content templating

1. `dispatch_agent_event` formats the rich template
   when both `path` + `heading` are Some.
2. Format matches the spec above. Implementer picks
   chrono / time / built-in via std::time + manual
   formatting (no preference; pick the lightest dep
   shape).
3. Fallback to bare `b"poke"` when either field is None.
4. Submit chord append behaviour unchanged from `-b-13`.

### Tests

1. Update existing `dispatch_agent_event_writes_poke_to_matching_tab`
   test (line 1829) — depending on whether you keep the
   bare-poke path for legacy events, this test either
   stays as-is (legacy path) or moves to the new template.
2. NEW test: `dispatch_agent_event_writes_rich_template_when_path_and_heading_present`.
   Fixture event with `path = Some("docs/journals/.../systacean-21.md".into())`
   + `heading = Some("2026-05-22-poke".into())`. Assert
   the output contains:
   * The literal "Poke, it's "
   * A weekday + date + time pattern
   * The literal "Check your task at <path>#<heading>"
3. NEW test: `dispatch_agent_event_falls_back_to_bare_poke_when_path_missing`.
   Fixture event with `heading = Some(...)` + `path = None`.
   Assert output is bare `b"poke"` + chord.
4. Schema round-trip test for the new fields (probably
   in `event_watcher.rs` tests).

### Gate

* `cargo fmt --check`: clean.
* `cargo clippy --all-targets -- -D warnings`: clean.
* `cargo test -p chan-server`: all green (+ ~3 new tests).
* `RUSTFLAGS="-D warnings" cargo build --no-default-features`:
  green.
* Web side: no changes; web/npm test unchanged from
  baseline.

## How to start

1. Read `event_watcher.rs:47-61` for the current
   `AgentEvent` shape.
2. Add the two optional fields.
3. Read `terminal_sessions.rs:525-549` for the current
   dispatch path.
4. Pick the timestamp crate (chrono / time / std). Lean
   toward whatever's already in the dep graph.
5. Implement the template + the fallback.
6. Write the 3 new tests.
7. Local pre-push gate green workspace-wide.
8. CI smoke via `gh workflow run ci.yml --ref systacean-21-smoke`
   on a fresh smoke branch.
9. Append "Commit readiness" + fire poke to @@Architect.

## Coordination

* @@Systacean lane (chan-server primary scope).
* SEQUENCING: pick up AHEAD of `-12` (which is parked
  on @@Alex's permission re-grant from `955ada1`).
  `-21` doesn't need a permission ask + delivers
  operational mitigation to the rate-limit pain @@Alex
  has been hitting daily.
* @@Alex grants `-12`'s permission separately;
  `-12` rides whenever that lands.

### Shared-infra authorization

**Authorization: yes** for this task to edit:

* `crates/chan-server/src/event_watcher.rs` (struct
  extension).
* `crates/chan-server/src/terminal_sessions.rs`
  (dispatch_agent_event templating).
* Possibly `Cargo.toml` for a timestamp dep IF the
  current dep graph doesn't include one (audit first;
  chrono / time may already be transitively available).
* `docs/journals/phase-8/systacean/systacean-21.md`
  (task tail).
* `docs/journals/phase-8/alex/event-systacean-architect.md`
  (outbound).

@@Systacean may proceed without further confirmation
from @@Alex. Standing atomic-audit-commit discipline
applies.

## Numbering

Highest dispatched `systacean-N` is `-20` (lock-contract
gates + smoke fixups). `-19` was C2 BM25 fallback;
`-16` was FileBucket. Next available: `-21`. This is
`-21`.

### Queue (revised 2026-05-22)

```
-21 (this task; enrich poke echo for cache-bust)
-12 (tauri-plugin-updater verify; gated on @@Alex permission re-grant)
```

Both can ride independently. `-21` ahead of `-12`
unblocks operational pain immediately + doesn't need an
interactive permission window.

## Out of scope

* The architect-side workflow tooling update (writing
  `path` + `heading` into outbound event payloads). The
  chan-server side preserves backward-compat; the
  architect can begin populating the fields when ready.
  If specific writer call sites need touching, flag at
  task tail.
* Per-agent template customization (different agents may
  want different prompt shapes). Single template for
  now; per-agent customization is Round-3 polish if
  needed.
* The architect side knowing which heading is the
  "current" one to point at. The architect writes the
  heading based on their own task-file structure +
  outbound poke convention; no chan-server logic on
  heading selection.
* TZ user-preference. System-local (or UTC if simpler)
  for now; user-configurable TZ is Round-3 polish.

## What this task is NOT

* A rewrite of the event-watcher / dispatch path.
  Narrow additive change.
* A change to the submit-mode / chord behaviour from
  `-b-13`. Submit chord append behaviour unchanged.
* An agent-side change. Pure server-side; agents see
  richer text + their existing input handling
  consumes it normally.

## 2026-05-22 — implementation + commit readiness

URGENT operational mitigation per @@Alex's bare-poke rate-limit observation. Authorization explicit in the task body.

### Schema extension (event_watcher.rs)

* Added `path: Option<String>` + `heading: Option<String>` to `AgentEvent`, both `#[serde(default)]` so pre-`-21` event files load cleanly with both as `None`.
* New unit test `parse_event_path_and_heading_are_optional_with_backward_compat` — synthesizes a legacy event (no `path`/`heading`) + a new-shape event (both present) + asserts the parser handles both.

### Content templating (terminal_sessions.rs)

* New free function `format_poke_text(path: Option<&str>, heading: Option<&str>) -> String`:
  * When both `Some`: builds `"Poke, it's <Weekday>, <day> <Month> at <HH:MM>. Check your task at <path>#<heading> and execute."` using `chrono::Local` + `%a, %-d %b at %H:%M` format spec.
  * Otherwise: returns bare `"poke"` (legacy + survey fallback).
* `dispatch_agent_event` rewritten to call `format_poke_text(event.path.as_deref(), event.heading.as_deref())` + append `mode.submit_chord()` (preserves `-b-13` chord behaviour).
* Submit-chord append behaviour unchanged — chord still appended whether template or legacy.

### Timestamp dep

`chrono` was already a workspace dep (used by chan-drive, chan-report); added an explicit declaration to `chan-server/Cargo.toml` (`chrono = { workspace = true }`) so the import is unambiguous. No new transitive deps; workspace already locked it.

### Tests (3 new + 1 updated)

* `dispatch_agent_event_writes_rich_template_when_path_and_heading_present` — fixture event with `path = Some("docs/.../systacean-21.md")` + `heading = Some("2026-05-22-poke")`; asserts the PTY output contains `"Poke, it's "` prefix + the literal `"Check your task at <path>#<heading> and execute."` anchor.
* `dispatch_agent_event_falls_back_to_bare_poke_when_path_missing` — fixture event with `heading = Some(...)` + `path = None`; asserts output contains `"poke"` + does NOT contain `"Check your task at"` (template was suppressed).
* `format_poke_text_emits_rich_template_with_chrono_spec` — pure unit test on the helper. Asserts the rich-template shape (prefix, anchor, no trailing chord) when both fields present; asserts `"poke"` for the 3 None combinations (heading-only / path-only / both-None).
* Existing `parse_event_path_and_heading_are_optional_with_backward_compat` (event_watcher.rs) — schema round-trip per the task spec.
* Existing 3 `AgentEvent { ... }` literal call sites in tests updated to include `path: None, heading: None` (required by struct construction; field-add isn't backward-compat for literal builders).

### Files

| File                                            | +    | -  |
|-------------------------------------------------|------|----|
| `crates/chan-server/Cargo.toml`                 | +1   | 0  |
| `Cargo.lock`                                    | +1   | 0  |
| `crates/chan-server/src/event_watcher.rs`       | +42  | 0  |
| `crates/chan-server/src/terminal_sessions.rs`   | +184 | -4 |

Plus task tail + outbound poke. 6 paths total.

### Pre-push gate

* `cargo fmt --check`: clean.
* `cargo clippy --all-targets -- -D warnings`: clean.
* `cargo test -p chan-server`: **209 passed; 0 failed** (was 205 pre-`-21`; +4 new tests).
* `cargo test` workspace: all crates green.
* `RUSTFLAGS="-D warnings" cargo build --no-default-features`: green.
* **Web side**: `cd web && npm run check` reports 24 errors (`GraphPanel.svelte` `Property 'markdown'/'source' does not exist on type 'GraphFilters'`) + 2 vitest failures (`fullstack-a-52 G10: link filter dropped`). **These are PRE-EXISTING from FullStackA's in-flight `-a-52` work, not from `-21`**. My changes are Rust-only; `git diff --stat -- web/` is empty. Flagged so the smoke run's web job is understood as not a `-21` regression.

### Suggested commit subject

```
chan-server: enrich poke event echo with timestamp + path + heading (systacean-21)
```

### Smoke plan

Atomic-audit-commit pattern + push to fresh `systacean-21-smoke` branch + `gh workflow run ci.yml --ref systacean-21-smoke`. Expected:

* rustfmt ✓
* Ubuntu cargo test ✓ (chan-server lib gets the 4 new tests)
* macOS cargo test ✓
* build no-default-features ✓
* **web — pre-existing fail per the FullStackA in-flight state**. NOT a `-21` regression.

If web is the only red on the smoke, that's the empirical evidence the Rust-side change shipped clean.

### Operational note

Once `-21` lands + the architect-side workflow tooling starts populating `path` + `heading` in outbound event payloads, the bare-`poke` cache-collision pattern goes away. Architect-side tooling update is out of scope per the task body; flagged as a follow-up.

Holding for @@Architect commit clearance + smoke-branch authorization.
