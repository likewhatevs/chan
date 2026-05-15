# chan-desktop release review

Pre-release code review covering correctness, resiliency, subprocess
management, efficiency, and idiomatic Rust. Findings are bucketed by
severity (P0 = blocker, P5 = polish) with file:line references and
concrete remediations.

## How to use this doc

- Work top-down; P0 items are release-blocking, P1 items are
  user-visible resiliency gaps.
- Each item has a file:line anchor and a concrete fix (often with a
  code snippet) so it can be picked up cold.
- The "ship next" list at the bottom is a suggested ordering with
  rough effort estimates.

## Repo at a glance

- `src-tauri/src/main.rs`: IPC commands, AppState, window menu,
  BinStatus preflight.
- `src-tauri/src/serve.rs`: per-drive `chan serve` supervisor,
  stderr reader thread.
- `src-tauri/src/tunnel/{mod,public,validator}.rs`: embedded
  `chan-tunnel-server` + per-tenant axum listeners.
- `src-tauri/src/{config,registry,watcher,auth}.rs`: sidecar JSON,
  chan TOML mirror, notify watcher, id.chan.app OAuth.
- `src/main.js`: vanilla-JS Drives window driving the IPC surface.

Three concurrency models are mixed: `std::sync::Mutex` +
`std::thread` for serve supervision, `tokio` async for the tunnel
server, single-threaded JS event loop on the renderer. The mixing
itself is reasonable (the supervisor predates the tunnel work), but
it is the source of several subtle issues below.

---

## P0: fix before public DMG

### P0.1 Updater pubkey is a DEV key, no password

Already noted in `CLAUDE.md`. `tauri.conf.json:41` ships an
unpassworded dev keypair. Anyone with read access to the build host
can sign a "valid" update for every install. Run the bridge-release
rotation from `CLAUDE.md` before announcing the build.

### P0.2 CI is disabled

`.github/workflows/ci.yml:7` triggers only on `workflow_dispatch`.
Nothing gates fmt/clippy/test on this release. Re-enable
`push`/`pull_request` once the `PRIVATE_REPO_TOKEN` secret is set.

### P0.3 `tracing` is wired but no subscriber is installed

`Cargo.toml:50` pulls `tracing`. `tunnel/mod.rs:193,310,328` and
`tunnel/public.rs:181` log via `tracing::info!/warn!`. `main()`
never installs a subscriber, so every log line drops. Zero
observability for tunnel failures.

Fix in `main()` before `tauri::Builder::default()`:

```rust
tracing_subscriber::fmt()
    .with_env_filter(
        tracing_subscriber::EnvFilter::try_from_env("CHAN_LOG")
            .unwrap_or_else(|_| {
                tracing_subscriber::EnvFilter::new("warn,chan_desktop=info")
            }),
    )
    .init();
```

Add `tracing-subscriber = { version = "0.3", features = ["env-filter"] }`.

### P0.4 Mid-flight `chan serve` crash is silent

User-visible failure mode they explicitly called out (OOM, disk
full, filesystem read-only). `serve.rs:193` only emits
`SERVE_FAILED` when `saw_url == false`. After the URL banner has
appeared, a crash silently flips the toggle off via
`SERVES_CHANGED` and the user gets no explanation.

Scenarios this hits:

- `chan serve` writes a file, filesystem full, chan panics, process
  exits, toggle flips off with zero feedback.
- macOS memory pressure killer SIGKILLs `chan serve`, same silent
  flip.
- User unmounts the drive's volume, chan exits, same.

Fix in `serve.rs` after the EOF block:

```rust
let (exit_code, exit_signal) = exit_info(&exit_status);
let normal_termination = matches!(exit_code, Some(0))
    || matches!(exit_signal, Some(libc::SIGTERM | libc::SIGINT));

if !saw_url {
    // Hard failure on startup: modal (current behavior).
    let _ = app2.emit(SERVE_FAILED, payload);
} else if !normal_termination {
    // Mid-flight crash: soft inline notice, not modal.
    let _ = app2.emit(SERVE_CRASHED, payload);
}
```

