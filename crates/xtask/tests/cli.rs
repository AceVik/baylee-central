//! The command line takes exactly the tasks and flags the scripts had.
#![forbid(unsafe_code)]

use xtask::cli::{Task, USAGE, parse};

fn args(line: &str) -> Vec<String> {
    line.split_whitespace().map(str::to_owned).collect()
}

#[test]
fn every_task_and_flag_parses() {
    assert_eq!(parse(&args("gate")), Ok(Task::Gate));
    assert_eq!(
        parse(&args("policy")),
        Ok(Task::Policy {
            compiler_probe: false
        })
    );
    assert_eq!(
        parse(&args("policy --compiler-probe")),
        Ok(Task::Policy {
            compiler_probe: true
        })
    );
    assert_eq!(
        parse(&args("postgres --exercise-failure")),
        Ok(Task::Postgres {
            exercise_failure: true
        })
    );
    assert_eq!(
        parse(&args("smoke")),
        Ok(Task::Smoke {
            exercise_failure: false
        })
    );
    assert_eq!(parse(&args("secrets")), Ok(Task::Secrets));
}

#[test]
fn anything_else_is_the_usage() {
    for line in [
        "",
        "deploy",
        "gate --compiler-probe",
        "policy --exercise-failure",
        "smoke --exercise-failure extra",
    ] {
        assert_eq!(
            parse(&args(line)).map_err(|f| f.message().to_owned()),
            Err(USAGE.to_owned()),
            "{line}"
        );
    }
}
