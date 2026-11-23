//! Consent is private to the account authority. A verified operator sees only
//! two disposable identifiers for its own match, never a Central account record.
use super::{Error, api, column, replication, statement, unavailable, uuid};
use api::replica as rpc;
use central_api::terms::TermsVersion;
use sea_orm::{ConnectionTrait, DatabaseTransaction, QueryResult, prelude::Uuid};

struct Recipient {
    host: rpc::HostApproval,
    role: rpc::Role,
    epoch: i64,
    verified: bool,
    approved: [bool; 2],
    declined: [bool; 2],
    terms_ok: [bool; 2],
}

impl Recipient {
    fn ready(&self) -> bool {
        self.verified && self.approved == [true; 2] && self.terms_ok == [true; 2]
    }
    fn offer(&self, seat: usize) -> rpc::HostConsentOffer {
        rpc::HostConsentOffer {
            host: self.host.clone(),
            role: self.role,
            approved: self.approved[seat] && self.terms_ok[seat],
            needs_gateway_terms: !self.terms_ok[seat],
        }
    }
}

struct Review {
    fence: rpc::Fence,
    state: String,
    replay: bool,
    accounts: [Uuid; 2],
    recipients: Vec<Recipient>,
}
impl Review {
    fn seat(&self, me: Uuid) -> Result<usize, Error> {
        self.accounts
            .iter()
            .position(|a| *a == me)
            .ok_or(Error::Rejected)
    }
    fn active(&self) -> bool {
        matches!(self.state.as_str(), "reserved" | "playing")
    }
    fn approved_operator(&self, host: Uuid) -> bool {
        self.recipients
            .iter()
            .any(|r| r.host.host_id == host.into_bytes())
            && self.recipients.iter().all(Recipient::ready)
    }
    fn response(&self, seat: usize) -> rpc::SeatResponse {
        rpc::SeatResponse::Hosts {
            fence: self.fence,
            hosts: self.recipients.iter().map(|r| r.offer(seat)).collect(),
        }
    }
    fn recipient(&self, host: &rpc::HostApproval) -> Result<&Recipient, Error> {
        self.recipients
            .iter()
            .find(|r| r.verified && r.host == *host)
            .ok_or(Error::Conflict)
    }
}

fn recipient(a: &QueryResult, b: &QueryResult) -> Result<Recipient, Error> {
    let version: Option<String> = column(a, "version")?;
    let terms = version
        .map(|version| -> Result<TermsVersion, Error> {
            Ok(TermsVersion {
                version,
                digest: column::<Vec<u8>>(a, "digest")?
                    .try_into()
                    .map_err(unavailable)?,
            })
        })
        .transpose()?;
    let host: Uuid = column(a, "host")?;
    if host != column::<Uuid>(b, "host")?
        || column::<i16>(a, "seat")? != 0
        || column::<i16>(b, "seat")? != 1
    {
        return Err(Error::Unavailable);
    }
    Ok(Recipient {
        host: rpc::HostApproval {
            epoch: u64::try_from(column::<i64>(a, "epoch")?).map_err(unavailable)?,
            host_id: host.into_bytes(),
            origin: column::<Option<String>>(a, "origin")?.ok_or(Error::Conflict)?,
            terms,
        },
        role: if column(a, "primary_role")? {
            rpc::Role::Primary
        } else {
            rpc::Role::Standby
        },
        epoch: column(a, "epoch")?,
        verified: column(a, "verified")?,
        approved: [column(a, "approved")?, column(b, "approved")?],
        declined: [column(a, "declined")?, column(b, "declined")?],
        terms_ok: [column(a, "terms_ok")?, column(b, "terms_ok")?],
    })
}

