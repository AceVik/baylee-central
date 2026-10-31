# Baylee Central

Baylee's central accounts, encrypted personal storage, gateway directory,
scoped administration, website and training intake. The live service runs at
`https://central.baylee.app`; the website will be `https://baylee.app`.

- Game, client and gateways: [AceVik/baylee](https://github.com/AceVik/baylee)
- Community: [Discord](https://discord.gg/4pP3uAwgY)

Implementation started on 2026-10-19 with the owner's authorization. The
service, the account SDK (`central-client`), the policy libraries and the
PostgreSQL persistence exist and are deployed (milestone M0 of the
[roadmap](docs/roadmap-accounts-web.md)); registration, password reset and
the website's account area are still to come. See [status](docs/status.md)
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

**Licence.** The source is public so that anyone can read exactly how Baylee
Central handles their data. It is *source-available*, not open source: under
the [Baylee Central Source-Available License](LICENSE.md) you may read it,
build and run it unmodified to check its claims or to test software against
the official service, and propose changes; copying, changing, distributing or
operating it needs the owner's written permission. `crates/central-api`,
`crates/central-client` and `crates/central-crypto`, which the Baylee client
links, stay under the [GNU AGPL v3.0 only](LICENSE-AGPL). Versions published
before the licence change remain available under the AGPL.

No Wizards assets, user data, replay corpus, production secrets or credentials
belong in this repository. This is an unofficial, non-commercial project.
