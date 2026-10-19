ALTER TABLE central_identity.accounts DROP CONSTRAINT account_uuid_v7;
ALTER TABLE central_identity.accounts ADD CONSTRAINT account_uuid_v7
    CHECK (pg_catalog.uuid_extract_version(id) IS NOT DISTINCT FROM 7);
CREATE UNIQUE INDEX accounts_enrollment_digest ON central_identity.accounts(enrollment_digest)
    WHERE enrollment_digest IS NOT NULL;
ALTER TABLE central_identity.gateway_tickets DROP CONSTRAINT gateway_tickets_purpose_check;
ALTER TABLE central_identity.gateway_tickets ADD CONSTRAINT gateway_tickets_purpose_check
    CHECK (purpose IN ('admit', 'export', 'link', 'admin'));

CREATE TABLE central_identity.gateways (
    id uuid PRIMARY KEY DEFAULT pg_catalog.uuidv7()
        CHECK (pg_catalog.uuid_extract_version(id) IS NOT DISTINCT FROM 7),
    owner_id uuid NOT NULL REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    origin text NOT NULL UNIQUE CHECK (octet_length(origin) BETWEEN 1 AND 256),
    name text NOT NULL CHECK (octet_length(name) BETWEEN 1 AND 80),
    version text NOT NULL CHECK (octet_length(version) BETWEEN 1 AND 64),
    website text NOT NULL CHECK (octet_length(website) BETWEEN 1 AND 512),
    credential_digest bytea NOT NULL UNIQUE CHECK (octet_length(credential_digest) = 32),
    heartbeat_digest bytea NOT NULL UNIQUE CHECK (octet_length(heartbeat_digest) = 32),
    admin_revision bigint NOT NULL DEFAULT 1 CHECK (admin_revision > 0),
    last_seen timestamptz,
    created_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    CHECK (credential_digest <> heartbeat_digest)
);
CREATE INDEX gateways_owner ON central_identity.gateways(owner_id);

-- Approval is deliberately outside runtime mutation rights. An operator verifies
-- origin ownership out of band before adding/enabling this row. No URL is fetched.
CREATE TABLE central_identity.gateway_approvals (
    gateway_id uuid PRIMARY KEY REFERENCES central_identity.gateways(id) ON DELETE CASCADE,
    enabled boolean NOT NULL DEFAULT true,
    approved_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp()
);

CREATE TABLE central_identity.gateway_admins (
    gateway_id uuid NOT NULL REFERENCES central_identity.gateways(id) ON DELETE CASCADE,
    account_id uuid NOT NULL REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    PRIMARY KEY (gateway_id, account_id)
);
CREATE INDEX gateway_admins_account ON central_identity.gateway_admins(account_id);

CREATE TABLE central_identity.gateway_leases (
    digest bytea PRIMARY KEY CHECK (octet_length(digest) = 32),
    session_digest bytea NOT NULL REFERENCES central_identity.sessions(digest) ON DELETE CASCADE,
    gateway_id uuid REFERENCES central_identity.gateways(id) ON DELETE CASCADE,
    audience text NOT NULL CHECK (octet_length(audience) BETWEEN 1 AND 256),
    purpose text NOT NULL CHECK (purpose IN ('admit', 'admin')),
    expires_at timestamptz NOT NULL,
    created_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    CHECK (expires_at > created_at AND expires_at <= created_at + interval '30 minutes'),
    CHECK (gateway_id IS NOT NULL OR purpose = 'admit')
);
CREATE INDEX gateway_leases_session ON central_identity.gateway_leases(session_digest);
CREATE INDEX gateway_leases_expiry ON central_identity.gateway_leases(expires_at);
CREATE INDEX gateway_leases_gateway ON central_identity.gateway_leases(gateway_id);
REVOKE ALL ON central_identity.gateways, central_identity.gateway_approvals,
    central_identity.gateway_admins, central_identity.gateway_leases FROM PUBLIC;
