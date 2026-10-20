//! Terms of use over real HTTP (`docs/terms.md`): Central's terms in force
//! hold a fresh account back until it accepts exactly the current text; a
//! gateway publishes immutable versions, its players accept them, and the
//! gateway learns per lease whether they did.
use super::Http;
use central_api::{
    Error, Token,
    terms::{self, TermsDocument, TermsSubject, TermsText, TermsVersion},
    wire::{Request, Response, TicketPurpose},
};
use central_client::{Session, Transport};

fn texts(title: &str) -> Vec<TermsText> {
    ["de", "en"]
        .iter()
        .map(|lang| TermsText {
            lang: (*lang).into(),
            updated: Some("2026-10-20".into()),
            markdown: format!("# {title} ({lang})\n\nSynthetic terms.\n"),
        })
        .collect()
}

fn version(name: &str, texts: &[TermsText]) -> TermsVersion {
    TermsVersion {
        version: name.into(),
        digest: terms::digest(texts),
    }
}

/// The Central terms the fixture's service has in force.
pub(crate) fn central_document() -> TermsDocument {
    let texts = texts("Central");
    TermsDocument {
        version: version("2026-10-test", &texts),
        texts,
    }
}

/// Run between restoring the account and asking for its first gateway ticket.
pub(crate) fn central(http: &Http, session: &Session, official: &str) {
    let document = central_document();
    assert_eq!(
        central_client::central_terms(http).unwrap(),
        Some(document.clone()),
        "the terms in force are public"
    );
    let status = session.terms_status(http).unwrap();
    assert!(!status.accepted);
    assert_eq!(status.current, Some(document.version.clone()));
    assert!(matches!(
        session.gateway_ticket(http, official.into()),
        Err(Error::TermsRequired)
    ));
    assert!(matches!(
        session.create_object(http),
        Err(Error::TermsRequired)
    ));
    // A bearer that is no session learns nothing about terms.
    assert!(matches!(
        http.exchange(&Request::CreateObject, Some(&Token::generate().unwrap())),
        Err(Error::Rejected)
    ));
    // Reading one's own data stays possible.
    session.check_active(http).unwrap();
    let mut stale = document.version.clone();
    stale.digest[0] ^= 1;
    assert_eq!(
        session.accept_terms(http, TermsSubject::Central, &stale),
        Err(Error::Conflict)
    );
    let mut renamed = document.version.clone();
    renamed.version = "2026-10-other".into();
    assert_eq!(
        session.accept_terms(http, TermsSubject::Central, &renamed),
        Err(Error::Conflict)
    );
    assert!(!session.terms_status(http).unwrap().accepted);
    session
        .accept_terms(http, TermsSubject::Central, &document.version)
        .unwrap();
    session
        .accept_terms(http, TermsSubject::Central, &document.version)
        .expect("accepting again after a lost reply is no conflict");
    assert!(session.terms_status(http).unwrap().accepted);
}

fn publish(
    http: &Http,
    bridge: &Token,
    name: &str,
    texts: &[TermsText],
) -> Result<Response, Error> {
    http.exchange(
        &Request::PublishGatewayTerms {
            version: name.into(),
            texts: texts.to_vec(),
        },
        Some(bridge),
    )
}

fn authorize(
    http: &Http,
    bridge: &Token,
    lease: &Token,
    terms: Option<TermsVersion>,
) -> (bool, bool) {
    let Response::GatewayAuthorization {
        central_terms,
        gateway_terms,
        ..
    } = http
        .exchange(
            &Request::GatewayAuthorize {
                lease: Token::from_secret_bytes(lease.to_secret_bytes()),
                purpose: TicketPurpose::Admit,
                terms,
            },
            Some(bridge),
        )
        .unwrap()
    else {
        panic!("an authorization");
    };
    (central_terms, gateway_terms)
}

/// The official gateway (the bridge's origin) publishes and its players accept.
pub(crate) fn gateway(http: &Http, session: &Session, bridge: &Token, official: &str) {
    let first = published(http, bridge, official);
    accepted(http, session, bridge, official, &first);
}

