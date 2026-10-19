# HTTP foundation dependency proposal

Status: resolved only; not adopted by the application workspace. Owner acceptance of dependency-internal unsafe/assembly is required before adoption. No crypto/database dependencies are included.

The proposed direct dependencies are Axum 0.8.9, Tokio 1.53.3, Tower 0.5.3, Tower HTTP 0.6.8 and HTTP Body Util 0.1.5 (tests). Cargo.lock fixes 49 external packages, including target-specific packages. All application-facing APIs will remain safe Rust. The Rust standard library, compiler/runtime and OS are also part of the trusted computing base.

Marker counts below are a conservative textual inventory across package Rust sources, including tests/comments/cfg-inactive code. They are NOT a soundness audit or proof that a package without markers has no unsafe dependencies or macro expansions. Features and sources are reproducible from Cargo.toml/Cargo.lock; advisory verification and HTTP boundary tests are required before adoption/release.

| Package | License expression | Files with unsafe/asm markers | Build script |
| --- | --- | ---: | --- |
| atomic-waker 1.1.2 | Apache-2.0 OR MIT | 1 | False |
| axum 0.8.9 | MIT | 0 | False |
| axum-core 0.5.6 | MIT | 0 | False |
| bitflags 2.13.2 | MIT OR Apache-2.0 | 2 | False |
| bytes 1.12.1 | MIT | 13 | False |
| errno 0.3.14 | MIT OR Apache-2.0 | 3 | False |
| futures-core 0.3.34 | MIT OR Apache-2.0 | 2 | False |
| futures-sink 0.3.34 | MIT OR Apache-2.0 | 1 | False |
| futures-task 0.3.34 | MIT OR Apache-2.0 | 4 | False |
| futures-util 0.3.34 | MIT OR Apache-2.0 | 21 | False |
| http 1.5.0 | MIT OR Apache-2.0 | 11 | False |
| http-body 1.1.0 | MIT | 1 | False |
| http-body-util 0.1.5 | MIT | 1 | False |
| httparse 1.10.1 | MIT OR Apache-2.0 | 9 | True |
| httpdate 1.0.3 | MIT OR Apache-2.0 | 0 | False |
| hyper 1.12.0 | MIT | 16 | False |
| hyper-util 0.1.21 | MIT | 7 | False |
| itoa 1.0.18 | MIT OR Apache-2.0 | 1 | False |
| libc 0.2.190 | MIT OR Apache-2.0 | 70 | True |
| matchit 0.8.4 | MIT AND BSD-3-Clause | 2 | False |
| memchr 2.8.3 | Unlicense OR MIT | 20 | False |
| mime 0.3.17 | MIT OR Apache-2.0 | 1 | False |
| mio 1.2.4 | MIT | 34 | False |
| once_cell 1.21.4 | MIT OR Apache-2.0 | 7 | False |
| percent-encoding 2.3.2 | MIT OR Apache-2.0 | 1 | False |
| pin-project-lite 0.2.17 | Apache-2.0 OR MIT | 18 | False |
| proc-macro2 1.0.107 | MIT OR Apache-2.0 | 3 | True |
| quote 1.0.47 | MIT OR Apache-2.0 | 0 | True |
| serde_core 1.0.229 | MIT OR Apache-2.0 | 2 | True |
| serde_derive 1.0.229 | MIT OR Apache-2.0 | 0 | False |
| signal-hook-registry 1.4.8 | MIT OR Apache-2.0 | 3 | False |
| slab 0.4.12 | MIT | 1 | False |
| smallvec 1.16.3 | MIT OR Apache-2.0 | 2 | False |
| socket2 0.6.5 | MIT OR Apache-2.0 | 6 | False |
| syn 3.0.9 | MIT OR Apache-2.0 | 15 | False |
| sync_wrapper 1.0.2 | Apache-2.0 | 1 | False |
| tokio 1.53.3 | MIT | 153 | False |
| tokio-macros 2.7.2 | MIT | 0 | False |
| tokio-util 0.7.19 | MIT | 12 | False |
| tower 0.5.3 | MIT | 0 | False |
| tower-http 0.6.8 | MIT | 0 | False |
| tower-layer 0.3.3 | MIT | 0 | False |
| tower-service 0.3.3 | MIT | 0 | False |
| tracing 0.1.44 | MIT | 3 | False |
| tracing-core 0.1.36 | MIT | 7 | False |
| unicode-ident 1.0.27 | (MIT OR Apache-2.0) AND Unicode-3.0 | 1 | False |
| wasi 0.11.1+wasi-snapshot-preview1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | 1 | False |
| windows-link 0.2.1 | MIT OR Apache-2.0 | 0 | False |
| windows-sys 0.61.2 | MIT OR Apache-2.0 | 192 | False |

Owner acceptance on 2026-10-19: use this HTTP graph through safe APIs after security checks. Runtime adoption uses a smaller subset, pinned in the root Cargo.lock; approved features/versions are enforced by scripts/check-policy.ps1. RustSec and cargo-deny pass for the adopted graph. No claim of a full third-party source audit.
