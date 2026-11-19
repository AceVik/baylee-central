# The native push socket, `GET /v1/ws`

The Baylee desktop client has dialled this socket since 10 November 2026 and
polls when it cannot (`docs/central-chat-sync.md` in the Baylee repository).
Central serves it since 19 November 2026 (owner's decision of that day: wanted,
with presence, rebuilt on `main`). It carries no content: it tells a signed-in
client that something concerning its own account changed, and the client
reads what changed over `/v1/native` as before. Presence, which friends of the
account are online, is the one thing it says that `/v1/native` does not.

Code: `crates/central-http/src/ws.rs` (the route, the handshake, the socket),
`ws/frame.rs` (the frames), `server.rs` (the upgrade's connections),
`crates/central-identity/src/live.rs` (the registry: who is told what) and
`crates/central-postgres/src/accounts/live.rs` (what the store answers).

## The contract

`GET /v1/ws` on the account listener (`wss://central.baylee.app/v1/ws` behind
the proxy), with one `Authorization: Bearer <Central token>` of a **native**
session and an RFC 6455 opening handshake (version 13). No protocol header: the
socket's events are its own vocabulary, so the route stands below the
protocol's layer, as the key set does.

The server sends JSON text frames, each at most 4,096 bytes (the client reads
no more):

| frame | when |
| --- | --- |
| `{"event":"ready"}` | first, once the socket is registered; the client then reads everything once |
| `{"event":"presence","online":["002a",…],"states":"o…","me":"online"}` | right after `ready`, and whenever it changes, at most once a second: the tags (lower-case hex, at least four digits, as the client writes them) of the account's friends that are shown online, in order; a letter of `states` for each (§"Status"); the account's own choice |
| `{"event":"social"}` | the account's contacts, requests, blocks, groups, invitations or own devices changed |
| `{"event":"chat"}` | a delivery reached one of the account's devices (its own sends included) |
| `{"event":"revoked"}` | the socket's session ended; a close (1008) follows. The client signs out on it |

`social` and `chat` are hints: several changes may come as one frame, and a
frame says nothing of what changed. The client sends only control frames; it
answers the server's pings (every 30 seconds).

Closes: the client's own close is answered with its code (1000 when it gave
none; a code no peer may send, or a reason that is not UTF-8, is a protocol
error, 1002 or 1007); 1001 when the listener stops or
the socket is 12 hours old; 1002 for a frame RFC 6455 does not allow, 1003 for
a data frame from the client, 1008 after `revoked` or for more than 16 frames
in 10 seconds from the client, 1011 when what the socket says can no longer
be kept true (below). A socket silent for 75 seconds, pongs included, is
dropped without a close: it is taken for dead. Only 1008 after `revoked`
means the session ended; after any other close or drop the client dials again.

Refusals before the upgrade, in this order, with the account listener's
fixed vocabulary:

1. the boundary's: a `Host` other than the listener's, an `Origin` or
   `Sec-Fetch-Site` (403: no browser), the request's time;
2. a connection whose first line was not exactly `GET /v1/ws HTTP/1.1`: 400;
3. a body (`Content-Length` or `Transfer-Encoding` at all), a missing or
   doubled `Upgrade: websocket`, a `Connection` without `upgrade`, a key that
   is not sixteen bytes in canonical Base64: 400; a version other than 13: 426
   with `Sec-WebSocket-Version: 13`;
4. the address's dials (60 a minute) and sockets (32), the listener's sockets
   (512): 429;
5. no `Authorization` or two, a gateway's credential, a token that does not
   verify: 401;
6. the session's dials (12 a minute), the session budgets of a status request
   (`limits.rs`): 429; a session that does not stand: 401; Central's terms in
   force not accepted: 403; the account's sockets (4): 429; the store
   unavailable: 503.

A socket counts as one of its account's only from its `ready` on, and is
registered before it, so no news between `ready` and the socket's first wait is
lost.

## Why a socket hears only its own account's news

Nothing is broadcast. Each socket has a doorbell and a set of bits (`news` in
`live.rs`); news names accounts, and only those accounts' sockets are rung.
A socket that is not named is not woken at all, so it cannot learn by a
frame's timing that something happened elsewhere. What names the accounts:

