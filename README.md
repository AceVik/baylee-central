# Baylee Central

Public development repository for Baylee's central accounts, encrypted personal
storage, gateway directory, scoped administration, website and training intake.

**Implementation started on 2026-10-19 with owner authorization.** A local-only
HTTP service, reusable OPAQUE/encrypted-account SDK, policy libraries and
PostgreSQL account persistence exist, including a synthetic legacy bridge test
with the sibling Baylee repository.
PostgreSQL 18+, server-generated UUIDv7 and SeaORM 2 migrations are selected.
Native Linux, Windows and macOS are required. Deployed authentication, storage and
production privacy guarantees are not implemented. See [status](docs/status.md)
and [development commands](docs/dev-mode.md).

Priority: **security > QA > performance > clean code**. First-party unsafe Rust
and assembly are forbidden. See [AGENTS.md](AGENTS.md) for binding development rules.

- [Architecture and decisions](docs/architecture.md)
- [Cryptographic integration](docs/crypto-protocol.md)
- [Native login, migration and development account service](docs/native-login.md)
- [PostgreSQL and UUID policy](docs/postgresql.md)
- [SeaORM adapter and migration tests](docs/database-development.md)
- [Threat model](docs/threat-model.md)
- [Privacy and training](docs/privacy-training.md)
- [Implementation plan and acceptance gates](docs/implementation-plan.md)
- [Implementation handoff](docs/handoff.md)
- [Development tasks: `cargo xtask`](docs/xtask.md)
- [Security reporting](SECURITY.md)

Initial operator: one official central deployment; federation is deferred.
Repository visibility: public, authorized by the owner on 2026-10-19.
Licensed under [AGPL-3.0-only](LICENSE), as selected by the owner.
The official live service address is `https://central.baylee.app`;
source publication does not indicate that this service has been deployed.
No Wizards assets, user data, replay corpus, production secrets or credentials
belong in this repository. This is an unofficial, non-commercial project.
