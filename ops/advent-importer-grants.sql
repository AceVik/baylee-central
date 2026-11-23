-- Optional offline provisioning role. A DBA creates central_advent_importer before applying
-- this file as schema owner and grants membership only to the trusted provisioning login.
BEGIN;
GRANT USAGE ON SCHEMA central_identity TO central_advent_importer;
GRANT INSERT (digest, kind, tier, title_de, title_en, size, width, height, mime, content),
    SELECT (id, digest, kind, tier, title_de, title_en, size, width, height, mime)
    ON central_identity.advent_art TO central_advent_importer;
GRANT SELECT (year, hero_id, starts_at, closes_at), INSERT (year, hero_id, starts_at, closes_at)
    ON central_identity.advent_campaigns TO central_advent_importer;
REVOKE SELECT, INSERT ON central_identity.advent_doors FROM central_advent_importer;
GRANT SELECT (year, door, puzzle, hint1_de, hint1_en, hint2_de, hint2_en, art_id),
      INSERT (year, door, puzzle, hint1_de, hint1_en, hint2_de, hint2_en, art_id)
    ON central_identity.advent_doors TO central_advent_importer;
GRANT SELECT (art_id, framing), INSERT (art_id, framing)
    ON central_identity.playmat_framing TO central_advent_importer;
COMMIT;
