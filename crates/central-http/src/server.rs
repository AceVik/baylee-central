use std::{
    future::Future,
    io,
    net::SocketAddr,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use super::{
    browser::BrowserOrigin,
    ws::{self, Job, Sockets, Upgrades},
};
use axum::http::HeaderValue;
use central_policy::config::Config;
use hyper::server::conn::http1;
use hyper_util::{
    rt::{TokioIo, TokioTimer},
    service::TowerToHyperService,
};
use tokio::{
    net::TcpListener,
    sync::mpsc,
    task::{JoinError, JoinSet},
};
use tower::ServiceExt;

/// How long one connection may last: a request and its answer.
const CONNECTION: Duration = Duration::from_secs(10);

/// A bound local-only server. Drop before running to release the listener.
/// Runtime connections are owned by a join set and aborted when the run future drops.
pub struct DevelopmentServer {
    listener: TcpListener,
    config: Config,
    address: SocketAddr,
    /// Connection tasks that panicked since the listener was bound.
    panics: Arc<AtomicU64>,
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
            panics: Arc::default(),
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
        let app = super::boundary::router(
            authority,
            self.config.request_bytes().get(),
            self.config.concurrent_requests().get(),
        );
        self.run_router(app, CONNECTION, shutdown).await
    }

    /// Serve the native account API through the same bounded loopback transport.
    /// Live mode requires an external TLS reverse proxy preserving the canonical Host.
    /// No forwarded headers confer identity or change credential destinations; the
    /// proxy's `X-Forwarded-For` hop shares out the places of late and large
    /// bodies (`boundary.rs`) and is the address the budgets and the guesses at
    /// passwords are counted by (`addresses.rs`, `docs/native-login.md`).
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
            self.config.concurrent_requests().get(),
            super::native::router(Arc::clone(&identity), endpoint.legacy_origin().is_some()),
        );
        // The push sockets (`ws.rs`), and the periodic check of their sessions.
        let sockets = Sockets::new();
        sockets.check(identity);
        // a report part may take its upload's time
        self.serve(
            app,
            super::boundary::UPLOAD_TIME + CONNECTION,
            Some(sockets),
            shutdown,
        )
        .await
    }

    /// Serve the website's browser listener (`docs/web-accounts.md`): one
    /// route, `POST /api/v1`, for the page at `site` behind its proxy on
    /// loopback, which must pass the `Host` unchanged and append the client to
    /// `X-Forwarded-For`. Only the browser operations are served. The deployed
    /// service passes [`BrowserOrigin::live`]; a numeric loopback
    /// [`BrowserOrigin::development`] is for local runs only.
    ///
    /// # Errors
    /// Rejects a body capacity other than the browser listener's, or listener failures.
    pub async fn run_browser(
        self,
        identity: std::sync::Arc<central_identity::IdentityService>,
        site: BrowserOrigin,
        shutdown: impl Future<Output = ()>,
    ) -> io::Result<()> {
        if self.config.request_bytes().get() != super::browser::REQUEST_BYTES {
            return Err(io::Error::other(
                "browser listener requires its fixed body capacity",
            ));
        }
        // A picture's upload may take its body's and its conversion's time.
        self.run_router(
            super::browser::router(identity, site),
            super::browser::PICTURE_UPLOAD_TIME + CONNECTION,
            shutdown,
        )
        .await
    }

    /// Serve `app`, each connection for at most `connection` (one request
    /// each: no keep-alive), its headers within three seconds. A connection
    /// task that panics ends its own connection only (`ended`); the
    /// listener goes on.
    pub(crate) async fn run_router(
        self,
        app: axum::Router,
        connection: Duration,
        shutdown: impl Future<Output = ()>,
    ) -> io::Result<()> {
        self.serve(app, connection, None, shutdown).await
    }

    /// [`Self::run_router`], with push sockets when `sockets` is given
    /// (`ws.rs`). A connection whose first line asks for the socket may be
    /// upgraded, and every other answer on it ends it as an ordinary
    /// connection's does; every other connection is served as before, with no
    /// keep-alive. An upgraded socket leaves the connections, so it holds
    /// none of their places and is not cut at their time: it runs in a set of
    /// its own, bounded by its admission, until it ends or the listener
    /// stops, which closes it.
    async fn serve(
        self,
        app: axum::Router,
        connection: Duration,
        sockets: Option<Sockets>,
        shutdown: impl Future<Output = ()>,
    ) -> io::Result<()> {
        let mut connections = JoinSet::new();
        let mut open: JoinSet<()> = JoinSet::new();
        let (upgrades, mut jobs, stop) = match sockets {
            Some(sockets) => (
                Some(sockets.upgrades),
                Some(sockets.jobs),
                Some(sockets.stop),
            ),
            None => (None, None, None),
        };
        tokio::pin!(shutdown);
        loop {
            tokio::select! {
                biased;
                () = &mut shutdown => break,
                result = connections.join_next(), if !connections.is_empty() => {
                    if let Some(Err(error)) = result {
                        ended(&error, &self.panics)?;
                    }
                }
                result = open.join_next(), if !open.is_empty() => {
                    if let Some(Err(error)) = result {
                        ended(&error, &self.panics)?;
                    }
                }
                job = next_job(&mut jobs) => {
                    open.spawn(job);
                }
                result = self.listener.accept() => {
                    let (stream, peer) = result?;
                    if connections.len() >= self.config.concurrent_requests().get() {
                        // Drop immediately: no unbounded queue of accepted sockets/tasks.
                        drop(stream);
                        continue;
                    }
                    let app = app.clone();
                    let upgrades = upgrades.clone();
                    let limit = connection;
                    connections.spawn(async move {
                        let _ = tokio::time::timeout(limit, connect(stream, peer, app, upgrades)).await;
                    });
                }
            }
        }
        drop(self.listener);
        // The sockets close (`ws.rs`: a close frame, a short wait for the
        // client's); what has not ended when the drain is over is aborted.
        if let Some(stop) = stop {
            let _ = stop.send(true);
        }
        drop(jobs);
        let drained = tokio::time::timeout(Duration::from_secs(3), async {
            while let Some(result) = connections.join_next().await {
                if let Err(error) = result {
                    ended(&error, &self.panics)?;
                }
            }
            while let Some(result) = open.join_next().await {
                if let Err(error) = result {
                    ended(&error, &self.panics)?;
                }
            }
            Ok(())
        })
        .await;
        if let Ok(result) = drained {
            result
        } else {
            connections.abort_all();
            open.abort_all();
            while connections.join_next().await.is_some() {}
            while open.join_next().await.is_some() {}
            Ok(())
        }
    }
}

