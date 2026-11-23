# Ranked replication assurance

## Design review

A ranked duel has two independently authenticated players. Each stays connected to their home lobby. Central holds the authority to assign gateway roles, order accepted decisions, change the active host and settle Elo. A verified host receives private game evidence only for its own assigned match and current verification grant.

| Asset or boundary | Required invariant |
| --- | --- |
| Host selection | Prefer two verified gateways under different registered owners with matching compatibility and capacity. Single-host fallback is a choice made at initial assignment. |
| Player intent | Central authenticates the submitting seat independently. The primary cannot submit a decision as the other player. |
| Accepted decision | The match's durable head advances once, for the expected position and a stable command ID. An exact retransmission identifies the original acceptance. |
| Handover | Promotion requires a caught-up assigned standby. The old generation cannot append, report a result or void the promoted match. Revoking and regranting host verification cannot revive a stale assignment. |
| Terminal position | A replicated result uses the committed position and independently reported terminal evidence. Disagreement or a missing required report leaves Elo unchanged. |
| Privacy | Private setup and decisions stay out of the public board, player profile, queue status and diagnostics. The host journal is accessible only to eligible assigned hosts. |

Central presents the primary and any standby by canonical origin. Each participant
approves disclosure to that operator for this match, explicitly covering both
verification and later hosting. Independently, Central checks both acceptances
of each operator's current published terms. A changed document, verification
epoch, or replacement operator requires fresh approval. A participant can decline
an outstanding operator without rating changes. These checks apply to single-host
assignments too. Only the two participants may read each other's published names.

The host-facing ranked JWT uses the distinct signed purpose `ranked_seat` and
the host's origin as audience. Its subject is a database-generated UUID scoped
to this match and host. Its revocable session is separate from ordinary gateway
admission. Claims omit names, tags, migration IDs and terms. Host work and seat
claims use the same two aliases, in seat order, including during recovery.
To activate a seat the gateway also presents the ranked SID from the JWT the
player handed it. Central checks the exact live native session and account, the
host, match, generation, token expiry and revocation. The alias and registered
deck commitment alone cannot activate a seat.
Prior identity-bearing ranked JWTs cannot enter this admission path. Gameplay
itself, especially distinctive decks and timings, can still permit recognition.

The operator identity is the registered gateway owner. Distinct owner IDs help separate failure domains but cannot prove physical separation or rule out collusion. Matching engine builds can reproduce the same rules bug. The Central storage tests exercise authority and atomic accounting; the gateway and client integration checks exercise deterministic replay, private seat projection and the order in which players see effects.

## Implementation review

The transaction locks the match and its current role assignment before accepting private activity. The expected generation, authenticated host, verification epoch, capacity advertisement, compatibility, deadline and per-seat permission are checked at the corresponding authority boundaries. A takeover creates a new generation. Independently recorded votes bind a specific host, generation, position and history digest.

The command journal accepts at most 4,096 positions and 16 MiB of aggregate command bodies per game. An individual command is limited to 32 KiB, the opening to 192 KiB, and an ordered read to eight entries. The frozen opening binds Central's seed, both normalized pools, game and compatibility. Runtime grants limit writable columns. The terminal transaction applies both scores and both receipts together with certification. The host, the participant and the public reader use distinct response types.

The PostgreSQL fixture uses migrated synthetic accounts under the deployed runtime grants. It advances fixture deadlines with its operator role and submits protocol operations through authenticated store entry points. A constraint scoped to one synthetic match interrupts settlement at the receipt write, so rollback and idempotent redelivery can be observed. The SDK fixture signs in two native accounts over HTTP and verifies the issuer's signed host-scoped subject, seat proof, audience, match and generation. The public listener refuses direct token export and internal routing requests. A separate gateway process test exercises the relay and engine replay.
