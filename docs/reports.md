# Bug and crash reports in Central (design, pass 1)

**Status: Central's part built (31.10.2026, owner's go the same day); the
website's section and the switch follow (§"Order").**
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
  written once (`409` again), within an hour of the report. Four uploads
  at a time, two of one gateway, and none for a gateway that left thirty
  unfinished in the hour (§"What the reports may take" below; until
  12.11.2026 two at a time for everyone, a semaphore).
- Budgets: per gateway 120 reports an hour, besides the native listener's
  global budget every request spends, and what a gateway's reports and
  all reports may take in the store (below); `429` past them. Nothing of a
  body is logged, and nothing else of a report: Central writes no line
  when one is filed. (This said until 11.11.2026 that the log names the
  report id and the gateway; no such line exists.)
- The crash signature is made at intake as the feedback service makes it,
  with SHA-256 (no MD5 crate), its first 16 bytes in hex.

### What the reports may take (12.11.2026)

An approved gateway is a stranger's server that an administrator let in.
Until this date one of them could fill the disk (120 reports an hour,
each announcing up to 66 MiB of parts: about 7.7 GiB an hour) and could
hold both upload slots with uploads that never finish. Four numbers
answer that. Each is a constant in the code with its reason beside it,
and each is the owner's to change.

**In the store** (`central-postgres`, `accounts/reports/level.rs`). A
filing is weighed before it is written, and refused with `429` (`Busy`)
past a bound:

| whose reports | may take | constant |
| --- | --- | --- |
| a registered gateway's | 2 GiB | `GATEWAY_STORED` |
| the official gateway's | 8 GiB | `OFFICIAL_STORED` |
| everyone's together | 16 GiB | `ALL_STORED` |

- What counts: every report as 64 KiB (`REPORT_BYTES`, its body's bound,
  whatever its length), and each part it announced as its announced
  length while the part is stored or may still come. A part may come for
  an hour after its report; one that never came stops counting then. So a
  gateway that announces and never sends holds room for an hour at most.
- No count is kept beside the reports. The level is asked of the reports
  themselves in the filing's transaction (one pass over the reports'
  rows and the parts' index; no part's bytes are read). It cannot drift,
  a restart changes nothing, and a report that goes (a purge, the sweep)
  frees its room. No migration was needed.
- One filing is weighed at a time, under a transaction's advisory lock
  that is tried and never waited for. Two filings cannot both take the
  last room. A filing that finds another being weighed is `429` and is
  sent again by its gateway (the released gateway waits 250 ms, then a
  second, before it tells its player the report failed).
- The report is refused whole, so nothing of it is stored and no part can
  follow. The released gateway answers its player `503` then; the report
  is lost unless the player sends it again later.
- 2 GiB is 31 reports with both parts at their largest, about 2,000 with a
  megabyte of parts each, or 32,768 without parts. A gateway that
  announces all it may reaches it in a quarter of an hour.
- The official gateway has four times as much, since most players play
  through it. What the one-time copy brought counts in it. The copy
  itself is the operator's and is not weighed.
- The official gateway at its bound leaves four registered gateways
  theirs. More approved gateways than that can fill everyone's 16 GiB
  between them, and then every gateway's reports are refused, the
  official one's too.
- Room comes back when reports go: an administrator purges them (the
  trash alone frees nothing until its 30 days are over), or the sweep
  takes them (done for 90 days, in the trash for 30, older than 1,095).
  [Operations](operations.md) §"What reports may take" has the query that
  shows each gateway's level.
- What the numbers do not say: the database's files do not shrink when
  rows go, and the write-ahead log, indexes and backups take room of
  their own. 16 GiB counted is more than 16 GiB of disk.

**At the listener** (`central-http`, `uploads.rs`). In the service's
memory, so a restart frees every slot and forgets the counts:

| what | number | constant |
| --- | --- | --- |
| uploads at once | 4 | `UPLOADS` |
| uploads of one gateway at once | 2 | `UPLOADS_PER_GATEWAY` |
| uploads a gateway may leave unfinished in an hour | 30 | `UNFINISHED_PER_HOUR` |

- Two for one gateway is what the released gateway sends at once
  (`baylee-gateway`, `report/central.rs`). With four in all, one gateway
  that holds its two no longer refuses every other gateway's parts. Two
  approved gateways that each hold two still do. The official gateway is
  counted like any other, by its id.
