//! Per-tenant session presence: the leader and followers collaborating on one
//! served workspace (one `AppState`).
//!
//! [`window_presence`](crate::window_presence) answers "is this window
//! connected somewhere" with a bare `window_id -> socket count` refcount and
//! stays underneath unchanged (the `GET /api/windows` connected flag reads it).
//! This registry layers the COLLABORATION model on top: a per-tenant set of
//! participants keyed by the same `?w=<window_id>`, each with a role
//! (leader/follower), a lifecycle state, an optional display name, and a join
//! order. The first participant to connect is the leader; when the leader's
//! last socket drops and stays gone past the grace, the longest-connected live
//! participant is auto-promoted.
//!
//! Identity is the `window_id`, not the socket: a reload drops and reopens the
//! SAME `?w=` within the grace, so it must read as "still live, same
//! participant", never a leader-loss. The registry therefore keeps its own
//! per-participant socket count (independent of `window_presence`) so a reload's
//! brief old+new overlap holds the participant `Live`, and only a real
//! disconnect arms the grace clock.
//!
//! The state machine and grace clock are PURE and time-injectable
//! ([`reap_due`](SessionRegistry::reap_due) takes `now`); the `/ws` pump drives
//! the socket count via the RAII [`SessionGuard`], and a per-tenant reaper task
//! advances disconnected -> gone and broadcasts. Handover is a blocked
//! request/reply owned by the control layer; the registry holds only the
//! at-most-one pending request so the leader's separate `cs session handover
//! --accept` connection can find it.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::Serialize;
use tokio::sync::Notify;

/// A participant whose last socket dropped holds `Disconnecting` for this long
/// before it surfaces as `Disconnected` -- the reload-overlap window, during
/// which a reconnect of the same `window_id` is a silent reload, not a blip.
pub const RELOAD_GRACE: Duration = Duration::from_secs(5);

/// Total grace from the last socket dropping to the participant going `Gone`
/// (removed; leader auto-promotion fires). Outlasts a reload and a brief
/// network blip without stranding a session on a dead leader. The host-approved
/// "~30s grace".
pub const GONE_GRACE: Duration = Duration::from_secs(30);

/// A participant's connection lifecycle. `Gone` participants are removed from
/// the registry, so the enum surfaces in snapshots only as `Live` /
/// `Disconnecting` / `Disconnected`; `Gone` is the transition that drops the
/// entry (and is reported once in the reap outcome).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ParticipantState {
    /// At least one live `/ws` socket.
    Live,
    /// No socket, within [`RELOAD_GRACE`] -- treated as a reload in progress.
    Disconnecting,
    /// No socket, past [`RELOAD_GRACE`] but within [`GONE_GRACE`].
    Disconnected,
    /// Past [`GONE_GRACE`]: removed from the registry; if it was the leader,
    /// the slot is reassigned.
    Gone,
}

/// A participant's role in the session, derived from the elected leader.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Leader,
    Follower,
}

/// One participant's public state, as serialized for `cs session list` and the
/// `session_roster` `/ws` frame.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ParticipantInfo {
    pub window_id: String,
    /// The participant's chosen display name, if any (`cs session self --name`).
    pub name: Option<String>,
    pub role: Role,
    pub status: ParticipantState,
}

/// A point-in-time view of the whole session: every participant plus the
/// current leader's `window_id` (absent only when the session is empty or the
/// leader slot is transiently vacant).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SessionSnapshot {
    pub participants: Vec<ParticipantInfo>,
    pub leader: Option<String>,
}

/// An at-most-one in-flight handover request, parked here so the leader's
/// separate `cs session handover --accept/--reject` connection (or the overlay
/// reply route) can resolve the request the requester's blocked CLI is awaiting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingHandover {
    /// The `handover_bus` id the requester's handler is awaiting on.
    pub request_id: String,
    /// The window that asked for the handover (the prospective new leader when
    /// `target` is unset).
    pub requester: String,
    /// The window that becomes leader on accept.
    pub target: String,
    /// The leader being asked -- the only window allowed to accept/reject from
    /// the CLI.
    pub leader: String,
}

