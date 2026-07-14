#!/usr/bin/env bash
# gateway-zone.sh -- full-stack gateway e2e: multi-devserver routing,
# entry mints, cap enforcement, and tunnel reconnect against the REAL
# identity + profile + devserver-proxy services and REAL `chan
# devserver` processes.
#
# Topology: everything on host loopback, one process per service,
# `localtest.me` wildcard DNS (public A/AAAA -> 127.0.0.1/::1) for the
# devserver hosts. The devservers run host-local rather than in sdme
# zone containers: on this host the kernel firewall drops
# container->host TCP (see packaging/gateway/scripts/dev/sdme/
# devserver-tunnel-e2e/zone-isolation-probe.sh), so a zone topology
# would need the whole stack including Postgres inside the zone. The
# multi-devserver semantics under test are identical either way.
#
# Requirements:
#   - a running Postgres reachable at $E2E_DATABASE_URL (default
#     postgres://chan:chan@127.0.0.1:5432/chan_gateway_test); the
#     harness isolates itself in schema $E2E_SCHEMA of that database
#     and drops it on every run,
#   - node + npm on PATH (the SQL helper self-installs the `pg`
#     package into the work dir once),
#   - gateway service binaries + the chan binary; missing ones are
#     cargo-built on demand,
#   - internet DNS for localtest.me (or a resolver override).
#
# Usage:
#   scripts/e2e/gateway-zone.sh            # run everything
#   E2E_KEEP=1 scripts/e2e/gateway-zone.sh # leave the stack running
#
# Output: plain-text assertion log at $WORK/assertions.log (echoed),
# service logs under $WORK/logs/. Exit 0 iff every assertion passed.
set -uo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
WORK="${E2E_WORK:-$REPO/target/gateway-zone-e2e}"
LOGS="$WORK/logs"
E2E_DATABASE_URL="${E2E_DATABASE_URL:-postgres://chan:chan@127.0.0.1:5432/chan_gateway_test}"
E2E_SCHEMA="${E2E_SCHEMA:-gateway_zone_e2e}"
E2E_KEEP="${E2E_KEEP:-}"

# Fixed loopback ports; override only on collision.
ID_PORT="${E2E_ID_PORT:-17800}"
PROFILE_PORT="${E2E_PROFILE_PORT:-17801}"
PROXY_PORT="${E2E_PROXY_PORT:-17802}"
TUNNEL_PORT="${E2E_TUNNEL_PORT:-17810}"
OAUTH_PORT="${E2E_OAUTH_PORT:-17830}"
DS_PORTS=(17821 17822 17823)

# Headless Chrome for the consent-flow asserts (the puppeteer cache
# layout, overridable).
CHROME_BIN="${E2E_CHROME_BIN:-$(ls -d "$HOME"/.cache/puppeteer/chrome/linux-*/chrome-linux64/chrome 2>/dev/null | head -1)}"
ALICE_EMAIL="e2e-alice@example.com"

DOMAIN=localtest.me
ID_HOST="id.$DOMAIN:$ID_PORT"
APEX="devserver.$DOMAIN"

# Per-run shared secrets (the stack is torn down with the run).
GATE_SECRET="e2e-gate-secret-0123456789abcdef0123456789abcdef"
TOK_PROFILE="e2e-profile-bearer"
TOK_INTERNAL="e2e-internal-bearer"
TOK_ADMIN="e2e-admin-bearer"
# The tunnel-server cap under test: two devservers register, a third
# is refused.
MAX_DEVSERVERS=2

ASSERT_LOG="$WORK/assertions.log"
FAILURES=0
PIDS=()

log() { printf '%s\n' "$*"; }
assert_pass() { printf 'PASS %s\n' "$*" | tee -a "$ASSERT_LOG"; }
assert_fail() {
    printf 'FAIL %s\n' "$*" | tee -a "$ASSERT_LOG"
    FAILURES=$((FAILURES + 1))
}

cleanup() {
    [ -n "$E2E_KEEP" ] && {
        log "E2E_KEEP set: stack left running (pids: ${PIDS[*]:-none})"
        return
    }
    for pid in "${PIDS[@]:-}"; do
        [ -n "$pid" ] && kill "$pid" 2>/dev/null
    done
    wait 2>/dev/null
}
trap cleanup EXIT

