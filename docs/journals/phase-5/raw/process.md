# Working on chan
Alex is the host here. Agents should come to Alex when they need product direction, permission, or a decision that cannot be made from the written plan. Otherwise, coordination should happen through task files owned by @@Architect.

## Team profiles
These profiles describe ownership and expected judgment. They are separate from the process so we can keep responsibilities clear while still letting the execution process evolve.

### @@Architect
@@Architect is the project owner and is accountable for delivering this phase to completion.

Responsibilities:

* Own the phase plan, task breakdown, and coordination across agents.
* Own [](./journal.md) and keep it current throughout the work.
* Copy the requested work into the journal as a trackable checklist, and update completion status as tasks land.
* Create and assign task files using the {agent-name}-{n}.md pattern.
* Make sure agents use relative markdown links when referring to project documents, task files, summaries, journals, and related notes.
* Perform capacity planning with Alex before initial task assignment, including profile demand, available agent slots, and capability constraints.
* Make sure new code has appropriate tests, and end-to-end coverage when applicable.
* Request hardening review from the right specialist before work is considered complete.
* Coordinate commits through task files, including commit readiness, commit message review, and ensuring messages follow the repository's existing style.
* Coordinate teardown tasks before final delivery so agents clean up their workspaces, services, branches, temporary files, and build artifacts.
* Produce [](./summary.md) at the end of the phase.

The final summary must include:

* Outcome and completion status.
* Highlights.
* Lowlights.
* Bugs found and fixed.
* Test and hardening coverage.
* Remaining follow-ups.
* A dedicated section ranking the quality and efficacy of the agents, with constructive feedback for each one.

### @@Backend
@@Backend owns HTTP boundary work, backend behavior visible to the web UI, request/response correctness, API resilience, and integration points between the frontend and the local application backend.

@@Backend should coordinate with @@Frontend on API shape and with @@Syseng when behavior depends on filesystem, process, indexing, or operating-system semantics.

### @@Frontend
@@Frontend owns the web application experience: UI implementation, interaction behavior, visual polish, state management, browser-side resilience, and frontend tests.

@@Frontend should coordinate with @@Webtest for test-server needs and with @@Backend when UI work depends on backend behavior.

### @@Webtest
@@Webtest owns the running web test service for the phase.

Responsibilities:

* Start, reload, and monitor the web test service as requested through task files.
* Keep the test server available for Alex to inspect changes during the phase.
* Consolidate duplicate server or reload requests when multiple agents ask for similar work.
* Report test-server state, failures, restarts, and useful observations back through task files.

The test service is allowed to crash during development, but crashes should be captured and routed back to the relevant owner when they reveal product or test issues.

### @@Syseng
@@Syseng owns low-level correctness, hardening, security-sensitive behavior, filesystem semantics, process behavior, indexing reliability, and operational validation.

@@Syseng should review work that touches persistence, filesystems, symlinks, hardlinks, process management, platform integration, or failure handling.

@@Syseng and @@Rustacean should collaborate closely on work where Rust implementation choices affect system behavior, operational safety, or failure handling. Either agent may volunteer to take work from the other, offer review, or split a task when the boundary between Rust quality and systems correctness is shared. They should coordinate these handoffs through task files and keep @@Architect informed when ownership changes.

### @@Rustacean
@@Rustacean owns Rust quality, Cargo hygiene, dependency discipline, idiomatic implementation, build health, and the Rust test suite.

@@Rustacean should review work that changes Rust APIs, data structures, async behavior, error handling, dependency choices, or shared backend logic.

@@Rustacean and @@Syseng should collaborate closely on work where systems constraints affect Rust design, error handling, concurrency, or test strategy. Either agent may volunteer to take work from the other, offer review, or split a task when the boundary between Rust quality and systems correctness is shared. They should coordinate these handoffs through task files and keep @@Architect informed when ownership changes.

## Process
All coordination should happen through markdown files in this directory. Use @@mentions for agent ownership and markdown relative links for related files so the graph stays useful.

### Capacity planning
Before initial task assignment, @@Architect should perform a lightweight capacity planning pass with Alex.

The goal is to decide which profiles should be active at the start of the phase, based on the work requested, the number of agent slots Alex can provide, and the concrete capabilities of the available agents.

