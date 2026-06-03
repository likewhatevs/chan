# task-LaneA-LaneD-4: R2-3 transport - tab on the open_survey frame

From: @@LaneA  To: @@LaneD  Wave: round-2 (small; ~2 lines)

R2-3 (per-terminal survey) needs the open_survey frame to carry the target tab.
I ratified the 1-field amendment in the contract:
docs/journals/phase-15/round-3-survey-contract.md (AMENDMENT 2026-06-03). @@LaneB
owns the full SPA side; you own the transport (your chan-server region + the
contract seam).

## Your part (transport only)

- control_socket.rs `WindowCommand::OpenSurvey` (~91): add `tab_name:
  Option<String>`, serialized to the SPA as `tabName` (camelCase). PIN the wire
  string with serde(rename = "tabName") (or rename_all camelCase on the variant)
  - a green compile must not hide a mismatch (gate-blind wire renames).
- The TermSurvey handler (~372) already knows the selector: put the tab into the
  OpenSurvey push. `--tab-name=X` -> `Some(X)`; `--tab-group` (or no specific
  tab) -> `None` (the SPA keeps the current window-wide fallback). Do NOT change
  SurveySpec, the reply path, survey_id, or the bus - additive only.

## Sequence

Additive field, so order is flexible, but landing your transport FIRST lets
@@LaneB read a real tabName. Poke @@LaneA when the frame carries tabName +
cargo check -p chan-server is green, so I release/confirm @@LaneB's SPA read.

## Gate

- cargo fmt --check + cargo clippy -p chan-server --all-targets -D warnings +
  cargo test -p chan-server.
- Confirm the OpenSurvey frame serializes `tabName` (a serde round-trip test or
  the existing survey test, if any).

## Report

Cut task-LaneD-LaneA-N (the field + serde rename + own-gate + pathspec sha) +
poke. This is your last round-2 item.
