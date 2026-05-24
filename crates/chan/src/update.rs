// Self-upgrade for chan. Three pieces:
//
//   1. Banner. On `chan serve` startup, [`maybe_print_banner`] reads
//      the cached state file and prints a one-line stderr banner if a
//      newer release is known. No network access; the probe is what
//      populates the cache.
//   2. Probe. [`run_probe`] is spawned as a tokio task at the start
//      of `chan serve`. It hits GitHub's latest-release JSON with a
//      short timeout, writes the result to the state file,
//      and prints the banner inline if the just-fetched version is
//      newer than the running binary's. Throttled to once per
//      [`PROBE_INTERVAL_HOURS`] across `chan serve` restarts.
//   3. `chan upgrade`. [`run_upgrade`] resolves the running binary
//      via [`std::env::current_exe`], downloads the release archive
//      for the current target into a sibling temp file, verifies
//      SHA-256 against the release's `SHA256SUMS`,
//      extracts the `chan` binary out of the archive into a second
//      temp file, and atomically renames it over the running
//      executable.
//
// URLs are hardcoded to GitHub Releases. Self-hosted /
// mirrored deployments are not supported. Offline / proxy hosts:
// reqwest honors HTTP_PROXY / HTTPS_PROXY / ALL_PROXY / NO_PROXY
// from the environment, and probe failures are swallowed (verbose
// only) so an air-gapped `chan serve` keeps working.

use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const LATEST_RELEASE_URL: &str = "https://api.github.com/repos/fiorix/chan/releases/latest";
const RELEASE_DOWNLOAD_BASE: &str = "https://github.com/fiorix/chan/releases/download";

/// Disable the probe (banner still prints from cached state).
const ENV_DISABLE: &str = "CHAN_UPDATE_CHECK";

/// How often to re-probe across `chan serve` restarts.
const PROBE_INTERVAL_HOURS: u64 = 24;

/// Probe timeouts. Short enough that a chan serve in a sandbox /
/// no-network environment doesn't stall the banner path.
const PROBE_CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
const PROBE_TOTAL_TIMEOUT: Duration = Duration::from_secs(5);

/// Upgrade flow timeouts. Generous: a 60 MB binary on a 1 Mbps link
/// is ~8 minutes. The total timeout is the cap on the whole download.
const UPGRADE_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const UPGRADE_TOTAL_TIMEOUT: Duration = Duration::from_secs(15 * 60);

/// Safety cap on the downloaded archive (256 MiB). Real artifacts
/// today are ~75 MB on Linux + macOS (with the embedding model
/// bundled).
const MAX_ARCHIVE_SIZE: u64 = 256 * 1024 * 1024;

/// Safety cap on JSON / SHA256SUMS responses (1 MiB).
const MAX_METADATA_SIZE: u64 = 1024 * 1024;

const STATE_FILE: &str = "update-check.json";

/// Where to keep the per-machine update state.
fn state_path() -> PathBuf {
    chan_drive::paths::config_dir().join(STATE_FILE)
}

/// State recorded after a probe.
#[derive(Debug, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct State {
    /// Unix seconds when the probe last ran (success or failure).
    pub checked_at: u64,
    /// Version of the chan binary at the time of the check.
    pub checked_version: String,
    /// Latest version the probe saw, or `None` on failure.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_version: Option<String>,
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn env_disabled() -> bool {
    matches!(env::var(ENV_DISABLE), Ok(v) if v == "0")
}

fn read_state(path: &Path) -> Option<State> {
    let contents = fs::read_to_string(path).ok()?;
    serde_json::from_str(&contents).ok()
}

fn write_state(path: &Path, state: &State) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    let body = serde_json::to_vec_pretty(state).context("serializing state")?;
    chan_drive::fs_ops::atomic_write(path, &body).context("writing state file")
}

/// Match the standalone CLI tarballs published by release.yml and install.sh.
///
/// Returns `(target_triple, archive_extension, binary_filename)`.
pub fn current_target() -> Result<(&'static str, &'static str, &'static str)> {
    release_target_for(env::consts::OS, env::consts::ARCH)
}

