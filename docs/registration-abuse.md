# Abuse protection for registration

*Design, 28.10.2026; the owner's decisions of the same day below. Built
11.11.2026 ("As built" below): the proof of work (5), the budgets (6) and
the honeypot and fill time (7). The owner's decisions of 10 and 11.11.2026
replaced the modes, keys and invitations (1 to 4): registration is open, the
closed beta's keys are gone, and an administrator's switch is the brake.*

The owner made strong captcha and spam protection for registration an
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

## What existed (28.10.2026)

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

## As built (11.11.2026)

The table's size, a challenge's lifetime and the rise changed the same day:
see "Hardened" below.

- **The proof of work** is the puzzle the website's Impressum already
  solves (`central_api::register`): for each of eight parts, the smallest
  counter whose SHA-256 over `nonce:part:counter` has `difficulty` leading
  zero bits. `RegisterChallenge` (browser listener, 20 an hour per address)
  answers a random nonce (32 bytes from the same generator as every token,
  in hex) that Central keeps in memory: at most 10,000 at once, the oldest
  making room; five minutes; one use. `Register` carries `nonce` and
  `counters`; Central spends the challenge first, whatever the outcome, and
  takes a solution only three seconds after its challenge at the soonest.
  The page solves it in `pow-worker.js` and waits out the three seconds.
