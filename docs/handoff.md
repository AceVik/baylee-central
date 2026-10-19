# Next implementation session

Read AGENTS.md and the linked README design documents first. Implementation began
on 2026-10-19 with owner authorization. Read [status](status.md) for completed
milestones and remaining gates; no production deployment or live-user processing
has begun.

## Current integration and deployment handoff — 19 October 2026

Central is public, with the reviewed integration on `main` at
`43d89eed8cc89cd6ebc992e6de0f5e5610d54c0f`. The sibling Baylee integration is
tracked in [PR #390](https://github.com/AceVik/baylee/pull/390); check its actual
revision and merge state before continuing. It pins that Central revision and
implements native login, guarded legacy migration, gateway/admin SSO, exclusive
sessions and encrypted chat restoration. This is not full personal-data
centralization or a production deployment.

The owner chose to continue production provisioning from the MacBook, where the
SSH identity for `viktor@acevik.de` is configured. This Windows review makes no
SSH connection, deployment or live-account migration. Before deployment, build
and review a production account launcher and database connector: the existing
`--dev-accounts-stdin` launcher accepts only numeric-loopback development endpoints
and uses the local-only, non-TLS database adapter. Putting it behind a public
reverse proxy does not make it a production configuration.

Provision verified database TLS, separate runtime/migration/operator roles,
persisted OPAQUE and bridge secrets, backup/restore evidence and bounded abuse
controls. Review live discovery's enrollment-state disclosure and measure
capacity before exposure. Then provision `https://central.baylee.app`, deploy the
compatible official gateway bridge/schema and validate migration/restoration in
staging before real-account cutover. The native client must not require an
unavailable Central endpoint. See [native login](native-login.md),
[PostgreSQL](postgresql.md), [gateway registry](gateway-registry.md) and
[private objects](private-objects.md) for the protocol and runtime grants.

Private deck CRUD/history and the durable replay archive remain unimplemented.
The encrypted legacy deck snapshot is only a backup, and source migration is
still bounded to 192 KiB / 2,048 rows. The next implementation contract is in
Baylee's `docs/central-private-library-plan.md`; its `docs/central-login.md`
records the matching MacBook handoff. Registration, password change/recovery UI,
browser SSO, website and human-training release gates remain separate work.

Keep reusable libraries and isolated dev mode; finish the remaining milestone
gates in small reviewed changes rather than treating the integration as release approval.
Higher effort is not a substitute for QA or independent security review.

## Historical planning context

The initial Windows Baylee checkout was `C:/Users/vikto/Projects/baylee`, with
planning branch `codex/account-e2e-sync`. Its `docs/account-e2e-sync.md` and
`docs/password-vault.md` record the local-vault starting point and initial proposal;
they are not the current integration branch or deployment instructions. Central's
design supersedes gateway-bound personal-storage proposals. Inspect the actual
checkout and deployed gateway version before relying on migration support.

Important source areas to inspect again before edits:

- `crates/baylee-client/src/lobby/http.rs`: legacy password transport.
- `crates/baylee-gateway/src/routes/signin.rs` and moderation/password handlers.
- `crates/baylee-gateway/src/admin.rs`: loopback administration trust boundary.
- `crates/baylee-db/src/entity/`: existing persistent account/deck schema.
- Baylee dev-control skill for isolated client testing and owned-process cleanup.

The repository was initially private; the owner subsequently authorized public
source on 2026-10-19. Other accepted decisions remain: central personal storage and website,
gateway-scoped administrators, optional recovery code, real-account migration,
one final bounded legacy HTTPS password login, separate training copies, discoverable
privacy settings, persistent opt-out, unchecked one-game contribution action that
never re-enables global sharing. Default-on human training remains conditional
on reviewed anonymity/legal basis; until then it is disabled.

Binding new rule: security > QA > performance > clean code; no first-party unsafe
or assembly. Do not copy the sibling engine's contrary optimization rule.
