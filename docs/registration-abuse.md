# Abuse protection for registration

*Design, 28.10.2026; the owner's decisions of the same day below. Built
11.11.2026 ("As built" below): the proof of work (5), the budgets (6) and
the honeypot and fill time (7). The owner's decisions of 10 and 11.11.2026
replaced the modes, keys and invitations (1 to 4): registration is open, the
closed beta's keys are gone, and an administrator's switch is the brake. The
owner's decision of 11.11.2026 put a proof that costs memory in the SHA-256
proof's place ("A proof that costs memory", the last section): its function,
its numbers and Central's part in it are the ones in force.*

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
force; the hash, the base and the ceiling changed once more the same day
("A proof that costs memory").

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

Checked against a real Central behind the proxy's stand-in (11.11.2026,
`crates/central-web/browser/e2e-real`): Chromium's own requests score nothing, and Chromium driven
by a test tool reports `webdriver` and is asked for 18 bits, so the
project's browser tests pay four times the base and no more.

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
50,000 challenges (it was 10,000, then 40,000), more than the shared budget lets all
callers together ask for in twenty minutes, so no challenge makes room for
another before its own time is over. Full, it is a few megabytes.

The page says what it is doing once the proof takes longer than three
seconds: a line with the share done and a bar, announced politely
(`register-solving`).

### What stays

- *(The reason for "A proof that costs memory", which replaced this proof
  the same day.)* SHA-256 is cheap for a program that is not a page. On the laptop above,
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

## A proof that costs memory (11.11.2026)

The first point of "What stays" above was the reason: one core solved the
SHA-256 proof thirteen times as fast for a script as for the page. The
owner was asked on 11.11.2026, in these words: "Rätsel mit Speicherkosten:
Der Fork empfiehlt, das Registrierungsrätsel auf Argon2id umzustellen, weil
ein Skript das jetzige etwa 13-mal schneller löst als die Seite im Browser.
Das ist noch nicht gebaut.", and answered: "Beide Entscheidungen meiner
Seits beantworte ich mit: ja". This is what was built for it. As under
"Hardened": the flood's rise, the price of a request's look, the budgets,
the lifetimes, the three seconds and the table. New: the function, the base
and the ceiling, the answer on the wire, and what Central does with
`Register`.

### The function

One Rust function, `central_crypto::proof`, compiled natively into Central
and to WebAssembly into the page's crypto Worker (`central-web-crypto`:
`work_start`, `Work.search`). Nothing in JavaScript computes it. For a part
`p` and a counter `c` it is Argon2id (RFC 9106, version 0x13, one lane, 32
bytes of output) through the safe interface of the `argon2` release that
stretches the passwords here already, with nothing built around it:

- salt: the label `baylee-central/registration-proof/argon2id/v1`;
- password: the nonce's 64 hexadecimal digits as Central sent them, the
  part's index (one byte) and the counter (eight bytes, big-endian), 73
  bytes;
- memory and passes: the challenge's.

A counter solves its part when the output begins with `difficulty` zero
bits. The page finds the smallest; Central takes any that holds. The
[crypto protocol](crypto-protocol.md) §"Registration's proof of work" has
the owner's acceptance, why this is the vetted primitive and nothing of our
own, and the pinned vector that holds the two builds together: one nonce,
price and set of counters is checked natively, through the function the
Worker calls, and in Chromium with the committed module
(`crates/central-web/browser/e2e-site/account.spec.ts`), and OpenSSL's Argon2id gives the vector's
bytes too.

### The numbers in force

| | |
| --- | --- |
| Memory for one evaluation | 16 MiB (16,384 KiB) |
| Passes | 1 |
| Parts | 8 |
| Difficulty while registrations are few | 4 bits a part: 128 evaluations on average |
| Difficulty at the most (flood and look together, six bits, as before) | 10 bits a part: 8,192 evaluations on average |
| Lifetime | 5 minutes up to 8 bits, 10 minutes at 9, 20 minutes at 10 |
| Soonest | 3 seconds after the challenge |
| Table | 50,000 challenges |

