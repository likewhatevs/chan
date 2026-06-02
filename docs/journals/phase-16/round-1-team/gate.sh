#!/usr/bin/env bash
# @@Lead isolated batch gate. Gates the COMMITTED main (no shared-tree WIP) in
# a detached worktree so concurrent lane edits can't contaminate fmt/clippy.
# Scoped to the surfaces touched this round (Rust + web + web-marketing);
# gateway-build + build --no-default-features are reserved for the full
# pre-push run before any push. Usage: gate.sh [sha]   (default HEAD)
set -uo pipefail
SHA="${1:-HEAD}"
SRC=/Users/fiorix/dev/github.com/fiorix/chan
WT=/tmp/chan-gate-r1
export CARGO_TARGET_DIR=/tmp/chan-gate-target   # dedicated; warms across runs
cd "$SRC" || exit 2
TARGET_SHA="$(git rev-parse "$SHA")" || exit 2

if git worktree list --porcelain | grep -q "$WT"; then
  git -C "$WT" reset --hard "$TARGET_SHA" >/dev/null 2>&1
  git -C "$WT" clean -fd -e node_modules >/dev/null 2>&1
else
  git worktree add --detach "$WT" "$TARGET_SHA" >/dev/null 2>&1 || exit 3
fi

# Reuse the main tree's node_modules (no package.json changes this round).
for d in web web-marketing; do
  [ -e "$WT/$d/node_modules" ] || ln -s "$SRC/$d/node_modules" "$WT/$d/node_modules" 2>/dev/null
done

cd "$WT" || exit 4
echo "=== GATE @ $(git rev-parse --short HEAD) | start $(date +%H:%M:%S) ==="
step() { echo; echo ">>> $1"; shift; "$@"; local rc=$?; echo "<<< rc=$rc"; return $rc; }

fail=0
step "cargo fmt --check"        cargo fmt --check || fail=1
step "cargo clippy -D warnings" env RUSTFLAGS="-D warnings" cargo clippy --all-targets -- -D warnings || fail=1
step "cargo test --all-targets" env RUSTFLAGS="-D warnings" cargo test --all-targets || fail=1
step "make web-check"           make web-check || fail=1
step "make web-marketing-check" make web-marketing-check || fail=1

echo
echo "=== GATE-RESULT fail=$fail | end $(date +%H:%M:%S) ==="
exit $fail
