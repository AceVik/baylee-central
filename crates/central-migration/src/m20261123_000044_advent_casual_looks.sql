-- Only digest lookups on independently random presentations; a host can
-- accept one only for the admission SID, game and chair chosen by the owner.
CREATE TABLE central_identity.advent_table_looks (
    proof bytea PRIMARY KEY CHECK(octet_length(proof)=32),
    account_id uuid NOT NULL REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    gateway_id uuid NOT NULL REFERENCES central_identity.gateways(id) ON DELETE CASCADE,
    sid bytea NOT NULL REFERENCES central_identity.gateway_sids(sid) ON DELETE CASCADE,
    game uuid NOT NULL CHECK(uuid_extract_version(game)=7),
    seat smallint NOT NULL CHECK(seat BETWEEN 0 AND 7),
    sleeve uuid, playmat uuid, confirmed boolean NOT NULL DEFAULT false,
    expires_at timestamptz NOT NULL DEFAULT clock_timestamp()+interval '5 minutes',
    CHECK(sleeve IS NOT NULL OR playmat IS NOT NULL),
    FOREIGN KEY(account_id,sleeve) REFERENCES central_identity.advent_entitlements(account_id,art_id) ON DELETE CASCADE,
    FOREIGN KEY(account_id,playmat) REFERENCES central_identity.advent_entitlements(account_id,art_id) ON DELETE CASCADE
);
CREATE INDEX advent_table_looks_chair ON central_identity.advent_table_looks(sid,game,seat);
CREATE INDEX advent_table_looks_expiry ON central_identity.advent_table_looks(expires_at);
REVOKE ALL ON central_identity.advent_table_looks FROM PUBLIC;
