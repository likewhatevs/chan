# Channel: @@LaneB -> @@Alex

Append-only. @@LaneB writes here; @@Alex reads. Never edit prior entries.
STANDING GATE: the drive -> workspace scope decisions (tunnel-domain rename,
crate/registry break acceptability, the team-workspace name collision, uniffi
impact) go here and wait for @@Alex ratification BEFORE the codemod.

## 2026-05-27 @@LaneB -> @@Alex
Scope done. 4 decisions to ratify before any codemod. Full doc:
`docs/journals/phase-12/workspace-rename-spec.md` (section 1 has my analysis +
recommendation for each).

1. THE collision (settle first, it changes everything): "workspace" ALREADY
   means 4 things in the code (Cargo `[workspace]`, team workspace, draft
   workspace, Rich Prompt workspace) and all 3 non-Cargo ones are SUB-dirs
   inside a drive. So Drive -> Workspace inverts the hierarchy (a Workspace
   would contain "team workspaces"). Pick one:
     A. Workspace anyway, lean on qualifiers (permanent ambiguity; not rec).
     B. A non-colliding top-level term (Vault / Notebook / Space / ...) -- my
        lean, least churn for a clean namespace, but deviates from the
        backlog's literal "workspace" word so I need your blessing.
     C. Keep Workspace for the drive but rename the 4 incumbents out of the way
        first (more churn, honors the backlog word).

@@Alex: Option C
        

2. Tunnel domain `drive.chan.app` / `{user}.drive.chan.app`: rename or not? My
   rec = NO / decouple. It is DNS + cert + nginx + gateway + marketing, bigger
   than and orthogonal to the code rename; the code renames fine with the
   domain string left alone. Hand any domain rename to the future release lane.

@@Alex: do not rename; keep the same tunnel domain for now

3. On-disk registry (`default_drive_root`, `[[drives]]`, `~/.chan/drives/`) +
   HTTP routes (`/api/drive`): clean break now, or PIN them? My rec = PIN
   (serde/route strings are decoupled from Rust names). Zero-break upgrade for
   existing `~/.chan/config.toml`; the on-disk/route renames become optional
   later passes. PIN also keeps the codemod splittable (see seq below). A full
   clean break is simpler in code but orphans existing registries + bookmarks.

@@Alex we will refactor like the name drive never existed; we are in pre-release phase, we can make breaking changes now or never

4. uniffi: no live bindings exist (no dep, no .udl, no export attr) -- NOT a
   blocker. Future native shells would just link `chan-workspace`. FYI only.

Bonus (FYI, you can defer): is `Library` (the per-machine registry handle) in
scope too, or left as-is? Default = leave it.

@@Alex I don't understand, please clarify

Sequencing depends on #2/#3: if PINNED, I can land backend+desktop EARLY (no
lane blocked, both are frontend) and the frontend chunk LAST after @@LaneA goes
quiescent. If you UNPIN routes/domain, backend+frontend must land together as
one big quiescent-window pass. Detail in spec section 5; @@Architect picks the
actual windows once you rule on 1-3.

## 2026-05-27 @@LaneB -> @@Alex (decisions locked + Library clarified)
Recorded your three rulings: (1) Option C - Workspace = the drive, rename the
4 incumbents out of the way first; (2) keep tunnel domain `drive.chan.app`;
(3) full clean break, refactor as if "drive" never existed (on-disk + routes +
registry all rename, no migration). Updating the spec to match.

Library, in plain terms: it is just the Rust NAME of the parent object that
holds the list of all your registered drives and hands out the per-drive
handle. Today: `Library` owns the registry, `Drive` is one directory. After
the rename `Drive` becomes `Workspace`; the question was only whether the
PARENT keeps the name `Library` or changes too. It has no user-facing effect
(it never appears in the CLI, config, or URLs) - it is purely an internal type
name.

My call (low-stakes, overridable): KEEP `Library`. It reads cleanly as "a
library of workspaces" and renaming it is pure churn with no clarity gain. Its
methods that contain "drive" still rename (`open_drive` -> `open_workspace`,
`default_drive_root` -> `default_workspace_root`, etc.) regardless. If you'd
rather the parent match (e.g. `Workspaces` / `Registry`), say so and I'll fold
it in; otherwise I proceed with `Library` unchanged.

Sequencing implication of ruling 3: a full clean break UNPINS the routes +
on-disk format, so backend and frontend can no longer land fully independently
(the `/api/drive` -> `/api/workspace` rename must land atomically with the
frontend client that calls it). That pushes toward ONE larger quiescent-window
pass after @@LaneA is done, rather than "backend early, frontend late". I am
reworking spec section 5 around that and re-briefing @@Architect.
