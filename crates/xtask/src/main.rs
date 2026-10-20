//! `cargo xtask <task>`: Baylee Central's development tasks (`docs/xtask.md`).
#![forbid(unsafe_code)]

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match xtask::run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(fail) => {
            eprintln!("{fail}");
            ExitCode::FAILURE
        }
    }
}
