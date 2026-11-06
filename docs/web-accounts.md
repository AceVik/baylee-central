# Accounts on the website: registration and sign-in

Design for the first account features on `baylee.app` (roadmap M1/M3/M4, cut to
what the owner chose for Baylee 0.1.0-beta.8a, 21.10.2026). This file is the
contract between the pieces built in parallel: the server, the browser crypto,
the web pages and Baylee's gateway. A change to a shape here changes this file
first. Every piece goes through the three passes of
[security assurance](security-assurance.md).

## Owner decisions (21.10.2026)

- Registration and sign-in on `baylee.app` are what Baylee 0.1.0-beta.8a waits
  for. Password change, reset, e-mail, 2FA, profile and export come later (M1).
- Registration hands out a **recovery code**, shown once.
- Browser crypto is a WebAssembly crate built from `central-crypto`.
  `wasm-bindgen` 0.2.130 as a direct dependency, getrandom 0.4.3 `wasm_js` and
  0.2.17 `js`, and `'wasm-unsafe-eval'` in the site's CSP are approved.
  `#![forbid(unsafe_code)]` holds on the crate: rustc does not count the
  `unsafe` in `#[wasm_bindgen]`'s expansion against it (checked with a probe
  crate, and a first-party `unsafe` block in it is refused).
- The browser session is a strictly necessary `__Host-` cookie; the privacy
  notice says so before the pages go live.
- The official gateway (Sanctuary) is bound to Central once registration is
  live: its players move to Central at their first sign-in, it takes no guests
  and no sign-up in the client, and new players register on the website.

## Flows

**Registration** (website, no session yet):

1. `RegisterChallengeFor { signals }` → `RegisterWork { nonce, parts,
   difficulty, memory, passes, expires_at }`, then
   `Register { username, display_name, nonce, counters }` →
   `Enrollment { identity, ticket }` while registration is open. The proof of work is checked and spent first
   ([registration abuse](registration-abuse.md)); the username must be
   free under the versioned name policy; the display name, which the page
   always sends, keeps the name rule and is no reserved name (§"Display name
   and tag"), and a tag is drawn for it. The account row is created with no
   legacy source.
2. The browser runs OPAQUE registration with the context
   `(issuer, identity.account, identity.epoch)`; `EnrollmentStart { ticket,
   message }` → `EnrollmentChallenge`.
3. The browser makes a random master key, wraps it under the OPAQUE export key
   (the 73-byte wrapper of `vault.rs`), makes a recovery code and wraps the same
   master key under it (below), and seals the initial account document (below)
   as object 1. `EnrollmentFinish { ticket, record, wrapped_master, document,
   recovery }` → `Done`. The commit is atomic and consumes the key's use.
4. The page shows the recovery code once (the ways to keep it below, §"Keeping
   the recovery code", and a confirmation that it was kept) and signs in
   (below) to accept Central's terms; the sign-in's `AccountSummary` gives it
   the tag, and the code's step then shows the whole handle, `Name#3fa2`.
   Where Central mails, the step also takes an e-mail address for the
   account; it is named after the code is kept and the terms are accepted.

**Sign-in** (website): `Discover` → `LoginStart` → `LoginFinish`, as native.
On the browser surface `LoginFinish` answers `BrowserSession { identity,
username, wrapped_master, expires_at }` and sets the cookie; the token never
reaches the page. `SessionStatus` there answers `BrowserStatus { identity,
username, expires_at }`; `username` is the account's canonical (lowercase)
name. The native `SessionStatus` answer is unchanged: clients built before
these refuse unknown fields. The page unwraps the master key only to check the password worked and
keeps nothing. Then `TermsStatus`, and `AcceptTerms` where Central's terms in
force are not accepted. `Logout` ends the session and clears the cookie.

**Playing.** The native client signs in at Central for a gateway whose
`/auth/config` says `central` (Baylee `docs/central-login.md`): discover, login,
read and validate the document, `ConfirmRestore`, then a gateway token. The
official gateway, in bridge mode, admits an identity with no legacy account by
making it a local projection, as a registered gateway does (Baylee change G1).

### The initial account document

The plaintext sealed as object 1 is exactly

```json
{"version":1,"gateways":["https://baylee.acevik.de"],"preferences":{},"legacy":null,"chat":{"devices":[],"installations":[]}}
```

Baylee's `baylee_account::data::AccountData::read` accepts it (`preferences`
defaults field by field), so a freshly registered account lists the official
gateway after its first native sign-in. Baylee holds a test that parses these
exact bytes.

### The recovery code

20 random bytes, shown as 32 Crockford base32 characters in eight groups of four
(`XXXX-XXXX-…`), read back case-insensitively with `O`→`0`, `I`/`L`→`1`. The
recovery wrapping key is HKDF-SHA512 over the 20 bytes with salt
`baylee-central/recovery/v1` and the context's AAD as info (the password
wrapper's 86 bytes; [crypto protocol](crypto-protocol.md) §"Recovery code"
spells out every byte); the wrapper is the
same 73-byte XChaCha20-Poly1305 format as `wrapped_master` with the AAD domain
`baylee-central/recovery-wrap/v1\0`. No password stretching: the code carries 160
bits. `central-crypto` gains `RecoveryCode` (generate, parse, wrap, unwrap) with
known-answer tests. Central stores the 73 bytes as `recovery_wrapped_master`;
reset, which uses it, comes with M1.

### Keeping the recovery code

