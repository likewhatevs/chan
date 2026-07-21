use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use std::sync::{Arc, Mutex};

use uuid::Uuid;

const MAX_ENTRIES_PER_SUBJECT: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsumeError {
    Replay,
    AtCapacity,
}

#[derive(Clone)]
pub struct EntryReplayCache {
    inner: Arc<Mutex<ReplayState>>,
    max_entries: usize,
    max_entries_per_subject: usize,
}

#[derive(Default)]
struct ReplayState {
    entries: HashMap<Uuid, ReplayEntry>,
    expiries: BinaryHeap<Reverse<(i64, Uuid)>>,
    subject_counts: HashMap<Uuid, usize>,
}

struct ReplayEntry {
    expires_at: i64,
    subject_user_id: Uuid,
}

impl std::fmt::Debug for EntryReplayCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EntryReplayCache")
            .field("max_entries", &self.max_entries)
            .field("max_entries_per_subject", &self.max_entries_per_subject)
            .finish_non_exhaustive()
    }
}

impl EntryReplayCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(ReplayState::default())),
            max_entries,
            max_entries_per_subject: max_entries.min(MAX_ENTRIES_PER_SUBJECT),
        }
    }

    #[cfg(test)]
    fn with_subject_limit(max_entries: usize, max_entries_per_subject: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(ReplayState::default())),
            max_entries,
            max_entries_per_subject,
        }
    }

    /// Atomically consume a signed jti until its credential expiry. Capacity
    /// pressure never evicts an unexpired jti: doing so would make an earlier
    /// bearer replayable. Expired entries are pruned before fail-closed.
    pub fn consume(
        &self,
        jti: Uuid,
        subject_user_id: Uuid,
        expires_at: i64,
        now: i64,
    ) -> Result<(), ConsumeError> {
        let mut state = self.inner.lock().unwrap_or_else(|error| error.into_inner());
        prune_expired(&mut state, now);
        if state.entries.contains_key(&jti) {
            return Err(ConsumeError::Replay);
        }
        if state.entries.len() >= self.max_entries {
            return Err(ConsumeError::AtCapacity);
        }
        if state
            .subject_counts
            .get(&subject_user_id)
            .copied()
            .unwrap_or_default()
            >= self.max_entries_per_subject
        {
            return Err(ConsumeError::AtCapacity);
        }
        state.entries.insert(
            jti,
            ReplayEntry {
                expires_at,
                subject_user_id,
            },
        );
        *state.subject_counts.entry(subject_user_id).or_default() += 1;
        state.expiries.push(Reverse((expires_at, jti)));
        Ok(())
    }
}

fn prune_expired(state: &mut ReplayState, now: i64) {
    while let Some(Reverse((expiry, jti))) = state.expiries.peek().copied() {
        if expiry >= now {
            break;
        }
        state.expiries.pop();
        if state
            .entries
            .get(&jti)
            .is_some_and(|entry| entry.expires_at == expiry)
        {
            let entry = state.entries.remove(&jti).expect("entry checked above");
            decrement_count(&mut state.subject_counts, entry.subject_user_id);
        }
    }
}

fn decrement_count(counts: &mut HashMap<Uuid, usize>, key: Uuid) {
    if let Some(count) = counts.get_mut(&key) {
        *count -= 1;
        if *count == 0 {
            counts.remove(&key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replay_is_rejected_until_expiry_then_pruned() {
        let cache = EntryReplayCache::new(1);
        let jti = Uuid::new_v4();
        let subject = Uuid::new_v4();
        assert_eq!(cache.consume(jti, subject, 20, 10), Ok(()));
        assert_eq!(
            cache.consume(jti, subject, 20, 11),
            Err(ConsumeError::Replay)
        );
        assert_eq!(cache.consume(Uuid::new_v4(), subject, 30, 21), Ok(()));
    }

    #[test]
    fn verifier_skew_window_stays_replay_protected_at_every_boundary() {
        let raw_exp = 20;
        let effective_exp = raw_exp + gateway_common::devserver_gate::ENTRY_CLOCK_SKEW_SECONDS;
        for now in [raw_exp, raw_exp + 1, effective_exp] {
            let cache = EntryReplayCache::new(1);
            let jti = Uuid::new_v4();
            let subject = Uuid::new_v4();
            assert_eq!(cache.consume(jti, subject, effective_exp, 10), Ok(()));
            assert_eq!(
                cache.consume(jti, subject, effective_exp, now),
                Err(ConsumeError::Replay)
            );
        }
        let cache = EntryReplayCache::new(1);
        assert_eq!(
            cache.consume(
                Uuid::new_v4(),
                Uuid::new_v4(),
                effective_exp,
                effective_exp + 1
            ),
            Ok(())
        );
    }

    #[test]
    fn capacity_never_evicts_an_unexpired_consumed_jti() {
        let cache = EntryReplayCache::new(1);
        let oldest = Uuid::new_v4();
        let subject = Uuid::new_v4();
        assert_eq!(cache.consume(oldest, subject, 20, 10), Ok(()));
        assert_eq!(
            cache.consume(Uuid::new_v4(), Uuid::new_v4(), 30, 11),
            Err(ConsumeError::AtCapacity)
        );
        assert_eq!(
            cache.consume(oldest, subject, 20, 12),
            Err(ConsumeError::Replay)
        );
    }

    #[test]
    fn one_subject_cannot_exhaust_the_global_replay_capacity() {
        let cache = EntryReplayCache::with_subject_limit(4, 2);
        let attacker = Uuid::new_v4();
        let neighbor = Uuid::new_v4();
        assert_eq!(cache.consume(Uuid::new_v4(), attacker, 20, 10), Ok(()));
        assert_eq!(cache.consume(Uuid::new_v4(), attacker, 20, 10), Ok(()));
        assert_eq!(
            cache.consume(Uuid::new_v4(), attacker, 20, 10),
            Err(ConsumeError::AtCapacity)
        );
        assert_eq!(cache.consume(Uuid::new_v4(), neighbor, 20, 10), Ok(()));
    }

    #[test]
    fn expired_entries_release_the_subject_quota() {
        let cache = EntryReplayCache::with_subject_limit(4, 1);
        let subject = Uuid::new_v4();
        assert_eq!(cache.consume(Uuid::new_v4(), subject, 20, 10), Ok(()));
        assert_eq!(
            cache.consume(Uuid::new_v4(), subject, 30, 20),
            Err(ConsumeError::AtCapacity)
        );
        assert_eq!(cache.consume(Uuid::new_v4(), subject, 30, 21), Ok(()));
    }
}
