//! Attack the real HTTP parser, connection budget and cancellation path.
#![forbid(unsafe_code)]

use central_http::DevelopmentServer;
use central_policy::config::Config;
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    time::Duration,
};
use tokio::task::JoinHandle;

struct Fixture {
    address: SocketAddr,
    running: Option<JoinHandle<std::io::Result<()>>>,
}

impl Fixture {
    async fn start() -> Self {
        Self::taking("1").await
    }

    /// A listener that takes `connections` at once.
    async fn taking(connections: &str) -> Self {
        let config = Config::from_pairs([
            ("BAYLEE_CENTRAL_LISTEN", "127.0.0.1:0"),
            ("BAYLEE_CENTRAL_REQUEST_BYTES", "1024"),
            ("BAYLEE_CENTRAL_CONCURRENT_REQUESTS", connections),
        ])
        .unwrap();
        let server = DevelopmentServer::bind(config).await.unwrap();
        Self {
            address: server.local_addr(),
            running: Some(tokio::spawn(server.run(std::future::pending()))),
        }
    }

    async fn request(&self, tail: &str) -> String {
        let address = self.address;
        let bytes = format!("GET /health/live HTTP/1.1\r\nHost: {address}\r\n{tail}");
        tokio::task::spawn_blocking(move || exchange(address, &bytes))
            .await
            .unwrap()
    }

    async fn stop(mut self) {
        let task = self.running.take().unwrap();
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        assert!(TcpStream::connect_timeout(&self.address, Duration::from_millis(200)).is_err());
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        // Even an assertion failure cancels exactly this owned listener and its JoinSet.
        if let Some(task) = &self.running {
            task.abort();
        }
    }
}

// Windows may report a hard close as WSAECONNABORTED (10053), rather than reset.
// Only closed-connection errors count as closure; timeouts still fail the test.
fn exchange(address: SocketAddr, bytes: &str) -> String {
    exchange_with_deadline(address, bytes, Duration::from_secs(2))
}

fn exchange_with_deadline(address: SocketAddr, bytes: &str, timeout: Duration) -> String {
    let mut socket = TcpStream::connect_timeout(&address, Duration::from_secs(2)).unwrap();
    socket.set_read_timeout(Some(timeout)).unwrap();
    socket
        .set_write_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    if let Err(error) = socket.write_all(bytes.as_bytes()) {
        assert!(matches!(
            error.kind(),
            std::io::ErrorKind::ConnectionReset
                | std::io::ErrorKind::ConnectionAborted
                | std::io::ErrorKind::BrokenPipe
        ));
        return String::new();
    }
    let mut response = Vec::new();
    let mut buffer = [0; 1024];
    loop {
        match socket.read(&mut buffer) {
            Ok(0) => break,
            Ok(count) => response.extend_from_slice(&buffer[..count]),
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::ConnectionAborted
                ) =>
            {
                break;
            }
            Err(error) => panic!("bounded HTTP read failed: {error}"),
        }
        assert!(response.len() < 8192);
    }
    String::from_utf8(response).unwrap()
}

#[tokio::test]
async fn rejects_oversized_declared_and_streamed_bodies_without_echo() {
    let server = Fixture::start().await;
    // No body follows: the response must arrive before the five-second body timeout.
    let response = server.request("Content-Length: 999999999\r\n\r\n").await;
    assert!(response.starts_with("HTTP/1.1 413"));
    assert!(response.contains("cache-control: no-store"));
    let canary = "PRIVATE_BODY_CANARY".repeat(100);
    let response = server
        .request(&format!(
            "Transfer-Encoding: chunked\r\n\r\n{:x}\r\n{canary}\r\n0\r\n\r\n",
            canary.len()
        ))
        .await;
    assert!(response.starts_with("HTTP/1.1 413"));
    assert!(!response.contains("CANARY"));
    let response = server
        .request(&format!("Content-Length: 1024\r\n\r\n{}", "a".repeat(1024)))
        .await;
    assert!(response.starts_with("HTTP/1.1 200"));
    server.stop().await;
}

