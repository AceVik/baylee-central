# AI players

*Design pass only, 26 November 2026. Nothing described here is built.*

The owner approved "AI players" on 26.11.2026: accounts that are always
labelled as AI and otherwise play Ranked like people (Baylee's
`docs/proposals/ai-players.md`). This note is the first of the three passes in
[security assurance](security-assurance.md) for Central's part of the
concept's Stage 1: **one fair house-AI account fills a lonely person's ranked
search.** The implementation pass and the adversarial pass follow with the
code and are recorded here once they ran. Later stages (disconnect rules, half
stakes, the leaver ledger, language models, Tower, public replays) are out of
scope; where a choice here could block one, the section says why it does not.

## What it must keep

Assets: a person's choice not to meet an AI; whether an account is an AI
(who decides that, and who sees it); the AI runner tokens; the ladder's
integrity; host capacity and relay slots; the queue rules people rely on.

Invariants:

1. Only an administrator makes an account an AI or ends that, and ending it
   is possible only before the account's first ranked match. An AI account
   cannot change its own kind, profile, label, name or flag.
2. A client that cannot show the AI badge is never seated against an AI.
   Every reply that names an AI account to a client that can show the badge
   carries it.
3. A ticket that did not accept an AI is never seated against one, by any
   path.
4. People's queue rules stay as they are: one non-closed ticket per account
   (`ranked_single_ticket`), 24 joins an hour, request keys for retries. AI
   seats are not tickets.
5. Fill-in takes no opponent a person could have had: while another
   person's ticket of the same season, format and kind waits, no AI is
   seated.
6. One AI account plays at most its `max_games` matches at once, all AI
   seats together number at most `global_seats`, and an AI match never takes
   the host slots kept for people.
7. From the commit of an administrator's switch-off, the AI gets no new
   match. Its running matches end normally.
8. The runner holds only an AI runner token: scoped to listed AI accounts,
   kept as a digest, revocable, audited; an AI account has no password.
9. A person's AI preference reaches neither the opponent, nor a host, nor a
   public view.

## 1. Representation

An AI player is an account an administrator creates as an AI on the website
(owner, 26.11.2026): a name with the `KI-` prefix, a tag, the settings below,
and no password (§2). A person's account is never converted. Its AI state
lives in a new table, `central_identity.ai_players`:

| Column | Meaning |
| --- | --- |
| `account_id` | primary key, references `accounts` `ON DELETE CASCADE` |
| `kind` | `house`, `model` or `net` |
| `profile` | the runner's key for the mind, such as `house:expert` (`^[a-z]{1,12}:[a-z0-9_.-]{1,48}$`) |
| `label` | the public label, such as "Haus-KI · Expert": 1 to 40 characters, no control characters |
| `enabled` | takes part in fill-in and the carousel; false until an administrator switches it on |
| `max_games` | matches at once, 1 to 16, default 1 |
| `daily_games` | games a UTC day, `NULL` for no cap; a CHECK requires it for `model` |
| `set_at`, `set_by` | when the account was flagged and by which administrator (`NULL`: the operator) |
| `changed_at`, `changed_by` | the last change |

The transaction that changes a row locks the account row first and checks:

- **Flag and clear only before the first match.** An account is flagged only
  when it is enrolled, holds no administrator role, has no `ranked_matches`
  row in any state and no ticket that is not closed. Clearing (deleting the
  row) has the same condition. After the first match the flag stays for the
  account's life, and an administrator switches the AI off instead.
  Otherwise an AI's record could become a person's, or a person's an AI's.
- **The mind is fixed.** `kind` and `profile` never change. A different
  mind is a different account, because the name is the measurement
  (concept §"Who plays"). Before the first match an administrator clears and
  flags again. `label`, `max_games`, `daily_games`, `enabled` and the
  display name may change.
- **Fair only.** Every ranked AI is a fair remote seat (owner, 26.11.2026):
  a profile that sees hidden cards (`house:seer`) is refused, by the code and
  by a CHECK in the table, and the badge says nothing of fairness.
- **No administration through an AI.** `SetAdmin` refuses a flagged account,
  so an AI's token never opens the administration.
- **The administrator owns the AI's public face.** The display name is set
  with the flag and begins with the reserved prefix of §6. For a flagged
  account Central refuses the account's own profile changes (name, picture,
  showcase), its own deletion, browser sign-in, social requests (contacts,
  invitations, whispers, groups), gateway admission for any purpose but
  `ranked`, Tower tokens, and the ordinary ranked `Join`. The operator's
  erasure stays.