struct Participant {
    name: Option<String>,
    /// Live `/ws` socket count for this window (reload overlap can exceed 1).
    sockets: usize,
    /// Monotonic join order; the lowest among live participants wins an
    /// auto-promotion.
    join_seq: u64,
    state: ParticipantState,
    /// When the last socket dropped (`sockets == 0`); drives the grace clock.
    disconnected_at: Option<Instant>,
}

impl Participant {
    /// The lifecycle state implied by the socket count and the grace clock at
    /// `now`. Live while a socket is held; otherwise stepped by elapsed grace.
    fn computed_state(&self, now: Instant) -> ParticipantState {
        if self.sockets > 0 {
            return ParticipantState::Live;
        }
        let elapsed = self
            .disconnected_at
            .map(|t| now.saturating_duration_since(t))
            .unwrap_or_default();
        if elapsed >= GONE_GRACE {
            ParticipantState::Gone
        } else if elapsed >= RELOAD_GRACE {
            ParticipantState::Disconnected
        } else {
            ParticipantState::Disconnecting
        }
    }
}

#[derive(Default)]
struct Inner {
    /// window_id -> participant.
    participants: HashMap<String, Participant>,
    /// The elected leader's window_id.
    leader: Option<String>,
    /// Monotonic join counter for ordering.
    next_seq: u64,
    /// The single in-flight handover, if any.
    pending: Option<PendingHandover>,
}

/// The per-tenant session registry. One per `AppState`, shared by the `/ws`
/// pump (which drives the socket counts) and the control socket (which reads
/// snapshots and drives handover/takeover).
#[derive(Default)]
pub struct SessionRegistry {
    inner: Mutex<Inner>,
    /// Fired whenever a participant disconnects so the reaper recomputes its
    /// sleep deadline. Runtime-agnostic to fire; only the reaper awaits it.
    reaper_wake: Notify,
    /// Process-local id source for handover requests (lifetime-unique is
    /// enough; the registry is in memory).
    handover_counter: AtomicU64,
}

/// The outcome of a [`reap_due`](SessionRegistry::reap_due) pass: whether the
/// public snapshot changed (a state stepped, a participant went gone, or the
/// leader was reassigned) and when the next transition is due so the reaper can
/// sleep precisely.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReapOutcome {
    pub changed: bool,
    pub next_deadline: Option<Instant>,
}

