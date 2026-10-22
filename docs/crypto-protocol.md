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
The spike tests this cryptographic operation only. A real change still needs
fresh authorization, atomic database commit, revision conflict handling, session
revocation and idempotent retry. The vault epoch changes when intentionally
creating a new vault, not automatically on each password change.

Secret-containing public types do not implement Clone or Serialize; their Debug
output is redacted or absent. OPAQUE states are consumed once. Owned secret
buffers use zeroization, including explicit export/session key cleanup. This is
best-effort memory hygiene, not proof against compiler/register copies, allocator
history, swap, crash dumps, hostile devices or every internal dependency temporary.
The upstream Argon2 adapter does not wipe its allocated work area; our wrapper
addresses that specific gap without replacing the algorithm.

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