Every change writes its audit event in its transaction, under new `Action`s
in the existing pattern: `ai.flagged`, `ai.cleared`, `ai.enabled`,
`ai.disabled`, `ai.changed` and `ai.settings_changed`. Each holds the actor and
the account's id. Its `detail` holds numbers only (`{"max_games": 4}`, the
kind as a code), never the label or a name ([administration](administration.md)
§"Rules").

Global settings live in a one-row table, `ai_settings`, like registration's
switch: `fill_in_after` (seconds, 30 to 600, default 60), `drain_minutes`
(default 120, §2), `global_seats` (AI seats at once, where a match between
two AIs takes two; 0 to 16, default 8; 0 stops every AI), `host_reserve` (slots each host keeps for people, default
2), `carousel_games` and `carousel_daily` (default 0, §7), `changed_at`,
`changed_by`.

*Weighed and set aside:* a new account `source` would touch every path that
reads it. A flag column on `accounts` would put AI state on the most-read
table and still need the settings somewhere else. A table of its own is
enough, and it keeps `accounts` and its grants as they are (§9).

## 2. How the runner signs in

*The owner's decision of 26.11.2026 replaced this note's first design, a
password per AI account.* AI accounts have no password. An administrator
creates each AI account on the website (§1) and issues an **AI runner
token** for each runner machine, bound to the AI accounts that machine may
play. The token follows the machine-credential pattern of report tokens
(migration 000052): 32 random bytes, shown once, only its SHA-256 digest
kept, at most 366 days of life, revocable, its issue and revocation audited.

- **No password, no recovery.** The creation path writes the account
  without an OPAQUE record anyone could use and without a recovery wrapper.
  Every password, reset, recovery, pairing and deletion flow refuses an AI
  account, so none of them can ever succeed for one.
- **The exchange.** On the native listener the runner trades a live token
  for a session of one of the token's accounts. That session is an ordinary
  native session with the same signed bearer an OPAQUE sign-in yields, so
  admission, the seat claim and the relay need no change. The session row
  names the token that minted it.
- **Up to four sessions.** An AI account may hold up to four minted native
  sessions at once; a fifth exchange ends the oldest. Each lives 30 minutes,
  is renewed, and ends at most 12 hours after its exchange. The runner rolls
  over: new offers go under a newer session while older ones finish their
  matches. No drain rule is needed, and no new sign-in ends a running match.
  Fill-in still seats an AI only through an offer whose session has
  `drain_minutes` left before its cap.
- **Revocation** ends every session the token minted, in the same
  transaction; each `sid` is revoked by the trigger, so the hosts drop those
  seats. Taking an account off a token's list ends that account's sessions
  of that token.
- **Transport.** The runner reaches Central's public native endpoint over
  TLS from any machine. There is no address allowlist and no file of
  passwords; the runner keeps its token in a root-only credential of its unit
  and its sessions in memory.

**What a thief gains.** With a token, a thief gets runner sessions for the
accounts the token lists until an administrator revokes it. A person could
then play people as "the AI", which is the worst case; it is bounded by the
token's accounts and ends at once on revocation. The thief cannot reach an
account the token does not list, a website session, the administration or
a person's account. *Recommended:* one token per machine, 90 days of life,
and a runner that alerts when its token is refused.

## 3. Fill-in

**The person's consent.** `replica::SeatRequest::Join` is the live join;
plain `Request::Join` only replays old tickets or answers `UpdateRequired`.
`SeatRequest::Join` gains one field:

```rust
#[serde(default)]
ai: AiOpponents, // none (the default) | local | any
```

- `local` accepts house and net AIs; the person's moves stay on the
  operator's servers.
- `any` also accepts language models. That is Stage 2: the person's moves
  then go to a provider (concept §"Joining a search", point 4).
- An absent field is `none`. A client built before this cannot show a badge,
  so it is never seated against an AI (invariant 2).

The wire default departs from the brief, which had a "humans only" field off
by default. The player sees no difference: the client's "Nur Menschen" switch
stays off by default and sends `local` (open question 1). A retry with the
same key and another `ai` is a `Conflict`, as for the other fields. The
choice is stored beside the ticket in `ranked_ticket_ai (account_id,
request_id)`, which references `ranked_tickets ON DELETE CASCADE` and holds
`accepts` (`local` or `any`). It is written in the same transaction as the
ticket; `none` writes no row.

**When.** There is no server tick. Pairing runs inside `queue::handle` on
Join and Status, and so does fill-in, right after `matchmaking::pair`. It runs
while the person's ticket is still `waiting` and:

