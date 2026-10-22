# Names, recovery, the directory and administration: a proposal

Status: design only, nothing built. It turns the owner's decisions of
22.10.2026 into steps that each land as their own reviewed PR and go through
the three passes of [security assurance](security-assurance.md). It assumes
session renewal (PR #32, migration 000008) has merged: `sessions.created_at`
is the moment of the password sign-in, and renewal never moves it. No step
needs a new dependency, in Rust or in `web/`.

## The owner's decisions (22.10.2026)

1. **Display name and tag live at Central**, one per account, drawn as the
   client draws them (`Alice#0007`). A moved account keeps its gateway's name
   and tag. A new account picks a display name at registration; the tag is
   drawn. Every gateway shows Central's name and tag for Central accounts.
   Renaming on the website comes later, but the design allows for it.
2. **Central administration**, first step. The owner's account becomes the
   first administrator through a one-time operator command. An administrator
   appoints and removes administrators, locks and bans accounts, issues and
   revokes beta keys, approves and disables gateways, and sees the accounts
   (username, display name, created, source, last sign-in; nothing
   encrypted). A gateway's own console bans on that gateway only, and can
   report an account to Central, whose administrators decide.
3. **Two faults** from the owner's list: moved accounts have no recovery code
   and the website tells them to keep one safe; the website's gateway list
   does not work and would not list the official gateway.

## What exists today

- `browser_operation` (`crates/central-identity/src/lib.rs`) refuses
  `GatewayDirectory`. `web/src/central/directory.ts` posts to
  `/api/v1/native`, a path the browser listener answers 404, and sends no
  `X-Baylee-Central` header.
- The official gateway is the bridge: it has no `gateways` row, and
  `gateway_authority` (`crates/central-identity/src/gateways.rs`) answers the
  compiled `LEGACY_ORIGIN` for the bridge credential.
- Migration 000007's `account_recovery` CHECK forbids a recovery wrapper on a
  legacy account; no request sets one after enrollment. The account page
  (`web/src/account/AccountPage.tsx`, string `account.recovery`) tells every
  account to keep its code safe.
- Central stores no display name, tag, role or sign-in time.
- A gateway makes a Central account a projection named `Player` with the
  gateway's own tag (`baylee_db::central::projection` in
  `crates/baylee-db/src/central/sessions.rs`). No route renames an account,
  and `crates/baylee-gateway/src/namebook.rs` caches handles for good on that
  basis.
- A gateway tag is unique on its own and drawn as lowercase hex padded to
  four (`crates/baylee-gateway/src/handle.rs`); `Alice#0007` is tag 7. It is
  part of chat identity: a whisper is the conversation `dm:<tagA>:<tagB>`
  (`baylee-protocol` `social::Conv`). A gateway account's tag must never
  change.
- Beta keys: `--issue-beta-keys-stdin` and an operator `UPDATE`. Gateway
  approval: an operator `INSERT`. The runtime role can do neither, and its
  startup check refuses to run if it could
  (`crates/central-postgres/src/accounts/grants.rs`).
- Gateways ban locally already (`crates/baylee-gateway/src/moderation.rs`,
  baylee-db migration 000016), and `allowed()` in
  `crates/baylee-gateway/src/central/routes.rs` refuses a banned projection at
  its Central sign-in.

## Rules every step keeps

**Wire compatibility.** Every answer refuses unknown fields
(`deny_unknown_fields`), so a field added to an existing answer breaks each
gateway and client built before it. New facts travel in new requests and new
answers. A request may gain an optional field (`#[serde(default)]`), as
`EnrollmentFinish.recovery` did, because Central is always the newer side: the
deploy hook installs the Central revision a gateway links before that gateway
starts (`docs/operations.md` §"Deploying with the gateway"). The browser
answers change only together with `web/`, which ships from the same revision.

**Administration is one request.** `Request::Admin { request: AdminRequest }`
answers `Response::Admin { answer: AdminAnswer }` or `Done`. It is served on
the browser listener only, with a browser session whose account holds the
admin role, and like every account action it needs Central's terms accepted
(`needs_acceptance` in `crates/central-identity/src/terms.rs`).
`browser_operation` names it once; an exhaustive match over `AdminRequest` in
a new `crates/central-identity/src/admin.rs` decides which actions are
changes. The native listener refuses it.

**A change needs a recent password.** An administrator's change needs a
browser session whose `created_at` (the password sign-in) is less than 30
minutes old; renewal does not count. Reading lists does not. The page knows the
sign-in time from `AccountSummary` (step 2) and asks for the password again
before acting, which signs in anew and replaces the browser session (one per
account and kind). When TOTP arrives (roadmap M1), administrators' changes
require it too.

**The audit names ids, never words.** An audit event holds the actor, the
action and the ids it touched. It never holds a name, a reason, a note or a
key, which may name a person, as the gateway's audit line does not
(`docs/protocol.md` in Baylee, §"Acting on an account"). Reads are not
audited.

**Who writes what an administrator decides** is owner question 1. This
proposal recommends a separate database login, `central_admin`, used only by
the admin requests (step 5). Every grant below is written for that choice.

## 1. The gateway directory on the website (S, plus S in Baylee)

**What people see.** `/gateways` lists the approved gateways. The official
gateway comes first with an "Official" mark (the page knows its origin from
`web/src/config.ts`), the rest in Central's order. Online means Central had a
heartbeat in the last 90 seconds, as today.

**Wire.**

- `GatewayDirectory { after }` joins `browser_operation`. It uses no session
  (`uses_session` stays false) and gets a per-address budget of 60 a minute.
- `GatewayHeartbeat { metadata }` also accepts the bridge credential. It then
  updates the official row only, under the same five-second limit, with
  `metadata.validate(LEGACY_ORIGIN)`.
