//! Bounded own-account export; a session change invalidates the whole collection.
use super::super::{
    central::{Failure, call},
    session,
};
use central_api::wire;

pub(super) async fn receipts() -> Result<serde_json::Value, Failure> {
    let stamp = session::login_stamp().ok_or(Failure::Rejected)?;
    let mut after = None;
    let mut rows = Vec::new();
    let mut progress = None;
    for _ in 0..60 {
        let wire::Response::TowerAwardExport { export } =
            call(&wire::Request::TowerAwardExport { after }).await?
        else {
            return Err(Failure::Malformed);
        };
        if session::login_stamp().as_ref() != Some(&stamp) {
            return Err(Failure::Rejected);
        }
        export.validate(after).map_err(|_| Failure::Malformed)?;
        progress = Some(export.progress);
        rows.extend(export.receipts);
        after = export.next;
        if after.is_none() {
            break;
        }
    }
    Ok(
        serde_json::json!({"progress":progress,"receipts":rows,"complete":after.is_none(),"after":after}),
    )
}
