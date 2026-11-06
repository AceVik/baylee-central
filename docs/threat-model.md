# Initial threat model

This is a review input, not a completed security assessment. Assets: passwords,
master/device/content keys, identity and signing keys, account metadata, encrypted
backups, private decks/replays, roles, training data, software updates and audit logs.

Attackers include unauthenticated clients, malicious account owners, hostile or
compromised gateways, stolen devices/sessions, malicious uploads, compromised
operators/storage, dependency attackers and abusive directory registrants.
Trust official client distribution separately from foreign gateways. Central
compromise must not expose existing vault plaintext, but can expose metadata,
deny service and attack authentication; weak-password guessing remains a risk.

| Threat | Required control | Evidence before release |
| --- | --- | --- |
| Cross-account access/IDOR | Ownership on every metadata/object operation | Negative tests across accounts, devices and gateways |
| Credential replay/downgrade | Bound protocol transcripts, one-use challenges, migration state | RFC vectors and adversarial replay/destination tests |
| Token theft/confusion | Ed25519 only, the header compared with fixed bytes so that nothing in a token selects an algorithm or key; `iss`, `aud`, `exp`, `nbf` always checked; one `typ` per kind; a token for one gateway alone, an hour long; a bearer keeps to its side; rotation with overlap. A stolen gateway token works at that gateway until it expires or is revoked: nothing binds it to the client yet (DPoP, later). A taken-over service or a leaked signing key can mint gateway tokens for any account, and needs a session's capability besides for a Central token | Every refusal by its exact reason; a gateway token at Central and at another gateway; a copied capability; a changed token; a token whose session ended |
| Hostile gateway | Its `sid`s are random per session, gateway and purpose, so it cannot correlate players with another gateway or name a session it was not given; a message names `sid`s and no reason; it holds no token Central or another gateway takes. It can ignore revocations and keep a player seated until the token expires, as it can run its own table anyway | `sid` separation, revocations read per stream under the gateway's own credential |
| Missed revocation | Numbers with no holes in the transaction that ends the session; catch-up by heartbeat and fetch; two hours kept by age, longer than any token; a gateway unable to catch up ends what it holds. A gateway Central cannot reach hears in seconds, and one that loses Central entirely holds its players until their tokens expire, an hour at most | Rollback leaves no hole; concurrent ends; sweep keeps a prefix; a page that aged out |
| Admin escalation | Separate owner/heartbeat keys, scoped roles, step-up; Central administrators act through the runtime login, guarded by the browser-only `Admin` request, the role checked where it acts, a password sign-in under 30 minutes for changes, the append-only audit and the exact grant check ([administration](administration.md)); a taken-over service can appoint administrators and act without an audit event | Compromised heartbeat cannot mutate admins; refused stale, self and last-administrator changes; concurrent removals leave one |
| Vault substitution/rollback | AEAD context, epochs, revisions, explicit freshness limits | Swapped objects/accounts, stale revisions, fresh-device tests |
| Password reset takeover | Separate authentication reset and key recovery | Email/session possession cannot decrypt or pair devices |
| Account deletion by a session thief | A fresh OPAQUE proof of the password inside the deletion, bound to its session and listener, charged to the login budget; administrators and open moves refused, rechecked under the account's lock ([account deletion](account-deletion.md)); a taken-over service can delete accounts | Stolen-session, crossed, replaced and lost-answer tests; an appointment racing the deletion; no row naming the account left |
| Export over-disclosure | The session's own account only, a sign-in under 30 minutes, no credential, wrapper, digest or ciphertext, bounded lists | Stale, other-kind and foreign-session tests; field-by-field comparison |
| Data loss in migration | Durable state machine, preserved sources, restore proof | Failure injection at every transition |
| Legacy password guessing and verifier theft | Central's one password path, until the store's deadline ([native login](native-login.md#legacy-login)): its own schema and role, the runtime role closed out and both checked at startup; Argon2id under the gateway's fixed parameters only, wiped work memory, constant-time comparison; one answer and the same work for every refusal before the proof; 30 sign-ins a minute in all and five per name, ten failures lock an account for an hour; rows deleted at the close and after the deadline. A taken-over service or a copy of the store can attack these few verifiers offline, as one of the gateway's database could; the lockout lets anyone who knows a legacy name lock it out for an hour at a time | Wrong password, budget, lockout, replayed ticket, closed move, Central-only name and bridge-refusal tests over real HTTP; role-privilege tests both ways; the gateway's own hash reproduced |
| Resource exhaustion | Bounded KDF, input size/depth, concurrency and storage | Oversized/malformed inputs, load and cancellation tests |
| SSRF through registry | Central calls a gateway only to push a revocation, only once a person approved its origin: verified HTTPS, a registered path of a fixed alphabet and never a URL, public addresses checked and dialled in one step (`central-outbound`), no proxy, no redirect, short limits, the status alone read, a bounded queue. One named exception: the official gateway's compiled host at an operator-set loopback or private address, as the same verified HTTPS | Private, mapped and link-local spellings; a name with one private answer; hostile paths; a pin opening no other name; redirects not followed; no development sender |
| Upload parser compromise | Isolated bounded validation, no arbitrary execution | Fuzzers, decompression bombs, invalid replay versions |
| Website compromise | CSP/CSRF/output encoding, isolated credentials | XSS/CSRF/session fixation and origin tests |
| Training re-identification | Data minimization, separated metadata, risk assessment | Stream/public replay linkage attempts and log review |
| Poisoning/model cheating | Provenance, quarantine, engine validation, visibility masks | Forged transitions and hidden-information leakage tests |
| Supply-chain compromise | Pinned dependencies/actions, SBOM, review and signing | Reproducible provenance and update verification |
| Insider/backups compromise | Least privilege, key separation, encrypted backups | Restore drills, access audit and key rotation drill |

Specify rate limits without making shared IP addresses a trivial account lockout
weapon. Audit sensitive actions without recording secrets or complete payloads.
Define which metadata remains visible and retain only what serves an explicit purpose.
Account enumeration, device substitution, session fixation and recovery-code brute
force must be explicitly covered in M2 tests. Hardware compromise and malicious
client binaries cannot be solved by server-side encryption design.