async fn review(db: &DatabaseTransaction, id: Uuid) -> Result<Review, Error> {
    let rows = db
        .query_all_raw(statement(
            include_str!("privacy/operators.sql"),
            vec![id.into()],
        ))
        .await
        .map_err(unavailable)?;
    let first = rows.first().ok_or(Error::Rejected)?;
    if !matches!(rows.len(), 2 | 4) {
        return Err(Error::Unavailable);
    }
    Ok(Review {
        fence: rpc::Fence {
            match_id: id.into_bytes(),
            generation: u64::try_from(column::<i64>(first, "generation")?).map_err(unavailable)?,
        },
        accounts: [column(first, "account_a")?, column(first, "account_b")?],
        state: column(first, "state")?,
        replay: column(first, "replay")?,
        recipients: rows
            .as_chunks::<2>()
            .0
            .iter()
            .map(|p| recipient(&p[0], &p[1]))
            .collect::<Result<_, _>>()?,
    })
}

pub(in crate::accounts) async fn consented(
    db: &DatabaseTransaction,
    id: Uuid,
    host: Uuid,
) -> Result<bool, Error> {
    let selection = review(db, id).await?;
    Ok(selection.active() && selection.approved_operator(host))
}

pub(super) async fn consented_for_replay(
    db: &DatabaseTransaction,
    id: Uuid,
    host: Uuid,
) -> Result<bool, Error> {
    let selection = review(db, id).await?;
    Ok(selection.state == "finished" && selection.replay && selection.approved_operator(host))
}

/// The authority checks role/epoch before this call. Recheck disclosure at the
/// last private projection as well; another operator can never request the pair.
pub(in crate::accounts) async fn aliases(
    db: &DatabaseTransaction,
    id: Uuid,
    host: Uuid,
) -> Result<[api::Id; 2], Error> {
    if !consented(db, id, host).await? {
        return Err(Error::Rejected);
    }
    let row = db
        .query_one_raw(statement(
            "SELECT alias_a,alias_b FROM central_identity.ranked_host_aliases
        WHERE match_id=$1 AND host_id=$2",
            vec![id.into(), host.into()],
        ))
        .await
        .map_err(unavailable)?
        .ok_or(Error::Unavailable)?;
    Ok([
        column::<Uuid>(&row, "alias_a")?.into_bytes(),
        column::<Uuid>(&row, "alias_b")?.into_bytes(),
    ])
}

/// An alias is public to its operator. Only a live, actually issued ranked
/// credential for this participant can activate its seat. The stored SID is
/// revoked on rotation/logout and cannot outlive the signed token's deadline.
pub(super) async fn presented(
    db: &DatabaseTransaction,
    id: Uuid,
    host: Uuid,
    account: Uuid,
    proof: &api::SeatProof,
) -> Result<(), Error> {
    db.query_one_raw(statement(
        "SELECT 1 FROM central_identity.gateway_sids k
         JOIN central_identity.gateways g ON g.origin=k.stream
         JOIN central_identity.sessions s ON s.digest=k.session_digest
         JOIN central_identity.accounts a ON a.id=s.account_id AND a.credential_revision=s.credential_revision
         LEFT JOIN central_identity.ranked_replicas r ON r.match_id=k.ranked_match
         WHERE k.sid=$1 AND k.purpose='ranked' AND k.ranked_match=$2 AND g.id=$3 AND s.account_id=$4
           AND k.ranked_generation IS NOT DISTINCT FROM r.generation
           AND k.ranked_proof_until>clock_timestamp() AND s.expires_at>clock_timestamp()
           AND s.kind='native' AND a.restored",
        vec![proof.as_bytes().to_vec().into(),id.into(),host.into(),account.into()],
    )).await.map_err(unavailable)?.ok_or(Error::Rejected)?;
    Ok(())
}

pub(super) async fn consent_state(
    db: &DatabaseTransaction,
    id: Uuid,
    me: Uuid,
) -> Result<rpc::ConsentState, Error> {
    let selection = review(db, id).await?;
    let seat = selection.seat(me)?;
    Ok(
        if !selection.active() || selection.recipients.iter().all(Recipient::ready) {
            rpc::ConsentState::Ready
        } else if selection
            .recipients
            .iter()
            .any(|r| !r.verified || !r.approved[seat] || !r.terms_ok[seat])
        {
            rpc::ConsentState::Needed
        } else {
            rpc::ConsentState::Waiting
        },
    )
}

