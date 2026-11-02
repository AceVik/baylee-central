# Web Push dependency review — 2026-11-02

The owner instructed: review security, stability, active maintenance and alternatives,
then choose and use the best option. This conditional authorization replaces the initial
pending proposal; the first candidate was rejected by this review, before adoption.

## Decision

Use Mozilla `ece =2.4.2` (only `backend-openssl`) for RFC 8291, `jsonwebtoken =11.1.0`
without default crypto/PEM features, `jsonwebtoken-openssl =2.0.0`, and
`openssl =0.10.82` with **only `aws-lc`**, backed by pinned `aws-lc-sys =0.41.0`.
Use `ureq =3.5.0` with only Rustls for bounded outbound HTTPS. The OpenSSL API wrapper
uses AWS-LC, not a system OpenSSL installation and not openssl-src. No first-party FFI,
unsafe, primitive or JWT implementation is introduced. Existing chat/OPAQUE crypto is
unchanged, and no new consumer reaches the old bigint arithmetic.

This adds native C/assembly and unsafe FFI implementations inside dependencies:
AWS-LC, openssl-sys/openssl, foreign-types, clang-sys/libloading and build tooling.
The safe APIs are the application boundary, not a whole-stack unsafe-free claim.
CMake, a C compiler and libclang are required for this server-only feature.
No new browser package is needed. MPL-2.0 applies to Mozilla ECE; it is used unchanged
and its notices/source obligations must accompany distribution.

## Evidence and alternatives

