# Gateway registry and delegated sessions

The native API separates four authorities: a central account session, a gateway's
SSO credential, its heartbeat credential, and the official migration bridge credential.
Credentials are independent random capabilities; PostgreSQL stores only their SHA-256
digests. A session's bearer is a signed token ([below](#session-tokens)). A gateway never
receives a user's central bearer or OPAQUE password material.

## Registration and operator approval

`RegisterGateway { origin, metadata }` requires a restored central account
session. The response contains a PostgreSQL-generated UUIDv7, a redemption
`credential`, and a separate `heartbeat` token. Store both tokens privately.
Registration reserves the origin but does **not** authorize SSO or publish the
gateway. Each owner can reserve at most eight origins. The official gateway's
origins (the bridge's: `OFFICIAL_ORIGIN` and its aliases in a live service,
`central_api::is_official_origin`) are never registered: `RegisterGateway` for
one answers `Conflict`, since a registered credential for that audience could
read the official gateway's revocations. Before the first approval,
the same authenticated owner may repeat registration to recover a lost response;
this rotates both pending credentials. An approved or disabled origin cannot be
rotated this way, nor can another account claim an existing origin.

Live origins must be canonical HTTPS origins without credentials, path, query or
fragment. Isolated development instead accepts explicit numeric loopback HTTP
origins with a nonzero port. Metadata has bounded name/version fields and a
canonical website URL on the same origin. Metadata is untrusted display text:
render it as text, never HTML. Software versions are self-reported.

