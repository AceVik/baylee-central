//! Bounded personal expedition export; cursors must advance through owned run identities.
use crate::islands::{central::Failure, tower::request};
use central_api::tower as api;

pub(super) async fn runs() -> Result<serde_json::Value, Failure> {
    let mut datasets = serde_json::Map::new();
    let mut complete = true;
    for (table, name) in [
        (api::ExportTable::Runs, "runs"),
        (api::ExportTable::Actions, "actions"),
        (api::ExportTable::Participation, "participation"),
    ] {
        let (entries, after) = dataset(table).await?;
        complete &= after.is_none();
        datasets.insert(
            name.into(),
            serde_json::json!({ "entries": entries, "complete": after.is_none(), "after": after }),
        );
    }
    datasets.insert("complete".into(), complete.into());
    Ok(datasets.into())
}

async fn dataset(
    table: api::ExportTable,
) -> Result<(Vec<api::ExportEntry>, Option<[u8; 16]>), Failure> {
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
        if entries.iter().any(|entry| entry.identity().0 != table) {
            return Err(Failure::Malformed);
        }
        let ids: Vec<_> = entries.iter().map(|entry| entry.identity().1).collect();
        cursor(after, &ids, next)?;
        for entry in &entries {
            if let api::ExportEntry::Run { run, .. } = entry {
                run.progress.validate().map_err(|_| Failure::Malformed)?;
                run.season
                    .theme
                    .validate()
                    .map_err(|_| Failure::Malformed)?;
            }
        }
        all.extend(entries);
        after = next;
        if after.is_none() {
            break;
        }
    }
    Ok((all, after))
}

fn cursor(
    previous: Option<[u8; 16]>,
    ids: &[[u8; 16]],
    next: Option<[u8; 16]>,
) -> Result<(), Failure> {
    if ids.len() > 10 {
        return Err(Failure::Malformed);
    }
    let mut last = previous;
    for id in ids {
        if id[6] >> 4 != 7 || id[8] >> 6 != 2 || last.is_some_and(|last| *id <= last) {
            return Err(Failure::Malformed);
        }
        last = Some(*id);
    }
    if next.is_some() && (ids.len() != 10 || next != last) {
        return Err(Failure::Malformed);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::cursor;

    #[test]
    fn export_rejects_repeated_backward_partial_and_malformed_cursors() {
        let ids: Vec<_> = (1..=11)
            .map(|n| {
                let mut id = [0; 16];
                id[6] = 0x70;
                id[8] = 0x80;
                id[15] = n;
                id
            })
            .collect();
        assert!(cursor(None, &[], None).is_ok());
        assert!(cursor(None, &ids[..10], Some(ids[9])).is_ok());
        assert!(cursor(Some(ids[9]), &ids[10..], None).is_ok());
        assert!(cursor(None, &ids, None).is_err());
        assert!(cursor(Some(ids[1]), &ids[..2], None).is_err());
        assert!(cursor(None, &[ids[0], ids[0]], None).is_err());
        assert!(cursor(None, &ids[..9], Some(ids[8])).is_err());
        assert!(cursor(None, &ids[..10], Some(ids[10])).is_err());
        assert!(cursor(None, &[[0; 16]], None).is_err());
    }
}
