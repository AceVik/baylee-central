use std::{future::Future, io, net::SocketAddr, time::Duration};

use axum::http::HeaderValue;
use central_policy::config::Config;
use hyper::server::conn::http1;
use hyper_util::{
    rt::{TokioIo, TokioTimer},
    service::TowerToHyperService,
};
use tokio::{net::TcpListener, task::JoinSet};
use tower::ServiceExt;

/// A bound local-only server. Drop before running to release the listener.
/// Runtime connections are owned by a join set and aborted when the run future drops.
pub struct DevelopmentServer {
    listener: TcpListener,
    config: Config,
    address: SocketAddr,
}

impl DevelopmentServer {
    /// Bind a validated loopback address. Use port zero for isolated tests.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if the address cannot be bound.
    pub async fn bind(config: Config) -> io::Result<Self> {
        let listener = TcpListener::bind(config.listen()).await?;
        let address = listener.local_addr()?;
        Ok(Self {
            listener,
            config,
            address,
        })
    }

    /// Actual bound address, including the dynamically assigned test port.
    #[must_use]
    pub const fn local_addr(&self) -> SocketAddr {
        self.address
    }

    /// Serve until shutdown, then drain briefly and abort remaining connections.
    /// Connections have a fixed lifetime and no keep-alive in this dev foundation.
    ///
    /// # Errors
    ///
    /// Returns a listener error or an internal connection-task failure. Request
    /// parsing failures are deliberately not logged with attacker-controlled data.
    pub async fn run(self, shutdown: impl Future<Output = ()>) -> io::Result<()> {
        let authority = HeaderValue::from_str(&self.address.to_string())
            .map_err(|_| io::Error::other("invalid local authority"))?;
        let app = super::boundary::router(authority, self.config.request_bytes().get());
        self.run_router(app, shutdown).await
    }

    /// Serve the native account API through the same bounded loopback transport.
    /// Live mode requires an external TLS reverse proxy preserving the canonical Host.
    /// No forwarded headers confer identity or change credential destinations.
    ///
    /// # Errors
    /// Rejects mismatched development endpoints, inadequate body capacity or listener failures.
    pub async fn run_with_identity(
        self,
        endpoint: &central_api::Endpoint,
        identity: std::sync::Arc<central_identity::IdentityService>,
        shutdown: impl Future<Output = ()>,
    ) -> io::Result<()> {
        let authority = if endpoint.legacy_origin().is_some() {
            HeaderValue::from_static("central.baylee.app")
        } else {
            if endpoint.origin() != format!("http://{}", self.address) {
                return Err(io::Error::other(
                    "development endpoint does not match listener",
                ));
            }
            HeaderValue::from_str(&self.address.to_string())
                .map_err(|_| io::Error::other("invalid authority"))?
        };
        if self.config.request_bytes().get() != central_api::MAX_WIRE_BYTES {
            return Err(io::Error::other(
                "native API requires its fixed body capacity",
            ));
        }
        let app = super::boundary::with_routes(
            authority,
            self.config.request_bytes().get(),
            super::native::router(identity),
        );
        self.run_router(app, shutdown).await
    }

    /// Serve the website's browser listener (`docs/web-accounts.md`): one
    /// route, `POST /api/v1`, for `Host: baylee.app` behind the site's TLS
    /// proxy on loopback, which must pass the `Host` unchanged and append the
    /// client to `X-Forwarded-For`. Only the browser operations are served.
    ///
    /// # Errors
    /// Rejects a body capacity other than the browser listener's, or listener failures.
    pub async fn run_browser(
        self,
        identity: std::sync::Arc<central_identity::IdentityService>,
        shutdown: impl Future<Output = ()>,
    ) -> io::Result<()> {
        if self.config.request_bytes().get() != super::browser::REQUEST_BYTES {
            return Err(io::Error::other(
                "browser listener requires its fixed body capacity",
            ));
        }
        self.run_router(super::browser::router(identity), shutdown)
            .await
    }

