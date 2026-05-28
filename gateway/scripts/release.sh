#!/usr/bin/env bash
# Bump the workspace version, commit, tag and push in one shot.
#
# Usage:
#   scripts/release.sh 0.1.0          # creates v0.1.0 from current HEAD
#
# Refuses to run if:
#   - the working tree is dirty;
#   - the version doesn't look like x.y.z[-suffix];
#   - the tag already exists locally or on origin.
#
# The release workflow (.github/workflows/release.yml) runs the same
# tag/version equality check, so a mismatch here aborts before any
# build cost.

set -euo pipefail

cd "$(dirname "$0")/.."

if [ $# -ne 1 ]; then
    echo "usage: $0 <version>   (e.g. 0.1.0)" >&2
    exit 2
fi

VERSION="$1"
TAG="v$VERSION"

if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]]; then
    echo "error: version must look like x.y.z or x.y.z-suffix" >&2
    exit 1
fi

if [ -n "$(git status --porcelain)" ]; then
    echo "error: working tree is dirty; commit or stash first" >&2
    git status --short >&2
    exit 1
fi

if git rev-parse "$TAG" >/dev/null 2>&1; then
    echo "error: tag $TAG already exists locally" >&2
    exit 1
fi
if git ls-remote --tags --exit-code origin "refs/tags/$TAG" >/dev/null 2>&1; then
    echo "error: tag $TAG already exists on origin" >&2
    exit 1
fi

# Update [workspace.package].version in the root Cargo.toml. The
# pattern is anchored to the [workspace.package] table to avoid
# touching dep specs that happen to match.
python3 - "$VERSION" <<'PY'
import re, sys
new_version = sys.argv[1]
path = "Cargo.toml"
text = open(path).read()
pattern = re.compile(
    r'(\[workspace\.package\][^\[]*?\nversion = ")[^"]+(")',
    re.DOTALL,
)
text2, n = pattern.subn(rf'\g<1>{new_version}\g<2>', text, count=1)
if n != 1:
    sys.exit(f"could not find [workspace.package] version in {path}")
open(path, "w").write(text2)
print(f"bumped workspace version -> {new_version}")
PY

# cargo update on the lockfile so versions of our local crates also
# reflect the new version (they read from workspace.package.version).
cargo update -p profile -p identity -p workspace-proxy >/dev/null

git add Cargo.toml Cargo.lock
git commit -m "release: $TAG"
git tag -a "$TAG" -m "$TAG"

echo
echo "Created commit + tag $TAG. Push with:"
echo "  git push origin main $TAG"