fn release_target_for(os: &str, arch: &str) -> Result<(&'static str, &'static str, &'static str)> {
    match (os, arch) {
        ("linux", "x86_64") => Ok(("x86_64-unknown-linux-gnu", "tar.gz", "chan")),
        ("linux", "aarch64") => Ok(("aarch64-unknown-linux-gnu", "tar.gz", "chan")),
        ("macos", "aarch64") => Ok(("aarch64-apple-darwin", "tar.gz", "chan")),
        (os, arch) => bail!(
            "no published standalone chan CLI release for {os}/{arch}. \
             Supported targets: linux x86_64/aarch64, macos aarch64."
        ),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReleaseVersion {
    /// GitHub Release tag, matching release.yml's `chan-v*` trigger.
    tag: String,
    /// Bare semver used for comparisons and user-facing messages.
    version: String,
}

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
}

fn release_from_tag(tag: &str) -> Result<ReleaseVersion> {
    let tag = tag.trim();
    if tag.is_empty() {
        bail!("release tag cannot be empty");
    }
    let version = tag
        .strip_prefix("chan-v")
        .context("GitHub release tag must use the chan-v<version> form")?
        .to_string();
    validate_version(&version)?;
    Ok(ReleaseVersion {
        tag: tag.to_string(),
        version,
    })
}

fn release_from_version_override(version: &str) -> Result<ReleaseVersion> {
    let version = version.trim();
    if version.is_empty() {
        bail!("--version cannot be empty");
    }
    // Keep the CLI surface clean: users type versions, release.yml
    // owns the `chan-v` tag convention.
    if version.starts_with("chan-v") || version.starts_with('v') {
        bail!("--version expects a bare version such as 0.14.0");
    }
    validate_version(version)?;
    Ok(ReleaseVersion {
        tag: format!("chan-v{version}"),
        version: version.to_string(),
    })
}

fn validate_version(version: &str) -> Result<()> {
    if version.is_empty() {
        bail!("release version cannot be empty");
    }
    if !version
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '+')
    {
        bail!("release version contains suspicious characters: {version:?}");
    }
    Ok(())
}

fn archive_url(tag: &str, target: &str, ext: &str) -> String {
    format!("{RELEASE_DOWNLOAD_BASE}/{tag}/chan-{target}.{ext}")
}

fn checksums_url(tag: &str) -> String {
    format!("{RELEASE_DOWNLOAD_BASE}/{tag}/SHA256SUMS")
}

fn ensure_https_url(url: &str) -> Result<()> {
    if !url.starts_with("https://") {
        bail!("refusing non-https URL: {url}");
    }
    Ok(())
}

fn parse_semver(s: &str) -> Option<(u32, u32, u32)> {
    let s = s
        .trim()
        .strip_prefix("chan-v")
        .unwrap_or(s.trim())
        .trim_start_matches('v');
    let mut parts = s.split('.');
    let major: u32 = parts.next()?.parse().ok()?;
    let minor: u32 = parts.next()?.parse().ok()?;
    let patch_raw = parts.next().unwrap_or("0");
    let patch_digits: String = patch_raw
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    let patch: u32 = patch_digits.parse().ok()?;
    Some((major, minor, patch))
}

/// `true` iff `latest` parses strictly greater than `current`.
/// Unparseable inputs are treated as "not newer".
pub fn semver_newer(latest: &str, current: &str) -> bool {
    match (parse_semver(latest), parse_semver(current)) {
        (Some(l), Some(c)) => l > c,
        _ => false,
    }
}

/// Extract the SHA-256 hex for `name` from a GNU-style SHA256SUMS
/// body. Matches both `<hash>  path/to/name` and `<hash> *name`.
pub fn parse_sha256sums(body: &str, name: &str) -> Result<String> {
    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.splitn(2, char::is_whitespace);
        let hash = match parts.next() {
            Some(h) => h,
            None => continue,
        };
        let rest = match parts.next() {
            Some(r) => r.trim_start(),
            None => continue,
        };
        let path = rest.strip_prefix('*').unwrap_or(rest);
        let file = Path::new(path).file_name().and_then(|s| s.to_str());
        let matches = file.map(|f| f == name).unwrap_or(false) || path == name;
        if matches {
            if hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
                bail!("malformed SHA256 entry for {name}: {hash}");
            }
            return Ok(hash.to_ascii_lowercase());
        }
    }
    bail!("no SHA256 entry found for {name}")
}

