# AI players

*Design pass of 26 November 2026. The implementation pass is under way; what it built is recorded at the end.*

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

1. Only an administrator creates an AI account, always as a new account
   with no password; no path makes a person's account an AI. An
   administrator removes one only before its first ranked match. An AI
   account cannot change its own kind, profile, label or name, and no flow
   gives it a password.
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
| `set_at`, `set_by` | when the AI was created and by which administrator (`NULL`: the operator) |
| `changed_at`, `changed_by` | the last change |

**Creation** (`CreateAiPlayer`, `create_ai_player`) is the only way an
account becomes an AI. One transaction, for an administrator whose password
sign-in is under 30 minutes old, writes the account, its `ai_players` row
(switched off) and the audit event:

- `source = 'registered'` and a login name `ai-` with 20 hex digits from the
  operating system's generator. Nobody signs in with it, nobody can
  guess it, and only the administration's account rows show it.
- The **sealed credential**: the fixed bytes `baylee:ai:sealed`
  (`SEALED_CREDENTIAL` in `accounts/ai/lifecycle.rs`), 16 bytes. An OPAQUE
  record has the fixed length of a registration upload, and
  `enrollment_record` refuses any other, so no password is ever proven
  against it (a unit test holds this).
- A master key wrapper and a recovery wrapper of 73 random bytes each, and
  a document of 41 random bytes at revision 1, each with the version byte
  1. Nothing opens them.
- `restored = true`. The ranked `actor`, ranked admission
  (`issuing_session`) and fill-in's offer check all require it, and no
  client of the AI ever restores a vault.
- The display name and its time, and the administrator's tag or one drawn
  as registration draws one. A tag another account holds is `Conflict`; a
  drawn one is never put in its place.

So every CHECK of `accounts` holds as for an enrolled registered account
(`account_enrollment_atomic`, `account_recovery`,
`account_registered_pending`, the name, tag and reset rules), and nothing
has to tell an AI from a person to stay correct.

The transactions that change or remove an AI lock the account row first,
for no key update, and then the AI's row. An offer takes them in the same
order. A fill-in takes the AI's row and then only shares the account row's
key when it writes the match, which a no-key-update lock allows, so the
fill-in commits first. Each checks:

- **Remove only before the first match.** `RemoveAiPlayer` removes an AI
  that has no `ranked_matches` row in any state and no ticket that is not
  closed. Its account goes as a deletion takes one (`deletion::remove`),
  with everything it holds by cascade, its runner sessions and their
  `sid`s among them. The sessions' digests reach the push sockets as a
  revocation's do. After the first match the AI stays for the account's
  life, and an administrator switches it off instead (`Conflict`).
  Otherwise an AI's record could disappear from the ladder.
- **The mind is fixed.** `kind` and `profile` never change (`Conflict`).
  A different mind is a different account, because the name is the
  measurement (concept §"Who plays"). Before the first match an
  administrator removes the AI and creates another. `label`, `max_games`,
  `daily_games`, `enabled` and the display name may change.
- **Fair only.** Every ranked AI is a fair remote seat (owner, 26.11.2026):
  a profile that sees hidden cards (`house:seer`) is refused, by the code and
  by a CHECK in the table, and the badge says nothing of fairness.
- **No administration through an AI.** `SetAdmin` and the operator's
  appointment refuse an AI account.
- **The administrator owns the AI's public face.** The display name is set
  at creation and begins with the reserved prefix of §6. An AI's own
  requests are its runner's alone (§2, "What a runner session may do"). An
  administrator may change its tag. The operator's erasure stays.

