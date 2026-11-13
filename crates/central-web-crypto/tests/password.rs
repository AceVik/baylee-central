//! A reset with an administrator's code and a password change as the page
//! runs them, against a Central in memory that this module and
//! `central-client` (the native SDK) both talk to: what one sets the other
//! signs in with, and what one keeps the other opens. Real serialized
//! messages and the fixed Argon2 profile; synthetic accounts only.
#![forbid(unsafe_code)]

use std::cell::RefCell;

use central_api::{
    Bearer, Endpoint, Error, SignedToken, Token,
    reset_code::ResetCode,
    wire::{Discovery, Identity, Request, ResetVault, Response},
};
use central_client::{ResetData, Session, Transport, login, reset};
use central_crypto::{
    Context,
    opaque::server::{Attempt, Record, ServerKeys},
    vault::{RecoveryCode, WrappedMasterKey},
};
use central_web_crypto::flow::{
    self, DOCUMENT, SignedIn, enrollment_finish, enrollment_start, login_confirm, login_finish,
    login_start, password_finish, password_start, reset_keep, reset_new, reset_start,
};

const USERNAME: &str = "erika";
const ACCOUNT: [u8; 16] = [
    0x01, 0x92, 0xd0, 0xc5, 0x7b, 0x3e, 0x7a, 0x1f, 0x8c, 0x2d, 0x3e, 0x4f, 0x5a, 0x6b, 0x7c, 0x8d,
];
const FIRST: &str = "synthetic test-only password";
const SECOND: &str = "synthetic second password";
const THIRD: &str = "synthetic third password";

fn context(epoch: u64) -> Context {
    Context::new(Endpoint::live().issuer(), ACCOUNT, epoch).unwrap()
}

/// What Central stores of the one account.
struct Stored {
    epoch: u64,
    record: Record,
    wrapped_master: Vec<u8>,
    recovery: Option<Vec<u8>>,
    document: Vec<u8>,
    /// The digest of an outstanding reset code.
    code: Option<[u8; 32]>,
    /// The digest of the live session's token.
    session: Option<[u8; 32]>,
}

/// An exchange between its two requests.
enum Pending {
    Login(Attempt),
    Change(Attempt),
    Reset,
}

/// Central's part of the requests a reset, a change and a sign-in make, for
/// one account, as `central-identity` answers them (`docs/account-reset.md`,
/// `docs/native-login.md` §"Password change"): no budgets and no clock.
struct Central {
    endpoint: Endpoint,
    keys: ServerKeys,
    stored: RefCell<Stored>,
    pending: RefCell<Option<([u8; 32], Pending)>>,
}

impl Central {
    /// A Central holding the account as the page registers it at epoch 1,
    /// and the recovery code the page showed.
    fn registered() -> (Self, String) {
        let keys = ServerKeys::generate();
        let (state, request) = enrollment_start(FIRST.into(), &ACCOUNT, 1).unwrap();
        let response = keys.enrollment_response(context(1), &request).unwrap();
        let registered = enrollment_finish(state, &response).unwrap();
        let central = Self {
            endpoint: Endpoint::live(),
            stored: RefCell::new(Stored {
                epoch: 1,
                record: ServerKeys::enrollment_record(&registered.record).unwrap(),
                wrapped_master: registered.wrapped_master.as_bytes().to_vec(),
                recovery: Some(registered.recovery.as_bytes().to_vec()),
                document: registered.document.clone(),
                code: None,
                session: None,
            }),
            keys,
            pending: RefCell::new(None),
        };
        (central, registered.recovery_code.to_string())
    }

    fn identity(&self) -> Identity {
        Identity {
            account: ACCOUNT,
            epoch: self.stored.borrow().epoch,
        }
    }

    /// An administrator's reset code, as the person types it.
    fn issue(&self) -> String {
        let code = ResetCode::generate().unwrap();
        self.stored.borrow_mut().code = Some(code.digest());
        code.display().to_string()
    }

