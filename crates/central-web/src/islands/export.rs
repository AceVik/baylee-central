//! Merge separately paginated challenge metadata into the account's downloadable file.
use super::{
    central::{Failure, call},
    session,
};
use central_api::{challenges as api, wire};
mod echoes;
pub(super) mod tower;

pub(super) async fn request(request: api::Request) -> Result<api::Response, Failure> {
    match call(&wire::Request::Challenges { request }).await? {
        wire::Response::Challenges { response } => Ok(response),
        _ => Err(Failure::Malformed),
    }
}

/// The browser bounds the download to 7680 rows per dataset. The file explicitly marks
/// an incomplete dataset, matching the account export's existing bounded-list convention.
pub(crate) async fn merged(file: &str) -> Result<String, Failure> {
    let stamp = session::marker().ok_or(Failure::Rejected)?;
    let mut file: serde_json::Value = serde_json::from_str(file).map_err(|_| Failure::Malformed)?;
    let mut datasets = serde_json::Map::new();
    for (table, name) in [
        (api::ExportTable::Participation, "participation"),
        (api::ExportTable::Attempts, "attempts"),
    ] {
        let mut after = None;
        let mut all = Vec::new();
        for _ in 0..256 {
            let api::Response::Export {
                entries,
                after: next,
            } = request(api::Request::Export { table, after }).await?
            else {
                return Err(Failure::Malformed);
            };
            api::validate_export_page(table, after, &entries, next)
                .map_err(|_| Failure::Malformed)?;
            all.extend(entries);
            after = next;
            if after.is_none() {
                break;
            }
        }
        datasets.insert(
            name.into(),
            serde_json::json!({ "entries": all, "complete": after.is_none(), "after": after }),
        );
    }
    file.as_object_mut()
        .ok_or(Failure::Malformed)?
        .insert("challenges".into(), datasets.into());
    file.as_object_mut()
        .ok_or(Failure::Malformed)?
        .insert("tower".into(), tower::runs().await?);
    file.as_object_mut()
        .ok_or(Failure::Malformed)?
        .insert("tower_echoes".into(), echoes::receipts().await?);
    if session::marker().as_ref() != Some(&stamp) {
        return Err(Failure::Rejected);
    }
    serde_json::to_string_pretty(&file).map_err(|_| Failure::Malformed)
}
