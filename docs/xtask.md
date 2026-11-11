# Development tasks: `cargo xtask`

`crates/xtask` runs the repository's checks in Rust, one implementation for
Linux, macOS and Windows (owner, 2026-10-20). It replaced the PowerShell
scripts under `scripts/` (removed the same day), keeping their checks and their
sentences, so a message means what it meant before. The alias builds it
under its own profile (`xtask`, in `target/xtask`), so the workspace builds
the gate starts never replace the running executable, which Windows refuses.

| task | does | replaced |
| --- | --- | --- |
| `cargo xtask gate [--client]` | `cargo fmt --check`, the policy with its compiler probe, clippy `-D warnings`, every test, the doc tests, the release build; `--client` limits clippy and the tests to the client's crates and drops the release build ([below](#which-runner-runs-what)) | `scripts/test.ps1` |
| `cargo xtask crypto-scope` | exact graph and source bounds of the unreachable bigint advisory; also mandatory in `gate` | (new, 2026-10-28) |
| `cargo xtask policy [--compiler-probe]` | the code policy (below) | `scripts/check-policy.ps1` |
| `cargo xtask postgres [--exercise-failure]` | the `PostgreSQL` integration fixture ([database development](database-development.md)) | `scripts/test-postgres.ps1` |
| `cargo xtask smoke [--exercise-failure]` | the development listener's HTTP boundary ([dev mode](dev-mode.md)); build `--release` first | `scripts/smoke-dev.ps1` |
| `cargo xtask secrets` | the secret scan of the tree and the full history, after the scanner's own regressions | `scripts/scan-secrets.ps1`, `scripts/test-secret-scanner.ps1` |
| `cargo xtask wasm [--check]` | `central-web-crypto` built to WebAssembly with its wasm-bindgen glue, written to `web/src/central-wasm/`; `--check` requires the committed files to equal a fresh build | (new, 2026-10-21) |
| `cargo xtask site` | the server-rendered site's assets into `target/site/` ([below](#the-sites-assets-site)) | (new, 2026-11-10) |

`cargo xtask` is an alias (`.cargo/config.toml`) for
`cargo run --locked --quiet -p xtask --`. The policy's regressions are
ordinary tests of the crate (`crates/xtask/tests/policy/`), so `cargo test`
and the gate run them (they replaced `scripts/test-policy.ps1`). CI runs the
tasks under bash, on the runners the next section names.

## Which runner runs what

Production runs on one Linux x86-64 host, and only the Baylee client must be
built for every system and architecture (owner, 29.10.2026). So CI builds and
tests Central's server, `central-service` and the crates only it needs, on
Linux x86-64 alone, and checks on Windows and macOS just what the Baylee
desktop client links from this repository.

| runner | runs |
| --- | --- |
| ubuntu-24.04 | `gate` (the whole workspace, release build included), then `smoke` and `smoke --exercise-failure`; the separate Linux jobs run `postgres`, `crypto-scope` with the audit and deny tools, `secrets`, `wasm --check` and the web app |
| windows-2025, macos-15 (ARM64), macos-15-intel (x86-64) | `gate --client` |

`gate --client` runs the format check (the whole tree), the policy with its
compiler probe and the crypto scope exactly as `gate` does, then clippy
`-D warnings`, the tests and the doc tests with all features and a `-p` for
each package of its set. It skips the release build: Baylee's packaging builds
those crates in release for all six client targets.

The set is read from `cargo metadata`, not listed by hand: `central-api` and
`central-client`, the two crates Baylee pins in its root `Cargo.toml`, and
every workspace member they reach through normal or build dependencies.
Development dependencies serve only their own tests, so they do not count.
Today that is `central-api`, `central-client` and `central-crypto`. The task
prints its set first, and a unit test (`crates/xtask/src/client/tests.rs`)
pins it, so a new dependency changes this page as well. `xtask` joins the set
because its policy regressions (links, junctions, case-insensitive paths)
depend on the platform the gate runs on.

Two limits remain:

- Testing `central-client` still compiles the server crates (`central-http`,
  `central-identity`, `central-postgres` and `central-migration`, with SeaORM
  and Tokio) in debug on those runners. They are development dependencies of
  its PostgreSQL integration test (`tests/postgres.rs`, ignored there and run
  by `cargo xtask postgres`), and cargo builds every development dependency
  for any test target of a package. Their own tests do not run there.
