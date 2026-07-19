#!/usr/bin/env bash
set -euo pipefail

# Live validation for chronological `cs terminal write` batching. Run from a
# chan terminal whose CHAN_CONTROL_SOCKET targets the server build under test.
# Diagnostics go to stderr; one result row per run goes to stdout.

agent=codex
size_kib=64
runs=3
timeout_secs=120
cs_bin=${CS_BIN:-cs}

usage() {
  cat >&2 <<'EOF'
usage: terminal-queue-drain.sh [--agent codex|claude] [--size-kib N] [--runs N] [--timeout-secs N]

Environment:
  CS_BIN  cs-compatible binary (default: cs). A development chan binary must
          be invoked through a symlink named cs so it enters that command surface.

Payloads above the production 64 KiB batch ceiling are advisory and require a
scratch build with a temporarily raised selector ceiling.
EOF
}

while (($#)); do
  case "$1" in
    --agent)
      agent=$2
      shift 2
      ;;
    --size-kib)
      size_kib=$2
      shift 2
      ;;
    --runs)
      runs=$2
      shift 2
      ;;
    --timeout-secs)
      timeout_secs=$2
      shift 2
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage
      exit 2
      ;;
  esac
done

case "$agent" in
  codex)
    launch='codex --sandbox read-only -a never --no-alt-screen'
    ready_pattern='Codex'
    ;;
  claude)
    launch='claude --permission-mode plan --tools ""'
    ready_pattern='plan mode on'
    ;;
  *)
    echo "unsupported agent: $agent" >&2
    exit 2
    ;;
esac

[[ $size_kib =~ ^[1-9][0-9]*$ ]] || { echo "--size-kib must be positive" >&2; exit 2; }
[[ $runs =~ ^[1-9][0-9]*$ ]] || { echo "--runs must be positive" >&2; exit 2; }
[[ $timeout_secs =~ ^[1-9][0-9]*$ ]] || { echo "--timeout-secs must be positive" >&2; exit 2; }
: "${CHAN_CONTROL_SOCKET:?run from a chan terminal or export the test server control socket}"
: "${CHAN_WINDOW_ID:?run from a chan terminal or export the test browser window id}"

stamp="$(date +%s)-$$"
group="queue-drain-probe-$stamp"
tab="QueueDrain-${agent}-${stamp}"

cs_run() {
  "$cs_bin" "$@"
}

cleanup() {
  cs_run terminal close --tab-group="$group" >/dev/null 2>&1 || true
}
trap cleanup EXIT INT TERM

scrollback_contains() {
  local pattern=$1
  cs_run terminal scrollback --tab-name="$tab" 2>/dev/null | rg -a -F -q "$pattern"
}

wait_for_pattern() {
  local pattern=$1
  local deadline=$((SECONDS + timeout_secs))
  until scrollback_contains "$pattern"; do
    if ((SECONDS >= deadline)); then
      echo "timed out waiting for $pattern in $tab" >&2
      return 1
    fi
    sleep 0.1
  done
}

wait_for_session() {
  local deadline=$((SECONDS + timeout_secs))
  until cs_run terminal scrollback --tab-name="$tab" >/dev/null 2>&1; do
    if ((SECONDS >= deadline)); then
      echo "timed out waiting for terminal session $tab" >&2
      return 1
    fi
    sleep 0.1
  done
}

scrollback_matches_response() {
  local pattern='QUEUE_DRAIN_BATCH_OK'
  local token
  for token in "$@"; do
    pattern+="(?s:.)*$token"
  done
  cs_run terminal scrollback --tab-name="$tab" 2>/dev/null \
    | sed -E $'s|\x1b\\[[0-?]*[ -/]*[@-~]||g' \
    | tr -d '\r\n' \
    | rg -a -q "$pattern"
}

wait_for_response() {
  local deadline=$((SECONDS + timeout_secs))
  until scrollback_matches_response "$@"; do
    if ((SECONDS >= deadline)); then
      echo "timed out waiting for ordered response in $tab" >&2
      return 1
    fi
    sleep 0.1
  done
}

paste_placeholder_count() {
  local matches
  matches=$(cs_run terminal scrollback --tab-name="$tab" 2>/dev/null \
    | rg -a -o '\[Pasted text #[0-9]+' || true)
  if [[ -z $matches ]]; then
    echo 0
  else
    printf '%s\n' "$matches" | sort -u | wc -l
  fi
}

agent_version=$("$agent" --version 2>&1)
echo "starting $agent probe version=$agent_version tab=$tab group=$group size=${size_kib}KiB runs=$runs" >&2
cs_run terminal new --tab-name="$tab" --tab-group="$group" >/dev/null
wait_for_session
cs_run terminal write --tab-name="$tab" "$launch"$'\n' >/dev/null
wait_for_pattern "$ready_pattern"

target_bytes=$((size_kib * 1024))
envelope_reserve=1024
((target_bytes > envelope_reserve)) || envelope_reserve=512
filler_bytes=$(((target_bytes - envelope_reserve) / 5))
((filler_bytes > 0)) || filler_bytes=1

for ((run = 1; run <= runs; run++)); do
  placeholders_before=$(paste_placeholder_count)
  tokens=("a${run}x$$" "b${run}x$$" "c${run}x$$" "d${run}x$$" "e${run}x$$")
  warmup="QUEUE_DRAIN_WARMUP_${run}_$$"
  cs_run terminal write --submit="$agent" --tab-name="$tab" \
    "Reply with exactly $warmup and nothing else." >/dev/null
  wait_for_pattern "$warmup"
  for ((index = 1; index <= 5; index++)); do
    token=${tokens[index - 1]}
    awk -v message_index="$index" -v fill="$filler_bytes" -v token="$token" 'BEGIN {
      if (message_index == 1) {
        print "Read the entire queued-notification batch before acting. Reply with exactly one line. Start it by joining QUEUE, DRAIN, BATCH, and OK with underscores, then append the five token values in order. Do not use tools."
      }
      c = sprintf("%c", 64 + message_index)
      for (i = 0; i < fill; i++) printf "%s", c
      printf "\ntoken: %s\n", token
    }' | cs_run terminal write --stdin --submit="$agent" --tab-name="$tab" >/dev/null
  done

  wait_for_response "${tokens[@]}"
  placeholders_after=$(paste_placeholder_count)
  if ((placeholders_after > placeholders_before)); then
    placeholder=yes
  else
    placeholder=no
  fi
  printf 'agent=%s run=%d size_kib=%d submitted=yes tokens=ok paste_placeholder=%s\n' \
    "$agent" "$run" "$size_kib" "$placeholder"
done
