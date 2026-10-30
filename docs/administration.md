# Central administration

*28.10.2026, roadmap M1 step 2a.* The server side of the accounts and
administration design's step 5 (branch `c41/accounts-admin-design`,
`docs/accounts-admin-proposal.md`, with the owner's answers of 22.10.2026):
roles, the first administrator, an append-only audit with its sweep, the
last sign-in time, and the administrator's reads and role changes. The
pages (`/admin` in the web app) are M4; locks and bans, beta keys and
gateway approval on the website and reports are the design's later steps.
Every piece goes through the three passes of [security
assurance](security-assurance.md).

## Rules

- **One request, on the browser listener only.** `Request::Admin { request:
  AdminRequest }` answers `Response::Admin { answer: AdminAnswer }` for a
  read and `Done` for a change. It needs a browser session (the
  `__Host-central` cookie) whose account holds the administrator role, and
  Central's terms accepted. The native listener refuses it before anything
  else (`browser_only`); a native bearer is no browser session in the store
  either.
- **The role is checked where it acts.** Each read checks the
  administrator in its own transaction; a change locks the administrators'
  account rows first and checks the acting role after the lock, in a fresh
  statement. A removed administrator's next request fails.
- **A change needs a recent password.** The browser session's password
  sign-in (`sessions.created_at`, which renewal never moves) must be less
  than 30 minutes old; reads need not. The page knows the sign-in time
  from `AccountSummary` and asks for the password again, which signs in
  anew. TOTP for administrators' changes comes with M1 step 5.
