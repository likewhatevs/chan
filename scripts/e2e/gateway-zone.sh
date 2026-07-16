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
#   scripts/e2e/gateway-zone.sh            # everything: core suite + all scenarios
#   scripts/e2e/gateway-zone.sh core       # the core suite only
#   scripts/e2e/gateway-zone.sh <scenario> # stack bring-up + that scenario only
#   E2E_KEEP=1 scripts/e2e/gateway-zone.sh # leave the stack running
#
# Scenarios: self-contained assert groups that run against the
# brought-up stack after the core region. Register one by defining
# scenario_<name>() at the SCENARIO FUNCTIONS marker near the bottom
# and appending <name> to $SCENARIOS below.
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

# Scenario dispatch: "all" (default) = core suite + every registered
# scenario; "core" = the inline suite only; a registered name = stack
# bring-up + that scenario only. Lanes append their scenario name here.
SCENARIOS="sweeper watchdog roster"
SCENARIO="${1:-all}"
RUN_CORE=1
case "$SCENARIO" in all | core) ;; *) RUN_CORE=0 ;; esac

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
# The operator CLI (package `admin`) seeds the PATs below through
# identity's /admin/v1/tokens surface.
[ -x "$GW_BIN/chan-gateway-admin" ] ||
    (cd "$REPO/gateway" && cargo build -p admin >/dev/null) || exit 2
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
    IDENTITY_ADMIN_TOKEN="$TOK_ADMIN" \
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
# PATs mint through the operator surface (`chan-gateway-admin token
# create` -> identity /admin/v1/tokens), so the harness exercises the
# same path an operator provisioning a user does; the devserver row
# registers as a side effect of the mint (label = PAT label). The
# devserver id stays derivable client-side: lowercase hex
# sha256(secret), the api_tokens cross-service contract.

admin_mint() { # admin_mint <label> -> "secret dsid"
    local out secret dsid
    out="$("$GW_BIN/chan-gateway-admin" \
        --identity-url "http://127.0.0.1:$ID_PORT" --token "$TOK_ADMIN" --json \
        token create "$ALICE_EMAIL" \
        --scope tunnel --scope desktop.connect --label "$1" \
        2>> "$LOGS/admin-mint.log")" || return 1
    secret="$(printf %s "$out" | node -e 'let d="";process.stdin.on("data",c=>d+=c).on("end",()=>{try{console.log(JSON.parse(d).secret||"")}catch{console.log("")}})')"
    case "$secret" in chan_pat_*) ;; *) return 1 ;; esac
    dsid="$(printf %s "$secret" | sha256sum | awk '{print $1}')"
    echo "$secret $dsid"
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

# The mint IS an assertion (v0.68 item 8: admin token create end to
# end); everything downstream then proves the minted PATs actually
# dial, route, and gate.
if read -r PAT_A DS_A <<< "$(admin_mint e2e-a)" &&
    read -r PAT_B DS_B <<< "$(admin_mint e2e-b)" &&
    read -r PAT_C DS_C <<< "$(admin_mint e2e-c)"; then
    assert_pass "admin mint: token create provisioned 3 PATs via /admin/v1/tokens"
else
    assert_fail "admin mint: chan-gateway-admin token create failed (logs/admin-mint.log)"
    log "cannot seed PATs; aborting"
    exit 1
fi
# Guard probes: the surface refuses a wrong bearer outright and
# answers an unknown email with the same 404 the CLI narrates.
guard_status="$(curl -sS -o /dev/null -w '%{http_code}' \
    -X POST "http://127.0.0.1:$ID_PORT/admin/v1/tokens" \
    -H "authorization: Bearer wrong-$TOK_ADMIN" \
    -H "content-type: application/json" \
    -d "{\"email\":\"$ALICE_EMAIL\"}")"
if [ "$guard_status" = "401" ]; then
    assert_pass "admin mint: wrong bearer refused (401)"
else
    assert_fail "admin mint: wrong bearer expected 401, got $guard_status"
fi
unknown_status="$(curl -sS -o /dev/null -w '%{http_code}' \
    -X POST "http://127.0.0.1:$ID_PORT/admin/v1/tokens" \
    -H "authorization: Bearer $TOK_ADMIN" \
    -H "content-type: application/json" \
    -d '{"email":"e2e-nobody@example.com"}')"
if [ "$unknown_status" = "404" ]; then
    assert_pass "admin mint: unknown email is 404"
else
    assert_fail "admin mint: unknown email expected 404, got $unknown_status"
fi
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

