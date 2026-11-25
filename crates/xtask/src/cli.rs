//! The command line: `cargo xtask <task> [flag]`, nothing else.

use crate::fail::{Fail, Result};
pub use crate::postgres::Scope as PostgresScope;
use crate::{
    cards_ingest, chrono_scope, crypto_scope, dev_site, gate, pgo, policy, postgres, secrets,
    security_audio, site, smoke, wasm, workspace,
};

/// What to run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Task {
    /// fmt, the policy with its compiler probe, clippy, the tests, the doc
    /// tests and the release build.
    Gate {
        /// Only what the Baylee client links (and these tasks): clippy, the
        /// tests and the doc tests, without the release build.
        client: bool,
    },
    /// The exact source and dependency bounds for the bigint advisory.
    CryptoScope,
    /// The bound of the `chrono` advisory's exception: in the lock file
    /// only, in no build.
    ChronoScope,
    /// The code policy.
    Policy {
        /// Also check that rustc rejects unsafe code under the forbid lint.
        compiler_probe: bool,
    },
    /// The `PostgreSQL` integration fixture.
    Postgres {
        /// Fail on purpose once the fixture is up, to prove its cleanup.
        exercise_failure: bool,
        /// All suites, store regressions, or only expedition authority.
        scope: PostgresScope,
    },
    /// The development listener smoke test.
    Smoke {
        /// Fail on purpose once the listener is up, to prove its cleanup.
        exercise_failure: bool,
    },
    /// The secret scan of the tree and its history.
    Secrets,
    /// `central-web-crypto`'s WebAssembly module and glue for the website.
    Wasm {
        /// Compare a fresh build with the committed files instead of writing them.
        check: bool,
    },
    /// The website's assets: the islands' WebAssembly, styles, scripts,
    /// fonts and pictures under hashed names, into `target/site`.
    Site,
    /// Local loopback development runner for Central, the website and the
    /// proxy on `http://127.0.0.1:28794`.
    DevSite,
    /// Imports Scryfall's `all-cards` catalog into the local `dev-site` Postgres.
    CardsIngest {
        /// Optional path to an existing Scryfall `.jsonl` or `.jsonl.gz` bulk file.
        bulk_path: Option<String>,
    },
    /// Synthesizes `/security` narrated `.mp3` clips and step-sync `cues.json`
    /// manifests via `ElevenLabs` `eleven_v4`.
    SecurityAudio(security_audio::Options),
    /// Automated PGO + `FatLTO` (`x86-64-v3`) build pipeline for production servers.
    Pgo(pgo::Options),
}

/// How to call it.
pub const USAGE: &str = "usage: cargo xtask <task>

tasks:
  crypto-scope                 verify the reviewed Ristretto advisory scope
  chrono-scope                 verify chrono is in Cargo.lock only, in no build
  gate [--client]               fmt, policy, clippy, tests, doc tests, release build;
                                --client: only the client's crates, no release build
  policy [--compiler-probe]     the code policy: unsafe, native sources, links, dependencies
  postgres [--exercise-failure | --store-only | --tower-only | --tower-access-only | --tower-awards-only] the PostgreSQL integration fixture (needs Docker and OpenSSL)
  smoke [--exercise-failure]    the development listener smoke test (build --release first)
  secrets                       the secret scan of the tree and the full history (pinned gitleaks)
  wasm [--check]                the website's WebAssembly crypto and glue (crates/central-web/crypto)
  site                          the website's assets into target/site (BAYLEE_SRC: screenshots)
  dev-site                      run local Central + website stack on http://127.0.0.1:28794
  cards-ingest [bulk-path]      import Scryfall's all-cards catalog into local dev-site PostgreSQL
  security-audio [--lang <list>] [--voice-id <id>] [--dry-run] [--force]
                                synthesize /security audio clips + cues.json via ElevenLabs v4 (reads ELEVENLABS_API_KEY from env)
  pgo [--out <dir>] [--target-cpu <cpu>] [--profile-only]
                                collect local HTTP/codec profiles and build PGO+FatLTO server binaries";

