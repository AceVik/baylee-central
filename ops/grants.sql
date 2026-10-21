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
GRANT SELECT ON central_identity.gateway_approvals TO central_runtime;
GRANT SELECT, INSERT, DELETE ON central_identity.gateway_admins, central_identity.gateway_leases TO central_runtime;
GRANT SELECT, INSERT ON central_identity.gateway_terms, central_identity.gateway_terms_texts,
  central_identity.terms_acceptances TO central_runtime;