impl SessionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Recover from a poisoned lock: every critical section is a small state
    /// mutation that cannot leave the map inconsistent, and presence must never
    /// panic a `/ws` teardown path.
    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// A `Notify` the reaper task awaits to recompute its sleep when a socket
    /// drops (the only event that arms a new grace deadline).
    pub fn reaper_wake(&self) -> &Notify {
        &self.reaper_wake
    }

    /// Register one live `/ws` socket for `window_id` and return the RAII guard
    /// that releases it. The first participant ever becomes leader; a socket
    /// arriving for a participant in its grace window re-lives it (a reload).
    /// `changed` reports whether the public snapshot moved (a new participant, a
    /// revived one, or a freshly elected leader) so the caller can broadcast.
    pub fn join(self: &Arc<Self>, window_id: &str) -> JoinResult {
        let changed = {
            let mut inner = self.lock();
            match inner.participants.get_mut(window_id) {
                Some(p) => {
                    let was_live = p.sockets > 0;
                    p.sockets += 1;
                    p.disconnected_at = None;
                    if !was_live && p.state != ParticipantState::Live {
                        p.state = ParticipantState::Live;
                        true
                    } else {
                        false
                    }
                }
                None => {
                    let seq = inner.next_seq;
                    inner.next_seq += 1;
                    inner.participants.insert(
                        window_id.to_string(),
                        Participant {
                            name: None,
                            sockets: 1,
                            join_seq: seq,
                            state: ParticipantState::Live,
                            disconnected_at: None,
                        },
                    );
                    if inner.leader.is_none() {
                        inner.leader = Some(window_id.to_string());
                    }
                    true
                }
            }
        };
        JoinResult {
            guard: SessionGuard {
                registry: Arc::clone(self),
                window_id: window_id.to_string(),
            },
            changed,
        }
    }

    /// Release one socket for `window_id` (the guard's Drop). When the last
    /// socket drops the participant enters `Disconnecting` and the grace clock
    /// starts at `at`; the reaper is woken to recompute its deadline.
    fn socket_dropped(&self, window_id: &str, at: Instant) {
        let mut armed = false;
        {
            let mut inner = self.lock();
            if let Some(p) = inner.participants.get_mut(window_id) {
                p.sockets = p.sockets.saturating_sub(1);
                if p.sockets == 0 {
                    p.state = ParticipantState::Disconnecting;
                    p.disconnected_at = Some(at);
                    armed = true;
                }
            }
        }
        if armed {
            self.reaper_wake.notify_waiters();
        }
    }

    /// Advance every disconnected participant's state by the grace clock at
    /// `now`: step `Disconnecting -> Disconnected -> Gone`, remove gone
    /// participants, and auto-promote the longest-connected live participant
    /// when a gone participant held the leader slot. Idempotent; the reaper
    /// calls it on each wake and `next_deadline` tells it when to wake next.
    pub fn reap_due(&self, now: Instant) -> ReapOutcome {
        let mut inner = self.lock();
        let mut changed = false;

        // Step states and collect the windows that went gone.
        let mut gone: Vec<String> = Vec::new();
        for (window_id, p) in inner.participants.iter_mut() {
            if p.sockets > 0 {
                continue;
            }
            let next = p.computed_state(now);
            if next != p.state {
                p.state = next;
                changed = true;
            }
            if next == ParticipantState::Gone {
                gone.push(window_id.clone());
            }
        }

        // Remove gone participants and reassign the leader if it left.
        let mut leader_lost = false;
        for window_id in &gone {
            inner.participants.remove(window_id);
            if inner.leader.as_deref() == Some(window_id.as_str()) {
                inner.leader = None;
                leader_lost = true;
            }
            // A pending handover whose leader or target vanished is stale.
            if inner
                .pending
                .as_ref()
                .is_some_and(|h| h.leader == *window_id || h.target == *window_id)
            {
                inner.pending = None;
            }
        }
        if leader_lost {
            Self::elect_leader(&mut inner);
        }

        let next_deadline = Self::soonest_deadline(&inner, now);
        ReapOutcome {
            changed,
            next_deadline,
        }
    }

    /// The earliest future state transition across disconnected participants,
    /// so the reaper sleeps exactly until the next `reap_due` would do work.
    fn soonest_deadline(inner: &Inner, now: Instant) -> Option<Instant> {
        inner
            .participants
            .values()
            .filter(|p| p.sockets == 0)
            .filter_map(|p| {
                let t = p.disconnected_at?;
                // The next threshold this participant has not crossed yet.
                let elapsed = now.saturating_duration_since(t);
                if elapsed < RELOAD_GRACE {
                    Some(t + RELOAD_GRACE)
                } else if elapsed < GONE_GRACE {
                    Some(t + GONE_GRACE)
                } else {
                    None
                }
            })
            .min()
    }

    /// Pick the lowest-`join_seq` live participant as leader (the
    /// longest-connected). Leaves the slot vacant when none is live.
    fn elect_leader(inner: &mut Inner) {
        let next = inner
            .participants
            .iter()
            .filter(|(_, p)| p.sockets > 0)
            .min_by_key(|(_, p)| p.join_seq)
            .map(|(id, _)| id.clone());
        inner.leader = next;
    }

    /// Rename a participant (the `cs session self --name` target). Returns
    /// whether a participant matched (and the snapshot changed).
    pub fn rename(&self, window_id: &str, name: &str) -> bool {
        let mut inner = self.lock();
        match inner.participants.get_mut(window_id) {
            Some(p) => {
                let trimmed = name.trim();
                p.name = if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                };
                true
            }
            None => false,
        }
    }

    /// The current leader's window_id, if any.
    pub fn leader(&self) -> Option<String> {
        self.lock().leader.clone()
    }

    /// A public snapshot of every participant plus the leader, ordered by join
    /// sequence. `now` resolves each participant's grace-clock state.
    pub fn snapshot(&self, now: Instant) -> SessionSnapshot {
        let inner = self.lock();
        let leader = inner.leader.clone();
        let mut rows: Vec<(u64, ParticipantInfo)> = inner
            .participants
            .iter()
            .map(|(window_id, p)| {
                let role = if Some(window_id.as_str()) == leader.as_deref() {
                    Role::Leader
                } else {
                    Role::Follower
                };
                (
                    p.join_seq,
                    ParticipantInfo {
                        window_id: window_id.clone(),
                        name: p.name.clone(),
                        role,
                        status: p.computed_state(now),
                    },
                )
            })
            .collect();
        rows.sort_by_key(|(seq, _)| *seq);
        SessionSnapshot {
            participants: rows.into_iter().map(|(_, info)| info).collect(),
            leader,
        }
    }

    /// Mint a fresh handover request id (`handover-{n}`), lifetime-unique.
    pub fn mint_handover_id(&self) -> String {
        let n = self.handover_counter.fetch_add(1, Ordering::Relaxed);
        format!("handover-{n}")
    }

    /// Park a handover request from `requester` (becoming leader as `target`,
    /// defaulting to the requester). Errors when the requester is not a
    /// participant, there is no live leader to ask, the requester already leads,
    /// or another handover is already pending. On success returns the leader's
    /// window_id (the prompt recipient).
    pub fn request_handover(
        &self,
        request_id: &str,
        requester: &str,
        target: Option<&str>,
    ) -> Result<String, HandoverError> {
        let mut inner = self.lock();
        if inner.pending.is_some() {
            return Err(HandoverError::AlreadyPending);
        }
        if !inner.participants.contains_key(requester) {
            return Err(HandoverError::NotAParticipant);
        }
        let target = target.unwrap_or(requester).to_string();
        if !inner.participants.contains_key(&target) {
            return Err(HandoverError::UnknownTarget);
        }
        let leader = inner.leader.clone().ok_or(HandoverError::NoLeader)?;
        if leader == target {
            return Err(HandoverError::AlreadyLeader);
        }
        // The leader must be live to consent; a non-live leader is taken over,
        // not handed over.
        let leader_live = inner
            .participants
            .get(&leader)
            .is_some_and(|p| p.sockets > 0);
        if !leader_live {
            return Err(HandoverError::LeaderNotLive);
        }
        inner.pending = Some(PendingHandover {
            request_id: request_id.to_string(),
            requester: requester.to_string(),
            target,
            leader: leader.clone(),
        });
        Ok(leader)
    }

    /// The pending handover this `leader` may answer, for the CLI
    /// accept/reject path (which has no request id of its own).
    pub fn pending_for_leader(&self, leader: &str) -> Option<PendingHandover> {
        self.lock()
            .pending
            .as_ref()
            .filter(|h| h.leader == leader)
            .cloned()
    }

    /// Resolve the pending handover identified by `request_id`: on `accept`
    /// promote the target to leader; either way clear the pending slot. Returns
    /// the new leader on a leadership change (so the caller can broadcast), or
    /// `None` when the id no longer matches (already resolved / stale).
    pub fn resolve_handover(&self, request_id: &str, accept: bool) -> Option<HandoverResolved> {
        let mut inner = self.lock();
        let matches = inner
            .pending
            .as_ref()
            .is_some_and(|h| h.request_id == request_id);
        if !matches {
            return None;
        }
        let pending = inner.pending.take().expect("matched just above");
        if accept && inner.participants.contains_key(&pending.target) {
            inner.leader = Some(pending.target.clone());
            Some(HandoverResolved {
                accepted: true,
                new_leader: Some(pending.target),
            })
        } else {
            Some(HandoverResolved {
                accepted: accept,
                new_leader: None,
            })
        }
    }

    /// Drop the pending handover identified by `request_id` without resolving
    /// it (the requester's handler timed out or disconnected). No-op when the id
    /// no longer matches.
    pub fn cancel_handover(&self, request_id: &str) {
        let mut inner = self.lock();
        if inner
            .pending
            .as_ref()
            .is_some_and(|h| h.request_id == request_id)
        {
            inner.pending = None;
        }
    }

    /// Take leadership for `caller`. Plain takeover (`force == false`) succeeds
    /// only when there is no live leader (none elected, or the leader's sockets
    /// are gone); `force` seizes even a live leader. The caller must be a
    /// participant. Returns whether leadership actually moved.
    pub fn takeover(&self, caller: &str, force: bool) -> Result<bool, HandoverError> {
        let mut inner = self.lock();
        if !inner.participants.contains_key(caller) {
            return Err(HandoverError::NotAParticipant);
        }
        if inner.leader.as_deref() == Some(caller) {
            return Ok(false);
        }
        let leader_live = inner
            .leader
            .as_ref()
            .and_then(|l| inner.participants.get(l))
            .is_some_and(|p| p.sockets > 0);
        if leader_live && !force {
            return Err(HandoverError::LeaderLive);
        }
        inner.leader = Some(caller.to_string());
        // A seized handover no longer applies.
        inner.pending = None;
        Ok(true)
    }
}