# The core suite (sections A-I). Skipped when a single scenario is
# requested; the bring-up and its asserts above always run.
if [ "$RUN_CORE" = 1 ]; then

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
# I: account-mode consent flow in headless Chrome: sign in via the
# stub OAuth, assert the picker-less account consent, read the
# chan:// handoff fragment, redeem the one-time code, then drive the
# roster and a roster-targeted desktop entry with the redeemed
# account PAT.
# ---------------------------------------------------------------

frag_get() { # frag_get <url> <key> -> percent-decoded value
    node -e 'const [u,k]=process.argv.slice(1);const h=u.split("#")[1]||"";
        for(const p of h.split("&")){const [a,...r]=p.split("=");
        if(a===k){console.log(decodeURIComponent(r.join("=").replace(/\+/g," ")));break}}' \
        "$1" "$2"
}

roster_row() { # roster_row <roster-json> <dsid> -> "owner online role"
    printf %s "$1" | node -e 'let d="";process.stdin.on("data",c=>d+=c).on("end",()=>{
        const id=process.argv[1];
        try{const r=JSON.parse(d).devservers.find(x=>x.devserver_id===id);
            console.log(r?`${r.owner} ${r.online} ${r.role}`:"")}catch{console.log("")}})' \
        "$2"
}

AUTH_PATH="/desktop/authorize?redirect_uri=chan%3A%2F%2Fauth%2Fcallback&state=e2e-nonce&label=chan-desktop+%40+e2e&scopes=desktop.account&expires_in=2592000"
if [ -x "$CHROME_BIN" ]; then
    # Run from a copy inside the work dir: ESM resolves node_modules
    # (puppeteer-core) relative to the script's own location.
    cp "$REPO/scripts/e2e/gateway-zone-browser.mjs" "$WORK/"
    browser_json="$(CHROME_BIN="$CHROME_BIN" ID_ORIGIN="http://$ID_HOST" \
        AUTH_PATH="$AUTH_PATH" \
        node "$WORK/gateway-zone-browser.mjs" 2> "$LOGS/browser.log")" || browser_json=""
    if [ -z "$browser_json" ]; then
        assert_fail "consent: browser run produced no output (see logs/browser.log)"
    else
        # radios is an array; count it explicitly (json_get prints
        # node's inspect form for non-strings).
        radios_n="$(printf %s "$browser_json" | node -e 'let d="";process.stdin.on("data",c=>d+=c).on("end",()=>{try{console.log(String(JSON.parse(d).radios.length))}catch{console.log("-1")}})')"
        consent_text="$(printf %s "$browser_json" | json_get consent_text)"
        handoff="$(printf %s "$browser_json" | json_get handoff_url)"
        if [ "$radios_n" = "0" ]; then
            assert_pass "consent: account consent renders no devserver picker"
        else
            assert_fail "consent: expected 0 devserver radios, found $radios_n"
        fi
        case "$consent_text" in
        *"access to your account on this gateway"*)
            assert_pass "consent: the account copy renders"
            ;;
        *)
            assert_fail "consent: account copy missing (see logs/browser.log)"
            ;;
        esac
        if [ -z "$(frag_get "$handoff" devserver_owner)" ] &&
            [ -z "$(frag_get "$handoff" devserver_id)" ]; then
            assert_pass "consent: fragment carries no retired devserver_* keys"
        else
            assert_fail "consent: retired devserver_* keys present: $handoff"
        fi

        # Redeem the one-time code: 200 exactly once, 410 on replay.
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

        # The redeemed account PAT reads the roster; own live rows and
        # bob's claimed share replace the old picker listing.
        roster_json="$(curl -sS -H "Authorization: Bearer $browser_pat" \
            "http://127.0.0.1:$ID_PORT/desktop/v1/devservers")"
        row_a="$(roster_row "$roster_json" "$DS_A")"
        if [ "$row_a" = "$ALICE_USER true owner" ]; then
            assert_pass "roster: redeemed PAT lists devserver A online (owner row)"
        else
            assert_fail "roster: devserver A row wrong: '$row_a' in: $roster_json"
        fi
        row_b="$(roster_row "$roster_json" "$DS_B")"
        if [ "$row_b" = "$ALICE_USER true owner" ]; then
            assert_pass "roster: redeemed PAT lists devserver B online (owner row)"
        else
            assert_fail "roster: devserver B row wrong: '$row_b' in: $roster_json"
        fi
        row_bob="$(roster_row "$roster_json" "$DS_BOB")"
        if [ "$row_bob" = "$BOB_USER false editor" ]; then
            assert_pass "roster: bob's claimed share listed (offline, editor)"
        else
            assert_fail "roster: bob share row wrong: '$row_bob' in: $roster_json"
        fi

        # Entry mint targeted from the roster row, then the same
        # two-hop routing check the picker flow used to cover.
        entry_owner="${row_a%% *}"
        entry_body="$(entry_for "$browser_pat" "{\"owner\":\"$entry_owner\",\"devserver_id\":\"$DS_A\"}")"
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
                assert_pass "redeem: account PAT opens the roster-picked devserver ($hop2)"
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

