//! The native ranked relay. Only Central chooses the upstream. No client address,
//! ambient headers or Central credential enter the upstream HTTP or socket handshake.
use central_api::{
    Error, Token,
    ranked::{
        self, Id,
        replica::{
            Fence, SeatRequest, SeatResponse,
            relay::{Call, Reply, Secret},
        },
    },
    wire::{Request, Response},
};
use central_identity::{IdentityService, Place, Presented};
use std::sync::Arc;
use tokio::sync::Semaphore;
use zeroize::Zeroizing;

mod capabilities;
#[cfg(test)]
mod transport_tests;
mod upstream;

#[derive(PartialEq, Eq)]
struct Route {
    origin: String,
    game: Option<Id>,
    seat: u8,
}

async fn route(
    identity: &IdentityService,
    session: &Presented,
    place: Option<Place>,
    fence: Fence,
) -> Result<Route, Error> {
    let request = Request::Ranked {
        request: ranked::Request::Replicate {
            request: SeatRequest::RelayRoute { fence },
        },
    };
    match identity.handle_from(request, Some(session), place).await? {
        Response::Ranked {
            response:
                ranked::Response::Replica {
                    response: SeatResponse::RelayRoute { origin, game, seat },
                },
        } if seat < 2 => Ok(Route { origin, game, seat }),
        _ => Err(Error::Rejected),
    }
}

pub(crate) struct Grant {
    pub(crate) origin: String,
    pub(crate) game: Id,
    pub(crate) seat: u8,
    pub(crate) development: bool,
    pub(crate) upstream_ticket: Zeroizing<String>,
    /// The AI account whose runner holds the session, as the store said when
    /// Central issued the ticket (`IdentityService::relay_runner`); `None`
    /// for every other session. Nothing the client sends sets it: the client
    /// holds a random handle, and this stays in Central's memory.
    pub(crate) runner: Option<[u8; 16]>,
    fence: Fence,
    // Revalidate the signed session's expiration, revocation and scoped route on each renewal.
    authorization: Zeroizing<String>,
}

impl Grant {
    /// The ranked match the grant is for.
    pub(crate) const fn match_id(&self) -> Id {
        self.fence.match_id
    }

    /// For the relay socket's tests: a grant for seat `seat` of `match_id`,
    /// a runner's when `runner` names an AI account.
    #[cfg(test)]
    pub(crate) fn synthetic(runner: Option<[u8; 16]>, match_id: Id, seat: u8) -> Self {
        Self {
            origin: "https://primary.invalid".into(),
            game: [3; 16],
            seat,
            development: false,
            upstream_ticket: Zeroizing::new("a".repeat(64)),
            runner,
            fence: Fence {
                match_id,
                generation: 0,
            },
            authorization: Zeroizing::new("synthetic".into()),
        }
    }

    pub(crate) async fn current(&self, identity: &IdentityService) -> bool {
        let Ok(session @ Presented::Session(_)) = identity.present(&self.authorization, None)
        else {
            return false;
        };
        matches!(route(identity, &session, None, self.fence).await, Ok(route) if route.origin == self.origin && route.game == Some(self.game) && route.seat == self.seat)
    }
}

#[derive(Clone)]
pub(crate) struct Relay {
    vault: capabilities::Vault,
    requests: Arc<Semaphore>,
    development: bool,
}

impl Relay {
    pub(crate) fn new(development: bool) -> Self {
        Self {
            vault: capabilities::Vault::default(),
            requests: Arc::new(Semaphore::new(64)),
            development,
        }
    }

    pub(crate) fn take(&self, bytes: &[u8]) -> Result<Grant, Error> {
        self.vault.take(bytes)
    }

    pub(crate) async fn forward(
        &self,
        identity: &IdentityService,
        presented: Option<&Presented>,
        authorization: &str,
        place: Option<Place>,
        fence: Fence,
        call: Call,
    ) -> Result<Response, Error> {
        let Some(session @ Presented::Session(owner)) = presented else {
            return Err(Error::Rejected);
        };
        let slot = self
            .requests
            .clone()
            .try_acquire_owned()
            .map_err(|_| Error::Busy)?;
        let before = route(identity, session, place, fence).await?;
        let scope = capabilities::Scope::new(owner, fence, &before);
        let call = self.vault.authorize(&scope, before.game, call)?;
        let admission = if matches!(call, Call::Admit { .. }) {
            let request = Request::RankedToken {
                gateway: before.origin.clone(),
                fence,
            };
            let Response::GatewayToken { token, .. } =
                identity.handle_from(request, Some(session), place).await?
            else {
                return Err(Error::Rejected);
            };
            Some(token)
        } else {
            None
        };
        let request = upstream::prepare(call, fence, before.game, admission.as_ref())?;
        let development = self.development;
        let origin = before.origin.clone();
        let result = tokio::task::spawn_blocking(move || {
            let _slot = slot;
            upstream::send(&origin, development, &request)
        })
        .await
        .map_err(|_| Error::Unavailable)??;
        let after = route(identity, session, place, fence).await?;
        // Registering the second deck may create the game during the request.
        if before.origin != after.origin
            || before.seat != after.seat
            || before.game.is_some() && before.game != after.game
        {
            return Err(Error::Conflict);
        }
        let reply = match result {
            upstream::ResultBody::Status(status) => Reply::Http { status, body: None },
            upstream::ResultBody::Admitted { token, renew_in } => {
                upstream::admitted(self.vault.admission(&scope, token, renew_in)?, renew_in)?
            }
            upstream::ResultBody::Seat { token, game, seat } => {
                let id = after
                    .game
                    .filter(|id| game_id(id) == game && seat == after.seat)
                    .ok_or(Error::Conflict)?;
                upstream::seated(self.vault.seat(&scope, id, token)?, id, seat)?
            }
            upstream::ResultBody::Socket { ticket, ttl, game } => {
                if after.game != Some(game) {
                    return Err(Error::Conflict);
                }
                // Central's own word on whose relay this is, never the client's.
                // Unanswered, it is no runner's: the socket keeps the address's
                // budget, the stricter one, and a person's ticket is not lost.
                let runner = identity.relay_runner(owner).await.unwrap_or(None);
                let grant = Grant {
                    origin: after.origin,
                    game,
                    seat: after.seat,
                    development,
                    upstream_ticket: Zeroizing::new(ticket.expose().to_owned()),
                    runner,
                    fence,
                    authorization: Zeroizing::new(authorization.to_owned()),
                };
                Reply::Socket {
                    ticket: self.vault.issue(grant, ttl)?,
                }
            }
        };
        Ok(Response::Ranked {
            response: ranked::Response::Replica {
                response: SeatResponse::Relayed { reply },
            },
        })
    }
}

pub(crate) fn game_id(id: &Id) -> String {
    let s = central_api::reports::hex(id);
    format!(
        "{}-{}-{}-{}-{}",
        &s[..8],
        &s[8..12],
        &s[12..16],
        &s[16..20],
        &s[20..]
    )
}
