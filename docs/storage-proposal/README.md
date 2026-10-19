# SeaORM 2 / PostgreSQL dependency review

Owner selected PostgreSQL 18+, database-generated UUIDv7, SeaORM 2 and versioned
migrations on 2026-10-19. The owner accepted the transitive unsafe/native graph below on 2026-10-19
subject to review and tests. It is used by the isolated development adapter and
explicit migrations; see ../database-development.md for evidence and limits.

Candidate direct dependencies: sea-orm 2.0.4 and sea-orm-migration 2.0.4,
default features disabled, PostgreSQL + Tokio/Rustls, entity macros and UUID values.
The migration library transitively enables SeaORM defaults/schema support;
therefore the resolved graph is larger than the explicitly requested feature set.
Do not call schema-sync in the service.

Cargo.lock resolves 294 external packages across target/optional metadata.
That is not a count of code linked into each executable. Cargo tree shows no
active sea-orm-cli or libsqlite3-sys path on the current Windows target; no MySQL
or SQLite runtime backend is requested. Each actual target needs its own checks.
packages.json freezes source versions/features for review.

TLS path: SQLx 0.9.0 -> rustls 0.23.46 -> ring 0.17.14, which contains native
C/assembly and a build script. SQLx, allocators/locking and system adapters also
contain internal unsafe. All first-party Rust calls use safe APIs; the project
ban on first-party unsafe/assembly remains. No custom crypto/TLS substitute.

RustSec cargo-audit 0.22.2: no known advisory/warning on 2026-10-19.
cargo-deny advisory/source checks pass. Initial license policy correctly rejects
unlisted Zlib, ISC and CDLA-Permissive-2.0; source licenses have been inventoried
and the three named permissive licenses were added within the frozen package
policy during adoption. No blanket advisory or source exclusions were added.
These are source/advisory checks, not an audit or successful runtime tests.
The combined graph has exact duplicate-version allowances, each with a reason.
It retains approved crypto-common 0.1.7 and libc 0.2.190 instead of proposal
patches 0.1.6/0.2.191; their corresponding std/extra_traits feature changes were
reviewed and included explicitly in the approved feature inventory.

Sources:
- https://github.com/SeaQL/sea-orm/releases/tag/2.0.4
- https://docs.rs/sea-orm/2.0.4/
- https://docs.rs/sea-orm-migration/2.0.4/
- https://www.sea-ql.org/SeaORM/docs/migration/setting-up-migration/
- https://www.postgresql.org/docs/18/functions-uuid.html

Next verification: native Linux/Windows/macOS build, PostgreSQL 18 migrations and
returning UUIDv7, repeat/failed upgrade and unsupported-version rejection, a
restricted runtime role, parameterized ownership queries, TLS identity rejection,
transaction rollback/cancellation and exact owned-container cleanup.
No production migration or connection is authorized by these local tests.

Marker counts include comments, tests and cfg-inactive code; they are an inventory
signal, not proof of unsoundness or proof that marker-free code is secure.

