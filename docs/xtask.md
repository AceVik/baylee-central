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
| `cargo xtask chrono-scope` | `chrono` (RUSTSEC-2026-0543) is in `Cargo.lock` at 0.4.45 only and in no build; also mandatory in `gate` | (new, 2026-11-18; `sqlite-scope` before it, 2026-11-12 to 2026-11-18) |
| `cargo xtask policy [--compiler-probe]` | the code policy (below) | `scripts/check-policy.ps1` |
| `cargo xtask postgres [--exercise-failure]` | the `PostgreSQL` integration fixture ([database development](database-development.md)) | `scripts/test-postgres.ps1` |
| `cargo xtask smoke [--exercise-failure]` | the development listener's HTTP boundary ([dev mode](dev-mode.md)); build `--release` first | `scripts/smoke-dev.ps1` |
| `cargo xtask secrets` | the secret scan of the tree and the full history, after the scanner's own regressions | `scripts/scan-secrets.ps1`, `scripts/test-secret-scanner.ps1` |
| `cargo xtask wasm [--check]` | `central-web-crypto` built to WebAssembly with its wasm-bindgen glue, written to `crates/central-web/crypto/`; `--check` requires the committed files to equal a fresh build | (new, 2026-10-21) |
| `cargo xtask site` | the server-rendered site's assets into `target/site/` ([below](#the-sites-assets-site)) | (new, 2026-11-10) |
| `cargo xtask pgo [--compare] [--profile-only \| --profile-use <file>]` | the website and the picture worker trained, built from their profile and checked that it took, `central-service` beside them; `--compare` measures them against the deploy hook's build ([below](#profile-guided-builds-pgo)) | (new, 2026-11-25; rebuilt 2026-11-28) |

`cargo xtask` is an alias (`.cargo/config.toml`) for
`cargo run --locked --quiet -p xtask --`. The policy's regressions are
ordinary tests of the crate (`crates/xtask/tests/policy/`), so `cargo test`
and the gate run them (they replaced `scripts/test-policy.ps1`). CI runs the
tasks under bash, on the runners the next section names.

`cargo xtask postgres --store-only` reruns the store/migration suite without
repeating an already successful SDK transport suite. The default still requires
both. `cargo xtask postgres --tower-only` runs the expedition authority regression
against the same owned PostgreSQL 18 fixture and cleanup checks, omitting the
unrelated account/HTTP suites. It is a focused development check, not a replacement
for the full `postgres` release gate, which also includes the expedition tests.
`cargo xtask postgres --tower-access-only` exercises just native Tower identity
issuance/introspection through the real SDK, HTTP listener and PostgreSQL, including
credential separation, concurrent refresh bounds and revocation. It uses the same
owned fixture and cleanup checks. The full command includes it as well.

`cargo xtask postgres --tower-awards-only` runs the v2 permanent award store and
native SDK/HTTP slices: restricted issuer grants, atomic/deduplicated progression,
receipt ordering, bounded account export, migration and deletion races, credential
scope confusion, browser refusal and bound SDK acknowledgments. It does not run
game rules. The full PostgreSQL command includes both scenarios.

## Which runner runs what

Production runs on one Linux x86-64 host, and only the Baylee client must be
built for every system and architecture (owner, 29.10.2026). So CI builds and
tests Central's server, `central-service` and the crates only it needs, on
Linux x86-64 alone, and checks on Windows and macOS just what the Baylee
desktop client links from this repository.

| runner | runs |
| --- | --- |
| ubuntu-24.04 | `gate` (the whole workspace, release build included), then `smoke` and `smoke --exercise-failure`; the separate Linux jobs run `postgres`, `crypto-scope` and `chrono-scope` with the audit and deny tools, `secrets`, `wasm --check` and the web app |
| windows-2025, macos-15 (ARM64), macos-15-intel (x86-64) | `gate --client` |

The verification job has a 40-minute ceiling. On 14 November 2026 its former
25-minute ceiling canceled a cold Linux release build after the tests and doc
tests had passed, while `central-service` was still compiling. The larger bound
retains the complete gate and the lifecycle smoke checks; it does not change
their assertions, per-operation deadlines or which checks are required to merge.

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
It enforces the `chrono` advisory's exception the same way through
`chrono-scope` ([the bound and its removal](dependency-proposal/chrono-advisory.md)):
the package may be in the lock file at 0.4.45 and in no build. (Until
18.11.2026 `sqlite-scope` held `libsqlite3-sys` the same way;
[ended](dependency-proposal/sqlite-advisory.md).)

