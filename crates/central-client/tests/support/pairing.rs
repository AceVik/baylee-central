//! Pairing a signed-in website with Baylee's client over the real listeners
//! (`docs/web-accounts.md` §"Pairing"), the page's half as the website's
//! WebAssembly crypto runs it and the client's through the SDK. The SDK
//! stores the key check after a password sign-in; the page starts a pairing,
//! checks the client's claim and approves with the master key sealed to the
//! client and a proof under the key check; the client redeems once for a
//! native session and the account's name and tag. The identity service
//! refuses a wrong proof, a proof over another seal, wrong lengths, a second
//! approval and a wrong verifier, and answers a resent claim the same; a seal
//! the client cannot open, to a stranger's key or bound to another expiry,
//! ends the session it came with: the SDK signs it out.
use super::{
    Http,
    browser::Browser,
    password::{self, OLD, Registered},
};
use central_api::{
    Endpoint, Error, Token,
    wire::{Request, Response},
};
use central_client::{Pairing, PairingSecret, Transport};
use central_crypto::{
    pairing::{self, ClientPairing, SEALED_LEN, SealBinding},
    vault::KeyCheck,
};
use sea_orm::{ConnectionTrait, DatabaseConnection};
use std::cell::Cell;
use tokio::runtime::Runtime;

/// The native transport, counting the sign-outs that carry a session.
struct Counting<'a> {
    http: &'a Http,
    logouts: Cell<u32>,
}

impl Transport for Counting<'_> {
    fn endpoint(&self) -> &Endpoint {
        self.http.endpoint()
    }
    fn exchange(&self, request: &Request, bearer: Option<&Token>) -> Result<Response, Error> {
        if matches!(request, Request::Logout) && bearer.is_some() {
            self.logouts.set(self.logouts.get() + 1);
        }
        self.http.exchange(request, bearer)
    }
}

/// The page's view of a claimed pairing.
struct Claimed {
    handle: Token,
    expires_at: i64,
    public_key: [u8; 32],
    challenge: [u8; 32],
    mac: Vec<u8>,
}

impl Claimed {
    fn bytes(&self) -> [u8; 32] {
        *self.handle.to_secret_bytes()
    }

    fn binding(&self, account: &Registered, expires_at: i64) -> SealBinding {
        SealBinding {
            context: account.context,
            handle: self.bytes(),
            expires_at,
        }
    }

    fn proof(&self, check: &KeyCheck, sealed: &[u8; SEALED_LEN]) -> Vec<u8> {
        pairing::approval_proof(
            check,
            &self.bytes(),
            &self.public_key,
            &self.challenge,
            sealed,
        )
        .to_vec()
    }

    fn approve(&self, sealed: &[u8], proof: Vec<u8>) -> Request {
        Request::PairApprove {
            handle: copy(&self.handle),
            sealed: sealed.to_vec(),
            proof,
        }
    }
}

fn copy(token: &Token) -> Token {
    Token::from_secret_bytes(token.to_secret_bytes())
}

/// The live native sessions of the account `pairing-web`.
fn natives(runtime: &Runtime, operator: &DatabaseConnection) -> i64 {
    runtime
        .block_on(operator.query_one_raw(sea_orm::Statement::from_string(
            sea_orm::DbBackend::Postgres,
            "SELECT count(*) AS count FROM central_identity.sessions s
             JOIN central_identity.accounts a ON a.id = s.account_id
             WHERE a.username_key = 'pairing-web' AND s.kind = 'native'
             AND s.expires_at > pg_catalog.clock_timestamp()",
        )))
        .unwrap()
        .unwrap()
        .try_get("", "count")
        .unwrap()
}

pub(crate) fn run(
    http: &Http,
    browser: &Browser,
    runtime: &Runtime,
    operator: &DatabaseConnection,
) {
    // Its own address: the other fixtures spent the shared one's registrations.
    let browser = &Browser {
        address: browser.address,
        client: "198.51.100.31",
    };
    let account = password::register("pairing-web", http, browser, runtime, operator);
    let check = account.master.key_check(account.context).unwrap();

    // After a password sign-in the SDK stores the key check, once.
    let mut native = central_client::login(http, account.identity, OLD.into()).unwrap();
    native.ensure_key_check(http).unwrap();
    native.ensure_key_check(http).unwrap();
    let cookie = password::browser_sign_in(browser, account.identity, account.context, OLD);
    assert!(matches!(
        browser.answer(&Request::KeyCheckStatus, Some(&cookie)),
        Ok(Response::KeyCheck { set: true })
    ));
    assert_eq!(
        browser
            .answer(&Request::SetKeyCheck { check: vec![9; 32] }, Some(&cookie))
            .err(),
        Some(Error::Conflict),
        "the cookie's holder cannot replace the check"
    );

    let (client, claimed) = claimed(http, browser, &cookie);
    resent(http, &client, &claimed);
    approved(http, browser, &account, &check, &cookie, &client, &claimed);
    redeemed(http, &account, &claimed, client, &mut native);
    wrong_verifier(http, browser, &account, &check, &cookie);
    unopened(http, browser, runtime, operator, &account, &check, &cookie);
}

