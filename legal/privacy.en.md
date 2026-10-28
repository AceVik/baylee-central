<!-- version: 2026-10-28 -->
<!-- updated: 2026-10-28 -->

# Baylee Central privacy policy

This policy describes which personal data Baylee Central (baylee.app and
central.baylee.app) processes, why, and for how long. Gateways you play on
process data of their own; their own privacy notices explain that.

*This is a translation for your convenience. Only the German version is
legally binding.*

## 1. Controller

The controller is the operator of Baylee Central. Name, address and contact
details are in the [legal notice](https://baylee.app/impressum). There is no
data protection officer, as none is required.

## 2. Our principles

- **As little as possible.** An account needs only a username and a password.
  Everything else is optional.
- **We do not know your password.** Sign-in uses OPAQUE: your password never
  leaves your device; we store only a record it cannot be read from.
- **Your private data is unreadable to us.** Settings, deck backups and chat
  history are encrypted on your device before they reach us.
- **No tracking, no ads, no sale.** We use no analytics or advertising tools
  and pass no data on for advertising.
- **Verifiable.** The source code of Baylee Central is publicly readable, so
  anyone can check that this policy is true.

## 3. What we process, why, and on what basis

- **Username, account ID, creation time, display name and tag** (a profile picture will follow) – running and showing your account. Other players see your display name and tag, never your username; an account moved from the official gateway keeps the name and tag it had there. *Art. 6(1)(b) GDPR (contract)*
- **Roles (administrator)** – administering Baylee Central. *Art. 6(1)(b) GDPR*
- **The time of your last sign-in with your password** (no address) – administering accounts and noticing misuse. *Art. 6(1)(f) GDPR (our interest in secure operation)*
- **OPAQUE sign-in record, encrypted key container, credential state** – sign-in, protecting your encrypted data. *Art. 6(1)(b) GDPR*
- **A reset code's fingerprint, if an administrator made one for you** (we keep only its SHA-256 digest, never the code) – letting you set a new password when you have forgotten yours. *Art. 6(1)(b) GDPR*
- *Not available yet:* **e-mail address (optional) and its confirmation** – password reset, account notices. *Art. 6(1)(b) GDPR*
- **Your key container, encrypted under your recovery code** (we never learn the code); a second sign-in factor (optional) will follow – protecting your account and your encrypted data. *Art. 6(1)(b) GDPR*
- **Sessions, sign-in tickets for gateways, gateway access** – keeping you signed in and signing you in to gateways. *Art. 6(1)(b) GDPR*
- **Encrypted objects (settings, gateway list, deck backups, chat history) with size, revision and time** – backup across devices. *Art. 6(1)(b) GDPR*
- **The versions of our terms and of the terms of the gateways you play on that you accepted, with the time** – proof of acceptance, showing new versions. *Art. 6(1)(b) and (c) GDPR*
- **Listed gateways, their addresses, description and administrators** – gateway directory and management. *Art. 6(1)(b) GDPR*
- **Bug reports you send, with the attachments you leave ticked** (directly or through a gateway) – finding and fixing bugs. *Art. 6(1)(f) GDPR (our interest in a game without bugs)*
- **Crash reports, if you allowed them** – finding and fixing crashes. *Art. 6(1)(a) GDPR (consent, which you can withdraw at any time in the settings)*
- **Messages sent with the website's contact form** (the text; name and e-mail address only if you give them; language; time) – answering you. *Art. 6(1)(f) GDPR*
- **Anonymous usage count:** during an offline game the client sends a random identifier of that game once a minute and nothing else; we keep it in memory only – counting how many people play offline. *Art. 6(1)(f) GDPR*; can be switched off ("Send anonymous usage count").
- **Reports about content or accounts, moderation decisions** – enforcing the terms, duties as a hosting service. *Art. 6(1)(c) and (f) GDPR*
- **A record of what our administrators did** (who acted, what, which account or gateway it concerned, when; no names, reasons or notes) – keeping administrative decisions traceable. *Art. 6(1)(c) and (f) GDPR*
- **Technical access data (IP address, time, requested address) briefly in memory and in logs** – security, defending against abuse and overload. *Art. 6(1)(f) GDPR*

Our legitimate interest in the last item is secure and undisturbed
operation. Sign-in attempts are counted to slow down password guessing;
these counters live only in memory.

Baylee Central does not see what you play on gateways: games, decks on
gateways, chat delivery and replays are processed by the gateway concerned
under its own privacy notice, the gateway we run ourselves included.

## 4. What the Baylee app connects to by itself

Besides Baylee Central and the gateways you choose, the Baylee client
connects only to:

- **GitHub** (github.com, api.github.com) for the desktop version to look for
  updates and download them; you can switch this off ("Check for updates
  automatically"). GitHub's terms apply.
- **Scryfall** (scryfall.com) for card images and card data when you are not
  signed in to a gateway; signed in, the gateway provides them (the browser
  version always loads card images from Scryfall). Scryfall's terms apply.

These providers learn your IP address, but nothing about your account.

## 5. Cookies and browser storage

The website sets only one technically necessary cookie: a session cookie
while you are signed in, for at most 12 hours. It protects itself against
forged requests without a cookie. This needs no consent (section 25(2) no. 2 TDDDG). Your choice of language and display is
stored locally by your browser. There are no tracking cookies and no embedded
third-party services that collect data about you.

## 6. Recipients

- **Hosting:** our servers are with a hosting provider in Germany acting as a
  processor (Art. 28 GDPR) on our instructions only.
- **E-mail** (once it exists): if you add an e-mail address, we send confirmations and reset
  links through a mail provider in the European Economic Area acting as a
  processor.
- **Our administrators** see, for administering accounts, your username,
  display name and tag, where your account came from, when it was created and
  last signed in, whether it has a recovery code, and the gateways you own;
  never your encrypted data, which we cannot read.
- **Gateways:** when you sign in to a gateway, it receives your account ID,
  your display name and tag, and the confirmation that you are signed in, but
  not your password or your encrypted data.
- **Authorities** receive data only where we are legally obliged.

No data is transferred to countries outside the European Economic Area.

## 7. How long we keep data

- Account data is kept until you delete your account. We then remove it from
  the running system at once and from backups within 12 months at the latest,
  when the backups are overwritten in rotation. A gateway you registered is
  deleted with your account; players can then no longer sign in to it through
  Baylee Central. If your account moved from the official gateway, that
  gateway keeps its own copy of your old account there, its password removed;
  we delete it on request, and for every account once moving from the
  official gateway ends.
- Sessions end 30 minutes after their last use and at most 12 hours after
  signing in, sign-in tickets after seconds.
- The time of your last sign-in is kept while your account exists.
- The record of what our administrators did is kept for three years, then
  deleted automatically.
- A reset code's fingerprint is deleted when the code is used or you change
  your password, and otherwise within a day after it expires, 24 hours after
  it was made.
- Server logs are deleted after 14 days at the latest.
- Reports and moderation decisions are kept as long as needed to keep them
  traceable, at most three years.
- Bug and crash reports are deleted once the bug is dealt with, after three
  years at the latest.
- Contact form messages are deleted 90 days after they are dealt with, a
  year after they arrived at the latest.
- The anonymous usage count forgets a game 150 seconds after its last sign
  of life.
- The record of which version of our terms you accepted is kept for as long as
  your account exists.

## 8. Your rights

You have the right of access (Art. 15), rectification (Art. 16), erasure
(Art. 17), restriction (Art. 18), data portability (Art. 20) and to object to
processing based on legitimate interests (Art. 21 GDPR). You can exercise
many of them yourself: deleting your account, exporting your data and
changing details is in the settings; deleting asks for your password again,
and a deleted account cannot be restored. Otherwise a message to the contact
details in the legal notice is enough.

You may lodge a complaint with a data protection supervisory authority, for
example the one where you live or the one responsible for us.

## 9. Required data, automated decisions

Without a username and password no account can exist; everything else is
optional. Offline you play without any account. There is no automated
decision-making or profiling within the meaning of Art. 22 GDPR.

## 10. Changes

If this policy changes, we say so on the website and in the client.
