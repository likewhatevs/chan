//! Terminal-scoped event-file watcher for agent pokes.
//!
//! Producers own the atomic-write contract: write a temp file in the
//! watched directory, fsync as needed, then rename to the final event
//! file. This watcher reads exactly once after notify reports Create
//! or the final side of a Rename. It never writes into the watched
//! directory; dispatch is structurally a PTY write. If a future
//! feature must emit files in the watched tree, route it through
//! `self_writes.rs`-style suppression instead of adding writes here.
//!
//! # Watcher event-file naming convention
//!
//! Filenames in a watched directory MUST match the regex
//! `^(event|pre-flight)-<id>\.(md|json)$`. Recommended extension is
//! `.md` (existing event files all use `.md` despite the content
//! being JSON, for `chan view`-friendly readability); `.json` is
//! accepted for compatibility. Content is JSON conforming to
//! `AgentEvent`.
//!
//! Anything else in the watched directory (non-matching filenames,
//! hidden files, directories) is silently ignored: no read, no
//! parse, no `tracing::warn!`, no `dropped_events.fetch_add`. Parse
//! failures for files whose names DO match the pattern keep their
//! warn + counter-bump behaviour (a producer wrote bad JSON; that
//! IS a dropped event).
//!
//! The SPA-side filter (`web/src/state/watcherEvents.ts`) and the
//! server-side read endpoint
//! (`routes/terminal.rs::is_watcher_event_filename`, from
//! systacean-9) apply the same regex. The fsnotify ingestion path
//! here mirrors that filter so the rich-prompt UI doesn't surface
//! red toasts for non-event files in the watched dir.

use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::Context;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};

const SEEN_EVENT_IDS_CAP: usize = 1024;

pub(crate) type EventDispatch = dyn Fn(AgentEvent) + Send + Sync + 'static;
pub(crate) type WatcherFailure = dyn Fn(String) + Send + Sync + 'static;

