-- Closing migration retires the legacy bridge's ability to invalidate enrollment.
-- Preserve only the newest existing login when upgrading from multi-session builds.
DELETE FROM central_identity.sessions s USING central_identity.sessions newer
    WHERE s.account_id = newer.account_id
      AND (s.created_at, s.digest) < (newer.created_at, newer.digest);
CREATE UNIQUE INDEX sessions_one_per_account ON central_identity.sessions(account_id);
ALTER TABLE central_identity.accounts ADD COLUMN legacy_closed boolean NOT NULL DEFAULT false;
ALTER TABLE central_identity.accounts ADD COLUMN object_bytes bigint NOT NULL DEFAULT 0
    CHECK (object_bytes BETWEEN 0 AND 536870912);
ALTER TABLE central_identity.accounts ADD CONSTRAINT account_closed_enrolled
    CHECK (NOT legacy_closed OR (credential IS NOT NULL AND restored));

-- Interrupted pre-cutover enrollment is never silently destroyed on a legacy
-- password change. The encrypted prior copy stays available to an operator-led
-- recovery process; neither the bridge nor the server receives a decryption key.
CREATE TABLE central_identity.migration_archives (
    id uuid PRIMARY KEY DEFAULT pg_catalog.uuidv7()
        CHECK (pg_catalog.uuid_extract_version(id) IS NOT DISTINCT FROM 7),
    account_id uuid NOT NULL REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    vault_epoch bigint NOT NULL CHECK (vault_epoch > 0),
    credential bytea NOT NULL CHECK (octet_length(credential) BETWEEN 1 AND 1024),
    wrapped_master bytea NOT NULL CHECK (octet_length(wrapped_master) = 73),
    document bytea NOT NULL CHECK (octet_length(document) BETWEEN 41 AND 262185),
    document_revision bigint NOT NULL CHECK (document_revision > 0),
    created_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    UNIQUE (account_id, vault_epoch)
);

CREATE TABLE central_identity.private_objects (
    id uuid PRIMARY KEY DEFAULT pg_catalog.uuidv7()
        CHECK (pg_catalog.uuid_extract_version(id) IS NOT DISTINCT FROM 7),
    account_id uuid NOT NULL REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    revision bigint NOT NULL DEFAULT 0 CHECK (revision >= 0),
    document bytea CHECK (octet_length(document) BETWEEN 41 AND 262185),
    deleted boolean NOT NULL DEFAULT false,
    CHECK ((revision = 0 AND document IS NULL AND NOT deleted)
        OR (revision > 0 AND ((document IS NOT NULL AND NOT deleted) OR (document IS NULL AND deleted))))
);
CREATE INDEX private_objects_account ON central_identity.private_objects(account_id, id);
REVOKE ALL ON central_identity.migration_archives, central_identity.private_objects FROM PUBLIC;
