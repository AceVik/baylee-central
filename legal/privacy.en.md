<!-- version: 2026-11-26 -->
<!-- updated: 2026-11-26 -->

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

- **Username, account ID, creation time, display name and tag** – running and showing your account. Other players see your display name and tag, never your username; an account moved from the official gateway keeps the name and tag it had there. Your public player profile at `/{lang}/player/{Name}-{tag}` (display name, tag, profile picture, ranked standings and earned badges) is visible by default and can be turned off at any time in your Profile settings. *Art. 6(1)(b) GDPR (contract)*
- **Optional personal profile details (first name, last name, date of birth)** – stored exclusively for you in your private profile settings and never shown publicly or to other players or gateways. Date of birth is validated for the minimum age of 13. These fields are included in your data export and deleted when your account is deleted. *Art. 6(1)(b) GDPR*
- **Roles (administrator)** – administering Baylee Central. *Art. 6(1)(b) GDPR*
- **The time of your last sign-in with your password** (no address) – administering accounts and noticing misuse. *Art. 6(1)(f) GDPR (our interest in secure operation)*
- **OPAQUE sign-in record, encrypted key container, credential state** – sign-in, protecting your encrypted data. *Art. 6(1)(b) GDPR*
- **A reset code's fingerprint, if an administrator made one for you or one was sent to your e-mail address** (we keep only its SHA-256 digest, never the code) – letting you set a new password when you have forgotten yours. *Art. 6(1)(b) GDPR*
- **E-mail address (optional)**, the language its mails are written in and when you confirmed it; an address you named that waits for its code, with the code's digest; the times reset codes were mailed to you in the last 24 hours – confirming the address, sending a reset code when you or anyone who knows your sign-in name asks for one (three a day at most), telling the old address when it is replaced or removed, telling the confirmed address when a reset code set a new password, when your password was changed or when an administrator made a reset code for your account. *Art. 6(1)(b) GDPR*
- **Your key container, encrypted under your recovery code** (we never learn the code); a second sign-in factor (optional) will follow – protecting your account and your encrypted data. *Art. 6(1)(b) GDPR*
- **Sessions, sign-in tickets for gateways, gateway access**; when you pair the Baylee app with the signed-in website, for five minutes at most, the app's public key and your master key encrypted for it – keeping you signed in, pairing devices and signing you in to gateways. *Art. 6(1)(b) GDPR*
- **Encrypted objects (settings, gateway list, deck backups, chat history) with size, revision and time** – backup across devices. *Art. 6(1)(b) GDPR*
- **Profile picture (optional):** a square picture of 256 × 256 pixels (AVIF and PNG, and for a moving picture an animation of at most 100 frames) without the photo's details (such as place, camera or time), its fingerprint (SHA-256) and when you set it, under a random key – showing it beside your display name; anyone can see it at its address, signed in or not. *Art. 6(1)(b) GDPR.* Reports about a picture (the reporting account, the picture's fingerprint, the reason, an optional note, the time; the person whose picture it is never learns who reported it) and the count of your uploads (ten an hour, thirty a day) – removing unlawful and abusive pictures. *Art. 6(1)(f) GDPR (our interest in a service free of such content)*
- **Daily and weekly challenges and Tower expeditions:** attempts, results, hints used, chosen upgrades, remaining lives and badge progress; from the official gateway, a game's result and measured time – offering and scoring the challenges. Your display name, tag and result appear on a leaderboard only if you enable it for that challenge or season. *Art. 6(1)(b) GDPR*
- **The versions of our terms and of the terms of the gateways you play on that you accepted, with the time** – proof of acceptance, showing new versions. *Art. 6(1)(b) and (c) GDPR*
- **Listed gateways, their addresses, description and administrators** – gateway directory and management. *Art. 6(1)(b) GDPR*
- **Bug reports you send, with the attachments you leave ticked** (directly or through a gateway) – finding and fixing bugs. *Art. 6(1)(f) GDPR (our interest in a game without bugs)*
- **Crash reports, if you allowed them** – finding and fixing crashes. *Art. 6(1)(a) GDPR (consent, which you can withdraw at any time in the settings)*
- **Messages sent with the website's contact form** (the text; name and e-mail address only if you give them; language; time) – answering you. *Art. 6(1)(f) GDPR*
- **Anonymous usage count:** during an offline game the client sends a random identifier of that game once a minute and nothing else; we keep it in memory only – counting how many people play offline. *Art. 6(1)(f) GDPR*; can be switched off ("Send anonymous usage count").
- **Reports about content or accounts, moderation decisions** – enforcing the terms, duties as a hosting service. *Art. 6(1)(c) and (f) GDPR*
- **A record of what our administrators did** (who acted, what, which account or gateway it concerned, when; no names, reasons or notes) – keeping administrative decisions traceable. *Art. 6(1)(c) and (f) GDPR*
- **Donations through GitHub Sponsors and Ko-fi:** the amount, the currency, when it reached us, whether it was given once, regularly or refunded, and a fingerprint (SHA-256) of the platform's transaction number so that a payment reported twice counts once; no names, addresses, messages or accounts. Ko-fi notifies us of each payment; everything else in the notice is discarded without being stored or logged. Of GitHub Sponsors we ask only the account's totals and its one-time sponsorships, without the sponsors' names, of Liberapay only its public weekly total; the euro exchange rates come from the European Central Bank – showing how much of the monthly costs is covered and what was given once, as totals only, never as a single donation. *Art. 6(1)(f) GDPR (our interest in showing openly how Baylee is funded)*
- **Technical access data (IP address, time, requested address) briefly in memory and in logs** – security, defending against abuse and overload. *Art. 6(1)(f) GDPR*

