//! Durable ciphertext delivery; all authorization is rechecked under ordered locks.
use super::{Error, Session, by_tag, caller, lock_accounts, person, statement, unavailable};
use central_api::chat::{AccountKeys, ChatRequest, ChatResponse, DeviceKey, DeviceRegistration};
use central_crypto::chat::{DevicePublic, b64};
use sea_orm::{ConnectionTrait, QueryResult, prelude::Uuid};
use std::collections::{BTreeMap, BTreeSet};

mod inbox;
mod send;
pub use send::CHAT_SENDER_SHARE;

fn rejected(_: impl Sized) -> Error {
    Error::Rejected
}

fn decode<const N: usize>(value: &str) -> Result<[u8; N], Error> {
    if value.len() > N.div_ceil(3) * 4 {
        return Err(Error::Rejected);
    }
    b64::url::decode(value)
        .map_err(rejected)?
        .try_into()
        .map_err(rejected)
}

fn public(row: &QueryResult) -> Result<DeviceKey, Error> {
    let bytes = |column| row.try_get::<Vec<u8>>("", column).map_err(unavailable);
    let time: i64 = row.try_get("", "added_at").map_err(unavailable)?;
    Ok(DeviceKey {
        id: row.try_get("", "fingerprint").map_err(unavailable)?,
        ik: b64::url::encode(&bytes("ik")?),
        dk: b64::url::encode(&bytes("dk")?),
        dsig: b64::url::encode(&bytes("dsig")?),
        label: row.try_get("", "label").map_err(unavailable)?,
        added_at: u64::try_from(time).map_err(unavailable)?,
    })
}

const DEVICE_COLUMNS: &str = "d.fingerprint, d.ik, d.dk, d.dsig, d.label,
    floor(extract(epoch FROM d.created_at) * 1000)::bigint AS added_at";

