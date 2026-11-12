//! Central's mail (`docs/email.md`): the optional `mail` block of a
//! service's configuration, and the one worker that does the mail work the
//! requests queue. Central speaks no SMTP: the mailer is a separate service
//! on this host, reached over loopback HTTP and nowhere else.
use central_identity::{Delivery, IdentityService};
use central_mail::{Client, Token};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::watch;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// The mailer on this host, the site its mails link to and Central's token
/// for the mailer. It arrives with the other private configuration, over
/// standard input. Absent, Central sends no mail; no address is assumed.
#[derive(Deserialize, Zeroize, ZeroizeOnDrop)]
#[serde(deny_unknown_fields)]
pub(super) struct Mail {
    /// `http://`, a numeric loopback address and a port, nothing after it.
    endpoint: String,
    /// The site's origin, which the mails' two links start with.
    site: String,
    /// Central's bearer token for the mailer.
    token: String,
}

impl Mail {
    /// The mailer's client; nothing is sent to make it. The token is moved
    /// out, not copied. `development` takes a numeric loopback HTTP origin
    /// as the site instead of a canonical HTTPS one.
    ///
    /// Refuses an endpoint that is not loopback HTTP, a site that is no
    /// such origin and a token that is no bearer token of 32 to 256
    /// characters, so that a service configured wrongly does not start.
    pub(super) fn client(&mut self, development: bool) -> Result<Arc<Client>, ()> {
        let token = Token::new(std::mem::take(&mut self.token)).map_err(|_| ())?;
        Client::new(&self.endpoint, &self.site, token, development)
            .map(Arc::new)
            .map_err(|_| ())
    }
}

/// What the worker asks of the identity service; a test puts a queue of
/// its own in its place.
trait Queue {
    /// One piece of the work that waits, to its end; `None` when none waits.
    fn deliver(&self) -> impl Future<Output = Option<Delivery>>;
    /// Resolves once work was queued.
    fn queued(&self) -> impl Future<Output = ()>;
}

impl Queue for IdentityService {
    fn deliver(&self) -> impl Future<Output = Option<Delivery>> {
        self.deliver_mail()
    }

    fn queued(&self) -> impl Future<Output = ()> {
        self.mail_queued()
    }
}

/// Do the mail work `identity` queues, one piece at a time, until `stop`
/// says so. A piece in hand is finished first: a reset code that was made
/// is mailed or taken back, never left. What still waits then is dropped:
/// a request whose code was not made yet, a notice not sent yet.
///
/// The log says that a mail was not handed over and nothing else: no
/// address, no name, not which mail. Without a mailer nothing is ever
/// queued, and this only waits for the stop.
pub(super) async fn run(identity: Arc<IdentityService>, stop: watch::Receiver<bool>) {
    work(identity.as_ref(), stop).await;
}

