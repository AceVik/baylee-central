-- Central administration (docs/administration.md; the accounts and
-- administration design, step 5): who holds the administrator role, an
-- append-only record of what administrators and the operator did, and when
-- an account last signed in with its password.
--
-- A role row exists while the account holds the role; `granted_by` is the
-- administrator who granted it, or NULL for the operator command. The audit
-- names ids, never words: no name, reason, note or key, which may name a
-- person. Its ids carry no foreign keys, since an event outlives what it
-- names. The runtime may insert events and never rewrite or delete one; the
-- operator's sweep deletes them after three years.

CREATE TABLE central_identity.account_roles (
    account_id uuid NOT NULL REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    role text NOT NULL CHECK (role IN ('admin')),
    granted_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    granted_by uuid,
    PRIMARY KEY (account_id, role)
);

CREATE TABLE central_identity.audit_events (
    id uuid PRIMARY KEY DEFAULT pg_catalog.uuidv7()
        CHECK (pg_catalog.uuid_extract_version(id) IS NOT DISTINCT FROM 7),
    at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    actor_kind text NOT NULL CHECK (actor_kind IN ('admin', 'operator')),
    actor_account uuid,
    action text NOT NULL CHECK (action ~ '^[a-z_]{1,32}\.[a-z_]{1,32}$'),
    account_id uuid,
    gateway_id uuid,
    beta_key_id uuid,
    report_id uuid,
    detail jsonb CHECK (octet_length(detail::text) <= 1024),
    CONSTRAINT audit_actor CHECK ((actor_kind = 'admin') = (actor_account IS NOT NULL))
);
CREATE INDEX audit_events_account ON central_identity.audit_events(account_id, id)
    WHERE account_id IS NOT NULL;
CREATE INDEX audit_events_at ON central_identity.audit_events(at);

-- Set at every password sign-in, of either kind; renewal does not move it.
-- Only the time, no address.
ALTER TABLE central_identity.accounts ADD COLUMN last_sign_in_at timestamptz;

REVOKE ALL ON central_identity.account_roles, central_identity.audit_events FROM PUBLIC;