- The workspace sets `unsafe_code = "forbid"`. Every member inherits the
  workspace lints, is not published (an absent `publish`, which allows
  crates.io, is refused too), lives inside the reviewed tree and outside the
  tool output, and every target's root says `#![forbid(unsafe_code)]`.
- The whole tree, hidden paths included, holds no symbolic link or junction,
  no C/C++ or assembly source, and no unsafe block, inline assembly or unsafe
  allowance in any `.rs` file (textually: comments and strings count). Only
  tool output is skipped: `.git`, `target` and `node_modules` at the root, and
  `crates/central-web/browser/node_modules` exactly, as a real directory. No Cargo target may live in
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
`crates/central-web/crypto/` so that the website builds without Rust; this task
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
  `crates/central-web/crypto/`; `--check` writes them to a temporary directory
  instead and fails unless the committed directory holds exactly those files
  (and its `README.md`), equal byte for byte, with equal sums, and names the
  sizes and the first differing byte when they are not. CI runs the check.
- **Only on x86-64 Linux (glibc)**, CI's host, and the task refuses any
  other. Cargo folds the build host into every crate's `-C metadata`; that
  changes the symbols' names and, through link-time optimisation, the
  module's layout, so a build on macOS differs from CI's in every function
  although no path or flag does (measured 21.10.2026: the crate's
  disambiguator `Csl73WTcpK3im` on Linux, `CsgacuhQaamlp` on macOS). Remapping
  cannot change that, and cargo's `trim-paths` is not stable in 1.100. The
  build is reproducible on that host: two machines' builds there are equal.

From any other machine, the same task runs in the pinned Rust image
(x86-64, emulated on ARM; slow the first time), its target directory beside
the host's so neither replaces the other's binaries:

