-- More AI seats at once (owner, 28.11.2026; docs/ai-players.md, "More AI
-- seats at once"): a runner's relay sockets have a budget of their own, so
-- every AI together may hold 32 seats, and the carousel 16 matches between
-- two AIs (two seats each). The two CHECKs that 000053 wrote are widened and
-- nothing else: no row changes, so the settings keep what an administrator or
-- the operator set, and no grant changes.
ALTER TABLE central_identity.ai_settings
    DROP CONSTRAINT ai_settings_global_seats_check,
    ADD CONSTRAINT ai_settings_global_seats_check CHECK (global_seats BETWEEN 0 AND 32),
    DROP CONSTRAINT ai_settings_carousel_games_check,
    ADD CONSTRAINT ai_settings_carousel_games_check CHECK (carousel_games BETWEEN 0 AND 16);
