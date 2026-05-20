//! Terminal-scoped event-file watcher for agent pokes.
//!
//! Producers own the atomic-write contract: write a temp file in the
//! watched directory, fsync as needed, then rename to the final event
//! file. This watcher reads exactly once after notify reports Create
//! or the final side of a Rename. It never writes into the watched
//! directory; dispatch is structurally a PTY write. If a future
//! feature must emit files in the watched tree, route it through
//! `self_writes.rs`-style suppression instead of adding writes here.

use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::Context;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};

const SEEN_EVENT_IDS_CAP: usize = 1024;

pub(crate) type EventDispatch = dyn Fn(AgentEvent) + Send + Sync + 'static;

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
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum AgentEventType {
    Survey,
    SurveyReply,
    Poke,
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
    ) -> anyhow::Result<Self> {
        let seen = Arc::new(Mutex::new(SeenEventIds::default()));
        let callback_dir = dir.clone();
        let mut watcher =
            notify::recommended_watcher(move |res: notify::Result<notify::Event>| match res {
                Ok(event) => {
                    if let Some(path) = event_final_path(&event) {
                        ingest_once(&callback_dir, path, &dispatch, &dropped_events, &seen);
                    }
                }
                Err(e) => {
                    dropped_events.fetch_add(1, Ordering::Relaxed);
                    tracing::warn!("event watcher error for {}: {e}", callback_dir.display());
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
    if std::fs::metadata(&path).is_ok_and(|m| m.is_dir()) {
        return;
    }
    if path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with('.'))
    {
        return;
    }
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) => {
            dropped_events.fetch_add(1, Ordering::Relaxed);
            tracing::warn!("failed to read event file {}: {e}", path.display());
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
        return;
    }
    dispatch(event);
}

pub(crate) fn parse_agent_event(text: &str) -> serde_json::Result<AgentEvent> {
    serde_json::from_str(text)
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
        );
        // Case 2: a subdirectory inside the watch root.
        ingest_once(dir.path(), subdir, &dispatch, &dropped, &seen);

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
    fn watcher_dispatches_atomic_rename_once() {
        let dir = tempfile::tempdir().expect("temp event dir");
        let (tx, rx) = mpsc::channel();
        let dropped = Arc::new(AtomicU64::new(0));
        let dispatch = Arc::new(move |event: AgentEvent| {
            tx.send(event).expect("send event");
        });
        let _watcher =
            EventWatcherHandle::start(dir.path().to_path_buf(), dispatch, dropped.clone())
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
