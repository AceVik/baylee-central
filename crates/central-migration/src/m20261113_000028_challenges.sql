-- Private packages are immutable. The runtime may withdraw a task, never rewrite it.
CREATE TABLE central_identity.challenges (
    id uuid PRIMARY KEY DEFAULT pg_catalog.uuidv7()
        CHECK (pg_catalog.uuid_extract_version(id) IS NOT DISTINCT FROM 7),
    kind text NOT NULL CHECK (kind IN ('daily','weekly')),
    starts_at bigint NOT NULL CHECK (starts_at >= 0),
    ends_at bigint NOT NULL,
    description jsonb NOT NULL CHECK (jsonb_typeof(description) = 'object'
        AND octet_length(description::text) <= 24000),
    digest bytea NOT NULL CHECK (octet_length(digest) = 32),
    publication bytea NOT NULL CHECK (octet_length(publication) BETWEEN 1 AND 1056768),
    withdrawn boolean NOT NULL DEFAULT false,
    created_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    UNIQUE (kind, starts_at),
    CHECK ((kind = 'daily' AND starts_at % 86400 = 0 AND ends_at = starts_at + 86400)
        OR (kind = 'weekly' AND (starts_at + 259200) % 604800 = 0 AND ends_at = starts_at + 604800))
);
CREATE INDEX challenges_calendar ON central_identity.challenges (starts_at DESC, kind);

-- The account row serializes hints, attempt starts, deletion and result accounting.
CREATE TABLE central_identity.challenge_participation (
    account_id uuid NOT NULL REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    challenge_id uuid NOT NULL REFERENCES central_identity.challenges(id),
    hint_level smallint NOT NULL DEFAULT 0 CHECK (hint_level BETWEEN 0 AND 2),
    listed boolean NOT NULL DEFAULT false,
    starts integer NOT NULL DEFAULT 0 CHECK (starts BETWEEN 0 AND 1000000),
    failures integer NOT NULL DEFAULT 0 CHECK (failures BETWEEN 0 AND 1000000),
    first_win uuid,
    PRIMARY KEY (account_id, challenge_id)
);

CREATE TABLE central_identity.challenge_attempts (
    id uuid PRIMARY KEY DEFAULT pg_catalog.uuidv7()
        CHECK (pg_catalog.uuid_extract_version(id) IS NOT DISTINCT FROM 7),
    account_id uuid NOT NULL REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    challenge_id uuid NOT NULL REFERENCES central_identity.challenges(id),
    kind text NOT NULL CHECK (kind IN ('daily','weekly')),
    request_id uuid NOT NULL
        CHECK (pg_catalog.uuid_extract_version(request_id) IS NOT DISTINCT FROM 7),
    state text NOT NULL DEFAULT 'reserved'
        CHECK (state IN ('reserved','playing','won','lost','abandoned','void')),
    number integer NOT NULL DEFAULT 0 CHECK (number BETWEEN 0 AND 1000000),
    failures integer NOT NULL DEFAULT 0 CHECK (failures BETWEEN 0 AND 1000000),
    game_id uuid UNIQUE
        CHECK (game_id IS NULL OR pg_catalog.uuid_extract_version(game_id) IS NOT DISTINCT FROM 7),
    assisted boolean NOT NULL DEFAULT false,
    live boolean NOT NULL,
    turn integer CHECK (turn BETWEEN 0 AND 10000),
    milliseconds bigint CHECK (milliseconds BETWEEN 1 AND 604800000),
    created_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    started_at timestamptz,
    finished_at timestamptz,
    received_at timestamptz,
    completion jsonb,
    UNIQUE (account_id, request_id),
    CHECK ((state IN ('reserved','playing')) = (finished_at IS NULL)),
    CHECK (state != 'playing' OR (game_id IS NOT NULL AND started_at IS NOT NULL)),
    CHECK (state != 'won' OR (game_id IS NOT NULL AND turn > 0 AND completion IS NOT NULL))
);
-- A player cannot farm parallel openings, including via another gateway or archive.
CREATE UNIQUE INDEX challenge_one_active_kind
    ON central_identity.challenge_attempts (account_id, kind)
    WHERE state IN ('reserved','playing');
CREATE INDEX challenge_attempts_owned
    ON central_identity.challenge_attempts (account_id, challenge_id, created_at DESC);
CREATE INDEX challenge_attempts_leaderboard
    ON central_identity.challenge_attempts (challenge_id, turn, milliseconds)
    WHERE state = 'won' AND live AND NOT assisted;

-- These rows survive withdrawal and failed later attempts; an account deletion cascades.
CREATE TABLE central_identity.challenge_completions (
    account_id uuid NOT NULL REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    challenge_id uuid NOT NULL REFERENCES central_identity.challenges(id),
    kind text NOT NULL CHECK (kind IN ('daily','weekly')),
    earned_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    live_week bigint,
    tactician boolean NOT NULL,
    first_try boolean NOT NULL,
    PRIMARY KEY (account_id, challenge_id)
);
