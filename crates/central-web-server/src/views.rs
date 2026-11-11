//! How often the site's pages were opened: one number for every page and
//! every visitor together, shown in the start page's strip.
//!
//! A browser's navigation to a page adds one. Nothing about the request is
//! kept: no address, no cookie, no identifier, not even which page it was.
//! A request that is not a person opening a page is not counted where the
//! browser says so: a script's or a crawler's fetch carries no
//! `Sec-Fetch-Mode: navigate`, and a page fetched ahead of a visit says
//! `Sec-Purpose`.
//!
//! Counting is the deployment's choice (`Config::views`, a file's path).
//! The file holds the number and the day it began, and is the one thing
//! this server ever writes; without it nothing is counted and nothing is
//! shown. The pages show the number as it stood at the last minute
//! (`spawn`), never per request: a page's bytes and its entity tag change
//! with the data only.

use std::fs;
use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::http::HeaderMap;
use central_web::data;
use serde::{Deserialize, Serialize};

use crate::fetch::{self, Shared};

/// The most bytes read from the file: it holds two short values.
const LIMIT: u64 = 1024;

/// What the file holds.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Stored {
    /// The pages opened since `since`.
    views: u64,
    /// The day the count began, `YYYY-MM-DD`.
    since: String,
}

/// The count and where it is kept.
#[derive(Debug)]
pub struct Views {
    count: AtomicU64,
    /// The count as the file holds it.
    saved: AtomicU64,
    since: String,
    file: PathBuf,
}

