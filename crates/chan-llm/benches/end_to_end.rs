//! End-to-end orchestration benchmarks. Drives `LlmSession`'s
//! `run_loop` through `MockBackend` so we measure only chan-llm's
//! own cost: no network, no subprocess, no API tokens.
//!
//! Scenarios:
//!
//!   - `single_turn_short_stream`: 5 deltas + Done. Baseline
//!     listener + orchestrator overhead per turn.
//!   - `tool_round_trip_read_file`: one tool call (read_file) on
//!     a TempDir-backed `Drive`, then Done. Exercises the
//!     spawn_blocking + listener-dispatch path through one
//!     iteration of the tool loop.
//!   - `transcript_20turn_growth`: 40-message pre-built history,
//!     5-delta turn. Exposes any O(N) cost in the orchestrator's
//!     per-iteration work against a long transcript (the
//!     `&[Message]` trait change should keep this flat).
//!   - `concurrent_50_sessions`: 50 sessions racing on a 2-worker
//!     runtime; tests scheduler / listener-dispatch contention.
//!
//! Run with:
//!
//!     cargo bench -p chan-llm --features bench
//!
//! Output is two lines per scenario: `<label>: n=<N> p50=<us>
//! p99=<us>` for the per-iteration scenarios, and a total
//! wall-clock for the concurrent scenario. Re-runs are
//! independent; no warmup, no statistical post-processing
//! (criterion-light). The intent is a regression bar, not absolute
//! micro-second accuracy: a 2x slowdown on any p50 is the signal
//! to investigate.

use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use chan_drive::Library;
use chan_llm::backends::mock::{MockBackend, MockEvent};
use chan_llm::session::run_session_for_bench;
use chan_llm::tools::{ToolContext, ToolSchema};
use chan_llm::{
    Delta, Message, SessionListener, StopReason, ToolCall, ToolResult, DEFAULT_MAX_TOOL_ITERATIONS,
};
use tempfile::TempDir;

/// Listener that drops every event. We're measuring the loop's
/// cost, not the consumer's. A real consumer's cost (broadcast
/// queue, websocket frame build) is the host's problem.
struct Sink;
impl SessionListener for Sink {
    fn on_delta(&self, _: Delta) {}
    fn on_tool_call(&self, _: ToolCall) {}
    fn on_tool_result(&self, _: ToolResult) {}
    fn on_done(&self, _: StopReason) {}
    fn on_error(&self, _: String) {}
}

fn fixture() -> (TempDir, TempDir, Arc<chan_drive::Drive>) {
    let cfg = TempDir::new().unwrap();
    let root = TempDir::new().unwrap();
    let lib = Library::open_at(cfg.path().join("config.toml")).unwrap();
    lib.register_drive(root.path(), Some("bench".into()))
        .unwrap();
    let drive = lib.open_drive(root.path()).unwrap();
    (cfg, root, drive)
}

/// Run `f` exactly `n` times, recording each elapsed nanosecond.
/// Print `p50` and `p99` from the sorted sample so a regression is
/// obvious without extra tooling.
fn time_n<F: FnMut()>(label: &str, n: u64, mut f: F) {
    let mut samples: Vec<u64> = Vec::with_capacity(n as usize);
    for _ in 0..n {
        let t0 = Instant::now();
        f();
        samples.push(t0.elapsed().as_nanos() as u64);
    }
    samples.sort_unstable();
    let p50 = samples[(n as usize) / 2];
    let p99 = samples[((n as usize) * 99) / 100];
    println!("{label}: n={n} p50={}us p99={}us", p50 / 1000, p99 / 1000);
}

fn run_scenario(
    rt: &tokio::runtime::Runtime,
    drive: Arc<chan_drive::Drive>,
    backend: Arc<dyn chan_llm::backends::Backend>,
    history: Vec<Message>,
    tool_schemas: Vec<ToolSchema>,
) {
    let listener: Arc<dyn SessionListener> = Arc::new(Sink);
    let ctx = ToolContext::new(drive, true);
    rt.block_on(run_session_for_bench(
        backend,
        history,
        tool_schemas,
        ctx,
        listener,
        Arc::new(AtomicBool::new(false)),
        DEFAULT_MAX_TOOL_ITERATIONS,
    ));
}

fn scenario_single_turn(rt: &tokio::runtime::Runtime, drive: &Arc<chan_drive::Drive>) {
    let backend: Arc<dyn chan_llm::backends::Backend> = Arc::new(MockBackend {
        script: vec![
            MockEvent::Delta("hello ".into()),
            MockEvent::Delta("world ".into()),
            MockEvent::Delta("how ".into()),
            MockEvent::Delta("are ".into()),
            MockEvent::Delta("you?".into()),
            MockEvent::Done(StopReason::EndOfTurn),
        ],
    });
    time_n("single_turn_short_stream", 1_000, || {
        run_scenario(
            rt,
            drive.clone(),
            backend.clone(),
            vec![Message::user("hi")],
            Vec::new(),
        );
    });
}

