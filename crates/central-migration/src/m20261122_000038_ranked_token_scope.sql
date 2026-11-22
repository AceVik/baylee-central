-- Existing scoped credentials retain exactly the privileges their owner granted.
ALTER TABLE central_identity.gateway_tokens
    ADD COLUMN ranked boolean NOT NULL DEFAULT false,
    DROP CONSTRAINT gateway_token_scope,
    ADD CONSTRAINT gateway_token_scope CHECK (sso OR listing OR ranked);