async fn account_tag(db: &impl ConnectionTrait, account: Uuid) -> Result<String, Error> {
    let row = db
        .query_one_raw(statement(
            "SELECT tag FROM central_identity.accounts WHERE id = $1 AND tag IS NOT NULL",
            vec![account.into()],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Rejected)?;
    let tag: i32 = row.try_get("", "tag").map_err(unavailable)?;
    Ok(format!("{tag:04x}"))
}

pub(super) async fn dispatch(
    db: &impl ConnectionTrait,
    session: Session<'_>,
    me: Uuid,
    request: ChatRequest,
) -> Result<ChatResponse, Error> {
    match request {
        ChatRequest::Register { device } => {
            lock_accounts(db, vec![me]).await?;
            caller(db, session, true).await?;
            register(db, me, &device).await
        }
        ChatRequest::Keys { tags } => keys(db, me, &tags).await,
        ChatRequest::Send { conv, sealed } => send::send(db, session, me, &conv, &sealed).await,
        ChatRequest::Inbox { device, after } => inbox::read(db, me, &device, after).await,
    }
}

async fn register(
    db: &impl ConnectionTrait,
    me: Uuid,
    registration: &DeviceRegistration,
) -> Result<ChatResponse, Error> {
    let ik = decode(&registration.ik)?;
    let dk = decode(&registration.dk)?;
    let dsig = decode(&registration.dsig)?;
    if !(1..=24).contains(&registration.label.len())
        || registration.label.trim() != registration.label
        || !registration
            .label
            .bytes()
            .all(|b| (b' '..=b'~').contains(&b))
    {
        return Err(Error::Rejected);
    }
    let key = DevicePublic {
        id: central_crypto::chat::device_id(&ik),
        ik,
        dk,
        dsig,
    };
    key.verify(&account_tag(db, me).await?).map_err(rejected)?;
    let epoch: i64 = db
        .query_one_raw(statement(
            "SELECT vault_epoch FROM central_identity.accounts WHERE id = $1",
            vec![me.into()],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Rejected)?
        .try_get("", "vault_epoch")
        .map_err(unavailable)?;
    if let Some(old) = db.query_one_raw(statement("SELECT account_id, vault_epoch, ik, dk, dsig FROM central_identity.chat_devices WHERE fingerprint = $1", vec![key.id.clone().into()])).await.map_err(unavailable)? {
        let owner: Uuid = old.try_get("", "account_id").map_err(unavailable)?;
        if owner != me || old.try_get::<i64>("", "vault_epoch").map_err(unavailable)? != epoch || old.try_get::<Vec<u8>>("", "ik").map_err(unavailable)? != ik
            || old.try_get::<Vec<u8>>("", "dk").map_err(unavailable)? != dk
            || old.try_get::<Vec<u8>>("", "dsig").map_err(unavailable)? != dsig { return Err(Error::Conflict); }
        return Ok(ChatResponse::Registered { id: key.id });
    }
    let count: i64 = db
        .query_one_raw(statement(
            "SELECT count(*) FROM central_identity.chat_devices WHERE account_id = $1 AND vault_epoch = $2",
            vec![me.into(), epoch.into()],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Unavailable)?
        .try_get("", "count")
        .map_err(unavailable)?;
    if count >= 5 {
        return Err(Error::Busy);
    }
    db.query_one_raw(statement("INSERT INTO central_identity.chat_devices (fingerprint, account_id, ik, dk, dsig, label, vault_epoch) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id",
        vec![key.id.clone().into(), me.into(), ik.to_vec().into(), dk.to_vec().into(), dsig.to_vec().into(), registration.label.clone().into(), epoch.into()],
    )).await.map_err(unavailable)?.ok_or(Error::Unavailable)?;
    Ok(ChatResponse::Registered { id: key.id })
}

async fn keys(db: &impl ConnectionTrait, me: Uuid, tags: &[u32]) -> Result<ChatResponse, Error> {
    if tags.is_empty()
        || tags.len() > 60
        || tags.iter().collect::<BTreeSet<_>>().len() != tags.len()
    {
        return Err(Error::Rejected);
    }
    let tags: Vec<i32> = tags
        .iter()
        .map(|tag| i32::try_from(*tag).map_err(rejected))
        .collect::<Result<_, _>>()?;
    // One query for every requested account/device, not one round trip per peer.
    let rows = db
        .query_all_raw(statement(
            &format!(
                "SELECT a.tag, a.display_name, {DEVICE_COLUMNS} FROM central_identity.accounts a
         LEFT JOIN central_identity.chat_devices d ON d.account_id = a.id AND d.vault_epoch = a.vault_epoch
         WHERE a.tag = ANY($2) AND a.credential IS NOT NULL AND a.display_name IS NOT NULL
         AND (a.id = $1 OR (EXISTS (SELECT 1 FROM central_identity.social_contacts c WHERE c.accepted
             AND c.low = LEAST(a.id, $1) AND c.high = GREATEST(a.id, $1))
             AND NOT EXISTS (SELECT 1 FROM central_identity.social_blocks b WHERE
                (b.owner = $1 AND b.blocked = a.id) OR (b.owner = a.id AND b.blocked = $1)))
             OR EXISTS (SELECT 1 FROM central_identity.chat_members mine
                JOIN central_identity.chat_members peer ON peer.group_id = mine.group_id
                WHERE mine.account_id = $1 AND peer.account_id = a.id AND mine.accepted AND peer.accepted))
         ORDER BY a.tag, d.fingerprint"
            ),
            vec![me.into(), tags.into()],
        ))
        .await
        .map_err(unavailable)?;
    let mut keys = BTreeMap::<u32, AccountKeys>::new();
    for row in &rows {
        let p = person(row)?;
        let entry = keys.entry(p.tag).or_insert_with(|| AccountKeys {
            tag: format!("{:04x}", p.tag),
            handle: format!("{}#{:04x}", p.display_name, p.tag),
            devices: Vec::new(),
        });
        if row
            .try_get::<Option<String>>("", "fingerprint")
            .map_err(unavailable)?
            .is_some()
        {
            entry.devices.push(public(row)?);
        }
    }
    Ok(ChatResponse::Keys {
        keys: keys.into_values().collect(),
    })
}