Every change writes its audit event in its transaction, under new `Action`s
in the existing pattern: `ai.created`, `ai.removed`, `ai.enabled`,
`ai.disabled`, `ai.changed` and `ai.settings_changed`. Each holds the actor and
the account's id. Its `detail` holds numbers only (`{"max_games": 4}`),
never the label or a name ([administration](administration.md)
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
play. The token follows the machine credentials before it: a gateway's token
(migration 000014) and a report token (migration 000052).

**The token.** 32 bytes from the operating system's generator
(`Token::generate`), shown once as `bat_` and 64 lowercase hex digits
(`central_api::ai_runner::token_text`), only its SHA-256 digest kept. It has
a name (1 to 64 bytes), lives 1 to 366 days (the page offers 90), lists 1 to
16 AI accounts, and is revocable. At most 32 are live at once and 32 are made
in a day. Its making and its revocation are audit events
(`ai.runner_token_created`, `ai.runner_token_revoked`) that name it by id in
`audit_events.ai_runner_token_id`, never by name or digest. It does not die
with its maker's role, unlike a report token: it plays AI accounts and opens
no administration, every administrator sees and revokes every token, and a
runner should not stop because the administrator who set it up stepped down.

**No password, no recovery.** The creation path writes the account with a
sealed credential: fixed bytes that no OPAQUE record parses, a random master
key wrapper, recovery wrapper and document that nothing can open, and an
unguessable login name (`ai-` and 20 hex digits). So every check that asks for
an enrolled account (`credential IS NOT NULL`, the revision, the table's
CHECKs) holds unchanged, and nothing has to tell an AI from a person to stay
correct (§1). On top of that the store answers an AI account's login name
and credential as it answers an unknown name, and every password, reset,
recovery, address, pairing and deletion flow refuses an AI account (below),
so none of them can ever give one a password.

**What a runner session may do.** A session of an AI account makes its
runner's requests and nothing else. One exhaustive `match` decides every
`wire::Request` (`ForAi::of` in `central-identity/src/ai_gate.rs`), down to
the ranked, seat and AI requests and the subject of a terms acceptance, so
a new request does not compile until it is decided there. Allowed:

- its session: `SessionStatus`, `RenewSession`, `Logout`;
- the terms: `Terms`, `GatewayTerms`, `TermsStatus`, `AcceptTerms` (Central's
  and a host's);
- ranked play: `Ai` (`Offer`, `Work`), the queue `Status`, `Board`,
  `Profile`, `OwnProfile`, `Seasons`, and of a seat `Relay`, `RelayRoute`,
  `Hosts`, `ApproveHosts`, `DeclineHost`, `Participants`, `Presence` and
  `Status`;
- ranked admission: `GatewayToken` and `RankedToken`. The store gives an AI
  the `ranked` purpose alone.

Requests that act on no session (public reads, a gateway's, host's,
Tower's or the bridge's credential, the sessionless sign-in, discovery,
registration, reset and pairing flows) are not the gate's; where they name
an account, the store refuses an AI there. Everything else is refused with
`Rejected`: its own profile, pictures and badges, its deletion, social
requests, push subscriptions and notification settings, presence, Tower,
tasks and events, a password change, the e-mail address, recovery and its
backup mail, the key check, the export of its account and profile,
pairing, its document and objects, gateways of its own, moves, a person's
ranked `Join`, a deck preview, the ranked export, and a replicated seat's
`Submit`, in which no AI is ever seated.

Where it is enforced, so that people pay nothing for it:

- A request that needs Central's terms reads them in one statement
  (`session_gate`) that also says whether the account is an AI. The AI is
  refused first, also while no terms are in force.
- A refused request that reads no terms asks in a statement of its own
  before anything is done for it, on the native listener. The browser
  listener needs none: no AI account holds a browser session. The store
  issues none for one (`issue_session` refuses any AI), a browser sign-in
  finds no credential, a refresh only rotates a browser session that
  exists, and a pairing begins with one.
- The library and a presence report are refused where they are served, as
  people make them often. The statement that finds an object's owner
  (`OBJECT_ACCESS`) refuses an AI. A report is kept only for a push socket,
  and an AI's session opens none: the dial asks in the same statement.
- The store refuses an AI in the sessionless flows: `discover` and
  `credential` answer as for an unknown name, so a sign-in looks like one
  on a name that does not exist. Every reset code, an administrator's
  (`ResetLogin`), a mailed one and its use with or without the recovery
  code, is made and used only for a `RESETTABLE` account, which excludes an
  AI. `issue_session` refuses an AI, `SetAdmin` and the operator's
  appointment refuse one, and `SetTag` is allowed.

**The terms.** A created AI has accepted nothing. Its runner accepts
Central's terms itself with its session (`TermsStatus`, then `AcceptTerms`)
before it offers: `Offer` needs them, as `Join` does for people, and
fill-in seats an AI only through a live offer, so an AI that has not
accepted the terms in force is never seated. It accepts a host's terms for
each match the same way, as §4 says.

**The exchange.** On the native listener, without an `Authorization`
header, the runner sends `wire::Request::AiRunnerSession { token, account }`
and gets `wire::Response::AiRunnerSession { token, identity, expires_at }`:
an ordinary native session and the same signed bearer an OPAQUE sign-in
yields, so admission, the seat claim and the relay need no change. In one
transaction, in this order:

1. the token row by digest, live (not revoked, not expired) and listing
   `account`, `FOR NO KEY UPDATE`; the fetched digest is compared with the
   presented one in constant time as well. A revocation waits for the
   exchange and then ends its session too; an exchange that waited for a
   revocation finds no row. Not `FOR SHARE`: step 5 updates the same row, and
   two exchanges of one token that each held a share would wait for each
   other there. So exchanges of one token run one after the other;
2. the account row `FOR UPDATE`, which must be enrolled and an AI player.
   `enabled` is not asked: a switched-off AI still rolls over to end its
   running matches (§5);
3. the account's runner sessions: expired ones are deleted, and while four
   live ones remain the oldest is deleted;
4. the new session, `kind = 'native'`, `runner_token` set, 30 minutes;
5. the token's `last_used_at`, the account's `last_sign_in_at` (the page's
   "last runner sign-in") and a row in `ai_runner_signins` (token, account,
   time), the exchange's audit, kept 90 days and pruned by the exchange.

Every refusal is the same `Rejected`: unknown, revoked or expired token, an
account it does not list, an account that is no AI any more. The sessions
ended in step 3, and those a revocation ends, are handed to the push sockets
as ended, as a sign-out's are. Budgets: 60 exchanges an hour from one address,
and after 30 refused ones in an hour the store is not asked again from it;
12 successful exchanges an hour for one account, so a thief cannot churn the
runner's sessions faster than that.

**Up to four sessions.** An AI account may hold up to four minted native
sessions at once (`central_api::ai_runner::MAX_SESSIONS`); a fifth exchange
ends the oldest. The table's one-native-session rule
(`sessions_one_per_account`) becomes a partial index over the sessions no
token minted, so every person's session and every row the previous binary
writes keep it. Each runner session lives 30 minutes, is renewed, and ends at
most 12 hours after its exchange. The runner rolls over: new offers go under
a newer session while older ones finish their matches. No drain rule is
needed, and no new sign-in ends a running match. Fill-in still seats an AI
only through an offer whose session has `drain_minutes` left before its cap.

**Revocation** ends every session the token minted, in the same transaction;
each `sid` is revoked by the trigger, so the hosts drop those seats. It needs
no fresh password, like switching an AI off: it only takes exposure away, and
it is audited. A token's list is fixed when it is made; to change it, an
administrator makes a new token and revokes the old one. Removing an AI
account ends its sessions by cascade.

**Transport.** The runner reaches Central's public native endpoint over
TLS from any machine. There is no address allowlist and no file of
passwords; the runner keeps its token in a root-only credential of its unit
and its sessions in memory. In `central-client` it calls
`ai_runner_session(transport, &token, account)`, which answers a `Session`
without a master key: the operations that need one (the document, objects,
chat keys, the vault key, a password change, pairing) refuse.

**What a thief gains.** With a token, a thief gets runner sessions for the
accounts the token lists until an administrator revokes it. A person could
then play people as "the AI", which is the worst case; it is bounded by the
token's accounts and ends at once on revocation. The thief cannot reach an
account the token does not list, a website session, the administration, a
person's account, or a password, address or recovery code for the AI
account. *Recommended:* one token per machine, 90 days of life, and a runner
that alerts when its token is refused.

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
administrators' account rows, then the AI's account row for no key update
when it changes or removes one, then the AI row (§1). No path takes them the
other way round, with one exception, older than the creation path and
not changed by it: a fill-in for a person who is an administrator shares
the key of that person's account row when it writes the match, a row the
administration holds `FOR UPDATE` while it changes an AI. PostgreSQL then
ends one of the two as a deadlock, which answers `Unavailable` once (open,
below). The match fill-in writes only shares the AI's own account key,
which the no-key-update lock allows. Fill-in reads eligibility without locks first and takes
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
session and is `Rejected` for a person's account. It has three
operations:

- `Offer { offers }` takes at most three (format, commitment) pairs and
  replaces the set. It is accepted only while the AI is enabled.
- `Work` answers with the account's AI settings (kind, profile, label,
  enabled, `max_games`), every live match (`reserved` or `playing`), each as
  the `Assignment` a person gets, and when its offers expire. The list is
  bounded by the hard maximum of 16 (`MAX_GAMES`), not by `max_games`: an
  administrator who lowers `max_games` below the live matches does not hide
  any, and the client checks the same bound. `Work` locks nothing; only
  `Offer` takes the AI row, so that a switch-off waits for it and then
  withdraws what it stored. A disabled AI gets an answer too, so its running
  matches can end.
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
match-bound token). Three things change, each only for an AI account:

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
  for an AI account, decided in the same transaction. Every row a person's
  session writes, and every row the previous binary writes, still falls under
  both rules as before.