1. the ticket accepts an AI;
2. it has waited `fill_in_after` (60 s) since `created_at`;
3. no other live waiting ticket of the same season, format and `replicable`
   exists. This is "people first", whatever kept `pair` from pairing: the
   radius, newcomer protection, `SKIP LOCKED` or a full host. A person who
   arrives during the countdown is paired as soon as the queue's own rules
   allow;
4. an AI is eligible and a host has room (below).

**Locks**, always in this order:

1. the person's ticket (`FOR UPDATE SKIP LOCKED`, then checked for `waiting`
   again);
2. the `ai_settings` row (`FOR UPDATE`), which serializes fill-in and the
   carousel against `global_seats`;
3. the chosen AI's `ai_players` row (`FOR UPDATE`), after which its live
   matches are counted, and its offer row (`FOR SHARE`, so the session's end
   waits);
4. the host, by `allocate`'s own locks.

`pair` locks tickets and then hosts. The administration locks
administrators' account rows and then the AI row. No path takes them the
other way round. Fill-in reads eligibility without locks first and takes
`ai_settings` only when an AI looks available. When several people wait, no
fill-in runs at all.

**Eligible AIs.** These are hard rules, not policy:

- the AI is enabled, and its kind is allowed by the ticket;
- it has accepted Central's current terms, as `Join` requires of people;
- it has a live offer for the ticket's format (§4), made under a session that
  is live and has `drain_minutes` left before its cap;
- its live matches are below `max_games`, counted after the lock;
- its matches today are below `daily_games`;
- all AI seats together are below `global_seats`.

If the chosen AI fails a check after its lock, fill-in tries the next one, at
most four in all.

**The choice is one swappable policy.** The SQL gathers the eligible AIs. A
pure function in a module of its own, `fill_in::policy::choose(person,
candidates) -> Option<usize>`, ranks them and has its own tests. The proposal
(concept §"Joining a search", point 3; open question 2):

- cost = |queue Elo gap| + 140 × min(meetings in the last six hours, 3).
  These are the queue's own constants (`matchmaking::preference`), so a
  person does not meet the same AI every time;
- among the candidates within 100 of the cheapest, the free kinds (`house`,
  `net`) come before `model`;
- then lower cost, fewer live matches, and the account id.

A language model is eligible only while its `daily_games` has room, which is
a hard rule. Other policies the owner might choose (the nearest only, a draw
among the near ones, house only) fit the same signature.

**The match.** One host, never a pair. One seat is the operator's own AI, so
a second operator protects nothing the person does not already trust the
first with, and production has a single owner. Allocation takes a compatible
single host, as the replicated path's fallback does today, and leaves
`host_reserve` slots free. The `ranked_matches` row is written as `pair`
writes it, with the AI's commitment from its offer. The seat order is drawn
for each match. `pair` puts the polling ticket in seat 0, which between
people is chance; with an AI it would always seat the person first.
`ai_matches (match_id, arranged, ai_seats)` records how the match was made
(`fill_in` or `carousel`) for the caps and the later stages. It references
`ranked_matches ON DELETE CASCADE`. The person's ticket turns `assigned` in
the same transaction, and the next reply is `Assigned`. A person who cancels
before claiming voids the match as today, which frees the AI's slot.

**The countdown.** Protocol 3 (§6) gives the `Waiting` reply an
`ai_fill_in_at`. It is `created_at + fill_in_after` while the ticket accepts
an AI and an enabled AI has a live offer for the format; otherwise it is
absent. It is the earliest time an AI may be seated, not a promise. The
client counts down to it, then shows the search again until `Assigned`. It
deliberately ignores whether other people are waiting: otherwise it would
tell one person about another person's ticket. The person's own poll is the
clock, so a client polls Status at least every 2 to 3 seconds near that time.

## 4. Several games at once

People's queue paths assume one match per account:

- the `ranked_single_ticket` index and the `LIMIT 1` in `queue::ticket`;
- the scalar subquery in `host::expire`, which fails when an account has two
  assigned tickets;
- `streams::assigned_match`, which takes the account's one assigned ticket and
  then compares it with the fence;
- `streams::held_sid`, where a new ranked `sid` deletes the session's other
  ranked `sid`s and so revokes the earlier match at its host;
