//! Migration 000032's donations against real PostgreSQL: a delivery kept once
//! however often it comes, the bounds, conversion of a row that came without a
//! rate, Ko-fi's window, what was given once with refunds taken off, the day the
//! count began, GitHub's window, the sweep's fold, and the runtime's grants both
//! ways. Synthetic amounts and references only.
use super::{AccountStore, Error, config};
use central_postgres::donations::{
    DONATION_DAYS, Donation, DonationKind, DonationSums, KOFI_WINDOW_DAYS, MOST_AMOUNT,
    MOST_EURO_CENTS, Platform, Recorded,
};
use central_postgres::sweep;
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};

fn reference(n: u8) -> [u8; 32] {
    let mut reference = [0x5a; 32];
    reference[0] = n;
    reference
}

fn kofi(n: u8, kind: DonationKind, euro_cents: Option<u64>) -> Donation<'static> {
    Donation {
        platform: Platform::Kofi,
        kind,
        amount: euro_cents.unwrap_or(500),
        currency: if euro_cents.is_some() { "EUR" } else { "USD" },
        euro_cents,
        reference: reference(n),
    }
}

fn github(n: u8, kind: DonationKind, euro_cents: u64) -> Donation<'static> {
    Donation {
        platform: Platform::GitHub,
        kind,
        amount: euro_cents + 100,
        currency: "USD",
        euro_cents: Some(euro_cents),
        reference: reference(n),
    }
}

/// Moves a row's arrival `days` into the past, as the operator can.
async fn age(operator: &DatabaseConnection, n: u8, days: i32) {
    operator
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "UPDATE central_identity.donations
             SET received_at = received_at - pg_catalog.make_interval(days => $1)
             WHERE pg_catalog.get_byte(reference, 0) = $2",
            vec![days.into(), i32::from(n).into()],
        ))
        .await
        .unwrap();
}

async fn rows(operator: &DatabaseConnection) -> i64 {
    super::super::super::scalar(
        operator,
        "SELECT count(*)::bigint AS value FROM central_identity.donations",
    )
    .await
}

pub(super) async fn run(store: &AccountStore, operator: &DatabaseConnection) {
    let today = super::super::super::scalar(
        operator,
        "SELECT (pg_catalog.to_char(since, 'YYYYMMDD'))::bigint AS value
         FROM central_identity.donation_count",
    )
    .await;
    let since = format!(
        "{}-{:02}-{:02}",
        today / 10_000,
        today / 100 % 100,
        today % 100
    );
    // Nothing yet: zero, from the day the count began.
    assert_eq!(
        store.donation_sums().await.unwrap(),
        DonationSums {
            kofi_month: 0,
            kofi_payments: 0,
            once_total: 0,
            once_count: 0,
            since: since.clone(),
        }
    );
    once_only(store).await;
    bounds(store).await;
    conversion(store, operator).await;
    sums(store, operator, &since).await;
    window(store, operator, &since).await;
    folded(store, operator, &since).await;
    grants(operator).await;
}

/// A delivery that comes again is kept once, whatever it says the second time, and
/// one platform's reference is not another's.
async fn once_only(store: &AccountStore) {
    let first = kofi(1, DonationKind::Once, Some(300));
    assert_eq!(store.record_donation(&first).await, Ok(Recorded::New));
    assert_eq!(store.record_donation(&first).await, Ok(Recorded::Again));
    let changed = Donation {
        amount: 99_999,
        euro_cents: Some(99_999),
        kind: DonationKind::Recurring,
        ..first
    };
    assert_eq!(store.record_donation(&changed).await, Ok(Recorded::Again));
    let sums = store.donation_sums().await.unwrap();
    assert_eq!((sums.once_total, sums.once_count), (300, 1));
    assert_eq!((sums.kofi_month, sums.kofi_payments), (0, 0));
    // the same reference on the other platform is another donation
    let elsewhere = github(1, DonationKind::Once, 400);
    assert_eq!(store.record_donation(&elsewhere).await, Ok(Recorded::New));
    assert_eq!(store.record_donation(&elsewhere).await, Ok(Recorded::Again));
    // and many deliveries of one at once leave one row
    let racing: Vec<_> = (0..8)
        .map(|_| {
            let store = store.clone();
            tokio::spawn(async move {
                store
                    .record_donation(&kofi(2, DonationKind::Once, Some(200)))
                    .await
            })
        })
        .collect();
    let mut new = 0;
    for task in racing {
        if task.await.unwrap() == Ok(Recorded::New) {
            new += 1;
        }
    }
    assert_eq!(new, 1, "one delivery of many is the new one");
    let sums = store.donation_sums().await.unwrap();
    assert_eq!((sums.once_total, sums.once_count), (900, 3));
}