The relay already allows several matches for one session: its capabilities
are scoped per session, fence, origin and seat. What bounds the runner are
the listener's 64 relay slots and the per-address limits it shares with push
sockets: 32 sockets and 60 dials a minute. All the runner's relays come from
one address, and a match between two AIs needs two. So the global bound
counts seats, not matches: `global_seats` (default 8, at most 16) leaves the
address room for the runner's redials after a restart. AI accounts hold no
push socket (§2). Raising it beyond 16 needs a review of those limits.

**The bound.** `max_games` is counted after locking the AI's row, at every
match Central makes for it. `global_seats` is counted after locking
`ai_settings`.

## 5. The administrator's switch

The administration gains these browser-only `AdminRequest`s:

- `AiPlayers` (a read): every AI account with its settings, live matches,
  games today, offers and its runner's last sign-in.
- `CreateAiPlayer { settings, tag }`: a new AI account (§1), answered with
  `AiPlayerCreated { account, tag }`.
- `SetAiPlayer { account, settings }`: changes the label, the caps and the
  display name; the kind and the profile stay.
- `RemoveAiPlayer { account }`: before its first match only (§1).
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
| A person makes their account look like an AI | Only an administrator creates an AI, always as a new account with no password (browser listener, recent password, audited); no path makes a person's account one. The badge comes from `ai_players` and never from a request. People cannot choose the prefixes `KI-` and `AI-`. | Every `Ai` request with a person's session is `Rejected`. A name change to either prefix is refused. No reply carries a badge for a person's account. |
| A stolen AI runner token: a person plays people as "the AI" | The token's account list, its lifetime, digest-only storage, an administrator's revocation that ends its sessions at once, the runner's alert, the audit; a runner session makes its runner's requests alone, so no address, reset, recovery or password outlives the revocation (§2) | A revoked token mints nothing and its sessions are gone in the same transaction. A token never mints a session for an account it does not list. A fifth session ends the oldest. Password, reset and recovery flows refuse an AI account. |
| An AI passes as a person | An AI stays after its first match. `kind` and `profile` have no `UPDATE` grant. No own profile change, deletion, administrator role, social request or lobby admission. | Removal after a match is refused. `SetAdmin` on an AI account is refused. An AI account's Join, contact request, whisper and `admit` grant are refused. |
| A ticket that refused AIs meets an AI | AI accounts cannot Join. Fill-in reads the ticket's choice. The carousel pairs AIs only. As a second guard, `matchmaking::candidate` excludes AI accounts. | Every path is tried with a `none` ticket and an eligible AI: no match. An absent field decodes as `none`. |
| An old client gets an AI without a badge | An absent `ai` is `none`. Protocol `2` replies carry no new field, and AI names carry the prefix. | A protocol `2` Join waits past 60 s and gets no AI. A protocol `2` board decodes with the old types. |
| Farming a fixed-strength AI | The repeat cost in the choice, the repeat damping (half, then a quarter), the AI's rating falling as it loses, one ticket and 24 joins an hour, a fixed profile, the switch | Policy tests: a person who keeps meeting one AI is offered another when one is near. Settlement tests for repeat damping with an AI seat. |
| Exhausting the AIs | One ticket per person, 24 joins an hour, `max_games`, `global_seats`, a cancel before the claim freeing the slot, the carousel yielding to people, registration's own limits | Parallel fill-ins race for the last slot and exactly one wins. `max_games` holds under concurrency. |
| Exhausting hosts or relays with AI matches | `host_reserve`; `global_seats` at most 16 against 64 relay slots and 32 sockets an address | Fill-in is refused when a host would fall below its reserve. A match between two AIs counts two seats. |
| A dead runner leaves a person stranded | Offers live 45 s. A match the AI never claims becomes void after the 2-minute lease, as today. | Offer expiry. The void and the person's next search. |
| The switch races a fill-in | The switch locks the AI row, and fill-in counts after the same lock. | Interleaved switch and fill-in: no match after the switch commits. |
| Whether an account is an AI, or a person's preference, leaks | Public: kind and label. The administration only: profile, caps, who created it. A person's preference lives only in `ranked_ticket_ai`, in no reply but their own, in no host's work and no public view. `ai_fill_in_at` ignores other people's tickets. | Replies to the opponent, host work and public reads hold no preference. The host's assignment is unchanged. |
| A fence names another match | The AI branch of `assigned_match` requires the account to sit in the match. `owned` and `RelayRoute` check it as today. | A foreign match's fence is refused. |
| An AI in a protected (replicated) match | Fill-in and the carousel never create `ranked_replicas`. The replicated rules are unchanged. | AI matches are never protected. |

