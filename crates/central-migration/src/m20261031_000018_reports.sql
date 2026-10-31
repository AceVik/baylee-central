-- Bug and crash reports (docs/reports.md, pass 2 of 31.10.2026): what a
-- gateway files, its gzip parts as they came (Central never unpacks them),
-- and an administrator's triage. The reporter is the gateway's pseudonym or a
-- device's HMAC; no column joins a report to an account.

CREATE TABLE central_identity.reports (
    id uuid PRIMARY KEY DEFAULT pg_catalog.uuidv7() CHECK (pg_catalog.uuid_extract_version(id) = 7),
    created_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    updated_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    -- the authenticated gateway, and its name in Central then; no reference,
    -- so a gateway's or its owner's going takes no report with it
    gateway_id uuid NOT NULL,
    gateway_label text NOT NULL CHECK (octet_length(gateway_label) BETWEEN 1 AND 256),
    -- what the gateway said of itself, beside it
    gateway_says text CHECK (octet_length(gateway_says) BETWEEN 1 AND 256),
    gateway_url text CHECK (octet_length(gateway_url) BETWEEN 1 AND 256),
    gateway_version text CHECK (octet_length(gateway_version) BETWEEN 1 AND 256),
    channel text NOT NULL CHECK (channel IN ('gateway', 'direct')),
    reporter text NOT NULL CHECK (reporter ~ '^[0-9A-Fa-f]{1,128}$'),
    kind text NOT NULL CHECK (kind IN ('bug', 'improvement', 'feedback', 'crash', 'other')),
    status text NOT NULL DEFAULT 'new'
        CHECK (status IN ('new', 'triaged', 'in_progress', 'resolved', 'wont_fix', 'duplicate')),
    text text NOT NULL CHECK (char_length(text) BETWEEN 1 AND 20000),
    game_id text CHECK (char_length(game_id) BETWEEN 1 AND 128),
    build_version text CHECK (octet_length(build_version) BETWEEN 1 AND 256),
    build_commit text CHECK (build_commit ~ '^[0-9A-Fa-f]{1,64}$'),
    crash_message text CHECK (char_length(crash_message) <= 2000),
    crash_location text CHECK (char_length(crash_location) <= 512),
    signature text CHECK (signature ~ '^[0-9a-f]{32}$'),
    -- the parts as announced; their bytes follow in report_parts
    client_bytes integer CHECK (client_bytes BETWEEN 1 AND 2097152),
    client_sha256 bytea CHECK (octet_length(client_sha256) = 32),
    record_bytes integer CHECK (record_bytes BETWEEN 1 AND 67108864),
    record_sha256 bytea CHECK (octet_length(record_sha256) = 32),
    record_complete boolean,
    record_origin text CHECK (record_origin IN ('gateway', 'client')),
    -- the triage
    issue_number integer CHECK (issue_number BETWEEN 1 AND 10000000),
    assignee uuid,
    priority text CHECK (priority IN ('critical', 'high', 'normal', 'low')),
    labels text[] NOT NULL DEFAULT '{}' CHECK (cardinality(labels) <= 16),
    duplicate_of uuid REFERENCES central_identity.reports(id) ON DELETE SET NULL,
    revision integer NOT NULL DEFAULT 0 CHECK (revision >= 0),
    done_at timestamptz,
    deleted_at timestamptz,
    CONSTRAINT report_client CHECK ((client_bytes IS NULL) = (client_sha256 IS NULL)),
    CONSTRAINT report_record CHECK (
        (record_bytes IS NULL) = (record_sha256 IS NULL)
        AND (record_bytes IS NULL) = (record_complete IS NULL)
        AND (record_bytes IS NULL) = (record_origin IS NULL)
        AND (record_origin IS DISTINCT FROM 'client' OR record_bytes <= 4194304)),
    CONSTRAINT report_direct CHECK (channel = 'gateway' OR game_id IS NULL),
    CONSTRAINT report_crash CHECK ((kind = 'crash') OR (crash_message IS NULL AND crash_location IS NULL AND signature IS NULL)),
    CONSTRAINT report_duplicate CHECK ((status = 'duplicate') = (duplicate_of IS NOT NULL) AND duplicate_of IS DISTINCT FROM id),
    CONSTRAINT report_done CHECK ((done_at IS NOT NULL) = (status IN ('resolved', 'wont_fix', 'duplicate')))
);
CREATE INDEX reports_gateway_time ON central_identity.reports(gateway_id, created_at);
CREATE INDEX reports_status ON central_identity.reports(status) WHERE deleted_at IS NULL;
CREATE INDEX reports_signature ON central_identity.reports(signature) WHERE signature IS NOT NULL;
CREATE INDEX reports_duplicate ON central_identity.reports(duplicate_of) WHERE duplicate_of IS NOT NULL;
CREATE INDEX reports_trash ON central_identity.reports(deleted_at) WHERE deleted_at IS NOT NULL;
CREATE INDEX reports_done ON central_identity.reports(done_at) WHERE done_at IS NOT NULL;

-- A part's gzip as it came, written once after its length and SHA-256 equal
-- the report's announcement; never read by a list.
CREATE TABLE central_identity.report_parts (
    report_id uuid NOT NULL REFERENCES central_identity.reports(id) ON DELETE CASCADE,
    part text NOT NULL CHECK (part IN ('client', 'record')),
    gzip bytea NOT NULL CHECK (substring(gzip FROM 1 FOR 2) = '\x1f8b'::bytea),
    PRIMARY KEY (report_id, part)
);

-- An administrator's notes: internal, removed only by their author, gone with the report.
CREATE TABLE central_identity.report_notes (
    id uuid PRIMARY KEY DEFAULT pg_catalog.uuidv7() CHECK (pg_catalog.uuid_extract_version(id) = 7),
    report_id uuid NOT NULL REFERENCES central_identity.reports(id) ON DELETE CASCADE,
    at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    author uuid NOT NULL,
    text text NOT NULL CHECK (char_length(text) BETWEEN 1 AND 4000)
);
CREATE INDEX report_notes_report ON central_identity.report_notes(report_id, at);

-- A report's history: who changed what, never its words; kept after it goes.
CREATE TABLE central_identity.report_audit (
    id uuid PRIMARY KEY DEFAULT pg_catalog.uuidv7() CHECK (pg_catalog.uuid_extract_version(id) = 7),
    report_id uuid NOT NULL,
    at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    actor uuid,
    action text NOT NULL CHECK (action ~ '^report\.[a-z_]{1,32}$'),
    detail text NOT NULL CHECK (char_length(detail) <= 500)
);
CREATE INDEX report_audit_report ON central_identity.report_audit(report_id, id);

-- An administrator's saved lists and the reports they opened: theirs alone.
CREATE TABLE central_identity.report_views (
    id uuid PRIMARY KEY DEFAULT pg_catalog.uuidv7() CHECK (pg_catalog.uuid_extract_version(id) = 7),
    admin uuid NOT NULL,
    name text NOT NULL CHECK (char_length(name) BETWEEN 1 AND 64),
    query text NOT NULL CHECK (octet_length(query) <= 2048),
    created_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    UNIQUE (admin, name)
);
CREATE TABLE central_identity.report_seen (
    admin uuid NOT NULL,
    report_id uuid NOT NULL REFERENCES central_identity.reports(id) ON DELETE CASCADE,
    PRIMARY KEY (admin, report_id)
);
CREATE INDEX report_seen_report ON central_identity.report_seen(report_id);