/// The page starts a pairing; the SDK claims it; the page reads the claim
/// and checks its MAC and code.
fn claimed(http: &Http, browser: &Browser, cookie: &Token) -> (Pairing, Claimed) {
    let Ok(Response::PairStarted { handle, expires_at }) =
        browser.answer(&Request::PairStart, Some(cookie))
    else {
        panic!("a pairing");
    };
    let secret = PairingSecret::generate().unwrap();
    let client = central_client::claim(http, *handle.to_secret_bytes(), &secret).unwrap();
    assert_eq!(client.expires_at(), expires_at);
    let Ok(Response::PairClaimed {
        public_key,
        challenge,
        mac,
    }) = browser.answer(
        &Request::PairStatus {
            handle: copy(&handle),
        },
        Some(cookie),
    )
    else {
        panic!("the claim");
    };
    let claimed = Claimed {
        handle,
        expires_at,
        public_key: public_key.try_into().unwrap(),
        challenge: challenge.try_into().unwrap(),
        mac,
    };
    assert!(pairing::claim_ok(
        &secret,
        &http.endpoint().issuer(),
        &claimed.bytes(),
        &claimed.public_key,
        &claimed.challenge,
        &claimed.mac,
    ));
    assert_eq!(
        pairing::comparison_code(&claimed.bytes(), &claimed.public_key),
        client.comparison_code(),
        "both screens show one code"
    );
    (client, claimed)
}

/// A lost answer's resend is the same claim; any other is a conflict.
fn resent(http: &Http, client: &Pairing, claimed: &Claimed) {
    let resend = |mac: Vec<u8>| Request::PairClaim {
        handle: copy(&claimed.handle),
        public_key: claimed.public_key.to_vec(),
        challenge: claimed.challenge.to_vec(),
        mac,
    };
    assert!(matches!(
        http.exchange(&resend(claimed.mac.clone()), None),
        Ok(Response::PairAccepted { expires_at }) if expires_at == claimed.expires_at
    ));
    assert_eq!(
        http.exchange(&resend(vec![0; 32]), None).err(),
        Some(Error::Conflict)
    );
    assert!(!client.approved(http).unwrap(), "not approved yet");
}

/// Approval through the identity service: only the right proof over this
/// seal, once.
fn approved(
    http: &Http,
    browser: &Browser,
    account: &Registered,
    check: &KeyCheck,
    cookie: &Token,
    client: &Pairing,
    claimed: &Claimed,
) {
    let sealed = account
        .master
        .seal_for_pairing(
            &claimed.public_key,
            &claimed.binding(account, claimed.expires_at),
        )
        .unwrap();
    let other = account
        .master
        .seal_for_pairing(
            &claimed.public_key,
            &claimed.binding(account, claimed.expires_at),
        )
        .unwrap();
    let stranger = KeyCheck::from_bytes(&[9; 32]).unwrap();
    for (request, why) in [
        (
            claimed.approve(&sealed, claimed.proof(&stranger, &sealed)),
            "a proof under another key check, as a stolen cookie could make it",
        ),
        (
            claimed.approve(&sealed, claimed.proof(check, &other)),
            "a proof over another seal",
        ),
        (
            claimed.approve(&sealed[..SEALED_LEN - 1], claimed.proof(check, &sealed)),
            "a seal one byte short",
        ),
        (
            claimed.approve(&sealed, claimed.proof(check, &sealed)[..31].to_vec()),
            "a proof one byte short",
        ),
        (
            Request::PairApprove {
                handle: Token::generate().unwrap(),
                sealed: sealed.to_vec(),
                proof: claimed.proof(check, &sealed),
            },
            "another handle",
        ),
    ] {
        assert_eq!(
            browser.answer(&request, Some(cookie)).err(),
            Some(Error::Rejected),
            "{why}"
        );
    }
    assert!(!client.approved(http).unwrap(), "no refusal approved it");
    assert!(matches!(
        browser.answer(
            &claimed.approve(&sealed, claimed.proof(check, &sealed)),
            Some(cookie)
        ),
        Ok(Response::Done)
    ));
    assert_eq!(
        browser
            .answer(
                &claimed.approve(&other, claimed.proof(check, &other)),
                Some(cookie)
            )
            .err(),
        Some(Error::Rejected),
        "approved once"
    );
    assert!(client.approved(http).unwrap());
}

