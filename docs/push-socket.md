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
| `{"event":"presence","online":["002a",…]}` | right after `ready`, and whenever the list changes: the tags (lower-case hex, at least four digits, as the client writes them) of the account's friends that hold a socket, in order |
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
| dial | the session's status, the terms' acceptance, the account's friends if no socket of it is open, and the session again if news of an ended session came meanwhile: 2–4 statements | bounded registry entries |
| chat send, group change, new device | none beyond the operation's own | bits for the named accounts' sockets (at most 51 accounts × 4 sockets for a group) |
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
| a frame to a client | 4,096 bytes | the client's limit; the longest presence (300 friends, eight-digit tags) fits |
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

## No switch, by the owner's choice

Presence has no per-account switch. Asked on 19.11.2026, the owner chose the
contract as its legal basis, with the terms describing it, over a legitimate
interest with a switch to object by. It is shown only to accounts both sides
accepted and is not stored. A switch ("appear offline": the account's sockets
would stay but never count as online) would need a stored setting, a
migration, if it is ever wanted.
