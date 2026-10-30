-- The legacy store (docs/native-login.md §"Legacy login"): the official
-- gateway's accounts that had not moved when Central took their sign-in over,
-- each until its move closes or the deadline passes. Central verifies their
-- old passwords here, the one place it ever sees a password, and hands their
-- source snapshot to the client that moves them.
--
-- A schema of its own, closed to PUBLIC: the runtime role gets nothing in it,
-- and the legacy role (`central_legacy`, ops/grants.sql) nothing outside it.
-- Rows arrive once, at the cutover, under the migrator login
-- (`--legacy-import-stdin`), and are deleted, never marked: by the move's
-- close, or by the sweep. A row holds only what the move needs:
--
-- * `legacy_id`, `username_key`, `display_name`, `tag`: who it is, as the
--   gateway held it; the reservation keeps the name and the tag;
-- * `password_salt`, `password_hash`: Argon2id, version 0x13, m = 19456 KiB,
--   t = 2, p = 1, 32-byte output: the gateway's parameters, the only ones
--   Central verifies (central-crypto's `legacy`), so a stored row cannot make
--   one sign-in cost more;
-- * `terms_version`, `must_change_password`, `banned`, `banned_until`: the
--   gateway's standing for the account; a forced change or a ban in force
--   refuses the move;
-- * `snapshot`, `snapshot_digest`: the source as the gateway exported it,
--   bounded as the first snapshot format is, and its SHA-256;
-- * `failures`, `locked_until`: the lockout's state, the only columns the
--   legacy role may change.
--
-- `settings` holds the deadline after which no legacy sign-in is taken and the
-- sweep deletes what is left: the end of 30.06.2027 in Europe/Berlin, the
-- owner's date, until the operator moves or clears it.

CREATE SCHEMA central_legacy;
REVOKE ALL ON SCHEMA central_legacy FROM PUBLIC;

CREATE TABLE central_legacy.accounts (
    legacy_id text PRIMARY KEY CHECK (legacy_id ~ '^[A-Za-z0-9-]{1,64}$'),
    username_key text NOT NULL UNIQUE
        CHECK (username_key ~ '^[a-z0-9][a-z0-9_.-]{1,22}[a-z0-9]$'),
    display_name text CHECK (octet_length(display_name) BETWEEN 1 AND 64),
    tag integer CHECK (tag > 0),
    password_salt bytea NOT NULL CHECK (octet_length(password_salt) = 16),
    password_hash bytea NOT NULL CHECK (octet_length(password_hash) = 32),
    terms_version text CHECK (octet_length(terms_version) BETWEEN 1 AND 64),
    must_change_password boolean NOT NULL,
    banned boolean NOT NULL,
    banned_until timestamptz,
    snapshot bytea NOT NULL CHECK (octet_length(snapshot) BETWEEN 2 AND 196608),
    snapshot_digest bytea NOT NULL,
    failures integer NOT NULL DEFAULT 0 CHECK (failures BETWEEN 0 AND 9),
    locked_until timestamptz,
    imported_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    CONSTRAINT legacy_ban CHECK (banned OR banned_until IS NULL),
    CONSTRAINT legacy_snapshot_digest CHECK (snapshot_digest = pg_catalog.sha256(snapshot))
);

CREATE TABLE central_legacy.settings (
    singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
    deadline timestamptz
);
INSERT INTO central_legacy.settings (singleton, deadline)
VALUES (true, TIMESTAMP '2027-07-01 00:00:00' AT TIME ZONE 'Europe/Berlin');

REVOKE ALL ON central_legacy.accounts, central_legacy.settings FROM PUBLIC;
