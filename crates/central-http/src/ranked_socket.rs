//! Central's seat-bound native game relay. The client's first binary message
//! redeems a single-use ticket before Central opens a connection to the host.
mod frames;
mod tls;
mod upstream;

use crate::{
    in_flight::Client,
    relay::{Grant, Relay},
    ws,
};
use axum::{
    extract::Request,
    http::StatusCode,
    response::{IntoResponse as _, Response},
};
use central_identity::IdentityService;
use hyper_util::rt::TokioIo;
use std::{sync::Arc, time::Duration};
use tokio::{
    io::AsyncReadExt as _,
    sync::Semaphore,
    time::{Instant, interval_at, timeout},
};
use zeroize::Zeroizing;

/// Each relay needs two file descriptors and shares the listener's existing
/// per-address socket and dial limits with push connections.
static SLOTS: Semaphore = Semaphore::const_new(64);
const AUTH_TIME: Duration = Duration::from_secs(3);
const DIAL_TIME: Duration = Duration::from_secs(6);
const RECHECK: Duration = Duration::from_secs(3);
const RECHECK_TIME: Duration = Duration::from_secs(2);

pub(super) fn dial(
    relay: Relay,
    identity: Arc<IdentityService>,
    from: Option<Client>,
    request: Request,
) -> Response {
    if request.uri().query().is_some()
        || request.headers().contains_key(axum::http::header::ORIGIN)
        || request
            .headers()
            .contains_key(axum::http::header::AUTHORIZATION)
    {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let opening = match ws::Opening::admit(&request, from) {
        Ok(opening) => opening,
        Err(error) => return error.into_response(),
    };
    let Ok(slot) = SLOTS.try_acquire() else {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    };
    opening.start(request, move |place, upgrade, mut stop| async move {
        let (_slot, _place) = (slot, place);
        tokio::select! {
            ()=serve(relay,identity,upgrade)=>{},
            _=stop.changed()=>{},
            ()=tokio::time::sleep(Duration::from_hours(12))=>{},
        }
    })
}

async fn serve(relay: Relay, identity: Arc<IdentityService>, upgrade: hyper::upgrade::OnUpgrade) {
    let Ok(Ok(io)) = timeout(AUTH_TIME, upgrade).await else {
        return;
    };
    let mut client = TokioIo::new(io);
    let Ok(Ok(ticket)) = timeout(AUTH_TIME, first_ticket(&mut client)).await else {
        return;
    };
    let Ok(mut grant) = relay.take(&ticket[..]) else {
        return;
    };
    drop(ticket);
    if !live(&grant, &identity).await {
        return;
    }
    let Ok(Ok(mut host)) = timeout(DIAL_TIME, upstream::open(&grant)).await else {
        return;
    };
    drop(std::mem::take(&mut grant.upstream_ticket));
    if !live(&grant, &identity).await {
        return;
    }
    let (native_read, native_write) = tokio::io::split(client);
    let (gateway_read, gateway_write) = tokio::io::split(&mut host.io);
    tokio::select! {
        _=frames::forward(native_read,gateway_write,frames::From::Native)=>{},
        _=frames::forward(gateway_read,native_write,frames::From::Gateway)=>{},
        ()=revoked(&grant,&identity)=>{},
    }
}

async fn live(grant: &Grant, identity: &IdentityService) -> bool {
    timeout(RECHECK_TIME, grant.current(identity))
        .await
        .unwrap_or(false)
}

async fn revoked(grant: &Grant, identity: &IdentityService) {
    let mut ticks = interval_at(Instant::now() + RECHECK, RECHECK);
    ticks.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        ticks.tick().await;
        if !live(grant, identity).await {
            return;
        }
    }
}

async fn first_ticket(
    reader: &mut (impl tokio::io::AsyncRead + Unpin),
) -> std::io::Result<Zeroizing<[u8; 64]>> {
    let header = frames::Header::read(reader, frames::From::Native).await?;
    if !header.fin || header.opcode != 2 || header.length != 64 {
        return Err(frames::invalid());
    }
    let mut ticket = Zeroizing::new([0u8; 64]);
    reader.read_exact(&mut ticket[..]).await?;
    frames::mask(&mut ticket[..], header.mask, 0);
    if !ticket
        .iter()
        .all(|&b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(frames::invalid());
    }
    Ok(ticket)
}

#[cfg(test)]
mod tests;