Central sets the price. Memory, passes, parts and difficulty are kept with
the nonce and are the only ones a solution is checked against. They travel
in the answer; `Register` carries nonce and counters only, as before. The
page refuses a price above compile-time ceilings before it works on it
(`central_api::register`): each field (8 KiB to 64 MiB, 4 passes, 16 parts,
16 bits) and the whole, `WORK_MAX_COST`, which is memory × passes × parts ×
2^difficulty at exactly the most Central asks. So the page never works on
what it could not hand in or what a phone could not hold. Asserts at
compile time hold Central's constants to that ceiling, the field ceilings
of `central-api` and `central-crypto` equal, and the table and the
lifetimes to the shared budget as before.

Why these:

- **16 MiB.** A Worker of the page takes 64 MiB for the password right
  afterwards, on every device that can register at all, so a quarter of
  that is known to fit. The proof runs in a Worker of its own that is
  ended before the password's is started: a Worker's WebAssembly memory
  does not shrink, and one Worker for both would have held about 80 MiB.
  So the page never holds more than it did before this change. Central
  holds four checks at once at the most:
  64 MiB. More would cost Central more for each check and make the page's
  steps coarser; less buys less against hardware that runs many
  evaluations side by side.
- **One pass.** A second doubles everyone's time, Central's too, and
  changes nothing about who is faster.
- **Eight parts.** The time to solve is the sum of eight chances and
  spreads far less than one chance would, and Central is done with a wrong
  answer after one evaluation.
- **Four bits** cost the laptop below about a second, what sixteen bits of
  SHA-256 cost it. **Ten bits** cost it about a minute, what 22 bits of
  SHA-256 cost it.

The numbers are a price and protect no password, so what RFC 9106
recommends for passwords is not their measure.

### On the wire

`RegisterChallengeFor { signals }` is answered with an answer of its own,
`RegisterWork { nonce, parts, difficulty, memory, passes, expires_at }`
(`{"result":"register_work", …}`): the older answer cannot gain a field,
and the request had not been deployed. `Register` is unchanged.
`RegisterChallenge`, the request for the SHA-256 proof, is answered with
`RegistrationClosed` whether registration is open or not, and no nonce
comes of it. Wire tests hold every shape (`central-api`, `wire.rs`).

### No cheap way left

A Central of this build hands no SHA-256 challenge to any request and takes
no SHA-256 solution: every `Register` is judged by the one function above
at the price kept with its nonce. Over the real listener and PostgreSQL
(`crates/central-client/tests/support/registration.rs`, `older`): the old
request gets `RegistrationClosed`, with a person's headers and with a
script's; a SHA-256 solution is `Rejected`, for a made-up nonce and for one
this Central issued; no account of that name exists afterwards. Counters
sent without any work hold with a chance of one in 2^32 at the base, within
five tries an hour.

What each side sees across the change:

- **A register page from before** (it asks `RegisterChallenge`) at this
  Central: "Registration is paused right now. Try again later."
  (`register-closed`) until the page is loaded again. A tab left open
  cannot register until then. That answer was chosen because the page has
  words for it; a refusal it would have shown as a registration that
  failed.
