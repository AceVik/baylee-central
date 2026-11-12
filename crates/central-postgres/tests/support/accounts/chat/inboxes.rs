//! A device's inbox and its senders: one other account's messages take at
//! most its share of an inbox, a send skips the devices that have no room
//! for it instead of being refused whole, and is refused when nobody else
//! could take it or the sender's own device is full.
use super::groups::{change, contact, create, message, post};
use super::*;
use central_api::groups::Change;
use central_postgres::CHAT_SENDER_SHARE;
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement, prelude::Uuid};

const A: u32 = 0x7fff_ffdb;
const B: u32 = 0x7fff_ffdc;
const C: u32 = 0x7fff_ffdd;
/// A device's inbox (`central_api::chat::MAX_DELIVERIES`).
const INBOX: i64 = 5_000;

/// One of the three: its account, its session and its device.
#[derive(Clone, Copy)]
struct Member<'a> {
    account: [u8; 16],
    token: &'a Token,
    device: &'a DeviceSecrets,
}

async fn counted(operator: &DatabaseConnection, text: &str, values: Vec<sea_orm::Value>) -> i64 {
    operator
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            text,
            values,
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "n")
        .unwrap()
}

/// How many deliveries the device of `of` holds.
async fn held(operator: &DatabaseConnection, of: Member<'_>) -> i64 {
    counted(
        operator,
        "SELECT count(*)::bigint AS n FROM central_identity.chat_deliveries WHERE device = $1",
        vec![of.device.device_id().into()],
    )
    .await
}

/// How many of them are messages of `from`.
async fn held_from(operator: &DatabaseConnection, of: Member<'_>, from: Member<'_>) -> i64 {
    counted(
        operator,
        "SELECT count(*)::bigint AS n FROM central_identity.chat_deliveries i
         JOIN central_identity.chat_messages m ON m.id = i.message_id
         WHERE i.device = $1 AND m.sender = $2",
        vec![
            of.device.device_id().into(),
            Uuid::from_bytes(from.account).into(),
        ],
    )
    .await
}

/// `many` messages of `from` in the inbox of `to`'s device, a day old (so
/// that the sender's minute is not spent), written by the operator.
async fn fill(operator: &DatabaseConnection, from: Member<'_>, to: Member<'_>, many: i64) {
    operator
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "WITH piled AS (
                 INSERT INTO central_identity.chat_messages
                     (sender, correlation, device, conversation, envelope, created_at)
                 SELECT $1, pg_catalog.uuidv7(), $2, 'fill', '\\x00'::bytea,
                     pg_catalog.clock_timestamp() - interval '1 day'
                 FROM pg_catalog.generate_series(1, $4::bigint)
                 RETURNING id)
             INSERT INTO central_identity.chat_deliveries (message_id, device, key_wrap)
             SELECT piled.id, $3, pg_catalog.decode(pg_catalog.repeat('00', 48), 'hex') FROM piled",
            vec![
                Uuid::from_bytes(from.account).into(),
                from.device.device_id().into(),
                to.device.device_id().into(),
                many.into(),
            ],
        ))
        .await
        .unwrap();
}

async fn emptied(operator: &DatabaseConnection) {
    operator
        .execute_unprepared(
            "DELETE FROM central_identity.chat_messages WHERE conversation = 'fill'",
        )
        .await
        .unwrap();
}

