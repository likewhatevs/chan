#!/usr/bin/env bash
# devserver-tunnel-e2e -- cross-CONTAINER end-to-end for the chan devserver tunnel.
#
# Stands up the REAL devserver-proxy-service and a REAL `chan devserver
# --tunnel-url` in two SEPARATE sdme containers, then drives a request through
# the proxy's PUBLIC surface, over the tunnel, into the devserver's mounted
# workspace. A 200 carrying the workspace SPA (with the injected
# <meta name="chan-prefix"> for the mounted prefix) is the proof -- that request
# travelled host -> proxy:7002 -> (gate) -> tunnel -> chan devserver -> workspace.
#
# Topology (see zone-isolation-probe.sh + README for WHY one zone):
#   zone gw-e2e
#     gw-e2e-proxy : devserver-proxy-service (real) + stub-identity (loopback)
#     gw-e2e-ds    : chan devserver --tunnel-url + a mounted workspace
#   The tunnel is gw-e2e-ds -> gw-e2e-proxy:7100 (same-zone container IP). On
#   this host the kernel firewall drops container->host and cross-zone TCP
#   (ICMP only), and `-p` does not bridge zones, so same-zone is the only path
#   the host permits without root iptables. The containers are still separate
#   (own netns/fs/process tree); the tunnel genuinely crosses between them.
#
# Identity is a tiny stub on the proxy's loopback (the proxy validates the
# tunnel PAT against it); the per-request gate cookie is self-minted with the
# same DEVSERVER_GATE_SECRET. Both are auth shims -- the binaries under test
# (proxy + devserver + tunnel crates) are the real release builds.
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(git -C "$HERE" rev-parse --show-toplevel 2>/dev/null || echo "$HERE/../../../../../..")"
BIN_DIR="${BIN_DIR:-$REPO/target/devserver-e2e/bin}"
PROXY_BIN="$BIN_DIR/devserver-proxy-service"
CHAN_BIN="$BIN_DIR/chan"
STUB_PY="$HERE/stub-identity.py"
MINT_PY="$HERE/mint-gate-token.py"
RFS_SDME="$HERE/chan-e2e-run.sdme"
SDME="sudo -n sdme"
RFS="${RFS:-chan-e2e-run}"

ZONE="${ZONE:-gw-e2e}"
C_PROXY="${C_PROXY:-gw-e2e-proxy}"
C_DS="${C_DS:-gw-e2e-ds}"

APEX="devserver.localtest.me"
SUFFIX=".devserver.localtest.me"
TENANT_USER="alice"
USER_ID="11111111-1111-4111-8111-111111111111"
EDITOR_USER_ID="22222222-2222-4222-8222-222222222222"
GATE_SECRET="e2e-workspace-gate-secret-0123456789"
IDENTITY_TOKEN="e2e-identity-internal-token"
PAT="chan_pat_e2e_dummy_token"
DEVSERVER_ID="$(printf '%s' "$PAT" | sha256sum | awk '{print $1}')"
DESKTOP_OWNER_PAT="chan_pat_e2e_desktop_owner"
DESKTOP_EDITOR_PAT="chan_pat_e2e_desktop_editor"
STUB_PORT="7799"
WS_NAME="notes"
PROXY_PUB_PORT=7002
PROXY_TUN_PORT=7100
DS_PORT=8787
HOST_NAME="${TENANT_USER}--${DEVSERVER_ID:0:12}${SUFFIX}"
HOSTHDR="$HOST_NAME:$PROXY_PUB_PORT"
PROXY_ORIGIN="http://$HOSTHDR"

say()  { printf '\n\033[1;36m== %s\033[0m\n' "$*"; }
info() { printf '   %s\n' "$*"; }
die()  { printf '\033[1;31mFAIL: %s\033[0m\n' "$*" >&2; exit 1; }
cip()  { $SDME exec "$1" -- /usr/bin/hostname -I 2>/dev/null | awk '{print $1}'; }

cleanup() {
  $SDME rm -f "$C_PROXY" "$C_DS" >/dev/null 2>&1 || true
  info "removed containers $C_PROXY $C_DS"
}
[ "${1:-}" = "--clean" ] && { say "cleanup"; cleanup; exit 0; }

