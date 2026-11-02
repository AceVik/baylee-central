use super::{
    ChatResponse, ConnectionTrait, DEVICE_COLUMNS, Error, Uuid, b64, person, public, statement,
    unavailable,
};
use central_api::chat::{Delivery, PAGE_SIZE, Sealed, Wrap};

pub(super) async fn read(
    db: &impl ConnectionTrait,
    me: Uuid,
    device: &str,
    after: u64,
) -> Result<ChatResponse, Error> {
    if device.len() != 16
        || !device
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        || after > 9_007_199_254_740_991
    {
        return Err(Error::Rejected);
    }
    let owned = db.query_one_raw(statement("SELECT d.id FROM central_identity.chat_devices d JOIN central_identity.accounts a ON a.id = d.account_id WHERE d.fingerprint = $1 AND d.account_id = $2 AND d.vault_epoch = a.vault_epoch", vec![device.into(), me.into()])).await.map_err(unavailable)?.is_some();
    if !owned {
        return Err(Error::Rejected);
    }
    let rows = db.query_all_raw(statement(&format!(
        "SELECT i.seq, i.key_wrap, m.envelope, a.tag, a.display_name, {DEVICE_COLUMNS},
         floor(extract(epoch FROM m.created_at) * 1000)::bigint AS at
         FROM central_identity.chat_deliveries i JOIN central_identity.chat_messages m ON m.id = i.message_id
         JOIN central_identity.accounts a ON a.id = m.sender
         JOIN central_identity.chat_devices d ON d.fingerprint = m.device
         WHERE i.device = $1 AND i.seq > $2 ORDER BY i.seq LIMIT 51"),
        vec![device.into(), i64::try_from(after).map_err(unavailable)?.into()],
    )).await.map_err(unavailable)?;
    let more = rows.len() > PAGE_SIZE;
    let messages = rows
        .iter()
        .take(PAGE_SIZE)
        .map(|row| {
            let bytes: Vec<u8> = row.try_get("", "envelope").map_err(unavailable)?;
            let mut sealed: Sealed = central_api::decode(&bytes).map_err(unavailable)?;
            let wrap: Vec<u8> = row.try_get("", "key_wrap").map_err(unavailable)?;
            sealed.wraps = vec![Wrap {
                device: device.into(),
                k: b64::url::encode(&wrap),
            }];
            let seq: i64 = row.try_get("", "seq").map_err(unavailable)?;
            let at: i64 = row.try_get("", "at").map_err(unavailable)?;
            Ok(Delivery {
                seq: u64::try_from(seq).map_err(unavailable)?,
                sender: person(row)?,
                device: public(row)?,
                sealed,
                at: u64::try_from(at).map_err(unavailable)?,
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;
    Ok(ChatResponse::Inbox { messages, more })
}
