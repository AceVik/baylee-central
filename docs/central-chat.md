# Account-owned chat and contacts

## Boundary and deployment state

`central-api::{social,chat,groups}` defines one account-owned protocol for the
browser cookie listener and the native bearer listener. Gateway credentials are
not accepted. The existing origin/CSRF, body-size, session-kind and terms gates
apply before database access. No production migration or deployment is performed
by this change. Native host integration and the generated browser WASM bundle
must be validated together before release.

`operation: social` carries an `action` of `contacts`, `lookup`, `contact`,
`chat` or `group`. Chat and group commands are nested under `request`.
The typed Rust enums are the normative closed request/response vocabulary.
Unknown fields, invalid tags and unauthorized transitions fail closed. Contacts,
groups, display names, device public keys, delivery times and message sizes are
server-visible metadata; message bodies and account keys are not.

## Existing live data at cutover

On 2026-11-02 the owner explicitly authorized deleting the existing live messages
and contact links instead of importing that broken legacy state. They are no longer
a preservation prerequisite for cutover. This authorization concerns the existing
live chat/contact records; it is not a request to delete accounts, account keys,
archived encrypted documents or unrelated game data, or an ongoing retention policy.
No live database was changed by the local PWA work. A reset is unnecessary for
installability or push delivery and is not silently run on service startup.

## Identity and confidentiality

The existing Baylee Ed25519/X25519/XChaCha20-Poly1305 message format lives in
`central-crypto::chat`. It includes ciphertext key commitments and a signature
over the exact header/ciphertext bytes. Recipient wraps use the native `d` field.
The account SDK derives 96 bytes of chat material from the recovered master key
using HKDF-SHA512, salt `baylee-central/chat-identity/v1`, and info comprising
`baylee-central/chat-identity/v1\0`, issuer (32), account (16), epoch (8 BE).
Those bytes are the signing seed, agreement seed and local vault key. Derivation
has an independently computed HMAC known-answer test and context-separation tests.
A password rewrap preserves the identity; a new-vault epoch creates another.
The account's public tag cannot be reassigned after a chat device is registered:
self-signatures, peer pins and conversation IDs bind that tag permanently.

Each message gets independent OS randomness. This protocol has no double ratchet
and does not promise forward secrecy after compromise of a recipient's key.
Peer directories use TOFU; subsequent key-set changes block sends until the user
confirms the exact displayed snapshot. The barrier survives directory rollback
and restart. A safety-number comparison over an independent trusted channel is
needed to authenticate first contact against a malicious directory. Group
membership is authorized by Central, not a cryptographically authenticated group
transcript. The SDK refuses injected own-account devices even on first use.

The website's worker owns the unlocked WASM keys. A sign-in to an account with
a public tag unlocks the chat in the sign-in's own worker: `chat_unlock` opens
the master key exactly as `login_confirm` would, so it proves the password the
same way, derives the chat identity and wipes the master key; from then on the
worker answers only the chat's steps (`web/src/chat/steps.ts`). The page keeps
that worker (`web/src/chat/held.ts`) while it is open.

The owner decided on 3 November 2026 that a signed-in account sees its chats and
contacts without unlocking anything, on the website as in the client. So the
browser keeps the chat identity while the sign-in lasts (`web/src/chat/kept.ts`):
right after `chat_unlock` the worker takes the identity's 96 secret bytes from
the module (`Chat::keep`), seals them with AES-256-GCM through the Web Crypto
API under a key generated non-extractable for that one record, with the
account, epoch and tag in the associated data, stores key and ciphertext in
IndexedDB and overwrites its copy of the bytes; they never reach the page. After
a reload the chat's own worker opens the record for exactly that account, epoch
and tag and rebuilds the identity from it (`chat_resume`); a record for anything
else, or one that does not open, is deleted and the chat page falls back to the
password once (a browser that never kept it, blocked storage). The browser holds
one record: signing out, the session's end (also when it ended while no page
was open), Central's refusal of the session, a sign-out in another tab and
another account's chat forget it.

What this costs: whoever can run script in this origin or read this browser
profile while the record exists can obtain the chat identity, as they could
the session and the page's memory before. It is no key to anything alone:
message ciphertext lives only on Central and is read with a session; once the
record is forgotten the browser holds neither. The non-extractable key keeps
the raw secret out of reach of script that cannot run in this origin and of a
copy of `localStorage`; it is no protection against the profile's owner. The
wasm-bindgen copy of the bytes is freed but not overwritten. A password rewrap
keeps the identity; a new vault epoch makes a record useless.

