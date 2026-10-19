CREATE TABLE central_identity.accounts (
    id uuid PRIMARY KEY DEFAULT pg_catalog.uuidv7(),
    legacy_id text NOT NULL UNIQUE CHECK (length(legacy_id) BETWEEN 1 AND 64),
    username_key text NOT NULL UNIQUE CHECK (username_key ~ '^[a-z0-9][a-z0-9_.-]{1,22}[a-z0-9]$'),
    vault_epoch bigint NOT NULL DEFAULT 1 CHECK (vault_epoch > 0),
    credential_revision bigint NOT NULL DEFAULT 1 CHECK (credential_revision > 0),
    credential bytea CHECK (octet_length(credential) BETWEEN 1 AND 1024),
    wrapped_master bytea CHECK (octet_length(wrapped_master) = 73),
    document bytea CHECK (octet_length(document) BETWEEN 41 AND 262185),
    document_revision bigint NOT NULL DEFAULT 0 CHECK (document_revision >= 0),
    restored boolean NOT NULL DEFAULT false,
    enrollment_digest bytea CHECK (octet_length(enrollment_digest) = 32),
    enrollment_expires timestamptz,
    created_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    CONSTRAINT account_uuid_v7 CHECK (pg_catalog.uuid_extract_version(id) = 7),
    CONSTRAINT account_enrollment_ticket CHECK ((enrollment_digest IS NULL) = (enrollment_expires IS NULL)),
    CONSTRAINT account_enrollment_atomic CHECK (
        (credential IS NULL AND wrapped_master IS NULL AND document IS NULL AND document_revision = 0 AND NOT restored)
        OR (credential IS NOT NULL AND wrapped_master IS NOT NULL AND document IS NOT NULL AND document_revision > 0
            AND enrollment_digest IS NULL AND enrollment_expires IS NULL)
    )
);

CREATE TABLE central_identity.sessions (
    digest bytea PRIMARY KEY CHECK (octet_length(digest) = 32),
    account_id uuid NOT NULL REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    credential_revision bigint NOT NULL CHECK (credential_revision > 0),
    expires_at timestamptz NOT NULL,
    created_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    CONSTRAINT session_lifetime CHECK (expires_at > created_at AND expires_at <= created_at + interval '30 minutes')
);
CREATE INDEX sessions_account ON central_identity.sessions(account_id);
CREATE INDEX sessions_expiry ON central_identity.sessions(expires_at);

CREATE TABLE central_identity.gateway_tickets (
    digest bytea PRIMARY KEY CHECK (octet_length(digest) = 32),
    session_digest bytea NOT NULL REFERENCES central_identity.sessions(digest) ON DELETE CASCADE,
    audience text NOT NULL CHECK (length(audience) BETWEEN 1 AND 256),
    expires_at timestamptz NOT NULL,
    created_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    CONSTRAINT gateway_ticket_lifetime CHECK (expires_at > created_at AND expires_at <= created_at + interval '30 seconds')
);
CREATE INDEX gateway_tickets_expiry ON central_identity.gateway_tickets(expires_at);
CREATE INDEX gateway_tickets_session ON central_identity.gateway_tickets(session_digest);

REVOKE ALL ON central_identity.accounts, central_identity.sessions, central_identity.gateway_tickets FROM PUBLIC;
