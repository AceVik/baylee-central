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
gateway. Each owner can reserve at most eight origins. Before the first approval,
the same authenticated owner may repeat registration to recover a lost response;
this rotates both pending credentials. An approved or disabled origin cannot be
rotated this way, nor can another account claim an existing origin.

Live origins must be canonical HTTPS origins without credentials, path, query or
fragment. Isolated development instead accepts explicit numeric loopback HTTP
origins with a nonzero port. Metadata has bounded name/version fields and a
canonical website URL on the same origin. Metadata is untrusted display text:
render it as text, never HTML. Software versions are self-reported.

An operator must independently verify control of the claimed origin and the
gateway operator's identity before inserting its `gateway_approvals` row. This is
a deliberate administrative action using the separate operator connection, not
an automatic network probe or public API. Coordinate the approval transaction by
locking the relevant `gateways` row `FOR UPDATE`, verifying the current ownership
record, and inserting approval before commit. Disabling approval immediately
hides the entry and rejects credential authentication, new admission and lease
introspection. Never approve solely because registration succeeded. The service
does not fetch registered URLs, follow redirects, or accept callback challenges
that could create SSRF.

The runtime may use these registry privileges without grant options:

```sql
GRANT SELECT, INSERT, UPDATE ON central_identity.gateways TO central_runtime;
GRANT SELECT ON central_identity.gateway_approvals TO central_runtime;
GRANT SELECT, INSERT, DELETE ON central_identity.gateway_admins,
    central_identity.gateway_leases TO central_runtime;
```

Existing native account/session/ticket permissions remain necessary. The runtime
also requires the [private object and archive grants](private-objects.md).
The reusable SDK exposes `gateway_directory`, `Session::register_gateway` and
`Session::set_gateway_admins`; it validates returned IDs and public metadata.
The runtime
must not own these objects or inherit operator privileges. Startup rejects write,
DDL-related, column-write and grant-option permissions on the approval table,
including inherited effective privileges. Operator credential rotation/revocation
is an administrative DB operation; there is no public operator-approval endpoint.
The current executable remains an isolated development service, not a reviewed
production deployment.

## Discovery, heartbeats and administrators

`GatewayDirectory { after }` is public and returns up to 100 approved gateways in
UUID order. Pass the last returned UUID as `after` for the next page. Each entry
contains origin, public metadata and `online`. Online means Central accepted a
heartbeat within 90 seconds; it is not an independent availability guarantee.
`GatewayHeartbeat { metadata }` requires only the heartbeat token, is limited to
one update per five seconds, and cannot alter owner, origin, administrator list,
approval, or SSO credentials.

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
Central rechecks approval and admin membership on both redemption and use.

Exactly **one** central session is active per account, even on the same device. A successfully verified
new login atomically deletes earlier sessions and their tickets/leases before
inserting the new session under an account row lock. Concurrent successful
logins leave the last committed session active. Incorrect password proofs or
stale credential revisions do not revoke the current session. Sessions last at
most 30 minutes. Explicit logout also cascades to all delegated leases. There
are at most eight outstanding tickets and 32 leases per session. These caps bound
capability storage; account ciphertext and device metadata are not deleted by
login replacement or logout.

## Verification

`central-api` unit tests cover canonical origin and metadata restrictions.
The owned PostgreSQL fixture of `cargo xtask postgres` exercises pending
registration, credential separation, approval privilege rejection, directory
and heartbeat behavior, owner isolation, administrator CAS and revocation,
audience/purpose/replay isolation, absence of foreign legacy IDs, and central
single-session replacement including concurrent logins. No production accounts
or credentials are used. CI must also validate the shared Rust implementation
on Windows, Linux and macOS before release.