    fn pend(&self, pending: Pending) -> Token {
        let attempt = Token::generate().unwrap();
        *self.pending.borrow_mut() = Some((attempt.digest(), pending));
        attempt
    }

    fn take(&self, attempt: &Token) -> Result<Pending, Error> {
        match self.pending.borrow_mut().take() {
            Some((digest, pending)) if digest == attempt.digest() => Ok(pending),
            _ => Err(Error::Rejected),
        }
    }

    /// A client's own session, as its signed bearer. This stand-in signs
    /// nothing: its bearer is the capability's hex, which a client never
    /// reads, and a bare capability is no session.
    fn live(&self, bearer: Option<Bearer<'_>>) -> Result<(), Error> {
        let session = self.stored.borrow().session;
        let Some(Bearer::Signed(bearer)) = bearer else {
            return Err(Error::Rejected);
        };
        match session {
            Some(session) if session == Token::from_hex(bearer.expose())?.digest() => Ok(()),
            _ => Err(Error::Rejected),
        }
    }

    /// One new session, in place of any other.
    fn session(&self) -> Response {
        let capability = Token::generate().unwrap();
        let mut stored = self.stored.borrow_mut();
        stored.session = Some(capability.digest());
        Response::Session {
            token: SignedToken::new(capability.hex()).unwrap(),
            identity: Identity {
                account: ACCOUNT,
                epoch: stored.epoch,
            },
            wrapped_master: stored.wrapped_master.clone(),
        }
    }

    /// Stores a new password's record and wrapper; an outstanding code goes.
    fn set(&self, record: &[u8], wrapped_master: &[u8]) -> Result<(), Error> {
        let record = ServerKeys::enrollment_record(record).map_err(|_| Error::Rejected)?;
        WrappedMasterKey::from_bytes(wrapped_master).map_err(|_| Error::Rejected)?;
        let mut stored = self.stored.borrow_mut();
        stored.record = record;
        stored.wrapped_master = wrapped_master.to_vec();
        stored.code = None;
        stored.session = None;
        Ok(())
    }

    fn login_start(&self, identity: Identity, message: &[u8]) -> Result<Response, Error> {
        let (attempt, message) = {
            let stored = self.stored.borrow();
            if identity.account != ACCOUNT || identity.epoch != stored.epoch {
                return Err(Error::Rejected);
            }
            self.keys
                .start_login(context(stored.epoch), Some(&stored.record), message)
                .map_err(|_| Error::Rejected)?
        };
        Ok(Response::LoginChallenge {
            attempt: self.pend(Pending::Login(attempt)),
            message,
        })
    }

    fn reset_start(
        &self,
        username: &str,
        code: &ResetCode,
        registration: &[u8],
    ) -> Result<Response, Error> {
        let (registration, recovery) = {
            let stored = self.stored.borrow();
            if username != USERNAME || stored.code != Some(code.digest()) {
                return Err(Error::Rejected);
            }
            let response = self
                .keys
                .enrollment_response(context(stored.epoch), registration)
                .map_err(|_| Error::Rejected)?;
            (response, stored.recovery.clone())
        };
        Ok(Response::ResetChallenge {
            attempt: self.pend(Pending::Reset),
            identity: self.identity(),
            registration,
            recovery,
        })
    }

    fn reset_finish(
        &self,
        record: &[u8],
        wrapped_master: &[u8],
        vault: &ResetVault,
    ) -> Result<Response, Error> {
        self.set(record, wrapped_master)?;
        if let ResetVault::New { document, recovery } = vault {
            let mut stored = self.stored.borrow_mut();
            stored.epoch += 1;
            stored.document.clone_from(document);
            stored.recovery.clone_from(recovery);
        }
        Ok(Response::Done)
    }

