-- What each gateway's reports count as, and all reports together (the line
-- without a gateway), by the measure a filing is weighed with
-- (docs/reports.md, "What the reports may take"; the store's
-- reports/level.rs). It reads and changes nothing, and reads no part's
-- bytes. A report counts as 64 KiB, and each part it announced as its
-- announced length while it is stored or may still come (the report is
-- less than an hour old).
--
--   psql -X -v ON_ERROR_STOP=1 -f ops/report-levels.sql
--
-- The bounds: 2 GiB (2147483648) for a registered gateway, 8 GiB
-- (8589934592) for the official one, 16 GiB (17179869184) for all.
SELECT w.gateway_id,
       max(w.gateway_label) AS name,
       count(*)::bigint AS reports,
       sum(w.bytes)::bigint AS bytes,
       pg_catalog.pg_size_pretty(sum(w.bytes)) AS counted
FROM (SELECT r.gateway_id, r.gateway_label, 65536::bigint
          + CASE WHEN COALESCE(p.client, false)
                   OR r.created_at > pg_catalog.clock_timestamp() - interval '3600 seconds'
                 THEN COALESCE(r.client_bytes, 0) ELSE 0 END
          + CASE WHEN COALESCE(p.record, false)
                   OR r.created_at > pg_catalog.clock_timestamp() - interval '3600 seconds'
                 THEN COALESCE(r.record_bytes, 0) ELSE 0 END AS bytes
      FROM central_identity.reports r
      LEFT JOIN (SELECT s.report_id, bool_or(s.part = 'client') AS client,
                        bool_or(s.part = 'record') AS record
                 FROM central_identity.report_parts s GROUP BY s.report_id) p
        ON p.report_id = r.id) w
GROUP BY ROLLUP (w.gateway_id)
ORDER BY sum(w.bytes) DESC, w.gateway_id;
