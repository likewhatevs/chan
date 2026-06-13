// Item-2 manual-recipe walker against the throwaway standalone server.
// Drives the REAL wire: terminal WS attach, busy-agent quiet-gate hold,
// tagged gemini 2-write prompt, cs-terminal-write interleave, drain
// ordering (delivered-before-queue), reattach depth re-sync, idle fast
// path, all-or-nothing cap straddle + rejected ack, CLI stdout regression.
import { spawnSync } from "node:child_process";

const BASE = "ws://localhost:8923";
const TOKEN = "zbQCMmYVnD8zVBdaXDJ3WZognveZ7DYF";
const SOCK = process.env.SMOKE_CONTROL_SOCKET;
if (!SOCK) throw new Error("SMOKE_CONTROL_SOCKET not set");

const log = (...a) => console.log(`[${(Date.now() - t0) / 1000}s]`, ...a);
const t0 = Date.now();
let failures = 0;
function check(cond, name, detail = "") {
  if (cond) log(`PASS ${name}`);
  else {
    failures += 1;
    log(`FAIL ${name} ${detail}`);
  }
}

function connect(tabName, session) {
  const params = new URLSearchParams({ cols: "100", rows: "30", tab_name: tabName, t: TOKEN });
  if (session) {
    params.set("session", session);
    params.set("since", "0");
  }
  const ws = new WebSocket(`${BASE}/api/terminal/ws?${params}`);
  ws.binaryType = "arraybuffer";
  const conn = { ws, frames: [], output: "", waiters: [] };
  ws.onmessage = (ev) => {
    if (typeof ev.data === "string") {
      const frame = JSON.parse(ev.data);
      conn.frames.push(frame);
      log(`  <- ${tabName}#${session ? "B" : "A"}:`, JSON.stringify(frame).slice(0, 140));
      for (const w of [...conn.waiters]) w();
    } else {
      conn.output += Buffer.from(ev.data).toString("utf8");
    }
  };
  return conn;
}

function waitFor(conn, pred, what, ms = 15000) {
  return new Promise((resolve, reject) => {
    const scan = () => conn.frames.find(pred);
    const hit = scan();
    if (hit) return resolve(hit);
    const timer = setTimeout(() => reject(new Error(`timeout waiting for ${what}`)), ms);
    conn.waiters.push(() => {
      const hit = scan();
      if (hit) {
        clearTimeout(timer);
        resolve(hit);
      }
    });
  });
}
const send = (conn, frame) => conn.ws.send(JSON.stringify(frame));
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
// cs prints the control response on stderr (pre-existing CLI behavior;
// the regression contract is the STRING + position semantics). Merge fds.
const csWrite = (text) => {
  const res = spawnSync("cs", ["terminal", "write", "--tab-name", "@@Smoke", text], {
    env: { ...process.env, CHAN_CONTROL_SOCKET: SOCK },
    encoding: "utf8",
  });
  return `${res.stdout ?? ""}${res.stderr ?? ""}`.trim();
};

const a = connect("@@Smoke");
await new Promise((r) => (a.ws.onopen = r));
const sessionFrame = await waitFor(a, (f) => f.type === "session", "session A");
check(sessionFrame.queue_depth === 0, "fresh session frame carries queue_depth 0",
  JSON.stringify(sessionFrame));
await waitFor(a, (f) => f.type === "ready", "ready A");
await sleep(1200); // let the shell prompt settle

// 1. Busy agent: sub-800ms output holds the quiet gate.
send(a, { type: "input", data: "while true; do date; sleep 0.3; done\r" });
await sleep(1500);

// 2. Tagged gemini prompt (plain shell => 2 writes: text + CR).
send(a, { type: "prompt", data: "echo SMOKE-QUEUE-1", agent: "gemini", id: "smoke-1" });
const ack1 = await waitFor(a, (f) => f.type === "prompt-ack" && f.id === "smoke-1", "ack smoke-1");
check(ack1.queued === true && ack1.depth === 1, "ack: queued at message position 1",
  JSON.stringify(ack1));
await waitFor(a, (f) => f.type === "queue" && f.depth === 1, "queue depth 1");

// 3. Busy agent holds delivery: no prompt-delivered while the loop runs.
await sleep(2500);
check(!a.frames.some((f) => f.type === "prompt-delivered"),
  "busy agent: nothing delivered while the loop floods output");

// 4. cs terminal write x3: CLI positions are RAW entries (3,4,5 behind the
//    2-entry gemini pair) while queue depths are MESSAGES (2,3,4).
const w1 = csWrite("echo poke-1\n");
check(w1 === "queued at position 3", "cs stdout: raw position 3 behind the pair", w1);
await waitFor(a, (f) => f.type === "queue" && f.depth === 2, "queue depth 2");
const w2 = csWrite("echo poke-2\n");
check(w2 === "queued at position 4", "cs stdout: raw position 4", w2);
const w3 = csWrite("echo poke-3\n");
check(w3 === "queued at position 5", "cs stdout: raw position 5", w3);
await waitFor(a, (f) => f.type === "queue" && f.depth === 4, "queue depth 4");

