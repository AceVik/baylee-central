-- An account's chosen status as its friends see it (docs/push-socket.md
-- §"Status", the owner's decision of 19.11.2026): online, away, or offline,
-- which friends never see. Only the choice is stored, never whether or when
-- the account is online, nor what it is doing: that stays in the service's
-- memory while a push socket stands.
--
-- Additive and with a default: the previous binary, which never names the
-- column, starts and serves on the migrated schema (the deploy hook's way back
-- starts it without migrating), but its own migration run refuses a history
-- that names this migration. To take it back entirely, by hand, with the
-- previous binary in place, both together:
--   ALTER TABLE central_identity.accounts DROP COLUMN presence;
--   DELETE FROM public.seaql_migrations WHERE version = 'm20261119_000033_presence';
-- The column dropped alone would leave a history this binary trusts: it would
-- not add the column again.
ALTER TABLE central_identity.accounts
    ADD COLUMN presence text NOT NULL DEFAULT 'online'
        CHECK (presence IN ('online', 'afk', 'offline'));