fi # RUN_CORE

# ---------------------------------------------------------------
# SCENARIO FUNCTIONS: lanes append scenario_<name>() definitions
# below this marker (and register the name in $SCENARIOS at the top).
# Each function asserts via assert_pass/assert_fail against the
# running stack and must leave the stack usable (clean up what it
# stops/starts).
# ---------------------------------------------------------------

# Scenario: devserver registry sweeper (profile-service). Proves the
# mark/delete cycle against the real stack: a stale never-dialed row
# leaves the owner's list while the live devservers are marked in the
# SAME tick and survive, the swept row's grant cascades away, a
# stopped devserver's row ages out after the retention, and a redial
# after a sweep comes back live-unlabeled (tunnel up, no registry row
# until a mint recreates it). Runs profile-service with
# DEVSERVER_RETENTION_MINUTES=1 for the duration, then restores the
# bring-up config and re-inserts whatever the 1-minute window swept.
scenario_sweeper() {
    sweeper_sql() { sql "$E2E_DATABASE_URL" "SET search_path TO $E2E_SCHEMA; $1"; }

    sweeper_owned() { # alice's owned devserver_ids, one per line
        # Fetch and extraction split on purpose: under pipefail an
        # empty list (grep exit 1) would otherwise read as a fetch
        # failure, and a fetch failure must NOT read as an empty list.
        local body
        body="$(curl -fsS -H "Authorization: Bearer $TOK_PROFILE" \
            "http://127.0.0.1:$PROFILE_PORT/v1/users/$ALICE_ID/grants/owned")" || return 1
        printf '%s' "$body" | grep -o '"devserver_id":"[0-9a-f]*"' | cut -d'"' -f4
        return 0
    }

    sweeper_respawn_profile() { # sweeper_respawn_profile [retention-minutes]
        local pid
        pid="$(cat "$WORK/pids/profile.pid" 2>/dev/null)"
        [ -n "$pid" ] && kill "$pid" 2>/dev/null
        for _ in $(seq 25); do
            kill -0 "$pid" 2>/dev/null || break
            sleep 0.2
        done
        spawn profile env \
            BIND_ADDR="127.0.0.1:$PROFILE_PORT" \
            DATABASE_URL="$DB_URL" \
            PROFILE_AUTH_TOKEN="$TOK_PROFILE" \
            DEVSERVER_ADMIN_TOKEN="$TOK_ADMIN" \
            DEVSERVER_ADMIN_URL="http://127.0.0.1:$PROXY_PORT" \
            ${1:+DEVSERVER_RETENTION_MINUTES="$1"} \
            RUST_LOG=info \
            "$GW_BIN/profile-service"
        wait_http profile "http://127.0.0.1:$PROFILE_PORT/healthz" || return 1
        require_alive profile
    }

    sweeper_wait_owned_gone() { # <dsid> [tries]; polls every 2s
        local dsid="$1" tries="${2:-45}" out
        for _ in $(seq "$tries"); do
            # Only a SUCCESSFUL fetch may declare the id gone: during
            # a profile respawn the list is unreachable, not empty.
            if out="$(sweeper_owned)"; then
                printf '%s\n' "$out" | grep -q "^$dsid$" || return 0
            fi
            sleep 2
        done
        return 1
    }

    # Snapshot the registry so the scenario can put back whatever the
    # 1-minute retention sweeps besides its own seeds (bob's shared
    # row, the cap PAT's row -- both offline and older than a minute).
    sweeper_sql "
        CREATE TABLE sweeper_snap_ds AS SELECT * FROM devservers;
        CREATE TABLE sweeper_snap_grants AS SELECT * FROM devserver_grants;" || {
        assert_fail "sweeper: registry snapshot failed"
        return
    }

    # A stranded row: registered long ago, never dialed, carrying a
    # grant (the ruling: grants do NOT protect a row from the sweep).
    local stale_id
    stale_id="$(openssl rand -hex 32)"
    sweeper_sql "
        INSERT INTO devservers (owner_user_id, devserver_id, label, created_at)
        VALUES ('$ALICE_ID', '$stale_id', 'stale-e2e', now() - interval '10 minutes');
        INSERT INTO devserver_grants (owner_user_id, devserver_id, grantee_email, role)
        VALUES ('$ALICE_ID', '$stale_id', 'e2e-swept@example.com', 'viewer');" || {
        assert_fail "sweeper: stale-row seed failed"
        return
    }

    sweeper_respawn_profile 1 || {
        assert_fail "sweeper: profile restart with retention=1 failed"
        return
    }

    # The first tick fires immediately on spawn: the stale row must go,
    # and the two LIVE devservers must be marked in the same tick and
    # survive (mark-before-delete, observed end to end).
    if sweeper_wait_owned_gone "$stale_id"; then
        assert_pass "sweeper: stale never-dialed row left the owned list"
    else
        assert_fail "sweeper: stale row still listed after retention + tick"
    fi
    local grants_left
    grants_left="$(sweeper_sql \
        "SELECT COUNT(*) FROM devserver_grants WHERE devserver_id = '$stale_id';")"
    if [ "$grants_left" = "0" ]; then
        assert_pass "sweeper: the swept row's grant cascaded away"
    else
        assert_fail "sweeper: expected 0 grants on the swept row, got $grants_left"
    fi
    if sweeper_owned | grep -q "^$DS_A$"; then
        assert_pass "sweeper: live devserver A survived the sweep tick"
    else
        assert_fail "sweeper: live devserver A vanished from the owned list"
    fi
    local seen_a
    seen_a="$(sweeper_sql "SELECT COUNT(*) FROM devservers
        WHERE devserver_id = '$DS_A' AND last_seen_at IS NOT NULL;")"
    if [ "$seen_a" = "1" ]; then
        assert_pass "sweeper: live devserver A carries a last_seen_at mark"
    else
        assert_fail "sweeper: devserver A has no last_seen_at mark"
    fi

    # Stop devserver B: its registration drops, then the row ages past
    # the retention (backdated so the next tick, at most 60s away,
    # collects it instead of a wall-clock wait).
    if [ -f "$WORK/pids/ds-b.pid" ]; then
        kill "$(cat "$WORK/pids/ds-b.pid")" 2>/dev/null
        rm -f "$WORK/pids/ds-b.pid"
    fi
    if wait_tunnels 1 75; then
        sweeper_sql "UPDATE devservers SET last_seen_at = now() - interval '2 minutes'
            WHERE devserver_id = '$DS_B';"
        if sweeper_wait_owned_gone "$DS_B"; then
            assert_pass "sweeper: stopped devserver's row left the owned list"
        else
            assert_fail "sweeper: stopped devserver B still listed after retention + tick"
        fi
    else
        assert_fail "sweeper: devserver B's tunnel did not drop after kill"
    fi

    # Redial: the tunnel returns but no registry row does -- the
    # live-unlabeled comeback state (a later mint recreates the row).
    spawn_devserver b "${DS_PORTS[1]}" "$PAT_B"
    if wait_tunnels 2 150; then
        local owned_after
        if ! admin_tunnels | grep -q "$DS_B"; then
            assert_fail "sweeper: redial registered but B's id missing: $(admin_tunnels)"
        elif ! owned_after="$(sweeper_owned)"; then
            # Absence only counts from a successful fetch.
            assert_fail "sweeper: owned list unreachable during the live-unlabeled check"
        elif printf '%s\n' "$owned_after" | grep -q "^$DS_B$"; then
            assert_fail "sweeper: redial recreated the registry row (expected live-unlabeled)"
        else
            assert_pass "sweeper: redialed devserver is live-unlabeled (tunnel up, no row)"
        fi
    else
        assert_fail "sweeper: devserver B did not re-register after redial"
    fi

    # Restore: bring-up profile config (default retention) and every
    # row the 1-minute window swept; devservers before grants (FK).
    sweeper_respawn_profile || {
        assert_fail "sweeper: profile restore restart failed"
        return
    }
    sweeper_sql "
        INSERT INTO devservers SELECT * FROM sweeper_snap_ds s
            WHERE NOT EXISTS (SELECT 1 FROM devservers d
                WHERE d.owner_user_id = s.owner_user_id
                  AND d.devserver_id = s.devserver_id);
        INSERT INTO devserver_grants SELECT * FROM sweeper_snap_grants g
            WHERE NOT EXISTS (SELECT 1 FROM devserver_grants x WHERE x.id = g.id);
        DROP TABLE sweeper_snap_ds;
        DROP TABLE sweeper_snap_grants;" || {
        assert_fail "sweeper: registry restore failed"
        return
    }
    if sweeper_owned | grep -q "^$DS_B$"; then
        assert_pass "sweeper: registry restored (B's row back for later scenarios)"
    else
        assert_fail "sweeper: restore did not bring B's row back"
    fi
}