# ---------------------------------------------------------------
# Tooling
# ---------------------------------------------------------------

need() { command -v "$1" >/dev/null || { log "missing required tool: $1"; exit 2; }; }
need curl
need node
need npm
need openssl
need basenc
need sha256sum

mkdir -p "$WORK" "$LOGS"
: > "$ASSERT_LOG"

# Kill leftovers from a previous run (idempotency): every process we
# spawn records a pidfile under $WORK/pids.
mkdir -p "$WORK/pids"
for f in "$WORK"/pids/*.pid; do
    [ -e "$f" ] || continue
    pid="$(cat "$f")"
    kill "$pid" 2>/dev/null && log "killed leftover $(basename "$f" .pid) (pid $pid)"
    rm -f "$f"
done

# Belt and braces: free every port the run binds. A stray listener
# (e.g. a hand-started debug service) would otherwise swallow the
# traffic while our own spawn dies on the bind conflict.
free_port() { # free_port <port>
    local pids
    pids="$(ss -ltnp 2>/dev/null | grep ":$1 " | grep -oP 'pid=\K[0-9]+' | sort -u)"
    local pid
    for pid in $pids; do
        kill "$pid" 2>/dev/null && log "freed port $1 (killed stray pid $pid)"
    done
}
for p in "$ID_PORT" "$PROFILE_PORT" "$PROXY_PORT" "$TUNNEL_PORT" "$OAUTH_PORT" "${DS_PORTS[@]}"; do
    free_port "$p"
done

# SQL + browser helpers: node deps self-installed into the work dir
# once (pg for seeding, puppeteer-core to drive the host Chrome).
if [ ! -d "$WORK/node_modules/pg" ] || [ ! -d "$WORK/node_modules/puppeteer-core" ]; then
    log "installing pg + puppeteer-core into $WORK (once)"
    (cd "$WORK" && npm install --no-fund --no-audit --loglevel=error pg puppeteer-core >/dev/null) || {
        log "npm install pg puppeteer-core failed"
        exit 2
    }
fi
cat > "$WORK/sql.mjs" <<'EOF'
// Tiny SQL runner: node sql.mjs <database-url> "<sql>" -- executes one
// statement batch, prints rows as tab-separated values.
import pg from "pg";
const [url, sql] = process.argv.slice(2);
const c = new pg.Client({ connectionString: url });
await c.connect();
try {
    const r = await c.query(sql);
    const rows = Array.isArray(r) ? r.flatMap((x) => x.rows) : r.rows;
    for (const row of rows) console.log(Object.values(row).join("\t"));
} finally {
    await c.end();
}
EOF
sql() { node "$WORK/sql.mjs" "$1" "$2"; }

# ---------------------------------------------------------------
# Binaries (cargo-build on demand; warm target = cheap no-op)
# ---------------------------------------------------------------

# E2E_GW_BIN / E2E_CHAN_BIN point at prebuilt binaries (e.g. built
# from a committed ref while the working tree holds in-flight edits).
GW_BIN="${E2E_GW_BIN:-$REPO/gateway/target/debug}"
CHAN_BIN="${E2E_CHAN_BIN:-$REPO/target/debug/chan}"
for b in identity-service profile-service devserver-proxy-service; do
    [ -x "$GW_BIN/$b" ] || (cd "$REPO/gateway" && cargo build -p "${b%-service}" >/dev/null) || exit 2
done
# --no-default-features drops the ML/embeddings deps; the devserver
# serves workspaces without them (same shape the pre-push no-default
# build checks).
[ -x "$CHAN_BIN" ] || (cd "$REPO" && cargo build -p chan --no-default-features >/dev/null) || exit 2

# ---------------------------------------------------------------
# Database: fresh schema inside the existing database
# ---------------------------------------------------------------

log "resetting schema $E2E_SCHEMA"
sql "$E2E_DATABASE_URL" \
    "DROP SCHEMA IF EXISTS $E2E_SCHEMA CASCADE; CREATE SCHEMA $E2E_SCHEMA;" || exit 2
# Services reach the schema via search_path in the URL options; both
# identity and profile run their migrations into it on boot.
DB_URL="$E2E_DATABASE_URL?options=-csearch_path%3D$E2E_SCHEMA"

# ---------------------------------------------------------------
# Services
# ---------------------------------------------------------------

spawn() { # spawn <name> <cmd...>
    local name="$1"
    shift
    "$@" > "$LOGS/$name.log" 2>&1 &
    local pid=$!
    PIDS+=("$pid")
    echo "$pid" > "$WORK/pids/$name.pid"
    log "started $name (pid $pid)"
}

wait_http() { # wait_http <name> <url> [tries]
    local name="$1" url="$2" tries="${3:-50}"
    for _ in $(seq "$tries"); do
        if curl -fsS -o /dev/null --max-time 2 "$url"; then return 0; fi
        sleep 0.2
    done
    log "$name did not answer at $url; last log lines:"
    tail -5 "$LOGS/$name.log" || true
    return 1
}

# Stub GitHub: identity's provider endpoints point here so the
# browser phase can sign in without real OAuth.
spawn stub-oauth node "$REPO/scripts/e2e/stub-oauth.mjs" "$OAUTH_PORT" "$ALICE_EMAIL"

spawn profile env \
    BIND_ADDR="127.0.0.1:$PROFILE_PORT" \
    DATABASE_URL="$DB_URL" \
    PROFILE_AUTH_TOKEN="$TOK_PROFILE" \
    DEVSERVER_ADMIN_TOKEN="$TOK_ADMIN" \
    DEVSERVER_ADMIN_URL="http://127.0.0.1:$PROXY_PORT" \
    RUST_LOG=info \
    "$GW_BIN/profile-service"

spawn identity env \
    BIND_ADDR="127.0.0.1:$ID_PORT" \
    BASE_URL="http://$ID_HOST" \
    DATABASE_URL="$DB_URL" \
    PROFILE_SERVICE_URL="http://127.0.0.1:$PROFILE_PORT" \
    PROFILE_AUTH_TOKEN="$TOK_PROFILE" \
    IDENTITY_INTERNAL_TOKEN="$TOK_INTERNAL" \
    DEVSERVER_GATE_SECRET="$GATE_SECRET" \
    DEVSERVER_ADMIN_TOKEN="$TOK_ADMIN" \
    DEVSERVER_ADMIN_URL="http://127.0.0.1:$PROXY_PORT" \
    CHAN_DOMAIN="$DOMAIN" \
    PUBLIC_SCHEME=http \
    DEVSERVER_PUBLIC_SCHEME=http \
    DEVSERVER_PUBLIC_PORT=":$PROXY_PORT" \
    GITHUB_CLIENT_ID=e2e-dummy \
    GITHUB_CLIENT_SECRET=e2e-dummy \
    IDENTITY_OAUTH_ENDPOINTS_BASE="http://127.0.0.1:$OAUTH_PORT" \
    RUST_LOG=info \
    "$GW_BIN/identity-service"

spawn proxy env \
    BIND_ADDR="127.0.0.1:$PROXY_PORT" \
    TUNNEL_BIND_ADDR="127.0.0.1:$TUNNEL_PORT" \
    IDENTITY_URL="http://127.0.0.1:$ID_PORT" \
    IDENTITY_INTERNAL_TOKEN="$TOK_INTERNAL" \
    DEVSERVER_GATE_SECRET="$GATE_SECRET" \
    DEVSERVER_ADMIN_TOKEN="$TOK_ADMIN" \
    DASHBOARD_URL="http://$ID_HOST/workspaces" \
    CHAN_DOMAIN="$DOMAIN" \
    PUBLIC_SCHEME=http \
    FORWARDED_PROTO=http \
    MAX_DEVSERVERS_PER_USER="$MAX_DEVSERVERS" \
    RUST_LOG=info \
    "$GW_BIN/devserver-proxy-service"

wait_http profile "http://127.0.0.1:$PROFILE_PORT/healthz" || exit 2
wait_http identity "http://127.0.0.1:$ID_PORT/healthz" || exit 2
wait_http proxy "http://127.0.0.1:$PROXY_PORT/healthz" || exit 2

# healthz answering is not enough: a stray listener could serve it
# while our own spawn died on the bind conflict. The spawned pid
# itself must still be alive.
require_alive() { # require_alive <name>
    local pid
    pid="$(cat "$WORK/pids/$1.pid")"
    if ! kill -0 "$pid" 2>/dev/null; then
        log "$1 (pid $pid) died at startup; log tail:"
        tail -5 "$LOGS/$1.log" || true
        exit 2
    fi
}
for svc in stub-oauth profile identity proxy; do
    require_alive "$svc"
done
log "stack is up"

# ---------------------------------------------------------------
# Seed: one user, three PATs (A/B live devservers, C for the cap)
# ---------------------------------------------------------------
# PAT shape mirrors identity::api_tokens::generate_token: secret =
# chan_pat_<base64url(32B)>, stored hash = base64url(sha256(secret)),
# devserver id = lowercase hex sha256(secret). Once chan-gateway-admin
# grows `token create` (v0.68 item 8), this seeding should switch to
# it and drop the direct INSERTs.

mint_pat() { # mint_pat -> "secret hash dsid"
    local secret hash dsid
    secret="chan_pat_$(openssl rand 32 | basenc --base64url -w0 | tr -d '=')"
    hash="$(printf %s "$secret" | openssl dgst -sha256 -binary | basenc --base64url -w0 | tr -d '=')"
    dsid="$(printf %s "$secret" | sha256sum | awk '{print $1}')"
    echo "$secret $hash $dsid"
}

# The browser phase signs in through the stub OAuth with this email;
# profile's upsert-by-identity attaches the github identity to this
# pre-seeded row by email match. oauth_login gates the callback and
# ships default-off, so the e2e schema flips the default.
sql "$E2E_DATABASE_URL" "SET search_path TO $E2E_SCHEMA;
    UPDATE feature_flags SET default_enabled = true WHERE key = 'oauth_login';" || exit 2
ALICE_ID="$(sql "$E2E_DATABASE_URL" "SET search_path TO $E2E_SCHEMA;
    INSERT INTO users (id, email, username)
    VALUES (gen_random_uuid(), '$ALICE_EMAIL',
            'u' || substr(md5(random()::text), 1, 12))
    RETURNING id;")"
ALICE_USER="$(sql "$E2E_DATABASE_URL" \
    "SET search_path TO $E2E_SCHEMA; SELECT username FROM users WHERE id = '$ALICE_ID';")"
log "seeded user $ALICE_USER ($ALICE_ID)"

read -r PAT_A HASH_A DS_A <<< "$(mint_pat)"
read -r PAT_B HASH_B DS_B <<< "$(mint_pat)"
read -r PAT_C HASH_C DS_C <<< "$(mint_pat)"
for row in "$HASH_A e2e-a $DS_A" "$HASH_B e2e-b $DS_B" "$HASH_C e2e-c $DS_C"; do
    read -r hash label dsid <<< "$row"
    sql "$E2E_DATABASE_URL" "SET search_path TO $E2E_SCHEMA;
        INSERT INTO api_tokens (user_id, label, token_hash, scopes)
        VALUES ('$ALICE_ID', '$label', '$hash',
                ARRAY['tunnel','desktop.connect']);
        INSERT INTO devservers (owner_user_id, devserver_id, label)
        VALUES ('$ALICE_ID', '$dsid', '$label');" || exit 2
done
disc() { printf %s "${1:0:12}"; }
log "devservers: A=$(disc "$DS_A") B=$(disc "$DS_B") C=$(disc "$DS_C") (cap candidate)"

# Bob shares a devserver with alice (claimed grant): the consent
# picker's "shared with you" row. Bob's devserver never dials in, so
# it renders offline; the pick list does not require liveness.
DS_BOB="$(openssl rand -hex 32)"
BOB_ID="$(sql "$E2E_DATABASE_URL" "SET search_path TO $E2E_SCHEMA;
    INSERT INTO users (id, email, username)
    VALUES (gen_random_uuid(), 'e2e-bob@example.com', 'e2ebobhandle')
    RETURNING id;")"
BOB_USER="$(sql "$E2E_DATABASE_URL" \
    "SET search_path TO $E2E_SCHEMA; SELECT username FROM users WHERE id = '$BOB_ID';")"
sql "$E2E_DATABASE_URL" "SET search_path TO $E2E_SCHEMA;
    INSERT INTO devservers (owner_user_id, devserver_id, label)
    VALUES ('$BOB_ID', '$DS_BOB', 'bob-box');
    INSERT INTO devserver_grants
        (owner_user_id, devserver_id, grantee_email, grantee_user_id, role, accepted_at)
    VALUES ('$BOB_ID', '$DS_BOB', '$ALICE_EMAIL', '$ALICE_ID', 'editor', now());" || exit 2
log "seeded bob ($BOB_USER) sharing $(disc "$DS_BOB") with alice"

# ---------------------------------------------------------------
# Devservers (host-local foreground processes)
# ---------------------------------------------------------------

spawn_devserver() { # spawn_devserver <name> <port> <pat>
    local name="$1" port="$2" pat="$3"
    mkdir -p "$WORK/home-$name"
    spawn "ds-$name" env \
        CHAN_HOME="$WORK/home-$name" \
        CHAN_TUNNEL_TOKEN="$pat" \
        "$CHAN_BIN" devserver --service=none \
        --bind 127.0.0.1 --port "$port" \
        --tunnel-url="http://127.0.0.1:$TUNNEL_PORT/v1/tunnel"
}

admin_tunnels() { # admin_tunnels -> JSON list of alice's live tunnels
    curl -fsS -H "Authorization: Bearer $TOK_ADMIN" -H "Host: $APEX" \
        "http://127.0.0.1:$PROXY_PORT/admin/v1/users/$ALICE_USER/tunnels"
}

tunnel_count() { admin_tunnels | grep -o '"devserver_id"' | wc -l; }

wait_tunnels() { # wait_tunnels <expected-count> [tries]
    local want="$1" tries="${2:-75}"
    for _ in $(seq "$tries"); do
        [ "$(tunnel_count)" = "$want" ] && return 0
        sleep 0.4
    done
    return 1
}

spawn_devserver a "${DS_PORTS[0]}" "$PAT_A"
spawn_devserver b "${DS_PORTS[1]}" "$PAT_B"

if wait_tunnels 2; then
    assert_pass "tunnels: 2 devservers of one user registered"
else
    assert_fail "tunnels: expected 2 registrations, got: $(admin_tunnels)"
fi

# ---------------------------------------------------------------
# Assertions
# ---------------------------------------------------------------

# Follow-up curls hit wildcard hosts on the loopback listener; the
# localtest.me public DNS resolves them, --resolve pins v4 loopback so
# a v6-first resolver can't route past the 127.0.0.1 binds.
curl_host() { # curl_host <host> <args...>
    local host="$1"
    shift
    curl -sS --resolve "$host:$PROXY_PORT:127.0.0.1" "$@"
}

entry_for() { # entry_for <pat> <json-body> -> desktop entry response body
    curl -sS -X POST "http://127.0.0.1:$ID_PORT/desktop/v1/devserver/entry" \
        -H "Authorization: Bearer $1" \
        -H "content-type: application/json" \
        -d "$2"
}

json_get() { # json_get <key> (reads object on stdin)
    node -e 'let d="";process.stdin.on("data",c=>c&&(d+=c)).on("end",()=>{try{const v=JSON.parse(d)[process.argv[1]];console.log(v===undefined?"":v)}catch{console.log("")}})' "$1"
}

# A: desktop entry with an explicit devserver_id mints on that disc
# host, and the entry URL routes through the tunnel (303 cookie mint,
# then a devserver-served response behind it). The gate cookie is
# minted with `Secure`, which curl refuses to replay over plain http,
# so the harness captures Set-Cookie itself and sends it as an
# explicit Cookie header (we are testing routing, not browser cookie
# policy).
check_entry_routes() { # check_entry_routes <name> <pat> <dsid>
    local name="$1" pat="$2" dsid="$3"
    local body entry_url host hdrs cookie code
    body="$(entry_for "$pat" "{\"devserver_id\":\"$dsid\"}")"
    entry_url="$(printf %s "$body" | json_get entry_url)"
    host="$ALICE_USER--$(disc "$dsid").devserver.$DOMAIN"
    if [ -z "$entry_url" ]; then
        assert_fail "entry($name): no entry_url in: $body"
        return
    fi
    case "$entry_url" in
    "http://$host:$PROXY_PORT/"*) assert_pass "entry($name): minted on disc host $host" ;;
    *) assert_fail "entry($name): expected disc host $host, got $entry_url" ;;
    esac
    hdrs="$WORK/hdrs-$name.txt"
    code="$(curl_host "$host" -o /dev/null -w '%{http_code}' -D "$hdrs" "$entry_url")"
    cookie="$(sed -n 's/^[Ss]et-[Cc]ookie: \(devserver_gate=[^;]*\).*/\1/p' "$hdrs" | head -1)"
    printf %s "$cookie" > "$WORK/cookie-$name.txt"
    if [ "$code" = "303" ] && [ -n "$cookie" ]; then
        assert_pass "entry($name): entry token 303s and mints the gate cookie"
    else
        assert_fail "entry($name): expected 303 + devserver_gate cookie, got $code"
        return
    fi
    code="$(curl_host "$host" -o "$WORK/root-$name.html" -w '%{http_code}' \
        -H "Cookie: $cookie" "http://$host:$PROXY_PORT/")"
    # 200 = launcher SPA served through the tunnel. A chan binary
    # built without the web bundles answers with its own "bundle not
    # built" banner instead; both are devserver-generated responses,
    # so either proves the request crossed
    # host -> proxy -> tunnel -> devserver (the proxy's own 404 is
    # a bare {"error":"not found"}).
    if [ "$code" = "200" ]; then
        assert_pass "entry($name): cookie admits; devserver root serves 200"
    elif grep -qi "bundle not built" "$WORK/root-$name.html"; then
        assert_pass "entry($name): cookie admits; devserver answered ($code no-bundle banner: chan built without web bundles)"
    else
        assert_fail "entry($name): expected the devserver root, got $code: $(head -c 120 "$WORK/root-$name.html")"
    fi
}
check_entry_routes a "$PAT_A" "$DS_A"
check_entry_routes b "$PAT_B" "$DS_B"

