//! The development listener's HTTP boundary, spoken by hand over a socket
//! (no HTTP client is needed for six requests): the status of four routes,
//! `Cache-Control: no-store` on each, a foreign `Origin` and `Host` refused,
//! and the header deadline closing a stalled request after two to four and
//! a half seconds.

use std::io::{ErrorKind, Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::time::{Duration, Instant};

use crate::fail::{Fail, OrFail, Result};

const ROUTES: [(&str, u16); 4] = [
    ("/health/live", 200),
    ("/health/ready", 503),
    ("/auth/login", 404),
    ("/v1/training", 404),
];

struct Response {
    status: u16,
    no_store: bool,
}

fn connect(port: u16) -> Result<TcpStream> {
    let stream = TcpStream::connect_timeout(
        &SocketAddr::from((Ipv4Addr::LOCALHOST, port)),
        Duration::from_secs(5),
    )
    .or_fail("Development listener unreachable")?;
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .or_fail("Development listener unreachable")?;
    Ok(stream)
}

fn get(port: u16, path: &str, host: Option<&str>, origin: Option<&str>) -> Result<Response> {
    let mut stream = connect(port)?;
    let host = host.map_or_else(|| format!("127.0.0.1:{port}"), str::to_owned);
    let origin = origin
        .map(|o| format!("Origin: {o}\r\n"))
        .unwrap_or_default();
    let request =
        format!("GET {path} HTTP/1.1\r\nHost: {host}\r\n{origin}Connection: close\r\n\r\n");
    stream
        .write_all(request.as_bytes())
        .or_fail("Development listener unreachable")?;
    let mut reply = Vec::new();
    stream
        .take(65_536)
        .read_to_end(&mut reply)
        .or_fail("Development listener did not answer")?;
    parse(&String::from_utf8_lossy(&reply)).or_fail("Development listener did not answer")
}

fn parse(reply: &str) -> Option<Response> {
    let head = reply.split("\r\n\r\n").next()?;
    let mut lines = head.split("\r\n");
    let status = lines.next()?.split(' ').nth(1)?.parse().ok()?;
    let no_store = lines
        .filter_map(|line| line.split_once(':'))
        .any(|(name, value)| {
            name.trim().eq_ignore_ascii_case("cache-control")
                && value
                    .split(',')
                    .any(|d| d.trim().eq_ignore_ascii_case("no-store"))
        });
    Some(Response { status, no_store })
}

pub(super) fn check_boundary(port: u16) -> Result<()> {
    for (path, status) in ROUTES {
        let response = get(port, path, None, None)?;
        if response.status != status {
            return Err(Fail::new("Unexpected endpoint status"));
        }
        if !response.no_store {
            return Err(Fail::new("Missing no-store policy"));
        }
    }
    let attacks = [
        get(port, "/health/live", None, Some("https://attacker.invalid"))?.status == 403,
        get(port, "/health/live", Some("attacker.invalid"), None)?.status == 400,
    ];
    if attacks.contains(&false) {
        return Err(Fail::new("HTTP boundary did not reject attacker header"));
    }
    Ok(())
}

/// A request that stops inside its headers is closed by the server's own
/// deadline (outside request middleware): an empty reply or a 408, after
/// two to four and a half seconds. Only a reset or an abort counts as the
/// peer closing; any other error proves nothing.
pub(super) fn check_header_deadline(port: u16) -> Result<()> {
    let mut stream = connect(port)?;
    stream
        .write_all(b"GET /health/live HTTP/1.1\r\nHost: ")
        .or_fail("Development listener unreachable")?;
    let started = Instant::now();
    let mut reply = Vec::new();
    let mut buffer = [0_u8; 256];
    loop {
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => {
                reply.extend_from_slice(&buffer[..read]);
                if reply.len() > 1024 {
                    return Err(Fail::new("Unexpected timeout response size"));
                }
            }
            Err(error)
                if matches!(
                    error.kind(),
                    ErrorKind::ConnectionReset | ErrorKind::ConnectionAborted
                ) =>
            {
                break;
            }
            Err(_) => {
                return Err(Fail::new(
                    "Header deadline did not close stalled connection",
                ));
            }
        }
    }
    let elapsed = started.elapsed();
    if !reply.is_empty() && !reply.starts_with(b"HTTP/1.1 408") {
        return Err(Fail::new("Stalled headers received an unexpected response"));
    }
    if elapsed < Duration::from_secs(2) || elapsed > Duration::from_millis(4_500) {
        return Err(Fail::new(
            "Header deadline did not close stalled connection",
        ));
    }
    Ok(())
}