- `directory.ts` posts to `${CENTRAL_API}/v1` with `X-Baylee-Central: 1` and
  `credentials: "omit"`; every check of the answer stays.

**Schema: migration 000009 `official_gateway`.**

```sql
ALTER TABLE central_identity.gateways ADD COLUMN official boolean NOT NULL DEFAULT false;
ALTER TABLE central_identity.gateways ALTER COLUMN owner_id DROP NOT NULL,
    ALTER COLUMN credential_digest DROP NOT NULL, ALTER COLUMN heartbeat_digest DROP NOT NULL;
ALTER TABLE central_identity.gateways ADD CONSTRAINT gateway_official CHECK (
    official = (owner_id IS NULL) AND official = (credential_digest IS NULL)
    AND official = (heartbeat_digest IS NULL));
CREATE UNIQUE INDEX gateways_one_official ON central_identity.gateways ((true)) WHERE official;
```

The official row has no owner and no credential: the bridge stays the only
authority for its origin. A row registered with `RegisterGateway` for that
origin would hold a second redemption credential for the audience the bridge
already serves, so tickets for the official gateway could be redeemed twice
over; the official row avoids that. The store treats the official row as the
official gateway only while its origin is `LEGACY_ORIGIN`, and
`register_gateway` refuses that origin outright (`Conflict`), where today's
`ON CONFLICT` path would answer an unexplained `Busy`. Until step 7 the
operator lists it, under the migrator login:

```sql
INSERT INTO central_identity.gateways (official, origin, name, version, website)
    VALUES (true, 'https://baylee.acevik.de', 'Baylee', '0', 'https://baylee.acevik.de/');
INSERT INTO central_identity.gateway_approvals (gateway_id)
    SELECT id FROM central_identity.gateways WHERE official;
```

The first heartbeat replaces the name, version and website. The entry is
`https://baylee.acevik.de` until the official gateway moves to
`https://gateway.baylee.app` (roadmap M5). That move also moves the bridge's
compiled audience; listing the new name earlier would hand clients an origin
Central issues no tickets for.

**Grants.** None new. The runtime keeps `SELECT, INSERT, UPDATE` on
`gateways` and `SELECT` on `gateway_approvals`. It could insert an official
row, but not approve one.

**Baylee (G1).** `Settings::validate` in `crates/baylee-gateway/src/central.rs`
accepts `metadata` beside `bridge` (still never `origin`, `credential` or
`heartbeat`), and `Heartbeat::start` runs in bridge mode, sending
`GatewayHeartbeat` with the bridge credential. The official gateway's
`BAYLEE_CENTRAL_CONFIG_FILE` gains `metadata`. Without it, the entry is listed
and shown offline.

**Tests.**

- `central-identity`: the exhaustive operation lists move `GatewayDirectory`
  to the served list.
- `central-http`: the directory answers on `/api/v1`; the per-address budget.
- `central-postgres`: the upgrade test from 000008 with registered gateways;
  one official row at most, without owner or credentials; a bridge heartbeat
  updates the official row only; no registered credential authenticates it;
  listed when approved, hidden when disabled.
- `web/test/central` and `web/test/gateways`: the request's path and headers,
  the Official mark.
- Baylee: the settings test (bridge with metadata accepted, bridge with
  `credential` or `heartbeat` refused); the heartbeat in bridge mode in
  `cargo dev central integration`.

**Docs.** `gateway-registry.md` §"Discovery", `operations.md` (the SQL and
the budget), `web-accounts.md` (operations and budgets), `web/README.md`
§"The gateway directory" and §"Known gaps", Baylee's `docs/central-login.md`.

## 2. The account page and recovery codes for moved accounts (M)

**What people see.** The account page shows who is signed in, where the
account came from, and its recovery state.

- With a recovery code (every registered account, and a moved one that made
  one):
  - en: "You have a recovery code. With it, your encrypted data (settings,
    deck backups, chat history) survives a forgotten password, once password
    reset exists. Keep it where only you can reach it: we cannot show it
    again."
  - de: "Du hast einen Wiederherstellungscode. Mit ihm überstehen deine
    verschlüsselten Daten (Einstellungen, Deck-Sicherungen, Chatverlauf) ein
    vergessenes Passwort, sobald es das Zurücksetzen gibt. Bewahre ihn so
    auf, dass nur du ihn erreichst: Wir können ihn dir nicht noch einmal
    zeigen."
- Without one (a moved account), with a button "Make a recovery code":
  - en: "Your account has no recovery code yet. It moved here from the
    official gateway, which had none. If you forget your password without
    one, your encrypted data is lost."
  - de: "Dein Konto hat noch keinen Wiederherstellungscode. Es ist vom
    offiziellen Gateway hierher umgezogen, das keinen kannte. Vergisst du
    ohne ihn dein Passwort, sind deine verschlüsselten Daten verloren."