- Memory. A slot holds its part once while it is read and, by reading the
  code and not by measurement, up to three times while it is stored (the
  bytes read, the statement's parameter, the driver's message). A record
  of 64 MiB is then about 192 MiB, and four of them stored in the same
  moment about 768 MiB; with two slots it was 384 MiB. (This document
  said 128 MiB until 12.11.2026, which counted the reading only.) Parts
  are usually far smaller. A server that cannot spare that much needs
  `UPLOADS` at 2 again, and then one gateway's two uploads refuse
  everyone else's as before.
- Unfinished is an upload that took a slot and ended without its part
  stored: too slow (`408`), a connection that went away, the upload's
  minute over, or bytes other than announced. A part that the gateway's
  other upload stored meanwhile (`409`) and a store that failed (`503`)
  are held against nobody. Past thirty in an hour, counted from the first,
  the gateway's uploads are `429` until that hour is over.
- Why thirty. The released gateway sends a part at most five times (after
  2, 8, 20 and 40 seconds) and a report has two parts, so a report whose
  every send fails leaves ten. Thirty is three such reports in one hour.
  A gateway restarted in the middle of its uploads leaves two. A restart
  of Central forgets the count. A part that stalls is answered `408` and
  is not sent again, so it leaves one.
- A record too large for its gateway's line to carry in a minute is the
  honest case that reaches the count soonest. The gateway's own minute
  may end before Central's does, and then it sends the record five times:
  six such records in an hour are thirty. No number is out of reach of a
  line that carries nothing.
- What the count costs an honest gateway that reaches it: its parts are
  `429` for the rest of that hour and are given up after five sends, the
  small ones that would have arrived among them; the reports themselves
  are still filed, without those parts. Every ten more in the number are
  five more minutes an hour in which two hostile gateways can hold all
  four slots.