async fn record(
    db: &DatabaseTransaction,
    id: Uuid,
    seat: usize,
    r: &Recipient,
    allowed: bool,
) -> Result<(), Error> {
    let (version, digest) = r.host.terms.as_ref().map_or((None, None), |t| {
        (Some(t.version.clone()), Some(t.digest.to_vec()))
    });
    if allowed {
        // Allocation and approval commit together. Work polls only read this row.
        db.execute_raw(statement(
            "INSERT INTO central_identity.ranked_host_aliases(match_id,host_id) VALUES ($1,$2)
             ON CONFLICT (match_id,host_id) DO NOTHING",
            vec![id.into(), uuid(r.host.host_id).into()],
        ))
        .await
        .map_err(unavailable)?;
    }
    db.execute_raw(statement("INSERT INTO central_identity.ranked_host_permissions(match_id,host_id,seat,epoch,origin,version,digest,allowed)
        VALUES($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT(match_id,host_id,seat) DO UPDATE SET epoch=excluded.epoch,
        origin=excluded.origin,version=excluded.version,digest=excluded.digest,allowed=excluded.allowed",
        vec![id.into(),uuid(r.host.host_id).into(),i16::try_from(seat).map_err(unavailable)?.into(),r.epoch.into(),r.host.origin.clone().into(),version.into(),digest.into(),allowed.into()]))
        .await.map_err(unavailable)?;
    Ok(())
}

pub(super) async fn seat(
    db: &DatabaseTransaction,
    me: Uuid,
    request: &rpc::SeatRequest,
) -> Result<rpc::SeatResponse, Error> {
    let (id, fence) = match request {
        rpc::SeatRequest::Hosts { match_id } => (uuid(*match_id), None),
        rpc::SeatRequest::ApproveHosts { fence, .. }
        | rpc::SeatRequest::DeclineHost { fence, .. } => (uuid(fence.match_id), Some(*fence)),
        _ => return Err(Error::Rejected),
    };
    // A participant can only lock its own match. Routing and settlement serialize
    // against this same row so stale approval cannot authorize a replacement.
    db.query_one_raw(statement(
        "SELECT id FROM central_identity.ranked_matches
        WHERE id=$1 AND $2 IN (account_a,account_b) FOR UPDATE",
        vec![id.into(), me.into()],
    ))
    .await
    .map_err(unavailable)?
    .ok_or(Error::Rejected)?;
    let selection = review(db, id).await?;
    let seat = selection.seat(me)?;
    if fence.is_some_and(|seen| seen != selection.fence) {
        return Err(Error::Conflict);
    }
    match request {
        rpc::SeatRequest::Hosts { .. } if selection.active() => Ok(selection.response(seat)),
        rpc::SeatRequest::ApproveHosts { hosts, .. } if selection.active() => {
            if hosts.is_empty()
                || hosts.len() > 2
                || (hosts.len() == 2 && hosts[0].host_id == hosts[1].host_id)
            {
                return Err(Error::Rejected);
            }
            for host in hosts {
                let r = selection.recipient(host)?;
                if !r.terms_ok[seat] {
                    return Err(Error::Conflict);
                }
                record(db, id, seat, r, true).await?;
            }
            Ok(review(db, id).await?.response(seat))
        }
        rpc::SeatRequest::DeclineHost { host, .. } => {
            let r = selection.recipient(host)?;
            if selection.active() && !r.approved[seat] {
                record(db, id, seat, r, false).await?;
                replication::void(db, id).await?;
            } else if selection.state != "void" || !r.declined[seat] {
                return Err(Error::Conflict);
            }
            Ok(rpc::SeatResponse::Status {
                status: replication::seat_status(db, id, me).await?,
            })
        }
        _ => Err(Error::Conflict),
    }
}
