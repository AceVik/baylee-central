//! Export retains exact consent, retry records and the frozen competitive prefix.
use super::*;

async fn page(
    store: &AccountStore,
    token: &Token,
    table: tower::ExportTable,
    after: Option<[u8; 16]>,
) -> (Vec<tower::ExportEntry>, Option<[u8; 16]>) {
    let tower::Response::Export { entries, after } =
        call(store, Some(token), tower::Request::Export { table, after })
            .await
            .unwrap()
    else {
        panic!()
    };
    (entries, after)
}

pub(super) async fn privacy(store: &AccountStore, a: &Token, b: &Token, season: [u8; 16]) {
    let mut after = None;
    let mut ids = std::collections::BTreeSet::new();
    let mut pages = 0;
    loop {
        let (entries, next) = page(store, a, tower::ExportTable::Actions, after).await;
        assert!(entries.len() <= 10);
        for entry in entries {
            let tower::ExportEntry::Action {
                request_id,
                request,
                created_at_micros,
                ..
            } = entry
            else {
                panic!()
            };
            assert!(created_at_micros > 0);
            assert!(after.is_none_or(|prior| request_id > prior));
            assert!(ids.insert(request_id));
            let (tower::Request::Begin {
                request_id: exact, ..
            }
            | tower::Request::Start {
                request_id: exact, ..
            }
            | tower::Request::Choose {
                request_id: exact, ..
            }) = *request
            else {
                panic!("only accepted retry-safe operations are retained")
            };
            assert_eq!(exact, request_id);
        }
        pages += 1;
        after = next;
        if after.is_none() {
            break;
        }
        assert!(pages < 10);
    }
    assert!(pages > 1, "exercise the real action cursor");
    assert!(
        page(store, b, tower::ExportTable::Actions, None)
            .await
            .0
            .is_empty()
    );
    let (entries, after) = page(store, a, tower::ExportTable::Participation, None).await;
    assert!(after.is_none());
    assert!(
        matches!(&entries[..], [tower::ExportEntry::Participation { season: own, listed: false, accomplishments, .. }] if *own == season && accomplishments.len() == 5)
    );
    call(
        store,
        Some(b),
        tower::Request::Visibility {
            season,
            listed: true,
        },
    )
    .await
    .unwrap();
    let (entries, after) = page(store, b, tower::ExportTable::Participation, None).await;
    assert!(after.is_none());
    assert!(
        matches!(&entries[..], [tower::ExportEntry::Participation { season: own, listed: true, accomplishments, .. }] if *own == season && accomplishments.is_empty())
    );
    assert!(
        page(store, b, tower::ExportTable::Runs, None)
            .await
            .0
            .is_empty(),
        "runless consent is exported independently"
    );
    assert!(
        call(
            store,
            None,
            tower::Request::Export {
                table: tower::ExportTable::Actions,
                after: None
            }
        )
        .await
        .is_err()
    );
}

pub(super) async fn frozen_prefix(store: &AccountStore, token: &Token, run_id: [u8; 16]) {
    let (entries, after) = page(store, token, tower::ExportTable::Runs, None).await;
    assert!(after.is_none());
    let [
        tower::ExportEntry::Run {
            run,
            ranked: Some(ranked),
            request_id,
            created_at_micros,
        },
    ] = entries.as_slice()
    else {
        panic!()
    };
    assert_eq!(run.id, run_id);
    assert_eq!(run.progress.cleared, 2);
    assert!(!run.progress.competitive);
    assert_eq!(
        ranked.floors, 1,
        "export preserves the actual ranked prefix"
    );
    assert_eq!(request_id[6] >> 4, 7);
    assert!(*created_at_micros > 0);
    let (entries, _) = page(store, token, tower::ExportTable::Participation, None).await;
    assert!(
        matches!(&entries[..], [tower::ExportEntry::Participation { accomplishments, .. }] if accomplishments.iter().map(|record| record.floor).collect::<Vec<_>>() == [1,2])
    );
}
