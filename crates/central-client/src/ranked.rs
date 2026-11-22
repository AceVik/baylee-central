//! Competitive play through the configured Central, independent of the player's home gateway.
use super::{Bearer, Error, Request, Response, Session, Token, Transport};
use central_api::ranked as api;

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
    bearer: Option<Bearer<'_>>,
) -> Result<api::Response, Error> {
    let wire = Request::Ranked { request };
    let reply = transport.exchange(&wire, bearer)?;
    match (wire, reply) {
        (Request::Ranked { request }, Response::Ranked { response })
            if responds(&request, &response) =>
        {
            Ok(response)
        }
        _ => Err(Error::Rejected),
    }
}

fn responds(request: &api::Request, reply: &api::Response) -> bool {
    use api::Request as Q;
    use api::Response as A;
    match (request, reply) {
        (Q::Join { .. } | Q::Cancel { .. } | Q::Status, A::Queue { .. })
        | (Q::Seasons { .. }, A::Seasons { .. }) => true,
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

impl Session {
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
        exchange(transport, request, Some((&self.token).into()))
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
