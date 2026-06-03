//! The survey bus: the blocked-transport side of `cs terminal survey`.
//!
//! A `cs terminal survey` call BLOCKS in the control socket until the user
//! answers in the SPA. The control handler parks a oneshot here keyed by a
//! server-minted `survey_id` and awaits it; the SPA's reply route
//! (`POST /api/survey/reply`, owned by @@LaneC) deserializes the
//! [`SurveyReply`] and calls [`SurveyBus::complete_survey`], which fires the
//! oneshot and unblocks the handler. Keeping the bus here (D's side) and the
//! reply route there (C's side) on the two ends of one stable
//! `complete_survey` API is what keeps the C<->D seam from touching one file
//! (round-3-survey-contract.md).

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use chan_shell::SurveyReply;
use tokio::sync::oneshot;

/// A `survey_id -> oneshot<SurveyReply>` registry. One entry per in-flight
/// `cs terminal survey`. Process-local: the ids only need to be unique
/// within this server's lifetime (the map is in memory), so a monotonic
/// counter suffices.
#[derive(Default)]
pub struct SurveyBus {
    pending: Mutex<HashMap<String, oneshot::Sender<SurveyReply>>>,
    counter: AtomicU64,
}

impl SurveyBus {
    pub fn new() -> Self {
        Self::default()
    }

    /// Mint a fresh `survey_id`, park a oneshot under it, and return the id
    /// plus the receiver the control handler awaits. The handler stamps the
    /// id onto the outgoing [`chan_shell::SurveySpec`] so the SPA echoes it
    /// back in its reply.
    pub fn register(&self) -> (String, oneshot::Receiver<SurveyReply>) {
        let n = self.counter.fetch_add(1, Ordering::Relaxed);
        let survey_id = format!("survey-{n}");
        let (tx, rx) = oneshot::channel();
        self.pending
            .lock()
            .expect("survey bus poisoned")
            .insert(survey_id.clone(), tx);
        (survey_id, rx)
    }

    /// Drop a parked survey without firing it. The control handler calls
    /// this on any early-exit path after `register` so an abandoned survey
    /// does not leak its sender.
    pub fn cancel(&self, survey_id: &str) {
        self.pending
            .lock()
            .expect("survey bus poisoned")
            .remove(survey_id);
    }

    /// Complete a parked survey: take its sender out of the map and fire the
    /// oneshot. Returns `false` when no survey with that id is parked (it was
    /// already answered, or the id is stale), which the reply route maps to a
    /// 404. C's `POST /api/survey/reply` is the only caller, so this is dead
    /// on D's side until that route lands (drop the allow then); the unit
    /// tests below still exercise it.
    #[allow(dead_code)]
    pub fn complete_survey(&self, survey_id: &str, reply: SurveyReply) -> bool {
        let sender = self
            .pending
            .lock()
            .expect("survey bus poisoned")
            .remove(survey_id);
        match sender {
            // `send` fails only if the receiver was dropped (the CLI
            // disconnected); the survey is gone either way, so report it
            // delivered to keep the route idempotent from C's side.
            Some(tx) => tx.send(reply).is_ok(),
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn register_then_complete_delivers_the_reply() {
        let bus = SurveyBus::new();
        let (id, rx) = bus.register();
        assert!(bus.complete_survey(
            &id,
            SurveyReply::Option {
                survey_id: id.clone(),
                option_index: 1,
                option_label: "Yes".into(),
            },
        ));
        match rx.await.expect("reply delivered") {
            SurveyReply::Option { option_label, .. } => assert_eq!(option_label, "Yes"),
            other => panic!("unexpected reply: {other:?}"),
        }
    }

    #[test]
    fn complete_unknown_survey_is_false() {
        let bus = SurveyBus::new();
        assert!(!bus.complete_survey(
            "survey-nope",
            SurveyReply::Followup {
                survey_id: "survey-nope".into(),
                followup_path: Some("team/followups/x.md".into()),
            },
        ));
    }

    #[test]
    fn each_register_mints_a_distinct_id() {
        let bus = SurveyBus::new();
        let (a, _ra) = bus.register();
        let (b, _rb) = bus.register();
        assert_ne!(a, b);
    }

    #[tokio::test]
    async fn cancel_drops_the_parked_survey() {
        let bus = SurveyBus::new();
        let (id, rx) = bus.register();
        bus.cancel(&id);
        // A cancelled survey no longer matches, and its receiver observes
        // the dropped sender.
        assert!(!bus.complete_survey(
            &id,
            SurveyReply::Option {
                survey_id: id.clone(),
                option_index: 0,
                option_label: "x".into(),
            },
        ));
        assert!(rx.await.is_err());
    }
}
