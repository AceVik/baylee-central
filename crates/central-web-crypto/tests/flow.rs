//! The browser's registration, sign-in and recovery-code steps against
//! `central-crypto`'s server side, with real serialized messages and the
//! fixed Argon2 profile: what the WebAssembly exports do, driven on the host.
#![forbid(unsafe_code)]

use central_api::{Endpoint, LEGACY_ORIGIN, LIVE_ORIGIN};
use central_crypto::{
    Context, CryptoError,
    opaque::{Enrollment, Login, server::ServerKeys},
    vault::{MasterKey, ObjectContext, RecoveryCode, WrappedMasterKey},
};
use central_web_crypto::flow::{
    self, DOCUMENT, Recovery, SignedIn, enrollment_finish, enrollment_start, login_confirm,
    login_finish, login_start, recovery_create,
};

const ACCOUNT: [u8; 16] = [
    0x01, 0x92, 0xd0, 0xc5, 0x7b, 0x3e, 0x7a, 0x1f, 0x8c, 0x2d, 0x3e, 0x4f, 0x5a, 0x6b, 0x7c, 0x8d,
];
const PASSWORD: &str = "synthetic test-only password";

fn context(epoch: u64) -> Context {
    Context::new(Endpoint::live().issuer(), ACCOUNT, epoch).unwrap()
}

/// Registers `ACCOUNT` at `epoch`; the server's record and what the browser sent.
fn register(
    server: &ServerKeys,
    epoch: u64,
) -> (central_crypto::opaque::server::Record, flow::Registered) {
    let (state, request) = enrollment_start(PASSWORD.into(), &ACCOUNT, epoch).unwrap();
    assert_eq!(request.len(), 32, "registration request");
    let response = server
        .enrollment_response(context(epoch), &request)
        .unwrap();
    let registered = enrollment_finish(state, &response).unwrap();
    let record = ServerKeys::enrollment_record(&registered.record).unwrap();
    (record, registered)
}

#[test]
fn the_document_is_the_designs_bytes_and_names_the_official_gateway() {
    assert_eq!(
        DOCUMENT,
        br#"{"version":1,"gateways":["https://baylee.acevik.de"],"preferences":{},"legacy":null,"chat":{"devices":[],"installations":[]}}"#
    );
    let text = std::str::from_utf8(DOCUMENT).unwrap();
    assert!(text.contains(&format!("[\"{LEGACY_ORIGIN}\"]")));
}

#[test]
fn every_step_is_bound_to_the_live_central() {
    let endpoint = flow::endpoint();
    assert_eq!(endpoint.origin(), LIVE_ORIGIN);
    assert_eq!(endpoint.issuer(), Endpoint::live().issuer());
}

#[test]
fn registration_then_sign_in_then_recovery_open_one_master_key() {
    let server = ServerKeys::generate();
    let (record, registered) = register(&server, 1);
    assert_eq!(registered.record.len(), 192, "registration upload");
    assert_eq!(registered.wrapped_master.as_bytes().len(), 73);
    assert_eq!(registered.recovery.as_bytes().len(), 73);
    assert_eq!(registered.document.len(), DOCUMENT.len() + 41);
    assert_ne!(registered.wrapped_master, registered.recovery);

    // the browser's sign-in, as the page runs it
    let (state, ke1) = login_start(PASSWORD.into(), &ACCOUNT, 1).unwrap();
    assert_eq!(ke1.len(), 96, "KE1");
    let (attempt, ke2) = server.start_login(context(1), Some(&record), &ke1).unwrap();
    let (key, ke3) = login_finish(state, &ke2).unwrap();
    assert_eq!(ke3.len(), 64, "KE3");
    attempt.finish(&ke3).unwrap();
    login_confirm(key, registered.wrapped_master.as_bytes()).unwrap();

    // the document opens under the password's master key (the native client's way) ...
    let object = ObjectContext::new(context(1), ACCOUNT, 1).unwrap();
    let (login, ke1) = Login::start(PASSWORD.into(), context(1)).unwrap();
    let (_, ke2) = server.start_login(context(1), Some(&record), &ke1).unwrap();
    let master = login
        .finish(&ke2)
        .unwrap()
        .wrapping_key
        .unwrap(&registered.wrapped_master)
        .unwrap();
    assert_eq!(
        &**master.open_object(object, &registered.document).unwrap(),
        DOCUMENT
    );

    // ... and under the recovery code's, typed back in lower case
    let code = RecoveryCode::parse(&registered.recovery_code.to_lowercase()).unwrap();
    let recovered = code.unwrap(context(1), &registered.recovery).unwrap();
    assert_eq!(
        &**recovered.open_object(object, &registered.document).unwrap(),
        DOCUMENT
    );
}

