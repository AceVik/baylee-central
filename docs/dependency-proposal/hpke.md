# `hpke` for the pairing's seal

Adopted on 30.10.2026 by the owner's decision ("choose the more secure
variant"), relayed by the PM: the pairing (Baylee
`docs/proposals/deep-links.md` §"Protocol specification") seals the master key
to the client with RFC 9180 HPKE. Writing HPKE here was the alternative and is
ruled out by `AGENTS.md` (no custom cryptographic primitives; vetted crypto is
not replaced by our own). One suite only, DHKEM(X25519, HKDF-SHA256),
HKDF-SHA256, ChaCha20-Poly1305, in PSK mode (below); base mode appears only
in a known-answer test.

```toml
hpke = { version = "=0.15.1", default-features = false, features = ["x25519", "chacha", "alloc"] }
```

`mlkem`, `nistp`, `aes`, `shake` and `getrandom` stay off. The production
randomness is getrandom 0.4's `SysRng` (its `sys_rng` feature), wrapped as
`opaque-ke`'s already is: an entropy failure panics rather than falling back.
The crate's `danger` module (unstable API) is not used.

## PSK mode (30.10.2026)

The second model's review of the pairing found that in base mode Central, or
anyone writing its database, could seal a master key of its own to the
client's public key: it knows the public key and everything `info` binds.
The owner chose PSK mode ("Yes, PSK mode"): the link's secret `S`, which
Central never sees, is the pre-shared key and the handle its ID
(`OpModeS::Psk(PskBundle::new(S, handle))`, `OpModeR::Psk` to open). It
needs no feature, dependency or `danger` API: `PskBundle` and both modes are
the crate's public API under the features above. RFC 9180 Appendix A.2.2,
this suite in PSK mode, is tested whole beside A.2.1.

## Review of the release (30.10.2026)

- **No upstream tag.** rozbb/rust-hpke's newest tag is `v0.14.0`; 0.14.1,
  0.15.0 and 0.15.1 are untagged. The crate was compared with the commit its
  `.cargo_vcs_info.json` names, `7992688f3fa03fc2d26cd6a7e2363b51481d229a`
  ("Forgot to update Cargo.lock", 27.10.2026, on `main` right after "Prep
  v0.15.1 (#124)"). The packaged files equal that commit's: `Cargo.toml.orig`
  is byte for byte its `Cargo.toml`; what differs is crates.io's packaging
  (normalized manifest, VCS info, `Cargo.lock`) and the `.github/` and
  `test-vectors/` folders the manifest excludes.
- **Against 0.15.0** (25.10.2026): `src/` is identical; the version, README,
  changelog ("Fixed typo in docs") and lock file changed. 0.15.0 itself raised
  the MSRV to 1.87 for the crypto-bigint and ctutils advisories
  (RUSTSEC-2026-0387, -0385) and clamps X25519 private keys when serializing
  them (an equivalent key; our tests compare the RFC's private keys after
  clamping). No private key of ours is ever serialized.
- **unsafe, assembly, FFI** (textual search of the packaged sources):
  - `hpke` 0.15.1: none in `src/`; the crate declares no
    `#![forbid(unsafe_code)]` but uses none.
  - `x25519-dalek` 3.0.0 (the dalek monorepo's commit `07bef73`, "Prep v5.0
    major release", crate equal to it): none; `no_std`.
  - `curve25519-dalek` 5.0.0 (same commit, crate equal to it): 119 `unsafe`
    sites in the SIMD vector backends (AVX2, IFMA, `packed_simd`, compiled
    only for x86_64 with those target features) and one in `constants.rs`
    (the Ristretto basepoint table). No assembly, no FFI. The same profile as
    `curve25519-dalek` 4.1.3, accepted with `opaque-ke`
    (`../crypto-proposal/README.md`).

## A second major version of `curve25519-dalek`

`opaque-ke` 4.0.1 pins `curve25519-dalek` 4.1.3; `x25519-dalek` 3.0.0 needs
5.0.0. Both now build into `central-crypto`, each used by one protocol: OPAQUE
on 4.1.3, the pairing's X25519 on 5.0.0. They share no types or keys, so the
duplicate costs build time and size, not correctness; it goes away when
`opaque-ke` moves to 5. `cargo deny`'s duplicate check carries it as a known
exception with this reason. Baylee's client already linked 5.0.0 and
`x25519-dalek` 3.0.0 (`baylee-seal`), so the client gains no new crate.

## What else enters the lock

Two entries the policy check (`cargo xtask policy`) asked for beyond the
three crates. The owner's approval of 30.10.2026 names only the three, so
`approved-packages.json` lists these two as a **proposal** awaiting the
owner's decision, not as approved:

- `fiat-crypto` 0.3.0: `curve25519-dalek` 5's dependency under
  `cfg(curve25519_dalek_backend = "fiat")`, which no build here sets, the
  same as `fiat-crypto` 0.2.9 is for 4.1.3. In the lock, compiled on no
  target; `deny.toml` skips the older one as a duplicate. MIT OR Apache-2.0
  OR BSD-1-Clause; generated arithmetic without `unsafe`, `asm!` or a build
  script.
- `subtle` 2.6.1's `const-generics` feature, which `curve25519-dalek` 5 asks
  for: an already approved package, one more feature. It adds a single safe
  impl, `ConditionallySelectable` for `[T; N]`, and is compiled wherever
  `curve25519-dalek` 5 is.

`hpke`'s `x25519` feature turns on its `hkdfsha2` and `x25519-dalek`'s
`static_secrets`. Nothing else is new; `hmac` 0.12.1 and getrandom's
`sys_rng` were approved before.

## The bigint advisory's guard

`cargo xtask crypto-scope` fingerprints `central-crypto` and pins its graph
(`../crypto-proposal/bigint-advisory.md`). It now pins `hpke`, `x25519-dalek`
and the second `curve25519-dalek` generation too, with their features and
reverse edges; none of them reaches `elliptic-curve` or `crypto-bigint`.

## How the suite is held

`central-crypto`'s `pairing/tests.rs` runs RFC 9180 Appendix A.2.1 whole
through the crate's public API (the KEM's keys, the encapsulated key, every
sequence number from 0 to 256 in both directions, the three exported values),
with a fixed-output RNG that exists only in the test code, so a later release
cannot change the suite silently. Misuse tests cover every changed byte of a
seal, a wrong key, account, issuer, epoch, handle or expiry, and a low-order
public key (the all-zero shared secret RFC 9180 §7.1.4 refuses). The vectors
come from rfc-editor.org's text of RFC 9180, not from the crate's repository.

Before landing, a second model reviews this use of the crate (`Review:
pending` in the commit; the PM arranges it).
