-- A recovery code for accounts moved from the official gateway, which kept
-- none (docs/web-accounts.md §"A recovery code after a sign-in"): an enrolled
-- legacy account may now hold a recovery wrapper, which `SetRecovery` stores
-- once its move has finished. A pending account still holds none, and every
-- enrolled registered account still holds one. The wrapper is bound to the
-- vault epoch: whatever raises the epoch (`invalidate_legacy`, and later an
-- administrator's reset) clears it in the same statement. The previous binary,
-- which the deploy hook restarts when a new one fails to start, still works:
-- it writes no wrapper on a legacy account, and its `invalidate_legacy`
-- refuses a closed move, the only legacy account that can hold one, before it
-- changes anything.

ALTER TABLE central_identity.accounts DROP CONSTRAINT account_recovery;
ALTER TABLE central_identity.accounts ADD CONSTRAINT account_recovery CHECK (
    CASE
        WHEN credential IS NULL THEN recovery_wrapped_master IS NULL
        WHEN source = 'registered' THEN recovery_wrapped_master IS NOT NULL
        ELSE true
    END
);
