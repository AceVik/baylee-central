//! A native wearer's invitation and a host's subsequent independent seat check.
use super::{
    AccountStore, DatabaseTransaction, Error, Session, Uuid, api, art, col, inventory, statement,
    unavailable, uuid,
};
use crate::RankedHostCredential;
use crate::accounts::{NOT_DISABLED, gateways::valid_id};
use api::table::{HostRequest, HostResponse, Proof, Request, Response, Worn};
use sea_orm::{ConnectionTrait, TransactionTrait};

mod recovery;

/// The operator's retention sweep also clears abandoned receipts when no
/// player has requested another appearance on the service.
pub(in crate::accounts) async fn sweep(db: &DatabaseTransaction) -> Result<(), Error> {
    db.execute_unprepared(
        "DELETE FROM central_identity.advent_table_looks WHERE proof IN
        (SELECT proof FROM central_identity.advent_table_looks WHERE expires_at<=statement_timestamp()
         ORDER BY expires_at LIMIT 256 FOR UPDATE SKIP LOCKED);
        DELETE FROM central_identity.advent_table_outfits WHERE (gateway_id,game,seat) IN
        (SELECT gateway_id,game,seat FROM central_identity.advent_table_outfits WHERE expires_at<=statement_timestamp()
         ORDER BY expires_at LIMIT 256 FOR UPDATE SKIP LOCKED)",
    )
    .await
    .map_err(unavailable)?;
    Ok(())
}

fn chair(game: api::Id, seat: u8) -> Result<(Uuid, i16), Error> {
    if !valid_id(game) || seat >= 8 {
        return Err(Error::Rejected);
    }
    Ok((uuid(game), i16::from(seat)))
}

// Serialize confirmations with replacement, explicit removal and host close.
async fn lock_table(db: &DatabaseTransaction, host: Uuid, game: Uuid) -> Result<(), Error> {
    db.query_one_raw(statement(
        "SELECT pg_advisory_xact_lock(hashtextextended(
        'advent-table/'||$1::uuid::text||'/'||$2::uuid::text,0))",
        vec![host.into(), game.into()],
    ))
    .await
    .map_err(unavailable)?;
    Ok(())
}

pub(super) async fn handle(
    db: &DatabaseTransaction,
    me: Uuid,
    session: Session<'_>,
    request: Request,
) -> Result<Response, Error> {
    begin(db, me, session, request, None).await
}

async fn begin(
    db: &DatabaseTransaction,
    me: Uuid,
    session: Session<'_>,
    request: Request,
    official_stream: Option<&str>,
) -> Result<Response, Error> {
    let (origin, game, seat, look) = match request {
        Request::Begin {
            origin,
            game,
            seat,
            look,
        } => (origin, game, seat, Some(look)),
        Request::Resume { origin, game, seat } => (origin, game, seat, None),
    };
    let (game, seat) = chair(game, seat)?;
    if !matches!(session, Session::Native(_)) || origin.len() > 256 {
        return Err(Error::Rejected);
    }
    let row = db
        .query_one_raw(statement(
            &format!(
                "SELECT g.id,i.sid FROM central_identity.gateway_sids i
        JOIN central_identity.gateways g ON CASE WHEN $3 THEN g.official ELSE g.origin=i.stream END
        WHERE i.session_digest=$1 AND i.stream=$2 AND i.purpose='admit' AND {NOT_DISABLED}"
            ),
            vec![
                session.digest().to_vec().into(),
                official_stream.unwrap_or(&origin).to_owned().into(),
                official_stream.is_some().into(),
            ],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Rejected)?;
    let host: Uuid = col(&row, "id")?;
    let sid: Vec<u8> = col(&row, "sid")?;
    lock_table(db, host, game).await?;
    let look = match look {
        Some(look) if look == api::Equipment::default() => {
            recovery::clear(db, me, host, game, seat).await?;
            return Ok(Response::NoLook);
        }
        Some(look) => look,
        None => match recovery::prior(db, me, host, game, seat).await? {
            Some(look) => look,
            None => return Ok(Response::NoLook),
        },
    };
    inventory::check_look(db, me, look).await?;
    db.execute_raw(statement(
        "DELETE FROM central_identity.advent_table_looks WHERE proof IN (
        SELECT proof FROM central_identity.advent_table_looks WHERE sid=$1 AND expires_at<=statement_timestamp()
        ORDER BY expires_at LIMIT 32 FOR UPDATE SKIP LOCKED)",
        vec![sid.clone().into()],
    ))
    .await
    .map_err(unavailable)?;
    db.execute_raw(statement(
        "DELETE FROM central_identity.advent_table_looks
        WHERE sid=$1 AND game=$2 AND seat=$3 AND NOT confirmed",
        vec![sid.clone().into(), game.into(), seat.into()],
    ))
    .await
    .map_err(unavailable)?;
    let proof = Proof::generate()?;
    let res=db.execute_raw(statement("INSERT INTO central_identity.advent_table_looks
        (proof,account_id,gateway_id,sid,game,seat,sleeve,playmat)
        SELECT $1,$2,$3,$4,$5,$6,$7,$8 WHERE (SELECT count(*) FROM central_identity.advent_table_looks
        WHERE sid=$4 AND expires_at>clock_timestamp())<32",
        vec![proof.digest().to_vec().into(),me.into(),host.into(),sid.into(),game.into(),seat.into(),
             look.sleeve.map(uuid).into(),look.playmat.map(uuid).into()])).await.map_err(unavailable)?;
    if res.rows_affected() != 1 {
        return Err(Error::Rejected);
    }
    Ok(Response::Grant { proof })
}

