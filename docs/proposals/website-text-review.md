# Website text and legal review: what is left for the owner

*11.11.2026.* A proofreading and legal pass over the site's words
(`crates/central-web/locales/*/site.ftl`) changed 124 messages and added three
sections to the privacy notice the site shows (links to other services, reports
and the usage count from the game, the right to object set apart as Art. 21(4)
GDPR asks). This file holds what needs a decision or a fact only the owner has. On the
owner's word (11.11.2026) the legal texts were then corrected too: the
changes at the end are applied as version 2026-11-11, and the notice on the
site no longer calls itself a draft. Nothing here is legal advice, and the
pass is no evidence of compliance: no lawyer has read the texts.

## Open questions

The reviewer's notes, in German as written. (Closed on 12.11.2026: a reset
code is redeemed on the new site at `/{lang}/reset`, and the FAQ's answer
links to it; `../account-reset.md`, "The website's page".)

- Zwei Datenschutzerklärungen: Die Website zeigt die privacy-*-Texte aus site.ftl (legal.rs), daneben gibt es legal/privacy.de.md/.en.md (Version 2026-11-02, laut legal/README.md 'binding'). Sie widersprechen sich (Serverprotokolle: 'kein Zugriffsprotokoll' gegen '14 Tage Protokolle'; Drittländer; Sprachwahl im Browser gespeichert; Umfang). Welche gilt? Empfehlung: ein einziger Text, aus dem beide Stellen gespeist werden. legal/README.md nennt außerdem Version 2026-10-28, die Datei 2026-11-02.


- Verfahren beim Reset-Code: Wie prüft ein Administrator ohne E-Mail-Adresse, dass die anfragende Person der Account gehört? Die Texte (faq-password-a, Bedingungen §3) versprechen den Reset; das Verfahren sollte feststehen, bevor es öffentlich zugesagt wird. Die neue Website hat keine Seite zum Einlösen eines Reset-Codes (kein ResetStart in crates/central-web); der alte FAQ-Satz 'mit dem Wiederherstellungscode setzt du ein neues' stimmt laut docs/account-reset.md nicht (ein neues Passwort braucht immer den Reset-Code, der Wiederherstellungscode rettet nur die Daten). faq-password-a, code-lead, profile-code-kept und Bedingungen §3 bitte daran messen, was Website und Client wirklich anbieten.

- Kontenlöschung und Nachrichten: chat_messages.sender hat ON DELETE CASCADE, Central löscht also die gesendeten Nachrichten samt Zustellungen bei den Empfängern. Der bisherige Satz 'Nachrichten ... bleiben bei denen, die sie erhalten haben' stimmt nur für Kopien auf Geräten. Bitte bestätigen, was Empfänger nach der Löschung noch sehen, und was mit Gruppen geschieht, deren Eigentümer der gelöschte Account war.

- Datenexport (Art. 15, 20 DSGVO): Der Export (docs/account-deletion.md) enthält keine Kontakte, Blockierungen, Gruppen, Chat-Metadaten und Push-Abonnements; er wurde vor dem Chat entworfen. Soll er erweitert werden? Bis dahin müssen diese Daten auf Anfrage von Hand beauskunftet werden.

- Speicherdauer des Chats: Nachrichten und Metadaten bleiben bis zur Kontolöschung; docs/central-chat.md nennt die automatische Löschfrist eine offene Eigentümer-Entscheidung. Für Art. 5 Abs. 1 lit. e DSGVO (Speicherbegrenzung) wäre eine Frist besser begründbar.

