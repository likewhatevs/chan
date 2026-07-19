#!/usr/bin/env bash
set -euo pipefail

# Live validation for chronological `cs terminal write` batching. Run from a
# chan terminal whose CHAN_CONTROL_SOCKET targets the server build under test.
# Diagnostics go to stderr; one result row per run goes to stdout.
#
# Every run gets its OWN tab inside one probe group, so every scrollback
# assertion below is scoped to the run that made it. The group is closed on
# exit, including on failure.
#
# What each case proves, and how:
#
#   batch       Five notifications enqueued while the agent is provably
#               generating drain as ONE turn. Oracles: the server framed one
#               5-message envelope (`--- notification 5/5 ---` present, no
#               1/1..1/4 partial framing), the agent answered once for all five
#               (it BUILDS the sentinel `QUEUE_DRAIN_BATCH_5`, which never
#               appears in its input, so an echo cannot satisfy it), the five
#               tail tokens arrived in order, and the polled queue depth went
#               5 -> 0 without an intermediate 4/3/2/1.
#   boundaries  A no-submit write and an override-backed write each END the
#               batch prefix. Five messages whose every neighbour is a boundary
#               produce NO envelope at all, and the tail tokens still arrive in
#               FIFO order, so nothing was skipped to batch a later message.
#   late        A sixth notification enqueued once the batch has been selected
#               lands in the NEXT turn: the 5/5 envelope exists and no 6/6 one
#               does.
#
# NOT covered here: the Rich Prompt boundary. Rich Prompt enters the queue over
# the terminal WebSocket, which `cs` does not speak, so no shell harness can
# place one between notifications. That boundary is pinned by the chan-library
# and chan-server unit tests and by a browser smoke.

agent=codex
size_kib=64
runs=3
timeout_secs=180
case_name=batch
gap_ms=50
# Mirrors `WRITE_QUEUE_BATCH_MAX_BYTES` in
# crates/chan-library/src/terminal_sessions.rs. The payload is sized against
# THIS number rather than a hand-picked headroom, and the 5/5 envelope check
# below fails loudly if a framing change pushes the last notification out.
max_batch_bytes=$((64 * 1024))
cs_bin=${CS_BIN:-cs}

usage() {
  cat >&2 <<'EOF'
usage: terminal-queue-drain.sh [--agent codex|claude] [--case batch|boundaries|late|all]
                               [--size-kib N] [--runs N] [--timeout-secs N]
                               [--gap MS] [--max-batch-bytes N]

Environment:
  CS_BIN                      cs-compatible binary (default: cs). A development
                              chan binary must be invoked through a symlink
                              named cs so it enters that command surface.
  CHAN_TERMINAL_INPUT_GAP_MS  The SERVER's Claude body/chord split gap. --gap
                              only asserts this matches; the value is read once
                              per server process, so sweeping it means
                              restarting the server under test with the new
                              value exported.

Payloads above the batch ceiling are advisory and require a scratch build with
a temporarily raised selector ceiling; pass --max-batch-bytes to match it.
EOF
}

while (($#)); do
  case "$1" in
    --agent)
      agent=$2
      shift 2
      ;;
    --case)
      case_name=$2
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
    --gap)
      gap_ms=$2
      shift 2
      ;;
    --max-batch-bytes)
      max_batch_bytes=$2
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
    submit_override='\e[200~{}\e[201~\r'
    ;;
  claude)
    launch='claude --permission-mode plan --tools ""'
    ready_pattern='plan mode on'
    submit_override='{}\e[27;9;13~'
    ;;
  *)
    echo "unsupported agent: $agent" >&2
    exit 2
    ;;
esac

case "$case_name" in
  batch | boundaries | late | all) ;;
  *)
    echo "unsupported case: $case_name" >&2
    exit 2
    ;;
esac

[[ $size_kib =~ ^[1-9][0-9]*$ ]] || { echo "--size-kib must be positive" >&2; exit 2; }
[[ $runs =~ ^[1-9][0-9]*$ ]] || { echo "--runs must be positive" >&2; exit 2; }
[[ $timeout_secs =~ ^[1-9][0-9]*$ ]] || { echo "--timeout-secs must be positive" >&2; exit 2; }
[[ $gap_ms =~ ^[1-9][0-9]*$ ]] || { echo "--gap must be positive" >&2; exit 2; }
[[ $max_batch_bytes =~ ^[1-9][0-9]*$ ]] || { echo "--max-batch-bytes must be positive" >&2; exit 2; }
: "${CHAN_CONTROL_SOCKET:?run from a chan terminal or export the test server control socket}"
: "${CHAN_WINDOW_ID:?run from a chan terminal or export the test browser window id}"

