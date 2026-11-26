//! The operator's modes after a restore of an older backup
//! (`docs/operations.md` §"After a restore"): `--after-restore-stdin` moves
//! every gateway's revocation stream past each number issued before, and
//! `--erase-accounts-stdin` erases again, by id, the accounts deleted since
//! the backup. Both run under the migrator login, from the document the
//! operator keeps, and answer with one JSON line on standard output and the
//! exit status 0, or a line on standard error and 1. Neither prints an id.
use super::production::{Database, parse, read_settings};
use central_postgres::{
    ERASE_BATCH, Erased, administrators, connect_server, erase_account, step_revocations,
};
use serde::Deserialize;
use std::{collections::BTreeSet, process::ExitCode};
use zeroize::{Zeroize, ZeroizeOnDrop};

/// `--after-restore-stdin`: the migration document as it is.
#[derive(Deserialize, Zeroize, ZeroizeOnDrop)]
#[serde(deny_unknown_fields)]
struct AfterRestore {
    database: Database,
}

/// `--erase-accounts-stdin`: the migration document and the ids to erase.
#[derive(Deserialize, Zeroize, ZeroizeOnDrop)]
#[serde(deny_unknown_fields)]
struct Erase {
    database: Database,
    accounts: Vec<String>,
}

/// Every gateway's revocation stream moves past the numbers issued before the
/// restore, so each gateway ends the Central sessions it holds.
pub(super) async fn run_after_restore() -> ExitCode {
    if let Ok(streams) = stepped().await {
        println!("{{\"streams\":{streams}}}");
        eprintln!(
            "central: {streams} revocation streams moved past every number issued before; each gateway ends the Central sessions it holds at its next catch-up"
        );
        ExitCode::SUCCESS
    } else {
        eprintln!(
            "central: the revocation streams were not moved: configuration rejected or failed"
        );
        ExitCode::FAILURE
    }
}

async fn stepped() -> Result<u64, ()> {
    let bytes = read_settings()?;
    let mut settings: AfterRestore = parse(&bytes)?;
    drop(bytes);
    let database = settings.database.server()?;
    drop(settings);
    let db = connect_server(database).await.map_err(|_| ())?;
    let moved = step_revocations(&db, central_api::OFFICIAL_ORIGIN)
        .await
        .map_err(|_| ());
    let closed = db.close().await.map_err(|_| ());
    let moved = moved?;
    closed?;
    Ok(moved)
}

/// What one run of `--erase-accounts-stdin` did.
#[derive(Default)]
struct Erasure {
    erased: u64,
    absent: u64,
}

/// Erase the listed accounts, each in its own transaction; one that is gone
/// already counts as absent. Stopped by a failure, what was erased stays
/// erased, and the same list runs again.
pub(super) async fn run_erase() -> ExitCode {
    let mut done = Erasure::default();
    let Ok(left) = erase(&mut done).await else {
        eprintln!(
            "central: erasure rejected its input or failed after {} accounts erased and {} not found; run the same list again",
            done.erased, done.absent
        );
        return ExitCode::FAILURE;
    };
    println!(
        "{{\"erased\":{},\"absent\":{},\"administrators\":{left}}}",
        done.erased, done.absent
    );
    eprintln!(
        "central: {} accounts erased, {} not found",
        done.erased, done.absent
    );
    if left == 0 {
        eprintln!("central: no administrator is left; appoint one (--appoint-admin-stdin)");
    }
    ExitCode::SUCCESS
}

