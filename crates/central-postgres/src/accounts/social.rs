//! Central social state. Session ownership is checked at the database boundary.
use super::{AccountStore, Error, Session, live::Concerns, statement};
use central_api::social::{Contact, Person, Relation, SocialRequest, SocialResponse};
use sea_orm::{ConnectionTrait, QueryResult, TransactionTrait, prelude::Uuid};

mod cards;
mod chat;
pub use chat::CHAT_SENDER_SHARE;
mod contacts;
pub(super) mod groups;
pub(super) mod invites;
mod limits;
pub(super) use limits::charge_lookups;

pub(super) fn unavailable(_: impl Sized) -> Error {
    Error::Unavailable
}

/// Rechecked after account locks on every mutation, so revocation cannot race a write.
pub(super) async fn caller(
    db: &impl ConnectionTrait,
    session: Session<'_>,
    lock: bool,
) -> Result<Uuid, Error> {
    db.query_one_raw(statement(
        &format!(
            "SELECT a.id FROM central_identity.accounts a JOIN central_identity.sessions s
         ON s.account_id = a.id AND s.credential_revision = a.credential_revision
         WHERE s.digest = $1 AND s.kind = $2 AND s.expires_at > pg_catalog.clock_timestamp()
         AND a.credential IS NOT NULL AND a.display_name IS NOT NULL AND a.tag IS NOT NULL{}",
            if lock { " FOR SHARE OF s" } else { "" }
        ),
        vec![session.digest().to_vec().into(), session.kind().into()],
    ))
    .await
    .map_err(unavailable)?
    .ok_or(Error::Rejected)?
    .try_get("", "id")
    .map_err(unavailable)
}

fn person(row: &QueryResult) -> Result<Person, Error> {
    let tag: i32 = row.try_get("", "tag").map_err(unavailable)?;
    Ok(Person {
        tag: u32::try_from(tag).map_err(unavailable)?,
        display_name: row.try_get("", "display_name").map_err(unavailable)?,
    })
}

async fn by_tag(db: &impl ConnectionTrait, tag: u32) -> Result<Option<QueryResult>, Error> {
    let tag = i32::try_from(tag).map_err(|_| Error::Rejected)?;
    db.query_one_raw(statement(
        "SELECT id, tag, display_name FROM central_identity.accounts
         WHERE tag = $1 AND credential IS NOT NULL AND display_name IS NOT NULL",
        vec![tag.into()],
    ))
    .await
    .map_err(unavailable)
}

/// All social mutations lock affected accounts in ascending UUID order. This also
/// serializes quotas and account deletion without opposing-contact deadlocks.
pub(super) async fn lock_accounts(db: &impl ConnectionTrait, ids: Vec<Uuid>) -> Result<(), Error> {
    db.query_all_raw(statement(
        "SELECT id FROM central_identity.accounts WHERE id = ANY($1) ORDER BY id FOR UPDATE",
        vec![ids.into()],
    ))
    .await
    .map_err(unavailable)?;
    Ok(())
}

impl AccountStore {
    /// Run an account-owned social operation on either authenticated surface.
    ///
    /// # Errors
    /// Rejects invalid sessions, self-actions and unauthorized transitions; bounds
    /// return `Busy`; database failure never substitutes empty contacts.
    pub async fn social(
        &self,
        session: Session<'_>,
        request: SocialRequest,
    ) -> Result<SocialResponse, Error> {
        self.social_concerning(session, request)
            .await
            .map(|(response, _)| response)
    }

