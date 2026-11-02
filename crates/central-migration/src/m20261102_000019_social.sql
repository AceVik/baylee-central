-- Canonical pairs serialize both directions without duplicate friendships.
CREATE TABLE central_identity.social_contacts (
    low uuid NOT NULL REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    high uuid NOT NULL REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    requested_by uuid NOT NULL REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    accepted boolean NOT NULL DEFAULT false,
    created_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    PRIMARY KEY (low, high),
    CHECK (low < high),
    CHECK (requested_by = low OR requested_by = high)
);
CREATE INDEX social_contacts_high ON central_identity.social_contacts(high, low);
CREATE TABLE central_identity.social_blocks (
    owner uuid NOT NULL REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    blocked uuid NOT NULL REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    PRIMARY KEY (owner, blocked),
    CHECK (owner <> blocked)
);
CREATE INDEX social_blocks_other ON central_identity.social_blocks(blocked, owner);
-- Fixed-size per-account budgets survive restarts and bound request churn.
CREATE TABLE central_identity.social_budgets (
    account_id uuid PRIMARY KEY REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    window_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    requests integer NOT NULL DEFAULT 0 CHECK (requests BETWEEN 0 AND 60),
    lookups integer NOT NULL DEFAULT 0 CHECK (lookups BETWEEN 0 AND 120)
);