fn http_client(connect: Duration, total: Duration) -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .connect_timeout(connect)
        .timeout(total)
        .user_agent(format!("chan/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .context("building http client")
}

async fn fetch_text(client: &reqwest::Client, url: &str) -> Result<String> {
    ensure_https_url(url)?;
    let resp = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("GET {url}"))?
        .error_for_status()
        .with_context(|| format!("GET {url} returned an HTTP error"))?;
    let bytes = resp
        .bytes()
        .await
        .with_context(|| format!("reading body of {url}"))?;
    if (bytes.len() as u64) > MAX_METADATA_SIZE {
        bail!("metadata response from {url} exceeds {MAX_METADATA_SIZE} byte cap");
    }
    String::from_utf8(bytes.to_vec()).with_context(|| format!("decoding utf-8 body of {url}"))
}

fn parse_latest_release(body: &str) -> Result<ReleaseVersion> {
    let release: GithubRelease =
        serde_json::from_str(body).context("latest release response is not JSON")?;
    // This is intentionally strict. Phase 10 is pre-release work, so
    // the new public contract starts with `chan-v*` and no legacy
    // tag spelling needs to survive.
    release_from_tag(&release.tag_name).context("latest release JSON has invalid tag_name")
}

/// Fetch and normalize the latest version from GitHub Releases.
async fn fetch_latest_release(client: &reqwest::Client) -> Result<ReleaseVersion> {
    let body = fetch_text(client, LATEST_RELEASE_URL).await?;
    parse_latest_release(&body)
}

/// Print the banner if a newer release is cached. Stderr-only,
/// errors swallowed.
pub fn maybe_print_banner() {
    if env_disabled() {
        return;
    }
    let Some(state) = read_state(&state_path()) else {
        return;
    };
    let Some(latest) = state.latest_version.as_deref() else {
        return;
    };
    let current = env!("CARGO_PKG_VERSION");
    if !semver_newer(latest, current) {
        return;
    }
    eprintln!(
        "chan: update available: {latest} (you have {current}). \
         Run `chan upgrade` to update, or set CHAN_UPDATE_CHECK=0 to silence."
    );
}

fn probe_due(state: Option<&State>) -> bool {
    let Some(s) = state else {
        return true;
    };
    let interval = PROBE_INTERVAL_HOURS.saturating_mul(3600);
    now_unix().saturating_sub(s.checked_at) >= interval
}

/// Background probe. Designed to be `tokio::spawn`'d from
/// `chan serve` startup. Swallows errors so an offline / proxied
/// host stays silent; failures are logged at `debug` so they're
/// visible under `RUST_LOG=debug` / `chan -vv`.
pub async fn run_probe() {
    if env_disabled() {
        return;
    }
    let path = state_path();
    let prior = read_state(&path);
    if !probe_due(prior.as_ref()) {
        return;
    }
    let client = match http_client(PROBE_CONNECT_TIMEOUT, PROBE_TOTAL_TIMEOUT) {
        Ok(c) => c,
        Err(e) => {
            tracing::debug!(error = ?e, "update probe: client build failed");
            return;
        }
    };
    let checked_version = env!("CARGO_PKG_VERSION").to_string();
    let now = now_unix();
    let state = match fetch_latest_release(&client).await {
        Ok(latest) => {
            // Print the banner inline so the user sees it on the
            // session that triggered the probe, not just the next
            // restart. Only when strictly newer.
            if semver_newer(&latest.version, &checked_version) {
                eprintln!(
                    "chan: update available: {} (you have {checked_version}). \
                     Run `chan upgrade` to update, or set CHAN_UPDATE_CHECK=0 to silence.",
                    latest.version
                );
            }
            State {
                checked_at: now,
                checked_version,
                latest_version: Some(latest.version),
            }
        }
        Err(e) => {
            tracing::debug!(error = ?e, "update probe: fetch failed");
            // Persist the timestamp so we don't retry every restart
            // on an air-gapped host.
            State {
                checked_at: now,
                checked_version,
                latest_version: None,
            }
        }
    };
    if let Err(e) = write_state(&path, &state) {
        tracing::debug!(error = ?e, "update probe: state write failed");
    }
}

