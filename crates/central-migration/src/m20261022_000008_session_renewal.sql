-- Renewable sessions (docs/operations.md §"Sessions"): a live session is
-- renewed to 30 minutes from now, never past 12 hours after the password
-- sign-in that issued it (`created_at`), and the gateway leases delegated
-- from it follow. Both lifetime checks move from 30 minutes to that cap; the
-- 30 minutes stay what one issue or renewal grants (accounts/sessions.rs).

ALTER TABLE central_identity.sessions DROP CONSTRAINT session_lifetime;
ALTER TABLE central_identity.sessions ADD CONSTRAINT session_lifetime
    CHECK (expires_at > created_at AND expires_at <= created_at + interval '12 hours');

-- The lease's lifetime check was created without a name: it is found by its
-- definition, exactly one, or the migration fails rather than guess.
DO $$
DECLARE
    found text[];
BEGIN
    SELECT array_agg(c.conname) INTO found
    FROM pg_catalog.pg_constraint c
    WHERE c.conrelid = 'central_identity.gateway_leases'::regclass
      AND c.contype = 'c'
      AND pg_catalog.pg_get_constraintdef(c.oid) LIKE '%expires_at > created_at%'
      AND (pg_catalog.pg_get_constraintdef(c.oid) LIKE '%00:30:00%'
           OR pg_catalog.pg_get_constraintdef(c.oid) LIKE '%30 minutes%');
    IF pg_catalog.array_length(found, 1) IS DISTINCT FROM 1 THEN
        RAISE EXCEPTION 'gateway_leases: expected one 30-minute lifetime check';
    END IF;
    EXECUTE pg_catalog.format('ALTER TABLE central_identity.gateway_leases DROP CONSTRAINT %I', found[1]);
END
$$;
ALTER TABLE central_identity.gateway_leases ADD CONSTRAINT lease_lifetime
    CHECK (expires_at > created_at AND expires_at <= created_at + interval '12 hours');
