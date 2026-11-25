-- Report tokens (docs/reports.md, "Report tokens"): the credential an
-- administrator makes for a script, a build machine or an agent that files,
-- reads or triages bug and crash reports. Only the SHA-256 digest is stored.
-- A token holds rights (file, read, triage, purge), is named by its maker,
-- expires within 366 days, can be revoked, and authenticates only while its
-- maker is an administrator.
--
-- The maker and the revoker are account ids without a reference: a token and
-- what it did outlive them, and the authenticating query asks for the role.
-- A revocation without a revoker is the operator's.
CREATE TABLE central_identity.report_tokens (
    id uuid PRIMARY KEY DEFAULT pg_catalog.uuidv7()
        CHECK (pg_catalog.uuid_extract_version(id) IS NOT DISTINCT FROM 7),
    name text NOT NULL CHECK (octet_length(name) BETWEEN 1 AND 64),
    digest bytea NOT NULL UNIQUE CHECK (octet_length(digest) = 32),
    file boolean NOT NULL,
    read boolean NOT NULL,
    triage boolean NOT NULL,
    purge boolean NOT NULL,
    created_by uuid NOT NULL,
    created_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    expires_at timestamptz NOT NULL,
    revoked_at timestamptz,
    revoked_by uuid,
    last_used_at timestamptz,
    -- something is allowed; who changes a report reads it, and who deletes
    -- one for good may move it to the trash first
    CONSTRAINT report_token_rights CHECK (
        (file OR read OR triage OR purge) AND (read OR NOT triage) AND (triage OR NOT purge)),
    CONSTRAINT report_token_lifetime CHECK (
        expires_at > created_at AND expires_at <= created_at + interval '366 days'),
    CONSTRAINT report_token_revoked CHECK (
        (revoked_at IS NULL OR revoked_at >= created_at)
        AND (revoked_by IS NULL OR revoked_at IS NOT NULL))
);
REVOKE ALL ON central_identity.report_tokens FROM PUBLIC;

-- Who filed a report: a gateway (or the bridge), as every report until now,
-- or a report token. A token's id then stands in gateway_id and its name, as
-- it was, in gateway_label, so that the store weighs its reports and its
-- parts find them as a gateway's do. A token is no gateway: it files no
-- record as one's own, and no direct report.
ALTER TABLE central_identity.reports
    ADD COLUMN filed_by text NOT NULL DEFAULT 'gateway' CHECK (filed_by IN ('gateway', 'token')),
    ADD CONSTRAINT report_by_token CHECK (
        filed_by = 'gateway' OR (channel = 'gateway' AND record_origin IS DISTINCT FROM 'gateway'));
CREATE INDEX reports_by_token ON central_identity.reports(gateway_id) WHERE filed_by = 'token';

-- Who wrote a note or changed a report: an administrator, or a report token,
-- whose name then, beside its id, says so.
ALTER TABLE central_identity.report_notes
    ADD COLUMN token_name text CHECK (octet_length(token_name) BETWEEN 1 AND 64);
ALTER TABLE central_identity.report_audit
    ADD COLUMN token_name text CHECK (octet_length(token_name) BETWEEN 1 AND 64),
    ADD CONSTRAINT report_audit_token CHECK (token_name IS NULL OR actor IS NOT NULL);

-- The token an administrator's change made or revoked, by its id. Like the
-- record's other ids it refers to nothing.
ALTER TABLE central_identity.audit_events ADD COLUMN report_token_id uuid;
