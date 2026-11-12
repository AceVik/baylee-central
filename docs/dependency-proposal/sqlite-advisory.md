# RUSTSEC-2026-0459 in `libsqlite3-sys`: a bounded exception

*Owner's decision, 12.11.2026.* `cargo audit --deny warnings` fails on
RUSTSEC-2026-0459 (`libsqlite3-sys` 0.37.0, fixed in 0.39.0). The package
is in `Cargo.lock` because `sqlx` 0.9.0 names `sqlx-sqlite` as an optional
dependency, through SeaORM 2.0.4 (the storage graph the owner accepted,
`docs/storage-proposal`). No crate of this workspace turns SQLite on:
`cargo tree --workspace --all-features --target all --edges all` does not
name `sqlx-sqlite` or `libsqlite3-sys`, so neither is compiled into
anything this repository builds or ships.

The lock file cannot take the fixed version: `sqlx-sqlite` 0.9.0, the
newest published, as are `sqlx` 0.9.0 and `sea-orm` 2.0.4, requires
`libsqlite3-sys >=0.30.1, <0.38.0` (checked on 12.11.2026). The owner chose
a bounded exception over a red dependency job until `sqlx` moves.

## The bound

- `.cargo/audit.toml` ignores RUSTSEC-2026-0459 and nothing more for it.
- `cargo xtask sqlite-scope`, in the gate and in CI's dependency job before
  `cargo audit`, fails when the full graph above names `libsqlite3-sys` or
  `sqlx-sqlite`, or when `Cargo.lock` holds any version of
  `libsqlite3-sys` other than 0.37.0 (another version, or none, means the
  exception has to be looked at again or removed).
- `cargo deny check` needs no ignore: it reads the resolved graph, which
  does not hold the package.

## Removal

When `sqlx` allows `libsqlite3-sys` 0.39 or later, or drops it from the
lock file, raise the lock (with the owner's approval of the lock change),
remove the ignore from `.cargo/audit.toml`, and remove `sqlite-scope` from
the gate, CI and `crates/xtask`.
