//! An AI account's life (`docs/ai-players.md` §1): an administrator creates
//! it with no password, changes its label, limits and name, and removes it
//! while it has never played. No path makes a person's account an AI.
use super::{audit, get, has_played, unavailable};
use crate::accounts::{AccountStore, Action, Error, Token, deletion, statement};
use central_api::admin::AiPlayerSettings;
use central_api::profile::valid_tag;
use central_api::reports::hex;
use sea_orm::{
    ConnectionTrait, DatabaseTransaction, QueryResult, SqlErr, Statement, prelude::Uuid,
};

/// The credential every AI account holds: fixed bytes that no OPAQUE record
/// parses. A record has the fixed length of a registration upload
/// (`RegistrationUploadLen`), and `ServerKeys::enrollment_record` refuses any
/// other, so no password is ever proven against it. It keeps the account
/// enrolled as every check asks (`credential IS NOT NULL`, the table's
/// CHECKs), so nothing has to tell an AI from a person to stay correct; the
/// sessionless flows answer an AI as an unknown name besides ([`super::NO_AI`]).
pub(crate) const SEALED_CREDENTIAL: &[u8] = b"baylee:ai:sealed";

/// The bytes of a master key wrapper (the recovery one too) and the least
/// bytes of a document, as `accounts`' CHECKs require.
const WRAPPED: usize = 73;
const DOCUMENT: usize = 41;

/// The hex digits after `ai-` in an AI account's login name: 80 bits from
/// the operating system: nobody signs in with it or guesses it.
const LOGIN_BYTES: usize = 10;

/// `len` bytes from the operating system's generator ([`Token::generate`]).
fn random(len: usize) -> Result<Vec<u8>, Error> {
    let mut bytes = Vec::with_capacity(len + 32);
    while bytes.len() < len {
        bytes.extend_from_slice(&*Token::generate()?.to_secret_bytes());
    }
    bytes.truncate(len);
    Ok(bytes)
}

/// Random bytes of `len` that begin with the format's version byte 1, as a
/// stored wrapper and document do, and that nothing can open.
fn sealed(len: usize) -> Result<Vec<u8>, Error> {
    let mut bytes = random(len)?;
    bytes[0] = 1;
    Ok(bytes)
}

/// An unguessable login name, `ai-` and 20 lowercase hex digits.
fn login() -> Result<String, Error> {
    Ok(format!("ai-{}", hex(&random(LOGIN_BYTES)?)))
}

/// The account row of an AI that is to change or go, for no key update: the
/// lock an offer's `FOR SHARE` and an exchange's `FOR UPDATE` wait for, and
/// that a fill-in's match, which only shares the key, does not. So the
/// order is this row, then the AI's (`ai_players`), as an offer takes them;
/// a fill-in that holds the AI's row writes its match and commits first.
async fn lock(tx: &DatabaseTransaction, account: Uuid) -> Result<(String, String), Error> {
    tx.query_one_raw(statement(
        "SELECT id FROM central_identity.accounts WHERE id = $1 FOR NO KEY UPDATE",
        vec![account.into()],
    ))
    .await
    .map_err(unavailable)?
    .ok_or(Error::Rejected)?;
    let row = tx
        .query_one_raw(statement(
            "SELECT kind, profile FROM central_identity.ai_players WHERE account_id = $1
             FOR UPDATE",
            vec![account.into()],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Rejected)?;
    Ok((get(&row, "kind")?, get(&row, "profile")?))
}

/// The audit's detail of a creation or a change: numbers only.
fn detail(settings: &AiPlayerSettings) -> String {
    format!("{{\"max_games\":{}}}", settings.max_games)
}

/// A taken tag is the unique index's to refuse; any other failure is storage's.
fn taken(error: &sea_orm::DbErr) -> Error {
    if matches!(error.sql_err(), Some(SqlErr::UniqueConstraintViolation(_))) {
        Error::Conflict
    } else {
        Error::Unavailable
    }
}

/// The parts of a new AI account that nothing opens: an unguessable login
/// name, the master key's wrappers and a document, all random.
struct Sealed {
    name: String,
    wrapped: Vec<u8>,
    recovery: Vec<u8>,
    document: Vec<u8>,
}

impl Sealed {
    fn new() -> Result<Self, Error> {
        Ok(Self {
            name: login()?,
            wrapped: sealed(WRAPPED)?,
            recovery: sealed(WRAPPED)?,
            document: sealed(DOCUMENT)?,
        })
    }

    /// The account row with `tag`: registered, the sealed credential,
    /// restored, named.
    fn row(&self, settings: &AiPlayerSettings, tag: i32) -> Statement {
        statement(
            "INSERT INTO central_identity.accounts (source, username_key, credential,
                wrapped_master, recovery_wrapped_master, document, document_revision,
                restored, display_name, display_name_at, tag)
             VALUES ('registered', $1, $2, $3, $4, $5, 1, true, $6,
                pg_catalog.clock_timestamp(), $7)
             RETURNING id, tag",
            vec![
                self.name.clone().into(),
                SEALED_CREDENTIAL.to_vec().into(),
                self.wrapped.clone().into(),
                self.recovery.clone().into(),
                self.document.clone().into(),
                settings.display_name.clone().into(),
                tag.into(),
            ],
        )
    }
}