/// The new-leader result of resolving a handover.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandoverResolved {
    pub accepted: bool,
    pub new_leader: Option<String>,
}

/// Why a handover/takeover request could not be parked or applied. The control
/// handler maps each to a clear CLI message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandoverError {
    /// The calling window is not a session participant.
    NotAParticipant,
    /// No leader is currently elected.
    NoLeader,
    /// The requested target window is not a participant.
    UnknownTarget,
    /// The requester (or target) already leads.
    AlreadyLeader,
    /// Another handover is already in flight.
    AlreadyPending,
    /// The leader has no live socket, so it cannot consent (use takeover).
    LeaderNotLive,
    /// Plain takeover refused because the leader is live (needs `--force`).
    LeaderLive,
}

/// The result of [`SessionRegistry::join`]: the RAII guard plus whether the
/// public snapshot changed (so the caller can decide to broadcast).
pub struct JoinResult {
    pub guard: SessionGuard,
    pub changed: bool,
}

/// RAII handle for one `/ws` socket's participation. Dropping it releases the
/// socket; the last drop arms the grace clock. Held by the `/ws` pump for the
/// socket's lifetime, so every exit path (clean close, network drop, shutdown)
/// releases without bookkeeping at the call site.
pub struct SessionGuard {
    registry: Arc<SessionRegistry>,
    window_id: String,
}

