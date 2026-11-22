# Global ranked play

The native client enters Standard, Baylee Highlander or two-player Commander through its signed Central session. Each format has a player ladder and a deck ladder. The home gateway stays selected throughout a match. Personal contacts, private chat and the deck vault retain their Central account scope.

## Who can do what

| Operation | Authority |
| --- | --- |
| Read seasons, division boards or public profiles | Anyone, including a signed-out native client. The native and browser listeners apply the directory rate budget. |
| Join, inspect or cancel a ticket | The live native Central session for its own account. A join requires acceptance of the current terms. |
| Enable a ranked host | A Central administrator using the existing fresh web administration session. Directory approval and ranked verification are separate stored decisions. |
| Advertise capacity or claim, start, finish or void a game | A verified host using its registered host credential, a named token explicitly granted the ranked scope, or the official bridge. Operations on a match also require its assigned host and current verification epoch. |
| Assign ranks and award points | Central, once for the engine-confirmed result of the assigned game. |

The queue holds at most one active ticket per account across all formats. A client retains its request key on retries and its frozen deck commitment. In a busy queue the matcher checks older tickets and nearby Elo neighbours. Its range grows with waiting time. In a sparse queue it opens the broadest range after 60 seconds. Recent repeats cost preference, so two people alone can still play. A fresh capacity announcement and a free slot on a verified host are required to reserve an opponent.

The client obtains a short-lived signed token with the ranked purpose, exact match ID and selected gateway audience. A game and the home lobby use different revocable session IDs, including when the home gateway hosts the match. The host authenticates each player and verifies that the submitted legal deck matches the commitment. It binds two seats to one engine game. The engine supplies the terminal game state to the clients. The host delivers that result to Central. An identical delivery can be retried. Changing the game or the recorded winner conflicts. Removing host verification fences outstanding host assignments.

## Progress and deck edits

Ten completed games in a format place the player. Ten games with a particular combined pool in that same format place the deck. Before the tenth result the rank is **Unranked**. Player and deck can finish placements on different games. The initial reveal can reach Bronze. From Silver upward, progression requires the full number of confirmed games. Master requires 2,000 games, 2,240 Elo, at least 184 elapsed days since the first game and recent activity. At ten games per day this takes at least 200 playing days.

Each account has one initial deck reference per format. A previously played composition resumes its stored Elo and placement count. Card identity and multiplicity are combined across main deck and sideboard for rating. The commitment sent for the game also covers the actual zones and play choices.

When a player explicitly requests a ranked deck preview, the native client sends the selected format, normalized card identity digests and copy counts using the player's Central session. Central compares only that owner's versions in a read-only database transaction and returns the projected Elo, comparison size, deduction and placement progress. Selecting an unplayed list cannot reserve the first reference or earn a placement. Actual registration takes place with the verified result of a match.

For a new composition let `q = 1 − multiset_overlap / max(reference_size, new_size)`. Its inherited Elo loses the fraction `0.2q + 0.8q²` of the reference's points above 150. A 50% rebuild from a 1,500 reference inherits 1,095. A complete rebuild inherits 150. Playing one game between edits cannot turn every intermediate deck into a new reference. A composition with at least forty results of its own can qualify as an additional reference at a season boundary.

Player Elo accounts for the opponent's expected strength. The consequences of a first upset across a wide gap are capped separately for each player. Further surprising results are evaluated with recent opponent history. The same two accounts repeatedly playing each other provide less independent evidence. The reproducible experiments and their assumptions are in the [placement study](ranked-placement-study.md).

## Publication and storage

Read a board by season, format, player/deck track, league and optional division. A keyset cursor breaks ties deterministically. Decks use opaque public identifiers so a spectator cannot match a public row against a known decklist hash. Profile lookup uses the public account ID or full display handle. The daily Champion publication shares at most 100 distinct people across the formats and both tracks. A player who qualifies in several categories occupies one of those 100 places.

The active season is the unique database interval containing the current time. Season boundaries are exclusive at the end. History, first-match age and career totals survive a rollover. The public profile distinguishes season results, career totals and awarded badges. A closed season has its own board.

The authority receives account and opponent identifiers, timestamps, match/host identifiers, deck commitments, normalized card-identity digests and copy counts, confirmed outcomes and rating history. Normalized identities are derived from public card data and can be compared by the authority. Deck names, personal notes, reorderable cosmetics, encrypted library objects and seat tokens have no field in the public ranked protocol. Ranked account rows reference the account with erasure cascades. For the administrative procedure see [gateway verification](gateway-registry.md).