- two unique rules of `gateway_sids` in the database: one ranked `sid` per
  session (`gateway_sids_one_ranked`, migration 000040) and one `sid` per
  session, gateway and purpose (the table's `UNIQUE`, migration 000024);
- one native session per account.

An AI seat is therefore never a ticket. `ranked_tickets`, its index and the
limit of 24 joins an hour stay people's alone, and an AI account's `Join` is
`Rejected`.

**Standing offers.** `ai_offers` has the primary key `(account_id, format)`
and these columns:

- `session_digest` references `sessions ON DELETE CASCADE`, so an offer ends
  with the session it was made under;
- `commitment` is the frozen deck, 32 bytes, as in Join;
- `renewed_at`;
- `expires_at` is `renewed_at` plus 45 seconds.

An AI has one deck per format. The runner renews every 15 seconds and changes
decks by renewing with another commitment. A match keeps the commitment it
was made with, so the runner keeps every deck it offered for at least the
2-minute reservation lease.

**The runner's requests.** `ranked::Request::Ai { request }` needs a native
session and is `Rejected` for an account without the flag. It has three
operations:

- `Offer { offers }` takes at most three (format, commitment) pairs and
  replaces the set. It is accepted only while the AI is enabled.
- `Work` answers with the account's AI settings (kind, profile, label,
  enabled, `max_games`), its live matches (`reserved` or `playing`, at most
  `max_games`), each as the `Assignment` a person gets, and when its offers
  expire. A disabled AI gets an answer too, so its running matches can end.
- `Carousel { format }` belongs to Stage 2 (§7).

The runner reads `Work` every two seconds while it offers, well inside a
session's 128 control requests a second ([native login](native-login.md)
§"Counted"). A push event `{"event":"ranked"}` on its socket could replace the
poll later. Clients built before such an event pass over it
([push socket](push-socket.md) §"Tables").

**Admission for each match.** For each assignment the runner does what the
native client does. It accepts the host's current terms through the terms
API, approves the host for this match (`Hosts`, `ApproveHosts`), and opens
the ranked socket with the match's fence (`RelayRoute` and the relay's
match-bound token). Three things change, each only for a flagged account:

- `assigned_match` resolves the match by the fence's `match_id`. It requires
  the account to sit in that match and applies the same host, epoch and lease
  checks, but needs no ticket. People keep the ticket path.
- `held_sid` keeps the session's ranked `sid`s of other live matches. It
  deletes only those of matches that have ended, which still publishes their
  revocations. With `max_games` at most 16, a session stays below
  `MAX_SIDS_PER_SESSION` (32).
- `gateway_sids` gains `ai_seat boolean NOT NULL DEFAULT false`, with a CHECK
  that only a `ranked` row has it. The table's `UNIQUE (session_digest, stream,
  purpose)` and `gateway_sids_one_ranked` become partial unique indexes
  `WHERE NOT ai_seat`. A new unique index on `(session_digest, ranked_match)
  WHERE ai_seat` allows one `sid` per match. `held_sid` sets `ai_seat` only
  for a flagged account, decided in the same transaction. Every row a person's
  session writes, and every row the previous binary writes, still falls under
  both rules as before.

The relay already allows several matches for one session: its capabilities
are scoped per session, fence, origin and seat. What bounds the runner are
the listener's 64 relay slots and the per-address limits it shares with push
sockets: 32 sockets and 60 dials a minute. All the runner's relays come from
one address, and a match between two AIs needs two. So the global bound
counts seats, not matches: `global_seats` (default 8, at most 16) leaves the
address room for the AI accounts' push sockets and the runner's redials after
a restart. Raising it beyond 16 needs a review of those limits.

**The bound.** `max_games` is counted after locking the AI's row, at every
match Central makes for it. `global_seats` is counted after locking
`ai_settings`.

## 5. The administrator's switch

The administration gains these browser-only `AdminRequest`s:

- `AiPlayers` (a read): every flagged account with its settings, live
  matches, games today, offers and last sign-in.
- `SetAiPlayer { account, settings }`: flags, changes or clears (`None`)
  under §1's rules.
- `SetAiEnabled { account, enabled }`: the switch.
- `AiSettings` (a read) and `SetAiSettings`.

Every change needs the recent password and writes its audit event.

Switching an AI off locks its row. A fill-in that holds the row commits first,
and every later one reads `enabled = false`. The switch touches no running
match: admission (the `assigned_match` branch) and the host's start and
finish do not ask for `enabled`, so the match ends normally. `Work` tells the
runner to stop offering. `global_seats = 0` stops every AI the same way.

Ending running matches as well needs the AI's sessions to end: an
administrator revokes the runner token, or takes the account off its list,
which ends the sessions it minted in the same transaction (§2). Switching off
needs no recent password (open question 6, recommended and taken).

## 6. Labelling

**The badge.**

```rust
pub struct AiBadge { pub kind: AiKind, pub label: String }
pub enum AiKind { House, Model, Net, #[serde(other)] Other }
```

`AiBadge` has no `deny_unknown_fields`, and `AiKind` has a catch-all
variant. A later stage can then add a field (a provider icon) or a kind
without another protocol step. This departs from the crate's rule on purpose,
for a display type that only Central sends; the push socket's table does the
same. The implementation pass proves the catch-all with a test.

**Where it goes.** Each new field is optional on the wire
(`#[serde(default, skip_serializing_if = …)]`):

- `ranked::Row.ai`
- `ranked::Response::Profile.ai`
- `profile::PublicProfile.ai`
- `SeatResponse::Participants.ai`, in seat order
- `Assignment.opponent_ai`
- `QueueState::Finished.opponent_ai`
- `QueueState::Waiting.ai_fill_in_at`
- the administration's account rows

A public board also takes a filter (owner, 26.11.2026, "proper statistics"):
`Request::Board.players` is `everyone` (absent on the wire, as before),
`people` or `ai`. Later stages add a runner's per-game report, a model
ranking and the administration's usage numbers (Baylee's concept,
§"Statistics and features"); `ai_matches` keeps one row per match with an AI
seat for them to hang off, and nothing here blocks them.

