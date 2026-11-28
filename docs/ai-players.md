# AI players

*Design pass of 26 November 2026; the implementation pass (26 and 27 November)
and the adversarial pass are recorded at the end, and so are the three passes
for more AI seats at once (28 November).*

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
two AIs takes two; 0 to 32 since migration 000057, default 8; 0 stops every
AI), `host_reserve` (slots each host keeps for people, default 2),
`carousel_games` and `carousel_daily` (default 0, §7; `carousel_games` 0 to
16 since 000057), `changed_at`, `changed_by`.

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
- ranked play: `Ai` (`Offer`, `Work`, `Carousel`), the queue `Status`, `Board`,
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
runner's sessions faster than that. An exchange counts against that hour
from before the store is asked until it ends, so exchanges sent at once
cannot pass it together, and one the store refuses spends nothing.

**Never past the token.** A runner session is minted and renewed never past
its token's expiry, and an expired or revoked token's sessions renew no more:
a session minted just before the token ends dies with it.

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
which the no-key-update lock allows; fill-in takes that share with the AI's
row, skipping an AI whose account a runner's exchange holds, since an
exchange that ends a held offer's session waits for fill-in. A runner's
exchange takes its token, then the AI's account `FOR UPDATE`, then its
sessions (§2); the operator's plan takes the administrators' rows, the
settings' row, the tokens it revokes, the AI accounts and rows, and only
then the revoked tokens' sessions (§5). Fill-in reads eligibility without
locks first and takes `ai_settings` only when an AI looks available, and
asks again under that lock whether another person waits. When several people wait, no
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
  It also lists the account's matches settled in the last day, newest first,
  at most 32 (`FINISHED`), each a `Finished { match_id, format, seat, result,
  elo, delta, settled_at, opponent_ai }` read from the receipt written when
  it was settled: a runner with several matches in one format tells which
  ended how by the match id, not by its standing. A match that left the live
  list and is not among them was voided. The field is absent while empty.
- `Carousel { format }` asks for a match against another AI (§7), answered
  `Seated { assignment }` or `Unseated { reason }`.

The runner reads `Work` every two seconds while it offers, well inside a
session's 128 control requests a second ([native login](native-login.md)
§"Counted"). An AI account holds no push socket in Stage 1 (§2). A later
stage that opens one for the runner could send `{"event":"ranked"}` there
and replace the poll; clients built before such an event pass over it
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
are scoped per session, fence, origin and seat. All the runner's relays come
from one address, a home line it shares with people, and a match between two
AIs needs two, so the global bound counts seats, not matches. Before its
ticket a runner's relay counts against that address as anyone's socket does
(32 sockets and 60 dials a minute, shared with push sockets). Once the ticket
proves it is a runner's (Central asked the store when it issued the ticket
for the session), it gives the address back its dial and its place and counts
in the runners' own budget: 16 relays an AI account (`MAX_GAMES`), 32 for
all runners together (`MAX_GLOBAL_SEATS`, half of the listener's 64 relay
slots, so the other half stays with people), 32 proven dials a minute an
account and 64 for all runners (past them the address pays), and one relay
a seat, a newer one taking the older one's place.
So `global_seats` (default 8, at most 32) starves no person on the runner's
address (the record "More AI seats at once" at the end). AI accounts hold no
push socket (§2).

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