# A chan terminal inherits the server's environment, so this reads the gap the
# server under test actually uses. Asserting it here is what makes a gap sweep
# reproducible from committed code instead of a rebuild nobody can repeat.
server_gap_ms=${CHAN_TERMINAL_INPUT_GAP_MS:-50}
if [[ $server_gap_ms != "$gap_ms" ]]; then
  echo "server split gap is ${server_gap_ms}ms, --gap asked for ${gap_ms}ms" >&2
  echo "restart the server under test with CHAN_TERMINAL_INPUT_GAP_MS=$gap_ms" >&2
  exit 2
fi

stamp="$(date +%s)-$$"
group="queue-drain-probe-$stamp"
tab=""

cleanup() {
  "$cs_bin" terminal close --tab-group="$group" >/dev/null 2>&1 || true
}
trap cleanup EXIT INT TERM

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

# Scrollback with ANSI escapes removed and newlines folded away, so an
# assertion is not defeated by a TUI wrapping a line at the pane width.
flat_scrollback() {
  "$cs_bin" terminal scrollback --tab-name="$tab" 2>/dev/null \
    | sed -E $'s|\x1b\\[[0-?]*[ -/]*[@-~]||g' \
    | tr -d '\r\n'
}

flat_contains() {
  flat_scrollback | rg -a -F -q -e "$1"
}

wait_for_flat() {
  local needle=$1
  local deadline=$((SECONDS + timeout_secs))
  until flat_contains "$needle"; do
    if ((SECONDS >= deadline)); then
      fail "timed out waiting for '$needle' in $tab"
    fi
    sleep 0.1
  done
}

wait_for_session() {
  local deadline=$((SECONDS + timeout_secs))
  until "$cs_bin" terminal scrollback --tab-name="$tab" >/dev/null 2>&1; do
    if ((SECONDS >= deadline)); then
      fail "timed out waiting for terminal session $tab"
    fi
    sleep 0.1
  done
}

# Pending logical messages for this tab. `cs terminal list --json` reports the
# same count the SPA queue badge shows.
queue_depth() {
  "$cs_bin" terminal list --json 2>/dev/null | python3 -c '
import json, sys
want = sys.argv[1]
try:
    groups = json.load(sys.stdin).get("groups", {})
except ValueError:
    print("-1")
    sys.exit(0)
for sessions in groups.values():
    for session in sessions:
        if session.get("name") == want:
            print(session.get("queue_depth", -1))
            sys.exit(0)
print("-1")
' "$tab"
}

require_depth() {
  local want=$1 now
  now=$(queue_depth)
  [[ $now == "$want" ]] || fail "expected queue depth $want, got $now"
}

wait_for_depth() {
  local want=$1
  local deadline=$((SECONDS + timeout_secs))
  until [[ $(queue_depth) == "$want" ]]; do
    if ((SECONDS >= deadline)); then
      fail "timed out waiting for queue depth $want (now $(queue_depth))"
    fi
    sleep 0.05
  done
}

# Sample the depth until it reaches 0 and print the observed sequence. A sample
# can only FALSIFY the one-step contract: seeing 4/3/2/1 after a 5 proves
# per-message drains, while never seeing them is consistent with one step.
sample_depth_to_zero() {
  local deadline=$((SECONDS + timeout_secs))
  local depth
  while :; do
    depth=$(queue_depth)
    printf '%s\n' "$depth"
    [[ $depth == 0 ]] && return 0
    if ((SECONDS >= deadline)); then
      fail "timed out sampling queue depth (now $depth)"
    fi
    sleep 0.05
  done
}

paste_placeholder_count() {
  local matches
  matches=$("$cs_bin" terminal scrollback --tab-name="$tab" 2>/dev/null \
    | rg -a -o '\[Pasted text #[0-9]+' || true)
  if [[ -z $matches ]]; then
    echo 0
  else
    printf '%s\n' "$matches" | sort -u | wc -l
  fi
}

# The framed envelope this server build produces around N messages, with empty
# bodies. Sizing the payload against the REAL ceiling means measuring this
# rather than reserving a guessed constant.
envelope_overhead() {
  python3 -c '
import sys
count = int(sys.argv[1])
out = "# Queued terminal notifications\n\n%d messages, oldest first. Read the entire batch before acting. Later\nmessages may update or supersede earlier messages.\n\n" % count
for number in range(1, count + 1):
    out += "--- notification %d/%d ---\n" % (number, count)
    out += "\n"
    out += "--- end notification %d/%d ---\n" % (number, count)
    if number != count:
        out += "\n"
print(len(out.encode()))
' "$1"
}

