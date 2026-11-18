-- Independent identity capabilities for Baylee's global Tower service. No game state.
CREATE TABLE central_identity.tower_access (
    id uuid PRIMARY KEY DEFAULT pg_catalog.uuidv7()
        CHECK (pg_catalog.uuid_extract_version(id) IS NOT DISTINCT FROM 7),
    digest bytea NOT NULL UNIQUE CHECK (octet_length(digest) = 32),
    session_digest bytea NOT NULL REFERENCES central_identity.sessions(digest) ON DELETE CASCADE,
    slot smallint NOT NULL CHECK (slot BETWEEN 0 AND 1),
    origin text NOT NULL CHECK (octet_length(origin) BETWEEN 1 AND 2048),
    created_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    expires_at timestamptz NOT NULL,
    UNIQUE (session_digest, slot),
    CHECK (expires_at > created_at AND expires_at <= created_at + interval '5 minutes')
);
REVOKE ALL ON central_identity.tower_access FROM PUBLIC;