/// The settings' checks, the same for every path that creates or changes an
/// AI: the kind as stored and the daily cap as a column.
fn checked(settings: &AiPlayerSettings) -> Result<(&'static str, Option<i32>), Error> {
    let kind = settings.kind.as_str().ok_or(Error::Rejected)?;
    if !settings.valid() {
        return Err(Error::Rejected);
    }
    let daily = settings
        .daily_games
        .map(i32::try_from)
        .transpose()
        .map_err(|_| Error::Rejected)?;
    Ok((kind, daily))
}

/// A tag as its column holds it, refused out of range.
fn tag_column(tag: u32) -> Result<i32, Error> {
    if !valid_tag(tag) {
        return Err(Error::Rejected);
    }
    i32::try_from(tag).map_err(|_| Error::Rejected)
}

/// After the account row: the AI's row, switched off, and `ai.created`, for
/// an administrator or the operator (`actor` none). Answers id and tag.
async fn created(
    tx: &DatabaseTransaction,
    actor: Option<Uuid>,
    settings: &AiPlayerSettings,
    row: &QueryResult,
) -> Result<([u8; 16], u32), Error> {
    let (kind, daily) = checked(settings)?;
    let account: Uuid = get(row, "id")?;
    let tag = u32::try_from(get::<i32>(row, "tag")?).map_err(unavailable)?;
    tx.execute_raw(statement(
        "INSERT INTO central_identity.ai_players
           (account_id, kind, profile, label, max_games, daily_games, set_by, changed_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $7)",
        vec![
            account.into(),
            kind.into(),
            settings.profile.clone().into(),
            settings.label.clone().into(),
            i16::from(settings.max_games).into(),
            daily.into(),
            actor.into(),
        ],
    ))
    .await
    .map_err(unavailable)?;
    audit(
        tx,
        actor,
        Action::AiCreated,
        Some(account),
        Some(detail(settings)),
    )
    .await?;
    Ok((account.into_bytes(), tag))
}

/// Create an AI player with `tag` in `tx`, for an administrator or the
/// operator (`actor` none): the account row ([`Sealed`]) and [`created`].
/// The one creation both [`AccountStore::create_ai_player`] and the
/// operator's command take.
///
/// # Errors
/// `Rejected` for settings out of bounds and a tag out of range;
/// `Conflict` for a tag another account holds; storage failure.
pub(crate) async fn create(
    tx: &DatabaseTransaction,
    actor: Option<Uuid>,
    settings: &AiPlayerSettings,
    tag: u32,
) -> Result<([u8; 16], u32), Error> {
    checked(settings)?;
    let tag = tag_column(tag)?;
    let row = tx
        .query_one_raw(Sealed::new()?.row(settings, tag))
        .await
        .map_err(|e| taken(&e))?
        .ok_or(Error::Unavailable)?;
    created(tx, actor, settings, &row).await
}

