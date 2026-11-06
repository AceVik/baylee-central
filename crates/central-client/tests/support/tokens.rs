//! The session tokens over real HTTP and PostgreSQL, from the outside: what an old client,
//! a gateway and an attacker holding a real token each get. The rules of the format itself
//! are tested where it lives, in `central-crypto`.
use super::{
    Http, now,
    password::{OLD, close, moved},
};
use central_api::{
    Bearer, Endpoint, Error, PROTOCOL_HEADER, PROTOCOL_VERSION, SignedToken, Token,
    UPDATE_REQUIRED_BODY,
    gateway::GatewayMetadata,
    wire::{
        GatewayPurpose::{Admin, Admit},
        Identity, Request, Response,
    },
};
use central_client::{Session, Transport};
use central_crypto::jwt::{
    self, CentralClaims, Expected, GATEWAY_TOKEN_SECONDS, GatewayClaims, JwtError, ReplayCache,
    RevocationClaims,
};
use central_identity::{Delivery, GatewayDelivery};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement, prelude::Uuid};
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;

const OFFICIAL: &str = "http://127.0.0.1:28766";
const ORIGIN: &str = "http://127.0.0.1:29201";
const OTHER: &str = "http://127.0.0.1:29202";
const PATH: &str = "/auth/central/invalidate";

/// Passes everything on, and keeps the Central token that goes by: what the client holds,
/// which the SDK gives nobody.
pub(super) struct Recording<'a> {
    http: &'a Http,
    central: Mutex<Option<String>>,
}

impl<'a> Recording<'a> {
    pub(super) const fn new(http: &'a Http) -> Self {
        Self {
            http,
            central: Mutex::new(None),
        }
    }

    /// The Central token that went by, as a client presents it.
    pub(super) fn taken(&self) -> SignedToken {
        let text = self.central.lock().unwrap().clone().unwrap();
        SignedToken::new(zeroize::Zeroizing::new(text)).unwrap()
    }
}

impl Transport for Recording<'_> {
    fn endpoint(&self) -> &Endpoint {
        self.http.endpoint()
    }
    fn exchange(&self, request: &Request, bearer: Option<Bearer<'_>>) -> Result<Response, Error> {
        let response = self.http.exchange(request, bearer)?;
        if let Response::Session { token, .. } = &response {
            *self.central.lock().unwrap() = Some(token.expose().to_owned());
        }
        Ok(response)
    }
}

/// A gateway that answers as it is told, and keeps what it was sent.
struct Recorded {
    answer: Delivery,
    calls: Mutex<Vec<(String, String)>>,
}

impl GatewayDelivery for Recorded {
    fn deliver(&self, url: &str, message: &SignedToken) -> Delivery {
        self.calls
            .lock()
            .unwrap()
            .push((url.to_owned(), message.expose().to_owned()));
        self.answer
    }
}

fn metadata(origin: &str) -> GatewayMetadata {
    GatewayMetadata {
        name: "Synthetic gateway".into(),
        version: "1.0-test".into(),
        website: format!("{origin}/status"),
    }
}

/// The status of a native request with this bearer text, and the protocol header.
fn status_with(http: &Http, bearer: &str, request: &Request) -> u16 {
    let body = central_api::encode(request).unwrap();
    let authorization = format!("Bearer {bearer}");
    http.raw(
        "POST /v1/native",
        &[
            (PROTOCOL_HEADER, PROTOCOL_VERSION),
            ("Content-Type", "application/json"),
            ("Authorization", &authorization),
        ],
        &body,
    )
    .0
}

pub(crate) fn run(http: &Http, bridge: &Token, runtime: &Runtime, operator: &DatabaseConnection) {
    an_old_client_is_told_to_update(http);
    the_key_set_is_public_and_cached(http);
    let (identity, _) = moved(http, bridge, "token-tests");
    close(http, bridge, "token-tests", identity);
    let recording = Recording::new(http);
    let mut session = central_client::login(&recording, identity, OLD.into()).unwrap();
    session.confirm_restored(http).unwrap();
    let taken = recording.taken();
    let central = taken.expose();
    a_central_token_is_for_central_alone(http, central);
    a_copied_capability_opens_nothing(http, central);
    a_changed_token_opens_nothing(http, central);
    let (credential, heartbeat, other) = gateways(http, &session, runtime, operator);
    a_gateway_token_is_for_that_gateway_alone(http, &session, identity);
    a_bearer_keeps_to_its_own_side(http, central, &credential, &heartbeat);
    revocations_reach_the_gateway(
        http,
        bridge,
        session,
        &credential,
        &heartbeat,
        &other,
        runtime,
    );
    a_token_outlives_no_session(http, central);
}