Add a frontend listener on `SERVE_CRASHED` that calls
`showError(...)` with drive name and last 5 stderr lines. Inline
banner, not modal, since the drive was working previously.

### P0.5 Stop-then-start race: fast toggling leaves the drive off

`serve.rs:233` `stop` only calls `child.kill()` and leaves the
entry in the `serves` map. The reader thread is the sole place
that removes the entry, after EOF on stderr. If the user toggles
Off then On within ~50ms:

1. `set_drive_on(on: false)` -> `serve::stop` sends SIGKILL,
   returns.
2. `set_drive_on(on: true)` -> `serve::start` checks
   `if state.serves.lock().unwrap().contains_key(&key) { return Ok(()); }`,
   returns Ok silently.
3. Reader thread eventually wakes, removes entry, emits
   `SERVES_CHANGED`, frontend refreshes, toggle shows off.

Net effect: toggle flips off, user expected on, no spawn happens,
no error. Repros with a double-click.

Fix: make `stop` synchronous w.r.t. map removal:

```rust
pub fn stop(state: &AppState, key: &str) {
    let mut handle = match state.serves.lock().unwrap().remove(key) {
        Some(h) => h,
        None => return,
    };
    let _ = handle.child.kill();
    // Reader thread will observe EOF, find the entry already gone,
    // and exit cleanly.
    let _ = handle.child.wait();
}
```

### P0.6 No startup timeout on `chan serve`

If chan hangs after spawn but before printing the URL banner
(deadlock, blocking permission prompt, stuck mount), the reader
thread sits on `reader.lines()` forever. Toggle is "on" in the UI,
Launch stays disabled, no recovery.

Add a deadline. `reader.lines()` blocks, so options:

- Move per-drive supervision to `tokio::process::Command` +
  `select!` on `tokio::time::sleep` and the line reader. Cleaner,
  matches tunnel side.
- Stay sync; spawn a watchdog thread per drive that sleeps for the
  deadline, checks `saw_url`, kills the child if not.

Tokio is the right answer long-term (same model as tunnel
supervisor, no per-drive OS thread, easy deadlines). For a quick
fix, the watchdog thread is fine.

Suggested deadline: 15s.

### P0.7 `remove_drive` skips the translocation guard

`main.rs:229-250`: `remove_drive` calls `chan_bin()?` directly but
never `require_bin(&state.bin_status)?`. In a translocated bundle,
`chan_bin()` succeeds (file exists) but `bin_status.ok == false`.
The frontend disables the toggle, and the Forget button has
`disabledAttr` applied in render (main.js:298), but the IPC
command itself is unguarded.

Add `require_bin(&state.bin_status)?;` at the top of `remove_drive`
for defense in depth.

---

## P1: resiliency and UX

### P1.1 No chan-version probe on boot

`compute_bin_status` only checks file existence and translocation.
A bundled chan older than this desktop expects fails in weird ways
at runtime.

Add: spawn `chan --version` once at boot, parse, compare against
`MIN_CHAN_VERSION` const, add `BinStatus { kind: "version-mismatch", ... }`.
~10ms cost, runs once.

### P1.2 Watcher failure is invisible

`main.rs:692-697`: `eprintln!` on watcher::spawn error. On a
packaged macOS bundle, stderr goes nowhere visible. User sees no
symptom until they `chan add` from a terminal and the row never
appears.

Surface via a soft banner: "Auto-refresh disabled; close and
reopen the window after running chan add".

Same pattern in `serve.rs:361,365` and `tunnel/mod.rs::supervisor`
at `mod.rs:310`. All `eprintln!`, all invisible.

### P1.3 Translocation detection via substring match

`main.rs:572-576`: matching `"/AppTranslocation/"` in the
executable path is the standard heuristic and works on macOS
10.12+. Fine for release. Add a comment noting that future macOS
versions may move the prefix. More robust checks (libsecinit,
LSFileQuarantineEnabled) are rabbit-hole; substring is pragmatic.

### P1.4 Mutex held across `child.wait()` syscall

