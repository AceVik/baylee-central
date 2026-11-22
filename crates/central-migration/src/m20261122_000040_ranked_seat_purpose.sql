-- A ranked game and the persistent home gateway have separate revocable sessions.
ALTER TABLE central_identity.gateway_sids
    DROP CONSTRAINT gateway_sids_purpose_check,
    ADD CONSTRAINT gateway_sids_purpose_check CHECK (purpose IN ('admit', 'admin', 'ranked')),
    ADD COLUMN ranked_match uuid REFERENCES central_identity.ranked_matches(id) ON DELETE CASCADE,
    ADD CONSTRAINT gateway_sid_ranked_match CHECK ((purpose='ranked') = (ranked_match IS NOT NULL));
CREATE UNIQUE INDEX gateway_sids_one_ranked ON central_identity.gateway_sids(session_digest)
    WHERE purpose='ranked';
CREATE INDEX gateway_sids_ranked_match ON central_identity.gateway_sids(ranked_match)
    WHERE ranked_match IS NOT NULL;
