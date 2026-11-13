# Daily/Weekly challenge assurance, 13 November 2026

Design: Central owns calendar, attempts, scores and rewards. Public reads contain
no private package, future seed or winning record. Ordinary gateways cannot publish
or score. The official host and generator are trusted principals with separately
provisioned authority; this is not a proof against a malicious official operator.

Implementation: publications bind immutable content and exact executors. Account
locks serialize attempts, hints, results and deletion; publication locks serialize
withdrawal against scoring. Scores come from an authenticated engine and a durable
host outbox. Completion time survives retries and is checked against the calendar.
Listings default to private, hints require confirmation, and exports are bounded.
No dependency or cryptographic primitive was introduced for this feature.

Engineering improvement rounds before independent review:

1. A real automatic publisher run exhausted its Weekly candidate budget. Baylee
   changed deck construction, added four Daily positions and hardened private
   evidence storage, retries, host ownership and queue rotation. The publisher
   subsequently filled fresh slots and published zero duplicates on a second pass.
2. The implementing agent solved a fresh Weekly from public player views without
   reading a witness, winning on game turn seven. This led to excluding explosive
   mana and rejecting candidates with an observed victory before turn nine. Native
   current tasks now precede archives. This website refreshes once per minute,
   preserves the archive page and pauses during hint confirmation. Ten browser
   checks passed at desktop/phone sizes, with both one and five workers. Native
   rendering could not be inspected because the prescribed test monitor was absent;
   the client exited and its owned services were removed.
3. Baylee added exact retained-executor routing and capacity checks before claiming,
   independent ownership heartbeats and failure-injection/queue tests. Real Central
   HTTP rejected an incorrectly supplied host credential on a public metadata read;
   the caller was corrected. The real Daily/Weekly engine integration passed again
   in 110.71 seconds, including no consumed start without capacity, disconnect/resume,
   Sprint exclusion, all rankings and badges. All 49 Baylee PostgreSQL schema tests
   passed, including deliberate connection loss and queue rotation beyond 32 rows.

Central's PostgreSQL suite covers foreign account/gateway rejection, concurrent
starts/results, conflicting replays, assistance, publication immutability, archive
rewards, withdrawal, equal ranks, late delivery and the exact UTC deadline, export
isolation, upgrade and deletion. The final complete gate (format, policy, clippy,
tests, doc tests and release build), HTTP smoke, crypto scope, audit 0.22.2, deny
0.20.2, browser type/lint and secret scans passed. The final PostgreSQL rerun passed:
real native HTTP integration in 160.54 seconds and the complete store suite in
147.15 seconds, with its owned container/listener removed. Earlier runs under
concurrent compilation hit statement/fixture deadlines; those failures were not
counted as passes. The runner now gives each required suite its own five-minute
deadline and reports it separately. One cold run still exceeded that bound; the
subsequent run with compiled binaries passed both suites. A process sample showed
active Argon2 enrollment work rather than a database deadlock.

The PR's first Linux PostgreSQL CI run also exposed the existing signature-flood
test's dependence on completing real Ed25519 checks within one second. Development
builds now optimise the existing curve25519-dalek and ed25519-dalek arithmetic,
as they already do Argon2. Production budgets, assertions and dependency versions
are unchanged. The complete gate and real integration passed with these settings;
the updated Linux PostgreSQL CI job also passed in 5 minutes 34 seconds.

The final real automatic publisher check also passed in 128.30 seconds, including
fresh Daily/Weekly tasks and zero duplicate publications on a repeated pass. A
second blind agent-played Weekly from the slower pool was won on turn eleven
through combat, retained burn and correctly ordered triggers. No private proof
was read and no ranked result was submitted. A subsequent blind Daily was won
through combat and Giant Growth on turn one, leaving the alternative burn spells
unused.

Independent PR review examined the authority/API/SQL paths and the website/SDK
integration separately. The authority review found no actionable issue. The web
review found two P2 issues: leaving a pending hint confirmation paused background
refresh indefinitely, and switching between calendar and detail lost keyboard
focus. Back now clears hint state and restores focus to the task (including its
archive), with Refresh as a fallback. Opening a task focuses its heading, and
unchanged refresh responses preserve existing controls. Both corrections passed
the independent static re-review.

Following the owner's instruction to limit follow-up tests to changed areas,
only the two affected browser scenarios ran on desktop and phone. All four checks
passed; two test-selector/expected-open-state mistakes were corrected during the
focus case, without weakening its assertions. Production assets rebuilt, browser
type/lint checks passed, and the owned server exited with port 28792 closed.
The prior complete gate was not repeated for these focused UI corrections.

Boundaries: website browser tests use synthetic responses; the real engine path
has separate transport/database evidence. Three agent-played games and a finite solver
sample do not establish human difficulty. Retained executors still need a matching
wire/preset protocol. No production migration, credentials or deployment was used.