Our legitimate interest in the last item is secure and undisturbed
operation. Sign-in attempts are counted to slow down password guessing;
these counters live only in memory.

Baylee Central does not see what you play on gateways: games, decks on
gateways, game chat delivery and replays are processed by the gateway concerned
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

The website sets only strictly necessary session cookies while you are signed in: a short-lived access cookie (`__Host-central`, 15 minutes) and a sliding refresh cookie (`__Host-central-refresh`, 3 days of inactivity, capped at 30 days from sign-in). It protects itself against forged requests without a cookie. This needs no consent (section 25(2) no. 2 TDDDG). Your choice of language and display is stored locally by your browser. There are no tracking cookies and no embedded third-party services that collect data about you.

## 5a. Chat and notifications

For the chat (Art. 6(1)(b) GDPR), Central stores personal contacts, contact
requests, blocks and group memberships, device public keys,
and encrypted personal-message envelopes and delivery metadata. Chat text is decrypted
only on your device. Personal chat uses Central; game chat remains with the gateway.
Central stores at most 5,000 messages per sender and per device; after that,
new ones are refused. History is not deleted automatically; it stays until you
delete your account.

While the Baylee app is signed in, it keeps a connection to Central open, on
which Central tells it that something about your own chats, contacts or groups
changed; the content itself does not travel on it. While that connection is
open, your friends (contacts you both accepted) see that you are online,
recognisable by your tag (the code after your display name), and whether you
are away, playing or sitting at a table; they do not see what you play, and
where only if you share your table (see below).
Nobody else sees it, neither pending requests nor blocked accounts
(Art. 6(1)(b) GDPR). Whether you are online and what you are doing, Central
keeps only in memory while the connection is open; it is neither stored nor
logged. The status a gateway shows while you play is that gateway's own.

In the Baylee app you choose whether your friends see you online, away or
offline. If you choose "offline", they never see you online, and nothing else
of your being connected reaches them either. Central stores this choice with
your account. "Playing" means you sit in a game that is running, online or
against the house AI; "at a table", that you sit at a table whose game has
not begun; you also appear away after ten minutes without input. The app
tells Central so (Art. 6(1)(b) GDPR).

If you share your table (a setting of the gateway you play on; it is on until
you turn it off), the app also tells Central which table you sit at while you
play or sit at one: the gateway's address, the game's identifier there, the
table's name, how many seats it has and how many are taken; never for a table
with a password. Central passes this to your friends as it does your status,
to nobody else and not while you chose "offline", likewise keeps it only in
memory while the connection is open, and neither stores nor logs it
(Art. 6(1)(b) GDPR).

If you enable notifications (Art. 6(1)(a) GDPR, section 25(1) TDDDG: your
consent, which you can withdraw for the future at any time in your profile),
Central stores this browser's push endpoint, public key,
authentication secret, language, a random installation generation and your category
choices. Your browser's push provider (Google, Mozilla or Apple) carries encrypted
generic hints and can see transport metadata such as IPs, times and sizes. Hints
contain no chat text, names or report contents. You can disable each category or
this device in your profile; administrator-only categories require current admin rights.
Logout removes subscriptions bound to that session, and account deletion removes
all subscriptions. Pending hints expire after one day. Subscriptions older than
90 days are no longer used and are removed when the account next subscribes.

The installed web app caches its public files and fonts for offline startup. Browser
storage also holds notification consent and an installation generation, never plaintext
chat history. Clearing site data removes that local state. Permission is requested
only after you choose to enable notifications; OS/browser settings can block delivery.

## 6. Recipients

- **Hosting:** our servers are with a hosting provider in Germany acting as a
  processor (Art. 28 GDPR) on our instructions only.
- **E-mail:** if you add an e-mail address, Baylee's own mail service on
  our server sends its confirmation code, the reset codes asked for and the
  notices through mailbox.org (Heinlein Hosting GmbH, Berlin, Germany),
  acting as a processor, which receives the address and the mail.
- **AI assistants for bug reports:** we triage some bug and crash reports
  with AI assistants that a provider runs for us (Anthropic PBC and OpenAI,
  L.L.C., both USA). For that they receive the report with the attachments
  you sent along (such as your text, the screenshot, the game record, the
  technical details and the pseudonym you report under), never your
  password or your encrypted data. They act as processors (Art. 28 GDPR) on
  our instructions only and do not use the reports to train their models.
