-- Terms of use (docs/terms.md). A gateway's published versions and every
-- acceptance are immutable records: the runtime role may only SELECT and
-- INSERT them, and an account's acceptances go with the account.
CREATE TABLE central_identity.gateway_terms (
    audience text NOT NULL CHECK (octet_length(audience) BETWEEN 1 AND 256),
    version text NOT NULL CHECK (version ~ '^[!-~]{1,64}$' AND version <> 'draft'),
    digest bytea NOT NULL CHECK (octet_length(digest) = 32),
    published_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    PRIMARY KEY (audience, version)
);
CREATE INDEX gateway_terms_current ON central_identity.gateway_terms(audience, published_at DESC, version DESC);

CREATE TABLE central_identity.gateway_terms_texts (
    audience text NOT NULL,
    version text NOT NULL,
    lang text NOT NULL CHECK (lang ~ '^[a-z]{2,3}$'),
    updated text CHECK (updated ~ '^[!-~]{1,32}$'),
    markdown text NOT NULL CHECK (octet_length(markdown) BETWEEN 1 AND 65536),
    PRIMARY KEY (audience, version, lang),
    FOREIGN KEY (audience, version) REFERENCES central_identity.gateway_terms(audience, version) ON DELETE CASCADE
);

CREATE TABLE central_identity.terms_acceptances (
    account_id uuid NOT NULL REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    -- 'central', or the gateway's canonical origin.
    subject text NOT NULL CHECK (octet_length(subject) BETWEEN 1 AND 256),
    version text NOT NULL CHECK (version ~ '^[!-~]{1,64}$' AND version <> 'draft'),
    digest bytea NOT NULL CHECK (octet_length(digest) = 32),
    accepted_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    PRIMARY KEY (account_id, subject, version, digest)
);
REVOKE ALL ON central_identity.gateway_terms, central_identity.gateway_terms_texts,
    central_identity.terms_acceptances FROM PUBLIC;
