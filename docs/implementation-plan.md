# Implementation sequence and acceptance gates

Priorities are ordered: security, QA, performance, clean code. Work proceeds in
small reviewed commits. This plan does not authorize production deployment.

| Milestone | Deliverables | Exit evidence |
| --- | --- | --- |
| M0 — preparation | Private repo, decisions, threat model, privacy plan, handoff | Documentation links and diff checked; no implementation |
| M1 — safe foundation | Reviewed dependency inventory, Rust workspace, CI, isolated DB/object store fixtures, configuration/redaction | Unsafe prohibition coverage; fmt/clippy/tests; dependency/advisory/license/secret scans; clean-room build |
| M2 — protocol design and spike | Exact OPAQUE suite/library, browser SSO decision, key hierarchy, credential/reset/migration state machines, API schemas | RFC vectors, negative tests and design review before production implementation |
| M3 — identity and sessions | Accounts/devices, registration/login, refresh/revocation, MFA/step-up and recovery authorization | Enumeration/rate limits, replay, token confusion, concurrency, expiry and secret-leak tests |
| M4 — vault and synchronization | Automatic creation/restore, encrypted objects/manifests, password change, optional recovery, device lifecycle | Multi-device restore, corruption/rollback, lost responses, atomic change, recovery and revoked-device tests |
| M5 — legacy migration | Bound legacy bridge, identity linking, durable progress, rollback/reconciliation | Realistic synthetic legacy fixtures; fault injection at each step; no data loss or downgrade after cutover |
| M6 — gateway directory/admin | Ownership, listing, scoped roles, audience tokens, revocation and safe health probes | Cross-gateway denial, heartbeat-key isolation, SSRF, last-owner transfer and outage tests |
| M7 — personal data and website | Deck/replay/settings sync, contact metadata, public/account/operator pages, export/deletion | Browser accessibility/security, ownership and retention tests; no unsolicited vault decryption in browser |
| M8 — training pipeline | Synthetic-data intake, validators, visibility masks, preferences, one-game grants and lineage | Poisoning/malformed input tests, no hidden-info leakage, preference races, re-identification assessment |
| M9 — security/operations release | Independent review, deployment isolation, restore/rotation/incident runbooks, monitoring and capacity evidence | Closed blocking findings; restore drill; legal/privacy gates; owner release approval |

Human training ingestion is a separate disabled feature until legal basis,
anonymization limits, rights review and preference enforcement are approved. M8
can be implemented and tested using synthetic data while that gate is unresolved.

## QA strategy

- Unit/property tests cover protocol state transitions and authorization matrices;
  fuzz untrusted parsers and replay input. Crypto correctness relies on maintained
  implementations plus published vectors and integration tests, not DIY primitives.
- Integration tests use real isolated PostgreSQL/object storage and the actual
  client/server serialization. Exercise cancellation, retry, clock skew, deadlocks,
  optimistic-write conflicts, duplicate requests and partial network failure.
- End-to-end tests cover registration, second-device restore, password change,
  optional recovery, reset without recovery, explicit new-vault creation, migration,
  admin removal, account export/deletion and training opt-out across devices.
- Adversarial tests verify cross-tenant isolation and redaction in logs, metrics,
  error bodies and crash artifacts. All tests use fabricated accounts and data.
- Pin CI actions/dependencies and retain reproducible build/test evidence. Record
  scanner scope and suppressions with justification; no blanket ignore lists.
- An upstream crypto audit is not an application audit. Before release obtain
  independent review of the exact protocol, integration and deployment boundaries.

## Performance after correctness

Set measurable SLOs from a stated workload in M1: concurrent accounts/gateways,
object sizes, login KDF memory budget, p95/p99 latency, throughput and storage cost.
Benchmark on representative hardware and supported client devices. Bound KDF and
validation concurrency, use backpressure, cursor pagination and bounded streams.
Never lower cryptographic strength or reuse secrets/nonces to meet a latency goal.
No unsafe/assembly optimizations. Compare measured safe alternatives only.

## Decisions still required (not reasons to skip preparatory work)

1. Confirm license before external distribution (proposed AGPL-3.0-only).
2. Approve exact dependency inventory, especially unsafe/assembly internals; a
   whole-stack prohibition may rule out the Rust runtime and mainstream crypto.
3. Select reviewed native authentication and website/SSO integration in M2;
   neither Rauthy nor a home-grown identity provider is silently assumed.
4. Identify legal operator, jurisdiction/hosting/processors, minors policy,
   actual retention values and training rights/legal basis before release.
5. Select measured SLOs and operational scale; configure private reporting,
   branch protection and reviewer responsibilities before collaborative rollout.

Implementation may begin after the owner's next instruction, resolving M1/M2
design gates in sequence. "Ready for implementation" does not mean release-ready.
