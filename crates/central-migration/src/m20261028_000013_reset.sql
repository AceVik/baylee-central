-- Reset codes (docs/account-reset.md): an administrator's one-use permission
-- to set a new password for an enrolled account. Only the SHA-256 digest of the
-- code's 160 bits is kept, with its expiry, 24 hours after it was made; a new
-- code replaces the previous one, and using it or changing the password clears
-- it. The administrator who made it is in the audit.

ALTER TABLE central_identity.accounts
    ADD COLUMN reset_digest bytea CHECK (octet_length(reset_digest) = 32),
    ADD COLUMN reset_expires timestamptz;
ALTER TABLE central_identity.accounts ADD CONSTRAINT account_reset_code
    CHECK ((reset_digest IS NULL) = (reset_expires IS NULL)
        AND (reset_digest IS NULL OR credential IS NOT NULL));
