//! The command line: `cargo xtask <task> [flag]`, nothing else.

use crate::fail::{Fail, Result};
use crate::{gate, policy, postgres, secrets, smoke, workspace};

/// What to run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Task {
    /// fmt, the policy with its compiler probe, clippy, the tests, the doc
    /// tests and the release build.
    Gate,
    /// The code policy.
    Policy {
        /// Also check that rustc rejects unsafe code under the forbid lint.
        compiler_probe: bool,
    },
    /// The `PostgreSQL` integration fixture.
    Postgres {
        /// Fail on purpose once the fixture is up, to prove its cleanup.
        exercise_failure: bool,
    },
    /// The development listener smoke test.
    Smoke {
        /// Fail on purpose once the listener is up, to prove its cleanup.
        exercise_failure: bool,
    },
    /// The secret scan of the tree and its history.
    Secrets,
}

/// How to call it.
pub const USAGE: &str = "usage: cargo xtask <task>

tasks:
  gate                          fmt, policy, clippy, tests, doc tests, release build
  policy [--compiler-probe]     the code policy: unsafe, native sources, links, dependencies
  postgres [--exercise-failure] the PostgreSQL integration fixture (needs Docker and OpenSSL)
  smoke [--exercise-failure]    the development listener smoke test (build --release first)
  secrets                       the secret scan of the tree and the full history (pinned gitleaks)";

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
        ("gate", None) => Ok(Task::Gate),
        ("policy", None) => Ok(Task::Policy {
            compiler_probe: false,
        }),
        ("policy", Some("--compiler-probe")) => Ok(Task::Policy {
            compiler_probe: true,
        }),
        ("postgres", None) => Ok(Task::Postgres {
            exercise_failure: false,
        }),
        ("postgres", Some("--exercise-failure")) => Ok(Task::Postgres {
            exercise_failure: true,
        }),
        ("smoke", None) => Ok(Task::Smoke {
            exercise_failure: false,
        }),
        ("smoke", Some("--exercise-failure")) => Ok(Task::Smoke {
            exercise_failure: true,
        }),
        ("secrets", None) => Ok(Task::Secrets),
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
        Task::Gate => gate::run(&root),
        Task::Policy { compiler_probe } => {
            policy::check(&root, compiler_probe)?;
            println!("{}", policy::PASSED);
            Ok(())
        }
        Task::Postgres { exercise_failure } => postgres::run(&root, exercise_failure),
        Task::Smoke { exercise_failure } => smoke::run(&root, exercise_failure),
        Task::Secrets => secrets::run(&root),
    }
}
