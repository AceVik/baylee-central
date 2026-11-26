# Legal texts of Baylee Central

**Status: the terms and the privacy policy are at version 2026-11-25** (ranked play added on 26.11.2026, below; the
minimum age, below; before it 2026-11-19). The
privacy policy's presence text (§ 5a's new paragraph, § 7's new point) was
reviewed by the owner on 19.11.2026; the status his later decision of the same
day added changed that paragraph and point again, and he approved the German
status sentences too, the same day, through the PM ("Ja, passt"; § 5a's
changed sentence and new paragraph, § 7's retention, the terms' § 1, § 6 named
as saying the same in one sentence; told that the website's nine other
languages are machine translations no speaker has read). The version's other
changes await his review (see the last paragraphs below; before
it, the terms at 2026-11-11 and the privacy policy at 2026-11-18. On 11.11.2026
the owner asked for the texts to be corrected: an administrator's
reset code, deleting the account in its settings on the website, the Fan
Content notice in its given words, the chat's and the notifications' legal
bases, and the push services as recipients outside the EEA; before that the
terms at 2026-10-22 and the privacy policy at 2026-11-02). Being a new
version of the terms, every account is asked to accept it again. What is
still the owner's to decide or confirm stands in
[the text review](../docs/proposals/website-text-review.md). The owner published the drafts on
21.10.2026 with registration on `baylee.app`, after only the statements the
deployed service did not yet bear out were corrected: deletion is on request
until it is in the settings, there is no password reset or e-mail yet, and the
recovery code wraps the key container. What the
[roadmap](../docs/roadmap-accounts-web.md) still adds (e-mail, 2FA, display
name, profile picture, bug reports in Central) is marked as to follow. The
owner's full review, and ideally a lawyer's, are still to come; each change is
a new version.

On 12.11.2026 the owner had e-mail built (an optional address, a reset
code by mail) and decided that version 2026-11-11, not yet published,
carries it: the terms' two sentences on e-mail and the privacy policy's
e-mail lines changed (`updated: 2026-11-12`), the version stayed. Nobody
outside local tests has accepted 2026-11-11 before the change. The same
day the privacy policy's e-mail line gained the notices to a confirmed
address (a reset finished, the password changed, an administrator's
reset code), which Central sends since then; the version stayed.

On 18.11.2026 the owner decided that the donations Baylee Central collects
(`docs/operations.md` §"Donations") make a new version of the privacy
policy, 2026-11-18: § 3 names what a donation leaves behind and why, § 7
how long a single record is kept (40 days, then one total per platform).
The terms are unchanged and stay at 2026-11-11, so nobody is asked to
accept anything again; a new privacy policy is shown, not accepted.

On 19.11.2026 the owner decided that Central serves the Baylee client's
push socket with presence ([push socket](../docs/push-socket.md)). That
makes the privacy policy's version 2026-11-19: § 5a says what the socket
carries and that accepted friends see when an account is online, held in
memory only, § 7 how long (while the client is connected). Written by the
implementing agent in the same pull request; the owner read § 5a's
paragraph and § 7's point in German, with "erkennbar an deinem Tag (dem
Kürzel hinter deinem Anzeigenamen)", and answered "Ja, passt" (19.11.2026,
relayed by the PM). The English follows the German.

The same day the owner asked for all of Central's legal texts to be brought
in line, and chose the contract (Art. 6(1)(b) GDPR) as presence's legal
basis, with the terms describing it. So both documents are at 2026-11-19:

- **Terms:** § 1 names what Central offers today beside the account, the
  storage and the directory: personal chats, contacts and groups, friends
  seeing that one is online, a profile picture, the daily and weekly
  challenges and Tower expeditions with opt-in leaderboards, notifications;
  § 6 says that people, not machines, decide on reports and moderation, and
  that an objection is decided again (Art. 14 and 17 DSA). Being a new
  version of the terms, every account is asked to accept it again.
