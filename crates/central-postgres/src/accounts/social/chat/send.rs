use super::{
    ChatResponse, ConnectionTrait, Error, Session, Uuid, account_tag, by_tag, caller, decode,
    lock_accounts, rejected, statement, unavailable,
};
use central_api::chat::{Header, MAX_CIPHERTEXT, MAX_DELIVERIES, Sealed};
use central_crypto::chat::{Envelope, b64};
use std::collections::{BTreeMap, BTreeSet};

/// What a device's inbox holds (`central_api::chat::MAX_DELIVERIES`).
const INBOX: i64 = 5_000;
const _: () = assert!(MAX_DELIVERIES == 5_000);

/// What one other account's messages may take of a device's inbox: half.
/// Nothing deletes a delivery yet (retention is the owner's pending
/// decision, `docs/central-chat.md`), so without a share one contact fills
/// an inbox for good and nobody else is heard there. At half, the two
/// sides of a direct conversation take the same, and it takes two senders
/// to fill an inbox. The account's own messages on its own devices are not
/// held to it: its outbox bounds them.
pub const CHAT_SENDER_SHARE: i64 = INBOX / 2;

struct Parsed {
    header: Header,
    hdr: Vec<u8>,
    n: [u8; 24],
    ct: Vec<u8>,
    epk: [u8; 32],
    sig: [u8; 64],
    correlation: Uuid,
}

impl Parsed {
    fn read(conv: &str, sealed: &Sealed) -> Result<Self, Error> {
        if conv.len() > 100
            || sealed.hdr.len() > 684
            || sealed.ct.len() > MAX_CIPHERTEXT.div_ceil(3) * 4
            || sealed.wraps.len() > 250
        {
            return Err(Error::Rejected);
        }
        let hdr = b64::url::decode(&sealed.hdr).map_err(rejected)?;
        if hdr.len() > 512 {
            return Err(Error::Rejected);
        }
        let header: Header = central_api::decode(&hdr)?;
        let correlation = Uuid::parse_str(&header.id).map_err(rejected)?;
        if correlation.get_version_num() != 7
            || correlation.as_bytes()[8] & 0xc0 != 0x80
            || correlation.to_string() != header.id
            || header.v != 1
            || header.kind != "text"
            || header.conv != conv
        {
            return Err(Error::Rejected);
        }
        let ct = b64::url::decode(&sealed.ct).map_err(rejected)?;
        if !(48..=MAX_CIPHERTEXT).contains(&ct.len()) {
            return Err(Error::Rejected);
        }
        Ok(Self {
            header,
            correlation,
            hdr,
            n: decode(&sealed.n)?,
            ct,
            epk: decode(&sealed.epk)?,
            sig: decode(&sealed.sig)?,
        })
    }

    fn envelope(&self) -> Envelope<'_> {
        Envelope {
            hdr: &self.hdr,
            n: &self.n,
            ct: &self.ct,
            epk: &self.epk,
            sig: &self.sig,
        }
    }
}

fn partner(conv: &str, own: &str) -> Result<u32, Error> {
    let fields: Vec<&str> = conv.split(':').collect();
    let ["dm", a, b] = fields.as_slice() else {
        return Err(Error::Rejected);
    };
    let parse = |s: &str| -> Result<u32, Error> {
        let n = u32::from_str_radix(s, 16).map_err(rejected)?;
        if !central_api::profile::valid_tag(n) || format!("{n:04x}") != s {
            return Err(Error::Rejected);
        }
        Ok(n)
    };
    let (left, right) = (parse(a)?, parse(b)?);
    if left >= right {
        return Err(Error::Rejected);
    }
    if *a == own {
        Ok(right)
    } else if *b == own {
        Ok(left)
    } else {
        Err(Error::Rejected)
    }
}

