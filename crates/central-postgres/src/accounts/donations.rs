//! Donations (migration 000032, `docs/operations.md` §"Donations"): what
//! GitHub Sponsors and Ko-fi report, kept 40 days and then folded into one sum
//! per platform. The runtime adds a row once, converts a row that arrived
//! without a rate, and reads sums. It never changes an amount, never deletes a
//! row and never writes a sum: the operator's sweep does that under the
//! migrator's login. Nothing in a row names who gave.
use super::{AccountStore, Error, statement};
use sea_orm::ConnectionTrait;

/// How long a row is kept, in days: past Ko-fi's window ([`KOFI_WINDOW_DAYS`]) and
/// past the window of GitHub's activities that is read again ([`GITHUB_WINDOW_DAYS`]),
/// so that a swept row never comes back as a new one.
pub const DONATION_DAYS: i32 = 40;
/// Ko-fi's regular payments that make its monthly figure: those of the last 31 days.
/// A month's payments are 28 to 31 days apart, so every supporter who pays monthly
/// is counted, and twice only for the days a shorter month leaves (February's three).
pub const KOFI_WINDOW_DAYS: i32 = 31;
/// The days of GitHub's activities that each poll reads again.
pub const GITHUB_WINDOW_DAYS: i32 = 30;
/// How long a row that arrived without a rate may still be converted, in days: the
/// rate then is no longer the rate of the day it came.
pub const CONVERT_DAYS: i32 = 7;
/// The most one row may be worth, in euro cents: a hundred thousand euros.
pub const MOST_EURO_CENTS: u64 = 10_000_000;
/// The most one row's amount may be, in hundredths of its currency.
pub const MOST_AMOUNT: u64 = 99_999_999_999;

/// Where a donation was made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    /// GitHub Sponsors.
    GitHub,
    /// Ko-fi.
    Kofi,
}

impl Platform {
    const fn as_str(self) -> &'static str {
        match self {
            Self::GitHub => "github",
            Self::Kofi => "kofi",
        }
    }
}

/// What the money did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DonationKind {
    /// Given once.
    Once,
    /// A regular payment (Ko-fi only).
    Recurring,
    /// A one-time sponsorship's money went back (GitHub only).
    Refund,
}

impl DonationKind {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Once => "once",
            Self::Recurring => "recurring",
            Self::Refund => "refund",
        }
    }
}

/// One donation as a platform reported it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Donation<'a> {
    /// Where.
    pub platform: Platform,
    /// What the money did.
    pub kind: DonationKind,
    /// The amount, in hundredths of `currency`, 1 to [`MOST_AMOUNT`].
    pub amount: u64,
    /// Three capital Latin letters.
    pub currency: &'a str,
    /// The amount in euro cents at the rate Central holds, at most
    /// [`MOST_EURO_CENTS`]; `None` without a rate.
    pub euro_cents: Option<u64>,
    /// The digest of the platform's own id of it.
    pub reference: [u8; 32],
}

/// Whether a row was added.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Recorded {
    /// A new donation.
    New,
    /// One with this reference was there already; it is left as it was.
    Again,
}

/// The sums Central publishes of the stored donations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DonationSums {
    /// Ko-fi's regular payments in the last [`KOFI_WINDOW_DAYS`], in euro cents.
    pub kofi_month: u64,
    /// How many there were.
    pub kofi_payments: u64,
    /// What was given once on GitHub Sponsors and Ko-fi, refunds taken off, in euro
    /// cents; never below nothing.
    pub once_total: u64,
    /// How many, refunds taken off; never below none.
    pub once_count: u64,
    /// The day the count began, `YYYY-MM-DD`.
    pub since: String,
}

fn shaped(currency: &str) -> bool {
    currency.len() == 3 && currency.bytes().all(|b| b.is_ascii_uppercase())
}

fn count(row: &sea_orm::QueryResult, column: &str) -> Result<u64, Error> {
    let value: i64 = row.try_get("", column).map_err(|_| Error::Unavailable)?;
    u64::try_from(value).map_err(|_| Error::Unavailable)
}