- **A social operation**, on either listener (the website's changes reach the
  native sockets too), says whom it concerns in the transaction that made
  it, from the rows it locked (`Concerns`, `central-postgres`), and the service
  tells exactly those after the commit:
  - a **chat send** names the sender and every account at least one of whose
    devices took a delivery. An account that blocked the sender, or whose inbox
    had no room, took none and is not told; a retried send delivers nothing new
    and tells nobody;
  - a **contact change** names the side whose own view changed: a request or an
    acceptance both, a removal both if there was a contact, a block or its
    lifting the blocker only (the blocked account sees no list of who blocked
    it, only a contact that ended, if there was one). A change that changed
    nothing names nobody;
  - a **group change** names who sees it in the group snapshot: a joined
    member sees everything, an invitee only the group's name and its own
    inviter, so an invitee is told of a rename, the group's end and the end of
    its own invitation only; a new group its creator;
  - a **new device** its own account only (others read keys when they send);
  - reads name nobody.
- **A session's end**: a native sign-out revokes exactly that session's
  sockets; a native sign-in, a new password, a reset, a deletion, a pairing
  and ending the client's session on the website ask the account's own sockets
  to check their session (one statement each, at most four). A dial during
  which any such news came (between its status and its registration, when
  the news found no socket of it) asks the store once more before it is
  admitted.
- **An account's deletion** also tells the accounts that counted it among their
  friends (they forget it at once); **a new tag** set by an administrator, those
  that count the account among their friends.

## Presence

An account is online while one of its sockets has said `ready`. A socket
reports the tags of its own account's friends that are online: friends are
accepted contacts, either side having asked; a block ends the contact, so a
blocked account never appears, and neither does a pending request.

The registry reads an account's friends once, when its first socket opens, and
again for both accounts of a friendship that began or ended (an acceptance, a
removal or a block of a friend; read, not inferred). A request, or a block of
somebody who was no friend, reads nobody's friends: otherwise anyone could
block and unblock any tag and tell by the answer's time whether its account
holds a socket. Going online or offline reads nothing: the accounts to
tell are those that count the account among their friends, found in memory.
Each list keeps its own account's view, so what a socket is told always agrees
with what it reports. A newer reading always wins over an older one that ends
later. A reading that fails closes the account's sockets (1011) rather than
keep reporting a friendship that may have ended; their clients dial again.

The readings run in a task of their own, begun when the change has
committed: the request does not wait for them, so neither its answer's time
nor its caller's connection depends on them, and a caller that goes away
cancels nothing. A reading that fails, or a task that never runs, ends as
failed and the sockets close, so a list that was to be read again is never
left as it was. Readings are numbered over all accounts, so one begun before
an account's sockets closed and opened again is older than every later one.
What remains is the instant between the store's commit and its answer: a
request dropped exactly there tells nobody, and the two accounts' sockets
keep the friends they had until their next reading or their end.

