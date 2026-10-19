# Development mode and reusable crates

Run with `cargo run --locked -p central-service -- --dev`. The explicit flag is
required. Defaults: http://127.0.0.1:28780, 64 KiB request bodies, 32 concurrent
connections. Environment settings:

- BAYLEE_CENTRAL_LISTEN: numeric loopback socket address; port 0 selects a free port.
- BAYLEE_CENTRAL_REQUEST_BYTES: 1024..1048576.
- BAYLEE_CENTRAL_CONCURRENT_REQUESTS: 1..128.

Unknown reserved settings fail closed. There is no production flag, auth bypass,
hardcoded account, master password or training activation flag. /health/live
reports the process; /health/ready deliberately returns 503 until real dependencies
and release requirements are implemented. Future storage fixtures must use isolated
synthetic databases/objects and must not read production configuration.

Current HTTP scope: native local tooling only; exact Host, no browser Origin or
Fetch-Metadata, no CORS, no keep-alive. Host and Origin filtering is defense in
depth, not authentication. No sensitive endpoints are implemented. Future website
integration needs its own explicit origin and CSRF policy.

Crate boundaries:
- central-policy: portable, synchronous policy/configuration types, no external
  dependencies, network, database or secrets. Can be used from client/gateway code.
- central-http: bounded local server library, independent of the CLI.
- central-service: thin executable for lifecycle/configuration only.
- Planned central-protocol: versioned public wire types, not storage records.
- Planned central-client: native client SDK after authentication protocol review;
  no dependency on server stores, admin internals or signing keys.

Public Rust APIs are documented; no pre-1.0 stability guarantee yet. Library users
must preserve trusted input and transactional persistence requirements. No release
to crates.io until license/API/security gates pass.

Use scripts/test.ps1 and scripts/smoke-dev.ps1. Smoke tests launch their own server
on an ephemeral loopback port and always stop that exact process, checking port
closure. Agent-started interactive clients must likewise be closed after testing.