fn parse_pgo(flags: &[String]) -> Result<Task> {
    let usage = || Fail::new(USAGE);
    let mut opts = pgo::Options::default();
    let mut at = 0;
    while at < flags.len() {
        let arg = flags[at].as_str();
        match arg {
            "--profile-only" => {
                opts.profile_only = true;
                at += 1;
            }
            "--out" => {
                let raw = flags.get(at + 1).ok_or_else(usage)?;
                if !pgo::valid_out_dir(raw) {
                    return Err(Fail::new("invalid PGO output directory"));
                }
                opts.out.clone_from(raw);
                at += 2;
            }
            "--target-cpu" => {
                let raw = flags.get(at + 1).ok_or_else(usage)?;
                if !pgo::valid_target_cpu(raw) {
                    return Err(Fail::new("invalid --target-cpu value"));
                }
                opts.target_cpu.clone_from(raw);
                at += 2;
            }
            _ => {
                if let Some(raw) = arg.strip_prefix("--out=") {
                    if !pgo::valid_out_dir(raw) {
                        return Err(Fail::new("invalid PGO output directory"));
                    }
                    raw.clone_into(&mut opts.out);
                    at += 1;
                } else if let Some(raw) = arg.strip_prefix("--target-cpu=") {
                    if !pgo::valid_target_cpu(raw) {
                        return Err(Fail::new("invalid --target-cpu value"));
                    }
                    raw.clone_into(&mut opts.target_cpu);
                    at += 1;
                } else {
                    return Err(usage());
                }
            }
        }
    }
    Ok(Task::Pgo(opts))
}

fn parse_security_audio(flags: &[String]) -> Result<Task> {
    let usage = || Fail::new(USAGE);
    let mut opts = security_audio::Options::default();
    let mut at = 0;
    while at < flags.len() {
        let arg = flags[at].as_str();
        match arg {
            "--dry-run" => {
                opts.dry_run = true;
                at += 1;
            }
            "--force" => {
                opts.force = true;
                at += 1;
            }
            "--lang" => {
                let raw = flags.get(at + 1).ok_or_else(usage)?;
                security_audio::push_langs(&mut opts.langs, raw)?;
                at += 2;
            }
            "--voice-id" => {
                let raw = flags.get(at + 1).ok_or_else(usage)?;
                if !security_audio::valid_voice_id(raw) {
                    return Err(Fail::new("invalid ElevenLabs voice ID"));
                }
                opts.voice_id = Some(raw.to_owned());
                at += 2;
            }
            _ => {
                if let Some(raw) = arg.strip_prefix("--lang=") {
                    security_audio::push_langs(&mut opts.langs, raw)?;
                    at += 1;
                } else if let Some(raw) = arg.strip_prefix("--voice-id=") {
                    if !security_audio::valid_voice_id(raw) {
                        return Err(Fail::new("invalid ElevenLabs voice ID"));
                    }
                    opts.voice_id = Some(raw.to_owned());
                    at += 1;
                } else {
                    return Err(usage());
                }
            }
        }
    }
    Ok(Task::SecurityAudio(opts))
}