// 5. Reattach mid-queue on a second socket: session frame re-syncs depth.
const b = connect("@@Smoke", sessionFrame.id);
await new Promise((r) => (b.ws.onopen = r));
const sessB = await waitFor(b, (f) => f.type === "session", "session B");
check(sessB.queue_depth === 4, "reattach session frame re-syncs queue_depth 4",
  JSON.stringify(sessB));

// 6. Ctrl-C the loop -> drains one message per idle/gen cycle. The tagged
//    pair drains body (no events) then chord: delivered THEN queue, both 3.
send(a, { type: "input", data: "" });
const delivered1 = await waitFor(a, (f) => f.type === "prompt-delivered" && f.id === "smoke-1",
  "delivered smoke-1", 30000);
check(delivered1.depth === 3, "delivered carries remaining message depth 3",
  JSON.stringify(delivered1));
const dIdx = a.frames.indexOf(delivered1);
await waitFor(
  a,
  (f, i) => f.type === "queue" && a.frames.indexOf(f) > dIdx,
  "a queue frame after delivered",
);
const firstQueueAfter = a.frames.slice(dIdx + 1).find((f) => f.type === "queue");
check(firstQueueAfter?.depth === 3,
  "the first queue frame after delivered carries depth 3 (delivered-first ordering)",
  JSON.stringify(firstQueueAfter));
check(b.frames.some((f) => f.type === "prompt-delivered" && f.id === "smoke-1"),
  "observer socket sees the delivered event too (ignores id, reads depth)");

// 7. Pokes drain 3 -> 0 as untagged tails: queue events only.
await waitFor(a, (f) => f.type === "queue" && f.depth === 0, "queue drains to 0", 45000);
const deliveredCount = a.frames.filter((f) => f.type === "prompt-delivered").length;
check(deliveredCount === 1, "untagged pokes emit no prompt-delivered", `${deliveredCount}`);
await sleep(1000);
check(a.output.includes("SMOKE-QUEUE-1") && a.output.includes("poke-3"),
  "queued commands actually executed in order on the PTY");

// 8. Idle fast path: ack + delivered within ~2s on a quiet shell.
const idleT = Date.now();
send(a, { type: "prompt", data: "echo SMOKE-IDLE", agent: "gemini", id: "smoke-idle" });
await waitFor(a, (f) => f.type === "prompt-ack" && f.id === "smoke-idle", "idle ack");
await waitFor(a, (f) => f.type === "prompt-delivered" && f.id === "smoke-idle", "idle delivered", 10000);
log(`idle fast path: delivered in ${Date.now() - idleT}ms`);
await waitFor(a, (f) => f.type === "queue" && f.depth === 0, "idle drains to 0", 15000);

// 9. Cap straddle: busy loop again, fill to raw 99 (49 gemini pairs + 1 CLI
//    write), then a 2-write pair MUST reject all-or-nothing while a final
//    1-write CLI poke still fits; the next CLI write reports the cap.
send(a, { type: "input", data: "while true; do date; sleep 0.3; done\r" });
await sleep(1500);
for (let i = 0; i < 49; i++)
  send(a, { type: "prompt", data: `echo fill-${i}`, agent: "gemini", id: `fill-${i}` });
await waitFor(a, (f) => f.type === "prompt-ack" && f.id === "fill-48", "49 pairs acked", 20000);
const w99 = csWrite("echo filler-99\n");
check(w99 === "queued at position 99", "raw 99 after 49 pairs + 1 poke", w99);
send(a, { type: "prompt", data: "echo straddle", agent: "gemini", id: "straddle" });
const ackS = await waitFor(a, (f) => f.type === "prompt-ack" && f.id === "straddle", "straddle ack");
check(ackS.queued === false, "2-write message rejected all-or-nothing at 99/100",
  JSON.stringify(ackS));
check(ackS.depth === 50, "rejected ack carries the unchanged message depth 50",
  JSON.stringify(ackS));
const w100 = csWrite("echo filler-100\n");
check(w100 === "queued at position 100", "1-write poke still fits the last slot", w100);
const capMsg = csWrite("echo overflow\n");
check(capMsg.includes("queue cap") && capMsg.includes("nothing queued"),
  "CLI at-cap stdout regression (full branch)", capMsg);

// 10. Close the session: the queue dies with it (no 100-entry drain wait).
send(a, { type: "close" });
await waitFor(a, (f) => f.type === "closed", "closed frame", 10000);

log(failures === 0 ? "ALL CHECKS PASSED" : `${failures} FAILURES`);
process.exit(failures === 0 ? 0 : 1);
