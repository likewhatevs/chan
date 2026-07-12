//! Cross-stack resilience: spawn the real `chan` binary, signal it, and
//! assert clean teardown -- no hung process, no orphaned PTYs, no held flock,
//! intact on-disk config. Unit tests cover the pieces in-process; this suite
//! drives the whole stack at the process boundary the way an operator and the
//! desktop client do.
//!
//! Each test runs in a sandbox: `CHAN_HOME`, `HOME`, and `XDG_RUNTIME_DIR` point
//! at fresh tempdirs, so the entire chan library (workspace registry, devserver
//! config, per-uid discovery socket) is isolated from the developer's real
//! state and from other tests. `CHAN_NO_DESKTOP_HANDOFF` +
//! `CHAN_NO_DEVSERVER_HANDOFF` keep a `chan open` standalone instead of
//! handing the workspace off to whatever is already running on the box.
//!
//! What this suite does NOT reach (hand-smoke only): the Tauri window
//! destroy/bury path and the desktop disconnect/forget UI wiring that calls
//! `WorkspaceHost::close_terminal_tenant` in-process. There is no HTTP route
//! for that synchronous reap, so the desktop's own teardown is verified by
//! `chan-server`'s unit tests plus a manual smoke. What IS covered here: the
//! spawned-process signal behavior (SIGINT/SIGTERM/SIGKILL of `chan open`
//! and `chan devserver`), advisory-flock release, persisted-config survival,
//! and the host-side workspace-tenant PTY reap reachable over the management
//! API.

#![cfg(unix)]

use std::net::{SocketAddr, TcpListener};
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tempfile::TempDir;

/// The built `chan` binary under test (Cargo points this at the target dir).
const CHAN: &str = env!("CARGO_BIN_EXE_chan");

/// `serve` cancels its in-flight reindex on the shutdown signal and the
/// shared drain force-exits after the grace window, so even a SIGINT mid-boot
/// returns well inside this bound. Generous enough to absorb a loaded CI box.
const EXIT_BUDGET: Duration = Duration::from_secs(12);

// ---------------------------------------------------------------------------
// Sandbox + process plumbing.
// ---------------------------------------------------------------------------

/// Per-test sandbox: a redirected `HOME` and `XDG_RUNTIME_DIR`, plus a scratch
/// area for workspace roots and pid files. Dropping it removes everything.
struct Sandbox {
    chan_home: TempDir,
    home: TempDir,
    runtime: TempDir,
    scratch: TempDir,
}

impl Sandbox {
    fn new() -> Self {
        Self {
            chan_home: tempfile::tempdir().expect("chan home tempdir"),
            home: tempfile::tempdir().expect("home tempdir"),
            runtime: tempfile::tempdir().expect("runtime tempdir"),
            scratch: tempfile::tempdir().expect("scratch tempdir"),
        }
    }

    /// A fresh, empty workspace root under the scratch area.
    fn workspace(&self, name: &str) -> std::path::PathBuf {
        let root = self.scratch.path().join(name);
        std::fs::create_dir_all(&root).expect("create workspace root");
        root
    }

    /// A `chan` command preloaded with the sandbox env. The inherited
    /// `CHAN_*` terminal-session vars are stripped so a test launched from
    /// inside a chan terminal doesn't accidentally drive handoff.
    fn command(&self) -> Command {
        let mut cmd = Command::new(CHAN);
        cmd.env("CHAN_HOME", self.chan_home.path())
            .env("HOME", self.home.path())
            .env("XDG_RUNTIME_DIR", self.runtime.path())
            .env("TMPDIR", self.runtime.path())
            .env("CHAN_NO_DESKTOP_HANDOFF", "1")
            .env("CHAN_NO_DEVSERVER_HANDOFF", "1")
            .env_remove("CHAN_CONTROL_SOCKET")
            .env_remove("CHAN_WINDOW_ID")
            .env_remove("CHAN_TAB_NAME")
            .env_remove("CHAN_TAB_GROUP")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        cmd
    }
}

/// Drains a child's stdout+stderr in background threads into one transcript,
/// so the pipe buffers never fill (which would wedge the child) and a test
/// can wait for a marker line or scan the whole output after the fact.
#[derive(Clone)]
struct Transcript {
    lines: Arc<Mutex<Vec<String>>>,
}

impl Transcript {
    fn capture(child: &mut Child) -> Self {
        let lines = Arc::new(Mutex::new(Vec::new()));
        if let Some(out) = child.stdout.take() {
            drain(out, lines.clone());
        }
        if let Some(err) = child.stderr.take() {
            drain(err, lines.clone());
        }
        Self { lines }
    }

