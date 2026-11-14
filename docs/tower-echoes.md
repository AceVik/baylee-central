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
