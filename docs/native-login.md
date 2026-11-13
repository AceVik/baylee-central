# Native login and legacy migration integration

Owner authorization: 2026-10-19. Live issuer: https://central.baylee.app.
Since the cutover (owner's decision of 30.10.2026) a legacy password goes to
Central alone, once, from the legacy store's sign-in (§"Legacy login"); no
gateway receives one any more. Development endpoints are numeric loopback
addresses, selected explicitly.

## Boundaries

The native client authenticates centrally before displaying saved gateways.
OPAQUE registration/login, a random client master key and encrypted personal
objects use central-crypto. The protocol/endpoint rules are reusable in
central-api. Central sessions never go to a game gateway. A native session's bearer is a signed
Central token, which only Central reads; admission uses a separate token for one gateway, an
hour long, which that gateway checks by itself ([gateway registry](gateway-registry.md#session-tokens)).
Every native request names its protocol, and a build too old to be served is told so at its first
request ([below](gateway-registry.md#a-client-too-old)).
Neither a central transport error nor a failed OPAQUE proof is permission to
send a plaintext password elsewhere. HTTP redirects must be disabled on both
the central and the legacy credential transports.

An enrolled account document is mandatory. Decryption failure, an unsupported
schema or an unavailable read leaves login incomplete; none initializes empty
settings or overwrites the previous copy. Compare-and-swap revisions prevent
concurrent writers from replacing newer data. AEAD binds account, issuer,
vault epoch, object and revision. A malicious server can still replay a complete
old snapshot to a fresh device without a trusted freshness checkpoint; this
limitation is not solved by encryption or an HTTP success code.

## Migration state machine

1. A live discovery response explicitly selects legacy enrollment. Unknown
   usernames and not-yet-enrolled usernames have the same response. This
   exposes enrollment state, not whether an unknown username exists on the
   gateway. Deployment-level rate limiting remains mandatory.
2. The fixed legacy gateway verifies the password afresh and checks bans/reset
   requirements. An ordinary pre-existing bearer session cannot authorize this
   step. Through a separately authenticated bridge it reserves a central UUIDv7
   and an expiring enrollment capability bound to the immutable legacy ID.
   Names/emails never merge accounts.
3. The client exports its legacy data, encrypts the new account document and
   enrolls atomically: credential, master-key wrapper and ciphertext together.
   A repeated reserve cannot replace a committed credential or data.
4. The client performs a fresh central OPAQUE login, downloads the committed
   ciphertext, authenticates/decrypts it and confirms restoration.
5. The gateway verifies durable restored state and the unchanged source/credential
   generation under its exclusive mutation barrier. It calls idempotent
   `LegacyClose` before disabling legacy login and revoking the old sessions. Original data
   is retained. Cutover is retried after lost responses, never assumed from a
   client-supplied success boolean.

An already linked legacy account also repeats this idempotent close step during
login repair, so an upgrade cannot leave its private-object storage permanently
blocked. Legacy password changes invalidate unfinished enrollment before changing
the source credential; archived ciphertext and advanced vault epochs prevent
stale proofs from becoming the current migrated credential. Closed accounts cannot
be reopened by the bridge. See [private objects](private-objects.md).

The two databases have no distributed transaction. Pending/enrolled/restored
states, immutable legacy links, source retention and authenticated reconciliation
are necessary to recover from interruption at every step. Enrollment challenges
can be restarted after process restart; committed credentials, documents and
session revocation must survive it. Secret storage never silently regenerates
OPAQUE deployment keys after a missing/corrupt file.

## Legacy login

*Added 30.10.2026 (owner's decision: Central takes the old accounts' sign-in
and their move over from the official gateway, which then has no account
management).* The official gateway's accounts that had not moved at the
cutover are in Central's **legacy store** until their move closes or the
store's deadline passes. Central verifies their old passwords itself: the one
place it ever sees a password, ended by the deadline.

**The store.** Schema `central_legacy` (migration 000015), closed to
`PUBLIC`, with a login role of its own (`central_legacy`, `ops/grants.sql`):
it reads the accounts, counts the lockout (`failures`, `locked_until`, the only
columns it may change) and deletes a row when its move closes; it has no usage
of `central_identity`. The runtime role has no usage of `central_legacy` and no
privilege on anything in it. Each store checks its role at startup, the
negatives included (`legacy-role.sql`, `AccountStore::legacy_store_closed`),
and refuses to start otherwise. A row holds only what the move needs: the
legacy id, username key, display name (when it keeps the rule) and tag; the
Argon2id salt and output; the gateway's terms version, a forced password change
and a ban in force; the source snapshot (at most 192 KiB) and its SHA-256,
which a constraint keeps equal; the lockout's counters. No address, session or
Central-only account. The operator imports it once and sets its deadline under
the migrator login ([operations](operations.md#the-legacy-store)); the import
also reserves each account's Central identity under its name, so that nobody
registers the name before the account moves.

**The hash.** Only Argon2id, version 0x13, m = 19456 KiB, t = 2, p = 1, a
16-byte salt and a 32-byte output: the gateway's parameters (its argon2's
defaults). The gateway's export refuses any other hash, and the store keeps
salt and output as fixed-length bytes, so Central parses no PHC string and no
stored row chooses what a verification costs. `central-crypto`'s `legacy`
derives over a caller-owned work buffer it wipes, as the OPAQUE stretch does;
the comparison is constant-time (`subtle`).

**The sign-in.** `Discover` answers `Legacy` for every name it does not find
enrolled, unknown ones included, as before. `LegacyLogin { username, password }`
(native listener only; the browser listener refuses it) then:

1. spends the global request budget, then one of 30 legacy sign-ins a minute
   for all names and one of five a minute for this name (as typed,
   lowercased), and since 11.11.2026 one guess of its place's (§"Guessing a
   password online"; the proof gives it back); past any of them it is `Busy`
   and spends nothing;
2. reads the row for the canonical name, unless it is locked out or the
   deadline has passed;
3. derives on the worker pool, against the row's salt or against a fixed one
   when there is no usable row, under the same parameters, and compares in
   constant time; the password is dropped and wiped there;
4. on a failure counts one against the name (a statement that matches no row
   for an unknown or locked-out name): the tenth in a row locks the account out
   for an hour and starts the count again. `Rejected`;
5. on the proof starts the count again; a forced change or a ban in force is
   `Rejected`; otherwise it reserves the identity (`reserve_legacy`, keeping the
   name and the tag; `Conflict` once the account enrolled) and answers
   `LegacyEnrollment { identity, ticket, gateway, snapshot }`.

An unknown name, a closed move (its row is gone), a locked-out account, a
Central-only account (never in the store, its own password included) and a
wrong password are the same `Rejected` after the same derivation and
statement. Timing still differs by the success path's extra statements, which
follow only a proof; the database round trip of the failure statement is the
same for a row and for none.

**The move.** The client enrolls the snapshot, signs in through OPAQUE and
confirms its restored document as before. With Central's terms accepted
(`LegacyExport` and `LegacyLink` wait for them, as gateway tokens do),
`LegacyLink { digest }` closes it: the digest must be the stored snapshot's
(`Conflict` otherwise; `LegacyExport` hands the snapshot again), the account
must be restored, and then `legacy_closed` is set and the row deleted, in that
order through the two roles. Again after the close it is `Done` and deletes a
row whose deletion was lost; so does the daily sweep. A restored account whose
row the deadline removed still closes. A closed move is never reopened, and
`LegacyExport` refuses it.

**The bridge.** With the store configured, the bridge's `LegacyReserve`,
`LegacyInvalidate`, `LegacyClose`, `LegacyProfile` and `LegacyStatus` are
refused: Central is the source, and the gateway changes no legacy password
any more. A token for the official gateway is refused for a legacy account whose move has not closed, with
or without the store, so a `legacy_id` in a token means a closed move: the official gateway fixes
the old id's link to the Central identity at that first admission (Baylee `docs/central-login.md`
§"Legacy migration").

**The deadline.** The owner's date, the end of 30.06.2027 in
Europe/Berlin, set by migration 000015; the operator can move or clear it
(`docs/operations.md` §"The legacy store"). After it no row is handed out, sweep or not; the daily sweep then deletes every
row and every legacy reservation that never enrolled, which gives its name up.
An administrator lists what is left on the website's `/{lang}/account/admin/legacy`
(`AdminRequest::LegacyAccounts`: names,
tags, progress, failures, lockout, refusals, sizes; no hash, no snapshot).

## The bridge's reservation carries the name and tag

*Added 23.10.2026 (the accounts and administration design, step 3; [web
accounts](web-accounts.md) §"Display name and tag").* A moved account keeps
the display name and tag it had at the official gateway, so the bridge sends
both from the legacy row:

- `LegacyReserve { legacy_id, username, display_name, tag }`: both optional
  (`#[serde(default)]`, absent on the wire when `None`), so a bridge built
  before them still decodes and is still decoded. They are stored with the
  reservation, and a renewed reservation of a pending account refreshes them:
  the name it carries replaces the one held, and a tag, once held, never
  changes. A name that does not keep the rule (`profile::valid_display_name`)
  is `Rejected`; the bridge sends none for such a name (one written before the
  gateway's rule of 15.09.2026), and the account chooses a name on the
  website. Reserved names are kept as they are. A tag outside `1..=i32::MAX`
  is `Rejected`; one another account holds is replaced by a drawn one (the
  registered accounts' draw), and the name is kept.
- `LegacyProfile { legacy_id, display_name: Option<String>, tag: u32 }`
  (bridge only) → `Done`, for a legacy account reserved before Central kept
  them: it sets the tag and, when given, the name, once. `Conflict` once the
  account has a tag (and with it any name), so the bridge may send it at
  every sign-in of such an account and read `Conflict` as done; an unknown
  legacy account or a malformed name or tag is `Rejected`. The design's
  once-marker was the name; the tag is the one every moved account gets
  (see web accounts §"Display name and tag" for why). The browser listener
  refuses it.

Baylee's bridge sends both with every `LegacyReserve`, and `LegacyProfile`
at a moved account's Central sign-in (Baylee `docs/central-login.md`).

## Password change

*Added 28.10.2026 (roadmap M1, step 1).* A signed-in account changes its
password on either listener: the native client with its bearer, the website
with its cookie. The vault stays: the client wraps the **same** master key
under the new password, so the account document, the private objects, the
vault epoch and the recovery wrapper (bound to the epoch, not the password)
are untouched, and the recovery code still opens the key.

A session alone is not enough. Central cannot see whether a new wrapper
holds the same master key, so a stolen token able to change the password
would take the login and leave the owner's vault reachable only by the
recovery code. The change therefore carries a fresh OPAQUE proof of the
current password (ASVS 6.2.3, in OPAQUE's form), inside the exchange itself:

1. `ChangePasswordStart { login, registration }` with the session: KE1 under
   the current password and an OPAQUE registration request for the new one,
   both for the session's own account and epoch (there is no identity field;
   a client using another context fails at its own KE2 check). Central needs
   the session live, the account's move finished (registered, or the legacy
   move closed: while it is open the bridge's `LegacyInvalidate` is the reset
   authority) and Central's terms accepted. It charges the account's login
   budget (eight OPRF evaluations a minute, shared with `LoginStart`, and
   since 11.11.2026 the hours and days of §"Guessing a password online")
   twice:
   the registration response is an evaluation of the account's OPRF key as
   KE2 is, and with it a guessed password unmasks KE2's envelope, so one
   start can test two guesses. Charged double, a stolen session guesses no
   faster than anyone can without a session through `LoginStart`. It answers
   `PasswordChallenge { attempt, login: KE2, registration }`. The attempt
   lives 60 seconds, is bound to this purpose, this listener and the digest
   of the starting session's token, and shares the table of pending
   sign-ins (4096, more than the global budget can start in their 60
   seconds).
2. The client checks KE2 (a wrong current password fails here, before
   anything is sent), finishes the registration under the new password and
   wraps the master key it holds with the new export key's wrapping key.
3. `ChangePasswordFinish { attempt, proof: KE3, record, wrapped_master }` with
   the same session. The attempt is consumed first, then refused unless it
   is a change's (a sign-in's attempt is not, nor is a change's a sign-in's
   for `LoginFinish`), started on this listener by this session, unexpired,
   and its proof verifies. In one transaction Central then locks the account
   at the credential revision the start read, checks again after the lock
   that the session is live (a sign-in may have replaced it while the lock
   was awaited), stores the record and wrapper, raises `credential_revision`,
   deletes every session of the account (both kinds, each revoked at the gateways that
   held it) and inserts one new session of the caller's kind. The answer is
   the sign-in's: `Session { token, identity, wrapped_master }` natively,
   `BrowserSession` with the new `__Host-central` cookie on the website. The
   new session counts its 12 hours from the change, a fresh password proof.
   The same transaction reads the account's confirmed e-mail address, if it
   has one; it is told afterwards that the password was changed
   ([e-mail](email.md#notices-that-the-login-changed), since 12.11.2026).

Anything else is `Rejected` and changes nothing: no or a foreign session, a
session of the other kind, an unfinished move, a record or wrapper of the
wrong length, a credential changed since the start (another change), a
session replaced meanwhile. A sign-in begun before the change and finished
after it is refused by `issue_session`'s revision check. A change's start
that does not fit the minute's budget (two units of eight) is `Busy` and
spends nothing.

**A lost answer.** The commit ends the calling session, so `SessionStatus`
with the old token is the authenticated read: still live means nothing
changed and a retry is safe; `Rejected` means the change (or another sign-in)
ended it, and the client signs in again, with the new password first.
`central-client`'s `Session::change_password(transport, current, new)` does
this check itself and refuses a new password shorter than eight characters
(`MIN_PASSWORD_CHARS`, the website's rule) before anything is sent; on
success the `Session` holds the new token and its master key and document
stay.

**Accepted limits.** The client is trusted with its own vault's continuity:
a modified client can wrap another key and lose its own data, nothing of
anyone else's. A device signed in elsewhere learns of the change when its
session is refused; no mail tells of a changed password (the mails
Central sends are in [e-mail](email.md)). The native client's form is M6.

**The website's form** (12.11.2026; `crates/central-web`,
`islands/password.rs`, in the account area's security section). A browser
keeps no master key after a sign-in, so the form signs in once more with the
current password inside one Worker (`LoginStart`, `LoginFinish`, which
replaces the browser session as every sign-in does) and changes in the same
Worker with `central-web-crypto`'s `password_start` and `password_finish`,
which do what `Session::change_password` does: the wrapper the sign-in
answered is opened and the same master key wrapped under the new password.
Nothing on the wire changed. What follows from the two steps:

- A change through the page is three guesses where the client's is two (a
  sign-in's start and the change's); the right password gives them back.
  A wrong current password fails at the sign-in's KE2, in the Worker,
  before `ChangePasswordStart` is sent, and counts one.
- The page cannot read its cookie, so it has no `SessionStatus` with the old
  token. When the finish's answer is lost it asks `AccountSummary`: refused
  means the session ended, and the page forgets it; answered means either
  that nothing changed or that the new cookie arrived without its answer.
  The page says in both cases that it cannot tell, and to try the new
  password first at the next sign-in.
- After a change the page says what Central did: this browser stays signed
  in and every other session of the account, the client's included, is
  ended.

## Guessing a password online

*Added 11.11.2026.* A sign-in's start hands out KE2, and with KE2 a guessed
password is checked on the guesser's own machine. So each `LoginStart` is one
guess, whether a finish follows or not; a `ChangePasswordStart` is two
(above) and a `DeleteAccountStart` one. Central cannot tell a typing mistake
from a guess: it counts starts, and gives a start back when its proof
verifies. The counts are `central-identity`'s (`guesses.rs`, `limits.rs`),
in memory only, never written down or logged; every refusal is the `Busy`
the client and the page already know (429 with the route's fixed text), and
a start that a count refuses spends nothing. No request or response changed.

A **place** is the client address as the listener counts it: an IPv4
address, or an IPv6 address's /64.

| Counted | An hour | A day | Besides |
| --- | --- | --- | --- |
| One account, from every place that is not its own | 30 | 100 | 8 a minute, as before |
| One account at one place | 10 | 30 | |
| One place, at all accounts it is not an own place of | 30 | 100 | |
| One IPv6 /48, over its /64s | 120 | 400 | |

- **A proof gives the guess back.** A start whose proof verifies gets its
  hour and day back wherever they were counted; the minute stays, since the
  evaluation happened. Signing in with the right password spends nothing,
  however often.
- **An account's own places.** A place an account signed in from, or set
  its password from, is its own for 30 days, at most eight for one
  account; a ninth takes the place of the one that would end
  first. At an own place a start draws on that account's count at that
  place and on nothing else. So what everyone else guesses at an account
  does not keep its holder out where the holder signs in, and the sign-ins
  of a household or an office do not spend their address's count. Another
  password ends them: after a change or a reset only the place it was made
  from is the account's own, so whoever knew the old password guesses at
  the new one as a stranger does, and a deletion leaves none. A place
  becomes an account's own only when the store has issued the session or
  stored the new password: a sign-in begun before a change and finished
  after it proves the old password, gets its guess back and no place.
- **Ids that are nobody's** (made up, or an account without a credential
  for the epoch asked) are answered as before, with OPAQUE's stand-in
  record after the same work. All of them together count as one account (8
  a minute, 30 an hour, 100 a day), besides the place's own counts, and
  take no entry among the accounts.
- **A reset's start** needs a valid reset code before anything is
  evaluated. It spends one of the account's eight evaluations a minute and
  nothing of its hour or day, so guessers who spent those do not keep a
  holder with a code from the reset ([account reset](account-reset.md)).
- **The legacy sign-in** keeps its own budgets and lockout (above) and
  counts one guess at its place as well, given back by the proof.
- **Order.** A start whose place's counts or whose account's are spent is
  refused before a worker is taken or the store is asked. Whether an id is
  nobody's shows only once the store was asked: a start at a made-up id
  that nobody's count alone refuses has taken a worker and one statement
  first, as many of them as its address's 30 starts a minute and the
  shared 32 a second let through (40 with the 8 kept beside them since
  12.11.2026). The worker is taken before a start is
  counted and before a pending attempt is consumed, so a busy moment
  spends no guess and loses no right proof.
- **Bounds.** Four tables of 16,384 entries each: accounts, accounts at
  places, places, and accounts with places of their own. A full one forgets
  what has passed, then the entry that spent least of its day (the oldest
  of those), so filling a table with single guesses never makes it forget
  the account guessed at most; the table of own places gives up the account
  whose last proof is oldest. None refuses a newcomer. Once more than
  16,384 accounts sign in within 30 days, the least recent lose their own
  places early: the number must grow with the service. (Before 11.11.2026
  one table of 1,024 held whatever id a start presented and refused
  newcomers when full, so about 17 starts a second with made-up ids kept
  every real sign-in at `Busy`.)
- A restart forgets all of it, the own places too.

**Requests per address.** Besides the guesses, each address has a budget
for the operations a guesser or a collector of names repeats. The browser
listener's are in [web accounts](web-accounts.md); the native listener had
none before 11.11.2026 (`central-http`'s `addresses.rs`):

| Native operation | Budget per address |
| --- | --- |
| `Discover` | 30 a minute |
| `LoginStart` | 30 a minute, apart from `Discover` |
| `LegacyLogin` | 10 a minute |
| `ResetStart` | 10 an hour |
| `GatewayDirectory` and `PublicFigures` | 60 a minute for the two together, as on the browser listener (since the review of 11.11.2026; before, only the place's 600 a minute) |

On both listeners every request that needs no session (the ones that draw
on the 32 a second all clients share, a report's two routes included)
first draws on its place's share, 600 a minute, and the client's pairing
requests (`PairClaim`, `PairPoll`, `PairRedeem`) on 240 a minute of the
64 a second they share. One address can therefore no longer spend what
everyone signs in with; four that keep it up together still can. An IPv6
address counts by its /64 here and nothing wider: a subscriber whose
provider hands out a /56 holds 256 of them and is those four alone.

*Added 12.11.2026.* When the 32 are spent, a place that was served fewer
than 30 such requests in its minute draws on 8 a second kept beside them
(`limits.rs`, `RESERVE_PER_SECOND`, `LIGHT_PER_MINUTE`): six sign-ins of
five requests each. So a household still signs in while four addresses
spend the 32. How little a place asked is counted by the /48 around an
IPv6 /64, as the guesses are, so one subscriber's many /64s draw these as
one place. What it draws of the 8 counts toward its 30. A request the
listener knows no place of never draws on them. Sixteen places that each
ask thirty a minute spend the 8 as well: many addresses still win. The
most requests a second without a session is now 40, and the tables that
are sized by it are sized by 40.

**The native listener's address** is the last `X-Forwarded-For` hop of a
request that comes from loopback, which the proxy sets, or the peer itself
if it is not loopback. A loopback request without a well-formed hop has no
place: it meets each account's own counts (30 an hour, 100 a day) and the
shared budgets, and no place's. That is deliberate. A proxy that sent no
header would otherwise make every client one address, and one address's
count would keep everyone out. The browser listener refuses such a request
(400), as before. Nothing but these counts follows from an address: no
identity, no credential, and no log line.

**What it bounds.**

- Everyone together guesses at one account at most 100 times a day from
  places that are not its own (36,500 a year), and whoever shares one of
  its own places 30 times a day there (at most eight such places). Before:
  8 a minute, 11,520 a day.
- One address guesses at all accounts together at most 30 times an hour and
  100 a day, the accounts it signs in to itself left out. Before: 30 starts
  a minute on the website (43,200 a day), and on the native listener as
  many as the shared budget let through.
- It does not make a weak password safe. A password among the hundred most
  common is still found within a day; the page asks for eight characters
  and nothing else.

**What it costs: keeping someone out.** Whoever knows an account's id
(`Discover` gives it for a name) can spend the account's count: 30 starts in
an hour from three addresses, or 100 in a day from four, and until that
window ends the account's password sign-in is `Busy` at every place that is
not its own. Before, eight requests a minute from one address did the same
everywhere, the holder's own devices included. Now the holder's own places
are served: wherever the account proved its password in the last 30 days,
since the last restart. After a restart no place is anyone's own until the
first proof from it. Someone at one of the holder's own addresses (a
household, a campus, a carrier's shared address) can still keep the holder
out there, with 10 starts an hour or 30 a day. Sessions that exist go on,
and pairing a client from a signed-in website needs no password
([web accounts](web-accounts.md)).

**What it tells.** Whoever sends from an address can find out whether a
given account proved its password from that address in the last 30 days,
since the last restart: once the address's hour is spent (30 unfinished
starts, which anyone there can make), or the account's, a start at that
account from there is answered only if the place is the account's own, and
refused otherwise. It takes the account's id (`Discover` gives it for a
name) and the same IPv4 address or IPv6 /64: a household, an office, a
campus, a carrier's shared address, a VPN's exit. It tells nothing about
any other address and nothing to anyone elsewhere. It is the price of
serving the holder where strangers are refused, as long as a place is an
address; the device token below would end it.

**The released client** asks a `Busy` request again three times within
about a second (never a legacy sign-in) and then shows that the service is
busy; it cannot say for how long, and the answer does not tell it (no
`Retry-After` yet).

**What a later client version would need** (not built, since a released
client cannot do either):

- A proof of work for sign-in, as registration has on the website: a
  challenge request before `LoginStart` and the solution inside it, required
  only once an account or a place has spent part of its count. It needs a
  new request and a new field, so a protocol version the client announces
  and an answer that sends old clients to update. The session-token branch
  (`c42/session-tokens`) adds both, a `Baylee-Central-Protocol` header and
  status 426; neither is on this branch.
- A device token (OWASP's "device cookie"): issued after a proof, kept by
  the client beside its session and presented with `LoginStart`, so that a
  known device has its own count and neither a shared address nor a
  spent account count keeps it out, and an own place survives a restart
  and a change of address. On the website it could be a second `__Host-`
  cookie today, without any change to the client.

## A bearer nobody issued

*Added 11.11.2026.* A request that needs a session comes with a token: the
native client's bearer, the website's cookie, a gateway's credential.
Central looks the token up when it serves the request, not before, so the
budgets cannot know whether a token is anyone's when they count its
request. Until 11.11.2026 they counted every request with a token as a
session's: against the windows all sessions share (512 control and 256 data
requests a second) and a table of 1,024 bearers a second. One address
sending made-up tokens spent all three, and every signed-in client and
every gateway was answered `Busy` for as long as it went on. What follows
changed no request and no response; every refusal is the `Busy` the client
and the page already know.

**Vouched for.** `central-identity` (`limits.rs`) keeps, in memory only,
never written down or logged, the digests of the bearers an answer has
vouched for:

- a session that a sign-in, a password change or a pairing has just issued;
- a token with which one of these requests was answered, each of which the
  store refuses for a token that is no live session or credential:
  `SessionStatus`, `RenewSession`, `AccountSummary`, `TermsStatus`,
  `Social`, `Notifications`, the object and document requests,
  `GatewayToken`, `RegisterInvalidate`, `GatewayRevocations`,
  `GatewayHeartbeat`, `GatewayPulse`, `GatewayStatus` (a request that is
  not on this list vouches for nothing);
- the official gateway's bridge credential, always.

A bearer stays vouched for for 30 minutes after the last such answer (a
session lives 30 minutes between two renewals, and the client renews every
ten), at most 16,384 at once; one more takes the place of the one whose
last such answer is the oldest. Four `Rejected` answers in a row take it
back, and the session's end does at once: `Logout`, a password change's
finish, a deletion's finish, and `PairCancel`, which is answered alike for
any token. Being vouched for is a count's input and nothing else: what a
token may do is the store's to decide at every request, as before.

**Signed bearers** (*since migration 000024*). On the account listener a
native session's bearer is a signed Central token
([gateway registry](gateway-registry.md#session-tokens)). It is verified
before these budgets count anything, under a budget of its own: 1,024
signature checks a second for all native requests together
(`SIGNATURES_PER_SECOND`), of which one place (an IPv4 address, an IPv6
/48) draws a quarter at most (`PLACE_SIGNATURES_PER_SECOND`). A token
Central issued or verified before is not checked again while its `exp`
is ahead (`verified.rs`, since 13.11.2026). A token that does not verify is refused there,
and a capability presented bare is refused for a client's operation
(`credential_operation`), both before these budgets. What reaches them as
a session's bearer is the capability inside a token that verified, and
that is what is vouched for: a sign-in, a password change or a pairing
vouches for it where it is issued. So on that listener the "any other
bearer" below is mostly a token whose session ended, and a gateway's bare
credential.

**Counted.**

| A request that needs a session, with | draws on |
| --- | --- |
| a bearer that is vouched for | its own 128 control and 64 data requests a second, looked at first; then the 256 and 128 that all the bearers of its address draw together (since 12.11.2026); then the 512 and 256 all such bearers share |
| any other bearer | 64 a second for all of them together, 600 a minute for one place |
| no bearer | the budget of those who are signing in, as before |

So made-up tokens spend nothing of what sessions share, take no room among
their bearers, and nothing of the sign-ins' budget. A session whose bearer
is not vouched for is served like any other; its first answer vouches for
it. That is every session's first request after Central restarts, and a
session's first after half an hour without one.

**One address, and one account's gateways** (*added 12.11.2026*). Four
vouched-for bearers that each kept up their own budget filled a shared
window (4 x 128, 4 x 64), from one address: two accounts have four
sessions between them, and one account alone has up to eight gateways with
eight tokens each, every one a bearer. Two things changed, and no request
or response did:

- *An address's share.* All the vouched-for bearers of one place draw half
  of each shared window between them, 256 control and 128 data requests a
  second (`PLACE_CONTROL`, `PLACE_DATA`), looked at after the bearer's own
  budget and before the shared window, and nothing is spent when it is
  used up. Half and not a quarter: two owners' gateways behind one address
  each keep their whole budget, and a household's clients ask a small part
  of it. The place is
  the IPv4 address or the IPv6 /64. A request the listener knows no place
  of (a process on Central's own host, the official gateway if it is
  pointed at the listener) has no such share and meets the other two
  budgets, so a full address never refuses it.
- *One account's gateways are one bearer.* When an answer first vouches
  for a bearer, the store says whose gateways it is a credential, a
  heartbeat token or a link token of (`bearer_owner`, one statement by
  three unique digests). All such bearers of one account are counted in
  one budget of 128 and 64 a second. A session is counted by itself, and
  so is the official gateway's bridge. If the store cannot say, the
  bearer is not vouched for yet and the next answer asks again.

**What stays** (for the owner; the security records of 11.11.2026 and
12.11.2026 have the options):

- Two addresses with two bearers each still fill a shared window (2 x 2 x
  128), and one account has two sessions. So two accounts at two addresses
  can still keep the other sessions at `Busy`; it took one address before,
  and nothing at all before 11.11.2026. How much wider a shared window may
  be is a question of what the store's four connections serve.
- One account's two sessions at their full budgets use up their address's
  half. Everyone else behind that address is `Busy` meanwhile: a
  household, and also a campus or a carrier's shared address.
- All of one account's gateways share one bearer's budget. An owner of
  several busy gateways is held to 128 and 64 a second for all of them.
- The store is asked once more for each bearer an answer first vouches
  for, and again after it was forgotten (half an hour without an answer,
  a restart).
- Seven addresses that each keep up their 600 a minute fill the strangers'
  second all the time; one alone fills it for nine or ten seconds of each
  minute. A session that is not vouched for then waits: after a restart,
  after half an hour without a request, or after four refusals in a row
  (every refusal counts: no request carries a player's token to Central
  any more, so no player's bad token makes a gateway a stranger).
- Made-up signed tokens spend nothing of these budgets, but each costs one
  of the 1,024 signature checks a second, before anything of the session
  is known. Since 13.11.2026 two things keep that from reaching sessions
  (until then one address could keep every native session at `Busy`):
  - **Remembered tokens.** A token Central signed is remembered as it is
    issued, and one that verified only while its session is vouched for
    (above: an answer of the store or its issue within the half hour) and
    at most 60 a minute from one place: by the SHA-256 of its whole text,
    with its capability, until its `exp`, for at most 16,384 tokens, the
    one used longest ago giving way. Such a token is not checked again. A
    made-up token is never remembered, so a flood, from however many
    addresses, spends only the checks; the token of a session that was
    replaced still verifies, but enters only while vouched for, so what
    turns the table over is sign-ins, at the sign-in's budget. What the
    table says is only what verification said: whether the session is
    live is the store's to say at every request, so one that ended is
    refused as before. The key set is read at the start and never changes
    while the process runs, and the table goes with the process.
  - **A share by place.** One place draws at most a quarter of the
    second's checks, counted by its IPv4 address or its IPv6 /48, before
    all places' second; a place refused at its share spends nothing of
    the others'. A request without a place meets the shared second alone.

  What stays: four places, or more, can still spend every check of a
  second while they keep it up. Then a token that is not remembered waits
  with them: a session's first request after a restart, or a session that
  was quiet while sign-ins elsewhere turned the table over (16,384 of
  them; at 30 sign-in starts a minute from one address that takes many
  addresses or hours). The remembered sessions go on. Sign-in, the
  website and gateways' credentials are not touched. The table holds the
  session capabilities themselves, beside the signing key, for as long as
  it remembers them; a memory image of the process gives both.
- A request without a proxy's address has no place and no share: it meets
  the 64 a second alone, like the other budgets by address. Since
  12.11.2026 the deployed account listener counts such requests and says
  so in its log, without anything of the request
  ([operations](operations.md#sign-in-guesses-and-address-budgets), "The
  proxy must send the client's address"). They are still served.
- The list lives in memory; a restart forgets it.

## Implementation status

This document specifies the integration in progress. It does not claim that
client UI, the legacy gateway bridge or production deployment are complete.
The typed API, bounded HTTP library integration, account persistence and native
SDK have now passed synthetic real-HTTP/PostgreSQL restore and failure tests.
Live discovery reveals central enrollment state for a supplied private username;
it is not a claim of account-enumeration resistance. This requires further review
before enabling the live migration discovery endpoint. Development does not
offer legacy discovery and cannot send a password to the live gateway.
The existing release gates and synthetic-data-only testing rules still apply.

## Development process

`central-service --dev-accounts-stdin` starts the real account protocol on an
explicit numeric loopback address. It takes one bounded JSON configuration from
standard input: `endpoint`, `gateway`, `database_port`, `database_user`,
`database_name`, `database_password`, `opaque_setup`, `bridge` and `token_keys`. The middle
two are byte arrays, the persisted versioned server setup and a random 32-byte bridge credential;
`token_keys` is as in [operations](operations.md#rotating-the-signing-key). An optional `browser` (`listen`, `origin`) starts the website's
browser listener for a page served on a loopback origin ([web
accounts](web-accounts.md) §"Local runs"). An optional `mail` (`endpoint`,
`site`, `token`) names a mailer on loopback, with a numeric loopback HTTP
origin as the site ([e-mail](email.md#configuration)); without it the
service sends no mail. Keep this configuration outside source control with owner-only file
permissions; never pass credentials on the command line. A restart must reuse
the setup. The service does not create keys or run migrations implicitly.
This command only accepts loopback development endpoints and the restricted
development database connector. The deployed `--accounts-stdin` mode, with the
live endpoint and the verified-TLS connector, is described in
[operations](operations.md).