struct Selection {
    sleeve: Option<Uuid>,
    playmat: Option<Uuid>,
    confirmed: bool,
    valid_for: u16,
}

async fn selection(
    db: &DatabaseTransaction,
    host: Uuid,
    game: Uuid,
    seat: i16,
    proof: &Proof,
) -> Result<Selection, Error> {
    let row=db.query_one_raw(statement("SELECT l.sleeve,l.playmat,l.confirmed,
        floor(extract(epoch FROM l.expires_at-statement_timestamp()))::bigint AS seconds
        FROM central_identity.advent_table_looks l JOIN central_identity.gateway_sids i ON i.sid=l.sid
        JOIN central_identity.sessions s ON s.digest=i.session_digest AND s.account_id=l.account_id
        JOIN central_identity.accounts a ON a.id=s.account_id AND a.credential_revision=s.credential_revision
        WHERE l.proof=$1 AND l.gateway_id=$2 AND l.game=$3 AND l.seat=$4
        AND i.purpose='admit' AND s.kind='native' AND s.expires_at>statement_timestamp()
        AND l.expires_at>statement_timestamp()",vec![proof.digest().to_vec().into(),host.into(),game.into(),seat.into()]))
        .await.map_err(unavailable)?.ok_or(Error::Rejected)?;
    Ok(Selection {
        sleeve: col(&row, "sleeve")?,
        playmat: col(&row, "playmat")?,
        confirmed: col(&row, "confirmed")?,
        valid_for: u16::try_from(col::<i64>(&row, "seconds")?).map_err(unavailable)?,
    })
}

async fn metadata(
    db: &DatabaseTransaction,
    id: Option<Uuid>,
) -> Result<Option<api::AssetMeta>, Error> {
    match id {
        Some(id) => Ok(Some(art::metadata(db, id).await?)),
        None => Ok(None),
    }
}

async fn confirm(
    db: &DatabaseTransaction,
    host: Uuid,
    game: Uuid,
    seat: i16,
    proof: &Proof,
    sid: &[u8],
) -> Result<HostResponse, Error> {
    lock_table(db, host, game).await?;
    let fresh = !selection(db, host, game, seat, proof).await?.confirmed;
    let verified=db.execute_raw(statement("UPDATE central_identity.advent_table_looks SET confirmed=true,
        expires_at=CASE WHEN confirmed THEN expires_at ELSE clock_timestamp()+interval '10 minutes' END
        WHERE proof=$1 AND sid=$2 AND gateway_id=$3 AND game=$4 AND seat=$5 AND expires_at>clock_timestamp()",
        vec![proof.digest().to_vec().into(),sid.to_vec().into(),host.into(),game.into(),seat.into()])).await.map_err(unavailable)?;
    if verified.rows_affected() != 1 {
        return Err(Error::Rejected);
    }
    let selected = selection(db, host, game, seat, proof).await?;
    if fresh {
        recovery::remember(db, host, game, seat, proof).await?;
    }
    db.execute_raw(statement(
        "DELETE FROM central_identity.advent_table_looks
        WHERE gateway_id=$1 AND game=$2 AND seat=$3 AND proof<>$4 AND confirmed",
        vec![
            host.into(),
            game.into(),
            seat.into(),
            proof.digest().to_vec().into(),
        ],
    ))
    .await
    .map_err(unavailable)?;
    Ok(HostResponse::Confirmed {
        worn: Worn {
            event_sleeve: metadata(db, selected.sleeve).await?,
            event_playmat: metadata(db, selected.playmat).await?,
        },
        valid_for: selected.valid_for,
    })
}

impl AccountStore {
    /// Begin a casual presentation at the official gateway. Its approved
    /// aliases share the canonical revocation stream and one registry row.
    ///
    /// # Errors
    /// Rejects another origin or an invalid native session, seat or selection.
    pub async fn advent_official_table(
        &self,
        session: Session<'_>,
        request: Request,
        origins: &[String],
    ) -> Result<api::Response, Error> {
        let canonical = origins
            .first()
            .filter(|_| origins.iter().any(|origin| origin == request.origin()))
            .ok_or(Error::Rejected)?;
        let db = self.db.begin().await.map_err(unavailable)?;
        let me = super::caller(&db, session, true).await?;
        let response = begin(&db, me, session, request, Some(canonical)).await?;
        db.commit().await.map_err(unavailable)?;
        Ok(api::Response::Table { response })
    }

    /// Serve a registered casual host's own table after verifying its SSO credential.
    ///
    /// # Errors
    /// Rejects invalid hosts, proofs, seats, sessions, unselected slots and ranges.
    pub async fn advent_table_host(
        &self,
        credential: RankedHostCredential<'_>,
        request: HostRequest,
    ) -> Result<HostResponse, Error> {
        let db = self.db.begin().await.map_err(unavailable)?;
        let host = match credential {
            RankedHostCredential::Registered(token) => {
                uuid(Self::authenticate_gateway_on(&db, token).await?.0)
            }
            RankedHostCredential::Official(origin) => {
                let id = match &request {
                    HostRequest::Closed { game }
                    | HostRequest::Confirm { game, .. }
                    | HostRequest::Art { game, .. } => uuid(*game),
                };
                let row = db
                    .query_one_raw(statement(
                        &format!(
                            "SELECT g.id FROM central_identity.gateways g WHERE g.official AND {NOT_DISABLED}
                    AND (EXISTS (SELECT 1 FROM central_identity.advent_table_looks l
                        JOIN central_identity.gateway_sids i ON i.sid=l.sid
                        WHERE l.gateway_id=g.id AND i.stream=$1 AND l.game=$2)
                    OR EXISTS (SELECT 1 FROM central_identity.advent_table_outfits o
                        WHERE o.gateway_id=g.id AND o.stream=$1 AND o.game=$2))"
                        ),
                        vec![origin.to_owned().into(),id.into()],
                    ))
                    .await
                    .map_err(unavailable)?;
                let Some(row) = row else {
                    return if matches!(request, HostRequest::Closed { .. }) {
                        Ok(HostResponse::Closed)
                    } else {
                        Err(Error::Rejected)
                    };
                };
                col(&row, "id")?
            }
        };
        let response = match request {
            HostRequest::Closed { game } => {
                let (game, _) = chair(game, 0)?;
                lock_table(&db, host, game).await?;
                recovery::close(&db, host, game).await?;
                HostResponse::Closed
            }
            HostRequest::Confirm {
                game,
                seat,
                proof,
                admission,
            } => {
                let (game, seat) = chair(game, seat)?;
                confirm(&db, host, game, seat, &proof, admission.as_bytes()).await?
            }
            HostRequest::Art {
                game,
                seat,
                proof,
                kind,
                offset,
                cached,
            } => {
                let (game, seat) = chair(game, seat)?;
                let selected = selection(&db, host, game, seat, &proof).await?;
                if !selected.confirmed {
                    return Err(Error::Rejected);
                }
                let id = match kind {
                    api::Kind::Sleeve => selected.sleeve,
                    api::Kind::Playmat => selected.playmat,
                }
                .ok_or(Error::Rejected)?;
                match art::page(&db, id, offset, cached).await? {
                    api::Response::Asset { page } => HostResponse::Asset { page },
                    api::Response::Cached { asset } => HostResponse::Cached { asset },
                    _ => return Err(Error::Unavailable),
                }
            }
        };
        db.commit().await.map_err(unavailable)?;
        Ok(response)
    }
}
