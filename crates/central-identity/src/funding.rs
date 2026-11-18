//! The donation figures (`docs/operations.md` §"Donations"): what the service's
//! pollers last read from Liberapay and GitHub Sponsors, the reference rates, the
//! stored sums, and Ko-fi's deliveries. The public answer is built from memory alone
//! and never reaches the store: a flood of it reads no row. A delivery from Ko-fi
//! reaches the store only with the right token.
//!
//! This module logs nothing. It counts: deliveries refused by reason, payments
//! recorded, deliveries that came again, payments verified while recording is off,
//! and sales; the service writes the counts to its log.
use super::{Error, IdentityService, Response};
use central_api::funding::{Funding as Figures, MOST_MONTH, Monthly, Once};
use central_funding::{
    Currency, Rates,
    github::{self, Activity},
    kofi::{self, Delivery, Refusal},
};
use central_postgres::donations::{
    Donation, DonationKind, DonationSums, MOST_EURO_CENTS, Platform, Recorded,
};
use std::sync::{
    Mutex,
    atomic::{AtomicU64, Ordering},
};

/// The most of a Ko-fi delivery's body that is taken; a longer one is refused and
/// counted as `size`.
pub const KOFI_LIMIT: usize = kofi::LIMIT;

/// The most of a Ko-fi delivery's body the listener reads at all, so that a delivery
/// past [`KOFI_LIMIT`] is counted; a longer one is refused before it is read.
pub const KOFI_READ: usize = kofi::READ;

/// The digest Central holds of Ko-fi's verification token ([`Kofi::token`]).
#[must_use]
pub fn kofi_digest(token: &str) -> [u8; 32] {
    kofi::digest(token)
}

/// What the configuration turned on. Without it ([`IdentityService::with_funding`]
/// not called, as in development) every figure is absent.
#[derive(Clone, Copy)]
pub struct Sources {
    /// Liberapay's public figures are read.
    pub liberapay: bool,
    /// GitHub Sponsors is asked with the account's token.
    pub github: bool,
    /// Ko-fi's webhook is answered.
    pub kofi: Option<Kofi>,
}

/// Ko-fi's webhook as configured.
#[derive(Clone, Copy)]
pub struct Kofi {
    /// The SHA-256 digest of the verification token; the token itself is not kept.
    pub token: [u8; 32],
    /// Whether a verified payment is stored and counted. Off, a delivery is verified
    /// and read and nothing is kept: the state for Ko-fi's test delivery.
    pub record: bool,
}

/// How a Ko-fi delivery was answered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KofiAnswer {
    /// Taken: recorded, recorded before, verified without recording, or a sale.
    Taken,
    /// Refused, for any reason: one plain answer for all of them.
    Refused,
    /// The store did not answer; the platform may deliver again.
    Unavailable,
}

/// What became of Ko-fi's deliveries since the start.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KofiCounts {
    /// Refused, by [`Refusal::ALL`]'s order.
    pub refused: [u64; Refusal::ALL.len()],
    /// Payments recorded.
    pub recorded: u64,
    /// Of those, recorded without a rate and so not counted yet.
    pub unconverted: u64,
    /// Deliveries of a payment recorded before.
    pub again: u64,
    /// Payments verified and not recorded, because recording is off.
    pub unrecorded: u64,
    /// Sales verified and not counted.
    pub sales: u64,
}

#[derive(Default)]
struct Counters {
    refused: [AtomicU64; Refusal::ALL.len()],
    recorded: AtomicU64,
    unconverted: AtomicU64,
    again: AtomicU64,
    unrecorded: AtomicU64,
    sales: AtomicU64,
}

impl Counters {
    fn refuse(&self, reason: Refusal) -> KofiAnswer {
        if let Some(index) = Refusal::ALL.iter().position(|each| *each == reason) {
            self.refused[index].fetch_add(1, Ordering::Relaxed);
        }
        KofiAnswer::Refused
    }

    fn snapshot(&self) -> KofiCounts {
        let read = |counter: &AtomicU64| counter.load(Ordering::Relaxed);
        KofiCounts {
            refused: std::array::from_fn(|index| read(&self.refused[index])),
            recorded: read(&self.recorded),
            unconverted: read(&self.unconverted),
            again: read(&self.again),
            unrecorded: read(&self.unrecorded),
            sales: read(&self.sales),
        }
    }
}