| Package | License | Files with unsafe/asm markers | Build script |
| --- | --- | ---: | --- |
| ahash 0.8.12 | MIT OR Apache-2.0 | 1 | True |
| aho-corasick 1.1.5 | Unlicense OR MIT | 10 | False |
| aliasable 0.1.3 | MIT | 4 | False |
| allocator-api2 0.2.21 | MIT OR Apache-2.0 | 12 | False |
| android_system_properties 0.1.6 | MIT OR Apache-2.0 | 1 | False |
| arrayvec 0.7.8 | MIT OR Apache-2.0 | 4 | False |
| arrow 60.0.0 | Apache-2.0 | 8 | False |
| arrow-arith 60.0.0 | Apache-2.0 | 2 | False |
| arrow-array 60.0.0 | Apache-2.0 AND MIT | 37 | False |
| arrow-buffer 60.0.0 | Apache-2.0 | 18 | False |
| arrow-cast 60.0.0 | Apache-2.0 | 7 | False |
| arrow-cmp 60.0.0 | Apache-2.0 | 1 | False |
| arrow-data 60.0.0 | Apache-2.0 | 7 | False |
| arrow-ord 60.0.0 | Apache-2.0 | 3 | False |
| arrow-row 60.0.0 | Apache-2.0 | 4 | False |
| arrow-schema 60.0.0 | Apache-2.0 | 1 | False |
| arrow-select 60.0.0 | Apache-2.0 | 10 | False |
| arrow-string 60.0.0 | Apache-2.0 | 3 | False |
| async-stream 0.3.6 | MIT | 1 | False |
| async-stream-impl 0.3.6 | MIT | 1 | False |
| async-trait 0.1.92 | MIT OR Apache-2.0 | 4 | False |
| atoi 2.0.0 | MIT | 0 | False |
| atoi 3.1.1 | MIT | 0 | False |
| autocfg 1.5.1 | Apache-2.0 OR MIT | 0 | False |
| base64 0.22.1 | MIT OR Apache-2.0 | 2 | False |
| base64 0.23.1 | MIT OR Apache-2.0 | 7 | False |
| bigdecimal 0.4.11 | MIT/Apache-2.0 | 0 | True |
| bitflags 2.13.2 | MIT OR Apache-2.0 | 2 | False |
| block-buffer 0.10.4 | MIT OR Apache-2.0 | 2 | False |
| block-buffer 0.12.1 | MIT OR Apache-2.0 | 3 | False |
| borsh 1.8.1 | MIT OR Apache-2.0 | 3 | True |
| borsh-derive 1.8.1 | Apache-2.0 | 0 | False |
| bumpalo 3.20.3 | MIT OR Apache-2.0 | 7 | False |
| byteorder 1.5.0 | Unlicense OR MIT | 2 | False |
| bytes 1.12.1 | MIT | 13 | False |
| cc 1.9.0 | MIT OR Apache-2.0 | 7 | False |
| cfg_aliases 0.2.2 | MIT | 0 | False |
| cfg-if 1.0.5 | MIT OR Apache-2.0 | 0 | False |
| chacha20 0.10.2 | MIT OR Apache-2.0 | 6 | False |
| chrono 0.4.45 | MIT OR Apache-2.0 | 7 | False |
| cmov 0.5.4 | Apache-2.0 OR MIT | 5 | False |
| const-random 0.1.18 | MIT OR Apache-2.0 | 0 | False |
| const-random-macro 0.1.16 | MIT OR Apache-2.0 | 0 | False |
| core-foundation-sys 0.8.7 | MIT OR Apache-2.0 | 4 | False |
| cpufeatures 0.2.17 | MIT OR Apache-2.0 | 3 | False |
| cpufeatures 0.3.1 | MIT OR Apache-2.0 | 3 | False |
| crc 3.4.0 | MIT OR Apache-2.0 | 0 | False |
| crc-catalog 2.5.0 | MIT OR Apache-2.0 | 0 | False |
| crossbeam-queue 0.3.14 | MIT OR Apache-2.0 | 2 | False |
| crossbeam-utils 0.8.23 | MIT OR Apache-2.0 | 8 | True |
| crunchy 0.2.4 | MIT | 0 | True |
| crypto-common 0.1.6 | MIT OR Apache-2.0 | 0 | False |
| crypto-common 0.2.2 | MIT OR Apache-2.0 | 0 | False |
| ctutils 0.4.3 | Apache-2.0 OR MIT | 1 | False |
| darling 0.20.11 | MIT | 0 | False |
| darling_core 0.20.11 | MIT | 1 | False |
| darling_macro 0.20.11 | MIT | 0 | False |
| deranged 0.5.9 | MIT OR Apache-2.0 | 3 | False |
| derive_more 2.1.1 | MIT | 3 | False |
| derive_more-impl 2.1.1 | MIT | 1 | False |
| derive-where 1.7.0 | MIT OR Apache-2.0 | 5 | False |
| digest 0.10.7 | MIT OR Apache-2.0 | 0 | False |
| digest 0.11.3 | MIT OR Apache-2.0 | 0 | False |
| displaydoc 0.2.7 | MIT OR Apache-2.0 | 0 | False |
| dotenvy 0.15.7 | MIT | 0 | False |
| either 1.19.0 | MIT OR Apache-2.0 | 1 | False |
| equivalent 1.0.3 | Apache-2.0 OR MIT | 0 | False |
| etcetera 0.11.0 | MIT OR Apache-2.0 | 4 | False |
| event-listener 5.4.2 | Apache-2.0 OR MIT | 4 | False |
| find-msvc-tools 0.1.15 | MIT OR Apache-2.0 | 6 | False |
| flume 0.12.0 | Apache-2.0/MIT | 1 | False |
| fnv 1.0.7 | Apache-2.0 / MIT | 0 | False |
| foldhash 0.2.0 | Zlib | 3 | False |
| form_urlencoded 1.2.2 | MIT OR Apache-2.0 | 1 | False |
| futures-channel 0.3.34 | MIT OR Apache-2.0 | 3 | False |
| futures-core 0.3.34 | MIT OR Apache-2.0 | 2 | False |
| futures-executor 0.3.34 | MIT OR Apache-2.0 | 2 | False |
| futures-intrusive 0.5.0 | MIT OR Apache-2.0 | 16 | False |
| futures-io 0.3.34 | MIT OR Apache-2.0 | 0 | False |
| futures-sink 0.3.34 | MIT OR Apache-2.0 | 1 | False |
| futures-task 0.3.34 | MIT OR Apache-2.0 | 4 | False |
| futures-util 0.3.34 | MIT OR Apache-2.0 | 21 | False |
| generic-array 0.14.9 | MIT | 7 | True |
| getrandom 0.2.17 | MIT OR Apache-2.0 | 21 | False |
| getrandom 0.3.4 | MIT OR Apache-2.0 | 28 | True |
| getrandom 0.4.3 | MIT OR Apache-2.0 | 29 | True |
| glob 0.3.4 | MIT OR Apache-2.0 | 0 | False |
| half 2.7.1 | MIT OR Apache-2.0 | 5 | False |
| hashbrown 0.16.1 | MIT OR Apache-2.0 | 19 | False |
| hashbrown 0.17.1 | MIT OR Apache-2.0 | 19 | False |
| hashlink 0.11.1 | MIT OR Apache-2.0 | 1 | False |
| heck 0.4.1 | MIT OR Apache-2.0 | 0 | False |
| heck 0.5.0 | MIT OR Apache-2.0 | 0 | False |
| hex 0.4.3 | MIT OR Apache-2.0 | 0 | False |
| hkdf 0.13.0 | MIT OR Apache-2.0 | 0 | False |
| hmac 0.13.0 | MIT OR Apache-2.0 | 0 | False |
| hybrid-array 0.4.15 | MIT OR Apache-2.0 | 6 | False |
| iana-time-zone 0.1.65 | MIT OR Apache-2.0 | 6 | False |
| iana-time-zone-haiku 0.1.2 | MIT OR Apache-2.0 | 1 | True |
| icu_collections 2.3.0 | Unicode-3.0 | 2 | False |
| icu_locale_core 2.3.0 | Unicode-3.0 | 3 | False |
| icu_normalizer 2.3.0 | Unicode-3.0 | 2 | False |
| icu_normalizer_data 2.3.0 | Unicode-3.0 | 0 | True |
| icu_properties 2.3.0 | Unicode-3.0 | 0 | False |
| icu_properties_data 2.3.0 | Unicode-3.0 | 0 | True |
| icu_provider 2.3.1 | Unicode-3.0 | 4 | False |
| ident_case 1.0.1 | MIT/Apache-2.0 | 0 | False |
| idna 1.1.0 | MIT OR Apache-2.0 | 1 | False |
| idna_adapter 1.2.2 | Apache-2.0 OR MIT | 0 | False |
| indexmap 2.14.2 | Apache-2.0 OR MIT | 5 | False |
| indoc 2.0.8 | MIT OR Apache-2.0 | 0 | False |
| itertools 0.14.0 | MIT OR Apache-2.0 | 3 | False |
| itoa 1.0.18 | MIT OR Apache-2.0 | 1 | False |
| js-sys 0.3.106 | MIT OR Apache-2.0 | 5 | False |
| lazy_static 1.5.1 | MIT OR Apache-2.0 | 2 | False |
| lexical-core 1.0.6 | MIT/Apache-2.0 | 1 | False |
| lexical-parse-float 1.0.6 | MIT/Apache-2.0 | 7 | False |
| lexical-parse-integer 1.0.6 | MIT/Apache-2.0 | 2 | False |
| lexical-util 1.0.7 | MIT/Apache-2.0 | 6 | False |
| lexical-write-float 1.0.6 | MIT/Apache-2.0 | 7 | False |
| lexical-write-integer 1.0.6 | MIT/Apache-2.0 | 7 | False |
| libc 0.2.191 | MIT OR Apache-2.0 | 70 | True |
| libm 0.2.16 | MIT | 9 | True |
| libsqlite3-sys 0.37.0 | MIT | 10 | True |
| litemap 0.8.3 | Unicode-3.0 | 1 | False |
| lock_api 0.4.14 | MIT OR Apache-2.0 | 4 | False |
| log 0.4.34 | MIT OR Apache-2.0 | 1 | False |
| mac_address 1.2.1 | MIT OR Apache-2.0 | 2 | False |
| matchers 0.2.0 | MIT | 1 | False |
| md-5 0.11.0 | MIT OR Apache-2.0 | 1 | False |
| memchr 2.8.3 | Unlicense OR MIT | 20 | False |
| memoffset 0.9.1 | MIT | 2 | True |
| mio 1.2.4 | MIT | 34 | False |
| nix 0.31.3 | MIT | 73 | True |
| num-bigint 0.4.8 | MIT OR Apache-2.0 | 6 | False |
| num-bigint 0.5.1 | MIT OR Apache-2.0 | 6 | False |
| num-complex 0.4.6 | MIT OR Apache-2.0 | 1 | False |
| num-conv 0.2.2 | MIT OR Apache-2.0 | 0 | False |
| num-integer 0.1.47 | MIT OR Apache-2.0 | 0 | False |
| num-traits 0.2.19 | MIT OR Apache-2.0 | 1 | True |
| once_cell 1.21.4 | MIT OR Apache-2.0 | 7 | False |
| ordered-float 4.6.0 | MIT | 2 | False |
| ouroboros 0.18.5 | MIT OR Apache-2.0 | 1 | False |
| ouroboros_macro 0.18.5 | MIT OR Apache-2.0 | 8 | False |
| parking 2.2.1 | Apache-2.0 OR MIT | 0 | False |
| parking_lot 0.12.5 | MIT OR Apache-2.0 | 9 | False |
| parking_lot_core 0.9.12 | MIT OR Apache-2.0 | 15 | True |
| percent-encoding 2.3.2 | MIT OR Apache-2.0 | 1 | False |
| pgvector 0.4.2 | MIT OR Apache-2.0 | 0 | False |
| pin-project-lite 0.2.17 | Apache-2.0 OR MIT | 18 | False |
| pkg-config 0.3.34 | MIT OR Apache-2.0 | 0 | False |
| pluralizer 0.5.0 | MIT/Apache-2.0 | 0 | False |
| potential_utf 0.1.6 | Unicode-3.0 | 3 | False |
| powerfmt 0.2.1 | MIT OR Apache-2.0 | 2 | False |
| ppv-lite86 0.2.21 | MIT OR Apache-2.0 | 5 | False |
| proc-macro-crate 3.5.0 | MIT OR Apache-2.0 | 0 | False |
| proc-macro2 1.0.107 | MIT OR Apache-2.0 | 3 | True |
| proc-macro2-diagnostics 0.10.1 | MIT/Apache-2.0 | 0 | True |
| quote 1.0.47 | MIT OR Apache-2.0 | 0 | True |
| r-efi 5.3.0 | MIT OR Apache-2.0 OR LGPL-2.1-or-later | 3 | False |
| r-efi 6.0.0 | MIT OR Apache-2.0 OR LGPL-2.1-or-later | 49 | False |
| rand 0.10.3 | MIT OR Apache-2.0 | 5 | False |
| rand 0.8.8 | MIT OR Apache-2.0 | 7 | False |
| rand 0.9.5 | MIT OR Apache-2.0 | 7 | False |
| rand_chacha 0.3.1 | MIT OR Apache-2.0 | 1 | False |
| rand_chacha 0.9.0 | MIT OR Apache-2.0 | 0 | False |
| rand_core 0.10.1 | MIT OR Apache-2.0 | 0 | False |
| rand_core 0.6.4 | MIT OR Apache-2.0 | 1 | False |
| rand_core 0.9.5 | MIT OR Apache-2.0 | 0 | False |
| redox_syscall 0.5.18 | MIT | 17 | False |
| regex 1.13.1 | MIT OR Apache-2.0 | 1 | False |
| regex-automata 0.4.18 | MIT OR Apache-2.0 | 10 | False |
| regex-syntax 0.8.11 | MIT OR Apache-2.0 | 0 | False |
| ring 0.17.14 | Apache-2.0 AND ISC | 50 | True |
| rust_decimal 1.43.0 | MIT | 0 | True |
| rustc_version 0.4.1 | MIT OR Apache-2.0 | 0 | False |
| rustls 0.23.46 | Apache-2.0 OR ISC OR MIT | 4 | True |
| rustls-pki-types 1.15.1 | MIT OR Apache-2.0 | 1 | False |
| rustls-webpki 0.103.15 | ISC | 0 | False |
| rustversion 1.0.23 | MIT OR Apache-2.0 | 2 | True |
| ryu 1.0.23 | Apache-2.0 OR BSL-1.0 | 13 | False |
| scopeguard 1.2.0 | MIT OR Apache-2.0 | 1 | False |
| sea-bae 0.2.2 | MIT | 0 | False |
| sea-orm 2.0.4 | MIT OR Apache-2.0 | 1 | False |
| sea-orm-arrow 2.0.0-rc.5 | MIT OR Apache-2.0 | 0 | False |
| sea-orm-cli 2.0.4 | MIT OR Apache-2.0 | 0 | False |
| sea-orm-macros 2.0.4 | MIT OR Apache-2.0 | 1 | False |
| sea-orm-migration 2.0.4 | MIT OR Apache-2.0 | 1 | False |
| sea-query 1.0.2 | MIT OR Apache-2.0 | 0 | False |
| sea-query-derive 1.0.0 | MIT OR Apache-2.0 | 0 | False |
| sea-query-sqlx 0.9.1 | MIT OR Apache-2.0 | 0 | False |
| sea-schema 0.18.1 | MIT OR Apache-2.0 | 0 | False |
| sea-schema-derive 0.3.0 | MIT OR Apache-2.0 | 0 | False |
| semver 1.0.28 | MIT OR Apache-2.0 | 2 | False |
| serde 1.0.229 | MIT OR Apache-2.0 | 2 | True |
| serde_core 1.0.229 | MIT OR Apache-2.0 | 2 | True |
| serde_derive 1.0.229 | MIT OR Apache-2.0 | 0 | False |
| serde_json 1.0.154 | MIT OR Apache-2.0 | 5 | True |
| sha1 0.11.0 | MIT OR Apache-2.0 | 4 | False |
| sha2 0.10.9 | MIT OR Apache-2.0 | 8 | False |
| sha2 0.11.1 | MIT OR Apache-2.0 | 14 | False |
| sharded-slab 0.1.7 | MIT | 6 | False |
| shlex 2.0.1 | MIT OR Apache-2.0 | 1 | False |
| slab 0.4.12 | MIT | 1 | False |
| smallvec 1.16.3 | MIT OR Apache-2.0 | 2 | False |
| socket2 0.6.5 | MIT OR Apache-2.0 | 6 | False |
| spin 0.9.9 | MIT | 7 | False |
| sqlx 0.9.0 | MIT OR Apache-2.0 | 1 | False |
| sqlx-core 0.9.0 | MIT OR Apache-2.0 | 2 | False |
| sqlx-macros 0.9.0 | MIT OR Apache-2.0 | 0 | False |
| sqlx-macros-core 0.9.0 | MIT OR Apache-2.0 | 0 | False |
| sqlx-mysql 0.9.0 | MIT OR Apache-2.0 | 0 | False |
| sqlx-postgres 0.9.0 | MIT OR Apache-2.0 | 1 | False |
| sqlx-sqlite 0.9.0 | MIT OR Apache-2.0 | 15 | False |
| stable_deref_trait 1.2.1 | MIT OR Apache-2.0 | 1 | False |
| static_assertions 1.1.0 | MIT OR Apache-2.0 | 1 | False |
| stringprep 0.1.5 | MIT/Apache-2.0 | 0 | False |
| strum 0.28.0 | MIT | 0 | False |
| subtle 2.6.1 | BSD-3-Clause | 1 | False |
| syn 2.0.119 | MIT OR Apache-2.0 | 16 | False |
| syn 3.0.9 | MIT OR Apache-2.0 | 15 | False |
| synstructure 0.14.0 | MIT | 1 | False |
| thiserror 2.0.21 | MIT OR Apache-2.0 | 0 | True |
| thiserror-impl 2.0.21 | MIT OR Apache-2.0 | 1 | False |
| thread_local 1.1.10 | MIT OR Apache-2.0 | 2 | False |
| time 0.3.55 | MIT OR Apache-2.0 | 34 | False |
| time-core 0.1.9 | MIT OR Apache-2.0 | 0 | False |
| time-macros 0.2.32 | MIT OR Apache-2.0 | 7 | False |
| tiny-keccak 2.0.2 | CC0-1.0 | 1 | True |
| tinystr 0.8.4 | Unicode-3.0 | 5 | False |
| tinyvec 1.13.3 | Zlib OR Apache-2.0 OR MIT | 3 | False |
| tokio 1.53.3 | MIT | 153 | False |
| tokio-macros 2.7.2 | MIT | 0 | False |
| tokio-stream 0.1.19 | MIT | 2 | False |
| toml_datetime 1.1.2+spec-1.1.0 | MIT OR Apache-2.0 | 1 | False |
| toml_edit 0.25.17+spec-1.1.0 | MIT OR Apache-2.0 | 0 | False |
| toml_parser 1.1.5+spec-1.1.0 | MIT OR Apache-2.0 | 6 | False |
| tracing 0.1.44 | MIT | 3 | False |
| tracing-attributes 0.1.31 | MIT | 1 | False |
| tracing-core 0.1.36 | MIT | 7 | False |
| tracing-subscriber 0.3.23 | MIT | 8 | False |
| typenum 1.20.1 | MIT OR Apache-2.0 | 0 | False |
| unicode-bidi 0.3.18 | MIT OR Apache-2.0 | 1 | False |
| unicode-ident 1.0.27 | (MIT OR Apache-2.0) AND Unicode-3.0 | 1 | False |
| unicode-normalization 0.1.26 | (MIT OR Apache-2.0) AND Unicode-3.0 | 1 | False |
| unicode-properties 0.1.4 | MIT/Apache-2.0 | 0 | False |
| unicode-xid 0.2.6 | MIT OR Apache-2.0 | 0 | False |
| untrusted 0.9.0 | ISC | 0 | False |
| url 2.5.8 | MIT OR Apache-2.0 | 1 | False |
| utf8_iter 1.0.4 | Apache-2.0 OR MIT | 2 | False |
| uuid 1.28.0 | Apache-2.0 OR MIT | 3 | False |
| vcpkg 0.2.15 | MIT/Apache-2.0 | 0 | False |
| version_check 0.9.5 | MIT/Apache-2.0 | 0 | False |
| wasi 0.11.1+wasi-snapshot-preview1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | 1 | False |
| wasip2 1.0.4+wasi-0.2.12 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | 3 | False |
| wasm-bindgen 0.2.129 | MIT OR Apache-2.0 | 10 | True |
| wasm-bindgen-macro 0.2.129 | MIT OR Apache-2.0 | 0 | False |
| wasm-bindgen-macro-support 0.2.129 | MIT OR Apache-2.0 | 3 | False |
| wasm-bindgen-shared 0.2.129 | MIT OR Apache-2.0 | 0 | True |
| web-time 1.1.0 | MIT OR Apache-2.0 | 0 | False |
| webpki-roots 1.0.9 | CDLA-Permissive-2.0 | 0 | False |
| whoami 2.1.3 | Apache-2.0 OR BSL-1.0 OR MIT | 3 | False |
| windows_aarch64_gnullvm 0.52.6 | MIT OR Apache-2.0 | 0 | True |
| windows_aarch64_msvc 0.52.6 | MIT OR Apache-2.0 | 0 | True |
| windows_i686_gnu 0.52.6 | MIT OR Apache-2.0 | 0 | True |
| windows_i686_gnullvm 0.52.6 | MIT OR Apache-2.0 | 0 | True |
| windows_i686_msvc 0.52.6 | MIT OR Apache-2.0 | 0 | True |
| windows_x86_64_gnu 0.52.6 | MIT OR Apache-2.0 | 0 | True |
| windows_x86_64_gnullvm 0.52.6 | MIT OR Apache-2.0 | 0 | True |
| windows_x86_64_msvc 0.52.6 | MIT OR Apache-2.0 | 0 | True |
| windows-core 0.62.2 | MIT OR Apache-2.0 | 27 | False |
| windows-implement 0.60.2 | MIT OR Apache-2.0 | 2 | False |
| windows-interface 0.59.3 | MIT OR Apache-2.0 | 1 | False |
| windows-link 0.2.1 | MIT OR Apache-2.0 | 0 | False |
| windows-result 0.4.1 | MIT OR Apache-2.0 | 6 | False |
| windows-strings 0.5.1 | MIT OR Apache-2.0 | 11 | False |
| windows-sys 0.52.0 | MIT OR Apache-2.0 | 132 | False |
| windows-sys 0.61.2 | MIT OR Apache-2.0 | 192 | False |
| windows-targets 0.52.6 | MIT OR Apache-2.0 | 0 | False |
| winnow 1.0.4 | MIT | 9 | False |
| wit-bindgen 0.57.1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | 16 | True |
| writeable 0.6.4 | Unicode-3.0 | 2 | False |
| yansi 1.0.1 | MIT OR Apache-2.0 | 2 | False |
| yoke 0.8.3 | Unicode-3.0 | 8 | False |
| yoke-derive 0.8.4 | Unicode-3.0 | 1 | False |
| zerocopy 0.8.63 | BSD-2-Clause OR Apache-2.0 OR MIT | 122 | True |
| zerocopy-derive 0.8.63 | BSD-2-Clause OR Apache-2.0 OR MIT | 47 | False |
| zerofrom 0.1.8 | Unicode-3.0 | 0 | False |
| zerofrom-derive 0.1.8 | Unicode-3.0 | 0 | False |
| zeroize 1.9.1 | Apache-2.0 OR MIT | 7 | False |
| zerotrie 0.2.5 | Unicode-3.0 | 4 | False |
| zerovec 0.11.8 | Unicode-3.0 | 31 | False |
| zerovec-derive 0.11.6 | Unicode-3.0 | 6 | False |
| zmij 1.0.23 | MIT | 2 | True |