**Backward compatibility.** Every type in `central-api`'s ranked module has
`deny_unknown_fields`, and the Baylee client decodes with those types. A field
that appears only on an AI's row would still break an old client's decoding
of the whole board page. So the new fields go only to a caller that can read
them, through the native protocol header `Baylee-Central-Protocol`. Today it
must be exactly `2`, and anything else gets a 426. The change:

- Central accepts `2` and `3`, and central-http passes the version to the
  reads;
- a `2` caller's replies carry none of the new fields;
- `central-api`'s `PROTOCOL_VERSION` becomes `3`, so a build with the new pin
  sends it;
- the website is always current;
- when the owner retires `2`, a `2` caller gets the 426 as today.

**Honest names everywhere else.** Places without a badge, such as a protocol
`2` client's ladder or a gateway's own pages, show the display name. So an AI's
name says what it is too: it begins with the reserved prefix `KI-`, as in
`KI-Haus-Expert`, which fits the 16 ASCII characters `valid_display_name`
allows. `choosable` refuses that prefix and `AI-` to people (open question 3).
The concept's names ("Haus-KI · Expert", "Claude Haiku · Hoch") cannot be
display names; they are labels.

**Nothing else to label in Stage 1.** AI accounts have no social rows (§1),
no lobby presence at gateways (they get `ranked` admission only) and no Tower.
A stage that opens one of these adds the badge there first, including the
gateway grant's claims. A host learns nothing new in Stage 1. The freeze
rules of Stage 3 will tell the host which seat is an AI, which is public
anyway, and never the person's preference.

*Weighed and set aside:* a public "AI roster" read that clients match against
account ids. It would change no reply, but a client's copy can be stale, and
a stale copy shows an AI without its badge.

## 7. AI against AI

**The carousel** is Stage 2; this design keeps it possible. The runner sends
`Ai::Carousel { format }` for an AI with a live offer and a free slot. Central
pairs it with another eligible AI's offer for that format, by the same policy
and under the same locks. It does so only when:

- no person's ticket waits in that format;
- fewer than `carousel_games` AI-against-AI matches are running;
- fewer than `carousel_daily` were made today (UTC);
- every host keeps its reserve.

Language models count carousel games against their `daily_games`. The match
is recorded with `ai_matches.arranged = 'carousel'`. Scoring and repeat
damping are those of people (the third and fourth meeting in six hours at
half, later ones at a quarter; `rules::player::settle`).

**Calibration** is the carousel with high `carousel_games` and
`carousel_daily` and no fill-in, before people meet the AIs. Ratings carry
across seasons (`scoring::players::load` reads the newest row), so calibrated
AIs enter Season 1 at their rating. But a season has been open since
migration 000035, which started one at migration time with no end, and only
SQL can end a season or open a new one (open question 5).

## 8. Threats