    fn find(&self, needle: &str) -> Option<String> {
        self.lines
            .lock()
            .unwrap()
            .iter()
            .find(|line| line.contains(needle))
            .cloned()
    }

    /// Poll the transcript for a line containing `needle` until `timeout`.
    async fn wait_for(&self, needle: &str, timeout: Duration) -> Option<String> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(line) = self.find(needle) {
                return Some(line);
            }
            if Instant::now() >= deadline {
                return None;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }

    fn dump(&self) -> String {
        self.lines.lock().unwrap().join("\n")
    }
}

fn drain<R: std::io::Read + Send + 'static>(reader: R, lines: Arc<Mutex<Vec<String>>>) {
    use std::io::BufRead;
    std::thread::spawn(move || {
        let buf = std::io::BufReader::new(reader);
        for line in buf.lines().map_while(Result::ok) {
            lines.lock().unwrap().push(line);
        }
    });
}

/// A spawned server process plus its captured output. Dropping it always
/// kills and reaps the child, so a panicking test never strands a server.
struct Server {
    child: Child,
    out: Transcript,
}

impl Server {
    fn pid(&self) -> u32 {
        self.child.id()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Spawn `chan open <root>` standalone on an OS-assigned port and return it
/// once the ready URL is printed. The serve prints `chan is ready:\n<url>` to
/// stderr with the real bound address (it binds `:0` then reads `local_addr`).
async fn spawn_serve(sandbox: &Sandbox, root: &Path, no_token: bool) -> (Server, SocketAddr) {
    let mut cmd = sandbox.command();
    cmd.arg("open")
        .arg(root)
        // `--here` serves the path verbatim, sidestepping the enclosing-VCS
        // refusal in case the temp dir ever lands inside a working tree.
        .arg("--here")
        .arg("--standalone")
        .args(["--port", "0", "--no-browser"]);
    if no_token {
        cmd.arg("--no-token");
    }
    let mut child = cmd.spawn().expect("spawn chan open");
    let out = Transcript::capture(&mut child);
    let server = Server { child, out };
    let line = server
        .out
        .wait_for("http://127.0.0.1:", Duration::from_secs(30))
        .await
        .unwrap_or_else(|| panic!("chan open never became ready:\n{}", server.out.dump()));
    (server, parse_addr(&line))
}

/// Spawn `chan devserver` on a concrete port and return it with its bearer
/// token. The devserver prints a `listening on http://<local_addr>` line and
/// the `CHAN_DEVSERVER_TOKEN=<token>` marker to stdout.
async fn spawn_devserver(sandbox: &Sandbox, port: u16) -> (Server, SocketAddr) {
    let mut child = sandbox
        .command()
        .arg("devserver")
        .args(["--bind", "127.0.0.1"])
        .args(["--port", &port.to_string()])
        .spawn()
        .expect("spawn chan devserver");
    let out = Transcript::capture(&mut child);
    let server = Server { child, out };
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
    server
        .out
        .wait_for("listening on http://", Duration::from_secs(30))
        .await
        .unwrap_or_else(|| panic!("chan devserver never listened:\n{}", server.out.dump()));
    wait_devserver_up(&http(), addr).await;
    (server, addr)
}

fn devserver_token(server: &Server) -> String {
    let line = server
        .out
        .find("CHAN_DEVSERVER_TOKEN=")
        .unwrap_or_else(|| panic!("no devserver token marker:\n{}", server.out.dump()));
    line.rsplit("CHAN_DEVSERVER_TOKEN=")
        .next()
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap()
        .to_string()
}

/// Extract `host:port` from a line carrying an `http://host:port/...` URL.
fn parse_addr(line: &str) -> SocketAddr {
    let start = line.find("http://").expect("http:// in url line") + "http://".len();
    let rest = &line[start..];
    let end = rest.find(['/', '?']).unwrap_or(rest.len());
    rest[..end]
        .trim()
        .parse()
        .unwrap_or_else(|_| panic!("could not parse addr from: {line}"))
}

/// An unused loopback port, found by binding `:0` and releasing it. The brief
/// gap before the server rebinds is an accepted TOCTOU for a local test.
fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("bind :0")
        .local_addr()
        .unwrap()
        .port()
}

/// Send `signal` (e.g. `INT`, `TERM`) to `pid` via `kill(1)` -- no signal
/// crate dependency for a unix-only suite.
fn send_signal(pid: u32, signal: &str) {
    let status = Command::new("kill")
        .arg(format!("-{signal}"))
        .arg(pid.to_string())
        .status()
        .expect("run kill");
    assert!(status.success(), "kill -{signal} {pid} failed");
}

/// True while `pid` exists (signal 0 probes without delivering). stderr is
/// dropped so a "No such process" line doesn't clutter the test output once
/// the child is gone.
fn pid_alive(pid: u32) -> bool {
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Wait up to `timeout` for `server` to exit, returning its status and how
/// long it took. `None` means it was still running at the deadline (the
/// caller's `Drop` then force-kills it).
async fn wait_exit(server: &mut Server, timeout: Duration) -> Option<(ExitStatus, Duration)> {
    let start = Instant::now();
    loop {
        match server.child.try_wait().expect("try_wait") {
            Some(status) => return Some((status, start.elapsed())),
            None if start.elapsed() >= timeout => return None,
            None => tokio::time::sleep(Duration::from_millis(50)).await,
        }
    }
}

fn assert_output_ok(out: std::process::Output, label: &str) -> (String, String) {
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(
        out.status.success(),
        "{label} failed: status={:?}\nstdout={stdout}\nstderr={stderr}",
        out.status
    );
    (stdout, stderr)
}

fn daemon_record_path(sandbox: &Sandbox) -> std::path::PathBuf {
    sandbox
        .chan_home
        .path()
        .join("devserver")
        .join("daemon.json")
}

fn read_daemon_record(sandbox: &Sandbox) -> serde_json::Value {
    let path = daemon_record_path(sandbox);
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_json::from_str(&text).expect("daemon record json")
}

fn daemon_pid(sandbox: &Sandbox) -> u32 {
    read_daemon_record(sandbox)["pid"]
        .as_u64()
        .expect("pid field") as u32
}

// ---------------------------------------------------------------------------
// Devserver management API (a thin client over the spawned binary).
// ---------------------------------------------------------------------------

fn http() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("reqwest client")
}

/// Poll the unauthenticated info probe until the devserver answers.
async fn wait_devserver_up(client: &reqwest::Client, addr: SocketAddr) {
    let url = format!("http://{addr}/api/devserver/info");
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        if let Ok(resp) = client.get(&url).send().await {
            if resp.status().is_success() {
                return;
            }
        }
        if Instant::now() >= deadline {
            panic!("devserver /info never came up at {addr}");
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

async fn wait_devserver_down(client: &reqwest::Client, addr: SocketAddr) -> bool {
    let url = format!("http://{addr}/api/devserver/info");
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        let down = match client.get(&url).send().await {
            Ok(resp) => !resp.status().is_success(),
            Err(_) => true,
        };
        if down {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

/// `POST /api/devserver/workspaces` -- mount `root`, returning its prefix.
async fn mount_workspace(
    client: &reqwest::Client,
    addr: SocketAddr,
    token: &str,
    root: &Path,
) -> String {
    let url = format!("http://{addr}/api/devserver/workspaces");
    let body = serde_json::json!({ "path": root.to_string_lossy() }).to_string();
    let resp = client
        .post(&url)
        .bearer_auth(token)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(body)
        .send()
        .await
        .expect("POST workspaces");
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    assert!(status.is_success(), "POST workspaces -> {status}: {text}");
    let value: serde_json::Value = serde_json::from_str(&text).expect("prefix json");
    value["prefix"].as_str().expect("prefix field").to_string()
}

/// `GET /api/devserver/workspaces` as raw JSON values.
async fn list_workspaces(
    client: &reqwest::Client,
    addr: SocketAddr,
    token: &str,
) -> Vec<serde_json::Value> {
    let url = format!("http://{addr}/api/devserver/workspaces");
    let resp = client
        .get(&url)
        .bearer_auth(token)
        .send()
        .await
        .expect("GET workspaces");
    let text = resp.text().await.unwrap_or_default();
    serde_json::from_str(&text).expect("workspace list json")
}

/// `DELETE /api/devserver/workspaces<prefix>` -- the prefix (which starts with
/// `/api/`) is appended verbatim to the route base.
async fn forget_workspace(
    client: &reqwest::Client,
    addr: SocketAddr,
    token: &str,
    prefix: &str,
) -> reqwest::StatusCode {
    let url = format!("http://{addr}/api/devserver/workspaces{prefix}");
    client
        .delete(&url)
        .bearer_auth(token)
        .send()
        .await
        .expect("DELETE workspace")
        .status()
}

/// `GET /api/library/windows` as raw JSON values: the full library window set
/// every client reconciles to.
async fn list_library_windows(
    client: &reqwest::Client,
    addr: SocketAddr,
    token: &str,
) -> Vec<serde_json::Value> {
    let url = format!("http://{addr}/api/library/windows");
    let resp = client
        .get(&url)
        .bearer_auth(token)
        .send()
        .await
        .expect("GET library windows");
    let text = resp.text().await.unwrap_or_default();
    serde_json::from_str(&text).expect("library windows json")
}

/// `DELETE /api/library/windows/{window_id}` -- discard a window by its id.
async fn discard_library_window(
    client: &reqwest::Client,
    addr: SocketAddr,
    token: &str,
    window_id: &str,
) -> reqwest::StatusCode {
    let url = format!("http://{addr}/api/library/windows/{window_id}");
    client
        .delete(&url)
        .bearer_auth(token)
        .send()
        .await
        .expect("DELETE library window")
        .status()
}

/// `POST <prefix>/api/terminals` on a mounted tenant -- spawn a PTY running
/// `command` through the login shell (`$SHELL -lc`). Auth is the per-workspace
/// token from the workspace list.
async fn spawn_tenant_terminal(
    client: &reqwest::Client,
    addr: SocketAddr,
    prefix: &str,
    token: &str,
    name: &str,
    command: &str,
) -> reqwest::StatusCode {
    let url = format!("http://{addr}{prefix}/api/terminals");
    let body = serde_json::json!({ "name": name, "command": command }).to_string();
    client
        .post(&url)
        .bearer_auth(token)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(body)
        .send()
        .await
        .expect("POST tenant terminal")
        .status()
}

// ---------------------------------------------------------------------------
// Scenarios.
// ---------------------------------------------------------------------------

/// SIGINT a `chan open` whose cold index is still settling: it must exit
/// inside the grace budget, and a second serve on the same root must then
/// reacquire the writer flock (proving the first released it cleanly).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn serve_sigint_during_reindex_exits_and_frees_flock() {
    let sandbox = Sandbox::new();
    let root = sandbox.workspace("notes");
    // Enough files to give the indexer something to chew on right after the
    // ready URL prints, so the SIGINT lands during reindex rather than idle.
    for i in 0..150 {
        let body = format!("# Note {i}\n\nlorem ipsum [[link-{}]] body text\n", i % 30);
        std::fs::write(root.join(format!("note-{i:03}.md")), body).unwrap();
    }

    let (mut server, _addr) = spawn_serve(&sandbox, &root, true).await;
    // A beat so the background reindex is genuinely in flight.
    tokio::time::sleep(Duration::from_millis(400)).await;
    send_signal(server.pid(), "INT");

    let (status, elapsed) = wait_exit(&mut server, EXIT_BUDGET)
        .await
        .unwrap_or_else(|| {
            panic!(
                "serve did not exit within {EXIT_BUDGET:?}:\n{}",
                server.out.dump()
            )
        });
    assert!(
        status.success() || matches!(status.code(), Some(0)),
        "serve exited uncleanly after SIGINT: {status:?}"
    );
    assert!(
        elapsed < EXIT_BUDGET,
        "serve took {elapsed:?} to exit (budget {EXIT_BUDGET:?})"
    );

    // The flock is advisory and released on exit: a fresh serve on the same
    // root must reach ready instead of failing WorkspaceLocked.
    let (_reopened, _addr2) = spawn_serve(&sandbox, &root, true).await;
}

/// SIGINT a `chan devserver`: it must shut down cleanly (exit 0) inside the
/// grace budget.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn devserver_sigint_exits_clean() {
    let sandbox = Sandbox::new();
    let (mut server, _addr) = spawn_devserver(&sandbox, free_port()).await;

    send_signal(server.pid(), "INT");

    let (status, elapsed) = wait_exit(&mut server, EXIT_BUDGET)
        .await
        .unwrap_or_else(|| panic!("devserver did not exit on SIGINT:\n{}", server.out.dump()));
    assert!(
        status.success(),
        "devserver SIGINT exit not clean: {status:?}"
    );
    assert!(elapsed < EXIT_BUDGET, "devserver SIGINT took {elapsed:?}");
}

/// SIGTERM a `chan devserver`: it must shut down cleanly (exit 0) inside the
/// grace budget -- the same guarantee a service manager relies on. The
/// devserver routes SIGTERM through the shared graceful drain, so it does not
/// fall through to the default-terminate disposition.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn devserver_sigterm_exits_clean() {
    let sandbox = Sandbox::new();
    let (mut server, _addr) = spawn_devserver(&sandbox, free_port()).await;

    send_signal(server.pid(), "TERM");

    let (status, elapsed) = wait_exit(&mut server, EXIT_BUDGET)
        .await
        .unwrap_or_else(|| panic!("devserver did not exit on SIGTERM:\n{}", server.out.dump()));
    assert!(
        status.success(),
        "devserver SIGTERM exit not clean: {status:?}"
    );
    assert!(elapsed < EXIT_BUDGET, "devserver SIGTERM took {elapsed:?}");
}

/// The devserver's stable control-socket names in `dir`
/// (`chan-control-s<identity+prefix hash>.sock`), sorted. The short
/// single-hash name keeps the full path under the macOS 104-byte
/// `sun_path` cap; the `s` marker separates it from the pid-scoped
/// `chan-control-<pid>-<rand>.sock` names.
fn stable_control_sockets(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .flatten()
                .filter_map(|entry| entry.file_name().to_str().map(str::to_string))
                .filter(|name| name.starts_with("chan-control-s") && name.ends_with(".sock"))
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

/// A devserver restart must keep `cs` working in shells opened BEFORE the
/// restart: the control-socket path derives from the persisted library id
/// (not the pid), so the new instance rebinds the exact path already baked
/// into every pre-restart `$CHAN_CONTROL_SOCKET`.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn devserver_restart_rebinds_the_same_control_socket_paths() {
    let sandbox = Sandbox::new();
    let port = free_port();
    let (mut server, _addr) = spawn_devserver(&sandbox, port).await;
    let before = stable_control_sockets(sandbox.runtime.path());
    assert!(
        !before.is_empty(),
        "devserver mints stable control sockets:\n{}",
        server.out.dump()
    );

    send_signal(server.pid(), "TERM");
    wait_exit(&mut server, EXIT_BUDGET)
        .await
        .unwrap_or_else(|| panic!("devserver did not exit on SIGTERM:\n{}", server.out.dump()));
    drop(server);

    let (server, _addr) = spawn_devserver(&sandbox, port).await;
    let after = stable_control_sockets(sandbox.runtime.path());
    assert_eq!(
        before,
        after,
        "restart rebinds the same stable control sockets:\n{}",
        server.out.dump()
    );

    // A client holding a pre-restart $CHAN_CONTROL_SOCKET value reaches the
    // new instance.
    let pre_restart_path = sandbox.runtime.path().join(&before[0]);
    let reply =
        chan_shell::send_control_request(&pre_restart_path, chan_shell::ControlRequest::Identify)
            .await
            .expect("identify over the pre-restart control socket path");
    let identity: serde_json::Value = serde_json::from_str(&reply).expect("identity json");
    assert_eq!(identity["kind"], "devserver");
}

/// The portable `--service=chan` backend starts a detached daemon, reports
/// status from its pidfile, lets a joiner detach without stopping it, restarts
/// onto a new port, and stops idempotently.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn chan_service_start_status_join_restart_stop() {
    let sandbox = Sandbox::new();
    let client = http();
    let port = free_port();
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();

    let out = sandbox
        .command()
        .arg("devserver")
        .arg("--service=chan")
        .args(["--bind", "127.0.0.1"])
        .args(["--port", &port.to_string()])
        .output()
        .expect("start chan service");
    let (stdout, stderr) = assert_output_ok(out, "chan service start");
    assert!(
        stdout.contains("CHAN_DEVSERVER_TOKEN="),
        "start must surface the token marker: stdout={stdout:?} stderr={stderr:?}"
    );
    wait_devserver_up(&client, addr).await;
    let first_pid = daemon_pid(&sandbox);
    assert!(pid_alive(first_pid), "daemon pid {first_pid} should run");

    let out = sandbox
        .command()
        .arg("devserver")
        .arg("--service=chan")
        .arg("--status")
        .output()
        .expect("status chan service");
    let (stdout, _stderr) = assert_output_ok(out, "chan service status");
    assert!(stdout.contains("running"), "status output: {stdout}");
    assert!(
        stdout.contains(&port.to_string()),
        "status output: {stdout}"
    );

    let mut join_child = sandbox
        .command()
        .arg("devserver")
        .arg("--service=chan")
        .arg("--join")
        .args(["--bind", "127.0.0.1"])
        .args(["--port", &port.to_string()])
        .spawn()
        .expect("join chan service");
    let join_out = Transcript::capture(&mut join_child);
    let mut join = Server {
        child: join_child,
        out: join_out,
    };
    join.out
        .wait_for(
            "attached to the running self-managed daemon",
            Duration::from_secs(15),
        )
        .await
        .unwrap_or_else(|| panic!("join never attached:\n{}", join.out.dump()));
    send_signal(join.pid(), "INT");
    let (status, _elapsed) = wait_exit(&mut join, EXIT_BUDGET)
        .await
        .unwrap_or_else(|| panic!("join did not detach:\n{}", join.out.dump()));
    assert!(status.success(), "join detach exit not clean: {status:?}");
    wait_devserver_up(&client, addr).await;
    assert!(
        pid_alive(first_pid),
        "daemon pid {first_pid} should survive join detach"
    );

    let restart_port = free_port();
    let restart_addr: SocketAddr = format!("127.0.0.1:{restart_port}").parse().unwrap();
    let out = sandbox
        .command()
        .arg("devserver")
        .arg("--service=chan")
        .arg("--restart")
        .args(["--bind", "127.0.0.1"])
        .args(["--port", &restart_port.to_string()])
        .output()
        .expect("restart chan service");
    let (_stdout, _stderr) = assert_output_ok(out, "chan service restart");
    assert!(
        wait_devserver_down(&client, addr).await,
        "old daemon address {addr} still answered after restart"
    );
    wait_devserver_up(&client, restart_addr).await;
    let restarted_pid = daemon_pid(&sandbox);
    assert!(
        pid_alive(restarted_pid),
        "restarted daemon pid {restarted_pid} should run"
    );

    let out = sandbox
        .command()
        .arg("devserver")
        .arg("--service=chan")
        .arg("--stop")
        .output()
        .expect("stop chan service");
    let (_stdout, _stderr) = assert_output_ok(out, "chan service stop");
    assert!(
        wait_devserver_down(&client, restart_addr).await,
        "daemon still answered after stop"
    );

    let out = sandbox
        .command()
        .arg("devserver")
        .arg("--service=chan")
        .arg("--status")
        .output()
        .expect("status stopped chan service");
    let (stdout, _stderr) = assert_output_ok(out, "chan service stopped status");
    assert!(stdout.contains("not running"), "status output: {stdout}");
}

/// Re-running start against the same bind returns successfully and leaves the
/// original daemon in place.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn chan_service_start_is_idempotent() {
    let sandbox = Sandbox::new();
    let port = free_port();