A person must independently verify control of the claimed origin and the
gateway operator's identity before approving it. Since 30.10.2026 Central's
administrators approve, disable and enable gateways on the website
([administration](administration.md) §"Gateways"), which writes the
`gateway_approvals` row through the runtime with the grants that decision
needs; the operator's SQL below stays the fallback. It is a deliberate
administrative action, not an automatic network probe. Coordinate the approval transaction by
locking the relevant `gateways` row `FOR UPDATE`, verifying the current ownership
record, and inserting approval before commit. Disabling approval immediately
hides the entry, rejects credential authentication and new admission, and ends the
gateway's sessions. Never approve solely because registration succeeded. The service
probes nothing at registration. It calls a gateway only to push a revocation, only once
it is approved, and only under the rules of [the push](#revocations): a person's
approval of the origin is what makes that call safe to make.


The runtime may use these registry privileges without grant options:

```sql
GRANT SELECT, INSERT, UPDATE ON central_identity.gateways TO central_runtime;
GRANT SELECT, INSERT, UPDATE (enabled) ON central_identity.gateway_approvals TO central_runtime;
GRANT SELECT, INSERT, UPDATE (revoked_at, last_used_at) ON central_identity.gateway_tokens TO central_runtime;
GRANT SELECT, INSERT, DELETE ON central_identity.gateway_admins TO central_runtime;
GRANT SELECT, INSERT, UPDATE ON central_identity.gateway_streams TO central_runtime;
GRANT SELECT, INSERT, DELETE ON central_identity.gateway_sids,
    central_identity.gateway_revocations TO central_runtime;
```

Existing native account and session permissions remain necessary. The runtime
also requires the [private object and archive grants](private-objects.md).
The reusable SDK exposes `gateway_directory`, `Session::register_gateway` and
`Session::set_gateway_admins`; it validates returned IDs and public metadata.
The runtime
must not own these objects or inherit operator privileges. Startup rejects any
privilege on the approval table beyond `SELECT`, `INSERT` and `UPDATE (enabled)`
(no `DELETE`, no other column, no grant option), including inherited effective
privileges. Credential-pair rotation and revocation stay an administrative DB
operation; gateway tokens are revoked on the website.
The current executable remains an isolated development service, not a reviewed
production deployment.

## Gateway tokens

*30.10.2026, migration 000014.* A token is the credential a self-hosted
gateway pastes into its admin console (Baylee `docs/central-login.md`
§"Linking a gateway with a token"); the owner's request on the website, in the
owner's words: tokens to host one's own gateway, with which it connects to
Central for its players' sign-ins and its entry in the gateway list.

- **Registration on the website.** `Owner { AddGateway { origin, name } }`
  (browser listener only, the cookie's session, Central's terms accepted)
  reserves the origin for the account, as `RegisterGateway` does, but without
  a credential pair: the row authenticates by tokens alone. An origin
  registered already is a `Conflict` (no rotation as with
  `RegisterGateway`); the official gateway's origins are refused; eight per
  account, shared with `RegisterGateway`; the account's move must be
  finished. Its tokens sign players in at once; the row waits for an
  administrator's approval for the directory only.
- **A token** is a [`Token`](../crates/central-api/src/token.rs): 32 random
  bytes; Central stores its SHA-256 digest in `gateway_tokens` and shows the
  token once, in the answer to `Owner { CreateToken }`, whose text form is
  `bgt_` and 64 lowercase hex digits (`central_api::owner::token_text`; the
  gateway reads it back with `parse_token_text`). It has
  - a **name**, the owner's label (1 to 64 bytes, no control characters and
    none of the invisible characters that change how text reads, such as a
    direction override; `central_api::gateway::shown_char`, the rule of a
    gateway's own name as well);
  - **scopes**: `sso` (read its revocations, name its invalidate path, publish its
    terms) and `listing` (its directory heartbeat), at least one;
  - an **expiry**, 1 to 366 days (the table's CHECK holds both ends);
  - a **revocation**, by its owner or an administrator, at once.
- **Minting** needs a password sign-in less than 30 minutes old, as an
  administrator's change does: a stolen cookie mints nothing after that.
  A gateway holds at most eight live tokens, and is given at most 32 a day.
- **Authentication.** `authenticate_gateway` (every SSO request) accepts the
  credential pair's redemption credential or a live token with `sso`;
  `gateway_heartbeat` the heartbeat credential or a live token with
  `listing`. **Approval is the directory's, not admission's** (owner,
  30.10.2026): a token authenticates while its gateway is pending or
  approved, since it names an owner who answers for the gateway and can be
  revoked at once; a credential pair, which its owner cannot revoke, still
  needs the approval. A disabled gateway's token authenticates nothing, and
  a revoked or expired one fails at once: the check is in the
  authenticating query, not a cache. A token for a player follows the same rule
  (not disabled). Revoking one gateway token ends no player's session; disabling
  the gateway does.
- **Status.** `GatewayStatus`, under a live token whatever the approval,
  answers the gateway's id, origin, metadata and state, and the token's
  scopes and expiry, so a console can say "waits for approval" before it is
  listed. It is refused on the browser listener.
- **Last use.** A heartbeat and a status check mark the token used, at most once a
  minute.
- **Owner's view.** `Owner { Gateways }` lists the account's gateways with
  their state, liveness and tokens (live ones, and those revoked or expired
  in the last 30 days), never a token or digest.

## Discovery, heartbeats and administrators

`GatewayDirectory { after }` is public and returns up to 100 approved gateways (a
pending one beats, so its owner sees it seen, but is not listed) in
UUID order. Pass the last returned UUID as `after` for the next page. Each entry
contains origin, public metadata and `online`. Online means Central accepted a
heartbeat within 90 seconds; it is not an independent availability guarantee.
It is served on the native route and on the website's browser listener, where it
uses no session and has a per-address budget of 60 a minute
([web accounts](web-accounts.md)); `baylee.app/gateways` reads it there.
`GatewayHeartbeat { metadata }` requires only the heartbeat token, is limited to
one update per five seconds, and cannot alter owner, origin, administrator list,
approval, or SSO credentials.

**The official gateway** is the bridge's, and the directory lists it
like any other approved gateway (the website marks it and shows it first). Its
row (migration 000009) has `official = true`, no owner and no credential: a
CHECK pairs `official` with a NULL `owner_id`, `credential_digest` and
`heartbeat_digest`, and a partial unique index allows one such row. No
registered credential therefore authenticates it, and the bridge stays the only
authority for its origin. The operator lists and approves it under the migrator
login ([operations](operations.md#the-official-gateway)); the runtime role could
insert such a row but, as for every gateway, not approve it.
`GatewayHeartbeat { metadata }` with the bridge credential updates that row and
no other, under the same five-second limit, after `metadata.validate` against
one of the bridge's origins. The row is the official gateway only while its
origin is one of the bridge's: `OFFICIAL_ORIGIN` or an alias in a live service
(the store refuses any other there), the configured development gateway and its
`gateway_aliases` in a development one. Without a listed, approved and enabled
row the bridge's heartbeat is `Rejected`; a disabled row is hidden like any
other.

**Its old address** (30.10.2026). The official gateway moved from
`https://baylee.acevik.de` (`LEGACY_ORIGIN`, which migrated account documents
keep naming) to `https://gateway.baylee.app` (`OFFICIAL_ORIGIN`). The old address
is a compiled alias (`OFFICIAL_ALIASES`): `GatewayToken` for any official origin is the
official gateway's, with the origin the client asked for as its `aud`, and revocations
have one stream under the canonical origin; `GatewayTerms` for an alias answers the
canonical origin's publication, and an acceptance sent for an alias is kept under the
canonical one, while one kept under the alias before the move still counts. Dropping the
alias is a release.

`SetGatewayAdmins { gateway, expected_revision, accounts }` requires the owner's
central session and replaces at most 32 explicit administrator UUIDs with a
revision compare-and-swap (initial revision 1). All listed accounts must be
enrolled. Registration starts with an empty list. Ownership permits changing this
list but grants no implicit backend access: the owner must also be explicitly
listed and loses backend access upon removal. Heartbeats never carry administrator
claims. A token for the `admin` purpose needs current membership, and a removed member's
`admin` `sid` is revoked in the same transaction, while their `admit` `sid` stays.

## Session tokens

*6.11.2026, migration 000023.* A gateway checks a player by itself, locally and without
the network; Central's sign-outs still reach it at once. This replaces tickets and leases.
The wire contract with Baylee is the section below and `central_crypto::jwt`, which both
sides use, so that neither writes a second verifier.

**Three tokens.** All are compact JWS, Ed25519 only.

| Token | `typ` | Audience | Lives |
| --- | --- | --- | --- |
| Central token | `baylee-central+jwt` | Central alone | to the session's 12-hour cap; its row says whether it is live |
| Gateway token | `baylee-gateway+jwt` | exactly one gateway | an hour, never past that cap |
| Revocation message | `baylee-revocation+jwt` | exactly one gateway | a minute |

**The format.** A header is compared with the exact bytes `central_crypto::jwt` writes,
`{"alg":"EdDSA","typ":"<kind>","kid":"<id>"}`, so nothing in a token selects an algorithm or
a key, and an extra header field is a refusal by construction. The key comes from the
verifier's own key set, by `kid`, which is the key's RFC 7638 thumbprint. `iss`, `aud`, `exp`
and `nbf` are required and always checked against what the verifier is configured with, with
30 seconds of tolerance. `aud` is one string. Unknown and duplicate claims are refused, and
each kind has a longest size (2 KiB, and 16 KiB for a message) and a longest `exp - iat`.
Signatures verify with `verify_strict`.

**The Central token** never leaves the path between the client and Central. Its `sid` is the
session's random capability, of which Central keeps only a digest, so that the signing key
alone forges no session. That capability opens nothing bare either:
`central_identity::credential_operation` names, for every operation, whether it is a
gateway's or a client's, so a bare credential opens no client operation and a signed session
no gateway operation. Renewal changes nothing in the token. A website's session stays its
`__Host-central` cookie, which only Central ever reads.

**A gateway token.** `GatewayToken { gateway, purpose }`, under the session, answers
`GatewayToken { token, expires_at }`. It is refused for Central's terms not accepted, an
unfinished move, a disabled or unknown gateway, an `admin` purpose without current
membership. The official gateway has no Central administrators, and admits a moved account
once its move is closed. `purpose` is `admit` or `admin`. Other claims: `sub`, `sid`, `jti`,
`iat`, `nbf`, `exp`, and, while there is one, `name` and `tag` (a gateway learns the names of
the people who sign in there, nobody else's), `legacy_id` (the official gateway, closed moves
only) and `terms` (the newest of this gateway's terms the account accepted, which the gateway
compares with its own). Central's own terms are a condition of issue, so no claim says them.
Asking again refreshes: `exp` moves, `name`, `tag` and `terms` are read again, `sid` stays.

**`sid`.** Sixteen random bytes per session, gateway and purpose, in
`central_identity.gateway_sids`, at most 32 per session (`Busy`; a refresh is no new one). Two
gateways cannot tell they saw the same session. `exp` is capped by the 12-hour cap rather than
the session's 30-minute expiry, which would make every token half an hour: a session the client
stops renewing leaves its token valid until `exp`, an hour at most, and it cannot be refreshed.

**Keys.** `GET /.well-known/jwks.json` is public: RFC 8037 OKP keys, at most eight and 16 KiB,
`application/jwk-set+json`, `Cache-Control: public, max-age=300`, 256 reads a second in all,
then `429`. A gateway refreshes every five minutes and on an unknown `kid`, at most once a
minute, only from its configured Central URL over verified TLS, and keeps what it holds when a
read fails. Rotation is an operator's ([operations](operations.md#rotating-the-signing-key)).

## Revocations

Every end of a `sid` is a revocation, whichever statement ended it: the trigger on
`gateway_sids` numbers it, in the same transaction, in `gateway_revocations`. So sign-out, a
replacing sign-in, a password change, a reset, a deleted account, a removed administrator and a
disabled gateway all reach the gateway, and no path can forget. "One active session per
account" is the second of those: the replacing sign-in deletes the old session, and the gateway
that held it hears.

**The numbers** are per gateway, under its canonical origin, from 1, without holes: `last_seq`
moves in the transaction that ends the session and rolls back with it. A session's streams are
locked in one order before any is touched, so two ending sessions cannot deadlock. Central keeps
each number for two hours, by age alone, which is longer than any token lives, so a gateway
reading from 0 after a start finds every revocation still in force. The sweep removes a prefix of
each stream, so a clock step cannot leave a hole. Growth is bounded by the sign-in budget.

**The message** names only `sid`s that gateway was given, never a token and never why: `iss`,
`aud` (the canonical origin), `jti`, `iat`, `nbf`, `exp`, `from`, `seq` and `sids` (1 to 256, one
for each number from `from` to `seq`). It is signed afresh for each send, so a retry has a new
`jti` and the same numbers. A gateway applies its `sids` whatever the numbers, since a revocation
can only end sessions, takes each `jti` once (`central_crypto::jwt::ReplayCache`), and moves its
applied number only when `from <= applied + 1`.

**Catch-up** is what makes a revocation certain, and also what serves a gateway Central cannot
reach. `GatewayHeartbeat` answers `GatewayHeartbeat { last_revocation }`.
`GatewayRevocations { after }`, under the SSO credential or the bridge, answers `{ token, last,
floor }`: one fresh message for the next numbers, or none; the last number; the lowest held, or
`last + 1`. The stream is the credential's, never the request's. With `after + 1 < floor` a
gateway may have missed something it cannot learn, and ends every Central session it holds.

**The push** is what makes a revocation prompt. `RegisterInvalidate { path }`, under the SSO
credential or the bridge, is sent when a gateway links and at a sign-in until it has been
answered. It names a **path, never a URL** (`central_api::gateway::valid_invalidate_path`, held by
the table as well): at most 128 bytes of `A-Za-z0-9/_-`, so nothing in it can climb out of the
path or be decoded into something else. Scheme, host and port come only from the approved origin.
Only an approved, enabled gateway with a path is called; a pending gateway catches up. Central
sends `POST {origin}{path}`, the compact token as the body, `Content-Type: application/jwt`. `204`
is taken, `429` and `5xx` are retried, and anything else is refused and not asked again.

**The call** is `central-outbound`'s, Central's one way out, shared with Web Push: HTTPS only, no
proxy, no redirect, two seconds to connect and five in all, and only the status of the answer is
read. A name resolves once, and what it resolved to is what is dialled, so it cannot answer
differently the second time; one private, loopback, link-local, multicast, reserved or
documentation address among its answers refuses the whole name, however the address is spelled.
A development service runs no sender.

**One exception.** The official gateway runs on Central's machine, where its public name may not
lead back to it. `official_address` in the service's configuration names one loopback or private
address for that one compiled host, and nothing else: the URL, the name sent in TLS and the
certificate check stay what they were, so it is the same verified HTTPS to another address. Any
other name reaches no private address, with the setting or without it. A public, link-local or
multicast address is refused, and a value that cannot be read stops the service.

**The queue** is the log and one cursor per gateway, so it holds no more than the log. A claim
counts an attempt and sets the next, one second doubling to sixteen; a sender that dies mid-call
loses nothing. After five attempts, or a refusal, the numbers are left to catch-up and the gateway
rests a minute, so one that never answers costs a few calls a minute however many sessions end.
Four senders run at once, so one that hangs for its whole limit holds up one of them.

## A client too old

Every request to `/v1/native` and `/v1/reports*` carries `Baylee-Central-Protocol: 2`
(`central_api::PROTOCOL_HEADER`). Without exactly that the answer is `426 Upgrade Required` with
the body `{"refusal":{"code":"update_required"}}`, before the request is decoded and before its
credential, its budget or the database: an old client hears it at its first request, not as a
failure to decode the answer to a sign-in. It names no version, because Central does not know
client releases. `central_api::Error::UpdateRequired` is that answer in the SDK. The key set and
the browser listener need no header. The client's updater goes to GitHub Releases and needs no
sign-in, so Central is not on its path.

## One session per account

Exactly **one** central session of each kind is active per account, even on the same device. A
successfully verified new login atomically deletes the earlier session before inserting the new one
under an account row lock, which revokes its `sid`s. Concurrent successful logins leave the last
committed session active. Incorrect password proofs or stale credential revisions do not revoke the
current session. A session lasts 30 minutes and renews to 30 minutes from each renewal, at most 12
hours after its sign-in. Explicit logout revokes in the same way. Account ciphertext and device
metadata are not deleted by login replacement or logout.

## Verification

`central-crypto` tests every refusal of the format, each by its exact reason: wrong `aud` and
`iss`, expired and not yet valid at the edge of the tolerance, every other algorithm, a header
with anything more, an unknown `kid`, a key named by another `kid`, a changed payload or
signature, one kind read as another, unknown and duplicate claims, a replayed message, and the
RFC 8037 vectors. `central-outbound` tests the address rules, a name with one private answer,
and that a pin opens no other name. `central-api` unit tests cover canonical origin and metadata
restrictions and the invalidate path. The owned PostgreSQL fixture of `cargo xtask postgres` exercises pending
registration, credential separation, approval privilege rejection, directory
and heartbeat behavior, the official row (one at most, without owner or
credentials; the bridge's heartbeat moves it alone; no registered credential
reaches it; listed when approved, hidden when disabled; the upgrade over registered
gateways), owner isolation, administrator CAS and revocation, absence of foreign legacy IDs,
central single-session replacement including concurrent logins, `sid`s (stable, separate,
bounded), the numbering (every end of a session, no hole after a rollback, no deadlock between
two ending sessions, the sweep's prefix), the push queue (only approved gateways with a path,
bounded attempts, a sender dying mid-call, a page that aged out) and migration 000023 over live
rows. Over real HTTP it exercises the old client's refusal, a gateway token offered at Central
and at another gateway, a capability copied out of a token, a changed token, a token whose session
ended, each bearer on the other's side, catch-up and a push. No production accounts or credentials
are used. CI must also validate the shared Rust implementation
on Windows, Linux and macOS before release.

## Terms of use

A gateway publishes its terms with `PublishGatewayTerms` under its SSO credential (the bridge
for the official origin), and reads a player's acceptance from the token's `terms`. Published
versions are immutable. See [terms](terms.md).

