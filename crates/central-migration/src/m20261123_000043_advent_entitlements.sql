-- Operator-only campaign materials. A fresh install contains no real prizes,
-- artwork, challenge definitions, solutions or day-to-cosmetic mapping.
CREATE TABLE central_identity.advent_art (
    id uuid PRIMARY KEY DEFAULT uuidv7(),
    kind text NOT NULL CHECK (kind IN ('sleeve','playmat','hero')),
    tier text NOT NULL DEFAULT 'standard' CHECK (tier IN ('standard','special','finale')),
    title_de text NOT NULL CHECK (octet_length(title_de) BETWEEN 1 AND 160),
    title_en text NOT NULL CHECK (octet_length(title_en) BETWEEN 1 AND 160),
    mime text NOT NULL CHECK (mime='image/webp'),
    size integer NOT NULL CHECK (size BETWEEN 1 AND 16777216),
    width integer NOT NULL CHECK (width BETWEEN 1 AND 4096),
    height integer NOT NULL CHECK (height BETWEEN 1 AND 4096),
    digest bytea NOT NULL UNIQUE CHECK (octet_length(digest)=32),
    content bytea NOT NULL CHECK (octet_length(content)=size AND sha256(content)=digest),
    UNIQUE(id,kind)
);
-- Authorized range reads need only the requested TOAST chunks.
ALTER TABLE central_identity.advent_art ALTER COLUMN content SET STORAGE EXTERNAL;
CREATE TABLE central_identity.advent_campaigns (
    year smallint PRIMARY KEY CHECK (year BETWEEN 2026 AND 2200),
    starts_at timestamptz NOT NULL, closes_at timestamptz NOT NULL,
    CHECK (closes_at=starts_at+interval '888 hours'),
    hero_id uuid NOT NULL, hero_kind text GENERATED ALWAYS AS ('hero'::text) STORED,
    FOREIGN KEY (hero_id,hero_kind) REFERENCES central_identity.advent_art(id,kind)
);
CREATE TABLE central_identity.advent_doors (
    year smallint NOT NULL REFERENCES central_identity.advent_campaigns(year),
    door smallint NOT NULL CHECK (door BETWEEN 1 AND 24),
    puzzle text NOT NULL CHECK (octet_length(puzzle) BETWEEN 2 AND 32768),
    hint1_de text NOT NULL CHECK (octet_length(hint1_de) BETWEEN 1 AND 512),
    hint1_en text NOT NULL CHECK (octet_length(hint1_en) BETWEEN 1 AND 512),
    hint2_de text NOT NULL CHECK (octet_length(hint2_de) BETWEEN 1 AND 512),
    hint2_en text NOT NULL CHECK (octet_length(hint2_en) BETWEEN 1 AND 512),
    art_id uuid NOT NULL REFERENCES central_identity.advent_art(id),
    PRIMARY KEY(year,door), UNIQUE(year,art_id), UNIQUE(year,door,art_id)
);
CREATE TABLE central_identity.advent_solutions (
    account_id uuid NOT NULL REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    year smallint NOT NULL, door smallint NOT NULL,
    solved_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY(account_id,year,door),
    FOREIGN KEY(year,door) REFERENCES central_identity.advent_doors(year,door)
);
CREATE TABLE central_identity.advent_entitlements (
    account_id uuid NOT NULL, year smallint NOT NULL, door smallint NOT NULL, art_id uuid NOT NULL,
    earned_at timestamptz NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY(account_id,year,door), UNIQUE(account_id,art_id),
    FOREIGN KEY(account_id,year,door) REFERENCES central_identity.advent_solutions(account_id,year,door) ON DELETE CASCADE,
    FOREIGN KEY(year,door,art_id) REFERENCES central_identity.advent_doors(year,door,art_id)
);
REVOKE ALL ON central_identity.advent_art, central_identity.advent_campaigns,
    central_identity.advent_doors,central_identity.advent_solutions,
    central_identity.advent_entitlements FROM PUBLIC;

-- Only the two assigned players can disclose their own deck choices; an
-- opponent can resolve exactly one displayed slot during this game.
CREATE TABLE central_identity.advent_ranked_looks (
    match_id uuid NOT NULL REFERENCES central_identity.ranked_matches(id) ON DELETE CASCADE,
    account_id uuid NOT NULL REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    sleeve uuid, playmat uuid,
    PRIMARY KEY(match_id,account_id),
    FOREIGN KEY(account_id,sleeve) REFERENCES central_identity.advent_entitlements(account_id,art_id) ON DELETE CASCADE,
    FOREIGN KEY(account_id,playmat) REFERENCES central_identity.advent_entitlements(account_id,art_id) ON DELETE CASCADE
);
REVOKE ALL ON central_identity.advent_ranked_looks FROM PUBLIC;
CREATE OR REPLACE FUNCTION central_identity.retire_advent_looks() RETURNS trigger
LANGUAGE plpgsql SET search_path=pg_catalog AS $$
BEGIN
    DELETE FROM central_identity.advent_ranked_looks WHERE match_id=NEW.id;
    RETURN NEW;
END
$$;
CREATE TRIGGER retire_advent_looks AFTER UPDATE OF state ON central_identity.ranked_matches
FOR EACH ROW WHEN (OLD.state IS DISTINCT FROM NEW.state AND NEW.state IN ('finished','void'))
EXECUTE FUNCTION central_identity.retire_advent_looks();
REVOKE ALL ON FUNCTION central_identity.retire_advent_looks() FROM PUBLIC;
