# Boundary for the upcoming player-echo Tower

Owner clarification, 14 November 2026: Tower gameplay belongs to the gateway
domain, in a global Tower service implemented in the Baylee repository. Players
play each duel on their selected gateway and retain their run across gateways.
Central must not own
its economy, shops, inventory, opponent selection, Tower rankings or duel progression. The design
and implementation plan live in Baylee's `docs/proposals/tower-echoes.md`.

Central continues to own global Daily/Weekly tasks and their rankings. Its role
in the new Tower is identity and confirmed permanent accomplishments, plus any
separately agreed account-wide projections. Tower service failures must not block
the calendar activities, login or already earned badges.

Before accepting achievements from another service, define the permitted issuer
and scope, server-authenticated result provenance, account ownership, replay and
duplicate protection, and deletion/export behavior. A registered gateway is not
automatically trusted to award unrestricted global achievements. The milestones
below implement the separate scoped contract; deployment still requires explicit
operator provisioning. Baylee's delivery worker is implemented separately; neither
repository's feature branch is an external deployment.

The current Central-owned five-floor certified expedition (`tower.md`) predates
this clarification. It is implemented but does not yet satisfy the new Tower
architecture or the player-echo/shop requirements. Migration must preserve its
earned accomplishments and distinguish old records from the new rules.

## Permanent award contract — design pass

The v2 authority sends only a stable event UUID, existing Central account UUID and
verified run/cohort/stage/outcome counters. Central accepts this under a separate,
operator-provisioned issuer credential for the configured Tower origin. Neither a
player session, Tower identity token, ordinary gateway nor the legacy challenge
bridge inherits this authority. Revocation is checked on every ingestion. The
issuer authenticates provenance; these counters are not independent game replay.

Receipts must bind issuer, event, account, run and stage. Repeating the same fact is
idempotent; conflicting content or a reused run/stage is rejected. Account deletion
and ingestion must serialize on the account row, and deletion removes its receipts
and projection. An already deleted account is never recreated by a late delivery.
An outage retains the Baylee outbox and must not block gameplay or change rewards.

The cosmetic projection distinguishes total verified duels, victories, eight-win
expeditions and flawless eight-win expeditions. Three permanent tracks reward
victories, completed expeditions and flawless completions. They have cumulative
bronze/silver/gold/platinum/obsidian thresholds, no lost streak and no spendable
currency. Existing five-floor expedition achievements remain separate.

The shared contract rejects invalid UUID versions, unknown fields/schema/outcomes,
impossible cumulative counts and results after a run's terminal threshold. Any
projection update is checked and atomic on overflow. Shape validation never grants
issuer permission or deduplicates an event by itself. Transport, persistence,
deletion/export and both UI projections remain required before this is complete.

Implementation pass: `central-api::tower_awards` defines only fixed-size verified
facts and cosmetic totals. Unknown JSON fields fail closed, all identifiers require
UUIDv7 with the RFC variant, and a run's receipts must arrive in ordinal order.
An out-of-order delivery is retried by the issuer instead of projecting an
incomplete run. Facts and envelopes deliberately have no Debug implementation.
The existing reviewed dependencies and badge materials are reused unchanged.

Adversarial validation: five targeted tests pass in
`/tmp/central-tower-awards-contract-tests-20261115.log` (0.01s). They traverse all
4,096 bounded win/loss decision paths, include rules draws, reject schema/field/
UUID/counter substitution and result gaps, and check atomic overflow failure and
every badge threshold. Strict API library/test Clippy passes in
`/tmp/central-tower-awards-contract-clippy-20261115-r2.log`; policy and working-tree/
full-history secret checks pass in `/tmp/central-tower-awards-policy-20261115.log`
and `/tmp/central-tower-awards-secrets-20261115.log`. This pure contract adds no
database migration, network endpoint or issuer trust grant yet, and does not claim
end-to-end award delivery or an independent security review.

## Ingestion persistence — design pass

A separate issuer registry is provisioned with migration/operator credentials.
Runtime may read it and update only its last-use timestamp: it cannot insert an
issuer, replace a credential, change an origin or enable a revoked issuer. The
last-use update also locks/rechecks the issuer before accepting any fact. This
deliberately serializes the small background delivery stream per issuer; no game
request waits for this path. Then ingestion locks the existing Central account,
checks an exact prior receipt and validates the next ordinal before atomically
saving the run receipt and its permanent counters.