batch_instruction() {
  printf '%s' "Read the entire queued-notification batch before acting. Reply with exactly one line. Start it by joining QUEUE, DRAIN, BATCH, and the NUMBER of notification blocks you received with underscores, then append the token values in order. Do not use tools."
}

# One notification body: filler bytes, then a tail token on its own line. The
# first message of a run also carries the instruction the agent must answer.
emit_message() {
  local index=$1 fill=$2 token=$3 preamble=$4
  awk -v message_index="$index" -v fill="$fill" -v token="$token" -v preamble="$preamble" 'BEGIN {
    if (preamble != "") print preamble
    c = sprintf("%c", 64 + message_index)
    for (i = 0; i < fill; i++) printf "%s", c
    printf "\ntoken: %s\n", token
  }'
}

tokens_in_order() {
  local pattern=$1
  shift
  local token
  for token in "$@"; do
    pattern+="(?s:.)*$token"
  done
  flat_scrollback | rg -a -q -e "$pattern"
}

wait_for_tokens_in_order() {
  local deadline=$((SECONDS + timeout_secs))
  until tokens_in_order "$@"; do
    if ((SECONDS >= deadline)); then
      fail "timed out waiting for the ordered tail tokens in $tab"
    fi
    sleep 0.1
  done
}

# A fresh tab per run keeps every scrollback assertion scoped to its own run.
start_terminal() {
  local run=$1
  tab="QueueDrain-${agent}-${stamp}-${case_name}-${run}"
  "$cs_bin" terminal new --tab-name="$tab" --tab-group="$group" >/dev/null
  wait_for_session
  "$cs_bin" terminal write --tab-name="$tab" "$launch"$'\n' >/dev/null
  wait_for_flat "$ready_pattern"
}

# Make the agent generate for long enough that the notifications below are
# provably enqueued against a BUSY agent. The start marker is BUILT by the
# agent, so it cannot be satisfied by the echo of this instruction.
warmup_until_busy() {
  local run=$1
  "$cs_bin" terminal write --submit="$agent" --tab-name="$tab" \
    "Without using tools: first print one line joining WARMUP, START, $run, and $$ with underscores, then count from 1 to 400 with one number per line, then print one line joining WARMUP, DONE, $run, and $$ with underscores." >/dev/null
  wait_for_flat "WARMUP_START_${run}_$$"
}

instruction=$(batch_instruction)
agent_version=$("$agent" --version 2>&1)
echo "starting $agent probe version=$agent_version group=$group case=$case_name size=${size_kib}KiB gap=${gap_ms}ms runs=$runs" >&2