    let out = sandbox
        .command()
        .arg("devserver")
        .arg("--service=chan")
        .args(["--bind", "127.0.0.1"])
        .args(["--port", &port.to_string()])
        .output()
        .expect("first chan service start");
    let (_stdout, _stderr) = assert_output_ok(out, "first chan service start");
    let first_pid = daemon_pid(&sandbox);

    let out = sandbox
        .command()
        .arg("devserver")
        .arg("--service=chan")
        .args(["--bind", "127.0.0.1"])
        .args(["--port", &port.to_string()])
        .output()
        .expect("second chan service start");
    let (_stdout, stderr) = assert_output_ok(out, "second chan service start");
    let second_pid = daemon_pid(&sandbox);
    assert_eq!(first_pid, second_pid, "idempotent start must keep pid");
    assert!(
        stderr.contains("already running"),
        "second start should report idempotency: {stderr}"
    );

    let out = sandbox
        .command()
        .arg("devserver")
        .arg("--service=chan")
        .arg("--stop")
        .output()
        .expect("stop chan service");
    let (_stdout, _stderr) = assert_output_ok(out, "stop chan service");
}

/// A leaked pidfile without a held daemon flock is stale and must be cleared
/// instead of reported as running or signalled.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn chan_service_status_clears_stale_pidfile() {
    let sandbox = Sandbox::new();
    let record_path = daemon_record_path(&sandbox);
    std::fs::create_dir_all(record_path.parent().unwrap()).unwrap();
    std::fs::write(
        &record_path,
        serde_json::json!({
            "pid": 999999999u32,
            "creation_time": 0u64,
            "addr": "127.0.0.1:8787",
            "started_at": "2026-01-01T00:00:00Z"
        })
        .to_string(),
    )
    .unwrap();

    let out = sandbox
        .command()
        .arg("devserver")
        .arg("--service=chan")
        .arg("--status")
        .output()
        .expect("status stale chan service");
    let (stdout, _stderr) = assert_output_ok(out, "status stale chan service");
    assert!(stdout.contains("not running"), "status output: {stdout}");
    assert!(
        !record_path.exists(),
        "status should remove stale pidfile {}",
        record_path.display()
    );
}