/// Only the gateway publishes, a version never changes, and Central serves
/// exactly what was published.
fn published(http: &Http, bridge: &Token, official: &str) -> TermsVersion {
    assert!(
        central_client::gateway_terms(http, official)
            .unwrap()
            .is_none()
    );
    let first_texts = texts("Gateway one");
    let first = version("g-1", &first_texts);
    for refused in [
        publish(http, &Token::generate().unwrap(), "g-1", &first_texts),
        publish(http, bridge, "draft", &first_texts),
        publish(http, bridge, "g 1", &first_texts),
        publish(
            http,
            bridge,
            "g-1",
            &first_texts.iter().rev().cloned().collect::<Vec<_>>(),
        ),
    ] {
        assert!(matches!(refused, Err(Error::Rejected)));
    }
    assert!(
        http.exchange(
            &Request::PublishGatewayTerms {
                version: "g-1".into(),
                texts: first_texts.clone()
            },
            None
        )
        .is_err()
    );
    for _ in 0..2 {
        let Response::TermsPublished { digest } =
            publish(http, bridge, "g-1", &first_texts).unwrap()
        else {
            panic!("published");
        };
        assert_eq!(digest, first.digest, "publishing again is idempotent");
    }
    assert!(matches!(
        publish(
            http,
            bridge,
            "g-1",
            &texts("Changed under the same version")
        ),
        Err(Error::Conflict)
    ));
    let served = central_client::gateway_terms(http, official)
        .unwrap()
        .expect("published terms are public");
    assert_eq!(served.version, first);
    assert_eq!(served.texts, first_texts);
    first
}

/// Players accept the current version; the gateway learns per lease whether
/// they accepted its own current one.
fn accepted(http: &Http, session: &Session, bridge: &Token, official: &str, first: &TermsVersion) {
    let first = first.clone();
    let ticket = session.gateway_ticket(http, official.into()).unwrap();
    let Response::GatewayIdentity {
        lease: Some(lease), ..
    } = http
        .exchange(
            &Request::RedeemGateway {
                ticket,
                purpose: TicketPurpose::Admit,
            },
            Some(bridge),
        )
        .unwrap()
    else {
        panic!("a lease");
    };
    assert_eq!(authorize(http, bridge, &lease, None), (true, true));
    assert_eq!(
        authorize(http, bridge, &lease, Some(first.clone())),
        (true, false),
        "not accepted yet"
    );
    let gateway = || TermsSubject::Gateway {
        origin: official.into(),
    };
    let mut forged = first.clone();
    forged.digest[31] ^= 1;
    assert_eq!(
        session.accept_terms(http, gateway(), &forged),
        Err(Error::Conflict)
    );
    assert_eq!(
        session.accept_terms(
            http,
            TermsSubject::Gateway {
                origin: "http://127.0.0.1:28799".into()
            },
            &first
        ),
        Err(Error::Conflict),
        "a gateway that published nothing has nothing to accept"
    );
    session.accept_terms(http, gateway(), &first).unwrap();
    assert_eq!(
        authorize(http, bridge, &lease, Some(first.clone())),
        (true, true)
    );
    assert_eq!(
        authorize(http, bridge, &lease, Some(forged.clone())),
        (true, false),
        "the gateway's own version decides, digest included"
    );

    // A new version is current at once; the old acceptance no longer covers it.
    let second_texts = texts("Gateway two");
    let second = version("g-2", &second_texts);
    publish(http, bridge, "g-2", &second_texts).unwrap();
    assert_eq!(
        central_client::gateway_terms(http, official)
            .unwrap()
            .unwrap()
            .version,
        second
    );
    assert_eq!(
        authorize(http, bridge, &lease, Some(second.clone())),
        (true, false)
    );
    assert_eq!(
        session.accept_terms(http, gateway(), &first),
        Err(Error::Conflict),
        "only the current version can be accepted"
    );
    session.accept_terms(http, gateway(), &second).unwrap();
    assert_eq!(authorize(http, bridge, &lease, Some(second)), (true, true));
    // The older lease check still works for gateways that do not know terms.
    assert!(matches!(
        http.exchange(
            &Request::GatewaySession {
                lease,
                purpose: TicketPurpose::Admit
            },
            Some(bridge)
        )
        .unwrap(),
        Response::GatewaySession { .. }
    ));
}