The code is made in two places. Right after a sign-in: once `LoginFinish` has
answered, the sign-in page asks `AccountSummary` while the worker still holds
the sign-in's state. `recovery_create` (below) stands in for `login_confirm`:
it proves the password the same way, by unwrapping, so the page calls exactly
one of the two. For an account without a code it calls `recovery_create` at
once and keeps only the code and the 73-byte wrapper, never the key, until the
person decides; "Not now" discards both and nothing reaches Central. Later, on
the account page, the button asks for the password again ("To make the code,
sign in again") and signs in anew the same way. Either way the page shows the
code with `RecoveryStep.tsx` (copy, download, "I kept it"), and only after
that confirmation sends it. The page keeps the 73 bytes until Central
answers, so a lost answer is retried with the same bytes.

**Wire.**

- `AccountSummary` (browser, cookie, no terms needed) → `Response::
  AccountSummary { identity, username, display_name: Option<String>, tag:
  Option<u32>, source: legacy | registered, recovery: bool, admin: bool,
  signed_in_at: i64, expires_at: i64 }`. Steps 3 and 5 fill `display_name`,
  `tag` and `admin`; the shape is complete from this step, so it changes once.
- `SetRecovery { recovery }` (browser, cookie, terms accepted) → `Done`.
  `recovery` is 73 bytes. Central stores it when the account is enrolled, has
  finished moving (`legacy_closed` or registered), has no wrapper, and the
  browser session's sign-in is less than 10 minutes old. The same bytes
  again are `Done` at any time, so a slow retry after a lost answer still
  succeeds; other bytes while one exists are `Conflict`. A stolen cookie can
  therefore not put a wrapper on an account long after its owner signed in.
  Replacing a code comes with password reset (M1).

**Browser crypto.** `central-web-crypto` gains `recovery_create(state:
LoginKey, wrapped_master) -> Recovered`, with the getters `recovery_code`
(wiped when freed, as in `Enrolled`) and `recovery`. It unwraps the master
key with the sign-in's wrapping key, failing as `login_confirm` does, makes a
`RecoveryCode` and wraps the same master key under it with the account's
context (`crypto-protocol.md` §"Recovery code"; nothing changes there). In
`flow.rs` it is `recovery_create(SignedIn, &[u8])`, and the worker
(`web/src/account/crypto-worker.ts`) gains the step. `cargo xtask wasm`
rebuilds `web/src/central-wasm/` and its sums. The master key exists only
inside one flow's worker, as today (`web-accounts.md`): that is why the code
is made right after a sign-in and not later from a stored key.

**Schema: migration 000010 `legacy_recovery`.**

```sql
ALTER TABLE central_identity.accounts DROP CONSTRAINT account_recovery;
ALTER TABLE central_identity.accounts ADD CONSTRAINT account_recovery CHECK (
    CASE
        WHEN credential IS NULL THEN recovery_wrapped_master IS NULL
        WHEN source = 'registered' THEN recovery_wrapped_master IS NOT NULL
        ELSE true
    END
);
```

`invalidate_legacy` also clears `recovery_wrapped_master`, although
`SetRecovery` already refuses an unfinished move. The wrapper is bound to the
vault epoch: whatever later raises the epoch (an administrator's reset, M1)
must clear it in the same statement.

**Grants.** None new: the runtime updates `accounts`.

**Baylee and the client.** Nothing. Making the code in the client is M6.

**Tests.**

- `central-postgres`: the upgrade test from 000009 with legacy and
  registered rows; `SetRecovery` for a moved account (stored, repeated,
  different bytes, a sign-in older than 10 minutes, an unfinished move, a
  registered account); `invalidate_legacy` with a wrapper.
- `central-web-crypto`: the wrapper unwraps with the code to the master key
  the password wrapper holds; a wrong key fails; the state is consumed.
  `cargo xtask wasm --check` in CI.
- `web/test/account`: `parse.accountSummary` takes exactly its shape; both
  wordings; both ways to make the code; retry with the same bytes.
- `web/e2e-real`: a moved account (a synthetic legacy enrollment) makes a
  code.

**Docs.** `web-accounts.md` (the operations, the export, the account page),
`crypto-protocol.md` (a code made after enrollment), `status.md`.

## 3. Display name and tag at Central (M)

**What people see.**

- Registration gains "Display name" below the username: "How other players
  see you: 3 to 16 letters, digits, - and _. Your tag is drawn for you."
  After registration the recovery step shows the whole handle, `Name#3fa2`.
- The header and the account page show the handle. The username is the
  private sign-in name (`Discover` treats it so); the account page shows it
  small, as "Sign-in name", and nowhere else shows it.
- An account without a display name (registered before this step) sees
  "Choose your display name" first on the account page. Until then gateways
  show it as `Player` with its Central tag.
- A moved account keeps the name and tag it had at the official gateway.

**Rules** (a new `central-api` module, `profile`):

- A display name keeps the gateway's rule exactly
  (`crates/baylee-gateway/src/auth.rs` `valid_display_name`): 3 to 16 ASCII
  letters and digits, with `_` and `-` only inside; case kept. Both
  repositories test the same table of cases.
- A new name may not be one of `admin`, `administrator`, `baylee`, `central`,
  `moderator`, `official`, `support`, `system`, compared without case, `_`
  and `-`. Carried legacy names are kept as they are.
- Names are not unique; the tag tells two apart.
- A tag is unique at Central, positive, at most `i32::MAX` (a gateway's
  column), shown as `handle.rs` shows it. It never changes once drawn,
  because chat on gateways is keyed by tag; a rename changes the name only.
- A registered account draws its tag at random from `0x1000..=0xffff` while
  fewer than half of those are taken, then from `0x10000..=0xfffff`, and so
  on. A taken draw is retried up to eight times, then the request is `Busy`.
  `REGISTERED_TAG_FLOOR = 0x1000` lives in `central-api`. Random rather than
  counted, so a tag says nothing about how many accounts exist.
- A moved account keeps its gateway tag. Those are the official gateway's
  identity values, below the floor. Before this step goes live, the operator
  checks the official gateway's highest tag, guests included
  (`SELECT max(tag) FROM account`); if it is not below `0x1000`, the floor
  moves to the next multiple of `0x1000` above it. The official gateway makes
  no local accounts once bound to Central, so the number cannot grow
  afterwards. If a carried tag is taken anyway, the account keeps its name
  and draws a new tag (a test covers it).

**Wire.**

- `Register { username, beta_key, display_name: Option<String> }`. The page
  always sends the name; absent means "choose later". A malformed or
  reserved name is `Rejected`, after the key check. The tag is drawn with the
  reservation, and an abandoned registration frees both.
- `SetDisplayName { display_name }` (browser, cookie, terms accepted) →
  `Done`, only while the account has none in this step (renaming is step 9).
- `AccountSummary` carries `display_name` and `tag`.
- `LegacyReserve { legacy_id, username, display_name: Option<String>, tag:
  Option<u32> }`: the bridge sends both from the legacy row; a renewed
  reservation of a pending account refreshes them.
- `LegacyProfile { legacy_id, display_name, tag }` (bridge only) → `Done`,
  for a legacy account reserved before this step: it sets name and tag once.
  `Conflict` when the account has a name already.
- `GatewayProfile { lease, purpose }`, under a gateway's redemption
  credential or the bridge, as `GatewaySession` → `Response::GatewayProfile {
  identity, display_name: Option<String>, tag: Option<u32> }`. It answers
  only for a live lease the asking gateway holds: a gateway learns the names
  of the people who sign in there, nobody else's. It is a request of its own
  because a field on `GatewayIdentity` would break every gateway built
  before it.

**Schema: migration 000011 `profiles`.**

```sql
ALTER TABLE central_identity.accounts
    ADD COLUMN display_name text
        CHECK (display_name ~ '^[A-Za-z0-9][A-Za-z0-9_-]{1,14}[A-Za-z0-9]$'),
    ADD COLUMN display_name_at timestamptz,
    ADD COLUMN tag integer UNIQUE CHECK (tag > 0);
ALTER TABLE central_identity.accounts ADD CONSTRAINT account_display_name_time
    CHECK ((display_name IS NULL) = (display_name_at IS NULL));
```

The migration draws a tag for every registered account (a `DO` block over
`random()`, retrying until free; tags are public, so `random()` is enough).
Legacy rows stay without one until the bridge carries it. `tag` stays
nullable so that the previous binary, which the deploy hook restarts when a new
one fails to start, can still register (000007's rule); the new binary draws a
missing tag at the account's next sign-in.

**Grants.** None new.

**Tests.**

- `central-api`: the name table, reserved names, tag text (`7` → `0007`,
  `0x10000` → `10000`), and that `Register` and `LegacyReserve` from before
  still decode.
- `central-postgres`: the upgrade from 000010 with the backfill (distinct
  tags at or above the floor, legacy rows empty); widening and retries, with
  the range injected small; `LegacyReserve` stores both; `LegacyProfile`
  once only; `GatewayProfile` refuses another gateway's lease and an expired
  one; a taken carried tag draws a new one.
- `web/test/account`: the field and its rules, the handle in the header,
  choosing a name.
- `central-client` `tests/support/browser.rs`: register with a name, see it
  in `AccountSummary`.

**Docs.** `web-accounts.md` (registration, account page), `native-login.md`
(the bridge's reservation), `gateway-registry.md` (`GatewayProfile`),
`operations.md` (the floor check), `status.md`.

## 4. Central's names on gateways (M, Baylee)

**What people see.** On every gateway a Central account appears as Central's
`Name#tag` in the lobby, on seats, among friends, in chat and in the console.
Moved accounts on the official gateway look exactly as before. An account
without a name shows as `Player` with its tag.

**One tag column.** A gateway keeps one tag per account, because chat is keyed
by it. A new projection is inserted with Central's tag as its `account.tag`
(the column is `GENERATED BY DEFAULT`, so an explicit value is allowed). An
existing projection takes Central's tag at its next admission only when that
tag is free on the gateway and no whisper on the gateway names the
projection's old tag yet (no `dm:` conversation with it); otherwise it keeps
its own tag, with Central's name. On the official gateway
Central's tags are always free (the floor). Local sign-up and guest creation
retry with the next sequence value when the drawn tag is taken, because an
explicit tag does not move the identity sequence (the comment on the tag
column in baylee-db migration 000001 warns of exactly that).

**Changes.**

- `baylee_db::central::projection` takes the profile and inserts
  `display_name` (or `Player`) and the tag.
- `baylee_db::central::set_profile(db, account, name, tag) -> changed`
  updates a `central_only` row by the rule above. A name failing
  `valid_display_name` is stored as `Player`: the gateway does not trust
  Central's check.
- `login_for` (`crates/baylee-gateway/src/central/routes.rs`) asks
  `GatewayProfile` after `redeem`. A refusal, or an older Central that does
  not know the request, leaves the row as it is. In bridge mode, a linked
  legacy account whose answer has no tag first gets `LegacyProfile` with the
  row's own name and tag.
- `migrate` sends `display_name` and `tag` with `LegacyReserve`.
- `NameBook` gains `forget(account)`, called when `set_profile` changed
  something. Its module comment changes to "a handle changes only at a
  Central account's admission".

**Client.** Nothing: it shows what the gateway's sign-in answer and lobby say.

**Tests.** baylee-db: `set_profile` and the tag rule. `e2e_account`: a
projection takes Central's name and tag; a changed name shows after the next
admission; an older Central keeps `Player`; a local sign-up whose drawn tag is
taken gets the next one. `cargo dev central integration`: a move carries name
and tag; `LegacyProfile` fills an account reserved without them. The NameBook
test of `forget`.

**Docs.** Baylee's `docs/central-login.md` (projections),
`docs/protocol.md` (handles of Central accounts), the namebook comment.

## 5. Administration: roles, the admin login, audit, the account list (M)

**What people see.**

- The header shows "Admin" to an administrator (`AccountSummary.admin`).
  `/admin` is its own lazily loaded chunk of the one web app (roadmap
  §"Decisions"), mobile first, every word in Fluent (`admin.*`, en and de).
- `/admin/accounts`: a search box (part of a username or display name, a
  `#tag`, or an account id), newest first, 50 a page. Each row: handle,
  sign-in name, source (moved or registered), created, last sign-in, and
  marks (Administrator, Pending, and from step 6 Locked or Banned until …).
- `/admin/accounts/<id>`: the same facts, whether a recovery code exists, the
  gateways it owns, its audit events; "Make administrator", "Remove
  administrator". Steps 6 and 8 add restrictions and reports. Nothing
  encrypted is shown, because Central holds nothing it could read.
- `/admin/audit`: newest first, filtered by account if wanted.
- A change older than 30 minutes after the sign-in opens a password prompt
  in the dialog first.

**Rules.** An administrator cannot remove their own role, and the last one
cannot be removed. Only an enrolled account can be appointed. The role is
checked in the transaction that acts, so a removed administrator's next
request fails.

**The first administrator.** A new launcher mode,
`central-service --appoint-admin-stdin`, reads `{"database": {…the migrator
login…}, "username": "…"}` from standard input, appoints that enrolled
account, writes an audit event with the actor `operator`, and says so on
standard error. Appointing an administrator again succeeds and changes
nothing. It is added to `main.rs`'s match and `production.rs` like
`--issue-beta-keys-stdin`, and is also the way back in when no administrator
can sign in. The owner's account is the first, once it has moved to Central.
The username arrives at run time, so the repository names nobody.

**Wire.** `AdminRequest` (tagged by `action`, snake case, unknown fields
refused):

- `Accounts { query: Option<String>, before: Option<[u8; 16]> }` →
  `AdminAnswer::Accounts { accounts, more }`, each `{ id, username,
  display_name, tag, source, state: pending | enrolled | restored,
  created_at, last_sign_in_at, admin, restriction }`
- `Account { account }` → `AdminAnswer::Account { … the row, recovery,
  gateways, restrictions, reports, audit }`, each list bounded
- `SetAdmin { account, admin: bool }` → `Done`
- `Audit { account: Option<[u8; 16]>, before: Option<[u8; 16]> }` →
  `AdminAnswer::Audit { events, more }`

**Schema: migration 000012 `administration`.**

```sql
CREATE TABLE central_identity.account_roles (
    account_id uuid NOT NULL REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    role text NOT NULL CHECK (role IN ('admin')),
    granted_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    granted_by uuid,  -- NULL: the operator command
    PRIMARY KEY (account_id, role)
);
CREATE TABLE central_identity.audit_events (
    id uuid PRIMARY KEY DEFAULT pg_catalog.uuidv7()
        CHECK (pg_catalog.uuid_extract_version(id) IS NOT DISTINCT FROM 7),
    at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    actor_kind text NOT NULL CHECK (actor_kind IN ('admin', 'operator')),
    actor_account uuid,
    action text NOT NULL CHECK (action ~ '^[a-z_]{1,32}\.[a-z_]{1,32}$'),
    account_id uuid, gateway_id uuid, beta_key_id uuid, report_id uuid,
    detail jsonb CHECK (octet_length(detail::text) <= 1024),
    CHECK ((actor_kind = 'admin') = (actor_account IS NOT NULL))
);
CREATE INDEX audit_events_account ON central_identity.audit_events(account_id, id)
    WHERE account_id IS NOT NULL;
ALTER TABLE central_identity.accounts ADD COLUMN last_sign_in_at timestamptz;
CREATE VIEW central_identity.account_overview AS
    SELECT id, username_key, display_name, tag, source, created_at, last_sign_in_at,
           credential_revision, credential IS NOT NULL AS enrolled, restored,
           recovery_wrapped_master IS NOT NULL AS recovery
    FROM central_identity.accounts;
```

The closed list of actions lives in Rust, so later steps add actions without
a migration. Audit ids carry no foreign keys: an event outlives what it names,
and a deleted account leaves an id that no longer resolves. `issue_session`
sets `last_sign_in_at` for both kinds; renewal does not. Only the time is kept,
no address.

**Grants: the admin login** (recommended; owner question 1). A new role,
`central_admin` (`LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION
NOBYPASSRLS`), with exactly:

- `SELECT` on the view `account_overview`, never on `accounts` itself, so it
  cannot read a credential record, a wrapper or a document
- `SELECT` on `sessions` (for its own actor check) and `gateways`
- `SELECT, INSERT, DELETE` on `account_roles`; `SELECT, INSERT` on
  `audit_events`

The runtime gains only `SELECT` on `account_roles` (for `AccountSummary`).
`central.json` gains an optional `"admin": {"database": {…}}`, a full database
block for that login; without it, admin requests answer `Unavailable` and
everything else runs. The startup check verifies both logins exactly, and it
still refuses the runtime any write on approvals, roles or restrictions and
any issuing of beta keys. `pg_hba.conf`'s `hostssl` line names
`central_admin`. In the code only `admin.rs` holds the second pool.

The limit, said plainly: one process holds both logins. A mistake in an
ordinary code path, or any statement run as the runtime, cannot approve,
appoint or ban. Someone who controls the process can. Central compromise is
already outside what these rights protect (`threat-model.md`).

**Tests.** The grant contract for `central_admin`, including that `SELECT
credential FROM central_identity.accounts` fails as it; the runtime's
refusals unchanged; the launcher mode (parsing, appointing, again, unknown
and pending usernames, a process test as for beta keys); the role rules (last
administrator, oneself, an unenrolled target, a non-administrator, the native
listener, a stale sign-in refused for a change and accepted for a list, a
renewal that does not freshen it); one audit event per change, without words;
`last_sign_in_at` at sign-in and not at renewal; the upgrade from 000011. In
`web/`: the lazy chunk, the guard, the pages, the password prompt; in
`web/e2e-real`: appoint, then appoint a second administrator from the website.

**Docs.** `operations.md` (the role, `pg_hba.conf`, the `admin` block, the
mode, `ops/grants.sql`), `threat-model.md` (the "Admin escalation" row),
`architecture.md` §"Gateways and administration", `gateway-registry.md`,
`web-accounts.md`, `security-assurance.md`, `status.md`, the roadmap's M1 and M2.
`reports.md` gives bug-report triage writes to the runtime; once built, they
belong under the same login.

## 6. Locking and banning (M)

**Lock and ban.** Both have the same mechanics. A **lock** holds an account
for its own protection or while something is looked into (a suspected
takeover, its owner's request). The person is told that the account is locked
and how to reach us, not why. A **ban** is a sanction under Central's terms
(§7.2), and the person is told its reason. Both may have an end.

**What people see.**

- The administrator, on an account: "Lock", "Ban", "Lift", "Reset display
  name". The dialog takes an end (a date and time, or none), for a ban a
  reason (1 to 500 characters, one line, shown to the person), an internal
  note (at most 500, administrators only), and optionally the report it
  answers (step 8).
- The person, signing in on the website with the right password:
  - ban: "This account is banned until 1 November 2026, 18:00 (or: for good).
    Reason: … If you think this is wrong, write to us: [contact]."
  - lock: "This account is locked for its protection (until …). Write to us:
    [contact]."
- In a client released before this step: a plain sign-in failure (owner
  question 3). A client that knows the new answer says the same sentences.
- On a gateway: the next lease check fails, at the latest five seconds later.
  Lobby, chat, watch and game sockets close and the session ends. A running
  game plays on, and the house takes the chair when the reconnect window runs
  out, as for any lost player. The account's gateway admin sessions end as
  well.

**What a restriction does**, in one transaction under the account's row lock
(the lock `issue_session` takes, so a sign-in in flight commits either before
and is ended, or after and is refused):

1. a restriction in force is lifted and the new one inserted;
2. every session of the account, native and browser, is deleted; tickets and
   leases go with them (`ON DELETE CASCADE` from `sessions`);
3. the audit event is written.

Afterwards `LoginFinish` with a correct proof answers `Restricted` and issues
no session. A wrong proof is `Rejected` as before, so nothing about a
restriction reaches a guesser, and `Discover` is unchanged. `RenewSession`,
`GatewayTicket`, `GatewaySession` and `GatewayAuthorize` find no session or
lease. A restriction whose end has passed is no restriction, read so at each
sign-in, and stays on record until lifted or replaced. Lifting restores
nothing: the person signs in again. Nothing is deleted: the encrypted data
and the gateways' data stay. Refused: restricting an administrator (remove
the role first), oneself, or a pending account.

Resetting a display name clears it (terms §2.3: "we may have names that break
this rule changed"); the person chooses a new one at the next website sign-in,
and gateways show `Player` from the next admission.

**Wire.**

- `Response::Restricted { restriction: { kind: lock | ban, until:
  Option<i64>, reason: Option<String> } }` answers `LoginFinish` on both
  listeners; `reason` only for a ban. It is a typed answer rather than an
  error, because the error vocabulary is fixed status text and carries no
  reason. A native client from before decodes it as `Rejected`; a 403 is not
  used, because clients read 403 as `TermsRequired`.
- `AdminRequest` gains `Restrict { account, kind, until: Option<i64>, reason:
  Option<String>, note: Option<String>, report: Option<[u8; 16]> }`, `Lift {
  account }` and `ResetDisplayName { account }`. Each answers `Done`. `until`
  is in the future and at most ten years ahead. A ban needs a reason; a lock
  takes none.

**Schema: migration 000013 `restrictions`.**

```sql
CREATE TABLE central_identity.account_restrictions (
    id uuid PRIMARY KEY DEFAULT pg_catalog.uuidv7()
        CHECK (pg_catalog.uuid_extract_version(id) IS NOT DISTINCT FROM 7),
    account_id uuid NOT NULL REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    kind text NOT NULL CHECK (kind IN ('lock', 'ban')),
    reason text CHECK (length(reason) BETWEEN 1 AND 500),
    note text CHECK (length(note) <= 500),
    created_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    created_by uuid NOT NULL,
    until timestamptz,
    report_id uuid,
    lifted_at timestamptz,
    lifted_by uuid,
    CHECK ((kind = 'ban') = (reason IS NOT NULL)),
    CHECK (until IS NULL OR until > created_at),
    CHECK ((lifted_at IS NULL) = (lifted_by IS NULL))
);
CREATE UNIQUE INDEX account_restrictions_current
    ON central_identity.account_restrictions(account_id) WHERE lifted_at IS NULL;
```

**Grants.** The runtime: `SELECT` on `account_restrictions`. The admin login:
`SELECT, INSERT, UPDATE (lifted_at, lifted_by)` on it; `DELETE` on `sessions`;
on `accounts` `SELECT (id)` (to take the row lock) and `UPDATE (display_name,
display_name_at)`, nothing more.

**Tests.** Sessions, tickets and leases gone; `GatewayAuthorize` refused;
`Restricted` after a correct proof only; a past end signs in; lift and
replace; the race between a sign-in and a restriction; the refused targets;
the reason rules; the name reset. `central-http`: the answer on both
listeners. `web/`: both sentences on the sign-in page, the dialogs. Baylee
`cargo dev central integration`: a seated player's sockets close within five
seconds of a Central ban.

**Docs.** `web-accounts.md` (sign-in answers), `gateway-registry.md`
§"SSO and revocation", `threat-model.md`, Baylee's `docs/central-login.md`
(what a Central ban does at a gateway).

## 7. Beta keys and gateway approval on the website (M)

**What people see.**

- `/admin/keys`: newest first, with note, uses left, reserved, expiry,
  created and state (active, used up, expired, revoked). "Issue keys" takes a
  count (1 to 100), uses (1 to 1000), an expiry in days (optional) and a
  note, and shows the keys once, with copy and download and the warning that
  they cannot be shown again. "Revoke" ends the key's pending registrations
  too, as today's SQL does.
- `/admin/gateways`: pending first, then approved and disabled. Origin,
  name, version and website (as text), the owner's handle, last heartbeat,
  number of administrators, and the Official mark. "Approve" opens a dialog
  that says what to check outside Central first (that the owner controls the
  origin, and who runs it), with a box to confirm it was checked. "Disable",
  "Enable", and "List the official gateway" while there is none (replacing
  step 1's SQL).

**Rules.** Approval stays a person's decision after checking outside Central
(`gateway-registry.md`); Central still fetches no registered address.
Disabling hides the entry at once, its credentials stop working (the joins
that exist), and its leases are deleted, so every session there fails at its
next check. Enabling again uses the same row and credentials.

**Wire.** `AdminRequest` gains `BetaKeys { before }` → `AdminAnswer::BetaKeys`,
`IssueBetaKeys { count, uses, expires_days, note }` → `AdminAnswer::IssuedBetaKeys
{ keys }`, `RevokeBetaKey { key }`, `Gateways { before }` →
`AdminAnswer::Gateways`, `SetGatewayApproval { gateway, enabled }` and
`ListOfficialGateway { metadata }`. Keys come from `BetaKey::generate` and are
stored through `BetaKeyBatch`, with the operator mode's bounds and checks.

**Schema.** None.

**Grants.** The admin login: `SELECT, INSERT, UPDATE (revoked_at)` on
`beta_keys`; `SELECT, INSERT, UPDATE (enabled)` on `gateway_approvals`;
`INSERT` on `gateways` (the official row only, checked in code); `DELETE` on
`gateway_leases`. The runtime is unchanged. `--issue-beta-keys-stdin` and the
SQL stay as the fallback.

**Tests.** The grant contracts; issuing within bounds, digests only, shown
once; revoking ends reservations; approve, disable and enable as seen by the
directory, `authenticate_gateway`, leases and admin tickets; the official
listing; one audit event each; the pages.

**Docs.** `operations.md` §"Beta keys" (the website first, the mode as the
fallback) and the approval procedure; `gateway-registry.md` §"Registration
and operator approval".

## 8. Reports from gateways (M, plus S in Baylee)

**What people see.**

- In a gateway's console (Baylee `web/feedback/src/admin/AccountDetail.tsx`),
  on a Central account: "Report to Baylee Central", with a category
  (cheating, harassment, offensive name, spam, illegal content, other) and a
  note of 1 to 1000 characters. Afterwards: "Sent. Baylee Central's
  administrators decide. A ban here stays this gateway's own." When a report
  is still open: "This gateway has an open report about this account."
- In Central's `/admin/reports`: open reports first, with received, gateway
  (name and origin), account (handle), category, note (as text) and state.
  "Lock" or "Ban" opens step 6's dialog with the report linked; "Dismiss"
  takes a note. An account's page lists its reports.
- The reported person is not told of a report, only of a restriction.

**Wire.**

- `ReportAccount { account: [u8; 16], category, note }`, under a gateway's
  redemption credential or the bridge (`gateway_authority`) → `Done`.
  `Conflict` while that gateway has an open report about the account; `Busy`
  beyond 200 open reports, or 30 an hour, from one gateway; `Rejected` for an
  unknown or pending account. The bridge reports as the official row, so step
  1 comes first.
- `AdminRequest` gains `Reports { open: bool, before }` →
  `AdminAnswer::Reports` and `DecideReport { report, decision: dismissed |
  actioned, note }`. `Restrict` with `report` marks it actioned.

**Schema: migration 000014 `account_reports`.**

```sql
CREATE TABLE central_identity.account_reports (
    id uuid PRIMARY KEY DEFAULT pg_catalog.uuidv7()
        CHECK (pg_catalog.uuid_extract_version(id) IS NOT DISTINCT FROM 7),
    gateway_id uuid NOT NULL REFERENCES central_identity.gateways(id) ON DELETE CASCADE,
    account_id uuid NOT NULL REFERENCES central_identity.accounts(id) ON DELETE CASCADE,
    category text NOT NULL
        CHECK (category IN ('cheating', 'harassment', 'name', 'spam', 'illegal', 'other')),
    note text NOT NULL CHECK (length(note) BETWEEN 1 AND 1000),
    created_at timestamptz NOT NULL DEFAULT pg_catalog.clock_timestamp(),
    state text NOT NULL DEFAULT 'open' CHECK (state IN ('open', 'dismissed', 'actioned')),
    decided_at timestamptz,
    decided_by uuid,
    decision_note text CHECK (length(decision_note) <= 500),
    CHECK ((state = 'open') = (decided_at IS NULL)),
    CHECK ((decided_at IS NULL) = (decided_by IS NULL))
);
CREATE UNIQUE INDEX account_reports_open
    ON central_identity.account_reports(gateway_id, account_id) WHERE state = 'open';
```

These are kept apart from bug reports (`reports.md`) and from reported chat
messages, which stay with the gateway, the host of what is said there.

**Retention.** The privacy notice keeps reports and moderation decisions at
most three years (§7). Before each admin list, a sweep deletes decided
reports, lifted or ended restrictions and audit events older than that. The
admin login therefore also holds `DELETE` on those three tables: the audit
is append-only for every other purpose.

**Grants.** The runtime: `SELECT, INSERT` on `account_reports`. The admin
login: `SELECT, UPDATE (state, decided_at, decided_by, decision_note)` on it,
and the sweep's `DELETE`.

**Baylee (G3).** A console route `POST /admin/accounts/{id}/central-report
{"category", "note"}` answers `204`, `404` for no account, `409` for an
account that is not Central's or has an open report, `400` past the bounds,
`503` without Central. It works with a registered gateway's credential and
with the bridge. Its audit line is `action=account.central_report` with the
account id and the category, never the note. The button and dialog are in
`AccountDetail.tsx`; `docs/protocol.md` §"Acting on an account" describes the
route.

**Tests.** `central-postgres`: both credentials accepted, the heartbeat token
refused, the open duplicate, the caps, unknown accounts, decisions, a linked
restriction, the sweep. Baylee: the route in `e2e_admin`, the dialog in
`moderation.test.tsx`. `web/`: the reports page.

**Docs.** `gateway-registry.md`, Baylee's `docs/protocol.md` and
`docs/central-login.md`.

## 9. Later: renaming (S)

`SetDisplayName` with a name already set: allowed once every 30 days, keeping
the tag, setting `display_name_at`. Gateways show it from the account's next
admission (step 4), and `NameBook` forgets the old one. The account page says
when the next rename is possible. Old names are not kept.

## Legal texts (the owner reviews them)

The **privacy notice** (`legal/privacy.de.md`, binding, and `privacy.en.md`)
must change before steps 3, 5, 6 and 8 go live:

- §3: display name and tag are processed now, no longer "will follow", and
  are shown to other players on the gateways one signs in to and to our
  administrators; roles (administrator); the time of the last sign-in,
  without an address, for administering accounts and security; locks and
  bans with their reason, end and author, and the audit of administrators'
  actions, to enforce the terms and keep decisions traceable; reports a
  gateway makes about an account (which gateway, category, its note).
- §5 and §7: sessions and the cookie last up to 12 hours with renewal (stale
  already since PR #32).
- §6 "Recipients": a gateway one signs in to receives the display name and
  tag besides the account ID; a gateway may send us a report about an
  account.
- §7: the last sign-in is kept while the account exists; restrictions,
  reports and the audit at most three years (the existing "reports and
  moderation decisions" line covers them; say so).
- §9: an account needs a display name too, freely chosen, so pseudonymous use
  stays possible.

The **terms** already cover this: §2.3 (names, and that we may have them
changed), §6 (reports, reasons, objection) and §7.2 (suspension with reasons).
No new version, and so no new acceptance by everyone, is needed for this
proposal. For the next version anyway: the tag is drawn and cannot be chosen;
a lock protects an account; a gateway operator can report an account, the
decision is ours, and a gateway's own ban is the gateway's.

## Order of work

| Step | What | Size | Needs |
| --- | --- | --- | --- |
| 1 | Directory on the browser listener, the official gateway listed | S (Central), S (Baylee) | — |
| 2 | Account summary, recovery codes for moved accounts | M | — |
| 3 | Display name and tag at Central | M | 2 |
| 4 | Central's names on gateways | M (Baylee) | 3 deployed |
| 5 | Admin login, roles, the first administrator, audit, account list | M | owner question 1; 3 for names |
| 6 | Locks and bans, name reset | M | 5 |
| 7 | Beta keys and gateway approval on the website | M | 5, 1 |
| 8 | Reports from gateways | M (Central), S (Baylee) | 6, 1 |
| 9 | Renaming | S | 3, 4 |

S is about a day, M a few days, L a week or more. Steps 1 and 2 can be built
side by side. Migration numbers are taken in merge order, because the registry
in `central-migration` is append-only; the numbers above assume this order.

## What ships without a client release

All of it.

- Central and the website ship through the deploy hook, with a gateway
  deploy or by hand (`10-central manual <revision>`, `operations.md`).
  Steps 2, 3, 5, 6 and 7 need nothing else.
- The Baylee parts (step 1's heartbeat, step 4, step 8's route) ship with a
  gateway deploy of a commit (`cargo dev deploy`, Baylee
  `docs/releasing.md` §"Server deploys"), after Baylee pins the new
  `central-api` revision. The hook installs that Central revision first.
- No answer that exists changes shape, so clients and gateways already out
  keep working. A client release only adds the sentence for a restricted
  sign-in (step 6) and, with M6, the recovery code and display name in the
  client.
- Before steps go live: the privacy notice's new version (3, 5, 6, 8); the
  tag floor check (3); `central_admin` provisioned and in `central.json`, and
  the owner's account moved and appointed (5); the official row listed
  (1).

## Questions for the owner

1. **Who writes what an administrator decides?** (a) A separate
   `central_admin` login used only by the admin requests (recommended: the
   runtime still cannot approve gateways, issue keys, appoint or ban, and the
   database refuses the admin login every encrypted column; it costs one more
   role and password on the server). (b) The runtime login with wider grants
   (simplest; drops today's "the runtime cannot approve" guarantee). (c) The
   website proposes and the operator confirms by command (keeps today's
   guarantee, but every approval and key needs the shell).
2. **Does a lock need a reason?** As proposed, a lock tells the person only
   that the account is locked and how to reach us, and a ban gives its
   reason. Does that fit your reading of terms §7.2 ("we give reasons"), or
   should a lock carry a reason too?
3. **Ship step 6 before a client release?** Clients released before it show a
   locked or banned account only a plain sign-in failure; the website says
   why. Ship it anyway, or wait for the next client release?
