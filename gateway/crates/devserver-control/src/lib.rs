#![forbid(unsafe_code)]

use std::collections::{BTreeMap, HashMap, HashSet};

use devserver_control_proto::{AdmissionDecision, CanonicalOrigin, ProxyId, TunnelRow};
use serde::Serialize;
use uuid::Uuid;

mod actor;
mod config;

pub use actor::{spawn_controller, ControllerHandle};
pub use config::Config;

type TunnelKey = (String, String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProxyStatus {
    Joining,
    Active,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProxyView {
    pub proxy_id: String,
    pub proxy_base_url: String,
    pub package_version: String,
    pub boot_id: Uuid,
    pub tunnel_count: usize,
    pub status: ProxyStatus,
}

#[derive(Debug, Clone, Serialize)]
pub struct TunnelView {
    pub user: String,
    pub devserver_id: String,
    pub peer_addr: Option<std::net::SocketAddr>,
    pub connected_at: chrono::DateTime<chrono::Utc>,
    pub proxy_id: String,
    pub proxy_base_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KillTarget {
    pub proxy_id: ProxyId,
    pub registration_id: Uuid,
}

#[derive(Debug, Clone)]
struct OwnedTunnel {
    proxy_id: ProxyId,
    row: TunnelRow,
}

#[derive(Debug, Clone)]
struct ProxySession {
    base_url: CanonicalOrigin,
    package_version: String,
    boot_id: Uuid,
    generation: u64,
    registrations: HashSet<Uuid>,
    status: ProxyStatus,
}

#[derive(Debug, Clone)]
struct PendingClaim {
    proxy_id: ProxyId,
    request_id: Uuid,
    registration_id: Uuid,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum StateError {
    #[error("proxy session is not active")]
    ProxyNotActive,
    #[error("registry generation mismatch: expected {expected}, got {got}")]
    Generation { expected: u64, got: u64 },
    #[error("snapshot contains duplicate registration id {0}")]
    DuplicateRegistration(Uuid),
}

pub struct ControllerState {
    max_devservers_per_user: usize,
    ready: bool,
    proxies: BTreeMap<String, ProxySession>,
    tunnels: HashMap<TunnelKey, OwnedTunnel>,
    by_registration: HashMap<Uuid, TunnelKey>,
    pending: HashMap<TunnelKey, PendingClaim>,
}

impl ControllerState {
    pub fn new(max_devservers_per_user: usize) -> Self {
        Self {
            max_devservers_per_user,
            ready: false,
            proxies: BTreeMap::new(),
            tunnels: HashMap::new(),
            by_registration: HashMap::new(),
            pending: HashMap::new(),
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
    ) {
        self.disconnect(proxy_id.as_str());
        self.proxies.insert(
            proxy_id.as_str().to_string(),
            ProxySession {
                base_url,
                package_version,
                boot_id,
                generation: 0,
                registrations: HashSet::new(),
                status: ProxyStatus::Joining,
            },
        );
    }

    pub fn accept_snapshot(
        &mut self,
        proxy_id: &ProxyId,
        base_generation: u64,
        rows: Vec<TunnelRow>,
    ) -> Result<Vec<KillTarget>, StateError> {
        let mut ids = HashSet::with_capacity(rows.len());
        for row in &rows {
            if !ids.insert(row.registration_id) {
                return Err(StateError::DuplicateRegistration(row.registration_id));
            }
        }
        let session = self
            .proxies
            .get_mut(proxy_id.as_str())
            .ok_or(StateError::ProxyNotActive)?;
        if session.status != ProxyStatus::Joining {
            return Err(StateError::ProxyNotActive);
        }
        session.generation = base_generation;
        session.registrations = ids;
        session.status = ProxyStatus::Active;

        let mut kills = Vec::new();
        for row in rows {
            self.install_authoritative(proxy_id.clone(), row, &mut kills);
        }
        self.ready = !self.proxies.is_empty()
            && self
                .proxies
                .values()
                .any(|session| session.status == ProxyStatus::Active);
        Ok(kills)
    }

    pub fn tunnel_up(
        &mut self,
        proxy_id: &ProxyId,
        generation: u64,
        row: TunnelRow,
    ) -> Result<Vec<KillTarget>, StateError> {
        self.advance(proxy_id, generation)?;
        let key = (row.user.clone(), row.devserver_id.clone());
        if let Some(pending) = self.pending.get(&key) {
            if pending.proxy_id != *proxy_id || pending.registration_id != row.registration_id {
                return Ok(vec![KillTarget {
                    proxy_id: proxy_id.clone(),
                    registration_id: row.registration_id,
                }]);
            }
        }
        self.pending.remove(&key);
        let mut kills = Vec::new();
        self.install_authoritative(proxy_id.clone(), row, &mut kills);
        Ok(kills)
    }

    pub fn tunnel_down(
        &mut self,
        proxy_id: &ProxyId,
        generation: u64,
        registration_id: Uuid,
    ) -> Result<(), StateError> {
        self.advance(proxy_id, generation)?;
        self.remove_registration(registration_id);
        Ok(())
    }

    pub fn request_admission(
        &mut self,
        proxy_id: &ProxyId,
        request_id: Uuid,
        registration_id: Uuid,
        user: String,
        devserver_id: String,
    ) -> AdmissionDecision {
        if !self.ready
            || !self
                .proxies
                .get(proxy_id.as_str())
                .is_some_and(|session| session.status == ProxyStatus::Active)
        {
            return AdmissionDecision::ControlWarming;
        }
        let key = (user.clone(), devserver_id);
        let reconnect = self.tunnels.contains_key(&key) || self.pending.contains_key(&key);
        let used = self.distinct_for_user(&user);
        if self.max_devservers_per_user > 0 && !reconnect && used >= self.max_devservers_per_user {
            return AdmissionDecision::AtCapacity;
        }
        self.pending.insert(
            key,
            PendingClaim {
                proxy_id: proxy_id.clone(),
                request_id,
                registration_id,
            },
        );
        AdmissionDecision::Admit
    }

    pub fn cancel_admission(&mut self, request_id: Uuid, registration_id: Uuid) {
        self.pending.retain(|_, claim| {
            claim.request_id != request_id || claim.registration_id != registration_id
        });
    }

    pub fn disconnect(&mut self, proxy_id: &str) {
        let Some(session) = self.proxies.remove(proxy_id) else {
            return;
        };
        for registration_id in session.registrations {
            self.remove_registration(registration_id);
        }
        self.pending
            .retain(|_, pending| pending.proxy_id.as_str() != proxy_id);
        if !self
            .proxies
            .values()
            .any(|session| session.status == ProxyStatus::Active)
        {
            self.ready = false;
        }
    }

    pub fn tunnel_views(&self) -> Vec<TunnelView> {
        let mut out: Vec<_> = self
            .tunnels
            .values()
            .filter_map(|owned| {
                let session = self.proxies.get(owned.proxy_id.as_str())?;
                Some(TunnelView {
                    user: owned.row.user.clone(),
                    devserver_id: owned.row.devserver_id.clone(),
                    peer_addr: owned.row.peer_addr,
                    connected_at: owned.row.connected_at,
                    proxy_id: owned.proxy_id.as_str().to_string(),
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

    pub fn proxy_views(&self) -> Vec<ProxyView> {
        self.proxies
            .iter()
            .map(|(proxy_id, session)| ProxyView {
                proxy_id: proxy_id.clone(),
                proxy_base_url: session.base_url.as_str().to_string(),
                package_version: session.package_version.clone(),
                boot_id: session.boot_id,
                tunnel_count: session.registrations.len(),
                status: session.status,
            })
            .collect()
    }

    fn advance(&mut self, proxy_id: &ProxyId, generation: u64) -> Result<(), StateError> {
        let session = self
            .proxies
            .get_mut(proxy_id.as_str())
            .filter(|session| session.status == ProxyStatus::Active)
            .ok_or(StateError::ProxyNotActive)?;
        let expected = session.generation + 1;
        if generation != expected {
            return Err(StateError::Generation {
                expected,
                got: generation,
            });
        }
        session.generation = generation;
        Ok(())
    }

    fn install_authoritative(
        &mut self,
        proxy_id: ProxyId,
        row: TunnelRow,
        kills: &mut Vec<KillTarget>,
    ) {
        let key = (row.user.clone(), row.devserver_id.clone());
        if let Some(old) = self.tunnels.get(&key) {
            let old_order = (old.proxy_id.as_str(), old.row.registration_id);
            let new_order = (proxy_id.as_str(), row.registration_id);
            if old_order <= new_order {
                kills.push(KillTarget {
                    proxy_id,
                    registration_id: row.registration_id,
                });
                return;
            }
            kills.push(KillTarget {
                proxy_id: old.proxy_id.clone(),
                registration_id: old.row.registration_id,
            });
            self.by_registration.remove(&old.row.registration_id);
        }
        self.by_registration
            .insert(row.registration_id, key.clone());
        self.tunnels.insert(key, OwnedTunnel { proxy_id, row });
    }

    fn remove_registration(&mut self, registration_id: Uuid) {
        let Some(key) = self.by_registration.remove(&registration_id) else {
            return;
        };
        if self
            .tunnels
            .get(&key)
            .is_some_and(|owned| owned.row.registration_id == registration_id)
        {
            self.tunnels.remove(&key);
        }
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

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;

    fn proxy(id: &str) -> ProxyId {
        ProxyId::parse(id).unwrap()
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

    fn join(state: &mut ControllerState, id: &str, rows: Vec<TunnelRow>) -> ProxyId {
        let id = proxy(id);
        state.begin_session(
            id.clone(),
            CanonicalOrigin::parse(&format!("https://{}.proxy.example.test", id.as_str())).unwrap(),
            env!("CARGO_PKG_VERSION").into(),
            Uuid::new_v4(),
        );
        state.accept_snapshot(&id, 0, rows).unwrap();
        id
    }

    #[test]
    fn snapshot_is_atomic_and_disconnect_removes_only_owner_rows() {
        let mut state = ControllerState::new(100);
        let id = proxy("p1");
        state.begin_session(
            id.clone(),
            CanonicalOrigin::parse("https://p1.proxy.example.test").unwrap(),
            "1".into(),
            Uuid::new_v4(),
        );
        assert!(!state.is_ready());
        assert!(state.tunnel_views().is_empty());
        state
            .accept_snapshot(&id, 7, vec![row("alice", "one", Uuid::new_v4())])
            .unwrap();
        assert!(state.is_ready());
        assert_eq!(state.tunnel_views().len(), 1);
        state.disconnect("p1");
        assert!(!state.is_ready());
        assert!(state.tunnel_views().is_empty());
    }

    #[test]
    fn generation_gaps_do_not_mutate_state() {
        let mut state = ControllerState::new(100);
        let id = join(&mut state, "p1", Vec::new());
        let registration_id = Uuid::new_v4();
        assert_eq!(
            state.tunnel_up(&id, 2, row("alice", "one", registration_id)),
            Err(StateError::Generation {
                expected: 1,
                got: 2
            })
        );
        assert!(state.tunnel_views().is_empty());
    }

    #[test]
    fn fleet_cap_counts_active_and_pending_but_not_reconnect() {
        let mut state = ControllerState::new(1);
        let id = join(&mut state, "p1", Vec::new());
        assert_eq!(
            state.request_admission(
                &id,
                Uuid::new_v4(),
                Uuid::new_v4(),
                "alice".into(),
                "one".into(),
            ),
            AdmissionDecision::Admit
        );
        assert_eq!(
            state.request_admission(
                &id,
                Uuid::new_v4(),
                Uuid::new_v4(),
                "alice".into(),
                "two".into(),
            ),
            AdmissionDecision::AtCapacity
        );
        assert_eq!(
            state.request_admission(
                &id,
                Uuid::new_v4(),
                Uuid::new_v4(),
                "alice".into(),
                "one".into(),
            ),
            AdmissionDecision::Admit
        );
    }

    #[test]
    fn duplicate_snapshot_ownership_chooses_deterministic_winner() {
        let mut state = ControllerState::new(100);
        let late = row("alice", "one", Uuid::from_u128(2));
        join(&mut state, "p2", vec![late]);
        let early = row("alice", "one", Uuid::from_u128(1));
        let id = proxy("p1");
        state.begin_session(
            id.clone(),
            CanonicalOrigin::parse("https://p1.proxy.example.test").unwrap(),
            "1".into(),
            Uuid::new_v4(),
        );
        let kills = state.accept_snapshot(&id, 0, vec![early]).unwrap();
        assert_eq!(kills.len(), 1);
        assert_eq!(kills[0].proxy_id.as_str(), "p2");
        assert_eq!(state.tunnel_views()[0].proxy_id, "p1");
    }
}
