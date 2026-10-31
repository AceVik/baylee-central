//! An account's own sessions and gateways on the website, and the gateway
//! tokens it mints (`docs/gateway-registry.md` §"Gateway tokens", migration
//! 000014). Every function takes the browser session's cookie and finds the
//! account through a live browser session of the current credential; a
//! native bearer is no browser session here. Only digests of tokens are
//! read or written.
use super::{AccountStore, Error, Session, Token, statement};
use central_api::{
    gateway::{GatewayMetadata, GatewayState, validate_origin},
    is_official_origin,
    owner::{
        GatewayScopes, MAX_TOKEN_DAYS, MyGateway, SessionInfo, SessionKind, TOKENS_PER_GATEWAY,
        TokenInfo, token_name,
    },
};
use sea_orm::{ConnectionTrait, QueryResult, TransactionTrait, prelude::Uuid};

/// Gateways one account may register, as `RegisterGateway` allows.
const GATEWAYS_PER_OWNER: i64 = 8;
/// Tokens one gateway may be given in a day, live or not: a bound on rows a
/// churn of creating and revoking could write.
const TOKENS_PER_DAY: i64 = 32;
/// The state of `gateways g` from `gateway_approvals p`, left-joined.
pub(super) const STATE: &str = "CASE WHEN p.gateway_id IS NULL THEN 'pending' WHEN p.enabled THEN 'approved' ELSE 'disabled' END";
/// The columns of a [`TokenInfo`], from `gateway_tokens t`.
pub(super) const TOKEN_COLUMNS: &str = "t.id, t.name, t.sso, t.listing,
    floor(extract(epoch FROM t.created_at))::bigint AS created_at,
    floor(extract(epoch FROM t.expires_at))::bigint AS expires_at,
    floor(extract(epoch FROM t.revoked_at))::bigint AS revoked_at,
    floor(extract(epoch FROM t.last_used_at))::bigint AS last_used_at";
/// A token that authenticates: neither revoked nor expired.
pub(super) const LIVE_TOKEN: &str =
    "t.revoked_at IS NULL AND t.expires_at > pg_catalog.clock_timestamp()";

pub(super) fn state(row: &QueryResult) -> Result<GatewayState, Error> {
    match row
        .try_get::<String>("", "state")
        .map_err(|_| Error::Unavailable)?
        .as_str()
    {
        "pending" => Ok(GatewayState::Pending),
        "approved" => Ok(GatewayState::Approved),
        "disabled" => Ok(GatewayState::Disabled),
        _ => Err(Error::Unavailable),
    }
}

pub(super) fn token_info(row: &QueryResult) -> Result<TokenInfo, Error> {
    let get = |column| {
        row.try_get::<bool>("", column)
            .map_err(|_| Error::Unavailable)
    };
    let id: Uuid = row.try_get("", "id").map_err(|_| Error::Unavailable)?;
    Ok(TokenInfo {
        id: *id.as_bytes(),
        name: row.try_get("", "name").map_err(|_| Error::Unavailable)?,
        scopes: GatewayScopes {
            sso: get("sso")?,
            listing: get("listing")?,
        },
        created_at: row
            .try_get("", "created_at")
            .map_err(|_| Error::Unavailable)?,
        expires_at: row
            .try_get("", "expires_at")
            .map_err(|_| Error::Unavailable)?,
        revoked_at: row
            .try_get("", "revoked_at")
            .map_err(|_| Error::Unavailable)?,
        last_used_at: row
            .try_get("", "last_used_at")
            .map_err(|_| Error::Unavailable)?,
    })
}

pub(super) fn metadata(row: &QueryResult) -> Result<GatewayMetadata, Error> {
    Ok(GatewayMetadata {
        name: row.try_get("", "name").map_err(|_| Error::Unavailable)?,
        version: row.try_get("", "version").map_err(|_| Error::Unavailable)?,
        website: row.try_get("", "website").map_err(|_| Error::Unavailable)?,
    })
}