fn an_old_client_is_told_to_update(http: &Http) {
    let body = central_api::encode(&Request::Terms).unwrap();
    let json = ("Content-Type", "application/json");
    for headers in [
        vec![json],
        vec![json, (PROTOCOL_HEADER, "1")],
        vec![json, (PROTOCOL_HEADER, "3")],
    ] {
        let (status, head, answer) = http.raw("POST /v1/native", &headers, &body);
        assert_eq!(status, 426, "{headers:?}");
        assert_eq!(answer, UPDATE_REQUIRED_BODY.as_bytes());
        assert!(
            head.to_lowercase()
                .contains("content-type: application/json")
        );
    }
    // Before anything else: an old sign-in is told, not refused for its form.
    let old = br#"{"operation":"gateway_ticket","gateway":"x","purpose":"admit"}"#;
    assert_eq!(http.raw("POST /v1/native", &[json], old).0, 426);
    // A gateway too old hears the same at its reports.
    assert_eq!(http.raw("POST /v1/reports", &[json], b"{}").0, 426);
    // This build is served.
    let (status, ..) = http.raw(
        "POST /v1/native",
        &[json, (PROTOCOL_HEADER, PROTOCOL_VERSION)],
        &body,
    );
    assert_eq!(status, 200);
}

fn the_key_set_is_public_and_cached(http: &Http) {
    let (status, head, _) = http.raw("GET /.well-known/jwks.json", &[], b"");
    let head = head.to_lowercase();
    assert_eq!(status, 200, "no session and no protocol header");
    assert!(head.contains("content-type: application/jwk-set+json"));
    assert!(head.contains("cache-control: public, max-age=300"));
    assert_eq!(http.keys().keys().len(), 1);
    assert_eq!(http.raw("POST /.well-known/jwks.json", &[], b"").0, 405);
}

fn a_central_token_is_for_central_alone(http: &Http, central: &str) {
    let origin = http.endpoint().origin();
    let at = |audience| Expected {
        issuer: origin,
        audience,
        now: now(),
    };
    let claims: CentralClaims = jwt::verify(central, &http.keys(), at(origin)).ok().unwrap();
    assert_eq!(claims.aud, origin);
    // A gateway it reaches by mistake cannot take it for one of its own.
    assert_eq!(
        jwt::verify::<GatewayClaims>(central, &http.keys(), at(ORIGIN)).err(),
        Some(JwtError::Kind)
    );
}

fn a_copied_capability_opens_nothing(http: &Http, central: &str) {
    assert_eq!(status_with(http, central, &Request::SessionStatus), 200);
    let origin = http.endpoint().origin();
    let claims: CentralClaims = jwt::verify(
        central,
        &http.keys(),
        Expected {
            issuer: origin,
            audience: origin,
            now: now(),
        },
    )
    .ok()
    .unwrap();
    // The capability is in the token, and bare it is no session.
    for request in [
        Request::SessionStatus,
        Request::ReadDocument,
        Request::Logout,
    ] {
        assert_eq!(status_with(http, &claims.sid, &request), 401);
    }
    assert_eq!(status_with(http, central, &Request::SessionStatus), 200);
}

fn a_changed_token_opens_nothing(http: &Http, central: &str) {
    let parts: Vec<&str> = central.split('.').collect();
    let unsigned = format!("{}.{}.", parts[0], parts[1]);
    let swapped = format!("{}.{}.{}", parts[0], parts[0], parts[2]);
    let mut flipped = central.to_owned();
    let middle = central.len() / 2;
    let replacement = if &central[middle..=middle] == "A" {
        "B"
    } else {
        "A"
    };
    flipped.replace_range(middle..=middle, replacement);
    for token in [
        unsigned.as_str(),
        swapped.as_str(),
        flipped.as_str(),
        "a.b.c",
        ".",
    ] {
        assert_eq!(
            status_with(http, token, &Request::SessionStatus),
            401,
            "{token}"
        );
    }
}

