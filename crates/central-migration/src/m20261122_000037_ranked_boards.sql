-- Independent public identifiers. Publishing a card-pool hash would allow
-- other players to identify a known list (or confirm a guessed sideboard).
ALTER TABLE central_identity.ranked_deck_versions ADD COLUMN public_id bytea
    NOT NULL UNIQUE DEFAULT pg_catalog.sha256(pg_catalog.uuid_send(pg_catalog.uuidv7()))
    CHECK (octet_length(public_id) = 32);

-- A day's two podiums use one shared allocation of 100 people. The same
-- person may have one qualified player row and one qualified deck row.
CREATE TABLE central_identity.ranked_publications (
    season_id uuid PRIMARY KEY REFERENCES central_identity.ranked_seasons(id),
    published_on date NOT NULL
);
CREATE TABLE central_identity.ranked_champions (
    season_id uuid NOT NULL REFERENCES central_identity.ranked_seasons(id),
    account_id uuid NOT NULL REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    player_format text CHECK (player_format IN ('standard','highlander','commander')),
    deck_format text CHECK (deck_format IN ('standard','highlander','commander')),
    version bytea CHECK (octet_length(version) = 32),
    place smallint NOT NULL CHECK (place BETWEEN 1 AND 100),
    CHECK ((deck_format IS NULL) = (version IS NULL)),
    CHECK (player_format IS NOT NULL OR deck_format IS NOT NULL),
    PRIMARY KEY (season_id,account_id),
    UNIQUE (season_id,place)
);
CREATE INDEX ranked_deck_contenders ON central_identity.ranked_deck_ratings
    (season_id,account_id,elo DESC) WHERE elo >= 2490 AND games >= 3000;

-- The player keeps every earned division and each distinct podium design,
-- including badges reached with a previously played deck. A promotion is
-- recorded in the transaction that awards it. Removing an account erases
-- its entire ranked collection.
CREATE TABLE central_identity.ranked_badge_peaks (
    account_id uuid NOT NULL REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    season_id uuid NOT NULL REFERENCES central_identity.ranked_seasons(id),
    track text NOT NULL CHECK (track IN ('player','deck')),
    format text NOT NULL CHECK (format IN ('standard','highlander','commander')),
    league smallint NOT NULL CHECK (league BETWEEN 0 AND 10),
    division smallint NOT NULL CHECK (division BETWEEN 0 AND 4),
    place smallint CHECK (place BETWEEN 1 AND 100),
    podium smallint GENERATED ALWAYS AS (CASE WHEN place <= 3 THEN place ELSE 0 END) STORED,
    CHECK ((league = 10) = (place IS NOT NULL)),
    PRIMARY KEY (account_id,season_id,track,format,league,division,podium)
);
CREATE FUNCTION central_identity.ranked_earned_badge() RETURNS trigger
LANGUAGE plpgsql SECURITY INVOKER SET search_path = pg_catalog AS $$
BEGIN
    IF NEW.games < 10 THEN RETURN NULL; END IF;
    IF TG_OP = 'UPDATE' THEN
        IF OLD.games >= 10 AND (OLD.league,OLD.division) = (NEW.league,NEW.division) THEN
            RETURN NULL;
        END IF;
    END IF;
    INSERT INTO central_identity.ranked_badge_peaks(account_id,season_id,track,format,league,division)
        VALUES (NEW.account_id,NEW.season_id,TG_ARGV[0],NEW.format,NEW.league,NEW.division)
        ON CONFLICT DO NOTHING;
    RETURN NULL;
END;
$$;
REVOKE ALL ON FUNCTION central_identity.ranked_earned_badge() FROM PUBLIC;
CREATE TRIGGER ranked_player_earned AFTER INSERT OR UPDATE OF games,league,division
    ON central_identity.ranked_ratings FOR EACH ROW
    EXECUTE FUNCTION central_identity.ranked_earned_badge('player');
CREATE TRIGGER ranked_deck_earned AFTER INSERT OR UPDATE OF games,league,division
    ON central_identity.ranked_deck_ratings FOR EACH ROW
    EXECUTE FUNCTION central_identity.ranked_earned_badge('deck');

-- Operators may give a season a public title. An untitled season is
-- displayed by its unambiguous UTC start date.
ALTER TABLE central_identity.ranked_seasons ADD COLUMN name text
    CHECK (name IS NULL OR octet_length(name) BETWEEN 1 AND 96);
CREATE UNIQUE INDEX ranked_seasons_by_start ON central_identity.ranked_seasons(starts_at);
CREATE INDEX ranked_deck_profile_season ON central_identity.ranked_deck_ratings
    (account_id,format,last_at DESC,season_id DESC);
