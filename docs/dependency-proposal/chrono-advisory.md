# RUSTSEC-2026-0543 in `chrono`: a bounded exception

*Owner's decision, 18.11.2026.* `cargo audit --deny warnings` failed on
RUSTSEC-2026-0543 (`chrono` 0.4.45 is unmaintained). Until that day
`chrono` was built into the service: `sea-orm-migration` 2.0.4, the newest,
takes SeaORM with all its default features, `with-chrono` among them.
`central-migration` now runs its migrations itself and `central-postgres`
takes SeaORM's `with-time` ([PostgreSQL](../postgresql.md#the-migration-runner-18112026)),
so `cargo tree --workspace --all-features --target all --edges all` names
neither `chrono` nor anything that carries it. `cargo deny check`, which
reads the resolved graph, passes without an ignore.

`chrono` stays in `Cargo.lock` because SeaORM 2.0.4 names `sea-orm-arrow`
as an optional dependency (its `with-arrow` feature), and Cargo writes every
optional dependency into the lock file whether a build turns it on or not.
No newer SeaORM exists that drops it (checked on 18.11.2026). The owner
chose a bounded exception over a red dependency job.

## The bound

- `.cargo/audit.toml` ignores RUSTSEC-2026-0543 and nothing more for it.
- `cargo xtask chrono-scope`, in the gate and in CI's dependency job before
  `cargo audit`, fails when the full graph above names `chrono`,
  `sea-orm-arrow` or `sea-orm-migration`, or when `Cargo.lock` holds any
  version of `chrono` other than 0.4.45 (another version, or none, means the
  exception has to be looked at again or removed).
- `cargo deny check` needs no ignore.

## Removal

When SeaORM drops `sea-orm-arrow` or `sea-orm-arrow` drops `chrono`, so
that the lock file no longer holds it, remove the ignore from
`.cargo/audit.toml` and remove `chrono-scope` from the gate, CI and
`crates/xtask`. Should a build ever need `chrono` again, the exception ends:
the advisory then has to be answered by another crate, not by an ignore.