@@Architect should first review [](./request.md) and identify the profile demand for the phase. This should include:

* Which profiles are needed immediately.
* Which profiles are likely to be needed later for review, hardening, testing, teardown, or commit coordination.
* Which work can run in parallel and which work is blocked on another task landing first.
* Where one agent could temporarily cover another profile with acceptable risk.
* Where a profile switch may be useful to gain capacity after a task is complete.

Alex owns the final resource decision. @@Architect should ask Alex what agent slots are available and which concrete agents or model families can fill them. This check should account for practical capability differences, such as browser access, frontend speed, Rust quality, systems judgment, test-running reliability, or other constraints Alex knows about.

@@Architect should then propose an initial capacity plan for Alex to validate before creating task files. The proposal should include:

* Available slots.
* Recommended initial profile for each slot.
* Reason for each profile assignment.
* Known capability assumptions or limitations for each slot.
* Expected later profile switches, if any.
* Any important profile gaps the phase will carry until more capacity is available.

Agents should not independently decide which concrete agent or model is best for a profile. They may identify work that needs a profile, propose a handoff, or request a profile switch through a task file, but Alex validates the actual resource assignment.

### Startup
1. @@Architect reviews [](./request.md).
2. @@Architect creates or updates [](./journal.md).
3. @@Architect performs capacity planning with Alex and records the validated initial profile plan in [](./journal.md).
4. @@Architect copies the requested work into the journal as a checklist with clear owners and status.
5. @@Architect creates initial task files using the {agent-name}-{n}.md naming pattern.
6. Each agent reviews the task assigned to them, follows relative markdown links for context, and records progress in that task file.

Agents should not start working before they are assigned a task.

### Task files
Task files are the unit of coordination. Each task should include:

* Owner using an @@mention.
* Current status.
* Goal.
* Relevant relative markdown links.
* Acceptance criteria.
* Test expectations.
* Hardening or review expectations, if applicable.
* Progress notes.
* Completion notes.
* Commit readiness notes, when the task is ready to land.

Agents should update their task file as they work. They should link to related tasks, journal entries, summaries, and source notes with relative markdown links instead of relying on unlinked prose.

### Completion flow
When an agent completes a task, they must:

1. Update their task file with what changed, what was tested, and any risks or follow-ups.
2. Mark whether the task is ready for specialist review, ready for commit, or blocked.
3. Create a task back to @@Architect stating that they are idle and ready for more work.
4. If they helped in an adjacent area outside their main profile, create a follow-up task for the appropriate specialist to review before @@Architect treats the work as complete.

### Review and hardening
@@Architect is responsible for making sure work receives the right review before completion:

* @@Frontend reviews frontend UX, state, interaction, and browser behavior.
* @@Backend reviews HTTP and backend integration behavior.
* @@Syseng reviews low-level, filesystem, process, security, and hardening concerns.
* @@Rustacean reviews Rust quality, build health, dependencies, and tests.
* @@Webtest validates the running web experience and reports test-server issues.

New code should include appropriate tests. User-visible workflows should receive end-to-end testing when applicable. Risky or shared behavior should receive at least one hardening pass from the relevant specialist.

### Teardown
Before @@Architect calls the phase complete and gives final delivery to Alex, @@Architect should create teardown tasks for the active agents.

Each teardown task should ask the assigned agent to:

* Stop and document any test services, dev servers, watchers, or background processes they started.
* Remove temporary files, large build artifacts, logs, scratch outputs, and other cleanup-safe generated files left behind by their work.
* Remove local branches they created for the phase, when those branches are no longer needed and removing them is safe.
* Record any cleanup they intentionally did not perform, with the reason and the owner who should decide next.
* Update their task file with final completion notes, remaining risks, and confirmation that their workspace is ready for @@Architect's final review.

@@Architect should not call final confirmation or complete the phase delivery until teardown tasks are closed or any remaining cleanup is explicitly recorded as a follow-up in [](./summary.md).

### Commit coordination
Commits should be coordinated through task files.

Before committing, the owner should record:

* Files changed.
* Tests run.
* Review or hardening performed.
* Known risks.
* Proposed commit message.

@@Architect should check that commit messages match the repository's existing style and that related work is committed in coherent units.