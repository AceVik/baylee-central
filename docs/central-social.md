# Central contacts and chat

Implementation in progress on `codex/central-chat-contacts`.

## Design invariants

Central owns account relationships independently of gameplay gateways. The browser
uses its existing same-origin HttpOnly session; the native SDK uses its existing
native bearer. Neither credential is forwarded to a gateway. Both surfaces address
the same account graph through `Social` operations. Exact public tags resolve public
display names only; login names, account UUIDs and credentials are never returned.

Contact changes lock the affected account rows in ascending UUID order, then
revalidate the session under that lock. A canonical pair stores one request or
friendship. Crossed requests accept once; accepting one's own request is rejected.
Blocks remove pending/accepted relationships atomically, and only their owner can
remove them. Unblocking never resurrects an old relationship. Account deletion
cascades through graph and budget rows.

Limits: 300 friends, 50 outgoing requests, 300 incoming requests, 1,000 owned blocks.
New requests consume an account budget of 60 per hour; exact lookups 120 per hour.
Budgets survive process restarts and use one fixed row per account. Repeated completed
transitions do not consume capacity. Snapshots are bounded and sorted by public tag.

Migration 000019 and `ops/grants.sql` provision the graph with the runtime's exact
privileges. No production migration has been run. Browser and native transports keep
their existing origin, body, connection and session boundaries.

## Validation

The real PostgreSQL fixture covers caller isolation, crossed concurrent requests,
block/request races, own-request refusal, independent blocks, browser/native session
separation, shared account state, revocation and request-churn budgets. Validation
results will be recorded after the fixture and full gates run.

Chat interoperability, archive migration and the web/native UI remain in progress.