impl SessionGuard {
    /// The window this guard's socket belongs to.
    pub fn window_id(&self) -> &str {
        &self.window_id
    }
}

impl Drop for SessionGuard {
    fn drop(&mut self) {
        self.registry
            .socket_dropped(&self.window_id, Instant::now());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status_of(reg: &SessionRegistry, now: Instant, window_id: &str) -> Option<ParticipantState> {
        reg.snapshot(now)
            .participants
            .into_iter()
            .find(|p| p.window_id == window_id)
            .map(|p| p.status)
    }

    #[test]
    fn first_participant_is_leader_others_follow() {
        let reg = Arc::new(SessionRegistry::new());
        let a = reg.join("w-a");
        assert!(a.changed);
        assert_eq!(reg.leader().as_deref(), Some("w-a"));

        let b = reg.join("w-b");
        assert!(b.changed);
        // The leader does not change just because a follower joins.
        assert_eq!(reg.leader().as_deref(), Some("w-a"));

        let now = Instant::now();
        let snap = reg.snapshot(now);
        assert_eq!(snap.participants.len(), 2);
        assert_eq!(snap.participants[0].window_id, "w-a");
        assert_eq!(snap.participants[0].role, Role::Leader);
        assert_eq!(snap.participants[1].role, Role::Follower);
    }

    #[test]
    fn reload_overlap_keeps_the_participant_live() {
        let reg = Arc::new(SessionRegistry::new());
        let g1 = reg.join("w-a").guard;
        // A second socket for the same window (the reload overlap) reports no
        // snapshot change and keeps it live after the first guard drops.
        let g2 = reg.join("w-a");
        assert!(!g2.changed);
        drop(g1);
        let now = Instant::now();
        assert_eq!(status_of(&reg, now, "w-a"), Some(ParticipantState::Live));
        drop(g2.guard);
    }

    #[test]
    fn lifecycle_steps_disconnecting_then_disconnected_then_gone() {
        let reg = Arc::new(SessionRegistry::new());
        let t0 = Instant::now();
        let guard = reg.join("w-a").guard;
        let _follower = reg.join("w-b").guard; // keep the session non-empty
        drop(guard); // last socket of w-a drops at ~t0

        // Immediately: Disconnecting.
        assert_eq!(
            status_of(&reg, t0, "w-a"),
            Some(ParticipantState::Disconnecting)
        );
        // Past the reload grace: Disconnected.
        let mid = t0 + RELOAD_GRACE + Duration::from_secs(1);
        let out = reg.reap_due(mid);
        assert!(out.changed);
        assert_eq!(
            status_of(&reg, mid, "w-a"),
            Some(ParticipantState::Disconnected)
        );
        // Past the gone grace: removed.
        let later = t0 + GONE_GRACE + Duration::from_secs(1);
        let out = reg.reap_due(later);
        assert!(out.changed);
        assert_eq!(status_of(&reg, later, "w-a"), None);
    }

    #[test]
    fn leader_gone_auto_promotes_longest_connected_live() {
        let reg = Arc::new(SessionRegistry::new());
        let t0 = Instant::now();
        let leader = reg.join("w-a").guard;
        let _b = reg.join("w-b").guard; // join_seq 1, live
        let _c = reg.join("w-c").guard; // join_seq 2, live
        assert_eq!(reg.leader().as_deref(), Some("w-a"));

        drop(leader);
        let later = t0 + GONE_GRACE + Duration::from_secs(1);
        let out = reg.reap_due(later);
        assert!(out.changed);
        // The longest-connected remaining live participant (lowest join_seq) wins.
        assert_eq!(reg.leader().as_deref(), Some("w-b"));
    }

    #[test]
    fn leader_vacant_when_no_live_participant_remains() {
        let reg = Arc::new(SessionRegistry::new());
        let t0 = Instant::now();
        let only = reg.join("w-a").guard;
        drop(only);
        let later = t0 + GONE_GRACE + Duration::from_secs(1);
        reg.reap_due(later);
        assert_eq!(reg.leader(), None);
    }

    #[test]
    fn next_deadline_tracks_the_soonest_transition() {
        let reg = Arc::new(SessionRegistry::new());
        let t0 = Instant::now();
        let g = reg.join("w-a").guard;
        let _keep = reg.join("w-b").guard;
        drop(g);
        // Before the reload grace, the next transition is the reload threshold.
        let out = reg.reap_due(t0);
        let deadline = out.next_deadline.expect("a disconnected participant");
        assert!(deadline <= t0 + RELOAD_GRACE + Duration::from_millis(1));
    }

    #[test]
    fn rename_sets_and_clears_the_display_name() {
        let reg = Arc::new(SessionRegistry::new());
        let _g = reg.join("w-a").guard;
        assert!(reg.rename("w-a", "  Alex  "));
        let now = Instant::now();
        assert_eq!(
            reg.snapshot(now).participants[0].name.as_deref(),
            Some("Alex")
        );
        // Whitespace-only clears it.
        assert!(reg.rename("w-a", "   "));
        assert_eq!(reg.snapshot(Instant::now()).participants[0].name, None);
        // Unknown window does not match.
        assert!(!reg.rename("w-nope", "x"));
    }

    #[test]
    fn handover_request_then_accept_moves_leadership() {
        let reg = Arc::new(SessionRegistry::new());
        let _leader = reg.join("w-a").guard;
        let _follower = reg.join("w-b").guard;
        let id = reg.mint_handover_id();
        // w-b asks to become leader; the prompt goes to the live leader w-a.
        let recipient = reg
            .request_handover(&id, "w-b", None)
            .expect("request parked");
        assert_eq!(recipient, "w-a");
        // The leader can find the pending request from its own connection.
        assert_eq!(
            reg.pending_for_leader("w-a").map(|h| h.request_id),
            Some(id.clone())
        );
        let resolved = reg.resolve_handover(&id, true).expect("resolved");
        assert!(resolved.accepted);
        assert_eq!(resolved.new_leader.as_deref(), Some("w-b"));
        assert_eq!(reg.leader().as_deref(), Some("w-b"));
        // The pending slot is cleared.
        assert_eq!(reg.pending_for_leader("w-b"), None);
    }

    #[test]
    fn handover_reject_keeps_leadership_and_clears_pending() {
        let reg = Arc::new(SessionRegistry::new());
        let _leader = reg.join("w-a").guard;
        let _follower = reg.join("w-b").guard;
        let id = reg.mint_handover_id();
        reg.request_handover(&id, "w-b", None).expect("parked");
        let resolved = reg.resolve_handover(&id, false).expect("resolved");
        assert!(!resolved.accepted);
        assert_eq!(reg.leader().as_deref(), Some("w-a"));
        assert!(reg.pending_for_leader("w-a").is_none());
    }

    #[test]
    fn handover_rejects_second_request_while_pending() {
        let reg = Arc::new(SessionRegistry::new());
        let _leader = reg.join("w-a").guard;
        let _b = reg.join("w-b").guard;
        let _c = reg.join("w-c").guard;
        let id = reg.mint_handover_id();
        reg.request_handover(&id, "w-b", None).expect("parked");
        let id2 = reg.mint_handover_id();
        assert_eq!(
            reg.request_handover(&id2, "w-c", None),
            Err(HandoverError::AlreadyPending)
        );
    }

    #[test]
    fn handover_to_a_non_live_leader_is_refused() {
        let reg = Arc::new(SessionRegistry::new());
        let t0 = Instant::now();
        let leader = reg.join("w-a").guard;
        let _b = reg.join("w-b").guard;
        drop(leader);
        reg.reap_due(t0 + RELOAD_GRACE + Duration::from_secs(1)); // leader Disconnected
        let id = reg.mint_handover_id();
        assert_eq!(
            reg.request_handover(&id, "w-b", None),
            Err(HandoverError::LeaderNotLive)
        );
    }

    #[test]
    fn plain_takeover_only_when_leader_not_live() {
        let reg = Arc::new(SessionRegistry::new());
        let t0 = Instant::now();
        let leader = reg.join("w-a").guard;
        let _b = reg.join("w-b").guard;
        // Live leader: plain takeover refused.
        assert_eq!(reg.takeover("w-b", false), Err(HandoverError::LeaderLive));
        // Leader drops and ages out of live: plain takeover succeeds.
        drop(leader);
        reg.reap_due(t0 + RELOAD_GRACE + Duration::from_secs(1));
        assert_eq!(reg.takeover("w-b", false), Ok(true));
        assert_eq!(reg.leader().as_deref(), Some("w-b"));
    }

    #[test]
    fn force_takeover_seizes_a_live_leader() {
        let reg = Arc::new(SessionRegistry::new());
        let _leader = reg.join("w-a").guard;
        let _b = reg.join("w-b").guard;
        assert_eq!(reg.takeover("w-b", true), Ok(true));
        assert_eq!(reg.leader().as_deref(), Some("w-b"));
        // Taking over when you already lead is a no-op, not an error.
        assert_eq!(reg.takeover("w-b", false), Ok(false));
    }

    #[test]
    fn takeover_by_non_participant_errors() {
        let reg = Arc::new(SessionRegistry::new());
        let _leader = reg.join("w-a").guard;
        assert_eq!(
            reg.takeover("w-ghost", true),
            Err(HandoverError::NotAParticipant)
        );
    }
}
