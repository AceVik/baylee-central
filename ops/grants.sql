-- The runtime role's privileges, exactly those its startup check requires
-- (docs/operations.md §"Provisioning"). Idempotent: the deploy hook
-- (ops/deploy-hook) runs it after every migration. A migration that adds a
-- table adds its grant here.
GRANT USAGE ON SCHEMA central_identity TO central_runtime;
GRANT SELECT, INSERT ON central_identity.pending_accounts TO central_runtime;
GRANT SELECT, INSERT, UPDATE, DELETE ON central_identity.accounts, central_identity.sessions,
  central_identity.gateway_tickets, central_identity.private_objects TO central_runtime;
GRANT SELECT, INSERT ON central_identity.migration_archives TO central_runtime;
GRANT SELECT, INSERT, UPDATE ON central_identity.gateways TO central_runtime;
-- Administrators approve, enable and disable gateways on the website
-- (docs/administration.md §"Gateways"); an approval is never deleted.
GRANT SELECT, INSERT, UPDATE (enabled) ON central_identity.gateway_approvals TO central_runtime;
-- Gateway tokens (docs/gateway-registry.md §"Gateway tokens"): stored by
-- digest; afterwards only their revocation and last use change.
GRANT SELECT, INSERT, UPDATE (revoked_at, last_used_at) ON central_identity.gateway_tokens TO central_runtime;
GRANT SELECT, INSERT, DELETE ON central_identity.gateway_admins, central_identity.gateway_leases TO central_runtime;
-- A renewed session moves its leases' expiry with it, and nothing else of them.
GRANT UPDATE (expires_at) ON central_identity.gateway_leases TO central_runtime;
GRANT SELECT, INSERT ON central_identity.gateway_terms, central_identity.gateway_terms_texts,
  central_identity.terms_acceptances TO central_runtime;
-- A key's uses are spent by enrollment; issuing and revoking keys stay with the operator.
GRANT SELECT, UPDATE (uses_left) ON central_identity.beta_keys TO central_runtime;
-- Administrators' decisions are the runtime's (docs/administration.md): it
-- grants and removes roles, and writes the audit, which it can neither rewrite
-- nor delete; the operator's sweep deletes events after three years.
GRANT SELECT, INSERT, DELETE ON central_identity.account_roles TO central_runtime;
GRANT SELECT, INSERT ON central_identity.audit_events TO central_runtime;
-- Pairings (Baylee's deep links): the runtime starts, claims, approves and
-- redeems them, and deletes each when it is used or over.
GRANT SELECT, INSERT, UPDATE, DELETE ON central_identity.pairings TO central_runtime;
