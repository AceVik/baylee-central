//! One bounded background sender; shutdown waits for its in-flight HTTPS call.
use central_postgres::{AccountStore, PushResult};
use central_push::{Outcome, Sender};
use std::{sync::Arc, time::Duration};
use tokio::sync::watch;

pub(super) async fn run(
    store: AccountStore,
    sender: Option<Arc<Sender>>,
    mut stop: watch::Receiver<bool>,
) {
    let Some(sender) = sender else {
        let _ = stop.wait_for(|value| *value).await;
        return;
    };
    while !*stop.borrow() {
        tokio::select! {
            _=stop.wait_for(|value| *value)=>break,
            ()=tokio::time::sleep(Duration::from_millis(500))=>{}
        }
        let job = match store.claim_push().await {
            Ok(Some(job)) => job,
            Ok(None) => continue,
            Err(_) => {
                tokio::time::sleep(Duration::from_secs(2)).await;
                continue;
            }
        };
        let subscription = job.subscription.clone();
        let category = job.category;
        let sender = sender.clone();
        // Exactly one blocking HTTP operation at a time, with a ten-second total
        // timeout. Await even during shutdown: do not abandon owned work.
        let result =
            tokio::task::spawn_blocking(move || sender.send(&subscription, category)).await;
        let result = match result {
            Ok(Ok(Outcome::Delivered)) => PushResult::Delivered,
            Ok(Ok(Outcome::Gone)) => PushResult::Gone,
            Ok(Err(_) | Ok(Outcome::Retry)) | Err(_) => PushResult::Retry,
        };
        if store.finish_push(&job, result).await.is_err() {
            eprintln!("central: notification delivery acknowledgement unavailable");
        }
    }
}