| Threat | Control | Check for the adversarial pass |
| --- | --- | --- |
| A person makes their account look like an AI | Only an administrator flags an account (browser listener, recent password, audited). The badge comes from `ai_players` and never from a request. People cannot choose the prefixes `KI-` and `AI-`. | Every `Ai` request with a person's session is `Rejected`. A name change to either prefix is refused. No reply carries a badge for an unflagged account. |
| A stolen AI runner token: a person plays people as "the AI" | The token's account list, its lifetime, digest-only storage, an administrator's revocation that ends its sessions at once, the runner's alert, the audit | A revoked token mints nothing and its sessions are gone in the same transaction. A token never mints a session for an account it does not list. A fifth session ends the oldest. Password, reset and recovery flows refuse an AI account. |
| An AI passes as a person | The flag stays after the first match. `kind` and `profile` have no `UPDATE` grant. No own profile change, deletion, administrator role, social request or lobby admission. | Clearing after a match is refused. `SetAdmin` on a flagged account is refused. An AI account's Join, contact request, whisper and `admit` grant are refused. |
| A ticket that refused AIs meets an AI | AI accounts cannot Join. Fill-in reads the ticket's choice. The carousel pairs AIs only. As a second guard, `matchmaking::candidate` excludes flagged accounts. | Every path is tried with a `none` ticket and an eligible AI: no match. An absent field decodes as `none`. |
| An old client gets an AI without a badge | An absent `ai` is `none`. Protocol `2` replies carry no new field, and AI names carry the prefix. | A protocol `2` Join waits past 60 s and gets no AI. A protocol `2` board decodes with the old types. |
| Farming a fixed-strength AI | The repeat cost in the choice, the repeat damping (half, then a quarter), the AI's rating falling as it loses, one ticket and 24 joins an hour, a fixed profile, the switch | Policy tests: a person who keeps meeting one AI is offered another when one is near. Settlement tests for repeat damping with an AI seat. |
| Exhausting the AIs | One ticket per person, 24 joins an hour, `max_games`, `global_seats`, a cancel before the claim freeing the slot, the carousel yielding to people, registration's own limits | Parallel fill-ins race for the last slot and exactly one wins. `max_games` holds under concurrency. |
| Exhausting hosts or relays with AI matches | `host_reserve`; `global_seats` at most 16 against 64 relay slots and 32 sockets an address | Fill-in is refused when a host would fall below its reserve. A match between two AIs counts two seats. |
| A dead runner leaves a person stranded | Offers live 45 s. A match the AI never claims becomes void after the 2-minute lease, as today. | Offer expiry. The void and the person's next search. |
| The switch races a fill-in | The switch locks the AI row, and fill-in counts after the same lock. | Interleaved switch and fill-in: no match after the switch commits. |
| The AI flag or a person's preference leaks | Public: kind and label. The administration only: profile, caps, who flagged. A person's preference lives only in `ranked_ticket_ai`, in no reply but their own, in no host's work and no public view. `ai_fill_in_at` ignores other people's tickets. | Replies to the opponent, host work and public reads hold no preference. The host's assignment is unchanged. |
| A fence names another match | The AI branch of `assigned_match` requires the account to sit in the match. `owned` and `RelayRoute` check it as today. | A foreign match's fence is refused. |
| An AI in a protected (replicated) match | Fill-in and the carousel never create `ranked_replicas`. The replicated rules are unchanged. | AI matches are never protected. |

**Single host, single owner.** Production runs single-host with one owner,
who runs both the host and the AI. A person playing an AI trusts that owner
either way. The privacy notice says so when AI opponents arrive, and Stage 2
adds the provider transfer for language models.

**What is kept.** A person's choice per search is stored with the ticket
(`ranked_ticket_ai`) and goes with it by cascade. Flagged accounts, their
offers and `ai_matches` go with the account or the match. The audit keeps ids
only. The privacy notice's next version names the per-search choice; this
note asserts no legal basis.

## 9. Migrations, grants, API and the game repository

**The migration**, `m2026MMDD_000053_ai_players` (the next free number when it
is written), creates `ai_players`, `ai_settings` (with its one row),
`ai_offers`, `ranked_ticket_ai` and `ai_matches`. Of the existing tables it
changes only `gateway_sids`: the `ai_seat` column with its default and the
unique indexes of §4. It changes no grant of an existing table. The runtime's
`INSERT` on `gateway_sids` covers the whole table, and the new column gets no
`UPDATE`. The previous binary's startup check names the tables it knows and
their exact grants, and as far as that check shows, it passes over new
tables. So the deploy hook's fallback to that binary should need no rollback
of grants, which after migration 000014 it did ([operations](operations.md)).
The implementation pass runs the previous binary over the migrated schema to
prove it. The migration is forward-only. Like every migration it is tested for a fresh install, an upgrade from
000052, a failing DDL and privileges
([database development](database-development.md)).

