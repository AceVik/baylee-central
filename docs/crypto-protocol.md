# Native cryptographic integration spike, transcript version 2

Status: isolated reusable library, not a deployed login service. The owner accepted
the inventoried crypto dependencies on 2026-10-19 subject to review and tests.
No production account, password migration, recovery endpoint or session exists.

## Fixed profile

central-crypto uses opaque-ke 4.0.1, Ristretto255 OPRF and 3DH with SHA-512.
The private suite uses Argon2id v19, 65536 KiB, three passes, four lanes and
64-byte output. Servers cannot negotiate cheaper or arbitrarily costly settings.
The KSF delegates to the approved Argon2 implementation with caller-owned memory,
wiping its 64-MiB work area on success, error or unwinding. It matches the
upstream Argon2 KSF for the same parameters. The zero salt matches that adapter;
account/password separation comes from the OPRF, not an invented salt scheme.

RFC 9807 is a CFRG/IRTF Informational publication, not an IETF Standards Track
certification: https://www.rfc-editor.org/rfc/rfc9807.html
Argon2 parameter reference: https://www.rfc-editor.org/rfc/rfc9106.html#section-4

Passwords are bounded to 1..1024 UTF-8 bytes and are not normalized here. New
password policy belongs in the enrollment service; old migration passwords may
not meet the new policy. OPAQUE proofs/records must also never be logged.
Every received OPAQUE message must match the fixed suite's exact serialized length
before parsing or running the KDF. This rejects trailing-byte aliases accepted by
some upstream parsers. Lengths come from opaque-ke's suite-associated types.
The public client library does not link HTTP or database crates. Server APIs
require an explicit Cargo feature and cannot construct a client wrapping key.

## Binding and key hierarchy

The client supplies a trusted immutable deployment identifier (32 bytes), account
UUIDv7 (16 bytes) and nonzero vault epoch. These values do not authenticate
themselves. OPAQUE binds deployment/account identifiers and a fixed versioned
protocol transcript plus the vault epoch encoded as eight big-endian bytes.
Both peers must agree on that epoch. Transcript v2 replaces the undeployed v1
spike without a downgrade/fallback path; wrapper wire version remains 1.
The wrapper binds deployment/account/epoch and version. Fixed-width transcript
and wrapper context buffers require no heap allocations.
Normal central IDs are allocated by PostgreSQL before enrollment; see postgresql.md.

Generate a random 256-bit master key on the client. Derive its wrapping key with
HKDF-SHA-512 from the OPAQUE CLIENT EXPORT KEY, using separate fixed salt/info
labels. The shared OPAQUE session key is known to both parties and is never
used for the vault. XChaCha20-Poly1305 wraps the master key using a fresh random
192-bit nonce. Wire format is exactly 73 bytes: version 1 + 24-byte nonce +
48-byte ciphertext/tag. No plaintext key serialization API exists.
A parse/authentication error never creates a replacement vault.
Wrapping and unwrapping use fixed zeroized buffers instead of temporary AEAD heap
buffers. A compatibility test compares the detached operation with the upstream
allocating API; the version-1 ciphertext/tag layout is unchanged.

## Recovery code

Registration on the website ([web accounts](web-accounts.md)) also wraps the
master key under a recovery code, shown to the person once.
`vault::RecoveryCode` is that code:

- 20 bytes of operating-system randomness, written as 32 symbols of
  Crockford's base32 (`0123456789ABCDEFGHJKMNPQRSTVWXYZ`), most significant
  bit first, in eight groups of four joined by hyphens:
  `XXXX-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX`. Read back in any case, with `O`
  as 0, `I` and `L` as 1, hyphens and ASCII whitespace anywhere ignored, at
  most 128 input bytes and exactly 32 symbols; anything else is refused.
- Its wrapping key is HKDF-SHA-512 with the 20 bytes as input, the salt
  `baylee-central/recovery/v1` and as info the password wrapper's 86 bytes:
  `baylee-central/master-wrap/v1\0`, the 32-byte issuer, the 16-byte account,
  the epoch as eight big-endian bytes. No stretching: the code carries 160
  bits.