Only encrypted peer trust is stored in localStorage, scoped to the immutable
account ID. Messages remain in memory and are replayed from encrypted delivery
history. Local trust storage authenticates contents, not freshness: deletion or
rollback of that storage can remove previous TOFU knowledge. A damaged trust
record fails the chat's start closed (nothing is registered or read) without
failing the sign-in. Closing the worker clears its state. JavaScript does not
promise physical erasure of previously rendered text or password strings.

## Authorization, concurrency and storage

Every mutation rechecks the session under a row lock after acquiring affected
account locks in UUID order. Revocation, quota checks and opposing contact requests
therefore serialize. Group operations and sends first lock the group, then its
accounts; membership cannot change between recipient selection and commit.
PostgreSQL may abort a transaction that conflicts with an administrative account
deletion; no partial send or membership transition is committed.

Registration verifies the public statement against the account tag. Only devices
from the account's current vault epoch appear in directories, receive new messages,
may send, or may read an inbox. Old ciphertext is not silently deleted on reset.
At most five devices are registered per epoch. Native and browser account sessions
can read their shared logical device's durable history independently.

Direct messages require an accepted contact and no block in either direction.
A group permits up to 50 members and invitations, and each account may hold 20
memberships and invitations combined. A member may invite an accepted friend;
the invite grants no directory access, messages or earlier history. Only the owner
may rename, transfer ownership, remove another member or delete the group. An
owner must transfer or delete before leaving. Departing inviters' pending invites
are removed. A recipient who blocks a group sender still appears in the required
wrap set but receives no delivery, so the sender cannot infer that block from a
send response. New members get only messages accepted after joining. Previously
delivered ciphertext remains accessible after leaving or deleting a group.

A send verifies header ownership, canonical conversation, UUIDv7 correlation,
signature, exact current recipient set, wrap uniqueness, sizes and timestamp.
The server stores ciphertext once with device-specific wraps in the same
transaction. Exact retries return the original ID and acceptance time; conflicting
bytes for an existing sender/ID fail. All recipients' account locks are held
through sequence allocation and commit, so the ordered per-device cursor cannot
skip a later-committing transaction. Inbox reads return at most 50 deliveries and
only the authenticated device's wrap. Reads do not destructively acknowledge or
delete history. Clients retain the same ciphertext after an uncertain response.

Resource bounds: text at most 1000 characters / 4000 UTF-8 bytes; encoded plaintext
at most 4096 bytes; ciphertext at most 4144; signed header at most 512; envelope at
most 65536; directory lookup at most 60 accounts. Creation/invitation shares the
fixed account request budget (60/hour); exact public profile lookup has 120/hour.
Sending has 60/minute and a lifetime retained cap of 5000 messages per sender;
each device's retained inbox also caps at 5000. A full quota currently refuses new
sends without deleting history. Automatic retention remains an explicit pending
owner decision and is not enabled by this implementation.

### Full inboxes and one sender's share (12.11.2026)

Nothing deletes a delivery, so an inbox that is full stays full. Until this
date one contact could fill a device's inbox alone, and a send to a group
was refused whole while one member's device was full. Since then
(`central-postgres`, `social/chat/send.rs`):

- **One sender's share.** A device of another account takes a delivery
  while its inbox holds fewer than 5000 and fewer than 2500 of them are
  this sender's (`CHAT_SENDER_SHARE`, half an inbox). At half, the two
  sides of a direct conversation take the same, and it takes two senders
  to fill an inbox. The sender's own devices are held to the 5000 only:
  the sender's outbox bounds what they get.
- **A device without room is left out.** The message is stored and goes to
  every device that has room. Nothing tells the sender, or the device's
  owner, that it was left out.
- **Refused when nobody else has room.** If no device of another account
  has room, the send is `Busy` (429), so a direct message is never
  accepted for nobody. This is asked of every other device, a blocker's
  too, so setting or lifting a block does not change the answer to the
  next send. (Before, a full device was not looked at when its owner had
  blocked the sender: a group send that a full device refused went
  through once that member blocked.) What a block can still show over
  thousands of messages is under "What stays possible".
- **A message sent again.** A send whose answer was lost is sent again
  with the same bytes and is answered as the first was, before any room
  is looked at: also when that message used up the share or filled the
  inbox.
- **The sender's own device full:** `Busy`, as before. A message its own
  devices cannot keep would be missing when they read their history again.
