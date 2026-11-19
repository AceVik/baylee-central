-- An account's chosen status as its friends see it (docs/push-socket.md
-- §"Status", the owner's decision of 19.11.2026): online, away, or offline,
-- which friends never see. Only the choice is stored, never whether or when
-- the account is online, nor what it is doing: that stays in the service's
-- memory while a push socket stands.
--
-- Additive and with a default, so the previous binary, which never reads the
-- column, runs on unchanged (the deploy hook's way back). To take it back by
-- hand after the previous binary is in place again:
--   ALTER TABLE central_identity.accounts DROP COLUMN presence;
ALTER TABLE central_identity.accounts
    ADD COLUMN presence text NOT NULL DEFAULT 'online'
        CHECK (presence IN ('online', 'afk', 'offline'));
