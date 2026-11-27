-- AI runner tokens (docs/ai-players.md section 2): the credential an
-- administrator makes for a machine that runs AI players. Only the SHA-256
-- digest is stored. A token names the AI accounts it may play, expires within
-- 366 days and can be revoked; it trades itself for an ordinary native
-- session of one of those accounts, and that session names it.
--
-- The maker and the revoker are account ids without a reference: a token and
-- what it did outlive them. Unlike a report token, a runner token does not
-- end with its maker's role. A token without a maker, and a revocation
-- without a revoker, are the operator's (`central-service --ai-players-stdin`).
CREATE TABLE central_identity.ai_runner_tokens (
    id uuid PRIMARY KEY DEFAULT pg_catalog.uuidv7()
        CHECK (pg_catalog.uuid_extract_version(id) IS NOT DISTINCT FROM 7),
    name text NOT NULL CHECK (octet_length(name) BETWEEN 1 AND 64),
    digest bytea NOT NULL UNIQUE CHECK (octet_length(digest) = 32),
    created_by uuid,
    created_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    expires_at timestamptz NOT NULL,
    revoked_at timestamptz,
    revoked_by uuid,
    last_used_at timestamptz,
    CONSTRAINT ai_runner_token_lifetime CHECK (
        expires_at > created_at AND expires_at <= created_at + interval '366 days'),
    CONSTRAINT ai_runner_token_revoked CHECK (
        (revoked_at IS NULL OR revoked_at >= created_at)
        AND (revoked_by IS NULL OR revoked_at IS NOT NULL))
);
REVOKE ALL ON central_identity.ai_runner_tokens FROM PUBLIC;

-- The AI accounts a token may play, fixed when it is made. An account that
-- is no AI any more leaves every list, and its runner sessions with it.
CREATE TABLE central_identity.ai_runner_accounts (
    token_id uuid NOT NULL REFERENCES central_identity.ai_runner_tokens(id) ON DELETE CASCADE,
    account_id uuid NOT NULL REFERENCES central_identity.ai_players(account_id) ON DELETE CASCADE,
    PRIMARY KEY (token_id, account_id)
);
CREATE INDEX ai_runner_accounts_account ON central_identity.ai_runner_accounts(account_id);
REVOKE ALL ON central_identity.ai_runner_accounts FROM PUBLIC;

-- Each exchange, kept 90 days: the exchange prunes its token's older rows.
CREATE TABLE central_identity.ai_runner_signins (
    id uuid PRIMARY KEY DEFAULT pg_catalog.uuidv7()
        CHECK (pg_catalog.uuid_extract_version(id) IS NOT DISTINCT FROM 7),
    token_id uuid NOT NULL REFERENCES central_identity.ai_runner_tokens(id) ON DELETE CASCADE,
    account_id uuid NOT NULL REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp()
);
CREATE INDEX ai_runner_signins_token ON central_identity.ai_runner_signins(token_id, at);
CREATE INDEX ai_runner_signins_account ON central_identity.ai_runner_signins(account_id, at);
REVOKE ALL ON central_identity.ai_runner_signins FROM PUBLIC;

-- A runner session names the token that minted it and goes with the token's
-- listing of its account. An account keeps one session of each kind that no
-- token minted; a row the runtime writes without the column is such a
-- session, so a binary from before this migration keeps its rule.
ALTER TABLE central_identity.sessions
    ADD COLUMN runner_token uuid,
    ADD CONSTRAINT session_runner_native CHECK (runner_token IS NULL OR kind = 'native'),
    ADD CONSTRAINT session_runner_token FOREIGN KEY (runner_token, account_id)
        REFERENCES central_identity.ai_runner_accounts(token_id, account_id) ON DELETE CASCADE;
CREATE INDEX sessions_runner_token ON central_identity.sessions(runner_token)
    WHERE runner_token IS NOT NULL;
DROP INDEX central_identity.sessions_one_per_account;
CREATE UNIQUE INDEX sessions_one_per_account ON central_identity.sessions(account_id, kind)
    WHERE runner_token IS NULL;

-- The runner token an administrator's change made or revoked, by its id.
-- Like the record's other ids it refers to nothing.
ALTER TABLE central_identity.audit_events ADD COLUMN ai_runner_token_id uuid;