**Single host, single owner.** Production runs single-host with one owner,
who runs both the host and the AI. A person playing an AI trusts that owner
either way. The privacy notice says so when AI opponents arrive, and Stage 2
adds the provider transfer for language models.

**What is kept.** A person's choice per search is stored with the ticket
(`ranked_ticket_ai`) and goes with it by cascade. AI accounts, their
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

**The runner token's migration**, `m20261126_000054_ai_runner_tokens` (§2),
creates three tables:

- `ai_runner_tokens`: `id` (UUIDv7), `name` (1 to 64 bytes), `digest` (32
  bytes, unique), `created_by` (an account id without a reference),
  `created_at`, `expires_at` (at most 366 days after it), `revoked_at`,
  `revoked_by` and `last_used_at`;
- `ai_runner_accounts (token_id, account_id)`: a token's fixed list, gone
  with the token or with the account's `ai_players` row;
- `ai_runner_signins`: one row per exchange (token, account, time), indexed
  by token and time for the pruning and by account and time for reading.

Of the existing tables it changes two. `sessions` gains `runner_token`, with
a foreign key `(runner_token, account_id)` into `ai_runner_accounts`
(`ON DELETE CASCADE`), a CHECK that only a native session has one, and an
index; `sessions_one_per_account` (`UNIQUE (account_id, kind)` since
migration 000007) becomes partial, `WHERE runner_token IS NULL`.
`audit_events` gains `ai_runner_token_id`. No grant of an existing table
changes: `sessions` and `audit_events` have table-wide grants, which cover a
new column. The previous binary writes sessions without the column, so they
fall under the old rule, and its startup check passes over the new tables.