Only the digest of the existing reviewed random Token credential is stored. No
custom signing or token scheme is added. Receipts are fixed-size, bounded facts;
the per-account lifetime intake limit is one million duels. At capacity, retries
of existing receipts still succeed; new deliveries remain queued at Baylee. This
explicit operational bound needs monitoring and an archival design before raising
it, rather than silently discarding deduplication history. Deleted accounts return
a terminal no-account disposition and cannot be recreated by delivery. Existing
but unrestored accounts are retryable, not treated as deleted. The issuer registry
contains no player names or gameplay data; all account-owned rows cascade on
deletion. A separately paginated own-account export is required before exposure.

Implementation pass: migration 31 and the restricted store adapter now implement
issuer provisioning through an operator connection, live credential/origin checks,
immutable exact receipts, sequential run receipt heads and atomic permanent
totals. A reused event is compared before even returning a no-account disposition,
so changing its target to a nonexistent account cannot conceal a conflict. Normal
runtime grants cannot provision/re-enable issuers, replace credentials or rewrite
receipts. Extra issuer UPDATE privilege fails startup checks. The own-account
store export returns at most 128 rows plus an exclusive cursor; all reads use a
consistent snapshot and a live surface-bound session.

Adversarial validation passes against owned loopback PostgreSQL 18 in
`/tmp/central-tower-awards-store-tests-20261115-r4.log` (1.31s): real enrolled and
restored synthetic accounts, restricted runtime grants, concurrent exact retries,
issuer/session/origin confusion, account/event substitution, out-of-order results,
an injected write failure with full rollback, eight-win/flawless accounting,
129-row pagination and account isolation, capacity with exact retry, revocation,
simultaneous account deletion/ingestion and late delivery without resurrection.
The migration probe reconstructs v30 around a real enrolled account, applies the
exact new SQL and checks preservation before rolling the probe back. Owned
database processes/listeners/clusters are closed, including diagnostic failures.

Strict API/store/migration/xtask library/test Clippy passes in
`/tmp/central-tower-awards-store-clippy-20261115-r5.log`; policy and full-history
secret scans pass in `/tmp/central-tower-awards-store-policy-20261115.log` and
`/tmp/central-tower-awards-store-secrets-20261115.log`. The targeted check is
available as `cargo xtask postgres --tower-awards-only` and remains part of the
full store suite. No dependency versions or features changed. No new network
endpoint, SDK delivery worker or UI badge display is claimed by this storage
milestone; those integrations still need their own transport and visual checks.

## Native ingestion and own-account reads — three passes

Design: `TowerAward { delivery }` is an issuer-only native credential operation.
It accepts the fixed-size contract above and returns `TowerAward { receipt }`.
The browser listener rejects it even with the correct issuer secret. Signed
player sessions, Tower identity grants and the challenge bridge cannot ingest.
`TowerProgress {}` and `TowerAwardExport { after }` instead require a live,
surface-bound player session, work on both account surfaces, and return only that
account's totals or at most 128 receipts. They remain readable when the Tower
service is unavailable or a new terms version needs acceptance. An issuer cannot
use these operations to read accounts. Existing HTTP body/rate limits apply; none
of the operations bypasses authentication through an in-memory rate-limit cache.

Implementation: the native SDK checks event, account, run and ordinal in the
durable receipt before the caller may acknowledge its outbox. Wrong response
variants are rejected. Session methods enforce their original Central endpoint,
validate counters, ordered UUIDv7 export pages, the exclusive cursor and canonical
issuer origin. No counters, credentials or submitted payloads gain Debug output.
No dependency version or feature changed. These are additive native/browser RPC
operations; the permanent badges are not yet wired into either player interface.

Adversarial evidence: `/tmp/central-tower-awards-transport-tests-20261115-r4.log`
passes both the existing Tower identity transport test and the new award test
against real owned PostgreSQL 18 and native/browser loopback HTTP (2 tests,
4.42s). It covers issuer/bridge/player/Tower scope confusion, browser refusal,
idempotent delivery, changed facts, unknown JSON fields, own-account reads/export,
terms changes, issuer revocation and account deletion without resurrection.
Six contract tests and the SDK receipt-substitution test pass in
`/tmp/central-tower-awards-projection-sdk-tests-20261115.log`. Strict touched-crate
Clippy passes in `/tmp/central-tower-awards-transport-clippy-20261115-r3.log`.
The targeted `cargo xtask postgres --tower-awards-only` now runs both store and
native SDK slices. All owned listeners and database processes exited. A temporary
native fixture shutdown bug was corrected to use PostgreSQL fast shutdown rather
than SIGKILL; the final run also leaves no new System-V shared-memory segment.
These checks are local engineering evidence, not an independent security audit.
Policy and full-history secret checks pass in
`/tmp/central-tower-awards-transport-policy-20261115.log` and
`/tmp/central-tower-awards-transport-secrets-20261115.log`.