/// The next socket admitted, once there is one; never, without sockets.
async fn next_job(jobs: &mut Option<mpsc::Receiver<Job>>) -> Job {
    if let Some(jobs) = jobs
        && let Some(job) = jobs.recv().await
    {
        return job;
    }
    std::future::pending().await
}

/// Serve one connection from `peer`. One that asked for the push socket in
/// its first line (`ws::asks`) may be upgraded, and every other answer on it
/// says `Connection: close`, after which hyper ends it; every other
/// connection has no keep-alive, as it always had.
async fn connect(
    stream: tokio::net::TcpStream,
    peer: SocketAddr,
    app: axum::Router,
    upgrades: Option<Upgrades>,
) {
    let asked = match &upgrades {
        Some(_) => ws::asks(&stream).await,
        None => false,
    };
    let mut builder = http1::Builder::new();
    builder
        .max_headers(32)
        .max_buf_size(16_384)
        .timer(TokioTimer::new())
        .header_read_timeout(Duration::from_secs(3));
    // Each request learns its connection's peer: a forwarded address counts
    // only from the loopback proxy, for the browser listener's budgets and
    // both listeners' places. The push socket's request learns what lets it
    // upgrade, and only on a connection that asked for it.
    let upgrades = upgrades.filter(|_| asked);
    let app = app.map_request(
        move |mut request: axum::http::Request<hyper::body::Incoming>| {
            request.extensions_mut().insert(super::browser::Peer(peer));
            if let Some(upgrades) = &upgrades {
                request.extensions_mut().insert(upgrades.clone());
            }
            request
        },
    );
    if asked {
        let app = app.map_response(ws::closing);
        let _ = builder
            .serve_connection(TokioIo::new(stream), TowerToHyperService::new(app))
            .with_upgrades()
            .await;
    } else {
        builder.keep_alive(false);
        let _ = builder
            .serve_connection(TokioIo::new(stream), TowerToHyperService::new(app))
            .await;
    }
}