**The runtime's grants**, exact, in `ops/grants.sql` and the startup check
(`accounts/grants.rs`):

| Table | Grant |
| --- | --- |
| `ai_players` | `SELECT, INSERT, DELETE`; `UPDATE (label, enabled, max_games, daily_games, changed_at, changed_by)`. `kind`, `profile`, `set_at` and `set_by` are never updated. |
| `ai_settings` | `SELECT`; `UPDATE` of every column except the key |
| `ai_offers` | `SELECT, INSERT, DELETE`; `UPDATE (session_digest, commitment, renewed_at, expires_at)` |
| `ranked_ticket_ai` | `SELECT, INSERT` |
| `ai_matches` | `SELECT, INSERT` |
| `ai_runner_tokens` | `SELECT`; `INSERT (name, digest, created_by, created_at, expires_at)`; `UPDATE (revoked_at, revoked_by, last_used_at)`. No token is deleted. |
| `ai_runner_accounts` | `SELECT, INSERT` |
| `ai_runner_signins` | `SELECT, INSERT, DELETE` |

**Central's crates:**

- `central-api`:
  - `AiOpponents` on `SeatRequest::Join`;
  - `ranked::Request::Ai` with `Offer`, `Work` and `Carousel`, and
    `MAX_GAMES` (16), the bound of `max_games` and of `Work`'s list;
  - `AiBadge` and `AiKind` and the fields of §6;
  - the `AdminRequest`s of §5 and their answers, `CreateAiPlayer`,
    `SetAiPlayer` and `RemoveAiPlayer` among them;
  - `PROTOCOL_VERSION` `3`;
  - `ai_runner`: the token's text form (`bat_`), its bounds
    (`MAX_SESSIONS` 4, `MAX_ACCOUNTS` 16, `LIVE_TOKENS` 32, `TOKENS_PER_DAY`
    32, `LISTED_TOKENS` 64, `EXCHANGES_PER_HOUR` 12), `validate_new` and
    `AiRunnerTokenInfo`;
  - `wire::Request::AiRunnerSession` and `wire::Response::AiRunnerSession`;
  - `AdminRequest::AiRunnerTokens`, `CreateAiRunnerToken` and
    `RevokeAiRunnerToken`, answered with `AdminAnswer::AiRunnerTokens` and
    `AiRunnerTokenCreated`.
