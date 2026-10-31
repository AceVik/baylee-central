//! Exact runtime privileges: rewriting archives and issuing or revoking beta
//! keys remain operator-only; the runtime only spends a key's uses and moves a
//! lease's expiry with its renewed session. It grants and removes
//! administrator roles, appends to the audit, which it cannot rewrite or
//! delete, and approves, enables and disables gateways as an administrator
//! decides (`docs/administration.md`), never deleting an approval. It stores
//! gateway tokens and may set only when one was revoked or last used. It
//! files reports and their parts once, triages a report by column and
//! deletes reports; it never changes a report's words, a part or its history.
use super::{AccountStore, statement};
use sea_orm::ConnectionTrait;

impl AccountStore {
    pub(super) async fn valid_grants(&self) -> bool {
        self.db.query_one_raw(statement(
            "WITH required(name, privileges, updatable) AS (VALUES
             ('accounts', ARRAY['SELECT','INSERT','UPDATE','DELETE'], NULL::text[]),
             ('sessions', ARRAY['SELECT','INSERT','UPDATE','DELETE'], NULL),
             ('gateway_tickets', ARRAY['SELECT','INSERT','UPDATE','DELETE'], NULL),
             ('private_objects', ARRAY['SELECT','INSERT','UPDATE','DELETE'], NULL),
             ('gateways', ARRAY['SELECT','INSERT','UPDATE'], NULL),
             ('gateway_admins', ARRAY['SELECT','INSERT','DELETE'], NULL),
             ('gateway_leases', ARRAY['SELECT','INSERT','DELETE'], ARRAY['expires_at']),
             ('gateway_approvals', ARRAY['SELECT','INSERT'], ARRAY['enabled']),
             ('gateway_tokens', ARRAY['SELECT','INSERT'], ARRAY['revoked_at','last_used_at']),
             ('migration_archives', ARRAY['SELECT','INSERT'], NULL),
             ('gateway_terms', ARRAY['SELECT','INSERT'], NULL),
             ('gateway_terms_texts', ARRAY['SELECT','INSERT'], NULL),
             ('terms_acceptances', ARRAY['SELECT','INSERT'], NULL),
             ('beta_keys', ARRAY['SELECT'], ARRAY['uses_left']),
             ('account_roles', ARRAY['SELECT','INSERT','DELETE'], NULL),
             ('audit_events', ARRAY['SELECT','INSERT'], NULL),
             ('pairings', ARRAY['SELECT','INSERT','UPDATE','DELETE'], NULL),
             ('reports', ARRAY['SELECT','INSERT','DELETE'], ARRAY['status','issue_number','assignee','priority','labels','duplicate_of','revision','done_at','deleted_at','updated_at']),
             ('report_parts', ARRAY['SELECT','INSERT','DELETE'], NULL),
             ('report_notes', ARRAY['SELECT','INSERT','DELETE'], NULL),
             ('report_audit', ARRAY['SELECT','INSERT'], NULL),
             ('report_views', ARRAY['SELECT','INSERT','DELETE'], ARRAY['query']),
             ('report_seen', ARRAY['SELECT','INSERT','DELETE'], NULL))
             SELECT count(*) = 23 AND bool_and(
               NOT pg_catalog.pg_has_role(c.relowner, 'USAGE')
               AND (SELECT bool_and(pg_catalog.has_table_privilege(c.oid, p)) FROM unnest(r.privileges) p)
               AND NOT pg_catalog.has_table_privilege(c.oid, 'TRUNCATE, REFERENCES, TRIGGER, MAINTAIN, SELECT WITH GRANT OPTION, INSERT WITH GRANT OPTION, UPDATE WITH GRANT OPTION, DELETE WITH GRANT OPTION')
               AND NOT pg_catalog.has_any_column_privilege(c.oid, 'SELECT WITH GRANT OPTION, INSERT WITH GRANT OPTION, UPDATE WITH GRANT OPTION, REFERENCES')
               AND NOT EXISTS (SELECT 1 FROM unnest(ARRAY['INSERT','UPDATE','DELETE']) p
                 WHERE NOT p = ANY(r.privileges) AND pg_catalog.has_table_privilege(c.oid, p))
               AND NOT EXISTS (SELECT 1 FROM unnest(ARRAY['INSERT','UPDATE']) p
                 WHERE NOT p = ANY(r.privileges) AND (p = 'INSERT' OR r.updatable IS NULL)
                 AND pg_catalog.has_any_column_privilege(c.oid, p))
               -- A table with updatable columns: UPDATE on exactly those columns.
               AND (r.updatable IS NULL OR (
                 (SELECT bool_and(pg_catalog.has_column_privilege(c.oid, u, 'UPDATE')) FROM unnest(r.updatable) u)
                 AND NOT EXISTS (SELECT 1 FROM pg_catalog.pg_attribute a WHERE a.attrelid = c.oid
                   AND a.attnum > 0 AND NOT a.attisdropped AND NOT a.attname::text = ANY(r.updatable)
                   AND pg_catalog.has_column_privilege(c.oid, a.attnum, 'UPDATE'))))
             ) AS allowed FROM required r JOIN pg_catalog.pg_class c ON c.relname = r.name
             JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace WHERE n.nspname = 'central_identity'",
            vec![],
        )).await.ok().flatten().and_then(|row| row.try_get::<bool>("", "allowed").ok()) == Some(true)
    }
}