say "preflight"
[ -x "$PROXY_BIN" ] || die "missing proxy binary $PROXY_BIN (build first)"
[ -x "$CHAN_BIN" ]  || die "missing chan binary $CHAN_BIN (build first)"
[ -f "$STUB_PY" ] && [ -f "$MINT_PY" ] || die "missing helper scripts in $HERE"
$SDME ps >/dev/null 2>&1 || die "sudo -n sdme not working"
if ! $SDME fs ls 2>/dev/null | grep -qE "^${RFS}[[:space:]]"; then
  info "runtime rootfs '$RFS' missing -- building from $RFS_SDME (one-time)"
  ( cd "$HERE" && $SDME fs build -f "$RFS" "$(basename "$RFS_SDME")" ) >/tmp/e2e-rfs.log 2>&1 \
    || { tail -20 /tmp/e2e-rfs.log; die "rootfs build failed"; }
fi
info "binaries + helpers present; sdme ok; rootfs '$RFS' present"

mk() {  # name
  $SDME create "$1" -r "$RFS" --network-zone "$ZONE" --started -t 90 >/tmp/e2e-mk.log 2>&1
  for _ in $(seq 1 15); do $SDME ps 2>/dev/null | grep -qE "^$1[[:space:]].*running" && return 0; sleep 1; done
  cat /tmp/e2e-mk.log; return 1
}

say "create two containers in zone $ZONE"
$SDME rm -f "$C_PROXY" "$C_DS" >/dev/null 2>&1 || true
mk "$C_PROXY" || die "create $C_PROXY"
mk "$C_DS"    || die "create $C_DS"
sleep 2
PROXY_IP="$(cip "$C_PROXY")"; DS_IP="$(cip "$C_DS")"
info "$C_PROXY ip=$PROXY_IP   $C_DS ip=$DS_IP"
[ -n "$PROXY_IP" ] && [ -n "$DS_IP" ] || die "could not read container IPs"

say "stage binaries + helpers + workspace"
$SDME cp "$PROXY_BIN" "$C_PROXY:/root/devserver-proxy-service"
$SDME cp "$STUB_PY"   "$C_PROXY:/root/stub-identity.py"
$SDME exec "$C_PROXY" -- /bin/chmod +x /root/devserver-proxy-service
$SDME cp "$CHAN_BIN"  "$C_DS:/root/chan"
$SDME exec "$C_DS" -- /bin/chmod +x /root/chan
$SDME exec "$C_DS" -- /bin/sh -c \
  "mkdir -p /root/$WS_NAME /run/chan && printf '# e2e notes\nhello-through-the-tunnel\n' > /root/$WS_NAME/README.md"

say "start stub identity (loopback) in $C_PROXY"
# Mint close to use: the one-time rootfs/container setup above can take longer
# than an entry token lifetime on a cold host.
OWNER_ENTRY_TOKEN="$(python3 "$MINT_PY" --secret "$GATE_SECRET" --sub "$USER_ID" \
  --role owner --name 'Alice Owner' --email alice@example.test --drv "$DEVSERVER_ID" \
  --aud "$HOSTHDR" --typ entry --ttl 300)"
EDITOR_ENTRY_TOKEN="$(python3 "$MINT_PY" --secret "$GATE_SECRET" --sub "$EDITOR_USER_ID" \
  --role editor --name 'Ed Grantee' --email ed@example.test --drv "$DEVSERVER_ID" \
  --aud "$HOSTHDR" --typ entry --ttl 300)"
$SDME exec "$C_PROXY" -- /usr/bin/systemd-run --unit=stub --collect \
  --setenv=STUB_BIND=127.0.0.1:$STUB_PORT --setenv=STUB_USERNAME=$TENANT_USER \
  --setenv=STUB_USER_ID=$USER_ID --setenv=STUB_DEVSERVER_ID=$DEVSERVER_ID \
  --setenv=STUB_SCOPES=tunnel --setenv=STUB_PROXY_ORIGIN=$PROXY_ORIGIN \
  --setenv=STUB_DESKTOP_OWNER_PAT=$DESKTOP_OWNER_PAT \
  --setenv=STUB_DESKTOP_EDITOR_PAT=$DESKTOP_EDITOR_PAT \
  --setenv=STUB_OWNER_ENTRY_TOKEN=$OWNER_ENTRY_TOKEN \
  --setenv=STUB_EDITOR_ENTRY_TOKEN=$EDITOR_ENTRY_TOKEN \
  /usr/bin/python3 /root/stub-identity.py || die "systemd-run stub identity"
sleep 1