`serve.rs:180-183`:

```rust
let exit_status = {
    let mut serves = state2.serves.lock().unwrap();
    serves.remove(&key2).and_then(|mut h| h.child.wait().ok())
};
```

After EOF the child usually exited already, so wait returns fast.
If it has not (rare: child closed stderr but did not exit), every
thread that calls `list_drives` / `set_drive_on` / `add_drive` /
`remove_drive` blocks on the same Mutex until wait returns.

Fix:

```rust
let handle = state2.serves.lock().unwrap().remove(&key2);
let exit_status = handle.and_then(|mut h| h.child.wait().ok());
```

Lock released before the syscall.

### P1.5 Zombie processes from `reveal_in_finder`

`main.rs:497-510`: `Command::new(opener).arg(&path).spawn()`
returns a `Child` whose Drop does not reap. On macOS `open` exits
in tens of ms, but its zombie sits until chan-desktop exits. Over
a long session this accumulates.

Pick one:

```rust
// (a) Block-and-reap (~30ms while open execs):
Command::new(opener).arg(&path).status().map_err(...)?;

// (b) Use tauri_plugin_opener::OpenerExt::reveal_item_in_dir,
//     which handles this for you.
```

Do not install a SIGCHLD ignorer; it conflicts with the serve
supervisor's `Child::wait()`.

### P1.6 `new_state` fallback to time bytes is insecure

`auth.rs:128-143`: if `getrandom` fails, fall back to
`SystemTime::now().to_le_bytes()`. Time is predictable. Comment
says "getrandom failure is essentially impossible on the platforms
we ship to" - agreed, so the fallback is pure dead-code risk.

Fix:

```rust
fn new_state() -> Result<String, String> {
    let mut buf = [0u8; 16];
    getrandom::getrandom(&mut buf)
        .map_err(|e| format!("CSPRNG unavailable: {e}"))?;
    Ok(buf.iter().map(|b| format!("{b:02x}")).collect())
}
```

Propagate the error through `open_signin`; show a banner if it
ever fires, do not open the browser.

### P1.7 Hand-rolled percent encoding

`auth.rs:278-314`: `urlencode`/`urldecode` reinvent the wheel.
`url` crate is already in dev-deps. Add `url = "2"` as a runtime
dep, use `url::form_urlencoded::Serializer` to build the
authorize URL.

### P1.8 `hostname` shells out

`auth.rs:107`: `Command::new("hostname").output()` blocks the IPC
thread, spawns a subprocess, parses string. Use the `gethostname`
crate (~30 LoC, no deps) or `whoami`. Trivial swap.

### P1.9 App-exit grace period (SIGKILL only)

`design.md` 3.4 documents this as future work. For release this
matters more than the doc suggests: if `chan serve` has an open
write when SIGKILL lands, file integrity depends on chan's write
strategy.

Action: confirm chan's write strategy (tempfile+rename is safe
under SIGKILL; in-place truncate+write is not).

If unsafe, add SIGTERM-with-deadline now:

```rust
pub fn stop(state: &AppState, key: &str) {
    let mut handle = match state.serves.lock().unwrap().remove(key) {
        Some(h) => h,
        None => return,
    };
    #[cfg(unix)] {
        let pid = nix::unistd::Pid::from_raw(handle.child.id() as i32);
        let _ = nix::sys::signal::kill(pid, nix::sys::signal::SIGTERM);
    }
    #[cfg(windows)] {
        // No SIGTERM analog; fall through to kill below.
    }

    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while std::time::Instant::now() < deadline {
        match handle.child.try_wait() {
            Ok(Some(_)) => return,
            _ => std::thread::sleep(Duration::from_millis(50)),
        }
    }
    let _ = handle.child.kill();
    let _ = handle.child.wait();
}
```

Deps: `nix = { version = "0.29", default-features = false, features = ["signal"] }`
on Unix. Make `stop_all` share a deadline budget so app-exit does
not take 5s * N drives.

### P1.10 Window count is unbounded