/// Amounts, currencies and kinds out of bounds never reach a row.
async fn bounds(store: &AccountStore) {
    let good = kofi(3, DonationKind::Once, Some(100));
    for refused in [
        Donation { amount: 0, ..good },
        Donation {
            amount: MOST_AMOUNT + 1,
            ..good
        },
        Donation {
            currency: "eur",
            ..good
        },
        Donation {
            currency: "EURO",
            ..good
        },
        Donation {
            currency: "",
            ..good
        },
        Donation {
            euro_cents: Some(MOST_EURO_CENTS + 1),
            ..good
        },
        Donation {
            kind: DonationKind::Refund,
            ..good
        },
        Donation {
            platform: Platform::GitHub,
            kind: DonationKind::Recurring,
            ..good
        },
    ] {
        assert_eq!(
            store.record_donation(&refused).await,
            Err(Error::Rejected),
            "{refused:?}"
        );
    }
    // the largest of each is taken
    let largest = Donation {
        amount: MOST_AMOUNT,
        euro_cents: Some(MOST_EURO_CENTS),
        ..good
    };
    assert_eq!(store.record_donation(&largest).await, Ok(Recorded::New));
    let sums = store.donation_sums().await.unwrap();
    assert_eq!(sums.once_total, 900 + MOST_EURO_CENTS);
}

/// A row that came without a rate counts for nothing until it is converted, at the
/// rate given and rounded as `central_funding` rounds; a week later it is no longer.
async fn conversion(store: &AccountStore, operator: &DatabaseConnection) {
    let before = store.donation_sums().await.unwrap();
    // five dollars, twice, one of them a week and a day old
    assert_eq!(
        store
            .record_donation(&kofi(4, DonationKind::Once, None))
            .await,
        Ok(Recorded::New)
    );
    assert_eq!(
        store
            .record_donation(&kofi(5, DonationKind::Once, None))
            .await,
        Ok(Recorded::New)
    );
    age(operator, 5, 8).await;
    assert_eq!(store.donation_sums().await.unwrap(), before);
    assert_eq!(
        store.unconverted_donations().await,
        Ok(vec![("USD".to_owned(), 2)])
    );
    for (currency, rate) in [("usd", 1_096_800), ("USD", 0)] {
        assert_eq!(
            store.convert_donations(currency, rate).await,
            Err(Error::Rejected)
        );
    }
    // another currency's rate converts nothing
    assert_eq!(store.convert_donations("GBP", 832_900).await, Ok(0));
    assert_eq!(store.convert_donations("USD", 1_096_800).await, Ok(1));
    let after = store.donation_sums().await.unwrap();
    assert_eq!(after.once_total, before.once_total + 456);
    assert_eq!(after.once_count, before.once_count + 1);
    // converted once only, and the old row stays without a rate
    assert_eq!(store.convert_donations("USD", 1).await, Ok(0));
    assert_eq!(
        store.unconverted_donations().await,
        Ok(vec![("USD".to_owned(), 1)])
    );
    // a rate that would make a row worth more than the bound leaves it as it is:
    // five dollars at a millionth of a dollar to the euro, and the largest amount
    // at par
    assert_eq!(
        store
            .record_donation(&kofi(6, DonationKind::Once, None))
            .await,
        Ok(Recorded::New)
    );
    assert_eq!(store.convert_donations("USD", 1).await, Ok(0));
    let huge = Donation {
        amount: MOST_AMOUNT,
        ..kofi(7, DonationKind::Once, None)
    };
    assert_eq!(store.record_donation(&huge).await, Ok(Recorded::New));
    // at par the five dollars are converted, the largest amount is not
    assert_eq!(store.convert_donations("USD", 1_000_000).await, Ok(1));
    assert_eq!(
        store.unconverted_donations().await,
        Ok(vec![("USD".to_owned(), 2)])
    );
    // the rows that never got a rate leave the store with the sweep
    operator
        .execute_unprepared(
            "DELETE FROM central_identity.donations WHERE euro_cents IS NULL
             OR pg_catalog.get_byte(reference, 0) = 6",
        )
        .await
        .unwrap();
    assert_eq!(store.donation_sums().await.unwrap(), after);
}