    fn change_start(&self, login: &[u8], registration: &[u8]) -> Result<Response, Error> {
        let (attempt, login, registration) = {
            let stored = self.stored.borrow();
            let context = context(stored.epoch);
            let (attempt, login) = self
                .keys
                .start_login(context, Some(&stored.record), login)
                .map_err(|_| Error::Rejected)?;
            let registration = self
                .keys
                .enrollment_response(context, registration)
                .map_err(|_| Error::Rejected)?;
            (attempt, login, registration)
        };
        Ok(Response::PasswordChallenge {
            attempt: self.pend(Pending::Change(attempt)),
            login,
            registration,
        })
    }
}

impl Transport for Central {
    fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }

    fn exchange(&self, request: &Request, bearer: Option<Bearer<'_>>) -> Result<Response, Error> {
        match request {
            Request::Discover { username } if username == USERNAME => Ok(Response::Discovery {
                discovery: Discovery::Central {
                    identity: self.identity(),
                },
            }),
            Request::LoginStart { identity, message } => self.login_start(*identity, message),
            Request::LoginFinish { attempt, message } => {
                let Pending::Login(state) = self.take(attempt)? else {
                    return Err(Error::Rejected);
                };
                state.finish(message).map_err(|_| Error::Rejected)?;
                Ok(self.session())
            }
            Request::ReadDocument => {
                self.live(bearer)?;
                Ok(Response::Document {
                    revision: 1,
                    document: self.stored.borrow().document.clone(),
                })
            }
            Request::Logout => {
                self.live(bearer)?;
                self.stored.borrow_mut().session = None;
                Ok(Response::Done)
            }
            Request::ResetStart {
                username,
                code,
                registration,
            } => self.reset_start(username, code, registration),
            Request::ResetFinish {
                attempt,
                record,
                wrapped_master,
                vault,
            } => {
                let Pending::Reset = self.take(attempt)? else {
                    return Err(Error::Rejected);
                };
                self.reset_finish(record, wrapped_master, vault)
            }
            Request::ChangePasswordStart {
                login,
                registration,
            } => {
                self.live(bearer)?;
                self.change_start(login, registration)
            }
            Request::ChangePasswordFinish {
                attempt,
                proof,
                record,
                wrapped_master,
            } => {
                self.live(bearer)?;
                let Pending::Change(state) = self.take(attempt)? else {
                    return Err(Error::Rejected);
                };
                state.finish(proof).map_err(|_| Error::Rejected)?;
                self.set(record, wrapped_master)?;
                Ok(self.session())
            }
            _ => Err(Error::Rejected),
        }
    }
}

/// The page's sign-in up to Central's yes: the Worker's state, the session
/// and the wrapper `LoginFinish` answered.
fn page_signs_in(
    central: &Central,
    password: &str,
) -> Result<(SignedIn, SignedToken, Vec<u8>), Error> {
    let identity = central.identity();
    let (state, message) = login_start(password.into(), &identity.account, identity.epoch)
        .map_err(|_| Error::Rejected)?;
    let Response::LoginChallenge { attempt, message } =
        central.exchange(&Request::LoginStart { identity, message }, None)?
    else {
        return Err(Error::Rejected);
    };
    let (key, message) = login_finish(state, &message).map_err(|_| Error::Rejected)?;
    let Response::Session {
        token,
        wrapped_master,
        ..
    } = central.exchange(&Request::LoginFinish { attempt, message }, None)?
    else {
        return Err(Error::Rejected);
    };
    Ok((key, token, wrapped_master))
}

/// The page's whole sign-in: the wrapper Central answered opens.
fn page_can_sign_in(central: &Central, password: &str) -> bool {
    page_signs_in(central, password)
        .is_ok_and(|(key, _, wrapped)| login_confirm(key, &wrapped).is_ok())
}

/// The SDK's sign-in, which opens the document under the master key.
fn sdk_signs_in(central: &Central, password: &str) -> Result<Session, Error> {
    login(central, central.identity(), password.into())
}