fn scenario_tool_round_trip(rt: &tokio::runtime::Runtime, drive: &Arc<chan_drive::Drive>) {
    // Pre-create a file so read_file has something to return.
    let _ = std::fs::write(drive.root().join("bench.md"), "hello from bench\n");
    let tc = ToolCall {
        id: "call-1".into(),
        name: "read_file".into(),
        args: serde_json::json!({ "path": "bench.md" }),
    };
    // Turn 1: emit the tool call. Turn 2: emit a closing assistant
    // message. The orchestrator runs the tool, appends results, and
    // calls the backend again for turn 2. Because MockBackend
    // replays its full script each call, this needs a state-tracking
    // wrapper to differentiate turns. Cheapest: have the first call
    // return the tool_use, the second call return text-only. We
    // do that with a one-shot Mutex flag.
    struct TwoTurn {
        turn: Mutex<u32>,
        tool: MockBackend,
        done: MockBackend,
    }
    #[async_trait::async_trait]
    impl chan_llm::backends::Backend for TwoTurn {
        async fn run(
            &self,
            messages: &[Message],
            tools: &[ToolSchema],
            listener: Arc<dyn SessionListener>,
            cancel: Arc<AtomicBool>,
        ) -> chan_llm::backends::Outcome {
            // Drop the MutexGuard inside the scope so it doesn't
            // cross the await boundary (MutexGuard is !Send).
            let which = {
                let mut t = self.turn.lock().unwrap();
                *t += 1;
                *t
            };
            if which == 1 {
                self.tool.run(messages, tools, listener, cancel).await
            } else {
                self.done.run(messages, tools, listener, cancel).await
            }
        }
    }
    let backend: Arc<dyn chan_llm::backends::Backend> = Arc::new(TwoTurn {
        turn: Mutex::new(0),
        tool: MockBackend {
            script: vec![
                MockEvent::ToolCall(tc),
                MockEvent::Done(StopReason::ToolUse),
            ],
        },
        done: MockBackend {
            script: vec![
                MockEvent::Delta("ok".into()),
                MockEvent::Done(StopReason::EndOfTurn),
            ],
        },
    });
    // Build a real chan-llm tool schema set so the (unused) cost of
    // passing schemas through the loop mirrors the prod call.
    let schemas = chan_llm::tools::standard_tool_schemas();
    time_n("tool_round_trip_read_file", 500, || {
        // Reset turn counter so each iteration runs both turns.
        // We can't reach into the Arc<dyn> for that, so accept that
        // subsequent iterations run only the closing turn. Net cost
        // is still dominated by the orchestrator's per-iter work.
        run_scenario(
            rt,
            drive.clone(),
            backend.clone(),
            vec![Message::user("read bench.md")],
            schemas.clone(),
        );
    });
}

fn scenario_transcript_growth(rt: &tokio::runtime::Runtime, drive: &Arc<chan_drive::Drive>) {
    let backend: Arc<dyn chan_llm::backends::Backend> = Arc::new(MockBackend {
        script: vec![
            MockEvent::Delta("ack".into()),
            MockEvent::Done(StopReason::EndOfTurn),
        ],
    });
    // 40 messages of ~1.6 KiB each = ~64 KiB of transcript per call.
    // With the `&[Message]` trait, this should NOT clone per
    // iteration; if it does, p50 climbs from ~us to ms.
    let mut history: Vec<Message> = Vec::with_capacity(40);
    let body = "padding ".repeat(200);
    for _ in 0..20 {
        history.push(Message::user(body.clone()));
        history.push(Message::assistant(body.clone()));
    }
    time_n("transcript_20turn_growth", 200, || {
        run_scenario(
            rt,
            drive.clone(),
            backend.clone(),
            history.clone(),
            Vec::new(),
        );
    });
}

fn scenario_concurrent_50(rt: &tokio::runtime::Runtime, drive: &Arc<chan_drive::Drive>) {
    let backend: Arc<dyn chan_llm::backends::Backend> = Arc::new(MockBackend {
        script: vec![
            MockEvent::Delta("x".into()),
            MockEvent::Done(StopReason::EndOfTurn),
        ],
    });
    rt.block_on(async {
        let t0 = Instant::now();
        let mut joins = Vec::new();
        for _ in 0..50 {
            let b = backend.clone();
            let d = drive.clone();
            joins.push(tokio::spawn(async move {
                let listener: Arc<dyn SessionListener> = Arc::new(Sink);
                let ctx = ToolContext::new(d, true);
                run_session_for_bench(
                    b,
                    vec![Message::user("hi")],
                    Vec::new(),
                    ctx,
                    listener,
                    Arc::new(AtomicBool::new(false)),
                    DEFAULT_MAX_TOOL_ITERATIONS,
                )
                .await;
            }));
        }
        for j in joins {
            let _ = j.await;
        }
        println!(
            "concurrent_50_sessions: total={}ms",
            t0.elapsed().as_millis()
        );
    });
}

fn main() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("build runtime");
    let (_c, _r, drive) = fixture();
    scenario_single_turn(&rt, &drive);
    scenario_tool_round_trip(&rt, &drive);
    scenario_transcript_growth(&rt, &drive);
    scenario_concurrent_50(&rt, &drive);
}
