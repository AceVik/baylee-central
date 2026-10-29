# Development mode and reusable crates

Run with `cargo run --locked -p central-service -- --dev`. The explicit flag is
required. Defaults: http://127.0.0.1:28780, 64 KiB request bodies, 32 concurrent
connections. Environment settings:

- BAYLEE_CENTRAL_LISTEN: numeric loopback socket address; port 0 selects a free port.
- BAYLEE_CENTRAL_REQUEST_BYTES: 1024..1056768 (native mode requires the upper bound).
- BAYLEE_CENTRAL_CONCURRENT_REQUESTS: 1..128.

Reserved names must use the documented uppercase spelling on every platform.
Case variants and unknown reserved settings fail closed instead of silently using defaults. Development modes have no auth bypass,
hardcoded account, master password or training activation flag. The deployed
modes are separate and documented in [operations](operations.md). /health/live
reports the process; /health/ready deliberately returns 503 until real dependencies
and release requirements are implemented. Future storage fixtures must use isolated
synthetic databases/objects and must not read production configuration.

Current HTTP scope of `--dev`: native local tooling only; exact Host, no browser
Origin or Fetch-Metadata, no CORS, no keep-alive. Host and Origin filtering is
defense in depth, not authentication. The website's browser listener has its
own origin and CSRF policy ([web accounts](web-accounts.md)); for local runs
`--dev-accounts-stdin` can serve it for a numeric loopback page origin with
every other check unchanged ([native login](native-login.md)).

Crate boundaries:
- central-policy: portable, synchronous policy/configuration types, no external
  dependencies, network, database or secrets. Can be used from client/gateway code.
- central-http: bounded local server library, independent of the CLI.
- central-service: thin executable for lifecycle/configuration only: the
  development modes here and the deployed modes in operations.md.
- Planned central-protocol: versioned public wire types, not storage records.
- Planned central-client: native client SDK after authentication protocol review;
  no dependency on server stores, admin internals or signing keys.

Public Rust APIs are documented; no pre-1.0 stability guarantee yet. Library users
must preserve trusted input and transactional persistence requirements. No release
to crates.io until license/API/security gates pass.

Use `cargo xtask gate` and `cargo xtask smoke` on any platform
([xtask](xtask.md)). Smoke tests launch their own server
on an ephemeral loopback port and always stop that exact process, checking port
closure. Agent-started interactive clients must likewise be closed after testing.
Use `cargo xtask smoke --exercise-failure` to deliberately fail immediately after
startup and verify the same cleanup path. It must report Synthetic development
failure only after cleanup; a cleanup error is a failure, not an expected result.
CI exercises both paths on Linux x86-64, the server's one platform (owner,
29.10.2026; [xtask](xtask.md#which-runner-runs-what)). Cleanup probes have
deadlines and require a refused connection; a probe timeout is not proof of closure.

The policy checker scans all first-party source trees, including hidden paths.
Targets and workspace members cannot live under excluded .git/target/node_modules
directories or import sources through filesystem links. Dependency versions and
Cargo feature names must match the reviewed inventory exactly. Synthetic fixtures
test these boundaries, while compiler probes cover expanded/generated Rust.

Native platforms (owner, 29.10.2026): the server is built and tested for Linux
x86-64 only, where production runs (CI: ubuntu-24.04). The crates the Baylee
client links (central-api, central-client, central-crypto) are checked on Linux
x86-64, Windows x86-64, macOS ARM64 and x86-64 (CI: windows-2025, macos-15 and
macos-15-intel run `cargo xtask gate --client`). Device keychain
integration remains a platform adapter; no desktop keychain is implemented yet.
central-crypto is a reusable native-client library with an optional server feature,
no HTTP/database dependency and no production integration yet. See crypto-protocol.md.
central-postgres and central-migration use PostgreSQL 18+ and SeaORM 2 with explicit migrations;
see database-development.md and `cargo xtask postgres` for the disposable fixture;
client/policy libraries must not import the database implementation.