#[test]
fn every_registration_makes_new_keys_and_a_new_code() {
    let server = ServerKeys::generate();
    let (_, first) = register(&server, 1);
    let (_, second) = register(&server, 1);
    assert_ne!(first.recovery_code.as_str(), second.recovery_code.as_str());
    assert_ne!(first.wrapped_master, second.wrapped_master);
    assert_ne!(first.document, second.document);
    let first_code = RecoveryCode::parse(&first.recovery_code).unwrap();
    assert!(first_code.unwrap(context(1), &second.recovery).is_err());
}

#[test]
fn a_wrong_password_fails_before_anything_is_sent() {
    let server = ServerKeys::generate();
    let (record, _) = register(&server, 1);
    let (state, ke1) = login_start("synthetic WRONG password".into(), &ACCOUNT, 1).unwrap();
    let (_, ke2) = server.start_login(context(1), Some(&record), &ke1).unwrap();
    assert!(login_finish(state, &ke2).is_err());
}

#[test]
fn another_epoch_or_an_unknown_account_cannot_sign_in() {
    let server = ServerKeys::generate();
    let (record, _) = register(&server, 1);
    let (state, ke1) = login_start(PASSWORD.into(), &ACCOUNT, 2).unwrap();
    let (_, ke2) = server.start_login(context(1), Some(&record), &ke1).unwrap();
    assert!(login_finish(state, &ke2).is_err());
    let (state, ke1) = login_start(PASSWORD.into(), &ACCOUNT, 1).unwrap();
    let (_, ke2) = server.start_login(context(1), None, &ke1).unwrap();
    assert!(login_finish(state, &ke2).is_err());
}

#[test]
fn only_the_accounts_own_password_wrapper_confirms_a_sign_in() {
    let server = ServerKeys::generate();
    let (record, registered) = register(&server, 1);
    let sign_in = || {
        let (state, ke1) = login_start(PASSWORD.into(), &ACCOUNT, 1).unwrap();
        let (_, ke2) = server.start_login(context(1), Some(&record), &ke1).unwrap();
        login_finish(state, &ke2).unwrap().0
    };
    // the recovery wrapper, another registration's wrapper, any flipped bit,
    // any other length
    assert!(login_confirm(sign_in(), registered.recovery.as_bytes()).is_err());
    let (_, other) = register(&server, 1);
    assert!(login_confirm(sign_in(), other.wrapped_master.as_bytes()).is_err());
    for index in [0, 1, 24, 25, 56, 57, 72] {
        let mut changed = *registered.wrapped_master.as_bytes();
        changed[index] ^= 0x10;
        assert!(login_confirm(sign_in(), &changed).is_err(), "byte {index}");
    }
    for size in [0, 72, 74] {
        assert!(login_confirm(sign_in(), &vec![1; size]).is_err());
    }
    login_confirm(sign_in(), registered.wrapped_master.as_bytes()).unwrap();
}

#[test]
fn malformed_starts_and_answers_are_rejected() {
    for account in [
        &ACCOUNT[..15],
        &[ACCOUNT.as_slice(), &[0]].concat(),
        &[0; 16],
    ] {
        assert!(enrollment_start(PASSWORD.into(), account, 1).is_err());
        assert!(login_start(PASSWORD.into(), account, 1).is_err());
    }
    let mut version_four = ACCOUNT;
    version_four[6] = 0x4a;
    assert!(enrollment_start(PASSWORD.into(), &version_four, 1).is_err());
    assert!(enrollment_start(PASSWORD.into(), &ACCOUNT, 0).is_err());
    assert!(login_start(PASSWORD.into(), &ACCOUNT, 0).is_err());
    assert!(enrollment_start(String::new(), &ACCOUNT, 1).is_err());
    assert!(login_start("x".repeat(1025), &ACCOUNT, 1).is_err());

    let server = ServerKeys::generate();
    let (state, request) = enrollment_start(PASSWORD.into(), &ACCOUNT, 1).unwrap();
    let mut response = server.enrollment_response(context(1), &request).unwrap();
    response.push(0);
    assert!(enrollment_finish(state, &response).is_err());
    let (state, _) = login_start(PASSWORD.into(), &ACCOUNT, 1).unwrap();
    assert!(login_finish(state, &[0; 320]).is_err());
}

/// An account as the official gateway's move enrolls it (Baylee's native
/// client, `central-crypto` directly): a password wrapper and a document,
/// and no recovery wrapper. Returns the server's record, the wrapper and the
/// sealed document.
fn moved(
    server: &ServerKeys,
    epoch: u64,
) -> (
    central_crypto::opaque::server::Record,
    WrappedMasterKey,
    Vec<u8>,
) {
    let (enrollment, request) = Enrollment::start(PASSWORD.into(), context(epoch)).unwrap();
    let response = server
        .enrollment_response(context(epoch), &request)
        .unwrap();
    let enrolled = enrollment.finish(&response).unwrap();
    let master = MasterKey::generate().unwrap();
    let wrapped = enrolled.wrapping_key.wrap(&master).unwrap();
    let object = ObjectContext::new(context(epoch), ACCOUNT, 1).unwrap();
    let document = master
        .seal_object(object, b"a moved account's document")
        .unwrap();
    (
        ServerKeys::enrollment_record(&enrolled.upload).unwrap(),
        wrapped,
        document,
    )
}