/// What was given once on both platforms, refunds taken off, and never below
/// nothing; Ko-fi's regular payments apart from it.
async fn sums(store: &AccountStore, operator: &DatabaseConnection, since: &str) {
    operator
        .execute_unprepared("DELETE FROM central_identity.donations")
        .await
        .unwrap();
    for donation in [
        github(10, DonationKind::Once, 2_280),
        github(11, DonationKind::Once, 912),
        github(12, DonationKind::Refund, 912),
        kofi(13, DonationKind::Once, Some(500)),
        kofi(14, DonationKind::Recurring, Some(300)),
        kofi(15, DonationKind::Recurring, Some(700)),
    ] {
        assert_eq!(store.record_donation(&donation).await, Ok(Recorded::New));
    }
    assert_eq!(
        store.donation_sums().await.unwrap(),
        DonationSums {
            kofi_month: 1_000,
            kofi_payments: 2,
            once_total: 2_780,
            once_count: 2,
            since: since.to_owned(),
        }
    );
    // a refund of a sponsorship from before the count began takes off no more
    // than there is
    for n in 20..24 {
        assert_eq!(
            store
                .record_donation(&github(n, DonationKind::Refund, 9_000))
                .await,
            Ok(Recorded::New)
        );
    }
    let sums = store.donation_sums().await.unwrap();
    assert_eq!((sums.once_total, sums.once_count), (0, 0));
    operator
        .execute_unprepared(
            "DELETE FROM central_identity.donations WHERE pg_catalog.get_byte(reference, 0) >= 20",
        )
        .await
        .unwrap();
}

/// Ko-fi's month is its last 31 days; GitHub's activities are read from the later
/// of the day the count began and 30 days ago.
async fn window(store: &AccountStore, operator: &DatabaseConnection, since: &str) {
    age(operator, 14, KOFI_WINDOW_DAYS - 1).await;
    age(operator, 15, KOFI_WINDOW_DAYS).await;
    let sums = store.donation_sums().await.unwrap();
    assert_eq!((sums.kofi_month, sums.kofi_payments), (300, 1));
    let from = store.donation_window().await.unwrap();
    // the count began today, which is later than 30 days ago
    assert_eq!(from, format!("{since}T00:00:00Z"));
    operator
        .execute_unprepared("UPDATE central_identity.donation_count SET since = since - 400;")
        .await
        .unwrap();
    let from = store.donation_window().await.unwrap();
    let thirty = operator
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT pg_catalog.to_char((pg_catalog.clock_timestamp() - interval '30 days')
                AT TIME ZONE 'UTC', 'YYYY-MM-DD') AS day",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get::<String>("", "day")
        .unwrap();
    assert_eq!(from.len(), 20, "{from}");
    assert!(from.starts_with(&thirty) && from.ends_with('Z'), "{from}");
    operator
        .execute_unprepared("UPDATE central_identity.donation_count SET since = since + 400;")
        .await
        .unwrap();
}

