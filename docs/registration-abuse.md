# Abuse protection for registration

*Design, 28.10.2026; the owner's decisions of the same day below. Not built
yet.* The owner made strong captcha and spam protection for registration an
open-beta blocker (28.10.2026): privacy-friendly, no Google reCAPTCHA, the
design first. A new dependency or a third-party service needs the owner's
approval; this design needs neither.

## The owner's decisions (28.10.2026)

1. **Two registration modes, switched by an administrator at any time:**
   `open` (no code) and `invite` (an invitation from a member, or an
   operator's key). The open beta opens registration without a key; the
   invite mode stays available, for instance during an attack.
2. **One code, one account.** Every invitation and every newly issued key
   admits exactly one registration.
3. **No ceiling on registrations in all**, and no limit on how many
   invitations a member makes; Central records who invited whom, so an
   administrator can trace and end a chain of spam accounts.
4. **No third-party service,** as strong as a self-hosted design allows: a
   proof of work that grows harder while registrations flood in.

## What exists

- A beta key per registration; each key has a number of uses, an expiry and
  can be revoked (`docs/web-accounts.md`).
- Per-address budgets on the browser listener, from the proxy's last
  `X-Forwarded-For` hop, IPv6 counted by its /64: `Register` 5 an hour,
  `Discover` and `LoginStart` 30 a minute each, `ResetStart` 10 an hour; in
  memory, at most 4096 addresses, the oldest window making room.
- Global budgets (32 unauthenticated requests a second) and the per-account
  sign-in budget (eight OPRF evaluations a minute).
- Registration is the browser listener's only; the native client cannot
  register yet (M6).

## Design

1. **Registration mode.** A one-row table holds `open` or `invite`, with the
   time and the administrator of the last change. The migration starts it at
   `invite`, which is today's behaviour (a key is required); the owner's
   administrator switches to `open` when the open beta starts.
   `AdminRequest::SetRegistration { mode }` is a change (a password sign-in
   under 30 minutes) with the audit event `registration.mode_set`; the
   admin page shows the mode. In `open`, `Register` takes no code; a code
   sent anyway is checked and spent, so an invited person keeps the link to
   the inviter.
2. **Invitations.** A member (a restored account that accepted Central's
   terms) makes one with `CreateInvitation` on either listener and gets
   `Invitation { code, expires_at }`, shown once: 20 random bytes printed as
   `INVITE-XXXX-…` (Crockford base32, the prefix required, read like the
   reset code), valid seven days, one registration. Central keeps the
   SHA-256 digest in its own table, apart from the operator's beta keys, so
   the runtime gains `INSERT` and `DELETE` on invitations and nothing new on
   `beta_keys`. `ListInvitations` and `RevokeInvitation` show and end one's
   own open ones. No limit per member, as decided; only a storage bound (a
   thousand open invitations per account) keeps one account from filling
   the table. The account an invitation admits records its inviter
   (`accounts.invited_by`, cleared when the inviter's account is deleted);
   the inviter's deletion also ends their open invitations.
3. **Keys.** `--issue-beta-keys-stdin` issues single-use keys from now on.
   Keys already issued keep their remaining uses until they expire or the
   operator revokes them.
4. **For administrators.** `AdminAccount` names the inviter;
   `AdminRequest::Invited { account }` lists the accounts one account
   invited; `AdminRequest::RevokeInvitations { account }` ends its open
   ones (audited `account.invitations_revoked`). Removing the accounts of a
   chain is the administration API's deletion and locks (M2, the design's
   step 6); until then: switch to `invite` and revoke.
5. **A proof of work, issued and checked by Central** (ALTCHA-style, no
   service, no new dependency). `RegisterChallenge` (browser listener, its
   own per-address budget) answers `{ challenge, difficulty, expires_at }`:
   32 random bytes Central keeps in memory (bounded, one use, five
   minutes). The page finds a counter whose SHA-256 over challenge and
   counter has `difficulty` leading zero bits, in a Worker with the
   browser's own WebCrypto, and sends `challenge` and `counter` with
   `Register`. Central checks it before any code, spends it either way, and
   refuses a solution found less than three seconds after the challenge
   (the minimum fill time). Nothing about the person is processed: the
   challenge is random. The native client (M6) solves it in Rust.
   - *Growing harder:* the difficulty starts at about two seconds on a
     mid-range phone. For each doubling of the last hour's registrations
     above a baseline (30 an hour, to be tuned), it rises by one bit, which
     doubles the work, up to three bits (about 16 seconds); it falls again
     as the hour moves on. Each rise is logged with counts only, never an
     address. This replaces a ceiling: registration never closes, but a
     flood gets expensive.
   - *Considered and set aside:* Friendly Captcha (an EU service, but a
     processor needing an agreement, and a third-party script on the page);
     the ALTCHA widget (a dependency for what is a few dozen lines here);
     reCAPTCHA and hCaptcha (trackers, excluded by the owner).
6. **Per address:** today's 5 registrations an hour, plus 10 a day.
7. **A honeypot field** in the page's form (a filled one is never sent), and
   the page refusing to send a form filled in under three seconds. This only
   stops generic form bots; the proof of work is what binds a bot that
   speaks the API directly.
8. **Later, with e-mail (step 4):** e-mail is optional at registration, so
   disposable-address signals matter only for verification and 2FA by
   e-mail; a list would be a dependency and is left for then.

No self-hosted measure stops a determined attacker with enough machines; it
makes every account cost them time. The invite mode is the brake that does.

## Privacy

The proof of work processes no personal data. The per-address counters stay
in memory as today (the privacy notice already names them). New for the
notice (a draft for the owner's approval): who invited an account, kept
while both accounts exist, for tracing abuse; an invitation's digest, kept
until it is used, revoked or expires.

## Order

Built after M1 step 3 (deletion and export), before the open beta; the
native client's part (solving the proof, entering an invitation) is M6.
