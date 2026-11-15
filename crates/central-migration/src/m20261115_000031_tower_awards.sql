-- Only permanent verified facts. Tower game state remains in the Baylee service.
CREATE TABLE central_identity.tower_award_issuers (
    id uuid PRIMARY KEY DEFAULT pg_catalog.uuidv7()
        CHECK (pg_catalog.uuid_extract_version(id) IS NOT DISTINCT FROM 7),
    origin text NOT NULL UNIQUE CHECK (octet_length(origin) BETWEEN 1 AND 2048),
    credential_digest bytea NOT NULL UNIQUE CHECK (octet_length(credential_digest)=32),
    enabled boolean NOT NULL DEFAULT true,
    created_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    last_used_at timestamptz
);
CREATE TABLE central_identity.tower_award_runs (
    id uuid PRIMARY KEY DEFAULT pg_catalog.uuidv7()
        CHECK (pg_catalog.uuid_extract_version(id) IS NOT DISTINCT FROM 7),
    issuer_id uuid NOT NULL REFERENCES central_identity.tower_award_issuers(id),
    account_id uuid NOT NULL REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    remote_run uuid NOT NULL CHECK (pg_catalog.uuid_extract_version(remote_run) IS NOT DISTINCT FROM 7),
    last_fact bytea NOT NULL CHECK (octet_length(last_fact) BETWEEN 1 AND 1024),
    UNIQUE (issuer_id,remote_run),
    UNIQUE (id,issuer_id,account_id)
);
CREATE INDEX tower_award_runs_owner ON central_identity.tower_award_runs(account_id,id);
CREATE TABLE central_identity.tower_award_receipts (
    id uuid PRIMARY KEY DEFAULT pg_catalog.uuidv7()
        CHECK (pg_catalog.uuid_extract_version(id) IS NOT DISTINCT FROM 7),
    issuer_id uuid NOT NULL REFERENCES central_identity.tower_award_issuers(id),
    account_id uuid NOT NULL REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    run_id uuid NOT NULL,
    remote_event uuid NOT NULL CHECK (pg_catalog.uuid_extract_version(remote_event) IS NOT DISTINCT FROM 7),
    stage smallint NOT NULL CHECK (stage BETWEEN 1 AND 12),
    fact bytea NOT NULL CHECK (octet_length(fact) BETWEEN 1 AND 1024),
    created_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    FOREIGN KEY (run_id,issuer_id,account_id) REFERENCES central_identity.tower_award_runs(id,issuer_id,account_id) ON DELETE CASCADE,
    UNIQUE (issuer_id,remote_event),
    UNIQUE (run_id,stage)
);
CREATE INDEX tower_award_receipts_owner ON central_identity.tower_award_receipts(account_id,id);
CREATE TABLE central_identity.tower_award_progress (
    account_id uuid PRIMARY KEY REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    duels bigint NOT NULL CHECK (duels BETWEEN 0 AND 1000000),
    victories bigint NOT NULL CHECK (victories BETWEEN 0 AND duels),
    clears bigint NOT NULL CHECK (clears BETWEEN 0 AND victories/8),
    flawless bigint NOT NULL CHECK (flawless BETWEEN 0 AND clears)
);
REVOKE ALL ON central_identity.tower_award_issuers, central_identity.tower_award_runs,
    central_identity.tower_award_receipts, central_identity.tower_award_progress FROM PUBLIC;
