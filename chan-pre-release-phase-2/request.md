# Chan pre-release phase 2
List of items for this phase of product hardening, correctness, and UX impact.

## Context
Everyone should review [[chan-pre-release-phase-1/summary.md]] before starting work, so we carry forward the lessons, unresolved follow-ups, and execution notes from phase 1.

Alex is the host for this phase. Assistants should come to Alex when they need product direction, permission, or a decision that cannot be made from the written plan. Otherwise, coordination should happen through task files owned by @@Architect.

## Team profiles
These profiles describe ownership and expected judgment. They are separate from the process so we can keep responsibilities clear while still letting the execution process evolve.

### @@Architect
@@Architect is the project owner and is accountable for delivering this phase to completion.

Responsibilities:

* Own the phase plan, task breakdown, and coordination across assistants.
* Own [[chan-pre-release-phase-2/journal.md]] and keep it current throughout the work.
* Copy the requested work into the journal as a trackable checklist, and update completion status as tasks land.
* Create and assign task files using the {assistant-name}-{n}.md pattern.
* Make sure assistants use wiki links when referring to project documents, task files, summaries, journals, and related notes.
* Make sure new code has appropriate tests, and end-to-end coverage when applicable.
* Request hardening review from the right specialist before work is considered complete.
* Coordinate commits through task files, including commit readiness, commit message review, and ensuring messages follow the repository's existing style.
* Produce [[chan-pre-release-phase-2/summary.md]] at the end of the phase.

The final summary must include:

* Outcome and completion status.
* Highlights.
* Lowlights.
* Bugs found and fixed.
* Test and hardening coverage.
* Remaining follow-ups.
* A dedicated section ranking the quality and efficacy of the assistants, with constructive feedback for each one.

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
* Consolidate duplicate server or reload requests when multiple assistants ask for similar work.
* Report test-server state, failures, restarts, and useful observations back through task files.

The test service is allowed to crash during development, but crashes should be captured and routed back to the relevant owner when they reveal product or test issues.

### @@Syseng
@@Syseng owns low-level correctness, hardening, security-sensitive behavior, filesystem semantics, process behavior, indexing reliability, and operational validation.

@@Syseng should review work that touches persistence, filesystems, symlinks, hardlinks, process management, platform integration, or failure handling.

### @@Rustacean
@@Rustacean owns Rust quality, Cargo hygiene, dependency discipline, idiomatic implementation, build health, and the Rust test suite.

@@Rustacean should review work that changes Rust APIs, data structures, async behavior, error handling, dependency choices, or shared backend logic.

## Process
All coordination for this phase should happen through markdown files in [[chan-pre-release-phase-2]]. Use @@mentions for assistant ownership and wiki links for related files so the graph stays useful.

### Startup
1. @@Architect reviews [[chan-pre-release-phase-1/summary.md]].
2. @@Architect creates or updates [[chan-pre-release-phase-2/journal.md]].
3. @@Architect copies the requested work into the journal as a checklist with clear owners and status.
4. @@Architect creates initial task files using the {assistant-name}-{n}.md naming pattern.
5. Each assistant reviews the task assigned to them, follows wiki links for context, and records progress in that task file.

### Task files
Task files are the unit of coordination. Each task should include:

* Owner using an @@mention.
* Current status.
* Goal.
* Relevant wiki links.
* Acceptance criteria.
* Test expectations.
* Hardening or review expectations, if applicable.
* Progress notes.
* Completion notes.
* Commit readiness notes, when the task is ready to land.

Assistants should update their task file as they work. They should link to related tasks, journal entries, summaries, and source notes with wiki links instead of relying on unlinked prose.

### Completion flow
When an assistant completes a task, they must:

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

### Commit coordination
Commits should be coordinated through task files.

Before committing, the owner should record:

* Files changed.
* Tests run.
* Review or hardening performed.
* Known risks.
* Proposed commit message.

@@Architect should check that commit messages match the repository's existing style and that related work is committed in coherent units.

## Work items

### Graph
- [ ] On scope of single file, the depth slider shouldn't go beyond 1; on scope of N files, shouldn't go beyond N; on folders, depends on how many sub-folders; whole drive should know the max-depth
- [ ] On Scope: Folder (and possibly others) the documents are plotted stacked over each other in the graph... they need to have force against each other instead
- [ ] When we click nodes in the graph, some of them show a message in the graph's inspector: "not in the current file listen (try reload / chan index)
  - [ ] We must use the filesystem as source of truth before plotting; we shouldn't show files that don't actually exist
  - [ ] If a file or directory is deleted from the fs during open graph, we should "ghost" it in the plot
  - [ ] If a new file or directory is created while the graph is open, and it fits in the current filter, we should update it and add the new nodes
### Editor
- [ ] The indentation of the enumerated lists need some kind of vertical visual guidance during edits, because it's hard to figure out how bullets align vertically
  - [ ] I'm ok if we double the indent spaces
- [ ] Graph -> Graph this
- [ ] Files -> Show File
### Search
- [ ] We should only index #tags from markdown files, not from ANY other files - e.g. today i think we index #tags from include in source code etc... wrong, bad bad
- [ ] In the Search Status's code report, we should have a link to Graph This using the schema below
- [ ] In the search results we currently show 1 entry per section (heading) often resulting in various entries to the same file; let's collapse the results per file and rank the headings within the file and use the first result
- [ ] When we click on a search result that links to a heading, the inspector is empty, no details; it should show the details of the file, not the heading section
- [ ] The search overlay window has the inspector outside the main search area, that is wrong; the inspector's details hide button should be under the overlay's close button
### Search / Code / Graph: we've already elevated "language" for search, now let's do for grah as well
  - [ ] This means in the search status having a Graph This for the whole drive, max depth
  - [ ] The graph will elevate languages to nodes, and they will only connect to folders, ranked by folders with the most files of that language (use rank to define depth)
  - [ ] In the graph overlay, we should have Language as a filter too