    /// [`Self::social`], and whose view the operation changed, once it has
    /// committed ([`Concerns`]): the accounts a push socket tells. Read in
    /// the transaction that changed it, from the rows it locked, so it costs
    /// no statement of its own, and names nobody for a read or a change that
    /// changed nothing.
    ///
    /// # Errors
    /// As [`Self::social`].
    pub async fn social_concerning(
        &self,
        session: Session<'_>,
        request: SocialRequest,
    ) -> Result<(SocialResponse, Concerns), Error> {
        let tx = self.db.begin().await.map_err(unavailable)?;
        let me = caller(&tx, session, false).await?;
        let (response, concerns) = match request {
            SocialRequest::Group { request } => {
                let (response, concerns) = groups::dispatch(&tx, session, me, request).await?;
                (SocialResponse::Group { response }, concerns)
            }
            SocialRequest::Chat { request } => {
                let (response, concerns) = chat::dispatch(&tx, session, me, request).await?;
                (SocialResponse::Chat { response }, concerns)
            }
            SocialRequest::Contacts => (
                SocialResponse::Contacts {
                    contacts: snapshot(&tx, me).await?,
                },
                Concerns::default(),
            ),
            SocialRequest::Lookup { tag } => {
                lock_accounts(&tx, vec![me]).await?;
                caller(&tx, session, true).await?;
                limits::charge(&tx, me, limits::Budget::Lookup).await?;
                let found = by_tag(&tx, tag).await?;
                (
                    SocialResponse::Person {
                        person: found.as_ref().map(person).transpose()?,
                    },
                    Concerns::default(),
                )
            }
            SocialRequest::Search { query } => {
                let people = search(&tx, session, me, &query).await?;
                (SocialResponse::People { people }, Concerns::default())
            }
            SocialRequest::Pictures { tags } => {
                lock_accounts(&tx, vec![me]).await?;
                caller(&tx, session, true).await?;
                (
                    SocialResponse::Pictures {
                        pictures: super::pictures::pictures(&tx, me, &tags).await?,
                    },
                    Concerns::default(),
                )
            }
            SocialRequest::Ranks { tags } => {
                lock_accounts(&tx, vec![me]).await?;
                caller(&tx, session, true).await?;
                (
                    SocialResponse::Ranks {
                        ranks: ranks(&tx, me, &tags).await?,
                    },
                    Concerns::default(),
                )
            }
            SocialRequest::ReportPicture {
                tag,
                key,
                reason,
                note,
            } => {
                let other = by_tag(&tx, tag).await?.ok_or(Error::Rejected)?;
                let other: Uuid = other.try_get("", "id").map_err(unavailable)?;
                lock_accounts(&tx, vec![me]).await?;
                caller(&tx, session, true).await?;
                super::pictures::report(&tx, me, other, key, reason, note.as_deref()).await?;
                (SocialResponse::Done, Concerns::default())
            }
            SocialRequest::Contact { tag, change } => {
                let concerns = contact_change(&tx, session, me, tag, change).await?;
                (SocialResponse::Done, concerns)
            }
            SocialRequest::CardSuggest { .. } | SocialRequest::CardPrintings { .. } => {
                (social_cards(&tx, request).await?, Concerns::default())
            }
            SocialRequest::FriendInvites
            | SocialRequest::CreateFriendInvite { .. }
            | SocialRequest::RevokeFriendInvite { .. }
            | SocialRequest::RedeemFriendInvite { .. } => {
                invites::dispatch_tx(&tx, session, me, request).await?
            }
        };
        tx.commit().await.map_err(unavailable)?;
        Ok((response, concerns))
    }
}

async fn social_cards(
    tx: &impl ConnectionTrait,
    request: SocialRequest,
) -> Result<SocialResponse, Error> {
    match request {
        SocialRequest::CardSuggest {
            query,
            lang,
            set_filter,
            collector_filter,
            finish_filter,
        } => {
            let items = cards::suggest(
                tx,
                &query,
                &lang,
                set_filter.as_deref(),
                collector_filter.as_deref(),
                finish_filter.as_deref(),
            )
            .await?;
            Ok(SocialResponse::CardSuggestions { items })
        }
        SocialRequest::CardPrintings {
            name,
            lang,
            set_filter,
            collector_filter,
            finish_filter,
        } => {
            let (card, printings) = cards::printings(
                tx,
                &name,
                &lang,
                set_filter.as_deref(),
                collector_filter.as_deref(),
                finish_filter.as_deref(),
            )
            .await?;
            Ok(SocialResponse::CardPrintings { card, printings })
        }
        _ => unreachable!(),
    }
}

async fn contact_change(
    tx: &sea_orm::DatabaseTransaction,
    session: Session<'_>,
    me: Uuid,
    tag: u32,
    change: central_api::social::ContactAction,
) -> Result<Concerns, Error> {
    let other = by_tag(tx, tag).await?.ok_or(Error::Rejected)?;
    let other: Uuid = other.try_get("", "id").map_err(unavailable)?;
    if me == other {
        return Err(Error::Rejected);
    }
    lock_accounts(tx, vec![me, other]).await?;
    caller(tx, session, true).await?;
    let changed = contacts::change(tx, me, other, change).await?;
    Ok(Concerns {
        social: [(changed.me, me), (changed.other, other)]
            .into_iter()
            .filter(|(changed, _)| *changed)
            .map(|(_, account)| account.into_bytes())
            .collect(),
        // Only a friendship that began or ended changes what
        // presence shows; the lists are then read again, not
        // inferred from the action.
        pair: changed
            .friendship
            .then_some((me.into_bytes(), other.into_bytes())),
        ..Concerns::default()
    })
}