/// Change the AI `account`'s label, limits and display name in `tx`, for
/// an administrator or the operator (`actor` none), audited. Its kind and
/// profile stay as created.
///
/// # Errors
/// `Rejected` for settings out of bounds and an account that is no AI
/// player; `Conflict` for another kind or profile; storage failure.
pub(crate) async fn change(
    tx: &DatabaseTransaction,
    actor: Option<Uuid>,
    account: Uuid,
    settings: &AiPlayerSettings,
) -> Result<(), Error> {
    let (kind, daily) = checked(settings)?;
    let (held_kind, held_profile) = lock(tx, account).await?;
    if held_kind != kind || held_profile != settings.profile {
        return Err(Error::Conflict);
    }
    tx.execute_raw(statement(
        "UPDATE central_identity.ai_players SET label = $2, max_games = $3,
           daily_games = $4, changed_at = pg_catalog.clock_timestamp(), changed_by = $5
         WHERE account_id = $1",
        vec![
            account.into(),
            settings.label.clone().into(),
            i16::from(settings.max_games).into(),
            daily.into(),
            actor.into(),
        ],
    ))
    .await
    .map_err(unavailable)?;
    tx.execute_raw(statement(
        "UPDATE central_identity.accounts SET display_name = $2,
           display_name_at = pg_catalog.clock_timestamp()
         WHERE id = $1 AND display_name IS DISTINCT FROM $2",
        vec![account.into(), settings.display_name.clone().into()],
    ))
    .await
    .map_err(unavailable)?;
    audit(
        tx,
        actor,
        Action::AiChanged,
        Some(account),
        Some(detail(settings)),
    )
    .await
}

impl AccountStore {
    /// Create an AI player for a fresh administrator: a new account with no
    /// password ([`SEALED_CREDENTIAL`], wrappers and a document nothing
    /// opens, an unguessable login name), restored, named and tagged, its
    /// `ai_players` row switched off, and the audit's `ai.created`, all in
    /// one transaction. `tag` none draws one as registration does. Answers
    /// the account's id and tag.
    ///
    /// # Errors
    /// `Rejected` without a fresh administrator, for settings out of bounds
    /// (a name without the AI prefix, the unfair profile among them) and a
    /// tag out of range; `Conflict` for a tag another account holds; `Busy`
    /// when no tag is left to draw; storage failure.
    pub async fn create_ai_player(
        &self,
        browser: &Token,
        settings: &AiPlayerSettings,
        tag: Option<u32>,
    ) -> Result<([u8; 16], u32), Error> {
        checked(settings)?;
        tag.map(tag_column).transpose()?;
        let (tx, actor) = self.acting_administrator(browser).await?;
        // The administrator's tag or none: never another one drawn instead.
        let made = if let Some(tag) = tag {
            create(&tx, Some(actor), settings, tag).await?
        } else {
            let sealed = Sealed::new()?;
            let row = self
                .first_free_tag(&tx, None, None, Error::Unavailable, |tag| {
                    sealed.row(settings, tag)
                })
                .await?
                .ok_or(Error::Unavailable)?;
            created(&tx, Some(actor), settings, &row).await?
        };
        tx.commit().await.map_err(unavailable)?;
        Ok(made)
    }