/// A devserver with a live PTY on a mounted workspace must leave no orphan
/// process when it shuts down: SIGINT it, and the shell the tenant spawned is
/// gone once the process exits. This is the orphan-PTY guarantee reachable
/// over the binary. (The synchronous disconnect/forget reap of a control
/// terminal is `WorkspaceHost::close_terminal_tenant`, which the desktop calls
/// in-process -- there is no management route for it -- so it is covered by
/// `chan-server`'s unit tests and a hand-smoke, not here. The workspace-forget
/// route reaps lazily via the idle pruner, so it makes no synchronous promise
/// to assert.)
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn devserver_shutdown_reaps_tenant_pty() {
    let sandbox = Sandbox::new();
    let (mut server, addr) = spawn_devserver(&sandbox, free_port()).await;
    let token = devserver_token(&server);
    let client = http();

    let root = sandbox.workspace("reap-ws");
    let prefix = mount_workspace(&client, addr, &token, &root).await;
    let ws_token = list_workspaces(&client, addr, &token)
        .await
        .into_iter()
        .find(|e| e["prefix"] == prefix)
        .and_then(|e| e["token"].as_str().map(str::to_string))
        .expect("mounted workspace token");

    // The shell records its own pid (which `exec` keeps as the sleep's pid)
    // so the test can prove that exact process dies on shutdown.
    let pid_file = sandbox.scratch.path().join("pty.pid");
    let command = format!(
        "echo $$ > '{}'; exec sleep 1000000",
        pid_file.to_string_lossy()
    );
    let status =
        spawn_tenant_terminal(&client, addr, &prefix, &ws_token, "reaptest", &command).await;
    assert!(status.is_success(), "tenant terminal create -> {status}");

    let pid = read_pid(&pid_file).await;
    assert!(
        pid_alive(pid),
        "PTY child {pid} should be running before shutdown"
    );

    send_signal(server.pid(), "INT");
    let (status, _elapsed) = wait_exit(&mut server, EXIT_BUDGET)
        .await
        .unwrap_or_else(|| panic!("devserver did not exit on SIGINT:\n{}", server.out.dump()));
    assert!(
        status.success(),
        "devserver SIGINT exit not clean: {status:?}"
    );

    assert!(
        wait_until(|| !pid_alive(pid), Duration::from_secs(10)).await,
        "PTY child {pid} survived the devserver shutdown (orphaned)"
    );
}

