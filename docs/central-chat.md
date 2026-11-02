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

The website's separate worker owns the unlocked WASM keys. Only encrypted peer
trust is stored in localStorage, scoped to the immutable account ID. Messages
remain in memory and are replayed from encrypted delivery history. Local trust
storage authenticates contents, not freshness: deletion or rollback of that
storage can remove previous TOFU knowledge. Closing the worker clears its state;
page hide and five minutes hidden lock the chat. JavaScript does not promise
physical erasure of previously rendered text or password strings.

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
preferences. All 357 web tests, TypeScript checks and lint pass. Test services,
databases, browser profiles and the VM are stopped/removed after verification.

Deployment still requires the documented migration/runtime-grant workflow, a chosen
retention policy and staging acceptance for actual external push-provider delivery.
No production rollout, automatic legacy relationship import or history deletion
is performed by these changes.

PWA installation, opt-in notifications, granular profile switches and administrative
notification categories are described in [notifications](notifications.md). Chat
keys and plaintext never enter the PWA cache or push payloads.