# B: a cookie minted for devserver A does not admit on B's disc host
# (drv/aud isolation), same 404 shape as unknown.
HOST_A="$ALICE_USER--$(disc "$DS_A").devserver.$DOMAIN"
HOST_B="$ALICE_USER--$(disc "$DS_B").devserver.$DOMAIN"
COOKIE_A="$(cat "$WORK/cookie-a.txt" 2>/dev/null || true)"
if [ -n "$COOKIE_A" ]; then
    code="$(curl_host "$HOST_B" -o /dev/null -w '%{http_code}' -H "Cookie: $COOKIE_A" "http://$HOST_B:$PROXY_PORT/x/")"
    if [ "$code" = "404" ]; then
        assert_pass "isolation: devserver A's cookie is 404 on B's disc host"
    else
        assert_fail "isolation: expected 404, got $code"
    fi
else
    assert_fail "isolation: no cookie captured for devserver A"
fi

# C: unknown disc (well-formed, not live) is 404.
HOST_U="$ALICE_USER--000000000000.devserver.$DOMAIN"
code="$(curl_host "$HOST_U" -o /dev/null -w '%{http_code}' "http://$HOST_U:$PROXY_PORT/x/")"
if [ "$code" = "404" ]; then
    assert_pass "routing: unknown disc host is 404"
