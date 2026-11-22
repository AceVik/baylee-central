-- Career progress is carried forward on a season's first settled game. Each
-- previous row remains an immutable record of the season it was earned in.
CREATE TABLE central_identity.ranked_ratings (
    season_id uuid NOT NULL REFERENCES central_identity.ranked_seasons(id),
    format text NOT NULL CHECK (format IN ('standard','highlander','commander')),
    account_id uuid NOT NULL REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    elo integer NOT NULL DEFAULT 1500,
    queue_elo integer NOT NULL DEFAULT 1500,
    score double precision NOT NULL DEFAULT 1500,
    placement double precision NOT NULL DEFAULT 1500,
    allowance double precision NOT NULL DEFAULT 0 CHECK (abs(allowance) <= 64.00000001),
    games integer NOT NULL DEFAULT 0 CHECK (games >= 0),
    wins integer NOT NULL DEFAULT 0 CHECK (wins >= 0),
    losses integer NOT NULL DEFAULT 0 CHECK (losses >= 0),
    draws integer NOT NULL DEFAULT 0 CHECK (draws >= 0),
    league smallint NOT NULL DEFAULT 0 CHECK (league BETWEEN 0 AND 9),
    division smallint NOT NULL DEFAULT 0 CHECK (division BETWEEN 0 AND 4),
    first_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
    last_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
    PRIMARY KEY (season_id,format,account_id),
    CHECK (wins+losses+draws=games)
);
CREATE INDEX ranked_career ON central_identity.ranked_ratings(account_id,format,last_at DESC);
CREATE INDEX ranked_player_board ON central_identity.ranked_ratings
    (season_id,format,league,division,elo DESC,account_id);
CREATE INDEX ranked_contenders ON central_identity.ranked_ratings
    (season_id,elo DESC,account_id) WHERE elo >= 2490 AND games >= 3000;

-- The private pool is a canonical multiset of rules identities. A file name,
-- publishing a copy, a printing or a change of sideboard cannot fork a score.
CREATE TABLE central_identity.ranked_deck_versions (
    account_id uuid NOT NULL REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    format text NOT NULL CHECK (format IN ('standard','highlander','commander')),
    version bytea NOT NULL CHECK (octet_length(version)=32),
    cards jsonb NOT NULL CHECK (jsonb_typeof(cards)='object' AND octet_length(cards::text)<=81920),
    copies integer NOT NULL CHECK (copies BETWEEN 1 AND 1024),
    elo integer NOT NULL CHECK (elo >= 150),
    anchor integer CHECK (anchor >= 150),
    initial_pool boolean NOT NULL,
    first_season uuid NOT NULL REFERENCES central_identity.ranked_seasons(id),
    CHECK (NOT initial_pool OR anchor IS NOT NULL),
    PRIMARY KEY(account_id,format,version)
);
CREATE UNIQUE INDEX ranked_initial_pool ON central_identity.ranked_deck_versions(account_id,format)
    WHERE initial_pool;
CREATE INDEX ranked_deck_anchors ON central_identity.ranked_deck_versions(account_id,format)
    WHERE anchor IS NOT NULL;
CREATE TABLE central_identity.ranked_deck_ratings (
    season_id uuid NOT NULL REFERENCES central_identity.ranked_seasons(id),
    account_id uuid NOT NULL,
    format text NOT NULL,
    version bytea NOT NULL,
    elo integer NOT NULL CHECK (elo >= 150),
    games integer NOT NULL DEFAULT 0 CHECK (games >= 0),
    season_games integer NOT NULL DEFAULT 0 CHECK (season_games >= 0 AND season_games <= games),
    wins integer NOT NULL DEFAULT 0 CHECK (wins >= 0),
    losses integer NOT NULL DEFAULT 0 CHECK (losses >= 0),
    draws integer NOT NULL DEFAULT 0 CHECK (draws >= 0),
    league smallint NOT NULL DEFAULT 0 CHECK (league BETWEEN 0 AND 9),
    division smallint NOT NULL DEFAULT 0 CHECK (division BETWEEN 0 AND 4),
    first_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
    last_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
    PRIMARY KEY (season_id,format,account_id,version),
    FOREIGN KEY(account_id,format,version) REFERENCES central_identity.ranked_deck_versions
        (account_id,format,version) ON DELETE CASCADE,
    CHECK (wins+losses+draws=games)
);
CREATE INDEX ranked_deck_career ON central_identity.ranked_deck_ratings(account_id,format,version,last_at DESC);
CREATE INDEX ranked_deck_board ON central_identity.ranked_deck_ratings
    (season_id,format,league,division,elo DESC,account_id,version);

-- Two receipts commit with both accounts and both deck versions. The two
-- indexes bound the independent-opponent evidence and the rematch count.
CREATE TABLE central_identity.ranked_receipts (
    match_id uuid NOT NULL REFERENCES central_identity.ranked_matches(id) ON DELETE CASCADE,
    account_id uuid NOT NULL REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    format text NOT NULL,
    opponent_id uuid NOT NULL REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    settled_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
    opponent_established boolean NOT NULL,
    elo_gap integer NOT NULL,
    outcome smallint NOT NULL CHECK (outcome IN (-1,0,1)),
    elo integer NOT NULL,
    delta integer NOT NULL,
    before_rank jsonb NOT NULL,
    after_rank jsonb NOT NULL,
    deck_result jsonb NOT NULL,
    PRIMARY KEY(match_id,account_id),
    CHECK (account_id <> opponent_id)
);
CREATE INDEX ranked_evidence ON central_identity.ranked_receipts
    (account_id,format,settled_at DESC,match_id DESC);
CREATE INDEX ranked_repeat ON central_identity.ranked_receipts
    (account_id,opponent_id,settled_at DESC);