/// The redeem: a verifier of the wrong length is refused before the store;
/// the right one gives the session, which replaces the account's earlier
/// native one, and the account's name and tag; once.
fn redeemed(
    http: &Http,
    account: &Registered,
    claimed: &Claimed,
    client: Pairing,
    native: &mut central_client::Session,
) {
    assert_eq!(
        http.exchange(
            &Request::PairRedeem {
                handle: copy(&claimed.handle),
                verifier: vec![0; 31],
            },
            None,
        )
        .err(),
        Some(Error::Rejected)
    );
    let (paired, named) = client.redeem(http).unwrap();
    let identity = paired.identity();
    assert_eq!(
        (identity.account, identity.epoch),
        (account.identity.account, account.identity.epoch)
    );
    assert_eq!(named.username, "pairing-web");
    assert_eq!(named.display_name.as_deref(), Some("Pass_Web"));
    assert!(named.tag.is_some(), "drawn at the account's first sign-in");
    assert_eq!(
        native.refresh(http).err(),
        Some(Error::Rejected),
        "the paired session replaced the earlier native one"
    );
    assert_eq!(
        http.exchange(
            &Request::PairRedeem {
                handle: copy(&claimed.handle),
                verifier: vec![0; 32],
            },
            None,
        )
        .err(),
        Some(Error::Rejected),
        "redeemed once"
    );
}

/// A wrong verifier ends the pairing: the client's own redeem comes too late.
fn wrong_verifier(
    http: &Http,
    browser: &Browser,
    account: &Registered,
    check: &KeyCheck,
    cookie: &Token,
) {
    let (client, claimed) = claimed(http, browser, cookie);
    let sealed = account
        .master
        .seal_for_pairing(
            &claimed.public_key,
            &claimed.binding(account, claimed.expires_at),
        )
        .unwrap();
    assert!(matches!(
        browser.answer(
            &claimed.approve(&sealed, claimed.proof(check, &sealed)),
            Some(cookie)
        ),
        Ok(Response::Done)
    ));
    assert_eq!(
        http.exchange(
            &Request::PairRedeem {
                handle: copy(&claimed.handle),
                verifier: vec![7; 32],
            },
            None,
        )
        .err(),
        Some(Error::Rejected)
    );
    assert_eq!(client.redeem(http).err(), Some(Error::Rejected));
}

/// A seal the client cannot open, though Central took its proof: sealed to a
/// stranger's key, or bound to another expiry. The SDK refuses it and signs
/// out the session it came with, so the account keeps no native session.
fn unopened(
    http: &Http,
    browser: &Browser,
    runtime: &Runtime,
    operator: &DatabaseConnection,
    account: &Registered,
    check: &KeyCheck,
    cookie: &Token,
) {
    let stranger = ClientPairing::generate().unwrap();
    for (to_stranger, why) in [
        (true, "sealed to a stranger's key"),
        (false, "another expiry"),
    ] {
        let (client, claimed) = claimed(http, browser, cookie);
        let (key, expires_at) = if to_stranger {
            (stranger.public_key(), claimed.expires_at)
        } else {
            (&claimed.public_key, claimed.expires_at + 1)
        };
        let sealed = account
            .master
            .seal_for_pairing(key, &claimed.binding(account, expires_at))
            .unwrap();
        assert!(matches!(
            browser.answer(
                &claimed.approve(&sealed, claimed.proof(check, &sealed)),
                Some(cookie)
            ),
            Ok(Response::Done)
        ));
        let counting = Counting {
            http,
            logouts: Cell::new(0),
        };
        assert_eq!(
            client.redeem(&counting).err(),
            Some(Error::Rejected),
            "{why}"
        );
        assert_eq!(counting.logouts.get(), 1, "{why}: the session signed out");
        assert_eq!(
            natives(runtime, operator),
            0,
            "{why}: no native session left"
        );
    }
}