pub(super) async fn run(
    store: &AccountStore,
    operator: &DatabaseConnection,
    accounts: [central_api::wire::Identity; 3],
    tokens: [&Token; 3],
    devices: [&DeviceSecrets; 2],
) {
    // C's device is the one the group scenario registered.
    let third = DeviceSecrets::from_bytes(&[74; 96]);
    let member = |at: usize, device| Member {
        account: accounts[at].account,
        token: tokens[at],
        device,
    };
    let members = [
        member(0, devices[0]),
        member(1, devices[1]),
        member(2, &third),
    ];
    let [a, b, c] = members;
    let all = [a.device, b.device, c.device];
    assert_eq!(i64::try_from(MAX_DELIVERIES), Ok(INBOX));
    assert_eq!(CHAT_SENDER_SHARE, INBOX / 2);
    // The three are contacts again and in one group.
    contact(store, b.token, A, ContactAction::Unblock).await;
    contact(store, a.token, B, ContactAction::Request).await;
    contact(store, b.token, A, ContactAction::Request).await;
    let id = create(store, a.token).await;
    let conv = format!("grp:{id}");
    for (member, tag) in [(b, B), (c, C)] {
        change(store, a.token, &id, Change::Invite { tag })
            .await
            .unwrap();
        change(store, member.token, &id, Change::Join)
            .await
            .unwrap();
    }
    let direct = "dm:7fffffdb:7fffffdd";
    let to_c = |id| message(direct, a.device, A, &[a.device, c.device], id);

    // One sender's share of an inbox. A's messages take all but one of
    // their share of C's inbox: one more is delivered, the next is not.
    let already = held_from(operator, c, a).await;
    fill(operator, a, c, CHAT_SENDER_SHARE - 1 - already).await;
    post(store, a.token, direct, to_c(200)).await.unwrap();
    assert_eq!(held_from(operator, c, a).await, CHAT_SENDER_SHARE);
    assert_eq!(
        post(store, a.token, direct, to_c(201)).await.err(),
        Some(Error::Busy),
        "A's share of C's inbox is used up, and nobody else would get this"
    );
    // C's inbox has room for everyone else: B is heard there,
    let c_held = held(operator, c).await;
    assert!(c_held < INBOX);
    post(
        store,
        b.token,
        &conv,
        message(&conv, b.device, B, &all, 202),
    )
    .await
    .unwrap();
    assert_eq!(held(operator, c).await, c_held + 1);
    // and A's message to the group reaches B, though not C.
    let b_held = held(operator, b).await;
    post(
        store,
        a.token,
        &conv,
        message(&conv, a.device, A, &all, 203),
    )
    .await
    .unwrap();
    assert_eq!(held(operator, b).await, b_held + 1);
    assert_eq!(held(operator, c).await, c_held + 1, "skipped");
    assert_eq!(held_from(operator, c, a).await, CHAT_SENDER_SHARE);

    full_devices(store, operator, members, &conv).await;
    emptied(operator).await;
    change(store, a.token, &id, Change::Delete).await.unwrap();
}

/// A full device is skipped, a send that nobody else could take is
/// refused, a block shows in neither, and the sender's own full device
/// refuses the send as before.
async fn full_devices(
    store: &AccountStore,
    operator: &DatabaseConnection,
    members: [Member<'_>; 3],
    conv: &str,
) {
    let [a, b, c] = members;
    let all = [a.device, b.device, c.device];
    // C's inbox is full (B's messages fill what is left of it): B's message
    // to the group is still sent, to A, where it was refused whole.
    fill(operator, b, c, INBOX - held(operator, c).await).await;
    assert_eq!(held(operator, c).await, INBOX);
    let a_held = held(operator, a).await;
    post(store, b.token, conv, message(conv, b.device, B, &all, 204))
        .await
        .unwrap();
    assert_eq!(held(operator, a).await, a_held + 1);
    assert_eq!(held(operator, c).await, INBOX, "never past its bound");
    // B blocks A. A's message has room at B's device, so it is sent as far
    // as A can tell; B's device and C's full one get nothing.
    contact(store, b.token, A, ContactAction::Block).await;
    let b_held = held(operator, b).await;
    post(store, a.token, conv, message(conv, a.device, A, &all, 205))
        .await
        .unwrap();
    assert_eq!(held(operator, b).await, b_held);
    // B's inbox is full too: nobody else could take A's message, blocked
    // or not, and it is refused.
    fill(operator, c, b, INBOX - b_held).await;
    assert_eq!(
        post(store, a.token, conv, message(conv, a.device, A, &all, 206))
            .await
            .err(),
        Some(Error::Busy)
    );
    contact(store, b.token, A, ContactAction::Unblock).await;
    assert_eq!(
        post(store, a.token, conv, message(conv, a.device, A, &all, 206))
            .await
            .err(),
        Some(Error::Busy),
        "the same answer without the block"
    );
    // The sender's own device is full: refused, as it was.
    emptied(operator).await;
    fill(operator, c, a, INBOX - held(operator, a).await).await;
    assert_eq!(
        post(store, a.token, conv, message(conv, a.device, A, &all, 207))
            .await
            .err(),
        Some(Error::Busy)
    );
}
