#!/usr/bin/env bash
# @@Lead event watcher. Blocks until a lane posts to an event file or git
# HEAD moves, then exits so the harness re-invokes @@Lead to process it.
# Re-armed by @@Lead each cycle. Not a product artifact.
set -u
cd /Users/fiorix/dev/github.com/fiorix/chan || exit 2
ph=docs/journals/phase-16
sig() {
  # @@Lead only reacts to actionable signals: a LANE (or @@Host) appending to
  # an event channel, or HEAD moving (a slice committed). @@Lead's OWN lines
  # are filtered out so my acks don't self-trigger the watcher. Mid-work
  # worktree edits are deliberately NOT in the signature (too noisy).
  cat "$ph"/event-lane-*.md "$ph"/event-lead.md 2>/dev/null | grep -v '@@Lead]' | md5
  git rev-parse HEAD 2>/dev/null
}
base="$(sig)"
# ~25 min ceiling (100 * 15s); if nothing changes @@Lead re-arms or re-checks.
for _ in $(seq 1 100); do
  sleep 15
  if [ "$(sig)" != "$base" ]; then
    echo "CHANGE-DETECTED"
    exit 0
  fi
done
echo "NO-CHANGE-TIMEOUT"
exit 0
