-- Optional complement to advent-importer-grants.sql for the private ranked operator.
BEGIN;
GRANT INSERT (source) ON central_identity.advent_art TO central_advent_importer;
GRANT SELECT (league, division, kind, art_id), INSERT (league, division, kind, art_id)
    ON central_identity.ranked_cosmetic_catalog TO central_advent_importer;
COMMIT;
