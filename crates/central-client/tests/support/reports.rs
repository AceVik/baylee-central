//! A gateway's reports over HTTP (`docs/reports.md` §"Intake"): one
//! gateway holds two uploads at once and no third, another gateway's part
//! is stored meanwhile, and a slot is free again once its connection went.
use super::{Http, pace};
use central_api::{
    PROTOCOL_HEADER, PROTOCOL_VERSION, Token,
    reports::{Announced, Channel, GatewaySays, ReportIntake, ReportKind},
};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement, prelude::Uuid};
use std::{
    io::{Read, Write},
    net::TcpStream,
    time::Duration,
};
use tokio::runtime::Runtime;

const ORIGIN: &str = "http://127.0.0.1:28769";
const TEXT: &str = "Synthetic report over HTTP";

fn sql(runtime: &Runtime, operator: &DatabaseConnection, text: &str, values: Vec<sea_orm::Value>) {
    runtime
        .block_on(operator.execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            text,
            values,
        )))
        .unwrap();
}

/// A second approved gateway of `owner`, written by the operator; its
/// credential.
fn second(runtime: &Runtime, operator: &DatabaseConnection, owner: [u8; 16]) -> Token {
    let credential = Token::generate().unwrap();
    let heartbeat = Token::generate().unwrap();
    sql(
        runtime,
        operator,
        "INSERT INTO central_identity.gateways (owner_id, origin, name, version, website,
             credential_digest, heartbeat_digest)
         VALUES ($1, $2, 'Second synthetic gateway', 'test-1', $2::text || '/', $3, $4)",
        vec![
            Uuid::from_bytes(owner).into(),
            ORIGIN.into(),
            credential.digest().to_vec().into(),
            heartbeat.digest().to_vec().into(),
        ],
    );
    sql(
        runtime,
        operator,
        "INSERT INTO central_identity.gateway_approvals (gateway_id)
         SELECT id FROM central_identity.gateways WHERE origin = $1",
        vec![ORIGIN.into()],
    );
    credential
}

/// Synthetic gzip (the magic, then anything) and its announcement; the
/// database makes the digest, as this crate's tests link no hash.
fn part(runtime: &Runtime, operator: &DatabaseConnection) -> (Vec<u8>, Announced) {
    let mut bytes = vec![9; 40_000];
    bytes[0] = 0x1f;
    bytes[1] = 0x8b;
    let digest: Vec<u8> = runtime
        .block_on(operator.query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT pg_catalog.sha256($1) AS digest",
            vec![bytes.clone().into()],
        )))
        .unwrap()
        .unwrap()
        .try_get("", "digest")
        .unwrap();
    let announced = Announced {
        bytes: u32::try_from(bytes.len()).unwrap(),
        sha256: digest.try_into().unwrap(),
    };
    (bytes, announced)
}

/// A request's head on a connection of its own; its body is the caller's
/// to send or to withhold.
fn begun(http: &Http, line: &str, kind: &str, length: usize, bearer: &Token) -> TcpStream {
    pace();
    let mut stream = TcpStream::connect_timeout(&http.address, Duration::from_secs(3)).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    stream
        .set_write_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    write!(
        stream,
        "{line} HTTP/1.1\r\nHost: {}\r\n{PROTOCOL_HEADER}: {PROTOCOL_VERSION}\r\nContent-Type: {kind}\r\nContent-Length: {length}\r\nAuthorization: {}\r\n\r\n",
        http.address,
        bearer.bearer().as_str()
    )
    .unwrap();
    stream
}

/// The answer's status and body, if one came whole.
fn answered(stream: TcpStream) -> Option<(u16, String)> {
    let mut response = Vec::new();
    stream.take(8_192).read_to_end(&mut response).ok()?;
    let text = String::from_utf8(response).ok()?;
    let (head, body) = text.split_once("\r\n\r\n")?;
    let status = head.split_whitespace().nth(1)?.parse().ok()?;
    Some((status, body.to_owned()))
}

/// File a report announcing `client`; its id as the routes spell it.
fn filed(http: &Http, bearer: &Token, client: &Announced) -> String {
    let report = ReportIntake {
        gateway: GatewaySays {
            name: Some("Synthetic gateway".into()),
            url: None,
            version: Some("test-1".into()),
        },
        channel: Channel::Gateway,
        reporter: "5bdc11".into(),
        kind: ReportKind::Other,
        text: TEXT.into(),
        game_id: None,
        build: None,
        crash: None,
        client: Some(client.clone()),
        record: None,
    };
    let body = central_api::encode(&report).unwrap();
    let mut stream = begun(
        http,
        "POST /v1/reports",
        "application/json",
        body.len(),
        bearer,
    );
    stream.write_all(&body).unwrap();
    let (status, answer) = answered(stream).unwrap();
    assert_eq!(status, 201);
    let id = answer
        .strip_prefix(r#"{"report_id":""#)
        .and_then(|rest| rest.strip_suffix(r#""}"#))
        .unwrap();
    assert_eq!(id.len(), 32);
    id.to_owned()
}

/// An upload of `report`'s client part whose body does not come.
fn withheld(http: &Http, report: &str, bearer: &Token, length: usize) -> TcpStream {
    begun(
        http,
        &format!("PUT /v1/reports/{report}/client"),
        "application/gzip",
        length,
        bearer,
    )
}

/// An upload of `report`'s client part, whole; its status, if an answer came.
fn sent(http: &Http, report: &str, bearer: &Token, gzip: &[u8]) -> Option<u16> {
    let mut stream = withheld(http, report, bearer, gzip.len());
    // An upload refused before its body is read may close on what was sent.
    let _ = stream.write_all(gzip);
    answered(stream).map(|(status, _)| status)
}

pub(crate) fn run(
    http: &Http,
    credential: &Token,
    owner: [u8; 16],
    runtime: &Runtime,
    operator: &DatabaseConnection,
) {
    let other = second(runtime, operator, owner);
    let (gzip, announced) = part(runtime, operator);
    let ours = filed(http, credential, &announced);
    let theirs = filed(http, &other, &announced);
    // A part that was never stored may be sent again, and twice at once:
    // two uploads of one gateway, announced and withheld.
    let first = withheld(http, &ours, credential, gzip.len());
    let second = withheld(http, &ours, credential, gzip.len());
    std::thread::sleep(Duration::from_millis(500));
    // The gateway's third is refused before any body is read,
    let third = answered(withheld(http, &ours, credential, gzip.len()));
    assert_eq!(third.map(|(status, _)| status), Some(429));
    // and another gateway's part is stored beside them.
    assert_eq!(sent(http, &theirs, &other, &gzip), Some(204));
    let again = answered(withheld(http, &theirs, &other, gzip.len()));
    assert_eq!(again.map(|(status, _)| status), Some(409));
    // The connections go: the slots are free again, which the gateway's
    // next send finds as the released one would, by sending again.
    drop((first, second));
    let stored = (0..50).find_map(|_| {
        let status = sent(http, &ours, credential, &gzip);
        if status == Some(204) {
            return status;
        }
        assert!(matches!(status, None | Some(429)), "{status:?}");
        std::thread::sleep(Duration::from_millis(100));
        None
    });
    assert_eq!(stored, Some(204));
    sql(
        runtime,
        operator,
        "DELETE FROM central_identity.reports WHERE text = $1",
        vec![TEXT.into()],
    );
    sql(
        runtime,
        operator,
        "DELETE FROM central_identity.gateways WHERE origin = $1",
        vec![ORIGIN.into()],
    );
}
