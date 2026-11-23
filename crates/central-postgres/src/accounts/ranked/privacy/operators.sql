SELECT m.account_a,m.account_b,m.state,coalesce(r.generation,0) AS generation,
       coalesce(r.phase='finished' AND r.retire_at>clock_timestamp(),false) AS replay,
       recipient.host,recipient.epoch,recipient.primary_role,g.origin,terms.version,terms.digest,
       coalesce(v.enabled AND v.ranked_verified AND v.ranked_epoch=recipient.epoch,false) AS verified,
       who.seat,coalesce(p.allowed,false) AS approved,coalesce(NOT p.allowed,false) AS declined,
       (terms.version IS NULL OR EXISTS (
          SELECT 1 FROM central_identity.terms_acceptances t WHERE t.account_id=who.account
            AND t.subject=g.origin AND t.version=terms.version AND t.digest=terms.digest
       )) AS terms_ok
FROM central_identity.ranked_matches m
LEFT JOIN central_identity.ranked_replicas r ON r.match_id=m.id
CROSS JOIN LATERAL (VALUES (m.host_id,m.ranked_epoch,true),(r.standby_id,r.standby_epoch,false))
    AS recipient(host,epoch,primary_role)
CROSS JOIN LATERAL (VALUES (0::smallint,m.account_a),(1::smallint,m.account_b)) AS who(seat,account)
LEFT JOIN central_identity.gateways g ON g.id=recipient.host
LEFT JOIN central_identity.gateway_approvals v ON v.gateway_id=g.id
LEFT JOIN LATERAL (
    SELECT version,digest FROM central_identity.gateway_terms WHERE audience=g.origin
    ORDER BY published_at DESC,version DESC LIMIT 1
) terms ON true
LEFT JOIN central_identity.ranked_host_permissions p ON p.match_id=m.id AND p.host_id=recipient.host
    AND p.seat=who.seat AND p.epoch=recipient.epoch AND p.origin=g.origin
    AND p.version IS NOT DISTINCT FROM terms.version AND p.digest IS NOT DISTINCT FROM terms.digest
WHERE m.id=$1 AND recipient.host IS NOT NULL
ORDER BY recipient.primary_role DESC,who.seat