```bash
docker run --rm --platform linux/amd64 -v "$PWD":/work -w /work -e CARGO_TARGET_DIR=target/linux-amd64 rust:1.100.0-bookworm@sha256:75d26c23c24cc6ea1cdabf0714a6bff8fd7bf3248716043fd7ae86dac7f41b94 bash -c 'rustup target add wasm32-unknown-unknown && cargo xtask wasm'
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

The 2026-11-26 bundle is the same source built by Rust 1.100.0, the
toolchain move of that day: on x86-64 Linux (glibc), CI's host, with
`cargo xtask wasm` and then `cargo xtask wasm --check`, which passed. Only
the module changed (613 689 to 614 363 bytes); the glue and both type files
are equal. The resulting WASM SHA-256 is
`286fff87f401ee1d90b3509bc5b526a8efec3a073ae19903aab8b4523c7ab1fe`.

The 2026-11-12 bundle (`reset_start`, `reset_keep`, `reset_new`,
`password_start`, `password_finish`) was built the same way, with the
command above under Rosetta, and `cargo xtask wasm --check` passed in the
same container. The resulting WASM SHA-256 is
`abeaad928229eb30338a15db4bdc9d5864ed85ffa07c882521675d5cceebbc39`.

The 2026-11-13 bundle is the same crate built from the combined sources of
the session tokens (#63), the website (#64) and the profile pictures (#65):
`central-api`'s `Error::UpdateRequired`, which the chat's errors name, and
the signed token in `central-client`'s session. Its exports and glue did not
change. Built the same way, under Rosetta, and `cargo xtask wasm --check`
passed in the same container. The resulting WASM SHA-256 is
`0c7ae18b463d4dbcf3db0ddfec091b09cf2d3004cde2cd3268ec455ffca3d1b1`.

The 2026-11-17 bundle is the same crate built from the Tower branch's
sources: `central-api`'s and `central-client`'s Tower operations, and
`enrollment_finish` in `central-web-crypto`'s `flow.rs`. Its three glue
files did not change. Built with the command above, in the pinned Rust image
on Apple Silicon under Docker Desktop's x86-64 emulation, and `cargo xtask
wasm --check` passed in the same container. The resulting WASM SHA-256 is
`dc2af609bbd78db237d96a87e2a242f8c9773596a486c09cbebff86b1dffba31`.
`cargo xtask site` takes the committed module (checked against its
`SHA256SUMS`) and builds none of its own.

**No SIMD** (measured 28.11.2026): the module's Argon2id, the browser's
key stretching (64 MiB, three passes, four lanes, RustCrypto `argon2` 0.6,
which has no WebAssembly SIMD path of its own), built once without and once
with `-C target-feature=+simd128` in a scratch crate, took 133 against
164 ms in Node 25 (V8) on Apple Silicon, the median of seven alternating
runs (137 against 169 and 147 against 176 in two repeats): the compiler's
own vectorisation makes it slower. The module stays without `simd128`,
which would also lock browsers before Safari 16.4 out of signing in. A
faster stretch would need a SIMD kernel, which this repository does not
write itself (no custom cryptographic code); a reviewed crate with one would
be the owner's decision.

## The site's assets (`site`)

The server-rendered website (`crates/central-web`, [the proposal](proposals/frontend-rewrite.md))
is served by `central-web` from `target/site/`, which this task writes:

- the islands, as two modules (`site::MODULES`): `central-web-islands`
  built for `wasm32-unknown-unknown` under the `islands` profile
  (size-optimised, one codegen unit, LTO, `panic = "abort"`), paths
  remapped as `wasm` remaps them (a module that still names the repository
  or Cargo's home fails), then the pinned wasm-bindgen CLI (the same
  0.2.130 install as `wasm`); once without features, the public module
  (`islands.wasm`, `islands-glue.js`), and once in a cargo run of its own
  with `--features admin`, the administration's (`admin-islands.wasm`,
  `admin-islands-glue.js`), which only `/{lang}/account/admin…` loads
  ([the proposal](proposals/frontend-rewrite.md) §27). Each gets
  `crates/central-web/js/islands.js`, the bootstrap, with the hashed
  addresses of its glue and its module written in (`islands.js`,
  `admin-islands.js`). A public module that holds one of the
  administration's marks (`site::ADMIN_ONLY`), or an administration's that
  lacks one, fails the build. Both are built with the toolchain's baseline
  features and `simd128` off, then shrunk with `wasm-opt -Oz`, which is not
  given SIMD either, where it is installed (`CENTRAL_REQUIRE_WASM_OPT=1`
  requires it, as CI does; about 15 % smaller). From 25 to 28.11.2026 they
  were built with `simd128`. Measured 28.11.2026 on the public module after
  wasm-bindgen: with it 5 872 428 bytes (Brotli 1 093 039), without
  5 972 556 (1 103 789), so SIMD made it 1 % smaller after compression, and
  the compiler used it in about 22 000 places; its effect on hydration was
  not measured. With it browsers before Safari 16.4 (iOS 15) could not run
  the islands, and so could not sign in, which the open beta should not
  cost for 1 %;
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
  islands' modules (the same graph for both) with its licence files, each
  text once (read with
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

## Profile-guided builds (`pgo`)

`cargo xtask pgo` builds what the production host runs (Linux x86-64,
`--target-cpu x86-64-v3` unless another is given) under `[profile.release-pgo]`:
fat link-time optimisation and one codegen unit. Two of the three binaries
are built from a profile of their own work:

- `central-web`, the server-rendered website, trained in its deployed mode
  (`--config-stdin`, no GitHub, no Central) on the site `cargo xtask site`
  built and the synthetic data of `crates/central-web-server/tests/data`:
  every page its sitemap lists in every language (so a page added there is
  trained without a change to the task; one it does not list must be added
  to `SLUGS` in `crates/xtask/src/pgo/train.rs`), and under each of the
  eleven language prefixes the listed pages, the sign-in,
  registration, reset and offline pages, the account area as a visitor
  without a session gets it, a player's profile, an invitation, a removed
  page (410) and a missing one (404); the addresses without a language in
  several browsers' languages (the redirects, `robots.txt`, the sitemap,
  `funding.json`, the service worker); the card suggestions; every hashed
  asset with Brotli, gzip and neither, and again with its `ETag`. Six
  passes, with a count of page views as production may keep one.
- `central-picture-worker`, one process per picture as systemd starts it:
  pictures ffmpeg draws from its `testsrc2` pattern in each format a browser
  uploads (JPEG at 1600 × 1200 and 720 × 1280, PNG with and without alpha,
  lossy and lossless WebP, a still and a moving GIF, a BMP), an ICO around
  the drawn PNG, an SVG, the codec tests' fixtures (AVIF, JPEG, WebP), the
  worker's own AVIF answer read back, a photo cut short, a PNG declared as a
  JPEG and bytes that are no picture. ffmpeg (with libwebp, as Ubuntu's has
  it) must be on the `PATH`; no photo, no person.

`central-service` is built the same way without a profile. Under a profile,
code the training never ran counts as cold and is optimised for size
(LLVM's profile-guided size optimisation), so a training that misses the real
work makes that work slower. The service's work is sign-in, its database and
the push socket, which need a database to train against; its development
mode answers only its health checks. It stays without a profile until a
training of its real requests exists.

The steps, each of which stops the task when it fails:

1. The two binaries instrumented (`-C profile-generate`), in `target/pgo-gen`.
2. The training above; the counts go to `<out>/profraw`.
3. `llvm-profdata merge` (the toolchain's `llvm-tools`) into
   `<out>/central.profdata`, and `rustc -vV` beside it in
   `central.profdata.rustc`: a profile is read only by the compiler that
   made it.
4. The two binaries from the profile (`-C profile-use`) in `target/pgo-use`,
   emptied first: the flag names the same file whatever it holds, so cargo
   cannot see a new profile, and a crate no commit touched would keep the
   code an older profile made. `central-service` in `target/pgo-plain`.
5. The profile must have taken. The build asks the compiler to name every
   function it found no counts for (`-pgo-warn-missing-function`, and a
   changed function's `hash mismatch`); the task counts those lines, shows
   three, and fails above 5 % of the profile's own total of functions
   (`llvm-profdata show`). A profile that matches nothing still builds
   binaries that work: an unprofiled build under another name. Both builds
   select the same packages with the same flags, so a profile that took
   matches nearly every function.

Then `<out>/dist/` holds the three binaries and their `SHA256SUMS.txt`.
Every build passes `--target` with the host's triple, so the flags reach what
ships and no build script or procedural macro, which would otherwise be
instrumented too and write counts while the build runs.

- `--profile-only` stops after step 3.
- `--profile-use <file>` skips steps 1 to 3 and builds from a profile made
  elsewhere, which must have its `.rustc` file beside it naming this exact
  toolchain.
- `--compare` builds what the deploy hook builds today (`cargo build
  --release` with the same `-C target-cpu`, `target/pgo-baseline`) and times
  both builds' `central-web` on a pass over the pages and both workers on
  four pictures: nine rounds each after a warm-up, the builds taking turns
  to go first, medians compared. A report and not a gate: a shared runner's
  load moves the figures. A page's time includes the client's, so the
  server's own gain is larger than the figure shown.

`release.yml` runs `cargo xtask pgo --compare` for every tag and by hand
(`workflow_dispatch`). Before it, a one-line program is built from a profile
of one function it does not have, and the compiler's report must word a
function without counts as step 5 reads it (`no profile data available for
function`). By hand, `portability-check` builds from the profile again in a
checkout and a Cargo home at other paths, as a server that builds from CI's
profile does.

Measured on 28.11.2026 on ubuntu-26.04 runners with four vCPUs (release
runs 46102057709 and 46113854970, the same commit's code): 49 count files
merged into a profile of 89 709 functions (55 MiB), of which the build from
it reported none without counts, and none at the other paths either. The
instrumented build took 14 to 19 minutes, the training 5 to 8 seconds, the
build from the profile 10 to 13 minutes and `central-service` beside it 4 to
8. Against the deploy hook's build, `--compare` gave 4 and 9 % less time for
a pass over the website's 331 addresses (214.2 against 205.1 ms, 117.9
against 107.0 ms; the client's time included) and 3 to 9 % less for a
picture, each in a worker process of its own (`photo.jpg` 5 and 3 %,
`screen.png` 9 and 5 %, `lossy.webp` 8 and 6 %, `moving.gif` 6 and 3 %).

A profile that matches nothing is no cheap failure. Built from a profile of
one function no build has, the whole selection did not finish in 80 minutes
(46102057709) nor in 50 (46113854970). In the second run its log grew to
57 829 lines within seven minutes, nearly all of them the blank line the
compiler writes after each report of a function without counts, which the
task counted and did not show (the last six lines were such blank lines, so
the reports reached the count through cargo); then it stood in
`central-web-server` for the rest of the time. Step 5 would refuse such a
build, but only after it; a profile is therefore only ever used for the
commit and the toolchain that made it, and a caller with a deadline has to
bound the build.

`profile.yml` trains a profile of every commit on main (`--profile-only`)
and puts it in the public pre-release `central-profiles`; the deploy hook
builds that commit from it on the server with `--profile-use`, bounded in
time, and without a profile as before
([operations](operations.md#deploying-with-the-gateway)).

No profile reaches the website's WebAssembly: the stable toolchain has no
profiling runtime for `wasm32-unknown-unknown` (`-C profile-generate` there
fails for want of `profiler_builtins`, tried with 1.100.0 on 28.11.2026). A
native build's profile is not expected to stand in (not tried): the
functions' names carry crate hashes that cargo derives with the target, so
they would not match.

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
ship both, as do macOS and the CI runners), the wasm-bindgen CLI it
installs itself (wasm), and `llvm-profdata` (the toolchain's `llvm-tools`)
and `ffmpeg` (pgo). A missing tool fails its step
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
