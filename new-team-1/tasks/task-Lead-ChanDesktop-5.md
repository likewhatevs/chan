# task-Lead-ChanDesktop-5 — authorizations for your task-1 flags

From: @@Lead. To: @@ChanDesktop. Resolves the decision flags in
task-ChanDesktop-Lead-1.md. Priority order for your queue:
task-3 (file-drop, severity high — start the empirical
DOM-vs-native test + contract agreement with @@Chan early, it gates
their half) > task-4 (bundle rename — NOTE: @@Alex has CLEARED the
rename; task-4 supersedes task-2's "do not change") > this task.

## Flag 1 + 4: vestigial features plumbing — AUTHORIZED, drop it

Pre-release posture applies. Remove `WorkspaceSettings.features`,
the write-only mirror write, and the dead `features` param on
`add_workspace`. If the `cfg.workspaces` map holds nothing else
after that, remove the map too. Delete the features-related serde
legacy defaults and their tests with it. `zoom_level`'s
missing-field default is your judgment call: if it exists only to
parse pre-field config.json files, drop it per no-backcompat; if it
doubles as the natural default for a fresh config, keep it.

Same commit, mechanical cross-boundary edit AUTHORIZED: update
docs/config-reference.md — remove the three `workspaces.*` rows from
the Desktop Config table and delete Open Finding #1 (the
mirror-drift finding; removing the mirror resolves it).

## Flag 2: desktop/release-review.md — DELETE

You verified zero inbound references; it reviews an architecture
that no longer exists and cites the dead chan-writer org. Git
history preserves it. `git rm` it as its own commit. This also
closes the release-review half of your task-4 chanwriter sweep.

## Flag 3: updater-bridge.md — SHRINK, your judgment

Keep the file (design.md and .agents/desktop.md cite it). Cut it to
the still-relevant halves (key identification + failure modes); the
one-time DEV→prod bridge narrative can go if every live install is
past it — @@Alex's is, and he's the only desktop user.

## Flag 5: .agents/desktop.md — handled

Mine; already scrubbed in b3202497 and rg-verified clean. Its
agent-role subject matter is in-scope for that doc's purpose.

Completion: fold into your next completion file. Gates as usual
(`-p chan-desktop` + fmt), config-reference edit rides your commit.
