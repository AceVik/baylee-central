-- What a gateway says is happening on it (docs/gateway-registry.md
-- §"Activity and the public figures"; the owner's decision of 11.11.2026:
-- the gateways tell Central, Central tells the website). Four counts and
-- when they were reported, on the gateway's own row: players connected, of
-- those the players seated in a running game, tables open to join, games
-- running. No name, no account, no address.
--
-- All five are NULL until a gateway reports (`GatewayPulse`) and again once
-- it sends a plain heartbeat, so a gateway that does not report shows
-- nothing. The counts are the gateway's claim; the bound keeps a claim from
-- showing an absurd number (central_api::gateway::GatewayActivity::MOST),
-- and no claim has more players in a game than connected.
--
-- The runtime's UPDATE on this table covers the new columns; it gains no
-- grant. No index: the public read is the directory's scan of approved
-- gateways with two more conditions, and an index on a column every pulse
-- writes would cost each pulse its in-place update.
ALTER TABLE central_identity.gateways
    ADD COLUMN activity_players integer,
    ADD COLUMN activity_playing integer,
    ADD COLUMN activity_tables integer,
    ADD COLUMN activity_games integer,
    ADD COLUMN activity_at timestamptz;
ALTER TABLE central_identity.gateways
    ADD CONSTRAINT gateway_activity_whole CHECK (pg_catalog.num_nonnulls(
        activity_players, activity_playing, activity_tables, activity_games,
        activity_at) IN (0, 5)),
    ADD CONSTRAINT gateway_activity_bounds CHECK (
        activity_players BETWEEN 0 AND 100000
        AND activity_playing BETWEEN 0 AND 100000
        AND activity_tables BETWEEN 0 AND 100000
        AND activity_games BETWEEN 0 AND 100000),
    ADD CONSTRAINT gateway_activity_playing CHECK (activity_playing <= activity_players);
