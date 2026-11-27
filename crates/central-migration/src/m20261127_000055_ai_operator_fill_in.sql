-- AI players (docs/ai-players.md), after 000053 and 000054 ran on test stacks:
-- what the operator's command and calibration need, as a migration of its own.
--
-- The operator's command (central-service --ai-players-stdin) makes runner
-- tokens without an administrator: a token without a maker is the operator's,
-- as a revocation without a revoker already is.
ALTER TABLE central_identity.ai_runner_tokens ALTER COLUMN created_by DROP NOT NULL;

-- Calibration is the carousel without fill-in (docs/ai-players.md section 7):
-- a switch of its own, on unless an administrator or the operator sets it off.
ALTER TABLE central_identity.ai_settings ADD COLUMN fill_in boolean NOT NULL DEFAULT true;
