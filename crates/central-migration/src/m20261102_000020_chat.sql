-- Public device fingerprints are protocol identifiers, not server object IDs.
CREATE TABLE central_identity.chat_devices (
    id uuid PRIMARY KEY DEFAULT pg_catalog.uuidv7() CHECK (pg_catalog.uuid_extract_version(id) IS NOT DISTINCT FROM 7),
    fingerprint text NOT NULL UNIQUE CHECK (fingerprint ~ '^[0-9a-f]{16}$'),
    account_id uuid NOT NULL REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    vault_epoch bigint NOT NULL CHECK (vault_epoch > 0),
    ik bytea NOT NULL CHECK (octet_length(ik) = 32),
    dk bytea NOT NULL CHECK (octet_length(dk) = 32),
    dsig bytea NOT NULL CHECK (octet_length(dsig) = 64),
    label text NOT NULL CHECK (length(label) BETWEEN 1 AND 24),
    created_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    UNIQUE (account_id, fingerprint)
);
CREATE INDEX chat_devices_account ON central_identity.chat_devices(account_id);
CREATE TABLE central_identity.chat_messages (
    id uuid PRIMARY KEY DEFAULT pg_catalog.uuidv7() CHECK (pg_catalog.uuid_extract_version(id) IS NOT DISTINCT FROM 7),
    sender uuid NOT NULL REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    correlation uuid NOT NULL CHECK (pg_catalog.uuid_extract_version(correlation) IS NOT DISTINCT FROM 7),
    device text NOT NULL REFERENCES central_identity.chat_devices(fingerprint) ON DELETE CASCADE,
    conversation text NOT NULL CHECK (octet_length(conversation) BETWEEN 1 AND 100),
    envelope bytea NOT NULL CHECK (octet_length(envelope) BETWEEN 1 AND 65536),
    created_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    UNIQUE (sender, correlation)
);
CREATE INDEX chat_messages_expiry ON central_identity.chat_messages(created_at);
CREATE TABLE central_identity.chat_deliveries (
    id uuid PRIMARY KEY DEFAULT pg_catalog.uuidv7() CHECK (pg_catalog.uuid_extract_version(id) IS NOT DISTINCT FROM 7),
    seq bigint GENERATED ALWAYS AS IDENTITY UNIQUE CHECK (seq BETWEEN 1 AND 9007199254740991),
    message_id uuid NOT NULL REFERENCES central_identity.chat_messages(id) ON DELETE CASCADE,
    device text NOT NULL REFERENCES central_identity.chat_devices(fingerprint) ON DELETE CASCADE,
    key_wrap bytea NOT NULL CHECK (octet_length(key_wrap) = 48),
    UNIQUE (device, message_id)
);
CREATE INDEX chat_deliveries_inbox ON central_identity.chat_deliveries(device, seq);
