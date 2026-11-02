# PWA and Web Push

The browser app installs at `/account` with a root-scoped service worker. The
worker caches only the public build, fonts and icons. It never caches `/api/`,
`/site/`, account responses, decrypted conversations or report contents. Offline
navigation can draw the shell; reading an account and sending messages need a
connection. Two public builds are retained so another open tab can still load its
previous chunks. An update waits for the user to confirm reloading, including a
warning about unsent input. No automatic update reload discards a draft.

Browsers exposing [`beforeinstallprompt`](https://developer.mozilla.org/en-US/docs/Web/API/Window/beforeinstallprompt_event)
show an explicit “Install Baylee” offer only after confirming installability. Its
one-use `prompt()` is invoked directly by a click; installation is never automatic.
Already installed apps hide the offer. Other browsers retain their own installation
menu; the notification settings explain the iPhone/iPad Home Screen requirement.

## Consent and ownership

A signed-in user is offered notification settings; browser permission is requested
only by clicking the device switch. The offer also appears when browser permission
already exists but the previous sign-in's device subscription has been disconnected.
An old permanent dismissal no longer hides this case. A deliberate opt-out suppresses
the offer for the current page's sign-in; new authentication may offer it again. Profile settings have independent switches for
chat, incoming contacts and versions. Administrators additionally have feedback
and gateway applications. The category mask belongs to the account and updates
one bit atomically, preserving other tabs' choices. The device switch separately
controls this browser's subscription. Category defaults are selected, but there is
no delivery before explicit device opt-in. Denied browser permission must be changed
in the browser; the page cannot override it. Safari on iOS requires an installed
Home Screen app. HTTPS is required (apart from local browser development exceptions).

`notifications` is a cookie-only operation: native and gateway credentials cannot
manage subscriptions. Current administrator status is checked both on preference
changes and on queued delivery. Reading preferences and withdrawing consent remain
available even before accepting newly published terms. An installation has a fresh random generation;
the worker discards old-account or old-installation hints. Signing in to another
account invalidates the old browser subscription. Logout revokes subscriptions
bound to that session, also server-side. Explicit device activation retires the
previous endpoint when it belongs to the same account, so repeated activations do
not consume the device quota. When an authenticated flow such as chat unlock verifies the password
again, the login transaction moves only this account's subscriptions bound to the
presented previous browser cookie to the new session. An expired cookie can identify
existing consent but cannot authorize a login. This preserves notifications and
server-side logout revocation without adopting another account's subscriptions or
revoking another device's consent. Delayed worker registration cannot undo a newer local opt-out. Password/credential changes prevent any
queued delivery to older revisions. Ordinary session expiry does not disable an
explicitly opted-in device: notifications can arrive while the app is closed.
Browser permission can still suppress delivery at any time.

At most ten subscriptions per account are stored. A subscription is usable for
90 days after registration/renewal; stale rows are removed when that account next
subscribes. Opt-out deletes the subscription and pending jobs; account deletion
cascades them. A worker-local opt-out is applied even when a server request fails.
A failed account-wide setting change is reported and must be retried to affect
other devices. Current preferences are synchronized when opening the profile.

## Delivery and boundaries

PostgreSQL migration 22 adds preferences, subscriptions, release deduplication and
a durable outbox. Inserts into chat deliveries, incoming contacts, reports,
gateways and release announcements queue fixed category hints in the same
transaction. Chat hints exclude the sender's own devices. Jobs coalesce by
installation/category and expire after one day. A 30-second lease has a unique
UUIDv7 identity; acknowledgement checks that lease and the event revision. A late
worker cannot delete a newer event or acknowledge its successor. Transient failure
retries with backoff capped at five minutes; provider 404/410 removes only the
matching installation generation. Delivery is at least once, best effort; browser
tags coalesce visible notices. No receipt promises that an OS showed a message.

The sender performs one bounded outbound operation at a time. HTTP uses verified
Rustls, a ten-second total timeout, a three-second connect timeout and an 8-KiB
response-header cap. Proxy configuration and redirects are disabled. Only canonical
HTTPS endpoints at `fcm.googleapis.com`, `updates.push.services.mozilla.com` and
`web.push.apple.com` are accepted. DNS results must be public IPs on port 443;
private, loopback, mapped private and special-purpose ranges are rejected before
connecting. Adding another provider requires a reviewed allowlist change.

RFC 8291 encrypts only `{v,kind,lang,nonce}`. Names, message text, report contents,
account IDs and end-to-end chat keys never enter push payloads. The browser provider
still learns transport metadata (endpoint, timing, sizes and IPs). ES256 VAPID
identifies this service and binds a short-lived token to the provider origin.
Dependencies, native internals, alternatives and exact features are recorded in
`docs/dependency-proposal/push.md`; first-party Rust remains unsafe-free.

## Operator setup

Build the service with a C compiler, CMake and libclang available (AWS-LC's native
backend). Apply migrations and the matching `ops/grants.sql` with the operator
workflow before starting the new binary. Add `push: {vapid_key: [...]}` to the
private stdin configuration; the bytes are an EC P-256 PKCS8 private key produced
by `central-service --generate-secrets`. Existing OPAQUE and bridge keys must be
preserved. Back up the VAPID key too: changing it requires browsers to subscribe
again. Omitting `push` disables registration and delivery, and the UI explains that
notifications are unavailable. `--generate-secrets` must never be rerun over an
existing live configuration. Send no private key in an environment variable, CLI
argument, frontend bundle or log.

Permit outbound DNS and HTTPS to the supported providers. Serve `/sw.js` and the
manifest with `Cache-Control: no-cache`; their correct MIME types are JavaScript
and `application/manifest+json`. Keep hashed build assets immutable and deploy the
complete build atomically. Service startup announces its package version once;
the waiting-worker banner also detects changes to the actual web build. Existing
release migration, legal and independent security-review gates still apply.

## Validation

- Real PostgreSQL tests cover browser/native separation, ownership theft, atomic
  category updates, real contact/feedback/gateway triggers, role revocation,
  release deduplication, device caps, logout, credential changes, concurrent leases,
  new events during delivery, lease expiry and late acknowledgements.
- Crypto tests decrypt the published RFC 8291 receiver vector and actual encrypted
  hints, verify VAPID signature/audience and reject tampering, malformed points,
  wrong curves, hostile URLs and private DNS destinations.
- Web tests execute the actual worker source with deterministic cache/IDB adapters:
  consent, installation changes, hostile payloads, fixed click destinations, no API
  caching and retaining a previous build. Settings tests cover opt-in, all category
  switches, denied/offline changes, failed consent storage, delayed registration,
  reauthentication cleanup and unmounted-account races.
- `web/e2e-real/pwa.spec.ts` uses an isolated HTTPS fixture and browser notifications;
  CDP injects delivery instead of contacting a real push provider. Full Chromium
  is used headlessly: the headless-shell binary denies notification permission
  ([Playwright issue 23954](https://github.com/microsoft/playwright/issues/23954)). The fixture
  certificate's exact SPKI is trusted only by that owned test browser.
- Actual provider delivery and OS-specific installation behavior require a staging
  acceptance run; local tests do not send notifications to human devices.
