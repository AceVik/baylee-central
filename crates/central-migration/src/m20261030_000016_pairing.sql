-- Pairing a signed-in website's master key to Baylee's client (Baylee
-- docs/proposals/deep-links.md §"Protocol specification", owner's decision of
-- 30.10.2026; docs/web-accounts.md §"Pairing").

-- The key check (docs/crypto-protocol.md §"Derived from the master key"): one
-- way from the master key, so a page proves it holds the key without Central
-- learning it. Set at registration, by a new vault, or once within two
-- minutes of a password proof while none is set.
ALTER TABLE central_identity.accounts
    ADD COLUMN key_check bytea CHECK (octet_length(key_check) = 32);

-- The password proof a session rests on, when it is not the session's own
-- creation: a pairing's session takes the browser session's. The 12-hour cap
-- counts from it. NULL is the creation itself, so every existing insert means
-- what it meant.
ALTER TABLE central_identity.sessions ADD COLUMN proven_at timestamptz;
ALTER TABLE central_identity.sessions DROP CONSTRAINT session_lifetime;
ALTER TABLE central_identity.sessions ADD CONSTRAINT session_lifetime
    CHECK (expires_at > created_at
        AND (proven_at IS NULL OR proven_at <= created_at)
        AND expires_at <= COALESCE(proven_at, created_at) + interval '12 hours');

-- One pairing: started by a browser session, claimed by a client within two
-- minutes, approved with the sealed master key, redeemed once (the row is then
-- deleted), all within five minutes. Only the handle's digest is kept; the
-- page's one-time secret never reaches Central.
CREATE TABLE central_identity.pairings (
    digest bytea PRIMARY KEY CHECK (octet_length(digest) = 32),
    account_id uuid NOT NULL REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    session_digest bytea NOT NULL
        REFERENCES central_identity.sessions(digest) ON DELETE CASCADE,
    epoch bigint NOT NULL CHECK (epoch > 0),
    proven_at timestamptz NOT NULL,
    created_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    expires_at timestamptz NOT NULL,
    public_key bytea CHECK (octet_length(public_key) = 32),
    challenge bytea CHECK (octet_length(challenge) = 32),
    mac bytea CHECK (octet_length(mac) = 32),
    sealed bytea CHECK (octet_length(sealed) = 80),
    polls integer NOT NULL DEFAULT 0 CHECK (polls BETWEEN 0 AND 300),
    CONSTRAINT pairing_lifetime
        CHECK (expires_at > created_at AND expires_at <= created_at + interval '5 minutes'),
    CONSTRAINT pairing_claim
        CHECK ((public_key IS NULL) = (challenge IS NULL) AND (challenge IS NULL) = (mac IS NULL)),
    CONSTRAINT pairing_sealed CHECK (sealed IS NULL OR public_key IS NOT NULL),
    CONSTRAINT pairing_proof CHECK (proven_at <= created_at)
);
CREATE INDEX pairings_account ON central_identity.pairings(account_id);
CREATE INDEX pairings_expiry ON central_identity.pairings(expires_at);
