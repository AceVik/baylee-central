# Global ranked play

## Permanent division cosmetics

Completing ten placement games in a format starts the player's cosmetic collection.
Each reached player division earns a sleeve and a playmat for the whole account.
A jump from Gold IV to Gold II includes Gold III. Wood, Iron, Bronze and Silver
each have three divisions. Gold, Platinum and Diamond each have four. Paper,
Master, Grandmaster and Champion each have one set, for 28 sets in total.
Champion requires publication on the player track of the global Top 100.

Central derives ownership from its confirmed player badge ledger across
Standard, Highlander and Commander. The first acquisition persists through
demotion, another format and future seasons. The migration recognizes existing
players' highest recorded achievement and uses the migration time for older
acquisitions whose original timestamp was never recorded. Newly imported
assets become available immediately to qualifying accounts.

The native `Collection` request returns account-owned items with a ranked or
calendar source. Calendar-only clients keep their original `Inventory` and
`Claim` operations. Both sources use the same private media and per-deck
selection paths. Central checks ownership, the equipment slot and current seat
authorization before delivering the privately provisioned image.

## Play

The native client enters Standard, Baylee Highlander or two-player Commander through its signed Central session. Each format has a player ladder and a deck ladder. The home gateway stays selected throughout a match. Personal contacts, private chat and the deck vault retain their Central account scope.

## Who can do what

| Operation | Authority |
| --- | --- |
| Read seasons, division boards or public profiles | Anyone, including a signed-out native client. The native and browser listeners apply the directory rate budget. |
| Read your own placement progress | An authenticated session. The account comes from that session. Before the first confirmed game, each format shows Unranked at 0/10, including for accounts without a public handle. |
| Join, inspect or cancel a ticket | The live native Central session for its own account. A join requires acceptance of the current terms. |
| Enable a ranked host | A Central administrator using the existing fresh web administration session. Directory approval and ranked verification are separate stored decisions. |
| Advertise capacity or claim, start, finish or void a game | A verified host using its registered host credential, a named token explicitly granted the ranked scope, or the official bridge. Operations on a match also require its assigned host and current verification epoch. |
| Assign ranks and award points | Central, once for the engine-confirmed result of the assigned game. |

The queue holds at most one active ticket per account across all formats. A client retains its request key on retries and its frozen deck commitment. In a busy queue the matcher checks older tickets and nearby Elo neighbours. Its range grows with waiting time. In a sparse queue it opens the broadest range after 60 seconds. Recent repeats cost preference, so two people alone can still play. A fresh capacity announcement and a free slot on a verified host are required to reserve an opponent.

For native admission the Central listener authenticates the participant, resolves the current verified primary and presents a match-bound ranked token on the relayed connection. Its host-scoped subject and revocable session are distinct from ordinary lobby admission. The primary verifies the frozen deck commitment against the submitted legal deck and binds both seats to one engine game. A single-host table can settle its own engine result. A protected table settles on matching evidence from both executors. Removing verification fences outstanding assignments.

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

Central keeps ratings, deck versions, receipts, matches, badges, cosmetic unlocks and Champion places until the account is deleted. Closed queue tickets are the exception. The operator's daily sweep (`--sweep-stdin`) deletes a closed ticket 24 hours after its waiting ended, or 24 hours after its match ended when it had one (`CLOSED_TICKET_HOURS`). It deletes in batches of 4,096 until a batch comes back short. Open tickets are never pruned. A day is far longer than anything that still reads a closed ticket:

- the admission cap of 24 tickets an hour, which counts the last hour's tickets;
- the sparse-queue check, which counts the last two minutes';
- a `Join` retried after a lost answer, which finds its own key instead of queueing again;
- `Status`, which shows the last match's result from its ticket.

Nothing settles from a closed ticket. A match's consent check reads its tickets only while they are assigned. Rematch spacing reads `ranked_matches`. After pruning, `Status` answers `Idle`; the receipt stays. A retry of that key after the day is a new request: it is still the single ticket the account may hold, and the client shows it as waiting and can cancel it.

### Own data in the account download

`Ranked::Export { table, after }` pages through the session's own ranked rows, on both listeners. It needs no acceptance of the current terms, and the account comes from the session alone. Released clients never send it. `AccountExport` is unchanged. The website's account download adds the result as `ranked`, with one `{entries, complete, after}` object per dataset. A dataset stops at 7,680 rows and then reports `complete: false` and its `after` key. Pages are checked before their key is followed (`ranked::export::validate_page`). A malformed page fails the whole download.