# Scenario: gateway devserver liveness watchdog (item 5). Holds devserver A's
# window-feed WS through the REAL proxy and proves the pieces the client
# watchdog depends on: the keepalive Ping is answered end to end (the proxy
# bridge forwards Ping/Pong, the devserver's axum /watch auto-pongs), a SIGSTOP'd
# proxy leaves the held socket a half-open zombie -- no Pong AND no onclose --
# while a fresh dial still routes (the dishonest-green asymmetry), and SIGCONT
# heals the socket. The SPA DisconnectOverlay and the desktop Unreachable
# rendering are unit-covered (workspace-app transportHeartbeat/disconnectOverlay,
# chan-desktop entry_from_devserver); this scenario cannot drive a browser
# because the rig's chan is built --no-default-features and serves no SPA bundle,
# so it asserts the server+proxy behaviour those clients sit on.
scenario_watchdog() {
    # A lone-scenario run skips the core suite, so mint devserver A's entry +
    # gate cookie here (the same two-hop check_entry_routes does).
    local host body entry_url hdrs cookie
    host="$ALICE_USER--$(disc "$DS_A").devserver.$DOMAIN"
    body="$(entry_for "$PAT_A" "{\"devserver_id\":\"$DS_A\"}")"
    entry_url="$(printf %s "$body" | json_get entry_url)"
    if [ -z "$entry_url" ]; then
        assert_fail "watchdog: no entry_url minted for devserver A: $body"
        return
    fi
    hdrs="$WORK/hdrs-watchdog.txt"
    curl_host "$host" -o /dev/null -D "$hdrs" "$entry_url" >/dev/null
    cookie="$(sed -n 's/^[Ss]et-[Cc]ookie: \(devserver_gate=[^;]*\).*/\1/p' "$hdrs" | head -1)"
    if [ -z "$cookie" ]; then
        assert_fail "watchdog: no gate cookie minted for devserver A"
        return
    fi

    # A fresh dial through the proxy still routes to the devserver (200, or the
    # no-bundle banner from a chan built without web assets) -- the poll-heals
    # path the launcher green dot rides even while a held socket is dead.
    local fresh
    fresh="$(curl_host "$host" -o /dev/null -w '%{http_code}' -H "Cookie: $cookie" \
        "http://$host:$PROXY_PORT/")"
    case "$fresh" in
    200 | 404) assert_pass "watchdog: a fresh dial routes to the devserver ($fresh)" ;;
    *) assert_fail "watchdog: fresh dial did not reach the devserver (got $fresh)" ;;
    esac

    # The probe holds the feed WS and drives three phases (alive / proxy SIGSTOP
    # / SIGCONT), reporting per-phase whether a WS Ping is answered and whether
    # onclose fired.
    cat > "$WORK/watchdog-probe.mjs" <<'PROBE'