/// The page's reset as the island runs it: discovery, `ResetStart` with the
/// Worker's registration request, the Worker's finish, `ResetFinish`.
/// Returns a new vault's recovery code.
fn page_resets(
    central: &Central,
    code: &str,
    password: &str,
    recovery_code: Option<&str>,
) -> Result<Option<String>, Error> {
    let Response::Discovery {
        discovery: Discovery::Central { identity },
    } = central.exchange(
        &Request::Discover {
            username: USERNAME.into(),
        },
        None,
    )?
    else {
        return Err(Error::Rejected);
    };
    let (state, registration) = reset_start(
        password.into(),
        &identity.account,
        identity.epoch,
        recovery_code.map(str::to_owned),
    )
    .map_err(|_| Error::Rejected)?;
    let Response::ResetChallenge {
        attempt,
        identity: answered,
        registration,
        recovery,
    } = central.exchange(
        &Request::ResetStart {
            username: USERNAME.into(),
            code: ResetCode::parse(code)?,
            registration,
        },
        None,
    )?
    else {
        return Err(Error::Rejected);
    };
    assert_eq!(answered.epoch, identity.epoch);
    let (record, wrapped_master, vault, shown) = if recovery_code.is_some() {
        let kept = reset_keep(state, &registration, &recovery.ok_or(Error::Rejected)?)
            .map_err(|_| Error::Rejected)?;
        (kept.record, kept.wrapped_master, ResetVault::Keep, None)
    } else {
        let made = reset_new(state, &registration).map_err(|_| Error::Rejected)?;
        let vault = ResetVault::New {
            document: made.document,
            recovery: Some(made.recovery.as_bytes().to_vec()),
        };
        let shown = Some(made.recovery_code.to_string());
        (made.record, made.wrapped_master, vault, shown)
    };
    let finish = Request::ResetFinish {
        attempt,
        record,
        wrapped_master: wrapped_master.as_bytes().to_vec(),
        vault,
    };
    match central.exchange(&finish, None)? {
        Response::Done => Ok(shown),
        _ => Err(Error::Rejected),
    }
}

/// The page's password change as the island runs it: a sign-in in the
/// Worker, then the change with the master key that sign-in opened.
fn page_changes(central: &Central, current: &str, new: &str) -> Result<(), Error> {
    let (key, token, wrapped_master) = page_signs_in(central, current)?;
    let (state, login, registration) =
        password_start(key, &wrapped_master, current.into(), new.into())
            .map_err(|_| Error::Rejected)?;
    let Response::PasswordChallenge {
        attempt,
        login,
        registration,
    } = central.exchange(
        &Request::ChangePasswordStart {
            login,
            registration,
        },
        Some((&token).into()),
    )?
    else {
        return Err(Error::Rejected);
    };
    let changed = password_finish(state, &login, &registration).map_err(|_| Error::Rejected)?;
    let sent = changed.wrapped_master.as_bytes().to_vec();
    let Response::Session { wrapped_master, .. } = central.exchange(
        &Request::ChangePasswordFinish {
            attempt,
            proof: changed.proof,
            record: changed.record,
            wrapped_master: sent.clone(),
        },
        Some((&token).into()),
    )?
    else {
        return Err(Error::Rejected);
    };
    assert_eq!(
        wrapped_master, sent,
        "Central answers the wrapper it stored"
    );
    Ok(())
}

#[test]
fn a_reset_with_the_recovery_code_keeps_what_the_sdk_opens() {
    let (central, recovery_code) = Central::registered();
    let code = central.issue();
    // typed back in lower case, as a person may
    let shown = page_resets(&central, &code, SECOND, Some(&recovery_code.to_lowercase())).unwrap();
    assert_eq!(shown, None, "a kept vault keeps its recovery code");
    assert_eq!(central.identity().epoch, 1, "the epoch stays");

    // the SDK signs in with the new password and opens the same document
    let session = sdk_signs_in(&central, SECOND).unwrap();
    assert_eq!(session.document(), DOCUMENT);
    assert!(sdk_signs_in(&central, FIRST).is_err(), "the old password");
    assert!(page_can_sign_in(&central, SECOND));
    assert!(!page_can_sign_in(&central, FIRST));

    // the recovery wrapper is untouched, and the code is spent
    let wrapper = central.stored.borrow().recovery.clone().unwrap();
    let wrapper = WrappedMasterKey::from_bytes(&wrapper).unwrap();
    RecoveryCode::parse(&recovery_code)
        .unwrap()
        .unwrap(context(1), &wrapper)
        .unwrap();
    assert!(page_resets(&central, &code, THIRD, Some(&recovery_code)).is_err());
    assert!(page_can_sign_in(&central, SECOND));
}