- Serverprotokolle (am 11.11.2026 mit deiner Erlaubnis nur lesend auf dem Live-Server nachgesehen): Caddy führt kein Zugriffsprotokoll, und das Journal ist auf 14 Tage begrenzt (`MaxRetentionSec=14day`). **Aber:** journald reicht alle Meldungen an rsyslog weiter (`ForwardToSyslog=yes`), und `/var/log/syslog` wird wöchentlich mit `rotate 4` gedreht, bleibt also rund fünf Wochen; ebenso `fail2ban.log`. Die Website sagt jetzt wie legal/privacy § 7 „höchstens 14 Tage“. Damit das stimmt, musst du auf dem Server die Rotation kürzen; die Zeilen stehen in docs/operations.md, „Logs and how long they stay“. Ich habe am Server nichts geändert. Den Inhalt von /var/log/syslog konnte ich nicht lesen (keine Rechte), also nicht prüfen, ob dort tatsächlich IP-Adressen stehen.
- Impressum: Das Rätsel bleibt (deine Entscheidung vom 11.11.2026: Crawler sollen die Angaben nicht automatisiert abgreifen). Das Restrisiko, dass ein Gericht „unmittelbar erreichbar“ (§ 5 DDG) enger sieht, ist dir bekannt.
- Service Worker: auf deine Entscheidung vom 11.11.2026 nur noch nach Anmeldung, in der installierten App oder bei eingeschalteten Benachrichtigungen; wer nur liest, bekommt nichts gespeichert.

- netcup: Ist ein Auftragsverarbeitungsvertrag (Art. 28 DSGVO) abgeschlossen? Die Erklärung nennt netcup als Auftragsverarbeiter. Gibt es weitere Dienstleister (Backup-Speicher, DNS/CDN, E-Mail-Postfach für Antworten auf das Kontaktformular und die Impressum-Adresse)? Antworten per E-Mail laufen über einen Mailanbieter, der nirgends genannt ist.

- Wer liest Kontaktnachrichten und Berichte? privacy-contact sagt 'nur die im Impressum genannte Person', die Benachrichtigungskategorie 'Feedback-Meldungen (Administration)' und legal/privacy §6 sprechen von 'Administratoren'. Gibt es weitere Administratoren? Dann stimmt der Satz nicht, und sie brauchen eine Vertraulichkeitsverpflichtung.

- Web Push und Drittland: Google, Mozilla und Apple erhalten Endpunkt, verschlüsselte Hinweise und Verbindungsdaten in den USA. Auf welche Grundlage stützt sich das (Angemessenheitsbeschluss/Data Privacy Framework je Anbieter, oder der Push-Dienst als vom Nutzer gewählter eigener Dienst)? Ich habe nur die Tatsache benannt, keinen Mechanismus behauptet.


- Zuständige Aufsichtsbehörde: Die LDI NRW ist nur richtig, wenn der Verantwortliche in Nordrhein-Westfalen wohnt. Bitte bestätigen.

- Mindestalter 16: Die Bedingungen verlangen bei Minderjährigen zusätzlich die Zustimmung der Eltern; das Formular fragt nur 'mindestens 16'. Reicht dir das? Für die Push-Einwilligung passt 16 zu Art. 8 DSGVO.

- Neue Fassung der Bedingungen: Jede Änderung an legal/terms.*.md ist laut legal/README.md eine neue Version, die jeder Account neu annehmen muss. Die vorgeschlagenen Änderungen (§3 Reset, §7.1 Löschen, §8 Fan-Hinweis, ein Komma) am besten bündeln. Auch jede Änderung an legal/privacy.*.md braucht in beiden Sprachen eine neue Version und ein neues Datum im Kopf. Die deutschen Bedingungen sagen durchgehend 'Konto', die Website laut Kopfkommentar 'Account'; ich habe die Rechtstexte nicht umgestellt.

- Streitbeilegungshinweis (impressum-dispute, Bedingungen §11.2): § 36 VSBG trifft Unternehmer; auf einer ausdrücklich privaten, nicht kommerziellen Seite ist er überflüssig und kann als Indiz für geschäftsmäßiges Handeln gelesen werden. Behalten oder streichen? Ebenso 'Verantwortlich für den Inhalt' (§ 18 Abs. 2 MStV gilt nur für journalistisch-redaktionelle Angebote).

- Meldeweg (Bedingungen §6): 'Meldefunktion im Client oder auf der Website' - gibt es auf der Website eine Meldefunktion für Namen, Gruppen oder Nachrichten? Ich habe keine gefunden; sonst bleibt nur das Kontaktformular.