## Website collection and export — design and implementation

The challenges page reads confirmed v2 accomplishments directly from Central,
independently of the Tower gameplay service. It presents three separate original
motifs: crossed swords for duel victories, a crowned spire for eight-win clears,
and an infinity shield for flawless clears. The shared API supplies the permanent
thresholds; no client increments them or infers a win from a table closing. The
five-tier preview uses the existing ornaments and reduced-motion preference, with
distinct motifs and colours from the legacy expedition badges. All eleven site
languages contain the collection's names, requirements and confirmation notice.

Each page starts with unknown progress, not invented zero totals. A transient or
malformed reply preserves the last confirmed values for that session. Rejection,
logout or account replacement removes them and any open earned-tier preview.
The minute refresh keeps unchanged cards and keyboard focus stable. Old in-flight
responses are fenced by the session generation; they cannot restore the previous
account's collection after a storage event. This browser-local fence supplements
the server's live-cookie account authorization; it is not an authentication token.

The account download collects at most 7,680 ordered v2 receipts, with explicit
continuation metadata. It validates every page and refuses an unavailable or
malformed dataset rather than offering a silently incomplete merged file. The
whole assembly is tied to the session from before its first account request;
cross-tab session changes remove an existing download and invalidate pending work.
Each sign-in now changes a non-secret random browser marker, even for identical
expiry seconds; the new reads and export compare the whole marker. No player name,
account identity or credential is stored in it. The same-session expiry collision
found during the implementation review therefore also emits a cross-tab event.
No new dependency or public score-ingestion path is added. The native client's
badge integration and operator release gates remain separate work.

Adversarial and visual evidence, 15 November 2026:

- `/tmp/central-echoes-browser-tests-20261115-r2.log`: 42 Chromium desktop/phone
  cases pass (23.8s), covering the new collection/export and affected Daily/Weekly
  and legacy expedition views. They check tier thresholds, distinct silhouettes,
  unchanged-refresh focus, 320px reflow, 44px controls, dark/light WCAG checks,
  reduced motion, unavailable/corrupt/rejected progress, old-account delayed replies,
  two-page export, repeated cursors, retry and session changes during both the
  initial account request and receipt collection. An earlier run's two failures
  were a test selecting an intentionally hidden empty alert; the corrected selector
  checks its actual DOM text rather than requiring it in the accessibility tree.
- `/tmp/central-echoes-session-export-tests-20261115.log`: ten selected desktop/phone
  cases pass (9.5s), including existing account/email export behavior, fresh local
  markers for equal-expiry sign-ins, and injected entropy/storage failures that
  remove the previous marker. These browser tests use synthetic server/crypto
  stand-ins; the earlier transport milestone provides the real session evidence.
- `/tmp/central-echoes-web-visual-20261115/`: reviewed English dark/light and German
  maximum-tier screenshots. A second visual pass shortened the maximum-tier count,
  applied locale number formatting and captured the section in a taller viewport
  at the checked width, keeping the real sticky navigation above the collection.
  Four targeted screenshot cases pass in 6.3s. All Playwright-owned servers and
  browser contexts stopped; the owned loopback listener at 28792 is closed.
- `/tmp/central-echoes-web-tests-20261115-r2.log`: 125 library and six contrast,
  hover and inert-element tests pass; the doc-test target contains no examples.
  Strict all-target/all-feature web Clippy, formatting, the full first-party policy
  with compiler probe and tree/history secret scans pass in the corresponding
  `central-echoes-web-{clippy,policy,secrets}` logs (`r7`, `r2`, `r2`).
- The final public/admin WebAssembly and hashed site build passes in
  `/tmp/central-echoes-site-build-20261115-r5.log`. The macOS debug server links
  successfully, with the existing large-DWARF-unwind-table linker warning; this
  does not claim a Linux release build, production deployment or independent audit.
- The changed `central-web` library also passes its locked, all-feature release
  build in `/tmp/central-echoes-web-release-20261115.log` (2m 59s). Crypto/SQLite
  dependency-scope checks pass without dependency changes. This is the scoped
  gate requested by the owner; a whole-workspace server gate was not rerun.

No engine, database, issuer or courier behavior changed in this UI milestone, so
their long suites were not repeated. The native collection still needs its own
renderer, interaction and measured shader checks.