pub(super) async fn send(
    db: &impl ConnectionTrait,
    session: Session<'_>,
    me: Uuid,
    conv: &str,
    sealed: &Sealed,
) -> Result<ChatResponse, Error> {
    let parsed = Parsed::read(conv, sealed)?;
    let own = account_tag(db, me).await?;
    if parsed.header.from != own {
        return Err(Error::Rejected);
    }
    let accounts = if let Some(group) = conv.strip_prefix("grp:") {
        super::super::groups::recipients(db, group, me).await?
    } else {
        let peer = by_tag(db, partner(conv, &own)?)
            .await?
            .ok_or(Error::Rejected)?;
        vec![me, peer.try_get::<Uuid>("", "id").map_err(unavailable)?]
    };
    lock_accounts(db, accounts.clone()).await?;
    caller(db, session, true).await?;
    if conv.starts_with("dm:") {
        authorize(db, me, accounts[1]).await?;
    }
    let key = db.query_one_raw(statement("SELECT d.ik FROM central_identity.chat_devices d JOIN central_identity.accounts a ON a.id = d.account_id WHERE d.account_id = $1 AND d.fingerprint = $2 AND d.vault_epoch = a.vault_epoch",
        vec![me.into(), parsed.header.dev.clone().into()],
    )).await.map_err(unavailable)?.ok_or(Error::Rejected)?;
    let ik: Vec<u8> = key.try_get("", "ik").map_err(unavailable)?;
    central_crypto::chat::verify_message(&ik.try_into().map_err(unavailable)?, &parsed.envelope())
        .map_err(rejected)?;
    let payload = central_api::encode(sealed)?;
    if payload.len() > 65_536 {
        return Err(Error::Rejected);
    }
    if let Some(at) = repeated(db, me, parsed.correlation, &payload).await? {
        return Ok(ChatResponse::Sent {
            id: parsed.header.id,
            at,
        });
    }
    let recipients = recipients(db, me, &accounts, sealed).await?;
    store(db, me, parsed, conv, payload, recipients).await
}

async fn authorize(db: &impl ConnectionTrait, me: Uuid, other: Uuid) -> Result<(), Error> {
    let allowed = db.query_one_raw(statement(
        "SELECT 1 FROM central_identity.social_contacts c WHERE c.low = LEAST($1, $2) AND c.high = GREATEST($1, $2) AND c.accepted
         AND NOT EXISTS (SELECT 1 FROM central_identity.social_blocks b WHERE
            (b.owner = $1 AND b.blocked = $2) OR (b.owner = $2 AND b.blocked = $1))",
        vec![me.into(), other.into()],
    )).await.map_err(unavailable)?.is_some();
    if allowed {
        Ok(())
    } else {
        Err(Error::Rejected)
    }
}

async fn repeated(
    db: &impl ConnectionTrait,
    me: Uuid,
    id: Uuid,
    payload: &[u8],
) -> Result<Option<u64>, Error> {
    let row = db.query_one_raw(statement("SELECT envelope, floor(extract(epoch FROM created_at) * 1000)::bigint AS at FROM central_identity.chat_messages WHERE sender = $1 AND correlation = $2", vec![me.into(), id.into()])).await.map_err(unavailable)?;
    row.map(|r| {
        let stored: Vec<u8> = r.try_get("", "envelope").map_err(unavailable)?;
        if stored != payload {
            return Err(Error::Conflict);
        }
        let at: i64 = r.try_get("", "at").map_err(unavailable)?;
        u64::try_from(at).map_err(unavailable)
    })
    .transpose()
}

