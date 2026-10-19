//! Integration tests with real serialized OPAQUE messages and the fixed KDF.
#![forbid(unsafe_code)]
#![cfg(feature = "server")]

use central_crypto::{
    Context,
    opaque::{Enrollment, Login, server::ServerKeys},
    vault::{MasterKey, WrappedMasterKey},
};

fn context() -> Context {
    let mut account = [2; 16];
    account[6] = 0x70;
    account[8] = 0x80;
    Context::new([1; 32], account, 1).unwrap()
}

#[test]
fn enrollment_login_password_change_and_negative_cases() {
    let context = context();
    let server = ServerKeys::generate();
    let (client, request) =
        Enrollment::start("test-only correct password".into(), context).unwrap();
    let reply = server.enrollment_response(context, &request).unwrap();
    let enrolled = client.finish(&reply).unwrap();
    let record = ServerKeys::enrollment_record(&enrolled.upload).unwrap();
    let master = MasterKey::generate().unwrap();
    let wrapped = enrolled.wrapping_key.wrap(&master).unwrap();

    let (login, request) = Login::start("test-only correct password".into(), context).unwrap();
    let (attempt, reply) = server
        .start_login(context, Some(&record), &request)
        .unwrap();
    let logged_in = login.finish(&reply).unwrap();
    attempt.finish(&logged_in.finalization).unwrap();
    let restored = logged_in.wrapping_key.unwrap(&wrapped).unwrap();

    let (login, request) = Login::start("test-only WRONG password".into(), context).unwrap();
    let (_, reply) = server
        .start_login(context, Some(&record), &request)
        .unwrap();
    assert!(login.finish(&reply).is_err());

    let (login, request) = Login::start("test-only correct password".into(), context).unwrap();
    let (_, reply) = server.start_login(context, None, &request).unwrap();
    assert!(login.finish(&reply).is_err());

    let mut other_account = [3; 16];
    other_account[6] = 0x70;
    other_account[8] = 0x80;
    let other = Context::new([1; 32], other_account, 1).unwrap();
    let (login, request) = Login::start("test-only correct password".into(), other).unwrap();
    let (_, reply) = server
        .start_login(context, Some(&record), &request)
        .unwrap();
    assert!(login.finish(&reply).is_err());

    let (client, request) = Enrollment::start("test-only NEW password".into(), context).unwrap();
    let reply = server.enrollment_response(context, &request).unwrap();
    let replacement = client.finish(&reply).unwrap();
    let new_wrapper = replacement.wrapping_key.wrap(&restored).unwrap();
    assert!(logged_in.wrapping_key.unwrap(&new_wrapper).is_err());
    let new_record = ServerKeys::enrollment_record(&replacement.upload).unwrap();
    let (login, request) = Login::start("test-only NEW password".into(), context).unwrap();
    let (attempt, reply) = server
        .start_login(context, Some(&new_record), &request)
        .unwrap();
    let result = login.finish(&reply).unwrap();
    attempt.finish(&result.finalization).unwrap();
    assert!(result.wrapping_key.unwrap(&new_wrapper).is_ok());
    // Credential/wrapper persistence must still be made atomic by M3/M4.
}

#[test]
fn malformed_messages_and_password_bounds_fail_without_panic() {
    let context = context();
    let server = ServerKeys::generate();
    assert!(Enrollment::start(String::new(), context).is_err());
    assert!(Login::start("a".repeat(1025), context).is_err());
    for size in [0, 1, 31, 1024, 4096] {
        let invalid = vec![0; size];
        assert!(server.enrollment_response(context, &invalid).is_err());
        assert!(ServerKeys::enrollment_record(&invalid).is_err());
        assert!(server.start_login(context, None, &invalid).is_err());
        assert!(WrappedMasterKey::from_bytes(&invalid).is_err());
    }
}

#[test]
fn altered_server_identity_and_final_proof_are_rejected() {
    let context = context();
    let server = ServerKeys::generate();
    let (client, request) =
        Enrollment::start("test-only identity password".into(), context).unwrap();
    let reply = server.enrollment_response(context, &request).unwrap();
    let enrolled = client.finish(&reply).unwrap();
    let record = ServerKeys::enrollment_record(&enrolled.upload).unwrap();

    let mut account = [2; 16];
    account[6] = 0x70;
    account[8] = 0x80;
    let foreign_issuer = Context::new([9; 32], account, 1).unwrap();
    let (login, request) =
        Login::start("test-only identity password".into(), foreign_issuer).unwrap();
    let (_, reply) = server
        .start_login(context, Some(&record), &request)
        .unwrap();
    assert!(login.finish(&reply).is_err());

    let (login, request) = Login::start("test-only identity password".into(), context).unwrap();
    let (attempt, reply) = server
        .start_login(context, Some(&record), &request)
        .unwrap();
    let mut result = login.finish(&reply).unwrap();
    result.finalization[0] ^= 1;
    assert!(attempt.finish(&result.finalization).is_err());

    // A valid final proof is bound to its own pending attempt, not reusable
    // to finish another exchange for the same account/password.
    let (login, request) = Login::start("test-only identity password".into(), context).unwrap();
    let (_, reply) = server
        .start_login(context, Some(&record), &request)
        .unwrap();
    let result = login.finish(&reply).unwrap();
    let (_, other_request) = Login::start("test-only identity password".into(), context).unwrap();
    let (other_attempt, _) = server
        .start_login(context, Some(&record), &other_request)
        .unwrap();
    assert!(other_attempt.finish(&result.finalization).is_err());
}

#[test]
fn client_rejects_invalid_server_messages() {
    for size in [0, 1, 31, 1024, 4096] {
        let invalid = vec![0; size];
        let (client, _) = Enrollment::start("test-only".into(), context()).unwrap();
        assert!(client.finish(&invalid).is_err());
        let (client, _) = Login::start("test-only".into(), context()).unwrap();
        assert!(client.finish(&invalid).is_err());
    }
}
