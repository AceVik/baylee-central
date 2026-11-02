# Dependency migration for 0.1.0-beta.9c

Status: owner accepted this exact reviewed scope on 2026-11-02, subject to the complete gates. No whole-stack safety or external audit is claimed. First-party Rust remains forbidden from using unsafe/assembly.

The owner requested Cargo upgrades including major versions on 2026-11-02. This inventory covers 45 new package/version or feature entries, including target-specific, build and transitive dependencies. Exact sources/checksums are in `Cargo.lock`; exact accepted features are in `approved-packages.json`. Existing accepted entries remain documented.

Runtime changes include Tokio 1.54.1, Tower HTTP 0.7.1, Argon2 0.6.0, SHA-2 0.11.1, HKDF/HMAC 0.13.0, ureq 3.5.1 and wasm-bindgen 0.2.130. OPAQUE 4 still requires the older rand/digest generation; those compatibility dependencies stay explicit. The Argon2 parameters and crypto transcript/wire formats are unchanged and require compatibility tests. Test-only ChaCha RNG moves to 0.10.

Counts are conservative text/file inventories, including tests/comments/cfg-inactive code, not a soundness audit. Unsafe markers count Rust files containing `unsafe`/`asm!`; native counts C/C++/header/assembly files. The package's declared build-script target is recorded separately. All packages are fetched through Cargo's crates.io source and lockfile integrity verification.

| Package | License | Unsafe-marker files | Native/assembly files | Build script | Enabled features |
| --- | --- | ---: | ---: | --- | --- |
| `argon2@0.6.0` | MIT OR Apache-2.0 | 3 | 0 | false | `zeroize` |
| `arrow-arith@60.1.0` | Apache-2.0 | 2 | 0 | false | `` |
| `arrow-array@60.1.0` | Apache-2.0 AND MIT | 37 | 0 | false | `` |
| `arrow-buffer@60.1.0` | Apache-2.0 | 18 | 0 | false | `` |
| `arrow-cast@60.1.0` | Apache-2.0 | 9 | 0 | false | `` |
| `arrow-cmp@60.1.0` | Apache-2.0 | 1 | 0 | false | `` |
| `arrow-data@60.1.0` | Apache-2.0 | 7 | 0 | false | `` |
| `arrow-ord@60.1.0` | Apache-2.0 | 3 | 0 | false | `` |
| `arrow-row@60.1.0` | Apache-2.0 | 4 | 0 | false | `` |
| `arrow-schema@60.1.0` | Apache-2.0 | 1 | 0 | false | `` |
| `arrow-select@60.1.0` | Apache-2.0 | 10 | 0 | false | `` |
| `arrow-string@60.1.0` | Apache-2.0 | 3 | 0 | false | `` |
| `arrow@60.1.0` | Apache-2.0 | 8 | 0 | false | `` |
| `blake2@0.11.0` | MIT OR Apache-2.0 | 1 | 0 | false | `` |
| `cc@1.11.0` | MIT OR Apache-2.0 | 7 | 2 | false | `parallel` |
| `ctutils@0.4.7` | Apache-2.0 OR MIT | 1 | 0 | false | `` |
| `icu_collections@2.4.0` | Unicode-3.0 | 3 | 0 | false | `` |
| `icu_locale_core@2.4.0` | Unicode-3.0 | 3 | 0 | false | `zerovec` |
| `icu_normalizer@2.4.0` | Unicode-3.0 | 2 | 0 | false | `compiled_data` |
| `icu_normalizer_data@2.4.0` | Unicode-3.0 | 0 | 0 | true | `` |
| `icu_properties@2.4.0` | Unicode-3.0 | 0 | 0 | false | `compiled_data` |
| `icu_properties_data@2.4.0` | Unicode-3.0 | 0 | 0 | true | `` |
| `icu_provider@2.4.0` | Unicode-3.0 | 4 | 0 | false | `baked` |
| `inout@0.2.5` | MIT OR Apache-2.0 | 4 | 0 | false | `` |
| `js-sys@0.3.107` | MIT OR Apache-2.0 | 5 | 0 | false | `default, std, unsafe-eval` |
| `log@0.4.35` | MIT OR Apache-2.0 | 1 | 0 | false | `` |
| `rand_chacha@0.10.0` | MIT OR Apache-2.0 | 1 | 0 | false | `std` |
| `smallvec@1.16.4` | MIT OR Apache-2.0 | 2 | 0 | false | `const_generics, const_new, serde` |
| `tinystr@0.8.5` | Unicode-3.0 | 5 | 0 | false | `zerovec` |
| `tokio@1.54.1` | MIT | 158 | 0 | false | `bytes, default, fs, io-util, libc, macros, mio, net, rt, rt-multi-thread, signal, signal-hook-registry, socket2, sync, time, tokio-macros, windows-sys` |
| `tower-http@0.7.1` | MIT | 2 | 0 | false | `timeout` |
| `unicode-properties@0.1.5` | (MIT OR Apache-2.0) AND Unicode-3.0 | 0 | 0 | false | `default, emoji, general-category` |
| `ureq-proto@0.7.1` | MIT OR Apache-2.0 | 1 | 0 | false | `client` |
| `ureq@3.5.1` | MIT OR Apache-2.0 | 2 | 0 | false | `_ring, _rustls, _tls, rustls, rustls-no-provider, rustls-webpki-roots` |
| `utf8_iter@1.1.0` | Apache-2.0 OR MIT | 2 | 0 | false | `` |
| `utf_types@1.0.0` | Apache-2.0 OR MIT | 2 | 0 | false | `` |
| `uuid@1.29.0` | Apache-2.0 OR MIT | 4 | 0 | false | `default, serde, std` |
| `wasm-bindgen-macro-support@0.2.130` | MIT OR Apache-2.0 | 3 | 0 | false | `` |
| `wasm-bindgen-macro@0.2.130` | MIT OR Apache-2.0 | 0 | 0 | false | `` |
| `wasm-bindgen-shared@0.2.130` | MIT OR Apache-2.0 | 0 | 0 | true | `` |
| `wasm-bindgen@0.2.130` | MIT OR Apache-2.0 | 11 | 0 | true | `default, serde, std` |
| `webpki-roots@1.0.11` | CDLA-Permissive-2.0 | 2 | 0 | false | `` |
| `yoke@0.8.4` | Unicode-3.0 | 8 | 0 | false | `derive, zerofrom` |
| `zerovec-derive@0.11.7` | Unicode-3.0 | 6 | 0 | false | `` |
| `zerovec@0.11.9` | Unicode-3.0 | 31 | 0 | false | `derive, yoke` |

