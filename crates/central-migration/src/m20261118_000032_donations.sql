-- What a donation leaves behind at Central (docs/operations.md §"Donations";
-- the owner's order of 18.11.2026: the three platforms added up in euros).
-- GitHub Sponsors' one-time sponsorships and their refunds, read from GitHub
-- with the sponsored account's token, and Ko-fi's payments, delivered to
-- Central's webhook. Liberapay's figure and GitHub's monthly estimate are
-- read fresh and never stored.
--
-- A row says which platform, whether the money was given once, is a regular
-- payment or went back, the amount in hundredths of its currency, that
-- currency, the amount in euro cents at the reference rate Central held when
-- it arrived (NULL while it held none: such a row is not counted until a rate
-- comes), when it arrived by Central's clock, and a SHA-256 digest of the
-- platform's own transaction id, so that a delivery that comes twice is kept
-- once. Nothing about the giver: no name, address, message, account or id a
-- reader could look up at the platform.
--
-- Rows are kept 40 days: long enough for Ko-fi's monthly window (31 days) and
-- for the window of GitHub's activities that is read again (30 days), so that
-- nothing swept can come back as new. The operator's daily sweep then adds
-- what was given once and what went back to one sum per platform and deletes
-- the rows; a regular payment past the window counts for nothing and is
-- deleted with them. What stays for good is two sums and two counts per
-- platform, and the day the count began.
CREATE TABLE central_identity.donations (
    id uuid PRIMARY KEY DEFAULT pg_catalog.uuidv7()
        CHECK (pg_catalog.uuid_extract_version(id) IS NOT DISTINCT FROM 7),
    platform text NOT NULL CHECK (platform IN ('github', 'kofi')),
    kind text NOT NULL CHECK (kind IN ('once', 'recurring', 'refund')),
    amount bigint NOT NULL CHECK (amount BETWEEN 1 AND 99999999999),
    currency text NOT NULL CHECK (currency COLLATE "C" ~ '^[A-Z]{3}$'),
    euro_cents bigint CHECK (euro_cents BETWEEN 0 AND 10000000),
    received_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    reference bytea NOT NULL CHECK (pg_catalog.octet_length(reference) = 32),
    UNIQUE (platform, reference),
    -- What each platform reports: GitHub's regular giving is its estimate,
    -- read fresh; Ko-fi reports no refund.
    CHECK (kind <> 'recurring' OR platform = 'kofi'),
    CHECK (kind <> 'refund' OR platform = 'github')
);
-- The window's sums and the sweep read by the time a row arrived.
CREATE INDEX donations_received ON central_identity.donations (received_at);

-- What the sweep leaves of the rows it deletes: per platform, the sum and
-- count of what was given once and of what went back.
CREATE TABLE central_identity.donation_sums (
    platform text NOT NULL CHECK (platform IN ('github', 'kofi')),
    kind text NOT NULL CHECK (kind IN ('once', 'refund')),
    euro_cents bigint NOT NULL CHECK (euro_cents >= 0),
    count bigint NOT NULL CHECK (count >= 0),
    PRIMARY KEY (platform, kind)
);

-- The day the count began, the one day the public figures name: the day
-- this migration ran, in UTC. Never the day of a donation.
CREATE TABLE central_identity.donation_count (
    only_row boolean PRIMARY KEY DEFAULT true CHECK (only_row),
    since date NOT NULL
);
INSERT INTO central_identity.donation_count (since)
    VALUES ((pg_catalog.clock_timestamp() AT TIME ZONE 'UTC')::date);
