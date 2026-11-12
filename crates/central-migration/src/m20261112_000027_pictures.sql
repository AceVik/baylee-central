-- Profile pictures (docs/profile-pictures.md; the owner's decisions of
-- 12.11.2026 in docs/dependency-proposal/profile-picture.md §0). Four
-- tables in central_identity, each hanging on the account's row and gone
-- with it. The previous binary names none of them, so it runs unchanged
-- over this migration.
--
-- pictures: an account's current picture, made by the sandboxed worker
-- from the upload's pixels alone: a 256 px AVIF and a 256 px PNG master
-- that is also the fallback, and for a picture that moves an AVIF image
-- sequence that ffmpeg made from the worker's own frames (an animation
-- whose encoding failed keeps only its still). `key` names it in its
-- public URLs: 16 bytes of
-- independent randomness, new at every upload, never the account's id.
-- `sha256` is the digest of the PNG master followed by the sequence,
-- computed by the service, not taken from the worker; reports and
-- decisions name a picture by it, so a report names the animation shown. Nothing of
-- the upload itself is kept.
CREATE TABLE central_identity.pictures (
    account_id uuid PRIMARY KEY
        REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    key bytea NOT NULL UNIQUE CHECK (octet_length(key) = 16),
    avif bytea NOT NULL CHECK (octet_length(avif) BETWEEN 32 AND 163840),
    png bytea NOT NULL CHECK (octet_length(png) BETWEEN 67 AND 327680),
    sequence bytea CHECK (octet_length(sequence) BETWEEN 32 AND 524288),
    sha256 bytea NOT NULL CHECK (octet_length(sha256) = 32),
    side smallint NOT NULL CHECK (side = 256),
    set_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp()
);
-- Compressed pictures do not compress again; out of line, a row stays small.
ALTER TABLE central_identity.pictures ALTER COLUMN avif SET STORAGE EXTERNAL;
ALTER TABLE central_identity.pictures ALTER COLUMN png SET STORAGE EXTERNAL;
ALTER TABLE central_identity.pictures ALTER COLUMN sequence SET STORAGE EXTERNAL;

-- picture_reports: a signed-in account's report of another account's
-- picture, by the picture's digest, with a reason from the closed list and
-- an optional note. Administrators see the reporter; the reported person
-- never does. The runtime files and decides reports but never deletes one:
-- the operator's sweep deletes decided reports a year after their
-- decision. A deleted reporter's reports lose their reporter, a deleted
-- account's reports go with it, and a deleted administrator's decisions
-- lose their decider, so no row names a deleted account.
CREATE TABLE central_identity.picture_reports (
    id uuid PRIMARY KEY DEFAULT pg_catalog.uuidv7()
        CHECK (pg_catalog.uuid_extract_version(id) = 7),
    account_id uuid NOT NULL
        REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    sha256 bytea NOT NULL CHECK (octet_length(sha256) = 32),
    reporter_id uuid
        REFERENCES central_identity.accounts(id) ON DELETE SET NULL,
    reason text NOT NULL CHECK (reason IN
        ('sexual', 'violence', 'hate', 'harassment', 'impersonation', 'private', 'other')),
    note text CHECK (char_length(note) BETWEEN 1 AND 1000),
    created_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    state text NOT NULL DEFAULT 'open'
        CHECK (state IN ('open', 'kept', 'removed', 'gone')),
    decided_at timestamptz,
    decided_by uuid
        REFERENCES central_identity.accounts(id) ON DELETE SET NULL,
    CHECK ((state = 'open') = (decided_at IS NULL)),
    CHECK (reporter_id IS DISTINCT FROM account_id)
);
-- One open report per reporter and picture.
CREATE UNIQUE INDEX picture_reports_open
    ON central_identity.picture_reports (account_id, sha256, reporter_id)
    WHERE state = 'open';
-- The administrators' queue, newest first, all or by state.
CREATE INDEX picture_reports_queue
    ON central_identity.picture_reports (state, id);
-- A reporter's rate and its own export.
CREATE INDEX picture_reports_reporter
    ON central_identity.picture_reports (reporter_id, created_at);
-- An account's case.
CREATE INDEX picture_reports_account
    ON central_identity.picture_reports (account_id, id);
-- The sweep.
CREATE INDEX picture_reports_decided
    ON central_identity.picture_reports (decided_at) WHERE decided_at IS NOT NULL;

-- picture_notices: what the person is told of an administrator's last
-- removal (when, the reason from the list, the administrator's sentence),
-- and an upload block: NULL none, a time, or 'infinity' until lifted. A
-- new picture deletes the notice; a block in force refuses the upload.
CREATE TABLE central_identity.picture_notices (
    account_id uuid PRIMARY KEY
        REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    removed_at timestamptz NOT NULL,
    reason text NOT NULL CHECK (reason IN
        ('sexual', 'violence', 'hate', 'harassment', 'impersonation', 'private', 'other')),
    note text CHECK (char_length(note) BETWEEN 1 AND 500),
    blocked_until timestamptz
);

-- picture_budgets: one fixed row per account that uploaded, ten uploads an
-- hour and thirty a day, counted once the body was read, stored or not.
CREATE TABLE central_identity.picture_budgets (
    account_id uuid PRIMARY KEY
        REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    hour_start timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    hour_count smallint NOT NULL DEFAULT 0 CHECK (hour_count BETWEEN 0 AND 10),
    day_start timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    day_count smallint NOT NULL DEFAULT 0 CHECK (day_count BETWEEN 0 AND 30)
);
