//! The native seat receives random Central handles. Only the relay holds the
//! corresponding host credential, scoped to one authenticated route.
use super::{Error, Fence, Grant, Id, Route, Secret, Token};
use central_api::ranked::replica::relay::Call;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, Once},
    time::{Duration, Instant},
};

const HANDLE_LIMIT: usize = 2048;
const GRANT_LIMIT: usize = 512;
const SEAT_LIFE: Duration = Duration::from_secs(30);
const SOCKET_LIFE: Duration = Duration::from_secs(15);
const SWEEP: Duration = Duration::from_secs(1);

#[derive(Clone, PartialEq, Eq)]
pub(super) struct Scope {
    session: [u8; 32],
    fence: Fence,
    // Central's canonical, unique gateway origin for this generation.
    origin: String,
    seat: u8,
}

impl Scope {
    pub(super) fn new(session: &Token, fence: Fence, route: &Route) -> Self {
        Self {
            session: session.digest(),
            fence,
            origin: route.origin.clone(),
            seat: route.seat,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Admission,
    Seat(Id),
}

struct Entry {
    scope: Scope,
    kind: Kind,
    expires: Instant,
    upstream: Secret,
}

#[derive(Default)]
struct Entries {
    handles: HashMap<[u8; 32], Entry>,
    sockets: HashMap<[u8; 32], (Instant, Grant)>,
}

impl Entries {
    fn purge(&mut self, now: Instant) {
        self.handles.retain(|_, h| h.expires > now);
        self.sockets.retain(|_, (expiry, _)| *expiry > now);
    }

    fn mint(
        &mut self,
        now: Instant,
        scope: &Scope,
        kind: Kind,
        upstream: Secret,
        ttl: Duration,
    ) -> Result<Secret, Error> {
        if ttl.is_zero() || !upstream.is_capability() {
            return Err(Error::Rejected);
        }
        let (key, handle) = fresh()?;
        self.purge(now);
        if self.handles.contains_key(&key) {
            return Err(Error::Unavailable);
        }
        self.handles.retain(|_, entry| {
            !(entry.scope.session == scope.session
                && entry.scope.fence.match_id == scope.fence.match_id
                && entry.scope.fence.generation < scope.fence.generation)
        });
        // Keep a short overlap for retried admissions and overlapping responses,
        // without permitting one player to fill the global handle table.
        let mut siblings = self
            .handles
            .iter()
            .filter(|(_, e)| e.scope == *scope && e.kind == kind)
            .map(|(key, e)| (e.expires, *key))
            .collect::<Vec<_>>();
        if siblings.len() >= 2 {
            siblings.sort_unstable();
            self.handles.remove(&siblings[0].1);
        }
        if self.handles.len() >= HANDLE_LIMIT {
            return Err(Error::Busy);
        }
        self.handles.insert(
            key,
            Entry {
                scope: scope.clone(),
                kind,
                upstream,
                expires: now + ttl,
            },
        );
        Ok(handle)
    }

    fn resolve(&mut self, scope: &Scope, kind: Kind, handle: &Secret) -> Result<Secret, Error> {
        let digest = Token::from_hex(handle.expose())?.digest();
        let entry = self.handles.get(&digest).ok_or(Error::Rejected)?;
        if entry.expires <= Instant::now() {
            self.handles.remove(&digest);
            return Err(Error::Rejected);
        }
        if entry.scope != *scope || entry.kind != kind {
            return Err(Error::Rejected);
        }
        if kind == Kind::Admission {
            return Ok(entry.upstream.clone());
        }
        self.handles
            .remove(&digest)
            .map(|e| e.upstream)
            .ok_or(Error::Rejected)
    }
}

fn fresh() -> Result<([u8; 32], Secret), Error> {
    let token = Token::generate()?;
    let handle = Secret::new(std::mem::take(&mut *token.hex()))?;
    Ok((token.digest(), handle))
}

#[derive(Clone)]
pub(super) struct Vault {
    entries: Arc<Mutex<Entries>>,
    sweeper: Arc<Once>,
}

impl Default for Vault {
    fn default() -> Self {
        Self {
            entries: Arc::default(),
            sweeper: Arc::new(Once::new()),
        }
    }
}

impl Vault {
    // Lazily started inside the native request runtime. One worker serves
    // every clone, wakes even during total inactivity and retains no owner.
    fn start(&self) -> Result<(), Error> {
        let runtime = tokio::runtime::Handle::try_current().map_err(|_| Error::Unavailable)?;
        self.sweeper.call_once(|| {
            let weak = Arc::downgrade(&self.entries);
            runtime.spawn(async move {
                loop {
                    tokio::time::sleep(SWEEP).await;
                    let Some(entries) = weak.upgrade() else {
                        return;
                    };
                    let Ok(mut entries) = entries.lock() else {
                        return;
                    };
                    entries.purge(Instant::now());
                }
            });
        });
        Ok(())
    }

    pub(super) fn admission(
        &self,
        scope: &Scope,
        secret: Secret,
        renew_in: u64,
    ) -> Result<Secret, Error> {
        if !(1..=1800).contains(&renew_in) {
            return Err(Error::Rejected);
        }
        self.start()?;
        self.entries.lock().map_err(|_| Error::Unavailable)?.mint(
            Instant::now(),
            scope,
            Kind::Admission,
            secret,
            Duration::from_secs(renew_in + 15),
        )
    }

    pub(super) fn seat(&self, scope: &Scope, game: Id, secret: Secret) -> Result<Secret, Error> {
        self.start()?;
        self.entries.lock().map_err(|_| Error::Unavailable)?.mint(
            Instant::now(),
            scope,
            Kind::Seat(game),
            secret,
            SEAT_LIFE,
        )
    }

    /// The only conversion of a native handle to a usable upstream capability.
    pub(super) fn authorize(
        &self,
        scope: &Scope,
        bound: Option<Id>,
        mut call: Call,
    ) -> Result<Call, Error> {
        let (kind, handle) = match &mut call {
            Call::Admit {} => return Ok(call),
            Call::Play { bearer, .. } => (Kind::Admission, bearer),
            Call::Reclaim { bearer, game } if Some(*game) == bound => (Kind::Admission, bearer),
            Call::SocketTicket { seat_token, game } if Some(*game) == bound => {
                (Kind::Seat(*game), seat_token)
            }
            _ => return Err(Error::Rejected),
        };
        *handle = self
            .entries
            .lock()
            .map_err(|_| Error::Unavailable)?
            .resolve(scope, kind, handle)?;
        Ok(call)
    }

    pub(super) fn issue(&self, grant: Grant, ttl: Duration) -> Result<Secret, Error> {
        if ttl.is_zero() {
            return Err(Error::Rejected);
        }
        self.start()?;
        let (key, handle) = fresh()?;
        let now = Instant::now();
        let mut entries = self.entries.lock().map_err(|_| Error::Unavailable)?;
        entries.purge(now);
        let siblings = entries
            .sockets
            .values()
            .filter(|(_, g)| g.fence == grant.fence && g.seat == grant.seat)
            .count();
        if entries.sockets.len() >= GRANT_LIMIT || siblings >= 2 {
            return Err(Error::Busy);
        }
        if entries.sockets.contains_key(&key) {
            return Err(Error::Unavailable);
        }
        entries
            .sockets
            .insert(key, (now + ttl.min(SOCKET_LIFE), grant));
        Ok(handle)
    }

    pub(super) fn take(&self, bytes: &[u8]) -> Result<Grant, Error> {
        let key =
            Token::from_hex(std::str::from_utf8(bytes).map_err(|_| Error::Rejected)?)?.digest();
        let (expires, grant) = self
            .entries
            .lock()
            .map_err(|_| Error::Unavailable)?
            .sockets
            .remove(&key)
            .ok_or(Error::Rejected)?;
        (expires > Instant::now())
            .then_some(grant)
            .ok_or(Error::Rejected)
    }
}

#[cfg(test)]
mod tests;