#[test]
fn a_reset_to_a_new_vault_makes_what_the_sdk_opens_and_a_code_the_sdk_takes() {
    let (central, old_code) = Central::registered();
    let old_wrapper = central.stored.borrow().wrapped_master.clone();
    let code = central.issue();
    let new_code = page_resets(&central, &code, SECOND, None)
        .unwrap()
        .expect("a new vault shows a new recovery code");
    assert_ne!(new_code, old_code);
    assert_eq!(central.identity().epoch, 2, "the next epoch");
    assert_ne!(central.stored.borrow().wrapped_master, old_wrapper);

    // the SDK opens the new vault's first document at the new epoch
    let session = sdk_signs_in(&central, SECOND).unwrap();
    assert_eq!(session.identity().epoch, 2);
    assert_eq!(session.document(), DOCUMENT);
    assert!(sdk_signs_in(&central, FIRST).is_err());
    assert!(page_can_sign_in(&central, SECOND));

    // the old recovery code opens nothing of the new vault; the new one does
    let wrapper = central.stored.borrow().recovery.clone().unwrap();
    let wrapper = WrappedMasterKey::from_bytes(&wrapper).unwrap();
    let old = RecoveryCode::parse(&old_code).unwrap();
    assert!(old.unwrap(context(2), &wrapper).is_err());
    assert!(old.unwrap(context(1), &wrapper).is_err());

    // the SDK's own reset keeps the vault with the code the page showed ...
    let next = ResetCode::parse(&central.issue()).unwrap();
    let shown = RecoveryCode::parse(&new_code).unwrap();
    let kept = reset(
        &central,
        USERNAME.into(),
        next,
        THIRD.into(),
        ResetData::Recovery(&shown),
    )
    .unwrap();
    assert!(kept.is_none());
    // ... and the page signs in to what the SDK set
    assert!(page_can_sign_in(&central, THIRD));
    assert_eq!(sdk_signs_in(&central, THIRD).unwrap().document(), DOCUMENT);
}

#[test]
fn the_page_keeps_a_vault_the_sdk_made_new() {
    let (central, _) = Central::registered();
    let code = ResetCode::parse(&central.issue()).unwrap();
    let made = reset(
        &central,
        USERNAME.into(),
        code,
        SECOND.into(),
        ResetData::New(b"{\"synthetic\":true}"),
    )
    .unwrap()
    .expect("the SDK's new vault has a recovery code");
    assert!(page_can_sign_in(&central, SECOND));

    let code = central.issue();
    page_resets(&central, &code, THIRD, Some(&made.to_text())).unwrap();
    let session = sdk_signs_in(&central, THIRD).unwrap();
    assert_eq!(session.identity().epoch, 2);
    assert_eq!(session.document(), b"{\"synthetic\":true}");
}

#[test]
fn a_wrong_recovery_code_fails_before_the_reset_code_is_spent() {
    let (central, recovery_code) = Central::registered();
    let code = central.issue();
    // a well-formed code of another account: refused in the Worker
    let other = RecoveryCode::generate().unwrap().to_text().to_string();
    assert!(page_resets(&central, &code, SECOND, Some(&other)).is_err());
    assert!(
        central.stored.borrow().code.is_some(),
        "the code is unspent"
    );
    assert!(page_can_sign_in(&central, FIRST), "nothing changed");
    // the right one still works with the same reset code
    page_resets(&central, &code, SECOND, Some(&recovery_code)).unwrap();
    assert!(page_can_sign_in(&central, SECOND));
}

