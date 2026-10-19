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
        let config = Config::from_pairs([
            ("BAYLEE_CENTRAL_LISTEN", "127.0.0.1:0"),
            ("BAYLEE_CENTRAL_REQUEST_BYTES", "1024"),
            ("BAYLEE_CENTRAL_CONCURRENT_REQUESTS", "1"),
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

fn exchange(address: SocketAddr, bytes: &str) -> String {
    let mut socket = TcpStream::connect_timeout(&address, Duration::from_secs(2)).unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    socket
        .set_write_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    if let Err(error) = socket.write_all(bytes.as_bytes()) {
        assert!(matches!(
            error.kind(),
            std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::BrokenPipe
        ));
        return String::new();
    }
    let mut response = Vec::new();
    let mut buffer = [0; 1024];
    loop {
        match socket.read(&mut buffer) {
            Ok(0) => break,
            Ok(count) => response.extend_from_slice(&buffer[..count]),
            Err(error) if error.kind() == std::io::ErrorKind::ConnectionReset => break,
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
            Err(error) if error.kind() == std::io::ErrorKind::ConnectionReset => {}
            result => panic!("cancelled socket was not closed: {result:?}"),
        }
    })
    .await
    .unwrap();
}
