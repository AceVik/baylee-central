-- A season whose final standing is no distinction (owner, 28.11.2026): the
-- pre-season that ran from the first ranked games until Season 0 closes
-- without season-finish awards, its Elo and placements carried on. Every
-- other season keeps them. Division badges, cosmetics and Champion places
-- are earned while a season runs and are not touched.
ALTER TABLE central_identity.ranked_seasons
    ADD COLUMN finish_awards boolean NOT NULL DEFAULT true;
