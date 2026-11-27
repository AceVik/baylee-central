# Deployed operation

The offline campaign operator procedure is in [private Advent provisioning](advent-provisioning.md).

This runbook covers the deployed launcher and its database connector. It is an
operator procedure, not a release approval: the gates in [status](status.md)
still apply. Registration, sign-in, a password change and a reset with an
administrator's code on `baylee.app` are served by the browser
listener ([below](#the-browser-listener)) once it is configured. An account's
e-mail address and a reset code by mail need the `mail` block
([below](#mail)). Nothing here enables browser SSO for gateways. Sessions
renew ([below](#sessions)).

The optional `tower_origin` setting enables identity access to the separate
Baylee Tower authority. It accepts a distinct canonical HTTPS origin and is
disabled when absent. [Tower access](tower-access.md) describes migration 30,
runtime grants and the online introspection contract; it enables no game hosting
or Tower economy inside Central.

## Launcher modes

`central-service` takes exactly one mode argument and no other argument. Every
secret arrives as one bounded JSON document on standard input (at most 128 KiB),
never in argv or the environment. Unknown fields fail closed. Anything else
than one known mode prints the usage and ends with status 2, which no mode
ends with (since 26.11.2026; status 1 before). `--modes` prints the modes
this build knows as one JSON line, `{"modes": ["--dev", …]}`, reads nothing
and connects to nothing, so that a tool can ask before it relies on one; a
build older than it ends `--modes` with its usage and status 1.

- `--accounts-stdin` serves the live native account protocol. The endpoint is
  always `https://central.baylee.app`, and the official gateway is the compiled
  `OFFICIAL_ORIGIN` with its compiled aliases (`central_api::OFFICIAL_ALIASES`,
  its old address, which is also the legacy migration's); none is configurable.
  Input:
  `listen`, `database`, `opaque_setup`, `bridge`, `token_keys`
  ([below](#rotating-the-signing-key)), and optionally `browser: {"listen": …}` for the
  browser listener (absent means off), `legacy: {"database": …}`, the legacy store's own
  login (absent means no legacy sign-in; [the legacy store](#the-legacy-store)),
  `official_address` ([the official gateway](#the-official-gateway)'s address on this machine),
  `mail: {"endpoint": …, "site": …, "token": …}`, the mailer on this host
  (absent means Central sends no mail; [mail](#mail)), `pictures: {"socket": …}`, the
  picture worker's socket (absent means uploads answer `Unavailable` and stored pictures
  are still served; [profile pictures](#profile-pictures)), and
  `funding: {"liberapay": …, "github": {"token": …}, "kofi": {"token": …, "record": …}}`,
  the donation platforms (absent means Liberapay's public figures and nothing else;
  [donations](#donations)).
- `--check-accounts-stdin` reads a service document as `--accounts-stdin` would, connects to
  nothing and exits, so that a build can be asked whether it would start; a `mail` block or a
  picture socket the service would refuse is refused here too. The deploy hook asks it before
  it stops anything.
- `--migrate-stdin` applies pending migrations once under the migration login
  and exits. Input: `database` only. The service never migrates by itself.
- `--after-restore-stdin` and `--erase-accounts-stdin`, under the migration
  login, are the operator's two steps after a restore of an older backup
  ([below](#after-a-restore)).
- `--legacy-import-stdin` and `--legacy-deadline-stdin` import the official
  gateway's legacy accounts once and set the legacy store's deadline, under the
  migration login ([the legacy store](#the-legacy-store)).
- `--import-reports-stdin` copies the feedback service's open reports once,
  under the migration login ([reports](#the-one-time-copy-of-the-feedback-services-reports)).
- `--generate-secrets` writes a fresh `{"opaque_setup": [...], "bridge": [...], "token_keys": {...}, "push": {"vapid_key": [...]}}`
  document to standard output and refuses when standard output is a terminal.
- `--generate-token-key` writes one new signing key, `{"seed": [...], "public": [...]}`, for a
  [rotation](#rotating-the-signing-key), and refuses a terminal the same way.
- `--dev` and `--dev-accounts-stdin` are the local development modes described
  in [dev mode](dev-mode.md) and [native login](native-login.md).

`listen` is a numeric loopback socket address. The listener speaks plain HTTP
on loopback only; a TLS reverse proxy terminates `https://central.baylee.app`
and must pass the original `Host` header unchanged (Caddy's `reverse_proxy`
does). Every other Host, an `Origin` or Fetch-Metadata header is refused.
Forwarded headers confer no identity and no credential; the proxy's
`X-Forwarded-For` hop shares out the places of late and large bodies
([below](#the-account-listeners-connections)) and, since 11.11.2026, is the
address the sign-in's guesses and the address budgets are counted by
([below](#sign-in-guesses-and-address-budgets)). `/health/live` answers;
`/health/ready` deliberately answers 503, so no proxy or supervisor health
check may use it. SIGTERM or Ctrl-C closes the listener, drains briefly and
closes the pool.

### The account listener's connections

The deployed account listener takes 128 connections at once (the policy's
most; a development listener 32), one request each: its headers within three
seconds, the request with its body within five (a report part's upload
within 60).

A body of at most 64 KiB that is there a quarter of a second after its
headers needs nothing more; a client sends its body with its headers, so
this is every sign-in and almost every other request. A body that is not
there by then, or a larger one (up to the 1 MiB of an encrypted object, or
one without a length), needs a place:

- four places per client address, IPv6 by its /64;
- a quarter of the listener's connections for all addresses together (32 of
  128);
- past either, 429 with the route's fixed text at once, which a client reads
  as `Busy`, before another byte is waited for.

A late body gives its place back when it has arrived, a large one when its
request ends. So bodies that are announced and withheld hold at most 32
connections, four an address, each for the request's five seconds at most,
and 96 connections stay with requests whose small body is there; no more
bodies of up to 1 MiB are in memory at once than before (32).

The address is the last `X-Forwarded-For` hop, which Caddy sets to the client
it saw (its default; without `trusted_proxies` it ignores a client's own),
and it counts only when the connection comes from loopback; a request
without a well-formed one counts by its peer. Only these places and the
budgets of the next section follow from it. Two things follow for the proxy:

- If the proxy does not send the header, every client counts as `127.0.0.1`
  and all of them share four places: small requests go on, but large writes
  and late bodies meet 429 early. A CDN in front of Caddy has the same
  effect: the last hop is then the CDN's address, not the client's.
- Every local and end-to-end run arrives as `127.0.0.1` and shares its four
  places for late and large bodies.

What this does not bound: eight addresses that withhold four bodies each
hold all 32 places, and until one comes back the late and large bodies of
everyone else are refused (429), while every request whose small body is
there is still served. A flood of whole requests is a matter of rate, which
the proxy or the network must limit; the firewall's limits
([below](#connection-limits-in-front-of-the-proxy)) bound the connections
one address opens and holds, not the requests it sends over one. Caddy can keep withheld bodies away
from Central altogether (from its documentation, not tried here): with
`request_buffers` it reads a body up to that size before the upstream sees
the request. Caddy then holds such a connection itself, under its own
limits (`timeouts`), which were not reviewed here.

```caddyfile
central.baylee.app {
	# The account listener. Caddy sets X-Forwarded-For to the client it saw;
	# no trusted_proxies here, so a client's own value is ignored.
	reverse_proxy 127.0.0.1:28790
	# Optional, from Caddy's documentation and not tried here: read a body of
	# up to the wire's 1032 KiB before Central sees the request (a report
	# part's longer upload is passed on after that much of it). The line
	# above then becomes:
	#   reverse_proxy 127.0.0.1:28790 {
	#   	request_buffers 1032KiB
	#   }
}
```

### Ranked game relay

The account listener serves `GET /v1/ranked/socket` over HTTP/1.1 Upgrade. The
site-wide Caddy rule above covers it. With a route allowlist, add this exact
path to the account listener's WebSocket rules. Preserve Host and Upgrade,
disable access and body logging, and allow long-lived bidirectional streams.
The native client sends its single-use relay credential in the first masked
binary frame. Admission shares the push listener's per-address socket and dial
limits. At most 64 game relays can run at once. Each uses one additional
outbound file descriptor compared with a push socket.

Central creates a fresh connection to the currently authorized game host.
Production requires public DNS answers, a trusted TLS certificate for the
registered gateway name, and an HTTP/1.1 WebSocket endpoint. Every resolved
address must be public. Central connects to the addresses it checked and
verifies the registered hostname during TLS. Only explicitly selected numeric
loopback development origins can use plaintext. The upstream handshake sends
the game identifier, a gateway-issued single-use ticket, and game protocol
version 70. Exclude this route's query string and request credentials from
gateway and proxy logs. The gateway receives Central's network address.

Central checks the grant before connecting, after the host accepts, and every
three seconds thereafter. A failed or timed-out check closes both streams.
After reassignment or disconnect the native client requests a fresh ticket.
Allow 64 KiB per client message and 16 MiB per gateway projection, including
fragmented messages. The relay closes a connection after twelve hours.

### The push socket

`GET /v1/ws` on the account listener is a WebSocket for signed-in Baylee
clients ([push socket](push-socket.md)). Caddy's `reverse_proxy` passes a
WebSocket upgrade on as it is, with no directive of its own; that the
deployed Caddy does so for this route has not been tried here, and is the
operator's check after the first deploy that serves it (a signed-in client
shows its friends online, or `curl` against the public name with the
handshake's headers and a bearer is answered `101`). Nothing about it is
logged.

- **File descriptors.** Each socket is a connection of its own besides the
  listener's 128, up to 512 at once. The unit sets no `LimitNOFILE`, so the
  service has systemd's 1,024: the sockets, both listeners' connections and the
  database's fit below that. A unit that lowers it, or a change that raises
  the sockets' limit, needs `LimitNOFILE` raised with it; at the limit an
  `accept` that fails stops the listener.
- **The firewall** (`ops/central-limits.nft`, next section) counts a
  socket's connection to Caddy among the 512 an address holds.
- **Its sessions** are checked once a minute in one statement for all
  sockets; one that cannot be checked twice in a row closes them all, and
  their clients poll and dial again. A stop closes every socket (1001), waits
  three seconds, then ends the rest.

### Connection limits in front of the proxy

`ops/central-limits.nft` is an nftables file for the host Caddy runs on. It
bounds, for each source address (an IPv4 address or an IPv6 /64, what Central
counts as one place), the connections to the proxy's ports (TCP 80 and 443,
UDP 443 for HTTP/3):

| Limit | Number | Past it |
|---|---|---|
| New connections | 300 a minute, after a burst of 300 | the connection's first packet is dropped |
| Connections held at once | 512 | the same |

The reasons for the numbers are in the file's head. It is not loaded by
anything in this repository: loading it is the operator's step, once, and
again after a change to the file.

**What it does not do.** It counts connections, not requests. A client that
keeps one connection and sends its requests over it (keep-alive, HTTP/2,
HTTP/3) is not slowed by it. So it does not keep four addresses from spending
the 32 requests a second that all clients without a session share
([below](#sign-in-guesses-and-address-budgets)), and it does not keep eight
addresses from holding the listener's 32 places for late and large bodies
(above): each of those needs one connection. It ends what nothing bounded
before: one address opening connections to the proxy without limit, or as
fast as it can.

**Before loading.** Three things to check on the host:

- Caddy listens on the host's own addresses (a process on the host, or a
  container with the host's network). The file's chain is in the `input`
  hook, which sees what is delivered to the host. A port that Docker
  publishes for a container is forwarded and passes the chain unseen. This
  deployment's Caddy proxies to `127.0.0.1`, so it is on the host's network;
  the check is `ss -ltnp 'sport = :443'`, which must name `caddy`.
- `nft --version` answers (the file was checked with nftables 1.1.3).
- Nothing on the host runs `nft flush ruleset` after boot (a stock
  `/etc/nftables.conf` begins with it): that would remove this table, and
  Docker's with it. The file itself never flushes: it replaces its own table,
  `inet baylee_central`, and touches no other.

**Load.**

```sh
sudo install -m 0644 -o root -g root ops/central-limits.nft /etc/baylee-central/central-limits.nft
sudo nft -c -f /etc/baylee-central/central-limits.nft   # checks, changes nothing
sudo nft -f /etc/baylee-central/central-limits.nft      # replaces the table at once
```

Connections that exist go on; the counts start empty. To load it at every
boot, a unit of its own (not `nftables.service`, for the reason above):

```ini
# /etc/systemd/system/baylee-central-limits.service
[Unit]
Description=Baylee Central connection limits
Wants=network-pre.target
Before=network-pre.target

[Service]
Type=oneshot
RemainAfterExit=yes
ExecStart=/usr/sbin/nft -f /etc/baylee-central/central-limits.nft
ExecStop=/usr/sbin/nft delete table inet baylee_central

[Install]
WantedBy=multi-user.target
```

**Check.**

```sh
sudo nft list table inet baylee_central
```

shows the four rules, each with a counter of the packets it dropped. Then,
from a machine that is not the host (the host's own requests arrive on the
loopback interface and are never counted):

- the proxy check of the next section (31 `Discover` in a minute end in one
  429) must give the same answer as before: 31 connections a minute are a
  tenth of the limit, so a 429 there is Central's, not the firewall's;
- a browser signs in, and a game client connects to the official gateway;
- the counters stay at zero while only that is done.

A counter that grows names no address. The sets do, while an address is
counted: `sudo nft list set inet baylee_central held4` (and `held6`,
`fresh4`, `fresh6`). They are in the kernel's memory only, hold source
addresses and no request, and forget an address two minutes after its last
new connection (`fresh`) or when its last connection is gone (`held`).
Nothing here writes a log. Whether the privacy notice's line on technical
access data kept briefly in memory needs a word about the firewall is the
owner's to judge.

**Remove.**

```sh
sudo nft delete table inet baylee_central
```

at once and without a restart of anything; with the unit,
`sudo systemctl disable --now baylee-central-limits.service`.

**What it does to a shared address.** A campus, an office behind one address
or a carrier's shared IPv4 address is one source. Past 512 connections held
at once (about 170 people playing, at three connections each) the next
connection from there is dropped until another closes: people who are
connected go on, new ones wait or fail. Past 300 new connections in a minute
after a burst of 300 the next ones are dropped: a connect is retried by the
client's system after one and three seconds, so a short excess shows as a
slow connect. If such a site reports that, look at the counters, and raise
the number in both places of the file (IPv4 and IPv6) rather than removing
the limit. A connection counts as held until both ends closed it, or until
the kernel's connection tracking forgets it.

**Tested, and not.** The file was checked (`nft -c`), loaded, reloaded and
removed with nftables 1.1.3 in a throwaway container with no network, and
exercised over IPv4 between two containers: a burst of 300 connections
passes, the rate applies after it, the 513th connection held is dropped, and
closing ten lets ten more in. It was not loaded on the server, not exercised
over IPv6 or UDP, and not run beside Docker's and the host's own rules.

### Sign-in guesses and address budgets

Since 11.11.2026 Central counts guesses at passwords and gives each client
address a budget ([native login](native-login.md) §"Guessing a password
online" has the numbers and the reasons). All of it is in memory: nothing
is written to the database or a log, and a restart forgets every count and
every "own place".

**What an operator sees.** Central logs none of this. A refusal is a 429
with the route's fixed text; the proxy's access log, if one is kept, shows
them (Caddy's sites here keep none, see below). There is no counter or
admin page for them yet. A person who is refused sees the client's or the
page's "busy" message and cannot tell for how long: at most until the hour
or the day ends, counted from the first guess of that window.

**What people may report, and what it means.**

- *One person cannot sign in, others can, existing sessions work.* The
  account's count is spent: someone guessed at it (30 an hour, 100 a day
  from places that are not its own). The account's own places are served,
  so the person can sign in where they did in the last 30 days, since the
  last restart. There is no switch for one account. A restart clears all
  counts, and with them every own place; a guesser who keeps asking spends
  an account's minute again within seconds, so a restart alone does not
  let a person in who is kept out on purpose. What does: keeping the
  guessing addresses away at the proxy (its access log shows them, if one
  is switched on for the purpose), then waiting for the window to end or
  restarting.
- *Nobody at one site (an office, a campus, a carrier's shared address) can
  sign in to an account they never used there.* That address spent its 30
  an hour or 100 a day on wrong passwords. Accounts that signed in there
  before are served.
- *Everyone is refused for seconds at a time.* The 32 requests a second all
  clients share are spent: a flood from several addresses (one alone is
  held to 600 a minute; an IPv6 subscriber with more than a /64, a /56
  say, is several). Since 12.11.2026 not quite everyone: an address that
  was served fewer than 30 such requests in the minute still draws on 8 a
  second kept for such addresses, so a person who signs in once is served,
  and a page or a client that asks often is not. Rate limits at the proxy
  or the network answer the flood itself.
- *Signed-in clients are refused for a moment right after a restart, or
  while signing in works.* A session's first request after a restart, or
  after half an hour without one, shares 64 a second with every bearer no
  answer has vouched for yet (600 a minute for one address); its first
  answer vouches for it. Very many clients at once fill that second, and
  so does a flood of made-up bearers from seven addresses or more. The
  client asks again within a second; sessions already answered are not
  held up ([native login](native-login.md#a-bearer-nobody-issued)).
- *Someone changed or reset a password and is refused elsewhere.* Another
  password ends the account's own places but the one it was set from. If
  the account's count is spent by guessing, the other devices are refused
  until the window ends; the place of the change is served.

**The proxy must send the client's address.** Caddy does by default. Check
it after any change to the proxy, to the host's firewall or to the site
block, and after every deploy of Caddy itself: 31 `Discover` requests
within a minute through the public name must end in a 429, and the same
from another address must not. From a machine that is not the host:

```sh
for n in $(seq 1 31); do
  curl -s -o /dev/null -w '%{http_code}\n' https://central.baylee.app/v1/native \
    -H 'Content-Type: application/json' \
    -d '{"operation":"discover","username":"proxy-check"}'
done | sort | uniq -c
```

must print 30 answers of `200` and one of `429`. Thirty-one of `200` mean
Central saw no address: the proxy does not send the header, or something
other than the proxy reaches the listener. Then run it once more within the
same minute from a second address: it must begin with `200`. If it begins
with `429`, both runs were counted as one address: something in front of
Caddy is the last hop. (The name is made up and need not exist; the answer
is the same for a name nobody has. The command is written from the wire's
definition and was not run against the deployed name here.)

Central also says so itself. On the deployed account listener, a request to
`/v1/native` or `/v1/reports` that arrives without the proxy's address is
served as before and counted, and the service's log has a fixed line for the
first one and then at most one an hour:

```
central: 12 request(s) since start reached the account listener without the proxy's address; they are counted by no address (docs/operations.md, "The proxy must send the client's address")
```

`journalctl -u <the unit> | grep "without the proxy's address"` shows them.
The line holds the count and nothing of any request; there is no address to
write. No such line is the normal state: the released gateway on the same
host reaches Central through the public name unless it was pointed elsewhere, so through Caddy, and
`/health/live` is not counted. A line means one of three things: the proxy
stopped sending `X-Forwarded-For` (run the check above), a process on the
host talks to `127.0.0.1:28790` directly (a script, a test left running), or
someone on the host does so on purpose. Such requests are not refused:
refusing them would refuse every local run and whatever on the host the
operator points at the listener, and whether to refuse them on the deployed
listener is the owner's decision. A development listener writes no such
line.

- Without the header the account listener counts no address at all: each
  account's own count (30 an hour, 100 a day) and the shared budgets still
  hold, the per-address budgets and the own places do not, and everyone
  who guesses shares each account's count with its holder. The browser
  listener refuses such requests (400).
- A CDN or a second proxy in front of Caddy makes its address the last
  hop: all clients are then one place, and 30 wrong passwords an hour from
  anyone keep everyone from signing in to accounts they have not used
  before from there. Do not put one in front as things are: Central
  believes the last hop only, and which hop to believe behind two proxies
  needs its own decision and code.
- Every local, preview and end-to-end run arrives as one place
  (`127.0.0.1`, or none on the account listener without a proxy): 30
  unproven guesses an hour for all accounts together, 10 for one account,
  600 requests a minute without a session. A test suite that makes more
  wrong sign-ins than that against one running Central is refused; start a
  fresh Central per run.

**Constants to tune**, each with the reason beside it in the code:
`central-identity/src/guesses.rs` (the guesses: per account 8 a minute, 30
an hour, 100 a day; per account at a place 10 and 30; per place 30 and 100;
per IPv6 /48 120 and 400; an own place's 30 days and eight per account; the
tables' 16,384 entries, which must stay above the number of accounts that
sign in within 30 days), `central-identity/src/limits.rs` (a place's 600
requests a minute without a session and 240 pairing requests, beside the
shared 32 and 64 a second; the 8 a second kept for places that were served
fewer than 30 in their minute; for bearers no answer has vouched for, 64 a
second together and 600 a minute a place; the 30 minutes an answer vouches
for a bearer, the 16,384 held at once and the four refusals that take it
back; a vouched-for bearer's own 128 control and 64 data requests a second,
its address's 256 and 128, of the 512 and 256 all share),
`central-http/src/addresses.rs` (the native
listener's `Discover` 30 and `LoginStart` 30 a minute, `LegacyLogin` 10 a
minute, `ResetStart` 10 an hour, the directory, the public figures and the
donation figures 60 a minute together, Ko-fi's webhook 30 a minute) and
`central-http/src/browser.rs` (the browser listener's). Lower numbers slow guessing and make it cheaper to
keep someone out; higher ones the reverse. Changing one needs a build and a
deploy; none is in `central.json`.

## The browser listener

`browser.listen` in `central.json` (`{"browser": {"listen": "127.0.0.1:28791"}}`)
starts a second listener in the same process, for the website's registration
and sign-in ([web accounts](web-accounts.md)). It must be a numeric loopback
address and differ from `listen`; both are bound before the database is
reached. One signal stops both, and either one stopping stops the other.

It serves one route, `POST /api/v1`, with the operations of
`central_identity::browser_operation`, and answers 403 (`request not
permitted`) to anything that is not the site's own page: a `Host` other than
exactly `baylee.app`, an `Origin` other than exactly `https://baylee.app`,
`Sec-Fetch-Site` other than `same-origin`, a content type other than
`application/json`, no `X-Baylee-Central: 1`, any `Authorization`, a peer that
is not loopback, or an operation outside the list. No CORS header is ever sent.
Bodies are at most 64 KiB. Errors keep the native route's statuses and fixed
text. The browser session is the cookie `__Host-central` (`Secure`, `HttpOnly`,
`SameSite=Strict`, `Path=/`, `Max-Age=43200`, the session's 12-hour cap),
never in a body; signing out
clears it. Sign-in and session status answer there with the account's
canonical username (`BrowserSession`, `BrowserStatus`). The site is fixed to
`baylee.app` in this mode; only the development launcher takes another.

Per-address budgets read the last `X-Forwarded-For` hop, which Caddy sets to
the client it saw: 5 registrations an hour, 30 discoveries and 30 sign-in starts
a minute, and 60 reads of the gateway directory a minute (`GatewayDirectory`,
which needs no session; the website's `/gateways` page; `PublicFigures`, the
numbers the site shows, and `PublicFunding`, the donation figures, spend the
same 60). IPv6 addresses count by their /64. The counters live in memory only,
at most 4096 addresses per budget, and forget an address once its window has
passed (at the latest a window's length later). Every request that needs no
session also draws on its address's 600 a minute, and the sign-in's guesses
are counted by the same address ([above](#sign-in-guesses-and-address-budgets)).
The headers are checked
before a body is read; an address has at most four requests in flight (429
past them), a body two seconds to arrive and the service five to answer
(408), and the listener takes 128 connections at once (as the account
listener does). A request without a well-formed last hop is refused (400). Do not set
Caddy's `trusted_proxies` for this site unless a CDN in front of it appends its
own hop: without it Caddy ignores a client's own `X-Forwarded-For`.

The `baylee.app` site block ([the server-rendered website](#the-server-rendered-website-central-web)) passes `/api/*`
to it instead of answering 404, with the `Host` unchanged (Caddy's default for
a plain-HTTP upstream):

```caddyfile
	# Profile and group picture uploads (docs/profile-pictures.md): the route's
	# own bound, 8 MiB and its frame; the longer path is tried first.
	@picture_upload path /api/v1/picture /api/v1/groups/*/picture
	handle @picture_upload {
		request_body {
			max_size 9MiB
		}
		reverse_proxy 127.0.0.1:28791
	}
	# Central's browser listener (docs/web-accounts.md).
	handle /api/* {
		request_body {
			max_size 64KiB
		}
		reverse_proxy 127.0.0.1:28791
	}
```

## Database transport

`database` holds `host`, `port`, `name`, `user`, `password` and
`ca_certificate` (a PEM bundle). The connector (`central_postgres::ServerConfig`)
always uses `sslmode=verify-full`: the server certificate must chain to the
configured authority and carry the configured address as an IP subject
alternative name. A server that declines TLS ends the attempt; no plaintext
startup message or password follows (`tests/tls_refusal.rs`). After connecting,
the pool checks `pg_stat_ssl` for its own session. The password and authority
reach the driver through typed options, never through the connection URL.

The host must be a loopback or private IP literal. The driver's trust store is
the public web roots plus the configured authority, not that authority alone;
public authorities do not certify loopback or private addresses, so only the
configured authority can vouch for such a server. DNS names are refused because
any public authority could vouch for one. A CA-only verifier would lift this.

Server side, PostgreSQL needs `ssl = on` with a certificate for that address
and its key owned by the database user with mode `0600`. Central's database must
refuse unencrypted sessions in `pg_hba.conf`, above every other `host` line:

```text
hostnossl baylee_central all all reject
hostssl   baylee_central central_migrator,central_runtime,central_legacy all scram-sha-256
```

## Provisioning (once, as a database superuser)

Generate each password with a CSPRNG (`openssl rand -hex 32`) and keep it only
in the configuration files below. The runtime role must neither own nor be a member
of a role owning anything it uses; the store refuses to start otherwise.

```sql
CREATE ROLE central_migrator LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS PASSWORD '<migrator>';
CREATE ROLE central_runtime  LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS PASSWORD '<runtime>';
CREATE ROLE central_legacy   LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS PASSWORD '<legacy>';
CREATE DATABASE baylee_central OWNER central_migrator;
REVOKE CONNECT, CREATE, TEMP ON DATABASE baylee_central FROM PUBLIC;
GRANT CONNECT ON DATABASE baylee_central TO central_migrator, central_runtime, central_legacy;
\connect baylee_central
REVOKE CREATE ON SCHEMA public FROM PUBLIC;
```

Then run `--migrate-stdin` with the migrator login, and grant the runtime role
exactly the privileges its startup checks require: `ops/grants.sql`, as
`central_migrator`, which owns the tables. It needs no superuser, and the
deploy hook, which runs it after every migration, refuses one (since
12.11.2026; before, it ran the file as the database's superuser, whose grant
PostgreSQL records as the owner's). It is idempotent, and it is SQL only:
the hook refuses a file with a backslash in it, a comment's too
([below](#the-grants-file-and-login-12112026)).

A later migration that adds a table also adds its grant to `ops/grants.sql`; the runtime
store's startup check names every table and privilege it needs. Upgrading a
running service across such a migration: stop it, migrate, grant, start it;
started before the grant, it refuses to run (the check fails closed).

The terms tables (`docs/terms.md`) get `SELECT, INSERT` only: published
gateway versions and acceptances are immutable records, and an account's
acceptances go with the account by cascade.

## Registration

Registration on `baylee.app` is open (owner, 10 and 11.11.2026; [registration
abuse](registration-abuse.md)): no key, but Central's own proof of work,
harder while registrations flood in, and the per-address budgets of the
browser listener (`Register` 5 an hour and 10 a day). The closed beta's keys
and `--issue-beta-keys-stdin` are gone; migration 000023 drops their table
and an unfinished registration's link to a key, nothing else.

**The brake.** An administrator closes registration during an attack, and
opens it again by hand, on the account overview of the website (or with
`AdminRequest::SetRegistration`); a change needs a password sign-in under 30
minutes old and is an audit event (`registration.closed`,
`registration.opened`). While closed, the page says registration is paused
and Central refuses every `Register`.

**Deploying it: the site first, then Central.** The register page of this
release asks for the proof that costs memory (`RegisterChallengeFor`,
answered `RegisterWork`); at a Central that does not know it, it asks the
older way and solves the older proof. So a Central with the SHA-256 proof
(the builds of 11.11.2026 before this one) goes on registering people
while the new site is already out, and there is no gap. The other order
has one: a Central of this release answers the older request with
"registration is paused" and takes no SHA-256 solution, so a register page
from before cannot register until it is replaced, and **a tab of it left
open cannot register until it is reloaded** (nothing else on that page is
affected; the message it shows is `register-closed`). Three things to know:

- `ops/deploy-hook` switches Central, then installs the server-rendered
  site (since 12.11.2026; it no longer touches the static build of `web/`,
  which is gone from the repository). What `baylee.app` serves is still
  Caddy's to say: until Caddy passes the site to `central-web`
  (§"The server-rendered website"), production keeps the **frozen** static
  build, whose register page sends a key that no Central since migration
  000023 takes. So the first run of the hook (or the manual switch) after
  this release should be followed at once by the Caddy switch, or preceded
  by it with the unit installed and the site in place: in between,
  registration does not work. That timing is the owner's.
- Once `baylee-app-web` is installed, a website that does not answer
  within 30 s after the hook restarts it puts the previous site back and
  **fails the hook, which stops the gateway's deploy** as a failed Central
  does (Central itself stays deployed). The unit was written without a
  systemd at hand; the owner's first start is its test.
- Coming from a Central that still has the keys, neither order registers
  anybody in between: that Central knows neither challenge request, and
  this one takes no key. Keep that switch short.
- Migrate and grant (`ops/grants.sql` now grants the switch's row instead
  of the keys) before the new service starts, as for every migration. The
  proof itself needs no migration, grant or setting.

**What it costs the service.** Checking a registration's proof takes one of
the service's four workers, the ones sign-in uses, and 16 MiB while it
runs: 64 MiB for all four at the most, a few milliseconds of one core for a
wrong answer and eight times that for a right one
([registration abuse](registration-abuse.md) §"A proof that costs memory"
has the measurements and what one address can make Central compute). With
no worker free the page is told to wait and sends its solution again.

## Administration

Central's administrators act on the website ([administration](administration.md));
the operator appoints the first one, and the way back in when none can sign
in, with the migrator login from a file the operator keeps:

```sh
central-service --appoint-admin-stdin < appoint.json
```

```json
{"database": {"host": "…", "port": 5432, "name": "baylee_central", "user": "central_migrator",
  "password": "…", "ca_certificate": "…"},
 "username": "…"}
```

The account must be enrolled. It prints `administrator appointed`, or
`already an administrator; nothing changed`, on standard error, and writes an
audit event with the operator as its actor. Keep the username out of the
repository and of shell history: the file holds it.

The audit keeps administrators' and the operator's actions for three years;
a daily timer deletes older events, the digests of expired reset codes,
expired pairings, e-mail addresses whose confirmation code has ended and
ranked queue tickets closed for a day ([ranked](ranked.md#publication-and-storage)),
with the migration document
(`/etc/baylee-central/migrate.json`), under the same `systemd-run` user as
the deploy hook's migration:

```ini
# /etc/systemd/system/baylee-central-sweep.service
[Service]
Type=oneshot
User=central
ExecStart=/opt/baylee-central/bin/central-service --sweep-stdin
StandardInput=file:/etc/baylee-central/migrate.json
NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=yes
PrivateTmp=yes

# /etc/systemd/system/baylee-central-sweep.timer
[Timer]
OnCalendar=daily
RandomizedDelaySec=1h
Persistent=true

[Install]
WantedBy=timers.target
```

The runtime role may grant and remove administrator roles and append audit
events, never rewrite or delete one (`ops/grants.sql`, checked at startup).
What a taken-over service gains by that is in [administration](administration.md)
and the [threat model](threat-model.md).

## AI players

AI players ([AI players](ai-players.md)) are accounts with no password that
fill a lonely person's ranked search and play each other in the carousel.
An administrator manages them on the website (`/account/admin/ai`); the
operator, who signs in to the website as no administrator, uses one command
under the migrator login instead. Both take the same code, the same checks
and write the same audit events; the operator's name the operator as their
actor.

```sh
central-service --ai-players-stdin < ai-players.json > runner-token.txt
```

```json
{"database": {"host": "…", "port": 5432, "name": "baylee_central", "user": "central_migrator",
  "password": "…", "ca_certificate": "…"},
 "players": [
   {"display_name": "KI-Haus-Expert", "tag": 6699, "kind": "house", "profile": "house:expert",
    "label": "Haus-KI · Expert", "max_games": 4, "enabled": true},
   {"display_name": "KI-Haus-Mittel", "tag": 6700, "kind": "house", "profile": "house:medium",
    "label": "Haus-KI · Mittel", "max_games": 4, "enabled": true}
 ],
 "settings": {"fill_in": false, "global_seats": 16, "carousel_games": 8, "carousel_daily": 10000},
 "revoke": [],
 "token": {"name": "runner-bigace", "accounts": ["KI-Haus-Expert", "KI-Haus-Mittel"], "days": 90}}
```

Every field but `database` may be left out. The plan is applied in one
transaction, whole or not at all:

- **`players`.** An AI player is named by its display name (which begins
  with `KI-`) and its tag. With neither taken, it is created switched off,
  with no password, and `enabled` then switches it on. With both held by one
  AI of the same kind and profile, it is reported as `unchanged`, or
  `changed` when the label, `max_games` or `daily_games` differ; nothing is
  made twice. Anything else is ambiguous and refuses the whole plan: a tag a
  person or another AI holds, the name under another tag, another kind or
  profile (a mind is fixed, [AI players](ai-players.md) §1). `daily_games`
  is required for `"kind": "model"`. `house:seer` is refused.
- **`settings`.** The settings for every AI; those left out stay. Calibration
  is `fill_in` false with high `carousel_games` and `carousel_daily`
  (§7); `global_seats` 0 stops every AI.
- **`revoke`.** Runner token ids, as the listing prints them. Each ends every
  session it minted in the same transaction; one revoked already is no
  error, an unknown id refuses the plan.
- **`token`.** A runner token for one runner machine, listing 1 to 16 AI
  players by display name (each must name exactly one AI), living `days`
  (1 to 366, 90 when left out). A live token of the same name refuses the
  plan: revoke the old one in the same plan to rotate. Its text, `bat_` and
  64 hex digits, is the only thing the command writes to standard output,
  once, after the plan committed. Central keeps its digest alone; the text
  cannot be shown again.

What was done goes to standard error: each player's name, tag and outcome,
the settings as written, each revocation with the number of sessions it
ended, and the new token's id and expiry. A refusal is one line,
`AI players rejected the input, found it ambiguous, or failed; nothing was
changed`, and changes nothing.

`{"database": {…}, "list": true}` alone lists the runner tokens on standard
output, one line each: id, name, accounts, expiry, last use, revocation,
and whether an administrator or the operator made it. Never a digest or a
text.

### A runner token: issuing, rotating, revoking

- **Issuing.** One token per runner machine, 90 days. Write the command's
  standard output straight into the runner's credential, readable by root
  alone (for a systemd unit, `LoadCredential=` from a root-only file), and
  delete the plan's file or keep it where the migrator login is kept. The
  runner trades the token for sessions on the public native endpoint
  (`central_client::ai_runner_session`); there is no address allowlist.
- **Rotating.** Before the old token expires: run a plan with the old id in
  `revoke` and a `token` of the same name, then put the new text in place and
  restart the runner. The old token's sessions end with the commit, so the
  runner's running matches end with them: rotate between matches, or with
  `enabled` false for its players first and wait until their live matches
  are 0 (the administration's page or `Work` shows them).
- **Revoking.** At once, when a machine or its credential may be lost: the
  id in `revoke` (or the website's Revoke). Every session the token minted
  ends in the same transaction and every seat it held is revoked at its
  host. A thief with the token could play the AI players it lists, against
  people, until then; nothing else ([AI players](ai-players.md) §2, "What a
  thief gains").
- **Watching.** The listing's last use moves at every exchange; the AI
  page shows each player's last runner sign-in. A runner should alert when
  its token is refused.
- **An older binary.** A binary from before AI players knows no runner
  session and refuses nothing to one: it would serve a live runner session
  as an ordinary native session, through which a holder could set an
  address and a password for the AI. The deploy hook's fallback goes to the
  binary just before, which knows them once this is deployed. Before running
  any older binary by hand, end every runner session as the migrator:
  `DELETE FROM central_identity.sessions WHERE runner_token IS NOT NULL;`

## Deleting and exporting accounts

A person deletes and exports their own account ([account
deletion](account-deletion.md)); nothing is deployed for it, no migration and
no grant. What stays the operator's:

- **An administrator who wants to leave:** another administrator removes the
  role first; the last one is replaced by an appointment (above) first.
- **An export that says `complete: false`:** a list passed its 256 entries.
  Answer the rest by hand, read-only, under the migrator login: the
  account's rows in `terms_acceptances`, `gateway_admins` and
  `audit_events` (as `account_id` and as `actor_account`).
- **A moved account's copy at the official gateway:** a move keeps the
  source data there, its password retired; a deletion at Central does not
  reach it. Until M8 removes it, delete that copy at the gateway when the
  person asks (the official gateway's own runbook).
- **An origin registered again after its owner was deleted:** the old
  owner's terms for that origin are still stored. Before approving the new
  registration, delete them under the migrator login (their texts go with
  them):

  ```sql
  DELETE FROM central_identity.gateway_terms WHERE audience = '<origin>';
  ```

- **Backups** keep a deleted account until they rotate out, at most 12
  months, as the privacy notice says. A restored backup brings deleted
  accounts back; they are erased again by id ([below](#after-a-restore)).
- **An erasure by id,** after a restore, or on a request whose sender the
  operator has verified as the account's holder: `--erase-accounts-stdin`
  ([below](#after-a-restore)).

## After a restore

*26.11.2026.* A restored backup is an earlier state of Central's database, and
two things in it are wrong for the present: the accounts deleted since the
backup are back, and every gateway's revocation stream stands at the backup's
number, behind what the gateways have read. `baylee-backup` runs both steps
after `pg_restore` and before it starts Central again, in this order, under
the migration document as the deploy hook's migration does:

```sh
systemd-run --wait --collect --quiet --pipe -p User=central -p Group=central \
    /opt/baylee-central/bin/central-service --after-restore-stdin \
    < /etc/baylee-central/migrate.json
```

**`--after-restore-stdin`** takes the migration document as it is. In one
transaction it moves every gateway's revocation stream 2^40 past its last
number (`central_postgres::RESTORE_STEP`) and drops the numbers still held, so
that each stream's lowest number is its last plus one. A gateway that had
read any number then finds, at its next catch-up (within 30 seconds), that it
missed something it cannot learn, ends every Central session it holds, and
goes on from there; its players present fresh tokens, which Central gives
only to live sessions ([gateway registry](gateway-registry.md#revocations)).
No gateway needs a restart. Streams are made for the official gateway and
for every registered gateway that had none, since a gateway may have read
numbers the backup never saw. 2^40 revocations at one gateway in the twelve
months a backup is kept would be 35,000 a second; a stream that would pass
2^53 - 1 refuses the whole step (8,191 restores from 0). Central's own
sessions stay: a backup holds none (`baylee-backup` leaves `sessions`, the
`sid`s and the revocations out), and ending them would make a mistaken run
cost everyone their password instead of a fresh token. It prints
`{"streams": N}` on standard output and ends with 0, or a line on standard
error and 1. Run again, it moves every stream once more, which costs the
gateways' players a fresh token and nothing else.

**`--erase-accounts-stdin`** takes the migration document with the ids to
erase ([account deletion](account-deletion.md#the-operators-erasure-26112026)):

```json
{"database": {"host": "…", "port": 5432, "name": "baylee_central", "user": "central_migrator",
  "password": "…", "ca_certificate": "…"},
 "accounts": ["0193…-…", "…"]}
```

1 to 1,024 ids (`ERASE_BATCH`; a longer list goes in batches), each a UUID in
its hyphenated text form, in either case; a repeated one counts once. The
whole document is checked before anything is erased, and one id that is no
UUID refuses it all. Each account is erased in its own transaction, as its
own deletion does, with its legacy store row and an `account.erased` audit
event; an administrator and an open move are not refused. An id with no
account counts as absent. It prints
`{"erased": N, "absent": M, "administrators": K}` on standard output, never
an id, and ends with 0 once every id is erased or absent; `K` is how many
administrators are left, and at 0 it says on standard error to appoint one
(`--appoint-admin-stdin`, [above](#administration)). A failure ends it with
1 and says on standard error how far it got; what it erased stays erased,
and the same list runs again. Keep the ids out of shell history and logs:
`baylee-backup`, which runs as root, reads `migrate.json` itself and writes
the document to the mode's standard input.

Both steps are safe with Central running, too: a revocation numbered while the
step waits is covered by it, and an erased account's sockets learn within 70
seconds that their session ended. What a restore undoes beyond this (revoked
gateway tokens, role changes, used reset codes and friend invites) the
operator does again by hand; `baylee-backup`'s report lists it.

## Gateways and their tokens

Since 30.10.2026 Central's administrators approve, disable and enable
gateways on the website (`/{lang}/account/admin/gateways` since 12.11.2026; the
old build's `/admin/gateways` before), and owners mint and revoke their
gateways' tokens there ([gateway registry](gateway-registry.md) §"Gateway
tokens"). The deploy hook applies `ops/grants.sql` after the migration,
which gives the runtime `INSERT (gateway_id, enabled)` and
`UPDATE (enabled, ranked_verified)` on
`gateway_approvals` and its rights on `gateway_tokens`; a binary from before
migration 000014 refuses to start over the wider grants, so roll back the
grants with the binary. The operator's SQL stays the fallback when no
administrator can sign in:

```sql
INSERT INTO central_identity.gateway_approvals (gateway_id)
    SELECT id FROM central_identity.gateways WHERE origin = 'https://…';
UPDATE central_identity.gateway_tokens SET revoked_at = clock_timestamp()
    WHERE gateway_id = (SELECT id FROM central_identity.gateways WHERE origin = 'https://…')
    AND revoked_at IS NULL;
```

### Enabling a ranked host

Sign in as a Central administrator, open **Administration → Gateways**, select
the approved origin and choose **Authorize ranked hosting**. The confirmation
names the origin entrusted with match results. When the host uses a named
gateway token, its owner opens the gateway's token form and explicitly selects
**Hosts ranked matches and reports results**. Deploy this newly issued token
through the host's normal credential store. Existing scoped tokens keep their
previous permissions. Withdrawing ranked hosting under Administration invalidates
outstanding assignments. To rotate a compromised named credential, revoke it
and withdraw ranked hosting before issuing a replacement.

### Replicated ranked operations

Roll out the Central migrations and runtime grants, then deploy compatible gateway and engine builds under at least two independently registered gateway owners. Both gateways must hold current ranked verification and advertise available capacity with the same compatibility digest. Two installations registered to one owner cannot form an independent pair. The native client displays the assigned operators and asks both players to approve private disclosure. Upgrading only one operator permits single-host play when the initial allocator finds no independent compatible pair.

During an outage, keep the remaining gateway online and advertise capacity on a compatible verified replacement. Each role holds a 45-second lease. An expired primary can be replaced by a standby that has certified the last committed position. A new generation fences prior host messages and revokes the old game admission. A newly assigned operator waits for both players' approval of that origin and the current terms before receiving the journal. During recovery the affected players see the paused table. The five-minute recovery window bounds the wait for an independent verifier. A disagreement is quarantined. Its recovery deadline releases the table without Elo. Host verification can be withdrawn through the same administrator control at any time.

For triage, read the match identifier, generation, phase, certified position and number of receipts through restricted operational access. A settled 1v1 match has two seat receipts. A technical void has zero. Use these metadata in incident reports. Private openings, deck pools, journal bodies, credentials and host evidence belong only on the restricted authority or approved executors. Successful games permit a two-minute post-finish replay window. A bounded maintenance pass then erases the authority's private replay bodies and opening. Voids become eligible for cleanup immediately. See [replication assurance](ranked-replication-assurance.md).

### An origin's host may not end in a dot (12.11.2026)

`https://host.` names the same host as `https://host` and was taken as a
second origin; Central now refuses it wherever an origin is checked
(`central_api::gateway::validate_origin`). A gateway that was registered
under such a spelling would have its registration, tokens and heartbeat
refused after the deploy. None is expected. Look once before deploying, as
the database's owner, and re-register what it finds under the plain
spelling:

```sql
SELECT id, origin FROM central_identity.gateways
 WHERE origin ~ '\.(:[0-9]+)?$';
```

### What gateways report (migration 000025)

Since 11.11.2026 a gateway may send its heartbeat as a `GatewayPulse` with
four counts (players, of those the players in a game, tables, games), and
anyone may read them and the number of accounts with `PublicFigures` ([gateway
registry](gateway-registry.md#activity-and-the-public-figures)). For the
operator:

- **Deploying it** is the usual order: stop, migrate, start. The migration
  adds five nullable columns to `gateways` and needs no new grant
  (`ops/grants.sql` is unchanged). The fourth count's column
  (`activity_playing`) is part of 000025 itself: a test database that ran
  the migration before that column was written into it is made anew
  ([PostgreSQL](postgresql.md#gateway-activity-migration-000025)); 000025
  had run on throwaway databases only. Migrate before the new binary starts:
  the startup check asks about grants, not about columns, so the new
  binary would start over the unmigrated table, and then every heartbeat
  and every read of the figures fails until the migration is applied
  (gateways drop out of the directory's `online` after 90 seconds).
  The other way round is expected to work and was not tried: the earlier
  binary's statements name their columns, none reads a gateway's row
  whole, its startup check does not look at the migration history, and
  its heartbeat statement runs over the migrated table in the upgrade
  fixture; the earlier binary itself was not started over it.
- **Nothing is switched on here.** Central takes a pulse from every gateway
  that may send a heartbeat. Whether a gateway reports is its operator's
  setting on the gateway, the official one included; until the game's
  gateway sends pulses, `PublicFigures` lists no gateway and still answers
  the number of accounts.
- **Taking a gateway's numbers away.** Disabling the gateway
  (`/{lang}/account/admin/gateways`) hides them at once. To withdraw the numbers alone
  while the gateway stays listed, clear them; they come back with the
  gateway's next pulse, so its operator has to stop reporting as well:

```sql
UPDATE central_identity.gateways
    SET activity_players = NULL, activity_playing = NULL, activity_tables = NULL,
        activity_games = NULL, activity_at = NULL
    WHERE origin = 'https://…';
```

- **What it costs.** A read is one page of the gateway table; the number of
  accounts is counted over `accounts` once a minute at most and kept in the
  service's memory, so it starts again from a count after a restart. No
  history is kept, nothing is logged, and there is nothing to prune.

## The official gateway

The directory lists the official gateway (the bridge's,
`https://gateway.baylee.app`, `central_api::OFFICIAL_ORIGIN`) from a row of its
own: no owner, no credential, so the bridge stays the only authority for its
origins ([gateway
registry](gateway-registry.md#discovery-heartbeats-and-administrators)).
Migration 000009 makes room for it; listing it is the operator's, under the
migrator login (`central_migrator`, which owns the tables), once:

```sql
INSERT INTO central_identity.gateways (official, origin, name, version, website)
    VALUES (true, 'https://gateway.baylee.app', 'Baylee', '0', 'https://gateway.baylee.app/');
INSERT INTO central_identity.gateway_approvals (gateway_id)
    SELECT id FROM central_identity.gateways WHERE official;
```

Until its first heartbeat the entry shows these values and offline. The
official gateway's `BAYLEE_CENTRAL_CONFIG_FILE` gains `metadata` beside its
`bridge` (Baylee `docs/central-login.md`), for example `"metadata": {"name":
"Baylee", "version": "0", "website": "https://gateway.baylee.app/"}`; the website
must be on one of its origins, and the gateway sends its own binary's version in
place of `version`. It then sends `GatewayHeartbeat` with the bridge credential every
30 seconds, which replaces the name, version and website and marks it online.
Without `metadata` the entry stays listed and offline. A second official row is
refused, as is an owner or a credential on it; `RegisterGateway` for the
origin answers `Conflict`.

**Reaching it from here.** Central calls a gateway only over HTTPS to public addresses, and the
official gateway runs on this machine, where its public name may not lead back to it. Set
`official_address` to the numeric address its TLS terminator answers at, usually
`"127.0.0.1:443"`. It applies to the compiled `OFFICIAL_ORIGIN` host alone, and the URL, the name
sent in TLS and the certificate check stay what they were, so the terminator must serve that
name on that address. Only a loopback or private address is taken, and a value that cannot be read
stops the service. Without the setting the name is resolved like any gateway's, and if it does not
lead back here, revocations reach the official gateway by catch-up, in seconds rather than at once.

**The move to `gateway.baylee.app` (30.10.2026).** The official gateway's
canonical origin is `OFFICIAL_ORIGIN`, and its old address is an alias
(`OFFICIAL_ALIASES`), both compiled: a token asked for the old address is the official
gateway's, with that address as its `aud`; an acceptance of the gateway's terms under
either counts; the old address cannot be registered. The heartbeat finds the
row under either origin, so the operator moves the row in the deploy's window,
after checking that no registered gateway holds the new origin:

```sql
SELECT id FROM central_identity.gateways WHERE origin = 'https://gateway.baylee.app';  -- no rows
UPDATE central_identity.gateways SET origin = 'https://gateway.baylee.app',
    website = 'https://gateway.baylee.app/' WHERE official;
```

Until then the directory shows the old address, which the website does not
mark official. Dropping the alias is a release that empties
`OFFICIAL_ALIASES`, once installed clients ask for the new address. To hide the
entry, disable its approval:

```sql
UPDATE central_identity.gateway_approvals SET enabled = false
    WHERE gateway_id = (SELECT id FROM central_identity.gateways WHERE official);
```

## Display names and tags

Migration 000011 (the accounts and administration design, step 3; [web
accounts](web-accounts.md) §"Display name and tag") draws a tag for every
registered account from `0x1000` up, and moved accounts keep the official
gateway's tags, which must stay below that floor
(`profile::REGISTERED_TAG_FLOOR` in `central-api`, and the `4096` of the
migration's backfill). Before the migration runs in production, read the
official gateway's highest tag, guests included, on its own database:

```sql
SELECT max(tag), count(*) FROM account;
```

It answered `13 | 7` on 23.10.2026, so the floor stays. Were it `0x1000` or
more, the floor would move to the next multiple of `0x1000` above it, in
both places, before 000011 is applied. The official gateway makes no local
accounts once bound to Central, so the number cannot grow afterwards; a
carried tag that is taken anyway is replaced by a drawn one.

No grant changes. The deploy order: Central first (the hook does so: it
installs the revision the gateway links, migrates and grants before the
gateway starts), then the gateway whose bridge sends the name and tag
(Baylee pinned to this revision). The previous Central binary keeps working
on the migrated schema (it registers without a tag; the new one draws it at
that account's next sign-in), and a bridge from before keeps reserving
without name and tag. The privacy notice's new version (display name and
tag, `legal/privacy.*.md` §3, §6 and §9) has to be published before the
website offers the field.

## The legacy store

From the cutover until its deadline, Central signs in the official gateway's
accounts that had not moved, from its legacy store ([native
login](native-login.md#legacy-login)), and the gateway has no password route.
The store is schema `central_legacy` (migration 000015) with a login role of
its own; the runtime role has nothing in it.

**Once, before deploying a release with migration 000015** on a database
provisioned before it, as a superuser (`openssl rand -hex 32` for the
password), and add `central_legacy` to the `pg_hba.conf` line above:

```sql
CREATE ROLE central_legacy LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS PASSWORD '<legacy>';
GRANT CONNECT ON DATABASE baylee_central TO central_legacy;
```

`ops/grants.sql` grants it exactly what its startup check requires, and
nothing while the role does not exist; a service whose `central.json` has a
`legacy` block then refuses to start.

**The cutover,** in the official gateway's maintenance window, in this order
(Baylee `docs/central-login.md` §"Cutover"): stop the gateway and deploy its
release without password routes; then, as the migrator, in one pipe and
without a file in between:

```sh
{ cat /etc/baylee-central/migrate.json; sudo -u baylee env DATABASE_URL=… baylee-gateway legacy export; } \
  | central-service --legacy-import-stdin
```

The first document is the migration document (`{"database": …}`), the
second the gateway's export, which holds every remaining password hash and is
refused on a terminal. The import checks every row (canonical username, the
display-name and tag rules, the snapshot's shape and bound), refuses the whole
import for one bad row, for a username another Central account holds, for a
move closed already, for a non-empty store, or for a count or digest that is
not the export's, and stores nothing then. In one transaction it stores the
rows, reserves each account's identity under its name, and computes the count
and digest again from what it stored. Compare its line on standard error with
the export's: the counts and digests are equal. Then add the `legacy` block
(the `central_legacy` login, `{"database": {…, "user": "central_legacy", …}}`)
to `central.json`, restart Central, start the gateway.

Migration 000039 keeps the original SHA-256 and source bytes for accounts with larger
deck libraries. Above 192 KiB the import stages an account-bound manifest for encrypted
adoption in 64-KiB pages. Each such source must contain decks and its serialized
preferences must fit within 192 KiB. The maximum source size is 8 MiB per account.
The two input documents together must fit within 32 MiB, including JSON byte-array encoding.

**The deadline** is the owner's date, **30.06.2027** (end of day,
Europe/Berlin): migration 000015 sets it, so it holds without a step. The
operator moves it, or clears it, only on the owner's word:

```sh
central-service --legacy-deadline-stdin < legacy-deadline.json
```

```json
{"database": {"host": "…", "port": 5432, "name": "baylee_central", "user": "central_migrator",
  "password": "…", "ca_certificate": "…"},
 "deadline": "2027-06-30"}
```

The date is the last day a legacy sign-in is taken: the deadline is the end
of that day in Europe/Berlin; `null` clears it (the store then stays until a
date is set again). From then no legacy sign-in is taken, and the daily sweep (`--sweep-stdin`, [administration](#administration))
deletes every row left and every legacy reservation that never enrolled; it
deletes the row of a closed move at any time. Administrators see what is left
on the website, at `/{lang}/account/admin/legacy` (`AdminRequest::LegacyAccounts`). After the deadline's sweep,
remove the `legacy` block, revoke the role's grants and drop the role; a later
release removes the schema.

**Refused moves.** A forced password change or a ban in force at the gateway
refuses the move, after a correct password, with the same answer as a wrong
one; the listing shows them. Ten failed sign-ins in a row lock an account out
for an hour (the listing shows until when).

## Reports (migration 000018)

Bug and crash reports ([reports](reports.md)) arrive on the native listener
beside `/v1/native`, from the bridge (the official gateway) or an approved
gateway's token with the `sso` scope:

- `POST /v1/reports`, JSON at most 64 KiB: `201 {"report_id": "<32 hex>"}`.
- `PUT /v1/reports/{id}/client` and `…/record`, `application/gzip` with a
  `Content-Length` (`411` without, `413` past 64 MiB): the credential and
  the report's announcement are checked before the body is read, which must
  be exactly the announced length; at most four uploads at once and two of
  one gateway (until 12.11.2026 two at once for everyone), each within
  60 seconds (the native listener's connections may last that long; every
  other request keeps its 5 seconds) and bringing at least 16 KiB in every
  five of them until it is whole, else `408` and its slot is free again;
  `204`.

Since 25.11.2026 report tokens use the same routes and more
([below](#report-tokens-migration-000052)), and the credential of every one
of them is asked for before its body is read. Metadata over 64 KiB is `413`
(it was `401`).

The browser listener adds `POST /api/v1/reports/{id}/client|record` for an
administrator's page, with every check of `POST /api/v1`, 120 a minute per
address; the answer is the gzip as stored. The proxy compresses the JSON
answers; the gzip parts it passes as they are.

The runtime's grants (`ops/grants.sql`, checked at startup like the
others): `SELECT, INSERT, DELETE` on `reports` and `UPDATE` of its triage
columns only; `SELECT, INSERT, DELETE` on `report_parts`, `report_notes`,
`report_seen` and `report_views` (with `UPDATE (query)`); `SELECT, INSERT`
on `report_audit`. The daily sweep (`--sweep-stdin`) deletes reports past
their retention and says how many. No secret and no setting is new.

### What reports may take, and how to make room (12.11.2026)

[Reports](reports.md) §"What the reports may take" has the rules and the
reasons. The numbers are constants in the code; changing one is a build.

| what | number | where |
| --- | --- | --- |
| a registered gateway's reports | 2 GiB | `central-postgres`, `GATEWAY_STORED` |
| the official gateway's reports | 8 GiB | `OFFICIAL_STORED` |
| all reports | 16 GiB | `ALL_STORED` |
| uploads at once | 4 | `central-http`, `UPLOADS` |
| uploads of one gateway at once | 2 | `UPLOADS_PER_GATEWAY` |
| uploads a gateway may leave unfinished in an hour | 30 | `UNFINISHED_PER_HOUR` |

**Before the first deploy with these bounds**, and whenever reports are
refused, look at the levels. As `central_migrator` (or any login that may
read the two tables), with the file from a checkout of the deployed
revision; the names are the deploy hook's defaults:

```sh
docker exec -i baylee-postgres-1 psql -X -q -v ON_ERROR_STOP=1 \
    -U central_migrator -d baylee_central < ops/report-levels.sql
```

It prints a line for each gateway and one without a gateway for all:
`bytes` is what a filing is weighed against, `counted` the same in words.
It changes nothing and reads no part's bytes. If the official gateway's
line is near 8 GiB (8589934592) before the deploy, its reports will be
refused from the first one after it: purge first, or raise the number.

**What a gateway sees past a bound:** `429` to `POST /v1/reports`. The
released gateway asks twice more (after 250 ms and a second) and then
tells its player the report failed. Nothing is logged: Central writes no
line for a report, and none for a refusal.

**To make room:** an administrator purges reports on the website
(*Feedback*: move to the trash, then purge; the trash alone frees nothing
for its 30 days). The sweep frees what is done for 90 days, in the trash
for 30 or older than 1,095. A gateway that is filling its bound with
reports nobody asked for is disabled like any other (`/{lang}/account/admin/gateways`);
its reports stay until they are purged.

**Memory.** With four uploads at once, parts of the largest size (64 MiB)
can hold about 768 MiB for a moment (by reading the code, not measured;
two uploads: 384 MiB). The unit sets no memory limit. On a server that
cannot spare it, build with `UPLOADS` at 2; one gateway's two uploads
then refuse every other gateway's parts again.

**What these numbers do not bound:** the disk itself. PostgreSQL's files
do not shrink when rows go, and the write-ahead log, the indexes and the
backups take room of their own. Watch the volume as before.

Not tested: none of this ran on the server. The bounds and the levels
query are tested against the PostgreSQL fixture, the slots over HTTP
against a local listener.

### Report tokens (migration 000052)

[Reports](reports.md) §"Report tokens" has the rights, the routes and the
reasons. An administrator makes a token on
`/{lang}/account/admin/feedback-tokens`; its holder sends

```
Authorization: Bearer brt_<64 hex digits>
Baylee-Central-Protocol: 2
Content-Type: application/json
```

to `https://central.baylee.app` (the proxy passes the whole host to the
native listener, so nothing in Caddy changes). A request from a page is
refused as ever (`Origin`, `Sec-Fetch-Site`).

| request | right | body | answer |
| --- | --- | --- | --- |
| `POST /v1/reports` | `file` | the metadata, `"channel":"token"`, `"gateway":{}` | `201 {"report_id"}` |
| `PUT /v1/reports/{id}/client`, `…/record` | `file` | gzip, `application/gzip`, `Content-Length` | `204` |
| `POST /v1/reports/search` | `read` | `{"filter":{…},"sort","descending","offset","limit"}`, all optional | `{"reports":[…],"total"}` |
| `POST /v1/reports/facets`, `…/crashes` | `read` | `{"filter":{…}}` | counts, crash groups |
| `GET /v1/reports/stats` | `read` | none | `{"days":[…]}` |
| `GET /v1/reports/{id}` | `read` | none | the report, its notes and history |
| `GET /v1/reports/{id}/client`, `…/record` | `read` | none | `application/gzip`, `X-Record-Origin` with a record |
| `GET /v1/reports/admins` | `triage` | none | `{"admins":[…]}` |
| `PATCH /v1/reports/{id}` | `triage` | `{"change":{…},"expected_revision":n}` | the report |
| `POST /v1/reports/{id}/trash`, `…/restore` | `triage` | none | `204` |
| `POST /v1/reports/{id}/notes` | `triage` | `{"text"}` | the report |
| `DELETE /v1/reports/{id}/notes/{note}` | `triage` | none | the report; a token's own note |
| `POST /v1/reports/bulk` | `triage` | `{"reports":[ids],"bulk":{"do":"change","change":{…}}}`, or `trash`, `restore` | `{"changed"}` |
| `DELETE /v1/reports/{id}` | `purge` | none | `204`, from the trash only |
| `POST /v1/reports/purge` | `purge` | `{"reports":[ids]}` | `{"changed"}`, from the trash only, all or none |

A `HEAD` of a part is `405`. Ids are 32 lowercase hex digits. The filter's
parts are `statuses`, `kinds`,
`priorities` (`null` is none), `q`, `from`, `to`, `has_record`, `channel`,
`gateway`, `token`, `assignee`, `unassigned`, `label`, `reporter`,
`signature`, `duplicate_of` and `trash`; `central_api::report_api` is the
contract. A record a token files has `"origin":"client"`, at most 4 MiB, and
its `gateway` stays empty: a token speaks for none.

| status | means |
| --- | --- |
| `400` | the token is good, the request is not (an id, a body, a rule; a report that is not there); or the token was revoked while this was answered |
| `401` | the token is not good here: unknown, revoked, expired, its maker no administrator, or without the right; a gateway's refusals as ever |
| `409` | the report moved past `expected_revision`, or the part is stored already |
| `413` | a body over 64 KiB |
| `426` | another protocol |
| `429` | a budget (below), or a second request at once |
| `503` | the store is away |

| what | number | where |
| --- | --- | --- |
| live tokens, new ones a day | 32, 32 | `central-api`, `report_tokens` |
| requests of one token an hour besides filing, a selection one for every ten reports | 600 | `REQUESTS_PER_HOUR` |
| of them, downloads | 120 | `DOWNLOADS_PER_HOUR` |
| duplicates one request hands on to another original | 50 | `central-api`, `MAX_HANDED_ON` |
| calls at once, all tokens and one | 2, 1 | `central-http`, `slots.rs` |
| downloads at once, all tokens and one | 2, 1 | `slots.rs` |
| refused tokens an hour from one address | 30 | `addresses.rs`, `REPORT_REFUSALS` |
| reports an hour, uploads at once, stored bytes of one token | as a registered gateway's | above |
| a report's notes together, each with 256 bytes besides its text | 192 KiB | `central-api`, `MAX_NOTES_BYTES`, `NOTE_BYTES` |

All but the stored bytes, the duplicates and the notes are in the service's
memory: a restart forgets them. The four uploads at once and the 16 GiB of
all reports are tokens' and gateways' together. A request whose sender hangs
up keeps its call's slot until the store is done with it, five seconds at
most.

**A lost or leaked token** is revoked on the page, at once. Without an
administrator who can sign in, as `central_migrator`, by its id, since two
tokens may have one name (the page, a report's history and
`audit_events.report_token_id` name it):

```sql
SELECT id, name, created_by, created_at, last_used_at
FROM central_identity.report_tokens
WHERE revoked_at IS NULL AND expires_at > pg_catalog.clock_timestamp();
UPDATE central_identity.report_tokens
SET revoked_at = pg_catalog.clock_timestamp()
WHERE id = 'its id' AND revoked_at IS NULL;
```

**Removing an administrator** stops every token they made
([administration](administration.md) §"Report tokens").

**Deploying.** Migration 000052 adds `report_tokens`, `reports.filed_by`,
`report_notes.token_name`, `report_audit.token_name` and
`audit_events.report_token_id`, each with a default or null, and rewrites no
row. Its grants are in `ops/grants.sql`:
`SELECT, INSERT (name, digest, file, read, triage, purge, created_by,
created_at, expires_at), UPDATE (revoked_at, revoked_by, last_used_at)` on
`report_tokens`, and nothing new on the others. The binary before it runs
over all of it; it shows a token's report like a gateway's of that name, and
it knows no report token, so their requests are refused by it. No secret
and no setting is new.

An administration page that was opened before the deployment stays at its old
build. It reads no answer that a token's report, note or change is in, nor
an audit page with a token's event, and says the request failed: reload it. Tell the administrators before the first
token is handed out.

**Memory.** Two downloads at once of the largest part (64 MiB) can hold
about 384 MiB for a moment, by reading the code and not measured, beside
the uploads' 768 MiB above.

### The one-time copy of the feedback service's reports

After the gateways file reports with Central (Baylee `BAYLEE_REPORTS_TO=central`),
the reports still open at the feedback service are copied once, with their
notes and history (Baylee `docs/feedback.md` §"The one-time copy to
Central"):

```sh
{ cat /etc/baylee-central/migrate.json
  echo '{"author": "<an administrator's account id, 32 hex digits>"}'
  sudo -u baylee-feedback sh -c 'set -a; . /etc/baylee-feedback/env; exec baylee-feedback export-reports'
} | central-service --import-reports-stdin
```

The migration document first, then the administrator the copied notes are
filed under on a line of its own, then the service's lines; its database
address is read from its own environment file, never from a command line.
In one transaction the import checks every report by the intake's rules
(and its status open, its labels, its issue), files it under the official
gateway with its id, times, triage and parts as they were, each note under
the author named, its first author's name in its text, and each line of the
old history at its time, its actor named in its detail; a report whose id is
here already is skipped. A count or SHA-256 that is not the lines', a line
out of the rules, a copy cut short or anything after its last line stores
nothing. Standard error says how many were stored and how many were here
already; run it again and every report is skipped.


## Secrets

Run `umask 077; central-service --generate-secrets > secrets.json` once and merge
the values into the service configuration. Optional `push.vapid_key` enables
Web Push; preserve an existing OPAQUE setup, bridge and VAPID key. `token_keys` signs every
token; it may be rotated ([below](#rotating-the-signing-key)) and is required. See
[notifications](notifications.md) for build tools, migration/grants, network
boundaries and subscription renewal. The OPAQUE setup must never be
regenerated: replacing it locks every enrolled account out. Keep an offline
copy. The bridge credential is shared only with the official gateway's
`BAYLEE_CENTRAL_CONFIG_FILE`, and only when legacy migration is enabled.
The mailer's token is not generated here: it is the one the mailer holds
for Central ([mail](#mail)). Neither are the donation platforms' two: GitHub
Sponsors' token and Ko-fi's verification token come from the owner's
accounts there ([donations](#donations)).

The OPAQUE setup together with a copy of the credential table permits offline
password guessing. Treat any backup that holds both as credential material.

### Rotating the signing key

`token_keys` is `{"active": [32 bytes], "published": [[32 bytes], ...]}`: the seed of the key
that signs, and the public halves of the others the key set lists, at most eight keys in all. The
seed stays in the configuration; a public key can go anywhere. A key is rotated in three restarts,
so that nobody holds a token no key verifies:

1. **Publish.** `umask 077; central-service --generate-token-key > new-key.json`. Add its `public`
   to `published` and restart. Gateways read the key set every five minutes, and on a `kid` they do
   not know, so wait at least ten minutes.
2. **Sign.** Move `seed` to `active`, take its `public` out of `published` (a key listed twice
   stops the start) and put the old key's public half there, and restart. The old seed goes, and
   the old key stays listed.
3. **Retire.** At least 12 hours and 30 seconds later, remove the old public key and restart: a
   Central token lives to its session's cap, and a gateway token an hour.

**A key that may have leaked** cannot wait for this. Replace `active`, drop the old key from
`published` and restart. Central stops taking what it signed at once, which signs every native
client out. A gateway goes on taking its tokens until it next reads the key set, up to five
minutes later, and only then sends its players back to Central. A forged Central token needs a
session's capability as well; a forged gateway token needs nothing else, so treat a leak as urgent.

## Mail

*Since migration 000026; the protocol and every number are in
[an account's e-mail address](email.md).*

Central sends the code that confirms an account's address, a reset code,
a notice to an address that was replaced or removed, and notices to the
account's confirmed address that a reset was finished, that the password
was changed and that an administrator made a reset code. It speaks no
SMTP. It hands each mail to `baylee-mailer` on the same host, one
`POST /v1/send` over loopback HTTP. The mailer's `API.md` is the contract.

Without the `mail` block Central sends no mail. An address that is stored
can be read and removed, none can be named or confirmed, and a request for
a reset code by mail is refused for every name. Reset codes from an
administrator work as before.

```json
"mail": {
  "endpoint": "http://127.0.0.1:<the mailer's port>",
  "site": "https://baylee.app",
  "token": "…",
  "logo": false
}
```

- `endpoint`: where the mailer listens. `http://`, a numeric loopback
  address and a port, nothing after it. **There is no default, and the
  mailer's default port must not be assumed**: read the port from the
  mailer's own configuration on this host. A name (also `localhost`),
  another host, `https://` or a path is refused.
- `site`: the origin the mails' links start with (`{site}/{lang}/reset`,
  `{site}/{lang}/account/security`, and the start page `{site}/{lang}/` in
  every mail's footer, shown as the origin's host). The website's
  canonical HTTPS origin, without a path or a trailing slash.
- `token`: the bearer token the mailer holds for Central, 32 to 256
  characters. It is a secret like the bridge: it lives in the root-owned
  configuration file and in the mailer's own configuration, never in an
  argument, the environment, a unit file or the repository. Whoever holds
  it can send mail through the mailer in Central's name, within the
  mailer's limits. To rotate it, change it in both places and restart
  Central.
- `logo` (optional, `true` or `false`; absent is `false`): whether the
  mailer attaches Baylee's logo. With `true` every mail asks the mailer
  for it (`"inline": ["baylee-logo"]` in the request) and shows it in its
  band beside the wordmark (`cid:baylee-logo`, a part of the mail, nothing
  loaded from elsewhere); with `false` the band shows the letter's tile and
  the request has no such field. **Deploy the mailer that has the picture
  first** (its own configuration names the image and attaches it as
  `Content-ID: <baylee-logo>`), **then set `logo` to `true` and restart
  Central.** An older mailer refuses the unknown field with `400`: with
  the flag set too early, every mail is refused and Central logs `central:
  a mail was not handed to the mailer` for the notices and reset codes;
  set it back to `false` and restart. To turn it off later, the other way
  round: first the flag, then the mailer.

A service whose block is malformed prints `central: invalid configuration`
and does not start; the line names no value. The development launcher takes
the same block, with a numeric loopback HTTP origin as `site`.

**Database.** Migration 000026 adds two tables, and `ops/grants.sql` their
grants ([PostgreSQL](postgresql.md#an-accounts-e-mail-address-migration-000026)).
The deploy hook applies both. The daily sweep (`--sweep-stdin`,
[administration](#administration)) deletes addresses that waited for a code
which has ended, and prints how many.

**What Central logs.** At start, with the block: `central: mail through the
mailer on this host`. Later, for a reset code or a notice the mailer did
not take: `central: a mail was not handed to the mailer`. No line holds an
address, a name, a code or which mail it was. A confirmation mail that was
not taken is answered `Busy` to the person and is not logged.

**Limits.** Central's own are in [the numbers](email.md#the-numbers): for
one account 5 confirmation mails an hour and 10 a day; for one client
address 5 requests for a reset code an hour on each listener, and 30 an
hour for everyone together; for one account one mailed reset code in 10
minutes and 3 in 24 hours. Only the last is in the database. The others are
in memory, and a restart forgets them. The mailer has limits of its own for
each `limit_key` (Central sends `account:` and the account's id), for each
recipient and for each day. They are the mailer's configuration and are not
set here. The mailer's queue is in memory: a mail it took is lost when the
mailer restarts, and the person asks again. A try counts also when the
mailer did not take the mail. While the mailer is down, an account that
asked three times gets no mailed reset code for 24 hours; an
administrator's code is the way then.

**After a deploy that adds or changes the block**, check:

1. The unit is active and the journal has `central: mail through the mailer
   on this host`.
2. The port in `endpoint` is the one the mailer listens on (`ss -ltn`, the
   mailer's unit), on loopback only.
3. The mailer accepts Central's token. A wrong token or port shows as
   `Busy` to a person who names an address, and as `a mail was not handed
   to the mailer` in the journal for a reset code.
4. With a test account of the operator's own: name an address, confirm it
   with the mailed code, ask for a reset code by mail and see that mail
   arrive. Until the website has its forms, `central-client` asks these
   requests ([the SDK](email.md#the-sdk)).
5. The links in the two mails open the site's reset page and the account's
   security page.

**The legal texts.** An address is personal data that Central did not store
before. The privacy notice (`legal/privacy.*.md` §§ 3 and 6) names it as
"not available yet", speaks of reset links where Central mails codes, and
knows a reset code's digest only as an administrator's. Bringing those
lines up to date is the owner's review. This runbook does not decide
whether the block may be enabled before that.

## Donations

*Since migration 000032 (18.11.2026); the owner's order: the three
platforms added up in euros, with a bar and a sum. Not deployed yet.*

Central collects what the three donation platforms say and gives the
website totals only (`PublicFunding`, on both listeners, under the
directory's budget): each platform's monthly figure in euro cents with its
supporters, and the sum of what was given once with its count and the day
the count began. No row, no transaction id and no time of a single
donation leaves Central. The bar counts the recurring giving of the three
platforms against the monthly goal; what was given once is a figure of its
own, never part of the bar.

| platform | how Central learns it | what it gives |
| --- | --- | --- |
| Liberapay | `GET https://liberapay.com/AceVik/public.json` every 30 minutes, no key | the week's receipts as a month's (× 52/12) and the patrons' count |
| GitHub Sponsors | GraphQL at `https://api.github.com/graphql` every 30 minutes, with the sponsored account's token | the monthly estimate (US cents, converted at the latest rate each time it is read), the active sponsorships, and the one-time sponsorships and their refunds of the last 30 days (stored once each) |
| Ko-fi | Ko-fi's webhook delivers each payment to `https://central.baylee.app/v1/kofi` | a one-time payment counts to the sum; a membership's payments of the last 31 days make Ko-fi's monthly figure, each payment one supporter |

The euro reference rates come from the European Central Bank's daily feed
(`https://www.ecb.europa.eu/stats/eurofxref/eurofxref-daily.xml`), fetched
every six hours after a fetch that worked and 30 minutes after one that
failed, when GitHub or
Ko-fi is on. An amount in a currency without a rate is stored without a
euro value and **not counted**; it is converted once a rate comes, if that
is within seven days; the log names its currency's code, never an amount.
Nothing is guessed.

### The `funding` block

```json
"funding": {
  "liberapay": true,
  "github": {"token": "…", "on": true},
  "kofi": {"token": "…", "record": false, "on": true}
}
```

- Without the block Central reads Liberapay and nothing else; GitHub's and
  Ko-fi's figures and the one-time sum are **absent**, never zero, and the
  page names the two platforms as not counted yet.
- `liberapay` (optional, `true` when absent): `false` stops asking Liberapay.
- `github.token`: a token of the sponsored account (20 to 255 visible
  characters). **Which scope GitHub needs for these fields is not
  documented**: the owner tests a classic token with `read:user` and notes
  here what was needed. A token without the scope gets an answer with
  errors, which Central treats as no answer: the log says `central: GitHub
  Sponsors' figures did not come …` once an hour and the figure stays
  absent. The query asks for no sponsor's name or login
  (`crates/central-funding/src/github.rs`).
- `kofi.token`: the verification token Ko-fi shows at
  `https://ko-fi.com/manage/webhooks` under *Advanced* (8 to 128 visible
  characters). Central keeps only its SHA-256 digest after the start.
- `kofi.record` (optional, `false` when absent): whether a verified payment
  is stored and counted. With `false` every delivery is verified and read
  and nothing is kept, and Ko-fi's figure is not shown: the state for
  Ko-fi's test delivery (below).
- `on` (optional, `true` when absent) in either block: `false` keeps the
  token in the file and switches the platform off. Switched off, GitHub is
  not asked, and `/v1/kofi` answers `404` as if it did not exist.
- A token of another shape, or an unknown key, stops the start
  (`central: invalid configuration`, naming no value); the check mode
  (`--check-accounts-stdin`) refuses it too.

**Where the tokens go.** Into `/etc/baylee-central/central.json` beside the
other secrets (root's, `0600`, opened by systemd as standard input), and
nowhere else: not in the repository, an argument, the environment, a unit
file, a URL or a chat. To change one: edit the file and restart the service.
To withdraw one: delete it at the platform, then remove it here.

**Outbound HTTPS** (port 443) from Central's host must be allowed to
`liberapay.com`, `api.github.com` and `www.ecb.europa.eu`. Central reaches
them through `central-outbound`: public addresses only, no proxy, no
redirect, 15 seconds a request, bounded bodies (64 KiB, 256 KiB, 64 KiB).
A poll that fails leaves the last figure standing until the next restart.

### Ko-fi's webhook

- **Address for Ko-fi's settings** (`https://ko-fi.com/manage/webhooks`,
  *Webhook URL*): `https://central.baylee.app/v1/kofi`. No secret is in the
  address; Ko-fi sends the token in the delivery. The `central.baylee.app`
  site block passes every path to the account listener already; Caddy needs
  no change.
- What the route takes: `POST` with `Content-Type:
  application/x-www-form-urlencoded`, at most 16 KiB (a longer one, up to
  64 KiB, is read, refused and counted as `size`; one past 64 KiB is
  refused with `413` before it is read and is in no count), one field
  `data` holding JSON.
  Central reads the token first and compares it in constant time; only with
  the right one does it read `type`, `amount`, `currency`,
  `is_subscription_payment`, `is_first_subscription_payment` and
  `kofi_transaction_id`. The giver's name, e-mail address, message,
  shipping data and links are never parsed into anything, stored or logged.
  `Donation` counts once (or as a membership's payment when
  `is_subscription_payment` says so), `Subscription` as a membership's
  payment; `Commission` and `Shop Order` are sales, verified and not
  counted. Every refusal (wrong or missing token, malformed body, unknown
  shape, hostile amount or currency) is the same plain `400`; a delivery
  taken, also one that comes again, is `200`; 30 deliveries a minute from
  one address, then `429`.
- **The format is not confirmed.** It was taken from descriptions of
  Ko-fi's webhook, not from Ko-fi's own documentation, which could not be
  read automatically. The first test delivery decides:
  1. Deploy with `"kofi": {"token": "…", "record": false}` and restart.
     The start line says `central: donations: …, Ko-fi (verifying only,
     kofi.record is false)`.
  2. In Ko-fi's webhook settings enter the address above and press *Send
     Test*.
  3. Within a minute the journal says either `central: 1 Ko-fi
     delivery(ies) verified and read, and not recorded (kofi.record is
     false)`: the format fits; or `central: Ko-fi deliveries refused since
     start, by reason: …`: it does not, and the reason (`content type`,
     `form`, `json`, `token`, `shape`, `amount`, `currency`, `size`) says
     where. Neither line within a minute means the delivery never reached
     the service: a body past 64 KiB (`413`), or the `403` below.
     `token` alone means the token in the file is not the one Ko-fi shows.
     A delivery with an `Origin` or `Sec-Fetch-Site` header would be
     refused with `403` before it is read (the account listener's rule)
     and appear in neither line; then report it, do not loosen the rule by
     hand.
  4. When the format fits, set `"record": true` and restart. Ko-fi's test
     delivery is never counted that way, since it arrived while recording
     was off. A test sent while recording is on is stored like a donation
     (its id is Ko-fi's, and a second test with the same id counts once);
     only the migrator's login can take it out again, by its platform and
     the minute it came, before the sweep folds it into the sum.
- **Cancellations.** Whether Ko-fi reports a membership that ends is not
  known. Ko-fi's monthly figure therefore counts the membership payments of
  the last 31 days, each payment one supporter: a supporter who stops drops
  out at most 31 days after their last payment; one whose payments are
  fewer than 31 days apart (February) counts twice for up to three days.

### What is stored, and for how long

Migration 000032 adds `central_identity.donations`: platform, kind (given
once, a membership's payment, a refund), the amount in hundredths of its
currency, the currency, the amount in euro cents at the rate held when it
came (or none), when it came by Central's clock, and a SHA-256 digest of
the platform's transaction id, unique per platform, so that a delivery
that comes twice is kept once. Nothing about the giver. Rows are kept 40
days, longer than Ko-fi's window (31 days) and the window of GitHub's
activities that is read again (30 days), so that a swept row never comes
back as a new one. The daily sweep (`--sweep-stdin`) then adds what was
given once and what went back to `central_identity.donation_sums` (one
sum and one count per platform and kind) and deletes the rows; a
membership's payment past its window and a row that never got a rate are
deleted without a trace. `central_identity.donation_count` holds the day
the count began (the day the migration ran). The runtime may add a row,
set a row's euro value once, and read; it cannot change an amount, delete
a row or write a sum (`ops/grants.sql`, checked at every start).

### What Central logs

At the start: `central: donations: Liberapay, GitHub Sponsors, Ko-fi` (the
sources that are on). Then, each at most once an hour: that Liberapay's,
GitHub's figures or the ECB's rates did not come, that the store did not
answer, the currencies of donations waiting for a rate with their number of
rows, and Ko-fi's refusals by reason since the start. At once, when they
change: the number of Ko-fi payments recorded, verified while `record` is
off, or verified as sales. No line holds a token, an amount, a name, an id
or a body. The sweep prints how many donations it folded and deleted.

**The legal texts.** The website's privacy page says, in its part on
donations, what Central collects and keeps of a payment and for how long
(German and English; the nine other languages carry the English until a
translation follows). The binding privacy policy (`legal/privacy.*.md`)
says the same in § 3 and § 7 since its version 2026-11-18 (the owner's
decision of 18.11.2026; [legal texts](../legal/README.md)); like every
version it awaits the owner's full review before it is published.

**Figures one can diff.** The public sum changes when a donation is
counted, at most a minute later, so whoever watches `/funding.json` can
tell the amount of a single donation that came alone. It names nobody and
no time finer than the minute the site read it; hiding it would need the
figures to be published in steps, which is the owner's call.

## Supervision

Keep the configuration root-owned `0600` and let the supervisor open it, so the
service identity never reads the file itself:

```ini
[Service]
User=central
ExecStart=/opt/baylee-central/bin/central-service --accounts-stdin
StandardInput=file:/etc/baylee-central/central.json
Restart=on-failure
NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=yes
PrivateTmp=yes
PrivateDevices=yes
CapabilityBoundingSet=
RestrictAddressFamilies=AF_INET AF_INET6 AF_UNIX
MemoryDenyWriteExecute=yes
SystemCallFilter=@system-service
```

### Logs and how long they stay

The privacy notice (`legal/privacy.*.md` § 7 and the site's "Hosting")
promises: no access log, and server logs deleted after 14 days at the
latest. Three settings on the host carry that promise; none is in this
repository, so check them after every change to the host:

- Caddy has no `log` directive for these sites (the default writes no
  access log).
- journald is capped, `/etc/systemd/journald.conf.d/baylee.conf`:

  ```ini
  [Journal]
  MaxRetentionSec=14day
  ```

- Where journald forwards to rsyslog (`ForwardToSyslog=yes`, Ubuntu's
  default), the same messages are written again to `/var/log/syslog`, and
  the distribution's `/etc/logrotate.d/rsyslog` keeps four weekly files:
  about five weeks. Cap it to match, in that file:

  ```
  daily
  rotate 13
  maxage 14
  ```

  (in place of `weekly` and `rotate 4`; the same for
  `/etc/logrotate.d/fail2ban` if it is installed, whose log holds the
  addresses it banned). Turning forwarding off instead also works, but
  check first that nothing reads `/var/log/auth.log` (an older fail2ban
  does).

Read on 11 November 2026 (read-only, by the owner's leave): the live host
has the journald cap and no access log, and still the weekly `rotate 4` for
syslog and fail2ban. Until that is changed, "14 days" is not true of
`/var/log/syslog*`.

## The server-rendered website (`central-web`)

*Added 10.11.2026, not deployed* ([proposal](proposals/frontend-rewrite.md)).
Until the owner switches Caddy over, `baylee.app` keeps serving the static
build of `web/` already on the server; nothing here changes production by
being merged. Since 12.11.2026 that React build is gone from this
repository, and the deploy hook builds and installs this site instead
([below](#deploying-with-the-gateway)).

- **Once, by the owner** (in this order): write `/etc/baylee-app/web.json`
  (below; root's, `0600`), install `ops/baylee-app-web.service` as
  `/etc/systemd/system/baylee-app-web.service`, run the hook (or put the
  first build in place by hand), `systemctl daemon-reload`, `systemctl
  enable --now baylee-app-web`, check `curl -fsS -H 'Host: baylee.app'
  http://127.0.0.1:28794/robots.txt`, then switch Caddy (below). The unit
  runs the binary as a dynamic user with a strict sandbox
  (`ProtectSystem=strict`, no capabilities, `NoExecPaths=/` but for the
  binary and the system's libraries, `@system-service` system calls); it was
  written without a systemd at hand and is checked by that first start.

- **Build** (as the build user, beside `central-service`):
  `cargo build --release --locked -p central-web-server` and, with
  `BAYLEE_SRC=/opt/baylee/src` for the screenshots and the fallback card
  status, `cargo xtask site` ([xtask](xtask.md#the-sites-assets-site));
  it reads the locked graph offline, so on a machine that has not built
  the whole workspace `cargo fetch --locked` comes first.
  Stage `target/release/central-web` and `target/site/`.
- **Run** as an unprivileged user of its own (no database, no secret, no
  write access anywhere but the optional file for the count of page views,
  below), `central-web --config-stdin` with
  `StandardInput=file:/etc/baylee-app/web.json`:

  ```json
  {"listen": "127.0.0.1:28794", "origin": "https://baylee.app",
   "site": "/opt/baylee-app/site", "central": "127.0.0.1:28791",
   "github": true, "fallback": "/opt/baylee-app/site/fallback"}
  ```

  `verification` may carry a search console's verification value (never
  committed). The process asks `api.github.com` and
  `raw.githubusercontent.com` (Baylee's repository only, every ten minutes)
  and Central's browser listener on loopback (every minute): outbound HTTPS
  to GitHub must be allowed. It logs nothing about requests.

  The donation page's bar and `/funding.json` show what Baylee Central
  says of the donations (`PublicFunding`, asked with the directory every
  minute; [donations](#donations)): each platform's monthly figure and
  the sum of what was given once, all in euros. This process asks no
  donation platform any more and needs no outbound HTTPS for them. A
  Central from before the figures, or one that does not answer, leaves
  the figures the site holds standing (at the start the fallback's
  `funding.json`, or none); with none the page shows the goal without a
  bar. **The `funding` setting is gone (18.11.2026)**: the file is read
  with unknown fields refused, so a `web.json` that still says
  `"funding": …` stops the start (`central-web: the settings are malformed`).
  Take the key out of `/etc/baylee-app/web.json` before the first deploy
  of this build.

  `views` may name a file, for example `/var/lib/baylee-app/views.json`
  in a directory this user owns (`StateDirectory=baylee-app`, or
  `ReadWritePaths=` under `ProtectSystem=strict`): the site then counts
  how often a browser opens one of its pages and shows the number in the
  start page's strip, and the privacy notice gains its section on it. The
  file is the one thing the process ever writes: the number and the day
  the count began, rewritten at most once a minute by a rename beside it,
  and once more at shutdown. Nothing about a request is kept, no address,
  no cookie, not even which page it was
  (`crates/central-web-server/src/views.rs`). Without `views` nothing is
  counted, nothing is shown and the process writes nowhere, as before. A
  file that is malformed, too large or cannot be written stops the start
  with a line that says so; it is never overwritten.
- **Caddy**: `/api/*` stays as it is (Central's browser listener);
  `/site/impressum*` and, new, `/site/contact` go to the feedback service
  (which needs `https://baylee.app` in `FEEDBACK_CONTACT_ORIGINS`);
  everything else to `central-web`, with `encode zstd gzip`. The `Cookie`
  header is removed on both ways that are not Central's: the session
  cookie is `__Host-central` with `Path=/`, so the browser sends it with
  every request to the site, and neither `central-web` nor the feedback
  service has any use for it. `/api/*` keeps the body bound it has
  [above](#the-browser-listener):

  ```caddyfile
  baylee.app {
  	@feedback path /site/impressum /site/impressum/* /site/contact
  	handle @feedback {
  		request_body {
  			max_size 32KiB
  		}
  		reverse_proxy <feedback service> {
  			header_up -Cookie
  		}
  	}
  	@picture_upload path /api/v1/picture /api/v1/groups/*/picture
  	handle @picture_upload {
  		request_body {
  			max_size 9MiB
  		}
  		reverse_proxy 127.0.0.1:28791
  	}
  	handle /api/* {
  		request_body {
  			max_size 64KiB
  		}
  		reverse_proxy 127.0.0.1:28791
  	}
  	handle {
  		reverse_proxy 127.0.0.1:28794 {
  			header_up -Cookie
  		}
  	}
  	encode zstd gzip
  }
  ```

  The security headers (CSP, Permissions-Policy, COOP, CORP, HSTS) come from
  `central-web` itself, on every answer it gives; Caddy adds none to those.
  HSTS is sent without `includeSubDomains`: whether every name under
  `baylee.app` is served over HTTPS for good is the owner's to say.
- **The BIMI mark**: `central-web` answers `/bimi/baylee.svg` itself (the
  catch-all handle above passes it), `image/svg+xml`, 200, the approved
  bytes, revalidated by its entity tag; every page's footer shows the same
  file. The DNS record `default._bimi.baylee.app` (`v=BIMI1;
  l=https://baylee.app/bimi/baylee.svg;`) belongs only after this site
  is live: the old `web/` build has no such file, and its `file_server`
  answers an unknown path with the app's HTML. DMARC must stay at
  `quarantine` or `reject`. A mark certificate (Gmail's CMC) asks that the
  mark was public on the domain for a year, from the day it first shows.
- **The administration is `central-web`'s** since 12.11.2026, under the
  account area: `/{lang}/account/admin/overview`, `/users` (and one
  account's `/users/<id in hex>`), `/gateways`, `/audit`, `/legacy` and
  the reset code's `/{lang}/account/admin` ([frontend
  rewrite](proposals/frontend-rewrite.md) §25). It needs no Caddy handle
  of its own: the catch-all passes it, with `central-web`'s headers. The
  feedback's triage is `/{lang}/account/admin/feedback`, and the old
  build's `/admin…` addresses redirect to these pages (`central-web`,
  302), so no `handle /admin*` for the old build is needed.

## The session tokens release

Migration 000024 and the token model go out together with a gateway and a client that speak them;
older ones are refused (`426`), and no compatibility path exists. The same deploy applies the
website release's migrations 000023, 000025 and 000026 ([registration](#registration),
[what gateways report](#what-gateways-report-migration-000025), [mail](#mail)) and the
profile pictures' 000027 ([below](#profile-pictures)), whose own conditions are in those
sections. The owner's order, as in Baylee
`docs/central-login.md` §"Deploying this":

1. **`central.json`, before step 3.** Generate `token_keys` (`--generate-secrets` holds one, or
   `--generate-token-key`'s `seed`) and add it, with `official_address`
   ([the official gateway](#the-official-gateway)). `token_keys` is required: this revision refuses
   to start without it. `official_address` is optional, and without it the official gateway hears
   of revocations by its public name or by catch-up.
2. **The client release.** Publish it first, so that the updater finds it. Installed clients go on
   working until step 3. A client already updated cannot sign in until then: the old Central answers
   a bearer it cannot read. Keep this gap short.
3. **Central and the gateway, together.** A gateway deploy installs the Central revision it links
   ([below](#deploying-with-the-gateway)), so one deploy does both. The hook first asks whether the
   grants' login reaches the database and whether the new build accepts the configuration
   (`--check-accounts-stdin`), and refuses the deploy while Central still runs. It then stops the
   service, runs `--migrate-stdin` and `ops/grants.sql`, and starts it; started before the grants,
   the service refuses to run. The migration drops `gateway_tickets` and `gateway_leases` and ends
   every native session, so every client signs in again; browser sessions stay. Do not get past the
   check by hand: after the migration the new binary would not start, and neither would the previous
   one, whose startup check needs the tables the migration drops.
4. **Clients update themselves.** The updater goes to GitHub Releases and needs no sign-in.

A client from before this release drops a refusal's body, so after step 3 it shows its own generic
sign-in failure until the updater has replaced it.

### Before the deploy: a dump

Two of this release's migrations cannot be undone, and there are no down migrations:
- 000023 drops `beta_keys` and the accounts' link to a key;
- 000024 drops `gateway_tickets` and `gateway_leases` and deletes every native session.

After 000024 the previous binary does not start (its startup check needs the tables 000024 drops).
000025 to 000027 only add. So the way back from this deploy is a restore. Right before step 3,
take a dump as the database's owner, the way the hook reaches it, into a file only root can read:

```sh
umask 077
docker exec baylee-postgres-1 pg_dump -w -U central_migrator -Fc baylee_central \
  > /root/baylee_central-before-000023.dump
docker exec -i baylee-postgres-1 pg_restore -l < /root/baylee_central-before-000023.dump | head
```

The dump holds the credential table. Together with the OPAQUE setup that is credential material
([secrets](#secrets)): keep it no longer than the deploy needs it.

A failure after the migration has started makes the hook start the previous binary and say that
the way back is this dump. **The way back:**
1. Stop Central.
2. Restore the dump into an empty `baylee_central` (`pg_restore --clean --if-exists`, as the
   owner).
3. Put `central-service.previous` back as `central-service`, and the previous `ops/grants.sql`.
4. Start it.

Nothing of this was rehearsed outside production; rehearse it on a copy first if there is time.

## Profile pictures

What is built is in [profile pictures](profile-pictures.md); the owner's
decisions of 12.11.2026 are in the
[proposal](dependency-proposal/profile-picture.md) (§0 and §0.8). Not
deployed yet. A fresh `central-picture-worker` converts each upload in its
own sandboxed unit; for an animation it starts ffmpeg inside that unit.

**Once, before the first deploy with pictures** (the owner's steps; read
on the server on 12.11.2026, where neither package was installed):
1. **nasm** for the build: `apt-get install nasm` (3.01-1 on Ubuntu
   26.04). The deploy hook builds the worker, whose `rav1e` and `rav1d`
   assembly needs it on x86-64; without it the build fails.
2. **ffmpeg** for animations: `apt-get install --no-install-recommends
   ffmpeg` (`7:8.0.1-3ubuntu2`, component universe; `apt-get -s` listed 90
   new packages). It links `libaom-av1` (`libaom3` 3.13.1, installed
   already). Without it every animation is kept as its still.
3. **Ubuntu Pro** (owner, 12.11.2026): attach the server (`pro attach`),
   so that universe, ffmpeg and its libraries among it, gets security
   updates (`esm-apps`); `pro status` must list `esm-apps` as enabled.
   They then come with the server's other updates.
4. **Caddy** must let an upload through: the `/api/*` block bounds bodies
   to 64 KiB, so every picture larger than that would get 413. Add the
   `handle /api/v1/picture` block with `max_size 9MiB` before it, as both
   snippets above show (the route itself refuses more than 8 MiB, and
   1 MiB for SVG and ICO, from `Content-Length` before reading).
5. **The worker's units:** copy `ops/central-picture.socket` and
   `ops/central-picture@.service` to `/etc/systemd/system/`, run
   `systemctl daemon-reload` and `systemctl enable --now
   central-picture.socket`, and give the service
   `"pictures": {"socket": "/run/baylee-central/picture.sock"}`. Without
   it uploads answer `Unavailable` and stored pictures are still served.

**After every deploy, and after every ffmpeg update:**
- `/opt/baylee-central/bin/central-picture-worker --ffmpeg /usr/bin/ffmpeg
  --probe` prints `animations: ffmpeg with libaom-av1 found` and exits 0.
  It names the encoder it found: a line that names `libsvtav1` (ffmpeg's
  other AV1 encoder, which the worker falls back to) means `libaom-av1` is
  gone, and in this unit `libsvtav1` is killed by the system-call filter
  ([profile pictures](profile-pictures.md#animations)), so animations
  would be kept as stills: restore `libaom-av1` rather than the unit.
- With a test account, upload an animated GIF on the profile page. It
  must play there, and `curl -sI
  https://baylee.app/api/v1/pictures/<its key>.sequence.avif` must answer
  `200` with `image/avif`; then remove the picture. This is the only check
  of ffmpeg inside the unit's sandbox (its system-call filter,
  `MemoryDenyWriteExecute`, 32 tasks, 512 MiB, 25 s), which could not be
  tested before deploy.
- The service's journal must hold no line `central: an animation was
  kept as its still (…)`. `no_encoder` means ffmpeg or an AV1 encoder
  for the animation (`libaom-av1`; for an opaque one also `libsvtav1`) is
  missing or did not answer in time; `failed` that ffmpeg failed or was
  ended: look at `journalctl -u 'central-picture@*'` for a kill by the
  system-call filter (`SIGSYS`) or by the memory limit. Pictures still
  work then, as stills; change the unit only with a review.

## Deploying with the gateway

The 2 November 2026 preparation sets the service and website to
`0.1.0-beta.9c`, the exact version confirmed by the owner. Shared library
versions remain `0.1.0`; authentication, chat wire formats and persisted
credentials remain compatible. This is lower in SemVer than the preceding
`0.1.1-beta.9b`; native updater downgrade protection is unchanged. Preparation
and merging do not authorize a production migration or deployment. The
dependency review is in [beta9c.md](dependency-proposal/beta9c.md).

For the 30 October 2026 release, the service executable and website package
are versioned `0.1.1-beta.9b`. Reusable library versions and the authentication
protocol are unchanged, and the wire changes since `0.1.1-beta.9a` are only
additions, so the `0.1.1-beta.9b` gateway, which still links `0.1.1-beta.9a`
(3ae5035), works beside it. The release is main at c531e9c (#42 to #50: password
change, administration, login reset, deletion and export, the website's design
guidelines, account area, gateway tokens and administration) plus this version
commit on `codex/release-0.1.1-beta.9b`, deployed by hand (`10-central manual`);
#51 (the official gateway's new origin) waits for the gateway that accepts it.
The 28 October release (`0.1.1-beta.9a`) was the sunlit Baylee memorial,
parchment story and photo viewer.


`ops/deploy-hook` keeps Central and the website (the server-rendered
`central-web`; the React build of `web/` was removed on 12.11.2026) in
lockstep with the official gateway on the same machine. It is a Baylee deploy hook (Baylee's
`docs/deploy-hooks.md`): installed root-owned, `0755`, as
`/etc/baylee/deploy-hooks.d/10-central`, it runs in every gateway deploy,
including the one the gateway's timer starts for a new release tag.

- **Which revision.** The one the gateway being deployed links
  (`central-api`'s `rev` in its `Cargo.toml`), so the service speaks that
  gateway's protocol. Central never goes back: when the installed revision
  (`/opt/baylee-central/state/deployed`) already contains it, the hook does
  nothing, so a newer Central deployed by hand stays.
- **`prepare`.** As the build user it fetches and checks the revision out in
  `/opt/baylee-central/src`. It then takes `ops/grants.sql` out of the
  revision's commit, before anything is built, and only then builds
  `central-service`, `central-web-server` and `central-picture-codec` in
  release, each binary named (`--bin central-service --bin central-web
  --bin central-picture-worker`; on x86-64 the codecs need `nasm`,
  [profile pictures](#profile-pictures)), and the site
  (`cargo xtask site` with `BAYLEE_SRC`, for the gateway's screenshots and
  fallback card status). It stages the three binaries, the grants and the
  site in `/opt/baylee-central/staged`. The build user needs Rust's
  `wasm32-unknown-unknown` target and the pinned wasm-bindgen CLI
  ([xtask](xtask.md#the-sites-assets-site)); `brotli` and `gzip` on the
  `PATH` make the compressed copies. A failed fetch answers "not ready" (75), so
  the next timer run tries again.
- **`before-switch`** (no game is running, admission is held): asks first
  whether the grants' login reaches the database and is no superuser, and
  whether the staged service accepts `/etc/baylee-central/central.json`
  (`--check-accounts-stdin`, as `central`), and stops with nothing changed
  when not. Then it stops the unit, installs the
  binary (the old one kept as `central-service.previous`) and the picture
  worker (systemd starts a fresh one for each picture), migrates
  (`--migrate-stdin` from `/etc/baylee-central/migrate.json` under
  `systemd-run` as `central`), grants as `central_migrator`, starts it and
  waits up to 30 s for `/health/live`. Then the website: the site goes to
  `SITE_ROOT` (`/opt/baylee-app/site`) beside the old one and the binary to
  `WEB_BIN` (`/opt/baylee-app/bin/central-web`, the old one kept as
  `central-web.previous`); where `WEB_UNIT` (`baylee-app-web`) is installed
  it is restarted and has 30 s to answer `robots.txt` on `WEB_LISTEN`, else
  the previous site and binary are put back and the hook fails (Central
  stays deployed). Where the unit is not installed yet, the files are put in
  place and nothing is started. A failed migration, grant or start of
  Central puts the previous binary back, starts it and fails the hook, which
  stops the deploy (migrations are transactional).
- **By hand,** for a Central or website change between gateway releases:
  `sudo /etc/baylee/deploy-hooks.d/10-central manual <central revision>`
  stages and switches that revision at once; nothing holds games for it.

Settings, all optional, in `/etc/baylee-central/deploy-hook.env` (root's,
not writable by others): `BUILD_USER`, `BAYLEE_SRC`, `CENTRAL_ROOT`,
`WEB_BIN`, `SITE_ROOT`, `WEB_UNIT`, `WEB_LISTEN`, `UNIT`, `LISTEN`,
`DB_CONTAINER`, `DB_GRANTOR`, `DB_NAME`; the defaults are this
deployment's. The website's Caddy block is the one in §"The server-rendered
website". The old static `WEB_ROOT` (`/opt/baylee-app/web`) is no longer
written; remove it once Caddy no longer serves it.

### The grants' file and login (12.11.2026)

Until this date root copied `ops/grants.sql` out of the tree a build had
just run in, and gave it to `psql` as the database's superuser (`baylee`,
the setting `DB_SUPERUSER`). A dependency's build script or an install
script of `npm ci` runs as the build user and can write that tree. A
rewritten file would have run with every right the database has, and a line
beginning with a backslash is a command to `psql` itself, which can start a
program.

Since then:

- **The file comes from the commit.** After the checkout and before the
  build, `git cat-file blob <revision>:ops/grants.sql` prints the file as
  the commit stores it (run as the build user, with
  `--no-replace-objects`; no filter and no conversion applies to it). The
  copy is root's from then on. The tree, the index, a filter and a
  replacement object do not change what is printed.
- **SQL only.** A file that is empty or holds a backslash is refused at
  `prepare` and again before the switch. `ops/grants.sql` needs none; keep
  it so, in comments too. The store's fixture refuses a backslash as well
  (`central-postgres`, `tests/support/accounts.rs`), so it shows before a
  deploy does.
- **The migrator's login, never a superuser.** `psql` runs as `DB_GRANTOR`
  (default `central_migrator`, the owner of the tables) with `-X` (no
  start-up file) and `-w` (never asks for a password). Before the unit is
  stopped the hook asks the database whether that login is a superuser. If
  it is, or the login does not reach the database, the hook ends there and
  the running service stays as it was. The file can then do what the
  migrations before it can do, and no more: it cannot make a role or a
  superuser, read or write the server's files, start a program, or change
  the server's settings.

**Before the first deploy with this hook** (once, on the server):

1. Install the new hook as `/etc/baylee/deploy-hooks.d/10-central`
   (root's, `0755`), as before.
2. If `/etc/baylee-central/deploy-hook.env` sets `DB_SUPERUSER`, remove the
   line. Nothing reads it any more. Do not set `DB_GRANTOR` to a superuser:
   the hook refuses it.
3. Remove what the old hook may have staged, so that its copy of the
   grants is not the one switched: `rm -f /opt/baylee-central/state/staged`.
   The next `prepare` stages the revision again.
4. Check that the migrator reaches the database over the container's
   socket without a password:

   ```sh
   docker exec -i baylee-postgres-1 psql -X -w -At -U central_migrator -d baylee_central \
     -c 'SELECT current_user, rolsuper FROM pg_catalog.pg_roles WHERE rolname = current_user' < /dev/null
   ```

   It must print `central_migrator|f`. The old hook reached the database
   the same way as `baylee`, so a `pg_hba.conf` that still has the image's
   `local all all trust` line answers this. If it asks for a password
   instead (the command fails, `-w`), the `local` lines of `pg_hba.conf`
   name who may use the socket: add `central_migrator` for
   `baylee_central` there with the method the superuser's line has, and
   reload. Whoever may run `docker exec` on that container is root on the
   host already.
5. Apply the grants of the revision that is deployed now, by hand, as the
   migrator. Step 4 shows that the login reaches the database. It does not
   show that the migrator may grant on everything the file names: until
   now the superuser applied the file, so this was never asked on the
   server. The hook applies the grants after the unit is stopped, and a
   failure there puts the old service back after a stop. By hand it shows
   beforehand, while the service runs. As root, with the build user's name
   for `BUILD_USER`:

   ```sh
   rev=$(cat /opt/baylee-central/state/deployed); echo "revision: $rev"
   f=$(mktemp)
   runuser -u BUILD_USER -- git --no-replace-objects -C /opt/baylee-central/src \
     cat-file blob "$rev^{commit}:ops/grants.sql" > "$f"; echo "git: $?"
   grep -c '' "$f"; grep -c '\\' "$f"
   docker exec -i baylee-postgres-1 psql -X -w -q -v ON_ERROR_STOP=1 \
     -U central_migrator -d baylee_central < "$f"; echo "psql: $?"
   rm -f "$f"
   ```

   Go on from one line to the next only if it says what follows.
   - `revision:` names a commit, forty characters. If `state/deployed` is
     missing (a Central deployed by hand), set `rev` to the revision the
     running service was built from. With an empty `rev` git ends with an
     error; without the `^{commit}` it would print the file of the index
     instead, which a build can write (tried).
   - `git: 0`, then the file's lines (more than none) and `0` for the
     lines with a backslash. If the second number is more, do not go on
     to `psql`, which would take such a line for a command of its own.
   - `psql` prints nothing at all, and then `psql: 0`. **Any line from
     `psql` is a failure, a warning too.** A grant on a table that is not
     the login's ends with an error (`permission denied`, status 3) when
     the login holds nothing on it. When the login holds the privilege
     but may not pass it on, `psql` only warns (`no privileges were
     granted`), grants nothing and still ends with 0 (both tried on
     PostgreSQL 18.6 with the hook's own call). The messages come in the
     server's language, so go by whether anything is printed.

   The file only grants (read it first, it is short), and the deployed
   service has these grants already, so applying it again changes
   nothing. If `psql` printed anything, a table or the schema on the
   server is not the migrator's: deploy nothing with this hook until its
   owner is `central_migrator` (the migrations make them as that login, so
   this is not expected). This step was not run on the server.

   The hook itself goes by `psql`'s status, so it does not see a grant
   that only warned. It would start the unit; the store checks at start
   that the runtime login holds each privilege it needs and refuses to
   run otherwise (the fixture tests that); the hook's probe would then
   fail after its thirty seconds and the previous binary would be started
   again, with the migration already applied.

**What stays possible.** The binary and the website are still what the
build made, and the binary runs the migrations with the migrator's login:
a build that was tampered with needs no grants file. The build user's git
(its settings, its object store, a `git` earlier in its path) is as
writable by an earlier build as the tree is, so "the file of the commit"
is as good as that user's git. `npm ci` still runs install scripts, and
the build user is a person's login with a home and keys. Building as a
user without either, and `npm ci --ignore-scripts`, are not done.

**Tested here, and not.** `shellcheck` and `bash -n` pass. A local dry run
used the hook's own three functions against a throwaway PostgreSQL 18.6
container without a network and a scratch clone: the commit's file came
out byte for byte with the tree, the index, a filter and a replacement
object changed; the migrator applied a stand-in grants file to tables it
had made and was refused seven things a superuser may do; a superuser, a
missing login, a missing database and a missing container were refused
before anything else ran. Both PostgreSQL fixtures apply the real
`ops/grants.sql` as `central_migrator` after the migrations. **Not
tested:** the hook as a whole (it needs root, systemd, the build user and
the server's container), the server's `pg_hba.conf`, and a deploy with it.

## Sessions

A sign-in issues a session for 30 minutes. A live session renews
(`RenewSession`, on both listeners) to 30 minutes from the renewal, but never
past 12 hours after the password sign-in that issued it (`created_at`; migration 000008). A
renewal changes nothing in the bearer, whose `exp` is that cap, and nothing at a gateway. The game
client renews every ten minutes while it runs, the website while a page is open and in view; after
the 12 hours the password is needed again. An expired, replaced or revoked
session renews nothing, and a renewal needs no accepted terms. The browser
cookie lives the 12 hours of the cap: an expired session's cookie finds no
session, so a renewal never sets the cookie again.

## Known limits of this deployment

Since 11.11.2026 both listeners count requests and guesses at passwords by
client address as well as globally and per account or bearer
([above](#sign-in-guesses-and-address-budgets)). What stays: four addresses
that each keep up their 600 requests a minute spend the 32 a second all
clients without a session share (service denial, not account takeover).
Since 12.11.2026 a place that was served fewer than 30 requests in its
minute (an IPv6 place is counted by its /48 for this) then draws on 8 a
second kept beside the 32, so a household still signs in meanwhile. Sixteen
places that each ask thirty a minute spend those too, and a flood from many
addresses is the proxy's or the network's to limit: the firewall's file
([above](#connection-limits-in-front-of-the-proxy)) bounds connections, not
requests. Requests that carry a bearer nobody issued were counted
as a session's until 11.11.2026, and one address sending them kept signed-in
clients and gateways at `Busy`. They now have a second of their own (64 for
all of them together, 600 a minute for one address) and spend nothing of
what sessions share (512 control and 256 data requests a second, 128 and 64
for one bearer, a table of 1,024 bearers a second), nor of the login budget
([native login](native-login.md#a-bearer-nobody-issued)). A bearer is a
session's for these counts once an answer has vouched for it; its own
budget is looked at before the shared windows are drawn on, so what it asks
beyond that spends nothing of anyone else's. Until 12.11.2026 four such
bearers at their own budgets spent a shared window, from one address, with
two accounts' sessions or one account's gateway tokens. Since then all the
bearers of one address draw half of each shared window between them (256
control and 128 data requests a second), and every credential and token of
one account's gateways is counted as one bearer (the store says whose a
bearer is when an answer first vouches for it: one statement).
What stays: two addresses with two sessions each still spend a shared
window, and one account has two sessions; one account's two sessions spend
their own address's half, and everyone else behind that address (a campus,
a carrier's shared address) is `Busy` meanwhile; one account's gateways
share 128 and 64 a second, however many they are; a request the listener
knows no address of has no address's share to draw on; and
seven addresses of made-up bearers fill the strangers' second, which a
session meets with its first request after a restart or after half an hour
without one. Each restart, so each deploy, forgets which bearers were
vouched for.
The table of unfinished sign-ins is larger than the shared budget
can fill (4096 against 40 a second for 60 seconds), so it is never the cheaper
target (31.10.2026; before, 128 entries filled at two requests a second).
Whoever knows an account's identity can still keep its password sign-in at
`Busy` everywhere but at the account's own places, now with 100 requests a
day from four addresses, and discovery still says whether a username is
enrolled; the counts live in memory, so a restart forgets who signed in
where. The guessing brake's numbers are the owner's to decide (the security
records of 31.10.2026 and 11.11.2026).
Central sessions last 30 minutes unless renewed, and at most 12 hours. Password change
is served on both listeners since 28.10.2026 ([native login](native-login.md)
§"Password change"; no migration or grant), with a form on the website since
12.11.2026 (the account area's security section) and none in the client yet.
A reset with an administrator's code is served on both listeners since
28.10.2026 ([account reset](account-reset.md)) and has its page on the
website since 12.11.2026 (`/{lang}/reset`). An administrator makes the
code on the new site's `/{lang}/account/admin` since 12.11.2026 (an
account's page in the administration leads there, opened at that
account). A reset code by mail is served on both
listeners since migration 000026, where the `mail` block is configured
([mail](#mail)), and the website's reset page asks for it. Central mails
30 reset codes an hour for all accounts together, counted in memory when a
code is made: requests for names that get no mail spend none of it. Past
the 30 a request is dropped and the journal says so once an hour
("the hour's reset mails are spent"). Discovery reveals enrollment state for a supplied
username (see [native login](native-login.md)), on both listeners.