else
    assert_fail "routing: unknown disc expected 404, got $code"
fi

# D: bare host with two live devservers and no credential is 404;
# naked roots (bare and disc) bounce to the dashboard.
HOST_BARE="$ALICE_USER.devserver.$DOMAIN"
code="$(curl_host "$HOST_BARE" -o /dev/null -w '%{http_code}' "http://$HOST_BARE:$PROXY_PORT/x/")"
if [ "$code" = "404" ]; then
    assert_pass "routing: bare host without credential is 404"
else
    assert_fail "routing: bare host expected 404, got $code"
fi
for h in "$HOST_BARE" "$HOST_A"; do
    loc="$(curl_host "$h" -o /dev/null -w '%{redirect_url}' "http://$h:$PROXY_PORT/")"
    if [ "$loc" = "http://$ID_HOST/workspaces" ]; then
        assert_pass "routing: naked root on $h bounces to the dashboard"
    else
        assert_fail "routing: naked root on $h expected dashboard, got '$loc'"
    fi
done

# E: bare-host compat: a session cookie minted on a disc host is
# host-scoped, but the proxy's bare-host path resolves by the
# credential's drv claim. Entry tokens are only minted for disc hosts
# now, so exercise the bare-host resolution with the A cookie's token
# replayed as a bare-host request: the aud mismatch must 404 (the
# pre-0.68 bare-host cookies that DO verify are pinned in
# devserver-proxy's integration tests, which mint bare-host tokens
# directly).
if [ -n "$COOKIE_A" ]; then
    code="$(curl_host "$HOST_BARE" -o /dev/null -w '%{http_code}' -H "Cookie: $COOKIE_A" "http://$HOST_BARE:$PROXY_PORT/x/")"
    if [ "$code" = "404" ]; then
        assert_pass "routing: disc-host cookie does not leak onto the bare host"
    else
        assert_fail "routing: disc cookie on bare host expected 404, got $code"
    fi
