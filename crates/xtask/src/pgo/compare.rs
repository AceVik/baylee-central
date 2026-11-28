//! The measurement of `cargo xtask pgo --compare`: the profiled binaries
//! against the build the deploy hook makes today (`cargo build --release`
//! with the same `-C target-cpu`), on the training's pages and pictures.
//!
//! A report and not a gate: a shared runner's load moves the figures, so
//! the two builds take turns (one first, then the other, and the other way
//! round in the next round) and the medians are compared. A page's time
//! includes the client's and the loopback's, the same for both builds, so
//! the server's own gain is larger than the figure shown.

use std::path::Path;
use std::time::{Duration, Instant};

use super::train::{self, Picture, Web};
use super::{Builds, FlagMode, PLAIN, PROFILED, bin_path, http_agent};
use crate::fail::{Fail, Result, after_cleanup};

/// The deploy hook's cargo profile (`ops/deploy-hook`).
const DEPLOYED: &str = "release";

/// Timed rounds of each build, after one round each to warm up.
const ROUNDS: usize = 9;

/// The pictures timed, one conversion of each per round.
const TIMED_PICTURES: [&str; 4] = ["photo.jpg", "screen.png", "lossy.webp", "moving.gif"];

pub(super) fn run(builds: &Builds<'_>, profiled: &Path) -> Result<()> {
    println!(
        "PGO compare: building what the deploy hook builds (--release, target-cpu={})...",
        builds.target_cpu
    );
    let target = builds.root.join("target/pgo-baseline");
    let mut deployed_bins = PROFILED.to_vec();
    deployed_bins.extend(PLAIN);
    builds.build(&target, DEPLOYED, &deployed_bins, FlagMode::Plain)?;
    let deployed = builds.bin_dir(&target, DEPLOYED);
    web(builds.root, &deployed, profiled)?;
    pictures(builds.root, &deployed, profiled)
}

fn web(root: &Path, deployed: &Path, profiled: &Path) -> Result<()> {
    let today = Web::start(root, &bin_path(deployed, "central-web"), None)?;
    let trained = match Web::start(root, &bin_path(profiled, "central-web"), None) {
        Ok(server) => server,
        Err(failure) => return after_cleanup(Err(failure), today.stop()),
    };
    let outcome = (|| -> Result<()> {
        let agent = http_agent();
        let bases = [today.base(), trained.base()];
        let mut times = [Vec::new(), Vec::new()];
        let mut requests = 0;
        for round in 0..=ROUNDS {
            for which in order(round) {
                let started = Instant::now();
                let (asked, answered) = train::pages(&agent, &bases[which], round);
                let took = started.elapsed();
                if answered != asked {
                    return Err(Fail::new("central-web stopped answering while measured"));
                }
                requests = asked;
                // the first round warms both up
                if round > 0 {
                    times[which].push(took);
                }
            }
        }
        println!(
            "PGO compare, website ({requests} requests a pass, median of {ROUNDS}): {}",
            verdict(&mut times)
        );
        Ok(())
    })();
    let stopped = after_cleanup(today.stop(), trained.stop());
    after_cleanup(outcome, stopped)
}

fn pictures(root: &Path, deployed: &Path, profiled: &Path) -> Result<()> {
    let workers = [
        bin_path(deployed, "central-picture-worker"),
        bin_path(profiled, "central-picture-worker"),
    ];
    let corpus = train::corpus(root)?;
    for name in TIMED_PICTURES {
        let picture = corpus
            .iter()
            .find(|p| p.name == name)
            .ok_or_else(|| Fail::new("A measured picture is not in the training"))?;
        let mut times = [Vec::new(), Vec::new()];
        for round in 0..=ROUNDS {
            for which in order(round) {
                let took = time_conversion(&workers[which], picture)?;
                if round > 0 {
                    times[which].push(took);
                }
            }
        }
        println!(
            "PGO compare, picture {name} ({} KiB, median of {ROUNDS}): {}",
            picture.bytes.len() / 1024,
            verdict(&mut times)
        );
    }
    Ok(())
}

fn time_conversion(worker: &Path, picture: &Picture) -> Result<Duration> {
    let started = Instant::now();
    train::convert(worker, picture.format, &picture.bytes, None)?;
    Ok(started.elapsed())
}

/// Which build goes first in `round`: today's in even rounds, the profiled
/// one in odd rounds.
fn order(round: usize) -> [usize; 2] {
    if round.is_multiple_of(2) {
        [0, 1]
    } else {
        [1, 0]
    }
}

/// The median of `times`, or zero for none.
fn median(times: &mut [Duration]) -> Duration {
    times.sort_unstable();
    times.get(times.len() / 2).copied().unwrap_or_default()
}

/// "today's build X ms, profiled Y ms: Z % less time" (or more).
fn verdict(times: &mut [Vec<Duration>; 2]) -> String {
    let today = median(&mut times[0]);
    let profiled = median(&mut times[1]);
    let (t, p) = (today.as_micros(), profiled.as_micros());
    let change = if t == 0 {
        "no time to compare".to_owned()
    } else if p <= t {
        format!("{} % less time", (t - p) * 100 / t)
    } else {
        format!("{} % more time", (p - t) * 100 / t)
    };
    format!(
        "today's build {:.1} ms, profiled {:.1} ms: {change}",
        millis(today),
        millis(profiled)
    )
}

/// Milliseconds with their fraction, for the report only.
fn millis(time: Duration) -> f64 {
    time.as_secs_f64() * 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_builds_take_turns_going_first() {
        assert_eq!(order(0), [0, 1]);
        assert_eq!(order(1), [1, 0]);
        assert_eq!(order(2), [0, 1]);
    }

    #[test]
    fn reports_the_medians_and_their_change() {
        let ms = Duration::from_millis;
        let mut faster = [vec![ms(30), ms(10), ms(20)], vec![ms(9), ms(15), ms(12)]];
        assert_eq!(
            verdict(&mut faster),
            "today's build 20.0 ms, profiled 12.0 ms: 40 % less time"
        );
        let mut slower = [vec![ms(10)], vec![ms(11)]];
        assert_eq!(
            verdict(&mut slower),
            "today's build 10.0 ms, profiled 11.0 ms: 10 % more time"
        );
        let mut none = [Vec::new(), Vec::new()];
        assert!(verdict(&mut none).ends_with("no time to compare"));
    }

    #[test]
    fn measures_only_pictures_the_training_draws() {
        for name in TIMED_PICTURES {
            assert!(train::drawn_names().any(|drawn| drawn == name), "{name}");
        }
    }
}