#[test]
fn a_reset_is_refused_before_anything_is_sent() {
    let (_, recovery_code) = Central::registered();
    let start = |password: &str, epoch: u64, code: Option<&str>| {
        reset_start(password.into(), &ACCOUNT, epoch, code.map(str::to_owned)).is_ok()
    };
    assert!(start(SECOND, 1, Some(&recovery_code)));
    assert!(start(SECOND, 1, None));
    // a password shorter than the SDK's eight characters, either way
    assert!(!start("seven77", 1, Some(&recovery_code)));
    assert!(!start("seven77", 1, None));
    assert!(start("eight888", 1, None));
    assert!(!start(&"x".repeat(1025), 1, None));
    // a recovery code that does not read as one: a reset code, a short one
    for malformed in [
        "RESET-0000-0000-0000-0000-0000-0000-0000-0000",
        "0000-0000-0000-0000-0000-0000-0000",
        "",
        "not a code",
    ] {
        assert!(!start(SECOND, 1, Some(malformed)), "{malformed:?}");
    }
    // no epoch, and no next epoch for a new vault
    assert!(!start(SECOND, 0, None));
    assert!(!start(SECOND, u64::MAX, None));
    assert!(start(SECOND, u64::MAX, Some(&recovery_code)));
    assert!(reset_start(SECOND.into(), &ACCOUNT[..15], 1, None).is_err());
}

#[test]
fn a_reset_finishes_only_the_way_it_started() {
    let (central, recovery_code) = Central::registered();
    let response = |request: &[u8]| {
        central
            .keys
            .enrollment_response(context(1), request)
            .unwrap()
    };
    let wrapper = central.stored.borrow().recovery.clone().unwrap();

    // started to keep: no new vault, and only the recovery wrapper opens
    let keep = || reset_start(SECOND.into(), &ACCOUNT, 1, Some(recovery_code.clone())).unwrap();
    let (state, request) = keep();
    assert!(reset_new(state, &response(&request)).is_err());
    let (state, request) = keep();
    let password_wrapper = central.stored.borrow().wrapped_master.clone();
    assert!(reset_keep(state, &response(&request), &password_wrapper).is_err());
    for size in [0, 72, 74] {
        let (state, request) = keep();
        assert!(reset_keep(state, &response(&request), &vec![1; size]).is_err());
    }
    let (state, request) = keep();
    let mut answer = response(&request);
    answer.push(0);
    assert!(reset_keep(state, &answer, &wrapper).is_err());
    let (state, request) = keep();
    reset_keep(state, &response(&request), &wrapper).unwrap();

    // started for a new vault: nothing is kept
    let (state, request) = reset_start(SECOND.into(), &ACCOUNT, 1, None).unwrap();
    assert!(reset_keep(state, &response(&request), &wrapper).is_err());

    // a wrapper of another epoch does not open under this one
    let (state, request) =
        reset_start(SECOND.into(), &ACCOUNT, 2, Some(recovery_code.clone())).unwrap();
    assert!(reset_keep(state, &response(&request), &wrapper).is_err());
}

#[test]
fn a_kept_reset_registers_at_the_accounts_epoch_and_a_new_one_at_the_next() {
    let (central, recovery_code) = Central::registered();
    let signs_in_at = |epoch: u64, password: &str| {
        let (state, ke1) = flow::login_start(password.into(), &ACCOUNT, epoch).unwrap();
        let stored = central.stored.borrow();
        let (_, ke2) = central
            .keys
            .start_login(context(stored.epoch), Some(&stored.record), &ke1)
            .unwrap();
        login_finish(state, &ke2).is_ok()
    };
    let code = central.issue();
    page_resets(&central, &code, SECOND, Some(&recovery_code)).unwrap();
    assert!(signs_in_at(1, SECOND));
    assert!(!signs_in_at(2, SECOND));

    let code = central.issue();
    page_resets(&central, &code, THIRD, None).unwrap();
    assert!(signs_in_at(2, THIRD));
    assert!(!signs_in_at(1, THIRD));
}

