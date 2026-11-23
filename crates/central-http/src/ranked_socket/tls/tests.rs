use super::*;
use rustls::{
    ServerConfig, ServerConnection,
    pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject as _},
};
use std::{net::TcpListener, process::Command, time::Duration};
use zeroize::Zeroizing;

#[tokio::test]
#[ignore = "explicit integration check using the PostgreSQL gate's openssl prerequisite"]
async fn real_tls_transfers_binary_and_refuses_the_wrong_certificate_name() {
    // A fresh, synthetic localhost-only test identity. Both PEM blocks stay
    // in the command's output pipe, never in a file or a test log.
    let generated = Command::new("openssl")
        .args([
            "req",
            "-x509",
            "-nodes",
            "-newkey",
            "ec",
            "-pkeyopt",
            "ec_paramgen_curve:P-256",
            "-days",
            "1",
            "-subj",
            "/CN=localhost",
            "-addext",
            "subjectAltName=DNS:localhost",
            "-addext",
            "basicConstraints=critical,CA:FALSE",
            "-keyout",
            "/dev/stdout",
        ])
        .output()
        .unwrap();
    assert!(generated.status.success());
    let pem = Zeroizing::new(generated.stdout);
    let cert = CertificateDer::from_pem_slice(&pem).unwrap();
    let key = PrivateKeyDer::from_pem_slice(&pem).unwrap();
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let server = Arc::new(
        ServerConfig::builder_with_provider(provider.clone())
            .with_safe_default_protocol_versions()
            .unwrap()
            .with_no_client_auth()
            .with_single_cert(vec![cert.clone()], key)
            .unwrap(),
    );
    let mut roots = RootCertStore::empty();
    roots.add(cert).unwrap();
    let client = Arc::new(
        ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .unwrap()
            .with_root_certificates(roots)
            .with_no_client_auth(),
    );
    for (name, success) in [("localhost", true), ("other.invalid", false)] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let config = server.clone();
        let peer = tokio::task::spawn_blocking(move || {
            let (s, _) = listener.accept().unwrap();
            s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
            let mut s = rustls::StreamOwned::new(ServerConnection::new(config).unwrap(), s);
            let mut msg = [0; 4];
            if s.read_exact(&mut msg).is_err() {
                return false;
            }
            assert_eq!(msg, *b"game");
            s.write_all(b"seat").unwrap();
            s.flush().unwrap();
            true
        });
        let socket = TcpStream::connect(addr).await.unwrap();
        let tls =
            ClientConnection::new(client.clone(), ServerName::try_from(name).unwrap()).unwrap();
        let (mut plain, pipe) = tokio::io::duplex(1024);
        let driver = tokio::spawn(drive(socket, tls, pipe));
        plain.write_all(b"game").await.unwrap();
        let mut reply = [0; 4];
        let received = tokio::time::timeout(Duration::from_secs(6), plain.read_exact(&mut reply))
            .await
            .unwrap();
        assert_eq!(received.is_ok(), success);
        if success {
            assert_eq!(reply, *b"seat");
        }
        drop(plain);
        assert_eq!(peer.await.unwrap(), success);
        let _ = driver.await.unwrap();
    }
}
