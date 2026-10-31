# Gateway registry and delegated sessions

The native API separates four authorities: a central account session, a gateway's
redemption credential, its heartbeat credential, and the official migration
bridge credential. Tokens are independent random capabilities; PostgreSQL stores
only their SHA-256 digests. A gateway never receives a user's central bearer or
OPAQUE password material.

## Registration and operator approval

`RegisterGateway { origin, metadata }` requires a restored central account
session. The response contains a PostgreSQL-generated UUIDv7, a redemption
`credential`, and a separate `heartbeat` token. Store both tokens privately.
Registration reserves the origin but does **not** authorize SSO or publish the
gateway. Each owner can reserve at most eight origins. The official gateway's
origins (the bridge's: `OFFICIAL_ORIGIN` and its aliases in a live service,
`central_api::is_official_origin`) are never registered: `RegisterGateway` for
one answers `Conflict`, since a registered credential for that audience could
redeem the bridge's tickets a second time. Before the first approval,
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
hides the entry and rejects credential authentication, new admission and lease
introspection. Never approve solely because registration succeeded. The service
does not fetch registered URLs, follow redirects, or accept callback challenges
that could create SSRF.

The runtime may use these registry privileges without grant options:

```sql
GRANT SELECT, INSERT, UPDATE ON central_identity.gateways TO central_runtime;
GRANT SELECT, INSERT, UPDATE (enabled) ON central_identity.gateway_approvals TO central_runtime;
GRANT SELECT, INSERT, UPDATE (revoked_at, last_used_at) ON central_identity.gateway_tokens TO central_runtime;
GRANT SELECT, INSERT, DELETE ON central_identity.gateway_admins,
    central_identity.gateway_leases TO central_runtime;
```

Existing native account/session/ticket permissions remain necessary. The runtime
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
  - **scopes**: `sso` (redeem its players' tickets; check their leases,
    profiles, admin rights and terms; publish its terms) and `listing` (its
    directory heartbeat), at least one;
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
  authenticating query, not a cache. Redemption, lease checks, profiles,
  admin checks and tickets for a registered origin follow the same rule
  (not disabled). A gateway with another live token keeps its players'
  leases when one token is revoked; disabling the gateway ends them.
- **Status.** `GatewayStatus`, under a live token whatever the approval,
  answers the gateway's id, origin, metadata and state, and the token's
  scopes and expiry, so a console can say "waits for approval" before it is
  listed. It is refused on the browser listener.
- **Last use.** A heartbeat, a redemption (an admission or an admin sign-in)
  and a status check mark the token used, at most once a minute; the
  per-request lease checks do not, so no hot path writes.
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
is a compiled alias (`OFFICIAL_ALIASES`), so installed clients keep working:
`GatewayTicket` for any official origin is the bridge's, bound to the origin the
client asked for; the bridge redeems it and checks leases, profiles and terms
under the whole set, and a lease keeps its ticket's origin; `GatewayTerms` for
an alias answers the canonical origin's publication, and an acceptance sent for
an alias is kept under the canonical one, while one kept under the alias before
the move still counts. The wire is unchanged. Dropping the alias is a release.

`SetGatewayAdmins { gateway, expected_revision, accounts }` requires the owner's
central session and replaces at most 32 explicit administrator UUIDs with a
revision compare-and-swap (initial revision 1). All listed accounts must be
enrolled. Registration starts with an empty list. Ownership permits changing this
list but grants no implicit backend access: the owner must also be explicitly
listed and loses backend access upon removal. A redemption credential can ask
`CheckGatewayAdmin { account }` only about its own gateway. Heartbeats never carry
administrator claims. Removed members lose existing admin leases as well as the
ability to redeem previously issued admin tickets.

## SSO and revocation

The client requests `GatewayTicket { gateway: origin, purpose }` using its central
session. Tickets expire after 30 seconds, are single use and bind the exact
audience and purpose. Registered gateways support `admit` and `admin`; admin
tickets require current membership. `export` and `link` are restricted to the
official bridge and never reveal legacy IDs to registered third-party gateways.

The gateway posts `RedeemGateway { ticket, purpose }` using its own redemption
credential. For admission and administration, `GatewayIdentity` includes a
gateway-held `lease` and `expires_at` in absolute Unix seconds. Foreign gateways
receive only the central identity; `legacy_id` is absent. The official bridge
may receive its legacy mapping. Export/link do not mint leases.

Store a lease only server-side. Every protected gateway request and existing
WebSocket authorization recheck must call `GatewaySession { lease, purpose }`
with that gateway's credential and fail closed if Central rejects or cannot be
reached. A previous successful response is not authorization for later requests.
The response is bound to the same identity and expires no later than its parent
central session. A lease from another gateway or purpose cannot be substituted.
Central rechecks that the gateway is not disabled, and admin membership, on both
redemption and use.

`GatewayProfile { lease, purpose }`, under the same authority as
`GatewaySession` (a gateway's redemption credential, or the bridge for the
official origin), answers `Response::GatewayProfile { identity, display_name:
Option<String>, tag: Option<u32> }`: the display name and tag of the account
behind the lease ([web accounts](web-accounts.md) §"Display name and tag"),
`null` while the account has none. It runs exactly `GatewaySession`'s lease
check (that gateway's lease, that audience and purpose, both expiries, a
gateway not disabled, an admin's membership for `admin`), so a gateway learns the
names of the people who sign in there, nobody else's, and an expired or
foreign lease is `Rejected`. It is a request of its own (added 23.10.2026)
because a field on `GatewayIdentity` would break every gateway built before
it; gateways start asking it with the accounts design's step 4. It counts
with the control requests and is refused on the browser listener.

Exactly **one** central session is active per account, even on the same device. A successfully verified
new login atomically deletes earlier sessions and their tickets/leases before
inserting the new session under an account row lock. Concurrent successful
logins leave the last committed session active. Incorrect password proofs or
stale credential revisions do not revoke the current session. A session lasts
30 minutes and renews to 30 minutes from each renewal, at most 12 hours after its
sign-in; its leases expire with it, renewed in the same transaction. A gateway
therefore learns a later expiry on its next check and must accept it (no earlier
one). Explicit logout also cascades to all delegated leases. There
are at most eight outstanding tickets and 32 leases per session. These caps bound
capability storage; account ciphertext and device metadata are not deleted by
login replacement or logout.

## Verification

`central-api` unit tests cover canonical origin and metadata restrictions.
The owned PostgreSQL fixture of `cargo xtask postgres` exercises pending
registration, credential separation, approval privilege rejection, directory
and heartbeat behavior, the official row (one at most, without owner or
credentials; the bridge's heartbeat moves it alone; no registered credential
or ticket reaches it; listed when approved, hidden when disabled; the upgrade
over registered gateways), owner isolation, administrator CAS and revocation,
audience/purpose/replay isolation, absence of foreign legacy IDs, central
single-session replacement including concurrent logins, and `GatewayProfile`
(the lease's account's name and tag; another gateway's lease, the bridge
asking for a registered gateway's, another purpose, a disabled gateway, an
unknown and an expired lease are refused). No production accounts
or credentials are used. CI must also validate the shared Rust implementation
on Windows, Linux and macOS before release.

## Terms of use

A gateway publishes its terms with `PublishGatewayTerms` under its redemption
credential (the bridge for the official origin), and checks its players with
`GatewayAuthorize`, which is `GatewaySession` plus their acceptance of
Central's and its own current terms. Published versions are immutable. See
[terms](terms.md).

