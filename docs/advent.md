# Advent doors and permanent cosmetics

Central opens one door at midnight in Europe/Berlin on each day from December 1 through December 24. Every account gets the same board and reward for a given door. Earlier doors remain playable through January 6. The 2026 campaign opens at 23:00 UTC on November 30 and closes at 23:00 UTC on January 6, 2027. Central selects the active campaign or the closest upcoming one for a fresh client, including in early January. It returns an empty discovery result when neither exists.

A player can request either of two free hints and try a released board as often as they wish. Central evaluates the submitted moves against the privately provisioned board. The first winning submission records the solution and grants its permanent inventory item in the same transaction. A lost response or a closing campaign therefore preserves the earned item. A claim retrieves that reward for presentation, including after closing or changing devices. Repeated claims return the same entitlement. [Tactical rules and validation](advent-puzzles.md)

## Acquisition and display

The calendar sends an account's door states and release times. During the campaign it can also return the private calendar mosaic. A winning submission adds the fixed reward to the player's inventory. Claiming reveals its title, finish and artwork metadata on the reward screen. The inventory remains available after the campaign. Account deletion cascades to the player's solutions and entitlements.

Sleeve and playmat selections belong to the individual deck. When a ranked player presents that deck for a match, Central verifies ownership and slot type, then freezes the two selections for that player and match. Only its two authenticated participants can fetch the metadata or bytes of the displayed slots while the duel is playing. Central drops the match selections when play ends. Ordinary gateways use the separate table appearance grant and check their admitted seats before delivering a displayed asset.

The shipping code contains the puzzle evaluator, calendar logic and generic presentation. The operator provisions the campaign boards and original static WebP files privately. The database verifies each image's declared length and SHA-256. It stores the bytes separately from the metadata projections. The runtime account has read access to campaign data and artwork, and insert access for solutions and grants. It cannot publish or replace campaigns.

Each private image transfer authorizes its scope again for every bounded 192 KiB range. Clients can confirm a cache hit using the expected digest. A changed or partial transfer is discarded. The receiving client verifies the final byte count, digest, declared dimensions and supported format before displaying an image.
