# Chat compatibility dependency proposal — 2026-11-02

Status: the owner explicitly accepted the three exact packages and the curve digest feature in this session on 2026-11-02 ("Ja, diesen Umfang freigeben"). Implementation and validation follow.

The native client's existing `baylee-seal` protocol uses Ed25519 signatures,
X25519 recipient key wraps and XChaCha20-Poly1305 ciphertext. Reusing its reviewed
implementation in `central-crypto::chat` lets the website's existing WebAssembly
worker and the native client exchange the same authenticated messages. Historical
ciphertext and device keys keep their format. No replacement cryptosystem is needed.

## Exact proposed addition

```toml
ed25519-dalek = { version = "=3.0.0", default-features = false, features = ["zeroize"] }
```

The additional registry packages are `ed25519-dalek` 3.0.0 (BSD-3-Clause),
`ed25519` 3.0.0 and `signature` 3.0.0 (both Apache-2.0 OR MIT). These exact
releases are already in Baylee's committed lockfile and installed source cache.
No batch, hazmat, legacy compatibility, PEM, PKCS#8, serde or RNG feature is
requested. All three manifests declare no build script. Their selected library
sources forbid unsafe code (dalek's exemption is for tests and the disabled batch
feature). Package checksums remain Cargo's registry checksums, pinned in the lock.

The transitive curve implementation is Central's already accepted
`curve25519-dalek` 5.0.0. It contains platform-specific unsafe SIMD internals,
as disclosed in `hpke.md`; the application's own Rust remains forbidden from
using unsafe code. Ed25519 additionally requires this crate's `digest` feature;
the optional `fast`/precomputed-table feature is deliberately not requested.
SHA-2, subtle and zeroize are already approved packages. Direct access to the
already approved X25519 and constant-time comparison APIs will use the existing
versions and approved features. No new native module or assembly is proposed.

## Adoption and validation

Owner acceptance is required before the manifest, lock and approved inventory
are changed. After acceptance: compare the imported source with the native
implementation; run its known-answer vectors across native and WebAssembly;
exercise malformed, weak and changed keys, forged headers, ciphertext and wraps,
recipient substitution and key erasure. Run the current advisory, licence and
source/graph scope checks. This proposal is a source inspection, not an advisory
clearance or an independent security audit.

The account/contact backend and UI can be completed independently while this
decision is pending. Gateways receive neither Central credentials nor plaintext.