/// The task `args` name.
///
/// # Errors
/// An unknown task, an unknown flag, or more than one flag: the usage.
pub fn parse(args: &[String]) -> Result<Task> {
    let usage = || Fail::new(USAGE);
    let (task, flags) = args.split_first().ok_or_else(usage)?;
    if task == "security-audio" {
        return parse_security_audio(flags);
    }
    if task == "pgo" {
        return parse_pgo(flags);
    }
    let flag = match flags {
        [] => None,
        [one] => Some(one.as_str()),
        _ => return Err(usage()),
    };
    match (task.as_str(), flag) {
        ("gate", None) => Ok(Task::Gate { client: false }),
        ("gate", Some("--client")) => Ok(Task::Gate { client: true }),
        ("crypto-scope", None) => Ok(Task::CryptoScope),
        ("chrono-scope", None) => Ok(Task::ChronoScope),
        ("policy", None) => Ok(Task::Policy {
            compiler_probe: false,
        }),
        ("policy", Some("--compiler-probe")) => Ok(Task::Policy {
            compiler_probe: true,
        }),
        ("postgres", None) => Ok(Task::Postgres {
            exercise_failure: false,
            scope: postgres::Scope::All,
        }),
        ("postgres", Some("--exercise-failure")) => Ok(Task::Postgres {
            exercise_failure: true,
            scope: postgres::Scope::All,
        }),
        ("postgres", Some("--tower-only")) => Ok(Task::Postgres {
            exercise_failure: false,
            scope: postgres::Scope::Tower,
        }),
        ("postgres", Some("--tower-access-only")) => Ok(Task::Postgres {
            exercise_failure: false,
            scope: postgres::Scope::TowerAccess,
        }),
        ("postgres", Some("--tower-awards-only")) => Ok(Task::Postgres {
            exercise_failure: false,
            scope: postgres::Scope::TowerAwards,
        }),
        ("postgres", Some("--store-only")) => Ok(Task::Postgres {
            exercise_failure: false,
            scope: postgres::Scope::Store,
        }),
        ("smoke", None) => Ok(Task::Smoke {
            exercise_failure: false,
        }),
        ("smoke", Some("--exercise-failure")) => Ok(Task::Smoke {
            exercise_failure: true,
        }),
        ("secrets", None) => Ok(Task::Secrets),
        ("wasm", None) => Ok(Task::Wasm { check: false }),
        ("wasm", Some("--check")) => Ok(Task::Wasm { check: true }),
        ("site", None) => Ok(Task::Site),
        ("dev-site", None) => Ok(Task::DevSite),
        ("cards-ingest", None) => Ok(Task::CardsIngest { bulk_path: None }),
        ("cards-ingest", Some(path)) => Ok(Task::CardsIngest {
            bulk_path: Some(path.to_owned()),
        }),
        _ => Err(usage()),
    }
}

/// Runs the task `args` name, printing what passed.
///
/// # Errors
/// The first step that fails, as a fixed sentence.
pub fn run(args: &[String]) -> Result<()> {
    let task = parse(args)?;
    let root = workspace::root()?;
    match task {
        Task::Gate { client } => gate::run(&root, client),
        Task::CryptoScope => crypto_scope::check(&root),
        Task::ChronoScope => chrono_scope::check(&root),
        Task::Policy { compiler_probe } => {
            policy::check(&root, compiler_probe)?;
            println!("{}", policy::PASSED);
            Ok(())
        }
        Task::Postgres {
            exercise_failure,
            scope,
        } => postgres::run(&root, exercise_failure, scope),
        Task::Smoke { exercise_failure } => smoke::run(&root, exercise_failure),
        Task::Secrets => secrets::run(&root),
        Task::Wasm { check } => wasm::run(&root, check),
        Task::Site => site::run(&root),
        Task::DevSite => dev_site::run(&root),
        Task::CardsIngest { bulk_path } => cards_ingest::run(&root, bulk_path.as_deref()),
        Task::SecurityAudio(options) => security_audio::run(&root, &options),
        Task::Pgo(options) => pgo::run(&root, &options),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vec_args(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn parses_security_audio_flags() {
        let parsed = parse(&vec_args(&[
            "security-audio",
            "--lang",
            "en,de",
            "--voice-id=nPczCjzI2devNBz1zQrb",
            "--dry-run",
            "--force",
        ]))
        .expect("valid security-audio args");
        assert_eq!(
            parsed,
            Task::SecurityAudio(security_audio::Options {
                langs: vec!["en".to_owned(), "de".to_owned()],
                voice_id: Some("nPczCjzI2devNBz1zQrb".to_owned()),
                dry_run: true,
                force: true,
            })
        );
        assert!(parse(&vec_args(&["security-audio", "--api-key", "secret"])).is_err());
        assert!(parse(&vec_args(&["security-audio", "--lang", "xx"])).is_err());
        assert!(parse(&vec_args(&["security-audio", "--voice-id", "../bad"])).is_err());
    }

    #[test]
    fn parses_pgo_flags() {
        let parsed = parse(&vec_args(&[
            "pgo",
            "--out",
            "target/pgo-custom",
            "--target-cpu=x86-64-v3",
            "--profile-only",
        ]))
        .expect("valid pgo args");
        assert_eq!(
            parsed,
            Task::Pgo(pgo::Options {
                out: "target/pgo-custom".to_owned(),
                target_cpu: "x86-64-v3".to_owned(),
                profile_only: true,
            })
        );
        assert!(parse(&vec_args(&["pgo", "--target-cpu", "bad cpu"])).is_err());
    }
}