- Zusagen ohne Beleg, die ich nicht geändert habe (Altseiten-Texte, Eigentümer 11.11.2026): 'Vollständige Regel-Engine' / 'Eine Engine, die jede Regel durchsetzt' neben 'teilweise umgesetzt'; 'ein Fehler stört niemals andere Tische'; 'Jede Partie wird lückenlos aufgezeichnet'; title-home (DE) sagt 'Bau deine Decks. Hol deine Freunde. Einfach spielen.', hero-title (DE) 'Decks bauen. Freunde einladen. Einfach spielen.', im Englischen sind beide gleich - Absicht? 'bleibt immer quelloffen und kostenlos ... Nicht jetzt, nicht später'; 'Anonyme Nutzungszahl' (die IP-Adresse erreicht den Server trotzdem; besser 'ohne Account und Gerätekennung'); 'API-Schlüssel bleiben geschützt'. Bitte entscheiden, ob sie so bleiben.

- Fan Content Policy: Der deutsche Hinweis im Fuß entspricht dem offiziellen Wortlaut, soweit ich ihn kenne; bitte einmal gegen die deutsche Richtlinienseite prüfen. Die Richtlinie verlangt freien Zugang ohne Zahlung, Abo oder E-Mail-Registrierung; ein Pflicht-Account für Online-Tische (ohne E-Mail) und die Verwendung von Kartenbildern, Manasymbolen und Regeltexten in einem Spiel-Client liegen im Graubereich der Richtlinie. Das ist eine Risikoentscheidung des Eigentümers, kein Textproblem.

- Discord und WhatsApp: Wer verwaltet die beiden Communities? Als Administrator sieht der Betreiber dort Namen und bei WhatsApp Telefonnummern; ich habe das im neuen Abschnitt privacy-links nur allgemein erwähnt. Soll der Abschnitt ausdrücklich sagen, dass in der WhatsApp-Community die Telefonnummer für andere sichtbar sein kann? Das konnte ich nicht belegen und habe es weggelassen.

- Handle oder Kürzel: Profil und Code-Seite sagten 'Handle', Chat und Kontakte 'Kürzel'. Ich habe auf 'Handle' vereinheitlicht (Kopfkommentar: englische Spielerbegriffe bleiben englisch). Wenn dir 'Kürzel' lieber ist, dann umgekehrt in code-handle, account-profile-lead und profile-handle.

- Baylees Erinnerungsseite (baylee-*) habe ich wie vorgegeben nicht angefasst; mir ist dort nichts aufgefallen, was eine Korrektur bräuchte. Im englischen Teil der Altseiten-Texte bleiben Title Case ('AI Opponents & Language Models', 'Self-hosting & Central Integration'), 'House AI'/'house AI' und Gedankenstriche ohne Leerzeichen uneinheitlich; in gw-lead (DE) ist 'Baylee betreibt das als offiziell markierte; die anderen betreiben andere für ihre Runde' schwer lesbar. Soll ich diese Altseiten-Texte auch glätten?

## Changes to `legal/*.md` (applied on 11.11.2026 as version 2026-11-11)

`legal/terms.*.md` is compiled into `central-service` and versioned: a change
is a new version every account accepts again (`legal/README.md`,
`docs/terms.md`). `legal/privacy.*.md` and the notice on the site disagree in
places; which one binds is the owner's call.

### `legal/terms.de.md`

Überholt: den Rücksetzcode eines Administrators gibt es (docs/account-reset.md, Datenschutzerklärung §3). Neue Fassung der Bedingungen nötig.

Old:

```text
- Vergisst du dein Passwort, kannst du es noch nicht zurücksetzen; das
  Zurücksetzen per E-Mail oder durch uns folgt.
```

New:

```text
- Vergisst du dein Passwort, kann dir ein Administrator einen Rücksetzcode
  geben, mit dem du ein neues setzt; das Zurücksetzen per E-Mail folgt.
```

### `legal/terms.en.md`

Outdated: the administrator's reset code exists (docs/account-reset.md, privacy policy §3). Needs a new version of the terms.

Old:

