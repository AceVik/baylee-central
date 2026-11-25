-- Website UX overhaul: sliding browser refresh sessions (3d idle, 30d cap),
-- optional owner-only profile details with default-public showcase opt-out,
-- and soft-deletable registered gateways without losing audit history.

-- 1. Sessions: browser sessions support short-lived access cookies plus 3-day
--    sliding single-use refresh tokens capped at 30 days from sign-in.
--    Native sessions keep their existing 12-hour bound and have no refresh state.
ALTER TABLE central_identity.sessions
    ADD COLUMN refresh_digest bytea CHECK (octet_length(refresh_digest) = 32),
    ADD COLUMN previous_refresh_digest bytea CHECK (octet_length(previous_refresh_digest) = 32),
    ADD COLUMN refresh_expires_at timestamptz,
    ADD COLUMN absolute_expires_at timestamptz;

CREATE UNIQUE INDEX sessions_refresh_digest
    ON central_identity.sessions (refresh_digest)
    WHERE refresh_digest IS NOT NULL;

CREATE INDEX sessions_previous_refresh_digest
    ON central_identity.sessions (previous_refresh_digest)
    WHERE previous_refresh_digest IS NOT NULL;

ALTER TABLE central_identity.pairings DROP CONSTRAINT pairings_session_digest_fkey;
ALTER TABLE central_identity.pairings ADD CONSTRAINT pairings_session_digest_fkey
    FOREIGN KEY (session_digest) REFERENCES central_identity.sessions(digest)
    ON DELETE CASCADE ON UPDATE CASCADE;

ALTER TABLE central_identity.sessions DROP CONSTRAINT session_lifetime;
ALTER TABLE central_identity.sessions ADD CONSTRAINT session_lifetime CHECK (
    expires_at > created_at
    AND (proven_at IS NULL OR proven_at <= created_at)
    AND (
        (kind = 'native'
         AND refresh_digest IS NULL
         AND previous_refresh_digest IS NULL
         AND refresh_expires_at IS NULL
         AND absolute_expires_at IS NULL
         AND expires_at <= COALESCE(proven_at, created_at) + interval '12 hours')
        OR
        (kind = 'browser'
         AND expires_at <= created_at + interval '30 days'
         AND (refresh_expires_at IS NULL OR refresh_expires_at > created_at)
         AND (absolute_expires_at IS NULL OR (
             absolute_expires_at > created_at
             AND absolute_expires_at <= created_at + interval '30 days'
             AND expires_at <= absolute_expires_at
             AND (refresh_expires_at IS NULL OR refresh_expires_at <= absolute_expires_at)
         )))
    )
);

-- 2. Accounts: optional owner-only personal details (never public) and public
--    profile visibility opt-out (enabled by default).
ALTER TABLE central_identity.accounts
    ADD COLUMN first_name text
        CHECK (first_name IS NULL OR (char_length(first_name) BETWEEN 1 AND 80 AND first_name = btrim(first_name))),
    ADD COLUMN last_name text
        CHECK (last_name IS NULL OR (char_length(last_name) BETWEEN 1 AND 80 AND last_name = btrim(last_name))),
    ADD COLUMN birth_date date
        CHECK (birth_date IS NULL OR birth_date >= DATE '1900-01-01'),
    ADD COLUMN public_profile boolean NOT NULL DEFAULT true;

-- 3. Gateways: soft-delete keeps audit foreign keys intact without requiring
--    DELETE privileges in ops/grants.sql, while freeing active origin uniqueness.
ALTER TABLE central_identity.gateways
    ADD COLUMN deleted_at timestamptz;

DO $$
DECLARE
    found text[];
BEGIN
    SELECT array_agg(c.conname) INTO found
    FROM pg_catalog.pg_constraint c
    WHERE c.conrelid = 'central_identity.gateways'::regclass
      AND c.contype = 'u'
      AND pg_catalog.pg_get_constraintdef(c.oid) = 'UNIQUE (origin)';
    IF pg_catalog.array_length(found, 1) IS DISTINCT FROM 1 THEN
        RAISE EXCEPTION 'gateways: expected one UNIQUE (origin) constraint';
    END IF;
    EXECUTE pg_catalog.format('ALTER TABLE central_identity.gateways DROP CONSTRAINT %I', found[1]);
END
$$;

CREATE UNIQUE INDEX gateways_origin_active
    ON central_identity.gateways (origin)
    WHERE deleted_at IS NULL;