    async fn run_router(
        self,
        app: axum::Router,
        shutdown: impl Future<Output = ()>,
    ) -> io::Result<()> {
        let mut connections = JoinSet::new();
        tokio::pin!(shutdown);
        loop {
            tokio::select! {
                biased;
                () = &mut shutdown => break,
                result = connections.join_next(), if !connections.is_empty() => {
                    if result.is_some_and(|result| result.is_err()) {
                        return Err(io::Error::other("connection task failed"));
                    }
                }
                result = self.listener.accept() => {
                    let (stream, peer) = result?;
                    if connections.len() >= self.config.concurrent_requests().get() {
                        // Drop immediately: no unbounded queue of accepted sockets/tasks.
                        drop(stream);
                        continue;
                    }
                    // Each request learns its connection's peer; only the
                    // browser listener reads it (its forwarded address counts
                    // only from the loopback proxy).
                    let app = app.clone().map_request(
                        move |mut request: axum::http::Request<hyper::body::Incoming>| {
                            request.extensions_mut().insert(super::browser::Peer(peer));
                            request
                        },
                    );
                    connections.spawn(async move {
                        let mut builder = http1::Builder::new();
                        builder
                            .keep_alive(false)
                            .max_headers(32)
                            .max_buf_size(16_384)
                            .timer(TokioTimer::new())
                            .header_read_timeout(Duration::from_secs(3));
                        let connection = builder.serve_connection(
                            TokioIo::new(stream),
                            TowerToHyperService::new(app),
                        );
                        let _ = tokio::time::timeout(Duration::from_secs(10), connection).await;
                    });
                }
            }
        }
        drop(self.listener);
        let drained = tokio::time::timeout(Duration::from_secs(3), async {
            while let Some(result) = connections.join_next().await {
                if result.is_err() {
                    return Err(io::Error::other("connection task failed"));
                }
            }
            Ok(())
        })
        .await;
        if let Ok(result) = drained {
            result
        } else {
            connections.abort_all();
            while connections.join_next().await.is_some() {}
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use tokio::sync::oneshot;

    async fn server() -> DevelopmentServer {
        let config = Config::from_pairs([("BAYLEE_CENTRAL_LISTEN", "127.0.0.1:0")]).unwrap();
        DevelopmentServer::bind(config).await.unwrap()
    }

    #[tokio::test]
    async fn shutdown_releases_listener_after_a_real_request() {
        let server = server().await;
        let address = server.local_addr();
        let (stop, stopped) = oneshot::channel();
        let running = tokio::spawn(server.run(async {
            let _ = stopped.await;
        }));
        let response = tokio::task::spawn_blocking(move || {
            let mut connection =
                std::net::TcpStream::connect_timeout(&address, Duration::from_secs(2)).unwrap();
            connection
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            write!(
                connection,
                "GET /health/live HTTP/1.1\r\nHost: {address}\r\n\r\n"
            )
            .unwrap();
            let mut response = String::new();
            connection.take(4096).read_to_string(&mut response).unwrap();
            response
        })
        .await
        .unwrap();
        stop.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(4), running)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert!(response.starts_with("HTTP/1.1 200"));
        assert!(
            std::net::TcpStream::connect_timeout(&address, Duration::from_millis(200)).is_err()
        );
    }

    #[tokio::test]
    async fn cancellation_releases_listener() {
        let server = server().await;
        let address = server.local_addr();
        let running = tokio::spawn(server.run(std::future::pending()));
        tokio::task::yield_now().await;
        running.abort();
        assert!(running.await.unwrap_err().is_cancelled());
        assert!(
            std::net::TcpStream::connect_timeout(&address, Duration::from_millis(200)).is_err()
        );
    }

    #[tokio::test]
    async fn shutdown_with_idle_connection_is_bounded() {
        let server = server().await;
        let address = server.local_addr();
        let (stop, stopped) = oneshot::channel();
        let running = tokio::spawn(server.run(async {
            let _ = stopped.await;
        }));
        let idle = std::net::TcpStream::connect_timeout(&address, Duration::from_secs(2)).unwrap();
        tokio::time::sleep(Duration::from_millis(20)).await;
        stop.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(4), running)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        drop(idle);
        assert!(
            std::net::TcpStream::connect_timeout(&address, Duration::from_millis(200)).is_err()
        );
    }
}
