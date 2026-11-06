-- Session tokens (docs/gateway-registry.md §"Session tokens"): Central signs a token for
-- one gateway, which that gateway checks by itself. Tickets and leases go.

DROP TABLE central_identity.gateway_leases;
DROP TABLE central_identity.gateway_tickets;

-- A native session's bearer is signed now, so no bearer of the old form opens anything:
-- those sessions end here rather than linger until they expire.
DELETE FROM central_identity.sessions WHERE kind = 'native';

-- One revocation stream per gateway, under its canonical origin: the official gateway
-- has one whether or not its row is listed. The numbers have no holes: `last_seq` moves
-- in the transaction that ends the session, and rolls back with it.
CREATE TABLE central_identity.gateway_streams (
    origin text PRIMARY KEY CHECK (octet_length(origin) BETWEEN 1 AND 256),
    last_seq bigint NOT NULL DEFAULT 0 CHECK (last_seq >= 0),
    -- The push queue is the log below and this cursor, so it holds no more than the log.
    pushed_seq bigint NOT NULL DEFAULT 0 CHECK (pushed_seq >= 0 AND pushed_seq <= last_seq),
    -- A path and never a URL: scheme, host and port come from the approved origin.
    invalidate_path text CHECK (octet_length(invalidate_path) <= 128
        AND invalidate_path ~ '^(/[A-Za-z0-9_-]+)+$'),
    attempts smallint NOT NULL DEFAULT 0 CHECK (attempts BETWEEN 0 AND 5),
    next_attempt_at timestamptz
);
CREATE INDEX gateway_streams_due ON central_identity.gateway_streams(next_attempt_at)
    WHERE pushed_seq < last_seq AND invalidate_path IS NOT NULL;

-- A session as one gateway knows it. `sid` is random and differs per gateway and
-- purpose, so two gateways cannot tell they saw the same session.
CREATE TABLE central_identity.gateway_sids (
    sid bytea PRIMARY KEY CHECK (octet_length(sid) = 16),
    session_digest bytea NOT NULL
        REFERENCES central_identity.sessions(digest) ON DELETE CASCADE,
    stream text NOT NULL REFERENCES central_identity.gateway_streams(origin),
    purpose text NOT NULL CHECK (purpose IN ('admit', 'admin')),
    created_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    UNIQUE (session_digest, stream, purpose)
);
CREATE INDEX gateway_sids_stream ON central_identity.gateway_sids(stream, purpose);

-- What each gateway must learn. Kept for two hours by age alone: longer than any token
-- lives, so a gateway reading from 0 after a start misses none that is still valid.
CREATE TABLE central_identity.gateway_revocations (
    stream text NOT NULL REFERENCES central_identity.gateway_streams(origin),
    seq bigint NOT NULL CHECK (seq >= 1),
    sid bytea NOT NULL CHECK (octet_length(sid) = 16),
    revoked_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    PRIMARY KEY (stream, seq)
);
CREATE INDEX gateway_revocations_age ON central_identity.gateway_revocations(revoked_at);

REVOKE ALL ON central_identity.gateway_streams, central_identity.gateway_sids,
    central_identity.gateway_revocations FROM PUBLIC;

-- A session may hold `sid`s at several gateways. Taking their streams in one order, before
-- any is touched, keeps two ending sessions from deadlocking; NO KEY leaves a token being
-- issued for that gateway unblocked.
CREATE FUNCTION central_identity.lock_session_streams() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog AS $$
BEGIN
    PERFORM 1 FROM central_identity.gateway_streams t
    WHERE t.origin IN (SELECT i.stream FROM central_identity.gateway_sids i
                       WHERE i.session_digest = OLD.digest)
    ORDER BY t.origin FOR NO KEY UPDATE;
    RETURN OLD;
END;
$$;
REVOKE ALL ON FUNCTION central_identity.lock_session_streams() FROM PUBLIC;
CREATE TRIGGER session_streams BEFORE DELETE ON central_identity.sessions
    FOR EACH ROW EXECUTE FUNCTION central_identity.lock_session_streams();

-- Every end of a `sid` is a revocation, whichever statement ended it: sign-out, a
-- replacing sign-in, a password change, a reset, a deleted account, a removed
-- administrator, a disabled gateway. No path can forget to send one.
CREATE FUNCTION central_identity.revoke_sid() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog AS $$
DECLARE
    number bigint;
BEGIN
    UPDATE central_identity.gateway_streams SET last_seq = last_seq + 1
    WHERE origin = OLD.stream RETURNING last_seq INTO STRICT number;
    INSERT INTO central_identity.gateway_revocations(stream, seq, sid)
    VALUES (OLD.stream, number, OLD.sid);
    RETURN OLD;
END;
$$;
REVOKE ALL ON FUNCTION central_identity.revoke_sid() FROM PUBLIC;
CREATE TRIGGER sid_revoked AFTER DELETE ON central_identity.gateway_sids
    FOR EACH ROW EXECUTE FUNCTION central_identity.revoke_sid();
