# Resetting a login

*28.10.2026, roadmap M1 step 2b; built on [administration](administration.md);
migration `m20261028_000013_reset`.* The owner's decision (roadmap §"Decisions", 20.10.2026): a
reset by an administrator, and later by e-mail (M1 step 4), restores the
login only; the encrypted data (account document, deck backups, chat
history) survives only with the recovery code or a still signed-in device,
and the UI says so before the reset. Every piece goes through the three
passes of [security assurance](security-assurance.md).

## Issuing a reset code

An administrator, on the website (`AdminRequest::ResetLogin { account }`, a
change: a password sign-in under 30 minutes, the audit event
`account.reset_issued`), makes a **reset code** for an enrolled account
whose move is finished, after checking out of band that the person asking
holds it. The answer `AdminAnswer::ResetCode { code, expires_at }` shows the
code once; the administrator hands it over. Central keeps only its SHA-256
digest, with its expiry, 24 hours later. A new code replaces the previous
one. Refused: an administrator's account (remove the role first), one's own,
a pending account, an unfinished legacy move (the bridge's
`LegacyInvalidate` is its reset authority), and a stale sign-in.

Issuing ends nothing: the old password keeps working until the code is used,
and a signed-in device keeps its session and the master key it holds, which
is one of the two ways the data survives. Locking an account at once is the
design's step 6. **So until step 6 ships, a reset is the answer to a
forgotten password, not to a taken-over account**: an intruder's session
and the old password stay until the code is used, and for good if it never
is. `status.md` says so.

The code is 20 bytes of operating-system randomness, written
`RESET-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX` (Crockford base32, read back
as the recovery code and beta keys are, except that the `RESET` prefix is
required), so a recovery code is never mistaken for it.

## Using it

The person, on the website or in the native client (`central-client`):

1. `Discover { username }` names the account and its epoch.
2. The person says whether they keep their data: with the recovery code, or
   on a device still signed in (the native client holds the master key), the
   vault stays at its epoch; otherwise a new vault is made at the next epoch,
   and the page says first that the old data cannot be opened any more.
3. `ResetStart { username, code, registration }`: an OPAQUE registration
   request for the new password. Central checks the code (looked up by the
   username and the code's SHA-256 digest, as every ticket is, unexpired), charges one unit of the
   account's login budget (the registration response is an evaluation of
   the account's OPRF key; only after the code is found, so a wrong code
   spends nothing of the account's), and answers `ResetChallenge { attempt, identity,
   registration, recovery }`: the registration response, the account's
   identity, and its recovery wrapper if it has one (opened only by the
   recovery code's 160 bits). The attempt lives 60 seconds and is bound to
   the account and the code. *Departure, on purpose:* elsewhere a wrapper
   leaves Central only after a verified KE3; here the reset code's 160 bits
   authorize it, and the recovery wrapper is useless without the recovery
   code's own 160. Do not copy this to a path without such a code.
4. The client finishes the registration under the chosen epoch and either
   wraps the kept master key (opened from the recovery wrapper with the
   recovery code, or held by the signed-in device), or makes a new master
   key, its first document and, for a registered account, a new recovery
   code.
5. `ResetFinish { attempt, record, wrapped_master, vault }`, `vault` being
   `keep` or `new { document, recovery }`. In one transaction, with the
   account locked and the code checked again: the code is spent; the record
   and wrapper stored; the credential revision raised; every session ended
   (with tickets and leases). For `new`: the epoch raised, the document
   replaced at revision 1, the recovery wrapper replaced (required for a
   registered account, optional for a moved one), every private object
   deleted (encrypted under a key nobody has) and the object quota emptied
   (`object_bytes`), and a registered account's `restored` cleared, so it
   takes no gateway ticket until a native client confirms the new document.
   A moved account keeps `restored`: migration 000005's
   `account_closed_enrolled` requires it once the move is closed, and that
   move's restore, which the flag guards, is long done. `Done`; the person
   signs in with the new password. Two finishes of one
   code (a signed-in device keeping its key and a browser keeping it with
   the recovery code) race for the code; the second is `Rejected`.

On the browser listener these three requests have a per-address budget (10
an hour) besides the global one; natively the global one applies.

## Accepted limits

- The code's holder spends the account's login budget with each start, so
  for the code's 24 hours it can keep the owner from signing in. Anyone can
  already do that with `LoginStart`, which needs no code, so this path adds
  nothing to it.

- Central cannot tell whether a kept wrapper holds the old master key. Whoever
  holds the code takes the login; with `keep` and a wrong key the data stays
  unreadable to them, and the owner's recovery wrapper is untouched.
- With `new`, the holder of a code destroys the account's data. A code comes
  only from an administrator, with a recent password sign-in, and its event
  is in the audit; an administrator who keeps the code for themselves is the
  insider threat the audit makes visible, not one it prevents.
- No message tells the person that a code was issued (e-mail is step 4).

## As built

- The pending reset is kept apart from the 128 pending sign-ins, at most 32,
  bound to the account, the code's digest and the listener; it has no OPAQUE
  login attempt.
- A code is read only with its `RESET` prefix, so a recovery code typed in
  its place never parses.
- The operator's daily sweep deletes an expired code's digest, so none is
  kept much beyond its 24 hours.
- A password change clears an outstanding code: the current password was
  just proven. A sign-in does not, since in a takeover the intruder has the
  password.
- `AdminAccount.reset_until` shows an outstanding code's expiry.
- `central-client`: `reset(transport, username, code, new, ResetData)` with
  `ResetData::Recovery(&RecoveryCode)` (keep) or `ResetData::New(document)`
  (a new vault, returning its new recovery code to show once), and
  `Session::redeem_reset` for a signed-in device. The SDK applies the
  epoch rule and checks the challenge's identity against discovery; after
  any reset a client holding an old identity discovers again.
- *Departures found in the code:* the design cleared `restored` for every
  new vault, which the schema forbids for a moved account (above); a new
  vault also has to empty `object_bytes`, the quota's byte counter.

## What the clients need

- M4 (website): a reset page (username, code, keep or new, the warning,
  the new password twice) and a `central-web-crypto` export for the
  recovery-code path; the admin page's "Reset login" with the warning text.
- M6 (Baylee client): the same choice; a signed-in client offers "keep my
  data" through `Session::redeem_reset`.
