//! Merge separately paginated challenge metadata into the account's downloadable file.
use super::{Failure, api, request};
mod tower;

/// The browser bounds the download to 7680 rows per dataset. The file explicitly marks
/// an incomplete dataset, matching the account export's existing bounded-list convention.
pub(crate) async fn merged(file: &str) -> Result<String, Failure> {
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
    serde_json::to_string_pretty(&file).map_err(|_| Failure::Malformed)
}