## Review and candidate validation

On 2026-11-02 the isolated candidate passed `cargo check --workspace
--all-targets --all-features --locked`, Clippy with warnings denied, and all
168 central-crypto tests (including doctests). Existing chat known answers,
RFC OPAQUE vectors, wrapping/pairing negative tests and the new comparison of
the fixed Argon2 KSF against opaque-ke's previous implementation pass. Argon2
0.5.3 and its Blake2 0.10 dependency are retained only as that test oracle;
exact duplicate exceptions record this reason. Runtime OPAQUE uses the fixed,
wiped Argon2 0.6 work buffer, with unchanged algorithm, salt and parameters.

`cargo audit 0.22.2 --deny warnings` and `cargo deny 0.20.2 check` pass. The
existing RUSTSEC-2026-0387 exception still requires the separate fail-closed
source/graph review and its negative tests; it is not a patched-bigint claim.
No new advisory ignore or broader licence/source exception was added.

Build scripts of the changed packages were inspected: ICU data scripts select
configuration from the documented data-directory environment variable;
wasm-bindgen selects target/compiler flags and its shared crate fingerprints
schema files and reads a Git revision. None downloads executable code.
Existing native crypto/build dependencies and their previous acceptance remain
in force. The inventory includes inactive platform sources as well as compiled
code; marker counts cannot establish soundness.

These are maintained upstream releases, rather than local crypto forks:
[Tokio 1.54.1](https://github.com/tokio-rs/tokio/releases/tag/tokio-1.54.1)
includes a timer regression fix; [Argon2's changelog](https://github.com/RustCrypto/password-hashes/blob/master/argon2/CHANGELOG.md)
documents the current RustCrypto trait generation and parameter checking.
Existing vetted algorithms remain preferable to implementing cryptography in
this project. OPAQUE 4 still binds its TripleDH hash and RNG to older traits;
those explicit compatibility versions cannot be upgraded independently.

The full local policy/integration gates and the canonical x86-64 Linux WASM
build and byte comparison pass. The four copied artifacts match their SHA-256
sums, and the owned build VM and its loopback listener were stopped. Web
typecheck, strict lint, all 373 unit tests, the production build and 58 browser
smoke tests pass (76 existing opt-in cases skipped). Four real Central/PostgreSQL
browser tests pass at desktop and phone widths, including registration, contacts,
encrypted chat/groups and PWA consent/offline behavior; push delivery is injected
locally through CDP. Their owned services, ports and secrets were removed. CI
remains required before merging. Owner acceptance applies to the 45 listed package/
feature changes and their documented dependency internals, subject to those
checks. First-party unsafe and assembly remain forbidden.

## Separate binding generator

The matching build tool is `wasm-bindgen-cli =0.2.130`, installed from its
published Cargo lockfile. Only the `wasm-bindgen` generator binary is installed
and executed on locally built modules; no test server is launched or shipped.
Its separate 254-package lockfile audit reports four maintenance notices:
RUSTSEC-2023-0028 (`buf_redux`), RUSTSEC-2023-0050 (`multipart`),
RUSTSEC-2023-0081 (`safemem`) and RUSTSEC-2021-0146 (`twoway`). The same four
packages were already present in the 0.2.129 tool lockfile. Upstream's `rouille`
dependency is used by `wasm_bindgen_test_runner`, not by `wasm_bindgen`'s
command path. These are disclosed toolchain maintenance limitations; the tool
audit with `--deny warnings` is not green. No application audit exception was
added, and the Central runtime/development workspace audit remains green.