- **Our administrators** see, for administering accounts, your username,
  display name and tag, where your account came from, when it was created and
  last signed in, whether it has a recovery code, and the gateways you own;
  never your encrypted data, which we cannot read.
- **Gateways:** when you sign in to a gateway, it receives your account ID,
  your display name and tag, and the confirmation that you are signed in, but
  not your password or your encrypted data.
- **Other players:** your contacts and the members of your groups see your
  display name and tag and the messages you send them; your friends also see
  that you are online, and whether you are away, playing or sitting at a
  table, and at which table if you share it, while the Baylee app is
  connected to Central and you did not choose
  "offline" (section 5a). Anyone can see your profile picture, signed in or not, and
  your entry on a leaderboard you enable.
- **Authorities** receive data only where we are legally obliged.

Data is transferred to countries outside the European Economic Area in two
cases only: if you enable notifications, your browser's push provider
(Google, Mozilla or Apple, USA) carries them; and the bug and crash reports
we triage with AI assistants reach their providers (Anthropic and OpenAI,
USA; section 6). In both cases the basis is the European Commission's
adequacy decision on the EU-US Data Privacy Framework, as far as the
provider is certified under it, and otherwise the European Commission's
standard contractual clauses. Gateways run by others can be in any country;
whether you sign in there is your decision.

## 7. How long we keep data

- Account data is kept until you delete your account. We then remove it from
  the running system at once and from backups within 12 months at the latest,
  when the backups are overwritten in rotation. A gateway you registered is
  deleted with your account; players can then no longer sign in to it through
  Baylee Central. If your account moved from the official gateway, that
  gateway keeps its own copy of your old account there, its password removed;
  we delete it on request, and for every account once moving from the
  official gateway ends.
- Native client sessions end 30 minutes after their last use and at most 12 hours after signing in. Browser sessions automatically refresh within a 3-day sliding inactivity window (capped at 30 days after signing in with your password); sign-in tickets expire after seconds.
- Whether you are online and what you are doing exists only while the Baylee
  app is connected to Central, and only in memory. Your choice of how your
  friends see you (online, away, offline) is kept until you change it or
  delete your account.
- The time of your last sign-in is kept while your account exists.
- The record of what our administrators did is kept for three years, then
  deleted automatically.
- A reset code's fingerprint is deleted when the code is used or you change
  your password, and otherwise within a day after it expires: 24 hours after
  it was made for an administrator's code, one hour for a code sent by
  e-mail.
- An e-mail address is kept until you remove it or delete your account. An
  address that waits for its code is kept until you confirm or withdraw it,
  and at most a day after its code expired (15 minutes after it was sent).
- Server logs are deleted after 14 days at the latest.
- Reports and moderation decisions are kept as long as needed to keep them
  traceable, at most three years; decided reports about profile pictures,
  one year.
- A profile picture stays until you replace or remove it, an administrator
  removes it or you delete your account; it is then deleted at once. A
  browser that showed it may keep a copy for up to an hour.
- Challenges, expeditions and leaderboard entries stay until you delete your
  account; you withdraw a leaderboard entry at any time by switching it off.
- What pairing a device leaves is deleted once the pairing is redeemed, at
  the latest after five minutes.
- Bug and crash reports are deleted once the bug is dealt with, after three
  years at the latest.
- Contact form messages are deleted 90 days after they are dealt with, a
  year after they arrived at the latest.
- The anonymous usage count forgets a game 150 seconds after its last sign
  of life.
- Single donation records are deleted after 40 days; before that their
  amounts are added to one total per platform, from which no single
  donation can be read any more.
- The record of which version of our terms you accepted is kept for as long as
  your account exists.

## 8. Your rights

You have the right of access (Art. 15), rectification (Art. 16), erasure
(Art. 17), restriction (Art. 18) and data portability (Art. 20 GDPR). You can
withdraw a consent (crash reports, notifications) at any time for the
future; what happened until then remains lawful (Art. 7(3) GDPR). If you are
under 16, a consent counts only with the agreement of your parents or other
guardians (Art. 8 GDPR). You can
exercise many of your rights yourself: deleting your account, exporting your
data and changing details is in the settings; deleting asks for your
password again, and a deleted account cannot be restored. Otherwise a message
to the contact details in the legal notice is enough.

You may lodge a complaint with a data protection supervisory authority, for
example the one where you live or the one responsible for us.

## 8a. Your right to object

Where we process data on the basis of Art. 6(1)(f) GDPR (the time of your last
sign-in, bug reports, the contact form, the anonymous usage count, reports
and moderation, the record of administration, donation totals, technical
access data), you may object at any time on grounds relating to your
particular situation (Art. 21 GDPR). We then stop, unless we can show
compelling legitimate grounds that override your interests, or the
processing serves to establish, exercise or defend legal claims. An informal
message to the contact details in the legal notice is enough.

## 9. Required data, automated decisions

Without a username and password no account can exist; everything else is
optional. Offline you play without any account. There is no automated
decision-making or profiling within the meaning of Art. 22 GDPR.

## 10. Changes

If this policy changes, we say so on the website and in the client.
