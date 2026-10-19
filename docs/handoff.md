# Next implementation session

Read AGENTS.md and all linked README design documents first. Implementation began on 2026-10-19 with owner authorization. Read status.md
for completed checks and remaining gates; no production deployment or live-user
processing has begun.

Continue M1/M2 gates in order. Keep reusable libraries and isolated dev mode;
do not attempt all milestones in one change.
Higher effort is not a substitute for QA or independent security review.

Existing Baylee checkout: `C:/Users/vikto/Projects/baylee`.
Existing planning branch: `codex/account-e2e-sync`; inspect current git state before
using it, as other sessions may change main. Its `docs/account-e2e-sync.md` and
`docs/password-vault.md` explain the existing local vault and initial proposal.
The new central design supersedes gateway-bound storage proposals. Never assume
the current gateway already supports this protocol or transparent migration.

Important source areas to inspect again before edits:

- `crates/baylee-client/src/lobby/http.rs`: legacy password transport.
- `crates/baylee-gateway/src/routes/signin.rs` and moderation/password handlers.
- `crates/baylee-gateway/src/admin.rs`: loopback administration trust boundary.
- `crates/baylee-db/src/entity/`: existing persistent account/deck schema.
- Baylee dev-control skill for isolated client testing and owned-process cleanup.

Accepted decisions: private new repository, central personal storage and website,
gateway-scoped administrators, optional recovery code, real-account migration,
one final bounded legacy HTTPS password login, separate training copies, discoverable
privacy settings, persistent opt-out, unchecked one-game contribution action that
never re-enables global sharing. Default-on human training remains conditional
on reviewed anonymity/legal basis; until then it is disabled.

Binding new rule: security > QA > performance > clean code; no first-party unsafe
or assembly. Do not copy the sibling engine's contrary optimization rule.