async fn work(queue: &impl Queue, mut stop: watch::Receiver<bool>) {
    while !*stop.borrow() {
        match queue.deliver().await {
            Some(Delivery::Failed) => {
                eprintln!("central: a mail was not handed to the mailer");
            }
            Some(Delivery::Sent | Delivery::Nothing) => {}
            // Only now is there nothing to do: everything queued before
            // this wait was worked on, however often the queue was told.
            None => {
                tokio::select! {
                    _ = stop.wait_for(|value| *value) => break,
                    () = queue.queued() => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        collections::VecDeque,
        sync::{
            Mutex,
            atomic::{AtomicUsize, Ordering},
        },
        time::Duration,
    };
    use tokio::{
        sync::{Notify, Semaphore},
        time::timeout,
    };

    /// A token as the operator generates one: 64 hexadecimal digits.
    const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn mail(endpoint: &str, site: &str, token: &str) -> Mail {
        serde_json::from_str(&format!(
            r#"{{"endpoint":"{endpoint}","site":"{site}","token":"{token}"}}"#
        ))
        .unwrap()
    }

    #[test]
    fn a_mail_block_yields_a_client_and_gives_its_token_away() {
        let mut live = mail("http://127.0.0.1:28795", "https://baylee.app", TOKEN);
        assert!(live.client(false).is_ok());
        assert_eq!(live.token, "", "moved out, not copied");
        let mut local = mail("http://[::1]:28795", "http://127.0.0.1:5173", TOKEN);
        assert!(local.client(true).is_ok());
    }

    #[test]
    fn a_mailer_anywhere_but_loopback_http_is_refused_at_start() {
        for endpoint in [
            "https://127.0.0.1:28795",
            "http://localhost:28795",
            "http://192.168.1.2:28795",
            "http://10.0.0.1:28795",
            "http://mailer.example.org:28795",
            "http://127.0.0.1",
            "http://127.0.0.1:0",
            "http://127.0.0.1:28795/",
            "http://127.0.0.1:28795/v1/send",
            "http://token@127.0.0.1:28795",
            "127.0.0.1:28795",
            "",
        ] {
            let mut block = mail(endpoint, "https://baylee.app", TOKEN);
            assert!(block.client(false).is_err(), "{endpoint}");
        }
    }

    #[test]
    fn the_site_is_an_origin_of_the_kind_the_service_runs_as() {
        for (site, development) in [
            ("http://baylee.app", false),
            ("https://baylee.app/", false),
            ("https://baylee.app/en", false),
            ("https://user@baylee.app", false),
            ("http://127.0.0.1:5173", false),
            ("https://baylee.app", true),
            ("http://localhost:5173", true),
            ("", false),
        ] {
            let mut block = mail("http://127.0.0.1:28795", site, TOKEN);
            assert!(block.client(development).is_err(), "{site}");
        }
    }

    #[test]
    fn a_token_that_is_no_bearer_token_is_refused_at_start() {
        let long = "a".repeat(257);
        for token in [
            "",
            "short",
            "0123456789abcdef 123456789abcdef0",
            long.as_str(),
        ] {
            let mut block = mail("http://127.0.0.1:28795", "https://baylee.app", token);
            assert!(block.client(false).is_err());
        }
    }

    #[test]
    fn a_mail_block_takes_its_three_fields_and_no_other() {
        let parse = |text: &str| serde_json::from_str::<Mail>(text);
        for text in [
            r#"{"endpoint":"http://127.0.0.1:28795","site":"https://baylee.app"}"#,
            r#"{"endpoint":"http://127.0.0.1:28795","token":"x"}"#,
            r#"{"site":"https://baylee.app","token":"x"}"#,
            r#"{"endpoint":"http://127.0.0.1:28795","site":"https://baylee.app","token":"x","from":"central@example.org"}"#,
            r#""http://127.0.0.1:28795""#,
            "{}",
        ] {
            assert!(parse(text).is_err(), "{text}");
        }
    }

    /// A queue of the test's own: what waits, how much was done, and a
    /// gate each piece waits at while it is in hand.
    struct Fake {
        waiting: Mutex<VecDeque<Delivery>>,
        told: Notify,
        done: AtomicUsize,
        in_hand: Notify,
        gate: Semaphore,
    }

    impl Fake {
        fn holding(work: &[Delivery], open: usize) -> Arc<Self> {
            Arc::new(Self {
                waiting: Mutex::new(work.iter().copied().collect()),
                told: Notify::new(),
                done: AtomicUsize::new(0),
                in_hand: Notify::new(),
                gate: Semaphore::new(open),
            })
        }
    }

    impl Queue for Fake {
        async fn deliver(&self) -> Option<Delivery> {
            let next = self.waiting.lock().unwrap().pop_front()?;
            self.in_hand.notify_one();
            self.gate.acquire().await.unwrap().forget();
            self.done.fetch_add(1, Ordering::SeqCst);
            Some(next)
        }

        async fn queued(&self) {
            self.told.notified().await;
        }
    }

    const SOON: Duration = Duration::from_secs(5);

    #[tokio::test]
    async fn everything_queued_before_a_wait_is_worked_on_though_told_once() {
        // Three pieces and one word of them: the queue keeps one word at
        // most, so the worker must not wait while work is left.
        let queue = Fake::holding(&[Delivery::Sent, Delivery::Failed, Delivery::Nothing], 8);
        queue.told.notify_one();
        queue.told.notify_one();
        let (stop, stopping) = watch::channel(false);
        let worker = tokio::spawn({
            let queue = queue.clone();
            async move { work(queue.as_ref(), stopping).await }
        });
        timeout(SOON, async {
            while queue.done.load(Ordering::SeqCst) < 3 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        // Work queued while the worker waits is taken up when it is told.
        queue.waiting.lock().unwrap().push_back(Delivery::Sent);
        queue.told.notify_one();
        timeout(SOON, async {
            while queue.done.load(Ordering::SeqCst) < 4 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        stop.send_replace(true);
        timeout(SOON, worker).await.unwrap().unwrap();
        assert!(queue.waiting.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn a_stop_finishes_the_piece_in_hand_and_leaves_the_rest() {
        let queue = Fake::holding(&[Delivery::Sent, Delivery::Sent], 0);
        let (stop, stopping) = watch::channel(false);
        let mut worker = tokio::spawn({
            let queue = queue.clone();
            async move { work(queue.as_ref(), stopping).await }
        });
        timeout(SOON, queue.in_hand.notified()).await.unwrap();
        stop.send_replace(true);
        // The piece in hand is not abandoned: the worker is still there.
        assert!(
            timeout(Duration::from_millis(100), &mut worker)
                .await
                .is_err()
        );
        assert_eq!(queue.done.load(Ordering::SeqCst), 0);
        queue.gate.add_permits(8);
        timeout(SOON, worker).await.unwrap().unwrap();
        assert_eq!(queue.done.load(Ordering::SeqCst), 1);
        assert_eq!(queue.waiting.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn a_worker_without_work_waits_for_the_stop_and_no_longer() {
        let queue = Fake::holding(&[], 8);
        let (stop, stopping) = watch::channel(false);
        let mut worker = tokio::spawn({
            let queue = queue.clone();
            async move { work(queue.as_ref(), stopping).await }
        });
        assert!(
            timeout(Duration::from_millis(100), &mut worker)
                .await
                .is_err()
        );
        stop.send_replace(true);
        timeout(SOON, worker).await.unwrap().unwrap();
        // A worker started after the stop does nothing at all.
        let late = Fake::holding(&[Delivery::Sent], 8);
        timeout(SOON, work(late.as_ref(), stop.subscribe()))
            .await
            .unwrap();
        assert_eq!(late.done.load(Ordering::SeqCst), 0);
    }
}
