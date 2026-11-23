ALTER TABLE central_identity.gateway_sids ADD COLUMN ranked_generation bigint
    CHECK (ranked_generation IS NULL OR (purpose='ranked' AND ranked_generation>0));
ALTER TABLE central_identity.gateway_sids ADD COLUMN ranked_proof_until timestamptz
    CHECK (ranked_proof_until IS NULL OR purpose='ranked');
ALTER TABLE central_identity.ranked_tickets ADD COLUMN replicable boolean NOT NULL DEFAULT false;
UPDATE central_identity.ranked_tickets SET state='closed' WHERE state='waiting';
ALTER TABLE central_identity.ranked_hosts ADD COLUMN compatibility bytea
    CHECK (octet_length(compatibility)=32);

-- A row persists for every protected game, including paused and terminal games.
-- Its presence permanently disables the single-gateway settlement path.
CREATE TABLE central_identity.ranked_replicas (
    match_id uuid PRIMARY KEY REFERENCES central_identity.ranked_matches(id) ON DELETE CASCADE,
    compatibility bytea NOT NULL CHECK (octet_length(compatibility)=32),
    -- SHA-256 extracts 64 uniform seed bits from a cryptographically random UUIDv4.
    seed bytea DEFAULT substring(pg_catalog.sha256(pg_catalog.uuid_send(pg_catalog.gen_random_uuid())) FROM 1 FOR 8) CHECK (octet_length(seed)=8),
    generation bigint NOT NULL DEFAULT 1 CHECK (generation>0),
    standby_id uuid,
    standby_epoch bigint,
    CHECK ((standby_id IS NULL)=(standby_epoch IS NULL)),
    primary_until timestamptz NOT NULL DEFAULT (clock_timestamp()+interval '45 seconds'),
    standby_until timestamptz DEFAULT (clock_timestamp()+interval '45 seconds'),
    paused_until timestamptz,
    phase text NOT NULL DEFAULT 'preparing' CHECK (phase IN ('preparing','playing','paused','disputed','finished','void')),
    position bigint NOT NULL DEFAULT 0 CHECK (position BETWEEN 0 AND 4096),
    head_digest bytea NOT NULL DEFAULT decode(repeat('00',32),'hex') CHECK (octet_length(head_digest)=32),
    state_digest bytea CHECK (octet_length(state_digest)=32),
    opening_chain bytea CHECK (octet_length(opening_chain)=32),
    setup bytea CHECK (octet_length(setup) BETWEEN 1 AND 196608),
    initial_pools text CHECK (octet_length(initial_pools)<=131072),
    initial_evidence text CHECK (octet_length(initial_evidence)<=2048),
    evidence text CHECK (octet_length(evidence)<=2048),
    journal_bytes integer NOT NULL DEFAULT 0 CHECK (journal_bytes BETWEEN 0 AND 16777216),
    retire_at timestamptz,
    CHECK (seed IS NOT NULL OR phase IN ('finished','void'))
);
CREATE INDEX ranked_replicas_standby ON central_identity.ranked_replicas(standby_id,standby_until)
    WHERE phase NOT IN ('finished','void');
CREATE INDEX ranked_replicas_retire ON central_identity.ranked_replicas(retire_at) WHERE retire_at IS NOT NULL;
CREATE INDEX ranked_replicas_primary_expiry ON central_identity.ranked_replicas(primary_until)
    WHERE phase IN ('playing','preparing');
CREATE INDEX ranked_replicas_standby_expiry ON central_identity.ranked_replicas(standby_until)
    WHERE phase IN ('playing','preparing');
CREATE INDEX ranked_replicas_recovery_expiry ON central_identity.ranked_replicas(paused_until)
    WHERE phase IN ('paused','disputed');