/// The file cannot hold the count.
#[derive(Debug, PartialEq, Eq)]
pub struct Unusable(pub &'static str);

/// The civil date of a day counted from 1970-01-01 (proleptic Gregorian).
fn civil(days: i64) -> (i64, i64, i64) {
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// Today in UTC, `YYYY-MM-DD`.
fn today() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    let days = i64::try_from(seconds / 86_400).unwrap_or(0);
    let (year, month, day) = civil(days);
    format!("{year:04}-{month:02}-{day:02}")
}

/// Whether `text` is a date as [`today`] writes it.
fn dated(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() == 10
        && bytes.iter().enumerate().all(|(at, byte)| match at {
            4 | 7 => *byte == b'-',
            _ => byte.is_ascii_digit(),
        })
}

/// Whether a request is a browser opening a page for a person: a
/// navigation to a document, not fetched ahead of a visit.
#[must_use]
pub fn is_visit(headers: &HeaderMap) -> bool {
    let says = |name: &str, value: &str| {
        headers
            .get(name)
            .and_then(|said| said.to_str().ok())
            .is_some_and(|said| said.eq_ignore_ascii_case(value))
    };
    says("sec-fetch-mode", "navigate")
        && says("sec-fetch-dest", "document")
        && !headers.contains_key("sec-purpose")
        && !headers.contains_key("purpose")
}

impl Views {
    /// Takes the count up from `file`, or begins it today where there is no
    /// file yet, and writes it once: a file that cannot be written is found
    /// at start, not a minute later.
    ///
    /// # Errors
    /// A file that is too large, is not this module's, or cannot be written.
    pub fn open(file: &Path) -> Result<Self, Unusable> {
        let stored = match fs::File::open(file) {
            Ok(found) => {
                let mut text = String::new();
                found
                    .take(LIMIT + 1)
                    .read_to_string(&mut text)
                    .map_err(|_| Unusable("the views file cannot be read"))?;
                if u64::try_from(text.len()).unwrap_or(u64::MAX) > LIMIT {
                    return Err(Unusable("the views file is too large"));
                }
                let stored: Stored = serde_json::from_str(&text)
                    .map_err(|_| Unusable("the views file is malformed"))?;
                if !dated(&stored.since) {
                    return Err(Unusable("the views file is malformed"));
                }
                stored
            }
            Err(missing) if missing.kind() == std::io::ErrorKind::NotFound => Stored {
                views: 0,
                since: today(),
            },
            Err(_) => return Err(Unusable("the views file cannot be read")),
        };
        let views = Views {
            count: AtomicU64::new(stored.views),
            saved: AtomicU64::new(stored.views),
            since: stored.since,
            file: file.to_owned(),
        };
        views
            .write(stored.views)
            .map_err(|()| Unusable("the views file cannot be written"))?;
        Ok(views)
    }

    /// One more page was opened, if `headers` are a visit's.
    pub fn count(&self, headers: &HeaderMap) {
        if is_visit(headers) {
            // a count that reached the end stays there
            let _ = self
                .count
                .try_update(Ordering::Relaxed, Ordering::Relaxed, |count| {
                    count.checked_add(1)
                });
        }
    }

    /// The count as it stands, for the pages.
    #[must_use]
    pub fn now(&self) -> data::Views {
        data::Views {
            count: self.count.load(Ordering::Relaxed),
            since: self.since.clone(),
        }
    }

    /// Writes `count` beside the file and moves it over the file, so a
    /// reader never finds half of it.
    fn write(&self, count: u64) -> Result<(), ()> {
        let stored = Stored {
            views: count,
            since: self.since.clone(),
        };
        let text = serde_json::to_string(&stored).map_err(|_| ())?;
        let mut beside = self.file.clone().into_os_string();
        beside.push(".new");
        let beside = PathBuf::from(beside);
        let mut out = fs::File::create(&beside).map_err(|_| ())?;
        out.write_all(text.as_bytes()).map_err(|_| ())?;
        out.write_all(b"\n").map_err(|_| ())?;
        out.sync_all().map_err(|_| ())?;
        drop(out);
        fs::rename(&beside, &self.file).map_err(|_| ())?;
        self.saved.store(count, Ordering::Relaxed);
        Ok(())
    }

    /// Writes the count if it grew since it was last written. A failure
    /// costs the growth since then at worst; the next minute tries again.
    pub fn save(&self) {
        let count = self.count.load(Ordering::Relaxed);
        if count != self.saved.load(Ordering::Relaxed) {
            let _ = self.write(count);
        }
    }
}

/// Shows the count to the pages now and once a minute from now on, and
/// keeps the file as fresh. Until the process ends.
pub fn spawn(views: &Arc<Views>, shared: &Shared) {
    let (views, shared) = (Arc::clone(views), Arc::clone(shared));
    tokio::spawn(async move {
        let mut every = tokio::time::interval(Duration::from_secs(60));
        loop {
            every.tick().await;
            let now = views.now();
            fetch::install(&shared, |snapshot| snapshot.views = Some(now));
            let views = Arc::clone(&views);
            let _ = tokio::task::spawn_blocking(move || views.save()).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers(pairs: &[(&'static str, &'static str)]) -> HeaderMap {
        let mut map = HeaderMap::new();
        for (name, value) in pairs {
            map.insert(*name, value.parse().expect("a header value"));
        }
        map
    }

    /// A file of this test's own in the system's temporary folder.
    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("central-web-views-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("a scratch folder");
        dir.join("views.json")
    }

    #[test]
    fn days_become_dates() {
        assert_eq!(civil(0), (1970, 1, 1));
        assert_eq!(civil(59), (1970, 3, 1));
        assert_eq!(civil(11_016), (2000, 2, 29));
        assert_eq!(civil(11_017), (2000, 3, 1));
        assert_eq!(civil(20_768), (2026, 11, 11));
        assert_eq!(civil(-1), (1969, 12, 31));
        assert!(dated(&today()));
        assert!(!dated("2026-11-1"));
        assert!(!dated("11.11.2026"));
    }

    #[test]
    fn only_a_browser_opening_a_page_is_a_visit() {
        let visit = [
            ("sec-fetch-mode", "navigate"),
            ("sec-fetch-dest", "document"),
        ];
        assert!(is_visit(&headers(&visit)));
        // a script's fetch, a crawler without the headers, a frame
        assert!(!is_visit(&headers(&[])));
        assert!(!is_visit(&headers(&[
            ("sec-fetch-mode", "cors"),
            ("sec-fetch-dest", "empty")
        ])));
        assert!(!is_visit(&headers(&[
            ("sec-fetch-mode", "navigate"),
            ("sec-fetch-dest", "iframe")
        ])));
        // fetched ahead of a visit that may never come
        let mut ahead = visit.to_vec();
        ahead.push(("sec-purpose", "prefetch"));
        assert!(!is_visit(&headers(&ahead)));
    }

    #[test]
    fn the_count_begins_today_grows_by_visits_and_survives_a_restart() {
        let file = scratch("grows");
        let views = Views::open(&file).expect("a new count");
        assert_eq!(views.now().count, 0);
        assert_eq!(views.now().since, today());
        // written at once: the file is there before anything was counted
        assert!(file.exists());
        let visit = headers(&[
            ("sec-fetch-mode", "navigate"),
            ("sec-fetch-dest", "document"),
        ]);
        views.count(&visit);
        views.count(&visit);
        views.count(&headers(&[]));
        assert_eq!(views.now().count, 2);
        views.save();
        drop(views);
        let again = Views::open(&file).expect("the count taken up");
        assert_eq!(again.now().count, 2);
        // the file holds the two values and nothing about a request
        let text = fs::read_to_string(&file).expect("the file");
        assert_eq!(
            text.trim(),
            format!(r#"{{"views":2,"since":"{}"}}"#, today())
        );
        let _ = fs::remove_dir_all(file.parent().expect("its folder"));
    }

    #[test]
    fn a_file_that_is_not_the_count_s_is_refused_not_overwritten() {
        let file = scratch("refused");
        for wrong in [
            "",
            "{}",
            r#"{"views":-1,"since":"2026-11-11"}"#,
            r#"{"views":1,"since":"yesterday"}"#,
            r#"{"views":1,"since":"2026-11-11","address":"203.0.113.9"}"#,
        ] {
            fs::write(&file, wrong).expect("written");
            assert_eq!(
                Views::open(&file).map(|_| ()),
                Err(Unusable("the views file is malformed")),
                "{wrong}"
            );
            assert_eq!(fs::read_to_string(&file).expect("still there"), wrong);
        }
        fs::write(&file, "x".repeat(2_000)).expect("written");
        assert_eq!(
            Views::open(&file).map(|_| ()),
            Err(Unusable("the views file is too large"))
        );
        // a folder that does not exist cannot take the file
        let nowhere = file.parent().expect("its folder").join("none/views.json");
        assert_eq!(
            Views::open(&nowhere).map(|_| ()),
            Err(Unusable("the views file cannot be written"))
        );
        let _ = fs::remove_dir_all(file.parent().expect("its folder"));
    }

    #[test]
    fn a_full_count_stays_full() {
        let file = scratch("full");
        fs::write(
            &file,
            format!(r#"{{"views":{},"since":"2026-11-11"}}"#, u64::MAX),
        )
        .expect("written");
        let views = Views::open(&file).expect("the count");
        views.count(&headers(&[
            ("sec-fetch-mode", "navigate"),
            ("sec-fetch-dest", "document"),
        ]));
        assert_eq!(views.now().count, u64::MAX);
        let _ = fs::remove_dir_all(file.parent().expect("its folder"));
    }
}