- `Busy` is an answer both clients knew already, and the one a full inbox
  and a sender's full outbox were answered with before. The website names
  it among the errors it shows (`web/src/chat/steps.ts`). The released
  native client takes it as a rate refusal (`baylee-client`,
  `social/central/poll.rs`; `baylee-client-core`,
  `social/machine/sending.rs`): it keeps the sealed message, shows no
  failure, and sends it again a second later and after the bridge's
  pause, which starts at two seconds and doubles to thirty with each
  refusal. It has no last try. Its other chat requests (later sends, the
  inbox's poll) are not held back for good, but each wait behind that
  pause. For a full inbox or a used-up share the answer does not change
  until something is deleted, so such a message is sent again for as long
  as the client runs, and neither client says which of the reasons it
  was. That was so at 5000 before; one sender now meets it at 2500 in one
  inbox.
- **What a `Busy` that does not end costs, and was not changed.** The
  released client keeps at most 64 sealed messages to send again
  (`social/central/worker.rs`). While 64 wait, it answers its own later
  sends through Central as busy without asking, in every conversation.
  Central is asked again for each waiting message, at most about one
  request in two seconds from one client by the pauses above. Each is
  taken as far as the room query (the accounts are locked, the signature
  is checked) and stores nothing. Central looks at a message's time after
  it has looked for room (`send.rs`), so a waiting message stays `Busy`
  and is never `Rejected` for its age. Three
  ways out, none built, each the owner's to choose: retention, which
  makes room; the time looked at before the room, so that the released
  client ends such a message as not allowed ten minutes after it was
  written (it would then show its words for a refused send, the same as
  for a sender who may not write there); or a client that gives up on a
  `Busy` after a time and says so.
- **No deletion and no time limit were built.** The review recommended
  deleting on acknowledged delivery. The wire has no acknowledgement
  (`Inbox { device, after }` reads by a cursor), both clients read their
  history again from the start, and the privacy notice in force says
  history is not deleted automatically. It needs the owner's retention
  decision and an addition to the protocol.
- **What stays possible.** Two contacts, or one person with two accounts,
  fill an inbox for good, 2500 each. A sender who reaches the share is
  refused in that inbox for good, the busier side of an honest
  conversation among them. A member whose inbox is full misses group
  messages and is not told. The sender's 5000 and the device's 5000 are
  for a lifetime as before.
- **What a block can still show.** A member who has blocked the sender
  takes none of the sender's messages, so the sender's share of that
  member's inbox never fills, and that device goes on counting as one
  with room. A sender who has used up the share with everyone else in a
  group and is still answered as sent can conclude that some member's
  device takes nothing from them: a member who blocked them, or one who
  joined late. It takes 2500 messages to that group to see it.
- Cost: one query a send, which counts each recipient device's deliveries
  and how many are the sender's. Before, it counted the deliveries only.

Migrations 000020/000021 add devices, messages, deliveries, groups and members.
Central object IDs use PostgreSQL UUIDv7 defaults with INSERT RETURNING. The
sender-generated message correlation UUIDv7 is an explicit offline-client
exception, distinct from the database primary key. The delivery sequence is a
bounded ordering cursor, not an identity or authorization capability. Runtime
roles have the exact new table privileges and sequence USAGE; they own no schema.
Apply forward migrations with the operator command and update runtime grants
before restarting the service. Backups must include the new tables as well as
account wrappers and server OPAQUE keys. There is no automatic down migration.

## Validation record and remaining release work

Implemented regression coverage includes native/browser session separation,
forged device statements/signatures, wrong-account inboxes, incomplete/duplicate
wraps, retries, parallel senders, epoch reset, group invitation privacy, membership
and send races, block suppression, owner transfer and group quotas against real
PostgreSQL 18.6 under the restricted runtime role. The imported crypto suite has
117 tests; SDK tests cover interoperability between installations, context binding,
TOFU persistence, changed keys, own-device injection and malformed plaintext.
Browser tests cover strict response parsing, acknowledgement-ID binding, worker
cancellation/timeouts, account changes, coalescing and stale contact/group reads.

These are implementing-agent reviews, not an independent audit. Real desktop/phone
browser chat, groups, key verification and reload history, plus the native
Central/Gateway migration and personal transport integration, pass. The canonical
x86-64 Linux WASM build and `cargo xtask wasm --check` pass in an isolated VM;
provenance and the resulting hash are recorded in `docs/xtask.md`. The final four
real browser tests use those exact generated bytes with PostgreSQL 18.6 and TLS,
including closed-page notifications, offline public shell and persisted category
preferences. All 365 web tests, TypeScript checks and lint pass. Test services,
databases, browser profiles and the VM are stopped/removed after verification.

Deployment still requires the documented migration/runtime-grant workflow, a chosen
retention policy and staging acceptance for actual external push-provider delivery.
No production rollout, automatic legacy relationship import or history deletion
is performed by these changes.

PWA installation, opt-in notifications, granular profile switches and administrative
notification categories are described in [notifications](notifications.md). Chat
keys and plaintext never enter the PWA cache or push payloads.