/// Drop-guard that unlinks a path on drop unless disarmed.
struct TempGuard(Option<PathBuf>);

impl TempGuard {
    fn new(path: PathBuf) -> Self {
        Self(Some(path))
    }
    fn disarm(&mut self) {
        self.0 = None;
    }
}

impl Drop for TempGuard {
    fn drop(&mut self) {
        if let Some(path) = self.0.take() {
            let _ = fs::remove_file(&path);
        }
    }
}

fn action_words(target: &str, current: &str) -> (&'static str, &'static str) {
    if semver_newer(current, target) {
        ("downgrading", "downgraded")
    } else {
        ("upgrading", "upgraded")
    }
}

/// Build the state to record after a successful upgrade.
///
/// The running process is still the **old** binary; the replacement
/// only takes effect on the next launch. If we stored the true
/// latest version here, a subsequent `chan serve` launched from the
/// same shell session would not print the "update available"
/// banner, but the running session would, since cached `latest >
/// current` for the still-old in-memory version. We zero out
/// `latest_version` so the next probe repopulates it.
fn post_upgrade_state(installed_version: &str) -> State {
    State {
        checked_at: now_unix(),
        checked_version: installed_version.to_string(),
        latest_version: None,
    }
}

pub struct UpgradeOptions {
    pub assume_yes: bool,
    pub check_only: bool,
    pub version_override: Option<String>,
    pub verbose: bool,
}

