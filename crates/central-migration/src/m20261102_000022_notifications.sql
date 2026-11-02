-- Push contains only generic category hints. No plaintext chat or report content.
CREATE TABLE central_identity.notification_preferences (
    account_id uuid PRIMARY KEY REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    enabled smallint NOT NULL DEFAULT 0 CHECK (enabled BETWEEN 0 AND 31)
);
CREATE TABLE central_identity.push_subscriptions (
    id uuid PRIMARY KEY DEFAULT pg_catalog.uuidv7() CHECK (pg_catalog.uuid_extract_version(id) IS NOT DISTINCT FROM 7),
    account_id uuid NOT NULL REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    credential_revision bigint NOT NULL,
    session_digest bytea NOT NULL CHECK (octet_length(session_digest) = 32),
    endpoint text NOT NULL UNIQUE CHECK (octet_length(endpoint) BETWEEN 1 AND 2048),
    p256dh text NOT NULL CHECK (octet_length(p256dh) = 87),
    auth text NOT NULL CHECK (octet_length(auth) = 22),
    nonce text NOT NULL CHECK (nonce ~ '^[0-9a-f]{32}$'),
    lang text NOT NULL CHECK (lang IN ('en','de')),
    renewed_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp()
);
CREATE INDEX push_subscriptions_account ON central_identity.push_subscriptions(account_id);
CREATE TABLE central_identity.push_jobs (
    id uuid PRIMARY KEY DEFAULT pg_catalog.uuidv7() CHECK (pg_catalog.uuid_extract_version(id) IS NOT DISTINCT FROM 7),
    subscription uuid NOT NULL REFERENCES central_identity.push_subscriptions(id) ON DELETE CASCADE,
    category smallint NOT NULL CHECK (category IN (1,2,4,8,16)),
    revision bigint NOT NULL DEFAULT 1 CHECK (revision > 0),
    attempts smallint NOT NULL DEFAULT 0 CHECK (attempts BETWEEN 0 AND 10),
    due_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    claimed_until timestamptz,
    lease uuid,
    expires_at timestamptz NOT NULL DEFAULT (pg_catalog.clock_timestamp() + interval '1 day'),
    UNIQUE(subscription,category)
);
CREATE INDEX push_jobs_expiry ON central_identity.push_jobs(expires_at);
CREATE INDEX push_jobs_due ON central_identity.push_jobs(due_at);
CREATE TABLE central_identity.push_releases (
    version text PRIMARY KEY CHECK (octet_length(version) BETWEEN 1 AND 100)
);
-- Security invoker, fixed search_path, fixed tables; PUBLIC receives no execution.
CREATE FUNCTION central_identity.queue_push(who uuid, kind smallint) RETURNS void
LANGUAGE sql SET search_path = pg_catalog AS $$
    INSERT INTO central_identity.push_jobs(subscription,category,due_at)
    SELECT s.id,kind,clock_timestamp() + interval '2 seconds'
    FROM central_identity.push_subscriptions s
    JOIN central_identity.accounts a ON a.id=s.account_id AND a.credential_revision=s.credential_revision
    JOIN central_identity.notification_preferences p ON p.account_id=a.id AND (p.enabled & kind)<>0
    WHERE (who IS NULL OR a.id=who) AND a.credential IS NOT NULL
      AND s.renewed_at>clock_timestamp()-interval '90 days'
      AND (kind<8 OR EXISTS(SELECT 1 FROM central_identity.account_roles r WHERE r.account_id=a.id AND r.role='admin'))
    ON CONFLICT(subscription,category) DO UPDATE SET
        revision=central_identity.push_jobs.revision+1,
        expires_at=clock_timestamp()+interval '1 day';
$$;
REVOKE ALL ON FUNCTION central_identity.queue_push(uuid,smallint) FROM PUBLIC;
CREATE FUNCTION central_identity.push_event() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog AS $$
DECLARE recipient uuid;
BEGIN
    IF TG_TABLE_NAME='chat_deliveries' THEN
        SELECT d.account_id INTO recipient FROM central_identity.chat_devices d
        JOIN central_identity.chat_messages m ON m.id=NEW.message_id
        WHERE d.fingerprint=NEW.device AND d.account_id<>m.sender;
        IF recipient IS NOT NULL THEN PERFORM central_identity.queue_push(recipient,1::smallint); END IF;
    ELSIF TG_TABLE_NAME='social_contacts' THEN
        IF NOT NEW.accepted THEN
            recipient:=CASE WHEN NEW.requested_by=NEW.low THEN NEW.high ELSE NEW.low END;
            PERFORM central_identity.queue_push(recipient,2::smallint);
        END IF;
    ELSIF TG_TABLE_NAME='reports' THEN PERFORM central_identity.queue_push(NULL,8::smallint);
    ELSIF TG_TABLE_NAME='gateways' THEN
        IF NOT NEW.official THEN PERFORM central_identity.queue_push(NULL,16::smallint); END IF;
    ELSIF TG_TABLE_NAME='push_releases' THEN PERFORM central_identity.queue_push(NULL,4::smallint);
    END IF;
    RETURN NEW;
END;
$$;
REVOKE ALL ON FUNCTION central_identity.push_event() FROM PUBLIC;
CREATE TRIGGER chat_push AFTER INSERT ON central_identity.chat_deliveries FOR EACH ROW EXECUTE FUNCTION central_identity.push_event();
CREATE TRIGGER contact_push AFTER INSERT ON central_identity.social_contacts FOR EACH ROW EXECUTE FUNCTION central_identity.push_event();
CREATE TRIGGER feedback_push AFTER INSERT ON central_identity.reports FOR EACH ROW EXECUTE FUNCTION central_identity.push_event();
CREATE TRIGGER gateway_push AFTER INSERT ON central_identity.gateways FOR EACH ROW EXECUTE FUNCTION central_identity.push_event();
CREATE TRIGGER release_push AFTER INSERT ON central_identity.push_releases FOR EACH ROW EXECUTE FUNCTION central_identity.push_event();