say "start devserver-proxy-service in $C_PROXY"
$SDME exec "$C_PROXY" -- /usr/bin/systemd-run --unit=dsp --collect \
  --setenv=RUST_LOG=info \
  --setenv=BIND_ADDR=0.0.0.0:$PROXY_PUB_PORT \
  --setenv=TUNNEL_BIND_ADDR=0.0.0.0:$PROXY_TUN_PORT \
  --setenv=IDENTITY_URL=http://127.0.0.1:$STUB_PORT \
  --setenv=IDENTITY_INTERNAL_TOKEN=$IDENTITY_TOKEN \
  --setenv=DEVSERVER_GATE_SECRET=$GATE_SECRET \
  --setenv=APEX_HOST=$APEX --setenv=WILDCARD_SUFFIX=$SUFFIX \
  --setenv=PUBLIC_SCHEME=http --setenv=FORWARDED_PROTO=http \
  --setenv=DASHBOARD_URL=http://id.localtest.me/workspaces \
  /root/devserver-proxy-service || die "systemd-run proxy"
sleep 2
$SDME exec "$C_PROXY" -- /usr/bin/systemctl is-active dsp >/dev/null 2>&1 \
  || { $SDME exec "$C_PROXY" -- /usr/bin/journalctl -u dsp --no-pager | tail -30; die "proxy not active"; }
curl -fsS "http://$PROXY_IP:$PROXY_PUB_PORT/healthz" >/dev/null 2>&1 \
  && info "proxy /healthz ok (host -> $PROXY_IP:$PROXY_PUB_PORT)" \
  || die "host cannot reach proxy public surface $PROXY_IP:$PROXY_PUB_PORT"

say "start chan devserver in $C_DS (tunnel -> $C_PROXY:$PROXY_TUN_PORT, same zone)"
TUNNEL_URL="http://$PROXY_IP:$PROXY_TUN_PORT/v1/tunnel"
info "tunnel-url = $TUNNEL_URL"
$SDME exec "$C_DS" -- /usr/bin/systemd-run --unit=chands --collect \
  --setenv=RUST_LOG=info --setenv=HOME=/root --setenv=XDG_RUNTIME_DIR=/run/chan \
  --setenv=CHAN_TUNNEL_TOKEN=$PAT --setenv=CHAN_DEVSERVER_LISTEN=1 \
  /root/chan devserver --bind 0.0.0.0 --port $DS_PORT --tunnel-url="$TUNNEL_URL" \
  || die "systemd-run chan devserver"

say "wait for tunnel registration"
REG=0
for _ in $(seq 1 30); do
  $SDME exec "$C_DS" -- /usr/bin/journalctl -u chands --no-pager 2>/dev/null | grep -q "tunnel connected" && { REG=1; break; }
  sleep 1
done
$SDME exec "$C_DS" -- /usr/bin/journalctl -u chands --no-pager 2>/dev/null | tail -12
[ "$REG" = 1 ] || { $SDME exec "$C_PROXY" -- /usr/bin/journalctl -u dsp --no-pager | tail -20; die "tunnel did not connect"; }
info "tunnel connected"

say "mount workspace via chan open (local devserver handoff)"
$SDME exec "$C_DS" -- /bin/sh -c "HOME=/root XDG_RUNTIME_DIR=/run/chan /root/chan open /root/$WS_NAME" 2>&1 | tail -6 || true
sleep 2
TOKEN="$($SDME exec "$C_DS" -- /bin/cat /root/.chan/devserver/config.json 2>/dev/null | python3 -c 'import sys,json;print(json.load(sys.stdin).get("devserver_token",""))' 2>/dev/null || true)"
WS_JSON="$(curl -fsS -H "Authorization: Bearer $TOKEN" "http://$DS_IP:$DS_PORT/api/devserver/workspaces" 2>/dev/null || true)"
info "workspaces: $WS_JSON"
PREFIX="$(printf '%s' "$WS_JSON" | python3 -c 'import sys,json
try:
 d=json.load(sys.stdin); rows=d if isinstance(d,list) else d.get("workspaces",[]); print(rows[0]["prefix"])
except Exception: print("")')"
[ -n "$PREFIX" ] || die "could not resolve mounted workspace prefix (token=${TOKEN:0:8}...)"
info "mounted prefix = $PREFIX"

# `chan open` registers through the live devserver handoff. Current lifecycle
# semantics keep a newly registered workspace off until the management API
# explicitly serves it, so activate it before asking the proxy for tenant HTML.
ON_JSON="$(curl -fsS -H "Authorization: Bearer $TOKEN" -H 'content-type: application/json' \
  --data '{"on":true}' "http://$DS_IP:$DS_PORT/api/devserver/workspaces$PREFIX/on")" \
  || die "could not activate mounted workspace"