- **This page at a Central from before**: `RegisterChallengeFor` is refused
  there, or answered with the older challenge; either way the page goes on
  the older way and solves the SHA-256 proof in `pow-worker.js`, which
  stays anyway for the Impressum's puzzle (the game repository's service,
  SHA-256, not ours to change). So the site goes out first and Central
  after it, without a gap ([operations](operations.md#registration)).
- A page built from the commits in between (it asks `RegisterChallengeFor`
  and knows only the older answer) cannot read `register_work` and shows
  its general error. None was deployed.

### What Central does with `Register`

1. The listener, as before: the address's budget (5 an hour, 10 a day) and
   the budgets all callers share.
2. A worker of the service's four, the ones sign-in uses. Without one the
   answer is `Busy` and the challenge stays in the table: the solution is
   not lost, and the page sends it once more a second and a half later.
3. The challenge leaves the table under the table's lock, one use. An
   unknown, expired or too early one is refused here, before anything is
   computed.
4. The counters are checked off the executor (`spawn_blocking`), part by
   part, ending at the first that does not hold. Then the name.

The order is one function without the store (`challenge::prove`), so a unit
test shows it: with every worker taken the answer is `Busy`, and the same
solution is taken afterwards.

**What this bounds.** At once: four checks, 64 MiB. One address: five
checks an hour and ten a day, one evaluation for a wrong answer and eight
for a right one, so 80 evaluations a day at the most, about half a second of
one core of the laptop below; and each right answer cost its sender 128 to
8,192 evaluations. Everybody together: the 32 requests a second that all
callers without a session share and, since 12.11.2026, the 8 kept beside
them for addresses that asked little. Were every one of the 40 a wrong
answer to a living challenge, that is 40 evaluations a second, a quarter
of one core of that laptop (it was 32 and a fifth before that date); and
each needs a challenge of its own (20 an hour per
address) and a unit of its address's five an hour. Central computes only
for an answer that names a living challenge after its three seconds.

### Measured (11.11.2026, Apple M1 Max)

One evaluation at 16 MiB and one pass:

| Where | One evaluation |
| --- | --- |
| Native: the Rust crate as Central runs it, release build, one core | 6.0 ms (median of 400; 5.6 the least, 6.1 the mean) |
| The page: the committed WebAssembly module in its Worker, Chromium | 7.9 ms (median of 9 runs of 40 evaluations; the runs 7.8 to 8.1) |
| The same Worker with a quarter of the processor's time | 34 ms (5 runs of 24; 32 to 35) |
| OpenSSL 4.1.0's Argon2id (portable C), native | 7.9 ms (200 in 1.58 s) |

**The page is 1.3 times as slow as native code on the same core. For
SHA-256 it was thirteen times.** That ratio is the point of the change.
The two stay as close at other sizes (native and page, in ms: 8 MiB 2.8 and 3.9, 19 MiB 7.2 and 9.5, 32 MiB 12.9 and 16.7, 64 MiB 27.6 and 35.4). Central's whole check, with taking and giving back its 16 MiB:
48 ms for a right answer (eight evaluations), 6.3 ms for a wrong one, 0.3 ms of it the memory (medians of 40).

A whole challenge in the Worker, as the page solves it (each part in turns
of eight counters):

| Bits a part | Evaluations expected | Evaluations taken: mean (least to most) | Seconds: median (least to most) | Runs |
| --- | --- | --- | --- | --- |
| 4 | 128 | 122 (59 to 230) | 0.88 (0.48 to 1.84) | 25 |
| 6 | 512 | 531 (301 to 919) | 4.11 (2.42 to 7.38) | 9 |
| 8 | 2,048 | 2,095 (1,437 to 2,692) | 16.95 (11.70 to 21.86) | 5 |
| 10 | 8,192 | 6,766 (1,664 to 10,970) | 45.42 (13.65 to 91.06) | 5 |
| 4, with a quarter of the time | 128 | 130 (62 to 198) | 4.66 (2.22 to 6.65) | 9 |
| 10, with a quarter of the time | 8,192 | 8,770 (3,868 to 11,499) | 373.88 (132.54 to 388.30) | 3 |

Through the page itself (the form, the page's code, the Worker; a stand-in
for Central at Central's price), from the challenge's arrival to the
`Register` that carries its solution, which is never sooner than three
seconds:

| Bits a part | Seconds: median (least to most) | Runs |
| --- | --- | --- |
| 4 | 3.21 (all three: the three seconds) | 3 |
| 6 | 5.66 (3.46 to 6.52) | 5 |
| 8 | 11.33 (9.93 to 22.39) | 3 |
| 10 | 60.91 and 73.32 | 2 |
| 4, with a quarter of the time | 4.40 (3.22 to 4.65) | 3 |
| 10, with a quarter of the time | 466.96 | 1 |

The spread is the puzzle's own: eight parts, each found by chance, so the
evaluations taken scatter around their mean by about a third of it (one
over the root of eight), and one challenge in a thousand takes more than
two and a half times the mean. The runs above fit that but for one: a ten-bit challenge solved in 1,664 evaluations, a fifth of the mean, which this reckoning expects once in about 3,400 challenges. So the module and the native function were set side by side past the output's first byte: at ten bits and 64 KiB, for 24 fixed nonces, the module in Chromium and the native build found the same 192 counters, and 120 native challenges at ten bits took 8,045 evaluations on average (2,299 to 18,948). It was luck. A ten-bit challenge lives twenty
minutes, eighteen times what the laptop needs on average; the runs with a quarter of the time used between a ninth and two fifths of it.

**How the quarter was made, and where this leaves the brief.** The brief
was Chromium's own throttle (`Emulation.setCPUThrottlingRate`, rate 4) as
the stand-in for a mid-range phone. It does not reach a Worker: sent to the
page it left the Worker's evaluation as it was (a factor of 1.04,
measured), and a Worker's own session refuses it ("Operation is only
supported for pages, not workers"). So the browser's renderer was stopped
and continued in turns from outside (20 ms running, 60 ms stopped), which
is what that throttle does to a thread; one evaluation then took 4.16
times as long. It stands for a slower processor and not for a phone, whose
memory is slower too. No phone was measured.

### Tests ask little

`IdentityService::with_registration_challenge(base, memory, minimum)` only
lowers: each value is capped at Central's own, and neither a setting nor a
request reaches it (the launch never calls it; `the_knob_only_lowers`). The
PostgreSQL fixture runs at one bit and 64 KiB, the unit tests at 64 KiB;
the site's suite checks the pinned vector at 64 KiB in the real module and
plays Central for the rest. `crates/central-web/browser/e2e-real` meets the real service at the
real price: against a throwaway Central of this tree (release build,
11.11.2026) Chromium under Playwright was asked 6 bits, two over the base
for the `navigator.webdriver` it reports, at 16 MiB, one pass and eight
parts, with five minutes to live. Six timed registrations in three runs,
each run a fresh Central: the solution was handed in 3.2 seconds after the
challenge four times (the three seconds' floor) and after 4.4 and 6.4
seconds (six bits take four seconds on average); Central answered
`Register` in 67 to 95 ms, its eight evaluations and the store's work
together; the recovery code was on the page 3.9 to 6.9 seconds after the
click. `site-chat.spec.ts` passed in each run with two more registrations
(29 seconds). So the project's driven registrations pay four times the
base, as before, and do not crawl. The first run was before the proof got
a Worker of its own, the other two after.

### What stays

- **A script is still faster than a person**, by the 1.3 above on one
  core, and by its cores: the page solves in one Worker, a script on every
  core it has. Solving the parts in several Workers, at 16 MiB each, would
  close that; not built. Code with vector instructions and graphics cards
  were not measured. What the function is made for is that every
  evaluation running at the same time needs its own 16 MiB, so such
  hardware is bounded by its memory and that memory's speed and not by how
  many hashes it could do side by side.
- **What a flood costs.** At the base a registration is about 0.8 seconds of
  one core for a script, and five an hour per address. From 1,920
  registrations an hour on each costs about 50 seconds of one core: about 26 cores for as
  long as it lasts, and 384 addresses. That is a price, and someone with
  machines and addresses pays it. Against them stand the budgets, the
  count of the block and the administrators' switch, as before.
- **Central computes now.** A wrong answer to a living challenge costs it
  one evaluation, and nothing that sender did beyond asking for the
  challenge. It is bounded as above, by the address's five an hour and by
  the budget all share, and not measured on the machine that serves
  `baylee.app`.
- **The workers are sign-in's too.** While four checks run, a sign-in or a
  registration that needs a worker is told `Busy`. Within the budgets that
  takes more right answers at once than the price makes likely; it was not
  driven in a test over the listener.
- **A second `Busy`** shows the page's "too many attempts" line, and each
  try spent a unit of the address's five an hour.
- **A device far slower than the measured quarter** can run out of a
  ten-bit challenge's twenty minutes in the unlucky cases, and is then
  refused like a wrong answer. Ten bits are asked only in a flood or of a
  request that looks automated.
- **A tab of a page from before** cannot register until it is reloaded.
- The points of "What stays" above that are not about SHA-256 stand: many
  addresses in many blocks, the block's shared count, the counts in memory.
