# e2e-real — the new site's specs against a real Central

The site's browser specs (`site-*.spec.ts`) run against a real Central in its
deployed mode (`--accounts-stdin` with `browser.listen`), with real OPAQUE and
Argon2 in the page's Worker and the real `__Host-central` cookie. It is not part
of `npm run e2e:site` or CI: it needs a PostgreSQL and a running Central, set up
by hand as below. Synthetic data only; every login, key and certificate is made
for the run and deleted after.

The browser must really be at `https://baylee.app`: the browser listener takes
only that `Host` and `Origin`, and a `__Host-` cookie needs HTTPS. Chromium maps
`baylee.app:443` to a local stand-in for the site's Caddy (`front.ts`: TLS,
`/api/*` to the listener with the Host unchanged and `X-Forwarded-For` set, every
other request to the server-rendered site), and the tests accept its throwaway
certificate. The deployed mode compiles in the live issuer, which the committed
WebAssembly module (`../../crates/central-web/crypto`) is bound to: no
development build is needed.

## Setting up (from the repository root)

1. **PostgreSQL 18 with TLS**, as `cargo xtask postgres` makes it: a
   throwaway authority, a server certificate naming only `127.0.0.1`, the
   pinned image (`crates/xtask/src/postgres/container.rs`) started with
   `ssl=on`, published on `127.0.0.1` only, under a label of its own.
2. **Provisioning** as in `docs/operations.md` §"Provisioning", with fresh
   `openssl rand -hex 32` passwords fed to `psql` on standard input.
3. **Central**, each document on standard input with `umask 077`:
   `central-service --generate-secrets`, `--migrate-stdin`, then
   `ops/grants.sql` as the database owner, and `--accounts-stdin` with
   `"listen": "127.0.0.1:47920", "browser": {"listen": "127.0.0.1:47921"}`.
4. **The site**: `cargo xtask site`, a certificate for `baylee.app`
   (`openssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:P-256 -nodes
   -subj /CN=baylee.app -addext subjectAltName=DNS:baylee.app`), then
   `node e2e-real/front.ts --tls <its folder> --site 28792`, with `central-web`
   started as in §"The new site" below.

## The new site (`crates/central-web`)

The same Central serves the server-rendered site
(`../../docs/proposals/frontend-rewrite.md`). Build it with `cargo xtask site`,
start `target/debug/central-web --dev target/site
crates/central-web-server/tests/data` (it listens on `127.0.0.1:28792`), and
put the front before it: `node e2e-real/front.ts --tls <its folder> --site
28792`. Run the specs in `web/`. Then

```bash
REAL_FRONT_SPKI=<pin> REAL_OUT=<empty folder> npx playwright test -c e2e-real/playwright.config.ts e2e-real/site-chat.spec.ts
```

`REAL_FRONT_SPKI` is the base64 SHA-256 of the throwaway certificate's
public key (`openssl x509 -in cert.pem -pubkey -noout | openssl pkey -pubin
-outform der | openssl dgst -sha256 -binary | base64`): Chromium registers
no service worker on an untrusted certificate, and the pages register one.