`open_local_drive` (main.rs:434) and `open_tunneled_drive`
(main.rs:457) open a new webview per click. Cap at 10 per drive;
show an inline notice when at cap.

### P1.11 `chan add`/`remove` blocks the IPC thread

`add_drive` / `remove_drive` are sync `#[tauri::command]` that
shell out and `.output()`. Several seconds on a large folder. UI
shows no feedback.

Two options:

- Convert to `async fn`, use `tokio::process::Command::output().await`.
- Spawn on `tokio::task::spawn_blocking`, emit `chan-busy` event
  for a frontend spinner.

The renderer disable state also benefits (double-click currently
runs two `chan add` in parallel).

---

## P2: subprocess management lifecycle audit

Per-drive child lifecycle:

```
stage           | current behavior                      | gap
----------------|---------------------------------------|----------------------------
port alloc      | TOCTOU on 127.0.0.1:0                 | acknowledged, OK
spawn           | stderr piped, stdout null             | discards stdout
startup wait    | reader blocks on "chan is ready:"     | no timeout (P0.6)
URL discovery   | first non-empty line after banner     | brittle if chan drifts
running         | reader tails, 50-line tail            | OK
user stop       | child.kill(), no wait, no map remove  | P0.5 race, P1.9 grace
mid-flight      | reader EOF -> reap -> serves-changed  | P0.4 silent
app exit clean  | stop_all -> kill, no wait             | races readers, OK
app exit crash  | nothing                               | children orphan
reader panic    | thread dies, child orphan, map stale  | unlikely, defend
port persist    | last_port saved per drive             | silent fallback on bind fail
```

Bigger recommendations:

### P2.1 Migrate the supervisor to tokio

Same model as `tunnel/mod.rs`. One `tokio::process::Child` per
drive, one task per drive (`select!` on `child.wait()` + stderr
line reader + cancel token + startup deadline). Drops per-drive
OS thread, gives free deadlines, cleaner shutdown.

Add `process` and `io-util` to tokio features:

```toml
tokio = { version = "1", features = [
    "rt-multi-thread", "macros", "net", "sync", "time",
    "process", "io-util",
] }
```

Single biggest refactor in this review. Everything else is more
localized.

### P2.2 Parent-death detection

Design doc flags as future work. On Linux,
`prctl(PR_SET_PDEATHSIG, SIGTERM)` in a `pre_exec` hook would
make chan-serve die when chan-desktop crashes. macOS has no clean
equivalent (kqueue watching the parent PID is the pattern, but
that lives in chan-serve). Defer to chan-side; leave a TODO with
the path.

### P2.3 PID file per drive

Write `~/.chan/desktop/runtime/serve-<hash>.pid` on spawn. On
boot, look for stale pid files, send SIGTERM to whatever is
there, clean up. Catches the crash-then-restart case where the
previous run's children orphaned.

---

## P3: correctness and data hazards

### P3.1 Canonical key as `String`

`canonical_key` (main.rs:524) returns `path.display().to_string()`.
On Windows, `Path::display` may emit `\\?\C:\...` for some inputs
and `C:\...` for others depending on canonicalize's verbatim
flag. Two routes to the same drive produce two map entries.

Practical impact today: Linux/macOS only, deferred. Add a TODO.

Long-term: `serves: HashMap<PathBuf, ServeHandle>` keyed on
normalized `PathBuf`.

### P3.2 design.md vs code drift

- `design.md:145,535` says we pass `--no-token`. Code
  (serve.rs:118-119) passes `--no-browser` and keeps the token.
  The code is correct; update the doc.
- `design.md:286` says "Resolution order: bundled first, fall
  back to $PATH only in dev builds." Code never falls back to
  $PATH. Either delete the sentence or add a
  `#[cfg(debug_assertions)]` branch in `chan_bin()`.

### P3.3 `compute_bin_status` cached vs `chan_bin()` fresh

If the user removes/replaces the bundled chan between boot and an
IPC call, `require_bin` says "ok" (frozen) but `chan_bin` returns
a fresh result. Either re-run preflight on chan errors, or drop
the cache and always call `chan_bin()`.

