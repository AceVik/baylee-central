//! Drives rustls over the already-pinned TCP peer. The relay owns the job
//! and aborts it together with the WebSocket on shutdown or revocation.
use rustls::{ClientConfig, ClientConnection, RootCertStore, pki_types::ServerName};
use std::{
    io::{self, Read as _, Write as _},
    sync::Arc,
};
use tokio::{
    io::{AsyncReadExt as _, AsyncWriteExt as _, DuplexStream},
    net::TcpStream,
    task::JoinSet,
};

pub(super) fn start(
    socket: TcpStream,
    host: &str,
    jobs: &mut JoinSet<()>,
) -> io::Result<DuplexStream> {
    let provider = rustls::crypto::ring::default_provider();
    let mut config = ClientConfig::builder_with_provider(Arc::new(provider))
        .with_safe_default_protocol_versions()
        .map_err(io::Error::other)?
        .with_root_certificates(RootCertStore {
            roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
        })
        .with_no_client_auth();
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    let name = ServerName::try_from(host.to_owned()).map_err(io::Error::other)?;
    let mut tls = ClientConnection::new(Arc::new(config), name).map_err(io::Error::other)?;
    tls.set_buffer_limit(Some(32 * 1024));
    let (local, pipe) = tokio::io::duplex(32 * 1024);
    jobs.spawn(async move {
        let _ = drive(socket, tls, pipe).await;
    });
    Ok(local)
}

struct Network<'a>(&'a TcpStream);

impl io::Read for Network<'_> {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        self.0.try_read(bytes)
    }
}

impl io::Write for Network<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.try_write(bytes)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

async fn drive(
    socket: TcpStream,
    mut tls: ClientConnection,
    mut plain: DuplexStream,
) -> io::Result<()> {
    let mut buf = vec![0; 16 * 1024];
    loop {
        if !tls.is_handshaking() {
            match tls.reader().read(&mut buf) {
                Ok(0) => return Ok(()),
                Ok(n) => {
                    plain.write_all(&buf[..n]).await?;
                    continue;
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
                Err(error) => return Err(error),
            }
        }
        tokio::select! {
            ready=socket.writable(), if tls.wants_write()=>{
                ready?;
                match tls.write_tls(&mut Network(&socket)) {
                    Ok(0)=>return Err(io::ErrorKind::WriteZero.into()),
                    Ok(_)=>{},
                    Err(error) if error.kind()==io::ErrorKind::WouldBlock=>{},
                    Err(error)=>return Err(error),
                }
            }
            ready=socket.readable(), if tls.wants_read()=>{
                ready?;
                match tls.read_tls(&mut Network(&socket)) {
                    Ok(0)=>return Ok(()),
                    Ok(_)=>{tls.process_new_packets().map_err(io::Error::other)?;}
                    Err(error) if error.kind()==io::ErrorKind::WouldBlock=>{},
                    Err(error)=>return Err(error),
                }
            }
            n=plain.read(&mut buf), if !tls.is_handshaking() && !tls.wants_write()=>{
                let n=n?;
                if n==0{return Ok(());}
                tls.writer().write_all(&buf[..n])?;
            }
        }
    }
}

#[cfg(test)]
mod tests;
