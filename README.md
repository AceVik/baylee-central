# Baylee Central

Private preparation repository for Baylee's central accounts, encrypted personal
storage, gateway directory, scoped administration, website and training intake.

**Implementation started on 2026-10-19 with owner authorization.** A local-only
HTTP foundation, reusable policy library and isolated cryptography spike exist.
PostgreSQL 18+, server-generated UUIDv7 and SeaORM 2 migrations are selected.
Native Linux, Windows and macOS are required. Deployed authentication, storage and
production privacy guarantees are not implemented. See [status](docs/status.md)
and [development commands](docs/dev-mode.md).

Priority: **security > QA > performance > clean code**. First-party unsafe Rust
and assembly are forbidden. See [AGENTS.md](AGENTS.md) for binding development rules.

- [Architecture and decisions](docs/architecture.md)
- [Cryptographic integration](docs/crypto-protocol.md)
- [PostgreSQL and UUID policy](docs/postgresql.md)
- [SeaORM adapter and migration tests](docs/database-development.md)
- [Threat model](docs/threat-model.md)
- [Privacy and training](docs/privacy-training.md)
- [Implementation plan and acceptance gates](docs/implementation-plan.md)
- [Implementation handoff](docs/handoff.md)
- [Security reporting](SECURITY.md)

Initial operator: one official central deployment; federation is deferred.
Repository visibility: private, explicitly selected by the owner. Publication
is a separate decision. AGPL-3.0-only is proposed to match Baylee, but license
selection must be confirmed before publishing or distributing this new project.
No Wizards assets, user data, replay corpus, production secrets or credentials
belong in this repository. This is an unofficial, non-commercial project.
