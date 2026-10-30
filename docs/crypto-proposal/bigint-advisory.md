# Bigint advisory scope, 28 October 2026

[RUSTSEC-2026-0387](https://rustsec.org/advisories/RUSTSEC-2026-0387.html)
describes secret-dependent branches in `crypto-bigint` arithmetic. The advisory
remains valid. This repository retains 0.5.5 as an indirect dependency, with a
reviewed **unreachable-code exception**, not a patched-library claim.

## Reachability review

`opaque-ke 4.0.1` and `voprf 0.5.0` require `elliptic-curve 0.13.8`, which
requires bigint 0.5. An independent lockfile bump to the corrected >=0.7.6
generation is incompatible. The published OPAQUE prerelease also retains the
old dependency. No crypto fork, protocol replacement or credential migration is
introduced to silence the scanner.

The unchanged private `central-crypto::opaque::Suite` fixes both OPRF and
Triple-DH to Ristretto255, with SHA-512. Its upstream implementations use
`curve25519-dalek 4.1.3` points/scalars for multiplication, reduction and inversion.
The reached `elliptic-curve::hash2curve::ExpandMsgXmd` and `Domain` helpers use
hashes, byte arrays, XOR and public lengths, without bigint arithmetic. `U256`
there is a typenum length. Neither callers nor remote messages can select a
different suite. Client, server and browser share this fixed implementation.

Reviewed sources (registry checksums remain in Cargo.lock):

- `crates/central-crypto/`, especially `src/opaque.rs` and `src/opaque/server.rs`.
- [opaque-ke's Ristretto implementation](https://docs.rs/crate/opaque-ke/4.0.1/source/src/key_exchange/group/ristretto255.rs).
- [voprf's Ristretto implementation](https://docs.rs/crate/voprf/0.5.0/source/src/group/ristretto.rs).
- [elliptic-curve's message expansion](https://docs.rs/crate/elliptic-curve/0.13.8/source/src/hash2curve/hash2field/expand_msg/xmd.rs)
  and [domain handling](https://docs.rs/crate/elliptic-curve/0.13.8/source/src/hash2curve/hash2field/expand_msg.rs).

This agrees with the sibling native client's existing reviewed scope in
`docs/central-login.md`. It is a source reachability review, not a new external
cryptographic audit or a general safety statement about bigint 0.5.5.

## Enforcement

`cargo xtask crypto-scope` is mandatory in `cargo xtask gate` on every native CI
platform and in the dependency job **before** audit/deny. It checks:

- Exact registry versions/sources and features of the five reviewed upstream
  packages and of the pairing's three (`hpke`, `x25519-dalek` and the second
  `curve25519-dalek` generation, 5.0.0, beside 4.1.3); no other duplicate
  versions, and no missing resolution nodes.
- All-feature metadata without target filtering, including inactive optional
  consumers. Reverse dependencies must remain
  `crypto-bigint ← elliptic-curve ← opaque-ke/voprf ← central-crypto`.
- Central crypto must be the workspace member at the expected manifest path.
  Its entire crate tree, including manifest and tests, must match the reviewed
  SHA-256 fingerprint. Added, removed, renamed or changed files fail; links fail.
  Relative paths use `/`, text normalizes CRLF to LF, and fields are length-prefixed.

The fingerprint is pinned in `crates/xtask/src/crypto_scope/sources.rs`, not
automatically regenerated. A change requires source/graph review, regression
validation and a review-record update. Synthetic negative tests cover changed
sources/versions/features, new consumers at every affected boundary, missing or
duplicate nodes, and source-tree edits/additions/removals/renames/links.

`.cargo/audit.toml` and `deny.toml` name only this advisory exception. Audit and
deny alone do not establish its applicability: run the scope check and full
gate too. Other advisories and all existing warnings remain blocking.

## Removal condition

Remove both exceptions and this guard when a compatible maintained upstream
OPAQUE/VOPRF stack no longer resolves the affected bigint. Before migration,
review the complete graph and run published vectors, stored-credential/wire
compatibility checks and the browser rebuild. No protocol, KDF strength, data
format, cryptographic dependency or new dependency-internal operations changed
in this fix.

## Changes since the review

Each change to `central-crypto` repeats the source and graph review for it,
moves the fingerprint only afterwards, and is recorded here.

### 30 October 2026: two values derived from the master key

`src/vault/derived.rs` (with `tests/derived-vectors.py`) derives the key
check and a local vault's key from the master key with HKDF-SHA-512
(`../crypto-protocol.md` §"Derived from the master key"), and
`Context::bound` became `pub(crate)` for it. No dependency, feature or graph
edge changed: `hkdf` and `sha2` were `central-crypto`'s already. Neither value
touches `opaque::Suite`, its Ristretto arithmetic or `elliptic-curve`, so
the reachability above is unchanged. Fingerprint `3d0f6adb…`; a second
model reviews the change before it lands (`Review: pending`).

### 30 October 2026: the pairing's seal

`src/pairing.rs` (tests in `src/pairing/tests.rs` and
`tests/pairing-vectors.py`) adds the pairing's MACs, its comparison code and
its RFC 9180 seal through `hpke` 0.15.1 (`../dependency-proposal/hpke.md`);
`MasterKey` gained two crate-private accessors for the seal. The graph
gained `hpke` 0.15.1 (from `central-crypto`), `x25519-dalek` 3.0.0 (from
`hpke`) and a second `curve25519-dalek`, 5.0.0 (from both), whose
`fiat-crypto` 0.3.0 sits behind a cfg no build sets. All-feature metadata
shows none of them depending on `elliptic-curve` or `crypto-bigint`:
`crypto-bigint`'s only caller is still `elliptic-curve`, whose only callers
are still `opaque-ke` and `voprf`. The seal's X25519 shares no type with the
Ristretto suite, which is unchanged. The guard now pins the three, their
features and reverse edges, exactly the two `curve25519-dalek` generations,
and 4.1.3's callers (`opaque-ke`, `voprf`), which it did not pin before; a
third generation, a new consumer of any of them or another feature fails
(`graph/tests.rs`, and with the 5.0.0 pin removed two of its tests fail).
Fingerprint `b3ab6d24…`; `Review: pending`, as above.
