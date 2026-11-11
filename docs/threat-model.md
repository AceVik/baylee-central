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
| Token theft/confusion | Issuer, audience, algorithm and action validation; rotation | Wrong issuer/audience, expired/revoked token tests |
| Admin escalation | Separate owner/heartbeat keys, scoped roles, step-up; Central administrators act through the runtime login, guarded by the browser-only `Admin` request, the role checked where it acts, a password sign-in under 30 minutes for changes, the append-only audit and the exact grant check ([administration](administration.md)); a taken-over service can appoint administrators and act without an audit event | Compromised heartbeat cannot mutate admins; refused stale, self and last-administrator changes; concurrent removals leave one |
| Kept browser chat identity | The chat identity sealed in IndexedDB under a non-extractable AES-256-GCM key, bound by AAD to account, epoch and tag, written only by the chat's Worker, one record per browser, forgotten on sign-out, at the session's end (also detected at the next page start), on Central's refusal and with another account's chat ([chat](central-chat.md)); script in the origin or the profile's owner can take it while it exists | Owner, foreign epoch/tag, altered ciphertext/IV and non-exportable key tests; e2e: kept after sign-in, opened after a reload without a password, empty after sign-out |
| Vault substitution/rollback | AEAD context, epochs, revisions, explicit freshness limits | Swapped objects/accounts, stale revisions, fresh-device tests |
| Password reset takeover | Separate authentication reset and key recovery | Email/session possession cannot decrypt or pair devices |
| Account deletion by a session thief | A fresh OPAQUE proof of the password inside the deletion, bound to its session and listener, charged to the login budget; administrators and open moves refused, rechecked under the account's lock ([account deletion](account-deletion.md)); a taken-over service can delete accounts | Stolen-session, crossed, replaced and lost-answer tests; an appointment racing the deletion; no row naming the account left |
| Export over-disclosure | The session's own account only, a sign-in under 30 minutes, no credential, wrapper, digest or ciphertext, bounded lists | Stale, other-kind and foreign-session tests; field-by-field comparison |
| Data loss in migration | Durable state machine, preserved sources, restore proof | Failure injection at every transition |
| Legacy password guessing and verifier theft | Central's one password path, until the store's deadline ([native login](native-login.md#legacy-login)): its own schema and role, the runtime role closed out and both checked at startup; Argon2id under the gateway's fixed parameters only, wiped work memory, constant-time comparison; one answer and the same work for every refusal before the proof; 30 sign-ins a minute in all and five per name, ten failures lock an account for an hour; rows deleted at the close and after the deadline. A taken-over service or a copy of the store can attack these few verifiers offline, as one of the gateway's database could; the lockout lets anyone who knows a legacy name lock it out for an hour at a time | Wrong password, budget, lockout, replayed ticket, closed move, Central-only name and bridge-refusal tests over real HTTP; role-privilege tests both ways; the gateway's own hash reproduced |
| Registration abuse and name probing | Open since 11.11.2026 ([registration abuse](registration-abuse.md)): Central's own proof of work (random nonce held in memory, one use, five minutes, three seconds at the soonest; 16 bits per part, rising with the hour's registrations), checked and spent before the name is looked at; `Register` 5 an hour and 10 a day per address, challenges 20 an hour; a honeypot and a minimum fill time on the page; an administrator's audited switch closes registration. What stays: many addresses with patience still create accounts, and whoever solves a proof learns whether one private sign-in name is free | Challenge once/too soon/expired/full-table tests; difficulty rise and fall; budget tests per hour and day; closed registration refused before the name; switch only for a fresh administrator, audited, a registration in flight serialised against it |
| Resource exhaustion | Bounded KDF, input size/depth, concurrency and storage. Bodies that are announced and withheld: the browser listener gives an address four requests in flight, the account listener four places per address and a quarter of its connections in all to bodies that are late or larger than 64 KiB, so requests whose small body is there are still served ([operations](operations.md#the-account-listeners-connections)). Many addresses can still fill those places for late and large bodies, and a flood of whole requests is left to the proxy | Oversized/malformed inputs, load and cancellation tests; withheld bodies over real sockets against both listeners, another address served meanwhile, places given back on every ending |
| SSRF through registry | Restricted probes and DNS/IP/redirect checks | IPv4/IPv6, rebinding and redirect attacks |
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