- `central-postgres`:
  - `queue.rs`: the fill-in call and the refusal of an AI's Join;
  - `matchmaking.rs`: AI accounts left out of `candidate`, a second
    guard;
  - `ranked/fill_in/`: the eligibility query and `policy.rs`;
  - `ranked/ai.rs`: the runner's requests;
  - `streams.rs`: the `assigned_match` and `held_sid` branches of §4;
  - `admin.rs`: the requests and the `Action`s;
  - `ai/lifecycle.rs`: an AI account's creation (the sealed credential),
    its change and its removal;
  - `terms.rs`: `session_gate`, the terms and whether the account is an AI
    in one statement;
  - `boards/standings.rs`, `boards.rs`, `profiles.rs` and `participants.rs`:
    the badge;
  - the store's refusals of §2: `discover`, `credential`, `issue_session`,
    `RESETTABLE` (every reset), `OBJECT_ACCESS`, `appoint_admin`;
  - `ai_runner.rs`: the exchange, and the administration's list, making and
    revocation of runner tokens;
  - `grants.rs`.
- `central-identity`: the exchange's dispatch (native listener, no bearer),
  each AI account's 12 exchanges an hour, the administration's AI and
  runner token requests (a removal's sessions told to their push sockets),
  and `ai_gate.rs`: what a runner session may do (§2), applied in
  `dispatch`, `require_terms`, the push socket's dial and a picture's
  admission.
- `central-http`: protocol `2` and `3`, the version passed to the reads, and
  the exchange's address budgets (60 an hour, and no store asked after 30
  refusals in an hour).
- `central-client`: the runner's calls, Join's `ai`, and
  `ai_runner_session`, which answers a `Session` without a master key;
  `Work`'s answer is checked against `MAX_GAMES`.
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
  3. An administrator creates, names and enables the AI account on the
     website and makes a runner token for the runner's machine.
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
   Switching on, creating and every other change keep the rule.
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

Not yet built at that step: the creation path for AI accounts (meanwhile
an administrator flagged an enrolled account that had never played; the
owner's decision removed that path), the refusals of social requests,
deletion, Tower, address, password, reset and recovery for AI accounts, the
website's administration page and badges, and the operator docs. The runner
token is the next section; the creation path and the refusals are the one
after it.

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

### The runner token (26 November 2026)

Built as §2 says, with one change to step 1 (below):

- **Migration 000054** (`m20261126_000054_ai_runner_tokens`): the tables
  and columns of §9, and their exact grants in `ops/grants.sql` and the
  startup check. Committed alone first (`0591f665`), with the store and its
  startup check as before, so that the previous binary's suite runs over the
  new schema: see "Evidence" below.
- **`central-api`**: `ai_runner` (`TOKEN_PREFIX` `bat_`, `token_text`,
  `parse_token_text`, the bounds, `validate_new`, `AiRunnerTokenInfo` with
  `AiRunnerAccount`), `wire::Request::AiRunnerSession { token, account }`
  and `wire::Response::AiRunnerSession { token, identity, expires_at }`,
  which `omit_ai` passes unchanged, and `AdminRequest::AiRunnerTokens`,
  `CreateAiRunnerToken { name, accounts, days }` (a change with a fresh
  sign-in) and `RevokeAiRunnerToken { token }` (a change without one),
  answered with `AdminAnswer::AiRunnerTokens { tokens }` and
  `AiRunnerTokenCreated { id, token, expires_at }`, the third answer that
  carries a secret.
- **`central-postgres`**: `accounts/ai_runner.rs`, the exchange in one
  transaction in §2's order, the list, the making (every account an AI
  player, kept by `FOR KEY SHARE` until the listing is written; the live and
  daily caps under the administrators' locks) and the revocation (the update
  first, then the token's sessions in a statement of their own, so that it
  sees the session of an exchange it waited for). The audit's
  `ai.runner_token_created` and `ai.runner_token_revoked` hold the actor and
  `ai_runner_token_id` and nothing else.
- **`central-identity`**: the exchange on the native listener only, refused
  with a bearer of either kind; each AI account's 12 exchanges an hour, read
  before the store and spent after a success; the sessions an exchange or a
  revocation ends are told to the push sockets. Every list of requests names
  the exchange, as one without a session, like `LoginStart`.
- **`central-http`**: 60 exchanges an hour from an address, and after 30
  refused ones in an hour the store is not asked again from it, counted as
  refused report tokens are.
- **`central-client`**: `ai_runner_session(transport, &token, account)`,
  which sends the exchange without a bearer, ends at once a session answered
  for another account, and answers a `Session` with no master key
  (`Session::is_runner`); every operation that needs the key, or a restore,
  refuses before it sends anything. `central_client::ai_runner` re-exports
  the text form and `MAX_SESSIONS`.

**The change: `FOR NO KEY UPDATE`, not `FOR SHARE`.** Step 5 updates the
token's row. Two exchanges of one token that each held a share would wait
for each other there, and PostgreSQL would end one as a deadlock. The row is
taken for update in step 1 instead: a revocation still waits for the
exchange, and exchanges of one token run one after the other. §2 says so
now; the store suite runs two exchanges of one token at once.