printf '%s' "$ON_JSON" | python3 -c '
import json, sys
row = json.load(sys.stdin)
assert row["on"] is True
assert row["status"] == "running"
assert row["token"]
' || die "workspace activation did not reach running state"
info "workspace active through management API"

say "mint authenticated desktop entry responses"
ENTRY_BODY="$(printf '{\"owner\":\"%s\",\"devserver_id\":\"%s\",\"path\":\"%s/\"}' \
  "$TENANT_USER" "$DEVSERVER_ID" "$PREFIX")"
BAD_ENTRY_CODE="$($SDME exec "$C_PROXY" -- /usr/bin/curl -sS -o /tmp/bad-entry.json \
  -w '%{http_code}' -H 'Authorization: Bearer wrong' -H 'content-type: application/json' \
  --data "$ENTRY_BODY" "http://127.0.0.1:$STUB_PORT/desktop/v1/devserver/entry" || true)"
[ "$BAD_ENTRY_CODE" = "401" ] || die "desktop entry accepted an invalid bearer ($BAD_ENTRY_CODE)"

OWNER_ENTRY_JSON="$($SDME exec "$C_PROXY" -- /usr/bin/curl -fsS \
  -H "Authorization: Bearer $DESKTOP_OWNER_PAT" -H 'content-type: application/json' \
  --data "$ENTRY_BODY" "http://127.0.0.1:$STUB_PORT/desktop/v1/devserver/entry")" \
  || die "owner desktop entry response"
EDITOR_ENTRY_JSON="$($SDME exec "$C_PROXY" -- /usr/bin/curl -fsS \
  -H "Authorization: Bearer $DESKTOP_EDITOR_PAT" -H 'content-type: application/json' \
  --data "$ENTRY_BODY" "http://127.0.0.1:$STUB_PORT/desktop/v1/devserver/entry")" \
  || die "editor desktop entry response"
printf '%s\n%s\n' "$OWNER_ENTRY_JSON" "$EDITOR_ENTRY_JSON" | python3 -c '
import json, sys
rows = [json.loads(line) for line in sys.stdin if line.strip()]
assert len(rows) == 2
for row in rows:
    assert row["username"] == sys.argv[1]
    assert row["devserver_id"] == sys.argv[2] and len(row["devserver_id"]) == 64
    assert row["proxy_origin"] == sys.argv[3]
    assert row["entry_url"].startswith(sys.argv[3] + sys.argv[4] + "/?t=")
    assert row["expires_at"].endswith("Z")
' "$TENANT_USER" "$DEVSERVER_ID" "$PROXY_ORIGIN" "$PREFIX" \
  || die "desktop entry response identity/origin validation"
info "entry response pins username + full id + exact origin for owner and editor"

OWNER_ENTRY_URL="$(printf '%s' "$OWNER_ENTRY_JSON" | python3 -c 'import json,sys;print(json.load(sys.stdin)["entry_url"])')"
EDITOR_ENTRY_URL="$(printf '%s' "$EDITOR_ENTRY_JSON" | python3 -c 'import json,sys;print(json.load(sys.stdin)["entry_url"])')"
OWNER_ENTRY_H="$(mktemp)"; EDITOR_ENTRY_H="$(mktemp)"
OWNER_ENTRY_CODE="$(curl --noproxy '*' --resolve "$HOST_NAME:$PROXY_PUB_PORT:$PROXY_IP" \
  -sS -o /dev/null -D "$OWNER_ENTRY_H" -w '%{http_code}' "$OWNER_ENTRY_URL" || echo 000)"
EDITOR_ENTRY_CODE="$(curl --noproxy '*' --resolve "$HOST_NAME:$PROXY_PUB_PORT:$PROXY_IP" \
  -sS -o /dev/null -D "$EDITOR_ENTRY_H" -w '%{http_code}' "$EDITOR_ENTRY_URL" || echo 000)"
[ "$OWNER_ENTRY_CODE" = "303" ] && [ "$EDITOR_ENTRY_CODE" = "303" ] \
  || die "entry exchange did not mint sessions (owner=$OWNER_ENTRY_CODE editor=$EDITOR_ENTRY_CODE)"
