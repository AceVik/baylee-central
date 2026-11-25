//! Exact runtime privileges: rewriting archives remains operator-only. The
//! runtime opens and closes registration and keeps its one row. It grants and removes
//! administrator roles, appends to the audit, which it cannot rewrite or
//! delete, and approves, enables and disables gateways as an administrator
//! decides (`docs/administration.md`), never deleting an approval. It stores
//! gateway tokens and may set only when one was revoked or last used. It adds
//! and ages out each gateway's revocations and never rewrites one. It
//! files reports and their parts once, triages a report by column and
//! deletes reports; it never changes a report's words, a part or its history.
//! It stores report tokens with the rights they keep, and may set only when
//! one was revoked, by whom, and last used.
//! It replaces and deletes profile pictures, files picture reports and
//! decides them by column, but never deletes one (the operator's sweep does).
//! It adds a donation once and converts one that came without a rate, by its
//! euro column alone; it reads the donations' sums and never writes them.
use super::{AccountStore, statement};
use sea_orm::ConnectionTrait;

const REQUIRED_GRANTS: &str = "WITH required(name, privileges, updatable) AS (VALUES
             ('accounts', ARRAY['SELECT','INSERT','UPDATE','DELETE'], NULL::text[]),
             ('sessions', ARRAY['SELECT','INSERT','UPDATE','DELETE'], NULL),
             ('private_objects', ARRAY['SELECT','INSERT','UPDATE','DELETE'], NULL),
             ('gateways', ARRAY['SELECT','INSERT','UPDATE'], NULL),
             ('gateway_admins', ARRAY['SELECT','INSERT','DELETE'], NULL),
             ('gateway_streams', ARRAY['SELECT','INSERT','UPDATE'], NULL),
             ('gateway_sids', ARRAY['SELECT','INSERT','DELETE'], ARRAY['ranked_proof_until']),
             ('gateway_revocations', ARRAY['SELECT','INSERT','DELETE'], NULL),
             ('gateway_approvals', ARRAY['SELECT'], ARRAY['enabled','ranked_verified']),
             ('gateway_tokens', ARRAY['SELECT','INSERT'], ARRAY['revoked_at','last_used_at']),
             ('migration_archives', ARRAY['SELECT','INSERT'], NULL),
             ('gateway_terms', ARRAY['SELECT','INSERT'], NULL),
             ('gateway_terms_texts', ARRAY['SELECT','INSERT'], NULL),
             ('terms_acceptances', ARRAY['SELECT','INSERT'], NULL),
             ('registration', ARRAY['SELECT'], ARRAY['open','changed_at','changed_by']),
             ('account_roles', ARRAY['SELECT','INSERT','DELETE'], NULL),
             ('account_emails', ARRAY['SELECT','INSERT','UPDATE','DELETE'], NULL),
             ('account_email_pending', ARRAY['SELECT','INSERT','UPDATE','DELETE'], NULL),
             ('audit_events', ARRAY['SELECT','INSERT'], NULL),
             ('pairings', ARRAY['SELECT','INSERT','UPDATE','DELETE'], NULL),
             ('social_contacts', ARRAY['SELECT','INSERT','UPDATE','DELETE'], NULL),
             ('social_blocks', ARRAY['SELECT','INSERT','DELETE'], NULL),
             ('social_budgets', ARRAY['SELECT','INSERT','UPDATE'], NULL),
             ('friend_invites', ARRAY['SELECT','INSERT'], ARRAY['pending_account_id','pending_until','redeemed_by','redeemed_at','revoked_at']),
             ('chat_devices', ARRAY['SELECT','INSERT','DELETE'], NULL),
             ('chat_messages', ARRAY['SELECT','INSERT','DELETE'], NULL),
             ('chat_deliveries', ARRAY['SELECT','INSERT','DELETE'], NULL),
             ('chat_groups', ARRAY['SELECT','INSERT','UPDATE','DELETE'], NULL),
             ('chat_members', ARRAY['SELECT','INSERT','UPDATE','DELETE'], NULL),
             ('chat_group_pictures', ARRAY['SELECT','INSERT','DELETE'], ARRAY['key','avif','png','sequence','sha256','side','set_at']),
             ('card_printings', ARRAY['SELECT'], NULL),
             ('card_search_names', ARRAY['SELECT'], NULL),
             ('notification_preferences', ARRAY['SELECT','INSERT','UPDATE','DELETE'], NULL),
             ('push_subscriptions', ARRAY['SELECT','INSERT','UPDATE','DELETE'], NULL),
             ('push_jobs', ARRAY['SELECT','INSERT','UPDATE','DELETE'], NULL),
             ('push_releases', ARRAY['SELECT','INSERT'], NULL),
             ('reports', ARRAY['SELECT','INSERT','DELETE'], ARRAY['status','issue_number','assignee','priority','labels','duplicate_of','revision','done_at','deleted_at','updated_at']),
             ('report_parts', ARRAY['SELECT','INSERT','DELETE'], NULL),
             ('report_notes', ARRAY['SELECT','INSERT','DELETE'], NULL),
             ('report_audit', ARRAY['SELECT','INSERT'], NULL),
             ('report_views', ARRAY['SELECT','INSERT','DELETE'], ARRAY['query']),
             ('report_seen', ARRAY['SELECT','INSERT','DELETE'], NULL),
             ('report_tokens', ARRAY['SELECT'], ARRAY['revoked_at','revoked_by','last_used_at']),
             ('pictures', ARRAY['SELECT','INSERT','DELETE'], ARRAY['key','avif','png','sequence','sha256','side','set_at']),
             ('picture_reports', ARRAY['SELECT','INSERT'], ARRAY['state','decided_at','decided_by']),
             ('picture_notices', ARRAY['SELECT','INSERT','UPDATE','DELETE'], NULL),
             ('picture_budgets', ARRAY['SELECT','INSERT','UPDATE'], NULL),
             ('challenges', ARRAY['SELECT','INSERT'], ARRAY['withdrawn']),
             ('challenge_participation', ARRAY['SELECT','INSERT','UPDATE','DELETE'], NULL),
             ('challenge_attempts', ARRAY['SELECT','INSERT','UPDATE','DELETE'], NULL),
             ('challenge_completions', ARRAY['SELECT','INSERT','UPDATE','DELETE'], NULL),
             ('tower_seasons', ARRAY['SELECT','INSERT'], ARRAY['published','withdrawn']),
             ('tower_nodes', ARRAY['SELECT','INSERT'], NULL),
             ('tower_participation', ARRAY['SELECT','INSERT','UPDATE','DELETE'], NULL),
             ('tower_runs', ARRAY['SELECT','INSERT','UPDATE','DELETE'], NULL),
             ('tower_actions', ARRAY['SELECT','INSERT','DELETE'], NULL),
             ('tower_scores', ARRAY['SELECT','INSERT','UPDATE','DELETE'], NULL),
             ('tower_accomplishments', ARRAY['SELECT','INSERT','UPDATE','DELETE'], NULL),
             ('tower_access', ARRAY['SELECT','INSERT','DELETE'], NULL),
             ('tower_award_issuers', ARRAY['SELECT'], ARRAY['last_used_at']),
             ('tower_award_runs', ARRAY['SELECT','INSERT','UPDATE'], NULL),
             ('tower_award_progress', ARRAY['SELECT','INSERT','UPDATE'], NULL),
             ('tower_award_receipts', ARRAY['SELECT','INSERT'], NULL),
             ('donations', ARRAY['SELECT','INSERT'], ARRAY['euro_cents']),
             ('donation_sums', ARRAY['SELECT'], NULL),
             ('donation_count', ARRAY['SELECT'], NULL),
             ('legacy_deck_transfers', ARRAY['SELECT','DELETE'], ARRAY['next_page']),
             ('ranked_host_aliases', ARRAY['SELECT'], NULL),
             ('ranked_host_permissions', ARRAY['SELECT'], ARRAY['epoch','origin','version','digest','allowed']),
             ('ranked_replicas', ARRAY['SELECT'], ARRAY['seed','generation','standby_id','standby_epoch','primary_until','standby_until','paused_until','phase','position','head_digest','state_digest','opening_chain','setup','initial_pools','initial_evidence','evidence','journal_bytes','retire_at']),
             ('ranked_commands', ARRAY['SELECT','INSERT','DELETE'], ARRAY['evidence']),
             ('ranked_witnesses', ARRAY['SELECT','INSERT','DELETE'], NULL),
             ('advent_art',ARRAY['SELECT'],NULL),
             ('playmat_framing',ARRAY['SELECT'],NULL),
             ('advent_campaigns',ARRAY['SELECT'],NULL),
             ('advent_doors',ARRAY['SELECT'],NULL),
             ('advent_solutions',ARRAY['SELECT'],NULL),
             ('advent_entitlements',ARRAY['SELECT'],NULL),
             ('cosmetic_collection',ARRAY['SELECT'],NULL),
             ('cosmetic_equip_rights',ARRAY['SELECT'],NULL),
             ('ranked_cosmetic_catalog',ARRAY['SELECT'],NULL),
             ('ranked_cosmetic_unlocks',ARRAY['SELECT'],NULL),
             ('advent_ranked_looks',ARRAY['SELECT','INSERT','DELETE'],NULL),
             ('advent_table_looks',ARRAY['SELECT','DELETE'],ARRAY['confirmed','expires_at']),
             ('advent_table_outfits',ARRAY['SELECT','DELETE'],NULL),
             ('ranked_seasons', ARRAY['SELECT'], NULL),
             ('ranked_clocks', ARRAY['SELECT','INSERT','DELETE'], ARRAY['checkpoint']),
             ('ranked_hosts', ARRAY['SELECT'], ARRAY['ranked_epoch','capacity','seen_at','continuity_since','compatibility']),
             ('ranked_matches', ARRAY['SELECT'], ARRAY['claimed_a','claimed_b','game_id','start_fingerprint','deck_a','deck_b','state','outcome','finished_at','lease_until','host_id','ranked_epoch','origin']),
             ('ranked_tickets', ARRAY['SELECT'], ARRAY['state','match_id']),
             ('ranked_ratings', ARRAY['SELECT','INSERT'], ARRAY['elo','queue_elo','score','placement','allowance','games','wins','losses','draws','league','division','last_at']),
             ('ranked_deck_versions', ARRAY['SELECT','INSERT'], ARRAY['elo','anchor']),
             ('ranked_deck_ratings', ARRAY['SELECT','INSERT'], ARRAY['elo','games','season_games','wins','losses','draws','league','division','last_at']),
             ('ranked_receipts', ARRAY['SELECT','INSERT'], NULL),
             ('ranked_publications', ARRAY['SELECT','INSERT'], ARRAY['published_on','finalized']),
             ('ranked_champions', ARRAY['SELECT','INSERT','DELETE'], NULL),
             ('ranked_badge_peaks', ARRAY['SELECT','INSERT'], ARRAY['place'])),
             insert_columns(name,cols) AS (VALUES
               ('advent_table_looks', ARRAY['proof','account_id','gateway_id','sid','game','seat','sleeve','playmat']),
               ('advent_table_outfits', ARRAY['account_id','gateway_id','game','seat','stream','sleeve','playmat']),
               ('ranked_host_aliases', ARRAY['match_id','host_id']),
               ('ranked_host_permissions', ARRAY['match_id','host_id','seat','epoch','origin','version','digest','allowed']),
               ('ranked_replicas', ARRAY['match_id','standby_id','standby_epoch','compatibility']),
               ('advent_solutions',ARRAY['account_id','year','door']),
               ('advent_entitlements',ARRAY['account_id','year','door','art_id']),
               ('cosmetic_equip_rights',ARRAY['account_id','art_id','advent_id','league','division']),
               ('gateway_approvals', ARRAY['gateway_id','enabled']),
               ('report_tokens', ARRAY['name','digest','file','read','triage','purge','created_by','created_at','expires_at']),
               ('ranked_hosts', ARRAY['host_id','ranked_epoch','capacity','seen_at','compatibility']),
               ('ranked_matches', ARRAY['season_id','format','account_a','account_b','commitment_a','commitment_b','host_id','ranked_epoch','origin']),
               ('ranked_tickets', ARRAY['account_id','request_id','season_id','format','commitment','queue_elo','replicable']))
             SELECT count(*) = (SELECT count(*) FROM required) AND bool_and(
               NOT pg_catalog.pg_has_role(c.relowner, 'USAGE')
               AND (SELECT bool_and(pg_catalog.has_table_privilege(c.oid, p)) FROM unnest(r.privileges) p)
               AND NOT pg_catalog.has_table_privilege(c.oid, 'TRUNCATE, REFERENCES, TRIGGER, MAINTAIN, SELECT WITH GRANT OPTION, INSERT WITH GRANT OPTION, UPDATE WITH GRANT OPTION, DELETE WITH GRANT OPTION')
               AND NOT pg_catalog.has_any_column_privilege(c.oid, 'SELECT WITH GRANT OPTION, INSERT WITH GRANT OPTION, UPDATE WITH GRANT OPTION, REFERENCES')
               AND NOT EXISTS (SELECT 1 FROM unnest(ARRAY['INSERT','UPDATE','DELETE']) p
                 WHERE NOT p = ANY(r.privileges) AND pg_catalog.has_table_privilege(c.oid, p))
               AND NOT EXISTS (SELECT 1 FROM unnest(ARRAY['INSERT','UPDATE']) p
                 WHERE NOT p = ANY(r.privileges) AND CASE p WHEN 'INSERT' THEN i.cols IS NULL ELSE r.updatable IS NULL END
                 AND pg_catalog.has_any_column_privilege(c.oid, p))
               AND (i.cols IS NULL OR (
                 (SELECT bool_and(pg_catalog.has_column_privilege(c.oid, u, 'INSERT')) FROM unnest(i.cols) u)
                 AND NOT EXISTS (SELECT 1 FROM pg_catalog.pg_attribute a WHERE a.attrelid = c.oid
                   AND a.attnum > 0 AND NOT a.attisdropped AND NOT a.attname::text = ANY(i.cols)
                   AND pg_catalog.has_column_privilege(c.oid, a.attnum, 'INSERT'))))
               -- A table with updatable columns: UPDATE on exactly those columns.
               AND (r.updatable IS NULL OR (
                 (SELECT bool_and(pg_catalog.has_column_privilege(c.oid, u, 'UPDATE')) FROM unnest(r.updatable) u)
                 AND NOT EXISTS (SELECT 1 FROM pg_catalog.pg_attribute a WHERE a.attrelid = c.oid
                   AND a.attnum > 0 AND NOT a.attisdropped AND NOT a.attname::text = ANY(r.updatable)
                   AND pg_catalog.has_column_privilege(c.oid, a.attnum, 'UPDATE'))))
             ) AS allowed FROM required r LEFT JOIN insert_columns i ON i.name = r.name
             JOIN pg_catalog.pg_class c ON c.relname = r.name
             JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace WHERE n.nspname = 'central_identity'";

impl AccountStore {
    pub(super) async fn valid_grants(&self) -> bool {
        self.db
            .query_one_raw(statement(REQUIRED_GRANTS, vec![]))
            .await
            .ok()
            .flatten()
            .and_then(|row| row.try_get::<bool>("", "allowed").ok())
            == Some(true)
    }
}
