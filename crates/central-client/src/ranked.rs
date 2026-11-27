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

/// An AI runner's answer fits its question: offers answered as offers;
/// work only for this account's own seats, at most [`api::ai::MAX_GAMES`] of
/// them: an administrator who lowers `max_games` below the live matches
/// does not shorten the list, whose matches still have to be played; and a
/// carousel seat only this account's, in the format asked for and against
/// an AI.
fn ai_responds(
    request: &api::ai::Request,
    reply: &api::ai::Response,
    owner: Option<api::Id>,
) -> bool {
    use api::ai::{Request as Q, Response as A};
    match (request, reply) {
        (Q::Offer { offers }, A::Offered { until }) => offers.is_empty() == until.is_none(),
        (Q::Work, A::Work { assignments, .. }) => {
            owner.is_some()
                && assignments.len() <= usize::from(api::ai::MAX_GAMES)
                && assignments
                    .iter()
                    .all(|a| Some(a.account) == owner && a.seat < 2)
        }
        (Q::Carousel { format }, A::Seated { assignment }) => {
            owner == Some(assignment.account)
                && assignment.seat < 2
                && assignment.format == *format
                && assignment.opponent_ai.is_some()
        }
        (Q::Carousel { .. }, A::Unseated { .. }) => true,
        _ => false,
    }
}

impl Session {
    /// An AI runner's operation on this account, which an administrator
    /// created as an AI player (`docs/ai-players.md` section 4): its standing
    /// offers, or its settings and live matches.
    ///
    /// # Errors
    /// Rejects foreign endpoints, a person's account, offers beyond one per
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

    /// Ask the carousel for a match between this AI account and another AI
    /// in `format` now (`docs/ai-players.md` section 7): its seat in the
    /// match made, which [`Self::ai`]'s work lists from now on too, or why
    /// none was made, which says when asking again can help.
    ///
    /// # Errors
    /// Rejects foreign endpoints, a person's account and mismatched replies,
    /// a seat against a person among them.
    pub fn ai_carousel(
        &self,
        transport: &impl Transport,
        format: api::Format,
    ) -> Result<Result<api::Assignment, api::ai::Unseated>, Error> {
        match self.ai(transport, api::ai::Request::Carousel { format })? {
            api::ai::Response::Seated { assignment } => Ok(Ok(assignment)),
            api::ai::Response::Unseated { reason } => Ok(Err(reason)),
            api::ai::Response::Offered { .. } | api::ai::Response::Work { .. } => {
                Err(Error::Rejected)
            }
        }
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

    /// An administrator lowered `max_games` to one under two live matches:
    /// the runner still reads both, and never more than the hard maximum.
    #[test]
    fn work_lists_every_live_match_up_to_the_hard_maximum() {
        let owner = [1; 16];
        let seat = |n: u8| api::Assignment {
            match_id: [n; 16],
            host_id: [2; 16],
            origin: "https://play.example".into(),
            format: api::Format::Standard,
            account: owner,
            seat: n % 2,
            deck: [3; 32],
            game: None,
            opponent_ai: None,
        };
        let work = |count: u8| api::Response::Ai {
            response: api::ai::Response::Work {
                settings: api::ai::Settings {
                    badge: api::ai::AiBadge {
                        kind: api::ai::AiKind::House,
                        label: "Haus-KI".into(),
                    },
                    profile: "house:expert".into(),
                    enabled: true,
                    max_games: 1,
                },
                assignments: (0..count).map(seat).collect(),
            },
        };
        let request = api::Request::Ai {
            request: api::ai::Request::Work,
        };
        for count in [0, 2, api::ai::MAX_GAMES] {
            assert!(responds(&request, &work(count), Some(owner)), "{count}");
        }
        assert!(!responds(
            &request,
            &work(api::ai::MAX_GAMES + 1),
            Some(owner)
        ));
        assert!(!responds(&request, &work(2), Some([9; 16])));
        assert!(!responds(&request, &work(2), None));
    }

    /// A carousel seat is this account's, in the format it asked for, and
    /// against an AI; a reason is read as it comes; the carousel's answers
    /// answer no other request.
    #[test]
    fn a_carousel_seat_is_the_account_s_own_against_an_ai() {
        let owner = [1; 16];
        let assignment = api::Assignment {
            match_id: [7; 16],
            host_id: [2; 16],
            origin: "https://play.example".into(),
            format: api::Format::Highlander,
            account: owner,
            seat: 1,
            deck: [3; 32],
            game: None,
            opponent_ai: Some(api::ai::AiBadge {
                kind: api::ai::AiKind::Net,
                label: "Netz".into(),
            }),
        };
        let request = api::Request::Ai {
            request: api::ai::Request::Carousel {
                format: api::Format::Highlander,
            },
        };
        let seated = |assignment: api::Assignment| api::Response::Ai {
            response: api::ai::Response::Seated { assignment },
        };
        assert!(responds(&request, &seated(assignment.clone()), Some(owner)));
        for wrong in [
            api::Assignment {
                account: [9; 16],
                ..assignment.clone()
            },
            api::Assignment {
                seat: 2,
                ..assignment.clone()
            },
            api::Assignment {
                format: api::Format::Standard,
                ..assignment.clone()
            },
            api::Assignment {
                opponent_ai: None,
                ..assignment.clone()
            },
        ] {
            assert!(!responds(&request, &seated(wrong), Some(owner)));
        }
        assert!(!responds(&request, &seated(assignment.clone()), None));
        for reason in [
            api::ai::Unseated::Off,
            api::ai::Unseated::Offer,
            api::ai::Unseated::Tomorrow,
            api::ai::Unseated::Later,
        ] {
            let reply = api::Response::Ai {
                response: api::ai::Response::Unseated { reason },
            };
            assert!(responds(&request, &reply, Some(owner)));
            let work = api::Request::Ai {
                request: api::ai::Request::Work,
            };
            assert!(!responds(&work, &reply, Some(owner)));
        }
        let offered = api::Response::Ai {
            response: api::ai::Response::Offered { until: None },
        };
        assert!(!responds(&request, &offered, Some(owner)));
        let work = api::Request::Ai {
            request: api::ai::Request::Work,
        };
        assert!(!responds(&work, &seated(assignment), Some(owner)));
    }
}