else
    assert_fail "routing: no cookie captured for the bare-host leak check"
fi

# F: share landing `?d=` while signed out stashes and bounces to login.
code_loc="$(curl -sS -o /dev/null -w '%{http_code} %{redirect_url}' \
    "http://127.0.0.1:$ID_PORT/s/$ALICE_USER?d=$(disc "$DS_A")" -H "Host: $ID_HOST")"
if [ "$code_loc" = "303 http://127.0.0.1:$ID_PORT/" ]; then
    assert_pass "share: unauthenticated /s/{owner}?d= bounces to login"
else
    assert_fail "share: expected 303 to /, got '$code_loc'"
fi

# G: cap: a third devserver for the same user is refused at
# MAX_DEVSERVERS_PER_USER=$MAX_DEVSERVERS; the live set stays at 2 and
# never contains C's id.
spawn_devserver c "${DS_PORTS[2]}" "$PAT_C"
sleep 4
tunnels_json="$(admin_tunnels)"
n="$(printf %s "$tunnels_json" | grep -o '"devserver_id"' | wc -l)"
if [ "$n" = "2" ] && ! printf %s "$tunnels_json" | grep -q "$DS_C"; then
    assert_pass "cap: third devserver refused at cap $MAX_DEVSERVERS"
else
    assert_fail "cap: expected 2 tunnels without $DS_C, got: $tunnels_json"
