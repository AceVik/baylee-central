-- Single-use 128-bit friend invite codes: only domain-separated SHA-256
-- digests are stored, never plaintext invite secrets.

CREATE TABLE central_identity.friend_invites (
    id                 uuid PRIMARY KEY DEFAULT pg_catalog.uuidv7(),
    inviter_id         uuid NOT NULL REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    code_hash          bytea NOT NULL UNIQUE,
    label              text,
    note               text,
    invited_email      text,
    lang               text,
    created_at         timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    expires_at         timestamptz NOT NULL,
    pending_account_id uuid REFERENCES central_identity.accounts(id) ON DELETE SET NULL,
    pending_until      timestamptz,
    redeemed_by        uuid REFERENCES central_identity.accounts(id) ON DELETE SET NULL,
    redeemed_at        timestamptz,
    revoked_at         timestamptz,

    CONSTRAINT friend_invites_id_v7 CHECK (pg_catalog.uuid_extract_version(id) = 7),
    CONSTRAINT friend_invites_code_hash_len CHECK (pg_catalog.octet_length(code_hash) = 32),
    CONSTRAINT friend_invites_label_len CHECK (label IS NULL OR pg_catalog.char_length(label) BETWEEN 1 AND 64),
    CONSTRAINT friend_invites_note_len CHECK (note IS NULL OR pg_catalog.char_length(note) BETWEEN 1 AND 280),
    CONSTRAINT friend_invites_email_len CHECK (invited_email IS NULL OR pg_catalog.octet_length(invited_email) BETWEEN 3 AND 254),
    CONSTRAINT friend_invites_lang_len CHECK (lang IS NULL OR pg_catalog.char_length(lang) BETWEEN 2 AND 16),
    CONSTRAINT friend_invites_expires_after_created CHECK (expires_at > created_at),
    CONSTRAINT friend_invites_pending_until_set CHECK (pending_account_id IS NULL OR pending_until IS NOT NULL),
    CONSTRAINT friend_invites_redeemed_at_set CHECK (redeemed_by IS NULL OR redeemed_at IS NOT NULL),
    CONSTRAINT friend_invites_state_exclusive CHECK (redeemed_at IS NULL OR revoked_at IS NULL),
    CONSTRAINT friend_invites_not_self_redeemed CHECK (redeemed_by IS NULL OR redeemed_by <> inviter_id),
    CONSTRAINT friend_invites_not_self_pending CHECK (pending_account_id IS NULL OR pending_account_id <> inviter_id)
);

CREATE INDEX friend_invites_inviter_created_idx
    ON central_identity.friend_invites (inviter_id, created_at DESC, id DESC);

CREATE INDEX friend_invites_pending_account_idx
    ON central_identity.friend_invites (pending_account_id)
    WHERE pending_account_id IS NOT NULL;

CREATE INDEX friend_invites_expires_idx
    ON central_identity.friend_invites (expires_at)
    WHERE redeemed_at IS NULL AND revoked_at IS NULL;

REVOKE ALL ON TABLE central_identity.friend_invites FROM PUBLIC;
