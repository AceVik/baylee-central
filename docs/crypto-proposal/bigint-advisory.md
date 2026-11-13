# Bigint advisory scope, 28 October 2026

[RUSTSEC-2026-0387](https://rustsec.org/advisories/RUSTSEC-2026-0387.html)
describes secret-dependent branches in `crypto-bigint` arithmetic. The advisory
remains valid. This repository retains 0.5.5 as an indirect dependency, with a
reviewed **unreachable-code exception**, not a patched-library claim.

## Reachability review

Rechecked for `0.1.0-beta.9c` on 2026-11-02: the private suite still fixes
Ristretto255 for both OPRF and TripleDH. `opaque-sha2` explicitly names the
same SHA-512 0.10.9 type required by opaque-ke; other application hashes/HMACs
move to the current RustCrypto traits. Argon2 0.6 and Blake2 0.11 add no edge
to elliptic-curve or bigint. The previous Argon2 integration is a test-only
compatibility oracle. Upstream Ristretto implementations and their message
expansion callers were inspected again; no generic suite selection or bigint
arithmetic became reachable. Chat changes consolidate equivalent dependency
aliases and update the test RNG trait; the existing known answers pass.

The reviewed source fingerprint is
`ddbb34985086f0d853bc6f23db4c8b9acd4e11ffd853d5fba976d6806762d87f`
(§"Changes since the review", 13 November 2026).
Source mutation/link checks and all exact caller/feature negative checks remain
mandatory. The source fingerprint's SHA-256 hexadecimal encoding is unchanged
by its migration to digest 0.11.

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

## Re-review for the legacy store's check, 30 October 2026

`central-crypto` gained `src/legacy.rs` behind `server` (Argon2id under the
official gateway's fixed parameters, for the legacy store's sign-in,
[native login](../native-login.md#legacy-login)) and `pub(crate)` on the
OPAQUE stretch's module and its `WorkBuffer`. The new module calls `argon2`
through opaque-ke's re-export alone (blake2,
base64ct, password-hash), which was already compiled in as opaque-ke's KSF;
`argon2`, `blake2`, `password-hash` and `base64ct` neither depend on nor
mention bigint or elliptic-curve. No suite, group, OPRF or key-exchange type
is touched; `FixedKsf` and its 64 MiB profile stay private to `opaque`. The
manifest and the lockfile are unchanged. Reverse dependencies stay `crypto-bigint ← elliptic-curve ←
opaque-ke/voprf ← central-crypto`, and the graph guard's versions and
features are unchanged. Reviewed by an independent agent pass (no finding on
reachability; its one hygiene finding, that argon2's own H0 and final block
are not wiped without argon2's `zeroize` feature, holds for the OPAQUE stretch
as well and waits for that feature's approval) and by the author; the
fingerprint is pinned again (`crypto_scope/sources.rs`). The gateway's own hash is reproduced by the module's test
vector and was checked against OpenSSL's Argon2id.


**argon2's `zeroize` feature, the same day.** The owner approved argon2's
`zeroize` feature (the crate itself was approved for the legacy check), so
`central-crypto` names `argon2 =0.5.3`, the version opaque-ke already uses,
as a direct dependency with that feature alone, and `legacy.rs` imports it
from there instead of through the re-export. No package is added: the
lockfile gains the edge `argon2 → zeroize` (zeroize was in the graph) and
`central-crypto → argon2`. Features unify, so the OPAQUE stretch's argon2 is
the same crate with the same feature: argon2 now wipes its own H0 and final
blocks for both, which closes the review's hygiene finding above; the
caller-owned work buffer is still wiped by `WorkBuffer`. Bigint's consumers
and the graph guard's versions and features are unchanged. Reviewed by the
author (a manifest line and an import); the fingerprint is pinned again.
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

After the second model's review of the same day, `KeyCheck` derives `Clone`
only: equality was variable-time and unused. No other source, dependency or
edge in `central-crypto` changed. Fingerprint `2cb6fb13…`.

The owner then chose HPKE's PSK mode for the seal: `seal_for_pairing` and
`open` take the link's secret, which becomes the pre-shared key with the
handle as its ID (`hpke`'s public `PskBundle`, no new feature or edge), and
the tests add RFC 9180 Appendix A.2.2. The graph and the reachability above
are unchanged. Fingerprint `d7adcac6…`.

### 31 October 2026: the legacy store's check and the pairing together

Merging the pairing onto the legacy store (§"Re-review for the legacy
store's check", fingerprint `a55d829a…`) changes no line of either reviewed
change. They touch different files but two: the manifest, where each adds
its own dependencies (`argon2` with `zeroize`; `hmac`, `hpke` and
`getrandom`'s `sys_rng`), and `lib.rs`, where each adds its own module.
Both reviews stand as written. Fingerprint `36dc9ccf…`.

### 31 October 2026: the crate keeps the AGPL under the new workspace licence

The source-available workspace licence (PR #9) does not apply to
`central-crypto`, which the Baylee client links: its manifest replaces
`license.workspace = true` with `license = "AGPL-3.0-only"` and a comment
saying why. Merged onto the pairing and the legacy store (fingerprint
`36dc9ccf…`), those two manifest lines are the only difference in the crate
tree. No source, dependency, feature or graph edge changed, and the reviews
above stand. Fingerprint `6db63ad9…`.


### 11 November 2026: registration's proof of work with a cost in memory

`src/proof.rs` is new and `lib.rs` names it (`pub mod proof;`, on every
build, not behind `server`: the page's Worker searches with it and Central
checks with it, [registration abuse](../registration-abuse.md) §"A proof
that costs memory"; the owner accepted the change on 11 November 2026,
[crypto protocol](../crypto-protocol.md) §"Registration's proof of work").
The module calls four items of `argon2` 0.6.0, already the crate's direct
dependency with its one feature (`zeroize`): `Params::new`, `Argon2::new`,
`Block::default` and `hash_password_into_with_memory`, the calls
`legacy.rs` and the OPAQUE stretch make. Its inputs are public (a label,
Central's nonce, a part's index, a counter) and it derives no key. Its test
compares one output with the Argon2 that `opaque-ke` brings, the
development dependency `opaque/stretch.rs` already used as its oracle, and
its pinned output was reproduced with OpenSSL 4.1.0's Argon2id.

Neither the manifest nor the lockfile changed: no package, feature or graph
edge is new, and `argon2`, `blake2`, `password-hash` and `base64ct` still
neither depend on nor mention bigint or elliptic-curve. The module shares no
type with `opaque::Suite`, its group, the OPRF or the key exchange, which
are unchanged; reverse dependencies stay `crypto-bigint ← elliptic-curve ←
opaque-ke/voprf ← central-crypto`. The reachability above is unchanged.
Fingerprint `52af94ea…`. This is the author's source and graph review
(one agent pass, the one that wrote the module).

A second reading the same day, by the agent pass that took the change into
the website's branch (another pass of the same model, not another model
and not a person): `proof.rs` line by line against the paragraph above;
the diff of the manifests and the lockfile, which is empty; the pinned
output computed again with OpenSSL 4.1.0's Argon2id from the bytes the
test's comment names, equal to the test's; and Central's side, which
allocates for a check only at the price it kept with the nonce, never
above its own 16 MiB (`central-identity`'s `challenge.rs`). It found
nothing to change. A review by another model or a person is still open
(`Review: pending`).

### 13 November 2026: the token format and the proof of work together

Combining the session tokens (§"Re-review for the token format", fingerprint
`aea8b222…`) with the website's release, which brought the proof of work above
(fingerprint `52af94ea…`), changes no line of either reviewed change. They
touch different files but one, `lib.rs`, where each adds its own module
declaration (`jwt` behind its feature, `proof` on every build); the manifest
is the token format's, since the proof of work changed neither manifest nor
lockfile. Every other file of the crate is byte for byte one side's, which was
checked file by file against both branches, and each side's fingerprint was
recomputed from its own tree before the combined one was pinned. Checked by
running: `cargo xtask crypto-scope` (graph guard, then the fingerprint) passes,
and `cargo tree -i crypto-bigint --all-features` still shows exactly
`crypto-bigint <- elliptic-curve <- opaque-ke/voprf <- central-crypto`. Both
reviews stand as written. Fingerprint `ddbb3498…`. This is the merging agent's
check, not an independent review.


## Chat source and graph review, 2 November 2026

The imported `central-crypto::chat` is Baylee's `baylee-seal` implementation:
Ed25519 signatures, X25519 wraps, XChaCha20-Poly1305, SHA-256/SHA-512 and HKDF.
The port changes module/import paths and a checked base64 index conversion;
the 117 original vectors/adversarial tests pass unchanged in their expected bytes.
The new account identity uses the existing HKDF-SHA-512 dependency, an independent
domain, the master key, and the fixed issuer/account/epoch context. It is covered
by an independent stdlib HMAC known answer and context-separation tests.

Review of the lock and all-feature resolution shows no new caller of bigint or
elliptic-curve and no change to the private OPAQUE suite. New paths are
`central-crypto → ed25519-dalek 3.0.0 → curve25519-dalek 5.0.0` and direct
`central-crypto → x25519-dalek 3.0.0`. Curve 5 has no elliptic-curve/bigint edge.
Ed25519's `ed25519 → signature` type-trait edge also reaches neither. The guard
now pins these packages' registry sources, versions, enabled features and exact
reverse edges, with negative tests for each new boundary. The existing Ristretto
4 path and its reviewed hash-expansion reachability remain unchanged. This is
an author source/graph review, not an independent cryptographic audit.

Existing SHA-2 0.11.1 and HMAC 0.13.0 enable their safe `zeroize` implementations;
this also enables digest 0.11.3 and block-buffer 0.12.1 wiping. Source inspection
of those feature-gated Drop/Zeroize implementations finds calls to the already
approved zeroize API, no new unsafe blocks, native module or assembly. Existing
unsafe buffer/CPU implementations remain covered by the prior package approvals.
The expanded inventory records these exact features; the owner-approved new
Ed25519 scope is recorded in `docs/dependency-proposal/chat.md`.

## Web Push trait generation — 2026-11-02

The reviewed push graph (docs/dependency-proposal/push.md) adds signature 2.2.0
(`alloc,std`) solely for jsonwebtoken 11.1.0 (no features). The graph guard now
pins both signature generations, their exact callers and features. AWS-LC is the
provider; no new consumer reaches elliptic-curve, crypto-bigint or voprf. Every
previous caller/feature check remains in force, as does the central-crypto source
fingerprint. Negative tests reject enabling JWT's RustCrypto provider or adding
an edge from JWT to any affected arithmetic boundary. This is a trait-inventory
extension, not an extension of the advisory's reachable-code exception.

## Re-review for the token format, 6 November 2026

`central-crypto` gained `src/jwt.rs` and `src/jwt/` behind a new feature `jwt`: the
compact JWS of the session tokens, Ed25519 only. Signing also needs `server`. The new
module imports `ed25519-dalek`, `sha2` 0.11, `base64ct`, `serde`, `serde_json`,
`zeroize` and `getrandom`, all approved and already in the graph with the features
used here, and nothing from `opaque`. No package is added. Nothing that existed
changes except the module's declaration in `lib.rs` and the manifest: no suite, group,
OPRF or key-exchange type is touched, and no `opaque` source differs.

Checked by running, not only by reading: `cargo tree -i crypto-bigint` with all features
still shows exactly `crypto-bigint <- elliptic-curve <- opaque-ke/voprf <- central-crypto`,
and the graph guard, which runs before the source check, passes with its callers and
features unchanged, `ed25519-dalek` keeping `central-crypto` as its only caller. The
new path signs and verifies through `curve25519-dalek` 5.0.0, which has no edge to
`elliptic-curve` or `crypto-bigint`. The affected arithmetic stays unreachable from it.

The new reviewed source fingerprint is
`aea8b222bd01ea3e0faa2682007a2050ef89c34613e07b45b582d1462c7f4904`.
This is the implementing agent's review, not an independent audit. Any later change to
the crate pins the fingerprint again and repeats this check.

Pinned once more the same day: a verified payload, which holds a Central token's capability, is
now wiped when dropped. No import or dependency changed.