/// The browser's sign-in up to the state `login_confirm` or
/// `recovery_create` takes.
fn signed_in(
    server: &ServerKeys,
    record: &central_crypto::opaque::server::Record,
    epoch: u64,
) -> SignedIn {
    let (state, ke1) = login_start(PASSWORD.into(), &ACCOUNT, epoch).unwrap();
    let (attempt, ke2) = server
        .start_login(context(epoch), Some(record), &ke1)
        .unwrap();
    let (key, ke3) = login_finish(state, &ke2).unwrap();
    attempt.finish(&ke3).unwrap();
    key
}

#[test]
fn a_code_made_after_a_sign_in_opens_the_master_key_the_password_wrapper_holds() {
    let server = ServerKeys::generate();
    let (record, wrapped, document) = moved(&server, 3);
    let made = recovery_create(signed_in(&server, &record, 3), wrapped.as_bytes()).unwrap();
    assert_eq!(made.recovery.as_bytes().len(), 73);
    assert_ne!(made.recovery, wrapped);
    // the code, typed back in lower case, opens the wrapper to the master key
    // the password wrapper holds: both open the same document
    let object = ObjectContext::new(context(3), ACCOUNT, 1).unwrap();
    let code = RecoveryCode::parse(&made.recovery_code.to_lowercase()).unwrap();
    let recovered = code.unwrap(context(3), &made.recovery).unwrap();
    assert_eq!(
        &**recovered.open_object(object, &document).unwrap(),
        b"a moved account's document"
    );
    // bound to the account's context: another epoch's does not open it
    assert!(code.unwrap(context(4), &made.recovery).is_err());
    // a second code is another code, with another wrapper the first cannot open
    let again = recovery_create(signed_in(&server, &record, 3), wrapped.as_bytes()).unwrap();
    assert_ne!(again.recovery_code.as_str(), made.recovery_code.as_str());
    assert_ne!(again.recovery, made.recovery);
    assert!(code.unwrap(context(3), &again.recovery).is_err());
}

#[test]
fn a_registered_accounts_new_code_opens_its_master_key_too() {
    let server = ServerKeys::generate();
    let (record, registered) = register(&server, 1);
    let made = recovery_create(
        signed_in(&server, &record, 1),
        registered.wrapped_master.as_bytes(),
    )
    .unwrap();
    let object = ObjectContext::new(context(1), ACCOUNT, 1).unwrap();
    let recovered = RecoveryCode::parse(&made.recovery_code)
        .unwrap()
        .unwrap(context(1), &made.recovery)
        .unwrap();
    assert_eq!(
        &**recovered.open_object(object, &registered.document).unwrap(),
        DOCUMENT
    );
}

#[test]
fn a_code_is_made_only_from_a_wrapper_this_password_opens() {
    let server = ServerKeys::generate();
    let (record, wrapped, _) = moved(&server, 1);
    // the wrong key: another account's wrapper, a recovery wrapper, any
    // flipped bit, any other length; each fails as `login_confirm` does
    let (_, other, _) = moved(&server, 1);
    assert_eq!(
        recovery_create(signed_in(&server, &record, 1), other.as_bytes()).err(),
        Some(CryptoError::Invalid)
    );
    let made = recovery_create(signed_in(&server, &record, 1), wrapped.as_bytes()).unwrap();
    assert!(recovery_create(signed_in(&server, &record, 1), made.recovery.as_bytes()).is_err());
    for index in [0, 1, 24, 25, 56, 57, 72] {
        let mut changed = *wrapped.as_bytes();
        changed[index] ^= 0x10;
        assert!(
            recovery_create(signed_in(&server, &record, 1), &changed).is_err(),
            "byte {index}"
        );
    }
    for size in [0, 72, 74] {
        assert!(recovery_create(signed_in(&server, &record, 1), &vec![1; size]).is_err());
    }
}

#[test]
fn a_sign_in_is_confirmed_or_makes_a_code_once() {
    // Both steps take the sign-in's state by value: once either has run, the
    // state is gone (in WebAssembly its JavaScript object is emptied), so a
    // sign-in can neither make two codes nor confirm after making one.
    let confirm: fn(SignedIn, &[u8]) -> Result<(), CryptoError> = login_confirm;
    let create: fn(SignedIn, &[u8]) -> Result<Recovery, CryptoError> = recovery_create;
    let server = ServerKeys::generate();
    let (record, wrapped, _) = moved(&server, 1);
    assert!(confirm(signed_in(&server, &record, 1), wrapped.as_bytes()).is_ok());
    assert!(create(signed_in(&server, &record, 1), wrapped.as_bytes()).is_ok());
}
