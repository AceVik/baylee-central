# Native login and legacy migration integration

Owner authorization: 2026-10-19. Live issuer: https://central.baylee.app.
Only https://baylee.acevik.de may receive a legacy password, and only in the
explicit first-enrollment flow. Development endpoints are numeric loopback
addresses, selected explicitly; they cannot authorize live legacy migration.

## Boundaries

The native client authenticates centrally before displaying saved gateways.
OPAQUE registration/login, a random client master key and encrypted personal
objects use central-crypto. The protocol/endpoint rules are reusable in
central-api. Central sessions never go to a game gateway. Admission uses a
separate one-use 30-second ticket, bound to the gateway that redeems it.
Tickets also bind the operation: admission, migration export or final linking.
The gateway route supplies the expected operation; a client cannot promote an
admission ticket into permission to export or finalize migration.
Neither a central transport error nor a failed OPAQUE proof is permission to
send a plaintext password elsewhere. HTTP redirects must be disabled on both
the central and the legacy credential transports.

An enrolled account document is mandatory. Decryption failure, an unsupported
schema or an unavailable read leaves login incomplete; none initializes empty
settings or overwrites the previous copy. Compare-and-swap revisions prevent
concurrent writers from replacing newer data. AEAD binds account, issuer,
vault epoch, object and revision. A malicious server can still replay a complete
old snapshot to a fresh device without a trusted freshness checkpoint; this
limitation is not solved by encryption or an HTTP success code.

## Migration state machine

1. A live discovery response explicitly selects legacy enrollment. Unknown
   usernames and not-yet-enrolled usernames have the same response. This
   exposes enrollment state, not whether an unknown username exists on the
   gateway. Deployment-level rate limiting remains mandatory.
2. The fixed legacy gateway verifies the password afresh and checks bans/reset
   requirements. An ordinary pre-existing bearer session cannot authorize this
   step. Through a separately authenticated bridge it reserves a central UUIDv7
   and an expiring enrollment capability bound to the immutable legacy ID.
   Names/emails never merge accounts.
3. The client exports its legacy data, encrypts the new account document and
   enrolls atomically: credential, master-key wrapper and ciphertext together.
   A repeated reserve cannot replace a committed credential or data.
4. The client performs a fresh central OPAQUE login, downloads the committed
   ciphertext, authenticates/decrypts it and confirms restoration.
5. The gateway verifies durable restored state and the unchanged source/credential
   generation under its exclusive mutation barrier. It calls idempotent
   `LegacyClose` before disabling legacy login and revoking the old sessions. Original data
   is retained. Cutover is retried after lost responses, never assumed from a
   client-supplied success boolean.

An already linked legacy account also repeats this idempotent close step during
login repair, so an upgrade cannot leave its private-object storage permanently
blocked. Legacy password changes invalidate unfinished enrollment before changing
the source credential; archived ciphertext and advanced vault epochs prevent
stale proofs from becoming the current migrated credential. Closed accounts cannot
be reopened by the bridge. See [private objects](private-objects.md).

The two databases have no distributed transaction. Pending/enrolled/restored
states, immutable legacy links, source retention and authenticated reconciliation
are necessary to recover from interruption at every step. Enrollment challenges
can be restarted after process restart; committed credentials, documents and
session revocation must survive it. Secret storage never silently regenerates
OPAQUE deployment keys after a missing/corrupt file.

## Implementation status

This document specifies the integration in progress. It does not claim that
client UI, the legacy gateway bridge or production deployment are complete.
The typed API, bounded HTTP library integration, account persistence and native
SDK have now passed synthetic real-HTTP/PostgreSQL restore and failure tests.
Live discovery reveals central enrollment state for a supplied private username;
it is not a claim of account-enumeration resistance. This requires further review
before enabling the live migration discovery endpoint. Development does not
offer legacy discovery and cannot send a password to the live gateway.
The existing release gates and synthetic-data-only testing rules still apply.

## Development process

`central-service --dev-accounts-stdin` starts the real account protocol on an
explicit numeric loopback address. It takes one bounded JSON configuration from
standard input: `endpoint`, `gateway`, `database_port`, `database_user`,
`database_name`, `database_password`, `opaque_setup` and `bridge`. The last two
are byte arrays: the persisted versioned server setup and a random 32-byte bridge
credential. An optional `browser` (`listen`, `origin`) starts the website's
browser listener for a page served on a loopback origin ([web
accounts](web-accounts.md) §"Local runs"). Keep this configuration outside source control with owner-only file
permissions; never pass credentials on the command line. A restart must reuse
the setup. The service does not create keys or run migrations implicitly.
This command only accepts loopback development endpoints and the restricted
development database connector. The deployed `--accounts-stdin` mode, with the
live endpoint and the verified-TLS connector, is described in
[operations](operations.md).
