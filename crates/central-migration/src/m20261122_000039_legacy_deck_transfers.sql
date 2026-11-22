-- The operator imports the exact original bytes and their existing SHA-256. External
-- uncompressed TOAST storage lets the legacy role serve one 64-KiB substring at a time.
ALTER TABLE central_legacy.accounts
    DROP CONSTRAINT accounts_snapshot_check,
    ADD CONSTRAINT legacy_source_size CHECK (octet_length(snapshot) BETWEEN 2 AND 8388608),
    ALTER COLUMN snapshot SET STORAGE EXTERNAL;

CREATE TABLE central_identity.legacy_deck_transfers (
    account_id uuid PRIMARY KEY REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    generation bytea NOT NULL CHECK (octet_length(generation)=32),
    source_digest bytea NOT NULL CHECK (octet_length(source_digest)=32),
    source_bytes integer NOT NULL CHECK (source_bytes BETWEEN 196609 AND 8388608),
    next_page smallint NOT NULL DEFAULT 0 CHECK (next_page BETWEEN 0 AND (source_bytes+65535)/65536)
);
ALTER TABLE central_identity.accounts ADD COLUMN legacy_deck_source bytea
    CHECK (legacy_deck_source IS NULL OR (octet_length(legacy_deck_source)=32 AND legacy_id IS NOT NULL));
REVOKE ALL ON central_identity.legacy_deck_transfers FROM PUBLIC;
