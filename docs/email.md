# An account's e-mail address

*12.11.2026, roadmap M1 step 4; migration `m20261112_000026_email`; built
on [account reset](account-reset.md). The record of its three passes is in
[security assurance](security-assurance.md), "An account's e-mail address
and a reset code by mail".*

An account may have one e-mail address. No account needs one: a sign-in
name and a password are enough, and nothing else in the protocol knows an
address. The address has two uses. Central mails a **reset code** to it when
the person asks for one, and it mails **notices** to it: when it is
replaced or removed, and since 12.11.2026 when the account's login changed
(a reset was finished, the password was changed, an administrator made a
reset code; [below](#notices-that-the-login-changed)). It is not a sign-in
name, it is not shown to anyone else, and it decrypts nothing: a reset
restores the login only, as an administrator's does
([account reset](account-reset.md)).

*Notices that the login changed: 12.11.2026, no migration and no change to
the wire. The record is in [security assurance](security-assurance.md),
"Notices of a changed login".*

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
| Notices that wait for the worker; the same notice to the same address of an account once | 64 | `NOTICE_QUEUE` |
| A mailed reset code lasts | 1 hour | `central_postgres::MAILED_RESET_HOURS` |
| No second mailed code for an account within | 10 minutes | `central_postgres::RESET_MAIL_GAP_MINUTES` |
| Mailed codes for an account in 24 hours | 3 | `central_postgres::RESET_MAILS_A_DAY` |
| An administrator's code lasts (unchanged), as its notice says | 24 hours | `central_postgres::ISSUED_RESET_HOURS`; `ISSUED_HOURS` (`email.rs`) |
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
one account's digest says nothing of another's. The code is never in a log
or a Debug output, and in a link only in the fragment of the mail's button
([below](#the-code-in-the-buttons-link)).

It is short because little depends on it and few guesses are taken. It
authorizes one thing: making the address it was mailed to the account's
address, for a session that is signed in to that account already. A session
has 5 tries at one code and 10 codes a day, 50 guesses a day at 2^40.

## The mails

Crate `central-mail`. Each mail has a plain text part and an HTML part:

- the code that confirms an address;
- a reset code, which names the account's sign-in name;
- a notice to an address that was replaced or removed;
- a notice to the account's confirmed address that a reset code set a new
  password, saying whether the data was kept or a new vault began;
- a notice to it that the password was changed;
- a notice to it that an administrator made a reset code.

The HTML is a table layout with inline styles. It has no image except,
where the operator turned it on, Baylee's logo, which the mailer attaches
to the mail itself (`cid:baylee-logo`, [below](#configuration)); no style
sheet, no script and nothing loaded from elsewhere. The links go to the
configured site and nowhere else: a button to the site's reset page (the
reset mail, and the notices of a reset and of a changed password) or to the
account's security page (the confirmation mail, and the notices of a
replaced or removed address and of an administrator's code), and in every
mail's footer the site's start page (`{site}/{lang}/`). A code is printed,
and the two code mails' buttons carry it in their link's fragment, and only
there: never in a path or a query, never in any other link; no notice
carries one ([below](#the-code-in-the-buttons-link)). Every value is
escaped.

### The code in the button's link

*12.11.2026, by the owner's word: whoever comes to the page through the
mail's button finds the code filled in, "a good, safe way".*

- **Where.** The reset mail's button links to `{site}/{lang}/reset#code=`
  and the reset code as printed (`RESET-…`); the confirmation mail's button
  ("Confirm address") to `{site}/{lang}/account/security#code=` and the
  confirmation code (`XXXX-XXXX`). The plain text has the same addresses
  under the button's words. The code is in the fragment and nowhere else in
  the address: a browser sends no fragment to the site, so it is in no
  request, no server's or proxy's log, and in no `Referer` (the site sends
  `Referrer-Policy: no-referrer` besides).
- **The page.** The island reads `location.hash` once, when it starts
  (`islands/fragment.rs`). It takes `#code=` and up to 64 letters, digits
  and hyphens, and keeps them only if they parse as its own kind of code
  (`ResetCode::parse` on the reset page, `MailCode::parse` on the security
  page); it fills the field with the code as printed, unless the person
  typed into it already. Whatever followed `#code=`, it removes the
  fragment at once with `history.replaceState`, the rest of the address
  and the entry's state kept: the address bar, the current entry of the
  history and a reload no longer hold it. Another fragment (a place on the
  page) stays.
- **Never by itself.** The island sends nothing because of the fragment:
  the person still presses the form's button. A link scanner that opens
  the link (some mail providers open every link they deliver) spends no
  code and learns nothing it did not read in the mail. Nothing logs or
  stores the code; it lives in the field as a typed one does.
- **The security page** has the code's field only while an address waits
  for its code. The account area takes the fragment on every one of its
  pages and keeps the code only on the security page, until the e-mail
  section's first read of Central's state: where an address waits, the
  code goes into its empty field; signed out, with no address that waits,
  without mail, or when the state could not be read, it is dropped
  silently. A later read never finds it.
- **The old way stays.** The code is printed in the mail, and typing it is
  as before.

What remains, and why it is acceptable, is in the record in [security
assurance](security-assurance.md), "The code in the mail button's
fragment".

### How a mail looks

*Since 12.11.2026, by the owner's word ("Mache bitte die E-Mails
schöner"). Both bodies say the same in the same order. The second round
(12.11.2026), after the owner read the first in Gmail and mailbox.org:
the plain text begins with its title and first sentence, the reset mail
sets the account's name in bold, a finished reset lists its facts, and
the footer links to the site.*

- **The card.** A card of at most 560 px on the site's sunlit ground
  (`#f2efe4`), with a thin edge and round corners; where the reader is
  narrower, the card is too. Outlook, which knows no maximum width, gets a
  560 px table in a conditional comment.
- **The band.** The card's top is a band in the blue hour's colour
  (`#0f1b2d`) with a gold edge (`#e3c07e`) under it. On it, the wordmark in
  letters: the name's first letter on a gold tile, as the site's header has
  its picture there (hidden from screen readers), and "Baylee" in the
  site's serif (Literata where a device has it, else Georgia). With the
  `logo` flag ([configuration](#configuration)) the logo stands in the
  tile's place: the picture the mailer attaches (`src="cid:baylee-logo"`),
  48 × 48 CSS px with `width` and `height` set, rounded, `alt=""` since
  the name stands beside it, and the name as high as the picture. Without
  the flag the band is exactly as before.
- **The title** is the mail's one heading (`h1`), in the serif at 26 px,
  with a short line in the accent (`#235b66`) under it. Then what happened,
  in paragraphs of 16 px. The reset mail sets the account's sign-in name
  in bold, in the title's colour, so the reader sees at once which account
  it is (only in the HTML: the plain text and the list's preview say it
  plainly).
- **The facts.** Where a notice says several things that came of what
  happened, they stand as a short list under its first paragraph: a line
  each behind a small dot in the accent (a character, not a picture; a
  table, as every reader draws one; the dots are not read aloud). The
  notices of a finished reset do this with their three facts (the data
  kept or a new vault, the sessions ended, what the recovery code does
  now). The other mails say one or two things and keep their paragraphs.
  No words changed.
- **The code** in a monospace font in a box of its own: a light tint with a
  dashed frame in the accent, its label above it in small capitals, and
  under it how long it lasts and that nobody at Baylee asks for it. A click
  selects the whole code where the reader allows it. The confirmation code
  (9 characters) at 30 px; the reset code (45 characters) at 15 px with no
  space added between its characters, about 406 px, so that it fits on one
  line of the 470 px inside the box (Gmail and mailbox.org broke it in two
  at 17 px on the old 520 px card). On a narrow screen it breaks at its
  hyphens.
- **The warning.** What to do if this was not you stands apart on the
  site's warning tint with an edge in its colour: in the notices, where it
  matters. The code mails end with the calmer "if this was not you, ignore
  this mail" in a quieter hand.
- **The button.** The link to the page that helps is a button made of a
  table cell in the accent with the words in the light ink (7.4:1). The
  cell carries the colour and Outlook's padding (`mso-padding-alt`), the
  link a padding of its own for every other reader: 48 px high, the whole
  of it a target. Every mail has one; the confirmation mail's
  ("Confirm address") came with the code in the link's fragment.
- **The footer** under a thin rule, at 13 px, and under it a quiet line:
  the site's host name (`baylee.app`), underlined in the accent, which
  links to the site's start page in the mail's language.
- **Dark readers.** Without a style sheet there is no dark variant, so the
  mail says it is light only (`color-scheme` and `supported-color-schemes`
  metadata, and on the root element). Readers that darken mails anyway do
  it in two ways: they invert everything, or they darken light grounds and
  lighten dark text. Every text stands on a ground the mail sets (band,
  card, box, warning, button), and the band and the button are dark under
  light words, which such readers leave alone. A test measures every pair
  of ink and ground at 4.5:1 or more as drawn and as inverted, and the
  frames at 3:1.
- **The list of mails** shows the first paragraph beside the subject: it
  stands first in the body, hidden. It is never a code.

The plain text is complete on its own: the title, an empty line, then the
paragraphs with an empty line between them, the facts a line each after
`- `, the code alone on a line under its label, the link's address alone on
a line under its words, and the footer and the start page's address under
the signature's mark (`-- `), which mail readers set apart. A list of mails
that shows the text's beginning (mailbox.org does) shows the title and the
first sentence; until the second round a line of `=` under the title stood
there.

To look at them: `BAYLEE_MAIL_SHOTS=<folder> cargo test -p central-mail --
--ignored` writes every mail in English and German as HTML and text into
the folder, with fresh random codes, for a browser, and each again with
the logo (`…-logo.html`; a browser cannot resolve `cid:`, so to look at it
the `src` is pointed at a local copy of the picture). They were looked at
in Chromium at 360 and 640 px, as drawn and inverted, on 12.11.2026; the
first round also by the owner in Gmail and mailbox.org, the second round
and the logo not yet in a real mail program.

The words are `id = text` lines in `crates/central-mail/locales/<lang>/mail.ftl`,
one file for each of the site's eleven languages, read by a small parser of
the crate's own. English and German are written by hand; the other nine
were translated by machine (12.11.2026, by the owner's instruction) and
have not been read by a speaker. A test holds that every language has its
own words, by id. The nineteen messages of the three notices that a login
changed (`reset-done-*`, `password-*`, `issued-*`) and the confirmation
mail's button (`confirm-link`) are English in those nine until they are
translated; the test allows exactly those twenty ids to be English there,
and German has its own for every id. A tag Central has no file for is
written in English.

A code mail is in the language the request asked in. A notice is in the
language the address was confirmed in.

## Notices that the login changed

Three changes to an account's login are told to its **confirmed** address,
if it has one and this Central has a mailer:

| Change | When | The notice says |
| --- | --- | --- |
| A reset was finished | `ResetFinish` succeeded, for `keep` and for `new` | a reset code set a new password; the data was kept, or the account starts with an empty vault and the data from before cannot be opened; every session was ended. If this was not you: ask an administrator for a reset code; with `keep`, a recovery code the account has still opens the data, with `new`, an earlier recovery code opens nothing |
| The password was changed | `ChangePasswordFinish` succeeded | every other session was signed out. If this was not you: someone knows the password; set a new one with a reset code from the reset page or from an administrator |
| An administrator made a reset code | `AdminRequest::ResetLogin` succeeded | a code exists and works once for 24 hours, and the password works until it is used. If you did not ask for it: sign in and change the password on the security page, which ends the code (`account-reset.md`, "As built"), then tell an administrator |

- **The address is read in the change's transaction.** `finish_reset`,
  `change_password` and `issue_reset` read the confirmed address and its
  language after they hold the account's row, which every write of the
  address locks first, and return it with their answer, as `email_confirm`
  and `email_remove` return the address that lost its place (`Told`). It
  is the address the account had when the change was committed. An address
  that only waits for its code is none. The statements that change the
  login are as they were; the read is one more `SELECT` in the same
  transaction.
- **After the commit only.** The identity service queues the notice once
  the store answered with a commit (`Mails::committed`), in the code that
  returns the answer, with nothing to await in between. A refused change
  (a wrong password, a spent or wrong code, a refused administrator) tells
  nobody. Without a mailer nothing is queued.
- **Never a reason to fail or wait.** The notice joins the queue the other
  notices wait in and is tried once by the worker, after the reset codes
  that wait. The answer is the store's whatever becomes of it. A full
  queue drops it; a mailer that does not take it leaves the change done.
- **Once while it waits.** The same notice to the same address of the same
  account is not queued a second time while the first waits, so an account
  that changes its password again and again holds one place of the 64.
- **Not for a mailed code.** A code mailed on the account's own request is
  made by the worker (`issue_mailed_reset`), not by `ResetLogin`: the
  reset mail itself is what the address learns.
- **No account named.** The notices say "the Baylee account of this
  address" and name no account. The sign-in name is half of what a reset
  by mail needs, and these mails go out without anyone asking for them;
  the confirmation mail and the two older notices name none either. Only
  the reset mail names it, and only someone who typed the name asks for
  that.
- **No code, no new link.** No notice carries a code; their links are the
  site's reset and security pages, like the other mails'.

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
  mail with a code. A notice has no `ttl_secs` and waits its turn;
- with `"inline": ["baylee-logo"]` as well, and only, where the `logo` flag
  is set ([below](#configuration)): the mailer attaches the picture its own
  configuration names, as `Content-ID: <baylee-logo>`. A mailer from before
  that field refuses it, as it refuses every field it does not know.

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
  "token": "…",
  "logo": false
}
```

- `endpoint`: `http://`, a numeric loopback address and a port, nothing
  after it. There is no default.
- `site`: the origin every link of the mails starts with. A canonical HTTPS
  origin on the live service, a numeric loopback HTTP origin in
  development.
- `token`: Central's bearer token for the mailer, 32 to 256 characters of a
  bearer token's alphabet. It arrives with the other secrets on standard
  input, never in an argument or the environment, and is wiped after use.
- `logo`: optional, `false` when absent. `true` says that the mailer
  attaches Baylee's logo when asked: every mail then asks for it
  (`inline`, above) and shows it in the band
  ([how a mail looks](#how-a-mail-looks)). Set it only once a mailer with
  the picture runs: an older one refuses the field, and with it every
  mail. Deploy the mailer first, then the flag
  ([operations](operations.md#mail)).

A service whose block names anything else does not start.

## A Central without a mailer

`status`, `withdraw` and `remove` work, and `status` says `mail: false`.
`set`, `resend` and `confirm` are `Rejected`, and so is `ResetMail` for
every name. An address removed there is not told, and no change of the
login is told either.

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
  The same notice to the same address of an account waits once.
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
- **The notices of a changed login name no account.** A mailbox whose
  address two accounts confirmed is not told which of them changed.
- **They tell the address the account has.** A thief who takes the login
  and then replaces the address gets the later notices; the old address is
  told once that it was replaced. The notice of the change that let the
  thief in goes to the address the account had when it was committed: a
  password change gives the new session five minutes in which it may name
  an address, but the notice was queued before. Whoever reads the mailbox
  is told what the account's owner is told; a reset made with a mailed
  code tells the mailbox that asked for it.
- **A notice can be lost.** It is tried once. A mailer that is down, a
  full queue, a service that stops while it waits, and a request cut off
  between the store's commit and its return each leave the change done and
  untold. A mail the mailer took can still be lost on its way.
- **The queue is shared.** Notices for all accounts wait in one queue of
  64. One account holds at most one place for each kind of notice, but
  someone who holds many accounts with confirmed addresses and changes
  their passwords (up to four changes a minute each, by the login budget) while
  the mailer is slow can keep it full, and others' notices are dropped.
- **An administrator's own account.** The notice of a changed password
  says to set a new one with a reset code, but no code is made for an
  administrator's account: an administrator who did not change their
  password removes the role first, as for any reset
  ([account reset](account-reset.md)). The words are the same for every
  account.
- **An administrator can mail an account's address** with each code they
  make. Each code is in the audit, and the mailer's limit for the account
  bounds the mails.
- **Nine languages** say the notices of a changed login and the
  confirmation mail's button in English until
  they are translated.

## What the clients need

- Website: an e-mail section in the account area, and a way to ask for a
  reset code by mail on the reset page. Both are another line of work and
  not part of this change.
- Baylee client (M6): nothing yet. `central-client` has the functions.