```text
- If you forget your password, you cannot reset it yet; a reset by e-mail or
  with our help will follow.
```

New:

```text
- If you forget your password, an administrator can give you a reset code to
  set a new one; a reset by e-mail will follow.
```

### `legal/terms.de.md`

Überholt: die Website löscht das Konto selbst (docs/account-deletion.md „As built“, Seite „Datenschutz und Daten“); die Datenschutzerklärung §8 sagt es bereits. Ob der Client es schon kann (M6), bitte prüfen.

Old:

```text
1. Du kannst dein Konto jederzeit löschen lassen: Schreib uns über die im
   [Impressum](https://baylee.app/impressum) genannten Kontaktdaten, dann
   löschen wir es unverzüglich. Das Löschen in den Einstellungen des Clients
   und auf der Website folgt.
```

New:

```text
1. Du kannst dein Konto jederzeit löschen: in den Einstellungen deines
   Kontos auf der Website, oder schreib uns über die im
   [Impressum](https://baylee.app/impressum) genannten Kontaktdaten, dann
   löschen wir es unverzüglich. Das Löschen in den Einstellungen des Clients
   folgt.
```

### `legal/terms.en.md`

Outdated: the website deletes the account itself (docs/account-deletion.md "As built"); the privacy policy §8 already says so. Whether the client can do it yet (M6) needs checking.

Old:

```text
1. You can have your account deleted at any time: write to us at the contact
   details in the [legal notice](https://baylee.app/impressum) and we delete it
   without delay. Deleting it in the client's settings and on the website will
   follow.
```

New:

```text
1. You can delete your account at any time: in your account's settings on
   the website, or write to us at the contact details in the
   [legal notice](https://baylee.app/impressum) and we delete it without
   delay. Deleting it in the client's settings will follow.
```

### `legal/terms.de.md`

Die Richtlinie verlangt den Hinweis im vorgegebenen Wortlaut; die Website (fan-content) benutzt ihn, die Bedingungen wichen ab. Wortlaut bitte gegen company.wizards.com/de/legal/fancontentpolicy prüfen.

Old:

```text
Baylee ist ein inoffizieller Fan-Inhalt im Sinne der Fan Content Policy von
Wizards of the Coast. Es ist nicht von Wizards of the Coast genehmigt oder
unterstützt. Teile der verwendeten Inhalte sind Eigentum von Wizards of the
Coast. © Wizards of the Coast LLC.
```

New:

```text
Baylee ist ein inoffizieller Fan-Inhalt im Rahmen der Richtlinie für
Fan-Inhalte. Nicht von Wizards gefördert/gesponsert. Bestandteile des
enthaltenen Materials sind Eigentum von Wizards of the Coast. ©Wizards of
the Coast LLC.
```

### `legal/terms.de.md`

Kein Komma vor „und“ zwischen gleichrangigen dass-Sätzen.

Old:

```text
   du es betreiben darfst, dass seine Adresse dir gehört, und dass du für
```

New:

```text
   du es betreiben darfst, dass seine Adresse dir gehört und dass du für
```

### `legal/privacy.de.md`

Widerspruch im selben Text: der Abschnitt zu Benachrichtigungen nennt Google, Mozilla und Apple als Empfänger (Art. 13 Abs. 1 lit. f DSGVO).

Old:

```text
Eine Übermittlung in Länder außerhalb des Europäischen Wirtschaftsraums
findet nicht statt.
```

New:

```text
Eine Übermittlung in Länder außerhalb des Europäischen Wirtschaftsraums
findet nur statt, wenn du Benachrichtigungen aktivierst: Dann überbringt
sie der Push-Anbieter deines Browsers (Google, Mozilla oder Apple, USA).
Gateways anderer Betreiber können in jedem Land stehen; ob du dich dort
anmeldest, entscheidest du.
```

### `legal/privacy.en.md`

Contradiction within the text: the notifications section names Google, Mozilla and Apple as recipients (Art. 13(1)(f) GDPR).

Old:

```text
No data is transferred to countries outside the European Economic Area.
```

New:

