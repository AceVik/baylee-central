-- Open registration (the owner's decisions of 10 and 11.11.2026,
-- docs/registration-abuse.md): the closed beta's keys go. Central's proof of
-- work and the per-address budgets protect registration instead, and an
-- administrator can close it during an attack.
--
-- Only what served the keys is removed: their table and an unfinished
-- registration's link to the key it held. Accounts, finished or pending,
-- keep everything else; a pending one keeps its ticket and can still enrol.
-- The audit's beta_key_id column never had a foreign key and stays as the
-- history it is.
ALTER TABLE central_identity.accounts DROP CONSTRAINT account_registered_pending;
ALTER TABLE central_identity.accounts DROP CONSTRAINT account_beta_key_pending;
ALTER TABLE central_identity.accounts DROP COLUMN beta_key_id;
ALTER TABLE central_identity.accounts ADD CONSTRAINT account_registered_pending
    CHECK (source = 'legacy' OR credential IS NOT NULL OR enrollment_digest IS NOT NULL);
DROP TABLE central_identity.beta_keys;

-- The administrators' brake: one row, open unless closed. Who closed or
-- opened it last and when; each change is also an audit event.
CREATE TABLE central_identity.registration (
    singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
    open boolean NOT NULL DEFAULT true,
    changed_at timestamptz,
    changed_by uuid REFERENCES central_identity.accounts(id) ON DELETE SET NULL
);
INSERT INTO central_identity.registration (singleton, open) VALUES (true, true);
REVOKE ALL ON central_identity.registration FROM PUBLIC;