OWNER_GATE="$(sed -n 's/^set-cookie: devserver_gate=\([^;]*\).*/\1/ip' "$OWNER_ENTRY_H" | head -1 | tr -d '\r')"
OWNER_CSRF="$(sed -n 's/^set-cookie: devserver_csrf=\([^;]*\).*/\1/ip' "$OWNER_ENTRY_H" | head -1 | tr -d '\r')"
EDITOR_GATE="$(sed -n 's/^set-cookie: devserver_gate=\([^;]*\).*/\1/ip' "$EDITOR_ENTRY_H" | head -1 | tr -d '\r')"
EDITOR_CSRF="$(sed -n 's/^set-cookie: devserver_csrf=\([^;]*\).*/\1/ip' "$EDITOR_ENTRY_H" | head -1 | tr -d '\r')"
[ -n "$OWNER_GATE" ] && [ -n "$OWNER_CSRF" ] && [ -n "$EDITOR_GATE" ] && [ -n "$EDITOR_CSRF" ] \
  || die "entry exchange omitted session/csrf cookies"

say "drive authenticated owner request through the proxy"
RESP_H="$(mktemp)"; RESP_B="$(mktemp)"
CODE="$(curl --noproxy '*' --resolve "$HOST_NAME:$PROXY_PUB_PORT:$PROXY_IP" \
  -sS -o "$RESP_B" -D "$RESP_H" -w '%{http_code}' \
  -H "Cookie: devserver_gate=$OWNER_GATE; devserver_csrf=$OWNER_CSRF" \
  "$PROXY_ORIGIN$PREFIX/" || echo 000)"

say "prove native-trust routes and require_local_mutation"
TRUST_PATH="/api/library/devservers/gw%3Afeedface%3A$TENANT_USER%3A$DEVSERVER_ID/native-trust"
MUT_B="$(mktemp)"
for METHOD in PUT DELETE; do
  OWNER_MUT_CODE="$(curl --noproxy '*' --resolve "$HOST_NAME:$PROXY_PUB_PORT:$PROXY_IP" \
    -sS -o "$MUT_B" -w '%{http_code}' -X "$METHOD" \
    -H "Cookie: devserver_gate=$OWNER_GATE; devserver_csrf=$OWNER_CSRF" \
    -H "x-chan-csrf: $OWNER_CSRF" "$PROXY_ORIGIN$TRUST_PATH" || echo 000)"
  [ "$OWNER_MUT_CODE" = "409" ] && grep -qx 'window management requires the chan desktop app' "$MUT_B" \
    || die "owner $METHOD native-trust did not reach desktop bridge guard ($OWNER_MUT_CODE)"

  EDITOR_MUT_CODE="$(curl --noproxy '*' --resolve "$HOST_NAME:$PROXY_PUB_PORT:$PROXY_IP" \
    -sS -o "$MUT_B" -w '%{http_code}' -X "$METHOD" \
    -H "Cookie: devserver_gate=$EDITOR_GATE; devserver_csrf=$EDITOR_CSRF" \
    -H "x-chan-csrf: $EDITOR_CSRF" "$PROXY_ORIGIN$TRUST_PATH" || echo 000)"
  [ "$EDITOR_MUT_CODE" = "403" ] \
    && grep -qx 'launcher mutation is not available for this gateway role' "$MUT_B" \
    || die "editor $METHOD native-trust bypassed require_local_mutation ($EDITOR_MUT_CODE)"
  info "$METHOD native-trust: owner reached route (409 no desktop); editor refused (403)"
done

say "RESULT"
echo "REQUEST : GET $PREFIX/   Host: $HOSTHDR (authenticated owner entry)"
echo "          via proxy public $PROXY_IP:$PROXY_PUB_PORT  ->  tunnel $TUNNEL_URL  ->  $C_DS"
echo "STATUS  : $CODE"
echo "--- response headers ---"; sed -n '1,12p' "$RESP_H"
echo "--- body (head) ---"; head -c 600 "$RESP_B"; echo
if [ "$CODE" = "200" ] && grep -q "chan-prefix" "$RESP_B"; then
  MARK="$(grep -o 'name="chan-prefix"[^>]*' "$RESP_B" | head -1)"
  printf '\n\033[1;32mPASS\033[0m: 200 through proxy+tunnel; body is the workspace SPA (%s)\n' "$MARK"
  rm -f "$RESP_H" "$RESP_B" "$OWNER_ENTRY_H" "$EDITOR_ENTRY_H" "$MUT_B"
  info "leaving containers up; re-run with --clean to remove"
  exit 0
fi
echo "--- proxy log ---";     $SDME exec "$C_PROXY" -- /usr/bin/journalctl -u dsp    --no-pager | tail -25
echo "--- devserver log ---"; $SDME exec "$C_DS"    -- /usr/bin/journalctl -u chands --no-pager | tail -25
die "expected 200 with chan-prefix marker; got $CODE"