### P3.4 `expires_at: String` empty-as-None

`auth.rs:60-61, 67-73`: empty-string sentinel. Use `Option<String>`:

```rust
pub struct StoredPat {
    pub id: String,
    pub secret: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}
```

### P3.5 `preferred_port: u16` sentinel

`config.rs:45`, `main.rs:283`: `0` means "OS-assigned." Use
`Option<u16>`, serialize as `null`.

### P3.6 `Drive` struct: tagged enum

`main.rs:122-138`: every tunneled-row field is `Option`, renderer
reads `kind`. Cleaner:

```rust
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum Drive {
    Local(LocalDrive),
    Tunneled(TunneledRow),
}
```

Frontend's `if (d.kind === 'tunneled')` switch stays unchanged.

### P3.7 `PrependPathLayer::new` debug_assert

`tunnel/public.rs:75-82`: panics in debug, silently misroutes in
release. Cost of getting it wrong is cross-tenant routing. Make
it total:

```rust
pub fn new(label: &str) -> Result<Self, &'static str> {
    if !chan_tunnel_proto::is_valid_username(label) {
        return Err("invalid tenant label");
    }
    Ok(Self { prefix: Arc::from(format!("/{label}")) })
}
```

---

## P4: efficiency and concurrency

### P4.1 Per-drive OS thread for stderr tailing

`serve.rs:147`. Fine for <50 drives, bad taste. Evaporates with
the tokio migration (P2.1).

### P4.2 `is_listening` can avoid the Mutex

`tunnel/mod.rs:120-122` locks `run`. The cached `bound_port`
atomic already tells you:

```rust
pub fn is_listening(&self) -> bool {
    self.bound_port.load(Ordering::Acquire) != 0
}
```

UI polling never blocks on the run Mutex.

### P4.3 Supervisor polls every 500ms

Documented. Acceptable. Promote to a notify channel from the
registry only if it shows up in a profile.

### P4.4 `registry::read` reparses on every refresh

Tiny TOML, microseconds. OK.

---

## P5: idiomatic Rust cleanups

### P5.1 `Box::leak(Box::new(d))` for the debouncer

`main.rs:692-697`. Works, heavy-handed. Cleaner via
`app.manage(Mutex::new(Some(d)))` or store on `AppState`.
Cosmetic.

### P5.2 `eprintln!` for failures everywhere

`main.rs:696`, `serve.rs:328,361,365,406`,
`tunnel/public.rs:181`. Wire `tracing` (P0.3) and use
`tracing::warn!` instead. Consistency.

### P5.3 Duplication between `stop_listening` and `shutdown`

`tunnel/mod.rs:230-266`: same body modulo the event emit.

```rust
fn stop_listening_inner(state: &Arc<TunnelState>) {
    let run = state.run.lock().unwrap().take();
    if let Some(run) = run { run.cancel.cancel(); }
    for (_, l) in state.listeners.lock().unwrap().drain() {
        l.cancel.cancel();
    }
    state.bound_port.store(0, Ordering::Release);
}

pub fn stop_listening(app: &AppHandle, state: &Arc<TunnelState>) {
    stop_listening_inner(state);
    crate::serve::close_all_tunneled_drive_windows(app);
    let _ = app.emit(
        TUNNEL_STATE_CHANGED,
        serde_json::json!({"listening": false, "port": null}),
    );
}

pub fn shutdown(state: &Arc<TunnelState>) {
    stop_listening_inner(state);
}
```

### P5.4 `lock().unwrap()` panics on poisoning

Hard panic on poisoned Mutex during exit hook is bad. Use
`.unwrap_or_else(|e| e.into_inner())` where the data is logically
valid. Optional.

### P5.5 Pipe both stdout and stderr from chan-serve

`Stdio::null()` for stdout (serve.rs:121). If chan ever logs to
stdout we miss it. Pipe both and merge in the tail.

### P5.6 Extract `chan_cmd(args)` helper

`main.rs:209,234`, `serve.rs:105`: same `Command::new(chan_bin)`
pattern three times.

