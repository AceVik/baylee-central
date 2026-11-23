//! A native wearer's invitation and a host's subsequent independent seat check.
use super::{
    AccountStore, DatabaseTransaction, Error, Session, Uuid, api, art, col, inventory, statement,
    unavailable, uuid,
};
use crate::RankedHostCredential;
use crate::accounts::{NOT_DISABLED, gateways::valid_id};
use api::table::{HostRequest, HostResponse, Proof, Request, Response, Worn};
use sea_orm::{ConnectionTrait, TransactionTrait};

fn chair(game: api::Id, seat: u8) -> Result<(Uuid, i16), Error> {
    if !valid_id(game) || seat >= 8 {
        return Err(Error::Rejected);
    }
    Ok((uuid(game), i16::from(seat)))
}

// Begin and Confirm share a per-chair transaction lock so a slower, older
// confirmation cannot replace a newer selection. Only the digest is stored.
async fn lock_chair(
    db: &DatabaseTransaction,
    sid: &[u8],
    game: Uuid,
    seat: i16,
) -> Result<(), Error> {
    db.query_one_raw(statement(
        "SELECT pg_advisory_xact_lock(hashtextextended(
        'advent-table/'||encode($1::bytea,'hex')||'/'||$2::uuid::text||'/'||$3::smallint::text,0))",
        vec![sid.to_vec().into(), game.into(), seat.into()],
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
    let Request::Begin {
        origin,
        game,
        seat,
        look,
    } = request;
    let (game, seat) = chair(game, seat)?;
    if !matches!(session, Session::Native(_))
        || origin.len() > 256
        || look == api::Equipment::default()
    {
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
    inventory::check_look(db, me, look).await?;
    lock_chair(db, &sid, game, seat).await?;
    db.execute_raw(statement(
        "DELETE FROM central_identity.advent_table_looks WHERE proof IN (
        SELECT proof FROM central_identity.advent_table_looks WHERE expires_at<=clock_timestamp()
        ORDER BY expires_at LIMIT 64 FOR UPDATE SKIP LOCKED)",
        vec![],
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
    lock_chair(db, sid, game, seat).await?;
    let verified=db.execute_raw(statement("UPDATE central_identity.advent_table_looks SET confirmed=true,
        expires_at=CASE WHEN confirmed THEN expires_at ELSE clock_timestamp()+interval '10 minutes' END
        WHERE proof=$1 AND sid=$2 AND gateway_id=$3 AND game=$4 AND seat=$5 AND expires_at>clock_timestamp()",
        vec![proof.digest().to_vec().into(),sid.to_vec().into(),host.into(),game.into(),seat.into()])).await.map_err(unavailable)?;
    if verified.rows_affected() != 1 {
        return Err(Error::Rejected);
    }
    let selected = selection(db, host, game, seat, proof).await?;
    db.execute_raw(statement(
        "DELETE FROM central_identity.advent_table_looks
        WHERE sid=$1 AND game=$2 AND seat=$3 AND proof<>$4 AND confirmed",
        vec![
            sid.to_vec().into(),
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
        let Request::Begin { origin, .. } = &request;
        let canonical = origins
            .first()
            .filter(|_| origins.contains(origin))
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
                let proof = match &request {
                    HostRequest::Confirm { proof, .. } | HostRequest::Art { proof, .. } => proof,
                };
                let row = db
                    .query_one_raw(statement(
                        &format!(
                            "SELECT g.id FROM central_identity.gateways g JOIN central_identity.advent_table_looks l ON l.gateway_id=g.id
                    JOIN central_identity.gateway_sids i ON i.sid=l.sid
                    WHERE i.stream=$1 AND l.proof=$2 AND g.official AND {NOT_DISABLED}"
                        ),
                        vec![origin.to_owned().into(),proof.digest().to_vec().into()],
                    ))
                    .await
                    .map_err(unavailable)?
                    .ok_or(Error::Rejected)?;
                col(&row, "id")?
            }
        };
        let response = match request {
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