#[tokio::test]
async fn rejects_header_ambiguity_and_header_resource_exhaustion() {
    let server = Fixture::start().await;
    for headers in [
        "Content-Length: 1\r\nContent-Length: 2\r\n\r\nx".to_owned(),
        format!("hOsT: {}\r\n\r\n", server.address),
    ] {
        assert!(server.request(&headers).await.starts_with("HTTP/1.1 400"));
    }
    for headers in [
        format!("{}\r\n", "X-Filler: x\r\n".repeat(33)),
        format!("X-Filler: {}\r\n\r\n", "x".repeat(20_000)),
    ] {
        assert!(server.request(&headers).await.starts_with("HTTP/1.1 431"));
    }
    // Persistent/pipelined requests never outlive the fixed one-request connection.
    let pipelined = format!(
        "\r\nGET /health/live HTTP/1.1\r\nHost: {}\r\n\r\n",
        server.address
    );
    let response = server.request(&pipelined).await;
    assert_eq!(response.matches("HTTP/1.1 200").count(), 1);
    server.stop().await;
}

#[tokio::test]
async fn overload_drops_excess_socket_and_cancellation_closes_active_socket() {
    let server = Fixture::start().await;
    let mut held = TcpStream::connect_timeout(&server.address, Duration::from_secs(2)).unwrap();
    held.write_all(b"GET /health/live HTTP/1.1\r\nHost: ")
        .unwrap();
    // TCP accept order reserves the first connection's only slot.
    assert_eq!(server.request("\r\n").await, "");
    server.stop().await;
    tokio::task::spawn_blocking(move || {
        held.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        let mut byte = [0];
        match held.read(&mut byte) {
            Ok(0) => {}
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::ConnectionAborted
                ) => {}
            result => panic!("cancelled socket was not closed: {result:?}"),
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn malformed_chunks_are_bad_requests_not_size_errors() {
    let server = Fixture::start().await;
    let response = server
        .request("Transfer-Encoding: chunked\r\n\r\nPRIVATE_INVALID_CHUNK_CANARY\r\n")
        .await;
    assert!(response.starts_with("HTTP/1.1 400"), "{response}");
    assert!(response.contains("cache-control: no-store"));
    assert!(!response.contains("CANARY"));
    server.stop().await;
}

#[tokio::test]
async fn unfinished_body_times_out_and_releases_connection_capacity() {
    let server = Fixture::start().await;
    let address = server.address;
    let started = std::time::Instant::now();
    let response = tokio::task::spawn_blocking(move || {
        exchange_with_deadline(
            address,
            &format!("GET /health/live HTTP/1.1\r\nHost: {address}\r\nContent-Length: 1\r\n\r\n"),
            Duration::from_secs(8),
        )
    })
    .await
    .unwrap();
    assert!(response.starts_with("HTTP/1.1 408"), "{response}");
    assert!(response.contains("cache-control: no-store"));
    assert!(started.elapsed() >= Duration::from_secs(4));
    assert!(server.request("\r\n").await.starts_with("HTTP/1.1 200"));
    server.stop().await;
}

/// A request whose one announced byte is withheld, its socket left open;
/// `forwarded` is the address the local proxy would name.
fn withhold(address: SocketAddr, forwarded: Option<&str>) -> TcpStream {
    let mut socket = TcpStream::connect_timeout(&address, Duration::from_secs(2)).unwrap();
    socket
        .set_write_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let forwarded = forwarded.map_or_else(String::new, |hop| format!("X-Forwarded-For: {hop}\r\n"));
    write!(
        socket,
        "GET /health/live HTTP/1.1\r\nHost: {address}\r\n{forwarded}Content-Length: 1\r\n\r\n"
    )
    .unwrap();
    socket
}

/// What the listener answers on `socket` within `wait`; nothing when it
/// is still waiting for the body.
fn answer(mut socket: &TcpStream, wait: Duration) -> String {
    socket.set_read_timeout(Some(wait)).unwrap();
    let mut response = Vec::new();
    let mut buffer = [0; 1024];
    loop {
        match socket.read(&mut buffer) {
            Ok(0) => break,
            Ok(count) => response.extend_from_slice(&buffer[..count]),
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock
                        | std::io::ErrorKind::TimedOut
                        | std::io::ErrorKind::ConnectionReset
                        | std::io::ErrorKind::ConnectionAborted
                ) =>
            {
                break;
            }
            Err(error) => panic!("bounded HTTP read failed: {error}"),
        }
        assert!(response.len() < 8192);
    }
    String::from_utf8(response).unwrap()
}

/// Longer than the boundary's grace for a body, far shorter than a request's time.
const PAST_GRACE: Duration = Duration::from_millis(700);

#[tokio::test]
async fn bodies_withheld_hold_their_address_s_places_and_the_listener_serves_on() {
    // 32 connections: eight places for late bodies, four an address. Once
    // through the proxy's forwarded address, once by the peer itself.
    for stalling in [Some("198.51.100.7"), None] {
        let server = Fixture::taking("32").await;
        let address = server.address;
        tokio::task::spawn_blocking(move || {
            let held: Vec<TcpStream> = (0..4).map(|_| withhold(address, stalling)).collect();
            std::thread::sleep(PAST_GRACE);
            // A fifth is refused when its grace ends, not at the request's five seconds.
            let started = std::time::Instant::now();
            let fifth = answer(&withhold(address, stalling), Duration::from_secs(4));
            assert!(fifth.starts_with("HTTP/1.1 429"), "{fifth}");
            assert!(fifth.ends_with("account request rejected\n"), "{fifth}");
            assert!(fifth.contains("cache-control: no-store"));
            assert!(started.elapsed() < Duration::from_secs(2));
            // Another address's late body has a place: it waits, and is
            // served when its byte comes.
            let mut other = withhold(address, Some("198.51.100.8"));
            assert_eq!(answer(&other, PAST_GRACE), "");
            other.write_all(b"x").unwrap();
            let late = answer(&other, Duration::from_secs(2));
            assert!(late.starts_with("HTTP/1.1 200"), "{late}");
            // A request whose body is there needs no place: the stalling
            // address itself is served meanwhile.
            let forwarded =
                stalling.map_or_else(String::new, |hop| format!("X-Forwarded-For: {hop}\r\n"));
            let prompt = exchange(
                address,
                &format!(
                    "GET /health/live HTTP/1.1\r\nHost: {address}\r\n{forwarded}Content-Length: 1\r\n\r\nx"
                ),
            );
            assert!(prompt.starts_with("HTTP/1.1 200"), "{prompt}");
            // The four still wait; when their connections go, the places come back.
            for socket in &held {
                assert_eq!(answer(socket, Duration::from_millis(1)), "");
            }
            drop(held);
            std::thread::sleep(Duration::from_millis(300));
            let again = withhold(address, stalling);
            assert_eq!(answer(&again, PAST_GRACE), "");
        })
        .await
        .unwrap();
        server.stop().await;
    }
}

#[tokio::test]
async fn bodies_withheld_from_many_addresses_hold_a_quarter_of_the_connections() {
    // eight connections: two places
    let server = Fixture::taking("8").await;
    let address = server.address;
    tokio::task::spawn_blocking(move || {
        let held = [
            withhold(address, Some("198.51.100.1")),
            withhold(address, Some("2001:db8::1")),
        ];
        std::thread::sleep(PAST_GRACE);
        for newcomer in ["198.51.100.3", "2001:db8:0:1::1"] {
            let refused = answer(&withhold(address, Some(newcomer)), Duration::from_secs(4));
            assert!(refused.starts_with("HTTP/1.1 429"), "{refused}");
        }
        // Six connections are left for requests whose body is there.
        for _ in 0..12 {
            let prompt = exchange(
                address,
                &format!("GET /health/live HTTP/1.1\r\nHost: {address}\r\nX-Forwarded-For: 198.51.100.3\r\n\r\n"),
            );
            assert!(prompt.starts_with("HTTP/1.1 200"), "{prompt}");
        }
        // The two end at the request's time with 408, as a withheld body always did.
        for socket in &held {
            let timed_out = answer(socket, Duration::from_secs(6));
            assert!(timed_out.starts_with("HTTP/1.1 408"), "{timed_out}");
        }
    })
    .await
    .unwrap();
    server.stop().await;
}
