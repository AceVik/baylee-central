//! An independently authenticated WebSocket client. A verified match grant
//! fixes its origin and path. Never copy headers from the native connection.
use super::tls;
use crate::{
    relay::{Grant, game_id},
    ws,
};
use axum::http::Uri;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use std::{
    io,
    net::{IpAddr, SocketAddr},
};
use tokio::{
    io::{AsyncRead, AsyncReadExt as _, AsyncWrite, AsyncWriteExt as _, BufReader},
    net::{TcpStream, lookup_host},
    task::JoinSet,
};
use zeroize::Zeroizing;

/// Baylee's game handshake v69. The negotiated ranked replica wire is additive.
const GAME_PROTOCOL: u32 = 69;
pub(super) trait Stream: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> Stream for T {}

pub(super) struct Upstream {
    pub(super) io: BufReader<Box<dyn Stream>>,
    _jobs: JoinSet<()>,
}

fn refused() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "ranked upstream refused")
}

pub(super) async fn open(grant: &Grant) -> io::Result<Upstream> {
    if grant.seat > 1 {
        return Err(refused());
    }
    connect(
        &grant.origin,
        grant.development,
        &grant.game,
        &grant.upstream_ticket,
    )
    .await
}

async fn connect(
    origin: &str,
    development: bool,
    game: &central_api::ranked::Id,
    ticket: &str,
) -> io::Result<Upstream> {
    let local = development && origin.starts_with("http://");
    central_api::gateway::validate_origin(origin, local).map_err(|_| refused())?;
    if ticket.len() != 64
        || !ticket
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(refused());
    }
    let uri = origin.parse::<Uri>().map_err(|_| refused())?;
    let authority = uri.authority().ok_or_else(refused)?;
    let host = authority.host().trim_matches(['[', ']']);
    let port = authority.port_u16().unwrap_or(443);
    let addresses = if local {
        let ip = host.parse::<IpAddr>().map_err(|_| refused())?;
        if !ip.is_loopback() {
            return Err(refused());
        }
        vec![SocketAddr::new(ip, port)]
    } else {
        public_addresses(lookup_host((host, port)).await?)?
    };
    let socket = TcpStream::connect(&addresses[..]).await?;
    socket.set_nodelay(true)?;
    let mut jobs = JoinSet::new();
    let stream: Box<dyn Stream> = if local {
        Box::new(socket)
    } else {
        Box::new(tls::start(socket, host, &mut jobs)?)
    };
    let mut io = BufReader::new(stream);
    let mut random = [0; 16];
    getrandom::fill(&mut random).map_err(|_| io::Error::other("entropy unavailable"))?;
    let key = STANDARD.encode(random);
    let request = Zeroizing::new(format!(
        "GET /games/{}/ws?ticket={ticket}&protocol={GAME_PROTOCOL} HTTP/1.1\r\nHost: {authority}\r\nConnection: Upgrade\r\nUpgrade: websocket\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Key: {key}\r\n\r\n",
        game_id(game)
    ));
    io.write_all(request.as_bytes()).await?;
    io.flush().await?;
    drop(request);
    let mut head = Vec::with_capacity(512);
    while head.len() < 8192 {
        head.push(io.read_u8().await?);
        if head.ends_with(b"\r\n\r\n") {
            verify(&head, &key)?;
            return Ok(Upstream { io, _jobs: jobs });
        }
    }
    Err(refused())
}

/// Validate *every* address from one resolution, then connect directly to
/// that immutable set. A mixed DNS answer cannot select a private peer.
fn public_addresses(addresses: impl Iterator<Item = SocketAddr>) -> io::Result<Vec<SocketAddr>> {
    let mut pinned = Vec::new();
    for addr in addresses {
        if !central_outbound::public(addr.ip()) || pinned.len() >= 32 {
            return Err(refused());
        }
        pinned.push(addr);
    }
    if pinned.is_empty() {
        return Err(refused());
    }
    Ok(pinned)
}

fn verify(head: &[u8], key: &str) -> io::Result<()> {
    let mut headers = [httparse::EMPTY_HEADER; 32];
    let mut response = httparse::Response::new(&mut headers);
    let parsed = response.parse(head).map_err(|_| refused())?;
    if !parsed.is_complete() || response.version != Some(1) || response.code != Some(101) {
        return Err(refused());
    }
    let accept = ws::accept(key).ok_or_else(refused)?;
    let mut accepted = false;
    let mut upgrade = false;
    let mut connection = false;
    for h in response.headers.iter() {
        if h.name.eq_ignore_ascii_case("Sec-WebSocket-Accept") {
            if accepted || h.value != accept.as_bytes() {
                return Err(refused());
            }
            accepted = true;
        } else if h.name.eq_ignore_ascii_case("Upgrade") {
            if upgrade || !h.value.eq_ignore_ascii_case(b"websocket") {
                return Err(refused());
            }
            upgrade = true;
        } else if h.name.eq_ignore_ascii_case("Connection") {
            if connection
                || !h
                    .value
                    .split(|b| *b == b',')
                    .any(|v| v.trim_ascii().eq_ignore_ascii_case(b"upgrade"))
            {
                return Err(refused());
            }
            connection = true;
        } else if [
            "Sec-WebSocket-Extensions",
            "Sec-WebSocket-Protocol",
            "Transfer-Encoding",
            "Content-Length",
        ]
        .iter()
        .any(|n| h.name.eq_ignore_ascii_case(n))
        {
            return Err(refused());
        }
    }
    if accepted && upgrade && connection {
        Ok(())
    } else {
        Err(refused())
    }
}

#[cfg(test)]
mod tests;
