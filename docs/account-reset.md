# Resetting a login

*28.10.2026, roadmap M1 step 2b; built on [administration](administration.md);
migration `m20261028_000013_reset`.* The owner's decision (roadmap §"Decisions", 20.10.2026): a
reset by an administrator, and since 12.11.2026 by a code mailed to the
account's confirmed address (M1 step 4, [e-mail](email.md)), restores the
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
is one of the two ways the data survives. Since 12.11.2026 the account's
confirmed address, if it has one, is told afterwards that a code exists,
not the code ([e-mail](email.md#notices-that-the-login-changed)); changing
the password ends the code, and the notice says so. Locking an account at once is the
design's step 6. **So until step 6 ships, a reset is the answer to a
forgotten password, not to a taken-over account**: an intruder's session
and the old password stay until the code is used, and for good if it never
is. `status.md` says so.

The code is 20 bytes of operating-system randomness, written
`RESET-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX-XXXX` (Crockford base32, read back
as the recovery code and beta keys are, except that the `RESET` prefix is
required), so a recovery code is never mistaken for it.

### A code by mail

*12.11.2026, migration 000026; the request, its numbers and its limits are
in [e-mail](email.md).* An account that confirmed an e-mail address can have
a reset code mailed to it. `ResetMail { username, lang }` needs no session
and is answered `Done` for every name; the code is made afterwards, off the
request, so the answer says nothing about the account.

- It is the same code: 20 bytes, the same form, made by the statement that
  makes an administrator's, with the same refusals (an administrator's
  account, a pending account, an unfinished legacy move). It is used as
  described below. Nothing in `ResetStart` and `ResetFinish` knows where a
  code came from.
- It lasts 1 hour, not 24: a mailbox is a weaker place to leave a code than
  a person's hand.
- It never replaces an administrator's code that has not expired. An
  administrator's new code replaces a mailed one. A new mailed code
  replaces the last mailed one.
- At most one is made for an account in 10 minutes and 3 in 24 hours.
- It ends when the mailer did not take its mail, and when the address it
  went to is replaced or removed. An administrator's code does not end
  with the address.
- No administrator acts, so the audit has no event for it.

A Central without a mailer makes no such code, and codes come from an
administrator only.

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
   signs in with the new password. The account's confirmed address, if it
   has one, is told afterwards whether the data was kept
   ([e-mail](email.md#notices-that-the-login-changed)); the address is read
   in this transaction. Two finishes of one
   code (a signed-in device keeping its key and a browser keeping it with
   the recovery code) race for the code; the second is `Rejected`.

On the browser listener these three requests have a per-address budget (10
an hour) besides the global one; natively `ResetStart` has the same since
11.11.2026, for an address the proxy forwards ([native login](native-login.md)
§"Guessing a password online").

## Accepted limits

- The code's holder spends one of the account's eight evaluations a minute
  with each start, and since 11.11.2026 nothing of its hour or day
  ([native login](native-login.md) §"Guessing a password online"): guessers
  who spent those do not keep a holder with a code from the reset. With
  eight starts a minute, for the code's 24 hours, the holder of a code can
  still keep the account's sign-ins from places that are not its own at
  `Busy`. Anyone can do as much with `LoginStart`, which needs no code, so
  this path adds nothing to it.
- A reset's finish ends the account's own places (the addresses it proved
  the old password from) and makes the one the finish came from its own:
  whoever knew the old password guesses at the new one as a stranger does,
  and the code's user signs in there next whatever others spent of the
  account's count.

- Central cannot tell whether a kept wrapper holds the old master key. Whoever
  holds the code takes the login; with `keep` and a wrong key the data stays
  unreadable to them, and the owner's recovery wrapper is untouched.
- With `new`, the holder of a code destroys the account's data. A code comes
  from an administrator, with a recent password sign-in, and its event
  is in the audit; an administrator who keeps the code for themselves is the
  insider threat the audit makes visible, not one it prevents.
- Or it comes by mail, to the address the account confirmed. Whoever reads
  that mailbox can ask for a code and use it: they take the login, and with
  `new` they destroy the account's data. That is what an address is for, and
  the reason it is optional. A mailed code has no audit event.
- Only an account with a confirmed address is told that an administrator
  issued a code or that a reset was finished (since 12.11.2026,
  [e-mail](email.md#notices-that-the-login-changed)); an account without
  one is told nothing. A notice is tried once and can be lost, and it goes
  to the mailbox the account has, which after a reset by mail is the one
  the code went to. A mailed code is its own message.

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
- `AdminAccount.reset_until` shows an outstanding code's expiry, a mailed
  code's too. It does not say whose code it is.
- `central-client`: `reset(transport, username, code, new, ResetData)` with
  `ResetData::Recovery(&RecoveryCode)` (keep) or `ResetData::New(document)`
  (a new vault, returning its new recovery code to show once), and
  `Session::redeem_reset` for a signed-in device. The SDK applies the
  epoch rule and checks the challenge's identity against discovery; after
  any reset a client holding an old identity discovers again.
- *Departures found in the code:* the design cleared `restored` for every
  new vault, which the schema forbids for a moved account (above); a new
  vault also has to empty `object_bytes`, the quota's byte counter.

## The website's page

*Built 12.11.2026 (`crates/central-web`, `islands/reset.rs`; the record is in
[security assurance](security-assurance.md), "The website redeems a reset
code and changes a password").* `/{lang}/reset`, linked from the sign-in
form and the FAQ, not indexed and in no menu, takes the sign-in name, the
reset code (an administrator's, or one Central mailed), the choice and the
new password twice. The crypto is
`central-web-crypto`'s `reset_start`, `reset_keep` and `reset_new`, which do
what `central-client`'s `reset` does with the same `central-crypto`
functions; no message changed.

- **Two of the three ways.** The page keeps the data with the recovery code
  or starts a new vault. It has no "still signed in" way: a browser holds no
  master key after a sign-in ([web accounts](web-accounts.md)).
- **The warning comes first.** A new vault is asked for only after the
  person confirmed, on the form, what is lost; nothing is sent before.
- **One answer for a name and its code.** Central answers a name nobody has
  as an account that has not moved (`Discover`), and a wrong, expired, used
  or replaced code with `Rejected` (`ResetStart`). The page says one
  sentence for all of them, and for a name that is none, so it tells a
  guesser less than Central's answers do, never more.
- **A wrong recovery code spends nothing.** The Worker reads the recovery
  code before `ResetStart` and opens the wrapper with it before
  `ResetFinish`; a code that is none or opens nothing stops there, the
  reset code still works, and the page says so. An account without a
  recovery wrapper reads the same. For a well-formed code that opens
  nothing, `ResetStart` has been sent by then: it counts one of the
  address's ten an hour and one of the account's eight evaluations a
  minute.
- **A lost answer is neither.** When the finish's answer does not arrive,
  the page says that it cannot tell, and to try the new password. A new
  vault's recovery code is shown all the same, with what it is worth in
  either case, since it cannot be shown again.
- **Secrets.** The password, the recovery code and a new vault's recovery
  code exist in the Worker and in the form's fields, which carry no name;
  the new code goes to the page once, to be shown, copied or saved as a
  file. After a reset Central confirmed, the page forgets this browser's
  session if Central no longer knows it.
- **A code by mail** *(12.11.2026; the website's half of an account's
  e-mail address, `docs/email.md`)*. Before the form stands a block for
  someone who has no code, "No reset code?": a sign-in name and "Send a
  code by e-mail" (`ResetMail`). Central answers `Done` whatever the name,
  and the page has one sentence for it: if the account has a confirmed
  address, a code is on its way, valid for one hour. It promises no mail.
  `Busy` is "try again later"; `Rejected`, which Central gives for every
  name where it sends no mail, is "ask an administrator". Only what is a
  sign-in name by its form is sent, so that this `Rejected` is never the
  answer to an e-mail address typed in the name's place. As a
  registration's form, one sent within three seconds of the page is not
  sent; here the page says so. The name is carried into the form, where a
  mailed code is redeemed as an administrator's is. The form's lead and
  the code's hint name both origins and both lifetimes (one hour by mail,
  24 hours from an administrator).

## What the clients need

- Website: the page above. Open: a way to ask for a code by mail on that
  page, and the new site's administration with "Reset
  login" and its warning text. Until then the new site has no page that
  issues a code; the old build's admin page (`web/`) does, where that
  build is still served under `/admin`.
- M6 (Baylee client): the same choice; a signed-in client offers "keep my
  data" through `Session::redeem_reset`.
