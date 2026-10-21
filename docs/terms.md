# Terms of use: documents, versions and acceptance

Owner decision, 2026-10-20: Central's terms are accepted at registration, each
gateway's terms before playing there; acceptance is an account fact kept on
Central, per document and version; a new version offers *accept*, *later* or
*delete*, and nobody continues under an older version. This page is the design
pass (security assurance pass 1) for that feature. The privacy policy is
information, not a contract, and is never "accepted".

## Assets and trust

- **Acceptance records** prove which text an account agreed to and when. They
  must not be forgeable by a gateway, by another account or by a client that
  never showed the text, and they must not be editable afterwards.
- **A gateway's published text** is what its players agree to. Only that
  gateway may publish it; a published version never changes.
- **Enforcement** must not depend on the client: a client that skips the sheet
  must still be refused.
- Callers: a central session (the player), a gateway credential or the
  migration bridge (the gateway), nobody (public reads).

## Documents

- **Central's terms** are `legal/terms.de.md` (binding) and
  `legal/terms.en.md`, compiled into the service. Each starts with
  `<!-- version: … -->`; the version `draft` means not in force, and while it
  is `draft` Central has no terms in force and enforces nothing. There is no
  second marker that could disagree with it. Both languages carry the same
  version, or a test fails. Publishing a version is a reviewed commit that
  replaces `draft` by a real version.
- **A gateway's terms** are published by that gateway:
  `PublishGatewayTerms { version, texts }`, authenticated by its redemption
  credential (or by the migration bridge for the official origin). Central keys
  them by the gateway's canonical origin, the same audience its tickets and
  leases use, so the official gateway (which has no registry row while it is
  the migration bridge) and registered gateways are handled alike. A version is
  immutable: publishing it again with the same texts is `Done`, with other
  texts `Conflict`. The newest published version is current.
- Bounds: version 1–64 printable ASCII without whitespace; 1–8 texts, one per
  language tag (2–3 lowercase letters), `en` or `de` required; each text 1–65
  536 bytes of UTF-8 without NUL. Text is untrusted display data: clients render
  only the Markdown subset (`legal/README.md`), never HTML.
- The **digest** of a document version is SHA-256 over its texts in language
  order, each as `lang`, NUL, length (8 bytes, big-endian), bytes. Central
  computes it; nobody supplies it.

## Acceptance

- `AcceptTerms { subject, version, digest }` with a central session. `subject`
  is `central` or a gateway origin. Central refuses (`Conflict`) a version that
  is not the subject's current one or a digest that does not match it, so a
  client can accept only the text that is in force, and only by naming the
  digest of what it fetched and showed. Accepting the current version again is
  `Done`, so a retry after a lost reply never conflicts.
- Currentness compares version **and** digest: a text changed under an
  unchanged version is a new text nobody has accepted.
- Records are append-only (`SELECT, INSERT` for the runtime role, no `UPDATE`
  or `DELETE`): account, subject, version, digest, time. They go with the
  account (`ON DELETE CASCADE`). The latest record per subject is the account's
  accepted version.
- Registration on the website (`docs/web-accounts.md`) shows Central's terms
  and records the acceptance with `AcceptTerms` on the browser surface after
  the first sign-in, before the account can do anything that needs it.
- A migrating legacy account accepts in the migration flow: after its first
  central login the client shows the sheet before it asks for the export and
  link tickets. Restoring (`ConfirmRestore`) stays allowed meanwhile, so an
  acceptance before or after it both work.
- Central has no account deletion yet (M1). Until then the Central sheet offers
  *accept* and *later*; *delete* appears with M1.

## Enforcement

- **Central.** While Central has terms in force and a session's account has not
  accepted the current version, that session may only read its status and the
  terms, accept, read its own documents and objects (export), confirm a
  restore, and log out.
  Everything else, gateway tickets included, is refused with a distinct error
  (`TermsRequired`, HTTP 403) so the client can show the sheet instead of a
  generic failure.
- **Gateways.** `GatewayAuthorize { lease, purpose, terms }` replaces
  `GatewaySession` for gateways that know it: the same live recheck in the
  same statement, plus whether the account accepted Central's terms in force
  and whether it accepted exactly the version and digest the gateway names as
  its own current one. The gateway refuses a stale session everything but
  reading its terms, signing out and deleting its own data there, and says
  why.
  `GatewaySession` stays, unchanged, for gateways not yet updated, because
  responses refuse unknown fields.
- A change of version takes effect at once: Central's at the next request of
  every session; a gateway's as soon as it has published the new one. The
  gateway applies the refusal at its HTTP routes and when a socket opens; a
  seat, watch or social socket that is already open is ended only by the lease
  check, so running games finish. Baylee's `docs/protocol.md` names the routes.

## Reads

- `Terms` (public): Central's current terms, with version and digest, or none.
- `GatewayTerms { origin }` (public): a gateway's current terms, for the website
  and for clients.
- `TermsStatus` (session): Central's current version and the account's
  accepted one.

## Failure modes and invariants

- Database unavailable: every check fails closed (`Unavailable`); never treat a
  failed lookup as "accepted".
- A gateway cannot accept on a player's behalf: acceptance needs the player's
  central session, which gateways never receive.
- A gateway cannot rewrite what players accepted: published versions are
  immutable and acceptances name the digest.
- One account cannot read another's acceptances: only `TermsStatus` for the
  session's own account and `GatewayAuthorize` for a lease the gateway holds.
- One query: the terms check before a gated session operation is one
  statement (live session, account, acceptance of the exact version and
  digest), run only while terms are in force, and a database error is
  `Unavailable`. A bearer that is no live session is `Rejected`, never
  `TermsRequired`, so the error says nothing about accounts.
- Size and rate: bounded texts (at most four languages of 64 KiB, so a
  document fits the wire limit even when every byte needs escaping), at most
  256 published versions per gateway (`Rejected` past it), and the public
  reads `Terms` and `GatewayTerms` fall under the global request limit like
  `Discover`; `GatewayAuthorize` and `TermsStatus` are control requests.
- Only an approved, enabled gateway (or the bridge, for the official origin)
  publishes, and only for its own origin. Central serves a stored version only
  if its texts still hash to its digest; tampered rows are `Unavailable`.
- Versions move forward: the newest published one is current, so an operator
  who wants an older text back publishes it under a new version.

## Deployment order

1. Central with these operations, terms still `draft`.
2. Gateways that publish their terms and use `GatewayAuthorize`. Against an
   older Central `GatewayAuthorize` is `Rejected` and the gateway fails
   closed, so Central always ships first.
3. Clients that know both sheets.
4. Only then a reviewed commit puts Central's terms in force, and gateways
   enforce theirs.
