-- AI players (docs/ai-players.md): accounts an administrator marks as an AI,
-- which fill a lonely person's ranked search. Everything lives in tables of
-- its own; of the existing tables only gateway_sids changes, so that one AI
-- session can hold a ranked sid for each of its live matches.
--
-- set_by, changed_by: account ids without a reference, as in the audit; NULL
-- is the operator. The flag goes with the account.
CREATE TABLE central_identity.ai_players (
    account_id uuid PRIMARY KEY REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    kind text NOT NULL CHECK (kind IN ('house', 'model', 'net')),
    profile text NOT NULL CHECK (profile ~ '^[a-z]{1,12}:[a-z0-9_.-]{1,48}$'),
    label text NOT NULL CHECK (pg_catalog.char_length(label) BETWEEN 1 AND 40
        AND label !~ '[[:cntrl:]]' AND label = pg_catalog.btrim(label)),
    enabled boolean NOT NULL DEFAULT false,
    max_games smallint NOT NULL DEFAULT 1 CHECK (max_games BETWEEN 1 AND 16),
    daily_games integer CHECK (daily_games BETWEEN 0 AND 10000),
    set_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    set_by uuid,
    changed_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    changed_by uuid,
    -- a language model plays only within a daily cap
    CONSTRAINT ai_model_capped CHECK (kind <> 'model' OR daily_games IS NOT NULL),
    -- every ranked AI is a fair remote seat; the level that sees hidden
    -- cards never plays Ranked (owner, 26.11.2026)
    CONSTRAINT ai_fair_only CHECK (profile <> 'house:seer')
);
REVOKE ALL ON central_identity.ai_players FROM PUBLIC;

-- The administrators' settings for every AI, one row like registration's.
CREATE TABLE central_identity.ai_settings (
    singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
    fill_in_after smallint NOT NULL DEFAULT 60 CHECK (fill_in_after BETWEEN 30 AND 600),
    drain_minutes smallint NOT NULL DEFAULT 120 CHECK (drain_minutes BETWEEN 30 AND 600),
    global_seats smallint NOT NULL DEFAULT 8 CHECK (global_seats BETWEEN 0 AND 16),
    host_reserve smallint NOT NULL DEFAULT 2 CHECK (host_reserve BETWEEN 0 AND 64),
    carousel_games smallint NOT NULL DEFAULT 0 CHECK (carousel_games BETWEEN 0 AND 8),
    carousel_daily integer NOT NULL DEFAULT 0 CHECK (carousel_daily BETWEEN 0 AND 10000),
    changed_at timestamptz,
    changed_by uuid
);
INSERT INTO central_identity.ai_settings (singleton) VALUES (true);
REVOKE ALL ON central_identity.ai_settings FROM PUBLIC;

-- A runner's standing offer: one frozen deck per format, renewed every few
-- seconds, gone with the session it was made under.
CREATE TABLE central_identity.ai_offers (
    account_id uuid NOT NULL REFERENCES central_identity.ai_players(account_id) ON DELETE CASCADE,
    format text NOT NULL CHECK (format IN ('standard', 'highlander', 'commander')),
    session_digest bytea NOT NULL REFERENCES central_identity.sessions(digest) ON DELETE CASCADE,
    commitment bytea NOT NULL CHECK (octet_length(commitment) = 32),
    renewed_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    expires_at timestamptz NOT NULL,
    PRIMARY KEY (account_id, format),
    CONSTRAINT ai_offer_lifetime CHECK (
        expires_at > renewed_at AND expires_at <= renewed_at + interval '45 seconds')
);
CREATE INDEX ai_offers_format ON central_identity.ai_offers(format, expires_at);
CREATE INDEX ai_offers_session ON central_identity.ai_offers(session_digest);
REVOKE ALL ON central_identity.ai_offers FROM PUBLIC;

-- A person's consent to an AI opponent for one search; no row is "no AI".
CREATE TABLE central_identity.ranked_ticket_ai (
    account_id uuid NOT NULL,
    request_id uuid NOT NULL,
    accepts text NOT NULL CHECK (accepts IN ('local', 'any')),
    PRIMARY KEY (account_id, request_id),
    FOREIGN KEY (account_id, request_id)
        REFERENCES central_identity.ranked_tickets(account_id, request_id) ON DELETE CASCADE
);
REVOKE ALL ON central_identity.ranked_ticket_ai FROM PUBLIC;

-- How a match with an AI seat was made. ai_seats: 1 seat zero, 2 seat one, 3 both.
CREATE TABLE central_identity.ai_matches (
    match_id uuid PRIMARY KEY REFERENCES central_identity.ranked_matches(id) ON DELETE CASCADE,
    arranged text NOT NULL CHECK (arranged IN ('fill_in', 'carousel')),
    ai_seats smallint NOT NULL CHECK (ai_seats BETWEEN 1 AND 3),
    created_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    CONSTRAINT ai_match_seats CHECK ((arranged = 'carousel') = (ai_seats = 3))
);
CREATE INDEX ai_matches_day ON central_identity.ai_matches(created_at);
REVOKE ALL ON central_identity.ai_matches FROM PUBLIC;

-- One ranked sid per session, and one sid per session, gateway and purpose,
-- stay the rule for every row but an AI seat's. An AI session holds one sid
-- per match instead. A row the runtime writes without the column is no AI
-- seat's, so a binary from before this migration keeps its rules.
ALTER TABLE central_identity.gateway_sids
    ADD COLUMN ai_seat boolean NOT NULL DEFAULT false,
    ADD CONSTRAINT gateway_sid_ai_seat CHECK (NOT ai_seat OR purpose = 'ranked');

DO $$
DECLARE
    found text[];
BEGIN
    SELECT array_agg(c.conname) INTO found
    FROM pg_catalog.pg_constraint c
    WHERE c.conrelid = 'central_identity.gateway_sids'::regclass
      AND c.contype = 'u'
      AND pg_catalog.pg_get_constraintdef(c.oid) = 'UNIQUE (session_digest, stream, purpose)';
    IF pg_catalog.array_length(found, 1) IS DISTINCT FROM 1 THEN
        RAISE EXCEPTION 'gateway_sids: expected one UNIQUE (session_digest, stream, purpose) constraint';
    END IF;
    EXECUTE pg_catalog.format('ALTER TABLE central_identity.gateway_sids DROP CONSTRAINT %I', found[1]);
END
$$;
CREATE UNIQUE INDEX gateway_sids_one_each ON central_identity.gateway_sids(session_digest, stream, purpose)
    WHERE NOT ai_seat;
DROP INDEX central_identity.gateway_sids_one_ranked;
CREATE UNIQUE INDEX gateway_sids_one_ranked ON central_identity.gateway_sids(session_digest)
    WHERE purpose = 'ranked' AND NOT ai_seat;
CREATE UNIQUE INDEX gateway_sids_ai_match ON central_identity.gateway_sids(session_digest, ranked_match)
    WHERE ai_seat;