- What the count does not end: a gateway that keeps the pace and never
  finishes holds its two slots for up to a quarter of every hour (thirty
  uploads of a minute each), and two such gateways hold all four for that
  long.

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
The `client` object and the record are read only by an administrator or,
since 25.11.2026, by the holder of a report token that may read
([below](#report-tokens-25112026)), one report at a time, and never logged.
Who may hold such a token is the administrators' to decide: the
administration's own tools, and a hosted AI agent only once its provider
is a processor under contract and named in the privacy policy
(§"Decided by the owner" there).

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

The new site (`docs/proposals/frontend-rewrite.md` §26) has the section
at `/{lang}/account/admin/feedback` and a report at
`/{lang}/account/admin/feedback/<id in hex>`, on the same requests and the
same query string, so saved views carry over. It leaves out the drawer,
the single-key shortcuts, the quiet refresh, and Scryfall's pictures (its
`img-src 'self'` admits no other origin); the screenshot is drawn from its
checked bytes into a canvas, never through an address.

### For the owner and the PM

- **Who files:** the bridge and approved gateways' `sso` tokens. A pending
  gateway and a listing-only token file nothing.
- **Order of deploys:** Central with this migration before any gateway
  that forwards to it; the import of open reports after the switch.
- **Dependencies:** none new.

## Pass 3: as built (31.10.2026)

What the building decided beyond pass 2:

- **A part's upload is authenticated before it is read.** The handler
  checks the credential, the report and the announcement first (a report
  of this gateway, filed within the hour, the part announced and not stored
  yet), then the declared length must equal the announced one, then it
  takes one of two slots and reads exactly that many bytes. A stranger's
  slow body holds no slot and is never read; the boundary only refuses a
  missing or oversized `Content-Length`. Since 11.11.2026 the gateway's own
  body must keep a pace too (16 KiB in every five seconds until it is
  whole), so one that is withheld gives its slot back then, with `408`.
  What that does not hold is an approved gateway that keeps the pace and
  never finishes: a part that was not stored may be sent again, and twice
  at once, so one gateway can hold both slots while other gateways' parts
  are answered `429`. Open since 11.11.2026, with the bound on stored
  bytes ([security assurance](security-assurance.md), "The review's other
  findings"). Since 12.11.2026 there are four slots, two for one gateway,
  a count of a gateway's unfinished uploads, and bounds on stored bytes
  ([above](#what-the-reports-may-take-12112026), with what stays open).
- **Triage asks for no fresh password; a purge does.** Triage is daily
  work, reversible and in the report's history; only the purge cannot be
  undone (`AdminRequest::fresh`).
- **A duplicate needs its original.** When a report goes (a purge, the
  sweep), its duplicates that stay are opened again as `triaged`, with a
  line in their history, before it is deleted; the schema holds that a
  duplicate names its original and that `done_at` goes with the done
  statuses.
- **The record's and the client's bytes are a table of their own**
  (`report_parts`), written once and never updated; the announced lengths
  and hashes are columns of the report, so a list reads none of the bytes.
- **The operator's daily sweep** takes reports too (`Swept::reports`), so the
  retention holds when nobody reads a list.

Tests: the API's bounds, crash signatures, the change's leave-or-clear and
the filter's types; the HTTP boundary (an upload's declared length, its
handler reading the body) and the browser listener's part route (the
site's proofs, the cookie, the bytes as stored, its own budget); and the
PostgreSQL fixture (`tests/support/accounts/reports.rs`): a pending
gateway files nothing, direct only through the bridge, a part's wrong
length, hash or magic refused and a second one `Conflict`, administrators
only, the filters and facets, the revision, assignees who are
administrators, one duplicate level and the hand-on, notes by their
author, a selection, the trash and a purge with a fresh password, the
released duplicate, views, the unread marker per administrator, the sweep
keeping the history, and the grants both ways.

## Pass 4: the one-time copy (31.10.2026)

Step 3's copy of the reports still open at the feedback service, as built:
Baylee's `baylee-feedback export-reports` writes them as JSON Lines (its
`docs/feedback.md` §"The one-time copy to Central" is the format), and
`central-service --import-reports-stdin` takes them under the migration login
(`docs/operations.md`).

- **Only open reports**, outside the service's trash; done ones stay there.
  Each keeps its id (the service's are UUID v7), its times, status, issue,
  priority and labels, its `client` object and record as the gzip they
  were, and a crash's signature made here.
- **What has no account here keeps its name in words.** Every copied
  report is filed under the official gateway, its service name as its
  label. A note is filed under the administrator the operator names, its
  first author's name before its text; a line of the old history keeps its
  time, its actor's name in its detail and its action where it reads as
  Central's (`imported` otherwise); the assignee is named in the report's
  own first line of history, since an account here cannot be guessed from a
  name there. Seen marks and saved lists stay at the service.
- **All or nothing, and once.** One transaction; the last line's count and
  SHA-256 over the lines before it must match, so a copy cut short stores
  nothing; a report whose id is here already is skipped, so running it
  again copies nothing twice.
- **The gzip travels as hex**, which Central reads without a decoder of its
  own and without a new dependency.
- **The checks are the intake's own** (`ReportIntake::validate`), the
  status open, the labels Central's, the issue within its range.

## Report tokens (25.11.2026)

The owner asked for tokens that administrators hand out for the report API,
with levels: some only post, some only read, some read and edit (delete,
move and so on), some everything. A report token is an administrator's
credential for a script, a build machine or an agent. It is no gateway, no
account and no administrator, and it signs nobody in.

### Pass 1: design

**Assets.** Report words, `client` objects (with a screenshot) and records;
the triage and its history; the store's room; the bounds that keep one
caller from filling it; the tokens themselves.

**Callers considered.** A stranger without a token; a holder of a revoked or
expired token; a holder who tries more than the token allows; a holder who
presents a gateway's credential, a session or a cookie instead, or a report
token where one of those belongs; an administrator whose role was removed
after making a token; a stolen browser cookie; a buggy script that repeats
itself; a page in a browser.

**Invariants.**

1. A report token does exactly what its rights say, on the report routes and
   nowhere else. No right implies a sign-in, an administration request, a
   gateway's operation or another table.
2. Whether a token is live and holds the right is asked by the query that
   authenticates it, every time, and again in the transaction that changes
   anything. Nothing is cached. Once a revocation is answered, nothing the
   token began commits any more.
3. A token never outlives its maker's role. The authenticating query also
   asks that the account that made it is an administrator now, so removing
   an administrator (or deleting the account) stops every token they know.
4. An unknown token, a revoked one, an expired one and one without the right
   are refused alike.
5. A report names the token it came from, authenticated and never claimed,
   and never looks like a gateway's: not in a list, a filter, a facet or the
   history. A token cannot file a record as a gateway's own, nor name one.
6. A change says which token made it, and no reader can take it for an
   administrator's.
7. What is secret is shown once, in the answer to its creation. Central
   keeps the SHA-256 digest and never writes token or digest to a log, an
   audit event or another answer.
8. Nothing of a body is read before its credential is known to be good.
9. The bounds that hold for a gateway hold for each token.

### Pass 2: the rights, the routes and the schema

**A token** is a [`Token`](../crates/central-api/src/token.rs) like a
gateway's: 32 random bytes, stored as their digest in `report_tokens`,
shown once as `brt_` and 64 lowercase hex digits. It has a name (the rules
of a gateway token's), its rights, a lifetime of 1 to 366 days, its maker,
its revocation and who revoked it, and when it was last used, to the minute.

| right | what it allows | needs |
| --- | --- | --- |
| `file` | file a report and its parts | – |
| `read` | search, facets, crash groups, statistics, one report with its notes and history, its parts | – |
| `triage` | status, priority, labels, issue, assignee and duplicate, one or a selection; into the trash and out of it; notes, and the removal of the token's own; the list of administrators an assignee is chosen from | `read` |
| `purge` | delete for good, from the trash only | `triage` |

The website offers four presets, "only post" (`file`), "only read" (`read`),
"read and edit" (`read`, `triage`) and "everything", and any other
combination whose needs are met. The table's CHECKs hold the same rules.

**Making and revoking one** is an administrator's change on the browser
listener (`ReportTokens`, `CreateReportToken`, `RevokeReportToken`): a
password sign-in less than 30 minutes old, at most 32 live tokens and 32 new
ones a day, and an event in the administration's audit
(`report_token.created`, `report_token.revoked`) that names the token by its
id.

**The routes** stand beside `/v1/native` on the native listener, under the
same boundary: the protocol header, the exact `Host`, no `Origin` and no
`Sec-Fetch-Site`, so no browser page reaches them.

| request | right | answer |
| --- | --- | --- |
| `POST /v1/reports` | `file` | `201 {"report_id"}` |
| `PUT /v1/reports/{id}/client`, `…/record` | `file` | `204` |
| `POST /v1/reports/search` | `read` | a page and the total |
| `POST /v1/reports/facets` | `read` | counts per dimension |
| `POST /v1/reports/crashes` | `read` | crash groups |
| `GET /v1/reports/stats` | `read` | reports per day and kind, 30 days |
| `GET /v1/reports/{id}` | `read` | one report, its notes and history |
| `GET /v1/reports/{id}/client`, `…/record` | `read` | the gzip as stored |
| `GET /v1/reports/admins` | `triage` | who a report may be assigned to |
| `PATCH /v1/reports/{id}` | `triage` | the report as changed |
| `POST /v1/reports/{id}/trash`, `…/restore` | `triage` | `204` |
| `POST /v1/reports/{id}/notes` | `triage` | the report |
| `DELETE /v1/reports/{id}/notes/{note}` | `triage` | the report |
| `POST /v1/reports/bulk` | `triage` | how many changed |
| `DELETE /v1/reports/{id}` | `purge` | `204` |
| `POST /v1/reports/purge` | `purge` | how many went |

- **The right follows from the route alone**, so it is known before the body
  is. A selection's purge has a route of its own for that reason.
- **The bearer says what it is.** `Authorization: Bearer brt_<64 hex>` is a
  report token and is looked for in `report_tokens` only; `Bearer <64 hex>`
  stays a gateway's or the bridge's and is never looked for there. The
  first two routes take both, the others a report token alone. Every other
  route refuses a `brt_` bearer, which is no credential there.
- **A filter travels in a body**, not in the address: a search may hold
  words of a report or a reporter's pseudonym, and an address ends up in
  shells' histories and proxies' logs.
- **The contract is its own** (`central_api::report_api`): ids are 32
  lowercase hex digits, unknown fields are refused, and the administration's
  wire types, which change with the website, are not it.
- **What is refused.** Before the credential is known to be good: `401` for
  everything, the same for every cause; past it, `400` for a request the
  rules refuse, `409` for a revision that moved on, `429` past a budget and
  `503` when the store is away.
- **Budgets.** Every request spends the listener's public budget and its
  address's share, as a gateway's does. Thirty refused report tokens an hour
  from one address are the last the store is asked about; the address's
  gateways are not counted or refused by it. A token files 120 reports an
  hour, its reports take 2 GiB in the store and it holds two uploads at
  once, as a gateway does; it makes 600 other requests an hour, 120 of them
  downloads, and one download at a time of two in all.

**The store** asks the same functions for both callers, an administrator's
session or a token with its rights (`ReportCaller`). A token marks nothing
as seen, keeps no saved lists and cannot filter by what it opened.

**Schema: migration 000052.** `report_tokens`; `reports.filed_by`
(`gateway`, or `token`, whose id then stands in `gateway_id` and whose name
in `gateway_label`, as it was); `report_audit.token_name` and
`report_notes.token_name`, set exactly when the acting id is a token's, with
the token's name as it was; `audit_events.report_token_id`. The previous
binary runs unchanged over all of it: it shows a token's report like a
gateway's of that name, and a token's note or change by an id it does not
know.

### Pass 3: as built

What the building decided beyond pass 2, and what the adversarial pass found
in it.

- **The boundary leaves these bodies unread.** It read every other route's
  body before the handler ran; for `/v1/reports` and below it now hands the
  body on, and the handler has it read by the boundary's own means once the
  credential is known to be good (`boundary.rs`, `Unread`): the same grace,
  the same places for late bodies, at most 64 KiB, within the request's five
  seconds. A gateway's `POST /v1/reports` is read this way too, so one with a
  wrong credential is refused earlier than before, and metadata over 64 KiB
  is `413` where it was `401`.
- **A change holds the token's row** (`FOR SHARE`) until it commits. A
  revocation therefore waits for a change that is being written, and a
  change that comes while a revocation is being written waits and is refused
  by it: once a revocation is answered, nothing the token began commits. One
  that would wait longer than the store's five seconds fails and is sent
  again; a token's changes come one at a time and its hour is bounded, so it
  can hold its row against a revocation for seconds and not for an hour. A
  filing and a part's upload, whose bodies come after the credential, ask for
  the token again in the transaction that stores them.
- **The maker's role** is asked by the authenticating query, as designed, and
  in a change once more after the token's row is locked, in a statement of
  its own: a statement that waited for the row looks at that row again and
  at no other. The removal of an administrator locks the tokens they made
  first, so it waits for what they are changing, and nothing of theirs is
  stored after it. A token whose maker is no administrator is shown as
  paused; with the role its maker's tokens work again. Revoking is final.
  (An operator's own SQL on the roles takes no such lock: a change that has
  begun still ends.)
- **What can be told apart.** Only the credential's own refusal is counted
  against an address; a request the rules refuse is not. The part's and the
  report's checks are asked after the credential, apart from it. A token
  revoked while its request is being answered is refused by the transaction
  that answers, which is `400`; its next request is `401`.
- **A token speaks for no gateway.** What a gateway says of itself (its name,
  its address, its build) is empty in a token's report, by the type and by
  the table, and the report's page calls the row it came from "Token" and
  not "Gateway". A CSV has the token's name and id in two columns of their
  own, after the old ones, and no gateway.
- **The audit's event** carries the token's id in a column of its own
  (`audit_events.report_token_id`), and a page of the audit names the token
  beside its events, not in them: `AuditEvent` is also in the account export,
  which the released Baylee client reads field by field
  (`central-api`, `export/released.rs`). A field there would have kept every
  administrator's client from reading its own name after sign-in.
- **The store's four connections.** A search reads every report's words, so
  a few of them at once could hold every connection and keep everyone's
  sign-in waiting. Report tokens have two calls in the store at once, one of
  each token (`central-http`, `slots.rs`); one more is `429` at once. A
  script sends one request after the other. The slot is taken when the body
  is there and kept until the store is done, whoever still waits: a request
  that is dropped leaves its statement running, and its connection taken,
  for up to the store's five seconds.
- **Downloads** hold their part in memory until it is sent: two at once, one
  of each token, in slots of their own. The slot is held while the part is
  sent and not only while it is read from the store.
- **A selection costs what it writes.** A change of two hundred reports
  writes as many rows and histories; it spends a request of the token's hour
  for every ten reports in it. A report that becomes a duplicate hands its
  own duplicates on, which the request does not pay for and which could be
  asked for again and again: a token's request hands on 50 in all and is
  `400` past it, before anything is written. Whoever has more of them moves
  them by a selection first.
- **A report stays readable.** One report's page is answered on the wire's
  1 MiB. Nothing bounded a report's notes together, nor the history it is
  shown with, and a script can write both faster than a person. A report's
  notes are now 192 KiB together, each weighed with 256 bytes besides its
  text, whoever writes them (`429` past it), and the newest 200 lines of its
  history are shown, with a line saying there are older ones. A facet lists
  the 200 most used labels. A token's answers have a bound of their own
  (8 MiB), which a page of two hundred reports at their largest fits.
- **A token's note** holds what a report's own words may hold: no control
  characters and none of the invisible ones that change how text reads.
- **Its name in words.** A report, a note and a line of the history keep the
  token's name as it was beside its id, as a report keeps its gateway's. Two
  tokens may have one name: the page shows the end of the id beside it, and
  *Feedback tokens* the whole.

Tests. `central-api`: the rights and their needs, the presets, the text form
and the header's two kinds and nothing else, redaction, the contract's
bounds, every answer's fields by name, and that answers without a token read
as before. `central-identity`: the hour's requests and downloads, what a
selection costs, the right each call needs. `central-http`, against a
stand-in behind the real boundary: each route's right, that no body is read
for a credential that is not good (a body that counts its polls), one answer
for every such refusal, a gateway's credential never asked about where a
token is, `400` and `401`, the refusals' budget by address and that a
gateway's are not in it, one call and one download at a time, the call's slot
kept until the store is done when its caller goes away and not taken by a
body on its way, the body's bounds, the protocol and a page's `Origin`, and
that no answer holds a credential. PostgreSQL
(`tests/support/accounts/reports/tokens.rs`): every set of rights a token
can hold against every act, eighteen of them; a token revoked, over or
without its maker; a revocation between the credential and the storing, and
one while a change runs; the end of the maker's role while a change runs, and
a change while that is being written; who may make and revoke; the audit; the
bounds on tokens, notes, history, labels and what a request hands on; the
filters, facets and history that tell a token
from a gateway of the same name; the store's bound; the checks of the table
and the runtime's exact grants both ways. Over real HTTP with PostgreSQL
(`central-client`, `tests/support/report_tokens.rs`): the whole way from the
administrator's browser session to filing, reading, triage and purge, the
refusals, and a token on the routes that are not its own.

**A review by a second agent** read the branch at `468c561b` without the
implementer's notes, and found nothing high, two of middle weight and seven
small ones. The two: a caller who hung up gave the slot back while the store
worked on, so that hanging up again and again held more than two of its
connections; and a report that became a duplicate wrote a line for each of
its own duplicates, unpaid and as often as asked. The small ones: a note of
one letter cost more than it was weighed as; a body on its way held a slot;
the end of the maker's role did not wait for a change; a token's report
could name a gateway; the labels' facet had no bound; an administration tab
opened before the deployment cannot read what a token filed; tokens of one
name could not be told apart. All of them are changed or, the tab, said in
[operations](operations.md), as the points above describe. It was the same
model as the implementer, so it is no independent audit.

**What was not done.** It has not run on the server. The memory of a download
and the time of a search over a full store are by reading the code and were
not measured. The browser specs were written and not run on the machine it
was built on (no npm packages there); CI runs them. These passes are the
implementing agent's and are not an audit.

### Decided by the owner

Asked on 25.11.2026, the owner settled the four questions the first build
left open:

- **Hosted AI agents may read reports.** The owner alone manages feedback.
  By the owner's decision of 25.11.2026, report tokens that may read may
  also go to hosted AI agents (such as Claude by Anthropic or Codex by
  OpenAI) that triage reports for the owner. The provider then receives
  players' words, screenshots and records and is a processor (Art. 28
  GDPR), in the USA. Before the first read token goes to such an agent,
  the provider's commercial terms with their data processing agreement
  must apply (consumer plans have none, and may train on inputs), and the
  privacy policy must name the provider as a recipient and the transfer
  (version 2026-11-26, drafted in a pull request of its own). Until then
  read tokens go only to the administration's own machines.
- **A purge by a token** needs no password, since a token has none: the
  preset "everything" may delete for good, from the trash only, and each
  report it deletes leaves a line naming the token in `report_audit`. Leave
  `purge` out where it is not needed.
- **Sixteen GiB for all reports** stays the bound for everyone together,
  beside each token's own budget. Eight tokens at their two GiB fill it, and
  then every gateway's reports are refused too. The four uploads at once are
  shared the same way: two tokens that use their two each leave a gateway
  none for that long. Tokens that file are an administrator's to hand out.
- **The translations** besides English and German do not hold the change
  back; a speaker reads them later.