/// Two registered gateways, the first approved with an invalidate path.
fn gateways(
    http: &Http,
    session: &Session,
    runtime: &Runtime,
    operator: &DatabaseConnection,
) -> (Token, Token, Token) {
    let one = session
        .register_gateway(http, ORIGIN.into(), metadata(ORIGIN))
        .unwrap();
    let two = session
        .register_gateway(http, OTHER.into(), metadata(OTHER))
        .unwrap();
    for gateway in [one.gateway, two.gateway] {
        runtime
            .block_on(operator.execute_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "INSERT INTO central_identity.gateway_approvals(gateway_id) VALUES ($1)",
                vec![Uuid::from_bytes(gateway).into()],
            )))
            .unwrap();
    }
    let path = |path: &str| Request::RegisterInvalidate { path: path.into() };
    let credential = Some((&one.credential).into());
    assert!(matches!(
        http.exchange(&path(PATH), credential).unwrap(),
        Response::Done
    ));
    for refused in [
        "https://evil.example/x",
        "//evil.example",
        "/a/../b",
        "/a?b",
    ] {
        assert_eq!(
            http.exchange(&path(refused), credential).err(),
            Some(Error::Rejected),
            "{refused}"
        );
    }
    // A heartbeat credential is not the one that names a path.
    assert_eq!(
        http.exchange(&path(PATH), Some((&one.heartbeat).into()))
            .err(),
        Some(Error::Rejected)
    );
    (one.credential, one.heartbeat, two.credential)
}

fn a_gateway_token_is_for_that_gateway_alone(http: &Http, session: &Session, identity: Identity) {
    let (token, expires_at) = session.gateway_token(http, ORIGIN.into(), Admit).unwrap();
    let claims = http.gateway_claims(&token, ORIGIN);
    assert_eq!(claims.iss, http.endpoint().origin());
    assert_eq!(claims.aud, ORIGIN);
    assert_eq!(claims.account(), Some(identity.account));
    assert_eq!(claims.exp, expires_at);
    assert_eq!(claims.exp - claims.iat, GATEWAY_TOKEN_SECONDS);
    assert!(
        claims.legacy_id.is_none(),
        "a registered gateway learns no old id"
    );
    // Offered anywhere else it is refused: at another gateway, and at Central.
    let origin = http.endpoint().origin();
    assert_eq!(
        jwt::verify::<GatewayClaims>(
            token.expose(),
            &http.keys(),
            Expected {
                issuer: origin,
                audience: OTHER,
                now: now(),
            },
        )
        .err(),
        Some(JwtError::Audience)
    );
    for request in [
        Request::SessionStatus,
        Request::ReadDocument,
        Request::Logout,
    ] {
        assert_eq!(status_with(http, token.expose(), &request), 401);
    }
    // A refresh is a new token for the same session at this gateway; another gateway has
    // another `sid`, so that the two cannot tell they saw one player.
    let (again, _) = session.gateway_token(http, ORIGIN.into(), Admit).unwrap();
    let refreshed = http.gateway_claims(&again, ORIGIN);
    assert_eq!(refreshed.sid, claims.sid);
    assert_ne!(refreshed.jti, claims.jti);
    let (elsewhere, _) = session.gateway_token(http, OTHER.into(), Admit).unwrap();
    assert_ne!(http.gateway_claims(&elsewhere, OTHER).sid, claims.sid);
    // Administration is asked for apart, and needs membership.
    assert_eq!(
        session.gateway_token(http, ORIGIN.into(), Admin).err(),
        Some(Error::Rejected)
    );
    // No token for a gateway Central does not know, and none the official one denies.
    assert_eq!(
        session
            .gateway_token(http, "https://attacker.invalid".into(), Admit)
            .err(),
        Some(Error::Rejected)
    );
    assert_eq!(
        session.gateway_token(http, OFFICIAL.into(), Admin).err(),
        Some(Error::Rejected)
    );
}

fn a_bearer_keeps_to_its_own_side(http: &Http, central: &str, credential: &Token, beat: &Token) {
    let bare = credential.hex();
    // A gateway's credential is no client.
    for request in [
        Request::SessionStatus,
        Request::ReadDocument,
        Request::Logout,
    ] {
        assert_eq!(status_with(http, &bare, &request), 401);
    }
    // A client's session is no gateway, for any of what gateways ask.
    for request in [
        Request::GatewayRevocations { after: 0 },
        Request::GatewayStatus,
        Request::RegisterInvalidate { path: PATH.into() },
        Request::GatewayHeartbeat {
            metadata: metadata(ORIGIN),
        },
    ] {
        assert_eq!(status_with(http, central, &request), 401);
    }
    // A heartbeat credential reads no revocations.
    assert_eq!(
        http.exchange(&Request::GatewayRevocations { after: 0 }, Some(beat.into()))
            .err(),
        Some(Error::Rejected)
    );
}