- The wrapper is the 73-byte version-1 format above (XChaCha20-Poly1305, a
  fresh random nonce) with the associated data
  `baylee-central/recovery-wrap/v1\0` followed by the same issuer, account
  and epoch (88 bytes), so neither wrapper opens as the other.

A code may also be made after enrollment, for an account that has none (one
moved from the official gateway, whose client enrolls without it): right
after a sign-in the browser opens the master key with the password's
wrapping key and wraps the same key under a new code, byte for byte as above
([web accounts](web-accounts.md) §"A recovery code after a sign-in"). Nothing
in the code, the key derivation or the wrapper changes.

Known-answer tests pin the written form, the key and the wrapper for two
vectors computed apart from the crate (`tests/recovery-vectors.py`: Python's
base32 translated to Crockford's alphabet, pyca/cryptography's HKDF and
libsodium's XChaCha20-Poly1305); all 584 wrapper bits, a changed issuer,
account, epoch or code, and each wrapper under the other's domain are refused.
Reset by the code, which uses this wrapper, is later work (M1).

Password changes re-enroll the new password and re-wrap the SAME master key.
The service does so since 28.10.2026 ([native login](native-login.md)
§"Password change"): fresh authorization is an OPAQUE proof of the current
password inside the change, bound to the starting session; the record, the
wrapper and a raised credential revision commit in one transaction, which
refuses a revision changed since the start; every session ends in it and the
caller gets a new one; a lost answer is resolved by the old session's status
(ended means committed), so a retry is safe. Nothing in the wire formats, the
key derivation or the wrapper changes. The vault epoch changes when
intentionally creating a new vault, not automatically on each password change,
so the recovery wrapper stays valid across a change.

Deleting an account ([account deletion](account-deletion.md), since
28.10.2026) uses the same proof: a fresh OPAQUE login (KE1, KE2, KE3) inside
the deletion, bound to the starting session, listener and purpose, so a
sign-in's attempt finishes no deletion and a deletion's no sign-in. No key,
wrapper or wire format is added.

Secret-containing public types do not implement Clone or Serialize; their Debug
output is redacted or absent. OPAQUE states are consumed once. Owned secret
buffers use zeroization, including explicit export/session key cleanup. This is
best-effort memory hygiene, not proof against compiler/register copies, allocator
history, swap, crash dumps, hostile devices or every internal dependency temporary.
The upstream Argon2 adapter does not wipe its allocated work area; our wrapper
addresses that specific gap without replacing the algorithm.

## Derived from the master key

Two values are derived one way from the master key, for the pairing that
brings a signed-in website's key to Baylee's client (Baylee
`docs/proposals/deep-links.md` §"Protocol specification", owner's decision
of 30.10.2026). Both are HKDF-SHA-512 with the master key as input, as the
object keys are; each has a salt of its own, and its info is its domain
followed by the 32-byte issuer, the 16-byte account and the epoch as eight
big-endian bytes, the same fields the wrappers bind. The master key keeps
no export; each value comes from a method that answers it alone
(`vault/derived.rs`).

- **Key check**: salt `baylee-central/key-check/v1`, domain
  `baylee-central/key-check/v1\0`, 32 bytes. Central stores it; a proof
  made with it shows the holder of the master key without the stored value
  revealing the key. A new epoch gives a new check.
- **Local vault key**: salt `baylee-central/local-vault/v1`, domain
  `baylee-central/local-vault/v1\0`, then the vault's 32-byte scope. The
  client wraps one local chat vault's data key under it, so that vault
  opens without the password. Never stored, never sent, wiped on drop.

Known-answer tests pin both for two vectors computed apart from the crate
(`tests/derived-vectors.py`: RFC 5869 over Python's standard-library HMAC,
itself checked against RFC 5869's test case 1). A test also holds that every
input (key, issuer, account, epoch, scope) moves every output and that no two
purposes share a value; `Debug` prints neither.

## Pairing

`pairing.rs` (the contract is `web-accounts.md` §"Pairing"). Every value is
32 bytes but the seal (80); every received MAC or proof is compared in
constant time (`hmac`'s `verify_slice`).

- **Claim MAC**: HMAC-SHA-256 under the page's one-time secret `S` over
  `baylee-central/pair-claim/v1\0`, the issuer, the handle, the client's
  X25519 public key and its challenge (SHA-256 of its verifier).
- **Comparison code**: Crockford base32 of the first 30 bits of SHA-256 over
  `baylee-central/pair-code/v1\0`, the handle and the public key, written
  `XXX-XXX`: one glance on both screens, nothing typed.
- **Seal**: RFC 9180 HPKE through the `hpke` crate (owner's choice,
  `dependency-proposal/hpke.md`), base mode, DHKEM(X25519, HKDF-SHA256),
  HKDF-SHA256, ChaCha20-Poly1305; plaintext the 32-byte master key, empty
  AAD, and `info` = `baylee-central/pair-seal/v1\0`, issuer, account, epoch
  (eight big-endian bytes), handle, the pairing's end (eight big-endian
  bytes). 32 bytes of encapsulated key and 48 of ciphertext and tag. A
  low-order public key, whose shared secret is all zero, is refused (RFC 9180
  §7.1.4). The client's private key is derived from 32 bytes of the system's
  randomness and never serialized; the sender's ephemeral key comes from
  getrandom's `SysRng`, which panics on an entropy failure as `opaque-ke`
  does.
- **Approval proof**: HMAC-SHA-256 under the key check over
  `baylee-central/pair-approve/v1\0`, the handle, the public key, the
  challenge and SHA-256 of the seal.

Tests: RFC 9180 Appendix A.2.1 whole through the crate's public API (the
KEM's keys, the encapsulated key, sequence numbers 0 to 256 both ways, the
exporter), with a fixed-output RNG in the test code only; every byte of a
seal; another key, account, issuer, epoch, handle or expiry; a low-order key;
the MACs and code against `tests/pairing-vectors.py` (the standard library's
HMAC and SHA-256); refusals of other secrets, fields and lengths; redacted
`Debug`.

## Evidence and remaining gates

- RFC 9807 Appendix C.1.1 registration upload, KE2/KE3, export key and both peers'
  session keys match published bytes. Login checks both client and server.
  The reference vector uses Identity KSF only in its separate test executable.
  The application suite is private and always uses the fixed Argon2 profile.
  Mutations at all 320 KE2 byte positions and 64 KE3 byte positions are rejected.
- Real serialized enrollment/login, wrong password, missing account, substituted
  account/issuer/epoch, corrupted final proof and replay to another attempt are tested.
- All six OPAQUE message types reject an otherwise valid message with a suffix.
  Both framing and epoch disagreement tests failed before their respective fixes.
- Password change and restoration succeed; old wrapping key cannot open the new
  wrapper. All 584 individual bits of the 73-byte wrapper are mutation-tested, as are foreign
  account/issuer/epoch/key contexts, invalid versions, sizes and redacted Debug.
- This covers one complete published registration/login vector, not every RFC
  suite/vector, a fuzz campaign, constant-time audit or independent review.
- The upstream OPAQUE API uses rand::OsRng's infallible interface: an OS entropy
  failure can panic. No fallback entropy or custom RNG was introduced. Future
  service integration must handle worker failure without issuing a session or
  replacing keys; this spike does not claim that lifecycle is implemented.
- Keep untrusted work off async I/O workers with bounded KDF concurrency. Capacity,
  client-device latency and cancellation behavior still need integration evidence.
- Registration tickets, enumeration-safe account discovery, rate limits, TLS,
  challenge expiry, final session/MFA issuance, persistent server secrets and their
  rotation are unresolved service gates. ServerKeys::generate is explicit and
  ephemeral for tests; no Default constructor silently replaces deployment keys.
- Reset by recovery code, pairing, encrypted object formats, rollback/fresh-device limits,
  browser SSO, legacy migration and production deployment remain separate work.
