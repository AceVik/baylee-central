# Bug and crash reports in Central (design, pass 1)

**Status: design, decided; pass 2 below awaits the owner's go (31.10.2026).**
Owner decisions, 2026-10-20: reports go to
Central; clients report through their gateway, which forwards; a client signed
in nowhere reports directly; Central absorbs Baylee's feedback service. Later
the same day: the open reports move to Central, resolved ones are not
migrated, and the triage UI is reworked with quality-of-life features and
more. The owner chose transport (i), raw-body routes in `central-http`, on
the same day. This page is the design pass (security assurance pass 1).

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
   - **(i) raw-body routes in `central-http`** (chosen, owner 2026-10-20):
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
   The owner chose **(i)**. The routes go through the three passes of
   `docs/security-assurance.md` before their code; (ii) is not built.
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

## Pass 2: the routes, the schema and the order (31.10.2026)

The owner asked for the feedback view in the website's administration,
"complete with every feature the feedback UI has, and more", and chose, on
31.10.2026, the path above (Central stores the reports; no proxy to the
feedback service) and the whole of its triage (not the official gateway's
operations console, which stays the feedback service's, as the gateway is
the operator's). This pass makes the transport (i) concrete. Nothing here
is built before the owner's go.

### Order

1. **Central** (this repository, one pull request): the schema, the intake
   routes, the triage requests, the downloads, the sweep, their tests.
   Nothing reaches it yet, so it changes nothing in production.
2. **The website** (this repository, its own pull request after #57): the
   *Feedback* section of the administration, every feature below.
3. **The switch** (Baylee, its own session, landed by the PM): the gateway
   forwards to Central instead of the feedback service; the official
   gateway takes direct client reports and forwards them marked `direct`;
   a one-time copy of the open reports, their notes and audit rows
   (`central-service --import-reports-stdin`, JSON Lines a Baylee export
   writes). The feedback service keeps its contact form, Impressum, offline
   count and gateway console until they move (point 6 above).

### Intake: native listener, raw bodies

Both on the native listener beside `/v1/native`, each with its own body
bound (the wire's 1 MiB does not apply, and nothing here is wire JSON):

**Gzip all the way (owner, 31.10.2026: "the feedback traffic should be
sent gzipped").** What is heavy travels and rests compressed: the `client`
object (system details, the window's picture) and the record are gzip from
the gateway to Central to the administrator's browser, and Central stores
those bytes as they came. Central never unpacks them: it checks their
length and SHA-256 against what the report announced, so it needs no
decompressor (no new crate) and no compressed body can grow inside it. The
browser unpacks one (`DecompressionStream`) and stops past a bound (16 MiB
for `client`, the record only on download). What a list filters and
searches by stays plain JSON in the report's metadata: the build, and a
crash's message and place. Answers to the browser are compressed by the
proxy as every answer is (`encode zstd gzip`).

- `POST /v1/reports`, `Authorization: Bearer <credential>`: the bridge's
  credential (the official gateway), or a live gateway token with the `sso`
  scope of an **approved** gateway (a pending one files nothing: a public
  intake for strangers is what approval exists for). Body, the metadata
  only, `deny_unknown_fields`, at most 64 KiB: `gateway {name, url,
  version}` as the gateway says it, `channel` (`direct` only from the
  bridge), `reporter` (1–128 hex), `kind` (`bug`, `improvement`,
  `feedback`, `crash`, `other`; a chat message is the gateway's
  moderation and refused), `text` (1–20000 characters), `game_id` (≤ 128),
  `build {version, commit}`, `crash {message, location} | null` (≤ 2000 and
  512 characters), and for each part it will send `{bytes, sha256}`:
  `client` (≤ 2 MiB of gzip) and `record` (`{complete, origin, bytes,
  sha256}`, ≤ 64 MiB, 4 MiB when its origin is `client`). The gateway is the
  authenticated one, never the body's claim (its name and address are
  stored beside what it says of itself). Answers `201 {"report_id"}`; the
  report is listed at once, its parts shown as awaited until they come.
- `PUT /v1/reports/{id}/client` and `PUT /v1/reports/{id}/record`, the same
  credential and the same gateway, the gzip as the raw body
  (`Content-Type: application/gzip`, `Content-Length` required): read with
  the announced length as the bound while hashing, refused unless length
  and SHA-256 equal what the report announced, starting with gzip's magic,
  written once (`409` again), within an hour of the report. Two uploads at
  a time (a semaphore), so at most 128 MiB of parts is in memory.
- Budgets: per gateway 120 reports an hour, besides the native listener's
  global budget every request spends; `429` past them. Nothing of a
  body is logged; the log names the report id and the gateway.
- The crash signature is made at intake as the feedback service makes it,
  with SHA-256 (no MD5 crate), its first 16 bytes in hex.

### Triage: browser listener, `Admin { request }`

Reads and changes as `AdminRequest`s, the administrator checked in the
statement's transaction like every admin request, changes needing the
fresh password sign-in and writing `report_audit` in the same transaction:

| request | answer |
| --- | --- |
| `Reports { filter, sort, page }` | a page (50, at most 200) and the total, metadata only: no `client`, no record |
| `ReportFacets { filter }` | counts per status, kind, priority, assignee, label, gateway, channel and the busiest reporters, each under the filter but its own dimension |
| `Report { report }` | one report's metadata, notes, audit rows, whether `client` and a record exist and their sizes |
| `ReportNeighbours { report, filter, sort }` | previous, next, position, total in that list |
| `CrashGroups { filter }` | crashes by signature: count, open, first, last, latest, message, location |
| `ReportStats` | reports per UTC day and kind, 30 days |
| `SetReport { report, change, expected_revision }` | status, issue number, assignee, priority, labels, duplicate of, as the feedback service's §"Triage" rules them (one duplicate level, revision `Conflict`) |
| `ReportsBulk { reports ≤ 200, change \| trash \| restore \| purge }` | one transaction, an audit row per report and part |
| `TrashReport`, `RestoreReport`, `PurgeReport` | the trash, 30 days |
| `AddReportNote`, `RemoveReportNote` | notes, 1–4000 characters, removed by their author only |
| `ReportViews`, `SaveReportView`, `DeleteReportView` | an administrator's saved lists, at most 50 |
| `SetReportsSeen { reports ≤ 500, seen }` | the unread marker, per administrator |

The filter is the feedback service's (`status`, `kind`, `priority` several
each; `q` words and pieces of the text, a crash's message or the id, `%`
and `_` literal; days `from`/`to`; `has_record`; `channel`; `gateway`;
`assignee` (an account or none); `label`; `signature`; `duplicate_of`;
`trash`; `unseen`), typed and bounded in `central-api`, never SQL text.
The assignee is an administrator's account id, shown by its handle.

Two raw reads on the browser listener, because a `client` object (2 MiB of
gzip, a screenshot inside) and a record (64 MiB) exceed the wire and are
not JSON numbers: `POST /api/v1/reports/{id}/client` and
`POST /api/v1/reports/{id}/record`, both `application/gzip` as stored, with every header
check of `POST /api/v1` (Host, Origin, Sec-Fetch-Site, `X-Baylee-Central`,
no Authorization, an administrator's cookie) and an empty body. The page
fetches them and makes the download itself. A screenshot inside `client`
(a PNG data URL) is checked for the PNG signature and turned into a `Blob`,
shown through a `blob:` address: `img-src` gains `blob:` and keeps `data:`
out (#55).

### Schema: migration 000018

As §"Data" above, in `central_identity`, with the gzip parts in a table of
their own (`report_parts(report_id, part client|record, gzip bytea, bytes,
sha256)`, so a list never touches them) and `reports` holding the plain
metadata, the announced parts' lengths and hashes, `build_version`,
`build_commit`, `crash_message` and `crash_location`, `report_notes`, `report_audit`,
`report_views`, `report_seen`. Indexes for the list's sorts and filters.
Runtime grants as §"Data", checked at startup like the others (a wider or
narrower grant refuses to start). Migration 000017 is the PM's (the
gateway directory's auth-mode badge); this one follows it whichever lands
first.

### Retention and the sweep

Before a list is read, in its transaction: a report done for 90 days, or
in the trash for 30, or older than three years, goes with its record,
notes, views' references and seen rows; its audit rows stay without its
text (Central's privacy notice, "deleted once the bug is dealt with, after
three years at the latest"). The operator's daily sweep does the same.

### Privacy invariants (unchanged, now enforced)

The reporter stays the gateway's pseudonym or the device's HMAC; Central
holds no key to either and never joins a reporter to an account. A
reported chat message is not filed here: it is the gateway's moderation.
The `client` object and the record are read only by an administrator, one
report at a time, and never logged.

### The website's section

`/admin/feedback` with the feedback UI's features, in the website's own
components: the inbox and the other views with counts, search and every
filter in the address as pills, facets, sortable columns, pages, saved
views, crash groups, a drawer beside the list and a page per report with
`j`/`k`, the record and the `client` object (a JSON tree, the screenshot,
the card chips with Scryfall's pictures), every triage field, notes, the
issue link with the neutral summary (never the player's words), the trash
with undo, bulk changes, the unread marker, CSV of the selection or of all
that match (made in the browser, a formula-starting cell prefixed with
`'`), the 30-day chart, a quiet refresh every 30 seconds while visible that
announces new reports, and `?` for the keys. Beyond it: the Overview's
tile for open reports and new crashes, and the navigation's count of
unread ones.

### For the owner and the PM

- **Who files:** the bridge and approved gateways' `sso` tokens. A pending
  gateway and a listing-only token file nothing.
- **Order of deploys:** Central with this migration before any gateway
  that forwards to it; the import of open reports after the switch.
- **Dependencies:** none new.

