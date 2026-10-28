# Deleting and exporting an account

*Design, 28.10.2026, roadmap M1 step 3; the owner's decisions of the same
day below. Not built yet.* GDPR Art. 15 (access), 17 (erasure) and 20 (portability). The
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
   login: KE2, gateways }`, one unit of the account's login budget, the
   attempt bound to the session, the listener and the purpose. `gateways`
   names the gateways the account owns (id, origin, name), so the page and
   the client warn before the proof is sent that their sign-in through
   Central ends with the account.
2. `DeleteAccountFinish { attempt, proof: KE3 }` → `Done`. Refused: an
   administrator (remove the role first; the last administrator cannot be
   removed, so the operator appoints another or the account stays), an
   unfinished legacy move (the bridge's move is still open; it is finished
   or invalidated first).
3. One transaction under the account's lock deletes the `accounts` row.
   Foreign keys take everything else with it: sessions (and their gateway
   tickets and leases, so every gateway sees the lease refused at its next
   check), private objects, migration archives, the gateways it owns (with
   their approvals, administrator lists and leases), its places
   on other gateways' administrator lists, its terms acceptances and roles.
   Audit events stay: they name only ids, which no longer resolve. The
   username and tag are free again for a later registration.

What Central cannot delete: what gateways keep about their players (each
gateway's own notice), backups until they rotate out (12 months), and
server logs (14 days). Nor the official gateway's copy of a moved account:
a move keeps the source data there with its password retired ([native
login](native-login.md) §"Migration state machine", step 5). Where the same
controller runs the official gateway, a deletion request covers that copy
too: until M8 removes the legacy data, the operator deletes it there on
request, and M8's cleanup should delete the retained source data of every
moved account (an open point for the owner, with the privacy text).

## The owner's decisions (28.10.2026)

1. **No fingerprint of a deleted moved account.** Central forgets its legacy
   id with the account. The move cannot be repeated anyway: when the official
   gateway links a moved account it retires the legacy password and ends the
   old sessions (Baylee `POST /auth/central/link`), so that password starts
   no new move; the person may register again, as anyone may. The owner's
   rule for sign-in stays: Central first, then the official gateway, whose
   move marks the legacy account as moved.
2. **Gateways the account owns are deleted with it,** after the warning
   above. There is no handover yet; refusing would only add a step.
3. **An export needs a password sign-in under 30 minutes,** as designed
   above.

The saved gateway list is a different thing: it lives in the encrypted
account document and goes with it. A move already writes the client's saved
gateways into the new document (Baylee `baylee_account::Account::login`).