- **Privacy policy:** besides presence (§ 5a, § 7), the profile picture
  (it said "will follow"), the challenges and expeditions, pairing a device,
  the other players as recipients, the legal basis of the push providers'
  transfer to the USA (the EU-US Data Privacy Framework, as far as a provider
  is certified; whether each is, is the owner's to confirm), withdrawing a
  consent, and the right to object in a part of its own (Art. 21(4) GDPR).
- **The website's privacy page** (`crates/central-web/locales/*/site.ftl`)
  says the same about presence, the other players and the transfer, in its
  eleven languages.

| file | what |
| --- | --- |
| `terms.de.md` | Nutzungsbedingungen (binding) |
| `terms.en.md` | terms of use (translation) |
| `privacy.de.md` | Datenschutzerklärung (binding) |
| `privacy.en.md` | privacy policy (translation) |

## Rules for these files

- German is binding; the English file is a translation and says so.
- Each file starts with `<!-- version: … -->` and `<!-- updated: … -->`.
  Both languages of one document carry the same version. The version `draft`
  means not in force, and it is the only thing that holds a text out of
  force (`docs/terms.md`): publishing replaces it by a real version in a
  reviewed commit. A new version of the terms needs every account's
  acceptance again (accept, later = signed out, or delete the account);
  nobody continues under an older one. The privacy policy is information,
  not a contract: a new version is shown, not accepted.
- The text uses only the Markdown subset the Baylee client renders: `#`/`##`
  headings, paragraphs, `**bold**`, `*italic*`, lists, rules and links. No
  tables, no HTML, no images.
- The links name the website's pages: `https://baylee.app/impressum`,
  `/agb` and `/datenschutz` (German), `/terms` and `/privacy` (English).
  The Impressum's data is the operator's and never in this repository.
- What a statement claims has to be true of the deployed service: retention
  times, recipients and legal bases are checked against the code and the
  server before a version is published (for example the 30-minute session
  limit in `central-migration`, the log retention on the host).

Each gateway has terms of its own, edited in its admin area. The Baylee
repository ships a default text and privacy notice any operator may adapt.

The status (the owner's decision of 19.11.2026, relayed by the PM; in short:
a person sets itself online, AFK or offline, a busy or in-game status is set
by itself, and at a table; [push socket](../docs/push-socket.md) §"Status")
is in the same unpublished version 2026-11-19: § 5a says what
friends see (away, playing, at a table; never what, and where only for a
shared table, below), that offline
hides the account, that the choice is stored and the rest is not; § 6 names
the friends among the recipients with what they see; § 7 how long the choice
is kept; the terms' § 1 names the status and appearing
offline. The website's privacy page says the same. The owner approved the
German status sentences on 19.11.2026 through the PM ("Ja, passt"); the
English texts and the website's other languages follow them.

**Awaiting the owner's review: the shared table.** Later on 19.11.2026 the
owner decided that a friend sees which table a friend sits at through
Central ([push socket](../docs/push-socket.md) §"Tables"). That made § 5a's
"nicht aber wo oder was du spielst" untrue for a player who shares the
table, so the implementing agent changed the privacy policy in the same pull
request: § 5a's first paragraph now says that friends see where only if the
table is shared, a new paragraph after the status says what the app tells
Central then (the gateway's address, the game's identifier, the table's
name, its seats and how many are taken; never for a table with a password),
to whom Central passes it and that it is kept only in memory, and § 6 adds
"und an welchem Tisch, wenn du ihn teilst". § 7 needs no change (what you are
doing exists only while connected). The terms are unchanged: § 1 says
nothing of where. The version stays 2026-11-19, which is not published yet;
should it be published before the owner reads these sentences, they make a
new version of the privacy policy (shown, not accepted). The English follows
the German; the website's privacy page (`privacy-chat`, eleven languages,
the nine beside German and English machine translations) says the same.

**Version 2026-11-25: the minimum age is 13.** On 25.11.2026 the owner
decided that Central, like the official gateway, takes players from 13, the
age Magic: The Gathering itself names (Baylee #316). The terms' § 2 says 13
and keeps the parents' consent for anyone not yet of age (§§ 106 ff. BGB);
being a new version of the terms, every account is asked to accept it again.
The privacy policy's date-of-birth check says 13, and § 8 adds that a
consent (crash reports, notifications) given under 16 counts only with the
parents' agreement (Art. 8 GDPR, 16 in Germany): the account itself rests on
the contract (Art. 6(1)(b)), which Art. 8 does not govern. Registration's
age box, the date picker's warning and the profile's birth-date check
(`central-postgres` `set_profile_details`) say and check 13; the website's
privacy page says the same in its eleven languages, the nine beside German
and English machine translations. Written by the implementing agent; the
owner's review of these sentences is still to come.

**Ranked play, in the same version 2026-11-25 (`updated: 2026-11-26`).**
Ranked play ([global ranked play](../docs/ranked.md)) was in neither text,
and the privacy policy said Central sees nothing of what is played on a
gateway (Baylee #316). Version 2026-11-25 is merged but not yet published,
so, as on 12.11.2026, the version stays and only `updated` changes (an
acceptance holds the text's digest too, so anyone who accepted the earlier
text is asked again in any case): the
privacy policy's § 3 gains an item for ranked play (the queue, the pairing,
the decks as card fingerprints and counts, ratings, placement, league,
badges and cosmetics; the game's opening, decks, decisions and clocks while
it runs; the relay that keeps the player's address from the game's
gateway; the public boards), the sentence on gateways names ranked play and
challenge results as the exception, § 6 adds what a ranked game's gateways
and the opponent receive and that boards are public, § 7 how long (until
the account is deleted; a game's private replay material after it ends,
usually within minutes, by the daily sweep at the latest), § 9 that the
rating is computed by a program without a legal or similarly significant
effect. The terms' § 1 names ranked play with its public boards. The boards
list every player with a rated game in the season by display name and tag,
whatever the public-profile switch says, as the owner decided on 26.11.2026
("Ja, öffentlich lassen") (`central-postgres`
`ranked::boards::standings`); the texts and the profile switch's hint say
so, and the basis is the contract (Art. 6(1)(b)), since a public ladder is
what ranked play offers. The website's privacy page has a part of its own
for it (`privacy-ranked`, eleven languages, the nine beside German and
English machine translations). Ranked data is not yet in the account's data
export. Written by the implementing agent; the owner's review is still to
come.
