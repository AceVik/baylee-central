# Native Tower identity access

Design pass: Tower v2 runs in the Baylee service, outside Central. Central only
issues an account's scoped identity credential and checks that its original native
session remains live. No Tower economy, engine state or matchmaking is added here.

Use the existing reviewed random 256-bit `Token` capability and digest lookup,
not a new token algorithm. Unlike a gateway JWT, this credential is introspected
online for every Tower account operation; it cannot be verified offline or used
as a Central session, bridge credential or gateway admission. This deliberately
avoids adding another JWT/revocation protocol. Logout, password changes, session
expiry and deletion are checked through the existing live session relation.

The operator configures one Tower origin. Issuance accepts no destination from
the client. The credential and introspection result bind that origin; the Tower
service compares it with its own configured origin before admitting an account.
The credential travels only in Authorization headers. Only its digest is stored,
and no credential-bearing type implements printable diagnostics.

Credentials last at most five minutes and never outlive the native session or
its proof cap. Issuance locks the native session and retains at most two tokens
per session, allowing one refresh overlap. A new request replaces older tokens,
not the underlying Central session. Current terms and profile are checked on
introspection. Central failure fails closed; no offline fallback is authorized.

The native version-two protocol adds `{"operation":"tower_token"}` under the
signed Central session and `{"operation":"tower_identity"}` under the distinct
opaque Tower bearer. Neither request accepts account, destination or game fields.
The SDK issues through `Session::tower_token` and introspects through
`central_client::tower_identity`, which checks the returned origin against the
operator's own origin. Browser operations refuse both paths.

Configure optional `tower_origin` in the service's existing stdin configuration.
Production accepts a canonical HTTPS origin distinct from Central; development
accepts a numeric loopback origin. Absent configuration disables this identity
delegation. The pre-start configuration check validates production origins too.
Migration 30 adds only `tower_access`, with independent digests, a live-session
foreign key, expiry bounds and two constrained slots. Apply the accompanying
`ops/grants.sql`; the runtime requires SELECT/INSERT/DELETE and refuses extra rights.
All record IDs use PostgreSQL UUIDv7; bearer secrets use independent randomness.

Implementation pass: no new dependencies, JWT algorithms, environment-derived
credential destinations or token logging. Refreshes lock the parent session before
bounded token replacement. Introspection rechecks credential revision, native
session/proof expiry, restoration and current terms. Session deletion cascades
all derived tokens. Central's existing request/body/credential budgets still apply.
The request variants use empty structures rather than unit variants: the negative
wire test demonstrated that Serde otherwise ignored extra fields despite the enum's
unknown-field policy. Both new operations now refuse such fields explicitly.

Adversarial evidence: the focused real SDK/HTTP/PostgreSQL test passes in
`/tmp/central-tower-access-postgres-20261114-r3.log`, including four concurrent
refreshes with exactly two survivors, wrong origin/scope, current profile/terms,
expiry, logout, credential revision change and account deletion. Listener, database
container and TLS material cleanup are verified even after fixture assertion failure.
This is engineering evidence, not independent review or production release approval.

The request-shape and production-origin tests pass; strict all-target Clippy for
the changed crates, the CLI regression, repository policy and secret scans pass.
The reviewed crypto-scope check, `cargo audit --deny warnings` (0.22.2) and
`cargo deny check` (0.20.2) also pass with the existing dependency graph.
No dependency versions or features changed. Broader unchanged test suites and CI
were not rerun for this milestone, following the owner's scoped-test instruction.
