//! The account's own ranked rows: every dataset paged to its end or to
//! [`export::DOWNLOAD_ROWS`], each page checked before its key is followed.
use crate::islands::central::{Failure, call};
use central_api::{
    ranked::{self as api, export},
    wire,
};

async fn page(
    table: export::Table,
    after: Option<export::Key>,
) -> Result<(Vec<export::Entry>, Option<export::Key>), Failure> {
    match call(&wire::Request::Ranked {
        request: api::Request::Export { table, after },
    })
    .await?
    {
        wire::Response::Ranked {
            response: api::Response::Export { entries, after },
        } => Ok((entries, after)),
        _ => Err(Failure::Malformed),
    }
}

/// `{"seasons": {...}, ..., "complete": bool}`, each dataset as
/// `{"entries", "complete", "after"}` like the other exports.
pub(super) async fn datasets() -> Result<serde_json::Value, Failure> {
    let mut datasets = serde_json::Map::new();
    let mut complete = true;
    for table in export::Table::ALL {
        let mut after = None;
        let mut all = Vec::new();
        for _ in 0..export::DOWNLOAD_ROWS / table.page_size() {
            let (entries, next) = page(table, after).await?;
            export::validate_page(table, after, &entries, next).map_err(|_| Failure::Malformed)?;
            all.extend(entries);
            after = next;
            if after.is_none() {
                break;
            }
        }
        complete &= after.is_none();
        datasets.insert(
            table.as_str().into(),
            serde_json::json!({ "entries": all, "complete": after.is_none(), "after": after }),
        );
    }
    datasets.insert("complete".into(), complete.into());
    Ok(datasets.into())
}