/// The account behind the live browser session `browser`, and whether its
/// password sign-in is less than 30 minutes old. `lock` takes the account
/// row, so two changes of one account run one after the other.
async fn owner(
    db: &impl ConnectionTrait,
    browser: &Token,
    lock: bool,
) -> Result<(Uuid, bool), Error> {
    let session = Session::Browser(browser);
    let row = db
        .query_one_raw(statement(
            &format!(
                "SELECT a.id, COALESCE(s.proven_at, s.created_at) > pg_catalog.clock_timestamp() - interval '30 minutes' AS fresh
                 FROM central_identity.accounts a JOIN central_identity.sessions s
                 ON s.account_id = a.id AND s.credential_revision = a.credential_revision
                 WHERE s.digest = $1 AND s.kind = 'browser' AND a.credential IS NOT NULL
                 AND s.expires_at > pg_catalog.clock_timestamp(){}",
                if lock { " FOR UPDATE OF a" } else { "" }
            ),
            vec![session.digest().to_vec().into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?
        .ok_or(Error::Rejected)?;
    Ok((
        row.try_get("", "id").map_err(|_| Error::Unavailable)?,
        row.try_get("", "fresh").map_err(|_| Error::Unavailable)?,
    ))
}

impl AccountStore {
    /// The live sessions of the account behind `browser`, the asking one
    /// first, each with the gateways it is signed in to now.
    ///
    /// # Errors
    /// `Rejected` without a live browser session; storage failure.
    pub async fn owner_sessions(&self, browser: &Token) -> Result<Vec<SessionInfo>, Error> {
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let (account, _) = owner(&tx, browser, false).await?;
        let rows = tx
            .query_all_raw(statement(
                "SELECT s.kind, s.digest = $2 AS current,
                     floor(extract(epoch FROM COALESCE(s.proven_at, s.created_at)))::bigint AS signed_in_at,
                     floor(extract(epoch FROM s.expires_at))::bigint AS expires_at,
                     (SELECT string_agg(DISTINCT l.audience, ' ' ORDER BY l.audience)
                      FROM central_identity.gateway_leases l
                      WHERE l.session_digest = s.digest AND l.expires_at > pg_catalog.clock_timestamp())
                     AS gateways
                 FROM central_identity.sessions s JOIN central_identity.accounts a
                 ON a.id = s.account_id AND a.credential_revision = s.credential_revision
                 WHERE s.account_id = $1 AND s.expires_at > pg_catalog.clock_timestamp()
                 ORDER BY current DESC, s.kind LIMIT 4",
                vec![
                    account.into(),
                    Session::Browser(browser).digest().to_vec().into(),
                ],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        rows.iter()
            .map(|row| {
                let kind: String = row.try_get("", "kind").map_err(|_| Error::Unavailable)?;
                let gateways: Option<String> = row
                    .try_get("", "gateways")
                    .map_err(|_| Error::Unavailable)?;
                Ok(SessionInfo {
                    kind: match kind.as_str() {
                        "native" => SessionKind::Native,
                        "browser" => SessionKind::Browser,
                        _ => return Err(Error::Unavailable),
                    },
                    current: row.try_get("", "current").map_err(|_| Error::Unavailable)?,
                    signed_in_at: row
                        .try_get("", "signed_in_at")
                        .map_err(|_| Error::Unavailable)?,
                    expires_at: row
                        .try_get("", "expires_at")
                        .map_err(|_| Error::Unavailable)?,
                    // an audience is an origin, which holds no space
                    gateways: gateways
                        .map(|text| text.split(' ').map(str::to_owned).collect())
                        .unwrap_or_default(),
                })
            })
            .collect()
    }

    /// End the client (native) session of the account behind `browser`,
    /// and with it its tickets and its leases at gateways.
    ///
    /// # Errors
    /// `Rejected` without a live browser session; storage failure.
    pub async fn end_client_session(&self, browser: &Token) -> Result<(), Error> {
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let (account, _) = owner(&tx, browser, true).await?;
        tx.execute_raw(statement(
            "DELETE FROM central_identity.sessions WHERE account_id = $1 AND kind = 'native'",
            vec![account.into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?;
        tx.commit().await.map_err(|_| Error::Unavailable)
    }

    /// The gateways the account behind `browser` owns, newest first, with
    /// their live tokens and those revoked or expired in the last 30 days.
    ///
    /// # Errors
    /// `Rejected` without a live browser session; storage failure.
    pub async fn owner_gateways(&self, browser: &Token) -> Result<Vec<MyGateway>, Error> {
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let (account, _) = owner(&tx, browser, false).await?;
        let rows = tx
            .query_all_raw(statement(
                &format!(
                    "SELECT g.id, g.origin, g.name, g.version, g.website, {STATE} AS state,
                         COALESCE(g.last_seen > pg_catalog.clock_timestamp() - interval '90 seconds', false) AS online,
                         floor(extract(epoch FROM g.last_seen))::bigint AS last_seen,
                         floor(extract(epoch FROM g.created_at))::bigint AS created_at
                     FROM central_identity.gateways g
                     LEFT JOIN central_identity.gateway_approvals p ON p.gateway_id = g.id
                     WHERE g.owner_id = $1 ORDER BY g.id DESC LIMIT $2"
                ),
                vec![account.into(), GATEWAYS_PER_OWNER.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        let tokens = tx
            .query_all_raw(statement(
                &format!(
                    "SELECT t.gateway_id, {TOKEN_COLUMNS}
                     FROM central_identity.gateway_tokens t JOIN central_identity.gateways g ON g.id = t.gateway_id
                     WHERE g.owner_id = $1
                     AND COALESCE(t.revoked_at, t.expires_at) > pg_catalog.clock_timestamp() - interval '30 days'
                     ORDER BY t.id DESC LIMIT $2"
                ),
                vec![
                    account.into(),
                    (GATEWAYS_PER_OWNER * TOKENS_PER_DAY).into(),
                ],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        let mut gateways = rows
            .iter()
            .map(|row| {
                let id: Uuid = row.try_get("", "id").map_err(|_| Error::Unavailable)?;
                Ok(MyGateway {
                    id: *id.as_bytes(),
                    origin: row.try_get("", "origin").map_err(|_| Error::Unavailable)?,
                    metadata: metadata(row)?,
                    state: state(row)?,
                    online: row.try_get("", "online").map_err(|_| Error::Unavailable)?,
                    last_seen: row
                        .try_get("", "last_seen")
                        .map_err(|_| Error::Unavailable)?,
                    created_at: row
                        .try_get("", "created_at")
                        .map_err(|_| Error::Unavailable)?,
                    tokens: Vec::new(),
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        for row in &tokens {
            let gateway: Uuid = row
                .try_get("", "gateway_id")
                .map_err(|_| Error::Unavailable)?;
            if let Some(owned) = gateways
                .iter_mut()
                .find(|owned| owned.id == *gateway.as_bytes())
            {
                owned.tokens.push(token_info(row)?);
            }
        }
        Ok(gateways)
    }

    /// Register a gateway at `origin` named `name` for the account behind
    /// `browser`, awaiting an administrator's approval for the directory. It has
    /// no credential pair: it authenticates by tokens alone, approved or not.
    /// The official gateway's origins are the bridge's ([`is_official_origin`]),
    /// so each is a `Conflict`, as is an origin registered already;
    /// `development` allows loopback http origins.
    ///
    /// # Errors
    /// `Rejected` without a live browser session, for an account whose move
    /// is unfinished, or for a malformed origin or name; `Conflict` for a
    /// taken origin; `Busy` past eight gateways; storage failure.
    pub async fn add_gateway(
        &self,
        browser: &Token,
        origin: &str,
        name: &str,
        development: bool,
    ) -> Result<[u8; 16], Error> {
        if is_official_origin(origin) {
            return Err(Error::Conflict);
        }
        validate_origin(origin, development)?;
        let metadata = GatewayMetadata {
            name: name.to_owned(),
            version: "0".into(),
            website: format!("{origin}/"),
        };
        metadata.validate(origin)?;
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let (account, _) = owner(&tx, browser, true).await?;
        let moved = tx
            .query_one_raw(statement(
                "SELECT legacy_closed OR source = 'registered' AS moved
                 FROM central_identity.accounts WHERE id = $1",
                vec![account.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Rejected)?
            .try_get::<bool>("", "moved")
            .map_err(|_| Error::Unavailable)?;
        if !moved {
            return Err(Error::Rejected);
        }
        let row = tx
            .query_one_raw(statement(
                "INSERT INTO central_identity.gateways (owner_id, origin, name, version, website)
                 SELECT $1, $2, $3, $4, $5
                 WHERE (SELECT count(*) FROM central_identity.gateways WHERE owner_id = $1) < $6
                 ON CONFLICT (origin) DO NOTHING RETURNING id",
                vec![
                    account.into(),
                    origin.into(),
                    metadata.name.into(),
                    metadata.version.into(),
                    metadata.website.into(),
                    GATEWAYS_PER_OWNER.into(),
                ],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        let Some(row) = row else {
            let taken = tx
                .query_one_raw(statement(
                    "SELECT 1 AS taken FROM central_identity.gateways WHERE origin = $1",
                    vec![origin.into()],
                ))
                .await
                .map_err(|_| Error::Unavailable)?
                .is_some();
            return Err(if taken { Error::Conflict } else { Error::Busy });
        };
        let id: Uuid = row.try_get("", "id").map_err(|_| Error::Unavailable)?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(*id.as_bytes())
    }

    /// Store the digest of `token` as a token of `gateway`, which the account
    /// behind `browser` must own, named `name`, with `scopes`, for `days`.
    /// The session's password sign-in must be less than 30 minutes old.
    /// Answers the token's id and its expiry in Unix seconds.
    ///
    /// # Errors
    /// `Rejected` without a fresh live browser session, for another
    /// account's or the official gateway, a malformed name, no scope or a
    /// lifetime out of 1 to 366 days; `Busy` past [`TOKENS_PER_GATEWAY`]
    /// live tokens or 32 tokens in a day; storage failure.
    pub async fn create_gateway_token(
        &self,
        browser: &Token,
        gateway: [u8; 16],
        name: &str,
        scopes: GatewayScopes,
        days: u16,
        token: &Token,
    ) -> Result<([u8; 16], i64), Error> {
        let name = token_name(name)?;
        let scopes = scopes.validate()?;
        if days == 0 || days > MAX_TOKEN_DAYS {
            return Err(Error::Rejected);
        }
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let (account, fresh) = owner(&tx, browser, false).await?;
        if !fresh {
            return Err(Error::Rejected);
        }
        let gateway = Uuid::from_bytes(gateway);
        tx.query_one_raw(statement(
            "SELECT id FROM central_identity.gateways WHERE id = $1 AND owner_id = $2 AND NOT official
             FOR UPDATE",
            vec![gateway.into(), account.into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?
        .ok_or(Error::Rejected)?;
        let counts = tx
            .query_one_raw(statement(
                &format!(
                    "SELECT count(*) FILTER (WHERE {LIVE_TOKEN})::bigint AS live,
                         count(*) FILTER (WHERE t.created_at > pg_catalog.clock_timestamp() - interval '1 day')::bigint AS today
                     FROM central_identity.gateway_tokens t WHERE t.gateway_id = $1"
                ),
                vec![gateway.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Unavailable)?;
        let live: i64 = counts.try_get("", "live").map_err(|_| Error::Unavailable)?;
        let today: i64 = counts
            .try_get("", "today")
            .map_err(|_| Error::Unavailable)?;
        if live >= i64::try_from(TOKENS_PER_GATEWAY).unwrap_or(i64::MAX) || today >= TOKENS_PER_DAY
        {
            return Err(Error::Busy);
        }
        // One clock reading for both ends, so a 366-day token meets the
        // table's bound exactly rather than by a few microseconds over.
        let row = tx
            .query_one_raw(statement(
                "WITH now AS (SELECT pg_catalog.clock_timestamp() AS at)
                 INSERT INTO central_identity.gateway_tokens
                     (gateway_id, name, digest, sso, listing, created_at, expires_at)
                 SELECT $1, $2, $3, $4, $5, now.at, now.at + pg_catalog.make_interval(days => $6) FROM now
                 RETURNING id, floor(extract(epoch FROM expires_at))::bigint AS expires_at",
                vec![
                    gateway.into(),
                    name.into(),
                    token.digest().to_vec().into(),
                    scopes.sso.into(),
                    scopes.listing.into(),
                    i32::from(days).into(),
                ],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Unavailable)?;
        let id: Uuid = row.try_get("", "id").map_err(|_| Error::Unavailable)?;
        let expires_at: i64 = row
            .try_get("", "expires_at")
            .map_err(|_| Error::Unavailable)?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok((*id.as_bytes(), expires_at))
    }

    /// Revoke a token of a gateway the account behind `browser` owns. One
    /// revoked already is `Ok` again.
    ///
    /// # Errors
    /// `Rejected` without a live browser session or for a token of a gateway
    /// it does not own; storage failure.
    pub async fn revoke_own_gateway_token(
        &self,
        browser: &Token,
        token: [u8; 16],
    ) -> Result<(), Error> {
        let tx = self.db.begin().await.map_err(|_| Error::Unavailable)?;
        let (account, _) = owner(&tx, browser, false).await?;
        let token = Uuid::from_bytes(token);
        let owned = tx
            .query_one_raw(statement(
                "SELECT t.revoked_at IS NULL AS live FROM central_identity.gateway_tokens t
                 JOIN central_identity.gateways g ON g.id = t.gateway_id
                 WHERE t.id = $1 AND g.owner_id = $2",
                vec![token.into(), account.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Rejected)?;
        if owned
            .try_get::<bool>("", "live")
            .map_err(|_| Error::Unavailable)?
        {
            tx.execute_raw(statement(
                "UPDATE central_identity.gateway_tokens SET revoked_at = pg_catalog.clock_timestamp()
                 WHERE id = $1 AND revoked_at IS NULL",
                vec![token.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        }
        tx.commit().await.map_err(|_| Error::Unavailable)
    }

    /// What Central knows about the gateway whose live token is `token`,
    /// whatever its approval: its id, origin, metadata and state, and the
    /// token's scopes and expiry. Marks the token used.
    ///
    /// # Errors
    /// `Rejected` for anything but a live token of a registered gateway;
    /// storage failure.
    pub async fn gateway_status(
        &self,
        token: &Token,
    ) -> Result<
        (
            [u8; 16],
            String,
            GatewayMetadata,
            GatewayState,
            GatewayScopes,
            i64,
        ),
        Error,
    > {
        let row = self
            .db
            .query_one_raw(statement(
                &format!(
                    "SELECT g.id, g.origin, g.name, g.version, g.website, {STATE} AS state, t.sso, t.listing,
                         floor(extract(epoch FROM t.expires_at))::bigint AS expires_at
                     FROM central_identity.gateway_tokens t JOIN central_identity.gateways g ON g.id = t.gateway_id
                     LEFT JOIN central_identity.gateway_approvals p ON p.gateway_id = g.id
                     WHERE t.digest = $1 AND {LIVE_TOKEN} AND NOT g.official"
                ),
                vec![token.digest().to_vec().into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Rejected)?;
        self.touch_gateway_token(token).await?;
        let id: Uuid = row.try_get("", "id").map_err(|_| Error::Unavailable)?;
        let get = |column| {
            row.try_get::<bool>("", column)
                .map_err(|_| Error::Unavailable)
        };
        Ok((
            *id.as_bytes(),
            row.try_get("", "origin").map_err(|_| Error::Unavailable)?,
            metadata(&row)?,
            state(&row)?,
            GatewayScopes {
                sso: get("sso")?,
                listing: get("listing")?,
            },
            row.try_get("", "expires_at")
                .map_err(|_| Error::Unavailable)?,
        ))
    }

    /// Note that a gateway used the token whose digest `token` has, at most
    /// once a minute; nothing for any other credential.
    ///
    /// # Errors
    /// Storage failure.
    pub async fn touch_gateway_token(&self, token: &Token) -> Result<(), Error> {
        self.db
            .execute_raw(statement(
                "UPDATE central_identity.gateway_tokens SET last_used_at = pg_catalog.clock_timestamp()
                 WHERE digest = $1 AND (last_used_at IS NULL
                     OR last_used_at < pg_catalog.clock_timestamp() - interval '1 minute')",
                vec![token.digest().to_vec().into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        Ok(())
    }
}
