use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::time::Duration;

use chrono::{DateTime, Utc};
use devserver_control_proto::{
    AdmissionDecision, CanonicalOrigin, ProxyId, ServerFrame, TunnelRow,
};
use serde::Serialize;
use tokio::time::Instant;
use uuid::Uuid;

pub const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
pub const SESSION_DEAD_AFTER: Duration = Duration::from_secs(15);
pub const CONVERGENCE_WINDOW: Duration = Duration::from_secs(30);
pub const ADMISSION_CLAIM_TTL: Duration = Duration::from_secs(15);
pub const COMMAND_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_OUTSTANDING_PINGS: usize = 8;

type TunnelKey = (String, String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SessionIncarnation(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProxyStatus {
    Joining,
    Active,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProxyView {
    pub proxy_id: String,
    pub proxy_base_url: String,
    pub package_version: String,
    pub boot_id: Uuid,
    pub connected_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub tunnel_count: usize,
    pub status: ProxyStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TunnelView {
    pub user: String,
    pub devserver_id: String,
    pub peer_addr: Option<std::net::SocketAddr>,
    pub connected_at: DateTime<Utc>,
    pub proxy_id: String,
    pub proxy_base_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OwnedTunnel {
    session: SessionKey,
    row: TunnelRow,
}

#[derive(Debug, Clone)]
struct ProxySession {
    incarnation: SessionIncarnation,
    base_url: CanonicalOrigin,
    package_version: String,
    boot_id: Uuid,
    generation: Option<u64>,
    rows: HashMap<Uuid, TunnelRow>,
    status: ProxyStatus,
    fleet_ready: bool,
    connected_at: DateTime<Utc>,
    last_seen_at: DateTime<Utc>,
    last_seen: Instant,
    last_ping: Instant,
    outstanding_pings: VecDeque<u64>,
}

#[derive(Debug, Clone)]
struct PendingClaim {
    session: SessionKey,
    request_id: Uuid,
    registration_id: Uuid,
    expires_at: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommandPurpose {
    Reconciliation,
    Runtime,
}

#[derive(Debug)]
struct PendingCommand {
    session: SessionKey,
    registration_ids: HashSet<Uuid>,
    purpose: CommandPurpose,
    expires_at: Instant,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ReconciliationKind {
    Initial,
    Joining(SessionKey),
}

#[derive(Debug)]
struct Reconciliation {
    kind: ReconciliationKind,
    command_ids: HashSet<Uuid>,
    failed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct SessionKey {
    pub(crate) proxy_id: String,
    pub(crate) incarnation: SessionIncarnation,
}

#[derive(Debug)]
pub(crate) enum Effect {
    Send {
        session: SessionKey,
        frame: ServerFrame,
    },
    Retire {
        session: SessionKey,
        reason: String,
    },
    CommandSettled {
        command_id: Uuid,
        outcome: CommandOutcome,
    },
}

/// Terminal state of a Runtime-purpose kill command. The state machine
/// cannot hold waiters, so it reports each settle through
/// `Effect::CommandSettled` and the actor resolves the oneshot it
/// registered when the command was issued.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandOutcome {
    /// Every targeted registration was reported killed or already missing.
    Confirmed {
        killed: usize,
        missing: usize,
    },
    /// The proxy reported a failure, or the result did not account for
    /// every targeted registration exactly once.
    Failed,
    TimedOut,
    /// The owning session ended before the command settled.
    SessionLost,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum StateError {
    #[error("controller is not ready")]
    NotReady,
    #[error("proxy session is stale")]
    StaleSession,
    #[error("proxy session is not joining")]
    ProxyNotJoining,
    #[error("snapshot exceeds the row limit: {0}")]
    SnapshotTooLarge(usize),
    #[error("snapshot contains duplicate registration id {0}")]
    DuplicateRegistration(Uuid),
    #[error("another reconciliation is in progress")]
    ReconciliationInProgress,
    #[error("pong nonce is not outstanding")]
    InvalidPong,
}

pub(crate) struct ControllerState {
    max_devservers_per_user: usize,
    ready: bool,
    next_incarnation: u64,
    next_ping_nonce: u64,
    proxies: BTreeMap<String, ProxySession>,
    tunnels: HashMap<TunnelKey, OwnedTunnel>,
    pending: HashMap<TunnelKey, PendingClaim>,
    commands: HashMap<Uuid, PendingCommand>,
    reconciliation: Option<Reconciliation>,
    convergence_deadline: Option<Instant>,
}

impl ControllerState {
    pub fn new(max_devservers_per_user: usize) -> Self {
        Self {
            max_devservers_per_user,
            ready: false,
            next_incarnation: 1,
            next_ping_nonce: 1,
            proxies: BTreeMap::new(),
            tunnels: HashMap::new(),
            pending: HashMap::new(),
            commands: HashMap::new(),
            reconciliation: None,
            convergence_deadline: None,
        }
    }

    pub fn is_ready(&self) -> bool {
        self.ready
    }

    pub fn begin_session(
        &mut self,
        proxy_id: ProxyId,
        base_url: CanonicalOrigin,
        package_version: String,
        boot_id: Uuid,
        now: Instant,
        wall_now: DateTime<Utc>,
    ) -> (SessionIncarnation, Vec<Effect>) {
        let mut effects = Vec::new();
        if let Some(old) = self.current_key(proxy_id.as_str()) {
            if let Some(session) = self.proxies.get(proxy_id.as_str()) {
                if session.boot_id != boot_id {
                    tracing::error!(
                        proxy_id = proxy_id.as_str(),
                        old_boot_id = %session.boot_id,
                        new_boot_id = %boot_id,
                        "proxy id reconnected with a different boot id",
                    );
                }
            }
            effects.extend(self.remove_session(&old, now));
            effects.push(Effect::Retire {
                session: old,
                reason: "proxy session replaced".to_string(),
            });
        }

        let incarnation = SessionIncarnation(self.next_incarnation);
        self.next_incarnation = self.next_incarnation.wrapping_add(1).max(1);
        self.proxies.insert(
            proxy_id.as_str().to_string(),
            ProxySession {
                incarnation,
                base_url,
                package_version,
                boot_id,
                generation: None,
                rows: HashMap::new(),
                status: ProxyStatus::Joining,
                fleet_ready: false,
                connected_at: wall_now,
                last_seen_at: wall_now,
                last_seen: now,
                last_ping: now,
                outstanding_pings: VecDeque::new(),
            },
        );
        (incarnation, effects)
    }

    pub fn accept_snapshot(
        &mut self,
        proxy_id: &ProxyId,
        incarnation: SessionIncarnation,
        base_generation: u64,
        rows: Vec<TunnelRow>,
        now: Instant,
        wall_now: DateTime<Utc>,
    ) -> Result<Vec<Effect>, StateError> {
        if self.ready && self.reconciliation.is_some() {
            return Err(StateError::ReconciliationInProgress);
        }
        if rows.len() > devserver_control_proto::MAX_SNAPSHOT_ROWS {
            return Err(StateError::SnapshotTooLarge(rows.len()));
        }
        let mut by_id = HashMap::with_capacity(rows.len());
        for row in rows {
            let registration_id = row.registration_id;
            if by_id.insert(registration_id, row).is_some() {
                return Err(StateError::DuplicateRegistration(registration_id));
            }
        }

        let key = self.require_key(proxy_id, incarnation)?;
        let session = self
            .proxies
            .get_mut(proxy_id.as_str())
            .expect("key was validated");
        if session.status != ProxyStatus::Joining {
            return Err(StateError::ProxyNotJoining);
        }
        session.generation = Some(base_generation);
        session.rows = by_id;
        session.status = if self.ready {
            ProxyStatus::Joining
        } else {
            ProxyStatus::Active
        };
        session.last_seen = now;
        session.last_seen_at = wall_now;

        let mut effects = vec![Effect::Send {
            session: key.clone(),
            frame: ServerFrame::SnapshotAccepted { base_generation },
        }];
        if self.ready {
            effects.extend(self.reconcile_joining(key, now)?);
        } else if self.convergence_deadline.is_none() {
            self.convergence_deadline = Some(now + CONVERGENCE_WINDOW);
        }
        Ok(effects)
    }

    pub fn tunnel_up(
        &mut self,
        proxy_id: &ProxyId,
        incarnation: SessionIncarnation,
        generation: u64,
        row: TunnelRow,
        now: Instant,
        wall_now: DateTime<Utc>,
    ) -> Result<Vec<Effect>, StateError> {
        let key = self.require_key(proxy_id, incarnation)?;
        if let Some(effects) = self.advance_or_resync(&key, generation)? {
            return Ok(effects);
        }
        self.touch(&key, now, wall_now)?;
        if self
            .proxies
            .values()
            .any(|session| session.rows.contains_key(&row.registration_id))
        {
            return Ok(self.force_resync(&key, generation.saturating_add(1)));
        }

        let tunnel_key = (row.user.clone(), row.devserver_id.clone());
        let matching_claim = self.pending.get(&tunnel_key).is_some_and(|claim| {
            claim.session == key && claim.registration_id == row.registration_id
        });
        if !matching_claim {
            let (_, effects) =
                self.issue_kill(key, vec![row.registration_id], CommandPurpose::Runtime, now);
            return Ok(effects);
        }
        self.pending.remove(&tunnel_key);

        self.proxies
            .get_mut(proxy_id.as_str())
            .expect("key was validated")
            .rows
            .insert(row.registration_id, row.clone());

        let mut effects = Vec::new();
        if let Some(old) = self
            .tunnels
            .insert(tunnel_key, OwnedTunnel { session: key, row })
        {
            if self
                .tunnels
                .values()
                .all(|current| current.row.registration_id != old.row.registration_id)
            {
                let (_, kill) = self.issue_kill(
                    old.session,
                    vec![old.row.registration_id],
                    CommandPurpose::Runtime,
                    now,
                );
                effects.extend(kill);
            }
        }
        Ok(effects)
    }

    pub fn tunnel_down(
        &mut self,
        proxy_id: &ProxyId,
        incarnation: SessionIncarnation,
        generation: u64,
        registration_id: Uuid,
        now: Instant,
        wall_now: DateTime<Utc>,
    ) -> Result<Vec<Effect>, StateError> {
        let key = self.require_key(proxy_id, incarnation)?;
        if let Some(effects) = self.advance_or_resync(&key, generation)? {
            return Ok(effects);
        }
        self.touch(&key, now, wall_now)?;
        let known = self
            .proxies
            .get(proxy_id.as_str())
            .is_some_and(|session| session.rows.contains_key(&registration_id));
        if !known {
            return Ok(self.force_resync(&key, generation.saturating_add(1)));
        }
        self.remove_registration(&key, registration_id);
        Ok(Vec::new())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn request_admission(
        &mut self,
        proxy_id: &ProxyId,
        incarnation: SessionIncarnation,
        request_id: Uuid,
        registration_id: Uuid,
        user: String,
        devserver_id: String,
        now: Instant,
        wall_now: DateTime<Utc>,
    ) -> Result<Vec<Effect>, StateError> {
        let session_key = self.require_key(proxy_id, incarnation)?;
        self.touch(&session_key, now, wall_now)?;
        let active = self
            .proxies
            .get(proxy_id.as_str())
            .is_some_and(|session| session.status == ProxyStatus::Active && session.fleet_ready);
        if !self.ready || !active {
            return Ok(vec![admission_effect(
                session_key,
                request_id,
                registration_id,
                AdmissionDecision::ControlWarming,
            )]);
        }

        let tunnel_key = (user.clone(), devserver_id);
        if let Some(claim) = self.pending.get_mut(&tunnel_key).filter(|claim| {
            claim.session == session_key
                && claim.request_id == request_id
                && claim.registration_id == registration_id
        }) {
            claim.expires_at = now + ADMISSION_CLAIM_TTL;
            return Ok(vec![admission_effect(
                session_key,
                request_id,
                registration_id,
                AdmissionDecision::Admit,
            )]);
        }
        let reconnect =
            self.tunnels.contains_key(&tunnel_key) || self.pending.contains_key(&tunnel_key);
        if self.max_devservers_per_user > 0
            && !reconnect
            && self.distinct_for_user(&user) >= self.max_devservers_per_user
        {
            return Ok(vec![admission_effect(
                session_key,
                request_id,
                registration_id,
                AdmissionDecision::AtCapacity,
            )]);
        }

        let mut effects = Vec::new();
        if let Some(old) = self.pending.remove(&tunnel_key) {
            effects.push(admission_effect(
                old.session,
                old.request_id,
                old.registration_id,
                AdmissionDecision::Stale,
            ));
        }
        self.pending.insert(
            tunnel_key,
            PendingClaim {
                session: session_key.clone(),
                request_id,
                registration_id,
                expires_at: now + ADMISSION_CLAIM_TTL,
            },
        );
        effects.push(admission_effect(
            session_key,
            request_id,
            registration_id,
            AdmissionDecision::Admit,
        ));
        Ok(effects)
    }

    pub fn cancel_admission(
        &mut self,
        proxy_id: &ProxyId,
        incarnation: SessionIncarnation,
        request_id: Uuid,
        registration_id: Uuid,
        now: Instant,
        wall_now: DateTime<Utc>,
    ) -> Result<(), StateError> {
        let key = self.require_key(proxy_id, incarnation)?;
        self.touch(&key, now, wall_now)?;
        self.pending.retain(|_, claim| {
            claim.session != key
                || claim.request_id != request_id
                || claim.registration_id != registration_id
        });
        Ok(())
    }

    pub fn record_activity(
        &mut self,
        proxy_id: &ProxyId,
        incarnation: SessionIncarnation,
        now: Instant,
        wall_now: DateTime<Utc>,
    ) -> Result<(), StateError> {
        let key = self.require_key(proxy_id, incarnation)?;
        self.touch(&key, now, wall_now)
    }

    pub fn pong(
        &mut self,
        proxy_id: &ProxyId,
        incarnation: SessionIncarnation,
        nonce: u64,
        now: Instant,
        wall_now: DateTime<Utc>,
    ) -> Result<(), StateError> {
        let key = self.require_key(proxy_id, incarnation)?;
        let session = self
            .proxies
            .get_mut(proxy_id.as_str())
            .expect("key was validated");
        let Some(position) = session
            .outstanding_pings
            .iter()
            .position(|candidate| *candidate == nonce)
        else {
            return Err(StateError::InvalidPong);
        };
        session.outstanding_pings.remove(position);
        self.touch(&key, now, wall_now)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn command_result(
        &mut self,
        proxy_id: &ProxyId,
        incarnation: SessionIncarnation,
        command_id: Uuid,
        killed: Vec<Uuid>,
        missing: Vec<Uuid>,
        failed: Vec<Uuid>,
        now: Instant,
        wall_now: DateTime<Utc>,
    ) -> Result<Vec<Effect>, StateError> {
        let key = self.require_key(proxy_id, incarnation)?;
        self.touch(&key, now, wall_now)?;
        let Some(command) = self.commands.remove(&command_id) else {
            return Ok(Vec::new());
        };
        if command.session != key {
            self.commands.insert(command_id, command);
            return Err(StateError::StaleSession);
        }

        let reported: HashSet<Uuid> = killed
            .iter()
            .chain(&missing)
            .chain(&failed)
            .copied()
            .collect();
        let report_len = killed.len() + missing.len() + failed.len();
        let invalid = reported.len() != report_len
            || reported
                .iter()
                .any(|registration_id| !command.registration_ids.contains(registration_id));
        let incomplete = command
            .registration_ids
            .iter()
            .any(|registration_id| !reported.contains(registration_id));
        for registration_id in killed.iter().chain(&missing).copied() {
            if command.registration_ids.contains(&registration_id) {
                self.remove_registration(&key, registration_id);
            }
        }

        if command.purpose == CommandPurpose::Reconciliation {
            if let Some(reconciliation) = self.reconciliation.as_mut() {
                reconciliation.command_ids.remove(&command_id);
                reconciliation.failed |= invalid || incomplete || !failed.is_empty();
            }
            return Ok(self.finish_reconciliation_if_complete(now));
        }
        let outcome = if invalid || incomplete || !failed.is_empty() {
            CommandOutcome::Failed
        } else {
            CommandOutcome::Confirmed {
                killed: killed.len(),
                missing: missing.len(),
            }
        };
        Ok(vec![Effect::CommandSettled {
            command_id,
            outcome,
        }])
    }

    pub fn disconnect(
        &mut self,
        proxy_id: &ProxyId,
        incarnation: SessionIncarnation,
        now: Instant,
    ) -> Result<Vec<Effect>, StateError> {
        let key = self.require_key(proxy_id, incarnation)?;
        Ok(self.remove_session(&key, now))
    }

    pub fn require_resync(
        &mut self,
        proxy_id: &ProxyId,
        incarnation: SessionIncarnation,
    ) -> Result<Vec<Effect>, StateError> {
        let key = self.require_key(proxy_id, incarnation)?;
        let expected_generation = self
            .proxies
            .get(proxy_id.as_str())
            .and_then(|session| session.generation)
            .map_or(0, |generation| generation.saturating_add(1));
        Ok(self.force_resync(&key, expected_generation))
    }

    pub fn tick(&mut self, now: Instant, wall_now: DateTime<Utc>) -> Vec<Effect> {
        self.pending.retain(|_, claim| claim.expires_at > now);

        let mut effects = Vec::new();
        let expired_commands: Vec<Uuid> = self
            .commands
            .iter()
            .filter_map(|(command_id, command)| (command.expires_at <= now).then_some(*command_id))
            .collect();
        let mut reconciliation_expired = false;
        for command_id in expired_commands {
            let Some(command) = self.commands.remove(&command_id) else {
                continue;
            };
            tracing::warn!(%command_id, session = %command.session.proxy_id, "controller command timed out");
            if command.purpose == CommandPurpose::Reconciliation {
                if let Some(reconciliation) = self.reconciliation.as_mut() {
                    reconciliation.command_ids.remove(&command_id);
                    reconciliation.failed = true;
                    reconciliation_expired = true;
                }
            } else {
                effects.push(Effect::CommandSettled {
                    command_id,
                    outcome: CommandOutcome::TimedOut,
                });
            }
        }
        if reconciliation_expired {
            effects.extend(self.finish_reconciliation_if_complete(now));
        }

        let mut dead = Vec::new();
        for (proxy_id, session) in &mut self.proxies {
            if now.duration_since(session.last_seen) >= SESSION_DEAD_AFTER {
                tracing::warn!(proxy_id, "proxy control heartbeat expired");
                dead.push(SessionKey {
                    proxy_id: proxy_id.clone(),
                    incarnation: session.incarnation,
                });
                continue;
            }
            if now.duration_since(session.last_ping) >= HEARTBEAT_INTERVAL {
                session.last_ping = now;
                let nonce = self.next_ping_nonce;
                self.next_ping_nonce = self.next_ping_nonce.wrapping_add(1);
                session.outstanding_pings.push_back(nonce);
                while session.outstanding_pings.len() > MAX_OUTSTANDING_PINGS {
                    session.outstanding_pings.pop_front();
                }
                tracing::debug!(proxy_id, nonce, "sending proxy control heartbeat");
                effects.push(Effect::Send {
                    session: SessionKey {
                        proxy_id: proxy_id.clone(),
                        incarnation: session.incarnation,
                    },
                    frame: ServerFrame::Ping { nonce },
                });
            }
        }
        for key in dead {
            effects.extend(self.remove_session(&key, now));
            effects.push(Effect::Retire {
                session: key,
                reason: "proxy control heartbeat expired".to_string(),
            });
        }

        if !self.ready
            && self.reconciliation.is_none()
            && self
                .convergence_deadline
                .is_some_and(|deadline| deadline <= now)
        {
            effects.extend(self.begin_initial_reconciliation(now));
        }

        let _ = wall_now;
        effects
    }

    pub fn tunnel_views(&self) -> Vec<TunnelView> {
        let mut out: Vec<_> = self
            .tunnels
            .values()
            .filter_map(|owned| {
                let session = self.proxies.get(&owned.session.proxy_id)?;
                Some(TunnelView {
                    user: owned.row.user.clone(),
                    devserver_id: owned.row.devserver_id.clone(),
                    peer_addr: owned.row.peer_addr,
                    connected_at: owned.row.connected_at,
                    proxy_id: owned.session.proxy_id.clone(),
                    proxy_base_url: session.base_url.as_str().to_string(),
                })
            })
            .collect();
        out.sort_by(|a, b| {
            a.user
                .cmp(&b.user)
                .then_with(|| a.devserver_id.cmp(&b.devserver_id))
        });
        out
    }

    pub fn read_tunnels(&self) -> Result<Vec<TunnelView>, StateError> {
        self.ready
            .then(|| self.tunnel_views())
            .ok_or(StateError::NotReady)
    }

    pub fn proxy_views(&self) -> Vec<ProxyView> {
        self.proxies
            .iter()
            .map(|(proxy_id, session)| ProxyView {
                proxy_id: proxy_id.clone(),
                proxy_base_url: session.base_url.as_str().to_string(),
                package_version: session.package_version.clone(),
                boot_id: session.boot_id,
                connected_at: session.connected_at,
                last_seen_at: session.last_seen_at,
                tunnel_count: session.rows.len(),
                status: session.status,
            })
            .collect()
    }

    pub fn read_proxies(&self) -> Result<Vec<ProxyView>, StateError> {
        self.ready
            .then(|| self.proxy_views())
            .ok_or(StateError::NotReady)
    }

    /// Exact admin kill for one `(user, devserver_id)` key. The command
    /// targets the registration UUID read at issue time, never the key, so
    /// a delayed command cannot kill a successor registration. Returns the
    /// command id when a row was found so the actor can register a waiter
    /// before applying the send effect.
    pub fn begin_exact_kill(
        &mut self,
        user: &str,
        devserver_id: &str,
        now: Instant,
    ) -> Result<(Option<Uuid>, Vec<Effect>), StateError> {
        if !self.ready {
            return Err(StateError::NotReady);
        }
        let key = (user.to_string(), devserver_id.to_string());
        let Some(owned) = self.tunnels.get(&key) else {
            return Ok((None, Vec::new()));
        };
        let session = owned.session.clone();
        let registration_id = owned.row.registration_id;
        if !self.owns_aggregate_rows(&session) {
            return Ok((None, Vec::new()));
        }
        let (command_id, effects) =
            self.issue_kill(session, vec![registration_id], CommandPurpose::Runtime, now);
        Ok((Some(command_id), effects))
    }

    /// User-wide admin kill. Pending admission claims for the user are
    /// cancelled before any command is issued (fleet admission rule 7); a
    /// late `TunnelUp` for a cancelled claim arrives without a matching
    /// claim and is killed by the unclaimed-row path. Authoritative rows
    /// group by current owning session, one command per proxy. Returns the
    /// issued command ids so the actor can register waiters before
    /// applying the send effects.
    pub fn begin_user_kill(
        &mut self,
        user: &str,
        now: Instant,
    ) -> Result<(Vec<Uuid>, Vec<Effect>), StateError> {
        if !self.ready {
            return Err(StateError::NotReady);
        }
        self.pending.retain(|(owner, _), _| owner != user);

        let mut grouped: BTreeMap<SessionKey, Vec<Uuid>> = BTreeMap::new();
        for ((owner, _), owned) in &self.tunnels {
            if owner == user && self.owns_aggregate_rows(&owned.session) {
                grouped
                    .entry(owned.session.clone())
                    .or_default()
                    .push(owned.row.registration_id);
            }
        }
        let mut command_ids = Vec::new();
        let mut effects = Vec::new();
        for (session, registration_ids) in grouped {
            let (command_id, kill) =
                self.issue_kill(session, registration_ids, CommandPurpose::Runtime, now);
            command_ids.push(command_id);
            effects.extend(kill);
        }
        Ok((command_ids, effects))
    }

    /// Aggregate rows are only ever published from Active sessions:
    /// `force_resync` and session removal retract a session's rows before
    /// its status can leave Active. A kill must route to a session that
    /// still owns the row, so a stale or Joining owner is treated as not
    /// found rather than commanding a session that no longer carries the
    /// registration.
    fn owns_aggregate_rows(&self, session: &SessionKey) -> bool {
        self.proxies.get(&session.proxy_id).is_some_and(|proxy| {
            proxy.incarnation == session.incarnation && proxy.status == ProxyStatus::Active
        })
    }

    fn require_key(
        &self,
        proxy_id: &ProxyId,
        incarnation: SessionIncarnation,
    ) -> Result<SessionKey, StateError> {
        self.proxies
            .get(proxy_id.as_str())
            .filter(|session| session.incarnation == incarnation)
            .map(|_| SessionKey {
                proxy_id: proxy_id.as_str().to_string(),
                incarnation,
            })
            .ok_or(StateError::StaleSession)
    }

    fn current_key(&self, proxy_id: &str) -> Option<SessionKey> {
        self.proxies.get(proxy_id).map(|session| SessionKey {
            proxy_id: proxy_id.to_string(),
            incarnation: session.incarnation,
        })
    }

    fn touch(
        &mut self,
        key: &SessionKey,
        now: Instant,
        wall_now: DateTime<Utc>,
    ) -> Result<(), StateError> {
        let session = self
            .proxies
            .get_mut(&key.proxy_id)
            .filter(|session| session.incarnation == key.incarnation)
            .ok_or(StateError::StaleSession)?;
        session.last_seen = now;
        session.last_seen_at = wall_now;
        Ok(())
    }

    fn advance_or_resync(
        &mut self,
        key: &SessionKey,
        generation: u64,
    ) -> Result<Option<Vec<Effect>>, StateError> {
        let expected = self
            .proxies
            .get(&key.proxy_id)
            .filter(|session| session.incarnation == key.incarnation)
            .and_then(|session| session.generation)
            .map(|current| current + 1);
        let Some(expected) = expected else {
            return Err(StateError::StaleSession);
        };
        if generation != expected {
            return Ok(Some(self.force_resync(key, expected)));
        }
        self.proxies
            .get_mut(&key.proxy_id)
            .expect("key was validated")
            .generation = Some(generation);
        Ok(None)
    }

    fn force_resync(&mut self, key: &SessionKey, expected_generation: u64) -> Vec<Effect> {
        if let Some(session) = self.proxies.get_mut(&key.proxy_id) {
            session.status = ProxyStatus::Joining;
            session.fleet_ready = false;
            session.generation = None;
            session.rows.clear();
        }
        self.tunnels.retain(|_, owned| owned.session != *key);
        self.pending.retain(|_, claim| claim.session != *key);
        self.leave_readiness_if_no_active_sessions();
        vec![Effect::Send {
            session: key.clone(),
            frame: ServerFrame::ResyncRequired {
                expected_generation,
            },
        }]
    }

    fn begin_initial_reconciliation(&mut self, now: Instant) -> Vec<Effect> {
        if !self
            .proxies
            .values()
            .any(|session| session.status == ProxyStatus::Active)
        {
            self.convergence_deadline = None;
            return Vec::new();
        }
        let candidates = self.proxies.iter().flat_map(|(proxy_id, session)| {
            let key = SessionKey {
                proxy_id: proxy_id.clone(),
                incarnation: session.incarnation,
            };
            session.rows.values().cloned().map(move |row| OwnedTunnel {
                session: key.clone(),
                row,
            })
        });
        let (desired, losers) = self.initial_reconciliation_plan(candidates);
        if losers.is_empty() {
            self.tunnels = desired;
            self.ready = true;
            self.convergence_deadline = None;
            return self.mark_fleet_ready();
        }
        self.start_reconciliation(ReconciliationKind::Initial, losers, now)
    }

    fn reconcile_joining(
        &mut self,
        joining: SessionKey,
        now: Instant,
    ) -> Result<Vec<Effect>, StateError> {
        if self.reconciliation.is_some() {
            return Err(StateError::ReconciliationInProgress);
        }
        let session = self
            .proxies
            .get(&joining.proxy_id)
            .filter(|session| session.incarnation == joining.incarnation)
            .ok_or(StateError::StaleSession)?;
        let rows = session.rows.values().cloned().collect();
        let (desired, losers) = self.joining_plan(&joining, rows);
        if losers.is_empty() {
            self.tunnels = desired;
            if let Some(session) = self.proxies.get_mut(&joining.proxy_id) {
                session.status = ProxyStatus::Active;
                session.fleet_ready = true;
            }
            return Ok(vec![Effect::Send {
                session: joining,
                frame: ServerFrame::FleetReady,
            }]);
        }
        Ok(self.start_reconciliation(ReconciliationKind::Joining(joining), losers, now))
    }

    /// Deterministic winner election for initial restart reconciliation.
    /// Recency is genuinely unavailable while the fleet is reconstructed
    /// from snapshots, so duplicates resolve to the lexicographically
    /// smallest `(proxy_id, registration_id)` and capacity trims sort by
    /// `(devserver_id, proxy_id, registration_id)`. This is the only path
    /// allowed to re-rank rows; a routine join uses `joining_plan` instead.
    fn initial_reconciliation_plan(
        &self,
        candidates: impl IntoIterator<Item = OwnedTunnel>,
    ) -> (HashMap<TunnelKey, OwnedTunnel>, Vec<OwnedTunnel>) {
        let mut grouped: BTreeMap<TunnelKey, Vec<OwnedTunnel>> = BTreeMap::new();
        for candidate in candidates {
            grouped
                .entry((
                    candidate.row.user.clone(),
                    candidate.row.devserver_id.clone(),
                ))
                .or_default()
                .push(candidate);
        }

        let mut desired = HashMap::new();
        let mut losers = Vec::new();
        for (key, mut rows) in grouped {
            rows.sort_by(|a, b| {
                a.session
                    .proxy_id
                    .cmp(&b.session.proxy_id)
                    .then_with(|| a.row.registration_id.cmp(&b.row.registration_id))
            });
            let winner = rows.remove(0);
            desired.insert(key, winner);
            losers.extend(rows);
        }

        if self.max_devservers_per_user > 0 {
            let mut by_user: BTreeMap<String, Vec<(TunnelKey, OwnedTunnel)>> = BTreeMap::new();
            for (key, owned) in &desired {
                by_user
                    .entry(key.0.clone())
                    .or_default()
                    .push((key.clone(), owned.clone()));
            }
            for rows in by_user.values_mut() {
                rows.sort_by(|(a_key, a), (b_key, b)| {
                    a_key
                        .1
                        .cmp(&b_key.1)
                        .then_with(|| a.session.proxy_id.cmp(&b.session.proxy_id))
                        .then_with(|| a.row.registration_id.cmp(&b.row.registration_id))
                });
                for (key, loser) in rows.iter().skip(self.max_devservers_per_user) {
                    desired.remove(key);
                    losers.push(loser.clone());
                }
            }
        }

        let mut seen = HashSet::new();
        losers.retain(|owned| seen.insert(owned.row.registration_id));
        (desired, losers)
    }

    /// Live-first reconciliation for a routine joining snapshot. Every row
    /// the controller currently publishes is an immutable winner: those rows
    /// were admitted during this controller lifetime, so recency is
    /// available and a joining snapshot must never outrank it. Joining rows
    /// that duplicate a live key lose, each user's live rows are reserved
    /// against the capacity limit first, and only novel joining keys that
    /// fit the remaining slots are admitted. Competing rows inside one
    /// snapshot resolve by registration id, an ordering local to that
    /// snapshot; proxy id is never treated as recency on a routine join.
    fn joining_plan(
        &self,
        joining: &SessionKey,
        rows: Vec<TunnelRow>,
    ) -> (HashMap<TunnelKey, OwnedTunnel>, Vec<OwnedTunnel>) {
        let mut desired = self.tunnels.clone();
        let mut live_per_user: HashMap<String, usize> = HashMap::new();
        for (user, _) in self.tunnels.keys() {
            *live_per_user.entry(user.clone()).or_default() += 1;
        }

        let mut grouped: BTreeMap<TunnelKey, Vec<TunnelRow>> = BTreeMap::new();
        for row in rows {
            grouped
                .entry((row.user.clone(), row.devserver_id.clone()))
                .or_default()
                .push(row);
        }

        let mut losers = Vec::new();
        for (key, mut rows) in grouped {
            rows.sort_by_key(|row| row.registration_id);
            let mut rows = rows.into_iter().map(|row| OwnedTunnel {
                session: joining.clone(),
                row,
            });
            let live = live_per_user.get(&key.0).copied().unwrap_or(0);
            let duplicates_live = desired.contains_key(&key);
            let over_capacity =
                self.max_devservers_per_user > 0 && live >= self.max_devservers_per_user;
            if duplicates_live || over_capacity {
                losers.extend(rows);
                continue;
            }
            let winner = rows.next().expect("grouped rows are non-empty");
            desired.insert(key.clone(), winner);
            live_per_user.insert(key.0, live + 1);
            losers.extend(rows);
        }
        (desired, losers)
    }

    fn start_reconciliation(
        &mut self,
        kind: ReconciliationKind,
        losers: Vec<OwnedTunnel>,
        now: Instant,
    ) -> Vec<Effect> {
        let mut grouped: BTreeMap<SessionKey, Vec<Uuid>> = BTreeMap::new();
        for loser in losers {
            grouped
                .entry(loser.session)
                .or_default()
                .push(loser.row.registration_id);
        }

        let mut command_ids = HashSet::new();
        let mut effects = Vec::new();
        for (session, registration_ids) in grouped {
            let (command_id, kill) = self.issue_kill(
                session,
                registration_ids,
                CommandPurpose::Reconciliation,
                now,
            );
            command_ids.insert(command_id);
            effects.extend(kill);
        }
        self.reconciliation = Some(Reconciliation {
            kind,
            command_ids,
            failed: false,
        });
        effects
    }

    fn finish_reconciliation_if_complete(&mut self, now: Instant) -> Vec<Effect> {
        if self
            .reconciliation
            .as_ref()
            .is_none_or(|reconciliation| !reconciliation.command_ids.is_empty())
        {
            return Vec::new();
        }
        let reconciliation = self.reconciliation.take().expect("checked above");
        if reconciliation.failed {
            return match reconciliation.kind {
                ReconciliationKind::Initial => {
                    self.convergence_deadline = Some(now + CONVERGENCE_WINDOW);
                    Vec::new()
                }
                ReconciliationKind::Joining(session) => {
                    let mut effects = self.remove_session(&session, now);
                    effects.push(Effect::Retire {
                        session,
                        reason: "joining snapshot reconciliation failed".to_string(),
                    });
                    effects
                }
            };
        }

        let (desired, losers) = match &reconciliation.kind {
            ReconciliationKind::Initial => {
                let candidates: Vec<OwnedTunnel> = self
                    .proxies
                    .iter()
                    .filter(|(_, session)| session.generation.is_some())
                    .flat_map(|(proxy_id, session)| {
                        let key = SessionKey {
                            proxy_id: proxy_id.clone(),
                            incarnation: session.incarnation,
                        };
                        session.rows.values().cloned().map(move |row| OwnedTunnel {
                            session: key.clone(),
                            row,
                        })
                    })
                    .collect();
                self.initial_reconciliation_plan(candidates)
            }
            ReconciliationKind::Joining(joining) => {
                // Recompute from current actor state so live deltas that
                // landed while the kill commands were outstanding keep
                // their immutable-winner status.
                let rows = self
                    .proxies
                    .get(&joining.proxy_id)
                    .filter(|session| session.incarnation == joining.incarnation)
                    .map(|session| session.rows.values().cloned().collect::<Vec<_>>())
                    .unwrap_or_default();
                self.joining_plan(joining, rows)
            }
        };
        if !losers.is_empty() {
            return self.start_reconciliation(reconciliation.kind, losers, now);
        }
        self.tunnels = desired;
        match reconciliation.kind {
            ReconciliationKind::Initial => {
                self.ready = true;
                self.convergence_deadline = None;
                self.mark_fleet_ready()
            }
            ReconciliationKind::Joining(session) => {
                let Some(proxy) = self
                    .proxies
                    .get_mut(&session.proxy_id)
                    .filter(|proxy| proxy.incarnation == session.incarnation)
                else {
                    return Vec::new();
                };
                proxy.status = ProxyStatus::Active;
                if proxy.generation.is_some() {
                    proxy.fleet_ready = true;
                } else {
                    return Vec::new();
                }
                vec![Effect::Send {
                    session,
                    frame: ServerFrame::FleetReady,
                }]
            }
        }
    }

    fn mark_fleet_ready(&mut self) -> Vec<Effect> {
        let mut effects = Vec::new();
        for (proxy_id, session) in &mut self.proxies {
            if session.status != ProxyStatus::Active {
                continue;
            }
            session.fleet_ready = true;
            effects.push(Effect::Send {
                session: SessionKey {
                    proxy_id: proxy_id.clone(),
                    incarnation: session.incarnation,
                },
                frame: ServerFrame::FleetReady,
            });
        }
        effects
    }

    fn issue_kill(
        &mut self,
        session: SessionKey,
        registration_ids: Vec<Uuid>,
        purpose: CommandPurpose,
        now: Instant,
    ) -> (Uuid, Vec<Effect>) {
        let command_id = Uuid::new_v4();
        self.commands.insert(
            command_id,
            PendingCommand {
                session: session.clone(),
                registration_ids: registration_ids.iter().copied().collect(),
                purpose,
                expires_at: now + COMMAND_TIMEOUT,
            },
        );
        (
            command_id,
            vec![Effect::Send {
                session,
                frame: ServerFrame::KillRegistrations {
                    command_id,
                    registration_ids,
                },
            }],
        )
    }

    fn remove_session(&mut self, key: &SessionKey, now: Instant) -> Vec<Effect> {
        let current = self
            .proxies
            .get(&key.proxy_id)
            .is_some_and(|session| session.incarnation == key.incarnation);
        if !current {
            return Vec::new();
        }
        self.proxies.remove(&key.proxy_id);
        self.tunnels.retain(|_, owned| owned.session != *key);
        self.pending.retain(|_, claim| claim.session != *key);

        let removed_commands: Vec<Uuid> = self
            .commands
            .iter()
            .filter_map(|(command_id, command)| (command.session == *key).then_some(*command_id))
            .collect();
        let mut effects = Vec::new();
        for command_id in removed_commands {
            let Some(command) = self.commands.remove(&command_id) else {
                continue;
            };
            if command.purpose == CommandPurpose::Runtime {
                effects.push(Effect::CommandSettled {
                    command_id,
                    outcome: CommandOutcome::SessionLost,
                });
            }
            if let Some(reconciliation) = self.reconciliation.as_mut() {
                reconciliation.command_ids.remove(&command_id);
            }
        }
        effects.extend(self.finish_reconciliation_if_complete(now));
        self.leave_readiness_if_no_active_sessions();
        effects
    }

    fn leave_readiness_if_no_active_sessions(&mut self) {
        let any_active = self
            .proxies
            .values()
            .any(|session| session.status == ProxyStatus::Active);
        if !any_active {
            self.ready = false;
            self.tunnels.clear();
            self.convergence_deadline = None;
        }
    }

    fn remove_registration(&mut self, session: &SessionKey, registration_id: Uuid) {
        if let Some(proxy) = self.proxies.get_mut(&session.proxy_id) {
            if proxy.incarnation == session.incarnation {
                proxy.rows.remove(&registration_id);
            }
        }
        self.tunnels.retain(|_, owned| {
            owned.session != *session || owned.row.registration_id != registration_id
        });
    }

    fn distinct_for_user(&self, user: &str) -> usize {
        self.tunnels
            .keys()
            .chain(self.pending.keys())
            .filter(|(owner, _)| owner == user)
            .map(|(_, devserver_id)| devserver_id)
            .collect::<HashSet<_>>()
            .len()
    }
}

fn admission_effect(
    session: SessionKey,
    request_id: Uuid,
    registration_id: Uuid,
    decision: AdmissionDecision,
) -> Effect {
    Effect::Send {
        session,
        frame: ServerFrame::AdmissionDecision {
            request_id,
            registration_id,
            decision,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proxy(id: &str) -> ProxyId {
        ProxyId::parse(id).unwrap()
    }

    fn origin(id: &str) -> CanonicalOrigin {
        CanonicalOrigin::parse(&format!("https://{id}.proxy.example.test")).unwrap()
    }

    fn row(user: &str, devserver: &str, registration_id: Uuid) -> TunnelRow {
        TunnelRow {
            registration_id,
            user: user.into(),
            devserver_id: devserver.into(),
            peer_addr: None,
            connected_at: Utc::now(),
        }
    }

    fn begin(state: &mut ControllerState, id: &str, now: Instant) -> (ProxyId, SessionIncarnation) {
        let id = proxy(id);
        let (incarnation, _) = state.begin_session(
            id.clone(),
            origin(id.as_str()),
            env!("CARGO_PKG_VERSION").into(),
            Uuid::new_v4(),
            now,
            Utc::now(),
        );
        (id, incarnation)
    }

    fn snapshot(
        state: &mut ControllerState,
        id: &ProxyId,
        incarnation: SessionIncarnation,
        rows: Vec<TunnelRow>,
        now: Instant,
    ) -> Vec<Effect> {
        state
            .accept_snapshot(id, incarnation, 0, rows, now, Utc::now())
            .unwrap()
    }

    fn ready_one(
        state: &mut ControllerState,
        id: &str,
        rows: Vec<TunnelRow>,
        now: Instant,
    ) -> (ProxyId, SessionIncarnation, Vec<Effect>) {
        let (id, incarnation) = begin(state, id, now);
        snapshot(state, &id, incarnation, rows, now);
        state
            .record_activity(&id, incarnation, now + CONVERGENCE_WINDOW, Utc::now())
            .unwrap();
        let effects = state.tick(now + CONVERGENCE_WINDOW, Utc::now());
        (id, incarnation, effects)
    }

    #[test]
    fn stale_incarnation_cannot_disconnect_replacement() {
        let now = Instant::now();
        let mut state = ControllerState::new(100);
        let (id, old) = begin(&mut state, "p1", now);
        let (_, current) = begin(&mut state, "p1", now);
        assert_ne!(old, current);
        assert!(matches!(
            state.disconnect(&id, old, now),
            Err(StateError::StaleSession)
        ));
        assert_eq!(state.proxy_views().len(), 1);
        assert_eq!(state.proxy_views()[0].proxy_id, "p1");
    }

    #[test]
    fn retired_incarnation_rejects_every_late_mutation() {
        let now = Instant::now();
        let mut state = ControllerState::new(100);
        let (id, retired) = begin(&mut state, "p1", now);
        let (_, current) = begin(&mut state, "p1", now);

        assert!(matches!(
            state.tunnel_up(
                &id,
                retired,
                1,
                row("alice", "one", Uuid::new_v4()),
                now,
                Utc::now(),
            ),
            Err(StateError::StaleSession)
        ));
        assert!(matches!(
            state.record_activity(&id, retired, now, Utc::now()),
            Err(StateError::StaleSession)
        ));
        assert!(matches!(
            state.command_result(
                &id,
                retired,
                Uuid::new_v4(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                now,
                Utc::now(),
            ),
            Err(StateError::StaleSession)
        ));
        assert!(matches!(
            state.disconnect(&id, retired, now),
            Err(StateError::StaleSession)
        ));
        assert_eq!(state.current_key(id.as_str()).unwrap().incarnation, current);
    }

    #[test]
    fn readiness_waits_for_the_full_convergence_window() {
        let now = Instant::now();
        let mut state = ControllerState::new(100);
        let (id, incarnation) = begin(&mut state, "p1", now);
        let effects = snapshot(&mut state, &id, incarnation, Vec::new(), now);
        assert!(effects.iter().any(|effect| matches!(
            effect,
            Effect::Send {
                frame: ServerFrame::SnapshotAccepted { .. },
                ..
            }
        )));
        assert!(!state.is_ready());
        state
            .record_activity(
                &id,
                incarnation,
                now + CONVERGENCE_WINDOW - Duration::from_millis(1),
                Utc::now(),
            )
            .unwrap();
        state.tick(
            now + CONVERGENCE_WINDOW - Duration::from_millis(1),
            Utc::now(),
        );
        assert!(!state.is_ready());
        state
            .record_activity(&id, incarnation, now + CONVERGENCE_WINDOW, Utc::now())
            .unwrap();
        let effects = state.tick(now + CONVERGENCE_WINDOW, Utc::now());
        assert!(state.is_ready());
        assert!(effects.iter().any(|effect| matches!(
            effect,
            Effect::Send {
                frame: ServerFrame::FleetReady,
                ..
            }
        )));
    }

    #[test]
    fn generation_gap_requests_resync_without_applying_delta() {
        let now = Instant::now();
        let mut state = ControllerState::new(100);
        let (id, incarnation, _) = ready_one(&mut state, "p1", Vec::new(), now);
        let effects = state
            .tunnel_up(
                &id,
                incarnation,
                2,
                row("alice", "one", Uuid::new_v4()),
                now + CONVERGENCE_WINDOW,
                Utc::now(),
            )
            .unwrap();
        assert!(effects.iter().any(|effect| matches!(
            effect,
            Effect::Send {
                frame: ServerFrame::ResyncRequired {
                    expected_generation: 1
                },
                ..
            }
        )));
        assert!(state.tunnel_views().is_empty());
        assert_eq!(state.proxy_views()[0].status, ProxyStatus::Joining);
    }

    #[test]
    fn illegal_registration_ids_force_a_fresh_snapshot() {
        let now = Instant::now();
        let mut state = ControllerState::new(100);
        let (id, incarnation, _) = ready_one(&mut state, "p1", Vec::new(), now);
        let effects = state
            .tunnel_down(
                &id,
                incarnation,
                1,
                Uuid::new_v4(),
                now + CONVERGENCE_WINDOW,
                Utc::now(),
            )
            .unwrap();
        assert!(has_resync(&effects, 2));
        assert_eq!(state.proxy_views()[0].status, ProxyStatus::Joining);

        let mut state = ControllerState::new(100);
        let registration_id = Uuid::new_v4();
        let (id, incarnation, _) = ready_one(
            &mut state,
            "p1",
            vec![row("alice", "one", registration_id)],
            now,
        );
        state
            .request_admission(
                &id,
                incarnation,
                Uuid::new_v4(),
                registration_id,
                "alice".into(),
                "two".into(),
                now + CONVERGENCE_WINDOW,
                Utc::now(),
            )
            .unwrap();
        let effects = state
            .tunnel_up(
                &id,
                incarnation,
                1,
                row("alice", "two", registration_id),
                now + CONVERGENCE_WINDOW,
                Utc::now(),
            )
            .unwrap();
        assert!(has_resync(&effects, 2));
        assert!(state.tunnel_views().is_empty());
    }

    #[test]
    fn pong_must_match_an_outstanding_ping() {
        let now = Instant::now();
        let mut state = ControllerState::new(100);
        let (id, incarnation) = begin(&mut state, "p1", now);
        snapshot(&mut state, &id, incarnation, Vec::new(), now);
        let effects = state.tick(now + HEARTBEAT_INTERVAL, Utc::now());
        let nonce = effects
            .iter()
            .find_map(|effect| match effect {
                Effect::Send {
                    frame: ServerFrame::Ping { nonce },
                    ..
                } => Some(*nonce),
                _ => None,
            })
            .unwrap();
        assert_eq!(
            state.pong(&id, incarnation, nonce + 1, now, Utc::now()),
            Err(StateError::InvalidPong)
        );
        state
            .pong(&id, incarnation, nonce, now, Utc::now())
            .unwrap();
        assert_eq!(
            state.pong(&id, incarnation, nonce, now, Utc::now()),
            Err(StateError::InvalidPong)
        );
    }

    #[test]
    fn expired_pending_claim_no_longer_consumes_capacity() {
        let now = Instant::now();
        let mut state = ControllerState::new(1);
        let (id, incarnation, _) = ready_one(&mut state, "p1", Vec::new(), now);
        let first = state
            .request_admission(
                &id,
                incarnation,
                Uuid::new_v4(),
                Uuid::new_v4(),
                "alice".into(),
                "one".into(),
                now + CONVERGENCE_WINDOW,
                Utc::now(),
            )
            .unwrap();
        assert!(has_decision(&first, AdmissionDecision::Admit));
        state
            .record_activity(
                &id,
                incarnation,
                now + CONVERGENCE_WINDOW + ADMISSION_CLAIM_TTL,
                Utc::now(),
            )
            .unwrap();
        state.tick(now + CONVERGENCE_WINDOW + ADMISSION_CLAIM_TTL, Utc::now());
        let second = state
            .request_admission(
                &id,
                incarnation,
                Uuid::new_v4(),
                Uuid::new_v4(),
                "alice".into(),
                "two".into(),
                now + CONVERGENCE_WINDOW + ADMISSION_CLAIM_TTL,
                Utc::now(),
            )
            .unwrap();
        assert!(has_decision(&second, AdmissionDecision::Admit));
    }

    #[test]
    fn newer_pending_claim_marks_the_old_request_stale() {
        let now = Instant::now();
        let mut state = ControllerState::new(100);
        let (id, incarnation, _) = ready_one(&mut state, "p1", Vec::new(), now);
        state
            .request_admission(
                &id,
                incarnation,
                Uuid::new_v4(),
                Uuid::new_v4(),
                "alice".into(),
                "one".into(),
                now + CONVERGENCE_WINDOW,
                Utc::now(),
            )
            .unwrap();
        let effects = state
            .request_admission(
                &id,
                incarnation,
                Uuid::new_v4(),
                Uuid::new_v4(),
                "alice".into(),
                "one".into(),
                now + CONVERGENCE_WINDOW,
                Utc::now(),
            )
            .unwrap();
        assert!(has_decision(&effects, AdmissionDecision::Stale));
        assert!(has_decision(&effects, AdmissionDecision::Admit));
    }

    #[test]
    fn admission_counts_pending_and_treats_retries_and_reconnects_as_neutral() {
        let now = Instant::now();
        let mut warming = ControllerState::new(1);
        let (id, incarnation) = begin(&mut warming, "p1", now);
        snapshot(&mut warming, &id, incarnation, Vec::new(), now);
        let effects = warming
            .request_admission(
                &id,
                incarnation,
                Uuid::new_v4(),
                Uuid::new_v4(),
                "alice".into(),
                "one".into(),
                now,
                Utc::now(),
            )
            .unwrap();
        assert!(has_decision(&effects, AdmissionDecision::ControlWarming));

        let mut state = ControllerState::new(1);
        let (p1, p1_incarnation, _) = ready_one(&mut state, "p1", Vec::new(), now);
        let request_id = Uuid::new_v4();
        let registration_id = Uuid::new_v4();
        let first = state
            .request_admission(
                &p1,
                p1_incarnation,
                request_id,
                registration_id,
                "alice".into(),
                "one".into(),
                now + CONVERGENCE_WINDOW,
                Utc::now(),
            )
            .unwrap();
        assert!(has_decision(&first, AdmissionDecision::Admit));
        let retry = state
            .request_admission(
                &p1,
                p1_incarnation,
                request_id,
                registration_id,
                "alice".into(),
                "one".into(),
                now + CONVERGENCE_WINDOW,
                Utc::now(),
            )
            .unwrap();
        assert!(has_decision(&retry, AdmissionDecision::Admit));
        assert!(!has_decision(&retry, AdmissionDecision::Stale));
        let capped = state
            .request_admission(
                &p1,
                p1_incarnation,
                Uuid::new_v4(),
                Uuid::new_v4(),
                "alice".into(),
                "two".into(),
                now + CONVERGENCE_WINDOW,
                Utc::now(),
            )
            .unwrap();
        assert!(has_decision(&capped, AdmissionDecision::AtCapacity));

        let (p2, p2_incarnation) = begin(&mut state, "p2", now + CONVERGENCE_WINDOW);
        snapshot(
            &mut state,
            &p2,
            p2_incarnation,
            Vec::new(),
            now + CONVERGENCE_WINDOW,
        );
        let reconnect = state
            .request_admission(
                &p2,
                p2_incarnation,
                Uuid::new_v4(),
                Uuid::new_v4(),
                "alice".into(),
                "one".into(),
                now + CONVERGENCE_WINDOW,
                Utc::now(),
            )
            .unwrap();
        assert!(has_decision(&reconnect, AdmissionDecision::Admit));
    }

    #[test]
    fn zero_fleet_recovery_waits_for_a_new_full_window() {
        let now = Instant::now();
        let mut state = ControllerState::new(100);
        let (p1, p1_incarnation, _) = ready_one(&mut state, "p1", Vec::new(), now);
        state.disconnect(&p1, p1_incarnation, now).unwrap();
        assert!(!state.is_ready());

        let restart_at = now + CONVERGENCE_WINDOW;
        let (p2, p2_incarnation) = begin(&mut state, "p2", restart_at);
        snapshot(&mut state, &p2, p2_incarnation, Vec::new(), restart_at);
        let before = restart_at + CONVERGENCE_WINDOW - Duration::from_millis(1);
        state
            .record_activity(&p2, p2_incarnation, before, Utc::now())
            .unwrap();
        state.tick(before, Utc::now());
        assert!(!state.is_ready());
        let deadline = restart_at + CONVERGENCE_WINDOW;
        state
            .record_activity(&p2, p2_incarnation, deadline, Utc::now())
            .unwrap();
        state.tick(deadline, Utc::now());
        assert!(state.is_ready());
    }

    #[test]
    fn joining_rows_publish_atomically_without_a_global_outage() {
        let now = Instant::now();
        let mut state = ControllerState::new(100);
        let winner = Uuid::from_u128(1);
        let loser = Uuid::from_u128(2);
        let (_p1, _p1_incarnation, _) =
            ready_one(&mut state, "p1", vec![row("alice", "one", winner)], now);
        let (p2, p2_incarnation) = begin(&mut state, "p2", now + CONVERGENCE_WINDOW);
        let effects = snapshot(
            &mut state,
            &p2,
            p2_incarnation,
            vec![
                row("alice", "one", loser),
                row("alice", "two", Uuid::new_v4()),
            ],
            now + CONVERGENCE_WINDOW,
        );
        assert!(state.is_ready());
        assert_eq!(state.tunnel_views().len(), 1);
        assert_eq!(state.proxy_views()[1].status, ProxyStatus::Joining);
        let command_id = kill_command(&effects, "p2", loser);
        let ready = state
            .command_result(
                &p2,
                p2_incarnation,
                command_id,
                vec![loser],
                Vec::new(),
                Vec::new(),
                now + CONVERGENCE_WINDOW,
                Utc::now(),
            )
            .unwrap();
        assert!(ready.iter().any(|effect| matches!(
            effect,
            Effect::Send {
                session,
                frame: ServerFrame::FleetReady,
            } if session.proxy_id == "p2"
        )));
        assert_eq!(state.tunnel_views().len(), 2);
        assert_eq!(state.proxy_views()[1].status, ProxyStatus::Active);
    }

    #[test]
    fn delta_during_joining_reconciliation_is_not_lost() {
        let now = Instant::now();
        let mut state = ControllerState::new(100);
        let winner = Uuid::from_u128(1);
        let loser = Uuid::from_u128(2);
        let (p1, p1_incarnation, _) =
            ready_one(&mut state, "p1", vec![row("alice", "one", winner)], now);
        let (p2, p2_incarnation) = begin(&mut state, "p2", now + CONVERGENCE_WINDOW);
        let effects = snapshot(
            &mut state,
            &p2,
            p2_incarnation,
            vec![row("alice", "one", loser)],
            now + CONVERGENCE_WINDOW,
        );
        let command_id = kill_command(&effects, "p2", loser);

        let second = Uuid::new_v4();
        state
            .request_admission(
                &p1,
                p1_incarnation,
                Uuid::new_v4(),
                second,
                "alice".into(),
                "two".into(),
                now + CONVERGENCE_WINDOW,
                Utc::now(),
            )
            .unwrap();
        state
            .tunnel_up(
                &p1,
                p1_incarnation,
                1,
                row("alice", "two", second),
                now + CONVERGENCE_WINDOW,
                Utc::now(),
            )
            .unwrap();
        state
            .command_result(
                &p2,
                p2_incarnation,
                command_id,
                vec![loser],
                Vec::new(),
                Vec::new(),
                now + CONVERGENCE_WINDOW,
                Utc::now(),
            )
            .unwrap();
        let ids: Vec<_> = state
            .tunnel_views()
            .into_iter()
            .map(|view| view.devserver_id)
            .collect();
        assert_eq!(ids, ["one", "two"]);
    }

    #[test]
    fn duplicate_restart_rows_wait_for_successful_command_result() {
        let now = Instant::now();
        let mut state = ControllerState::new(100);
        let winner_id = Uuid::from_u128(1);
        let loser_id = Uuid::from_u128(2);
        let (p2, p2_incarnation) = begin(&mut state, "p2", now);
        snapshot(
            &mut state,
            &p2,
            p2_incarnation,
            vec![row("alice", "one", loser_id)],
            now,
        );
        let (p1, p1_incarnation) = begin(&mut state, "p1", now);
        snapshot(
            &mut state,
            &p1,
            p1_incarnation,
            vec![row("alice", "one", winner_id)],
            now,
        );
        for (proxy, incarnation) in [(&p1, p1_incarnation), (&p2, p2_incarnation)] {
            state
                .record_activity(proxy, incarnation, now + CONVERGENCE_WINDOW, Utc::now())
                .unwrap();
        }
        let effects = state.tick(now + CONVERGENCE_WINDOW, Utc::now());
        assert!(!state.is_ready());
        let command_id = effects
            .iter()
            .find_map(|effect| match effect {
                Effect::Send {
                    session,
                    frame:
                        ServerFrame::KillRegistrations {
                            command_id,
                            registration_ids,
                        },
                } if session.proxy_id == "p2" && registration_ids == &[loser_id] => {
                    Some(*command_id)
                }
                _ => None,
            })
            .expect("loser command");
        let ready = state
            .command_result(
                &p2,
                p2_incarnation,
                command_id,
                vec![loser_id],
                Vec::new(),
                Vec::new(),
                now + CONVERGENCE_WINDOW,
                Utc::now(),
            )
            .unwrap();
        assert!(state.is_ready());
        assert_eq!(state.tunnel_views()[0].proxy_id, "p1");
        assert!(
            ready
                .iter()
                .filter(|effect| matches!(
                    effect,
                    Effect::Send {
                        frame: ServerFrame::FleetReady,
                        ..
                    }
                ))
                .count()
                == 2
        );
    }

    #[test]
    fn failed_restart_reconciliation_retries_after_a_new_window() {
        let now = Instant::now();
        let mut state = ControllerState::new(100);
        let duplicate = row("alice", "one", Uuid::from_u128(2));
        let (p2, p2_incarnation) = begin(&mut state, "p2", now);
        snapshot(&mut state, &p2, p2_incarnation, vec![duplicate], now);
        let (p1, p1_incarnation) = begin(&mut state, "p1", now);
        snapshot(
            &mut state,
            &p1,
            p1_incarnation,
            vec![row("alice", "one", Uuid::from_u128(1))],
            now,
        );
        for (proxy, incarnation) in [(&p1, p1_incarnation), (&p2, p2_incarnation)] {
            state
                .record_activity(proxy, incarnation, now + CONVERGENCE_WINDOW, Utc::now())
                .unwrap();
        }
        let effects = state.tick(now + CONVERGENCE_WINDOW, Utc::now());
        let (command_id, loser) = effects
            .iter()
            .find_map(|effect| match effect {
                Effect::Send {
                    frame:
                        ServerFrame::KillRegistrations {
                            command_id,
                            registration_ids,
                        },
                    ..
                } => Some((*command_id, registration_ids[0])),
                _ => None,
            })
            .unwrap();
        state
            .command_result(
                &p2,
                p2_incarnation,
                command_id,
                Vec::new(),
                Vec::new(),
                vec![loser],
                now + CONVERGENCE_WINDOW,
                Utc::now(),
            )
            .unwrap();
        assert!(!state.is_ready());
        assert_eq!(state.read_tunnels(), Err(StateError::NotReady));

        let retry_at = now + CONVERGENCE_WINDOW * 2;
        for (proxy, incarnation) in [(&p1, p1_incarnation), (&p2, p2_incarnation)] {
            state
                .record_activity(proxy, incarnation, retry_at, Utc::now())
                .unwrap();
        }
        let retry = state.tick(retry_at, Utc::now());
        let retry_command = kill_command(&retry, "p2", loser);
        state
            .command_result(
                &p2,
                p2_incarnation,
                retry_command,
                vec![loser],
                Vec::new(),
                Vec::new(),
                retry_at,
                Utc::now(),
            )
            .unwrap();
        assert!(state.is_ready());
        assert_eq!(state.tunnel_views()[0].proxy_id, "p1");
    }

    #[test]
    fn reconciliation_command_timeout_retries_instead_of_wedging() {
        let now = Instant::now();
        let mut state = ControllerState::new(100);
        let loser = Uuid::from_u128(2);
        let (p2, p2_incarnation) = begin(&mut state, "p2", now);
        snapshot(
            &mut state,
            &p2,
            p2_incarnation,
            vec![row("alice", "one", loser)],
            now,
        );
        let (p1, p1_incarnation) = begin(&mut state, "p1", now);
        snapshot(
            &mut state,
            &p1,
            p1_incarnation,
            vec![row("alice", "one", Uuid::from_u128(1))],
            now,
        );
        let convergence_at = now + CONVERGENCE_WINDOW;
        for (proxy, incarnation) in [(&p1, p1_incarnation), (&p2, p2_incarnation)] {
            state
                .record_activity(proxy, incarnation, convergence_at, Utc::now())
                .unwrap();
        }
        let first = state.tick(convergence_at, Utc::now());
        let expired_command = kill_command(&first, "p2", loser);
        assert!(state.commands.contains_key(&expired_command));

        let timeout_at = convergence_at + COMMAND_TIMEOUT;
        for (proxy, incarnation) in [(&p1, p1_incarnation), (&p2, p2_incarnation)] {
            state
                .record_activity(proxy, incarnation, timeout_at, Utc::now())
                .unwrap();
        }
        state.tick(timeout_at, Utc::now());
        assert!(!state.is_ready());
        assert!(state.commands.is_empty());
        assert!(state.reconciliation.is_none());

        let retry_at = timeout_at + CONVERGENCE_WINDOW;
        for (proxy, incarnation) in [(&p1, p1_incarnation), (&p2, p2_incarnation)] {
            state
                .record_activity(proxy, incarnation, retry_at, Utc::now())
                .unwrap();
        }
        let retry = state.tick(retry_at, Utc::now());
        let retry_command = kill_command(&retry, "p2", loser);
        assert_ne!(retry_command, expired_command);
    }

    #[test]
    fn runtime_command_timeout_releases_pending_state() {
        let now = Instant::now();
        let mut state = ControllerState::new(100);
        let (proxy, incarnation, _) = ready_one(&mut state, "p1", Vec::new(), now);
        let registration_id = Uuid::new_v4();
        let active_at = now + CONVERGENCE_WINDOW;
        let command = state
            .tunnel_up(
                &proxy,
                incarnation,
                1,
                row("alice", "unclaimed", registration_id),
                active_at,
                Utc::now(),
            )
            .unwrap();
        let command_id = kill_command(&command, "p1", registration_id);
        assert!(state.commands.contains_key(&command_id));

        let timeout_at = active_at + COMMAND_TIMEOUT;
        state
            .record_activity(&proxy, incarnation, timeout_at, Utc::now())
            .unwrap();
        state.tick(timeout_at, Utc::now());
        assert!(state.commands.is_empty());
        assert!(state.is_ready());
    }

    #[test]
    fn joining_snapshot_never_evicts_a_live_row() {
        let now = Instant::now();
        let mut state = ControllerState::new(100);
        let live_id = Uuid::from_u128(1);
        let stale_id = Uuid::from_u128(2);
        let (_p2, _p2_incarnation, _) =
            ready_one(&mut state, "p2", vec![row("alice", "one", live_id)], now);
        // p0 sorts before p2, so the restart tie-break would crown the stale
        // snapshot; a routine join must kill the joining row instead.
        let (p0, p0_incarnation) = begin(&mut state, "p0", now + CONVERGENCE_WINDOW);
        let effects = snapshot(
            &mut state,
            &p0,
            p0_incarnation,
            vec![row("alice", "one", stale_id)],
            now + CONVERGENCE_WINDOW,
        );
        let command_id = kill_command(&effects, "p0", stale_id);
        assert!(effects.iter().all(|effect| !matches!(
            effect,
            Effect::Send {
                session,
                frame: ServerFrame::KillRegistrations { .. },
            } if session.proxy_id == "p2"
        )));
        assert_eq!(state.tunnel_views().len(), 1);
        assert_eq!(state.tunnel_views()[0].proxy_id, "p2");
        let p0_view = state
            .proxy_views()
            .into_iter()
            .find(|view| view.proxy_id == "p0")
            .unwrap();
        assert_eq!(p0_view.status, ProxyStatus::Joining);

        let ready = state
            .command_result(
                &p0,
                p0_incarnation,
                command_id,
                vec![stale_id],
                Vec::new(),
                Vec::new(),
                now + CONVERGENCE_WINDOW,
                Utc::now(),
            )
            .unwrap();
        assert!(ready.iter().any(|effect| matches!(
            effect,
            Effect::Send {
                session,
                frame: ServerFrame::FleetReady,
            } if session.proxy_id == "p0"
        )));
        assert_eq!(state.tunnel_views().len(), 1);
        assert_eq!(state.tunnel_views()[0].proxy_id, "p2");
        let p0_view = state
            .proxy_views()
            .into_iter()
            .find(|view| view.proxy_id == "p0")
            .unwrap();
        assert_eq!(p0_view.status, ProxyStatus::Active);
    }

    #[test]
    fn joining_row_that_exceeds_live_capacity_loses() {
        let now = Instant::now();
        let mut state = ControllerState::new(1);
        let (_p2, _p2_incarnation, _) = ready_one(
            &mut state,
            "p2",
            vec![row("alice", "two", Uuid::from_u128(1))],
            now,
        );
        // "one" sorts before "two", so the restart capacity trim would evict
        // the live row; the live-first rule reserves alice's slot for it.
        let (p0, p0_incarnation) = begin(&mut state, "p0", now + CONVERGENCE_WINDOW);
        let joining_id = Uuid::from_u128(2);
        let effects = snapshot(
            &mut state,
            &p0,
            p0_incarnation,
            vec![row("alice", "one", joining_id)],
            now + CONVERGENCE_WINDOW,
        );
        let command_id = kill_command(&effects, "p0", joining_id);
        assert!(effects.iter().all(|effect| !matches!(
            effect,
            Effect::Send {
                session,
                frame: ServerFrame::KillRegistrations { .. },
            } if session.proxy_id == "p2"
        )));
        assert_eq!(state.tunnel_views().len(), 1);
        assert_eq!(state.tunnel_views()[0].devserver_id, "two");
        assert_eq!(state.tunnel_views()[0].proxy_id, "p2");

        state
            .command_result(
                &p0,
                p0_incarnation,
                command_id,
                vec![joining_id],
                Vec::new(),
                Vec::new(),
                now + CONVERGENCE_WINDOW,
                Utc::now(),
            )
            .unwrap();
        assert_eq!(state.tunnel_views().len(), 1);
        assert_eq!(state.tunnel_views()[0].devserver_id, "two");
        assert_eq!(state.tunnel_views()[0].proxy_id, "p2");
    }

    #[test]
    fn failed_joining_reconciliation_retires_only_the_joining_session() {
        let now = Instant::now();
        let mut state = ControllerState::new(100);
        let (_p2, _p2_incarnation, _) = ready_one(
            &mut state,
            "p2",
            vec![row("alice", "one", Uuid::from_u128(1))],
            now,
        );
        let (p0, p0_incarnation) = begin(&mut state, "p0", now + CONVERGENCE_WINDOW);
        let stale_id = Uuid::from_u128(2);
        let effects = snapshot(
            &mut state,
            &p0,
            p0_incarnation,
            vec![row("alice", "one", stale_id)],
            now + CONVERGENCE_WINDOW,
        );
        let command_id = kill_command(&effects, "p0", stale_id);
        let retired = state
            .command_result(
                &p0,
                p0_incarnation,
                command_id,
                Vec::new(),
                Vec::new(),
                vec![stale_id],
                now + CONVERGENCE_WINDOW,
                Utc::now(),
            )
            .unwrap();
        assert!(retired.iter().any(|effect| matches!(
            effect,
            Effect::Retire { session, .. } if session.proxy_id == "p0"
        )));
        assert!(state.is_ready());
        assert_eq!(state.proxy_views().len(), 1);
        assert_eq!(state.proxy_views()[0].proxy_id, "p2");
        assert_eq!(state.tunnel_views().len(), 1);
        assert_eq!(state.tunnel_views()[0].proxy_id, "p2");
    }

    #[test]
    fn joining_snapshot_duplicate_keys_pick_one_stable_winner() {
        let now = Instant::now();
        let mut state = ControllerState::new(100);
        let (_p1, _p1_incarnation, _) = ready_one(&mut state, "p1", Vec::new(), now);
        let (p0, p0_incarnation) = begin(&mut state, "p0", now + CONVERGENCE_WINDOW);
        let winner = Uuid::from_u128(1);
        let loser = Uuid::from_u128(2);
        let effects = snapshot(
            &mut state,
            &p0,
            p0_incarnation,
            vec![row("alice", "one", loser), row("alice", "one", winner)],
            now + CONVERGENCE_WINDOW,
        );
        let command_id = kill_command(&effects, "p0", loser);
        state
            .command_result(
                &p0,
                p0_incarnation,
                command_id,
                vec![loser],
                Vec::new(),
                Vec::new(),
                now + CONVERGENCE_WINDOW,
                Utc::now(),
            )
            .unwrap();
        assert_eq!(state.tunnel_views().len(), 1);
        assert_eq!(state.tunnel_views()[0].proxy_id, "p0");
        assert_eq!(state.tunnel_views()[0].devserver_id, "one");
    }

    #[test]
    fn live_delta_during_joining_completion_is_not_re_ranked() {
        let now = Instant::now();
        let mut state = ControllerState::new(100);
        let live_one = Uuid::from_u128(1);
        let dup = Uuid::from_u128(2);
        let joining_two = Uuid::from_u128(3);
        let live_two = Uuid::from_u128(4);
        let (p2, p2_incarnation, _) =
            ready_one(&mut state, "p2", vec![row("alice", "one", live_one)], now);
        let (p0, p0_incarnation) = begin(&mut state, "p0", now + CONVERGENCE_WINDOW);
        let effects = snapshot(
            &mut state,
            &p0,
            p0_incarnation,
            vec![row("alice", "one", dup), row("alice", "two", joining_two)],
            now + CONVERGENCE_WINDOW,
        );
        let first_command = kill_command(&effects, "p0", dup);

        // A live admission lands while the joining kill is outstanding; the
        // completion re-plan must treat it as an immutable winner too.
        state
            .request_admission(
                &p2,
                p2_incarnation,
                Uuid::new_v4(),
                live_two,
                "alice".into(),
                "two".into(),
                now + CONVERGENCE_WINDOW,
                Utc::now(),
            )
            .unwrap();
        state
            .tunnel_up(
                &p2,
                p2_incarnation,
                1,
                row("alice", "two", live_two),
                now + CONVERGENCE_WINDOW,
                Utc::now(),
            )
            .unwrap();

        let second = state
            .command_result(
                &p0,
                p0_incarnation,
                first_command,
                vec![dup],
                Vec::new(),
                Vec::new(),
                now + CONVERGENCE_WINDOW,
                Utc::now(),
            )
            .unwrap();
        let second_command = kill_command(&second, "p0", joining_two);
        assert!(second.iter().all(|effect| !matches!(
            effect,
            Effect::Send {
                session,
                frame: ServerFrame::KillRegistrations { .. },
            } if session.proxy_id == "p2"
        )));
        let done = state
            .command_result(
                &p0,
                p0_incarnation,
                second_command,
                vec![joining_two],
                Vec::new(),
                Vec::new(),
                now + CONVERGENCE_WINDOW,
                Utc::now(),
            )
            .unwrap();
        assert!(done.iter().any(|effect| matches!(
            effect,
            Effect::Send {
                session,
                frame: ServerFrame::FleetReady,
            } if session.proxy_id == "p0"
        )));
        let owners: Vec<_> = state
            .tunnel_views()
            .into_iter()
            .map(|view| (view.devserver_id, view.proxy_id))
            .collect();
        assert_eq!(
            owners,
            [
                ("one".to_string(), "p2".to_string()),
                ("two".to_string(), "p2".to_string()),
            ]
        );
    }

    #[test]
    fn initial_reconciliation_keeps_the_lexicographic_rule() {
        let now = Instant::now();
        let mut state = ControllerState::new(100);
        let low = Uuid::from_u128(1);
        let high = Uuid::from_u128(2);
        let (p2, p2_incarnation) = begin(&mut state, "p2", now);
        snapshot(
            &mut state,
            &p2,
            p2_incarnation,
            vec![row("alice", "one", low)],
            now,
        );
        let (p1, p1_incarnation) = begin(&mut state, "p1", now);
        snapshot(
            &mut state,
            &p1,
            p1_incarnation,
            vec![row("alice", "one", high)],
            now,
        );
        for (proxy, incarnation) in [(&p1, p1_incarnation), (&p2, p2_incarnation)] {
            state
                .record_activity(proxy, incarnation, now + CONVERGENCE_WINDOW, Utc::now())
                .unwrap();
        }
        let effects = state.tick(now + CONVERGENCE_WINDOW, Utc::now());
        // Recency is unavailable at restart, so the proxy id dominates the
        // registration id: p1 wins even though p2 holds the lower uuid.
        let command_id = kill_command(&effects, "p2", low);
        state
            .command_result(
                &p2,
                p2_incarnation,
                command_id,
                vec![low],
                Vec::new(),
                Vec::new(),
                now + CONVERGENCE_WINDOW,
                Utc::now(),
            )
            .unwrap();
        assert!(state.is_ready());
        assert_eq!(state.tunnel_views().len(), 1);
        assert_eq!(state.tunnel_views()[0].proxy_id, "p1");
    }

    #[test]
    fn user_kill_cancels_pending_claims_before_commanding() {
        let now = Instant::now();
        let mut state = ControllerState::new(100);
        let registration_id = Uuid::new_v4();
        let (id, incarnation, _) = ready_one(
            &mut state,
            "p1",
            vec![row("alice", "one", registration_id)],
            now,
        );
        let active_at = now + CONVERGENCE_WINDOW;
        let claimed_registration = Uuid::new_v4();
        let admit = state
            .request_admission(
                &id,
                incarnation,
                Uuid::new_v4(),
                claimed_registration,
                "alice".into(),
                "two".into(),
                active_at,
                Utc::now(),
            )
            .unwrap();
        assert!(has_decision(&admit, AdmissionDecision::Admit));
        assert_eq!(state.pending.len(), 1);

        let (command_ids, effects) = state.begin_user_kill("alice", active_at).unwrap();
        assert!(state.pending.is_empty());
        let command_id = kill_command(&effects, "p1", registration_id);
        assert_eq!(command_ids, vec![command_id]);

        // The cancelled claim no longer activates its registration: a late
        // TunnelUp takes the unclaimed path and is killed outright.
        let late = state
            .tunnel_up(
                &id,
                incarnation,
                1,
                row("alice", "two", claimed_registration),
                active_at,
                Utc::now(),
            )
            .unwrap();
        kill_command(&late, "p1", claimed_registration);
        assert_eq!(state.tunnel_views().len(), 1);
    }

    #[test]
    fn exact_kill_on_unknown_key_issues_nothing() {
        let now = Instant::now();
        let mut state = ControllerState::new(100);
        let (_id, _incarnation, _) = ready_one(
            &mut state,
            "p1",
            vec![row("alice", "one", Uuid::new_v4())],
            now,
        );
        let active_at = now + CONVERGENCE_WINDOW;
        for (user, devserver_id) in [("alice", "two"), ("bob", "one")] {
            let (command_id, effects) = state
                .begin_exact_kill(user, devserver_id, active_at)
                .unwrap();
            assert!(command_id.is_none());
            assert!(effects.is_empty());
        }
        assert!(state.commands.is_empty());
    }

    #[test]
    fn kills_require_readiness() {
        let now = Instant::now();
        let mut state = ControllerState::new(100);
        let (id, incarnation) = begin(&mut state, "p1", now);
        snapshot(
            &mut state,
            &id,
            incarnation,
            vec![row("alice", "one", Uuid::new_v4())],
            now,
        );
        assert!(matches!(
            state.begin_exact_kill("alice", "one", now),
            Err(StateError::NotReady)
        ));
        assert!(matches!(
            state.begin_user_kill("alice", now),
            Err(StateError::NotReady)
        ));
    }

    #[test]
    fn runtime_command_settles_confirmed_on_result() {
        let now = Instant::now();
        let mut state = ControllerState::new(100);
        let registration_id = Uuid::new_v4();
        let (id, incarnation, _) = ready_one(
            &mut state,
            "p1",
            vec![row("alice", "one", registration_id)],
            now,
        );
        let active_at = now + CONVERGENCE_WINDOW;
        let (command_id, _) = state.begin_exact_kill("alice", "one", active_at).unwrap();
        let command_id = command_id.expect("row exists");
        let effects = state
            .command_result(
                &id,
                incarnation,
                command_id,
                vec![registration_id],
                Vec::new(),
                Vec::new(),
                active_at,
                Utc::now(),
            )
            .unwrap();
        assert!(effects.iter().any(|effect| matches!(
            effect,
            Effect::CommandSettled {
                command_id: settled,
                outcome: CommandOutcome::Confirmed {
                    killed: 1,
                    missing: 0
                },
            } if *settled == command_id
        )));
        assert!(state.tunnel_views().is_empty());
    }

    #[test]
    fn runtime_command_settles_failed_on_reported_failure() {
        let now = Instant::now();
        let mut state = ControllerState::new(100);
        let registration_id = Uuid::new_v4();
        let (id, incarnation, _) = ready_one(
            &mut state,
            "p1",
            vec![row("alice", "one", registration_id)],
            now,
        );
        let active_at = now + CONVERGENCE_WINDOW;
        let (command_id, _) = state.begin_exact_kill("alice", "one", active_at).unwrap();
        let command_id = command_id.expect("row exists");
        let effects = state
            .command_result(
                &id,
                incarnation,
                command_id,
                Vec::new(),
                Vec::new(),
                vec![registration_id],
                active_at,
                Utc::now(),
            )
            .unwrap();
        assert!(effects.iter().any(|effect| matches!(
            effect,
            Effect::CommandSettled {
                command_id: settled,
                outcome: CommandOutcome::Failed,
            } if *settled == command_id
        )));
    }

    #[test]
    fn runtime_command_settles_timed_out_on_expiry() {
        let now = Instant::now();
        let mut state = ControllerState::new(100);
        let registration_id = Uuid::new_v4();
        let (id, incarnation, _) = ready_one(
            &mut state,
            "p1",
            vec![row("alice", "one", registration_id)],
            now,
        );
        let active_at = now + CONVERGENCE_WINDOW;
        let (command_id, _) = state.begin_exact_kill("alice", "one", active_at).unwrap();
        let command_id = command_id.expect("row exists");
        let timeout_at = active_at + COMMAND_TIMEOUT;
        state
            .record_activity(&id, incarnation, timeout_at, Utc::now())
            .unwrap();
        let effects = state.tick(timeout_at, Utc::now());
        assert!(effects.iter().any(|effect| matches!(
            effect,
            Effect::CommandSettled {
                command_id: settled,
                outcome: CommandOutcome::TimedOut,
            } if *settled == command_id
        )));
        assert!(state.commands.is_empty());
    }

    #[test]
    fn runtime_command_settles_session_lost_on_session_death() {
        let now = Instant::now();
        let mut state = ControllerState::new(100);
        let registration_id = Uuid::new_v4();
        let (id, incarnation, _) = ready_one(
            &mut state,
            "p1",
            vec![row("alice", "one", registration_id)],
            now,
        );
        let active_at = now + CONVERGENCE_WINDOW;
        let (command_id, _) = state.begin_exact_kill("alice", "one", active_at).unwrap();
        let command_id = command_id.expect("row exists");
        let effects = state.disconnect(&id, incarnation, active_at).unwrap();
        assert!(effects.iter().any(|effect| matches!(
            effect,
            Effect::CommandSettled {
                command_id: settled,
                outcome: CommandOutcome::SessionLost,
            } if *settled == command_id
        )));
        assert!(state.commands.is_empty());
    }

    #[test]
    fn user_kill_with_no_rows_issues_nothing() {
        let now = Instant::now();
        let mut state = ControllerState::new(100);
        let (_id, _incarnation, _) = ready_one(&mut state, "p1", Vec::new(), now);
        let (command_ids, effects) = state
            .begin_user_kill("alice", now + CONVERGENCE_WINDOW)
            .unwrap();
        assert!(command_ids.is_empty());
        assert!(effects.is_empty());
    }

    #[test]
    fn user_kill_groups_registrations_by_owning_session() {
        let now = Instant::now();
        let mut state = ControllerState::new(100);
        let first = Uuid::new_v4();
        let second = Uuid::new_v4();
        let bob_registration = Uuid::new_v4();
        let (p1, p1_incarnation) = begin(&mut state, "p1", now);
        snapshot(
            &mut state,
            &p1,
            p1_incarnation,
            vec![
                row("alice", "one", first),
                row("alice", "two", second),
                row("bob", "one", bob_registration),
            ],
            now,
        );
        let (p2, p2_incarnation) = begin(&mut state, "p2", now);
        let third = Uuid::new_v4();
        snapshot(
            &mut state,
            &p2,
            p2_incarnation,
            vec![row("alice", "three", third)],
            now,
        );
        for (proxy, incarnation) in [(&p1, p1_incarnation), (&p2, p2_incarnation)] {
            state
                .record_activity(proxy, incarnation, now + CONVERGENCE_WINDOW, Utc::now())
                .unwrap();
        }
        state.tick(now + CONVERGENCE_WINDOW, Utc::now());
        assert!(state.is_ready());

        let (command_ids, effects) = state
            .begin_user_kill("alice", now + CONVERGENCE_WINDOW)
            .unwrap();
        assert_eq!(command_ids.len(), 2);
        let mut per_session: HashMap<String, Vec<Uuid>> = HashMap::new();
        for effect in &effects {
            let Effect::Send {
                session,
                frame:
                    ServerFrame::KillRegistrations {
                        registration_ids, ..
                    },
            } = effect
            else {
                continue;
            };
            per_session.insert(session.proxy_id.clone(), registration_ids.clone());
        }
        assert_eq!(per_session.len(), 2);
        let p1_ids = per_session.get("p1").expect("p1 command");
        assert_eq!(p1_ids.len(), 2);
        assert!(p1_ids.contains(&first));
        assert!(p1_ids.contains(&second));
        assert_eq!(per_session.get("p2").expect("p2 command"), &vec![third]);
        // bob's row is never targeted.
        let targeted: HashSet<Uuid> = per_session.values().flatten().copied().collect();
        assert!(!targeted.contains(&bob_registration));
        assert_eq!(state.tunnel_views().len(), 4);
    }

    fn kill_command(effects: &[Effect], proxy_id: &str, registration_id: Uuid) -> Uuid {
        effects
            .iter()
            .find_map(|effect| match effect {
                Effect::Send {
                    session,
                    frame:
                        ServerFrame::KillRegistrations {
                            command_id,
                            registration_ids,
                        },
                } if session.proxy_id == proxy_id && registration_ids == &[registration_id] => {
                    Some(*command_id)
                }
                _ => None,
            })
            .expect("targeted kill command")
    }

    fn has_resync(effects: &[Effect], expected_generation: u64) -> bool {
        effects.iter().any(|effect| {
            matches!(
                effect,
                Effect::Send {
                    frame: ServerFrame::ResyncRequired { expected_generation: actual },
                    ..
                } if *actual == expected_generation
            )
        })
    }

    fn has_decision(effects: &[Effect], expected: AdmissionDecision) -> bool {
        effects.iter().any(|effect| {
            matches!(
                effect,
                Effect::Send {
                    frame: ServerFrame::AdmissionDecision { decision, .. },
                    ..
                } if *decision == expected
            )
        })
    }
}