async fn recipients(
    db: &impl ConnectionTrait,
    me: Uuid,
    accounts: &[Uuid],
    sealed: &Sealed,
) -> Result<BTreeMap<String, [u8; 48]>, Error> {
    let rows = db.query_all_raw(statement("SELECT d.fingerprint, d.account_id,
        EXISTS (SELECT 1 FROM central_identity.social_blocks b WHERE b.owner = d.account_id AND b.blocked = $2) AS blocked
        FROM central_identity.chat_devices d JOIN central_identity.accounts a ON a.id = d.account_id
        WHERE d.account_id = ANY($1) AND d.vault_epoch = a.vault_epoch ORDER BY d.fingerprint", vec![accounts.to_vec().into(), me.into()])).await.map_err(unavailable)?;
    let mut wraps = BTreeMap::new();
    for wrap in &sealed.wraps {
        if wraps
            .insert(wrap.device.clone(), decode(&wrap.k)?)
            .is_some()
        {
            return Err(Error::Rejected);
        }
    }
    let mut owners = BTreeSet::new();
    let mut devices = Vec::new();
    for row in &rows {
        let id: String = row.try_get("", "fingerprint").map_err(unavailable)?;
        let owner: Uuid = row.try_get("", "account_id").map_err(unavailable)?;
        owners.insert(owner);
        if !wraps.contains_key(&id) {
            return Err(Error::Conflict);
        }
        let blocked = row.try_get::<bool>("", "blocked").map_err(unavailable)?;
        devices.push((id, owner, blocked));
    }
    if wraps.len() != rows.len() || owners.len() != accounts.len() {
        return Err(Error::Conflict);
    }
    // Every wrap is validated before any delivery is left out, so neither
    // the directory nor the completeness answers show who is left out.
    let room = room(db, me, &devices).await?;
    let mut others = false;
    let mut heard = false;
    for (id, owner, blocked) in devices {
        let fits = room.get(&id).is_none_or(|held| held.fits(owner == me));
        if owner == me {
            // The sender's own device is full: refused, as it always was. A
            // message its own devices cannot keep would vanish from them.
            if !fits {
                return Err(Error::Busy);
            }
            continue;
        }
        others = true;
        // Whether anybody else has room is asked of every device, a
        // blocker's too: setting or lifting a block does not change the
        // answer to the next send. (A blocker's device takes none of the
        // sender's messages, so over a whole share it can still show:
        // docs/central-chat.md.)
        heard |= fits;
        if blocked || !fits {
            wraps.remove(&id);
        }
    }
    // Nobody else has room for it (a direct message's only reader among
    // them): refused, so that the sender is not told it was sent.
    if others && !heard {
        return Err(Error::Busy);
    }
    Ok(wraps)
}

/// What a device's inbox holds, and how much of it is one sender's.
struct Held {
    total: i64,
    theirs: i64,
}

impl Held {
    /// Whether one more delivery fits: the inbox is not full, and for
    /// another account's device the sender's share of it is not used up.
    fn fits(&self, own: bool) -> bool {
        self.total < INBOX && (own || self.theirs < CHAT_SENDER_SHARE)
    }
}

/// How full the inbox of each of `devices` is, and how much of it is
/// messages of `me`; a device that holds nothing is not in the answer.
async fn room(
    db: &impl ConnectionTrait,
    me: Uuid,
    devices: &[(String, Uuid, bool)],
) -> Result<BTreeMap<String, Held>, Error> {
    let ids: Vec<String> = devices.iter().map(|(id, _, _)| id.clone()).collect();
    let rows = db
        .query_all_raw(statement(
            "SELECT i.device, count(*)::bigint AS total, count(m.id)::bigint AS theirs
             FROM central_identity.chat_deliveries i
             LEFT JOIN central_identity.chat_messages m ON m.id = i.message_id AND m.sender = $2
             WHERE i.device = ANY($1) GROUP BY i.device",
            vec![ids.into(), me.into()],
        ))
        .await
        .map_err(unavailable)?;
    rows.iter()
        .map(|row| {
            Ok((
                row.try_get("", "device").map_err(unavailable)?,
                Held {
                    total: row.try_get("", "total").map_err(unavailable)?,
                    theirs: row.try_get("", "theirs").map_err(unavailable)?,
                },
            ))
        })
        .collect()
}

async fn store(
    db: &impl ConnectionTrait,
    me: Uuid,
    parsed: Parsed,
    conv: &str,
    payload: Vec<u8>,
    recipients: BTreeMap<String, [u8; 48]>,
) -> Result<ChatResponse, Error> {
    let at: i64 = db
        .query_one_raw(statement(
            "SELECT floor(extract(epoch FROM pg_catalog.clock_timestamp()) * 1000)::bigint AS at",
            vec![],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Unavailable)?
        .try_get("", "at")
        .map_err(unavailable)?;
    let now = u64::try_from(at).map_err(unavailable)?;
    if parsed.header.ts.abs_diff(now) > 600_000 {
        return Err(Error::Rejected);
    }
    let row = db.query_one_raw(statement("SELECT count(*) AS total, count(*) FILTER (WHERE created_at > pg_catalog.clock_timestamp() - interval '1 minute') AS recent FROM central_identity.chat_messages WHERE sender = $1", vec![me.into()])).await.map_err(unavailable)?.ok_or(Error::Unavailable)?;
    if row.try_get::<i64>("", "total").map_err(unavailable)? >= 5000
        || row.try_get::<i64>("", "recent").map_err(unavailable)? >= 60
    {
        return Err(Error::Busy);
    }
    let inserted = db.query_one_raw(statement("INSERT INTO central_identity.chat_messages (sender, correlation, device, conversation, envelope) VALUES ($1, $2, $3, $4, $5) RETURNING id, floor(extract(epoch FROM created_at) * 1000)::bigint AS at",
        vec![me.into(), parsed.correlation.into(), parsed.header.dev.into(), conv.into(), payload.into()],
    )).await.map_err(unavailable)?.ok_or(Error::Unavailable)?;
    let id: Uuid = inserted.try_get("", "id").map_err(unavailable)?;
    let at: i64 = inserted.try_get("", "at").map_err(unavailable)?;
    for (device, key) in recipients {
        db.query_one_raw(statement("INSERT INTO central_identity.chat_deliveries (message_id, device, key_wrap) VALUES ($1, $2, $3) RETURNING id", vec![id.into(), device.into(), key.to_vec().into()])).await.map_err(unavailable)?.ok_or(Error::Unavailable)?;
    }
    Ok(ChatResponse::Sent {
        id: parsed.header.id,
        at: u64::try_from(at).map_err(unavailable)?,
    })
}

#[cfg(test)]
mod tests {
    use super::{CHAT_SENDER_SHARE, Held, INBOX, partner};

    #[test]
    fn an_inbox_has_room_until_it_is_full_or_the_sender_s_share_is_used_up() {
        let held = |total, theirs| Held { total, theirs };
        // another account's device: the sender's share, then the inbox
        assert!(held(INBOX - 1, CHAT_SENDER_SHARE - 1).fits(false));
        assert!(!held(INBOX - 1, CHAT_SENDER_SHARE).fits(false));
        assert!(!held(INBOX, 0).fits(false));
        // the sender's own device: the inbox only
        assert!(held(INBOX - 1, INBOX - 1).fits(true));
        assert!(!held(INBOX, 0).fits(true));
        // two senders at their share fill an inbox, one alone does not
        assert_eq!(2 * CHAT_SENDER_SHARE, INBOX);
    }

    #[test]
    fn only_canonical_owned_direct_conversations_resolve() {
        assert_eq!(partner("dm:0007:3fa2", "0007"), Ok(0x3fa2));
        assert_eq!(partner("dm:0007:3fa2", "3fa2"), Ok(7));
        for (conv, own) in [
            ("dm:3fa2:0007", "0007"),
            ("dm:0007:0007", "0007"),
            ("dm:0007:3fa2", "ffff"),
            ("dm:0007:03fa2", "0007"),
            ("dm:0007:3FA2", "0007"),
            ("grp:any", "0007"),
            ("dm:0007:ffffffff", "0007"),
        ] {
            assert!(partner(conv, own).is_err());
        }
    }
}