**Tests.** Unit tests: the text form (round trip, a paste's whitespace,
another prefix, case, length or character refused, and no gateway's or
report token's text read as one), a new token's bounds, the listing's
shape, the admin and wire shapes, the store's choice of the sessions an
exchange ends, the account's hourly budget, the address budgets, the
client's checks of the answer and the refusals of a session without a
master key, the migration's content. The store suite
(`tests/support/accounts/ranked/ai_runner.rs`): an exchange mints a native
session that renews, reads its status and runs `Work`; an unlisted AI, a
person, an unknown account, an unknown token and an expired one are all the
same `Rejected` and mint nothing; two exchanges at once both mint; the fifth
live session ends the oldest and an expired one goes first, never more than
four; two runner sessions of one AI are seated by fill-in in two matches and
each holds its match's ranked sid at once; a revocation needs no fresh
sign-in, ends all four sessions with their sids (each revoked at its host),
and the token mints nothing after; making one needs a fresh sign-in; a
person's second native sign-in still replaces the first; the runtime's
grants on the three tables. The SDK suite
(`central-client/tests/support/ai_runner.rs`) runs the same through both
listeners: the website makes and revokes, the exchange is refused with
either kind of bearer and on the website's listener, and the thirteenth
exchange in an hour is `Busy`. Until the creation path lands, the AI
accounts in both are made by one helper each (`ai_account`), which the
next step rewrites.

**Evidence (26.11.2026),** on the owner's Mac under a load average of 40 to
75 from other sessions. After the rebase onto `main`: `cargo clippy
--all-targets --all-features -D warnings` over `central-api`, `-client`,
`-identity`, `-http`, `-postgres`, `-migration` and `-web`, and the tests
of the first six: `central-api` 199, `central-client` 26 and 5,
`central-http` 173 and 7, `central-identity` 159, `central-migration` 7,
`central-postgres` 46, all green. The store suite
(`cargo xtask postgres --store-only`) at `7c11f3ab` (`aec7810d` after the
rebase onto `main`): green in 829 s, with the suite's five-minute deadline
raised locally and not committed; one run before it stopped at a connection
timeout ahead of every account test. The previous binary over the migrated
schema, at `dcc1a658` (`0591f665` after the rebase; the migration and the
grants alone): its startup check accepted the schema, and its store
suite ran green through the sessions, ranked (with the AI path), advent,
gateway, one-session, objects, terms, registration, password,
administration, reset, deletion, pairing and report checks before it
stopped at a statement timeout in the chat suite; a second run stopped at
the clock check that read 179 s for 180 s under the same load. Neither is
in code this pass touched. The SDK suite compiles; it runs in CI.

**Open before this is deployed.** A runner session counts as a fresh
sign-in where Central asks for one by the session's age: naming an e-mail
address, setting the key check, an export. With the address, a thief could
ask for a reset code by mail and set a password for the AI account. §1's
refusals of the address, reset, recovery, pairing and deletion flows for an
AI account close this; they are the next steps, and until they land, §2's
"what a thief gains" does not hold. *Closed* by the next section: a runner
session makes its runner's requests alone.

### The creation path and the refusals (26 November 2026)

Built after the owner's decision that AI accounts have no password:

- **Creation, change, removal** (`central-postgres`
  `accounts/ai/lifecycle.rs`, dispatched in `central-identity` `admin.rs`):
  `CreateAiPlayer` as §1 says, the sealed credential among it; `SetAiPlayer`
  changes the label, the caps and the name; `RemoveAiPlayer` before the
  first match, through `deletion::remove`, its sessions' digests told to
  the push sockets. The interim path that flagged an existing enrolled
  account, and `ai.flagged` and `ai.cleared` with it, are gone; the audit
  writes `ai.created` and `ai.removed`. Both change and removal lock the
  AI's account row for no key update before its row, the order an offer
  takes, so a fill-in that holds the AI's row writes its match and commits
  first.
- **`restored`.** Set at creation. Three places that serve the runner
  require it: the ranked `actor` (offers, work, seats), ranked admission
  (`issuing_session`) and fill-in's offer check. An AI's runner never
  restores a vault, so without it nothing of §3 and §4 would run.
- **The store's refusals:** `discover` and `credential` answer an AI as an
  unknown name (`NO_AI`), `issue_session` refuses one of either kind,
  `RESETTABLE` excludes one (an administrator's code, a mailed code and the
  use of a code), `OBJECT_ACCESS` excludes one, and the operator's
  appointment refuses one.
- **The gate** (`central-identity` `ai_gate.rs`, §2): `ForAi::of` decides
  every request; `require_terms` takes whether the request is refused for
  an AI and reads it in `session_gate`'s one statement; `refuse_ai` asks on
  its own for the refused requests that read no terms, on the native
  listener; the push socket's dial and a picture's admission refuse an AI
  in the same statement as the terms.
- **`Work`** locks nothing; `Offer` alone takes the AI's row. `Work` lists
  every live match up to `MAX_GAMES` (16), and `central-client` checks that
  bound instead of `max_games`.

**What people pay.** Nothing where the terms are read. On the native
listener a person pays one indexed statement more for each refused request
that reads no terms and is not served elsewhere: `ReadDocument`,
`ConfirmRestore`, `KeyCheckStatus`, `SetKeyCheck`, `AccountExport`,
`DeleteAccountStart` and `DeleteAccountFinish`, `ChangePasswordFinish`,
`Email` (status, withdrawal, removal), `Presence`, `TowerProgress`,
`TowerAwardExport`, the task reads, and the ranked `Cancel`,
`PreviewDeck` and `Export`. The library's operations and the presence
report cost nothing.

**Open.** A fill-in for a waiting administrator can deadlock with an
administrator's change of an AI (§3); PostgreSQL ends one and it answers
`Unavailable` once. Locking the administrators' rows for no key update
instead would end it, but that touches every administration change and
needs its own pass. The browser listener's families (profile, pictures,
badges, recovery and its mail, notifications, the owner's area, pairing's
start) rest on no AI holding a browser session, which the store suite
shows for `issue_session`; the gate does not check them there.

