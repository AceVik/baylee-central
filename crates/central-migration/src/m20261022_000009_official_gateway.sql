-- The official gateway in the directory (docs/gateway-registry.md
-- §"Discovery"). It is the migration bridge's: it has no owner and no
-- credential, so the bridge stays the only authority for its origin, and no
-- registered credential can redeem tickets for that audience a second time.
-- Its heartbeat comes with the bridge credential and updates this row only.
-- The operator lists and approves it (docs/operations.md §"The official
-- gateway"); the runtime may insert it but, as for every gateway, not approve
-- it. Every other row keeps its owner and both credential digests.

ALTER TABLE central_identity.gateways ADD COLUMN official boolean NOT NULL DEFAULT false;
ALTER TABLE central_identity.gateways ALTER COLUMN owner_id DROP NOT NULL,
    ALTER COLUMN credential_digest DROP NOT NULL, ALTER COLUMN heartbeat_digest DROP NOT NULL;
ALTER TABLE central_identity.gateways ADD CONSTRAINT gateway_official CHECK (
    official = (owner_id IS NULL) AND official = (credential_digest IS NULL)
    AND official = (heartbeat_digest IS NULL));
CREATE UNIQUE INDEX gateways_one_official ON central_identity.gateways ((true)) WHERE official;
