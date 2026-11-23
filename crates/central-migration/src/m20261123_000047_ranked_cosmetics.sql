-- The operator pins every private asset to exactly one family, including
-- under concurrent calendar and ranked imports. IDs and hashes grant no rights.
ALTER TABLE central_identity.advent_art
    ADD COLUMN source text NOT NULL DEFAULT 'advent' CHECK(source IN ('advent','ranked')),
    ADD UNIQUE(id,source);
ALTER TABLE central_identity.advent_doors
    ADD COLUMN source text GENERATED ALWAYS AS ('advent'::text) STORED,
    ADD FOREIGN KEY(art_id,source) REFERENCES central_identity.advent_art(id,source);
ALTER TABLE central_identity.advent_campaigns
    ADD COLUMN source text GENERATED ALWAYS AS ('advent'::text) STORED,
    ADD FOREIGN KEY(hero_id,source) REFERENCES central_identity.advent_art(id,source);

-- The 28 placed ladder steps. Within a league, division I is the highest.
-- This contains no private artwork or mapping from a rank to an item.
CREATE TABLE central_identity.ranked_cosmetic_steps (
    league smallint NOT NULL, division smallint NOT NULL,
    PRIMARY KEY(league,division)
);
INSERT INTO central_identity.ranked_cosmetic_steps(league,division)
SELECT league, count-i FROM (VALUES (0,0),(1,3),(2,3),(3,3),(4,3),(5,4),(6,4),
    (7,4),(8,0),(9,0),(10,0)) AS l(league,count)
CROSS JOIN LATERAL generate_series(0,greatest(count-1,0)) AS steps(i);

-- One private, immutable asset per equipment slot and canonical division.
CREATE TABLE central_identity.ranked_cosmetic_catalog (
    league smallint NOT NULL, division smallint NOT NULL,
    kind text NOT NULL CHECK(kind IN ('sleeve','playmat')),
    art_id uuid NOT NULL UNIQUE, source text GENERATED ALWAYS AS ('ranked'::text) STORED,
    PRIMARY KEY(league,division,kind), UNIQUE(league,division,art_id),
    FOREIGN KEY(league,division) REFERENCES central_identity.ranked_cosmetic_steps,
    FOREIGN KEY(art_id,kind) REFERENCES central_identity.advent_art(id,kind),
    FOREIGN KEY(art_id,source) REFERENCES central_identity.advent_art(id,source)
);
-- Grants name the achievement, independently of whether its two images
-- have been privately provisioned yet. Releasing art needs no account scan.
CREATE TABLE central_identity.ranked_cosmetic_unlocks (
    account_id uuid NOT NULL REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    league smallint NOT NULL, division smallint NOT NULL,
    earned_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
    PRIMARY KEY(account_id,league,division),
    FOREIGN KEY(league,division) REFERENCES central_identity.ranked_cosmetic_steps
);

-- Placement, division hysteresis and the Champion publication have already
-- been resolved by the existing badge ledger. The same division in another
-- format or season is one permanent set. The runtime cannot write unlocks.
CREATE FUNCTION central_identity.earn_ranked_cosmetics() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog AS $$
BEGIN
    INSERT INTO central_identity.ranked_cosmetic_unlocks(account_id,league,division)
    SELECT NEW.account_id,s.league,s.division FROM central_identity.ranked_cosmetic_steps s
    WHERE s.league<NEW.league OR (s.league=NEW.league AND s.division>=NEW.division)
    ON CONFLICT DO NOTHING;
    RETURN NULL;
END
$$;
REVOKE ALL ON FUNCTION central_identity.earn_ranked_cosmetics() FROM PUBLIC;
CREATE TRIGGER ranked_cosmetic_earned AFTER INSERT ON central_identity.ranked_badge_peaks
FOR EACH ROW WHEN (NEW.track='player') EXECUTE FUNCTION central_identity.earn_ranked_cosmetics();
-- Historical badges carry no acquisition timestamp. Record the conversion
-- time once, independent of the order in which their artwork is imported.
INSERT INTO central_identity.ranked_cosmetic_unlocks(account_id,league,division)
SELECT p.account_id,s.league,s.division FROM (
    SELECT DISTINCT ON(account_id) account_id,league,division
    FROM central_identity.ranked_badge_peaks WHERE track='player'
    ORDER BY account_id,league DESC,division ASC
) p JOIN central_identity.ranked_cosmetic_steps s
    ON s.league<p.league OR (s.league=p.league AND s.division>=p.division)
ON CONFLICT DO NOTHING;

CREATE VIEW central_identity.cosmetic_collection WITH (security_invoker=true) AS
    SELECT account_id,art_id,earned_at,year,door,NULL::smallint AS league,NULL::smallint AS division
    FROM central_identity.advent_entitlements
    UNION ALL
    SELECT u.account_id,c.art_id,u.earned_at,NULL::smallint,NULL::smallint,u.league,u.division
    FROM central_identity.ranked_cosmetic_unlocks u
    JOIN central_identity.ranked_cosmetic_catalog c USING(league,division);

-- A table appearance retains the same composite ownership foreign key for both
-- acquisition paths. A new reference is materialized only when actually worn.
CREATE TABLE central_identity.cosmetic_equip_rights (
    account_id uuid NOT NULL, art_id uuid NOT NULL,
    advent_id uuid, league smallint, division smallint,
    PRIMARY KEY(account_id,art_id),
    CHECK((advent_id IS NOT NULL AND advent_id=art_id AND league IS NULL AND division IS NULL)
       OR (advent_id IS NULL AND league IS NOT NULL AND division IS NOT NULL)),
    FOREIGN KEY(account_id,advent_id) REFERENCES central_identity.advent_entitlements(account_id,art_id) ON DELETE CASCADE,
    FOREIGN KEY(account_id,league,division) REFERENCES central_identity.ranked_cosmetic_unlocks(account_id,league,division) ON DELETE CASCADE,
    FOREIGN KEY(league,division,art_id) REFERENCES central_identity.ranked_cosmetic_catalog(league,division,art_id) ON DELETE CASCADE
);
INSERT INTO central_identity.cosmetic_equip_rights(account_id,art_id,advent_id)
SELECT account_id,art_id,art_id FROM central_identity.advent_entitlements;
DO $$
DECLARE rel text;
BEGIN
    FOREACH rel IN ARRAY ARRAY['advent_ranked_looks','advent_table_looks','advent_table_outfits'] LOOP
        EXECUTE format('ALTER TABLE central_identity.%1$I
            DROP CONSTRAINT %2$I, DROP CONSTRAINT %3$I,
            ADD FOREIGN KEY(account_id,sleeve) REFERENCES central_identity.cosmetic_equip_rights(account_id,art_id) ON DELETE CASCADE,
            ADD FOREIGN KEY(account_id,playmat) REFERENCES central_identity.cosmetic_equip_rights(account_id,art_id) ON DELETE CASCADE',
            rel, rel || '_account_id_sleeve_fkey', rel || '_account_id_playmat_fkey');
    END LOOP;
END $$;
REVOKE ALL ON central_identity.cosmetic_collection, central_identity.cosmetic_equip_rights,
    central_identity.ranked_cosmetic_catalog, central_identity.ranked_cosmetic_unlocks,
    central_identity.ranked_cosmetic_steps FROM PUBLIC;
