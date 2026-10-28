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
| Vault substitution/rollback | AEAD context, epochs, revisions, explicit freshness limits | Swapped objects/accounts, stale revisions, fresh-device tests |
| Password reset takeover | Separate authentication reset and key recovery | Email/session possession cannot decrypt or pair devices |
| Data loss in migration | Durable state machine, preserved sources, restore proof | Failure injection at every transition |
| Resource exhaustion | Bounded KDF, input size/depth, concurrency and storage | Oversized/malformed inputs, load and cancellation tests |
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
