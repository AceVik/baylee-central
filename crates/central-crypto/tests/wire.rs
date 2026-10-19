//! Canonical wire framing and authenticated vault generation regression tests.
#![forbid(unsafe_code)]
#![cfg(feature = "server")]

use central_crypto::{
    Context,
    opaque::{Enrollment, Login, server::ServerKeys},
};

fn context(epoch: u64) -> Context {
    let mut account = [2; 16];
    account[6] = 0x70;
    account[8] = 0x80;
    Context::new([1; 32], account, epoch).unwrap()
}

fn suffixed(bytes: &[u8]) -> Vec<u8> {
    let mut bytes = bytes.to_vec();
    bytes.push(0);
    bytes
}

#[test]
fn every_message_rejects_trailing_bytes() {
    let context = context(1);
    let server = ServerKeys::generate();
    let (client, request) = Enrollment::start("synthetic wire test".into(), context).unwrap();
    let response = server.enrollment_response(context, &request).unwrap();
    let enrolled = client.finish(&response).unwrap();
    let record = ServerKeys::enrollment_record(&enrolled.upload).unwrap();

    assert!(
        server
            .enrollment_response(context, &suffixed(&request))
            .is_err()
    );
    assert!(ServerKeys::enrollment_record(&suffixed(&enrolled.upload)).is_err());

    let (client, request) = Enrollment::start("synthetic wire test".into(), context).unwrap();
    let response = server.enrollment_response(context, &request).unwrap();
    assert!(client.finish(&suffixed(&response)).is_err());

    let (client, request) = Login::start("synthetic wire test".into(), context).unwrap();
    assert!(
        server
            .start_login(context, Some(&record), &suffixed(&request))
            .is_err()
    );
    let (_, response) = server
        .start_login(context, Some(&record), &request)
        .unwrap();
    assert!(client.finish(&suffixed(&response)).is_err());

    let (client, request) = Login::start("synthetic wire test".into(), context).unwrap();
    let (attempt, response) = server
        .start_login(context, Some(&record), &request)
        .unwrap();
    let result = client.finish(&response).unwrap();
    assert!(attempt.finish(&suffixed(&result.finalization)).is_err());
}

#[test]
fn peers_must_agree_on_vault_epoch() {
    let server = ServerKeys::generate();
    let (client, request) = Enrollment::start("synthetic epoch test".into(), context(1)).unwrap();
    let response = server.enrollment_response(context(1), &request).unwrap();
    let enrolled = client.finish(&response).unwrap();
    let record = ServerKeys::enrollment_record(&enrolled.upload).unwrap();
    for (client_epoch, server_epoch) in [(1, 2), (2, 1), (u64::MAX, 1)] {
        let (client, request) =
            Login::start("synthetic epoch test".into(), context(client_epoch)).unwrap();
        let (_, response) = server
            .start_login(context(server_epoch), Some(&record), &request)
            .unwrap();
        assert!(client.finish(&response).is_err());
    }
}