*12.11.2026, by the owner's decision after the trade-off was explained
("Wie empfohlen + Mail an mich"). The record of its three passes is in
[security assurance](security-assurance.md), "Ways to keep a recovery
code".* `crates/central-web/src/islands/keep.rs`, used by the code's step
of a registration and of a sign-in that makes a code for a moved account
(`auth.rs`, `code_view`), and since the same day by the reset page's step
that shows a new vault's code (`reset.rs`, `code_view`; it knows no
address, so its mail leaves the recipient to the mail program). Each way is made in the page when its button is
pressed and not before; none of them sends anything to Central, logs the
code, keeps it in the page's storage or puts it into the page's address.

- **Copy** to the clipboard (`clip.rs`), as before.
- **Save as a text file**, `baylee-recovery-code.txt`: the title, the
  sign-in name, the code on a line of its own, and two lines on what it
  does and how to keep it. A `data:` link is made for the click and taken
  away again; until then no attribute of the page holds the code.
- **Print**: the button draws a sheet with the same words into the step
  and asks the browser to print. While the sheet is in the page, a print
  shows it alone, also from the registration's dialog (`pages.css`); on a
  screen it is never shown.
- **Save in password manager**, only where the browser has
  `PasswordCredential` (Chrome, Edge and the other Chromium browsers): the
  code is offered as a password under the id "Baylee recovery code
  (<sign-in name>)". The browser asks the person; the page learns only
  whether the browser took the offer, not whether it was saved. Firefox
  and Safari have no `PasswordCredential` and no other way for a page to
  offer a password: there the button is not shown.
- **Mail it to myself** opens the person's own mail program with the code
  in the body (`mailto:`): to the address typed into the step's e-mail
  field where it is one Central would mail to, else to nobody yet. The
  subject and body are made in the page on the click; Central never sees
  them. A warning stands before the button and describes it: the code
  then lies in the mailbox, and whoever reads the mailbox can have a reset
  code sent there too, and with both reset the sign-in and open the data.

The e-mail field (a registration only, and only where `EmailRequest::Status`
says this Central mails and the account has no address): an address typed
there is checked by Central's rule when the person goes on, and named once
the code is kept and Central's terms are accepted, which naming one needs
([e-mail](email.md)). The step after it is the account area's e-mail
section (`email.rs`, `Begin::Name`) as a step of its own: it sends `Set`
without asking for the password, since the registration's sign-in is less
than five minutes old, and shows the code's field; the person confirms the
code there or later on the security page. Where Central refuses `Set`
(most likely because the sign-in became older than five minutes; the
mailer refusing the address is answered the same), the section's form asks
for the password and signs in once more, as in the account area. Leaving
the field empty skips all of it.

### A recovery code after a sign-in

*Added 22.10.2026 (the accounts and administration design, step 2).* An
account moved from the official gateway has no recovery code: Baylee's
client enrols it without one, and the website used to tell it to keep one
safe. Such an account makes its code on the website:

1. After `LoginFinish` the page asks `AccountSummary` while the sign-in's
   Worker still holds its state. For an account without a code the Worker
   runs `recovery_create` in place of `login_confirm`: it opens the master key
   with the sign-in's wrapping key (so a wrong key fails as `login_confirm`
   does), makes a new `RecoveryCode` and wraps the same master key under it
   with the account's context (§"The recovery code"). The page keeps only the
   code and the 73-byte wrapper, never the key; the Worker ends with the
   sign-in. When `AccountSummary` does not answer, the sign-in confirms the
   password as before and offers no code.
2. On `/signin` the terms come first, since storing the code needs them
   accepted; then the code is shown with the registration's step (copy,
   download, "I kept it"), or put off with "Not now", which throws it away:
   nothing of it reaches Central. Only after "I kept it" does the page send
   `SetRecovery`, and it keeps the 73 bytes until Central answers, so a lost
   answer is retried with the same bytes; "Not now" is gone once a request
   may have stored the code.
3. Later, the account page's "Make a recovery code" asks for the password
   again ("To make the code, sign in again") and signs in anew the same way,
   which replaces the browser session. The master key exists only inside one
   flow's Worker, so the code is made right after a sign-in and never from a
   stored key.

`SetRecovery { recovery }` stores the 73 bytes when the browser session's
account is enrolled, has finished moving (`legacy_closed`, or registered),
has no wrapper, and the session's password sign-in (`sessions.created_at`,
which renewal never moves) is less than 10 minutes old, so a stolen cookie
cannot put a wrapper on an account long after its owner signed in. The same
bytes again are `Done` at any time; other bytes while a wrapper exists are
`Conflict`; an unfinished move, a sign-in 10 minutes old or older, a
malformed wrapper or no live browser session are `Rejected` (the design left
these two codes open). A registered account keeps the wrapper of its
registration. Replacing a code comes with password reset (M1). Migration
`m20261022_000010_legacy_recovery` lets an enrolled legacy account hold the
wrapper (`account_recovery`: none while pending, required for an enrolled
registered account, allowed for an enrolled legacy one), and
`invalidate_legacy` clears it with the vault epoch it was bound to; whatever
later raises the epoch (an administrator's reset, M1) must clear it in the
same statement.

### Display name and tag

*Added 23.10.2026 (the accounts and administration design, step 3).* What
other players see of an account is its handle, `Name#3fa2`: a display name,
which is not unique, and a tag, which is. The username is the private
sign-in name (`Discover` treats it so): the account page shows it small, as
"Sign-in name", and nothing else does; the header and the account page show
the handle.

- **The name** keeps the gateway's rule exactly (central-api's
  `profile::valid_display_name`, Baylee's `auth.rs` `valid_display_name`,
  tested on the same table of cases in both repositories): 3 to 16 ASCII
  letters and digits, with `_` and `-` only inside, case kept. A person may
  not choose `admin`, `administrator`, `baylee`, `central`, `moderator`,
  `official`, `support` or `system`, compared without case, `_` and `-`
  (`profile::reserved`); a name carried from the official gateway is kept as
  it is.