/// SIGKILL a `chan devserver` that has a workspace mounted: the advisory flock
/// must release (a standalone serve on the same root then starts), and a fresh
/// devserver on the same HOME must come back with the same token and re-mount
/// the workspace from its persisted config.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn devserver_sigkill_releases_flock_and_survives_config() {
    let sandbox = Sandbox::new();
    let port = free_port();
    let root = sandbox.workspace("survivor");

    let (mut first, addr) = spawn_devserver(&sandbox, port).await;
    let token1 = devserver_token(&first);
    let client = http();
    let prefix = mount_workspace(&client, addr, &token1, &root).await;

    // Hard kill: no chance to run cleanup, so this proves the kernel-released
    // advisory flock and the atomic config write carry the resilience.
    first.child.kill().expect("SIGKILL devserver");
    first.child.wait().expect("reap killed devserver");

    // Flock freed by the dead process: a standalone serve on the root starts.
    {
        let (_serve, _serve_addr) = spawn_serve(&sandbox, &root, true).await;
        // Dropped here, releasing the flock again before the devserver returns.
    }

    // Same HOME + port: the persisted token survives and the workspace
    // re-mounts on boot.
    let (second, addr2) = spawn_devserver(&sandbox, port).await;
    let token2 = devserver_token(&second);
    assert_eq!(
        token1, token2,
        "devserver token must persist across SIGKILL"
    );

    let entries = list_workspaces(&client, addr2, &token2).await;
    assert!(
        entries.iter().any(|e| e["prefix"] == prefix),
        "workspace did not re-mount from persisted config: {entries:?}"
    );
}

