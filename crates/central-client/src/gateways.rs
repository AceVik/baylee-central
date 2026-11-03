//! Reusable gateway directory and owner administration without exposing a central session bearer.
use super::{Error, Request, Response, Session, Token, Transport};
use central_api::gateway::{GatewayListing, GatewayMetadata, validate_origin};

/// One pending registration. Store credentials privately; neither token grants operator approval.
pub struct GatewayRegistration {
    /// PostgreSQL-generated gateway identity.
    pub gateway: [u8; 16],
    /// Gateway-server credential for ticket redemption and live authorization checks.
    pub credential: Token,
    /// Independent, narrowly scoped status publisher credential.
    pub heartbeat: Token,
}

fn valid_id(id: [u8; 16]) -> bool {
    id[6] >> 4 == 7 && id[8] >> 6 == 2
}

/// Read one public directory page; entries are display metadata, never login destinations for passwords.
///
/// # Errors
/// Rejects malformed origins/metadata/IDs, unordered pages and transport failure.
pub fn gateway_directory(
    transport: &impl Transport,
    after: Option<[u8; 16]>,
) -> Result<Vec<GatewayListing>, Error> {
    if after.is_some_and(|id| !valid_id(id)) {
        return Err(Error::Rejected);
    }
    let Response::GatewayDirectory { gateways } =
        transport.exchange(&Request::GatewayDirectory { after }, None)?
    else {
        return Err(Error::Rejected);
    };
    if gateways.len() > 100 {
        return Err(Error::Rejected);
    }
    let mut previous = after;
    for gateway in &gateways {
        if !valid_id(gateway.id) || previous.is_some_and(|id| id >= gateway.id) {
            return Err(Error::Rejected);
        }
        validate_origin(
            &gateway.origin,
            transport.endpoint().origin().starts_with("http://"),
        )?;
        gateway.metadata.validate(&gateway.origin)?;
        previous = Some(gateway.id);
    }
    Ok(gateways)
}

impl Session {
    /// Reserve an origin under this account. Separate operator verification is still required.
    ///
    /// # Errors
    /// Rejects malformed input, unavailable/revoked sessions and conflicting registration.
    pub fn register_gateway(
        &self,
        transport: &impl Transport,
        origin: String,
        metadata: GatewayMetadata,
    ) -> Result<GatewayRegistration, Error> {
        self.same_endpoint(transport)?;
        validate_origin(&origin, self.endpoint.origin().starts_with("http://"))?;
        metadata.validate(&origin)?;
        match transport.exchange(
            &Request::RegisterGateway { origin, metadata },
            Some(&self.token),
        )? {
            Response::GatewayRegistered {
                gateway,
                credential,
                heartbeat,
            } if valid_id(gateway) && credential.digest() != heartbeat.digest() => {
                Ok(GatewayRegistration {
                    gateway,
                    credential,
                    heartbeat,
                })
            }
            _ => Err(Error::Rejected),
        }
    }

    /// Replace explicitly named administrators of an owned gateway with revision protection.
    ///
    /// # Errors
    /// Rejects malformed IDs/revisions, more than 32 members, missing ownership and conflicts.
    pub fn set_gateway_admins(
        &self,
        transport: &impl Transport,
        gateway: [u8; 16],
        expected_revision: u64,
        accounts: Vec<[u8; 16]>,
    ) -> Result<u64, Error> {
        self.same_endpoint(transport)?;
        if !valid_id(gateway)
            || accounts.len() > 32
            || accounts.iter().any(|id| !valid_id(*id))
            || expected_revision == 0
        {
            return Err(Error::Rejected);
        }
        let next = expected_revision.checked_add(1).ok_or(Error::Rejected)?;
        match transport.exchange(
            &Request::SetGatewayAdmins {
                gateway,
                expected_revision,
                accounts,
            },
            Some(&self.token),
        )? {
            Response::GatewayAdmins { revision } if revision == next => Ok(revision),
            _ => Err(Error::Rejected),
        }
    }
}