/// Execute `chan upgrade`. Async because we share the reqwest
/// client wiring with the serve-time probe.
pub async fn run_upgrade(opts: UpgradeOptions) -> Result<()> {
    let (target, ext, bin_name) = current_target()?;
    let current = env!("CARGO_PKG_VERSION").to_string();

    let client = http_client(UPGRADE_CONNECT_TIMEOUT, UPGRADE_TOTAL_TIMEOUT)?;

    let target_release = match opts.version_override.as_deref() {
        Some(v) => release_from_version_override(v)?,
        None => {
            if opts.verbose {
                eprintln!("chan: checking latest release at {LATEST_RELEASE_URL}");
            }
            fetch_latest_release(&client).await?
        }
    };
    let target_version = target_release.version.clone();

    if target_version == current {
        println!("chan: already at version {current}");
        return Ok(());
    }

    if opts.check_only {
        if semver_newer(&target_version, &current) {
            println!("chan: update available: {target_version} (current: {current})");
        } else {
            println!("chan: target version {target_version} is not newer than current {current}");
        }
        return Ok(());
    }

    let exe_path = env::current_exe().context("resolving current executable")?;
    let exe_path = exe_path.canonicalize().unwrap_or(exe_path);
    let binary_dir = exe_path
        .parent()
        .context("current executable has no parent directory")?
        .to_path_buf();

    let archive_name = format!("chan-{target}.{ext}");
    let archive_url = archive_url(&target_release.tag, target, ext);
    let checksums_url = checksums_url(&target_release.tag);

    let (gerund, participle) = action_words(&target_version, &current);
    println!(
        "chan: {gerund} from {current} to {target_version} ({target}) at {}",
        exe_path.display()
    );

    if !opts.assume_yes {
        if !std::io::IsTerminal::is_terminal(&std::io::stdin()) {
            bail!("use -y to confirm upgrade in non-interactive mode");
        }
        let is_downgrade = semver_newer(&current, &target_version);
        let (prompt, default_yes) = if is_downgrade {
            ("proceed? [y/N] ", false)
        } else {
            ("proceed? [Y/n] ", true)
        };
        if !confirm(prompt, default_yes)? {
            bail!("aborted");
        }
    }

    // Pre-flight writability check on the binary's directory.
    // Writing a probe file is more reliable than checking mode bits
    // because of ACLs and bind mounts.
    {
        let probe = binary_dir.join(format!(".chan.upgrade-probe.{}", std::process::id()));
        fs::File::create(&probe).with_context(|| {
            format!("binary directory is not writable: {}", binary_dir.display())
        })?;
        let _ = fs::remove_file(&probe);
    }

    if opts.verbose {
        eprintln!("chan: downloading {archive_url}");
    }

    // Stream archive into a temp file alongside the running binary
    // and SHA-256 it on the fly.
    ensure_https_url(&archive_url)?;
    let archive_path = binary_dir.join(format!(".chan.upgrade-archive.{}", std::process::id()));
    let archive_guard = TempGuard::new(archive_path.clone());

    let mut resp = client
        .get(&archive_url)
        .send()
        .await
        .with_context(|| format!("GET {archive_url}"))?
        .error_for_status()
        .with_context(|| format!("GET {archive_url} returned an HTTP error"))?;

    let mut archive_file = fs::File::create(&archive_path)
        .with_context(|| format!("creating {}", archive_path.display()))?;
    let mut hasher = Sha256::new();
    let mut total: u64 = 0;
    while let Some(chunk) = resp
        .chunk()
        .await
        .with_context(|| format!("reading chunk from {archive_url}"))?
    {
        archive_file
            .write_all(&chunk)
            .with_context(|| format!("writing to {}", archive_path.display()))?;
        hasher.update(&chunk);
        total += chunk.len() as u64;
        if total > MAX_ARCHIVE_SIZE {
            bail!("downloaded archive exceeds safety cap of {MAX_ARCHIVE_SIZE} bytes");
        }
    }
    archive_file.flush()?;
    drop(archive_file);

    let actual_hash = format!("{:x}", hasher.finalize());
    if opts.verbose {
        eprintln!("chan: downloaded {total} bytes, sha256={actual_hash}");
    }

    if opts.verbose {
        eprintln!("chan: fetching {checksums_url}");
    }
    let sums_body = fetch_text(&client, &checksums_url).await?;
    let expected_hash = parse_sha256sums(&sums_body, &archive_name)?;
    if actual_hash != expected_hash {
        bail!("SHA256 mismatch for {archive_name}: expected {expected_hash}, got {actual_hash}");
    }

    // Extract the chan binary into a sibling temp file. We never
    // unpack the rest of the archive (LICENSE, README) because the
    // upgrade only swaps the executable.
    let bin_temp = binary_dir.join(format!(".chan.upgrade-bin.{}", std::process::id()));
    let mut bin_guard = TempGuard::new(bin_temp.clone());
    extract_binary(&archive_path, &bin_temp, bin_name, ext, opts.verbose)?;

    set_executable_mode(&bin_temp)?;

    install_replacement(&bin_temp, &exe_path)?;
    bin_guard.disarm();
    drop(archive_guard);

    let _ = write_state(&state_path(), &post_upgrade_state(&target_version));

    println!("chan: {participle} to {target_version}");
    Ok(())
}

fn extract_binary(
    archive: &Path,
    out: &Path,
    bin_name: &str,
    ext: &str,
    verbose: bool,
) -> Result<()> {
    if verbose {
        eprintln!("chan: extracting {bin_name} from {}", archive.display());
    }
    match ext {
        "tar.gz" => extract_tar_gz(archive, out, bin_name),
        other => bail!("unsupported archive extension: {other}"),
    }
}

fn install_replacement(new_bin: &Path, exe_path: &Path) -> Result<()> {
    fs::rename(new_bin, exe_path).with_context(|| {
        format!(
            "replacing {} with {}",
            exe_path.display(),
            new_bin.display()
        )
    })
}

