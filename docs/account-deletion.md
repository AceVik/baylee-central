# Deleting and exporting an account

*Design, 28.10.2026, roadmap M1 step 3. Not built; three questions for the
owner below.* GDPR Art. 15 (access), 17 (erasure) and 20 (portability). The
privacy notice already says (§7): account data is kept until the account is
deleted, then removed from the running system at once and from backups
within 12 months. Every piece goes through the three passes of [security
assurance](security-assurance.md).

## Export (Art. 15 and 20)

Central cannot read the encrypted data, so an export has two halves:

- **`AccountExport`** (both listeners, a live session, a password sign-in
  under 30 minutes, Central's terms not required): everything Central
  knows *about* the account, in one bounded JSON answer: id, username,
  display name, tag, source, created, last sign-in, roles, recovery wrapper
  present, outstanding reset code's expiry, terms acceptances (subject,
  version, time), sessions (kind, created, expiry), gateways owned (id,
  origin, public metadata, approved, created) and those it administers, the
  number and size of its encrypted objects, and the audit events naming it
  (ids and times only). No credential, wrapper or ciphertext, and no other
  account's data.
- **The encrypted half** is read and decrypted by the client: the account
  document and every private object, through the existing `ReadDocument`,
  `ListObjects` and `ReadObject`. `central-client` gains `Session::export`,
  which returns the metadata and the plaintexts together for the
  application to write out (Baylee's settings decide the file's shape). The
  website (M4) can offer only the metadata until its Worker holds the key
  after a sign-in, as for a password change.

## Deletion (Art. 17)

A stolen session must not delete an account, so deletion carries a fresh
OPAQUE proof of the password, as a password change does:

1. `DeleteAccountStart { login: KE1 }` with a session (either listener;
   Central's terms not required: a person who will not accept new terms
   may still leave) → `LoginChallenge`-like `DeleteChallenge { attempt,
   login: KE2 }`, one unit of the account's login budget, the attempt bound
   to the session, the listener and the purpose.
2. `DeleteAccountFinish { attempt, proof: KE3 }` → `Done`. Refused: an
   administrator (remove the role first; the last administrator cannot be
   removed, so the operator appoints another or the account stays), an
   unfinished legacy move (its bridge still holds the data; see question 1).
3. One transaction under the account's lock deletes the `accounts` row.
   Foreign keys take everything else with it: sessions (and their gateway
   tickets and leases, so every gateway sees the lease refused at its next
   check), private objects, migration archives, the gateways it owns (with
   their approvals, administrator lists and leases; question 2), its places
   on other gateways' administrator lists, its terms acceptances and roles.
   Audit events stay: they name only ids, which no longer resolve. The
   username and tag are free again for a later registration.

What Central cannot delete: what gateways keep about their players (each
gateway's own notice), backups until they rotate out (12 months), and
server logs (14 days).

## Questions for the owner

1. **A moved account, deleted, and the official gateway.** The gateway keeps
   its legacy row after a closed move. Should Central keep a fingerprint
   (SHA-256) of the deleted account's legacy id, so the bridge cannot move
   it in again, or may the person's next sign-in at the official gateway
   move it in anew, as a fresh account? (Keeping it is a small, lasting
   record about a deleted account; not keeping it means "delete" can be
   undone by signing in at the gateway, as long as M8 has not removed the
   legacy path.)
2. **Gateways the account owns.** Delete them with it (their directory
   entries and credentials end, players lose that gateway's sign-in), or
   refuse the deletion until the person hands them over or removes them?
3. **Export freshness.** Is a password sign-in under 30 minutes right for
   an export, or should a live session be enough (it can already read every
   ciphertext and most of the metadata)?
