//! Reusable gateway directory and owner administration without exposing a central session bearer.
use super::{Error, Request, Response, Session, Token, Transport};
use central_api::gateway::{
    GatewayActivity, GatewayFigures, GatewayListing, GatewayMetadata, validate_origin,
};

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

/// Central's public figures as one page reads them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublicFigures {
    /// Accounts that can sign in; the same on every page.
    pub accounts: u64,
    /// At most 100 online gateways that report, in ascending ID order, with
    /// the counts each last sent. They are the gateways' own claims.
    pub gateways: Vec<GatewayFigures>,
}

/// Read one page of the public figures: the number of accounts, and what
/// online gateways last reported. A gateway the directory lists and this
/// page does not is one that does not report, not one that is empty.
///
/// # Errors
/// Rejects malformed IDs, unordered pages, counts above their bound and
/// transport failure.
pub fn public_figures(
    transport: &impl Transport,
    after: Option<[u8; 16]>,
) -> Result<PublicFigures, Error> {
    if after.is_some_and(|id| !valid_id(id)) {
        return Err(Error::Rejected);
    }
    let Response::PublicFigures { accounts, gateways } =
        transport.exchange(&Request::PublicFigures { after }, None)?
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
        gateway.activity.validate()?;
        previous = Some(gateway.id);
    }
    Ok(PublicFigures { accounts, gateways })
}