/// The sweep adds what is older than 40 days to the sums and deletes it; the sums
/// the public sees stay what they were, and the window's payment goes.
async fn folded(store: &AccountStore, operator: &DatabaseConnection, since: &str) {
    let before = store.donation_sums().await.unwrap();
    for n in [10, 11, 12, 13, 15] {
        age(operator, n, DONATION_DAYS).await;
    }
    assert_eq!(rows(operator).await, 6);
    let swept = sweep(operator).await.unwrap();
    assert_eq!(swept.donations, 5);
    assert_eq!(rows(operator).await, 1, "the window's one payment stays");
    assert_eq!(store.donation_sums().await.unwrap(), before);
    assert_eq!(
        super::super::super::scalar(
            operator,
            "SELECT count(*)::bigint AS value FROM central_identity.donation_sums"
        )
        .await,
        3,
        "github once and refund, kofi once"
    );
    // a second sweep finds nothing, and what comes later adds to the sums
    assert_eq!(sweep(operator).await.unwrap().donations, 0);
    assert_eq!(
        store
            .record_donation(&github(30, DonationKind::Once, 1_000))
            .await,
        Ok(Recorded::New)
    );
    age(operator, 30, DONATION_DAYS).await;
    assert_eq!(sweep(operator).await.unwrap().donations, 1);
    let after = store.donation_sums().await.unwrap();
    assert_eq!(
        (after.once_total, after.once_count, after.since.as_str()),
        (before.once_total + 1_000, before.once_count + 1, since)
    );
    // a swept reference is gone: were it read again, it would count again, which is
    // why GitHub's window (30 days) is shorter than the rows' (40)
    assert_eq!(
        store
            .record_donation(&github(30, DonationKind::Once, 1_000))
            .await,
        Ok(Recorded::New)
    );
    operator
        .execute_unprepared(
            "DELETE FROM central_identity.donations WHERE pg_catalog.get_byte(reference, 0) = 30",
        )
        .await
        .unwrap();
}

/// The runtime adds rows and converts them, and does nothing else to them.
async fn grants(operator: &DatabaseConnection) {
    let runtime =
        central_postgres::connect_dev(config("central_runtime", "CENTRAL_TEST_RUNTIME_PASSWORD"))
            .await
            .unwrap();
    for statement in [
        "UPDATE central_identity.donations SET amount = 1",
        "UPDATE central_identity.donations SET received_at = pg_catalog.clock_timestamp()",
        "UPDATE central_identity.donations SET reference = reference",
        "DELETE FROM central_identity.donations",
        "TRUNCATE central_identity.donations",
        "INSERT INTO central_identity.donation_sums VALUES ('kofi', 'once', 1, 1)",
        "UPDATE central_identity.donation_sums SET euro_cents = 0",
        "DELETE FROM central_identity.donation_sums",
        "UPDATE central_identity.donation_count SET since = '2000-01-01'",
        "INSERT INTO central_identity.donation_count VALUES (true, '2000-01-01')",
        "DELETE FROM central_identity.donation_count",
    ] {
        assert!(
            runtime.execute_unprepared(statement).await.is_err(),
            "{statement}"
        );
    }
    // what it may do: convert by the euro column alone
    assert!(
        runtime
            .execute_unprepared(
                "UPDATE central_identity.donations SET euro_cents = euro_cents WHERE false"
            )
            .await
            .is_ok()
    );
    // and the table's own checks hold for whoever writes
    for statement in [
        "INSERT INTO central_identity.donations (platform, kind, amount, currency, reference)
         VALUES ('liberapay', 'once', 1, 'EUR', decode(repeat('01', 32), 'hex'))",
        "INSERT INTO central_identity.donations (platform, kind, amount, currency, reference)
         VALUES ('kofi', 'refund', 1, 'EUR', decode(repeat('02', 32), 'hex'))",
        "INSERT INTO central_identity.donations (platform, kind, amount, currency, reference)
         VALUES ('github', 'recurring', 1, 'USD', decode(repeat('03', 32), 'hex'))",
        "INSERT INTO central_identity.donations (platform, kind, amount, currency, reference)
         VALUES ('kofi', 'once', 1, 'eur', decode(repeat('04', 32), 'hex'))",
        "INSERT INTO central_identity.donations (platform, kind, amount, currency, reference)
         VALUES ('kofi', 'once', 0, 'EUR', decode(repeat('05', 32), 'hex'))",
        "INSERT INTO central_identity.donations (platform, kind, amount, currency, reference)
         VALUES ('kofi', 'once', 1, 'EUR', decode('06', 'hex'))",
        "INSERT INTO central_identity.donations (id, platform, kind, amount, currency, reference)
         VALUES (gen_random_uuid(), 'kofi', 'once', 1, 'EUR', decode(repeat('07', 32), 'hex'))",
        "INSERT INTO central_identity.donations (platform, kind, amount, currency, euro_cents, reference)
         VALUES ('kofi', 'once', 1, 'EUR', 10000001, decode(repeat('08', 32), 'hex'))",
    ] {
        assert!(
            operator.execute_unprepared(statement).await.is_err(),
            "{statement}"
        );
    }
    runtime.close().await.unwrap();
}
