//! Exact runtime privileges: approving origins and rewriting archives remain operator-only.
use super::{AccountStore, statement};
use sea_orm::ConnectionTrait;

impl AccountStore {
    pub(super) async fn valid_grants(&self) -> bool {
        self.db.query_one_raw(statement(
            "WITH required(name, privileges) AS (VALUES
             ('accounts', ARRAY['SELECT','INSERT','UPDATE','DELETE']),
             ('sessions', ARRAY['SELECT','INSERT','UPDATE','DELETE']),
             ('gateway_tickets', ARRAY['SELECT','INSERT','UPDATE','DELETE']),
             ('private_objects', ARRAY['SELECT','INSERT','UPDATE','DELETE']),
             ('gateways', ARRAY['SELECT','INSERT','UPDATE']),
             ('gateway_admins', ARRAY['SELECT','INSERT','DELETE']),
             ('gateway_leases', ARRAY['SELECT','INSERT','DELETE']),
             ('gateway_approvals', ARRAY['SELECT']),
             ('migration_archives', ARRAY['SELECT','INSERT']))
             SELECT count(*) = 9 AND bool_and(
               NOT pg_catalog.pg_has_role(c.relowner, 'USAGE')
               AND (SELECT bool_and(pg_catalog.has_table_privilege(c.oid, p)) FROM unnest(r.privileges) p)
               AND NOT pg_catalog.has_table_privilege(c.oid, 'TRUNCATE, REFERENCES, TRIGGER, MAINTAIN, SELECT WITH GRANT OPTION, INSERT WITH GRANT OPTION, UPDATE WITH GRANT OPTION, DELETE WITH GRANT OPTION')
               AND NOT pg_catalog.has_any_column_privilege(c.oid, 'SELECT WITH GRANT OPTION, INSERT WITH GRANT OPTION, UPDATE WITH GRANT OPTION, REFERENCES')
               AND NOT EXISTS (SELECT 1 FROM unnest(ARRAY['INSERT','UPDATE','DELETE']) p
                 WHERE NOT p = ANY(r.privileges) AND pg_catalog.has_table_privilege(c.oid, p))
               AND NOT EXISTS (SELECT 1 FROM unnest(ARRAY['INSERT','UPDATE']) p
                 WHERE NOT p = ANY(r.privileges) AND pg_catalog.has_any_column_privilege(c.oid, p))
             ) AS allowed FROM required r JOIN pg_catalog.pg_class c ON c.relname = r.name
             JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace WHERE n.nspname = 'central_identity'",
            vec![],
        )).await.ok().flatten().and_then(|row| row.try_get::<bool>("", "allowed").ok()) == Some(true)
    }
}
