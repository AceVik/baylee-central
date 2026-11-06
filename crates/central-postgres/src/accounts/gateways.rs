use super::tokens::LIVE_TOKEN;
use super::{
    AccountStore, Error, NOT_DISABLED, Token, any_origin, number, origins, statement, uuid,
};
use central_api::{
    gateway::{GatewayActivity, GatewayFigures, GatewayListing, GatewayMetadata, validate_origin},
    is_official_origin,
};
use sea_orm::{ConnectionTrait, DatabaseTransaction, TransactionTrait, Value, prelude::Uuid};

pub(super) fn valid_id(id: [u8; 16]) -> bool {
    id[6] >> 4 == 7 && id[8] >> 6 == 2
}

/// What a beat does to its row's activity, from parameters 5 to 8
/// ([`counts`]): a pulse's four counts with the time, or all five columns
/// cleared by a plain heartbeat's NULLs. Part of the beat's one UPDATE, so
/// the five-second limit and every other condition hold for it as well.
const ACTIVITY: &str =
    "activity_players=$5,activity_playing=$6,activity_tables=$7,activity_games=$8,
    activity_at=CASE WHEN $5::integer IS NULL THEN NULL ELSE pg_catalog.clock_timestamp() END";

/// A beat's four activity parameters, in [`ACTIVITY`]'s order: a pulse's
/// counts within their bound and with no more players in a game than
/// connected, or NULLs for a plain heartbeat.
fn counts(activity: Option<GatewayActivity>) -> Result<[Value; 4], Error> {
    let Some(activity) = activity else {
        let none = || Value::from(None::<i32>);
        return Ok([none(), none(), none(), none()]);
    };
    activity.validate()?;
    let count = |n: u32| {
        i32::try_from(n)
            .map(Value::from)
            .map_err(|_| Error::Rejected)
    };
    Ok([
        count(activity.players)?,
        count(activity.playing)?,
        count(activity.tables)?,
        count(activity.games)?,
    ])
}

