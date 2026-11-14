use crate::{Error, Request, Response, Transport};
use central_api::{Token, tower_access::Identity};

/// Introspect a Tower bearer through the configured Central transport. The caller
/// supplies its own configured origin, never an origin from the incoming request.
/// Call before every account operation; an expiry is not permission to cache logout.
/// # Errors
/// Wrong credential scope/origin, stale session, current terms missing or transport failure.
pub fn tower_identity(
    transport: &impl Transport,
    token: &Token,
    own_origin: &str,
) -> Result<Identity, Error> {
    central_api::gateway::validate_origin(
        own_origin,
        transport.endpoint().legacy_origin().is_none(),
    )?;
    match transport.exchange(&Request::TowerIdentity {}, Some(token.into()))? {
        Response::TowerIdentity { identity } if identity.origin == own_origin => Ok(identity),
        _ => Err(Error::Rejected),
    }
}
