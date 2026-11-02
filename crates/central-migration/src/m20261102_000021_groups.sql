CREATE TABLE central_identity.chat_groups (
    id uuid PRIMARY KEY DEFAULT pg_catalog.uuidv7() CHECK (pg_catalog.uuid_extract_version(id) IS NOT DISTINCT FROM 7),
    owner uuid NOT NULL REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    name text NOT NULL CHECK (length(name) BETWEEN 1 AND 48),
    created_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp()
);
CREATE INDEX chat_groups_owner ON central_identity.chat_groups(owner);
CREATE TABLE central_identity.chat_members (
    id uuid PRIMARY KEY DEFAULT pg_catalog.uuidv7() CHECK (pg_catalog.uuid_extract_version(id) IS NOT DISTINCT FROM 7),
    group_id uuid NOT NULL REFERENCES central_identity.chat_groups(id) ON DELETE CASCADE,
    account_id uuid NOT NULL REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    accepted boolean NOT NULL DEFAULT false,
    inviter uuid NOT NULL REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    created_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    UNIQUE (group_id, account_id)
);
CREATE INDEX chat_members_account ON central_identity.chat_members(account_id, group_id);