/// What is known now.
#[derive(Default)]
struct Known {
    liberapay: Option<Monthly>,
    /// GitHub's monthly estimate in US cents, and the active sponsorships.
    github: Option<(u64, u64)>,
    rates: Option<Rates>,
    stored: Option<DonationSums>,
}

/// The service's donation state.
#[derive(Default)]
pub(super) struct Funding {
    sources: Option<Sources>,
    known: Mutex<Known>,
    counters: Counters,
}

fn figures(sources: Option<Sources>, known: &Known) -> Figures {
    let Some(sources) = sources else {
        return Figures::default();
    };
    let usd = known
        .rates
        .as_ref()
        .and_then(|rates| rates.rate(Currency::USD));
    let github = known
        .github
        .filter(|_| sources.github)
        .and_then(|(cents, supporters)| {
            Some(Monthly {
                month: central_funding::rates::euro_cents(cents, usd?)?.min(MOST_MONTH),
                supporters,
            })
        });
    let recording = sources.kofi.is_some_and(|kofi| kofi.record);
    let stored = known.stored.as_ref();
    let kofi = stored.filter(|_| recording).map(|sums| Monthly {
        month: sums.kofi_month.min(MOST_MONTH),
        supporters: sums.kofi_payments.min(central_api::funding::MOST_COUNT),
    });
    let once = stored
        .filter(|_| recording || sources.github)
        .map(|sums| Once {
            total: sums.once_total.min(central_api::funding::MOST_TOTAL),
            count: sums.once_count.min(central_api::funding::MOST_COUNT),
            since: sums.since.clone(),
        });
    let figures = Figures {
        github,
        liberapay: known.liberapay.filter(|_| sources.liberapay),
        kofi,
        once,
    };
    // Every figure is bounded above; a day of another shape drops the sum.
    if figures.validate().is_err() {
        return Figures {
            once: None,
            ..figures
        };
    }
    figures
}

impl IdentityService {
    /// Collect the donation figures `sources` names (`docs/operations.md`
    /// §"Donations"). Whoever calls this also drives the pollers that feed
    /// [`Self::funding_liberapay`], [`Self::funding_github`],
    /// [`Self::funding_rates`] and [`Self::refresh_donations`].
    #[must_use]
    pub fn with_funding(mut self, sources: Sources) -> Self {
        self.funding.sources = Some(sources);
        self
    }

