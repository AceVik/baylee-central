-- Seasons use half-open periods. Even two concurrent operator writes cannot overlap.
CREATE TABLE central_identity.ranked_seasons (
    id uuid PRIMARY KEY DEFAULT pg_catalog.uuidv7()
        CHECK (pg_catalog.uuid_extract_version(id) IS NOT DISTINCT FROM 7),
    starts_at timestamptz NOT NULL,
    ends_at timestamptz,
    CHECK (ends_at IS NULL OR starts_at < ends_at),
    EXCLUDE USING gist (tstzrange(starts_at, ends_at, '[)') WITH &&)
);
INSERT INTO central_identity.ranked_seasons (starts_at) VALUES (CURRENT_TIMESTAMP);

-- A short-lived offer of total concurrent ranked games, tied to the verification epoch.
CREATE TABLE central_identity.ranked_hosts (
    host_id uuid PRIMARY KEY REFERENCES central_identity.gateways(id) ON DELETE CASCADE,
    ranked_epoch bigint NOT NULL,
    capacity smallint NOT NULL CHECK (capacity BETWEEN 0 AND 64),
    continuity_since timestamptz NOT NULL DEFAULT transaction_timestamp(),
    seen_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp()
);
CREATE INDEX ranked_hosts_recent ON central_identity.ranked_hosts(seen_at DESC) WHERE capacity > 0;

CREATE TABLE central_identity.ranked_matches (
    id uuid PRIMARY KEY DEFAULT pg_catalog.uuidv7()
        CHECK (pg_catalog.uuid_extract_version(id) IS NOT DISTINCT FROM 7),
    season_id uuid NOT NULL REFERENCES central_identity.ranked_seasons(id),
    format text NOT NULL CHECK (format IN ('standard','highlander','commander')),
    account_a uuid NOT NULL REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    account_b uuid NOT NULL REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    CHECK (account_a <> account_b),
    commitment_a bytea NOT NULL CHECK (octet_length(commitment_a) = 32),
    commitment_b bytea NOT NULL CHECK (octet_length(commitment_b) = 32),
    claimed_a boolean NOT NULL DEFAULT false,
    claimed_b boolean NOT NULL DEFAULT false,
    -- Only a verified host supplies these combined pools after both seats have claimed.
    deck_a jsonb CHECK (jsonb_typeof(deck_a) = 'array' AND octet_length(deck_a::text) <= 65536),
    deck_b jsonb CHECK (jsonb_typeof(deck_b) = 'array' AND octet_length(deck_b::text) <= 65536),
    start_fingerprint bytea CHECK (octet_length(start_fingerprint) = 32),
    host_id uuid NOT NULL,
    ranked_epoch bigint NOT NULL,
    origin text NOT NULL CHECK (octet_length(origin) BETWEEN 1 AND 256),
    state text NOT NULL DEFAULT 'reserved' CHECK (state IN ('reserved','playing','finished','void')),
    game_id uuid UNIQUE CHECK (game_id IS NULL OR pg_catalog.uuid_extract_version(game_id) IS NOT DISTINCT FROM 7),
    outcome smallint CHECK (outcome BETWEEN 0 AND 2),
    created_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    lease_until timestamptz NOT NULL DEFAULT (pg_catalog.clock_timestamp() + interval '2 minutes'),
    finished_at timestamptz,
    CHECK ((state IN ('finished','void')) = (finished_at IS NOT NULL)),
    CHECK ((state = 'finished') = (outcome IS NOT NULL)),
    CHECK ((state='playing') = (deck_a IS NOT NULL) AND (state='playing') = (deck_b IS NOT NULL)),
    CHECK (state NOT IN ('playing','finished') OR
        (claimed_a AND claimed_b AND game_id IS NOT NULL AND start_fingerprint IS NOT NULL))
);
CREATE INDEX ranked_matches_host_load ON central_identity.ranked_matches(host_id,lease_until)
    WHERE state IN ('reserved','playing');
CREATE INDEX ranked_matches_expires ON central_identity.ranked_matches(lease_until)
    WHERE state IN ('reserved','playing');
CREATE INDEX ranked_matches_repeat ON central_identity.ranked_matches(
    LEAST(account_a,account_b), GREATEST(account_a,account_b), finished_at DESC) WHERE state='finished';
CREATE INDEX ranked_matches_account_b ON central_identity.ranked_matches(account_b);
CREATE INDEX ranked_matches_account_a ON central_identity.ranked_matches(account_a);

-- Exactly one queued or assigned seat across all formats. Completed request keys remain
-- unique so a response lost on the network cannot silently enqueue the player twice.
CREATE TABLE central_identity.ranked_tickets (
    account_id uuid NOT NULL REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    request_id uuid NOT NULL CHECK (pg_catalog.uuid_extract_version(request_id) IS NOT DISTINCT FROM 7),
    season_id uuid NOT NULL REFERENCES central_identity.ranked_seasons(id),
    format text NOT NULL CHECK (format IN ('standard','highlander','commander')),
    commitment bytea NOT NULL CHECK (octet_length(commitment) = 32),
    queue_elo integer NOT NULL,
    created_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    expires_at timestamptz NOT NULL DEFAULT (pg_catalog.clock_timestamp() + interval '4 minutes'),
    state text NOT NULL DEFAULT 'waiting' CHECK (state IN ('waiting','assigned','closed')),
    match_id uuid REFERENCES central_identity.ranked_matches(id) ON DELETE SET NULL,
    PRIMARY KEY (account_id,request_id)
);
CREATE UNIQUE INDEX ranked_single_ticket ON central_identity.ranked_tickets(account_id) WHERE state <> 'closed';
CREATE INDEX ranked_tickets_owned ON central_identity.ranked_tickets(account_id,created_at DESC,request_id DESC);
CREATE INDEX ranked_tickets_wait ON central_identity.ranked_tickets(season_id,format,created_at,account_id) WHERE state='waiting';
CREATE INDEX ranked_tickets_arrivals ON central_identity.ranked_tickets(season_id,format,created_at DESC);
CREATE INDEX ranked_tickets_match ON central_identity.ranked_tickets(match_id) WHERE match_id IS NOT NULL;
CREATE INDEX ranked_tickets_skill ON central_identity.ranked_tickets(season_id,format,queue_elo) WHERE state='waiting';
CREATE INDEX ranked_tickets_expires ON central_identity.ranked_tickets(expires_at) WHERE state='waiting';
