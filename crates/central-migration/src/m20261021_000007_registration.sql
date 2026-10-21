-- Registration on baylee.app and browser sessions (docs/web-accounts.md).
-- An account is legacy (reserved by the bridge for a player of the official
-- gateway) or registered (with a beta key). Legacy rules apply to legacy
-- accounts only. `source` and `kind` keep their defaults so that the previous
-- binary, restarted by the deploy hook when a new one fails to start, still
-- serves legacy migration; current code names both in every write.

-- Beta keys: 80 random bits each, of which only the SHA-256 digest is kept.
-- `uses_left` counts uses not yet spent by an enrollment; a live reservation
-- (an account's `beta_key_id`) holds one of them until its ticket expires.
CREATE TABLE central_identity.beta_keys (
    id uuid PRIMARY KEY DEFAULT pg_catalog.uuidv7()
        CHECK (pg_catalog.uuid_extract_version(id) IS NOT DISTINCT FROM 7),
    digest bytea NOT NULL UNIQUE CHECK (octet_length(digest) = 32),
    uses_left integer NOT NULL CHECK (uses_left >= 0),
    expires_at timestamptz,
    note text NOT NULL CHECK (length(note) <= 200),
    created_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    revoked_at timestamptz,
    CONSTRAINT beta_key_expiry CHECK (expires_at IS NULL OR expires_at > created_at)
);
REVOKE ALL ON central_identity.beta_keys FROM PUBLIC;

ALTER TABLE central_identity.accounts ALTER COLUMN legacy_id DROP NOT NULL;
ALTER TABLE central_identity.accounts ADD COLUMN source text NOT NULL DEFAULT 'legacy'
    CHECK (source IN ('legacy', 'registered'));
ALTER TABLE central_identity.accounts ADD CONSTRAINT account_source_legacy_link
    CHECK ((source = 'legacy') = (legacy_id IS NOT NULL));
ALTER TABLE central_identity.accounts ADD CONSTRAINT account_closed_legacy
    CHECK (NOT legacy_closed OR source = 'legacy');

-- The master key wrapped under the recovery code: absent until enrollment,
-- present on every enrolled registered account, never on a legacy one.
ALTER TABLE central_identity.accounts ADD COLUMN recovery_wrapped_master bytea
    CHECK (octet_length(recovery_wrapped_master) = 73);
ALTER TABLE central_identity.accounts ADD CONSTRAINT account_recovery CHECK (
    CASE
        WHEN credential IS NULL THEN recovery_wrapped_master IS NULL
        WHEN source = 'registered' THEN recovery_wrapped_master IS NOT NULL
        ELSE recovery_wrapped_master IS NULL
    END
);

-- A pending registered account holds its ticket and one use of its key; the
-- link is cleared in the enrollment commit.
ALTER TABLE central_identity.accounts ADD COLUMN beta_key_id uuid
    REFERENCES central_identity.beta_keys(id);
ALTER TABLE central_identity.accounts ADD CONSTRAINT account_beta_key_pending
    CHECK (beta_key_id IS NULL OR (source = 'registered' AND credential IS NULL));
ALTER TABLE central_identity.accounts ADD CONSTRAINT account_registered_pending
    CHECK (source = 'legacy' OR credential IS NOT NULL
        OR (enrollment_digest IS NOT NULL AND beta_key_id IS NOT NULL));
CREATE INDEX accounts_beta_key ON central_identity.accounts(beta_key_id)
    WHERE beta_key_id IS NOT NULL;
CREATE INDEX accounts_registered_pending ON central_identity.accounts(enrollment_expires)
    WHERE source = 'registered' AND credential IS NULL;

-- One session per account and kind: a sign-in on the website never ends the
-- client's session, nor the other way round.
ALTER TABLE central_identity.sessions ADD COLUMN kind text NOT NULL DEFAULT 'native'
    CHECK (kind IN ('native', 'browser'));
DROP INDEX central_identity.sessions_one_per_account;
CREATE UNIQUE INDEX sessions_one_per_account ON central_identity.sessions(account_id, kind);