**The runtime's grants**, exact, in `ops/grants.sql` and the startup check
(`accounts/grants.rs`):

| Table | Grant |
| --- | --- |
| `ai_players` | `SELECT, INSERT, DELETE`; `UPDATE (label, enabled, max_games, daily_games, changed_at, changed_by)`. `kind`, `profile`, `set_at` and `set_by` are never updated. |
| `ai_settings` | `SELECT`; `UPDATE` of every column except the key |
| `ai_offers` | `SELECT, INSERT, DELETE`; `UPDATE (session_digest, commitment, renewed_at, expires_at)` |
| `ranked_ticket_ai` | `SELECT, INSERT` |
| `ai_matches` | `SELECT, INSERT` |

**Central's crates:**

- `central-api`:
  - `AiOpponents` on `SeatRequest::Join`;
  - `ranked::Request::Ai` with `Offer`, `Work` and `Carousel`;
  - `AiBadge` and `AiKind` and the fields of §6;
  - the `AdminRequest`s of §5 and their answers;
  - `PROTOCOL_VERSION` `3`.
- `central-postgres`:
  - `queue.rs`: the fill-in call and the refusal of an AI's Join;
  - `matchmaking.rs`: flagged accounts left out of `candidate`, a second
    guard;
  - `ranked/fill_in/`: the eligibility query and `policy.rs`;
  - `ranked/ai.rs`: the runner's requests;
  - `streams.rs`: the `assigned_match` and `held_sid` branches of §4;
  - `admin.rs`: the requests and the `Action`s;
  - `boards/standings.rs`, `boards.rs`, `profiles.rs` and `participants.rs`:
    the badge;
  - the refusals of §1;
  - `grants.rs`.
- `central-http`: protocol `2` and `3`, the version passed to the reads, and
  and the runner token's exchange.
- `central-client`: the runner's calls and Join's `ai`.
- `central-web`: the administration's AI page, and the badge on the player
  page and the boards.
- Docs: `ranked.md`, `administration.md`, `operations.md` (the credential
  runner tokens, seasons), the draft of the privacy notice, and the
  threat model.

**The game repository** pins `central-api` and `central-client` by git
revision. It changes in step:

- The pin bump comes as a step of its own and brings the new
  `PROTOCOL_VERSION`. Struct literals of the changed types need the new
  fields.
- Client:
  - Join's `ai`: `local`, or `none` when "Nur Menschen" is on;
  - the countdown from `ai_fill_in_at`;
  - badges from `opponent_ai`, `Participants.ai`, `Row.ai` and
    `Profile.ai`.
- Seat agent: the ranked runner. It exchanges its runner token for a
  session per AI account, renews it and rolls over to a new one before the
  cap. It offers decks and polls `Work`. For
  each match it accepts the host's terms, approves the host and opens the
  ranked socket. It reconnects after a restart.
- The gateway and the engine server change nothing in Stage 1. An AI seat is
  an ordinary ranked seat.
- Order:
  1. Central: the migration, the grants, and a binary that accepts `2` and
     `3`.
  2. The pin bump and a client release.
  3. The operator registers the AI account. An administrator flags, names
     and enables it.
  4. The runner starts.

## 10. Open questions for the owner

1. **The wire default.** An absent `ai` on Join means "no AI", so an old
   client never meets an AI it cannot label, against the brief's "humans
   only, default off". The client's switch keeps its default. *Recommended:*
   as designed.
2. **The fill-in choice** (still open in the concept). *Recommended:* the
   proposal of §3: queue distance plus 140 per recent meeting, free kinds
   first within 100, a model only while its daily cap has room. It is one
   function and can be swapped.
3. **AI display names.** *Recommended:* the prefix `KI-` for every AI's name,
   with `KI-` and `AI-` reserved from people. The concept's names become
   labels.
4. **How the runner signs in.** *Decided by the owner, 26.11.2026:* AI
   runner tokens per machine, no passwords, no address allowlist (§2).
5. **Calibration and seasons.** A season has been open since migration
   000035, and no tool ends one or opens another. *Recommended:* calibrate in
   the current test season with fill-in off. Then an operator command
   (reviewed, Stage 2) ends it and opens Season 1. The AIs' ratings carry
   over as everyone's do.
6. **A recent password to switch an AI off.** Every administration change
   needs one today. *Recommended:* exempt switching off (one AI, or
   `global_seats = 0`). It only reduces exposure and is still audited.
   Switching on, flagging and every other change keep the rule.
