# Bug and crash reports in Central (design, pass 1)

**Status: proposal for the owner.** Owner decision, 2026-10-20: reports go to
Central; clients report through their gateway, which forwards; a client signed
in nowhere reports directly; Central absorbs Baylee's feedback service. This
page is the design pass (security assurance pass 1). Nothing here is built.

## Today (Baylee `docs/feedback.md`)

- A signed-in client sends `POST {gateway}/reports`; the gateway adds its own
  record of the game (up to 64 MiB) and forwards to the operator's feedback
  service (`BAYLEE_FEEDBACK_URL`).
- A client signed in nowhere sends `POST {service}/client/reports` (a device
  id kept only as an HMAC, per-address and service-wide hourly allowances,
  its own record up to 4 MiB of gzip, marked `client`).
- The service also counts offline games (`/client/alive`), takes the landing
  page's contact form and serves the Impressum, and has its own triage UI
  with its own admin accounts.
- A reported **chat message** is moderation, not a bug report: it goes to the
  gateway's moderators (`POST /social/reports`) and stays there. A gateway is
  the hosting service for what is said on it.

## Proposal

1. **Central takes reports only from gateways**, under a gateway's
   credential or the bridge's. A client signed in nowhere reports to the
   official gateway's public client-report route, which forwards it marked
   `client`. Central's native boundary stays credential-only, and the per-
   address limits a public write needs already exist on the gateway; Central
   has only global budgets today (roadmap M3).
2. **Records are uploaded in chunks.** A record is up to 64 MiB, the native
   wire carries 1 MiB. `StartReport { kind, body, record_bytes,
   record_sha256, record_origin }` (body: the report's JSON, at most 256 KiB)
   answers a report id and a one-use upload capability;
   `ReportChunk { report, index, bytes }` (at most 512 KiB each, in order)
   and `FinishReport { report }` complete it, which checks the total length
   and the SHA-256. An unfinished report expires after an hour.
3. **Central stores the gzip as it came** and never unpacks it: no new
   dependency (`flate2` stays out), no decompression bomb on the server. The
   triage UI unpacks in the browser (`DecompressionStream`) or an operator
   tool does. Body shape is checked with bounded `serde_json` types.
4. **"Cannot be manipulated" is a server-side property.** The client is AGPL
   and can be changed; Central's terms forbid connecting a modified one, and
   a report carries the client's build and commit. What Central guarantees:
   a report names the gateway it came from (authenticated, never a claim),
   whether its record is the gateway's own or a client's (`record_origin`),
   and the record's hash; a stored report never changes but for its triage
   status, every status change is audited with the admin's account id, and
   the runtime role cannot rewrite or delete one (deletion is the sweep's).
5. **Retention**, as the draft privacy policy says: a report is deleted 90
   days after it was marked done, three years after it arrived at the
   latest; a sweep runs before each read of the list. Records go with their
   report.
6. **Contact form and Impressum** move with the website: they are browser
   requests and belong behind M3's same-origin `/api`, not the native wire.
   The anonymous offline count goes the same way, or is dropped.
7. **Triage** needs admins and browser sessions (M2, M3) and the admin area
   (M4). Until then one of:
   - **(a) transitional**: the feedback service's UI keeps running and reads
     reports from Central through a loopback operator route (the pattern its
     admin console already uses for the gateway), and gateways forward to
     Central at once; or
   - **(b) all at once**: reports keep going to the feedback service until
     M2 to M4 exist, then intake, storage and triage move together.

   **This choice is the owner's.**

## Data

- `reports(id uuidv7, gateway_audience, kind bug|crash, body bytea ≤ 256 KiB,
  record_origin gateway|client|none, record_bytes, record_sha256, status
  open|done|wontfix, created_at, done_at, finished_at)`.
- `report_chunks(report_id, index, bytes ≤ 512 KiB)`, `ON DELETE CASCADE`.
- `report_audit(report_id, at, admin_id, status)`, kept after the report
  goes, without its text.
- Runtime role: `SELECT, INSERT` on all three, `UPDATE (status, done_at,
  finished_at)` on `reports` by column grant, `DELETE` on `reports` for the
  sweep. No `UPDATE` of a body or a record.

## Dependencies

None new: `serde_json`, `sha2`, SeaORM and the existing HTTP stack suffice.
