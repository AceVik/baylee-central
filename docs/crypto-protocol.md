# Native cryptographic integration spike, version 1

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
The public client library does not link HTTP or database crates. Server APIs
require an explicit Cargo feature and cannot construct a client wrapping key.

## Binding and key hierarchy

The client supplies a trusted immutable deployment identifier (32 bytes), account
UUIDv7 (16 bytes) and nonzero vault epoch. These values do not authenticate
themselves. OPAQUE binds deployment/account identifiers and a fixed versioned
protocol transcript. The wrapper binds deployment/account/epoch and version.
Normal central IDs are allocated by PostgreSQL before enrollment; see postgresql.md.

Generate a random 256-bit master key on the client. Derive its wrapping key with
HKDF-SHA-512 from the OPAQUE CLIENT EXPORT KEY, using separate fixed salt/info
labels. The shared OPAQUE session key is known to both parties and is never
used for the vault. XChaCha20-Poly1305 wraps the master key using a fresh random
192-bit nonce. Wire format is exactly 73 bytes: version 1 + 24-byte nonce +
48-byte ciphertext/tag. No plaintext key serialization API exists.
A parse/authentication error never creates a replacement vault.

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

- RFC 9807 Appendix C.1.1 registration upload and export key match published bytes.
  The reference vector uses Identity KSF only in its separate test executable.
  The application suite is private and always uses the fixed Argon2 profile.
- Real serialized enrollment/login, wrong password, missing account, substituted
  account/issuer, corrupted final proof and replay to another attempt are tested.
- Password change and restoration succeed; old wrapping key cannot open the new
  wrapper. Every byte of the 73-byte wrapper is mutation-tested, as are foreign
  account/issuer/epoch/key contexts, invalid versions, sizes and redacted Debug.
- This is not a full published login-vector suite, fuzz campaign, constant-time
  audit or independent review. None is claimed.
- Keep untrusted work off async I/O workers with bounded KDF concurrency. Capacity,
  client-device latency and cancellation behavior still need integration evidence.
- Registration tickets, enumeration-safe account discovery, rate limits, TLS,
  challenge expiry, final session/MFA issuance, persistent server secrets and their
  rotation are unresolved service gates. ServerKeys::generate is explicit and
  ephemeral for tests; no Default constructor silently replaces deployment keys.
- Recovery codes, pairing, encrypted object formats, rollback/fresh-device limits,
  browser SSO, legacy migration and production deployment remain separate work.
