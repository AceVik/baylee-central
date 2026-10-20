# Bug and crash reports in Central (design, pass 1)

**Status: design, decided.** Owner decisions, 2026-10-20: reports go to
Central; clients report through their gateway, which forwards; a client signed
in nowhere reports directly; Central absorbs Baylee's feedback service. Later
the same day: the open reports move to Central, resolved ones are not
migrated, and the triage UI is reworked with quality-of-life features and
more. This page is the design pass (security assurance pass 1).

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
2. **Not the native wire.** A report's `client` part reaches 2 MiB (an
   embedded screenshot), a gateway's record 64 MiB; the native wire carries
   1 MiB and writes bytes as JSON numbers. Two transports fit, neither is
   the native wire:
   - **(i) raw-body routes in `central-http`** (recommended):
     `POST /v1/reports` (metadata and `client` JSON, bounded),
     `PUT /v1/reports/{id}/record` (the gzip as a raw body, streamed with
     its own limit and checked against its declared length and SHA-256),
     and for triage the matching reads, under a gateway's credential or a
     triage credential. New HTTP surface, so the three passes and the
     owner's review; M4's admin area needs the same routes to download a
     record, so they are not throwaway.
   - **(ii) a `central_reports` schema** in `baylee_central`, owned by
     Central's migrator and used by the feedback service under a narrow
     TLS role. No new network surface, but a cross-repository schema
     contract without a version, and Central's runtime would not use the
     reports until M4.
   **Which one is the owner's to choose.** Until then no Central code is
   written for reports.
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

   The owner's answer (2026-10-20) moves the open reports and reworks the
   UI, so the path is **(a)**. Order: the UI and its new triage features
   come first, in Baylee's feedback service, against its own database, with
   the new columns shaped as the Central schema below, so that the later
   move is a copy; then reports move to Central over the chosen transport
   (2), and the UI reads them there until M4 moves it into the website.
   A one-time migration copies every open report (`new`, `triaged`,
   `in_progress`) with its record and audit rows; `resolved`, `wont_fix` and
   `duplicate` are not migrated.

## Privacy invariants

- The reporter is the gateway's pseudonym (an HMAC under
  `BAYLEE_FEEDBACK_KEY`, which stays on the gateway) or a direct reporter's
  device HMAC. Neither key, nor any HMAC of an account id, ever reaches
  Central: "we cannot know who wrote this" depends on it, and Central holds
  account ids.
- A direct client report comes through the official gateway, which keeps
  checking a client record's shape (it links the decompressor); Central never
  decompresses.

## Data

As the feedback service's `feedback_report` has it today, plus the triage
columns its reworked UI adds, so a move is a copy:

- `reports(id uuidv7, created_at, updated_at, gateway, gateway_name,
  gateway_url, gateway_version, channel gateway|direct, reporter, kind,
  status new|triaged|in_progress|resolved|wont_fix|duplicate, text, game_id,
  client jsonb ≤ 2 MiB, record bytea ≤ 64 MiB, record_complete,
  record_origin gateway|client, issue_number, assignee, priority, labels,
  duplicate_of, signature, revision, done_at, deleted_at)`.
- `report_notes(id, report_id, at, author, text)`: internal notes, their own
  table, going with their report.
- `report_audit(report_id, at, actor, action, detail)`: every change of a
  report's state, kept after the report goes, without its text.
- `report_views(id, admin, name, query, created_at)` and
  `report_seen(admin, report_id, at)`: an admin's saved lists and which
  reports they opened, theirs alone.
- Runtime role: `SELECT, INSERT` on all five; `UPDATE (status, issue_number,
  assignee, priority, labels, duplicate_of, revision, done_at, deleted_at,
  updated_at)` on `reports` by column grant; `DELETE` on `reports` (the
  trash's purge and its 30-day sweep), `report_notes`, `report_views` and
  `report_seen`. No `UPDATE` of a body, a client object or a record, and
  none of `report_audit`.
- The migration copies an open report's triage as it is; an open report is
  nobody's duplicate (leaving `duplicate` clears `duplicate_of`), so no link
  points at a report left behind. Notes and audit rows of the reports left
  behind stay in the feedback service until it is retired.

## Dependencies

None new for (i) or (ii): `serde_json`, `sha2`, SeaORM and the existing HTTP
stack suffice; the gzip stays opaque.