```rust
fn chan_cmd(args: &[&str]) -> Result<Command, String> {
    let mut cmd = Command::new(chan_bin()?);
    cmd.args(args);
    Ok(cmd)
}
```

### P5.7 `AuthStatus` constructed inline three times

`auth.rs:146-162, 247-256, 266-270`: helper

```rust
fn auth_status_from(pat: Option<StoredPat>) -> AuthStatus { ... }
```

### P5.8 `auth::urlencode` builds via `format!` per byte

O(n) allocations. Use `url::form_urlencoded` (P1.7) or
`String::with_capacity(s.len() * 3)` with `write!`.

---

## P6: project hygiene

### P6.1 Clippy / audit / deny in CI

`make lint` is `-D warnings`. Good. When re-enabling CI:

- `cargo clippy --all-targets -- -W clippy::pedantic -W clippy::nursery`
  once before release, cherry-pick actionable lints.
- Add `cargo audit` and `cargo deny` jobs.
- Consider `clippy.toml`:

  ```toml
  # plus in code:
  # #![deny(clippy::await_holding_lock)]
  # #![deny(clippy::dbg_macro)]
  # #![warn(clippy::unwrap_used)]  # non-test only
  ```

### P6.2 Tokio features

Current: `rt-multi-thread, macros, net, sync, time`. After P2.1
add `process, io-util`.

### P6.3 `path = ...` deps for chan-tunnel-{server,proto}

Cargo.toml:41-42. Fine for development; breaks `cargo install
chan-desktop` (advertised in design.md 7). Pin via git rev once
chan-core is public:

```toml
chan-tunnel-server = { git = "https://github.com/chan-writer/chan-core", rev = "abc123" }
chan-tunnel-proto  = { git = "https://github.com/chan-writer/chan-core", rev = "abc123" }
```

### P6.4 CI sidecar placeholder hack

`.github/workflows/ci.yml:35-38` `touch`es an empty placeholder
file. Works for check/clippy/test, fails for `cargo tauri build`.
For release, add a `release` workflow (manual dispatch) that
builds the real artifact with a real chan binary copied in.

### P6.5 design.md drift items

See P3.2. Two doc/code mismatches.

### P6.6 `dirs` is on 5.x

Latest is 6.x. No functional difference; bump or leave.

---

## Execution plan

Items grouped by dependency, not effort. Anything inside a group is
independent and can run in parallel across sessions / colleagues.
Groups run sequentially because later groups assume earlier ones
landed.

### Group A: independent prep, no code-dependencies between items

Pick these up in parallel. None touch the same files or block each
other.

- P0.2 re-enable CI on push/PR with `PRIVATE_REPO_TOKEN` secret.
  Touches `.github/workflows/ci.yml` only.
- P0.3 install `tracing-subscriber` in `main()`. Touches `main.rs`
  + `Cargo.toml`. Unblocks observability for everything else.
- P0.7 add `require_bin` to `remove_drive`. Touches `main.rs:229`.
  Five-line patch.
- P1.4 lock-across-wait fix in `serve.rs:180-183`. Self-contained.
- P1.6 `new_state` getrandom error propagation. `auth.rs:128-143`
  + `open_signin`.
- P1.8 swap `hostname` shell-out for `gethostname` crate.
  `auth.rs:107`, `Cargo.toml`.
- P1.7 swap hand-rolled urlencode/decode for `url::form_urlencoded`.
  `auth.rs:278-314`, `Cargo.toml`.
- P3.2 fix design.md drift (--no-token, $PATH fallback).
  Docs only.
- P5.2 swap `eprintln!` for `tracing::warn!` at every site listed.
  Trivial mechanical edit, depends on A's tracing-subscriber
  landing.
- P5.6 extract `chan_cmd(args)` helper. `main.rs` + `serve.rs`.

### Group B: depends on Group A landing, can parallelize within

These each touch the supervisor or the IPC surface in ways that
benefit from tracing being present and from the `chan_cmd` helper.

