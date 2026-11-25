-- Group profiles (description and picture) and group administrators.
-- The group creator/owner stays an administrator permanently.
ALTER TABLE central_identity.chat_groups
    ADD COLUMN description text NOT NULL DEFAULT ''
    CHECK (char_length(description) <= 280);

ALTER TABLE central_identity.chat_members
    ADD COLUMN admin boolean NOT NULL DEFAULT false;

UPDATE central_identity.chat_members m
SET admin = true
FROM central_identity.chat_groups g
WHERE m.group_id = g.id AND m.account_id = g.owner;

CREATE TABLE central_identity.chat_group_pictures (
    group_id uuid PRIMARY KEY
        REFERENCES central_identity.chat_groups(id) ON DELETE CASCADE,
    key bytea NOT NULL UNIQUE CHECK (octet_length(key) = 16),
    avif bytea NOT NULL CHECK (octet_length(avif) BETWEEN 32 AND 163840),
    png bytea NOT NULL CHECK (octet_length(png) BETWEEN 67 AND 327680),
    sequence bytea CHECK (octet_length(sequence) BETWEEN 32 AND 524288),
    sha256 bytea NOT NULL CHECK (octet_length(sha256) = 32),
    side smallint NOT NULL CHECK (side = 256),
    set_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp()
);
ALTER TABLE central_identity.chat_group_pictures ALTER COLUMN avif SET STORAGE EXTERNAL;
ALTER TABLE central_identity.chat_group_pictures ALTER COLUMN png SET STORAGE EXTERNAL;
ALTER TABLE central_identity.chat_group_pictures ALTER COLUMN sequence SET STORAGE EXTERNAL;
REVOKE ALL ON central_identity.chat_group_pictures FROM PUBLIC;
