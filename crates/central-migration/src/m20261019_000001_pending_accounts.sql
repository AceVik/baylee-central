CREATE SCHEMA central_identity;
REVOKE ALL ON SCHEMA central_identity FROM PUBLIC;

CREATE TABLE central_identity.pending_accounts (
    id uuid PRIMARY KEY DEFAULT pg_catalog.uuidv7(),
    created_at timestamptz NOT NULL DEFAULT pg_catalog.now(),
    expires_at timestamptz NOT NULL DEFAULT (pg_catalog.now() + interval '15 minutes'),
    CONSTRAINT pending_account_uuid_v7 CHECK (
        pg_catalog.uuid_extract_version(id) IS NOT DISTINCT FROM 7
    ),
    CONSTRAINT pending_account_expiry CHECK (
        expires_at > created_at AND expires_at <= created_at + interval '15 minutes'
    )
);

CREATE INDEX pending_accounts_expiry ON central_identity.pending_accounts (expires_at);
REVOKE ALL ON TABLE central_identity.pending_accounts FROM PUBLIC;
