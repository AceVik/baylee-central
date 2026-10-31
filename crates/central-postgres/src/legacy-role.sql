-- The legacy role's fixed contract, checked at startup (docs/operations.md
-- §"The legacy store"): no attribute, membership or schema creation; usage of
-- central_legacy and of no other Central schema; exactly SELECT and DELETE on
-- its accounts, UPDATE on their two lockout columns alone, SELECT on settings;
-- owning nothing. Provisioning stays a trusted operator task.
SELECT NOT (rolsuper OR rolcreatedb OR rolcreaterole OR rolreplication OR rolbypassrls)
  AND NOT EXISTS (SELECT 1 FROM pg_catalog.pg_auth_members WHERE member = r.oid)
  AND NOT pg_catalog.has_database_privilege(current_user, current_database(), 'CREATE,TEMP')
  AND NOT EXISTS (
    SELECT 1 FROM pg_catalog.pg_namespace n
    WHERE pg_catalog.has_schema_privilege(current_user, n.oid, 'CREATE')
  )
  AND pg_catalog.has_schema_privilege(current_user, 'central_legacy', 'USAGE')
  AND NOT pg_catalog.has_schema_privilege(current_user, 'central_identity', 'USAGE')
  AND pg_catalog.has_table_privilege(current_user, 'central_legacy.accounts', 'SELECT')
  AND pg_catalog.has_table_privilege(current_user, 'central_legacy.accounts', 'DELETE')
  AND NOT pg_catalog.has_table_privilege(current_user, 'central_legacy.accounts',
    'INSERT,UPDATE,TRUNCATE,REFERENCES,TRIGGER,MAINTAIN,SELECT WITH GRANT OPTION,DELETE WITH GRANT OPTION')
  AND NOT pg_catalog.has_any_column_privilege(current_user, 'central_legacy.accounts',
    'INSERT,REFERENCES,UPDATE WITH GRANT OPTION')
  AND pg_catalog.has_column_privilege(current_user, 'central_legacy.accounts', 'failures', 'UPDATE')
  AND pg_catalog.has_column_privilege(current_user, 'central_legacy.accounts', 'locked_until', 'UPDATE')
  AND NOT EXISTS (
    SELECT 1 FROM pg_catalog.pg_attribute a
    WHERE a.attrelid = 'central_legacy.accounts'::regclass AND a.attnum > 0 AND NOT a.attisdropped
      AND a.attname NOT IN ('failures', 'locked_until')
      AND pg_catalog.has_column_privilege(current_user, a.attrelid, a.attnum, 'UPDATE')
  )
  AND pg_catalog.has_table_privilege(current_user, 'central_legacy.settings', 'SELECT')
  AND NOT pg_catalog.has_table_privilege(current_user, 'central_legacy.settings',
    'INSERT,UPDATE,DELETE,TRUNCATE,REFERENCES,TRIGGER,MAINTAIN,SELECT WITH GRANT OPTION')
  AND NOT pg_catalog.has_any_column_privilege(current_user, 'central_legacy.settings',
    'INSERT,UPDATE,REFERENCES')
  AND NOT EXISTS (
    SELECT 1 FROM pg_catalog.pg_class c JOIN pg_catalog.pg_namespace n ON c.relnamespace = n.oid
    WHERE n.nspname IN ('central_legacy', 'central_identity') AND c.relowner = r.oid
  ) AS allowed
FROM pg_catalog.pg_roles r WHERE rolname = current_user
