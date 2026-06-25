#!/usr/bin/env bash
# Resync sqlx migration checksums against the migration files in
# this repo.
#
# Use this when a previously-applied migration file has been edited
# (typically comments or formatting) and sqlx refuses to start
# with:
#
#   error: migration N was previously applied but has been modified
#
# This is a recovery tool, not a migration runner. sqlx applies
# pending migrations on service start; this script only patches the
# SHA-384 stored in `_sqlx_migrations` so existing rows match the
# current file content.
#
# WARNING: only safe for cosmetic changes (comments, whitespace).
# If the file's schema-affecting SQL changed, write a new forward
# migration instead --- patching the checksum tells sqlx to forget
# the divergence, but it does not run the new statements.
#
# Usage:
#   packaging/gateway/scripts/migrations-resync.sh                  # uses $DATABASE_URL
#   packaging/gateway/scripts/migrations-resync.sh <database-url>   # explicit URL
#   packaging/gateway/scripts/migrations-resync.sh --dry-run        # show diffs only
#   packaging/gateway/scripts/migrations-resync.sh --yes            # skip the prompt
#
# Requires psql in PATH and UPDATE on _sqlx_migrations.

set -euo pipefail

cd "$(git -C "$(dirname "$0")" rev-parse --show-toplevel)/gateway"

DRY_RUN=0
YES=0
DB_URL="${DATABASE_URL:-}"

for arg in "$@"; do
    case "$arg" in
        --dry-run)   DRY_RUN=1 ;;
        --yes|-y)    YES=1 ;;
        --help|-h)
            sed -n '2,/^[^#]/p' "$0" | sed 's/^# \{0,1\}//'
            exit 0
            ;;
        --*) echo "unknown flag: $arg" >&2; exit 2 ;;
        *)   DB_URL="$arg" ;;
    esac
done

if [ -z "$DB_URL" ]; then
    echo "error: set DATABASE_URL or pass a URL as the first argument" >&2
    exit 2
fi

# Linux ships sha384sum; macOS only has shasum. Pick whichever
# exists so the script runs in dev and on the chan-gw host.
if command -v sha384sum >/dev/null 2>&1; then
    sha384() { sha384sum "$1" | awk '{print $1}'; }
elif command -v shasum >/dev/null 2>&1; then
    sha384() { shasum -a 384 "$1" | awk '{print $1}'; }
else
    echo "error: need sha384sum or shasum installed" >&2
    exit 1
fi

if ! command -v psql >/dev/null 2>&1; then
    echo "error: psql not found in PATH" >&2
    exit 1
fi

PSQL=(psql "$DB_URL" -tA -v ON_ERROR_STOP=1)

declare -A applied
while IFS='|' read -r version checksum; do
    [ -z "$version" ] && continue
    applied[$version]="$checksum"
done < <("${PSQL[@]}" -c "SELECT version, encode(checksum, 'hex') FROM _sqlx_migrations ORDER BY version")

if [ ${#applied[@]} -eq 0 ]; then
    echo "no rows in _sqlx_migrations; nothing to resync"
    exit 0
fi

mismatched=0
declare -a updates

for file in migrations/*.sql; do
    [ -e "$file" ] || continue
    base=$(basename "$file")
    # Filename convention is NNNN_description.sql. Strip the prefix
    # and force base-10 parse so leading zeros don't trip arithmetic.
    raw="${base%%_*}"
    version=$((10#${raw}))

    if [ -z "${applied[$version]:-}" ]; then
        # Pending migration: sqlx will apply it on next service
        # start. Nothing for us to do.
        continue
    fi

    file_hex=$(sha384 "$file")
    db_hex="${applied[$version]}"
    if [ "$file_hex" = "$db_hex" ]; then
        continue
    fi

    mismatched=$((mismatched + 1))
    printf '%s (version %s):\n  db   = %s\n  file = %s\n' \
        "$base" "$version" "$db_hex" "$file_hex"
    updates+=("$version|$file_hex|$base")
done

if [ "$mismatched" -eq 0 ]; then
    echo "all applied migrations match files; nothing to do"
    exit 0
fi

echo
echo "mismatched: $mismatched"

if [ "$DRY_RUN" -eq 1 ]; then
    exit 0
fi

if [ "$YES" -eq 0 ]; then
    read -rp "patch these checksums in _sqlx_migrations? [y/N] " ans
    case "$ans" in
        y|Y|yes|YES) ;;
        *) echo "aborted"; exit 1 ;;
    esac
fi

for entry in "${updates[@]}"; do
    IFS='|' read -r version new_hex base <<< "$entry"
    "${PSQL[@]}" -c \
        "UPDATE _sqlx_migrations SET checksum = decode('${new_hex}', 'hex') WHERE version = ${version}" \
        >/dev/null
    echo "  patched $base (version $version)"
done

echo
echo "done. restart the chan-gateway services to pick up any pending migrations."