struct SearchQuery {
    exact_tag: Option<i32>,
    escaped_name: Option<String>,
}

fn parse_tag_hex(text: &str, canonical_only: bool) -> Option<i32> {
    if text.is_empty() || text.len() > 8 {
        return None;
    }
    if canonical_only {
        if !(4..=8).contains(&text.len())
            || !text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
        {
            return None;
        }
    } else if !text.as_bytes().iter().all(u8::is_ascii_hexdigit) {
        return None;
    }
    let parsed = u32::from_str_radix(text, 16).ok()?;
    if !central_api::profile::valid_tag(parsed) {
        return None;
    }
    if canonical_only && central_api::profile::tag_text(parsed) != text {
        return None;
    }
    i32::try_from(parsed).ok()
}

fn escape_like(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if matches!(c, '\\' | '%' | '_') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

fn parse_search_query(raw: &str) -> Result<SearchQuery, Error> {
    let trimmed = raw.trim();
    let chars = trimmed.chars().count();
    if trimmed.is_empty()
        || chars > central_api::social::MAX_SEARCH_QUERY
        || raw.chars().any(char::is_control)
    {
        return Err(Error::Rejected);
    }
    let (exact_tag, escaped_name) = if let Some((before, after)) = trimmed.rsplit_once('#') {
        let tag = parse_tag_hex(after, false);
        let name = before.trim();
        let escaped = (name.chars().count() >= 2).then(|| escape_like(name));
        (tag, escaped)
    } else {
        let tag = parse_tag_hex(trimmed, true);
        let escaped = (chars >= 2).then(|| escape_like(trimmed));
        (tag, escaped)
    };
    if exact_tag.is_none() && escaped_name.is_none() {
        return Err(Error::Rejected);
    }
    Ok(SearchQuery {
        exact_tag,
        escaped_name,
    })
}

async fn search(
    tx: &sea_orm::DatabaseTransaction,
    session: Session<'_>,
    me: Uuid,
    raw_query: &str,
) -> Result<Vec<Person>, Error> {
    let parsed = parse_search_query(raw_query)?;
    lock_accounts(tx, vec![me]).await?;
    caller(tx, session, true).await?;
    limits::charge(tx, me, limits::Budget::Lookup).await?;
    let prefix = parsed
        .escaped_name
        .as_deref()
        .map(|escaped| format!("{escaped}%"));
    let contains = parsed
        .escaped_name
        .as_deref()
        .map(|escaped| format!("%{escaped}%"));
    let limit = i64::try_from(central_api::social::MAX_SEARCH_RESULTS).map_err(unavailable)?;
    let rows = tx
        .query_all_raw(statement(
            "SELECT a.tag, a.display_name
             FROM central_identity.accounts a
             WHERE a.credential IS NOT NULL
               AND a.display_name IS NOT NULL
               AND a.tag IS NOT NULL
               AND a.id <> $1
               AND NOT EXISTS (
                   SELECT 1 FROM central_identity.social_blocks b
                   WHERE (b.owner = $1 AND b.blocked = a.id)
                      OR (b.owner = a.id AND b.blocked = $1)
               )
               AND (
                   ($2::integer IS NOT NULL AND a.tag = $2)
                   OR ($3::text IS NOT NULL AND a.display_name ILIKE $4::text ESCAPE '\\')
               )
             ORDER BY
                 CASE
                     WHEN $2::integer IS NOT NULL AND a.tag = $2 THEN 0
                     WHEN $3::text IS NOT NULL AND a.display_name ILIKE $3 ESCAPE '\\' THEN 1
                     ELSE 2
                 END,
                 lower(a.display_name),
                 a.tag
             LIMIT $5",
            vec![
                me.into(),
                parsed.exact_tag.into(),
                prefix.into(),
                contains.into(),
                limit.into(),
            ],
        ))
        .await
        .map_err(unavailable)?;
    rows.iter().map(person).collect()
}

async fn snapshot(db: &impl ConnectionTrait, me: Uuid) -> Result<Vec<Contact>, Error> {
    let rows = db.query_all_raw(statement(
        "SELECT a.tag, a.display_name, r.relation FROM (
            SELECT CASE WHEN low = $1 THEN high ELSE low END AS other,
                   CASE WHEN accepted THEN 'friend' WHEN requested_by = $1 THEN 'outgoing' ELSE 'incoming' END AS relation
            FROM central_identity.social_contacts WHERE low = $1 OR high = $1
            UNION ALL SELECT blocked AS other, 'blocked' FROM central_identity.social_blocks WHERE owner = $1
         ) r JOIN central_identity.accounts a ON a.id = r.other
         WHERE a.credential IS NOT NULL AND a.tag IS NOT NULL AND a.display_name IS NOT NULL
         ORDER BY a.tag LIMIT 1651", vec![me.into()],
    )).await.map_err(unavailable)?;
    if rows.len() > 1650 {
        return Err(Error::Unavailable);
    }
    rows.iter()
        .map(|row| {
            let relation = match row
                .try_get::<String>("", "relation")
                .map_err(unavailable)?
                .as_str()
            {
                "friend" => Relation::Friend,
                "incoming" => Relation::Incoming,
                "outgoing" => Relation::Outgoing,
                "blocked" => Relation::Blocked,
                _ => return Err(Error::Unavailable),
            };
            Ok(Contact {
                person: person(row)?,
                relation,
            })
        })
        .collect()
}

