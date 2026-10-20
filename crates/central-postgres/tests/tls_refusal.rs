//! A server that declines TLS must end the attempt: no plaintext startup, no password.
#![forbid(unsafe_code)]

use central_postgres::{ServerConfig, StoreError, connect_server};
use std::{
    io::{ErrorKind, Read, Write},
    net::TcpListener,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

/// PostgreSQL's `SSLRequest`: length 8, request code 80877103.
const SSL_REQUEST: [u8; 8] = [0, 0, 0, 8, 4, 210, 22, 47];

#[tokio::test(flavor = "multi_thread")]
async fn a_server_declining_tls_receives_no_plaintext_startup() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let port = listener.local_addr().unwrap().port();
    let stop = Arc::new(AtomicBool::new(false));
    let (requests, leaked) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
    let server = {
        let (stop, requests, leaked) = (stop.clone(), requests.clone(), leaked.clone());
        std::thread::spawn(move || {
            while !stop.load(Ordering::Acquire) {
                let mut stream = match listener.accept() {
                    Ok((stream, _)) => stream,
                    Err(error) if error.kind() == ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(5));
                        continue;
                    }
                    Err(_) => break,
                };
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_millis(500)))
                    .unwrap();
                let mut request = [0_u8; 8];
                if stream.read_exact(&mut request).is_err() {
                    continue;
                }
                if request == SSL_REQUEST {
                    requests.fetch_add(1, Ordering::AcqRel);
                }
                // "N": this server does not speak TLS. A fallback would now send
                // a plaintext StartupMessage naming the user, then the password.
                let _ = stream.write_all(b"N");
                let deadline = Instant::now() + Duration::from_millis(500);
                let mut rest = [0_u8; 256];
                while Instant::now() < deadline {
                    match stream.read(&mut rest) {
                        Ok(0) | Err(_) => break,
                        Ok(read) => {
                            leaked.fetch_add(read, Ordering::AcqRel);
                        }
                    }
                }
            }
        })
    };
    let config = ServerConfig::new(
        "127.0.0.1",
        port,
        "central_runtime",
        "PRIVATE_PASSWORD_CANARY".into(),
        "central",
        b"-----BEGIN CERTIFICATE-----\nMIIB\n-----END CERTIFICATE-----\n".to_vec(),
    )
    .unwrap();
    let result = connect_server(config).await;
    stop.store(true, Ordering::Release);
    server.join().unwrap();
    assert!(matches!(result, Err(StoreError::Unavailable)));
    assert!(
        requests.load(Ordering::Acquire) >= 1,
        "the client asked for TLS"
    );
    assert_eq!(
        leaked.load(Ordering::Acquire),
        0,
        "plaintext followed a refusal"
    );
}