```text
Data is transferred to countries outside the European Economic Area only if
you enable notifications: your browser's push provider (Google, Mozilla or
Apple, USA) then carries them. Gateways run by others can be in any country;
whether you sign in there is your decision.
```

### `legal/privacy.de.md`

Änderungsprotokoll-Sprache („diese Änderung führt … ein“) gehört nicht in eine Erklärung; Speicherdauer klar sagen (Art. 13 Abs. 2 lit. a; docs/central-chat.md Z. 133–136, chat_messages ON DELETE CASCADE).

Old:

```text
Die aktuellen Grenzen
pro Konto/Gerät lehnen neue Nachrichten bei vollem Speicher ab; diese Änderung
führt keine automatische Löschung des Verlaufs ein.
```

New:

```text
Je Absender und
je Gerät speichert Central höchstens 5.000 Nachrichten; danach werden neue
abgelehnt. Der Verlauf wird nicht automatisch gelöscht; er bleibt, bis du
dein Konto löschst.
```

### `legal/privacy.en.md`

Changelog wording ("introduced by this change") does not belong in a notice; state the retention plainly (Art. 13(2)(a); docs/central-chat.md l. 133–136).

Old:

```text
Current per-account/device message limits reject new sends when full; no automatic
history deletion is introduced by this change.
```

New:

```text
Central stores at most 5,000 messages per sender and per device; after that,
new ones are refused. History is not deleted automatically; it stays until you
delete your account.
```

### `legal/privacy.de.md`

Der Abschnitt steht unnummeriert zwischen 5 und 6; „optional“ trifft auf den Chat-Teil nicht zu (Kontakte und Gruppen werden gespeichert, sobald man sie nutzt).

Old:

```text
## Optionale Chat- und Benachrichtigungsfunktionen
```

New:

```text
## 5a. Chat und Benachrichtigungen
```

### `legal/privacy.en.md`

The section stands unnumbered between 5 and 6; same change as the German text.

Old:

```text
## Optional chat and notification features
```

New:

```text
## 5a. Chat and notifications
```

### `legal/privacy.de.md`

Dem Abschnitt fehlten Rechtsgrundlage und Hinweis auf den Widerruf (Art. 13 Abs. 1 lit. c, Abs. 2 lit. c DSGVO); der Chat-Absatz davor beruht auf lit. b, was ebenfalls fehlt (siehe offene Fragen).

Old:

```text
Wenn du Benachrichtigungen aktivierst, speichert Central den Push-Endpunkt dieses
```

New:

```text
Wenn du Benachrichtigungen aktivierst (Art. 6 Abs. 1 lit. a DSGVO, § 25 Abs. 1
TDDDG: deine Einwilligung, im Profil jederzeit für die Zukunft widerrufbar),
speichert Central den Push-Endpunkt dieses
```

### `legal/privacy.en.md`

The section lacked its legal basis and the note on withdrawal (Art. 13(1)(c), (2)(c) GDPR).

Old:

```text
If you enable notifications, Central stores this browser's push endpoint, public key,
```

New:

```text
If you enable notifications (Art. 6(1)(a) GDPR, section 25(1) TDDDG: your
consent, which you can withdraw for the future at any time in your profile),
Central stores this browser's push endpoint, public key,
```

### `legal/privacy.de.md`

Rechtsgrundlage fehlte; Kontaktanfragen werden ebenfalls gespeichert (docs/central-social.md).

Old:

```text
Central speichert persönliche Kontakte, Blockierungen und Gruppenmitgliedschaften,
```

New:

```text
Für den Chat (Art. 6 Abs. 1 lit. b DSGVO) speichert Central persönliche
Kontakte, Kontaktanfragen, Blockierungen und Gruppenmitgliedschaften,
```

### `legal/privacy.en.md`

Legal basis was missing; contact requests are stored too (docs/central-social.md).

Old:

```text
Central stores personal contacts, blocks and group memberships, device public keys,
```

New:

```text
For the chat (Art. 6(1)(b) GDPR), Central stores personal contacts, contact
requests, blocks and group memberships, device public keys,
```