/// The library-owned first-open rule, at the process boundary: a FRESH
/// devserver provisions exactly one `kind=terminal` window so a plain client
/// (a browser, not just the desktop) sees a window on connect. Discarding that
/// window then restarting the devserver on the same HOME must come back with
/// ZERO windows -- the first-open marker persisted under `~/.chan/devserver/`,
/// so "closed it → reopening has no terminal" holds for the headless library
/// exactly as for the desktop.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn devserver_first_open_mints_one_terminal_then_honors_the_marker() {
    let sandbox = Sandbox::new();
    let port = free_port();
    let client = http();

    // Fresh devserver: exactly one terminal window in the library feed.
    let (mut first, addr) = spawn_devserver(&sandbox, port).await;
    let token = devserver_token(&first);
    let windows = list_library_windows(&client, addr, &token).await;
    assert_eq!(
        windows.len(),
        1,
        "fresh devserver must mint exactly one window, got: {windows:?}"
    );
    assert_eq!(
        windows[0]["kind"], "terminal",
        "the first-open window is a terminal: {windows:?}"
    );
    let window_id = windows[0]["window_id"]
        .as_str()
        .expect("window_id field")
        .to_string();

    // Discard the terminal (the user closes it for good), then shut the
    // devserver down cleanly so the marker is durably persisted.
    let status = discard_library_window(&client, addr, &token, &window_id).await;
    assert_eq!(
        status,
        reqwest::StatusCode::NO_CONTENT,
        "discard the only terminal window"
    );
    assert!(
        list_library_windows(&client, addr, &token).await.is_empty(),
        "feed empty right after the discard"
    );
    send_signal(first.pid(), "INT");
    let (exit, _) = wait_exit(&mut first, EXIT_BUDGET)
        .await
        .unwrap_or_else(|| panic!("devserver did not exit on SIGINT:\n{}", first.out.dump()));
    assert!(exit.success(), "devserver SIGINT exit not clean: {exit:?}");

    // Restart on the same HOME: the marker is set, so NO terminal re-mints.
    let (second, addr2) = spawn_devserver(&sandbox, port).await;
    let token2 = devserver_token(&second);
    let after = list_library_windows(&client, addr2, &token2).await;
    assert!(
        after.is_empty(),
        "restart after the discard must mint no terminal (marker honored): {after:?}"
    );
}