- P0.5 stop-then-start race fix. `serve.rs::stop`. Add an
  integration test that asserts: stop -> start within 10ms ends
  with a running serve (currently fails). Pairs with:
- P0.4 mid-flight crash banner. `serve.rs` reader thread + new
  `SERVE_CRASHED` event + frontend handler in `src/main.js`. Share
  the exit-info classification helper with P0.5's test.
- P0.6 startup timeout. Two implementations possible:
  - Quick: watchdog `std::thread` per drive sleeping for 15s
    then checking `saw_url`. Lands as a small diff in `serve.rs`.
  - Right: migrate the supervisor to tokio (P2.1). Larger diff,
    blocks Group C cleanly.
  Pick "quick" for this release if P2.1 is not ready; otherwise
  do P2.1 here and skip the watchdog.
- P1.1 chan-version probe in `compute_bin_status`. Adds a
  `BinStatus { kind: "version-mismatch" }` variant.
- P1.2 watcher / supervisor / window-build failures surfaced via
  events instead of dropped into stderr.
- P1.5 zombie reap in `reveal_in_finder` (use
  `tauri_plugin_opener::OpenerExt::reveal_item_in_dir` if
  available, else `.status()`).
- P1.10 cap per-drive window count.
- P1.11 convert `add_drive`/`remove_drive` to async (or
  spawn_blocking with a `chan-busy` event). Independent of P0.5.

### Group C: structural, do after B stabilizes

- P2.1 tokio supervisor migration. Replaces the per-drive
  `std::thread` model with one tokio task per drive. Subsumes the
  P0.6 quick fix. Single biggest refactor; one person owning it
  end-to-end is cleaner than parallelizing.
- P1.9 SIGTERM-with-grace stop. Only worth doing if chan-serve
  writes in-place (confirm with the chan team). Lands well on top
  of P2.1.
- P2.3 PID-file-per-drive on-boot reaper. Catches the
  app-crashed-children-orphaned case.

### Group D: pre-flight before the public DMG

- P0.1 updater key rotation per `CLAUDE.md`. Coordinate with
  whoever stages the public DMG; the bridge release must ship
  signed with the OLD key.
- P6.3 swap `path = ...` deps for chan-tunnel-{server,proto} to
  git-rev pins once chan-core is publishable. Without this,
  `cargo install chan-desktop` (advertised in design.md 7) is
  broken.
- P6.4 add a release workflow (manual dispatch) that builds the
  real bundle with a real chan binary instead of the empty
  placeholder.

### Anything not assigned

P3, P4, P5 (minus P5.2 and P5.6 which are in Group A) and P6 (minus
P6.3, P6.4) are polish / hardening. Pick up opportunistically. None
of them are release-blocking.

### Parallelization map

```
A (independent prep, parallel)
  |
  +-- enables -->
  |
  v
B (supervisor + IPC, parallel within group)
  |
  +-- enables -->
  |
  v
C (structural refactor, sequential)
  |
  +-- enables -->
  |
  v
D (release staging)
```

A and D can overlap (D needs coordination, not code). B should not
start its supervisor-touching items until A is in main, otherwise
merge conflicts in `serve.rs` are likely.

## Notes on the specific scenarios the request called out

- "App run from DMG cannot find/open chan binary": handled by the
  translocation preflight (main.rs:572) and `BinStatus`. Banner
  copy is good. Add the version probe (P1.1) to catch the related
  "chan exists but is incompatible" case.
- "chan serve gets OOM killed": surfaced by P0.4 (mid-flight
  crash banner). Currently silent.
- "Filesystem read-only or full": chan-serve exits non-zero,
  caught by the same P0.4 path. The editor in the webview shows
  its own save-failed errors. The desktop's job is to surface
  the process-level failure, which P0.4 fixes.
- "App hanging because more threads could have been used": the
  IPC commands themselves are sync (P1.11) and block one Tauri
  worker each. Per-drive OS threads (P4.1) are fine at scale. The
  one place a Mutex is held across a syscall is P1.4. Overall
  threading is healthy; the main efficiency win is moving the
  supervisor to tokio (P2.1).