import { createRequire } from "node:module";
import fs from "node:fs";
const require = createRequire(process.env.WORK + "/x.js");
const { WebSocket } = require("ws");
const { WORK, PROXY_PORT, WD_HOST, WD_COOKIE } = process.env;
const PROXY_PID = Number(fs.readFileSync(`${WORK}/pids/proxy.pid`, "utf8").trim());
const log = (o) => console.log(JSON.stringify(o));
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
// Send a WS Ping; resolve true iff a Pong comes back within `ms`.
function pingPong(ws, ms) {
  return new Promise((resolve) => {
    let done = false;
    const onPong = () => { if (!done) { done = true; ws.off("pong", onPong); resolve(true); } };
    ws.on("pong", onPong);
    try { ws.ping(); } catch {}
    setTimeout(() => { if (!done) { done = true; ws.off("pong", onPong); resolve(false); } }, ms);
  });
}
const ws = new WebSocket(`ws://127.0.0.1:${PROXY_PORT}/api/library/windows/watch`, {
  headers: { Host: `${WD_HOST}:${PROXY_PORT}`, Cookie: WD_COOKIE },
});
let firstFrame = false, closed = false;
ws.on("message", () => { firstFrame = true; });
ws.on("close", () => { closed = true; });
ws.on("error", () => {});
const opened = await new Promise((res) => {
  ws.on("open", () => res(true));
  setTimeout(() => res(false), 5000);
});
if (!opened) { log({ SUMMARY: { opened: false } }); process.exit(0); }
await sleep(1000);
const aliveKeepalive = await pingPong(ws, 3000);
log({ phase: "alive", firstFrame, pong: aliveKeepalive });