#[cfg(not(target_os = "windows"))]
fn extract_tar_gz(archive: &Path, out: &Path, bin_name: &str) -> Result<()> {
    let f = fs::File::open(archive).with_context(|| format!("opening {}", archive.display()))?;
    let gz = flate2::read::GzDecoder::new(f);
    let mut tar = tar::Archive::new(gz);
    for entry in tar
        .entries()
        .with_context(|| format!("reading entries from {}", archive.display()))?
    {
        let mut entry = entry.context("reading tar entry")?;
        let path = entry.path().context("decoding tar entry path")?;
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if name != bin_name {
            continue;
        }
        let mut out_file =
            fs::File::create(out).with_context(|| format!("creating {}", out.display()))?;
        std::io::copy(&mut entry, &mut out_file)
            .with_context(|| format!("extracting {bin_name}"))?;
        out_file.flush()?;
        return Ok(());
    }
    bail!("archive {} does not contain {bin_name}", archive.display())
}

#[cfg(target_os = "windows")]
fn extract_tar_gz(_archive: &Path, _out: &Path, _bin_name: &str) -> Result<()> {
    bail!("chan upgrade is not published for Windows")
}

#[cfg(unix)]
fn set_executable_mode(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))
        .with_context(|| format!("chmod 755 {}", path.display()))
}

#[cfg(not(unix))]
fn set_executable_mode(_path: &Path) -> Result<()> {
    Ok(())
}

