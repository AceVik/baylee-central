# Development tasks: `cargo xtask`

`crates/xtask` runs the repository's checks in Rust, one implementation for
Linux, macOS and Windows (owner, 2026-10-20). It replaced the PowerShell
scripts under `scripts/` (removed the same day), keeping their checks and their
sentences, so a message means what it meant before. The alias builds it
under its own profile (`xtask`, in `target/xtask`), so the workspace builds
the gate starts never replace the running executable, which Windows refuses.

| task | does | replaced |
| --- | --- | --- |
| `cargo xtask gate` | `cargo fmt --check`, the policy with its compiler probe, clippy `-D warnings`, every test, the doc tests, the release build | `scripts/test.ps1` |
| `cargo xtask crypto-scope` | exact graph and source bounds of the unreachable bigint advisory; also mandatory in `gate` | (new, 2026-10-28) |
| `cargo xtask policy [--compiler-probe]` | the code policy (below) | `scripts/check-policy.ps1` |
| `cargo xtask postgres [--exercise-failure]` | the `PostgreSQL` integration fixture ([database development](database-development.md)) | `scripts/test-postgres.ps1` |
| `cargo xtask smoke [--exercise-failure]` | the development listener's HTTP boundary ([dev mode](dev-mode.md)); build `--release` first | `scripts/smoke-dev.ps1` |
| `cargo xtask secrets` | the secret scan of the tree and the full history, after the scanner's own regressions | `scripts/scan-secrets.ps1`, `scripts/test-secret-scanner.ps1` |
| `cargo xtask wasm [--check]` | `central-web-crypto` built to WebAssembly with its wasm-bindgen glue, written to `web/src/central-wasm/`; `--check` requires the committed files to equal a fresh build | (new, 2026-10-21) |

`cargo xtask` is an alias (`.cargo/config.toml`) for
`cargo run --locked --quiet -p xtask --`. The policy's regressions are
ordinary tests of the crate (`crates/xtask/tests/policy/`), so `cargo test`
and the gate run them (they replaced `scripts/test-policy.ps1`). CI runs
every task under bash on all four runners.

## The code policy

The gate additionally enforces the reviewed Ristretto advisory scope through
`crypto-scope` ([review and removal condition](crypto-proposal/bigint-advisory.md)).
CI's dependency job runs it before the pinned audit and deny tools. A scanner
pass alone does not validate the exception's reachability bounds.

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

## The website's WebAssembly (`wasm`)

The browser crypto of [web accounts](web-accounts.md) is a Rust crate the
website runs as WebAssembly. Its build is committed under
`web/src/central-wasm/` so that the website builds without Rust; this task
makes it and proves it:

- `cargo build -p central-web-crypto --target wasm32-unknown-unknown
  --profile wasm` (one codegen unit, link-time optimisation, panics abort),
  with `--remap-path-prefix` for the repository (`/central`) and Cargo's home
  (`/cargo`), so the module holds no local path; a module that still names
  either fails the task.
- The wasm-bindgen CLI at 0.2.129, the version of the crate's
  `wasm-bindgen` dependency, installed with `cargo install --locked` into
  the target directory's `wasm-bindgen-cli/0.2.129` on first use and checked
  by its `--version` on every run; `--target web`, with the name and
  producers sections removed.
- The four files and `SHA256SUMS` (the SHA-256 of each) go to
  `web/src/central-wasm/`; `--check` writes them to a temporary directory
  instead and fails unless the committed directory holds exactly those files
  (and its `README.md`), equal byte for byte, with equal sums, and names the
  sizes and the first differing byte when they are not. CI runs the check.
- **Only on x86-64 Linux (glibc)**, CI's host, and the task refuses any
  other. Cargo folds the build host into every crate's `-C metadata`; that
  changes the symbols' names and, through link-time optimisation, the
  module's layout, so a build on macOS differs from CI's in every function
  although no path or flag does (measured 21.10.2026: the crate's
  disambiguator `Csl73WTcpK3im` on Linux, `CsgacuhQaamlp` on macOS). Remapping
  cannot change that, and cargo's `trim-paths` is not stable in 1.99. The
  build is reproducible on that host: two machines' builds there are equal.

From any other machine, the same task runs in the pinned Rust image
(x86-64, emulated on ARM; slow the first time), its target directory beside
the host's so neither replaces the other's binaries:

```bash
docker run --rm --platform linux/amd64 -v "$PWD":/work -w /work -e CARGO_TARGET_DIR=target/linux-amd64 rust:1.99.0-bookworm@sha256:114c7a4425406451c2866b6aafe69fe29b1b298832db1277d411ac73c82d04d6 bash -c 'rustup target add wasm32-unknown-unknown && cargo xtask wasm'
```

(`cargo xtask wasm --check` in its place compares instead of writing.) On a
Linux host the container writes `target/linux-amd64` and the five files as
root; give them back with `chown`.

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

## Tools it calls

`cargo`, `rustc`, `git`, and for the tasks that need them `docker` and
`openssl` (postgres), `curl` and `tar` (secrets: Windows 10 1803 and later
ship both, as do macOS and the CI runners), and the wasm-bindgen CLI it
installs itself (wasm). A missing tool fails its step
with that step's sentence.

## Dependencies

Only crates the inventory already approves, with their approved features:
`serde_json`, `regex`, `getrandom` and `sha2` (without default features). No
HTTP client: the smoke test speaks HTTP over a socket, and the download runs
`curl` (Linux, macOS and Windows 10+ ship it) with HTTPS only.

## What changed against the PowerShell scripts

- An unset `publish` is refused (the PowerShell check had accepted it).
- gitleaks has pins for macOS and Linux ARM64 besides the CI platforms.
- The PowerShell scripts set and restored environment variables; the Rust
  tasks never change their own environment, only their children's.