impl AccountStore {
    /// Keeps `donation` once: a second one with the same platform and reference
    /// changes nothing and is answered [`Recorded::Again`].
    ///
    /// # Errors
    /// Rejects an amount, currency or euro amount out of bounds, a refund that is not
    /// GitHub's and a regular payment that is not Ko-fi's; `Unavailable` when the
    /// store fails.
    pub async fn record_donation(&self, donation: &Donation<'_>) -> Result<Recorded, Error> {
        // GitHub's regular giving is its estimate, and Ko-fi reports no refund.
        let misfit = matches!(
            (donation.platform, donation.kind),
            (Platform::GitHub, DonationKind::Recurring) | (Platform::Kofi, DonationKind::Refund)
        );
        if misfit
            || !(1..=MOST_AMOUNT).contains(&donation.amount)
            || !shaped(donation.currency)
            || donation
                .euro_cents
                .is_some_and(|cents| cents > MOST_EURO_CENTS)
        {
            return Err(Error::Rejected);
        }
        let amount = i64::try_from(donation.amount).map_err(|_| Error::Rejected)?;
        let euro = donation
            .euro_cents
            .map(i64::try_from)
            .transpose()
            .map_err(|_| Error::Rejected)?;
        let added = self
            .db
            .execute_raw(statement(
                "INSERT INTO central_identity.donations
                    (platform, kind, amount, currency, euro_cents, reference)
                 VALUES ($1, $2, $3, $4, $5, $6)
                 ON CONFLICT (platform, reference) DO NOTHING",
                vec![
                    donation.platform.as_str().into(),
                    donation.kind.as_str().into(),
                    amount.into(),
                    donation.currency.into(),
                    euro.into(),
                    donation.reference.to_vec().into(),
                ],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .rows_affected();
        Ok(if added == 1 {
            Recorded::New
        } else {
            Recorded::Again
        })
    }

    /// Converts the rows in `currency` that arrived without a rate in the last
    /// [`CONVERT_DAYS`], at `rate` millionths of a unit to the euro, rounded half up as
    /// `central_funding::rates::euro_cents` does; a row that would be worth more than
    /// [`MOST_EURO_CENTS`] stays unconverted. Returns how many were converted.
    ///
    /// # Errors
    /// Rejects a currency of another shape or a rate of nothing; `Unavailable` when
    /// the store fails.
    pub async fn convert_donations(&self, currency: &str, rate: u64) -> Result<u64, Error> {
        let rate = i64::try_from(rate).map_err(|_| Error::Rejected)?;
        if !shaped(currency) || rate <= 0 {
            return Err(Error::Rejected);
        }
        Ok(self
            .db
            .execute_raw(statement(
                "UPDATE central_identity.donations
                 SET euro_cents = (amount * 1000000 + $2 / 2) / $2
                 WHERE currency = $1 AND euro_cents IS NULL
                   AND received_at > pg_catalog.clock_timestamp() - pg_catalog.make_interval(days => $3)
                   AND (amount * 1000000 + $2 / 2) / $2 <= $4",
                vec![
                    currency.into(),
                    rate.into(),
                    CONVERT_DAYS.into(),
                    i64::try_from(MOST_EURO_CENTS)
                        .map_err(|_| Error::Rejected)?
                        .into(),
                ],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .rows_affected())
    }

    /// The currencies of rows still without a rate, and how many of each: what the
    /// operator's log names as not counted.
    ///
    /// # Errors
    /// `Unavailable` when the store fails.
    pub async fn unconverted_donations(&self) -> Result<Vec<(String, u64)>, Error> {
        let rows = self
            .db
            .query_all_raw(statement(
                "SELECT currency, count(*)::bigint AS rows FROM central_identity.donations
                 WHERE euro_cents IS NULL GROUP BY currency ORDER BY currency LIMIT 64",
                vec![],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        rows.iter()
            .map(|row| {
                let currency: String = row
                    .try_get("", "currency")
                    .map_err(|_| Error::Unavailable)?;
                Ok((currency, count(row, "rows")?))
            })
            .collect()
    }

    /// The sums Central publishes: Ko-fi's regular payments in its window, and what
    /// was given once on both platforms since the count began, refunds taken off.
    /// A row without a rate counts for nothing.
    ///
    /// # Errors
    /// `Unavailable` when the store fails.
    pub async fn donation_sums(&self) -> Result<DonationSums, Error> {
        let row = self
            .db
            .query_one_raw(statement(
                "WITH window_rows AS (
                    SELECT euro_cents FROM central_identity.donations
                    WHERE platform = 'kofi' AND kind = 'recurring' AND euro_cents IS NOT NULL
                      AND received_at > pg_catalog.clock_timestamp() - pg_catalog.make_interval(days => $1)
                 ), given AS (
                    SELECT CASE kind WHEN 'once' THEN euro_cents ELSE -euro_cents END AS cents,
                           CASE kind WHEN 'once' THEN 1 ELSE -1 END AS n
                    FROM central_identity.donations
                    WHERE kind IN ('once', 'refund') AND euro_cents IS NOT NULL
                    UNION ALL
                    SELECT CASE kind WHEN 'once' THEN euro_cents ELSE -euro_cents END,
                           CASE kind WHEN 'once' THEN count ELSE -count END
                    FROM central_identity.donation_sums
                 )
                 SELECT
                    (SELECT COALESCE(sum(euro_cents), 0) FROM window_rows)::bigint AS kofi_month,
                    (SELECT count(*) FROM window_rows)::bigint AS kofi_payments,
                    (SELECT GREATEST(COALESCE(sum(cents), 0), 0) FROM given)::bigint AS once_total,
                    (SELECT GREATEST(COALESCE(sum(n), 0), 0) FROM given)::bigint AS once_count,
                    (SELECT pg_catalog.to_char(since, 'YYYY-MM-DD')
                     FROM central_identity.donation_count) AS since",
                vec![KOFI_WINDOW_DAYS.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Unavailable)?;
        Ok(DonationSums {
            kofi_month: count(&row, "kofi_month")?,
            kofi_payments: count(&row, "kofi_payments")?,
            once_total: count(&row, "once_total")?,
            once_count: count(&row, "once_count")?,
            since: row.try_get("", "since").map_err(|_| Error::Unavailable)?,
        })
    }

    /// Where GitHub's activities are read from: the later of the day the count began
    /// and [`GITHUB_WINDOW_DAYS`] ago, as an RFC 3339 time in UTC.
    ///
    /// # Errors
    /// `Unavailable` when the store fails.
    pub async fn donation_window(&self) -> Result<String, Error> {
        self.db
            .query_one_raw(statement(
                "SELECT pg_catalog.to_char(GREATEST(since::timestamp AT TIME ZONE 'UTC',
                        pg_catalog.clock_timestamp() - pg_catalog.make_interval(days => $1))
                        AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"') AS since
                 FROM central_identity.donation_count",
                vec![GITHUB_WINDOW_DAYS.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Unavailable)?
            .try_get("", "since")
            .map_err(|_| Error::Unavailable)
    }
}

/// Folds the rows older than [`DONATION_DAYS`] into the sums and deletes them, in the
/// sweep's transaction. A regular payment past its window and a row that never got a
/// rate are deleted without a trace. Returns the rows deleted.
pub(super) async fn sweep(tx: &impl ConnectionTrait) -> Result<u64, Error> {
    let row = tx
        .query_one_raw(statement(
            "WITH old AS (
                DELETE FROM central_identity.donations
                WHERE received_at < pg_catalog.clock_timestamp() - pg_catalog.make_interval(days => $1)
                RETURNING platform, kind, euro_cents
             ), folded AS (
                INSERT INTO central_identity.donation_sums AS s (platform, kind, euro_cents, count)
                SELECT platform, kind, sum(euro_cents), count(*) FROM old
                WHERE kind IN ('once', 'refund') AND euro_cents IS NOT NULL
                GROUP BY platform, kind
                ON CONFLICT (platform, kind) DO UPDATE
                SET euro_cents = s.euro_cents + EXCLUDED.euro_cents, count = s.count + EXCLUDED.count
             )
             SELECT count(*)::bigint AS deleted FROM old",
            vec![DONATION_DAYS.into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?
        .ok_or(Error::Unavailable)?;
    count(&row, "deleted")
}
