# Architecture and decision record

Status: proposed implementation architecture; owner requirements are binding.
Date: 2026-10-19. No production implementation exists here.

## Scope and deployment boundaries

Use a modular Rust service with explicit interfaces initially, rather than many
independently evolving microservices. Separate runtime privileges even within
one repository. Website deployment, ingestion workers and authentication signing
must not share unrestricted credentials or file access.

| Boundary | Responsibility | Must not receive |
| --- | --- | --- |
| Identity | Accounts, devices, sessions, credential lifecycle | Vault plaintext or recovery secrets |
| Personal storage | Encrypted decks, replays, settings, history, manifests | Content keys |
| Directory | Gateway registration, ownership, listing, health | User refresh tokens |
| Authorization | Gateway-specific roles and short-lived capabilities | Gateway-supplied global roles |
| Training intake | Eligible sanitized replay copies and dataset provenance | Vault keys or account library access |
| Training workers | Validated datasets and model provenance | Identity database and signing keys |
| Website | Public pages, directory, account and operator management | Private vault keys by default |
| Game gateway | Games, engine, tables, local moderation | Unrelated account data |

Proposed infrastructure: PostgreSQL for transactional metadata; object storage
for ciphertext and separately permissioned training datasets. Exact libraries,
versions, hosting and service accounts are selected in M1 after review. Personal
content is encrypted client-side; TLS and storage encryption still apply.

Existing games should continue during a central outage. Fresh sensitive actions
fail closed. Specify token lifetimes, revocation freshness and outage behavior
per action before implementation; availability must not silently extend admin rights.

## Authentication and vault decision gate

Prefer native-client OPAQUE via an RFC 9807 implementation to meet the requirement
that central servers do not learn the password in the new protocol. Rauthy is
not selected: a normal password-based Rauthy login does not meet that requirement
by itself. Standards-compliant browser SSO and password-blind native login need
an explicit integration design; do not invent OAuth/OIDC endpoints and call them
compliant. M2 must choose reviewed components/protocols before implementation.

OPAQUE export-key derivation, password KDF costs and ciphersuite are versioned
and bounded locally. The shared session key is NOT a vault key. Generate a random
account master key client-side, use purpose-separated authenticated wrappers and
per-object keys, and bind issuer, immutable account, object identity, epoch and
version. Password changes atomically update credentials and the wrapper for the
same master key, with compare-and-swap and idempotent retry. Nonces must never
repeat under a key. Reject corrupt/unknown records; errors are not empty vaults.

Each device has its own identity; never clone signing seeds across devices.
Recovery code is optional, random and client-only. An authorized unlocked device
can recover through explicit, authenticated pairing. Password reset alone cannot
decrypt the old vault: preserve it, never silently replace it. Revocation cannot
erase previously copied keys; old backups may remain decryptable by old secrets.
Rollback protection must specify fresh-device limitations against a malicious
server; authenticated ciphertext alone does not prove it is the latest revision.

## Gateways and administration

- Registration is not publication or trust certification. Separate registered,
  listed, verified and official states. Initial listings require approval.
- Authenticate gateway ownership with a bound public key and endpoint challenge.
  Protect URL checks against SSRF, DNS rebinding, redirects and private networks.
- Separate gateway heartbeat credentials from owner/admin-management credentials.
  A stolen running gateway key must not appoint administrators.
- Roles use immutable `(gateway_id, account_id)` identifiers; invites require
  acceptance. Prevent accidental removal of the last owner and specify transfer.
- Tokens are short-lived and audience/action constrained. No central refresh
  token is sent to a gateway. Enforce rights on every backend operation.
- Role removal, account suspension and device revocation need bounded propagation;
  sensitive actions require fresh authorization and step-up authentication.
- Derive the actor from validated credentials, never a browser-supplied actor
  header. The existing Baylee loopback admin proxy is not a public SSO endpoint.
- A modified foreign gateway can bypass its own local checks, but must gain no
  additional central permissions or access to other gateways.

## Website

Public: project information, downloads, documentation, directory and status.
Account: sessions/devices, export/deletion, training preference and recovery setup.
Operator: ownership, gateway configuration and scoped administrator management.
Public fan content remains accessible without requiring email registration.
Private-browser vault access is deferred pending a separate threat review.

Plan CSP, CSRF protection, secure cookies, origin separation, accessible controls
and output encoding. Directory text and uploaded metadata are untrusted. A
compromised public website must not automatically compromise signed native updates.

## Migration from existing Baylee accounts

Real accounts exist. The owner explicitly permitted one final legacy plaintext
password login over verified HTTPS for migration. This exception does not become
a downgrade mechanism: bind it to a known legacy account, original gateway and
reviewed migration state; no arbitrary server-directed credential destination.
Never log that password or claim it was never received by the old gateway.

Bind legacy account ownership to enrollment through a short-lived, single-use,
audience/account-bound migration authorization. A stolen ordinary session alone
must not permit credential replacement. Never merge identities by matching names
or emails: require explicit ownership proof and link confirmation.

Keep old vault/history intact until authenticated restore of the new backup
succeeds. Maintain resumable migration state and cross-system reconciliation;
there is no atomic transaction spanning independent gateways and central storage.
After cutover, disable the legacy login path for that linked account, revoke old
sessions and prevent stale clients from undoing migration. Lost responses, offline
devices and failed cutover must have tested recovery paths without data loss.