7. **Starting numbers.** *Recommended* until the speed check and load
   measurements exist: `max_games` 4 for house levels, `global_seats` 8,
   `host_reserve` 2, `drain_minutes` 120, `fill_in_after` 60.
8. **Retiring protocol `2`.** *Recommended:* one client release after the
   badges ship. Until then AI names carry the prefix.

## Where the concept and the code disagree

- The concept's names ("Haus-KI · Expert", "Claude Haiku · Hoch") fail
  `valid_display_name`, which allows 3 to 16 ASCII letters, digits, `_` and
  `-`. They become labels (§6).
- The concept gives an AI account several games at once. The code allows one
  native session per account, and a new sign-in revokes its gateway `sid`s.
  It allows one ranked `sid` per session, in `held_sid` and in two unique
  indexes of `gateway_sids`, and one ticket, and some queries assume a single
  assigned ticket. Hence §4's offers, its three changes and §2's rolling
  runner sessions.
- The concept says the 60 s match the queue's own schedule. That holds for a
  quiet queue only: a busy queue opens fully at 120 s, and newcomer
  protection holds back a newcomer more than 350 points away for 10 s. The
  people-first condition of §3 makes "a person who arrives during the
  countdown takes precedence" hold either way.
- The concept calibrates "before Season 1 opens". A season has been open
  since migration 000035, and no tool manages seasons (open question 5).
- An additive reply field is not backward compatible here, because the
  ranked wire types have `deny_unknown_fields` (§6).
- The relay's 64 slots and per-address limits bound how many AI seats run at
  once, which the concept's carousel and several-games plans must fit.
- The concept has admins switch an AI off at once. Switching off works at
  once for new matches. Ending an AI's running matches needs its sessions to
  end, which revoking the runner token does (§5).

## Implementation pass, 26 November 2026 (in progress)

Built on branch `claude/ai-players`, Stage 1 without the runner token:

- **Migration 000053** and its exact grants (`ops/grants.sql`, the startup
  check). The previous binary runs over the migrated schema: its own store
  suite, at the commit with the migration and the grants alone (the store and
  its startup check as before): see "Evidence" below.
- **`central-api`**: `AiOpponents` on `SeatRequest::Join`, `BoardPlayers` on
  `Request::Board`, `ranked::ai` (`AiBadge`, `AiKind`, `Offer`, the runner's
  `Request` and `Response`), the badge fields of §6, the administration's
  AI requests, and `PROTOCOL_VERSION` 3 with `PROTOCOL_VERSIONS` 2 and 3.
  `wire::Response::omit_ai` names every answer, so a new one does not
  compile until it is decided; a test decodes every changed reply, sent down
  the protocol 2 path, with frozen copies of protocol 2's types.
- **`central-http`**: serves 2 and 3, and leaves out what 3 added for a
  caller of 2.
- **`central-postgres`**: fill-in after `pair` with the policy as one pure
  function (`ranked/fill_in/policy.rs`); offers and `Work`; the AI branches
  of `assigned_match` and `held_sid` with `gateway_sids.ai_seat`;
  `max_games`, `global_seats` and `host_reserve` counted after their locks;
  the switch, the settings and their audit; the badges on boards, profiles,
  participants, assignments and receipts; the board filter; the refusals of
  an AI's Join, browser sign-in, lobby admission and administrator role.
- **`central-client`**: `Session::ai` for the runner, and Join's `ai`.

Not yet built: the runner token (§2: issuance, exchange, up to four
sessions, revocation), the creation path for AI accounts (until it lands,
an administrator flags an enrolled account that has never played; the
owner's decision makes that path go), the refusals of social requests,
deletion, Tower, password, reset and recovery for AI accounts, the website's
administration page and badges, and the operator docs.

**Evidence (26.11.2026).** CI run 45866255892, on the tip before the
rebase onto `main` (`691b168e`; `85c8fc7c` after it), passed `postgres`
(the whole store suite with the AI path, on Linux), `lint`, `test`, `wasm`,
`build-release`, `verify`, `dependencies`, `secrets` and `web`; `site` was
still running when the next push cancelled it. On the owner's Mac the store
suite passed every existing check and the AI path once its deadline was
raised locally (the five minutes there include the build); other runs there
stopped at timing-sensitive checks under a shared CPU (a clock read 179 s
for 180 s), not at anything of this change. The previous binary over the
migrated schema: at the commit with the migration and the grants alone
(`11d20bc7` after the rebase), its startup check accepted the schema and its
store suite ran green up to that same clock check. That run used the
migration before `fair` was removed; a full green run is still owed and is
the next step's first check.