**Tests.** Unit tests: the gate's table (every operation decided: a
service's credential acts on no session, what needs the terms or the
cookie acts on one, the website's own operations are never the runner's,
the runner's list outside ranked play, each refused family named; the
ranked, seat and AI requests in `surface_tests.rs`), the sealed credential
(no OPAQUE record parses it, the column holds it), the random parts (the
login name a valid one, wrappers and the document with their version
byte), and the client's `Work` bound (16 matches pass under `max_games`
1, 17 do not). The store suite (`tests/support/accounts/ranked/ai.rs`):
creation refuses the unfair profile, a person's name and the `AI-`
prefix, a tag out of range and a taken one, and writes the row as §1
says; a change keeps kind and profile and refuses a person; a removal
before the first match ends the AI's runner session with its offer and
audits it, and is refused after the first match and with a stale sign-in;
the name, credential, sessions of both kinds, an administrator's code, a
mailed code to an address the operator planted and a code the operator
planted all refuse the AI; `SetAdmin` and the appointment refuse it,
`SetTag` serves it; it keeps no library; `session_gate` says it is an AI
and the runner accepts Central's terms with its session; `Work` lists
both live matches after `max_games` is lowered to 1; and a removal and a
fill-in for one AI, in four rounds (removal first, fill-in first, twice
at once): the first two take each way, and the two at once end in one of
them without a deadlock. The SDK suite (`central-client/tests/support/
ai_runner.rs`), over the real listeners with a runner session: the
address is refused before and after the terms, a reset by mail to a
planted address mails nothing and makes no code, `ResetLogin` is refused
while the same desk's fresh change is served, a planted code starts no
reset, the AI's login name discovers as an unknown one; `Offer` waits for
the terms, the runner accepts them and offers; twelve requests a
person's session is served (key check, presence and its choice, the
export, the address, Tower progress, tasks, contacts, the document, the
library, the ranked export) are refused to the runner; nine more that
would change its account are refused, three of them (a password change,
the deletion, a person's `Join`) at more than one place; the push socket's dial is 401; the
session's own, the terms' and the ranked requests are served, and its
logout ends it.

**Evidence (27.11.2026),** on the owner's Mac under the load of other
sessions: `cargo test` of `central-api` (199), `central-client` (27 and
5), `central-identity` (161) and `central-postgres` (48), green;
`cargo clippy --all-targets -D warnings` over the same four crates and
`cargo fmt --check`, clean. `cargo xtask postgres`: the SDK suite green in
147 s (a first run stopped at the person's control for contacts, a moved
account having no tag, which the test now gives it); then
`cargo xtask postgres --store-only` green in 120 s, after three runs that
stopped at the test's own mistakes (a fixture person without a tag, an
integer vector bound as bytes). The suite's deadline was raised locally
for these runs and is not committed.