/// Close then immediately reopen the same root in a tight loop: every mount
/// must succeed, proving the close path waits for the flock to release before
/// returning so the next mount never races a lingering lock.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn close_then_reopen_under_pressure() {
    let sandbox = Sandbox::new();
    let (server, addr) = spawn_devserver(&sandbox, free_port()).await;
    let token = devserver_token(&server);
    let client = http();
    let root = sandbox.workspace("churn");

    for round in 0..10 {
        let prefix = mount_workspace(&client, addr, &token, &root).await;
        let deleted = forget_workspace(&client, addr, &token, &prefix).await;
        assert_eq!(
            deleted,
            reqwest::StatusCode::NO_CONTENT,
            "round {round}: DELETE status"
        );
    }
    // A final mount still succeeds, leaving the host in a clean state.
    let _final = mount_workspace(&client, addr, &token, &root).await;
}

/// A `chan close <path>` reaches the devserver through the per-tenant control
/// socket. The served tenant is gone immediately, and the management API must
/// report the row as off with no stale tenant token.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn chan_close_marks_devserver_workspace_off() {
    let sandbox = Sandbox::new();
    let (server, addr) = spawn_devserver(&sandbox, free_port()).await;
    let token = devserver_token(&server);
    let client = http();
    let root = sandbox.workspace("close-state");

    let prefix = mount_workspace(&client, addr, &token, &root).await;
    let before = list_workspaces(&client, addr, &token)
        .await
        .into_iter()
        .find(|row| row["prefix"] == prefix)
        .expect("mounted row");
    assert_eq!(before["on"], true);
    assert_eq!(before["status"], "running");
    assert!(
        !before["token"].as_str().unwrap_or_default().is_empty(),
        "mounted workspace carries a tenant token"
    );

    let out = sandbox
        .command()
        .arg("close")
        .arg(&root)
        .output()
        .expect("run chan close");
    assert!(
        out.status.success(),
        "chan close failed: status={:?}\nstdout={}\nstderr={}",
        out.status,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("closed"),
        "chan close did not report success: {}",
        String::from_utf8_lossy(&out.stdout),
    );

    let after = list_workspaces(&client, addr, &token)
        .await
        .into_iter()
        .find(|row| row["prefix"] == prefix)
        .expect("closed row stays registered");
    assert_eq!(after["on"], false);
    assert_eq!(after["status"], "stopped");
    assert_eq!(after["token"], "");
}

