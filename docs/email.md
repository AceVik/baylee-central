# An account's e-mail address

*12.11.2026, roadmap M1 step 4; migration `m20261112_000026_email`; built
on [account reset](account-reset.md). The record of its three passes is in
[security assurance](security-assurance.md), "An account's e-mail address
and a reset code by mail".*

An account may have one e-mail address. No account needs one: a sign-in
name and a password are enough, and nothing else in the protocol knows an
address. The address has two uses. Central mails a **reset code** to it when
the person asks for one, and it mails a notice to it when it is replaced or
removed. It is not a sign-in name, it is not shown to anyone else, and it
decrypts nothing: a reset restores the login only, as an administrator's
does ([account reset](account-reset.md)).

Central speaks no SMTP. A separate service on the same host,
`baylee-mailer`, takes one mail a request on loopback and delivers it.
A Central without a mailer in its configuration sends no mail
([below](#a-central-without-a-mailer)).

## The requests

`central_api::email` and two requests in `central_api::wire`. Nothing that
existed before changed: no request, no answer, and the account export is as
it was.

### `Email { request }`

For a session, on both listeners: the website's cookie or the client's
bearer. `set`, `resend` and `confirm` need Central's terms in force
accepted; `status`, `withdraw` and `remove` do not, so a person who will
not accept new terms can still read the address and take it away, as they
can export and delete the account. The answer is always `Email { state }`, the address as Central holds it after
the action:

| Field | Meaning |
| --- | --- |
| `mail` | This Central can send mail. |
| `address`, `confirmed_at` | The confirmed address and when it was confirmed. |
| `pending`, `pending_until` | An address that waits for its code, and when that code ends. |

The actions:

- **`status`** reads the state.
- **`set { address, lang }`** names an address. Central mails a code to it
  and keeps it apart: the address **waits**. A confirmed address stays in
  use until the new one is confirmed. An address that waited before is
  replaced, and its code ends. This changes who can reset the login, so it
  needs a password sign-in less than 5 minutes old (`Rejected` otherwise);
  the website proves the password once more right before it names the
  address.
  `Conflict` when the address is the account's confirmed one already.
- **`resend { lang }`** mails a new code to the address that waits. The
  earlier code ends, the new one lasts its own 15 minutes, and the wrong
  tries start again. `Rejected` when no address waits.
- **`confirm { code }`** confirms the address that waits. It becomes the
  account's address in one transaction, and the address it replaces is
  told. A wrong code is `Rejected` and counted. The fifth wrong code gives
  the address up: it has to be named again.
- **`withdraw`** gives up the address that waits. The confirmed one stays.
- **`remove`** removes the confirmed address and one that waits. The
  removed address is told.

`set` and `resend` answer `Busy` when the mailer did not take the mail or
the account asked too often. Nothing new waits then, and what waited before
still waits with the code it had. They answer `Rejected` when the mailer
refuses the address itself.

An address is checked by the mailer's own rules (`central_api::email::address`):
ASCII, at most 254 characters, a dot-atom local part of at most 64, a host
name of at least two labels. No display name, no quoted local part, no
address literal. A language tag is 2 to 10 lower-case letters and hyphens
(`en`, `pt-br`, `zh-hant`).

Addresses are not unique. Two accounts may hold the same one, and no answer
says whether an address is in use.

### `ResetMail { username, lang }`

Without a session, on both listeners. Central answers **`Done` whatever the
name**: for a name that is nobody's, an account without an address, an
account whose address only waits, an administrator's and an ordinary
account. `Busy` when the asking address asked too often or the worker's
queue is full. `Rejected` for a language tag of the wrong shape, and for
every name where Central sends no mail.

The request reads nothing of any account. Its handler checks the tag,
puts the name in a queue and answers. It is not
asynchronous and asks the store nothing, so neither the answer nor the time
it takes depends on the name.

Afterwards, off the request, a worker takes the name from the queue. If it
is the name of an account with a confirmed address that may be reset, the
worker makes a reset code and mails it. The code is made by the statement
that makes an administrator's code, with the same refusals: no code for an
administrator's account, a pending account or an unfinished legacy move.
Three more rules hold for a mailed code:

- It never replaces an administrator's code that has not expired.
- None is made within 10 minutes of the last one for the account.
- At most 3 are made for an account in 24 hours.
- At most 30 are made in an hour for all accounts together. The worker
  counts a code when it makes one, not a request when it comes: a request
  for a name that gets no mail spends nothing of the hour. Past the 30 the
  worker drops a request without asking the store, and the log says so
  once an hour.

A mailed code lasts 1 hour. It is redeemed exactly as an administrator's
code is: `ResetStart` and `ResetFinish`, on the website's reset page or
through `central-client`. Nothing in those two requests knows where a code
came from.

If the mailer does not take the mail, the code is taken back at once. The
try stays counted.

## The numbers

Every number is a named constant.

| What | Number | Constant |
| --- | --- | --- |
| A sign-in is fresh enough to name an address | 5 minutes | `central_postgres::EMAIL_FRESH_MINUTES` |
| A confirmation code lasts | 15 minutes | `central_postgres::MAIL_CODE_MINUTES` |
| Wrong codes for an address that waits; the last gives it up | 5 | `central_postgres::MAIL_CODE_TRIES` |
| Mails with a confirmation code for one account | 5 an hour, 10 a day | `CODE_MAILS_PER_HOUR`, `CODE_MAILS_PER_DAY` (`central-identity`, `email.rs`) |
| Accounts whose mails are counted at once | 16,384 | `CODE_MAIL_ACCOUNTS` |
| Mails with a confirmation code handed to the mailer at once | 4 | `SENDING` |
| Requests for a reset code by mail from one client address, on each listener | 5 an hour | `central_http`'s `RESET_MAIL` (`addresses.rs`) |
| Reset codes mailed for all accounts together | 30 an hour | `RESET_MAILS_PER_HOUR` |
| Requests that wait for the worker | 256 | `RESET_QUEUE` |
| Notices that wait for the worker | 64 | `NOTICE_QUEUE` |
| A mailed reset code lasts | 1 hour | `central_postgres::MAILED_RESET_HOURS` |
| No second mailed code for an account within | 10 minutes | `central_postgres::RESET_MAIL_GAP_MINUTES` |
| Mailed codes for an account in 24 hours | 3 | `central_postgres::RESET_MAILS_A_DAY` |
| An administrator's code lasts (unchanged) | 24 hours | `ISSUED_RESET_HOURS` (`central-postgres`, `reset.rs`) |
| The mailer's client connects within | 1 second | `CONNECT` (`central-mail`, `client.rs`) |
| A request to the mailer takes at most | 3 seconds | `OVERALL` |
| The mailer's answer: head, and body of a refusal | 8,192 and 4,096 bytes | `HEAD_BYTES`, `ANSWER_BYTES` |
| The mailer's token | 32 to 256 characters | `TOKEN_MIN`, `TOKEN_MAX` |

The mails say the two lifetimes, and the mailer is given the same numbers
as `ttl_secs` (900 and 3600): it does not hand a code mail over after its
code has ended.

The counts of mails and requests live in the service's memory, like its
other budgets. A restart forgets them. The mailer's own limits stay: it
counts each account by the `limit_key` Central sends (`account:` and the
account id's 32 hexadecimal digits), each recipient and each day. The
per-account rules for a mailed reset code (10 minutes, 3 a day) are in the
database and survive a restart.

The three seconds for the mailer are less than the five the listeners give
a request, so a mailer that does not answer is answered as `Busy`.

## The confirmation code

`central_api::email::MailCode`: 40 bits of operating-system randomness,
written `XXXX-XXXX` in Crockford's base32 and read back as the reset code
is. Central stores SHA-256 over a label, the account's id and the code, so
one account's digest says nothing of another's. The code is never in a
link, a log or a Debug output.

It is short because little depends on it and few guesses are taken. It
authorizes one thing: making the address it was mailed to the account's
address, for a session that is signed in to that account already. A session
has 5 tries at one code and 10 codes a day, 50 guesses a day at 2^40.

## The mails

Crate `central-mail`. Three mails, each with a plain text part and an HTML
part:

- the code that confirms an address;
- a reset code, which names the account's sign-in name;
- a notice to an address that was replaced or removed.

The HTML is a table layout with inline styles in the site's light colours.
It has no image, no script and nothing loaded from elsewhere. A code is
printed and never part of a link. The only links are to the site's reset
page (in the reset mail) and to the account's security page (in a notice);
the confirmation mail has none. Every value is escaped.

The words are `id = text` lines in `crates/central-mail/locales/<lang>/mail.ftl`,
one file for each of the site's eleven languages, read by a small parser of
the crate's own. English and German are written by hand; the other nine
were translated by machine (12.11.2026, by the owner's instruction) and
have not been read by a speaker. A test holds that every language has its
own words. A tag Central has no file for is written in English.

A code mail is in the language the request asked in. A notice is in the
language the address was confirmed in.

## The mailer

`baylee-mailer`'s `API.md` is the contract. Central sends one
`POST /v1/send` a mail:

- over plain HTTP to a numeric loopback address and a port, and nowhere
  else. The client's resolver answers that one address, it takes no proxy,
  follows no redirect and keeps no connection;
- with `Authorization: Bearer` and Central's own token;
- with a random `Idempotency-Key` header for each mail. Central asks once
  and never again, so the key only keeps one request from becoming two
  mails on the way;
- with `to`, `subject`, `text`, `html`, `limit_key`, and `ttl_secs` for a
  mail with a code. A notice has no `ttl_secs` and waits its turn.

What the mailer answers becomes one of four outcomes: `202` and `200` are
sent, `429` and `503` are busy, `400` with `invalid_address` is a refused
address, and anything else, or no answer, is unavailable. Nothing of the
mailer's answer reaches a client of Central's.

The mailer's queue is in memory, and a mail it took may still not arrive.
So every mail can be asked for again: `resend`, and `ResetMail` after ten
minutes.

## Configuration

An optional `mail` block in the service's document, live
([operations](operations.md#mail)) and development:

```json
"mail": {
  "endpoint": "http://127.0.0.1:<the mailer's port>",
  "site": "https://baylee.app",
  "token": "…"
}
```

- `endpoint`: `http://`, a numeric loopback address and a port, nothing
  after it. There is no default.
- `site`: the origin the mails' two links start with. A canonical HTTPS
  origin on the live service, a numeric loopback HTTP origin in
  development.
- `token`: Central's bearer token for the mailer, 32 to 256 characters of a
  bearer token's alphabet. It arrives with the other secrets on standard
  input, never in an argument or the environment, and is wiped after use.

A service whose block names anything else does not start.

## A Central without a mailer

`status`, `withdraw` and `remove` work, and `status` says `mail: false`.
`set`, `resend` and `confirm` are `Rejected`, and so is `ResetMail` for
every name. An address removed there is not told.

## What is stored

Migration 000026 ([PostgreSQL](postgresql.md#an-accounts-e-mail-address-migration-000026)),
two tables beside the accounts, both deleted with their account:

- `account_emails`: the confirmed address, the language it was confirmed
  in, when, and what a reset by mail needs (the digest of the code that was
  mailed, and the times of the last mailed codes, three at most).
- `account_email_pending`: an address that waits, its language, the digest
  of its code, when the code ends and how many wrong codes were tried.

A row whose code has ended counts as absent, and the operator's daily sweep
deletes it. No code is stored, only digests.

**Reading the address.** The account export (`AccountExport`) is unchanged
and does not hold the address: the released client decodes that answer
strictly. A person reads the address Central holds with
`EmailRequest::Status`.

**Deleting an account** deletes both rows with it.

**The audit** has no event for an address. It records what administrators
do; an account's own password change has none either.

## The SDK

`central-client`: `Session::email(transport, EmailRequest)` returns the
state, and `reset_mail(transport, username, lang)` asks for a reset code by
mail. Both refuse a malformed address or tag before anything is sent.

## As built

- **Mail first.** `set` and `resend` hand the mail to the mailer before the
  store keeps anything. Everything the store would refuse is checked before
  the mail goes out, and the store, which decides, is asked again
  afterwards. A mail the mailer did not take leaves nothing behind.
- **An attempt counts.** The account's 5 an hour and 10 a day count the
  mails Central hands over, also those the mailer refuses.
- **Whose reset code.** A mailed code's digest is stored a second time,
  beside the address it went to. A code whose digest is not there is an
  administrator's. The accounts table and the statements of the previous
  binary are as they were.
- **A changed address ends a mailed code.** Confirming another address and
  removing the address end a reset code that was mailed to the old one. An
  administrator's code stays.
- **One worker.** The reset codes and the notices are mailed by one worker,
  one piece at a time. On a stop it finishes the piece in hand: a code that
  was made is mailed or taken back. What still waits is dropped.
- **Notices are tried once.** A notice the mailer does not take, or one
  that finds the queue full, is not sent. The change it tells of is done.
- **Logs.** The service logs one line when a mail was not handed over:
  no address, no name, not which mail. `Mail`, `Token`, `Client` and the
  store's types print nothing of what they hold.

## Accepted limits

- **Whoever reads the mailbox can take the login.** That is what the
  address is for, and why no account needs one. They ask for a code, read
  it and set a new password. With a new vault they also destroy the
  account's encrypted data, as the holder of an administrator's code can.
  The mail opens no data: keeping it needs the recovery code or a
  signed-in device.
- **A mailed code says that the account has an address**, to whoever reads
  that mailbox and to nobody else. No answer of Central's says it.
- **A mail on request.** Anyone who knows a sign-in name can have a reset
  mail sent to its address: once in ten minutes, three a day. A new mailed
  code replaces the last mailed one, so a person may be holding a code that
  has just ended.
- **Thirty mails an hour for everyone.** Requests for names that get no
  mail spend nothing of it, so asking about made-up names keeps nobody's
  mail back. Someone who holds accounts with confirmed addresses can spend
  it: each such account gives three mails a day, so keeping the hour spent
  takes about 240 accounts. Others' requests are then dropped without an
  answer that says so. An administrator's code still works. (Until
  12.11.2026 the 30 counted requests, and three addresses asking about any
  name spent them.)
- **The queue.** Requests without a session share 40 a second
  (`limits.rs`); whoever spends those keeps this request out with every
  other public one. The queue of 256 fills only while the store does not
  answer.
- **A session can remove the address.** `remove` and `withdraw` need no
  fresh sign-in. The removed address is told, if the mailer takes the
  notice.
- **A stolen session within five minutes of its sign-in can replace the
  address**, confirm it from a mailbox the thief reads, and then take the
  login with a mailed reset code, and with a new vault destroy the data. A
  session taken later cannot name an address: that needs a new sign-in
  with the password. The old address is told; an account that had none
  learns it only from its account area. A password change, which needs
  the password itself, ends every session.
- **The terms.** A person who does not accept new terms can read,
  withdraw and remove the address, and not name a new one.
- **Timing off the request.** The worker takes longer for a name it mails
  for than for one it does not. A requester sees none of it: the queue
  holds more than an hour's requests, so it is never the reason for an
  answer.
- **A request cut off** between the mail and the store's write leaves a
  mail whose code confirms nothing. Asking again mails a new one.
- **The three-a-day count** follows the address row: it survives a replaced
  address and ends with a removed one.
- **Four refusals in a row** (wrong codes, for one) count against the
  session's bearer in the service's request budgets, as the refusals of
  other requests do. The session is then counted with the bearers nobody
  vouched for, until an answer vouches for it again (its next status or
  summary request).
- **`Busy` after `set` has several reasons**: the account's own count, a
  mailer that is down, and the mailer's limits. One of those is the
  mailer's limit for a recipient, counted over all of its clients. A
  session that names an address and gets `Busy` may therefore be seeing
  that the address got much mail lately. The answer does not say which
  reason it was.
- **No mail says that a reset was finished** or that a password was
  changed. The three mails are the ones above.

## What the clients need

- Website: an e-mail section in the account area, and a way to ask for a
  reset code by mail on the reset page. Both are another line of work and
  not part of this change.
- Baylee client (M6): nothing yet. `central-client` has the functions.
