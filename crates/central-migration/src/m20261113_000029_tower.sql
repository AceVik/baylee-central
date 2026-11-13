-- Calendar tasks retain scope zero; private tower nodes never enter their catalogue.
ALTER TABLE central_identity.challenges ADD COLUMN scope smallint NOT NULL DEFAULT 0
    CHECK (scope BETWEEN 0 AND 35 AND (scope = 0 OR kind = 'weekly'));
ALTER TABLE central_identity.challenges DROP CONSTRAINT challenges_kind_starts_at_key;
ALTER TABLE central_identity.challenges ADD UNIQUE (kind, starts_at, scope);
ALTER TABLE central_identity.challenge_attempts DROP CONSTRAINT challenge_attempts_kind_check;
ALTER TABLE central_identity.challenge_attempts ADD CHECK (kind IN ('daily','weekly','tower'));

CREATE TABLE central_identity.tower_seasons (
    id uuid PRIMARY KEY DEFAULT pg_catalog.uuidv7()
        CHECK (pg_catalog.uuid_extract_version(id) IS NOT DISTINCT FROM 7),
    starts_at bigint NOT NULL UNIQUE CHECK (starts_at >= 0 AND (starts_at + 259200) % 604800 = 0),
    ends_at bigint NOT NULL CHECK (ends_at = starts_at + 604800),
    manifest jsonb NOT NULL CHECK (jsonb_typeof(manifest) = 'object'
        AND octet_length(manifest::text) <= 50000),
    published boolean NOT NULL DEFAULT false,
    withdrawn boolean NOT NULL DEFAULT false
);
CREATE TABLE central_identity.tower_nodes (
    season_id uuid NOT NULL REFERENCES central_identity.tower_seasons(id),
    node smallint NOT NULL CHECK (node BETWEEN 0 AND 34),
    challenge_id uuid NOT NULL UNIQUE REFERENCES central_identity.challenges(id),
    PRIMARY KEY (season_id, node)
);
CREATE TABLE central_identity.tower_participation (
    account_id uuid NOT NULL REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    season_id uuid NOT NULL REFERENCES central_identity.tower_seasons(id),
    listed boolean NOT NULL DEFAULT false,
    PRIMARY KEY (account_id, season_id)
);
CREATE TABLE central_identity.tower_runs (
    id uuid PRIMARY KEY DEFAULT pg_catalog.uuidv7()
        CHECK (pg_catalog.uuid_extract_version(id) IS NOT DISTINCT FROM 7),
    account_id uuid NOT NULL REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    season_id uuid NOT NULL REFERENCES central_identity.tower_seasons(id),
    request_id uuid NOT NULL CHECK (pg_catalog.uuid_extract_version(request_id) IS NOT DISTINCT FROM 7),
    revision integer NOT NULL DEFAULT 0 CHECK (revision BETWEEN 0 AND 1000000),
    progress jsonb NOT NULL CHECK (jsonb_typeof(progress) = 'object'
        AND octet_length(progress::text) <= 2000),
    active boolean NOT NULL DEFAULT true,
    active_attempt uuid UNIQUE REFERENCES central_identity.challenge_attempts(id) ON DELETE SET NULL,
    created_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    UNIQUE (account_id, request_id)
);
CREATE UNIQUE INDEX tower_one_active ON central_identity.tower_runs(account_id) WHERE active;
CREATE INDEX tower_owned ON central_identity.tower_runs(account_id, season_id, id DESC);
-- Exact retries are accepted; reusing a key for different arguments conflicts.
CREATE TABLE central_identity.tower_actions (
    account_id uuid NOT NULL REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    request_id uuid NOT NULL CHECK (pg_catalog.uuid_extract_version(request_id) IS NOT DISTINCT FROM 7),
    run_id uuid NOT NULL REFERENCES central_identity.tower_runs(id) ON DELETE CASCADE,
    request jsonb NOT NULL CHECK (octet_length(request::text) <= 1000),
    created_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    PRIMARY KEY (account_id, request_id)
);
-- Last qualifying prefix of a run remains frozen after the week's boundary.
CREATE TABLE central_identity.tower_scores (
    run_id uuid PRIMARY KEY REFERENCES central_identity.tower_runs(id) ON DELETE CASCADE,
    account_id uuid NOT NULL REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    season_id uuid NOT NULL REFERENCES central_identity.tower_seasons(id),
    floors smallint NOT NULL CHECK (floors BETWEEN 1 AND 5),
    losses smallint NOT NULL CHECK (losses BETWEEN 0 AND 2),
    turns integer NOT NULL CHECK (turns BETWEEN 1 AND 50000),
    milliseconds bigint CHECK (milliseconds BETWEEN 1 AND 3024000000)
);
CREATE INDEX tower_scores_rank ON central_identity.tower_scores(season_id, floors DESC, losses, turns);
CREATE TABLE central_identity.tower_accomplishments (
    account_id uuid NOT NULL REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    season_id uuid NOT NULL REFERENCES central_identity.tower_seasons(id),
    floor smallint NOT NULL CHECK (floor BETWEEN 1 AND 5),
    flawless boolean NOT NULL,
    PRIMARY KEY (account_id, season_id, floor)
);