async fn lock_owner(tx: &DatabaseTransaction, session: &Token) -> Result<Uuid, Error> {
    let owner = super::objects::locked_owner(tx, session, false).await?;
    let row = tx
        .query_one_raw(statement(
            "SELECT restored FROM central_identity.accounts WHERE id=$1",
            vec![owner.into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?
        .ok_or(Error::Rejected)?;
    if !row
        .try_get::<bool>("", "restored")
        .map_err(|_| Error::Unavailable)?
    {
        return Err(Error::Rejected);
    }
    Ok(owner)
}

impl AccountStore {
    /// Reserve a gateway for the authenticated account; separate operator approval is mandatory.
    ///
    /// # Errors
    /// An owned, never-approved reservation can be retried to recover a lost credential response.
    ///
    /// Rejects invalid metadata/session, foreign or approved origins and the owner quota.
    /// The official gateway's origins are the bridge's alone
    /// ([`is_official_origin`]): a registered credential for one would redeem
    /// that audience's tickets a second time, so it is a conflict before
    /// anything else is looked at.
    pub async fn register_gateway(
        &self,
        session: &Token,
        origin: &str,
        metadata: &GatewayMetadata,
        credential: &Token,
        heartbeat: &Token,
        development: bool,
    ) -> Result<[u8; 16], Error> {
        if is_official_origin(origin) {
            return Err(Error::Conflict);
        }
        validate_origin(origin, development)?;
        metadata.validate(origin)?;
        if credential.digest() == heartbeat.digest() {
            return Err(Error::Rejected);
        }
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let owner = lock_owner(&tx, session).await?;
        let row = tx.query_one_raw(statement(
            "INSERT INTO central_identity.gateways(owner_id,origin,name,version,website,credential_digest,heartbeat_digest)
             SELECT $1,$2,$3,$4,$5,$6,$7 WHERE (SELECT count(*) FROM central_identity.gateways WHERE owner_id=$1)<8
             OR EXISTS(SELECT 1 FROM central_identity.gateways g WHERE g.origin=$2 AND g.owner_id=$1
                 AND NOT EXISTS(SELECT 1 FROM central_identity.gateway_approvals p WHERE p.gateway_id=g.id))
             ON CONFLICT(origin) DO UPDATE SET (name,version,website,credential_digest,heartbeat_digest) =
                 (EXCLUDED.name,EXCLUDED.version,EXCLUDED.website,EXCLUDED.credential_digest,EXCLUDED.heartbeat_digest)
             WHERE gateways.owner_id=EXCLUDED.owner_id AND NOT EXISTS
                 (SELECT 1 FROM central_identity.gateway_approvals p WHERE p.gateway_id=gateways.id)
             RETURNING id",
            vec![owner.into(),origin.into(),metadata.name.clone().into(),metadata.version.clone().into(),metadata.website.clone().into(),credential.digest().to_vec().into(),heartbeat.digest().to_vec().into()]
        )).await.map_err(|_| Error::Conflict)?.ok_or(Error::Busy)?;
        let id: Uuid = row.try_get("", "id").map_err(|_| Error::Unavailable)?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(*id.as_bytes())
    }

    /// Return a bounded page of approved gateways; no private account or credential fields.
    ///
    /// # Errors
    /// Rejects invalid cursors and unavailable storage.
    pub async fn gateway_directory(
        &self,
        after: Option<[u8; 16]>,
    ) -> Result<Vec<GatewayListing>, Error> {
        if after.is_some_and(|id| !valid_id(id)) {
            return Err(Error::Rejected);
        }
        let rows = self.db.query_all_raw(statement(
            "SELECT g.id,g.origin,g.name,g.version,g.website,
             COALESCE(g.last_seen > pg_catalog.clock_timestamp()-interval '90 seconds',false) AS online
             FROM central_identity.gateways g JOIN central_identity.gateway_approvals p ON p.gateway_id=g.id AND p.enabled
             WHERE ($1::uuid IS NULL OR g.id>$1) ORDER BY g.id LIMIT 100", vec![after.map(uuid).into()]
        )).await.map_err(|_| Error::Unavailable)?;
        rows.into_iter()
            .map(|row| {
                let id: Uuid = row.try_get("", "id").map_err(|_| Error::Unavailable)?;
                Ok(GatewayListing {
                    id: *id.as_bytes(),
                    origin: row.try_get("", "origin").map_err(|_| Error::Unavailable)?,
                    metadata: GatewayMetadata {
                        name: row.try_get("", "name").map_err(|_| Error::Unavailable)?,
                        version: row.try_get("", "version").map_err(|_| Error::Unavailable)?,
                        website: row.try_get("", "website").map_err(|_| Error::Unavailable)?,
                    },
                    online: row.try_get("", "online").map_err(|_| Error::Unavailable)?,
                })
            })
            .collect()
    }

    /// One bounded page of what gateways report (`docs/gateway-registry.md`
    /// §"Activity and the public figures"): the id and the four counts of
    /// each gateway that is approved and enabled, online by the directory's
    /// 90 seconds, and whose activity was reported within them. A gateway
    /// that sends plain heartbeats, or stopped, is not in it. Ids and counts
    /// only: no name, origin, owner or credential field.
    ///
    /// # Errors
    /// Rejects invalid cursors and unavailable storage.
    pub async fn gateway_figures(
        &self,
        after: Option<[u8; 16]>,
    ) -> Result<Vec<GatewayFigures>, Error> {
        if after.is_some_and(|id| !valid_id(id)) {
            return Err(Error::Rejected);
        }
        let rows = self.db.query_all_raw(statement(
            "SELECT g.id,g.activity_players,g.activity_playing,g.activity_tables,g.activity_games
             FROM central_identity.gateways g JOIN central_identity.gateway_approvals p ON p.gateway_id=g.id AND p.enabled
             WHERE ($1::uuid IS NULL OR g.id>$1)
             AND g.last_seen > pg_catalog.clock_timestamp()-interval '90 seconds'
             AND g.activity_at > pg_catalog.clock_timestamp()-interval '90 seconds'
             ORDER BY g.id LIMIT 100", vec![after.map(uuid).into()]
        )).await.map_err(|_| Error::Unavailable)?;
        rows.into_iter()
            .map(|row| {
                let id: Uuid = row.try_get("", "id").map_err(|_| Error::Unavailable)?;
                let count = |column: &str| {
                    row.try_get::<i32>("", column)
                        .ok()
                        .and_then(|n| u32::try_from(n).ok())
                        .ok_or(Error::Unavailable)
                };
                let activity = GatewayActivity {
                    players: count("activity_players")?,
                    playing: count("activity_playing")?,
                    tables: count("activity_tables")?,
                    games: count("activity_games")?,
                };
                // The table's CHECKs hold the bound and that no more play than
                // are connected; a row outside either is not passed on.
                activity.validate().map_err(|_| Error::Unavailable)?;
                Ok(GatewayFigures {
                    id: *id.as_bytes(),
                    activity,
                })
            })
            .collect()
    }

    /// How many accounts can sign in: those with a stored credential, the
    /// administration's "enrolled" ([`Self::admin_overview`]). An unfinished
    /// registration or move has none and is not counted; a deleted account's
    /// row is gone. One count over the table: its caller keeps the answer
    /// for a while rather than ask for every request.
    ///
    /// # Errors
    /// Unavailable storage.
    pub async fn account_count(&self) -> Result<u64, Error> {
        let row = self
            .db
            .query_one_raw(statement(
                "SELECT count(*)::bigint AS accounts FROM central_identity.accounts
                 WHERE credential IS NOT NULL",
                vec![],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Unavailable)?;
        row.try_get::<i64>("", "accounts")
            .ok()
            .and_then(|n| u64::try_from(n).ok())
            .ok_or(Error::Unavailable)
    }

    /// Authenticate a gateway's redemption credential or a live gateway token
    /// with the `sso` scope (migration 000014), never a heartbeat credential
    /// or a listing-only token. The official gateway holds neither
    /// (migration 000009); the bridge is its authority. A token
    /// authenticates while its gateway awaits approval (the token makes its
    /// owner answerable and can be revoked; the approval is the directory's);
    /// a credential pair needs the approval.
    ///
    /// # Errors
    /// Rejects disabled gateways, an unapproved credential pair, revoked or
    /// expired tokens and incorrect credentials.
    pub async fn authenticate_gateway(
        &self,
        credential: &Token,
    ) -> Result<([u8; 16], String), Error> {
        // Two lookups by a unique digest each; a 256-bit random token cannot
        // be both a credential and a token, so at most one row answers.
        let row=self.db.query_one_raw(statement(
            &format!("SELECT g.id,g.origin FROM central_identity.gateways g JOIN central_identity.gateway_approvals p
             ON p.gateway_id=g.id AND p.enabled WHERE g.credential_digest=$1 AND NOT g.official
             UNION ALL
             SELECT g.id,g.origin FROM central_identity.gateway_tokens t
             JOIN central_identity.gateways g ON g.id=t.gateway_id
             WHERE t.digest=$1 AND t.sso AND {LIVE_TOKEN} AND NOT g.official AND {NOT_DISABLED} LIMIT 1"), vec![credential.digest().to_vec().into()]
        )).await.map_err(|_| Error::Unavailable)?.ok_or(Error::Rejected)?;
        let id: Uuid = row.try_get("", "id").map_err(|_| Error::Unavailable)?;
        Ok((
            *id.as_bytes(),
            row.try_get("", "origin").map_err(|_| Error::Unavailable)?,
        ))
    }

    /// The account whose gateway `bearer` is a redemption credential, a
    /// heartbeat credential or a token of, whatever the gateway's approval
    /// and whether the token is live; none for any other token. For
    /// counting a gateway's bearers together (`central-identity`,
    /// `limits.rs`), never for authorization: nothing is granted by an
    /// answer, and the caller asks only about a bearer another request
    /// was just answered for.
    ///
    /// # Errors
    /// Storage failure.
    pub async fn bearer_owner(&self, bearer: &Token) -> Result<Option<[u8; 16]>, Error> {
        // Three lookups by a unique digest each. A registered gateway has
        // an owner (`gateway_official`); the official one holds none of
        // the three.
        let row = self
            .db
            .query_one_raw(statement(
                "SELECT g.owner_id FROM central_identity.gateways g
                 WHERE g.credential_digest = $1 AND NOT g.official
                 UNION ALL
                 SELECT g.owner_id FROM central_identity.gateways g
                 WHERE g.heartbeat_digest = $1 AND NOT g.official
                 UNION ALL
                 SELECT g.owner_id FROM central_identity.gateway_tokens t
                 JOIN central_identity.gateways g ON g.id = t.gateway_id
                 WHERE t.digest = $1 AND NOT g.official LIMIT 1",
                vec![bearer.digest().to_vec().into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        let Some(row) = row else {
            return Ok(None);
        };
        let owner: Uuid = row
            .try_get("", "owner_id")
            .map_err(|_| Error::Unavailable)?;
        Ok(Some(*owner.as_bytes()))
    }

    /// Update public metadata and central-clock liveness only, at most once per five seconds.
    /// The credential is a registered gateway's heartbeat token or a live
    /// gateway token with the `listing` scope (migration 000014), which this
    /// marks used. A token beats while its gateway awaits approval, so its
    /// owner sees it seen; the directory lists approved gateways only. Returns its origin.
    /// A plain heartbeat reports no activity and clears what a pulse stored.
    ///
    /// # Errors
    /// Rejects wrong credentials, disabled gateways, an unapproved heartbeat
    /// credential, revoked or expired tokens, unsafe metadata and excessive
    /// heartbeat rate.
    pub async fn gateway_heartbeat(
        &self,
        token: &Token,
        metadata: &GatewayMetadata,
    ) -> Result<String, Error> {
        self.registered_beat(token, metadata, None).await
    }

    /// [`Self::gateway_heartbeat`] with the gateway's activity
    /// (`docs/gateway-registry.md` §"Activity and the public figures"): the
    /// same credential, checks and limit, and the four counts kept with the
    /// time. The counts are the gateway's claim; only their bound is checked,
    /// and that no more players are in a game than connected. Returns its origin.
    ///
    /// # Errors
    /// As [`Self::gateway_heartbeat`], and a count above
    /// [`GatewayActivity::MOST`] or more playing than players, before
    /// anything is read or written.
    pub async fn gateway_pulse(
        &self,
        token: &Token,
        metadata: &GatewayMetadata,
        activity: GatewayActivity,
    ) -> Result<String, Error> {
        self.registered_beat(token, metadata, Some(activity)).await
    }

    async fn registered_beat(
        &self,
        token: &Token,
        metadata: &GatewayMetadata,
        activity: Option<GatewayActivity>,
    ) -> Result<String, Error> {
        let [players, playing, tables, games] = counts(activity)?;
        let row=self.db.query_one_raw(statement(
            &format!("SELECT g.id,g.origin,false AS by_token FROM central_identity.gateways g JOIN central_identity.gateway_approvals p
             ON p.gateway_id=g.id AND p.enabled WHERE g.heartbeat_digest=$1 AND NOT g.official
             UNION ALL
             SELECT g.id,g.origin,true FROM central_identity.gateway_tokens t
             JOIN central_identity.gateways g ON g.id=t.gateway_id
             WHERE t.digest=$1 AND t.listing AND {LIVE_TOKEN} AND NOT g.official AND {NOT_DISABLED} LIMIT 1"),vec![token.digest().to_vec().into()]
        )).await.map_err(|_|Error::Unavailable)?.ok_or(Error::Rejected)?;
        let gateway: Uuid = row.try_get("", "id").map_err(|_| Error::Unavailable)?;
        let origin: String = row.try_get("", "origin").map_err(|_| Error::Unavailable)?;
        let by_token: bool = row
            .try_get("", "by_token")
            .map_err(|_| Error::Unavailable)?;
        metadata.validate(&origin)?;
        if by_token {
            self.touch_gateway_token(token).await?;
        }
        let updated=self.db.execute_raw(statement(
            &format!("UPDATE central_identity.gateways g SET name=$2,version=$3,website=$4,last_seen=pg_catalog.clock_timestamp(),
             {ACTIVITY}
             WHERE id=$1 AND NOT official
             AND (last_seen IS NULL OR last_seen<=pg_catalog.clock_timestamp()-interval '5 seconds')
             AND {NOT_DISABLED}"),
            vec![gateway.into(),metadata.name.clone().into(),metadata.version.clone().into(),metadata.website.clone().into(),players,playing,tables,games]
        )).await.map_err(|_|Error::Unavailable)?;
        if updated.rows_affected() != 1 {
            return Err(Error::Busy);
        }
        Ok(origin)
    }

    /// The official gateway's heartbeat, which its caller authenticated with the
    /// bridge credential: the public metadata and liveness of the official row
    /// and of no other, at most once per five seconds. The row is the official
    /// gateway while its origin is one of the bridge's `origins` (canonical
    /// first), which a live service (`development` false) holds to the
    /// compiled ones ([`is_official_origin`]); the website may be on any of
    /// them, so a gateway that still names its old address keeps its entry.
    /// A plain heartbeat reports no activity and clears what a pulse stored.
    ///
    /// # Errors
    /// Rejects another origin, metadata on none of them, an unlisted,
    /// unapproved or disabled official row, and an excessive heartbeat rate
    /// (`Busy`).
    pub async fn official_heartbeat<S: AsRef<str> + Sync>(
        &self,
        set: &[S],
        metadata: &GatewayMetadata,
        development: bool,
    ) -> Result<(), Error> {
        self.official_beat(set, metadata, development, None).await
    }

    /// [`Self::official_heartbeat`] with the official gateway's activity, as
    /// [`Self::gateway_pulse`] for a registered one.
    ///
    /// # Errors
    /// As [`Self::official_heartbeat`], and a count above
    /// [`GatewayActivity::MOST`] or more playing than players, before
    /// anything is read or written.
    pub async fn official_pulse<S: AsRef<str> + Sync>(
        &self,
        set: &[S],
        metadata: &GatewayMetadata,
        development: bool,
        activity: GatewayActivity,
    ) -> Result<(), Error> {
        self.official_beat(set, metadata, development, Some(activity))
            .await
    }

    async fn official_beat<S: AsRef<str> + Sync>(
        &self,
        set: &[S],
        metadata: &GatewayMetadata,
        development: bool,
        activity: Option<GatewayActivity>,
    ) -> Result<(), Error> {
        let [players, playing, tables, games] = counts(activity)?;
        if set.is_empty() || (!development && !set.iter().all(|o| is_official_origin(o.as_ref()))) {
            return Err(Error::Rejected);
        }
        for origin in set {
            validate_origin(origin.as_ref(), development)?;
        }
        if !set.iter().any(|o| metadata.validate(o.as_ref()).is_ok()) {
            return Err(Error::Rejected);
        }
        let any = any_origin(1);
        self.db.query_one_raw(statement(
            &format!("SELECT g.id FROM central_identity.gateways g JOIN central_identity.gateway_approvals p
             ON p.gateway_id=g.id AND p.enabled WHERE g.official AND g.origin={any}"),vec![origins(set)?]
        )).await.map_err(|_|Error::Unavailable)?.ok_or(Error::Rejected)?;
        let updated=self.db.execute_raw(statement(
            &format!("UPDATE central_identity.gateways g SET name=$2,version=$3,website=$4,last_seen=pg_catalog.clock_timestamp(),
             {ACTIVITY}
             WHERE official AND origin={any}
             AND (last_seen IS NULL OR last_seen<=pg_catalog.clock_timestamp()-interval '5 seconds')
             AND EXISTS(SELECT 1 FROM central_identity.gateway_approvals p WHERE p.gateway_id=g.id AND p.enabled)"),
            vec![origins(set)?,metadata.name.clone().into(),metadata.version.clone().into(),metadata.website.clone().into(),players,playing,tables,games]
        )).await.map_err(|_|Error::Unavailable)?;
        if updated.rows_affected() != 1 {
            return Err(Error::Busy);
        }
        Ok(())
    }

    /// Replace one owned gateway's administrator list under an account/session and revision check.
    /// Registry ownership alone never authorizes access to the gateway's admin backend.
    ///
    /// # Errors
    /// Rejects foreign ownership, stale revision, invalid/duplicate accounts and oversized lists.
    pub async fn set_gateway_admins(
        &self,
        session: &Token,
        gateway: [u8; 16],
        expected: u64,
        accounts: &[[u8; 16]],
    ) -> Result<u64, Error> {
        if !valid_id(gateway) || accounts.len() > 32 || accounts.iter().any(|id| !valid_id(*id)) {
            return Err(Error::Rejected);
        }
        let unique: std::collections::BTreeSet<_> = accounts.iter().collect();
        if unique.len() != accounts.len() {
            return Err(Error::Rejected);
        }
        let expected = number(expected)?;
        if expected < 1 || expected == i64::MAX {
            return Err(Error::Rejected);
        }
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let owner = lock_owner(&tx, session).await?;
        let result = tx
            .execute_raw(statement(
                "UPDATE central_identity.gateways SET admin_revision=admin_revision+1
             WHERE id=$1 AND owner_id=$2 AND admin_revision=$3",
                vec![uuid(gateway).into(), owner.into(), expected.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        if result.rows_affected() != 1 {
            return Err(Error::Conflict);
        }
        tx.execute_raw(statement(
            "DELETE FROM central_identity.gateway_admins WHERE gateway_id=$1",
            vec![uuid(gateway).into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?;
        for account in accounts {
            let result=tx.execute_raw(statement(
                "INSERT INTO central_identity.gateway_admins(gateway_id,account_id)
                 SELECT $1,id FROM central_identity.accounts WHERE id=$2 AND credential IS NOT NULL",
                vec![uuid(gateway).into(),uuid(*account).into()]
            )).await.map_err(|_|Error::Unavailable)?;
            if result.rows_affected() != 1 {
                return Err(Error::Rejected);
            }
        }
        // Whoever is no longer an administrator loses the token that said so, at once.
        tx.execute_raw(statement(
            "DELETE FROM central_identity.gateway_sids i USING central_identity.sessions s,
                 central_identity.gateways g
             WHERE g.id = $1 AND i.stream = g.origin AND i.purpose = 'admin'
             AND s.digest = i.session_digest AND NOT EXISTS(
                 SELECT 1 FROM central_identity.gateway_admins m
                 WHERE m.gateway_id = g.id AND m.account_id = s.account_id)",
            vec![uuid(gateway).into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        u64::try_from(expected + 1).map_err(|_| Error::Unavailable)
    }
}