- **The tag** is unique at Central, positive, at most `i32::MAX` (a
  gateway's column), drawn as `handle.rs` draws it (`7` is `0007`, `0x10000`
  is `10000`, `profile::tag_text`). It changes only when an administrator
  sets another one ([administration](administration.md) §"Gateways, tokens
  and tags"): chat on a gateway is keyed by it, so earlier whisper threads
  are gone after the change. A registered account draws its tag with its
  reservation, at random (operating-system randomness, rejection-sampled)
  from `REGISTERED_TAG_FLOOR = 0x1000` to `0xffff` while fewer than half of
  those are taken, then from `[16f, 256f - 1]`, and so on to `i32::MAX`
  (`profile::tag_ranges`). Each draw runs in a savepoint; one another account
  holds is drawn again, eight draws in all (`TAG_DRAWS`), then the request is
  `Busy`, as it is once no range is left. An abandoned registration frees the
  tag with its name. Random rather than counted, so a tag says nothing about
  how many accounts exist.
- **A moved account** keeps the name and tag it had at the official
  gateway. Those tags are the gateway's identity values, below the floor
  (its highest was 13 on 23.10.2026, guests included; it makes no local
  accounts once bound to Central). The bridge sends both in `LegacyReserve`
  ([native login](native-login.md)); a carried tag another account holds is
  replaced by a drawn one, and the name is kept.

*Departures from the design, found in the code:* names written before
15.09.2026 (when the gateway's rule was 3 to 32 characters without the edge
rule, and the importer of `gateway-store.json` copied names unchecked) may
not keep the rule, which migration 000011's CHECK holds; the bridge
therefore sends no name for such an account, which keeps its tag and
chooses a name on the website. So `LegacyProfile` sets name and tag once
while the account has **no tag** (a moved account has a name only with its
tag), not "while it has no name", and `SetDisplayName` refuses a moved
account whose tag the bridge has not carried yet, so the website never asks
a moved account for a name before its own arrives.

**Wire.**

- `Register` gains `display_name: Option<String>` (`#[serde(default)]`,
  absent when `None`); a malformed or reserved name is `Rejected`, after
  the key check. `None` chooses it later.
- `SetDisplayName { display_name }` (browser listener, the cookie's session,
  Central's terms accepted) → `Done`, only while the account has no name
  (renaming comes later): the same name again is `Done`, so a retry after a
  lost answer takes; another while one is set is `Conflict`; a malformed or
  reserved name, no live browser session, or an account without a tag (a
  moved one not carried yet) is `Rejected`.
- `AccountSummary` carries `display_name` and `tag` (its shape is step 2's).
- `LegacyReserve` and `LegacyProfile`, the bridge's, are in [native
  login](native-login.md); `GatewayProfile`, a gateway's, in [gateway
  registry](gateway-registry.md).

**Schema: migration `m20261023_000011_profiles`.** `accounts.display_name
text` with the rule as a CHECK (`^[A-Za-z0-9][A-Za-z0-9_-]{1,14}[A-Za-z0-9]$`),
`display_name_at timestamptz` (when the name was set, present exactly with
the name), and `tag integer UNIQUE CHECK (tag > 0)`. A `DO` block draws a
tag for every registered account, pending or enrolled, the same way (over
`random()`: tags are public); legacy rows stay without one until the bridge
carries it. `tag` stays nullable, so the previous binary, which the deploy
hook restarts when a new one fails, still registers (000007's rule); the new
binary draws a missing tag at that account's next sign-in, and a sign-in
whose draw fails goes on without one. No grant changes: the runtime updates
`accounts` already.

**The website.** Registration has "Display name" below the username ("How
other players see you: 3 to 16 letters, digits, - and _. Your tag is drawn
for you."). An account without a name sees "Choose your display name" first
on the account page, once the terms are settled; until it chooses, gateways
show it as `Player` with its tag (step 4). A moved account whose name and
tag have not been carried yet is told they come with its next sign-in at the
official gateway with the Baylee client.

## Server (Central)

**Migration `m20261021_000007_registration`:**

- `accounts.legacy_id` becomes nullable. A new `source` column (`legacy` |
  `registered`) has a CHECK that pairs it with `legacy_id`. Legacy rules
  (bridge-only operations, `legacy_closed`) apply to `source = legacy` only.
- `accounts.recovery_wrapped_master bytea NULL`, 73 bytes when present, required
  for `source = registered` once enrolled.
- `beta_keys (id uuid v7, digest bytea(32) UNIQUE, uses_left int CHECK >= 0,
  expires_at timestamptz NULL, note text NOT NULL CHECK length <= 200,
  created_at, revoked_at NULL)`. Only SHA-256 digests are stored.
- A reservation holds one use until the enrollment completes or expires
  (`accounts.beta_key_id` on the pending row): a key admits a registration
  while `uses_left` exceeds its live reservations, and the enrollment commit
  spends the use and clears the link. An expired reservation so gives the use
  back without a clean-up job.
- `sessions.kind` (`native` | `browser`). `sessions_one_per_account` becomes
  one per account **and kind**, so a browser sign-in never ends the client's
  session. Lifetime is 30 minutes for both, renewed while in use up to 12 hours
  after the sign-in ([operations](operations.md#sessions)). A browser session is stored
  under SHA-256 over a fixed domain label and the token, not the native
  bearer's digest: a cookie value presented as a bearer on `/v1/native` finds
  no session, and a native bearer is no browser session. (Changed in the
  server PR: the first draft left the kinds interchangeable, so a stolen
  cookie would have been a full native session.)
- `ops/grants.sql` and `grants.rs` gain the new table and privileges: the
  runtime role gets `SELECT` and `UPDATE (uses_left)` on `beta_keys`, nothing
  more; issuing and revoking keys stay with the operator.

**Beta keys (until 11.11.2026).** The closed beta's keys, issued with
`--issue-beta-keys-stdin`, went with the open beta: migration 000023 drops
`beta_keys` and `accounts.beta_key_id`, keeps the audit's `beta_key_id` as
history, and adds `registration`, the administrators' switch
([operations](operations.md#registration)).

**Wire.**

- `Request::RegisterChallengeFor { signals }` → `Response::RegisterWork {
  nonce, parts, difficulty, memory, passes, expires_at }`, or
  `RegistrationClosed` (11.11.2026, browser listener only): the proof of
  work of `docs/registration-abuse.md` §"A proof that costs memory", which
  `Register` carries as `nonce` and `counters`. For each of `parts` parts
  the page finds a counter whose Argon2id output (`memory` KiB, `passes`
  passes; `central_crypto::proof`, the one function for the page and for
  Central) begins with `difficulty` zero bits. Central asks 16,384 KiB,
  one pass, eight parts and 4 to 10 bits; `expires_at` is five to twenty
  minutes away, longer for a harder challenge. The page refuses a price
  above `central_api::register`'s ceilings (each field, and
  `WORK_MAX_COST` for the whole) before it works on it. `signals` is what
  the page saw of its own browser, one bit each
  (`central_api::register::SIGNALS`: 1 `navigator.webdriver`, 2 no
  `navigator.languages`; other bits are ignored); the report and the
  request's headers only make the challenge harder, by up to four bits
  (`docs/registration-abuse.md` §"Hardened").
- `Request::RegisterChallenge` → `Response::RegistrationClosed`, always
  (browser listener; `Rejected` natively). It asked for the SHA-256 proof
  of the first build of 11.11.2026, `Response::RegisterChallenge { nonce,
  parts, difficulty, expires_at }`, which this Central neither hands out
  nor takes. Both shapes stay in `central-api`: a page of this build that
  meets a Central from before asks this way and solves that proof
  (`pow-worker.js`).
- `Request::Register { username, display_name, nonce, counters }`
  (the name since 23.10.2026, optional on the wire; the key until 11.11.2026) → `Response::Enrollment`. It needs
  no credential, shares the unauthenticated budget and takes a per-address
  budget on the browser surface. It is served **on the browser surface only**;
  `/v1/native` answers `Rejected`. (Changed in the server PR: the per-address
  budget is the control against key guessing and name probing, and the native
  surface has none; registration in the native client is M6.)
- `EnrollmentFinish` gains `recovery: Option<Vec<u8>>` with
  `#[serde(default)]` and `skip_serializing_if = "Option::is_none"`, so native
  clients pinned before the change still decode, and a legacy enrollment is sent
  exactly as before. It is required for registered accounts and refused for
  legacy ones; the schema holds the same rule.
- `Register` is checked in this order: the address's budgets (the
  listener), a worker of the service's four (`Busy` without one; the
  challenge stays, and the page sends the same solution once more), then
  the challenge leaves the table (one use), then the counters are checked
  off the executor, part by part, ending at the first that does not hold.
  A wrong answer to a living challenge costs Central one evaluation of
  16 MiB, a right one eight.
- A missing, spent, expired or wrong proof of work is `Rejected`, and so is
  every `Register` while registration is closed, before the name is looked
  at. Only with a valid proof does a taken name answer `Conflict`, so the page
  can say so. (Until 11.11.2026 the beta key held that place.) What stays:
  whoever solves a proof learns whether one sign-in name is free; each answer
  costs a proof (a second of a laptop's processor and more, rising while
  registrations flood in)
  and spends the address's budgets (5 an hour, 10 a day), and sign-in names
  are private, never shown to other players. A malformed name is `Rejected`.
- A name is taken by any enrolled account, any legacy account (also one whose
  migration is unfinished) and any registration whose ticket still lives. An
  abandoned registration (its ticket expired) gives its name back to the next
  registration and to a legacy migration; each registration also removes up to
  16 abandoned ones, oldest first. A legacy name is never given to a
  registration.
- The enrollment of a registered account fails when its key was revoked after
  the reservation; it spends the reserved use otherwise, even if the key has
  expired since.
- `LoginStart` and `LoginFinish` belong to one surface: an attempt started on
  `/v1/native` cannot finish on the browser surface, nor the reverse.

**The browser surface.** It is a second listener, `browser.listen` in
`central.json` (absent means off), behind `baylee.app`'s Caddy as `/api/*`
with the `Host` header unchanged. It has one route, `POST /api/v1`, which
takes the same tagged `Request` JSON. Only these operations are allowed:
`Discover`, `LoginStart`, `LoginFinish`, `Register`, `EnrollmentStart`,
`EnrollmentFinish`, `SessionStatus`, `RenewSession`, `Logout`, `Terms`,
`TermsStatus`, `AccountSummary`, `SetRecovery`, `SetDisplayName`,
`AcceptTerms` for Central's terms (a gateway's terms are accepted from the
client), `GatewayDirectory`,
the public gateway directory of the `/gateways` page ([gateway
registry](gateway-registry.md)), and since 11.11.2026 `PublicFigures`, the
numbers read beside it ([gateway
registry](gateway-registry.md#activity-and-the-public-figures); both
listeners, no session), since 28.10.2026 `ChangePasswordStart`
and `ChangePasswordFinish` ([native login](native-login.md) §"Password
change"; served on both listeners, the page's form since 12.11.2026), and
`Admin`, an administrator's requests ([administration](administration.md);
browser listener only), and `ResetStart` and `ResetFinish`, a reset code's
use ([account reset](account-reset.md); both listeners, no session; the page since 12.11.2026), and
`AccountExport`, `DeleteAccountStart` and `DeleteAccountFinish` ([account
deletion](account-deletion.md); both listeners, with the session, Central's
terms not required; a finished deletion clears the cookie), and, since
30.10.2026, `Owner`, an account's own area (§"The account area"; browser
listener only, with the session). Every
other operation is refused. The list is
one exhaustive match in `central-identity` (`browser_operation`), so a new
operation does not compile until it is decided there. The cookie's token is
used only by `SessionStatus`, `RenewSession`, `Logout`, `TermsStatus`,
`AcceptTerms`, `AccountSummary`, `SetRecovery`, `SetDisplayName` and the
password change's two requests (whose finish sets a new cookie, as
`LoginFinish` does), `Admin`, `Owner`, and the export and the deletion's two; the directory and the
figures use none, and
the page sends it with `credentials: "omit"`.

- `AccountSummary` (the cookie's session; no accepted terms needed) answers
  `AccountSummary { identity, username, display_name, tag, source, recovery,
  admin, signed_in_at, expires_at }`: `source` is `legacy` or `registered`,
  `recovery` whether the account has a recovery wrapper, `signed_in_at` the
  session's password sign-in in Unix seconds (renewal never moves it).
  `display_name` and `tag` are the account's (§"Display name and tag", since
  23.10.2026), `null` until it has them; `admin` is `false` until the later
  step that keeps roles; the shape was complete from the start.
- `SetRecovery { recovery }` (the cookie's session; Central's terms
  accepted) answers `Done` (§"A recovery code after a sign-in").
- `SetDisplayName { display_name }` (the cookie's session; Central's terms
  accepted) answers `Done` (§"Display name and tag").

The native listener refuses all three (`Rejected`): making a code and
choosing a name in the client are M6.

A request is refused (403, `request not permitted`) unless all of these hold.
Each header is compared byte for byte and must appear exactly once.

- `Host` is `baylee.app`.
- `Origin` is present and is exactly `https://baylee.app`.
- `Sec-Fetch-Site`, when sent, is `same-origin`.
- `Content-Type` is `application/json` (no parameters).
- The header `X-Baylee-Central: 1` is present. A custom header forces a
  preflight cross-origin, and no CORS header is ever sent.
- No `Authorization` header is present: the session travels only in the
  cookie.
- The connection comes from loopback (the proxy), and the operation is one of
  the list above.

Other paths answer 404 and other methods 405, without reading the body.
The conditions above and the client address (below) are checked on the
headers alone, before the body is read; then the address takes one of four
places for its requests in flight, else 429 `Busy` at once (11.11.2026). A
body is at most 64 KiB (413) and must arrive within two seconds (408); the
service then has five seconds to answer (408).
A malformed or unknown request is `Rejected` (401), as on `/v1/native`; the
other answers keep the native statuses too (409 `Conflict`, 429 `Busy`, 503
`Unavailable`, 403 `TermsRequired`) with the same fixed text.

The cookie is
`__Host-central=<64 hex>; Path=/; Secure; HttpOnly; SameSite=Strict;
Max-Age=43200`: the session's 12-hour cap; the session itself ends after 30
minutes unless renewed, and the cookie of an ended session finds none. The session is the native session with `kind = browser`. Only
exactly one well-formed `__Host-central` counts; anything else in `Cookie` is
no session. `Logout` clears the cookie (`Max-Age=0`) in every answer, also when
the session was already gone.

Per-address budgets read the last `X-Forwarded-For` hop, and only from the
loopback proxy. Without a well-formed last hop a request is refused (400).
IPv6 addresses count by their /64; an IPv4 address mapped into IPv6 is that
IPv4 address. Each budget remembers at most 4096 addresses, in memory only, and
forgets one once its window has passed (when the address asks again, or at
the latest a window's length later); when full, the oldest window makes
room, so a crowd of addresses cannot lock new ones out.

| Operation | Budget per address |
| --- | --- |
| `Register` | 5 an hour and 10 a day |
| `RegisterChallenge` and `RegisterChallengeFor` | 20 an hour, together |
| `Discover` | 30 a minute |
| `LoginStart` | 30 a minute, apart from `Discover` |
| `GatewayDirectory` and `PublicFigures` | 60 a minute for the two together, apart from the others |
| `ResetStart` | 10 an hour |
| Every request that needs no session | 600 a minute, of the 32 a second all clients share; when those are spent, an address that was served fewer than 30 in its minute draws on 8 a second kept beside them (12.11.2026, [native login](native-login.md#guessing-a-password-online)) |

The listener also tells the service the address, and the service counts
guesses at passwords by it (11.11.2026): each sign-in start, each start of a
password change (two) and of a deletion is a guess until its proof, with
counts per account, per address and per account at an address, and a proof
gives the guess back and makes the address the account's own for 30 days
([native login](native-login.md) §"Guessing a password online"). The
refusal is the same `Busy`, which the page shows as it shows any other; it
can last until the hour or the day ends, and the answer does not say how
long.

The global budgets of the native surface apply as well: `AccountSummary`
counts with the control requests (`SessionStatus`, `RenewSession`, …),
`SetRecovery`, `SetDisplayName` and the password change with the data
requests, per session, and none has a per-address budget, since each needs a
live browser session; a change's start also spends two units of the
account's login budget. The
native surface (`/v1/native`) is unchanged, except that it refuses
`Register`, `AccountSummary`, `SetRecovery`, `SetDisplayName`, `Admin` and a
browser session. `AccountSummary.admin` is the account's role since
28.10.2026.

**Local runs.** `central-service --dev-accounts-stdin` takes an optional
`"browser": {"listen": "127.0.0.1:<port>", "origin": "http://127.0.0.1:<page
port>"}`. The origin is a numeric loopback `http` origin with a port (as
`Endpoint::development`; `localhost` is refused), and that listener expects
`Host` to be the origin's authority and `Origin` the origin exactly, in place
of `baylee.app`; every other condition above holds, so the page's development
proxy must keep `Host` and add `X-Forwarded-For`. The issuer is the
development `endpoint`'s, the native listener's origin, so a WebAssembly build
for such a run uses `Endpoint::development(<endpoint>)`, never the committed
live issuer. The deployed `--accounts-stdin` has no origin field: `baylee.app`
is fixed there.

### The account area

*30.10.2026.* `Request::Owner { request: OwnerRequest }` (central-api
`owner`) answers `Response::Owner { answer }`, or `Done`. Browser listener
only; the native listener refuses it before anything else. The store finds
the account through a live browser session of the current credential
(`s.kind = 'browser'`), so a native bearer acts on nothing here.

- `Sessions` → the account's live sessions, the asking one first: kind
  (`browser` or `native`), whether it is the asking one, its password
  sign-in, its end, and for each the gateways it holds a live lease at (their
  origins). Central keeps no device; the page says so.
- `EndClientSession` → `Done`: deletes the account's native session, and with
  it its tickets and leases (their foreign keys cascade). The website's own
  session ends with `Logout`.
- `Gateways`, `AddGateway { origin, name }`, `CreateToken { gateway, name,
  scopes, days }` and `RevokeToken { token }`: [gateway
  registry](gateway-registry.md) §"Gateway tokens". `CreateToken` alone
  needs a password sign-in less than 30 minutes old (`OwnerRequest::mints`).
- **Terms.** `AddGateway` and `CreateToken` need Central's terms accepted;
  reading the sessions and gateways, ending the client's session and
  revoking a token do not, so a person who will not accept new terms can
  still see what exists and shut it.

The pages: `/account` (the profile, as before), `/account/security` and
`/account/gateways`, in one frame with the administration's
(`/admin`, `/admin/users/<id>`, `/admin/gateways`, `/admin/audit`,
`/admin/legacy`), whose
part of the navigation only an administrator sees. A change that needs the
recent password asks for it first and signs in again (the same OPAQUE
sign-in, in its Worker), which replaces the browser session. A token is shown
once, in its text form, in a dialog that closes only when the person says
they kept it; the page keeps it nowhere.

## Pairing

The owner's decision of 30.10.2026 (Baylee `docs/proposals/deep-links.md`
§"Protocol specification" is the contract): a person signed in on
`baylee.app` starts Baylee's client signed in, without typing the password
again. The password opens the master key once, where it was typed, and is
dropped; the page brings the master key to the client sealed to a key the
client made, and Central hands the client a native session only together
with that sealed key, once.

**The key check.** `accounts.key_check` (32 bytes, migration 000016) is
derived one way from the master key (`crypto-protocol.md` §"Derived from the
master key"). It is set once, with `SetKeyCheck`, while none is set and
within two minutes of the session's password proof: the client and the
page send it after a password sign-in when `KeyCheckStatus` says there is
none, so a new account and an existing one gain it at their next sign-in.
A new vault (a reset to a new one) clears it, and the next sign-in sets the
new vault's. Unlike Baylee's specification, neither `EnrollmentFinish` nor
`ResetVault::New` carries a check: the sign-in after either sets it. Both
requests work on either listener; a set check is `Conflict`, a late one
`Rejected`. A stolen session or cookie therefore cannot plant a check of its
own. No pairing starts for an account without one.

**The steps.**

1. `PairStart` (browser listener, cookie) → `PairStarted { handle,
   expires_at }`: five minutes, at most three open per account and a
   thousand in all; ended pairings are swept first.
2. The page opens `baylee://pair?h=<handle>&s=<S>`, `S` being 32 random bytes
   of its own that Central never sees.
3. `PairClaim { handle, public_key, challenge, mac }` (native listener, no
   session) → `PairAccepted { expires_at }`: within the pairing's first two
   minutes, once. The same claim again, as a client resends it after losing
   the answer, gets the same answer (compared in constant time); any other
   is `Conflict`.
4. `PairStatus { handle }` (the starting cookie) → `PairClaimed { … }` or
   `Pending`. The page checks the MAC under `S`, shows the comparison code
   and the account's name beside Approve.
5. `PairApprove { handle, sealed, proof }` (the starting cookie) → `Done`:
   `sealed` is the master key sealed to the client's key under `S` (HPKE's
   PSK mode, `crypto-protocol.md` §"Pairing"), so Central cannot make one
   the client opens. Central checks the proof (HMAC under the stored key
   check over handle, key, challenge and the seal's digest) and stores the
   80-byte seal, once.
   `PairCancel { handle }` ends a pairing at once.
6. `PairPoll { handle }` (native, no session) → `Pending` or `Approved`, at
   most 300 times; then `PairRedeem { handle, verifier }` →
   `PairSession { token, identity, username, display_name, tag, sealed,
   expires_at }`, the name and tag for the client's question beside the
   username. The pairing is deleted whatever the redeem finds: a wrong
   verifier, an account at another epoch or a proof too old each end it,
   and only a storage failure leaves it. The session replaces the account's
   native session as a sign-in does.

**The 12 hours.** `sessions.proven_at` is the password proof a session rests
on when it is not its own creation (NULL: the creation). A paired session
takes the browser session's, the lifetime check and renewal count from
`COALESCE(proven_at, created_at)`, and a redeem whose proof leaves less than
a minute gives no session. Every check of a password sign-in's age (the
export's, recovery's, the tokens' and administration's step-ups) and the
"signed in at" an account sees read the same, so a pairing never counts as a
fresh password sign-in. NULL rather than a copy of `created_at`: every
existing insert stays valid, and the meaning is the same.

**Budgets.** `PairStart` 10 an hour and `PairStatus` 120 a minute per
address on the browser listener; `PairApprove`, `PairCancel` and the key
check's two requests have no address budget, since each acts on a live
session, which counts them as it counts its other requests. The native
listener's claim, polls and redeem have a window of their own (64 a second,
and of it 240 a minute for one address since 11.11.2026), so polling cannot
spend the sign-ins' budget and one address cannot spend everyone's polls,
besides the per-pairing limits above. `PairCancel` ends a pairing in any
state until it is redeemed, so a page that is left can end its own at
once. The browser listener
serves `PairStart`, `PairStatus`, `PairApprove`, `PairCancel`,
`KeyCheckStatus` and `SetKeyCheck`; the native listener refuses the first
four and serves the claim, poll and redeem.

**Deletion.** A pairing goes with its browser session (`ON DELETE CASCADE`):
signing out of the page ends it. An expired one goes with the next start and
with the operator's daily sweep ([administration](administration.md) §"The
sweep").

## Browser crypto (`central-web-crypto`)

A `cdylib`, `#![forbid(unsafe_code)]`, linking `central-crypto` (client side),
`central-api` (the live issuer) and `wasm-bindgen`. Its exports take bytes and
strings and return opaque state objects and objects of bytes; every network
call is the page's.

- `enrollment_start(password, account, epoch)` → an `Enrollment` state whose
  `message` is the registration request (32 bytes)
- `enrollment_finish(state, response)` → `Enrolled { record, wrapped_master,
  recovery_code, recovery, document }`; freeing it wipes the code
- `login_start(password, account, epoch)` → a `Login` state whose `message`
  is KE1 (96 bytes)
- `login_finish(state, response)` → a `LoginKey` state whose `message` is KE3
  (64 bytes); a wrong password fails here, before anything is sent
- `login_confirm(key, wrapped_master)`, which fails when the master key does
  not unwrap
- `recovery_create(key, wrapped_master)` → `Recovered { recovery_code,
  recovery }`, in place of `login_confirm` for an account without a recovery
  code: it fails as `login_confirm` does, then wraps the same master key
  under a new code; freeing it wipes the code (§"A recovery code after a
  sign-in", added 22.10.2026)
- `reset_start(password, account, epoch, recovery_code?)` → a `Reset` state
  whose `message` is the registration request (32 bytes) for `ResetStart`.
  With a recovery code the vault is kept and the password registered at
  `epoch`; without one a new vault is made and the password registered at
  `epoch + 1`, as `central-client`'s `reset` does. The recovery code is read
  here, before the reset code is sent anywhere
  ([account reset](account-reset.md), added 12.11.2026)
- `reset_keep(state, response, recovery)` → `ResetKept { record,
  wrapped_master }`: the challenge's recovery wrapper opened with the code
  and the same master key wrapped under the new password; a code that does
  not open it fails here, before `ResetFinish`, so the reset code is not
  spent
- `reset_new(state, response)` → `Enrolled`, as a registration's: a new
  master key under the new password and under a new recovery code, and its
  first document, for `ResetVault::New`
- `password_start(key, wrapped_master, current, new)` → a `PasswordChange`
  state whose `login` is KE1 (96 bytes) and whose `registration` is the
  request for the new password (32 bytes), for `ChangePasswordStart`. It
  takes the `LoginKey` of a sign-in made in the same Worker and opens the
  master key with it ([native login](native-login.md) §"Password change",
  added 12.11.2026)
- `password_finish(state, challenge, registration)` → `PasswordChanged {
  proof, record, wrapped_master }`: KE3 for the current password (a wrong
  one fails here) and the same master key under the new one, as
  `Session::change_password` makes them
- `central_origin()`, the Central the module is bound to (the live one)
- `work_start(nonce, memory, passes, parts, difficulty)` → a `Work` state,
  and `Work.search(part, from, tries)` → the first of `tries` counters
  from `from` that solves the part, or nothing: registration's proof of
  work (11.11.2026), `central_crypto::proof`, the function Central checks
  with. `work_start` refuses a price above the ceilings before it takes
  memory. The Worker searches in turns of eight counters, so the page can
  show how far it is. The page ends that Worker when the proof is found
  and stretches the password in a fresh one.

*Changed 21.10.2026 while building it:* `login_finish` first took
`wrapped_master` too, but that only arrives in the answer to the KE3
`login_finish` makes, so the check is a step of its own. The states are
exported objects rather than fields of plain objects, which would have needed
`js-sys` as a direct dependency; a step that takes a state empties its
JavaScript object, and an error is `rejected` or `unavailable`.

A state is opaque, consumed by the step that takes it and wiped when dropped,
also when the page frees it unused. Argon2id (64 MiB) runs in a module
Worker, one per flow, terminated at the flow's end. Registration has one
more before it, for its proof's 16 MiB, ended when the proof is found: a
Worker's memory does not shrink, so the two never share one. The JS glue is made by
`wasm-bindgen` (CLI, the same version) and committed under
`crates/central-web/crypto/` with its SHA-256 sums, so `npm run build` needs no
Rust toolchain; `cargo xtask wasm` rebuilds it on x86-64 Linux (other machines use the
pinned container), and `cargo xtask wasm --check` (a CI job) proves the
committed bytes are what the source builds ([xtask](xtask.md)).

## Web pages (`web/`)

`/register`, `/signin` and `/account` (shows the account's handle and, small,
its sign-in name; signs out). The header gains "Sign in" and the account's
handle (§"Display name and tag", since 23.10.2026; the username before).
Since 22.10.2026
the account page also says where the account came from and whether it has a
recovery code (`AccountSummary`), and offers a moved account "Make a
recovery code" (§"A recovery code after a sign-in").

Registration has these fields (the beta key until 11.11.2026; a hidden
field for form robots, and a form sent within three seconds is not sent):

- username, with the name rules shown
- display name, with its rules shown (since 23.10.2026)
- password, twice, with a strength hint and no composition rules
- confirmation of the minimum age of 16 (Central's terms §2)
- acceptance of Central's terms and privacy notice, linked and shown in the
  page's language

Then the recovery code. Every message is in Fluent, in English and German.

The privacy notice gains:

- the session cookie
- the account data (username, credential record, wrapped keys, encrypted
  document, recovery wrapper, sessions)
- the per-address counters (memory only)
- the third parties: none

The CSP gains `'wasm-unsafe-eval'` in `script-src`.

*As built (21.10.2026, the old `web/README.md` §"Accounts", removed 12.11.2026):*

- The password needs eight characters at least (OWASP ASVS 6.2.1) and at most
  1024 bytes (the crypto's bound); the strength hint is advice only.
- After `EnrollmentFinish` the page signs in at once, so the password is let
  go while the code is read; if that sign-in fails, the account still exists
  and the page sends the person to `/signin` after the code.
- A terms version is accepted only when the texts `Terms` serves hash to the
  digest `TermsStatus` names (central-api's `terms::digest`, recomputed in
  the page), so an acceptance names exactly what was shown.
- The browser surface names the account (agreed with the server work on
  21.10.2026): `BrowserSession { identity, username, wrapped_master,
  expires_at }`, and `SessionStatus` answered there as `browser_status {
  identity, username, expires_at }`. The page takes exactly these shapes.
  Since 23.10.2026 the name it shows is the handle of the account's
  `AccountSummary` (§"Display name and tag"), always Central's, held in
  memory; without a display name the header says "Your account".
- The page keeps only the session's end in localStorage
  (`baylee-central-session`), so after a reload it knows to ask whose
  session it is (`AccountSummary` since 23.10.2026, `SessionStatus` before;
  until then the header says "Your account"), and forgets it on signing
  out, at its end or on Central's 401.
  The privacy notice says so. The earlier key that also held the name
  (`baylee-central-account`) is deleted wherever the page finds it.

## Baylee (the gateway, G1)

In bridge mode `login_for` admits a redeemed identity without `legacy_id` as a
projection (`baylee_db::central::projection`), unless `BAYLEE_REGISTRATION` is
`off`. Central's beta key replaces the gateway's for these accounts. An identity
with a legacy link still has to match it. An e2e test covers both.

## Operations

1. Central's new release: migrate, grant, start, through `ops/deploy-hook`.
2. `central.json` gains `browser.listen` (`127.0.0.1:28791`). The `baylee.app`
   site block passes `/api/*` to it instead of answering 404.
3. Beta keys were issued with the new mode, until the open beta
   (11.11.2026) removed them.
4. The official gateway gets `BAYLEE_CENTRAL_CONFIG_FILE`
   (`{"endpoint": "live", "bridge": …}` from Central's `secrets.json`), is
   restarted, and its `/auth/config` says `central`.
5. The legal texts have to be published as a real version first (the owner's
   review). Today both of Central's documents are `draft`, which is never in
   force.

## Not in this round

- password change (served since 28.10.2026, the page's form since
  12.11.2026: [native login](native-login.md) §"Password change") and reset
  (by an administrator's code: served since 28.10.2026, the page since
  12.11.2026, [account reset](account-reset.md); by a mailed code: served
  since 12.11.2026, [e-mail](email.md), no form on the page yet)
- e-mail (served since 12.11.2026, [e-mail](email.md); no form in the
  account area yet), 2FA, profile picture (the display name came on
  23.10.2026)
- export and self-service deletion: by request to the operator until M1
- the administration UI (M2)
- registration in the native client (M6)
- deep links (M4/M6)
