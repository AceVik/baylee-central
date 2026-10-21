//! Exact runtime privileges: approving origins, rewriting archives and issuing or
//! revoking beta keys remain operator-only; the runtime only spends a key's uses.
use super::{AccountStore, statement};
use sea_orm::ConnectionTrait;

impl AccountStore {
    pub(super) async fn valid_grants(&self) -> bool {
        self.db.query_one_raw(statement(
            "WITH required(name, privileges, updatable) AS (VALUES
             ('accounts', ARRAY['SELECT','INSERT','UPDATE','DELETE'], NULL::text),
             ('sessions', ARRAY['SELECT','INSERT','UPDATE','DELETE'], NULL),
             ('gateway_tickets', ARRAY['SELECT','INSERT','UPDATE','DELETE'], NULL),
             ('private_objects', ARRAY['SELECT','INSERT','UPDATE','DELETE'], NULL),
             ('gateways', ARRAY['SELECT','INSERT','UPDATE'], NULL),
             ('gateway_admins', ARRAY['SELECT','INSERT','DELETE'], NULL),
             ('gateway_leases', ARRAY['SELECT','INSERT','DELETE'], NULL),
             ('gateway_approvals', ARRAY['SELECT'], NULL),
             ('migration_archives', ARRAY['SELECT','INSERT'], NULL),
             ('gateway_terms', ARRAY['SELECT','INSERT'], NULL),
             ('gateway_terms_texts', ARRAY['SELECT','INSERT'], NULL),
             ('terms_acceptances', ARRAY['SELECT','INSERT'], NULL),
             ('beta_keys', ARRAY['SELECT'], 'uses_left'))
             SELECT count(*) = 13 AND bool_and(
               NOT pg_catalog.pg_has_role(c.relowner, 'USAGE')
               AND (SELECT bool_and(pg_catalog.has_table_privilege(c.oid, p)) FROM unnest(r.privileges) p)
               AND NOT pg_catalog.has_table_privilege(c.oid, 'TRUNCATE, REFERENCES, TRIGGER, MAINTAIN, SELECT WITH GRANT OPTION, INSERT WITH GRANT OPTION, UPDATE WITH GRANT OPTION, DELETE WITH GRANT OPTION')
               AND NOT pg_catalog.has_any_column_privilege(c.oid, 'SELECT WITH GRANT OPTION, INSERT WITH GRANT OPTION, UPDATE WITH GRANT OPTION, REFERENCES')
               AND NOT EXISTS (SELECT 1 FROM unnest(ARRAY['INSERT','UPDATE','DELETE']) p
                 WHERE NOT p = ANY(r.privileges) AND pg_catalog.has_table_privilege(c.oid, p))
               AND NOT EXISTS (SELECT 1 FROM unnest(ARRAY['INSERT','UPDATE']) p
                 WHERE NOT p = ANY(r.privileges) AND (p = 'INSERT' OR r.updatable IS NULL)
                 AND pg_catalog.has_any_column_privilege(c.oid, p))
               -- A table with one updatable column: UPDATE on exactly that column.
               AND (r.updatable IS NULL OR (pg_catalog.has_column_privilege(c.oid, r.updatable, 'UPDATE')
                 AND NOT EXISTS (SELECT 1 FROM pg_catalog.pg_attribute a WHERE a.attrelid = c.oid
                   AND a.attnum > 0 AND NOT a.attisdropped AND a.attname <> r.updatable
                   AND pg_catalog.has_column_privilege(c.oid, a.attnum, 'UPDATE'))))
             ) AS allowed FROM required r JOIN pg_catalog.pg_class c ON c.relname = r.name
             JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace WHERE n.nspname = 'central_identity'",
            vec![],
        )).await.ok().flatten().and_then(|row| row.try_get::<bool>("", "allowed").ok()) == Some(true)
    }
}