/// A connection task that did not finish: one that panicked is counted and
/// reported with a fixed line (never its payload or anything of the
/// request; the launcher's panic hook prints none either), and the listener
/// goes on; any other failure (a cancelled task) stops the listener as before.
fn ended(error: &JoinError, panics: &AtomicU64) -> io::Result<()> {
    if !error.is_panic() {
        return Err(io::Error::other("connection task failed"));
    }
    let before = panics.fetch_add(1, Ordering::Relaxed);
    eprintln!(
        "central: a connection task failed and was dropped ({} since start)",
        before.saturating_add(1)
    );
    Ok(())
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

    fn exchange(address: SocketAddr, path: &str) -> String {
        let mut connection =
            std::net::TcpStream::connect_timeout(&address, Duration::from_secs(2)).unwrap();
        connection
            .set_read_timeout(Some(Duration::from_secs(4)))
            .unwrap();
        write!(connection, "GET {path} HTTP/1.1\r\nHost: {address}\r\n\r\n").unwrap();
        let mut response = Vec::new();
        // A connection whose task panicked is closed, maybe reset.
        let _ = connection.take(4096).read_to_end(&mut response);
        String::from_utf8_lossy(&response).into_owned()
    }

    #[tokio::test]
    async fn a_panicking_connection_ends_alone_and_the_listener_goes_on() {
        let bound = server().await;
        let address = bound.local_addr();
        let panics = Arc::clone(&bound.panics);
        let app = axum::Router::new()
            .route(
                "/panic",
                axum::routing::get(|| async {
                    assert!(!std::hint::black_box(true), "synthetic connection failure");
                    "unreachable\n"
                }),
            )
            .route("/health/live", axum::routing::get(|| async { "alive\n" }));
        let (stop, stopped) = oneshot::channel();
        let running = tokio::spawn(bound.run_router(app, CONNECTION, async {
            let _ = stopped.await;
        }));
        let (failed, served, again) = tokio::task::spawn_blocking(move || {
            (
                exchange(address, "/panic"),
                exchange(address, "/health/live"),
                exchange(address, "/health/live"),
            )
        })
        .await
        .unwrap();
        assert!(!failed.contains("unreachable") && !failed.contains("synthetic"));
        assert!(served.starts_with("HTTP/1.1 200"), "{served}");
        assert!(again.starts_with("HTTP/1.1 200"), "{again}");
        stop.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(4), running)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(panics.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn only_a_panic_lets_the_listener_go_on() {
        let panics = AtomicU64::new(0);
        let panicked = tokio::spawn(async {
            assert!(!std::hint::black_box(true), "synthetic connection failure");
        })
        .await
        .unwrap_err();
        assert!(ended(&panicked, &panics).is_ok());
        assert_eq!(panics.load(Ordering::Relaxed), 1);
        let cancelled = tokio::spawn(std::future::pending::<()>());
        cancelled.abort();
        let cancelled = cancelled.await.unwrap_err();
        assert!(ended(&cancelled, &panics).is_err());
        assert_eq!(panics.load(Ordering::Relaxed), 1);
    }

    /// Write `bytes` on a new connection to `address` and read until it ends
    /// (or four seconds pass): everything the server answered.
    fn answered(address: SocketAddr, bytes: &str) -> String {
        let mut connection =
            std::net::TcpStream::connect_timeout(&address, Duration::from_secs(2)).unwrap();
        connection
            .set_read_timeout(Some(Duration::from_secs(4)))
            .unwrap();
        connection.write_all(bytes.as_bytes()).unwrap();
        let mut response = Vec::new();
        let _ = connection.take(16_384).read_to_end(&mut response);
        String::from_utf8_lossy(&response).into_owned()
    }

    /// A route at `/v1/ws` that upgrades, as `ws.rs` does, when its
    /// connection asked for it and the request says so, into a socket that
    /// echoes until the sockets stop; and refuses otherwise.
    fn upgrading() -> axum::Router {
        use axum::{extract::Request, http::StatusCode, response::IntoResponse};
        axum::Router::new()
            .route(
                "/v1/ws",
                axum::routing::get(|mut request: Request| async move {
                    let Some(upgrades) = request.extensions().get::<Upgrades>().cloned() else {
                        return (StatusCode::BAD_REQUEST, "no upgrade here\n").into_response();
                    };
                    if !request.headers().contains_key("upgrade") {
                        return (StatusCode::UNAUTHORIZED, "refused\n").into_response();
                    }
                    let on_upgrade = hyper::upgrade::on(&mut request);
                    upgrades.echo(on_upgrade);
                    let mut response = axum::response::Response::new(axum::body::Body::empty());
                    *response.status_mut() = StatusCode::SWITCHING_PROTOCOLS;
                    let headers = response.headers_mut();
                    headers.insert("upgrade", HeaderValue::from_static("websocket"));
                    headers.insert("connection", HeaderValue::from_static("upgrade"));
                    response
                }),
            )
            .route("/health/live", axum::routing::get(|| async { "alive\n" }))
    }

    #[tokio::test]
    async fn only_a_connection_that_asked_is_upgraded_and_every_other_answer_closes() {
        let bound = server().await;
        let address = bound.local_addr();
        let (stop, stopped) = oneshot::channel();
        let running =
            tokio::spawn(
                bound.serve(upgrading(), CONNECTION, Some(Sockets::new()), async {
                    let _ = stopped.await;
                }),
            );
        let head = "Host: x\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\r\n";
        let (upgraded, refused, elsewhere, ordinary, echoed) =
            tokio::task::spawn_blocking(move || {
                // Upgraded: 101 says upgrade, not close, and the socket is its own.
                let mut socket =
                    std::net::TcpStream::connect_timeout(&address, Duration::from_secs(2)).unwrap();
                socket.set_read_timeout(Some(Duration::from_secs(4))).unwrap();
                write!(socket, "GET /v1/ws HTTP/1.1\r\n{head}").unwrap();
                let mut answer = [0_u8; 256];
                let read = socket.read(&mut answer).unwrap();
                let upgraded = String::from_utf8_lossy(&answer[..read]).into_owned();
                socket.write_all(b"after the upgrade").unwrap();
                let mut echoed = [0_u8; 17];
                socket.read_exact(&mut echoed).unwrap();
                // Refused on a connection that asked: the answer closes it, and a
                // request sent behind it is never answered.
                let refused = answered(
                    address,
                    "GET /v1/ws HTTP/1.1\r\nHost: x\r\n\r\nGET /health/live HTTP/1.1\r\nHost: x\r\n\r\n",
                );
                // The route on a connection whose first line asked for something else.
                let elsewhere = answered(address, &format!("GET /v1/ws?x=1 HTTP/1.1\r\n{head}"));
                // An ordinary connection: one request, then closed, as before.
                let ordinary = answered(
                    address,
                    "GET /health/live HTTP/1.1\r\nHost: x\r\n\r\nGET /health/live HTTP/1.1\r\nHost: x\r\n\r\n",
                );
                (upgraded, refused, elsewhere, ordinary, (socket, echoed))
            })
            .await
            .unwrap();
        let lower = upgraded.to_ascii_lowercase();
        assert!(lower.starts_with("http/1.1 101"), "{upgraded}");
        assert!(lower.contains("connection: upgrade"), "{upgraded}");
        assert!(!lower.contains("close"), "{upgraded}");
        let (mut socket, echoed) = echoed;
        assert_eq!(&echoed, b"after the upgrade");
        for (answer, status) in [(&refused, "401"), (&elsewhere, "400"), (&ordinary, "200")] {
            assert_eq!(answer.matches("HTTP/1.1 ").count(), 1, "{answer}");
            assert!(
                answer.starts_with(&format!("HTTP/1.1 {status}")),
                "{answer}"
            );
            assert!(
                answer.to_ascii_lowercase().contains("connection: close"),
                "{answer}"
            );
        }
        // The listener stops: the upgraded socket is closed, not left behind.
        stop.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(4), running)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let ended = tokio::task::spawn_blocking(move || {
            let mut rest = Vec::new();
            socket.read_to_end(&mut rest).map(|_| rest)
        })
        .await
        .unwrap();
        assert_eq!(ended.unwrap(), b"bye");
        assert!(
            std::net::TcpStream::connect_timeout(&address, Duration::from_millis(200)).is_err()
        );
    }

    #[tokio::test]
    async fn a_socket_that_ignores_the_stop_is_aborted_after_the_drain() {
        let bound = server().await;
        let address = bound.local_addr();
        let sockets = Sockets::new();
        let upgrades = sockets.upgrades.clone();
        let (stop, stopped) = oneshot::channel();
        let running = tokio::spawn(bound.serve(upgrading(), CONNECTION, Some(sockets), async {
            let _ = stopped.await;
        }));
        upgrades.stubborn();
        tokio::task::yield_now().await;
        stop.send(()).unwrap();
        let started = std::time::Instant::now();
        tokio::time::timeout(Duration::from_secs(6), running)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert!(started.elapsed() >= Duration::from_secs(3));
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