fn confirm(prompt: &str, default_yes: bool) -> Result<bool> {
    use std::io::Write as _;
    print!("{prompt}");
    std::io::stdout().flush().ok();
    let mut buf = String::new();
    std::io::stdin()
        .read_line(&mut buf)
        .context("reading confirmation")?;
    let answer = buf.trim().to_ascii_lowercase();
    if answer.is_empty() {
        return Ok(default_yes);
    }
    Ok(matches!(answer.as_str(), "y" | "yes"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semver_newer() {
        assert!(semver_newer("0.7.0", "0.6.11"));
        assert!(semver_newer("1.0.0", "0.99.99"));
        assert!(semver_newer("0.6.12", "0.6.11"));
        assert!(!semver_newer("0.6.11", "0.6.11"));
        assert!(!semver_newer("0.6.10", "0.6.11"));
        assert!(!semver_newer("garbage", "0.6.11"));
        assert!(!semver_newer("0.7.0", "garbage"));
        assert!(semver_newer("v0.7.0", "0.6.11"));
        assert!(!semver_newer("0.7.0-alpha", "0.7.0"));
    }

    #[test]
    fn test_parse_sha256sums_matches_filename() {
        let body = "\
deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef  dist/chan-x86_64-unknown-linux-gnu.tar.gz
cafebabecafebabecafebabecafebabecafebabecafebabecafebabecafebabe *chan-aarch64-apple-darwin.tar.gz
feedface00feedface00feedface00feedface00feedface00feedface00feed  unrelated.deb
";
        assert_eq!(
            parse_sha256sums(body, "chan-x86_64-unknown-linux-gnu.tar.gz").unwrap(),
            "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef"
        );
        assert_eq!(
            parse_sha256sums(body, "chan-aarch64-apple-darwin.tar.gz").unwrap(),
            "cafebabecafebabecafebabecafebabecafebabecafebabecafebabecafebabe"
        );
        assert!(parse_sha256sums(body, "chan-missing.tar.gz").is_err());
    }

    #[test]
    fn test_parse_sha256sums_rejects_malformed_hash() {
        let body = "NOT_HEX  chan-x86_64-unknown-linux-gnu.tar.gz\n";
        assert!(parse_sha256sums(body, "chan-x86_64-unknown-linux-gnu.tar.gz").is_err());
    }

    #[test]
    fn test_archive_url_shape() {
        let url = archive_url("chan-v0.14.0", "x86_64-unknown-linux-gnu", "tar.gz");
        assert_eq!(
            url,
            "https://github.com/fiorix/chan/releases/download/chan-v0.14.0/chan-x86_64-unknown-linux-gnu.tar.gz"
        );
        assert_eq!(
            checksums_url("chan-v0.14.0"),
            "https://github.com/fiorix/chan/releases/download/chan-v0.14.0/SHA256SUMS"
        );
    }

    #[test]
    fn test_parse_latest_release_requires_chan_tag() {
        let release = parse_latest_release(r#"{"tag_name":"chan-v0.14.0"}"#).unwrap();
        assert_eq!(
            release,
            ReleaseVersion {
                tag: "chan-v0.14.0".into(),
                version: "0.14.0".into(),
            }
        );
        assert!(parse_latest_release(r#"{"tag_name":"v0.14.0"}"#).is_err());
        assert!(parse_latest_release(r#"{"name":"chan-v0.14.0"}"#).is_err());
    }

    #[test]
    fn test_version_override_renders_fresh_release_tag() {
        let release = release_from_version_override("0.14.0").unwrap();
        assert_eq!(
            release,
            ReleaseVersion {
                tag: "chan-v0.14.0".into(),
                version: "0.14.0".into(),
            }
        );
        assert!(release_from_version_override("chan-v0.14.0").is_err());
    }

    #[test]
    fn test_ensure_https_url_rejects_plain_http() {
        assert!(ensure_https_url(LATEST_RELEASE_URL).is_ok());
        assert!(
            ensure_https_url("http://api.github.com/repos/fiorix/chan/releases/latest").is_err()
        );
    }

    #[test]
    fn test_install_replacement_renames() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("chan");
        let new_bin = dir.path().join(".chan.upgrade-bin.test");
        fs::write(&exe, b"old").unwrap();
        fs::write(&new_bin, b"new").unwrap();

        install_replacement(&new_bin, &exe).unwrap();

        assert_eq!(fs::read(&exe).unwrap(), b"new");
        assert!(!new_bin.exists());
    }

    #[test]
    fn test_action_words() {
        assert_eq!(action_words("0.8.0", "0.7.3"), ("upgrading", "upgraded"));
        assert_eq!(
            action_words("0.7.0", "0.7.3"),
            ("downgrading", "downgraded")
        );
        assert_eq!(action_words("nonsense", "0.7.0"), ("upgrading", "upgraded"));
    }

    #[test]
    fn test_post_upgrade_state_suppresses_banner() {
        // Regression: chan upgrade on the running (old) binary must
        // leave the cached state in a shape where maybe_print_banner
        // takes the early-exit (latest_version == None) path.
        let state = post_upgrade_state("0.7.3");
        assert_eq!(state.checked_version, "0.7.3");
        assert!(state.latest_version.is_none());
        assert!(state.checked_at > 0);
    }

    #[test]
    fn test_probe_due() {
        // No state -> always due.
        assert!(probe_due(None));
        // Recent state -> not due.
        let recent = State {
            checked_at: now_unix(),
            checked_version: "0.6.0".into(),
            latest_version: Some("0.6.0".into()),
        };
        assert!(!probe_due(Some(&recent)));
        // Old state -> due.
        let old = State {
            checked_at: now_unix() - PROBE_INTERVAL_HOURS * 3600 - 60,
            checked_version: "0.6.0".into(),
            latest_version: Some("0.6.0".into()),
        };
        assert!(probe_due(Some(&old)));
    }

    #[test]
    fn test_release_target_for_active_public_artifacts() {
        assert_eq!(
            release_target_for("linux", "x86_64").unwrap(),
            ("x86_64-unknown-linux-gnu", "tar.gz", "chan")
        );
        assert_eq!(
            release_target_for("linux", "aarch64").unwrap(),
            ("aarch64-unknown-linux-gnu", "tar.gz", "chan")
        );
        assert_eq!(
            release_target_for("macos", "aarch64").unwrap(),
            ("aarch64-apple-darwin", "tar.gz", "chan")
        );
    }

    #[test]
    fn test_release_target_for_inactive_public_artifacts() {
        let windows = release_target_for("windows", "x86_64")
            .unwrap_err()
            .to_string();
        assert!(windows.contains("no published standalone chan CLI release for windows/x86_64"));
        assert!(windows.contains("linux x86_64/aarch64, macos aarch64"));
        assert!(!windows.contains("windows x86_64/aarch64"));

        let mac_intel = release_target_for("macos", "x86_64")
            .unwrap_err()
            .to_string();
        assert!(mac_intel.contains("no published standalone chan CLI release for macos/x86_64"));
    }

    #[test]
    fn test_current_target_supported_pair() {
        // Only assert the running build's target resolves; the public
        // release matrix is tested above.
        let _ = current_target().expect("target supported");
    }
}