/// A `chan close --remove <path>` reaches the same devserver control socket but
/// asks the host to forget the workspace. The management API must drop the row
/// immediately rather than retaining the devserver's stale in-memory record.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn chan_close_remove_drops_devserver_workspace() {
    let sandbox = Sandbox::new();
    let (server, addr) = spawn_devserver(&sandbox, free_port()).await;
    let token = devserver_token(&server);
    let client = http();
    let root = sandbox.workspace("close-remove-state");

    let prefix = mount_workspace(&client, addr, &token, &root).await;
    assert!(
        list_workspaces(&client, addr, &token)
            .await
            .iter()
            .any(|row| row["prefix"] == prefix),
        "mounted workspace should be listed before removal"
    );

    let out = sandbox
        .command()
        .arg("close")
        .arg("--remove")
        .arg(&root)
        .output()
        .expect("run chan close --remove");
    assert!(
        out.status.success(),
        "chan close --remove failed: status={:?}\nstdout={}\nstderr={}",
        out.status,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );

    let after = list_workspaces(&client, addr, &token).await;
    assert!(
        after.iter().all(|row| row["prefix"] != prefix),
        "removed workspace must disappear from devserver list: {after:?}"
    );
}

// ---------------------------------------------------------------------------
// Small async polls.
// ---------------------------------------------------------------------------

async fn read_pid(path: &Path) -> u32 {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Ok(text) = std::fs::read_to_string(path) {
            if let Ok(pid) = text.trim().parse::<u32>() {
                return pid;
            }
        }
        if Instant::now() >= deadline {
            panic!("PTY never wrote its pid file at {}", path.display());
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

async fn wait_until(mut cond: impl FnMut() -> bool, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if cond() {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}