/// A gateway's heartbeat with its activity (`Request::GatewayPulse`), under
/// the credential its plain heartbeat uses: the heartbeat token or a gateway
/// token with the `listing` scope, or the bridge credential for the official
/// gateway. Sending it is the operator's choice; a plain heartbeat afterwards
/// withdraws the counts. A Central built before the pulse answers `Rejected`,
/// as it does for a wrong credential: a gateway then sends its plain
/// heartbeat and tries the pulse again later.
///
/// # Errors
/// Rejects metadata that is unsafe or off `origin`, a count above its bound
/// (neither is sent), a refused credential, the five-second limit (`Busy`)
/// and transport failure.
pub fn gateway_pulse(
    transport: &impl Transport,
    credential: &Token,
    origin: &str,
    metadata: GatewayMetadata,
    activity: GatewayActivity,
) -> Result<(), Error> {
    metadata.validate(origin)?;
    activity.validate()?;
    match transport.exchange(
        &Request::GatewayPulse { metadata, activity },
        Some(credential),
    )? {
        Response::Done => Ok(()),
        _ => Err(Error::Rejected),
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use central_api::Endpoint;
    use std::cell::RefCell;

    const ORIGIN: &str = "https://gateway.invalid";
    const COUNTED: GatewayActivity = GatewayActivity {
        players: 12,
        tables: 3,
        games: 4,
    };

    /// A stand-in Central: one prepared answer for whatever is asked, and
    /// what was asked, with whether a credential came along.
    struct Answering {
        endpoint: Endpoint,
        answer: Vec<u8>,
        asked: RefCell<Vec<(String, bool)>>,
    }

    impl Answering {
        fn with(answer: &Response) -> Self {
            Self {
                endpoint: Endpoint::live(),
                answer: central_api::encode(answer).unwrap(),
                asked: RefCell::default(),
            }
        }
    }

    impl Transport for Answering {
        fn endpoint(&self) -> &Endpoint {
            &self.endpoint
        }
        fn exchange(&self, request: &Request, bearer: Option<&Token>) -> Result<Response, Error> {
            let text = String::from_utf8(central_api::encode(request)?).unwrap();
            self.asked.borrow_mut().push((text, bearer.is_some()));
            central_api::decode(&self.answer)
        }
    }

    /// A well-formed id of Central's; a larger `n` is a later one.
    fn id(n: u8) -> [u8; 16] {
        let mut id = [0; 16];
        (id[0], id[6], id[8]) = (n, 0x70, 0x80);
        id
    }

    fn page(ids: &[[u8; 16]], activity: GatewayActivity) -> Response {
        Response::PublicFigures {
            accounts: 42,
            gateways: ids
                .iter()
                .map(|id| GatewayFigures { id: *id, activity })
                .collect(),
        }
    }

    fn metadata() -> GatewayMetadata {
        GatewayMetadata {
            name: "Synthetic".into(),
            version: "1.2.3-test".into(),
            website: format!("{ORIGIN}/"),
        }
    }

    #[test]
    fn the_figures_are_checked_before_they_are_believed() {
        let central = Answering::with(&page(&[id(1), id(2)], COUNTED));
        let read = public_figures(&central, None).unwrap();
        assert_eq!(read.accounts, 42);
        assert_eq!(
            read.gateways.iter().map(|g| g.id).collect::<Vec<_>>(),
            [id(1), id(2)]
        );
        // Asked of everyone's Central, without a credential.
        assert_eq!(
            *central.asked.borrow(),
            [(
                r#"{"operation":"public_figures","after":null}"#.to_owned(),
                false
            )]
        );
        // The next page starts after the cursor.
        let next = Answering::with(&page(&[id(3)], COUNTED));
        assert_eq!(
            public_figures(&next, Some(id(2))).unwrap().gateways.len(),
            1
        );
        assert_eq!(public_figures(&next, Some(id(3))), Err(Error::Rejected));
        assert_eq!(public_figures(&next, Some(id(4))), Err(Error::Rejected));
        // A cursor that is no id is not sent at all.
        assert_eq!(public_figures(&next, Some([0; 16])), Err(Error::Rejected));
        assert_eq!(next.asked.borrow().len(), 3);
        let over = GatewayActivity {
            games: GatewayActivity::MOST + 1,
            ..COUNTED
        };
        let crowd: Vec<[u8; 16]> = (1..=101).map(id).collect();
        for answer in [
            page(&[id(2), id(1)], COUNTED),
            page(&[id(1), id(1)], COUNTED),
            page(&[id(1), [0xff; 16]], COUNTED),
            page(&[id(1)], over),
            page(&crowd, COUNTED),
            Response::Done,
            Response::GatewayDirectory { gateways: vec![] },
        ] {
            assert_eq!(
                public_figures(&Answering::with(&answer), None),
                Err(Error::Rejected)
            );
        }
        // An empty page is an answer: nobody reports.
        let nobody = public_figures(&Answering::with(&page(&[], COUNTED)), None).unwrap();
        assert_eq!(nobody.gateways, []);
        let full: Vec<[u8; 16]> = (1..=100).map(id).collect();
        assert_eq!(
            public_figures(&Answering::with(&page(&full, COUNTED)), None)
                .unwrap()
                .gateways
                .len(),
            100
        );
    }

    #[test]
    fn a_pulse_sends_only_what_central_would_take() {
        let token = Token::generate().unwrap();
        let central = Answering::with(&Response::Done);
        let over = GatewayActivity {
            players: GatewayActivity::MOST + 1,
            ..COUNTED
        };
        assert_eq!(
            gateway_pulse(&central, &token, ORIGIN, metadata(), over),
            Err(Error::Rejected)
        );
        assert_eq!(
            gateway_pulse(
                &central,
                &token,
                "https://elsewhere.invalid",
                metadata(),
                COUNTED
            ),
            Err(Error::Rejected)
        );
        assert!(central.asked.borrow().is_empty(), "neither was sent");
        gateway_pulse(&central, &token, ORIGIN, metadata(), COUNTED).unwrap();
        assert_eq!(
            *central.asked.borrow(),
            [(
                concat!(
                    r#"{"operation":"gateway_pulse","metadata":{"name":"Synthetic","#,
                    r#""version":"1.2.3-test","website":"https://gateway.invalid/"},"#,
                    r#""activity":{"players":12,"tables":3,"games":4}}"#
                )
                .to_owned(),
                true
            )]
        );
        // Only `Done` is a pulse taken.
        let odd = Answering::with(&page(&[], COUNTED));
        assert_eq!(
            gateway_pulse(&odd, &token, ORIGIN, metadata(), COUNTED),
            Err(Error::Rejected)
        );
    }
}