fi

# H: kill + reconnect: admin-kill all tunnels; the devservers redial
# on their own and the disc host routes again. Devserver C is stopped
# first: it is still retrying against the cap, and after the kill it
# could win a slot from A or B and flake the re-registration count.
if [ -f "$WORK/pids/ds-c.pid" ]; then
    kill "$(cat "$WORK/pids/ds-c.pid")" 2>/dev/null || true
    rm -f "$WORK/pids/ds-c.pid"
fi
curl -fsS -X POST -H "Authorization: Bearer $TOK_ADMIN" -H "Host: $APEX" \
    "http://127.0.0.1:$PROXY_PORT/admin/v1/users/$ALICE_USER/tunnels/kill" >/dev/null
if wait_tunnels 2 150; then
    assert_pass "reconnect: both devservers re-registered after admin kill"
    check_entry_routes a-reconnect "$PAT_A" "$DS_A"
else
    assert_fail "reconnect: devservers did not re-register: $(admin_tunnels)"
fi

# ---------------------------------------------------------------
# I: consent flow in headless Chrome: sign in via the stub OAuth,
# assert the picker lists own + shared devservers, pick one, and read
# the chan:// handoff fragment.
# ---------------------------------------------------------------

frag_get() { # frag_get <url> <key> -> percent-decoded value
    node -e 'const [u,k]=process.argv.slice(1);const h=u.split("#")[1]||"";
        for(const p of h.split("&")){const [a,...r]=p.split("=");
        if(a===k){console.log(decodeURIComponent(r.join("=").replace(/\+/g," ")));break}}' \
        "$1" "$2"
}