- Nothing checks the server on Windows or macOS any more. It may still build
  there, but CI no longer shows whether it does.

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
- The wasm-bindgen CLI at 0.2.130, the version of the crate's
  `wasm-bindgen` dependency, installed with `cargo install --locked` into
  the target directory's `wasm-bindgen-cli/0.2.130` on first use and checked
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

An isolated x86-64 Linux/glibc VM is an equivalent build host when container
emulation is unavailable. Install the same pinned Rust and WASM target, transfer
only the source/lockfile, run both `cargo xtask wasm` and `cargo xtask wasm --check`,
and copy back only the four generated files plus `SHA256SUMS` after verifying the
hashes. Keep credentials outside the VM and close its owned process/listener after
building; there is no need to modify or restart unrelated container services.

The 2026-11-02 chat bundle passed that build and byte comparison in an isolated
QEMU 11.1.3 VM using the official [Ubuntu 24.04 amd64 image, 20261017](https://cloud-images.ubuntu.com/noble/20261017/),
verified against its published SHA-256
`04e8f2461a16a44e78a370284471f29616cdd9baf1893df25807b71f985b6c94`,
and Rust 1.99.0. The resulting WASM SHA-256 is
`c24adad5b8a4eaa4bed0f2ed1e44a3a2bd00505959b0737583b5962b50870c36`.
The task's host restriction and byte comparison were not bypassed.

The 2026-11-03 bundle (`Chat::keep`, `chat_resume`) was built with the command
above, in the pinned Rust image on Apple Silicon under Docker Desktop's Rosetta
emulation of x86-64 (QEMU emulation crashed `rustc`), and `cargo xtask wasm
--check` passed in the same container. The resulting WASM SHA-256 is
`7983d3ec71c4702ab124660038d92a0a0b3f76c40bfa644e440fcc8574f46025`; CI's
`wasm` job checks it again on its own runner.

## The site's assets (`site`)

The server-rendered website (`crates/central-web`, [the proposal](proposals/frontend-rewrite.md))
is served by `central-web` from `target/site/`, which this task writes:

- the islands: `central-web-islands` built for `wasm32-unknown-unknown`
  under the `islands` profile (size-optimised, one codegen unit, LTO,
  `panic = "abort"`), paths remapped as `wasm` remaps them (a module that
  still names the repository or Cargo's home fails), then the pinned
  wasm-bindgen CLI (the same 0.2.130 install as `wasm`); and
  `crates/central-web/js/islands.js`, the bootstrap, with the hashed
  addresses of the glue and the module written in;
- the stylesheet: `crates/central-web/style/` joined in the order of
  `site::STYLES`, every `url("/assets/<name>")` rewritten to the hashed
  file (a missing one fails);
- `boot.js`, `pwa.js` and `pow-worker.js`, the crypto Worker with the
  committed `central-web-crypto` module (checked against its `SHA256SUMS`),
  the fonts and pictures of `crates/central-web/assets/`;
- every file under `/assets/<name>.<sha-256 prefix>.<ext>` and recorded in
  `manifest.json` with its address and a `sha384-` Subresource Integrity
  value; `/favicon.ico` and `/apple-touch-icon.png` at the root;
- `/sw.js` at the root with this build's stamp (the digest of every
  asset's address, so any changed file makes a new service worker) and the
  hashed files the offline page needs written over its two placeholders
  (a changed placeholder fails);
- `notices.txt`: the fonts' and icons' licences and every crate in the
  islands' module with its licence files, each text once (read with
  `cargo metadata --locked --offline`: on a machine that built only this
  site, run `cargo fetch --locked` first, as CI's `site` job does);
- Brotli and gzip copies of every text file where `brotli` and `gzip` are
  on the `PATH` (without them, Caddy compresses);
- with `BAYLEE_SRC` set to a Baylee checkout (the deploy hook has one), the
  client's screenshots (`web/site/img/shot-*`) and a fallback card status
  (`fallback/cards.json` from `web/site/data/sets.json` and the newest tag).
  Both carry Wizards' material and never enter this repository.

Unlike `wasm`, the output is not committed and not checked byte for byte:
it is built where it is deployed.

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
