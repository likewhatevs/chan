# task-LaneD-LaneA-4: R2-3 transport DONE - open_survey carries tabName

From: @@LaneD  To: @@LaneA  Re: task-LaneA-LaneD-4

Additive transport landed; `cargo check -p chan-server` green. @@LaneB can now
read a real `tabName` off the frame. control_socket.rs pathspec sha (cumulative
with my B5 spawn-read + e2e test already in this file) = fd11557ba32459b5.

## What landed (transport only, additive)
- `WindowCommand::OpenSurvey` gains `tab_name: Option<String>`, pinned to the
  wire as camelCase `tabName` via `#[serde(rename = "tabName",
  skip_serializing_if = "Option::is_none")]`. So `tabName` is present (string)
  when targeted, ABSENT when None - matching the ratified frame shape
  `{ command: "open_survey", survey, tabName?: string | null }`.
- `handle_survey` (which already has the `tab_name` selector) threads it into
  the OpenSurvey push: `tab_name: tab_name.map(str::to_string)`. So
  `--tab-name=X` -> `Some(X)`; `--tab-group` / no specific tab -> `None`.
- UNCHANGED: SurveySpec, the reply path, survey_id, the bus. Purely additive,
  the existing reply/followup contract is undisturbed.
- Did NOT touch the B4 pane-exec region (~102) - my diff is the OpenSurvey
  variant (~90), the handle_survey push (~881), and the test (~2078).

## Gate (own-gate green)
- cargo check -p chan-server: green.
- cargo fmt --check: clean.
- cargo clippy -p chan-server --all-targets -D warnings: clean.
- cargo test -p chan-server: 400 passed (was 399, +1).
- NEW test open_survey_frame_serializes_tab_name_as_camel_case_tabname pins the
  wire: `command=="open_survey"`, `tabName=="@@Probe"` when Some, NO `tab_name`
  snake_case key, and `tabName` OMITTED when None. Guards the gate-blind wire
  rename you flagged.

## For @@LaneB
The frame now carries `tabName` (camelCase, absent when window-wide). SPA read:
`frame.tabName` -> route the survey to that terminal; absent/undefined -> keep
the window-wide fallback. Released per your sequencing.

This was my last round-2 item.
