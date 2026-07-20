# Release v0.72.1 - the AUR publication patch

Patch round run 2026-07-20 off the v0.72.0 tag. Two packaging fixes: the Arch AUR publication path, which v0.72.0 could not complete, and an early EL9 refusal in the chan-desktop RPM spec. Diagnosis, verification, and the release close were integrator-owned. Cut straight to GA with no rc.

## What shipped

- **The AUR post-install verification runs with the privileges it needs.** `systemd-analyze verify` creates `/run/systemd/` when that directory is missing. The AUR post-install smoke runs as an unprivileged user, and in a container `/run` is root-owned and not a separate mount, so the call failed with "Failed to create directory '/run/systemd/': Permission denied" whenever the directory was absent. That blocked the AUR push for both `chan` and `chan-desktop`; the GitHub release, COPR, the PPA, and the Docker images were unaffected, and v0.72.0 never reached the AUR at all. The verification now runs through sudo, which keeps it enforced rather than skipped. A sudoers rule covering only pacman was insufficient, because the call then prompts for a password, so the drop-in grants `systemd-analyze` as well. The grant is confined; other commands remain denied.
- **The chan-desktop spec fails fast on EL9.** EPEL Next 9 provides neither `webkit2gtk4.1-devel` nor `libsoup3-devel`, so the desktop shell cannot build there. The spec now refuses any EL release older than 10 with a message naming both packages, instead of failing deep in dependency resolution.
- **The AUR `chan-desktop` ldd check is gone.** It ran in the container that had just linked the binary, with every makedepend installed, so a soname it could report as missing would have failed the link first. namcap already covers that condition at error class with an empty waiver list, and reads the package rather than the container. Not a user-facing change; it removed a check that could not fail.

## Team / process

Integrator-owned, with one agent per fix and an adversarial verifier for each. Three commits on packaging files plus the removal of a temporary CI diagnostic workflow. The AUR failure was reproduced in a container before the fix rather than reasoned about, which is what identified the missing directory as the trigger.

Two things the implementing agents caught that the brief had wrong. The prescribed fix was incomplete: the sudoers drop-in granted pacman alone, so the call it was meant to enable still prompted for a password, and the rule had to name `systemd-analyze` too. And the ldd check was dead for a deeper reason than the one already fixed, which is what retired it rather than repairing it again.

The two fix agents were also run concurrently in a single worktree. One agent's commit swept the other's staged files, and a later commit reverted them; nothing had been pushed, so the work was reset and rebuilt as four commits each carrying only its own files. The standing rule is one worktree per lane, and it applies to a two-lane patch round exactly as it applies to a delivery round.

## Validation

- **Root cause reproduced** in `docker.io/archlinux/archlinux:base-devel`: unprivileged with `/run/systemd/` absent gives rc=1 with the permission-denied message; with the directory present, rc=0; as root with it absent, rc=0. Earlier attempts to reproduce all passed because an earlier root-run `pacman` had already created the directory.
- **Fix verified in the same container**: with the directory absent the fixed command returns rc=0, and on a deliberately broken unit it still returns rc=1 naming the fault, so the check retains its ability to fail.
- **EL9 guard verified with `rpmspec` in `fedora:41`**: Fedora with `rhel` undefined parses, EL10 parses, EL9 and EL8 fail with the message naming both packages. `chan.spec` is unaffected.

## Retrospective

### Highlights

- The fix keeps the verification enforced instead of skipping it where systemd is not booted, and it was proven still able to fail by breaking a unit on purpose. That is the counter to the v0.72.0 lowlight about checks that cannot go red.
- The reproduction explains why the earlier attempts were misleading rather than leaving them unexplained: the state that hid the bug was created by a root-run `pacman` earlier in the same container.

### Lowlights

- The condition depends on container state that varies with what ran before, so a green run was not evidence of a working check. It took a controlled matrix over the directory's presence and the running user to see it.
- The first attempt at a diagnostic probe reported success without running anything: `docker run` leaves stdin closed unless `-i` is given, so the heredoc carrying the probe never reached the shell it was written for. A marker proving the body executed was the fix, and the same standard applies to a diagnostic as to a test.
- Running two agents in one worktree cost a reset and a rebuild of the commits. The failure mode is not exotic and was already written down; it was applied to delivery rounds and not to a two-fix patch.

### Honest feedback

The AUR CI path cannot be validated before the release that carries it, so a packaging fix of this shape is exercised for the first time by the tag that ships it. That is a structural property of the path, not an oversight, and it is worth stating plainly rather than implying the fix rode a green pipeline.

## Follow-ups

- The AUR install-and-smoke still runs in the build container, so it cannot catch a runtime dependency that is present as a makedepend but missing from the runtime depends array. namcap covers that today; a clean-container install and smoke, mirroring what the COPR check already does for RPMs, would test it directly.
- The AUR CI path cannot be validated before the release that carries it: the validation job checks out the release tag, and dispatching it also requires a tag, so a fix to that path is first exercised by the tag that ships it.
- The chan-desktop EL9 exclusion in the COPR project is console state that no API reports. The spec guard now fails fast if it lapses, but the exclusion itself still cannot be asserted from the repository.