fn revocations_reach_the_gateway(
    http: &Http,
    bridge: &Token,
    session: Session,
    credential: &Token,
    heartbeat: &Token,
    other: &Token,
    runtime: &Runtime,
) {
    let (token, _) = session.gateway_token(http, ORIGIN.into(), Admit).unwrap();
    let sid = http.gateway_claims(&token, ORIGIN).sid;
    let (token, _) = session.gateway_token(http, OTHER.into(), Admit).unwrap();
    let other_sid = http.gateway_claims(&token, OTHER).sid;
    let (token, _) = session.gateway_token(http, OFFICIAL.into(), Admit).unwrap();
    let official_sid = http.gateway_claims(&token, OFFICIAL).sid;
    let (_, official_before, _) = http.revoked(bridge, OFFICIAL, u64::MAX >> 1);
    assert_eq!(
        http.revoked(credential, ORIGIN, 0).1,
        0,
        "nothing ended yet"
    );

    session.logout(http).unwrap();

    // Catch-up: each gateway reads its own stream, and learns only its own `sid`.
    assert_eq!(
        http.revoked(credential, ORIGIN, 0),
        (vec![sid.clone()], 1, 1)
    );
    assert_eq!(http.revoked(other, OTHER, 0).0, vec![other_sid]);
    assert_eq!(
        http.revoked(bridge, OFFICIAL, official_before).0,
        vec![official_sid]
    );
    // The heartbeat names the number to catch up to.
    let Response::GatewayHeartbeat { last_revocation } = http
        .exchange(
            &Request::GatewayHeartbeat {
                metadata: metadata(ORIGIN),
            },
            Some(heartbeat.into()),
        )
        .unwrap()
    else {
        panic!("a heartbeat's answer");
    };
    assert_eq!(last_revocation, 1);

    // A push: to the approved origin and the registered path, naming only that `sid`.
    let delivery = Arc::new(Recorded {
        answer: Delivery::Taken,
        calls: Mutex::new(Vec::new()),
    });
    let mut pushed = 0;
    while runtime
        .block_on(http.identity.push_revocations(delivery.clone()))
        .unwrap()
    {
        pushed += 1;
        assert!(
            pushed < 16,
            "a gateway that took its page is not called again"
        );
    }
    let calls = delivery.calls.lock().unwrap();
    assert_eq!(calls.len(), 1, "the one gateway approved with a path");
    let (url, message) = &calls[0];
    assert_eq!(url, &format!("{ORIGIN}{PATH}"));
    let origin = http.endpoint().origin();
    let expected = Expected {
        issuer: origin,
        audience: ORIGIN,
        now: now(),
    };
    let claims: RevocationClaims = jwt::verify(message, &http.keys(), expected).ok().unwrap();
    assert_eq!(claims.sids, vec![sid]);
    assert_eq!((claims.from, claims.seq), (1, 1));
    // Addressed to one gateway, it is refused at another, and taken once.
    assert_eq!(
        jwt::verify::<RevocationClaims>(
            message,
            &http.keys(),
            Expected {
                audience: OTHER,
                ..expected
            },
        )
        .err(),
        Some(JwtError::Audience)
    );
    let mut cache = ReplayCache::new(8);
    cache.take(&claims, now()).unwrap();
    assert_eq!(cache.take(&claims, now()), Err(JwtError::Replayed));
    // It is no gateway token, whatever it is offered as.
    assert_eq!(
        jwt::verify::<GatewayClaims>(message, &http.keys(), expected).err(),
        Some(JwtError::Kind)
    );
}

/// Signed by Central and hours from its `exp`, the token of a session that ended opens nothing:
/// the signature says who issued it, and the session's row whether it is live.
fn a_token_outlives_no_session(http: &Http, central: &str) {
    let origin = http.endpoint().origin();
    let verified = jwt::verify::<CentralClaims>(
        central,
        &http.keys(),
        Expected {
            issuer: origin,
            audience: origin,
            now: now(),
        },
    );
    assert!(verified.is_ok(), "the signature is still good");
    for request in [
        Request::SessionStatus,
        Request::RenewSession,
        Request::ReadDocument,
        Request::TermsStatus,
        Request::CreateObject,
        Request::GatewayToken {
            gateway: ORIGIN.into(),
            purpose: Admit,
        },
    ] {
        assert_eq!(status_with(http, central, &request), 401);
    }
}
