//! The command line: `cargo xtask <task> [flag]`, nothing else.

use crate::fail::{Fail, Result};
pub use crate::postgres::Scope as PostgresScope;
use crate::{
    crypto_scope, gate, policy, postgres, secrets, site, smoke, sqlite_scope, wasm, workspace,
};

/// What to run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    /// The bound of the `libsqlite3-sys` advisory's exception: in the lock
    /// file only, in no build.
    SqliteScope,
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
}

/// How to call it.
pub const USAGE: &str = "usage: cargo xtask <task>

tasks:
  crypto-scope                 verify the reviewed Ristretto advisory scope
  sqlite-scope                 verify libsqlite3-sys is in Cargo.lock only, in no build
  gate [--client]               fmt, policy, clippy, tests, doc tests, release build;
                                --client: only the client's crates, no release build
  policy [--compiler-probe]     the code policy: unsafe, native sources, links, dependencies
  postgres [--exercise-failure | --store-only | --tower-only] the PostgreSQL integration fixture (needs Docker and OpenSSL)
  smoke [--exercise-failure]    the development listener smoke test (build --release first)
  secrets                       the secret scan of the tree and the full history (pinned gitleaks)
  wasm [--check]                the website's WebAssembly crypto and glue (crates/central-web/crypto)
  site                          the website's assets into target/site (BAYLEE_SRC: screenshots)";

/// The task `args` name.
///
/// # Errors
/// An unknown task, an unknown flag, or more than one flag: the usage.
pub fn parse(args: &[String]) -> Result<Task> {
    let usage = || Fail::new(USAGE);
    let (task, flags) = args.split_first().ok_or_else(usage)?;
    let flag = match flags {
        [] => None,
        [one] => Some(one.as_str()),
        _ => return Err(usage()),
    };
    match (task.as_str(), flag) {
        ("gate", None) => Ok(Task::Gate { client: false }),
        ("gate", Some("--client")) => Ok(Task::Gate { client: true }),
        ("crypto-scope", None) => Ok(Task::CryptoScope),
        ("sqlite-scope", None) => Ok(Task::SqliteScope),
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
        Task::SqliteScope => sqlite_scope::check(&root),
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
    }
}