// Freeze the proxy: the held client<->proxy TCP stays ESTABLISHED (kernel), but
// the frozen app forwards nothing -- the sleep-zombie condition.
process.kill(PROXY_PID, "SIGSTOP");
await sleep(1000);
const frozenPong = await pingPong(ws, 4000);
const closedWhileFrozen = closed;
log({ phase: "frozen", pong: frozenPong, closed: closedWhileFrozen });

process.kill(PROXY_PID, "SIGCONT");
await sleep(1500);
const recovered = await pingPong(ws, 5000);
log({ phase: "recovered", pong: recovered });

log({ SUMMARY: {
  opened: true, firstFrame, aliveKeepalive,
  frozenSilent: !frozenPong, frozenNoOnclose: !closedWhileFrozen, recovered,
} });
try { ws.terminate(); } catch {}
process.exit(0);
PROBE

    local out
    out="$(WORK="$WORK" PROXY_PORT="$PROXY_PORT" WD_HOST="$host" WD_COOKIE="$cookie" \
        node "$WORK/watchdog-probe.mjs" 2>> "$LOGS/watchdog.log")"
    # Belt and braces: whatever the probe did, make sure the proxy is running
    # again so the stack stays usable for later scenarios / teardown.
    local ppid
    ppid="$(cat "$WORK/pids/proxy.pid" 2>/dev/null)"
    [ -n "$ppid" ] && kill -CONT "$ppid" 2>/dev/null
    :

    local summary
    summary="$(printf %s "$out" | grep '"SUMMARY"' | tail -1)"
    if [ -z "$summary" ]; then
        assert_fail "watchdog: probe produced no summary (see logs/watchdog.log)"
        return
    fi
    wd_field() { # wd_field <key>; prints SUMMARY.<key> from the probe JSON
        printf %s "$summary" | node -e 'let d="";process.stdin.on("data",c=>d+=c).on("end",()=>{try{console.log(String(JSON.parse(d).SUMMARY[process.argv[1]]))}catch{console.log("")}})' "$1"
    }

    if [ "$(wd_field opened)" = "true" ]; then
        assert_pass "watchdog: feed WS opened through the proxy"
    else
        assert_fail "watchdog: feed WS did not open through the proxy"
        return
    fi
    [ "$(wd_field firstFrame)" = "true" ] &&
        assert_pass "watchdog: server pushed the window snapshot on connect" ||
        assert_fail "watchdog: no window snapshot frame on connect"
    [ "$(wd_field aliveKeepalive)" = "true" ] &&
        assert_pass "watchdog: keepalive Ping answered through the proxy (server auto-pong)" ||
        assert_fail "watchdog: keepalive Ping got no Pong on a live proxy"
    [ "$(wd_field frozenSilent)" = "true" ] &&
        assert_pass "watchdog: a frozen proxy silences the Ping (the read-deadline trigger)" ||
        assert_fail "watchdog: Ping answered while the proxy was frozen"
    [ "$(wd_field frozenNoOnclose)" = "true" ] &&
        assert_pass "watchdog: the frozen socket is a half-open zombie (no onclose)" ||
        assert_fail "watchdog: onclose fired during the freeze (expected a silent zombie)"
    [ "$(wd_field recovered)" = "true" ] &&
        assert_pass "watchdog: SIGCONT heals the socket (Ping answered again)" ||
        assert_fail "watchdog: socket did not recover after SIGCONT"
}