    fn known(&self) -> Result<std::sync::MutexGuard<'_, Known>, Error> {
        self.funding.known.lock().map_err(|_| Error::Unavailable)
    }

    /// Liberapay's latest figure; an answer that failed is not reported, so the
    /// last one stands.
    pub fn funding_liberapay(&self, monthly: Monthly) {
        if let Ok(mut known) = self.known() {
            known.liberapay = Some(monthly);
        }
    }

    /// GitHub's latest estimate (US cents) and count of active sponsorships.
    pub fn funding_github(&self, cents: u64, sponsors: u64) {
        if let Ok(mut known) = self.known() {
            known.github = Some((cents, sponsors));
        }
    }

    /// The latest reference rates.
    pub fn funding_rates(&self, rates: Rates) {
        if let Ok(mut known) = self.known() {
            known.rates = Some(rates);
        }
    }

    /// The reference rates held, if any.
    #[must_use]
    pub fn reference_rates(&self) -> Option<Rates> {
        self.known().ok().and_then(|known| known.rates.clone())
    }

    /// Whether Ko-fi's webhook is answered at all.
    #[must_use]
    pub fn kofi_listening(&self) -> bool {
        self.funding
            .sources
            .is_some_and(|sources| sources.kofi.is_some())
    }

    /// What became of Ko-fi's deliveries since the start.
    #[must_use]
    pub fn kofi_counts(&self) -> KofiCounts {
        self.funding.counters.snapshot()
    }

    /// Reads the stored sums again; a store that fails leaves the last ones.
    ///
    /// # Errors
    /// `Unavailable` when the store fails.
    pub async fn refresh_donations(&self) -> Result<(), Error> {
        let sums = self.store.donation_sums().await?;
        self.known()?.stored = Some(sums);
        Ok(())
    }

    /// Converts the rows that came without a rate, for every currency the rates now
    /// have, and returns the currencies still without one and how many rows of each.
    ///
    /// # Errors
    /// `Unavailable` when the store fails.
    pub async fn convert_donations(&self) -> Result<Vec<(String, u64)>, Error> {
        let waiting = self.store.unconverted_donations().await?;
        if let Some(rates) = self.reference_rates() {
            for (currency, _) in &waiting {
                if let Some(rate) = Currency::parse(currency).and_then(|code| rates.rate(code)) {
                    self.store.convert_donations(currency, rate).await?;
                }
            }
        }
        self.store.unconverted_donations().await
    }

    /// Where GitHub's activities are read from (an RFC 3339 time).
    ///
    /// # Errors
    /// `Unavailable` when the store fails.
    pub async fn github_window(&self) -> Result<String, Error> {
        self.store.donation_window().await
    }

    /// Keeps GitHub's one-time sponsorships and refunds, each once, converted at the
    /// rate held (or later, [`Self::convert_donations`]). Returns how many were new.
    ///
    /// # Errors
    /// `Unavailable` when the store fails.
    pub async fn record_github(&self, activities: &[Activity]) -> Result<u64, Error> {
        let rates = self.reference_rates();
        let mut new = 0;
        for activity in activities {
            let euro_cents = rates
                .as_ref()
                .and_then(|rates| rates.euro_cents(activity.cents, Currency::USD))
                .filter(|cents| *cents <= MOST_EURO_CENTS);
            let kind = match activity.kind {
                github::Kind::Once => DonationKind::Once,
                github::Kind::Refund => DonationKind::Refund,
            };
            let recorded = self
                .store
                .record_donation(&Donation {
                    platform: Platform::GitHub,
                    kind,
                    amount: activity.cents,
                    currency: Currency::USD.as_str(),
                    euro_cents,
                    reference: activity.reference,
                })
                .await?;
            new += u64::from(recorded == Recorded::New);
        }
        Ok(new)
    }

    /// One delivery to Ko-fi's webhook, its header and body as they came. The token
    /// is checked before anything else of it is used (`central_funding::kofi`), and
    /// nothing of the body is kept but a payment's kind, amount, currency and the
    /// digest of its transaction id.
    pub async fn kofi(&self, content_type: Option<&[u8]>, body: &[u8]) -> KofiAnswer {
        let counters = &self.funding.counters;
        let Some(intake) = self.funding.sources.and_then(|sources| sources.kofi) else {
            return KofiAnswer::Refused;
        };
        let payment = match kofi::read(content_type, body, &intake.token) {
            Err(reason) => return counters.refuse(reason),
            Ok(Delivery::Sale) => {
                counters.sales.fetch_add(1, Ordering::Relaxed);
                return KofiAnswer::Taken;
            }
            Ok(Delivery::Payment(payment)) => payment,
        };
        if !intake.record {
            counters.unrecorded.fetch_add(1, Ordering::Relaxed);
            return KofiAnswer::Taken;
        }
        let euro_cents = self.reference_rates().map_or_else(
            // the euro needs no rate, whether rates are held or not
            || (payment.currency == Currency::EUR).then_some(payment.amount),
            |rates| rates.euro_cents(payment.amount, payment.currency),
        );
        if euro_cents.is_some_and(|cents| cents > MOST_EURO_CENTS) {
            return counters.refuse(Refusal::Amount);
        }
        let kind = match payment.kind {
            kofi::Kind::Once => DonationKind::Once,
            kofi::Kind::Recurring => DonationKind::Recurring,
        };
        let recorded = self
            .store
            .record_donation(&Donation {
                platform: Platform::Kofi,
                kind,
                amount: payment.amount,
                currency: payment.currency.as_str(),
                euro_cents,
                reference: payment.reference,
            })
            .await;
        match recorded {
            Ok(Recorded::New) => {
                counters.recorded.fetch_add(1, Ordering::Relaxed);
                if euro_cents.is_none() {
                    counters.unconverted.fetch_add(1, Ordering::Relaxed);
                }
                KofiAnswer::Taken
            }
            Ok(Recorded::Again) => {
                counters.again.fetch_add(1, Ordering::Relaxed);
                KofiAnswer::Taken
            }
            Err(Error::Rejected) => counters.refuse(Refusal::Amount),
            Err(_) => KofiAnswer::Unavailable,
        }
    }

    /// The public donation figures ([`super::Request::PublicFunding`]), from memory.
    pub(super) fn public_funding(&self) -> Result<Response, Error> {
        let known = self.known()?;
        Ok(Response::PublicFunding {
            funding: figures(self.funding.sources, &known),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sums() -> DonationSums {
        DonationSums {
            kofi_month: 1_000,
            kofi_payments: 2,
            once_total: 2_780,
            once_count: 2,
            since: "2026-11-18".into(),
        }
    }

    fn known() -> Known {
        Known {
            liberapay: Some(Monthly {
                month: 10_010,
                supporters: 3,
            }),
            github: Some((4_600, 2)),
            rates: Some(Rates::of("2026-11-18", &[(Currency::USD, 1_096_800)])),
            stored: Some(sums()),
        }
    }

    fn sources(github: bool, kofi: Option<bool>) -> Sources {
        Sources {
            liberapay: true,
            github,
            kofi: kofi.map(|record| Kofi {
                token: [7; 32],
                record,
            }),
        }
    }

    #[test]
    fn without_sources_every_figure_is_absent() {
        assert_eq!(figures(None, &known()), Figures::default());
    }

    #[test]
    fn with_nothing_configured_only_liberapay_is_counted() {
        let only = figures(Some(sources(false, None)), &known());
        assert_eq!(
            only,
            Figures {
                liberapay: Some(Monthly {
                    month: 10_010,
                    supporters: 3
                }),
                ..Figures::default()
            }
        );
        // and before Liberapay answered, nothing
        assert_eq!(
            figures(Some(sources(false, None)), &Known::default()),
            Figures::default()
        );
    }

    #[test]
    fn each_source_adds_its_figure() {
        let all = figures(Some(sources(true, Some(true))), &known());
        // 46 dollars at 1.0968 are 41.94 euros
        assert_eq!(
            all.github,
            Some(Monthly {
                month: 4_194,
                supporters: 2
            })
        );
        assert_eq!(
            all.kofi,
            Some(Monthly {
                month: 1_000,
                supporters: 2
            })
        );
        assert_eq!(
            all.once,
            Some(Once {
                total: 2_780,
                count: 2,
                since: "2026-11-18".into()
            })
        );
        // Ko-fi only verifying: no Ko-fi figure, the sum stays GitHub's
        let verifying = figures(Some(sources(true, Some(false))), &known());
        assert_eq!(verifying.kofi, None);
        assert!(verifying.once.is_some());
        // Ko-fi alone: its figure and the sum
        let kofi = figures(Some(sources(false, Some(true))), &known());
        assert!(kofi.kofi.is_some() && kofi.once.is_some() && kofi.github.is_none());
        // Ko-fi only verifying and no GitHub: no sum either
        let neither = figures(Some(sources(false, Some(false))), &known());
        assert_eq!((neither.kofi, neither.once), (None, None));
    }

    #[test]
    fn a_figure_waits_for_what_it_needs() {
        // GitHub's estimate without a dollar rate is not shown, never guessed
        let mut without_rate = known();
        without_rate.rates = None;
        assert_eq!(
            figures(Some(sources(true, None)), &without_rate).github,
            None
        );
        let mut other_rate = known();
        other_rate.rates = Some(Rates::of(
            "2026-11-18",
            &[(Currency::parse("GBP").unwrap(), 832_900)],
        ));
        assert_eq!(figures(Some(sources(true, None)), &other_rate).github, None);
        // the store's sums before their first read are absent, not zero
        let mut unread = known();
        unread.stored = None;
        let figures = figures(Some(sources(true, Some(true))), &unread);
        assert_eq!((figures.kofi, figures.once), (None, None));
        assert!(figures.github.is_some());
    }

    #[test]
    fn every_figure_is_bounded() {
        let mut huge = known();
        huge.github = Some((u64::MAX / 2_000_000, 5));
        huge.rates = Some(Rates::of("2026-11-18", &[(Currency::USD, 1)]));
        huge.stored = Some(DonationSums {
            kofi_month: u64::MAX,
            kofi_payments: u64::MAX,
            once_total: u64::MAX,
            once_count: u64::MAX,
            since: "<script>".into(),
        });
        let all = figures(Some(sources(true, Some(true))), &huge);
        assert_eq!(all.validate(), Ok(()));
        assert_eq!(all.github.map(|github| github.month), Some(MOST_MONTH));
        assert_eq!(all.kofi.map(|kofi| kofi.month), Some(MOST_MONTH));
        assert_eq!(all.once, None, "a day of another shape drops the sum");
    }
}