| Dataset | Key, page | Contents |
| --- | --- | --- |
| `seasons` | season ID, 8 | Season metadata. For each format the stored player rating: career games, wins, losses and draws; rank (Unranked before ten games, without the boards' inactivity clamp); Elo and queue Elo; the rating model's score, placement estimate and upset allowance in thousandths of a point; first and last game. Also the season's badge peaks and its Champion place, with the deck's public ID when a deck earned it. |
| `decks` | public deck ID, 6 | Each rated version with its format, card-identity digests and combined copies, total copies, Elo, reference Elo, initial-reference flag and first season. Per season: career counts, season games, rank, Elo, first and last game. |
| `receipts` | match ID, 30 | Format, settlement time, won/lost/drawn, the opponent's public account ID, Elo after the game and its change, player rank before and after, and the played deck's settlement. |
| `matches` | match ID, 30 | Season, format, this account's seat, own deck commitment and claim, host ID and origin, state, engine game ID, own result, reservation and end. Also this seat's own decisions about the named operators: host, origin, epoch, approved or declined, and the accepted terms version. |
| `tickets` | request key, 30 | Every ticket Central still keeps: season, format, own deck commitment, queue Elo, protection choice, state, match ID, admission and end of waiting. |
| `cosmetics` | league and division, 30 | Each unlocked division set and when it was earned. |

The opponent appears only as its public account ID on a settled receipt. Settlement gives both players a rating row, and with it a public profile under that ID. This also holds for an opponent without a display name. The export never carries the opponent's commitment, deck, ratings or seat, and never its operator decisions. It also leaves out the receipt's `opponent_established` and `elo_gap` columns, which describe the opponent. Host aliases, journals, witnesses, seeds, clocks, start fingerprints, ranked tokens and credentials have no field in it. The private deck version hash is replaced by the deck's public ID. An account that never played gets empty, complete datasets.

## Replicated games

Central prefers a pair of recently available verified gateways with the same rules and engine compatibility commitment, capacity for a duel and different registered owners. The primary serves the table. The standby replays the same opening and the ordered decisions independently. Both players stay signed in to their home gateway. If Central can allocate only one compatible operator at the initial assignment, the participants see single-host protection. A replicated match keeps its two-host settlement requirement throughout recovery.

Each participant reviews the selected operators and their current terms before releasing private game information to them. Central issues a separate player alias for each match and each operator. Admission tokens for a ranked game contain that alias, their intended audience and match binding. The two assigned players can view each other's published handles through their authenticated Central sessions. The gateway's replay material contains both decks, the random opening and private decisions. This requires trusting every operator the participants approve with that material.

The client connects to Central for ranked traffic. The assigned gameplay gateway sees the relay's network address. Central constructs the upstream request with a fixed set of headers and keeps the player's native credential on the Central connection. An operator who also hosts a player's ordinary lobby can still observe that separate lobby connection.

A reconnecting participant uses Central's committed match position. Each input carries a stable command identifier and its expected parent. A retry that matches the original seat, parent and payload returns the original journal position. The next certified position requires both executors' evidence for that position. Both reports have to agree on the private state digest, decision acceptance, outstanding decisions and terminal outcome. Player and deck Elo settle in the transaction that certifies the terminal position.

Both players must first reach the table before a player decision is admitted. Central allows 180 seconds of connected decision time per question and 60 seconds away. An account heartbeat or the primary's socket observation protects a connected seat. Changing the opponent's question or reconnecting preserves the time already spent on one's own question. Both-away abandonment and simultaneous deadlines yield a neutral result. When a deadline is reached, Central reserves that position until both executors have renewed their leases. A recovery refunds elapsed time following their last bilateral heartbeat. A valid expiry becomes one journal event bound to that seat and question, which both engines must execute before a result can settle.

When leadership expires, Central can promote an eligible standby that has independently certified the last committed position. A fresh standby needs approval from both players for that match and its current terms before receiving any prior game data. The replacement replays the certified prefix and pending decisions. A refusal, unresolved disagreement or expired recovery ends the match without rating it. [Operational recovery and privacy](operations.md#replicated-ranked-operations)