CREATE TABLE central_identity.ranked_commands (
    match_id uuid NOT NULL REFERENCES central_identity.ranked_replicas(match_id) ON DELETE CASCADE,
    position bigint NOT NULL CHECK (position BETWEEN 1 AND 4096),
    generation bigint NOT NULL CHECK (generation>0),
    command_id uuid NOT NULL CHECK (uuid_extract_version(command_id) IS NOT DISTINCT FROM 7),
    source smallint NOT NULL CHECK (source BETWEEN 0 AND 2),
    previous_chain bytea NOT NULL CHECK (octet_length(previous_chain)=32),
    chain bytea NOT NULL CHECK (octet_length(chain)=32),
    body bytea NOT NULL CHECK (octet_length(body) BETWEEN 1 AND 32768),
    evidence text CHECK (octet_length(evidence)<=2048),
    PRIMARY KEY (match_id,position), UNIQUE (match_id,command_id)
);

CREATE TABLE central_identity.ranked_witnesses (
    match_id uuid NOT NULL REFERENCES central_identity.ranked_replicas(match_id) ON DELETE CASCADE,
    generation bigint NOT NULL CHECK (generation>0),
    position bigint NOT NULL CHECK (position BETWEEN 0 AND 4096),
    host_id uuid NOT NULL,
    chain bytea NOT NULL CHECK (octet_length(chain)=32),
    evidence text NOT NULL CHECK (octet_length(evidence) BETWEEN 1 AND 2048),
    PRIMARY KEY (match_id,generation,position,host_id)
);

-- A host can correlate a seat within its own match only. Central's account IDs
-- never leave the match ledger. Revocable JWT session capabilities are separate.
CREATE TABLE central_identity.ranked_host_aliases (
    match_id uuid NOT NULL REFERENCES central_identity.ranked_matches(id) ON DELETE CASCADE,
    host_id uuid NOT NULL REFERENCES central_identity.gateways(id) ON DELETE CASCADE,
    alias_a uuid NOT NULL DEFAULT uuidv7(),
    alias_b uuid NOT NULL DEFAULT uuidv7(),
    CHECK (alias_a <> alias_b),
    PRIMARY KEY (match_id, host_id)
);
-- Permission covers execution and verification for the explicitly shown recipient.
-- A verification-epoch or terms change invalidates it even if the origin is reused.
CREATE TABLE central_identity.ranked_host_permissions (
    match_id uuid NOT NULL REFERENCES central_identity.ranked_matches(id) ON DELETE CASCADE,
    host_id uuid NOT NULL REFERENCES central_identity.gateways(id) ON DELETE CASCADE,
    seat smallint NOT NULL CHECK (seat IN (0, 1)),
    epoch bigint NOT NULL CHECK (epoch >= 0),
    allowed boolean NOT NULL,
    origin text NOT NULL CHECK (octet_length(origin) BETWEEN 1 AND 256),
    version text,
    digest bytea,
    CHECK ((version IS NULL) = (digest IS NULL)),
    CHECK (octet_length(version) BETWEEN 1 AND 64 AND octet_length(digest) = 32),
    PRIMARY KEY (match_id, host_id, seat)
);
REVOKE ALL ON central_identity.ranked_host_aliases, central_identity.ranked_host_permissions FROM PUBLIC;

-- Erase entropy after the replay window. Even a runtime role with permission
-- to purge the column cannot choose or reroll a seed on an existing match.
CREATE OR REPLACE FUNCTION central_identity.ranked_seed_erasure() RETURNS trigger
LANGUAGE plpgsql SET search_path=pg_catalog AS $$
BEGIN
    IF NEW.seed IS NOT NULL OR OLD.phase NOT IN ('finished','void')
        OR OLD.retire_at IS NULL OR OLD.retire_at>clock_timestamp() THEN
        RAISE EXCEPTION USING ERRCODE='23514', MESSAGE='invalid ranked seed transition';
    END IF;
    RETURN NEW;
END
$$;
CREATE TRIGGER ranked_seed_erasure BEFORE UPDATE OF seed ON central_identity.ranked_replicas
FOR EACH ROW WHEN (OLD.seed IS DISTINCT FROM NEW.seed)
EXECUTE FUNCTION central_identity.ranked_seed_erasure();