async fn erase(done: &mut Erasure) -> Result<u64, ()> {
    let bytes = read_settings()?;
    let mut settings: Erase = parse(&bytes)?;
    drop(bytes);
    // The whole list is checked before anything is erased.
    let accounts = ids(&settings.accounts)?;
    let database = settings.database.server()?;
    drop(settings);
    let db = connect_server(database).await.map_err(|_| ())?;
    let mut result = Ok(());
    for account in accounts {
        match erase_account(&db, account).await {
            Ok(Erased::Erased) => done.erased += 1,
            Ok(Erased::Absent) => done.absent += 1,
            Err(_) => {
                result = Err(());
                break;
            }
        }
    }
    let left = match result {
        Ok(()) => administrators(&db).await.map_err(|_| ()),
        Err(()) => Err(()),
    };
    let closed = db.close().await.map_err(|_| ());
    let left = left?;
    closed?;
    Ok(left)
}

/// The listed ids, each once: 1 to [`ERASE_BATCH`] of them, each a UUID in
/// its hyphenated text form, as PostgreSQL writes one, in either case.
fn ids(listed: &[String]) -> Result<BTreeSet<[u8; 16]>, ()> {
    if listed.is_empty() || listed.len() > ERASE_BATCH {
        return Err(());
    }
    listed.iter().map(|text| id(text)).collect()
}

/// One UUID in its hyphenated text form: 36 characters, the hyphens at 8,
/// 13, 18 and 23, hex digits elsewhere.
fn id(text: &str) -> Result<[u8; 16], ()> {
    let text = text.as_bytes();
    if text.len() != 36 {
        return Err(());
    }
    let mut digits = [0_u8; 32];
    let mut count = 0;
    for (at, &byte) in text.iter().enumerate() {
        if matches!(at, 8 | 13 | 18 | 23) {
            if byte != b'-' {
                return Err(());
            }
            continue;
        }
        digits[count] = char::from(byte)
            .to_digit(16)
            .and_then(|digit| u8::try_from(digit).ok())
            .ok_or(())?;
        count += 1;
    }
    let mut id = [0_u8; 16];
    let (pairs, _) = digits.as_chunks::<2>();
    for (byte, pair) in id.iter_mut().zip(pairs) {
        *byte = pair[0] << 4 | pair[1];
    }
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(ids: &[&str]) -> Vec<String> {
        ids.iter().map(|&id| id.to_owned()).collect()
    }

    #[test]
    fn ids_are_hyphenated_uuids_each_counted_once() {
        let one = "0193a5b2-7c41-7d3e-9f00-0123456789ab";
        let set = ids(&text(&[one, &one.to_ascii_uppercase(), one])).unwrap();
        assert_eq!(set.len(), 1);
        assert_eq!(
            set.first().copied(),
            Some([
                0x01, 0x93, 0xa5, 0xb2, 0x7c, 0x41, 0x7d, 0x3e, 0x9f, 0x00, 0x01, 0x23, 0x45, 0x67,
                0x89, 0xab
            ])
        );
    }

    #[test]
    fn a_list_out_of_bounds_or_one_bad_id_refuses_all() {
        assert!(ids(&[]).is_err());
        let full: Vec<String> = (0..=ERASE_BATCH)
            .map(|n| format!("00000000-0000-7000-8000-{n:012x}"))
            .collect();
        assert!(ids(&full[..ERASE_BATCH]).is_ok());
        assert!(ids(&full).is_err());
        let good = "0193a5b2-7c41-7d3e-9f00-0123456789ab";
        for bad in [
            "",
            "0193a5b27c417d3e9f000123456789ab",
            "{0193a5b2-7c41-7d3e-9f00-0123456789ab}",
            "urn:uuid:0193a5b2-7c41-7d3e-9f00-0123456789ab",
            " 0193a5b2-7c41-7d3e-9f00-0123456789ab",
            "0193a5b2-7c41-7d3e-9f00-0123456789ag",
            "0193a5b2-7c41-7d3e-9f00-0123456789abc",
            "0193a5b2+7c41-7d3e-9f00-0123456789ab",
            "0193a5b2-7c41-7d3e-9f00-0123456789a\u{e9}",
            "+193a5b2-7c41-7d3e-9f00-0123456789ab",
        ] {
            assert!(ids(&text(&[good, bad])).is_err(), "{bad:?}");
        }
    }
}
