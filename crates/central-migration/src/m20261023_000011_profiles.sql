-- Display names and tags at Central (docs/web-accounts.md §"Display name and
-- tag"): what other players see of an account, `Alice#0007`. The name keeps
-- the gateway's rule (central-api's `profile::valid_display_name`) and is not
-- unique; the tag is unique, positive, fits a gateway's signed 32-bit column,
-- and never changes once held. `display_name_at` is when the name was set.
--
-- Every registered account draws its tag here, at random from 0x1000 (4096,
-- central-api's `REGISTERED_TAG_FLOOR`) to 0xffff while fewer than half of
-- those are taken, then from sixteen times as wide, and so on; tags are
-- public, so `random()` is enough. Legacy accounts stay without one until the
-- bridge carries their official gateway's tag, which is below the floor.
-- `tag` stays nullable so that the previous binary, which the deploy hook
-- restarts when a new one fails to start, still registers (000007's rule);
-- the new binary draws a missing tag at the account's next sign-in.

ALTER TABLE central_identity.accounts
    ADD COLUMN display_name text
        CHECK (display_name ~ '^[A-Za-z0-9][A-Za-z0-9_-]{1,14}[A-Za-z0-9]$'),
    ADD COLUMN display_name_at timestamptz,
    ADD COLUMN tag integer UNIQUE CHECK (tag > 0);
ALTER TABLE central_identity.accounts ADD CONSTRAINT account_display_name_time
    CHECK ((display_name IS NULL) = (display_name_at IS NULL));

DO $$
DECLARE
    drawn_for uuid;
    low bigint;
    high bigint;
    candidate integer;
BEGIN
    FOR drawn_for IN SELECT id FROM central_identity.accounts
        WHERE source = 'registered' AND tag IS NULL ORDER BY id
    LOOP
        low := 4096;
        high := 65535;
        WHILE low <= 2147483647 AND (SELECT count(*) FROM central_identity.accounts
            WHERE tag BETWEEN low AND high) * 2 >= high - low + 1
        LOOP
            low := high + 1;
            high := least(low * 16 - 1, 2147483647);
        END LOOP;
        IF low > 2147483647 THEN
            RAISE EXCEPTION 'no tag is left to draw';
        END IF;
        LOOP
            candidate := (low + floor(random() * (high - low + 1)))::integer;
            EXIT WHEN NOT EXISTS (SELECT 1 FROM central_identity.accounts WHERE tag = candidate);
        END LOOP;
        UPDATE central_identity.accounts SET tag = candidate WHERE id = drawn_for;
    END LOOP;
END
$$;
