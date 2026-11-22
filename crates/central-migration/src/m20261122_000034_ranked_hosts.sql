-- Ranked trust is a second administrator decision, including for the official
-- gateway. Existing approvals start unverified. Directory approval, ordinary
-- registration, token issuance and heartbeats cannot confer ranked trust.
ALTER TABLE central_identity.gateway_approvals
    ADD COLUMN ranked_verified boolean NOT NULL DEFAULT false,
    ADD COLUMN ranked_epoch bigint NOT NULL DEFAULT 0 CHECK (ranked_epoch >= 0),
    ADD CONSTRAINT ranked_requires_approval CHECK (NOT ranked_verified OR enabled);

-- A disable through either the website or the operator's SQL invalidates
-- outstanding assignments. Re-enabling the directory never restores trust.
CREATE FUNCTION central_identity.ranked_approval_changed() RETURNS trigger
LANGUAGE plpgsql SECURITY INVOKER SET search_path = pg_catalog AS $$
BEGIN
    NEW.ranked_verified := NEW.enabled AND NEW.ranked_verified;
    IF NEW.ranked_verified IS DISTINCT FROM OLD.ranked_verified THEN
        NEW.ranked_epoch := OLD.ranked_epoch + 1;
    END IF;
    RETURN NEW;
END;
$$;
REVOKE ALL ON FUNCTION central_identity.ranked_approval_changed() FROM PUBLIC;
CREATE TRIGGER ranked_approval_changed BEFORE UPDATE OF enabled, ranked_verified
    ON central_identity.gateway_approvals FOR EACH ROW
    EXECUTE FUNCTION central_identity.ranked_approval_changed();
