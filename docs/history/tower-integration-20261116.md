# Tower integration review — 16 November 2026

Reviewed and merged `agy/tower-achievements` at `87d40e86`. This is an author review,
not independent security approval. No deployment or production migration occurred.

1. Design: compile-time development endpoints remain explicitly loopback-only;
   the default production issuer and bootstrap document are unchanged. No new
   dependency, cryptographic primitive or authorization bypass was introduced.
2. Implementation: parse development origins as a strict socket address, reject
   malformed brackets, userinfo, paths, queries and invalid ports. All-features
   builds without an environment setting fail closed at numeric loopback port 9.
   Keep the production bootstrap document borrowed rather than allocating it.
3. Adversarial checks: the origin parser regression passed; all-features flow and
   password suites passed (14 + 9 tests); the default production issuer test also
   passed. Strict clippy passed for central-web-crypto and xtask, all targets and
   features. Evidence is in `/tmp/central-tower-gemini-*.log` on the test host.

The canonical `cargo xtask wasm --check` requires x86_64 Linux. The attempted
pinned Docker runner could not start because Docker's overlay filesystem is
read-only. No container was created and no unrelated container was changed.
Production WASM reproducibility remains unverified for this milestone; do not
mark the PR ready or claim the full Central gates passed on this evidence.
The final recheck on 16 November reproduced the same Docker overlay error
(`/tmp/central-tower-wasm-recheck-20261116.log`, exit 125). The exact named
container was absent afterward; no service or container was left running.