async fn ranks(
    tx: &sea_orm::DatabaseTransaction,
    me: Uuid,
    tags: &[u32],
) -> Result<Vec<central_api::social::TaggedRank>, Error> {
    if tags.len() > central_api::picture::MAX_PICTURE_TAGS {
        return Err(Error::Rejected);
    }
    let mut asked: Vec<i32> = tags
        .iter()
        .map(|&tag| i32::try_from(tag).map_err(|_| Error::Rejected))
        .collect::<Result<_, _>>()?;
    asked.sort_unstable();
    asked.dedup();
    if asked.is_empty() {
        return Ok(Vec::new());
    }
    let rows = tx
        .query_all_raw(statement(
            "SELECT a.id, a.tag FROM central_identity.accounts a
             WHERE a.tag = ANY($2) AND a.credential IS NOT NULL AND a.display_name IS NOT NULL
             AND NOT EXISTS (SELECT 1 FROM central_identity.social_blocks b
                 WHERE (b.owner = $1 AND b.blocked = a.id) OR (b.owner = a.id AND b.blocked = $1))",
            vec![me.into(), asked.into()],
        ))
        .await
        .map_err(unavailable)?;
    let accounts: Vec<(Uuid, u32)> = rows
        .iter()
        .map(|r| {
            let id: Uuid = r.try_get("", "id").map_err(unavailable)?;
            let tag: i32 = r.try_get("", "tag").map_err(unavailable)?;
            Ok((id, u32::try_from(tag).map_err(unavailable)?))
        })
        .collect::<Result<_, Error>>()?;
    super::ranked::boards::peer_best_ranks(tx, &accounts).await
}

#[cfg(test)]
mod tests {
    use super::{escape_like, parse_search_query};

    #[test]
    fn search_queries_parse_tags_and_escape_like_wildcards() {
        for bad in ["", "   ", "a", "#", "a#", "ab\n", &"x".repeat(33)] {
            assert!(parse_search_query(bad).is_err(), "{bad:?}");
        }
        let by_hash = parse_search_query("#1a2b").unwrap();
        assert_eq!(by_hash.exact_tag, Some(0x1a2b));
        assert_eq!(by_hash.escaped_name, None);

        let by_handle = parse_search_query("Public_Alice#0007").unwrap();
        assert_eq!(by_handle.exact_tag, Some(7));
        assert_eq!(by_handle.escaped_name.as_deref(), Some(r"Public\_Alice"));

        let by_hex = parse_search_query("0007").unwrap();
        assert_eq!(by_hex.exact_tag, Some(7));
        assert_eq!(by_hex.escaped_name.as_deref(), Some("0007"));

        let non_canonical = parse_search_query("01a2b").unwrap();
        assert_eq!(non_canonical.exact_tag, None);
        assert_eq!(non_canonical.escaped_name.as_deref(), Some("01a2b"));

        assert_eq!(escape_like(r"a%b_c\d"), r"a\%b\_c\\d");
    }
}
