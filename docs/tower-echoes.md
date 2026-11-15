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
automatically trusted to award unrestricted global achievements. No new award
endpoint or trust grant is implemented by this design note.

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