- **Difficulty** 16 bits per part (about two seconds of a mid-range
  phone's JavaScript for all eight), one more bit for each doubling of the
  last hour's registrations from 60 on (a baseline of 30), three at most.
  The identity service logs nothing; the difficulty a challenge carries
  shows the rise. Tests build the service with an easy base
  (`with_registration_challenge`), which the service's own launch never
  calls.
- **Budgets per address**: `Register` 5 an hour and 10 a day; a refused one
  spends no daily unit.
- Nothing about the person is processed for the proof: the nonce is random,
  the counts are counts.
- **The brake** (owner, 11.11.2026, in place of the invite mode): one row,
  `registration.open`; `AdminRequest::SetRegistration { open }` by an
  administrator with a sign-in under 30 minutes, audited
  (`registration.closed`, `registration.opened`), on the website's account
  overview. While closed, `RegisterChallenge` answers `RegistrationClosed`
  and every `Register` is refused before the name is looked at.
- **The keys are gone**: their table, their issuing mode and an unfinished
  registration's link to one (migration 000023); the audit keeps the key ids
  its old events name. Invitations were never built.
- **The page** keeps a hidden field for form robots and does not send a form
  within three seconds of showing it.

## Hardened (11.11.2026)

The owner asked for registration's protection to be reviewed and
strengthened as the Impressum's is. First a review as an attacker who reads
the source and speaks the listener's protocol directly; then what changed.
Where the numbers below differ from "As built", these are the ones in
force.

### What the review found

**Held already**, and each now has a test that attacks it over the real
listener, the identity service and PostgreSQL
(`crates/central-client/tests/support/registration.rs`):

- *A proof used twice, or by several registrations at the same moment.*
  The challenge leaves the table under the table's lock before anything
  about it is judged (`challenge.rs`, `spend`). Of three `Register` with one
  solved challenge sent at the same instant, one makes an account and two
  are refused.
- *A proof for less work than was asked.* The difficulty is kept with the
  nonce and is the only one a solution is checked against; the client sends
  none.
- *A proof made up or worked out ahead.* The nonce is 32 random bytes that
  Central must have issued and still hold.
- *A way to an account around the proof.* The native listener refuses
  `Register` and both challenge requests before anything else. An enrollment
  needs the ticket of a reservation, and only three things make one:
  `Register` after its spent proof, the bridge with its own credential, and
  the legacy sign-in with a legacy account's password. A reset makes no
  account.
- *The name.* A refused proof says nothing about the name: it is refused
  before the name is looked at, the same for a taken and a free one. That a
  name is taken is told only after a spent proof and within five
  registrations an hour, where `Discover` tells it thirty times a minute.
- *The address and the clock.* The address is the last `X-Forwarded-For`
  hop, which the local Caddy sets to the client it saw, and the listener
  takes requests from loopback only. The three seconds and the lifetimes
  are measured on Central's clock from the moment it issued the challenge.
- *The budget tables* (4,096 addresses each). A crowd of addresses makes the
  oldest window start again and refuses nobody; it gains the crowd nothing
  a fresh address would not have had.

**Found and fixed:**

1. *A person on a slow connection was refused.* The page counted its three
   seconds from its request, Central from the challenge it issued. When the
   request took longer than about 200 ms to reach Central (a cold
   connection, a phone), the solution arrived too early, was refused, and
   the challenge was spent. The page now counts from the answer's arrival,
   which is never before Central issued it (`auth.rs`, `handover_wait`).
2. *That a crowd cannot push a living challenge out of the table was true by
   accident.* It rested on the challenge request spending the budget all
   callers share, which it did only because it carries no session, and on 32
   a second times five minutes staying under 10,000, which nothing checked.
   Both requests are now named in that budget's list, whatever token comes
   with them (`limits.rs`), and the table's size is checked when the service
   is compiled against that budget and the longest lifetime (`challenge.rs`).
3. *A program paid what a person pays.* A script with none of a browser's
   headers got the same challenge as a person's browser. See "A request that
   looks automated" below.
4. *One who holds a block of addresses was a new address with each request.*
   The budgets count IPv6 by /64, and a routed /48 is 65,536 of those, so no
   per-address budget ever applied to them. The block is now counted too,
   for the difficulty only. What remains is under "What stays".
5. *A flood stopped costing more at three bits*, sixteen seconds of a phone,
   however many registered. It now goes on to six.

**Seen and left:**

- The difficulty is fixed when a challenge is issued, so one asked for
  before a flood keeps its price. That is bounded by its five minutes and by
  twenty challenges an hour per address.
- Whoever shares an address with others (a carrier's NAT, a school) can use
  up its five registrations of the hour with refused requests, because the
  budget is spent before the proof is looked at. Per-address budgets are
  like that; charging only for a good proof is a decision for the owner.
- Outside registration, and the cheapest way to keep people out: the budget
  of 32 requests a second that all callers without a session share. One
  address uses it up with operations that have no budget of their own on
  the browser listener (`Terms`, `LoginFinish`, `EnrollmentStart`, a
  `SessionStatus` without a cookie; the last arm of `Budgets::charge` in
  `browser.rs`), and 64 addresses do with `Discover` alone. While it is used
  up, nobody signs in or registers. Not changed here: it is the sign-in's
  budget as much as registration's, and wants its own review.

### A request that looks automated

Central never refuses a request for how it looks: no person is ever locked
out (the owner's rule for the Impressum, 13.10.2026). It asks for more work,
up to four bits (16 times), counted as the Impressum's service counts:

| What Central sees | Bits |
| --- | --- |
| No `Accept-Language`, which every browser sends | 1 |
| No `Sec-Fetch-Site`, which every current browser sends (one that is not `same-origin` was refused already) | 1 |
| No `User-Agent`, or one that names a tool, a crawler or a headless browser | 2 |
| The page reports `navigator.webdriver`: a program drives the browser | 2 |
| The page reports no `navigator.languages` | 1 |
| More than 5, 10, 15 challenges from the address this hour (its budget ends at 20) | 1, 2, 3 |
| More than 20, 60, 200 challenges this hour from the block around it (an IPv6 /48, an IPv4 /24); the larger of the two counts | 1, 2, 3 |

A person's browser scores nothing. The browser listener reads the headers
and hands the identity service plain facts (`central_identity::Seen`: two
yes/no, one of three for the user agent, a number from 0 to 3), never a
header's text and never the address; of them only the challenge's
difficulty is kept, until the challenge ends. Nothing is logged. The
counts are the ones the budgets keep already, in memory for an hour, and
one more table for the blocks with the same bound of 4,096.

The page's report comes with a new request, `RegisterChallengeFor {
signals }` (bit 1 `webdriver`, bit 2 no languages, as the Impressum's page
sends them; other bits are ignored, so a later page may say more). Its
answer is the challenge as before. `RegisterChallenge` is unchanged on the
wire and is weighed by the same headers and counts, so asking the old way
saves nothing. Both spend the one budget of 20 an hour. A page that meets a
Central from before this asks the old way.

These signs cost a careless robot time and a careful one nothing: the
headers and the report are whatever the sender says. The counts are not.

### A flood

One bit for each doubling of the last hour's registrations: from 60, 120,
240, 480, 960 and 1,920 an hour, six bits at most (it was three). The hour
takes it back as before.

The flood's bits and the request's own add up, and stop at six over the
base: 22 bits a part, 64 times the base work, about two minutes of the
mid-range phone that needs two seconds for the base. Adding all ten would be
half an hour of that phone, which is a lock-out by another name.
`central_api::register::MAX_DIFFICULTY` is that 22, so the page takes no
challenge above what Central asks, and Central's own constants are checked
against it when it is compiled.

Measured on 11.11.2026 with the page's own Worker in Chromium on a laptop
(Apple M1 Max), three runs each: 16 bits under the three seconds the page
waits anyway; 20 bits 15, 15 and 40 seconds; 22 bits 26, 63 and 98 seconds.
That is about 0.55 million hashes a second and one second for the base, so
the design's phone is one half as fast as this laptop; a slower one needs
as much longer, and the lifetimes below are there for it. The spread is the
puzzle's own: eight parts, each found by chance.

A challenge lives five minutes up to four bits over the base, ten minutes
with five and twenty with six: about nine times what that phone needs on
average, so a slow device still hands its solution in. The table holds
40,000 challenges (it was 10,000), more than the shared budget lets all
callers together ask for in twenty minutes, so no challenge makes room for
another before its own time is over. Full, it is a few megabytes.

The page says what it is doing once the proof takes longer than three
seconds: a line with the share done and a bar, announced politely
(`register-solving`).

### What stays

- SHA-256 is cheap for a program that is not a page. On the laptop above,
  native code hashes about thirteen times as fast as the page's JavaScript
  (7 million a second on one core by `openssl speed`, against 0.55 million),
  so the hardest challenge is five seconds of one core for a script and a
  minute for a person; for a graphics card it is a blink. The proof slows
  the simplest scripts and costs a person little at the base; it does not
  stop someone with a card and a block of addresses. Against them stand the budgets, the count of the block, the
  rising price, and the administrators' switch.
- Many addresses in many blocks, with patience, still create accounts.
- The count of a block is shared by everyone in it. Addresses that ask at
  their full budgets make their neighbours' challenges harder for the hour:
  two in an IPv4 /24 by one bit, four by two, eleven by three (eight times
  the work); in an IPv6 /48 that a provider hands out in smaller pieces, one
  customer does. It is never a refusal, and it ends at the four bits a
  request's look can cost.
- The counts are in memory and start again with the service.
