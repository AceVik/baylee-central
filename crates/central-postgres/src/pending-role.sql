-- Fixed adapter contract, checked at startup. Provisioning remains a trusted operator task.
-- Reject indirect SET ROLE paths, DDL/TEMP, mutation and privilege delegation.
SELECT NOT (rolsuper OR rolcreatedb OR rolcreaterole OR rolreplication OR rolbypassrls)
  AND NOT EXISTS (SELECT 1 FROM pg_catalog.pg_auth_members WHERE member = r.oid)
  AND NOT pg_catalog.has_database_privilege(current_user, current_database(), 'CREATE,TEMP')
  AND NOT EXISTS (
    SELECT 1 FROM pg_catalog.pg_namespace n
    WHERE pg_catalog.has_schema_privilege(current_user, n.oid, 'CREATE')
  )
  AND pg_catalog.has_schema_privilege(current_user, 'central_identity', 'USAGE')
  AND pg_catalog.has_table_privilege(current_user, 'central_identity.pending_accounts', 'INSERT')
  AND pg_catalog.has_table_privilege(current_user, 'central_identity.pending_accounts', 'SELECT')
  AND NOT pg_catalog.has_table_privilege(current_user, 'central_identity.pending_accounts',
    'UPDATE,DELETE,TRUNCATE,REFERENCES,TRIGGER,MAINTAIN,INSERT WITH GRANT OPTION,SELECT WITH GRANT OPTION')
  AND NOT pg_catalog.has_any_column_privilege(current_user, 'central_identity.pending_accounts',
    'UPDATE,REFERENCES,INSERT WITH GRANT OPTION,SELECT WITH GRANT OPTION')
  AND NOT EXISTS (
    SELECT 1 FROM pg_catalog.pg_class c JOIN pg_catalog.pg_namespace n ON c.relnamespace = n.oid
    WHERE n.nspname = 'central_identity' AND c.relowner = r.oid
  ) AS allowed
FROM pg_catalog.pg_roles r WHERE rolname = current_user
