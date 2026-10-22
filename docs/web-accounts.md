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
  `wasm-bindgen` 0.2.129 as a direct dependency, getrandom 0.4.3 `wasm_js` and
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

1. `Register { username, beta_key }` → `Enrollment { identity, ticket }`. The
   key is checked and one use reserved; the username must be free under the
   versioned name policy. The account row is created with no legacy source.
2. The browser runs OPAQUE registration with the context
   `(issuer, identity.account, identity.epoch)`; `EnrollmentStart { ticket,
   message }` → `EnrollmentChallenge`.
3. The browser makes a random master key, wraps it under the OPAQUE export key
   (the 73-byte wrapper of `vault.rs`), makes a recovery code and wraps the same
   master key under it (below), and seals the initial account document (below)
   as object 1. `EnrollmentFinish { ticket, record, wrapped_master, document,
   recovery }` → `Done`. The commit is atomic and consumes the key's use.
4. The page shows the recovery code once (copy, download as a text file, and a
   confirmation that it was kept) and signs in (below) to accept Central's terms.

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
read and validate the document, `ConfirmRestore`, then a gateway ticket. The
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

**Beta keys.** A launcher mode `--issue-beta-keys-stdin` reads `{ "database":
…, "count": n ≤ 100, "uses": n ≤ 1000, "expires_days": n?, "note": "…" }`
(the migrator login, as `--migrate-stdin`). It prints each new key once as
`CENTRAL-XXXX-XXXX-XXXX-XXXX`: 80 random bits in Crockford base32, read like
the recovery code. The admin UI (M2) comes later.

**Wire.**

- `Request::Register { username, beta_key }` → `Response::Enrollment`. It needs
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
- A bad, used-up, expired or revoked key is `Rejected`, before the name is
  looked at. Only with a valid key does a taken name answer `Conflict`, so the
  page can say so: the key is what stops enumeration, and its holder may learn
  whether a name is free. A malformed name is `Rejected`.
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
`TermsStatus`, `AcceptTerms` for Central's terms (a gateway's terms are
accepted from the client) and `GatewayDirectory`, the public gateway directory
of the `/gateways` page ([gateway registry](gateway-registry.md)). Every other
operation is refused. The list is one exhaustive match in `central-identity`
(`browser_operation`), so a new operation does not compile until it is decided
there. The cookie's token is used only by `SessionStatus`, `RenewSession`,
`Logout`, `TermsStatus` and `AcceptTerms`; the directory uses none, and the
page sends it with `credentials: "omit"`.

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

Other paths answer 404 and other methods 405. A body is at most 64 KiB (413).
A malformed or unknown request is `Rejected` (401), as on `/v1/native`; the
other answers keep the native statuses too (409 `Conflict`, 429 `Busy`, 503
`Unavailable`, 403 `TermsRequired`) with the same fixed text.

The cookie is
`__Host-central=<64 hex>; Path=/; Secure; HttpOnly; SameSite=Strict;
Max-Age=1800`. The session is the native session with `kind = browser`. Only
exactly one well-formed `__Host-central` counts; anything else in `Cookie` is
no session. `Logout` clears the cookie (`Max-Age=0`) in every answer, also when
the session was already gone.

Per-address budgets read the last `X-Forwarded-For` hop, and only from the
loopback proxy. Without a well-formed last hop a request is refused (400).
IPv6 addresses count by their /64; an IPv4 address mapped into IPv6 is that
IPv4 address. Each budget remembers at most 4096 addresses, in memory only, and
forgets one when its window has passed; when full, the oldest window makes
room, so a crowd of addresses cannot lock new ones out.

| Operation | Budget per address |
| --- | --- |
| `Register` | 5 an hour |
| `Discover` | 30 a minute |
| `LoginStart` | 30 a minute, apart from `Discover` |
| `GatewayDirectory` | 60 a minute, apart from the others |

The global budgets of the native surface apply as well. The native surface
(`/v1/native`) is unchanged, except that it refuses `Register` and a browser
session.

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
- `central_origin()`, the Central the module is bound to (the live one)

*Changed 21.10.2026 while building it:* `login_finish` first took
`wrapped_master` too, but that only arrives in the answer to the KE3
`login_finish` makes, so the check is a step of its own. The states are
exported objects rather than fields of plain objects, which would have needed
`js-sys` as a direct dependency; a step that takes a state empties its
JavaScript object, and an error is `rejected` or `unavailable`.

A state is opaque, consumed by the step that takes it and wiped when dropped,
also when the page frees it unused. Argon2id (64 MiB) runs in a module
Worker, one per flow, terminated at the flow's end. The JS glue is made by
`wasm-bindgen` (CLI, the same version) and committed under
`web/src/central-wasm/` with its SHA-256 sums, so `npm run build` needs no
Rust toolchain; `cargo xtask wasm` rebuilds it on x86-64 Linux (other machines use the
pinned container), and `cargo xtask wasm --check` (a CI job) proves the
committed bytes are what the source builds ([xtask](xtask.md)).

## Web pages (`web/`)

`/register`, `/signin` and `/account` (shows the account's username, signs
out). The header gains "Sign in" and the account's name.

Registration has these fields:

- beta key
- username, with the name rules shown
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

*As built (21.10.2026, `web/README.md` §"Accounts"):*

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
  identity, username, expires_at }`. The page takes exactly these shapes;
  the name it shows is always Central's canonical one, held in memory.
- The page keeps only the session's end in localStorage
  (`baylee-central-session`), so after a reload it knows to ask
  `SessionStatus` whose session it is (until then the header says "Your
  account"), and forgets it on signing out, at its end or on Central's 401.
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
3. Beta keys are issued with the new mode.
4. The official gateway gets `BAYLEE_CENTRAL_CONFIG_FILE`
   (`{"endpoint": "live", "bridge": …}` from Central's `secrets.json`), is
   restarted, and its `/auth/config` says `central`.
5. The legal texts have to be published as a real version first (the owner's
   review). Today both of Central's documents are `draft`, which is never in
   force.

## Not in this round

- password change, reset (by recovery code, e-mail or administrator)
- e-mail, 2FA, profile picture, display name on Central
- export and self-service deletion: by request to the operator until M1
- the administration UI (M2)
- registration in the native client (M6)
- deep links (M4/M6)