**The operator's command.** The live operator signs in to the website as no
administrator, and no operator account is appointed one for this (owner,
27.11.2026). `central-service --ai-players-stdin` does what this page does
under the migrator login: it creates and changes AI players, switches them,
sets the settings for every AI (calibration's among them), revokes runner
tokens and makes one for a runner machine, or lists them
([operations](operations.md) §"AI players"). A plan is applied in one
transaction through the same functions as the page (`ai/lifecycle.rs`
`create` and `change`, `ai.rs` `switch` and `write_settings`, `ai_runner.rs`
`make` and `revoke`), and its audit events name the operator as their actor
(`actor_kind = 'operator'`; a token without a maker,
`ai_runner_tokens.created_by IS NULL`, is the operator's). It is idempotent
where it can be: an AI named by its display name and tag that is there with
the same mind is reported, not made again, and anything ambiguous refuses the
whole plan. A new token's text goes to standard output alone, once, after the
commit.

**Fill-in's switch.** `ai_settings.fill_in` (migration 000055, on by
default) stops fill-in alone: the countdown goes, and a fill-in that waited
for the settings' row reads it again under that lock. The carousel goes on,
so calibration (§7) runs with no person meeting an AI.

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
allows. `choosable` refuses that prefix and `AI-` to people, read without
case and with `_` as `-` as the reserved names are (open question 3).
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

**The carousel.** The runner sends `Ai::Carousel { format }` for an AI with
a live offer for the format and a free slot. Central pairs it with another
AI's offer for that format. The partner must pass fill-in's hard rules
(switched on, a live offer under a session with `drain_minutes` left, below
its `max_games` and its `daily_games`), and fill-in's policy chooses it, with
the AI that asks in the person's place. A match is made only when:

- no person's ticket waits in that format in the open season, whatever it
  accepts;
- fewer than `carousel_games` AI-against-AI matches are live;
- fewer than `carousel_daily` were made today (UTC);
- `global_seats` has room for two more seats;
- the AI that asks is below its own `max_games` and `daily_games`;
- a host keeps its `host_reserve` after the match.

The answer is `Seated { assignment }`, the AI's seat, which `Work` lists from
then on, or `Unseated { reason }`:

- `off`: the AI is switched off or its `daily_games` is 0, the carousel
  allows no match (`carousel_games` or `carousel_daily` 0, `global_seats`
  under two), or no season is open;
- `offer`: it has no live offer for the format under a session with
  `drain_minutes` left;
- `tomorrow`: its `daily_games` or `carousel_daily` is reached;
- `later`: everything that passes in a while, a waiting person among them.

A runner session is never told that a person waits, though a seat does show
that none did. That is the rule of `ai_fill_in_at` (§3). Like an offer, the
request needs Central's terms.

**Locks.** Every refusal that needs no lock is read first, down to whether a
partner and a host's room exist. A runner that asks often takes no lock
until a match is near. Then, in fill-in's order:

1. the `ai_settings` row (`FOR UPDATE`), which serializes the carousel with
   itself and with fill-in. Everything is counted again after it;
2. both AIs' `ai_players` rows in one statement, in account order,
   `FOR UPDATE SKIP LOCKED` and only while switched on, with their account
   rows' keys shared (`FOR KEY SHARE SKIP LOCKED`). Both are counted again
   after it. A partner whose row another transaction holds (the switch, an
   offer, a runner token being made, a runner's exchange) is skipped for the
   next, at most four in all. If the asking AI's own row is held, the answer
   is `later`. The account keys matter because an exchange holds its account
   row for update and may end the session of an offer the carousel holds:
   the carousel must not then wait for it;
3. both offers (`FOR SHARE`, in account order), so the end of a session they
   were made under waits for the match;
4. the host, by `allocate_single`'s own locks.

Before all of these, the caller's account and session rows are shared, as
for an offer. While it holds the settings, the carousel waits only for the
offers' share, so no two requests wait for each other.

**The match** is written as fill-in writes one: each seat with its offer's
commitment, a drawn seat order, `ai_matches.arranged = 'carousel'` and
`ai_seats = 3`. It goes on a single host that offers a compatibility digest,
since both seats are runners and play as the live client does, and it is
never protected. Both runners see it in `Work` and play it as any match.
Language models count carousel games against their `daily_games`, as every
AI match counts. Scoring and repeat damping are those of people (the third
and fourth meeting in six hours at half, later ones at a quarter;
`rules::player::settle`). Settlement does not ask who sits in a seat.

**Calibration** is the carousel with high `carousel_games` and
`carousel_daily` and no fill-in, before people meet the AIs. The settings:
`carousel_daily` 10,000 (its maximum), `carousel_games` and `global_seats`
(two seats for each carousel match; at most 16 and 32 since migration
000057, 8 and 16 before) as high as the runner's machine carries: the house
AIs' `max_games` together at most 12 on acenb (record "More AI seats at
once"), `host_reserve` as for people, and hosts with room for those matches
above their reserve.

Fill-in cannot be switched off on its own. `fill_in_after` ends at 600
seconds, and every switch that stops fill-in (`global_seats` 0, an AI
switched off, no offer) stops the carousel too. Until there is one,
calibration runs with `fill_in_after` 600 in the test season, where a person
seldom searches alone for ten minutes. A person who does meets an AI. The
smallest change is a `fill_in` switch in `ai_settings`: a column with its
grant and the startup check, a field of `AiSettings`, and an early return in
fill-in's `seat` and `due`. That needs a migration.

Ratings carry across seasons (`scoring::players::load` reads the newest row),
so calibrated AIs enter Season 1 at their rating. But a season has been open
since migration 000035, which started one at migration time with no end, and
only SQL can end a season or open a new one (open question 5).

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
| Exhausting hosts or relays with AI matches, or starving people on the runner's address | `host_reserve`; `global_seats` at most 32; a runner's proven relays leave its address for the runners' own budget (16 an AI account, 32 together, half of the 64 relay slots, 32 dials a minute an account and 64 together, one a seat), decided by Central's ticket alone (record "More AI seats at once") | Fill-in is refused when a host would fall below its reserve. A match between two AIs counts two seats. A person on the runner's address keeps every socket and dial; a socket that proves no runner's gains nothing; the ceilings hold and a closed relay gives back what it held. |
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

**Migration 000055**, `m20261127_000055_ai_operator_fill_in`, comes after 000053
and 000054 ran on test stacks, so neither is edited: it lets a runner token
have no maker (`created_by` drops `NOT NULL`: the operator's) and adds
`ai_settings.fill_in boolean NOT NULL DEFAULT true`. The runtime gains
`UPDATE (fill_in)`. A binary of this branch from before it checks the
settings' grants exactly, so it runs over 000055 only before the new grant
is applied; the previous `main` binary names none of these tables.

**Migration 000057**, `m20261128_000057_ai_global_seats` (28.11.2026, the
record "More AI seats at once"), widens two CHECKs of `ai_settings`, under
the names 000053 gave them: `global_seats` 0 to 32 and `carousel_games` 0 to
16. No row, grant or table changes; the settings keep what was set, so the
deploy's own fallback to the previous binary is safe as long as nobody has
raised them, and they are raised only once the deploy stands. The previous
binary reads the wider values (its read checks no bound; only a change is
checked, and stopping every AI still passes), but counts a runner's relays
against its address and its website refuses to save them: before a fallback
to it, the settings go back to `global_seats` 16 and `carousel_games` 8 or
less.

**The runtime's grants**, exact, in `ops/grants.sql` and the startup check
(`accounts/grants.rs`):

| Table | Grant |
| --- | --- |
| `ai_players` | `SELECT, INSERT, DELETE`; `UPDATE (label, enabled, max_games, daily_games, changed_at, changed_by)`. `kind`, `profile`, `set_at` and `set_by` are never updated. |
| `ai_settings` | `SELECT`; `UPDATE` of every column except the key, `fill_in` (000055) among them |
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
    `MAX_GAMES` (16), the bound of `max_games` and of `Work`'s list, and
    `MAX_GLOBAL_SEATS` (32) and `MAX_CAROUSEL_GAMES` (16), the bounds of
    `global_seats` and `carousel_games` (000057);
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
    revocation of runner tokens, and `relay_runner`, whose relay a
    socket ticket is (000057's record);
  - `grants.rs`.
- `central-identity`: the exchange's dispatch (native listener, no bearer),
  each AI account's 12 exchanges an hour, the administration's AI and
  runner token requests (a removal's sessions told to their push sockets),
  and `ai_gate.rs`: what a runner session may do (§2), applied in
  `dispatch`, `require_terms`, the push socket's dial and a picture's
  admission.
- `central-http`: protocol `2` and `3`, the version passed to the reads,
  the exchange's address budgets (60 an hour, and no store asked after 30
  refusals in an hour), and the runners' relay budget (`ws/runners.rs`,
  000057's record).
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
  once, which the concept's carousel and several-games plans must fit. Since
  28.11.2026 a runner's proven relays count in a budget of their own, half
  the slots, and no longer against its address (the record "More AI seats
  at once").
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
migration before `fair` was removed; the full green runs owed came later
(below, "The previous binary, proven").

**The previous binary, proven (27.11.2026).** `main`'s code with this
branch's migration crate, `ops/grants.sql` and the migration test's history
harness (the counts and the undoing of the new migrations before the
runner's history test; no store code and no other test changed): its whole
store suite green over 000053 and 000054, at `f1a19319`, on the owner's
Mac (263 s); and over 000053, 000054 and 000055, at `8d6ac023`, on bigace
(100 s, in a build directory of its own: a shared one let one tree's
artifacts stand for the other's, which a later run showed and which this
run excludes). Its startup check names none of the new tables, so the new tables
and columns, the partial indexes and the new grants leave it running.

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
`cargo fmt --check`, clean. `cargo xtask postgres` at `62bcff05`, with the
committed deadline: green, the SDK suite in 150 s, the store suite in 99 s,
the Tower access and Tower awards suites in 2 s each. Before it, with the
deadline raised locally and not committed: a first SDK run stopped at the
person's control for contacts (a moved account has no tag, which the test
now gives it), and three store runs stopped at the test's own mistakes (a
fixture person without a tag, an integer vector bound as bytes). The
store suite does not drive an AI seat's play through `dispatch`: its seat
requests are allowed by the table, which a unit test holds, and served by
the store suite's `admitting` through the store.

### The carousel (27 November 2026)

Built on branch `claude/ai-players-carousel`, as §7 says:

- **`central-api`**: `ai::Request::Carousel { format }`, answered with
  `ai::Response::Seated { assignment }` or `Unseated { reason }`, where
  `Unseated` is `off`, `offer`, `tomorrow` or `later`. All of them keep
  `deny_unknown_fields`. `ranked::Response::omit_ai` passes the runner's
  answers whole, as before: only a runner asks for them, and a runner speaks
  protocol 3. The comment there says so now.
- **`central-postgres`**: `ranked/carousel.rs`, the request, and
  `ranked/carousel/reason.rs`, the decision as a pure function of what was
  counted. Fill-in's match writing is shared (`fill_in::write`, `coin`,
  `Arranged`). `routing::room_single` reads a host's room without a lock.
  `Work` and the carousel's seat build the assignment with one function.
- **`central-identity`**: the gate allows the request to a runner session,
  and it needs Central's terms, as an offer does.
- **`central-client`**: `Session::ai_carousel(transport, format)`, which
  answers the seat or the reason. `ai_responds` takes a seat only for this
  account, in the format asked for and against an AI, and any reason.
- No migration and no new grant. The runtime's grants of migration 000053
  cover every statement: `ai_matches` `INSERT`, and the row locks under the
  column `UPDATE` grants fill-in already relies on.

**Design notes** (the first pass):

- The answer says when asking again can help, and no more. A waiting person
  is one cause of `later` and never a reason of its own, so a runner token
  in the wrong hands is never told that people search; a seat shows only
  that nobody waited at that moment. That is the rule `ai_fill_in_at` keeps
  between people.
- People first means any person's waiting ticket in the format and the open
  season, whatever it accepts and of either kind. No ticket is locked: a
  person who joins a moment later is paired by the queue as before.
- The partner is chosen by `fill_in::policy::order`, with the asking AI in
  the person's place: its queue Elo, and the two AIs' meetings in six hours,
  so one pair does not meet every time while another is near. A language
  model may be a partner. On a near tie the free kinds come first, as in
  fill-in.
- `global_seats` must have room for two seats. The match takes a single host
  that offers a compatibility digest, as fill-in does for the live client's
  searches, and it gets no replica.
- `daily_games` counts every row of `ai_matches`, so a model's carousel games
  count without a rule of their own.
- Calibration: no switch keeps fill-in off on its own (§7). The pass added no
  migration; the smallest change is in §7.

**Implementation checks** (the second pass):

- Locks, in the order §7 gives. While it holds the settings, the carousel
  waits for no row another request holds, except to share the offers, which
  only a session's end changes.
- Bounds: at most 64 candidates (fill-in's query), four tries, and a fixed
  number of statements for each. A runner that asks in a loop takes the
  settings only while a partner and a host's room exist. Once the caps are
  full, it is refused without a lock.
- The answer names the AI's own seat and the opponent's public badge. No
  secret, token or person's data is read into it or logged.
- No `unsafe`. Every crate keeps `forbid(unsafe_code)`.

**Adversarial checks** (the third pass):

- The store suite (`tests/support/accounts/ranked/carousel.rs`, four AIs:
  three house levels and a model capped at one game a day). Each refusal is
  followed by the same request with its one cause gone. The carousel is off
  as installed. A person's session is refused. An AI without an offer is
  told to offer first, and one offer alone finds nobody. A person's waiting
  ticket, even one that refused AIs, stops it. Then the two offers make one
  match with both seats AIs, `arranged = 'carousel'`, `ai_seats = 3`, the
  offers' decks, no replica and no ticket, and the partner's `Work` lists
  it. Each of these refuses: `carousel_games` reached; `global_seats` with
  one seat left (`later`) and with one seat in all (`off`); a host at its
  reserve; the asking AI's and a partner's `max_games`; a partner whose
  account row another transaction holds for update; the model's
  `daily_games`, which its carousel game fills; partners switched off with
  their offers standing; the asking AI switched off; `carousel_daily`. Two
  AIs that ask for each other at once for the last slot (the carousel's
  last live match, then each one's last game) make exactly one match in
  each of four rounds, and neither request fails. Five carousel matches
  between the same two AIs, from level ratings and played through the
  host's grants, claims, start and finish, gain 12, 12, 6, 6 and 3 points:
  the same as five matches between two people, so the third and fourth
  meeting count half and the fifth a quarter. The administration takes the
  calibration's settings and refuses `carousel_games` 9 and `carousel_daily`
  10,001.
- The SDK suite (`central-client/tests/support/ai_runner.rs`), over the real
  listeners: the carousel waits for the terms as an offer does, and after
  them answers `off` as installed.
- Fault injection, each on bigace and undone after the run. Ignoring the
  waiting person: the suite stops where a match was made while she waited.
  No count after the locks: the race makes two matches for the last slot.
  The account keys left out of the row lock: the request for the held
  partner waits for the lock until it fails with `Unavailable`.
- Found and fixed in this pass: the carousel and a runner's exchange could
  deadlock. The exchange holds the AI's account row for update and may end
  its oldest session, whose offer the carousel holds for share. The match's
  foreign key then waits for the exchange, and the exchange for the match.
  The carousel now shares both account keys in its row lock and skips an
  account an exchange holds.

**Open.** Fill-in has the same shape for its AI, an offer held for share
before the match's foreign key, and is left as it was; the same skipped key
share would close it. Fill-in cannot be switched off on its own (§7).

**Evidence (27.11.2026),** on bigace (Linux, 24 cores): `cargo test` of
`central-api` (200), `central-client` (28 and 5), `central-identity` (161)
and `central-postgres` (54), green; `cargo clippy --all-targets -D warnings`
over the same four crates and `cargo fmt --all --check`, clean;
`cargo check --all-targets` of `central-http` and `central-web`, clean.
`cargo xtask postgres --store-only`: green in 114 s with the carousel's
suite. `cargo xtask postgres`: green, the SDK suite in 161 s, the store
suite in 112 s, the Tower access and Tower awards suites in 2 s each. The
three injections above, applied one after the other, each made the store
suite fail as described; the full run above came after the last was undone,
with all three gone. A check of the whole workspace
stopped at the picture codec's build script, which needs `nasm` and finds
none there, so `central-service` and the web server, which link it, were
not compiled; neither names the runner's requests or answers.

### The website, the operator's command, fill-in's switch and the runner's results (27 November 2026)

- **The website** (`central-web`): the AI label (a mark by kind and the
  badge's text, the kind in words for screen readers) beside every AI
  account's name the site shows: the public player page, a chat mention's
  preview, the administration's account list and account page (no role or
  reset code is offered for an AI), the reset-code search and the audit's
  tables, which name the eight `ai.*` actions. The administration's page
  `/account/admin/ai` lists the AI players and creates, changes, switches
  and removes them, holds the settings for every AI with "stop every AI",
  and makes, shows once and revokes runner tokens. The site renders no
  ranked board, so the board filter has nowhere to go there yet; a client's
  board takes it (§6). The chat's people search finds no AI at all (below).
- **The operator's command** (§5, [operations](operations.md) §"AI
  players"): `central-service --ai-players-stdin`, `ai/operator.rs` in the
  store. The administration's change functions were split so that both
  paths take one: `lifecycle.rs` `create` and `change`, `ai.rs` `switch` and
  `write_settings`, `ai_runner.rs` `make`, `revoke` and `listed`; each
  audit writer takes an actor that may be none, the operator. A plan's
  locks follow the administration's and fill-in's order: the
  administrators' rows, the settings' row, the revoked tokens and their
  sessions, the AI accounts and rows, the new token.
- **Migration 000055** (§9): a token without a maker, and fill-in's switch.
  000053 and 000054 are not edited, since test stacks ran them.
- **`Work`'s results** (§4): `finished`, the last day's receipts of the
  account, at most 32, newest first.

Design pass for these, in short. Assets: the AI accounts' integrity, the
runner token's secret, the settings, the people's queue. Callers: the
operator, holding the migrator login; an administrator at the same time; a
runner. Invariants: the operator's command can do nothing the page cannot,
through the same code, with the same checks and audit; a plan is whole or
nothing; the token's text appears once, on standard output, after the
commit, and nowhere else; anything that could name two things is refused;
no edited migration.

Implementation pass. The plan's own bounds are checked before the
database is asked (`checked`: settings, distinct tags and names, the token's
name, lifetime and list, a secret exactly with a token). A player is named
by its display name and tag together: neither taken creates it, both on one
AI of the same mind reports or changes it, and a tag a person or another AI
holds, the name under another tag, or another kind or profile refuses the
plan. A token's accounts are named by display name and each must name one
AI; a live token of the same name refuses it, so rotating revokes the old
one in the same plan. The command reads its input as `--appoint-admin-stdin`
does (the login in the JSON, never an argument; unknown fields refused),
answers one refusal line that echoes nothing, writes what it did to
standard error and the token's text alone to standard output. A listing
goes alone and prints no digest or text.

Adversarial checks, as run. The store suite (`ai_operator.rs`): one plan
creates a switched-on AI with no password, writes calibration's settings
and a working token whose maker is none, and audits each change with the
operator as the actor; the same plan again reports it, a new label changes
it; six ambiguous or wrong plans (the tag under another name, the name
under another tag, a person's tag, another mind, a token for a name no AI
holds, a second live token of one name) and an unknown revocation are
refused whole, and the new player each also asked for is not there
afterwards; an operator's revocation ends the token's session in its
transaction, and the listing shows the token as the operator's and revoked.
Unit tests: the plan's bounds (ten wrong plans), settings left out stay,
the input's shapes (a listing alone, unknown fields refused, the id's
forms). The CLI suite: the mode is listed, refuses an extra argument
without echoing it, and refuses empty and wrong input with its one line and
no secret. The carousel suite: after five carousel matches each runner's
`Work` lists the five with their results and the same deltas as the
receipts. Fill-in's switch reads again under the settings' lock.

**Evidence (27.11.2026),** on bigace (Ubuntu, x86-64), never this Mac:
`cargo test` of `central-api` (200), `central-client` (28 and 5),
`central-identity` (161), `central-web` (172), `central-migration` (8),
`central-postgres` (lib 56), `central-service` (lib 51, CLI 8);
`cargo clippy --all-targets -D warnings` over those crates and `cargo fmt
--check`, clean; `cargo xtask postgres --store-only`, green.

### The adversarial pass over the whole pull request (27 November 2026)

A second agent reviewed the branch against `main` read-only, given the code
and the design but not the implementer's conclusions, attacking each area as
an internet caller, a person, the holder of a stolen or revoked runner token,
a runner session, a host and an administrator overstepping. Nothing high.
What it found, and what changed:

- **Medium: people could find, contact, block and invite an AI.** The social
  lookups (`by_tag`, which groups and contacts take, and the search) read
  enrolled accounts, which an AI is by design, and their replies carry no
  badge. Now an AI answers there as an unknown tag (`NO_AI`), so no contact,
  block or group row ever names one; the store suite shows a person finding a
  person by tag and finding no AI by tag or search, and a contact request or
  block of an AI refused.
- **Low: `ai_players` kept its `DELETE` grant** from the interim path; no
  statement deletes from it (an account's removal cascades as the table's
  owner). Revoked (`ops/grants.sql`, the startup check), so a runtime bug
  cannot turn a played AI back into an account that a reset code could give
  a password.
- **Low: fill-in did not ask "people first" again under its lock.** A person
  who joined while a fill-in waited for the settings' row could watch an AI
  take the lonely ticket. It asks again after the lock now, as the carousel
  does.
- **Low: the exchange's hourly budget was check-then-act.** Exchanges sent at
  once all passed it. An exchange now counts from before the store is asked
  until it ends (a guard, also when the caller hangs up), and spends the hour
  only when it minted a session; a unit test fills the hour with twelve at
  once.
- **Low: an expired token's sessions renewed for up to 12 hours.** A session
  is now minted and renewed never past its token's expiry, and an expired or
  revoked token's sessions renew no more (§2).
- **Low: the operator's plan locked in the exchange's reverse order.** It
  holds the tokens to revoke before any AI account now and ends their
  sessions after the accounts.
- **Low: people could choose `KI_` and `AI_` names.** The prefix check reads
  `_` as `-`, as the reserved names do.
- **Low: an older binary serves runner sessions as ordinary ones.** The
  operations runbook says to end them before running one by hand.
- **Low: fill-in could deadlock with an exchange** (the offer held, then the
  match's foreign key waiting for the account the exchange holds). Fill-in
  shares the AI account's key with `SKIP LOCKED` under its row lock, as the
  carousel does. Still open, and rare: a fill-in for a waiting
  administrator's ticket against that administrator's change of an AI; one
  of the two is ended by PostgreSQL and answers `Unavailable` once.
- **Nit:** a connection that failed to close after the operator's commit
  made the command say nothing changed; it says what was applied now.

What it found sound, and checked: the exchange's lookup, constant-time
compare and single refusal, its races with revocation and removal, the cap
of four across tokens; the gate's exhaustive table and that every refused
request reaches a check before any early return; that no AI can hold a
browser session or a password by any path; consent, caps and switches read
after their locks; protocol 2's omissions; the operator command's secrets and
atomicity; the website's escaping and that the token is shown once; the
migrations' partial indexes and CHECKs.

This review and the passes above are the implementing agents' engineering
passes, not an independent audit; the owner's release gates still apply.

**Evidence after the fixes (27.11.2026),** on bigace at the tree of
`832fe749`: `cargo xtask postgres` green (the SDK suite 162 s, the store
suite 97 s, both Tower suites 2 s), `cargo test` of `central-api` (201),
`central-identity` (161) and `central-service` (51 and the CLI's 8), clippy
`-D warnings` over `central-api`, `central-postgres`, `central-identity`
and `central-service`, `cargo fmt --check`. The website's browser specs
(CI's `site`) had failed at `eb6504e6` on two things of this branch: the
German navigation's list lacked the AI page, and the reset page's stand-in
Central refused the AI read; both are in `832fe749`.

After the review's second reading (27.11.2026): a fill-in also seats no AI
whose offer's session would end, by its token's expiry, within
`drain_minutes`, as it already did for the session's 12 hours; the store
suite mints a session under a token with ten minutes left (it gets ten, and
a renewal stays within them), renews none and mints none once the token has
passed, and the social check's control asks a person for contact. On bigace
at that tree, in a fresh build directory: `cargo xtask postgres` green (SDK
155 s, store 110 s, Tower 2 s each).

**The operator command's faults (27.11.2026),** after the dress rehearsal on
acenb, where a display name of 17 characters refused a plan with no more
than "rejected". The command now says every fault of a plan at once, each in
its place (`players[3].display_name "KI-DeepSeek-Flash": 17 characters, at
most 16`), before the database is asked, by the same rules the
administration takes (`operator/faults.rs` `plan_faults`, a unit test holding
it to `AiPlayerSettings::valid`, `AiSettings::valid` and `validate_new`);
what only the database shows (a tag a person holds, a name under another
tag, another mind, an unknown token account or id, a live token's name)
names its place too (`PlanFault`). A login that is no login is said so and
never quoted: the input is read a second time with the login read past
unseen. The network-facing requests keep their one `Rejected`. Evidence on
bigace: `central-postgres` (lib 58), `central-service` (51, the CLI's 9: a
new test sends six faulty plans, a bad login and an empty input, and finds
every fault once, the refusal last and no canary of the login), clippy and
fmt clean, the store suite green (95 s; each refusal of the operator's
suite names its place).

## More AI seats at once (28 November 2026)

The owner wants the AI players to play far more ranked games at once, so
that they use up an API credit within days (decided for the owner,
28.11.2026). `global_seats` was capped at 16, and §4 said why: the runner's
relay sockets all come from one address, a home line it shares with the
owner, and counted against that address's 32 sockets and 60 dials a
minute. Thirty-two AI seats would have starved every person on it.

### Design pass

Assets: a person's relay sockets and push socket on an address a runner
shares; the listener's relay slots and its file descriptors; the hosts'
capacity; the runner's games. Callers: anyone on the internet, before and
after a ticket; a person; a runner session; the holder of a stolen runner
token.

Invariants:

1. Whatever is counted before a socket proves what it is stays as it is,
   for everyone: the address's dial, the address's socket place, the
   listener's place, a relay slot. An unauthenticated client gains nothing.
2. A socket proves it is a runner's only by the first frame's ticket, and
   the fact behind it is Central's: when Central issues the ticket for a
   session, it asks the store whether that session was minted by a live
   AI runner token, and for which AI account, and keeps the answer with
   the ticket in memory. The client sends nothing that says it.
3. Once proven, a runner's socket leaves its address: its socket place and
   its dial are given back to the address, also when the runner's own
   budget then refuses it. A person on that address keeps every socket and
   dial a runner's proven sockets would have taken.
4. The runners' own budget is hard: one AI account holds at most
   `MAX_GAMES` (16) relay sockets and dials 32 a minute (two whole
   redials), all runners together at most `MAX_GLOBAL_SEATS` (32) relay
   sockets, half of the listener's 64 relay slots. The other half stays
   with people.
5. One relay per AI seat: a newer proven socket of the same AI account,
   match and seat takes the older one's place in the budget, and the older
   one closes, as the host's own newer socket for a chair replaces the
   older (Baylee's `seat_socket.rs`). So a runner whose line changed its
   address while every old relay hangs half-open (a header waits up to 600
   seconds) is not refused until they die; sockets stay at most one per
   live AI seat.
6. The relay slots and the listener's sockets are not raised: the file
   descriptors give no room (below).

*The adversarial pass below changed invariant 3 (a dial is given back only
when the runners pay for it) and the order (the session is checked before
the budget).*

Where it is counted. `ws.rs`'s `Seat` holds the socket's place; after the
ticket (`ranked_socket.rs`, straight after `relay.take`, before the session
is checked or the host is dialled) a grant that names an AI account calls
`Seat::runner`, which gives the address back its dial (in the window that
charged it) and its place, charges the account's dials, and enters the
seat in the runners' table under the listener's lock, or refuses and the
socket closes. Dropping the `Seat` gives back whatever it holds. Every
other socket keeps the budget it was admitted under.

Where the fact comes from. `Relay::forward`'s socket branch asks
`IdentityService::relay_runner`, one statement (`AccountStore::relay_runner`):
a live native session with a `runner_token` whose token is neither revoked
nor expired, of an enrolled account that is an AI player. Anything else,
a person's session among it, is no runner's, and keeps the address's
budget. It costs one indexed read per relay ticket, once a match and a
reconnect, for people too.

Weighed and set aside: a field on `SeatResponse::RelayRoute` (a wire type
the game repository pins, with `deny_unknown_fields`); the signed bearer's
claims (the same for a runner as for a person, §2); an allowlist of the
runner's address (none, §2); raising the per-address limits (they protect
every shared address from anyone).

Residuals named before the code:

- Until its ticket is read (at most two times `AUTH_TIME`, three seconds,
  in practice one round trip), a runner's socket counts against its
  address as anyone's does. A runner that dials all its seats at once can
  fill its address's 32 places for that moment; a person's new dial then is
  refused once and retried. The runner should dial a few at a time (the
  game repository's runner).
- The relay's 64 upstream calls at once (`Relay::requests`) are shared:
  a runner's calls are a handful a match and end within 8 seconds; the
  host they go to is the one people's calls go to, so a share of them
  would not keep people's calls fast when it is slow.

### Capacity, from what is written down

netcup's and the hosts' live numbers were not read (no login to the
servers); what follows is arithmetic over the numbers given.

- **Central's relay.** 64 relay slots, 32 for runners. The file
  descriptors (1,024, systemd's default; the unit sets no `LimitNOFILE`):
  the native listener holds at most 512 sockets (push and relay together)
  and one more descriptor for each of the 64 relays' host connections, the
  browser listener 512 push sockets of its own (`run_browser` makes a set of
  its own), each listener 128 connections, the database pools four each.
  At the worst that is 512 + 64 + 512 + 256 + the pools, past 1,024 before
  this change: the slots stay 64, the runners' share adds no descriptor,
  and the unit should set `LimitNOFILE=4096` ([operations](operations.md)
  §"Supervision"). Each relay's session and route are checked every three
  seconds: at most 64 relays are about 21 route transactions a second
  against a pool of four, as before; the runners' share changes who holds
  them, not how many.
  The people's half holds the people facing AIs too: each filled-in AI
  seat brings a person's relay. Up to 16 filled-in seats leave the people
  16 relays for each other, as before; at 32, none. Seats above 16 belong
  in the carousel, which takes no person's relay.
- **The firewall** (`ops/central-limits.nft`, whether it is loaded on the
  server was not read) counts the runner's connections to Caddy as any
  address's: 512 held and 300 new a minute after a burst of 300. Its 32
  relays and a whole redial of them fit. Its requests (`Work` every two
  seconds an AI account, offers, relay calls) fit only over kept
  connections: the runner's transport (Baylee's `baylee-account`
  `CentralHttp`) holds a `ureq` agent, whose pool keeps them; how many it
  opens a minute was not measured.- **netcup** (12 cores, 32 GB, one engine process per game). At 32 AI
  seats at most 32 matches hold an AI (every one filled in) and 16 when
  each is between two AIs. A harness probe measured big AI games at about
  6 MB each; real games are heavier, so take ten times that: 32 × 60 MB ≈
  1.9 GB, a sixteenth of the memory. The moves are decided on acenb, the
  engine only applies them. The host's own advertised capacity (at most 64
  a host) and `host_reserve` bound its matches whatever `global_seats` says.
- **acenb** (16 threads, 27 GB, about 21 GB available). The Baylee AI
  runner takes about 3.4 GB idle and 1.2 GB a game, counted a seat:
  (21 − 3.4) / 1.2 ≈ 14.7, so at most 14 house-AI seats, 12 with 3 GB to
  spare (3.4 + 12 × 1.2 = 17.8 GB). Language-model CLIs are light (not
  measured here).

**The maximum is 32** (`MAX_GLOBAL_SEATS`): Central's own bound, the
runners' half of the relay slots, and well inside netcup's memory as
written down. **The default stays 8**, the column's and the row every
install has (3.4 + 8 × 1.2 = 13 GB on acenb, whatever the mix). Any mix of
kinds is safe up to 12. Above 12, only with the house AIs' `max_games`
summed to 12 or less and the rest language models. The migration changes no
row: `global_seats` rises only when an administrator or the operator's plan
sets it.

`carousel_games` (matches between two AIs at once) was capped at 8, which
is 16 seats: with fill-in rare, the carousel is what spends the credit,
so 32 seats would have changed nothing. It may reach 16
(`MAX_CAROUSEL_GAMES`, half of `MAX_GLOBAL_SEATS`), so the carousel alone
can fill every AI seat. `carousel_daily` (10,000 a day) is far above 16
matches at once of a quarter hour each (1,536 a day).

**Migration 000057**, `m20261128_000057_ai_global_seats`, widens the two
CHECKs of `ai_settings` (`global_seats` 0 to 32, `carousel_games` 0 to 16)
and nothing else; no grant changes. The previous binary reads the wider
values, but its relay counts a runner's sockets against the address and its
website refuses to save them: before a fallback to it, set `global_seats` to
16 or less and `carousel_games` to 8 or less ([operations](operations.md)
§"AI players").

### Implementation pass

Built on branch `claude/ai-seats`:

- **`central-api`**: `ranked::ai::MAX_GLOBAL_SEATS` (32) and
  `MAX_CAROUSEL_GAMES` (16), which `AiSettings::valid` takes.
- **Migration 000057** (§9): the two CHECKs, nothing else.
- **`central-postgres`**: `AccountStore::relay_runner`, one statement (a
  live native session of the presented digest, its credential revision the
  account's, joined to `ai_players` and to a runner token neither revoked
  nor expired); the operator command's faults take the API's bounds
  (`settings.global_seats 33: 0 to 32`).
- **`central-identity`**: `IdentityService::relay_runner`, the store's word.
- **`central-http`**: `Grant.runner`, set in `Relay::forward`'s socket
  branch and kept in the vault with the grant; the client holds only the
  random handle. `ranked_socket.rs`: after `relay.take` and a first check
  of the session, `budget` calls `Seat::runner` for a runner's grant, and
  the relay's work runs beside `Replaced::wait`, so a replaced relay ends.
  `ws/runners.rs`: the runners' table, keyed by (AI account, match, seat),
  each entry with the serial of the socket that holds it and the switch
  (`oneshot`) that closes it once a newer one takes its place; a dropped
  `Seat` frees its entry only while its serial holds it. The runners'
  dials (`Book<[u8; 16]>` an account, `Book<()>` for all), and
  `Book::refund`, which gives back one unit only into the window that
  charged it. `Book` takes its key as a type parameter, an address by
  default. `RELAY_SLOTS` stays 64, with `2 × RUNNER_RELAYS ≤ RELAY_SLOTS`
  checked at compile time.
- **`central-web`**: the form's ranges (from named constants a test holds to
  the API's) and its hints in all eleven languages.

Locks: the address's dials, the runners' dials and the listener's sockets
are each taken alone, never one inside another, and poisoning is read
through; `Seat::drop` takes the listener's alone. Whatever way a relay's
task ends (its frames, its session, a stop, its twelve hours, a newer relay
of its seat), its `Seat` drops and gives back what it holds. Nothing new is
logged.

### The adversarial pass (28 November 2026)

A second agent reviewed the branch against `main` read-only, given the code
and the design and not the implementer's conclusions, attacking it as an
internet caller, a person on the runner's address, a runner session and the
holder of a stolen runner token. Nothing high. What it found, and what
changed:

- **Low (near medium): a refused proven dial was paid by nobody.** The
  address got its dial back before the account was charged, and a refused
  charge spends nothing. A runner session past its dials, sending each
  ticket late, could keep 32 sockets before their tickets on its address
  (beside 32 relays: every relay slot), bounded only by the firewall's new
  connections. Now the runners pay first (32 a minute an AI account, and
  64 a minute all runners together, two whole redials of their relays,
  about one address's 60); only then does the address get its dial back,
  so each dial is paid once. A unit test holds the address's dial spent
  past the account's and past all runners' dials.
- **Low: a ticket of an ended session replaced its seat's live relay.** The
  budget came before the first check of the session. The session is now
  checked first: a ticket up to 15 seconds old of a session that ended
  since takes no budget and closes no relay. A relay whose host dial then
  fails has still closed its older one, which the runner replaces at its
  next dial as the host would.
- **Low (capacity): "the other half stays with people" counts the runners'
  relays only.** With fill-in on, every filled-in AI seat brings a person's
  relay, so at 32 filled-in seats the people's half is all people facing
  AIs, and a match between two people finds no relay slot (before: at most
  16 and 16). Not changed in code: fill-in seats an AI only for a person
  who searched alone, and the carousel, which spends the credit, takes no
  person's slot. Named in the capacity notes and the operations runbook:
  above 16 seats, spend them in the carousel, or count people's matches
  in. A cap of 16 filled-in seats in the store would keep the old envelope;
  it is left for a later change with a test that seats seventeen.
- **Nit: an unanswered runner read failed a person's ticket.** It is now
  read as no runner's, the stricter budget.
- **Nit: a dial's time was read before its book's lock,** so a dial that
  lost the race for a new window lost its refund, and a window evicted and
  made again could take one. The time is read under the lock.
- **Tests:** the store suite now ages a runner's token (revoked, expired)
  and a session (expired) while the session row stays, and each is no
  runner's relay; the website's bounds are named constants a test holds to
  the API's. `Relay::forward`'s reading and `serve`'s wiring are not covered
  by a test of their own: they need a host to issue socket tickets, which
  no suite here runs (the step after the ticket, `budget`, is tested with
  Central's grants).
- **Docs:** the record of this pass, here.

What it found sound: no client input reaches `Grant.runner`, and a person's
grant never touches the runners' table; the locks are never nested and read
through poisoning; a seat is freed only by the serial that holds it; a
socket proves once; one with no known address enters the budget and gets
nothing back; replacement never crosses an account or a seat; the store's
statement; the ceilings under replacement and refusal; the listener's total
and the relay slots unchanged; the constraint names and the store suite's
writes on both sides of each bound; the bounds alike in the API, the
operator's faults, the website, its eleven languages and the migration.

**Evidence (28.11.2026),** on bigace (Ubuntu, x86-64), never this Mac, at
the tree of `46f54e88` (the fixes of the adversarial pass, `47038c59` after
the rebase onto `main`'s `8a1c02b8`, which changed no Rust code; what
followed changed docs alone): `cargo test` of `central-api` (202), `central-http`
(182 and the transport suite's 7), `central-identity` (161),
`central-migration` (10), `central-postgres` (lib 58), `central-service`
(51, the CLI's 9) and `central-web` (182 and 7); `cargo clippy
--all-targets -D warnings` over those seven crates and `cargo fmt --all
--check`, clean; `cargo xtask postgres` green (the SDK suite 149 s, the
store suite 106 s, both Tower suites 2 s). At the tree before the fixes
(`46c2a634`, `ffbf30c1` after the rebase) the same suites were green as well (SDK 160 s, store 97 s).

**Left.**

- Fill-in above 16 seats: seats past 16 belong in the carousel (capacity
  notes above); a cap of 16 filled-in seats in the store, with a test that
  seats seventeen, is a change of its own.
- The runner should dial its relays a few at a time (Baylee's runner).
- `LimitNOFILE=4096` on the service's unit ([operations](operations.md)
  §"Supervision"): both listeners' sockets were already past 1,024 at the
  worst before this change.
- `Relay::forward`'s reading of `relay_runner` and `serve`'s order (the
  session, then the budget, then the host) are held by reading, not by a
  test of their own.
- netcup's and the hosts' live numbers, the official gateway's advertised
  capacity and the firewall on the server were not read.

These are the implementing agents' engineering passes, not an independent
audit; the owner's release gates still apply.
