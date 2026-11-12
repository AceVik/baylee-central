-- An account's optional e-mail address (docs/email.md; the owner's decision
-- of 12.11.2026). No account needs one, and nothing else in the schema
-- knows an address: both tables hang on the account's row and go with it.
--
-- account_emails: the address an account confirmed with a mailed code, when,
-- and the language its code was asked in, which is the language a later
-- notice to that address is written in. Addresses are not unique: two
-- accounts may name the same mailbox. The row also keeps what a reset code
-- by mail needs: `reset_digest` is the digest of the reset code Central
-- last mailed for the account, and `reset_mailed` when it mailed the last
-- ones, three at most.
--
-- The reset code itself stays where 000013 put it, on the account's row,
-- and that row and its rules are not changed here: the statements of the
-- release before this one still make, use, clear and sweep a code. A code
-- on the account's row was mailed exactly when its digest is also the
-- `reset_digest` here; every other code is an administrator's, and a mailed
-- request never replaces one of those while it lasts.
CREATE TABLE central_identity.account_emails (
    account_id uuid PRIMARY KEY
        REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    address text NOT NULL CHECK (octet_length(address) BETWEEN 5 AND 254
        AND address ~ '^[!-?A-~]+@[A-Za-z0-9.-]+$'),
    lang text NOT NULL CHECK (lang ~ '^[a-z][a-z-]{0,8}[a-z]$'),
    confirmed_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    reset_digest bytea CHECK (octet_length(reset_digest) = 32),
    reset_mailed timestamptz[] NOT NULL DEFAULT '{}'
        CHECK (pg_catalog.cardinality(reset_mailed) <= 3)
);

-- account_email_pending: an address that waits for its code. Only the
-- digest of the code is kept (central_api::email::MailCode::digest, SHA-256
-- over a label, the account's id and the code's 40 bits), with its end, a
-- quarter of an hour after it was mailed, and how many wrong codes were
-- tried: the fifth wrong one deletes the row. A row past its end counts as
-- absent everywhere; the operator's daily sweep deletes it.
CREATE TABLE central_identity.account_email_pending (
    account_id uuid PRIMARY KEY
        REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    address text NOT NULL CHECK (octet_length(address) BETWEEN 5 AND 254
        AND address ~ '^[!-?A-~]+@[A-Za-z0-9.-]+$'),
    lang text NOT NULL CHECK (lang ~ '^[a-z][a-z-]{0,8}[a-z]$'),
    code_digest bytea NOT NULL CHECK (octet_length(code_digest) = 32),
    expires_at timestamptz NOT NULL,
    tries smallint NOT NULL DEFAULT 0 CHECK (tries BETWEEN 0 AND 4)
);
-- The sweep's read.
CREATE INDEX account_email_pending_expires
    ON central_identity.account_email_pending (expires_at);
