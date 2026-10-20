# Roadmap: accounts, website and administration

Owner request and decisions, 2026-10-20. This orders the work; it authorizes no
release. Each milestone still goes through the three-pass method in
[security assurance](security-assurance.md) and lands as small reviewed PRs.
Central code is worked on at the highest model effort (owner rule).

Why now: Baylee's native client on `main` already requires a Central login, while
Central serves no registration, password change or reset. No Baylee release
ships from `main` until M1 and M6 below are done.

## Decisions

- **Login name.** Accounts sign in with their username, as today. E-mail is
  optional; once verified it may also be used to sign in and to reset. The
  initial Central administrator is the owner's existing account, migrated from
  the legacy gateway, given the operator's administrative address and the admin
  role by an operator command (M1). It also administers the primary gateway.
- **Forgotten passwords.** Registration offers a recovery code that keeps the
  encrypted account data. A reset by e-mail or by an administrator restores the
  login only: encrypted data (deck backups, chat history) survives only with the
  recovery code or a still signed-in device. The UI says so before the reset.
- **Guests.** Gateways have no guest accounts. Playing without an account means
  playing offline. Legal basis, checked 2026-10-20 (engineering reading, not
  legal advice): § 19 Abs. 2 TDDDG obliges providers to allow anonymous *or*
  pseudonymous use where technically possible and reasonable, and to tell users
  about it. A Central account needs only a freely chosen username and a
  password: no real name, no e-mail. That is pseudonymous use. The registration
  page and the privacy notice must state this. Self-hosted gateways are their
  own providers and make their own assessment.
  Source: https://www.gesetze-im-internet.de/ttdsg/__19.html
- **One web app.** The administration consoles are part of the website, loaded
  lazily, not separate sites under `/admin`.
- **Gateways are configured through their admin UI**, as completely as possible.
  Secrets and bootstrap values stay in operator files.
- **Primary gateway addresses.** The official gateway moves to
  `https://gateway.baylee.app`; its administration is served at
  `admin.gateway.baylee.app` and accepts only Central SSO (no local password or
  token sign-in there). `central-api` compiles `LEGACY_ORIGIN =
  https://baylee.acevik.de`, so that name keeps serving the same gateway until
  the legacy migration code is removed (M8).
- **Self-service gateways.** Every signed-in user may register their own gateways
  and choose who administers them. Central administrators see and manage all of
  them; ordinary users only their own.

## Target shape

- `baylee.app` serves the web app (static files behind Caddy) and a same-origin
  browser API under `/api`, proxied to Central's browser routes.
  `central.baylee.app` stays the native API. Its boundary (no Origin, no
  cookies) stays as it is.
- The web app lives in this repository under `web/`. Stack as Baylee's
  `web/feedback`: React 19, TypeScript 7, Vite, oxlint, vitest and Playwright.
  German and English, mobile first, WCAG 2.1 AA, no third-party trackers.
- Browser sign-in uses the same OPAQUE protocol, compiled to WebAssembly from
  `central-crypto`'s client side; the server never sees the password. The browser
  session is a `__Host-` cookie with `SameSite=Strict`, a CSRF token and an
  explicit Origin allow-list, valid only on `/api`.
- Deep links: the installers register `baylee://`. Signing the client in from the
  website uses an authorization code with PKCE (RFC 8252, RFC 7636): the client
  makes the verifier, the website hands back only a one-use code, and the client
  redeems it at Central. An "add gateway" link only proposes a gateway; the client
  shows it and asks before saving. The client keeps its own sign-in and
  manual-address forms.
- Gateway administration: the gateway's admin API becomes reachable over its
  public HTTPS origin, authorized by a Central SSO admin bearer plus that
  gateway's admin list, with 2FA required for admins. The loopback operator-token
  console stays for recovery. Every change is audited with the Central account ID.

## Milestones

Dependencies are marked with →. Repository in brackets.

**M0 — Central in production** [central, ops]. Done 2026-10-20 from `ff0aac2`:
one shared PostgreSQL with TLS 1.3 and `hostnossl … reject` for Central, roles,
database, migrations and grants per [operations](operations.md), hardened systemd
unit, loopback staging checks, then `central.baylee.app`; backup and a restore
drill. No account is enrolled: registration does not exist yet and legacy
migration needs the gateway bridge (M5). Live discovery's enrollment disclosure
is still to be reviewed before accounts exist.

**M1 — account lifecycle** [central] → M0. Registration with Central beta keys
(single or multi-use, expiry, notes), username rules, recovery code, password
change, optional e-mail (add, verify, remove; sending through an HTTP mail API
because the host blocks outbound SMTP), e-mail reset, administrator reset, TOTP
2FA with backup codes, session renewal and a device/session list, display name,
profile picture (bounded, re-encoded server side, size and type checked),
account export and deletion (GDPR Art. 15, 17, 20), the role model (Central
admin, user) with an operator bootstrap command, and an audit log. The SDK
(`central-client`) gets each operation for the native client.

**M2 — administration API** [central] → M1. Users (search, lock, unlock, reset,
revoke sessions, delete), beta keys, gateway registry with self-service
registration and per-gateway admin lists, origin approval by Central admins
(existing operator privilege), audit search, counts dashboard.

**M3 — browser sign-in** [central] → M1. OPAQUE in WebAssembly, browser sessions,
CSRF and Origin policy, the same-origin `/api`, rate limits per client address
behind the proxy (today's budgets are global).

**M4 — the web app** [central] → public part none, account part M3, admin M2.
Public: download, project progress and card status, Impressum and privacy,
gateway directory (public, from the registry). Account: register, sign in, profile, security
(password, e-mail, 2FA, recovery code, sessions), my gateways. Admin: Central
administration and the gateway administration area. Baylee's `web/site` content
moves here; the public part can start now.

**M5 — gateway administration** [baylee] → M2 for SSO admins. Settings move from
environment into the database where safe (environment and files stay for secrets
and bootstrap), applied live where possible, audited. Gateway-local bans. Guest
accounts removed (sign-in route, `BAYLEE_GUESTS`, cap, purge, the guest key path),
existing guests left to lapse. The admin API reachable as described above, the
admin area at `admin.gateway.baylee.app` with Central SSO only, and the move of
the primary gateway to `gateway.baylee.app` (old name kept as an alias, M8).

**M6 — client front door** [baylee] → M1 SDK, M4 for deep links. Sign in,
register (with beta key), offline play, gateway choice from the directory or by
address, the `baylee://` handler with PKCE, and account settings in the client
(password, e-mail, 2FA, recovery code, profile). No guest path to gateways.

**M7 — beta.8** → M1, M5 (guest removal), M6. Release gates as `docs/status.md`
and Baylee's `docs/releasing.md` list them.

**M8 — legacy cleanup** [both] → every legacy account migrated, or 2026-12-20,
whichever comes first. Remove the legacy migration code (discovery's `legacy`
answer, the bridge, migration routes and archives, the password verifier
columns), the old account and password handling in the gateway and client, and
the `baylee.acevik.de` alias; a database migration retires the leftovers.

M1 → M2 → M3 run one after another in this repository. M4's public part and
M5's settings work can run beside them.

## Open questions

- Does each gateway also serve the web app build, scoped to itself (its page and
  its admin area), or is administration only reachable through `baylee.app`?
- Which HTTP mail provider (data processing agreement, EU hosting)?
- Profile pictures: moderation and reporting, or picture choice from a fixed set
  first?
- Minimum age and parental consent (GDPR Art. 8; 16 in Germany) at registration.
- Legal pages for `baylee.app` (deferred in Baylee #316) before public sign-up.