- **The audit names ids, never words.** An event holds the actor (an
  administrator's account id, or the operator), the action from a closed
  list in `central-postgres` (`Action`: `account.admin_granted`,
  `account.admin_removed`, `account.reset_issued`, `account.tag_changed`,
  `gateway.approved`, `gateway.disabled`, `gateway.token_revoked`) and the
  ids it touched. No name, reason, note or key; a tag change keeps the two
  tags, which are numbers. Reads are not audited. Each change writes its
  event in the same transaction.
- **The runtime decides, and cannot rewrite the record** (decision 4). It
  holds `SELECT, INSERT, DELETE` on `account_roles` and `SELECT, INSERT` on
  `audit_events`, and its startup check refuses any wider or narrower grant
  on either. Deleting events after three years is the operator's sweep.

## Roles

An administrator appoints and removes administrators (`SetAdmin { account,
admin }`). Nobody removes their own role, so the last administrator stays;
only an enrolled account is appointed (a pending or unknown one is
`Rejected`). The same state again is `Done` and writes nothing. Two
administrators removing each other at once leave one: both lock the
administrators' account rows in id order, the second sees after the lock
that it no longer holds the role, and is refused.

**The first administrator** is appointed by the operator:
`central-service --appoint-admin-stdin` with `{"database": {…the migrator
login…}, "username": "…"}` on standard input ([operations](operations.md)
§"Administration"). It appoints that enrolled account, writes an audit event
with the operator as its actor, and says so on standard error; again, it
changes nothing. It is also the way back in when no administrator can sign
in. The owner's account is the first once it has moved to Central; the
username arrives at run time, so the repository names nobody.

A second change, `ResetLogin { account }`, makes a reset code
([account reset](account-reset.md), M1 step 2b), audited as
`account.reset_issued`.

## Reads

- `Accounts { query, before }` → `Accounts { accounts, more }`: newest first,
  50 a page, `before` the last id of the page before. The query
  (`AccountQuery::parse`) is `#` with one to eight hex digits (a tag), or 1
  to 64 bytes of printable ASCII, trimmed and compared without case against
  the username and the display name as a plain substring (`strpos`, never a
  pattern). None lists all. Each row: id, username, display name, tag,
  source, state (`pending`, `enrolled`, `restored`), created, last sign-in,
  administrator, and an outstanding reset code's expiry (since step 2b).
- `Account { account }` → `Account { account, recovery, gateways, audit }`:
  the row, whether a recovery wrapper exists, the gateways it owns (id,
  origin, approved; at most 20) and its latest 20 audit events. An unknown
  id is `Rejected`.
- `Audit { account, before }` → `Audit { events, more }`: newest first, 50 a
  page, of one account or all.

The queries name their columns and never read a credential, a wrapper or a
document; the answer types have no field that could carry one.

*Accepted limit:* a read checks the role at its start, at `READ COMMITTED`,
so a removal that commits while a read runs lets that one read finish; the
removed administrator's next request is refused. Changes check after their
lock and are not affected.

## Gateways, tokens and tags (30.10.2026)

The design's step 7 for gateways, and the owner's request of 30.10.2026 that
administrators edit tags. Every change below needs the recent password and
writes its audit event in its transaction; reads do not.

- `Gateways { state, before }` → `Gateways { gateways, more }`: newest first,
  50 a page, all or those `pending`, `approved` or `disabled`. Each row: id,
  origin, metadata, official, the owner's id, display name and tag, state,
  online, last heartbeat, created, live tokens, and whether it still has a
  `RegisterGateway` credential pair. `Gateway { gateway }` adds its latest 20
  tokens (name, scopes, times; never a token or digest).
- `SetGatewayApproval { gateway, enabled }`: approve or enable again
  (`gateway.approved`), or disable (`gateway.disabled`). Approval stays a
  person's decision after checking outside Central who runs the origin; the
  page says so in its dialog. Disabling hides the gateway from the directory
  at once, stops its credentials and tokens (the joins that authenticate
  them), and deletes its leases, so each of its sessions fails at its next
  check. The official gateway's row can be disabled too, which hides it; its
  bridge is no approval's to stop. The same state again writes nothing.
- `RevokeGatewayToken { token }` (`gateway.token_revoked`, with the gateway's
  id): the token stops at once. Revoked already is `Done`.
- `SetTag { account, tag }` (`account.tag_changed`): an enrolled account
  gets another tag, 1 to `MAX_TAG`. Tags stay unique (the column's unique
  index; a taken tag is `Conflict`), so `Name#tag` does. The event's
  `detail` holds `{"from": old, "to": new}`, numbers only. Gateways take the
  new tag into their projection at the account's next sign-in there
  (Baylee's `take_profile`); whispers on a gateway are keyed by tag, so
  earlier whisper threads may not show under the new one: the dialog warns.

The runtime gains `INSERT` and `UPDATE (enabled)` on `gateway_approvals`
(never `DELETE`) and `SELECT, INSERT, UPDATE (revoked_at, last_used_at)` on
`gateway_tokens`; the startup check names updatable columns as a list now and
refuses any other column.

## Schema: migration `m20261028_000012_administration`

`account_roles (account_id → accounts ON DELETE CASCADE, role CHECK IN
('admin'), granted_at, granted_by)`, primary key `(account_id, role)`;
`audit_events (id uuidv7, at, actor_kind CHECK IN ('admin', 'operator'),
actor_account, action CHECK ~ '^[a-z_]{1,32}\.[a-z_]{1,32}$', account_id,
gateway_id, beta_key_id, report_id, detail jsonb ≤ 1024 bytes)`, with
`actor_account` present exactly for an administrator, no foreign keys (an
event outlives what it names), indexed by account and by time;
`accounts.last_sign_in_at`, set at every password sign-in of either kind
(and by a password change), never by renewal. Only the time, no address.
`detail`, `beta_key_id` and `report_id` are for the design's later steps;
nothing writes them yet. The previous binary, which the deploy hook
restarts when a new one fails, runs unchanged over it.

## The sweep

`central-service --sweep-stdin` with the migration document deletes audit
events older than three years, as the privacy notice allows, and (since
step 2b) the digests of expired reset codes, in one transaction, and prints
both counts on standard error. A daily systemd timer runs it ([operations](operations.md)
§"Administration"). The runtime cannot.

## Departures from the design

- The design's `SetAdmin` locked the role rows; `SELECT … FOR UPDATE` needs
  an `UPDATE` grant there, which the runtime does not hold. It locks the
  administrators' account rows instead (it may update `accounts`), in id
  order so two changes cannot deadlock.
- `Accounts` searches no account id; `Account` takes one.
- The account row has no restriction yet, and `Account` no restrictions or
  reports: they come with the design's steps 6 and 8. The admin answers are
  the browser's alone and change together with `web/`, so adding fields
  then breaks no gateway or client.
- The native client has no administration (the SDK has no call): the
  owner's design serves it on the website only.

## Before it goes live

- The privacy notice's new version (roles, the last sign-in time, the audit
  of administrators' actions and its three years): a draft is for the
  owner's approval; `legal/` is unchanged.
- The owner's account moved to Central and appointed; the sweep's timer
  installed.