registers two accounts at each width without a key (the open beta; the page
solves Central's proof of work), makes them contacts by handle, has
them write to each other (markup in a message stays text), reloads one page
and finds the chat open again without a password (the identity the sign-in
kept in this browser), compares the safety number on both sides, creates a
group, invites, joins, writes to it and leaves it (the history stays, the
composer goes), switches a notification category off and on across a reload
(the device's own Web Push needs a push service and is not run), and checks
that localStorage holds neither a message nor the password.

```bash
REAL_FRONT_SPKI=<pin> REAL_SERVICE=<central-service> REAL_MIGRATE=<the migrator document> REAL_NATIVE=<the native listener> \
  npx playwright test -c e2e-real/playwright.config.ts e2e-real/site-reset.spec.ts e2e-real/site-password.spec.ts --project desktop
```

the reset page and the password's change (12.11.2026; steps shared in
`site.ts`). `site-reset.spec.ts` registers an administrator, whom the
operator's `--appoint-admin-stdin` appoints, and an account. The administrator
makes reset codes on `/en/account/admin` (the search, the account, the warning,
its password once more, the code shown once and copied), and the account
redeems them on `/en/reset`: a recovery code that is not its own stops before
the code is spent; its own keeps the vault; the used code is refused in the
words of any wrong one; without the recovery code a new vault begins at the
next epoch and its new recovery code is shown once; the old recovery code opens
nothing any more and the new one does. `site-password.spec.ts` registers one
account and changes its password in the account area: a wrong current password
changes nothing, the right one leaves this browser signed in and ends the
client's session, the old password is refused and the new one signs in. Both
read what became of the vault from outside the page: a sign-in over the native
listener as Baylee's client makes it (the committed module, `REAL_NATIVE`,
default `127.0.0.1:47920`) tells whether the session lives, the fingerprint of
the chat identity, which is derived from the master key (the same after a kept
reset and a password change, another after a new vault), and the stored
document with its revision. A passing run prints no password, code or token and
writes none; a failing one may, in its assertion's message and in its trace
(`web/test-results-real`, kept on failure), which holds what was typed and sent,
the reset code in `ResetStart` included. All of it is synthetic; delete the
folder after reading. One project registers three accounts and starts six
resets (ten an hour from one address): run one project against a Central
started afresh.

Central allows five registrations an hour and ten a day per address, and counts
them in memory: one run of `site-chat.spec.ts` registers four, so start Central
afresh between runs. Since 11.11.2026 it also counts sign-ins that were not
completed with the right password (30 an hour from one address, 10 an hour at
one account from one address; `docs/native-login.md` §"Guessing a password
online"), and every local run comes from one address: a Central started afresh
has counted none.

`site-email.spec.ts` needs Central's `mail` block pointed at the stand-in
mailer `mailer.ts` (`node e2e-real/mailer.ts --token-file <file> [--port
47925] [--keep <dir>]`, loopback only), whose token file holds the block's
token. It takes a mail as the real `baylee-mailer` does, the `inline` field
of `"logo": true` included, keeps it in memory for `GET /mails?to=` and,
with `--keep`, writes each as `.txt` and `.html` (mode 0600) to look at.
The spec accepts a mail with no image, or, where the request names
`inline: ["baylee-logo"]`, exactly that one image by `cid:`.

### Profile pictures (`site-picture.spec.ts`)

The whole path of a picture: the browser prepares and sends it through
`front.ts`, which lets `PUT /api/v1/picture` through up to 9 MiB as the
Caddy block's own handle does (every other `/api/*` body stops at 64 KiB).
Central hands it to the worker behind its socket and serves what it
stores. A contact reports it from the chat, an administrator whom the
operator appointed removes it with a reason, the person is told, and every
file of the picture answers 404.

Central's configuration names the socket
(`"pictures": {"socket": "<absolute path>"}`; macOS allows about 104
characters for it). `picture-socket.ts` stands in for systemd's socket
activation: a worker for each connection, the connection as its input and
output, two at a time.

```bash
node e2e-real/picture-socket.ts --socket /private/tmp/claude-501/bp/picture.sock -- \
  docker run -i --rm --network none --read-only --tmpfs /tmp:rw,size=64m --memory 512m --pids-limit 32 \
  -v <a Linux central-picture-worker>:/usr/local/bin/central-picture-worker:ro <an Ubuntu 26.04 image with ffmpeg> \
  /usr/local/bin/central-picture-worker --ffmpeg /usr/bin/ffmpeg
```

The worker runs in a container because an animation needs ffmpeg with
libaom, which Homebrew's ffmpeg lacks. The Linux worker is built in the
pinned Rust image for the container's architecture. Without libaom, run
the worker itself (`-- target/release/central-picture-worker --ffmpeg
<ffmpeg>`) and set `REAL_PICTURE_SEQUENCE=0`: the animated GIF is then
expected to be kept as its still.

The spec registers three accounts per run: the person, a contact and the
administrator, whom it appoints with `REAL_SERVICE` and `REAL_MIGRATE` as
`site-reset.spec.ts` does. Start Central afresh before each project.

## Tearing down

Stop `front.ts` and Central (SIGTERM; their ports must refuse), remove the
container by its label, and delete the keys file, the configuration
documents, the passwords and the certificates' keys.
