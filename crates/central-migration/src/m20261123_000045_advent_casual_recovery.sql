-- A confirmed owner selection can survive rotating or replacing an ordinary
-- native SID. This private row never authorizes a read. Reclaim still needs
-- both a new owner invitation and the host's actual authenticated chair.
CREATE TABLE central_identity.advent_table_outfits (
    gateway_id uuid NOT NULL REFERENCES central_identity.gateways(id) ON DELETE CASCADE,
    game uuid NOT NULL CHECK(uuid_extract_version(game)=7),
    seat smallint NOT NULL CHECK(seat BETWEEN 0 AND 7),
    account_id uuid NOT NULL REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    stream text NOT NULL REFERENCES central_identity.gateway_streams(origin) ON DELETE CASCADE,
    sleeve uuid, playmat uuid,
    expires_at timestamptz NOT NULL DEFAULT clock_timestamp()+interval '24 hours',
    PRIMARY KEY(gateway_id,game,seat),
    CHECK(sleeve IS NOT NULL OR playmat IS NOT NULL),
    FOREIGN KEY(account_id,sleeve) REFERENCES central_identity.advent_entitlements(account_id,art_id) ON DELETE CASCADE,
    FOREIGN KEY(account_id,playmat) REFERENCES central_identity.advent_entitlements(account_id,art_id) ON DELETE CASCADE
);
CREATE INDEX advent_table_outfits_expiry ON central_identity.advent_table_outfits(expires_at);
CREATE INDEX advent_table_outfits_owner ON central_identity.advent_table_outfits(account_id);
REVOKE ALL ON central_identity.advent_table_outfits FROM PUBLIC;
