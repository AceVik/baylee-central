-- A ranked game and the persistent home gateway have separate revocable sessions.
ALTER TABLE central_identity.gateway_sids
    DROP CONSTRAINT gateway_sids_purpose_check,
    ADD CONSTRAINT gateway_sids_purpose_check CHECK (purpose IN ('admit', 'admin', 'ranked'));
