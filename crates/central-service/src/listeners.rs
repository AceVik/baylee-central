//! The account listener and, when configured, the browser listener, run
//! together: one signal stops both, and either one stopping stops the other.
use std::future::Future;
use tokio::sync::watch;

/// Resolves once `stop` says so.
pub(super) fn stopped(stop: &watch::Sender<bool>) -> impl Future<Output = ()> + use<> {
    let mut stopping = stop.subscribe();
    async move {
        let _ = stopping.wait_for(|stopped| *stopped).await;
    }
}

/// Run the account listener and, when configured, the browser listener until
/// `signal` resolves or either listener stops; then stop the other one too.
/// Both are built to stop on [`stopped`] of the same `stop`.
pub(super) async fn together(
    stop: &watch::Sender<bool>,
    native: impl Future<Output = std::io::Result<()>>,
    browser: Option<impl Future<Output = std::io::Result<()>>>,
    signal: impl Future<Output = ()>,
) -> std::io::Result<()> {
    let native = async {
        let result = native.await;
        stop.send_replace(true);
        result
    };
    let browser = async {
        let Some(browser) = browser else {
            return Ok(());
        };
        let result = browser.await;
        stop.send_replace(true);
        result
    };
    let either = stopped(stop);
    let signal = async {
        tokio::select! {
            () = signal => {}
            () = either => {}
        }
        stop.send_replace(true);
    };
    let ((), native, browser) = tokio::join!(signal, native, browser);
    native.and(browser)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        sync::atomic::{AtomicBool, Ordering},
        time::Duration,
    };
    use tokio::time::timeout;

    fn listener(
        stop: &watch::Sender<bool>,
        done: &AtomicBool,
    ) -> impl Future<Output = std::io::Result<()>> {
        let stopping = stopped(stop);
        async move {
            stopping.await;
            done.store(true, Ordering::SeqCst);
            Ok(())
        }
    }

    #[tokio::test]
    async fn a_signal_stops_both_listeners() {
        let (stop, _) = watch::channel(false);
        let (native, browser) = (AtomicBool::new(false), AtomicBool::new(false));
        let signalled = AtomicBool::new(false);
        let signal = async {
            tokio::task::yield_now().await;
            signalled.store(true, Ordering::SeqCst);
        };
        let result = timeout(
            Duration::from_secs(5),
            together(
                &stop,
                listener(&stop, &native),
                Some(listener(&stop, &browser)),
                signal,
            ),
        )
        .await
        .unwrap();
        assert!(result.is_ok());
        assert!(signalled.load(Ordering::SeqCst));
        assert!(native.load(Ordering::SeqCst) && browser.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn a_listener_that_fails_stops_the_other() {
        for browser_fails in [false, true] {
            let (stop, _) = watch::channel(false);
            let other = AtomicBool::new(false);
            let failed = async { Err(std::io::Error::other("synthetic listener failure")) };
            let result = if browser_fails {
                timeout(
                    Duration::from_secs(5),
                    together(
                        &stop,
                        listener(&stop, &other),
                        Some(failed),
                        std::future::pending(),
                    ),
                )
                .await
            } else {
                timeout(
                    Duration::from_secs(5),
                    together(
                        &stop,
                        failed,
                        Some(listener(&stop, &other)),
                        std::future::pending(),
                    ),
                )
                .await
            };
            assert!(result.unwrap().is_err(), "{browser_fails}");
            assert!(other.load(Ordering::SeqCst), "{browser_fails}");
        }
    }

    #[tokio::test]
    async fn without_a_browser_listener_the_account_listener_waits_for_the_signal() {
        let (stop, _) = watch::channel(false);
        let native = AtomicBool::new(false);
        let (send, receive) = tokio::sync::oneshot::channel::<()>();
        let signal = async {
            let _ = receive.await;
        };
        let running = together(
            &stop,
            listener(&stop, &native),
            None::<std::future::Ready<std::io::Result<()>>>,
            signal,
        );
        tokio::pin!(running);
        assert!(
            timeout(Duration::from_millis(50), &mut running)
                .await
                .is_err()
        );
        assert!(
            !native.load(Ordering::SeqCst),
            "no browser listener is no stop"
        );
        send.send(()).unwrap();
        timeout(Duration::from_secs(5), running)
            .await
            .unwrap()
            .unwrap();
        assert!(native.load(Ordering::SeqCst));
    }
}