#[test]
fn a_password_change_keeps_what_the_sdk_opens_and_the_recovery_code() {
    let (central, recovery_code) = Central::registered();
    let outstanding = central.issue();
    page_changes(&central, FIRST, SECOND).unwrap();

    // the SDK signs in with the new password and opens the same document
    let mut session = sdk_signs_in(&central, SECOND).unwrap();
    assert_eq!(session.document(), DOCUMENT);
    assert!(sdk_signs_in(&central, FIRST).is_err(), "the old password");
    assert!(!page_can_sign_in(&central, FIRST));
    assert_eq!(central.identity().epoch, 1);

    // the recovery code still opens the key, and the old reset code is gone
    let wrapper = central.stored.borrow().recovery.clone().unwrap();
    let wrapper = WrappedMasterKey::from_bytes(&wrapper).unwrap();
    RecoveryCode::parse(&recovery_code)
        .unwrap()
        .unwrap(context(1), &wrapper)
        .unwrap();
    assert!(page_resets(&central, &outstanding, THIRD, Some(&recovery_code)).is_err());

    // the SDK changes it again, and the page signs in and changes it back
    session
        .change_password(&central, SECOND.into(), THIRD.into())
        .unwrap();
    assert!(page_can_sign_in(&central, THIRD));
    page_changes(&central, THIRD, FIRST).unwrap();
    assert_eq!(sdk_signs_in(&central, FIRST).unwrap().document(), DOCUMENT);
}

#[test]
fn a_password_change_is_refused_in_the_worker() {
    let (central, _) = Central::registered();
    let signed_in = || page_signs_in(&central, FIRST).unwrap();

    // a new password shorter than eight characters, an empty current one
    let (key, _, wrapped) = signed_in();
    assert!(password_start(key, &wrapped, FIRST.into(), "seven77".into()).is_err());
    let (key, _, wrapped) = signed_in();
    assert!(password_start(key, &wrapped, String::new(), SECOND.into()).is_err());
    let (key, _, wrapped) = signed_in();
    assert!(password_start(key, &wrapped, FIRST.into(), "x".repeat(1025)).is_err());

    // a wrapper that is not this sign-in's: the recovery wrapper, a changed bit
    let recovery = central.stored.borrow().recovery.clone().unwrap();
    let (key, _, _) = signed_in();
    assert!(password_start(key, &recovery, FIRST.into(), SECOND.into()).is_err());
    let (key, _, mut wrapped) = signed_in();
    wrapped[40] ^= 1;
    assert!(password_start(key, &wrapped, FIRST.into(), SECOND.into()).is_err());

    // a wrong current password fails at Central's KE2, before the finish
    let (key, token, wrapped) = signed_in();
    let (state, login, registration) = password_start(
        key,
        &wrapped,
        "synthetic WRONG password".into(),
        SECOND.into(),
    )
    .unwrap();
    let Response::PasswordChallenge {
        login,
        registration,
        ..
    } = central
        .exchange(
            &Request::ChangePasswordStart {
                login,
                registration,
            },
            Some((&token).into()),
        )
        .unwrap()
    else {
        panic!("a challenge");
    };
    assert!(password_finish(state, &login, &registration).is_err());
    assert!(page_can_sign_in(&central, FIRST), "nothing changed");

    // a malformed registration response
    let (key, token, wrapped) = signed_in();
    let (state, login, registration) =
        password_start(key, &wrapped, FIRST.into(), SECOND.into()).unwrap();
    let Response::PasswordChallenge {
        login,
        mut registration,
        ..
    } = central
        .exchange(
            &Request::ChangePasswordStart {
                login,
                registration,
            },
            Some((&token).into()),
        )
        .unwrap()
    else {
        panic!("a challenge");
    };
    registration.push(0);
    assert!(password_finish(state, &login, &registration).is_err());
}
