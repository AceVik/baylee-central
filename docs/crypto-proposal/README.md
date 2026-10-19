# Cryptography dependency proposal

Status: owner accepted on 2026-10-19 subject to review and tests. Used only by the isolated central-crypto spike; see ../crypto-protocol.md for evidence and gaps. This acceptance is separate from the HTTP graph.

Direct candidates: opaque-ke 4.0.1 (RFC 9807 OPAQUE), sha2 0.10.9, rand 0.8.8 (opaque-ke compatibility), chacha20poly1305 0.11.0 with zeroization, hkdf 0.12.4, zeroize 1.9.1, secrecy 0.10.3 and getrandom 0.4.3. OPAQUE brings Argon2 0.5.3. Cargo.lock resolves 61 external packages.

The older SHA2/HKDF/Rand/Argon2 major versions match the maintained opaque-ke API; they must not be replaced independently without compatibility and security review. All application-facing calls remain safe Rust. Unsafe, assembly and CPU-specific operations inside crypto/system dependencies are expected and disclosed below. Do not replace them with home-grown primitives.

RustSec cargo-audit 0.22.2, 2026-10-19: no known vulnerabilities or warnings for this proposal. This is NOT a cryptographic audit. The upstream opaque-ke 2021 audit covers an older release; our selected release and application integration are not independently audited by that fact.

Adoption checks: owner acceptance recorded; exact features/native/build inventory reviewed; duplicate versions have narrow version-specific cargo-deny exceptions; the RFC registration vector and adversarial round trips pass. The application graph is frozen by the approved package/features list. AEAD zeroization is explicitly enabled. No weakened-round cipher feature is enabled.

Marker counts are textual indicators including tests/comments/cfg-inactive code, not proof of safety or vulnerability. See Cargo.toml/Cargo.lock for resolved sources and features.

| Package | License | Files with unsafe/asm markers | Build script |
| --- | --- | ---: | --- |
| aead 0.6.1 | MIT OR Apache-2.0 | 0 | False |
| argon2 0.5.3 | MIT OR Apache-2.0 | 1 | False |
| base16ct 0.2.0 | Apache-2.0 OR MIT | 2 | False |
| base64ct 1.8.3 | Apache-2.0 OR MIT | 1 | False |
| blake2 0.10.6 | MIT OR Apache-2.0 | 6 | False |
| block-buffer 0.10.4 | MIT OR Apache-2.0 | 2 | False |
| block-buffer 0.12.1 | MIT OR Apache-2.0 | 3 | False |
| cfg-if 1.0.5 | MIT OR Apache-2.0 | 0 | False |
| chacha20 0.10.2 | MIT OR Apache-2.0 | 6 | False |
| chacha20poly1305 0.11.0 | Apache-2.0 OR MIT | 0 | False |
| cipher 0.5.2 | MIT OR Apache-2.0 | 0 | False |
| cmov 0.5.4 | Apache-2.0 OR MIT | 5 | False |
| const-oid 0.9.6 | Apache-2.0 OR MIT | 0 | False |
| cpufeatures 0.3.1 | MIT OR Apache-2.0 | 3 | False |
| cpufeatures 0.2.17 | MIT OR Apache-2.0 | 3 | False |
| crypto-bigint 0.5.5 | Apache-2.0 OR MIT | 2 | False |
| crypto-common 0.1.7 | MIT OR Apache-2.0 | 0 | False |
| crypto-common 0.2.2 | MIT OR Apache-2.0 | 0 | False |
| ctutils 0.4.3 | Apache-2.0 OR MIT | 1 | False |
| curve25519-dalek 4.1.3 | BSD-3-Clause | 7 | True |
| curve25519-dalek-derive 0.1.1 | MIT/Apache-2.0 | 1 | False |
| der 0.7.10 | Apache-2.0 OR MIT | 0 | False |
| derive-where 1.7.0 | MIT OR Apache-2.0 | 5 | False |
| digest 0.10.7 | MIT OR Apache-2.0 | 0 | False |
| displaydoc 0.2.7 | MIT OR Apache-2.0 | 0 | False |
| elliptic-curve 0.13.8 | Apache-2.0 OR MIT | 0 | False |
| ff 0.13.1 | MIT/Apache-2.0 | 0 | False |
| fiat-crypto 0.2.9 | MIT OR Apache-2.0 OR BSD-1-Clause | 0 | False |
| generic-array 0.14.7 | MIT | 7 | True |
| getrandom 0.4.3 | MIT OR Apache-2.0 | 29 | True |
| getrandom 0.2.17 | MIT OR Apache-2.0 | 21 | False |
| group 0.13.0 | MIT/Apache-2.0 | 0 | False |
| hkdf 0.12.4 | MIT OR Apache-2.0 | 0 | False |
| hmac 0.12.1 | MIT OR Apache-2.0 | 0 | False |
| hybrid-array 0.4.15 | MIT OR Apache-2.0 | 6 | False |
| inout 0.2.4 | MIT OR Apache-2.0 | 4 | False |
| libc 0.2.191 | MIT OR Apache-2.0 | 70 | True |
| opaque-ke 4.0.1 | Apache-2.0 OR MIT | 0 | False |
| password-hash 0.5.0 | MIT OR Apache-2.0 | 0 | False |
| poly1305 0.9.1 | Apache-2.0 OR MIT | 5 | False |
| proc-macro2 1.0.107 | MIT OR Apache-2.0 | 3 | True |
| quote 1.0.47 | MIT OR Apache-2.0 | 0 | True |
| r-efi 6.0.0 | MIT OR Apache-2.0 OR LGPL-2.1-or-later | 49 | False |
| rand 0.8.8 | MIT OR Apache-2.0 | 7 | False |
| rand_core 0.6.4 | MIT OR Apache-2.0 | 1 | False |
| rustc_version 0.4.1 | MIT OR Apache-2.0 | 0 | False |
| sec1 0.7.3 | Apache-2.0 OR MIT | 0 | False |
| secrecy 0.10.3 | Apache-2.0 OR MIT | 0 | False |
| semver 1.0.28 | MIT OR Apache-2.0 | 2 | False |
| sha2 0.10.9 | MIT OR Apache-2.0 | 8 | False |
| subtle 2.6.1 | BSD-3-Clause | 1 | False |
| syn 2.0.119 | MIT OR Apache-2.0 | 16 | False |
| syn 3.0.9 | MIT OR Apache-2.0 | 15 | False |
| typenum 1.20.1 | MIT OR Apache-2.0 | 0 | False |
| unicode-ident 1.0.27 | (MIT OR Apache-2.0) AND Unicode-3.0 | 1 | False |
| universal-hash 0.6.1 | MIT OR Apache-2.0 | 0 | False |
| version_check 0.9.5 | MIT/Apache-2.0 | 0 | False |
| voprf 0.5.0 | MIT | 3 | False |
| wasi 0.11.1+wasi-snapshot-preview1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | 1 | False |
| zeroize 1.9.1 | Apache-2.0 OR MIT | 7 | False |
| zeroize_derive 1.5.0 | Apache-2.0 OR MIT | 0 | False |

References: https://www.rfc-editor.org/rfc/rfc9807.html ; https://github.com/facebook/opaque-ke#audit ; https://docs.rs/opaque-ke/4.0.1/opaque_ke/ ; https://github.com/RustCrypto/AEADs/tree/master/chacha20poly1305
