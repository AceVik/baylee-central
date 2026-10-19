# Private objects and exclusive sessions

The account manifest and every object are encrypted in the native SDK with the
account's randomly generated master key. OPAQUE derives the password-bound
wrapping key in the client. Neither the gateway nor Central receives the master
key, the wrapping key, or a reusable password hash used as a bearer credential.
Authenticated encryption binds the issuer, account, vault epoch, object UUID and
revision. The account UUID is reserved for its manifest and cannot be substituted
for a chunk ID. A new device must authenticate, unwrap the same master key and
download the encrypted manifest and referenced chunks before restoring history.
Backing up only device keys does not back up chat history.

`central-client::Session` provides create/read/write/list/delete operations without
exposing its master key. Object IDs come from PostgreSQL `uuidv7()`. Each account
has at most 16,384 stored IDs and 512 MiB of encrypted chunk content. A chunk holds
at most 256 KiB of plaintext; the separately bounded manifest references chunks.
Creation reserves an empty object. Writes and deletion use revision compare-and-
swap; callers authenticate a read after an ambiguous write response. Upload new
chunks first, commit their encrypted manifest second, and delete obsolete chunks
only after that manifest commit is confirmed. Unchanged immutable chunks can be
reused. Network errors never authorize overwriting a good local copy or creating
new chat keys.

Deleted objects retain a monotone tombstone until the ID quota requires bounded
reclamation (up to 1,000 tombstones per creation). Creation never accepts a supplied
UUID, so removing tombstones cannot recreate an old object. Missing objects fail
closed, exactly as deleted objects do. Object listing is reconciliation metadata,
not a durable deletion event stream. Abandoned reservations and uncertain uploads
require application reconciliation against the current authenticated manifest.

Exactly one session is active per account, including on the same device. A
successful OPAQUE login replaces the previous session atomically. Database row
locking and a unique account index enforce this under concurrent logins. Failed
proofs preserve the existing session. Revocation cascades to gateway tickets and
leases; device identities and encrypted data remain. Authenticated mutations lock
the account and recheck the session after acquiring that lock, preventing a query
that waited behind a replacement login from using its earlier authorization
snapshot. An already authorized operation may finish before the replacement
transaction commits; subsequent requests cannot use the revoked capability.

Ciphertext protects content from operators, not availability or all metadata:
Central observes identities, sizes, timestamps and access patterns. A malicious
server can withhold content. A previously unseen device cannot independently
detect replay of an entirely consistent older encrypted manifest without an
external trusted checkpoint. Existing devices enforce the revision floor they
have already authenticated. Password entropy and the trusted client remain
necessary assumptions; do not describe this as protection against weak-password
guessing after total server compromise or a malicious client.

## Migration interruption and runtime privileges

While migration remains open, the official gateway invalidates pending enrollment
before changing a legacy password. Central atomically archives the old encrypted
credential/wrapper/document, revokes sessions, clears enrollment and advances the
vault epoch. No private-object upload is permitted until migration has closed.
The archive is bounded to 32 entries; exhaustion fails rather than silently
destroying a retained encrypted copy. The archive currently supports operator-led
recovery investigation, not an automatic password recovery UI.

After validating that the legacy source and credential generation are unchanged
under its exclusive mutation barrier, the gateway calls `LegacyClose` with the
restored identity before local credential retirement. Closing is idempotent and
permanent. The bridge can no longer invalidate that Central account. If the local
commit or response fails, retry cutover; never reopen legacy authorization.

In addition to the native account/session/ticket and registry grants, use:

```sql
GRANT SELECT, INSERT, UPDATE, DELETE ON central_identity.private_objects TO central_runtime;
GRANT SELECT, INSERT ON central_identity.migration_archives TO central_runtime;
```

Startup checks effective table and column privileges for all nine native tables,
rejecting ownership, DDL-related privileges, extra write permissions and grant
options. The runtime cannot approve its own gateways or rewrite/delete archives.
Migrations use the separate operator connection. Synthetic real-PostgreSQL tests
cover ownership isolation, races, quota rollback, tombstone reclamation, migration
restart/closure, new-device decryption, revocation and privilege escalation.