AUTH_PATH="/desktop/authorize?redirect_uri=chan%3A%2F%2Fauth%2Fcallback&state=e2e-nonce&label=chan-desktop+%40+e2e&scopes=tunnel%2Cdesktop.connect&expires_in=2592000"
PICK_VALUE="$ALICE_USER:$DS_A"
if [ -x "$CHROME_BIN" ]; then
    # Run from a copy inside the work dir: ESM resolves node_modules
    # (puppeteer-core) relative to the script's own location.
    cp "$REPO/scripts/e2e/gateway-zone-browser.mjs" "$WORK/"
    browser_json="$(CHROME_BIN="$CHROME_BIN" ID_ORIGIN="http://$ID_HOST" \
        AUTH_PATH="$AUTH_PATH" PICK="$PICK_VALUE" \
        node "$WORK/gateway-zone-browser.mjs" 2> "$LOGS/browser.log")" || browser_json=""
    if [ -z "$browser_json" ]; then
        assert_fail "consent: browser run produced no output (see logs/browser.log)"
    else
        radios="$(printf %s "$browser_json" | json_get radios)"
        handoff="$(printf %s "$browser_json" | json_get handoff_url)"
        for want in "$ALICE_USER:$DS_A" "$ALICE_USER:$DS_B" "$BOB_USER:$DS_BOB"; do
            if printf %s "$radios" | grep -q "$want"; then
                assert_pass "consent: picker lists $want"
            else
                assert_fail "consent: picker missing $want in: $radios"
            fi
        done
        if [ "$(frag_get "$handoff" devserver_owner)" = "$ALICE_USER" ] &&
            [ "$(frag_get "$handoff" devserver_id)" = "$DS_A" ]; then
            assert_pass "consent: the pick rides the callback fragment"
        else
            assert_fail "consent: fragment lacks the pick: $handoff"
        fi

        # Redeem the one-time code: 200 exactly once, 410 on replay,
        # and the redeemed PAT drives a desktop entry for the picked
        # devserver end to end.
        code="$(frag_get "$handoff" code)"
        redeem1="$(curl -sS -o "$WORK/redeem.json" -w '%{http_code}' \
            -X POST "http://127.0.0.1:$ID_PORT/desktop/authorize/redeem" \
            -H "content-type: application/json" -d "{\"code\":\"$code\"}")"
        redeem2="$(curl -sS -o /dev/null -w '%{http_code}' \
            -X POST "http://127.0.0.1:$ID_PORT/desktop/authorize/redeem" \
            -H "content-type: application/json" -d "{\"code\":\"$code\"}")"
        if [ "$redeem1" = "200" ] && [ "$redeem2" = "410" ]; then
            assert_pass "redeem: one-time code answers 200 once, 410 on replay"
        else
            assert_fail "redeem: expected 200 then 410, got $redeem1 then $redeem2"
        fi
        browser_pat="$(json_get secret < "$WORK/redeem.json")"
        entry_body="$(entry_for "$browser_pat" "{\"owner\":\"$ALICE_USER\",\"devserver_id\":\"$DS_A\"}")"
        entry_url="$(printf %s "$entry_body" | json_get entry_url)"
        if [ -n "$entry_url" ]; then
            # Same two-hop shape as check_entry_routes: capture the
            # Secure cookie ourselves and replay it explicitly.
            hdrs="$WORK/hdrs-browser.txt"
            hop1="$(curl_host "$HOST_A" -o /dev/null -w '%{http_code}' -D "$hdrs" "$entry_url")"
            bcookie="$(sed -n 's/^[Ss]et-[Cc]ookie: \(devserver_gate=[^;]*\).*/\1/p' "$hdrs" | head -1)"
            hop2=""
            if [ "$hop1" = "303" ] && [ -n "$bcookie" ]; then
                hop2="$(curl_host "$HOST_A" -o "$WORK/root-browser.html" -w '%{http_code}' \
                    -H "Cookie: $bcookie" "http://$HOST_A:$PROXY_PORT/")"
            fi
            if [ "$hop2" = "200" ] ||
                { [ -n "$hop2" ] && grep -qi "bundle not built" "$WORK/root-browser.html"; }; then
                assert_pass "redeem: redeemed PAT opens the picked devserver ($hop2)"
            else
                assert_fail "redeem: entry hops expected 303 then devserver answer, got $hop1/$hop2"
            fi
        else
            assert_fail "redeem: desktop entry with the redeemed PAT failed: $entry_body"
        fi
    fi
else
    assert_fail "consent: headless Chrome not found (set E2E_CHROME_BIN)"
fi

# ---------------------------------------------------------------
# Extension seams (assert here as the round lands them):
#   - item 8: mint the PATs above via `chan-gateway-admin token
#     create` instead of SQL (owner: @@Tokens).
# ---------------------------------------------------------------

log ""
log "==== assertion summary ($ASSERT_LOG) ===="
cat "$ASSERT_LOG"
if [ "$FAILURES" -gt 0 ]; then
    log "RESULT: $FAILURES assertion(s) FAILED"
    exit 1
fi
log "RESULT: all assertions passed"
