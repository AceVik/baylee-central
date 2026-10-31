-- Gateway tokens (docs/gateway-registry.md §"Gateway tokens"): the
-- credential a self-hosted gateway pastes into its admin console, minted by
-- its owner on the website. Only the SHA-256 digest is stored. A token is
-- scoped (sso: its players' sign-ins; listing: its directory heartbeat),
-- named by its owner, expires within 366 days and can be revoked; it
-- authenticates as its gateway only while the gateway is approved.

-- A gateway registered on the website authenticates by tokens alone and has
-- no credential pair. Official rows still have none; a registered row has
-- both digests or neither. The previous binary's lookups (`credential_digest
-- = $1`, `heartbeat_digest = $1`) never match a NULL, so it runs unchanged
-- over rows without a pair.
ALTER TABLE central_identity.gateways DROP CONSTRAINT gateway_official;
ALTER TABLE central_identity.gateways ADD CONSTRAINT gateway_official CHECK (
    official = (owner_id IS NULL)
    AND (NOT official OR (credential_digest IS NULL AND heartbeat_digest IS NULL))
    AND (credential_digest IS NULL) = (heartbeat_digest IS NULL));

CREATE TABLE central_identity.gateway_tokens (
    id uuid PRIMARY KEY DEFAULT pg_catalog.uuidv7()
        CHECK (pg_catalog.uuid_extract_version(id) IS NOT DISTINCT FROM 7),
    gateway_id uuid NOT NULL REFERENCES central_identity.gateways(id) ON DELETE CASCADE,
    name text NOT NULL CHECK (octet_length(name) BETWEEN 1 AND 64),
    digest bytea NOT NULL UNIQUE CHECK (octet_length(digest) = 32),
    sso boolean NOT NULL,
    listing boolean NOT NULL,
    created_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    expires_at timestamptz NOT NULL,
    revoked_at timestamptz,
    last_used_at timestamptz,
    CONSTRAINT gateway_token_scope CHECK (sso OR listing),
    CONSTRAINT gateway_token_lifetime CHECK (
        expires_at > created_at AND expires_at <= created_at + interval '366 days'),
    CONSTRAINT gateway_token_revoked CHECK (revoked_at IS NULL OR revoked_at >= created_at)
);
CREATE INDEX gateway_tokens_gateway ON central_identity.gateway_tokens(gateway_id, id);
REVOKE ALL ON central_identity.gateway_tokens FROM PUBLIC;