    /// Change an AI player's label, limits and display name for a fresh
    /// administrator. Its kind and profile stay as created. Audited.
    ///
    /// # Errors
    /// `Rejected` without a fresh administrator, for settings out of bounds
    /// and for an account that is no AI player; `Conflict` for another kind
    /// or profile; storage failure.
    pub async fn set_ai_player(
        &self,
        browser: &Token,
        account: [u8; 16],
        settings: &AiPlayerSettings,
    ) -> Result<(), Error> {
        checked(settings)?;
        let (tx, actor) = self.acting_administrator(browser).await?;
        change(&tx, Some(actor), Uuid::from_bytes(account), settings).await?;
        tx.commit().await.map_err(unavailable)
    }

    /// Remove an AI player that has never been seated and holds no ticket
    /// that is not closed, for a fresh administrator ([`remove`]). Answers
    /// the digests of the sessions that ended, for their push sockets.
    ///
    /// # Errors
    /// `Rejected` without a fresh administrator and for an account that is
    /// no AI player; `Conflict` once it has played; storage failure.
    pub async fn remove_ai_player(
        &self,
        browser: &Token,
        account: [u8; 16],
    ) -> Result<Vec<[u8; 32]>, Error> {
        let (tx, actor) = self.acting_administrator(browser).await?;
        let ended = remove(&tx, Some(actor), Uuid::from_bytes(account)).await?;
        tx.commit().await.map_err(unavailable)?;
        Ok(ended)
    }
}

/// Remove the AI `account` in `tx`, for an administrator or the operator
/// (`actor` none), while it has never been seated and holds no ticket that
/// is not closed: its account goes as a person's deletion takes one
/// (`deletion::remove`), with everything it holds by cascade, its runner
/// sessions and their sids among them, and the audit's `ai.removed`. The
/// one removal the administration's page and the operator's command take.
/// Answers the digests of the sessions that ended.
///
/// # Errors
/// `Rejected` for an account that is no AI player; `Conflict` once it has
/// played; storage failure.
pub(crate) async fn remove(
    tx: &DatabaseTransaction,
    actor: Option<Uuid>,
    account: Uuid,
) -> Result<Vec<[u8; 32]>, Error> {
    lock(tx, account).await?;
    if has_played(tx, account).await? {
        return Err(Error::Conflict);
    }
    // After the locks: an exchange waits for the account row, so no
    // session is minted for it from here to the commit.
    let ended = tx
        .query_all_raw(statement(
            "SELECT digest FROM central_identity.sessions WHERE account_id = $1",
            vec![account.into()],
        ))
        .await
        .map_err(unavailable)?
        .iter()
        .map(|row| {
            get::<Vec<u8>>(row, "digest")?
                .as_slice()
                .try_into()
                .map_err(unavailable)
        })
        .collect::<Result<Vec<[u8; 32]>, Error>>()?;
    deletion::remove(tx, account).await?;
    audit(tx, actor, Action::AiRemoved, Some(account), None).await?;
    Ok(ended)
}

#[cfg(test)]
mod tests {
    use super::*;
    use central_crypto::opaque::server::ServerKeys;

    #[test]
    fn no_opaque_record_parses_the_sealed_credential() {
        assert!(ServerKeys::enrollment_record(SEALED_CREDENTIAL).is_err());
        // The column's own bounds hold it.
        assert!((1..=1024).contains(&SEALED_CREDENTIAL.len()));
    }

    #[test]
    fn the_random_parts_fit_the_account_row() {
        let name = login().unwrap();
        assert_eq!(name.len(), 23);
        assert!(name.starts_with("ai-"));
        assert_eq!(crate::accounts::username(&name).unwrap(), name);
        assert_ne!(name, login().unwrap());
        for len in [WRAPPED, DOCUMENT] {
            let bytes = sealed(len).unwrap();
            assert_eq!((bytes.len(), bytes[0]), (len, 1));
        }
        assert!(
            central_crypto::vault::WrappedMasterKey::from_bytes(&sealed(WRAPPED).unwrap()).is_ok()
        );
        assert_ne!(random(64).unwrap(), random(64).unwrap());
    }
}