- [Mozilla rust-ece](https://github.com/mozilla/rust-ece) is not archived; upstream
  activity 2026-09-02. Its published source includes RFC 8291 encryption and decryption
  vectors. Only its maintained provider API is used.
- [JWT](https://github.com/Keats/jsonwebtoken) has current 11.1.0 releases and upstream
  activity 2026-10-04. [OpenSSL provider](https://github.com/arckoor/jsonwebtoken-providers)
  activity 2026-09-24, no open issues at review. The provider checks key families;
  Central fixes ES256, checks P-256 key type/curve and never decodes authorization JWTs.
- web-push-native 0.5.0 + jwt-compact 0.8.0 was initially considered (29 new packages),
  then **rejected**: P-256 0.13 reaches crypto-bigint 0.5.5 arithmetic affected by
  [RUSTSEC-2026-0387](https://rustsec.org/advisories/RUSTSEC-2026-0387.html).
  [Upstream PR 13](https://github.com/leotaku/web-push-native/pull/13) is still unmerged.
  The existing OPAQUE unreachable-code exception must not be extended to this path.
- web-push 0.11 and jmap-web-push 0.4.2 pull the same affected P-256 generation;
  jwt-simple also adds unused algorithms and an unresolved RSA advisory.
- pushicino 2.0 has newer ECDH, but its jsonwebtoken RustCrypto provider still brings
  the affected generation, plus a broader HTTP graph. An unreleased fork was not chosen.
- A vendored OpenSSL variant was rejected: openssl-sys currently selects 3.6.3,
  predating September's fixes. [OpenSSL release history](https://www.openssl-library.org/news/timeline/)
  shows 3.6.5 and 4.0.3 fixes. System library selection would make this evidence machine-dependent.
- `cargo audit --deny warnings` passes the isolated chosen graph. The project's advisory
  ignore remains restricted by crypto-scope; scanner success alone did not clear the
  rejected first graph. Source/feature inventory, interoperability tests and required
  project gates are separate checks; this is not an independent security audit.

## Exact additional inventory

Registry source and checksums are pinned by Cargo.lock. Features below are the resolved
server graph, including target/build dependencies; source matches conservatively include
platform-specific code that might not execute on the local machine.

| Package | Version | Licence | Features | Build script |
| --- | --- | --- | --- | --- |
| aws-lc-sys | 0.41.0 | ISC AND (Apache-2.0 OR ISC) AND Apache-2.0 AND MIT AND BSD-3-Clause AND (Apache-2.0 OR ISC OR MIT) AND (Apache-2.0 OR ISC OR MIT-0) | all-bindings, bindgen, default, ssl | main.rs |
| bindgen | 0.72.1 | BSD-3-Clause | default, logging, prettyplease, runtime | build.rs |
| cexpr | 0.6.0 | Apache-2.0/MIT |  | none |
| clang-sys | 1.9.1 | Apache-2.0 | clang_10_0, clang_11_0, clang_3_5, clang_3_6, clang_3_7, clang_3_8, clang_3_9, clang_4_0, clang_5_0, clang_6_0, clang_7_0, clang_8_0, clang_9_0, libloading, runtime | build.rs |
| cmake | 0.1.58 | MIT OR Apache-2.0 |  | none |
| dunce | 1.0.5 | CC0-1.0 OR MIT-0 OR Apache-2.0 |  | none |
| ece | 2.4.2 | MPL-2.0 | backend-openssl, hkdf, lazy_static, openssl, sha2 | none |
| foreign-types | 0.3.2 | MIT/Apache-2.0 |  | none |
| foreign-types-shared | 0.1.1 | MIT/Apache-2.0 |  | none |
| fs_extra | 1.3.0 | MIT |  | none |
| itertools | 0.13.0 | MIT OR Apache-2.0 |  | none |
| jobserver | 0.1.35 | MIT OR Apache-2.0 |  | none |
| jsonwebtoken | 11.1.0 | MIT |  | none |
| jsonwebtoken-openssl | 2.0.0 | MIT |  | none |
| libloading | 0.8.9 | ISC |  | none |
| minimal-lexical | 0.2.1 | MIT/Apache-2.0 | std | none |
| nom | 7.1.3 | MIT | alloc, std | none |
| openssl | 0.10.82 | Apache-2.0 | aws-lc, default | build.rs |
| openssl-macros | 0.1.1 | MIT/Apache-2.0 |  | none |
| openssl-sys | 0.9.118 | MIT | aws-lc | main.rs |
| prettyplease | 0.2.37 | MIT OR Apache-2.0 | verbatim | build.rs |
| rustc-hash | 2.1.3 | Apache-2.0 OR MIT | default, std | none |
| shlex | 1.3.0 | MIT OR Apache-2.0 | default, std | none |
| signature | 2.2.0 | Apache-2.0 OR MIT | alloc, std | none |
| ureq | 3.5.0 | MIT OR Apache-2.0 | _ring, _rustls, _tls, rustls, rustls-no-provider, rustls-webpki-roots | none |
| ureq-proto | 0.6.5 | MIT OR Apache-2.0 | client | none |
| utf8-zero | 0.8.1 | MIT OR Apache-2.0 | default, std | none |

## Integration constraints

Push payloads contain only fixed categories, language and a random installation generation;
no chat text, account IDs, names, report contents or end-to-end keys. Endpoints are exact
recognized HTTPS provider hosts; redirects/proxies/private DNS results are refused.
Consent, credential revision and current admin role are rechecked at delivery. Durable
jobs coalesce per installation/category, with bounded retries and no chat-history deletion.
Production requires private VAPID configuration and outbound provider access. No deployment
or external human notifications are part of local validation.

## Integration inventory review

The registry manifests were checked against the integration metadata. Five existing
packages gain features directly required by the selected graph: base64 0.22.1
`default,std` and byteorder 1.5.0 `default` for ECE; thiserror 2.0.21 `default`
for ECE errors; cc 1.9.0 `parallel` for AWS-LC's native build; Rustls 0.23.46
`log,logging` for ureq. No TLS secret extraction feature is enabled and application
logging never formats endpoints, authentication headers or subscription keys.

`cargo deny check` identified four newly coexisting API generations: base64
0.22.1 (ECE/JWT) versus 0.23.1 (ureq), itertools 0.13.0 (bindgen) versus 0.14.0,
shlex 1.3.0 (bindgen) versus 2.0.1 (cc), and signature 2.2.0 (JWT provider)
versus 3.0.0 (Ed25519). Only these exact older versions need duplicate exceptions.
The MPL-2.0 licence exception is confined to ece 2.4.2; it is not a global licence
allowance. **No advisory exception is added or expanded.** The existing bigint
exception still requires the unchanged reviewed source/consumer fingerprint.