# Scenario: account-mode roster (browser-free). Mints a
# desktop.account PAT through the operator surface, then proves the
# roster read against the live stack: owned + shared rows with real
# liveness, the online flag flipping as a devserver stops and
# returns, ETag/If-None-Match 304 on an unchanged roster, and entry
# mints with the account PAT for an own live row (two-hop route) and
# the shared offline row (devserver_offline; the live shared mint is
# pinned in tests/desktop_entry.rs). Leaves the stack as found
# (devserver A redialed and online).
scenario_roster() {
    rrow() { # rrow <roster-json> <dsid> -> "owner online role"
        printf %s "$1" | node -e 'let d="";process.stdin.on("data",c=>d+=c).on("end",()=>{
            const id=process.argv[1];
            try{const r=JSON.parse(d).devservers.find(x=>x.devserver_id===id);
                console.log(r?`${r.owner} ${r.online} ${r.role}`:"")}catch{console.log("")}})' \
            "$2"
    }
    roster_get() { # roster_get <outfile> [etag] -> http code; headers to $WORK/roster-hdrs.txt
        curl -sS -o "$1" -w '%{http_code}' -D "$WORK/roster-hdrs.txt" \
            ${2:+-H "If-None-Match: $2"} \
            -H "Authorization: Bearer $ROSTER_PAT" \
            "http://127.0.0.1:$ID_PORT/desktop/v1/devservers"
    }
    roster_etag() { # the header line is CRLF; strip the CR or the echoed If-None-Match breaks
        tr -d '\r' < "$WORK/roster-hdrs.txt" | sed -n 's/^[Ee][Tt]ag: //p' | head -1
    }

    local out account_dsid
    out="$("$GW_BIN/chan-gateway-admin" \
        --identity-url "http://127.0.0.1:$ID_PORT" --token "$TOK_ADMIN" --json \
        token create "$ALICE_EMAIL" \
        --scope desktop.account --label roster-e2e \
        2>> "$LOGS/admin-mint.log")" || out=""
    ROSTER_PAT="$(printf %s "$out" | json_get secret)"
    case "$ROSTER_PAT" in
    chan_pat_*) assert_pass "roster: admin mint of a desktop.account PAT" ;;
    *)
        assert_fail "roster: account PAT mint failed (logs/admin-mint.log)"
        return
        ;;
    esac
    # The operator mint registers a devserver row for every PAT
    # (parity with the SPA mint); an account PAT is not a devserver,
    # so drop the side-effect row to keep the roster exact.
    account_dsid="$(printf %s "$ROSTER_PAT" | sha256sum | awk '{print $1}')"
    sql "$E2E_DATABASE_URL" "SET search_path TO $E2E_SCHEMA;
        DELETE FROM devservers WHERE devserver_id = '$account_dsid';" || {
        assert_fail "roster: side-effect row cleanup failed"
        return
    }

    # A tunnel/connect PAT must not read the roster.
    local code
    code="$(curl -sS -o /dev/null -w '%{http_code}' \
        -H "Authorization: Bearer $PAT_A" \
        "http://127.0.0.1:$ID_PORT/desktop/v1/devservers")"
    if [ "$code" = "401" ]; then
        assert_pass "roster: tunnel/connect PAT is 401 on the roster"
    else
        assert_fail "roster: expected 401 for a wrong-scope PAT, got $code"
    fi

    local roster_json
    code="$(roster_get "$WORK/roster1.json")"
    roster_json="$(cat "$WORK/roster1.json")"
    if [ "$code" = "200" ]; then
        assert_pass "roster: account PAT reads the roster (200)"
    else
        assert_fail "roster: expected 200, got $code: $roster_json"
        return
    fi
    if [ "$(rrow "$roster_json" "$DS_A")" = "$ALICE_USER true owner" ]; then
        assert_pass "roster: own live devserver A is online (owner row)"
    else
        assert_fail "roster: devserver A row wrong in: $roster_json"
    fi
    if [ "$(rrow "$roster_json" "$DS_C")" = "$ALICE_USER false owner" ]; then
        assert_pass "roster: own registered-but-dark devserver C is offline"
    else
        assert_fail "roster: devserver C row wrong in: $roster_json"
    fi
    if [ "$(rrow "$roster_json" "$DS_BOB")" = "$BOB_USER false editor" ]; then
        assert_pass "roster: bob's claimed share listed (offline, editor)"
    else
        assert_fail "roster: bob share row wrong in: $roster_json"
    fi

    local etag
    etag="$(roster_etag)"
    if [ -n "$etag" ]; then
        assert_pass "roster: the 200 carries an ETag"
    else
        assert_fail "roster: no ETag header on the roster 200"
    fi
    code="$(roster_get /dev/null "$etag")"
    if [ "$code" = "304" ]; then
        assert_pass "roster: If-None-Match answers 304 on an unchanged roster"
    else
        assert_fail "roster: expected 304 on the unchanged roster, got $code"
    fi

    # Stop devserver A: its row must flip offline (fresh 200s while
    # polling), and the pre-flip ETag must stop matching.
    if [ -f "$WORK/pids/ds-a.pid" ]; then
        kill "$(cat "$WORK/pids/ds-a.pid")" 2>/dev/null
        rm -f "$WORK/pids/ds-a.pid"
    fi
    local flipped=0
    for _ in $(seq 75); do
        code="$(roster_get "$WORK/roster2.json")"
        if [ "$code" = "200" ] &&
            [ "$(rrow "$(cat "$WORK/roster2.json")" "$DS_A")" = "$ALICE_USER false owner" ]; then
            flipped=1
            break
        fi
        sleep 0.4
    done
    if [ "$flipped" = 1 ]; then
        assert_pass "roster: stopping devserver A flips its row offline"
    else
        assert_fail "roster: devserver A never went offline in the roster"
    fi
    code="$(roster_get /dev/null "$etag")"
    if [ "$code" = "200" ]; then
        assert_pass "roster: the pre-flip ETag reads a fresh 200 after the change"
    else
        assert_fail "roster: stale ETag expected 200, got $code"
    fi

    # Redial devserver A and wait for the flip back so the stack is
    # left as found.
    spawn_devserver a "${DS_PORTS[0]}" "$PAT_A"
    local back=0
    for _ in $(seq 150); do
        code="$(roster_get "$WORK/roster3.json")"
        if [ "$code" = "200" ] &&
            [ "$(rrow "$(cat "$WORK/roster3.json")" "$DS_A")" = "$ALICE_USER true owner" ]; then
            back=1
            break
        fi
        sleep 0.4
    done
    if [ "$back" = 1 ]; then
        assert_pass "roster: devserver A returns online after redial"
    else
        assert_fail "roster: devserver A did not come back online"
    fi

    # Entry mints with the account PAT: the own live row routes end
    # to end (the same two-hop shape check_entry_routes uses); the
    # shared row is registered but dark, so it answers the
    # devserver_offline reason.
    local entry_body entry_url hdrs cookie hop1 hop2 host
    entry_body="$(entry_for "$ROSTER_PAT" "{\"owner\":\"$ALICE_USER\",\"devserver_id\":\"$DS_A\"}")"
    entry_url="$(printf %s "$entry_body" | json_get entry_url)"
    host="$ALICE_USER--$(disc "$DS_A").devserver.$DOMAIN"
    if [ -n "$entry_url" ]; then
        hdrs="$WORK/hdrs-roster-entry.txt"
        hop1="$(curl_host "$host" -o /dev/null -w '%{http_code}' -D "$hdrs" "$entry_url")"
        cookie="$(sed -n 's/^[Ss]et-[Cc]ookie: \(devserver_gate=[^;]*\).*/\1/p' "$hdrs" | head -1)"
        hop2=""
        if [ "$hop1" = "303" ] && [ -n "$cookie" ]; then
            hop2="$(curl_host "$host" -o "$WORK/root-roster.html" -w '%{http_code}' \
                -H "Cookie: $cookie" "http://$host:$PROXY_PORT/")"
        fi
        if [ "$hop2" = "200" ] ||
            { [ -n "$hop2" ] && grep -qi "bundle not built" "$WORK/root-roster.html"; }; then
            assert_pass "roster: account PAT entry routes to the own devserver ($hop2)"
        else
            assert_fail "roster: entry hops expected 303 then devserver answer, got $hop1/$hop2"
        fi
    else
        assert_fail "roster: entry mint for the own devserver failed: $entry_body"
    fi
    local shared_body shared_reason
    shared_body="$(entry_for "$ROSTER_PAT" "{\"owner\":\"$BOB_USER\",\"devserver_id\":\"$DS_BOB\"}")"
    shared_reason="$(printf %s "$shared_body" | json_get reason)"
    if [ "$shared_reason" = "devserver_offline" ]; then
        assert_pass "roster: shared-row entry answers devserver_offline (box dark)"
    else
        assert_fail "roster: shared-row entry expected devserver_offline, got: $shared_body"
    fi
}

run_scenarios() { # run_scenarios <all|name>
    local which="$1" ran=0 s
    for s in $SCENARIOS; do
        if [ "$which" = all ] || [ "$which" = "$s" ]; then
            log ""
            log "==== scenario: $s ===="
            "scenario_$s"
            ran=1
        fi
    done
    if [ "$which" != all ] && [ "$ran" = 0 ]; then
        assert_fail "unknown scenario '$which' (registered: ${SCENARIOS:-none})"
    fi
}
[ "$SCENARIO" != core ] && run_scenarios "$SCENARIO"

log ""
log "==== assertion summary ($ASSERT_LOG) ===="
cat "$ASSERT_LOG"
if [ "$FAILURES" -gt 0 ]; then
    log "RESULT: $FAILURES assertion(s) FAILED"
    exit 1
fi
log "RESULT: all assertions passed"
