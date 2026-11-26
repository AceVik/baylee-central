//! Competitive play through the configured Central, independent of the player's home gateway.
use super::{Error, Request, Response, Session, Token, Transport};
use central_api::ranked as api;

mod replica;
pub use replica::replica_host;

/// Read a public board, season or profile.
///
/// # Errors
/// Rejects private operations, transport failures and mismatched response types.
pub fn read(transport: &impl Transport, request: api::Request) -> Result<api::Response, Error> {
    if !request.is_public() {
        return Err(Error::Rejected);
    }
    exchange(transport, request, None)
}

fn exchange(
    transport: &impl Transport,
    request: api::Request,
    session: Option<&Session>,
) -> Result<api::Response, Error> {
    let wire = Request::Ranked { request };
    let reply = transport.exchange(&wire, session.map(|owner| (&owner.token).into()))?;
    match (wire, reply) {
        (Request::Ranked { request }, Response::Ranked { response })
            if responds(
                &request,
                &response,
                session.map(|owner| owner.identity().account),
            ) =>
        {
            Ok(response)
        }
        _ => Err(Error::Rejected),
    }
}

fn responds(request: &api::Request, reply: &api::Response, owner: Option<api::Id>) -> bool {
    use api::Request as Q;
    use api::Response as A;
    match (request, reply) {
        (Q::Replicate { request }, A::Replica { response }) => {
            owner.is_some() && replica::responds(request, response, owner)
        }
        (
            Q::OwnProfile { season },
            A::Profile {
                season: actual,
                account,
                ..
            },
        ) => owner == Some(*account) && season.is_none_or(|id| id == actual.id),
        (Q::PreviewDeck { format, .. }, A::DeckPreview { preview }) => preview.format == *format,
        (Q::Join { .. } | Q::Cancel { .. } | Q::Status, A::Queue { .. })
        | (Q::Seasons { .. }, A::Seasons { .. }) => true,
        (Q::Ai { request }, A::Ai { response }) => ai_responds(request, response, owner),
        (
            Q::Board {
                season,
                format,
                track,
                league,
                division,
                ..
            },
            A::Board {
                season: actual,
                entries,
                ..
            },
        ) => {
            season.is_none_or(|id| id == actual.id)
                && entries.iter().all(|entry| {
                    let s = &entry.standing;
                    s.format == *format
                        && s.track == *track
                        && s.rank.league == *league
                        && division.is_none_or(|number| s.rank.division == Some(number))
                })
        }
        (
            Q::Profile { target, season },
            A::Profile {
                season: actual,
                account,
                name,
                tag,
                ..
            },
        ) => {
            season.is_none_or(|id| id == actual.id)
                && match target {
                    api::ProfileTarget::Account(id) => account == id,
                    api::ProfileTarget::Handle(handle) => {
                        name.as_deref().zip(*tag).is_some_and(|(n, t)| {
                            central_api::profile::handle(n, t).eq_ignore_ascii_case(handle)
                        })
                    }
                }
        }
        _ => false,
    }
}

/// An AI runner's answer fits its question: offers answered as offers, and
/// work only for this account's own seats, at most `max_games` of them.
fn ai_responds(
    request: &api::ai::Request,
    reply: &api::ai::Response,
    owner: Option<api::Id>,
) -> bool {
    use api::ai::{Request as Q, Response as A};
    match (request, reply) {
        (Q::Offer { offers }, A::Offered { until }) => offers.is_empty() == until.is_none(),
        (
            Q::Work,
            A::Work {
                settings,
                assignments,
            },
        ) => {
            owner.is_some()
                && assignments.len() <= usize::from(settings.max_games)
                && assignments
                    .iter()
                    .all(|a| Some(a.account) == owner && a.seat < 2)
        }
        _ => false,
    }
}

impl Session {
    /// An AI runner's operation on this account, which an administrator
    /// flagged as an AI player (`docs/ai-players.md` section 4): its standing
    /// offers, or its settings and live matches.
    ///
    /// # Errors
    /// Rejects foreign endpoints, an unflagged account, offers beyond one per
    /// format or while switched off, and mismatched replies.
    pub fn ai(
        &self,
        transport: &impl Transport,
        request: api::ai::Request,
    ) -> Result<api::ai::Response, Error> {
        if let api::ai::Request::Offer { offers } = &request
            && offers.len() > api::ai::MAX_OFFERS
        {
            return Err(Error::Rejected);
        }
        let api::Response::Ai { response } =
            self.ranked(transport, api::Request::Ai { request })?
        else {
            return Err(Error::Rejected);
        };
        Ok(response)
    }

    /// Preview the explicitly selected pool in this account's private deck history.
    ///
    /// # Errors
    /// Rejects foreign endpoints, invalid pools, expired sessions and mismatched replies.
    pub fn preview_ranked_deck(
        &self,
        transport: &impl Transport,
        format: api::Format,
        pool: Vec<api::PoolEntry>,
    ) -> Result<api::DeckPreview, Error> {
        if !api::valid_pool(&pool) {
            return Err(Error::Rejected);
        }
        let api::Response::DeckPreview { preview } =
            self.ranked(transport, api::Request::PreviewDeck { format, pool })?
        else {
            return Err(Error::Rejected);
        };
        Ok(preview)
    }
    /// Manage this account's private queue entry or read the ladder.
    ///
    /// # Errors
    /// Requires this session's configured endpoint and a valid Central reply.
    pub fn ranked(
        &self,
        transport: &impl Transport,
        request: api::Request,
    ) -> Result<api::Response, Error> {
        self.same_endpoint(transport)?;
        exchange(transport, request, Some(self))
    }
}

/// Send an availability update or match operation using a gateway credential.
///
/// # Errors
/// Central rejects unverified hosts, stale assignments and conflicting results.
pub fn host(
    transport: &impl Transport,
    credential: &Token,
    request: api::HostRequest,
) -> Result<api::HostResponse, Error> {
    match transport.exchange(&Request::RankedHost { request }, Some(credential.into()))? {
        Response::RankedHost { response } => Ok(response),
        _ => Err(Error::Rejected),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn own_projection_binds_the_session_and_season() {
        let owner = [1; 16];
        let season = [2; 16];
        let request = api::Request::OwnProfile {
            season: Some(season),
        };
        let reply = api::Response::Profile {
            season: api::Season {
                id: season,
                name: String::new(),
                starts: 0,
                ends: None,
            },
            account: owner,
            name: None,
            tag: None,
            formats: vec![],
            awards: vec![],
            ai: None,
        };
        assert!(responds(&request, &reply, Some(owner)));
        assert!(!responds(&request, &reply, None));
        assert!(!responds(&request, &reply, Some([9; 16])));
        assert!(!responds(
            &api::Request::OwnProfile {
                season: Some([4; 16])
            },
            &reply,
            Some(owner)
        ));
    }
}