# Every byte of the framed batch is accounted for: the measured envelope, the
# instruction the first message carries, one "token: <token>" line per message,
# and a small rounding margin. Nothing here is a guessed headroom, so a wording
# change moves the filler instead of silently pushing a notification out.
target_bytes=$((size_kib * 1024))
((target_bytes <= max_batch_bytes)) || target_bytes=$max_batch_bytes
overhead=$(envelope_overhead 5)
instruction_bytes=$(( ${#instruction} + 1 ))
token_line_bytes=$((5 * 48))
margin=64
budget=$((target_bytes - overhead - instruction_bytes - token_line_bytes - margin))
((budget > 0)) || fail "size ${size_kib}KiB leaves no room for five bodies (envelope ${overhead}B)"
filler_bytes=$((budget / 5))
((filler_bytes > 0)) || filler_bytes=1
echo "payload: ceiling=${max_batch_bytes}B target=${target_bytes}B envelope=${overhead}B instruction=${instruction_bytes}B filler=${filler_bytes}B/message" >&2

run_batch_case() {
  local run=$1
  local tokens=("a${run}x$$" "b${run}x$$" "c${run}x$$" "d${run}x$$" "e${run}x$$")
  local index token preamble depths depth saw_five=0 placeholder=no

  start_terminal "$run"
  warmup_until_busy "$run"
  for ((index = 1; index <= 5; index++)); do
    token=${tokens[index - 1]}
    preamble=""
    if ((index == 1)); then
      preamble=$instruction
    fi
    emit_message "$index" "$filler_bytes" "$token" "$preamble" \
      | "$cs_bin" terminal write --stdin --submit="$agent" --tab-name="$tab" >/dev/null
  done
  # Nothing may reach a busy compose box: all five must still be pending.
  require_depth 5

  depths=$(sample_depth_to_zero)
  for depth in $depths; do
    if [[ $depth == 5 ]]; then
      saw_five=1
    elif ((saw_five == 1)) && [[ $depth != 0 ]]; then
      fail "depth stepped through $depth after 5; the batch drained per message"
    fi
  done

  wait_for_tokens_in_order 'QUEUE_DRAIN_BATCH_5' "${tokens[@]}"
  flat_contains "--- notification 5/5 ---" || fail "no 5-message envelope was framed"
  for index in 1 2 3 4; do
    if flat_contains "--- notification 1/${index} ---"; then
      fail "the prefix was framed as ${index} messages, not 5"
    fi
    if flat_contains "QUEUE_DRAIN_BATCH_${index}"; then
      fail "the agent answered a ${index}-notification turn"
    fi
  done

  if (($(paste_placeholder_count) > 0)); then
    placeholder=yes
  fi
  printf 'agent=%s case=batch run=%d size_kib=%d gap_ms=%d envelope=5/5 turns=1 tokens=ok paste_placeholder=%s\n' \
    "$agent" "$run" "$size_kib" "$gap_ms" "$placeholder"
}

run_boundaries_case() {
  local run=$1
  # Every neighbour is a boundary: submitted, raw, submitted, override,
  # submitted. Nothing may batch, and nothing may be skipped to reach a later
  # batchable message, so the tokens must still arrive in FIFO order.
  local tokens=("p${run}x$$" "q${run}x$$" "r${run}x$$" "s${run}x$$" "t${run}x$$")
  local env_key
  env_key="CHAN_SUBMIT_$(printf '%s' "$agent" | tr '[:lower:]' '[:upper:]')"

  start_terminal "$run"
  warmup_until_busy "$run"
  emit_message 1 32 "${tokens[0]}" "$instruction" \
    | "$cs_bin" terminal write --stdin --submit="$agent" --tab-name="$tab" >/dev/null
  # A no-submit write parks in the compose box: it is a boundary AND it must
  # not be reordered behind the submitted messages around it.
  emit_message 2 32 "${tokens[1]}" "" \
    | "$cs_bin" terminal write --stdin --tab-name="$tab" >/dev/null
  emit_message 3 32 "${tokens[2]}" "" \
    | "$cs_bin" terminal write --stdin --submit="$agent" --tab-name="$tab" >/dev/null
  # A runtime template override is carried over the control wire and stays
  # single-message even though its bytes match the built-in default.
  emit_message 4 32 "${tokens[3]}" "" \
    | env "$env_key=$submit_override" "$cs_bin" terminal write --stdin --submit="$agent" --tab-name="$tab" >/dev/null
  emit_message 5 32 "${tokens[4]}" "" \
    | "$cs_bin" terminal write --stdin --submit="$agent" --tab-name="$tab" >/dev/null

  require_depth 5
  wait_for_depth 0
  wait_for_tokens_in_order "${tokens[@]}"
  if flat_contains "# Queued terminal notifications"; then
    fail "messages separated by boundaries were framed as a batch"
  fi

  printf 'agent=%s case=boundaries run=%d envelope=none order=ok\n' "$agent" "$run"
}

run_late_case() {
  local run=$1
  local tokens=("f${run}x$$" "g${run}x$$" "h${run}x$$" "i${run}x$$" "j${run}x$$")
  local late="k${run}x$$"
  local index token preamble

  start_terminal "$run"
  warmup_until_busy "$run"
  for ((index = 1; index <= 5; index++)); do
    token=${tokens[index - 1]}
    preamble=""
    if ((index == 1)); then
      preamble=$instruction
    fi
    emit_message "$index" 32 "$token" "$preamble" \
      | "$cs_bin" terminal write --stdin --submit="$agent" --tab-name="$tab" >/dev/null
  done
  require_depth 5
  # The prefix is selected and popped atomically, so the depth leaving 5 is the
  # first instant at which a new message is guaranteed to miss it.
  wait_for_depth 0
  emit_message 6 32 "$late" "$instruction" \
    | "$cs_bin" terminal write --stdin --submit="$agent" --tab-name="$tab" >/dev/null
  wait_for_depth 0
  wait_for_tokens_in_order "${tokens[@]}" "$late"

  flat_contains "--- notification 5/5 ---" || fail "no 5-message envelope was framed"
  if flat_contains "--- notification 6/6 ---"; then
    fail "the late message joined the batch it was enqueued after"
  fi

  printf 'agent=%s case=late run=%d envelope=5/5 late_turn=separate\n' "$agent" "$run"
}

for ((run = 1; run <= runs; run++)); do
  case "$case_name" in
    batch) run_batch_case "$run" ;;
    boundaries) run_boundaries_case "$run" ;;
    late) run_late_case "$run" ;;
    all)
      run_batch_case "$run"
      run_boundaries_case "$run"
      run_late_case "$run"
      ;;
  esac
done
