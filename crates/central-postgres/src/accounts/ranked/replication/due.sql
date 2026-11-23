-- Each due queue is index ordered and bounded before they are combined. Finished
-- receipts can be retained for years without slowing down a gateway heartbeat.
WITH due(id) AS (
    (SELECT match_id FROM central_identity.ranked_replicas
     WHERE retire_at<=statement_timestamp() ORDER BY retire_at LIMIT 32)
    UNION
    (SELECT match_id FROM central_identity.ranked_replicas
     WHERE phase IN ('paused','disputed') AND paused_until<=statement_timestamp() ORDER BY paused_until LIMIT 32)
    UNION
    (SELECT match_id FROM central_identity.ranked_replicas
     WHERE phase IN ('playing','preparing') AND primary_until<=statement_timestamp() ORDER BY primary_until LIMIT 32)
    UNION
    (SELECT match_id FROM central_identity.ranked_replicas
     WHERE phase IN ('playing','preparing') AND standby_until<=statement_timestamp() ORDER BY standby_until LIMIT 32)
    UNION
    (SELECT m.id FROM central_identity.ranked_matches m JOIN central_identity.ranked_replicas r ON r.match_id=m.id
     WHERE m.state IN ('playing','reserved') AND m.lease_until<=statement_timestamp() ORDER BY m.lease_until LIMIT 32)
)
SELECT m.id,r.phase FROM due
JOIN central_identity.ranked_matches m ON m.id=due.id
JOIN central_identity.ranked_replicas r ON r.match_id=m.id
ORDER BY m.id LIMIT 32 FOR NO KEY UPDATE OF m SKIP LOCKED