const EVENT_FILE_MAX_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub(crate) struct AgentEvent {
    pub id: String,
    #[serde(rename = "type")]
    pub event_type: AgentEventType,
    pub from: String,
    pub to: String,
    pub topic: Option<String>,
    pub questions: Option<Vec<SurveyQuestion>>,
    pub standing_options: Option<Vec<SurveyOption>>,
    pub scope: Option<SurveyScope>,
    pub answers: Option<Vec<SurveyAnswer>>,
    pub scope_grant: Option<SurveyScope>,
    pub note: Option<String>,
    /// systacean-21: relative path to the task file the
    /// receiving agent should walk on the wake. When paired
    /// with `heading`, drives `dispatch_agent_event`'s rich
    /// template (cache-bust + immediate context). Missing on
    /// pre-`-21` events; the writer (architect-side tooling +
    /// any lane firing pokes) populates it going forward.
    #[serde(default)]
    pub path: Option<String>,
    /// systacean-21: heading anchor inside the task file
    /// (markdown slug, no leading `#`). Combined with `path`
    /// into `<path>#<heading>` in the rich template so the
    /// receiving agent jumps directly to the relevant section
    /// instead of walking the whole file.
    #[serde(default)]
    pub heading: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum AgentEventType {
    Survey,
    SurveyReply,
    Poke,
    PreFlight,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub(crate) struct SurveyQuestion {
    pub header: String,
    #[serde(rename = "text")]
    pub text: String,
    pub options: Vec<SurveyOption>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub(crate) struct SurveyOption {
    pub key: String,
    pub label: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum SurveyScope {
    OneShot,
    TopicSession,
    TopicPhase,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub(crate) struct SurveyAnswer {
    pub question_index: usize,
    pub key: String,
}

#[derive(Debug)]
pub(crate) struct EventWatcherHandle {
    _watcher: RecommendedWatcher,
}

impl EventWatcherHandle {
    pub(crate) fn start(
        dir: PathBuf,
        dispatch: Arc<EventDispatch>,
        dropped_events: Arc<AtomicU64>,
        on_failure: Option<Arc<WatcherFailure>>,
    ) -> anyhow::Result<Self> {
        let seen = Arc::new(Mutex::new(SeenEventIds::default()));
        let callback_dir = dir.clone();
        let mut watcher =
            notify::recommended_watcher(move |res: notify::Result<notify::Event>| match res {
                Ok(event) => {
                    // systacean-14: log every notify event so the
                    // ingest-wedge investigation can see what kinds
                    // fire. The previous `_ => None` branch in
                    // `event_final_path` swallowed Modify(Data) /
                    // Modify(Metadata) / Other / Any silently, which
                    // is one of the leading wedge hypotheses on macOS
                    // FSEvents bursts.
                    tracing::debug!(
                        kind = ?event.kind,
                        paths = ?event.paths,
                        dir = %callback_dir.display(),
                        "notify event"
                    );
                    match event_final_path(&event) {
                        Some(path) => ingest_once(
                            &callback_dir,
                            path,
                            &dispatch,
                            &dropped_events,
                            &seen,
                            on_failure.as_deref(),
                        ),
                        None => {
                            // systacean-14: kinds the matcher in
                            // `event_final_path` doesn't accept are
                            // mostly noise on macOS FSEvents
                            // (Modify(Metadata) from xattr / Spotlight
                            // ticks, Access from grep'ing the dir).
                            // Logging at debug keeps the wedge
                            // investigation visible under
                            // `RUST_LOG=chan_server::event_watcher=debug`
                            // without polluting `dropped_events`
                            // (which feeds the rich-prompt red-toast
                            // on the SPA side).
                            tracing::debug!(
                                kind = ?event.kind,
                                paths = ?event.paths,
                                dir = %callback_dir.display(),
                                "event watcher: unhandled notify event kind"
                            );
                        }
                    }
                }
                Err(e) => {
                    dropped_events.fetch_add(1, Ordering::Relaxed);
                    let message = e.to_string();
                    if let Some(on_failure) = on_failure.as_deref() {
                        on_failure(message.clone());
                    }
                    tracing::warn!(
                        "event watcher error for {}: {message}",
                        callback_dir.display()
                    );
                }
            })
            .with_context(|| format!("start event watcher for {}", dir.display()))?;
        watcher
            .watch(&dir, RecursiveMode::NonRecursive)
            .with_context(|| format!("watch event directory {}", dir.display()))?;
        Ok(Self { _watcher: watcher })
    }
}

fn event_final_path(event: &notify::Event) -> Option<PathBuf> {
    match event.kind {
        notify::EventKind::Create(_) => event.paths.first().cloned(),
        notify::EventKind::Modify(notify::event::ModifyKind::Name(_)) => {
            event.paths.get(1).or_else(|| event.paths.first()).cloned()
        }
        _ => None,
    }
}

fn ingest_once(
    _dir: &Path,
    path: PathBuf,
    dispatch: &Arc<EventDispatch>,
    dropped_events: &AtomicU64,
    seen: &Mutex<SeenEventIds>,
    on_failure: Option<&WatcherFailure>,
) {
    // notify (FSEvents on macOS, inotify on Linux) can deliver events
    // whose path is a directory rather than a regular file:
    //   * Create events for the watch root itself on first attach to a
    //     freshly-created dir (macOS FSEvents synthetic emit).
    //   * Rename events whose final side is a subdirectory.
    // `read_to_string` on a directory errors with EISDIR
    // ("Is a directory"), which the per-error branch below would log
    // and count as a dropped event. The /api/health
    // `terminal_event_watcher.dropped_events` counter then surfaces as
    // a red toast in the rich-prompt UI on a perfectly valid attach.
    // Skip directory paths silently: they aren't event-file payloads,
    // so dropping them isn't a dropped event. systacean-5.
    // systacean-10: the SPA filter + the systacean-9 server read
    // endpoint both apply the regex `^(event|pre-flight)-.+\.(md|json)$`.
    // Mirror it here so non-event files (and hidden files, which the
    // helper rejects via its leading-dot guard) are skipped silently:
    // no read, no parse, no warn, no `dropped_events` bump. A parse
    // failure on a matching filename still counts (a producer wrote
    // bad JSON), so only the filename filter is silenced; bad content
    // keeps the existing per-error branch below.
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        tracing::debug!(path = %path.display(), "event watcher: path has no valid filename");
        return;
    };
    if !is_watcher_event_filename(name) {
        tracing::debug!(
            name,
            path = %path.display(),
            "event watcher: filename does not match (event|pre-flight)-<id>.{{md,json}}"
        );
        return;
    }
    let meta = match std::fs::symlink_metadata(&path) {
        Ok(meta) => meta,
        Err(e) => {
            dropped_events.fetch_add(1, Ordering::Relaxed);
            let message = format!("failed to inspect event file {}: {e}", path.display());
            if let Some(on_failure) = on_failure {
                on_failure(message.clone());
            }
            tracing::warn!("{message}");
            return;
        }
    };
    let ft = meta.file_type();
    if ft.is_dir() {
        tracing::debug!(path = %path.display(), "event watcher: skipping directory path");
        return;
    }
    if !ft.is_file() || ft.is_symlink() {
        dropped_events.fetch_add(1, Ordering::Relaxed);
        let message = format!("refusing unsafe event file {}", path.display());
        if let Some(on_failure) = on_failure {
            on_failure(message.clone());
        }
        tracing::warn!("{message}");
        return;
    }
    if meta.len() > EVENT_FILE_MAX_BYTES {
        dropped_events.fetch_add(1, Ordering::Relaxed);
        let message = format!(
            "refusing oversized event file {}: {} bytes exceeds {} byte cap",
            path.display(),
            meta.len(),
            EVENT_FILE_MAX_BYTES
        );
        if let Some(on_failure) = on_failure {
            on_failure(message.clone());
        }
        tracing::warn!("{message}");
        return;
    }
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) => {
            dropped_events.fetch_add(1, Ordering::Relaxed);
            let message = format!("failed to read event file {}: {e}", path.display());
            if let Some(on_failure) = on_failure {
                on_failure(message.clone());
            }
            tracing::warn!("{message}");
            return;
        }
    };
    let event = match parse_agent_event(&text) {
        Ok(event) => event,
        Err(e) => {
            dropped_events.fetch_add(1, Ordering::Relaxed);
            tracing::warn!("failed to parse event file {}: {e}", path.display());
            return;
        }
    };
    if matches!(event.event_type, AgentEventType::Unknown) {
        tracing::warn!(
            id = %event.id,
            from = %event.from,
            to = %event.to,
            "ignoring unknown event type"
        );
        return;
    }
    if !seen
        .lock()
        .expect("event watcher seen ids poisoned")
        .insert(event.id.clone())
    {
        // systacean-14: duplicate event id (the producer rewrote the
        // same file or a different filename carrying an id the watcher
        // has already dispatched). Silent skip per the dedup contract,
        // but log at debug so the wedge investigation can tell "we
        // never saw this event" apart from "we saw it but it was a
        // dup".
        tracing::debug!(
            id = %event.id,
            from = %event.from,
            to = %event.to,
            path = %path.display(),
            "event watcher: duplicate event id, skipping"
        );
        return;
    }
    tracing::debug!(
        id = %event.id,
        from = %event.from,
        to = %event.to,
        path = %path.display(),
        "event watcher: dispatching"
    );
    dispatch(event);
}

pub(crate) fn parse_agent_event(text: &str) -> serde_json::Result<AgentEvent> {
    serde_json::from_str(text)
}

/// systacean-10: mirror the SPA / routes::terminal regex
/// `^(event|pre-flight)-.+\.(md|json)$` so the fsnotify ingest path
/// silently skips files that don't match the watcher event-file
/// naming convention. Hidden files (leading dot) are rejected here
/// too, so callers don't need a separate hidden-file guard.
fn is_watcher_event_filename(name: &str) -> bool {
    if name.starts_with('.') {
        return false;
    }
    let stem = if let Some(rest) = name.strip_prefix("event-") {
        rest
    } else if let Some(rest) = name.strip_prefix("pre-flight-") {
        rest
    } else {
        return false;
    };
    let Some(dot_idx) = stem.rfind('.') else {
        return false;
    };
    if dot_idx == 0 {
        return false;
    }
    let ext = &stem[dot_idx + 1..];
    matches!(ext, "md" | "json")
}

#[derive(Default)]
struct SeenEventIds {
    set: HashSet<String>,
    order: VecDeque<String>,
}

impl SeenEventIds {
    fn insert(&mut self, id: String) -> bool {
        if self.set.contains(&id) {
            return false;
        }
        self.set.insert(id.clone());
        self.order.push_back(id);
        while self.order.len() > SEEN_EVENT_IDS_CAP {
            if let Some(old) = self.order.pop_front() {
                self.set.remove(&old);
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn parse_event_accepts_locked_survey_shape() {
        let event = parse_agent_event(
            r#"{
              "id": "survey-1",
              "type": "survey",
              "from": "@@Architect",
              "to": "@@Alex",
              "topic": "spawn",
              "questions": [
                {
                  "header": "Spawn",
                  "text": "Open a terminal?",
                  "options": [
                    {"key": "1", "label": "Open"},
                    {"key": "2", "label": "Skip"}
                  ]
                }
              ],
              "standing_options": [
                {"key": "C", "label": "Check my comments first"}
              ],
              "scope": "one-shot"
            }"#,
        )
        .expect("parse survey event");

        assert_eq!(event.event_type, AgentEventType::Survey);
        assert_eq!(event.questions.expect("questions")[0].header, "Spawn");
        assert_eq!(event.scope, Some(SurveyScope::OneShot));
    }

    #[test]
    fn parse_event_accepts_locked_reply_shape() {
        let event = parse_agent_event(
            r#"{
              "id": "survey-1",
              "type": "survey-reply",
              "from": "@@Alex",
              "to": "@@Systacean",
              "answers": [{"question_index": 0, "key": "1"}],
              "scope_grant": "topic-session",
              "note": "go"
            }"#,
        )
        .expect("parse survey reply");

        assert_eq!(event.event_type, AgentEventType::SurveyReply);
        assert_eq!(event.answers.expect("answers")[0].key, "1");
        assert_eq!(event.scope_grant, Some(SurveyScope::TopicSession));
    }

    #[test]
    fn parse_event_accepts_preflight_shape() {
        let event = parse_agent_event(
            r#"{
              "id": "pre-flight-1",
              "type": "pre-flight",
              "from": "@@Spawned",
              "to": "@@Architect",
              "note": "please log in first"
            }"#,
        )
        .expect("parse pre-flight event");

        assert_eq!(event.event_type, AgentEventType::PreFlight);
        assert_eq!(event.note.as_deref(), Some("please log in first"));
    }

    #[test]
    fn parse_event_path_and_heading_are_optional_with_backward_compat() {
        // systacean-21: AgentEvent gains `path` + `heading`
        // optional fields for the rich-poke-template cache-bust
        // mitigation in dispatch_agent_event. Both fields are
        // #[serde(default)]; pre-`-21` event files (no path /
        // heading) must parse cleanly with both as None.
        let legacy = parse_agent_event(
            r#"{"id":"1","type":"poke","from":"@@Architect","to":"@@Systacean"}"#,
        )
        .expect("legacy event without path/heading should parse");
        assert_eq!(legacy.path, None);
        assert_eq!(legacy.heading, None);

        // New-shape event carries both fields; round-trip via
        // the parser.
        let rich = parse_agent_event(
            r#"{"id":"2","type":"poke","from":"@@Architect","to":"@@Systacean","path":"docs/journals/phase-8/systacean/systacean-21.md","heading":"2026-05-22-poke"}"#,
        )
        .expect("rich event with path+heading should parse");
        assert_eq!(
            rich.path.as_deref(),
            Some("docs/journals/phase-8/systacean/systacean-21.md")
        );
        assert_eq!(rich.heading.as_deref(), Some("2026-05-22-poke"));
    }

    #[test]
    fn parse_event_requires_core_fields_but_tolerates_unknown_type() {
        assert!(parse_agent_event(r#"{"id":"1","type":"poke","from":"@@A"}"#).is_err());

        let unknown = parse_agent_event(r#"{"id":"1","type":"future","from":"@@A","to":"@@B"}"#)
            .expect("unknown types stay parseable");
        assert_eq!(unknown.event_type, AgentEventType::Unknown);
    }

    #[test]
    fn ingest_once_skips_directory_paths_silently() {
        // Regression for systacean-5: notify (FSEvents on macOS) can
        // deliver a Create event whose path is the watch root itself
        // on first attach to a freshly-created empty dir. The pre-fix
        // path then `read_to_string`'d the dir, errored with EISDIR
        // ("Is a directory"), incremented `dropped_events`, and the
        // counter surfaced as a red toast via /api/health on a
        // perfectly valid attach. After the fix, directory paths are
        // skipped silently — no event dispatched, no counter bump.
        let dir = tempfile::tempdir().expect("temp event dir");
        let subdir = dir.path().join("nested");
        std::fs::create_dir(&subdir).expect("create subdir");

        let dropped = Arc::new(AtomicU64::new(0));
        let seen = Mutex::new(SeenEventIds::default());
        let (tx, rx) = mpsc::channel();
        let dispatch: Arc<EventDispatch> = Arc::new(move |event: AgentEvent| {
            tx.send(event).expect("send event");
        });

        // Case 1: watch root itself (the FSEvents-on-fresh-dir shape).
        ingest_once(
            dir.path(),
            dir.path().to_path_buf(),
            &dispatch,
            &dropped,
            &seen,
            None,
        );
        // Case 2: a subdirectory inside the watch root.
        ingest_once(dir.path(), subdir, &dispatch, &dropped, &seen, None);

        assert_eq!(
            dropped.load(Ordering::Relaxed),
            0,
            "directory paths must not count as dropped events"
        );
        assert!(
            rx.recv_timeout(Duration::from_millis(100)).is_err(),
            "no event should dispatch for a directory path"
        );
    }

    #[test]
    fn is_watcher_event_filename_matches_spa_regex() {
        // systacean-10: stay in lockstep with the SPA's
        // `web/src/state/watcherEvents.ts::eventFilename` and the
        // server-side read endpoint's `is_watcher_event_filename`
        // (regex `^(event|pre-flight)-.+\.(md|json)$`). Three-site
        // drift would silently re-introduce the non-event red-toast
        // bug.
        assert!(is_watcher_event_filename("event-1.json"));
        assert!(is_watcher_event_filename("event-survey.md"));
        assert!(is_watcher_event_filename("pre-flight-abc.md"));
        assert!(is_watcher_event_filename("pre-flight-x.json"));
        assert!(is_watcher_event_filename("event-reply-arch-survey-1.md"));
        // Empty id between the prefix and the extension.
        assert!(!is_watcher_event_filename("event-.md"));
        assert!(!is_watcher_event_filename("pre-flight-.json"));
        // Wrong extension.
        assert!(!is_watcher_event_filename("event-1.txt"));
        // Wrong prefix.
        assert!(!is_watcher_event_filename("notes-x.md"));
        assert!(!is_watcher_event_filename("survey.json"));
        // Hidden files (leading dot, includes atomic-rename temps
        // like `.event-1.tmp`).
        assert!(!is_watcher_event_filename(".event-1.json"));
        assert!(!is_watcher_event_filename(".event-1.tmp"));
        // No extension at all.
        assert!(!is_watcher_event_filename("event-1"));
    }

    #[test]
    fn ingest_once_silently_skips_nonmatching_filename() {
        // systacean-10: a non-event file dropped into the watched
        // dir (e.g. a user's `notes.md` or a stray `README.txt`)
        // must not bump `dropped_events`, not warn, not dispatch.
        // Producers + the protocol own the (event|pre-flight)-*.{md,json}
        // namespace; everything else is silently ignored, matching
        // the SPA-side filter.
        let dir = tempfile::tempdir().expect("temp event dir");
        let stray = dir.path().join("notes.md");
        std::fs::write(
            &stray,
            r#"{"id":"x","type":"poke","from":"@@A","to":"@@B"}"#,
        )
        .expect("write stray file");

        let dropped = Arc::new(AtomicU64::new(0));
        let seen = Mutex::new(SeenEventIds::default());
        let (tx, rx) = mpsc::channel();
        let dispatch: Arc<EventDispatch> = Arc::new(move |event: AgentEvent| {
            tx.send(event).expect("send event");
        });

        ingest_once(dir.path(), stray, &dispatch, &dropped, &seen, None);

        assert_eq!(
            dropped.load(Ordering::Relaxed),
            0,
            "non-matching filenames must not count as dropped events"
        );
        assert!(
            rx.recv_timeout(Duration::from_millis(100)).is_err(),
            "non-matching filenames must not dispatch"
        );
    }

    #[test]
    fn ingest_once_warns_and_bumps_dropped_for_invalid_json_with_matching_name() {
        // systacean-10: filename-filter silencing must NOT swallow
        // legitimate dropped-event signals. A file whose NAME matches
        // the convention but whose CONTENT is bad JSON IS a dropped
        // event — the producer wrote a malformed payload. Counter
        // bumps + tracing::warn! fires per the existing branch.
        let dir = tempfile::tempdir().expect("temp event dir");
        let bad = dir.path().join("event-bad.json");
        std::fs::write(&bad, "this is not json").expect("write bad payload");

        let dropped = Arc::new(AtomicU64::new(0));
        let seen = Mutex::new(SeenEventIds::default());
        let (tx, rx) = mpsc::channel();
        let dispatch: Arc<EventDispatch> = Arc::new(move |event: AgentEvent| {
            tx.send(event).expect("send event");
        });

        ingest_once(dir.path(), bad, &dispatch, &dropped, &seen, None);

        assert_eq!(
            dropped.load(Ordering::Relaxed),
            1,
            "matching filename + bad JSON IS a dropped event"
        );
        assert!(
            rx.recv_timeout(Duration::from_millis(100)).is_err(),
            "bad-JSON files must not dispatch"
        );
    }

    #[test]
    fn watcher_dispatches_burst_of_events() {
        // systacean-14: @@WebtestB observed the watcher silently
        // wedging after the first two events on `/tmp/chan-survey-wb-r2`.
        // This test fires a burst of N atomic-rename events back-to-
        // back and asserts every one dispatches. If notify (macOS
        // FSEvents on this host) coalesces or drops past the 2nd one,
        // the test catches it.
        const N: usize = 12;
        let dir = tempfile::tempdir().expect("temp event dir");
        let (tx, rx) = mpsc::channel();
        let dropped = Arc::new(AtomicU64::new(0));
        let dispatch = Arc::new(move |event: AgentEvent| {
            tx.send(event).expect("send event");
        });
        let _watcher =
            EventWatcherHandle::start(dir.path().to_path_buf(), dispatch, dropped.clone(), None)
                .expect("start watcher");
        // Give the watcher a beat to attach before producing.
        std::thread::sleep(Duration::from_millis(150));

        for i in 0..N {
            let id = format!("burst-{i}");
            let tmp = dir.path().join(format!(".{id}.tmp"));
            let final_path = dir.path().join(format!("event-{id}.json"));
            let body = format!(r#"{{"id":"{id}","type":"poke","from":"@@A","to":"@@B"}}"#);
            std::fs::write(&tmp, body).expect("write temp");
            std::fs::rename(&tmp, &final_path).expect("rename final");
        }

        let mut received = Vec::new();
        // Generous overall budget; the watcher should be quick but
        // FSEvents has a small coalescing latency on macOS.
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while received.len() < N && std::time::Instant::now() < deadline {
            match rx.recv_timeout(Duration::from_millis(250)) {
                Ok(event) => received.push(event),
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }

        let dropped_count = dropped.load(Ordering::Relaxed);
        assert_eq!(
            received.len(),
            N,
            "expected {N} dispatches, got {} (dropped_events={dropped_count})",
            received.len()
        );
        // Every burst-N id should appear exactly once.
        let mut ids: Vec<String> = received.into_iter().map(|e| e.id).collect();
        ids.sort();
        let mut expected: Vec<String> = (0..N).map(|i| format!("burst-{i}")).collect();
        expected.sort();
        assert_eq!(ids, expected, "every event id should dispatch exactly once");
    }

    #[test]
    fn watcher_handles_repeated_same_filename_writes() {
        // systacean-14 hypothesis: @@WebtestB tried "atomic mv" multiple
        // times. If each mv targets the SAME final filename (event-1.md
        // overwritten by another event-1.md, each with a different
        // payload), notify on macOS may merge them or report
        // Modify(Data) for the inode after the first Create. Each event
        // body carries a distinct id, so dedup shouldn't skip them.
        const N: usize = 6;
        let dir = tempfile::tempdir().expect("temp event dir");
        let (tx, rx) = mpsc::channel();
        let dropped = Arc::new(AtomicU64::new(0));
        let dispatch = Arc::new(move |event: AgentEvent| {
            tx.send(event).expect("send event");
        });
        let _watcher =
            EventWatcherHandle::start(dir.path().to_path_buf(), dispatch, dropped.clone(), None)
                .expect("start watcher");
        std::thread::sleep(Duration::from_millis(150));

        let final_path = dir.path().join("event-same.json");
        for i in 0..N {
            let tmp = dir.path().join(format!(".event-same-{i}.tmp"));
            let body = format!(r#"{{"id":"same-{i}","type":"poke","from":"@@A","to":"@@B"}}"#);
            std::fs::write(&tmp, body).expect("write temp");
            std::fs::rename(&tmp, &final_path).expect("rename overwrite");
            std::thread::sleep(Duration::from_millis(40));
        }

        let mut received = Vec::new();
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while received.len() < N && std::time::Instant::now() < deadline {
            match rx.recv_timeout(Duration::from_millis(250)) {
                Ok(event) => received.push(event),
                Err(_) => continue,
            }
        }
        // Each id ("same-0" .. "same-N-1") must dispatch exactly once.
        // The destination filename is identical across writes so this
        // pins that FSEvents's combined Create + Modify(Name) sequence
        // for rename-over-existing still reaches dispatch once per
        // distinct payload.
        let mut ids: Vec<String> = received.into_iter().map(|e| e.id).collect();
        ids.sort();
        let mut expected: Vec<String> = (0..N).map(|i| format!("same-{i}")).collect();
        expected.sort();
        assert_eq!(ids, expected);
        assert_eq!(dropped.load(Ordering::Relaxed), 0);
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn watcher_handles_tmp_symlink_path() {
        // systacean-14: @@WebtestB's wedge dir was `/tmp/chan-survey-wb-r2`.
        // On macOS `/tmp` is a symlink to `/private/tmp`. Notify might
        // watch the symlink target while the producer writes to the
        // symlink path (or vice versa), and the canonical-path
        // mismatch could explain why dispatch_count doesn't grow.
        let base = std::env::temp_dir(); // already canonical under /private/tmp.
                                         // Build a symlink-style path matching the bug scenario.
        let symlink_dir = std::path::PathBuf::from("/tmp")
            .join(format!("chan-systacean-14-{:016x}", rand::random::<u64>()));
        std::fs::create_dir_all(&symlink_dir).expect("create symlink-style dir");

        let (tx, rx) = mpsc::channel();
        let dropped = Arc::new(AtomicU64::new(0));
        let dispatch = Arc::new(move |event: AgentEvent| {
            tx.send(event).expect("send event");
        });
        // Watch the symlink path (/tmp/...) just like @@WebtestB did.
        let _watcher =
            EventWatcherHandle::start(symlink_dir.clone(), dispatch, dropped.clone(), None)
                .expect("start watcher");
        std::thread::sleep(Duration::from_millis(150));

        const N: usize = 8;
        for i in 0..N {
            let id = format!("sym-{i}");
            let tmp = symlink_dir.join(format!(".{id}.tmp"));
            let final_path = symlink_dir.join(format!("event-{id}.json"));
            let body = format!(r#"{{"id":"{id}","type":"poke","from":"@@A","to":"@@B"}}"#);
            std::fs::write(&tmp, body).expect("write temp");
            std::fs::rename(&tmp, &final_path).expect("rename");
            std::thread::sleep(Duration::from_millis(30));
        }

        let mut received = 0;
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while received < N && std::time::Instant::now() < deadline {
            if rx.recv_timeout(Duration::from_millis(250)).is_ok() {
                received += 1;
            }
        }
        eprintln!(
            "[systacean-14] /tmp symlink test: received={received}/{N}, dropped={}",
            dropped.load(Ordering::Relaxed)
        );
        let _ = std::fs::remove_dir_all(&symlink_dir);
        let _ = base; // suppress unused warning
        assert_eq!(
            received, N,
            "all events should dispatch even through /tmp symlink"
        );
    }

    #[test]
    fn watcher_dispatches_atomic_rename_once() {
        let dir = tempfile::tempdir().expect("temp event dir");
        let (tx, rx) = mpsc::channel();
        let dropped = Arc::new(AtomicU64::new(0));
        let dispatch = Arc::new(move |event: AgentEvent| {
            tx.send(event).expect("send event");
        });
        let _watcher =
            EventWatcherHandle::start(dir.path().to_path_buf(), dispatch, dropped.clone(), None)
                .expect("start watcher");
        std::thread::sleep(Duration::from_millis(100));

        let tmp = dir.path().join(".event-1.tmp");
        let final_path = dir.path().join("event-1.json");
        std::fs::write(
            &tmp,
            r#"{"id":"event-1","type":"poke","from":"@@A","to":"@@B"}"#,
        )
        .expect("write temp");
        std::fs::rename(&tmp, &final_path).expect("rename final");

        let event = rx
            .recv_timeout(Duration::from_secs(3))
            .expect("receive watcher event");
        assert_eq!(event.id, "event-1");
        assert_eq!(event.event_type, AgentEventType::Poke);
        assert!(rx.recv_timeout(Duration::from_millis(300)).is_err());
        assert_eq!(dropped.load(Ordering::Relaxed), 0);
    }
}