Presence is kept in memory only, never stored or logged, and ends with the
socket. It is a new disclosure. Its legal basis is the contract (Art. 6(1)(b)
GDPR; the owner's choice of 19.11.2026): the terms of use name it since their
version 2026-11-19 (§ 1), which every account accepts anew, and the privacy
policy (`legal/privacy.*.md` § 5a, § 6 and § 7) and the website's privacy page
say what it shows to whom. Both await the owner's review. There is no switch
to hide it.

## Cost

| event | database | in memory |
| --- | --- | --- |
| dial | the session's status, the terms' acceptance, the choice again (in the account's turn) if a choice settled meanwhile, the account's friends if no socket of it is open, and the session again if news of an ended session came meanwhile: 2–5 statements | bounded registry entries |
| chat send, group change, new device | none beyond the operation's own | bits for the named accounts' sockets (at most 51 accounts × 4 sockets for a group) |
| a choice (`SetPresence`) | the session's status and the write: two statements, in the account's turn | bits for the account's sockets, and its followers' if how it is shown changed |
| a reading of it (`Presence`) | the session's status: one statement | — |
| a report (`ReportPresence`) | none | bits for its followers, only if how it is shown changed |
| contact change | for a friendship that began or ended only, one read of each of the two accounts' friends that holds a socket (≤ 2 statements of ≤ 300 rows), in a task the request does not wait for; a request or a block of a stranger reads nothing | presence bits for the two and whom their lists gained or lost |
| sign-out | none | bits for that session's sockets |
| sign-in, password, reset, deletion, pairing, client session ended | none at the event; each of the account's sockets checks its session: ≤ 4 statements | bits |
| online / offline | none | one pass over the accounts with a socket (≤ 512) |
| periodic check | one statement a minute for every socket's session, at most 1,024 digests each | — |

At the listener's limit of 512 sockets the periodic check is one statement a
minute naming 512 digests, against the four connections of the pool; the
sockets' sessions are otherwise checked only when their account's sessions
changed. A check that fails twice in a row (two minutes without an answer)
closes every socket (1011), since a session may have ended meanwhile. Sockets
whose session ended by a path that does not pass this service (its expiry,
the operator's tools) learn it at the next check, within 70 seconds (a
minute between checks, ten seconds for one).

## Limits

| limit | value | why |
| --- | --- | --- |
| sockets a listener | 512 | each is a file descriptor beside the listener's 128 connections; the unit's default of 1,024 descriptors keeps room. Raising it needs `LimitNOFILE` in the unit (`docs/operations.md`) |
| sockets an address | 32 | a sixteenth: filling the listener takes sixteen addresses with eight signed-in accounts each |
| sockets an account | 4 | a client holds one; a redial before the old one's end is noticed holds a second for up to 75 seconds |
| dials a minute, address | 60 | thirty clients redialling at their fastest; counted before anything is asked |
| dials a minute, session | 12 | the client redials after 2 s at the earliest, doubling while it fails; counted for 4,096 sessions at once, a newcomer to a full table taking the oldest's place (tokens of ended sessions still verify until their `exp`, so they could fill it, but never lock a session out) |
| frames from a client | 16 in 10 s | a pong a ping, and pings of its own |
| a frame from a client | 131 bytes | a control frame's longest; a data frame is refused at its header |
| a frame to a client | 4,096 bytes | the client's limit; the longest presence (300 friends, eight-digit tags, a state each, the longest choice) fits |
| presence frames | one a second a socket | news within the second waits; the next frame says the latest |
| choices, reports | 10 and 30 a minute a session | a choice is a statement, a report none |
| a write | 10 s | a client that does not read loses its socket |
| idle | 75 s | two pings unanswered |
| lifetime | 12 h | no session lives longer |
| upgrade | 5 s | after the 101 |
| the first line | 1 s | to show `GET /v1/ws HTTP/1.1`; slower is an ordinary connection |

## The connection

The account listener answers one request per connection, with no keep-alive,
and that stays so for every connection whose first line is not exactly
`GET /v1/ws HTTP/1.1` (hyper would otherwise add `Connection: close` to the
101, which an intermediary may act on). The server peeks at the first line
without taking it; such a connection may be upgraded, and every other answer
on it says `Connection: close`, after which hyper ends it, so a request
pipelined behind a refusal is never answered. An upgraded socket leaves the
listener's connections (it holds none of their 128 places and is not cut at
their time) for a set of its own, which the listener's shutdown closes (1001,
then two seconds for the client's close) and, after the three seconds' drain,
aborts.

## Framing

There is no approved WebSocket library: the HTTP stack's approved features
do not include axum's `ws` (which needs `tokio-tungstenite`), so the framing is
first-party and small. The handshake's accept value uses the approved `sha1`
and `base64` crates; SHA-1 there is the protocol's checksum of the client's
key, not a security property. `sha1` 0.11.0 is new in the build (it was in the
lock through `sqlx-mysql` only); it chooses x86 SHA-NI or AArch64 SHA2
intrinsics at run time, in `unsafe` of its own, and has inline assembly only
for LoongArch, which nothing here targets. `cpufeatures` and `digest` under it
were in the build already, through `sha2`. The reader keeps what it has read in its own
buffer and awaits nothing but one `read`, so the `select!` that also waits for
news and timers loses no byte when news wins (tested with a frame cut in
half). Writes are never inside a `select!` and each has its time.

## What it does not do

- It does not deliver content, and a missed frame loses nothing: the client
  reads the inbox and its contacts once a minute while the socket is live.
- Changes that do not pass this service are told within 70 seconds, by the
  periodic check, not at once: a session's expiry, the operator's tools, and
  the bridge's own invalidation of a legacy account, which this service
  passes without knowing whose sessions it ended.
- A group member that is no friend of a deleted account is not told by the
  socket that the deletion changed its group; its client reads its groups
  within the minute.
- A native password change through the client's own SDK ends the old session,
  so its socket says `revoked`; the Baylee client does not change the password
  natively today (the website does).
- No administrator action ends a native session on `main`: an administrator's
  reset code does only once the person uses it.

## Status

The owner's decision of 19.11.2026: a person chooses how its friends see it,
and the client says what it is doing (`central_api::presence`).

- **The choice** (`SetPresence`, on either listener, under Central's terms):
  `online`, `afk` (away) or `offline`. Stored with the account
  (`accounts.presence`, migration 000033, online by default), so it holds on
  every device and after a restart; it is read with the dial's status.
  `Presence` (either listener, without the terms) reads it back; the
  website's download carries it. It is not in the account's export, which
  released clients decode strictly (`docs/account-deletion.md`). An account
  that chose `offline` is never shown: its friends see it offline, and its own
  news still reaches its sockets.
- **What the client does** (`ReportPresence`, native only): `playing` (it sits
  in a game that is running, online or against the house AI), `table` (it sits
  at a table whose game has not begun), or neither; and `idle` (no input for
  ten minutes, which shows the account away). Watching a game and building a
  deck are neither: online. Only these fixed values exist: no game, table,
  room or gateway is named. Heard only from a session that holds a push
  socket, and kept in memory on that session's sockets, going with them; a
  report without a socket changes and keeps nothing, and is answered alike.
  The newest report of the account's sockets stands, so a device that went
  says nothing more of what it did.
- **What friends see**: an account is shown while one of its sockets said
  `ready` and it did not choose `offline`; then away if it chose `afk` or is
  idle, and its activity. Friends are told only when that changes: connecting,
  disconnecting, reporting or idling while `offline` tells nobody anything;
  choosing `offline` and back is the one change they see of it.
- **The frame**: `online` lists the friends shown, as before, so a client that
  reads only `online` sees each of them online. `states` has one letter per
  tag, in the same order: `o` online, `a` away, `g` playing, `t` at a table,
  `G` and `T` playing or at a table while away. `me` is the account's own
  choice. At 300 friends with eight-digit tags the frame is 3,659 bytes, below
  the 4,096 the client reads (a const assertion holds it). A socket says
  presence at most once a second; news within that second waits, and the
  frame that follows says the latest. A frame said takes whatever was held
  back with it, so the next change waits a whole second again.
- **The account's turn** (`Live::turn`): a choice holds its account's turn
  from before its write until the registry has taken it, in a task of its own
  that a dropped request does not cut short (`Live::settle`); a dial admits
  its socket in the same turn (`Live::admit_in_turn`). So the registry takes
  an account's choices in the order the store wrote them, and a socket is
  never admitted with an older choice than the store's: a dial reads its
  status (one statement, with the choice), and only if a choice settled
  meanwhile reads it again in the turn, where no write is under way; its
  reading then replaces the entry's, whose sockets say it. Only a choice
  counts as settled, never a reading, so no dial makes another read again.
  `offline` is taken before its write (hidden is the side to err on), so no
  socket admitted earlier says `ready` under the older choice while the write
  is under way, and a write that fails leaves it hidden; any other choice is
  taken once stored, and one that fails is left to the next dial's reading. A
  write has five seconds (a request's time), then counts as failed and lets
  the turn go. However a choice's step ends, a panic included, it counts as
  settled before the turn is let go. One account's turn never waits for
  another's; the turns are kept only while held or awaited.
- **Left as it is, on purpose**: every settled choice counts, one that
  changed nothing and one that failed included, so each dial in flight then
  reads the choice once more; counting only real changes would need the
  stored value from before, and the cost is one statement a dial a choice,
  without a cascade. A choice's budget is spent on the digest of the bearer
  before the session is checked, so made-up cookies on the website's listener
  can take the place of real sessions in the table of 4,096; a session pushed
  out starts its minute anew. Checking the session first would bring back a
  statement before the budget, and refusing newcomers to a full table would
  let made-up cookies lock real sessions out (the table was built to avoid
  that, `SESSION_DIALS`); the general request limits per address bound both.
- **Budgets, before the work**: 10 choices and readings a minute a session,
  spent before the terms are read for a choice; 30 reports, spent before the
  registry looks for the session's sockets (found by the session, not by a
  pass over all). The registry's tables hold at most 4,096 sessions, a
  newcomer taking the oldest's place.

### For the client

- After each dial, once the socket said `ready`, report what the client is
  doing (`report_presence` in `central-client`), even if nothing: the
  registry starts every account at "online, doing nothing".
- Report again when it sits down at a table or gets up, when a game begins or
  ends (online or against the house AI), and when ten minutes without input
  begin or end. Nothing else, and not more than 30 times a minute.
- Set the choice with `set_presence`; the answer is the stored choice, and
  every socket of the account says it as `me`. `presence_choice` reads it.
- Read a frame with `central_client::read_presence`: when `states` is missing
  (an older Central), not exactly as long as `online`, or holds a letter not
  known, every friend it does not describe is read as online, never as an
  error. A frame's `online` stays the list of friends to show.

The Baylee client needs the Central of the pull request that brings this
(on `main` after f9d797f6, the push socket's merge).

## The switch, and its legal basis

Asked on 19.11.2026, the owner chose the contract as presence's legal basis,
with the terms describing it, and then wanted the status above, with
`offline` as the way to appear offline. Whether an account is online, and what
it is doing, is never stored; only its choice is.
