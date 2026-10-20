# Development tasks: `cargo xtask`

`crates/xtask` runs the repository's checks in Rust, one implementation for
Linux, macOS and Windows (owner, 2026-10-20). The PowerShell scripts under
`scripts/` state the same checks and remain for Windows development; both
fail with the same sentences, so a message means the same thing either way.

| task | does | PowerShell |
| --- | --- | --- |
| `cargo xtask gate` | `cargo fmt --check`, the policy with its compiler probe, clippy `-D warnings`, every test, the doc tests, the release build | `scripts/test.ps1` |
| `cargo xtask policy [--compiler-probe]` | the code policy (below) | `scripts/check-policy.ps1` |
| `cargo xtask postgres [--exercise-failure]` | the `PostgreSQL` integration fixture ([database development](database-development.md)) | `scripts/test-postgres.ps1` |
| `cargo xtask smoke [--exercise-failure]` | the development listener's HTTP boundary ([dev mode](dev-mode.md)); build `--release` first | `scripts/smoke-dev.ps1` |
| `cargo xtask secrets` | the secret scan of the tree and the full history, after the scanner's own regressions | `scripts/scan-secrets.ps1`, `scripts/test-secret-scanner.ps1` |

`cargo xtask` is an alias (`.cargo/config.toml`) for
`cargo run --locked --quiet -p xtask --`. The policy's regressions are
ordinary tests of the crate (`crates/xtask/tests/policy/`), so `cargo test`
and the gate run them; `scripts/test-policy.ps1` is their PowerShell twin.

## The code policy

- The workspace sets `unsafe_code = "forbid"`. Every member inherits the
  workspace lints, is not published (an absent `publish`, which allows
  crates.io, is refused too), lives inside the reviewed tree and outside the
  tool output, and every target's root says `#![forbid(unsafe_code)]`.
- The whole tree, hidden paths included, holds no symbolic link or junction,
  no C/C++ or assembly source, and no unsafe block, inline assembly or unsafe
  allowance in any `.rs` file (textually: comments and strings count). Only
  tool output is skipped: `.git`, `target` and `node_modules` at the root, and
  `web/node_modules` exactly, as a real directory. No Cargo target may live in
  any of them.
- Every dependency outside the workspace comes from crates.io at a version
  `docs/dependency-proposal/approved-packages.json` approves, with only the
  features it approves (names compared exactly); a path dependency must be a
  workspace member.
- With `--compiler-probe`, rustc must reject direct, macro-generated and
  `include!`d unsafe code under the forbid lint. The probes are text files in
  `crates/xtask/probes/`, since every `.rs` file is scanned for that very
  text; they are compiled only in a temporary directory.

## What the tasks guarantee

- No program runs through a shell; arguments are lists. A secret (a fixture
  password, a canary) reaches a child only through its environment or its
  standard input, never an argument or the output.
- Randomness comes from the operating system (`getrandom`).
- Temporary directories have random names, are removed with their contents,
  and are checked to be under the temporary directory first; links inside
  them are removed, never followed.
- Processes a task starts run in a process group of their own (Unix) and are
  stopped with their whole tree (Windows: `taskkill /T`); a task proves the
  process gone and its port refusing connections, also after a failure, and
  reports a cleanup that could not be proved as a failure.
- The `PostgreSQL` container is found by the random name chosen before it was
  created and removed only with the ownership label and the expected id.
- gitleaks 8.30.1 is pinned by SHA-256 for Linux (x86-64, ARM64), Windows
  (x86-64) and macOS (ARM64, x86-64), downloaded once into `target/gitleaks`
  and checked on every run. The Linux and Windows x86-64 values are the
  PowerShell script's; the others are from the release's checksum file, whose
  entries for those two match the pinned ones.

## Dependencies

Only crates the inventory already approves, with their approved features:
`serde_json`, `regex`, `getrandom` and `sha2` (without default features). No
HTTP client: the smoke test speaks HTTP over a socket, and the download runs
`curl` (Linux, macOS and Windows 10+ ship it) with HTTPS only.

## Differences from the PowerShell scripts

- An unset `publish` is refused (the PowerShell check now refuses it too).
- gitleaks has pins for macOS and Linux ARM64 besides the CI platforms.
- The PowerShell scripts restore environment variables they changed; the Rust
  tasks never change their own environment, only their children's.
