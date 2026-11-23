-- One bounded, versioned checkpoint per protected game. The match lock
-- serializes each observation, journal append and removal of ranked eligibility.
CREATE TABLE central_identity.ranked_clocks (
    match_id uuid PRIMARY KEY REFERENCES central_identity.ranked_replicas(match_id) ON DELETE CASCADE,
    checkpoint bytea NOT NULL CHECK (octet_length(checkpoint) BETWEEN 1 AND 2048)
);
