# Die Worte von baylee.app auf Deutsch. Englisch bestimmt, welche Nachrichten
# es gibt und welche Argumente sie nehmen. Der Leser ist „du“; Begriffe, die
# deutsche Spieler englisch sagen, bleiben englisch (Account, Gateway,
# Release, Deck); Anführungszeichen „…“, Gedankenstrich „ – “.

## Der Rahmen jeder Seite

site-name = Baylee
skip = Zum Inhalt springen
nav-label = Hauptnavigation
nav-home = Start
nav-download = Download
nav-gateways = Gateways
nav-faq = Fragen
nav-contact = Kontakt
nav-more = Mehr
nav-close = Menü schließen
nav-sign-in = Anmelden
nav-account = Dein Account
lang-label = Sprache
lang-switch = English
theme-label = Farbschema
theme-system = Farben: wie dein Gerät
theme-light = Farben: hell
theme-dark = Farben: dunkel

foot-label = Über diese Seite
foot-play = Spielen
foot-about = Über Baylee
foot-legal = Rechtliches
foot-repository = Quellcode auf GitHub
foot-releases = Alle Releases
foot-issues = Fehler melden
foot-discord = Discord-Community
foot-central-source = Quellcode dieser Seite
foot-dedication = Für Baylee. Für immer im Sonnenlicht.
fan-content = Baylee ist ein inoffizieller Fan-Inhalt im Rahmen der <policy>Richtlinie für Fan-Inhalte</policy>. Nicht von Wizards gefördert/gesponsert. Bestandteile des enthaltenen Materials sind Eigentum von Wizards of the Coast. ©Wizards of the Coast LLC.
fan-content-short = Inoffizieller Fan-Inhalt · <more>was das heißt</more>
foot-colophon = Schriften: Alegreya Sans und Faustina (SIL Open Font License), Icons: Lucide (ISC), alles von dieser Seite geladen. <notices>Hinweise</notices>

## Titel und Beschreibungen

title-suffix = Baylee
title-home = Baylee – Bau deine Decks. Hol deine Freunde. Einfach spielen.
desc-home = Baylee ist ein freies Open-Source-Fanspiel zu Magic: The Gathering für Linux, Windows und macOS: eine Regel-Engine, die die Karten kennt, Tische für bis zu acht Spieler und Gegner, wenn sonst niemand da ist.
title-download = Baylee herunterladen
desc-download = Lade Baylees Client für Windows, macOS und Linux herunter: Installer und Archive des neuesten Release, mit Prüfsummen und Signaturen.
title-gateways = Gateways: wo Baylee online gespielt wird
desc-gateways = Das öffentliche Verzeichnis der Baylee-Gateways, der Server, auf denen Online-Tische laufen, und wie du dein eigenes betreibst.
title-faq = Fragen und Antworten
desc-faq = Was Baylee ist, was es kostet, wie Online-Spiel und Accounts funktionieren und was mit deinen Daten geschieht.
title-contact = Kontakt
desc-contact = Schreib den Leuten hinter Baylee oder melde einen Fehler.
title-impressum = Impressum
desc-impressum = Wer baylee.app betreibt (§ 5 DDG).
title-privacy = Datenschutzerklärung
desc-privacy = Was baylee.app und Baylee Central über dich verarbeiten, wozu, auf welcher Rechtsgrundlage und wie lange.
title-terms = Nutzungsbedingungen
desc-terms = Die Nutzungsbedingungen von Baylee Central, dem Account-Dienst hinter baylee.app.
title-fan-content = Inoffizieller Fan-Inhalt
desc-fan-content = Baylee ist ein Fanprojekt im Rahmen der Richtlinie für Fan-Inhalte von Wizards of the Coast: was das heißt und was nicht.
title-not-found = Seite nicht gefunden
title-gone = Diese Seite gibt es nicht mehr
crumbs-label = Du bist hier
breadcrumb-home = Start

## Start

hero-kicker = Frei · Open Source · Inoffizieller Fan-Inhalt
hero-title = Bau deine Decks. Hol deine Freunde. Einfach spielen.
hero-lead = Ein umfassender Deckbuilder, Partien für bis zu acht Spieler – im Duell, jeder gegen jeden oder im Team – und eine Regel-Engine, die jeden Schritt prüft. So bleibt jeder Zug fair, ohne dass jemand etwas nachschlagen muss.
hero-offline = Solopartien und der Deckbuilder funktionieren offline, ohne Account. Online-Tische laufen auf Gateways; <host>du kannst dein eigenes betreiben</host>.
hero-account = Online spielen? <register>Leg einen Account an</register> oder <signin>melde dich an</signin>.
hero-memorial = Erinnerung an Baylee: ihr Fotoalbum öffnen
hero-other = Andere Systeme

status-label = Stand des Projekts
status-release = Release
status-released = erschienen am { $date }
status-ci-pass = Tests auf main bestanden
status-ci-fail = Tests auf main fehlgeschlagen
status-ci-unknown = Teststand unbekannt
status-commit = Letzte Änderung
status-gateways = { $online ->
    [one] { $online } Gateway online
   *[other] { $online } Gateways online
}
status-stale = Stand { $time }; GitHub hat seitdem nicht geantwortet.

cards-eyebrow = Die Karten
cards-title = Wie viele Karten Baylee kennt
cards-lead = Baylee setzt die Karten von Magic eine nach der anderen um, jede gegen ihren gedruckten Text geprüft. Das sind die Zahlen des neuesten Release.
cards-implemented = Vollständig umgesetzt
cards-implemented-help = Jede Regel der Karte funktioniert und ist getestet.
cards-generated = Generiert
cards-generated-help = Von Baylees Generator aus dem Regeltext der Karte umgesetzt, aber noch nicht von Hand getestet. Sie sind spielbar; in seltenen Sonderfällen kann eine sich anders verhalten.
cards-partial = Teilweise umgesetzt
cards-partial-help = Die Karte ist spielbar, aber eine ihrer Fähigkeiten oder ein Randfall ihrer Regeln fehlt noch.
cards-total = Karten insgesamt
cards-total-help = Alle Karten, die es in Magic gibt, wie Scryfall sie führt; die drei Zahlen davor sind die, die Baylee spielen kann.
cards-help = Was das bedeutet
cards-share = { $playable } von { $total } Karten sind heute spielbar.
cards-as-of = Release { $tag }, gezählt am { $date }.
cards-missing = Die Kartenzahlen konnten gerade nicht gelesen werden.

screens-eyebrow = Ein Blick auf den Tisch
screens-title = So sieht es aus
screens-credit = Kartenbilder in den Screenshots: Scryfall · © Wizards of the Coast.
shot-duel-alt = Ein Duell gegen die Haus-KI: sechs Kreaturen greifen an, drei werden geblockt
shot-duel = <b>Taktisches Duell</b> gegen die Haus-KI, mit klaren Kampflinien für jede Zuteilung.
shot-log-alt = Ein Duell mit offenem Spielprotokoll, drei Zauber auf dem Stapel
shot-log = <b>Das Spielprotokoll</b> hält jeden Schritt fest wie ein Chat; ein Kartenname öffnet ihre Vorschau.
shot-ring-alt = Ein Tisch mit vier Plätzen im Kreis
shot-ring = <b>Tische mit bis zu acht Plätzen</b> ordnen sich um das Spielfeld.
shot-lobby-alt = Die Lobby mit der Nachricht einer Freundin und ihrer Tischeinladung
shot-lobby = <b>Die Lobby:</b> offene Tische, Freunde, Einladungen und ein Ende-zu-Ende-verschlüsselter Chat.
shot-room-alt = Ein Raum, in dem drei leere Stühle an die Haus-KI gehen
shot-room = <b>Plätze für alle:</b> Menschen, die Haus-KI in fünf Stufen oder ein Sprachmodell.
shot-builder-alt = Der Deckbuilder filtert den Pool nach einem Engel
shot-builder = <b>Der Deckbuilder</b> durchsucht den ganzen Pool, während du tippst.

about-eyebrow = Was es ist
about-title = Eine Regel-Engine und ein klarer Tisch
about-engine-title = Eine Engine, die jede Regel kennt
about-engine = Priorität, der Stapel, Schichten, Ersatzeffekte, Auslöser, zustandsbasierte Aktionen und Kampf: Die Engine, in Rust geschrieben, wendet das ganze Regelwerk an und errechnet bei jedem Schritt genau die erlaubten Entscheidungen.
about-table-title = Ein ruhiger 3D-Tisch
about-table = Spielfeld, Hände, Stapel, Exil und Friedhöfe auf einem aufgeräumten Tisch, mit klaren Kampflinien und sofortiger Kartenvorschau. Aus der Lobby wechselst du zwischen Deckbuilder, Solopartien und Online-Tischen.
about-ai-title = Gegner, wenn niemand da ist
about-ai = Die Haus-KI spielt in fünf Stufen, von Anfänger bis Experte, offline und an Online-Tischen, und sieht nur, was ein Spieler sehen würde. Auch ein Sprachmodell kann Platz nehmen, mit deinem eigenen Schlüssel oder über ein Gateway, das eines anbietet.

gw-eyebrow = Online spielen
gw-title = Gateways
gw-lead = Online-Tische laufen auf Gateways: Servern, die Baylee Central aufführt, nachdem ihr Betreiber geprüft wurde. Baylees eigenes steht vorn.
gw-all = Alle Gateways
gw-host = Eigenes betreiben

open-eyebrow = Open Source
open-title = Frei, und das bleibt so
open-lead = Keine Bezahlschranke, keine bezahlten Funktionen, keine Werbung – nicht jetzt und nicht später. Der Code des Spiels steht unter der GNU Affero General Public License; auch der Code von Baylee Central ist öffentlich, damit jeder nachlesen kann, was mit seinen Daten geschieht.
open-repository = Das Spiel auf GitHub
open-central = Baylee Central auf GitHub

## Die Erinnerung

memorial-label = Baylees Fotoalbum und Geschichte
memorial-close = Album schließen

## Download

dl-eyebrow = Download
dl-title = Baylee herunterladen
dl-lead = Baylee läuft unter Windows, macOS und Linux. Jede Datei kommt direkt aus dem Release auf GitHub.
dl-release = { $tag }, erschienen am { $date }
dl-prerelease = Beta
dl-for = Download für { $system }
dl-size = { $size } MB
dl-checksum = SHA-256
dl-signature = Signatur
dl-notes = Release Notes
dl-all = Alle Dateien dieses Release
dl-unknown = Wir konnten dein System an diesem Browser nicht erkennen; wähle deins unten aus.
dl-mobile = Für Handys und Tablets gibt es noch keine Version. Baylee läuft auf einem Computer.
dl-mac-unsure = Dieser Browser verrät nicht, ob dein Mac einen Apple-Chip oder einen von Intel hat. Die Datei oben ist für Apple-Chips (M1 und neuer); ein Intel-Mac findet seine Datei unten.
dl-windows-unsure = Auf einem Windows-PC mit ARM-Prozessor (etwa einem Snapdragon-Laptop) nimm unten die ARM-Datei.
dl-linux-unsure = Dein Browser nennt deinen Prozessor nicht; angenommen ist x86-64. Die ARM-Datei steht unten.
dl-missing = Dieses Release hat keine Datei für dein System. Ältere Releases findest du auf der <releases>Release-Seite</releases>.
dl-none = GitHub hat gerade nicht geantwortet. Alle Releases findest du auf der <releases>Release-Seite</releases>.
dl-yours = Dein System
dl-installer = Installer
dl-deb = Debian-Paket
dl-archive = Archiv
dl-fallback = Archiv für ältere Prozessoren
dl-fallback-help = Für x86-64-Prozessoren ohne AVX2 (vor etwa 2013). Baylees Haupt-Builds brauchen AVX2 und starten ohne nicht.
plat-windows-x64 = Windows
plat-windows-x64-sub = die meisten PCs: 64-Bit Intel oder AMD
plat-windows-arm64 = Windows auf ARM
plat-windows-arm64-sub = Snapdragon-Laptops und ähnliche
plat-mac-arm64 = macOS
plat-mac-arm64-sub = Apple-Chips: M1 und neuer
plat-mac-x64 = macOS auf Intel
plat-mac-x64-sub = Macs mit Intel-Prozessor
plat-linux-x64 = Linux
plat-linux-x64-sub = x86-64
plat-linux-arm64 = Linux auf ARM
plat-linux-arm64-sub = ARM64 / aarch64
plat-none = Dieses Release hat keine Datei für dieses System.
install-title = Erster Start und was es braucht
install-first-title = Erster Start
install-first = Die Builds sind nicht codesigniert, weil Signieren Geld kostet; Windows und macOS warnen deshalb beim ersten Mal. Unter Windows wähle <i>Weitere Informationen</i> und dann <i>Trotzdem ausführen</i>. Unter macOS öffne <i>Systemeinstellungen → Datenschutz & Sicherheit</i> und wähle <i>Trotzdem öffnen</i>.
install-check-title = Einen Download prüfen
install-check = Neben jeder Datei liegt eine <code>.sha256</code> mit ihrer Prüfsumme; die Archive tragen außerdem eine <code>.sig</code>, die Ed25519-Signatur, die der Client prüft, bevor er sich selbst aktualisiert. Unter Linux prüft <code>sha256sum -c</code> eine Datei, unter macOS <code>shasum -a 256</code>, unter Windows <code>certutil -hashfile</code>.
install-needs-title = Was es braucht
install-windows = <b>Windows</b> 10 oder 11, x64 oder ARM64.
install-mac = <b>macOS</b> 11 oder neuer, mit Apple-Chip oder Intel.
install-linux = <b>Linux</b> mit glibc 2.35 oder neuer (Ubuntu 22.04 und neuer) und einem Vulkan-Treiber; das <code>.deb</code> bringt den Rest mit.
install-cpu = <b>Prozessor:</b> Die x86-64-Builds brauchen AVX2; ein älterer Prozessor nimmt das Archiv für ältere Prozessoren.

## Gateways

gws-title = Gateways
gws-lead = Ein Gateway ist ein Server, auf dem Online-Tische laufen. Central führt ein Gateway auf, sobald sein Betreiber es registriert und ein Administrator es freigegeben hat; ein Eintrag ist kein Versprechen darüber, was ein Gateway tut.
gws-count = { $n ->
    [one] { $n } Gateway aufgeführt
   *[other] { $n } Gateways aufgeführt
}
gws-official = Offiziell
gws-online = Online
gws-offline = Offline
gws-version = Version
gws-website = Website
gws-address = Adresse
gws-copy-hint = In Baylee fügst du ein Gateway über seine Adresse hinzu.
gws-none = Noch ist kein Gateway aufgeführt.
gws-failed = Das Verzeichnis konnte gerade nicht gelesen werden. Versuch es in einer Minute noch einmal.
host-title = Dein eigenes Gateway
host-lead = Ein Gateway ist Baylees Server: ein Programm und eine Datenbank. Du kannst eins für deine Spielgruppe betreiben, zu Hause oder auf einem gemieteten Server, und es bei Central registrieren, damit es Spieler mit ihrem Baylee-Account anmelden kann.
host-steps = Lade das Gateway aus dem neuesten Release, folge der <guide>Gateway-Anleitung</guide> und registriere es dann unter <account>den Gateways deines Accounts</account>. Central führt es in diesem Verzeichnis auf, sobald ein Administrator es freigegeben hat.

## Fragen

faq-eyebrow = Fragen
faq-title = Fragen und Antworten
faq-what-q = Was ist Baylee?
faq-what-a = Ein freies Fanspiel zu Magic: The Gathering für deinen Computer, mit Deckbuilder, einer Regel-Engine, die jeden Schritt prüft, einem Tisch für bis zu acht Spieler und Computergegnern. Es ist nicht von Wizards of the Coast gemacht, freigegeben oder unterstützt.
faq-cost-q = Was kostet es?
faq-cost-a = Nichts, und das bleibt so: keine Bezahlschranke, keine bezahlten Funktionen, keine Werbung.
faq-account-q = Brauche ich einen Account?
faq-account-a = Nicht, um allein zu spielen: Deckbuilder und Partien gegen die Haus-KI funktionieren offline. Für Online-Tische brauchst du einen Baylee-Account, den du auf dieser Seite anlegst.
faq-data-q = Was passiert mit meinen Daten?
faq-data-a = So wenig wie möglich, und es steht in der <privacy>Datenschutzerklärung</privacy>. Dein Passwort verlässt nie deinen Browser; deine privaten Daten und deine Chatnachrichten werden auf deinem Gerät verschlüsselt, bevor sie uns erreichen.
faq-password-q = Ich habe mein Passwort vergessen. Was nun?
faq-password-a = Mit dem Wiederherstellungscode, den du beim Anlegen deines Accounts bekommen hast, setzt du ein neues. Ohne ihn kann dir ein Administrator einen Reset-Code geben, aber deine verschlüsselten Daten lassen sich ohne den Code nicht wiederherstellen.
faq-gateway-q = Was ist ein Gateway?
faq-gateway-a = Ein Server, auf dem Online-Tische laufen. Jeder kann eins betreiben; das <gateways>Verzeichnis</gateways> zeigt die, die Central kennt.
faq-systems-q = Auf welchen Systemen läuft es?
faq-systems-a = Windows, macOS und Linux, auf x86-64 und ARM64. Für Handys und Tablets gibt es noch keine Version.
faq-open-q = Ist es Open Source?
faq-open-a = Das Spiel ja, unter der GNU Affero General Public License. Der Code von Baylee Central ist öffentlich, damit jeder prüfen kann, wie er mit seinen Daten umgeht.
faq-bug-q = Ich habe einen Fehler gefunden. Wo melde ich ihn?
faq-bug-a = Im Spiel schickt <kbd>F8</kbd> einen Bericht mit allem, was er braucht. Sonst im Issue-Tracker auf GitHub, auf Discord oder über das <contact>Kontaktformular</contact>.

## Kontakt

contact-eyebrow = Kontakt
contact-title = Schreib uns
contact-lead = Fragen, Ideen oder etwas, das nicht funktioniert: Schreib hier, und wir antworten per E-Mail, wenn du uns eine Adresse gibst.
contact-name = Dein Name
contact-name-hint = Freiwillig.
contact-email = Deine E-Mail-Adresse
contact-email-hint = Freiwillig; nur, damit wir antworten können.
contact-message = Deine Nachricht
contact-count = { $n } von 5.000 Zeichen
contact-send = Nachricht senden
contact-sending = Wird gesendet …
contact-sent-title = Danke
contact-sent = Deine Nachricht ist angekommen. Wenn du eine Adresse angegeben hast, antworten wir dorthin.
contact-error-empty = Schreib zuerst eine Nachricht.
contact-error-email = Das sieht nicht nach einer E-Mail-Adresse aus.
contact-error-busy = Gerade kamen zu viele Nachrichten aus deinem Netz. Versuch es in einer Stunde noch einmal.
contact-error-refused = Die Nachricht wurde nicht angenommen. Prüf ihre Länge und versuch es noch einmal.
contact-error-network = Die Nachricht konnte nicht gesendet werden. Prüf deine Verbindung und versuch es noch einmal.
contact-privacy = Was du schickst, wird zum Beantworten aufbewahrt und 90 Tage nach der Bearbeitung gelöscht, spätestens nach einem Jahr. Mehr in der <privacy>Datenschutzerklärung</privacy>.
contact-noscript = Das Formular braucht JavaScript. Du erreichst uns auch auf Discord oder auf GitHub.
contact-bugs-title = Einen Fehler melden
contact-bugs = Drück im Spiel <kbd>F8</kbd>: Der Bericht enthält das Protokoll und dein System, und du entscheidest, was du sonst noch mitschickst. Sonst eröffne ein Issue auf GitHub.

## Impressum

impressum-title = Impressum
impressum-lead = Angaben nach § 5 DDG. Baylee ist ein privates, nicht kommerzielles Projekt.
impressum-show = Angaben anzeigen
impressum-why = Die Angaben liegen hinter einem kleinen Rätsel, das dein Browser in wenigen Sekunden löst, damit Roboter, die Adressen sammeln, draußen bleiben. Mehr wird von dir nicht verlangt.
impressum-solving = Das Rätsel wird gelöst … { $percent } %
impressum-name = Name
impressum-address = Anschrift
impressum-email = E-Mail
impressum-phone = Telefon
impressum-copy = Kopieren
impressum-copied = Kopiert
impressum-failed = Die Angaben konnten nicht geladen werden. Versuch es noch einmal oder schreib uns über das <contact>Kontaktformular</contact>.
impressum-noscript = Die Angaben brauchen JavaScript. Ohne erreichst du uns über Discord oder GitHub.
impressum-responsible = Verantwortlich für den Inhalt: die oben genannte Person.
impressum-dispute = Wir sind weder verpflichtet noch bereit, an Streitbeilegungsverfahren vor einer Verbraucherschlichtungsstelle teilzunehmen.

## Fan-Inhalt

fc-title = Inoffizieller Fan-Inhalt
fc-lead = Baylee ist ein Fanprojekt. Es ist im Rahmen der Richtlinie für Fan-Inhalte von Wizards of the Coast erlaubt, deren Regeln es folgt.
fc-notice-title = Der Hinweis auf jeder Seite
fc-means-title = Was das heißt
fc-means = Baylee ist nicht von Wizards of the Coast gemacht, freigegeben oder unterstützt. Magic: The Gathering, die Kartennamen, Setnamen, Kartenbilder und Manasymbole sind Eigentum von Wizards of the Coast LLC. Baylee ist für alle frei: Es gibt keine Bezahlschranke, keine bezahlte Funktion und keine Registrierung per E-Mail.
fc-images = Das Spiel zeigt Kartenbilder von Scryfall, unverändert und mit ihrem Künstler; diese Seite zeigt keine. Kartendaten und -bilder stellt Scryfall bereit; Baylee ist nicht mit Scryfall verbunden.
fc-policy = Die <policy>Richtlinie für Fan-Inhalte</policy> von Wizards of the Coast

## Datenschutzerklärung

privacy-title = Datenschutzerklärung
privacy-lead = Was diese Seite und Baylee Central über dich verarbeiten, wozu, auf welcher Rechtsgrundlage und wie lange. Entwurf, am 10. November 2026 gegen den Code geprüft; vor der endgültigen Fassung prüft ihn ein Anwalt.
privacy-updated = Stand: 10. November 2026
privacy-controller-title = Wer verantwortlich ist
privacy-controller = Die im <impressum>Impressum</impressum> genannte Privatperson. Einen Datenschutzbeauftragten gibt es nicht; er ist nicht vorgeschrieben.
privacy-short-title = Kurz gesagt
privacy-short = Kein Tracking, keine Werbung, keine Analyse, kein Cookie, solange du dich nicht anmeldest, und nichts von fremden Servern: Schriften, Bilder und Skripte kommen alle von baylee.app. Dein Passwort erreicht uns nie, und deine Chatnachrichten und privaten Daten werden in deinem Browser verschlüsselt, bevor sie gesendet werden.
privacy-hosting-title = Die Seite besuchen
privacy-hosting = Die Seite läuft auf einem bei der netcup GmbH in Deutschland gemieteten Server. Wenn du eine Seite öffnest, schickt dein Browser, was jede Anfrage enthält: deine IP-Adresse, die angefragte Adresse, die Zeit, die Sprache deines Browsers und was er über sich sagt. Der Server braucht das, um zu antworten, und die Download-Seite schlägt mit der Beschreibung deines Browsers die passende Datei vor. Der Webserver führt kein Zugriffsprotokoll; diese Anfragen werden also nicht aufgeschrieben. Die eigenen Betriebsmeldungen des Servers (sein Systemjournal) können in seltenen Fehlerfällen eine IP-Adresse enthalten; sie bleiben nur so lange, wie das Journal sie hält. Rechtsgrundlage: Art. 6 Abs. 1 lit. f DSGVO (sichere Auslieferung der Seite).
privacy-device-title = Was dein Browser speichert
privacy-device = Diese Seite setzt kein Cookie, bis du dich anmeldest. Was dein Browser auf deinem Gerät speichert, und nur mit JavaScript: deine Farbwahl, falls du eine getroffen hast (<code>baylee-site-theme</code>); während du angemeldet bist, wann deine Sitzung endet (<code>baylee-central-session</code>), deinen Chatschlüssel und die vertrauten Schlüssel deiner Kontakte (siehe Chat); wenn du Mitteilungen eingeschaltet hast, eine Zufallszahl für dieses Gerät (<code>baylee-push-installation-v1</code>, <code>baylee-push</code>); und wenn du die Seite als App installierst, eine Kopie ihrer öffentlichen Dateien für die Offline-Nutzung. Nichts davon wird an uns gesendet, außer ein Abschnitt unten sagt es; die Einträge zu Chat und Sitzung werden beim Abmelden gelöscht. Rechtsgrundlage: § 25 Abs. 2 Nr. 2 TDDDG.
privacy-github-title = GitHub
privacy-github = Die Download-Seite und die Startseite zeigen das neueste Release, den Teststand und die Kartenzahlen. Danach fragt unser Server GitHub, nicht dein Browser; GitHub erfährt nichts über dich. Ein Download-Link führt erst zu GitHub (GitHub Inc., USA), wenn du ihn anklickst; dann gilt GitHubs eigene Datenschutzerklärung.
privacy-directory-title = Das Gateway-Verzeichnis
privacy-directory = Unser Server liest das Verzeichnis der Gateways bei Baylee Central und zeigt es an; nichts über dich ist daran beteiligt.
privacy-account-title = Dein Account
privacy-account = Für einen Account speichert Baylee Central: deinen Anmeldenamen und deinen Anzeigenamen mit seinem Tag, einen OPAQUE-Passwortdatensatz (mit dem du dein Passwort beweist, ohne dass unser Server es je erfährt), deinen Hauptschlüssel zweimal verschlüsselt (mit deinem Passwort und mit deinem Wiederherstellungscode), dein verschlüsseltes Account-Dokument, welche Version der Nutzungsbedingungen du angenommen hast, die Zeiten deiner Anmeldungen, deine Sitzungen und den Hashwert des Beta-Keys, den du benutzt hast. Rechtsgrundlage: Art. 6 Abs. 1 lit. b DSGVO (der Account, den du wolltest). Gespeichert, bis du den Account löschst; Sitzungen enden nach 30 Minuten ohne Nutzung und spätestens 12 Stunden nach der Anmeldung.
privacy-cookie-title = Das Sitzungscookie
privacy-cookie = Beim Anmelden wird ein Cookie gesetzt, <code>__Host-central</code>: eine Zufallszahl, die deine Sitzung kennzeichnet, nur an baylee.app gesendet, für Skripte nie lesbar, mit der Sitzung beendet. Es ist für das Angemeldetsein unbedingt erforderlich. Rechtsgrundlage: § 25 Abs. 2 Nr. 2 TDDDG und Art. 6 Abs. 1 lit. b DSGVO.
privacy-chat-title = Chat und Kontakte
privacy-chat = Nachrichten werden auf deinem Gerät für die Geräte jedes Empfängers verschlüsselt; Baylee Central speichert sie nur verschlüsselt und kann sie nicht lesen. Was es sehen kann und speichert: wer mit wem in Kontakt steht, die Mitglieder von Gruppen und ihre Namen, wann eine Nachricht gesendet wurde, wie groß sie ist, und die öffentlichen Schlüssel deiner Geräte. Dein Browser hält deinen Chatschlüssel (versiegelt unter einem Schlüssel, den nur dieser Browser hat, in seinem Speicher <code>baylee-chat</code>) und die Schlüssel deiner Kontakte, denen du vertraust (verschlüsselt, <code>baylee-chat-trust</code>), solange du angemeldet bist. Nachrichten und Zustellungen bleiben, bis du deinen Account löschst; jedes Gerät hält höchstens 5.000. Rechtsgrundlage: Art. 6 Abs. 1 lit. b DSGVO.
privacy-push-title = Mitteilungen
privacy-push = Wenn du in einem Browser Mitteilungen einschaltest, meldet sich dein Browser bei seinem Push-Dienst an, und Central speichert diese Adresse und eine Zufallszahl für dieses Gerät. Der Push-Dienst gehört dem Hersteller deines Browsers: Google (Firebase Cloud Messaging), Mozilla oder Apple. Eine Mitteilung trägt nur ihre Art (etwa „neue Nachricht“) und deine Sprache, nie Namen oder Text. Sie wird gelöscht, wenn du Mitteilungen auf diesem Gerät ausschaltest, dich abmeldest oder deinen Account löschst; eine Adresse, die 90 Tage nicht erneuert wurde, wird nicht mehr benutzt. Rechtsgrundlage: Art. 6 Abs. 1 lit. a DSGVO (deine Einwilligung, die du jederzeit in den Einstellungen deines Accounts widerrufen kannst).
privacy-counters-title = Schutz vor Missbrauch
privacy-counters = Gegen Raten und Überfluten zählt Central bestimmte Anfragen je IP-Adresse (Registrierungen, Anmeldeversuche, Passwort-Resets, Lesen des Verzeichnisses), nur im Arbeitsspeicher und höchstens eine Stunde lang. Rechtsgrundlage: Art. 6 Abs. 1 lit. f DSGVO.
privacy-contact-title = Das Kontaktformular
privacy-contact = Das Formular schickt deine Nachricht, deinen Namen und deine E-Mail-Adresse nur, wenn du sie angibst, die Sprache der Seite und die Zeit an Baylees Feedback-Dienst auf demselben Server; er vermerkt außerdem, ob die Nachricht erledigt ist. Deine IP-Adresse wird nicht gespeichert; eine Stunde lang wird sie nur im Arbeitsspeicher gezählt, gegen eine Flut von Nachrichten. Nur die im Impressum genannte Person liest die Nachrichten; nichts wird weitergegeben. Eine Nachricht wird 90 Tage nach ihrer Erledigung gelöscht, spätestens nach einem Jahr, oder früher, wenn du darum bittest. Rechtsgrundlage: Art. 6 Abs. 1 lit. f DSGVO (Nachrichten beantworten) oder lit. b, wenn deine Nachricht auf einen Vertrag zielt.
privacy-impressum-title = Das Rätsel des Impressums
privacy-impressum = Um das Impressum zu zeigen, zählt der Feedback-Dienst Rätsel je IP-Adresse eine Stunde lang im Arbeitsspeicher und behält ein gelöstes Rätsel fünf Minuten. Rechtsgrundlage: Art. 6 Abs. 1 lit. f DSGVO.
privacy-recipients-title = Empfänger und Länder
privacy-recipients = Wir verkaufen nichts und geben nichts weiter. Auftragsverarbeiter: netcup GmbH (Hosting, Deutschland). Nur wenn du Mitteilungen einschaltest, überbringt sie der Push-Dienst deines Browsers (Google, Mozilla oder Apple), möglicherweise von außerhalb der EU.
privacy-rights-title = Deine Rechte
privacy-rights = Du hast das Recht auf Auskunft (Art. 15), Berichtigung (16), Löschung (17), Einschränkung (18), Datenübertragbarkeit (20) und Widerspruch (21). Die Daten deines Accounts kannst du in seinen Einstellungen exportieren und den Account dort löschen. Du kannst dich bei einer Aufsichtsbehörde beschweren, etwa bei der Landesbeauftragten für Datenschutz und Informationsfreiheit Nordrhein-Westfalen. Es gibt keine automatisierte Entscheidung (Art. 22).

## Nutzungsbedingungen

terms-title = Nutzungsbedingungen
terms-draft = Diese Bedingungen sind ein Entwurf und noch nicht in Kraft.
terms-version = Version { $version }, aktualisiert am { $date }.

## Fehler

not-found-title = Diese Seite gibt es nicht
not-found-lead = Vielleicht ist die Adresse vertippt, oder die Seite ist umgezogen.
gone-title = Diese Seite gibt es nicht mehr
gone-lead = Was hier war, wurde entfernt. Die Startseite zeigt, wo es weitergeht.
error-home = Zur Startseite

## Baylees Erinnerung (am 27. Oktober 2026 von ihrer Familie freigegeben; Wort für Wort beibehalten)

baylee-dedication = Für Baylee. Für immer im Sonnenlicht.
baylee-invitation = Die kleine Seele hinter dieser Welt
baylee-open = An Baylee erinnern · Fotoalbum öffnen
baylee-eyebrow = Das Herz dieses Projekts
baylee-date = In liebevoller Erinnerung · 26. Oktober 2026
baylee-close = Zurück in ihre Welt
baylee-previous = Vorheriges Foto
baylee-next = Nächstes Foto
baylee-photo1 = Baylee, eingerollt in der weichen weißen Bettdecke.
baylee-photo2 = Baylee schaut aus der geblümten Bettdecke hervor.
baylee-photo3 = Ganz entspannt in ihrem runden, grauen Bettchen.
baylee-photo4 = Baylee schläft, ganz nah zusammengerollt.
baylee-photo5 = Ein stiller Moment, eingekuschelt in die Bettdecke.

baylee-storyTitle = Für Baylee. Für immer im Sonnenlicht.
baylee-together = Bei uns · 17. November 2025 – 26. Oktober 2026
baylee-chapters = Ihre Geschichte
baylee-chapterPrevious = Vorheriges Kapitel
baylee-chapterNext = Nächstes Kapitel
baylee-photos = Unsere Erinnerungen
baylee-read = Zum Lesen scrollen

baylee-chapter1Title = Ein Zuhause für Baylee
baylee-chapter1Body =
    Am 17. November 2025 kam Baylee zu uns nach Hause.

    Meine Frau und ich hatten sie aus dem Tierheim in Krefeld geholt. Ihr Alter wurde auf ungefähr dreizehn Jahre geschätzt. Sie hatte bösartige Mammatumoren, und der Krebs hatte bereits Metastasen in ihrer Leber gebildet. Baylee war eine Palliativkatze. Wir wussten, dass unsere gemeinsame Zeit begrenzt sein würde. Sie sollte bei uns ein Zuhause haben, in dem sie geliebt wurde.

    Wie schnell sie dieses Zuhause für sich beanspruchen würde, zeigte sie uns gleich selbst.

    Schon am ersten Tag schlich sie sich auf meinen Schoß, während ich arbeitete. Und in ihrer ersten Nacht riss sie ein Loch in einen Beutel Katzenfutter, den ich eigentlich für die Katzen meiner Eltern bestellt hatte, und bediente sich daraus. Natürlich hatte sie ihr eigenes Futter bekommen und war am Abend gut gefüttert worden. Aber Baylee hatte offenbar beschlossen, dass dieser Vorrat trotzdem eine nähere Untersuchung verdiente.

    Die kleine Maus hatte einen erstaunlichen Appetit – und unglaublich viel Energie.

baylee-chapter2Title = Vertrauen wächst
baylee-chapter2Body =
    Weil uns auffiel, dass sie sehr viel Wasser trank, gingen wir mit ihr zum Tierarzt. Eine Blutuntersuchung zeigte eine Erkrankung der Schilddrüse, die auch ihren großen Hunger und ihre lebhafte Art erklärte. Sie bekam Schilddrüsenmedikamente und vorübergehend ein Antibiotikum, weil ihre Entzündungswerte erhöht waren.

    Als sie sich plötzlich überall rieb, viel maunzte und ungewöhnlich verschmust und anhänglich wurde, machten wir uns zunächst Sorgen, dass sie eines der neuen Medikamente nicht vertrug. Bei der erneuten Untersuchung stellte sich jedoch heraus: Unsere kleine Mausi war rollig. Sie war also nicht kastriert worden.

    Auch diese Zeit standen wir gemeinsam durch. Baylee vertraute uns immer mehr, und wir schlossen uns gegenseitig immer tiefer ins Herz.

baylee-chapter3Title = Unsere kleine Prinzessin
baylee-chapter3Body =
    Ich erinnere mich noch genau daran, wie sehr sie sich über ihren Kratzbaum freute. Sie setzte sich sofort ganz oben auf den Sitz und schaute zufrieden in die Wohnung. Erstaunlicherweise benutzte sie ausgerechnet diesen Platz danach nie wieder. Warum, blieb ihr Geheimnis.

    Andere Plätze ließ sie dafür kaum aus: meinen Schoß, meine Beine und unser Bett. Dort schlief sie meistens auf mir oder auf meiner Frau. Am liebsten schaffte sie es, auf uns beiden gleichzeitig zu liegen.

    Mit erhobenem Schwanz stolzierte sie durch die Wohnung, als wüsste sie ganz genau, dass sie hier die Prinzessin war. Sie hatte Temperament und konnte deutlich zeigen, wo ihre Grenzen lagen. Sie durfte bei uns ganz sie selbst sein. Auch mit ihren Eigenheiten, ihren Forderungen und ihrem kleinen Dickkopf wurde sie geliebt.

    Noch nie hatte ich zu einem Tier eine so tiefe Beziehung aufgebaut. Baylee wuchs meiner Frau und mir sehr tief ins Herz.

baylee-chapter4Title = Sonnenstunden
baylee-chapter4Body =
    Als es draußen wärmer wurde, entdeckte sie den Balkon für sich. Sie liebte es, die ersten Sonnenstrahlen des Tages in ihrem Fell zu spüren. Bald verbrachte sie dort beinahe den ganzen Tag.

    Besonders die Tauben und Gartenvögel hatten es ihr angetan. Sie beobachtete sie aufmerksam, lag ihnen auf der Lauer und versuchte immer wieder, sie zu erschrecken. Zwischen Sonnenbaden und Vogelbeobachtung hatte unsere kleine Prinzessin einen gut gefüllten Tagesplan.

baylee-chapter5Title = Ihr kleiner Dickkopf
baylee-chapter5Body =
    Währenddessen schritt ihre Krankheit weiter voran.

    Der Tumor entzündete sich, und Baylee leckte die betroffene Stelle immer wieder auf. Beim Tierarzt bekam sie ihren ersten Body, einen roten. Von da an musste sie täglich einen tragen, damit sie die Wunde nicht weiter verletzte.

    An das Tragen gewöhnte sie sich schnell. Das Anziehen konnte sie allerdings bis zum letzten Tag nicht leiden. Selbst mit einer Schale Futter ließ sie sich dann nur bedingt hervorlocken. So groß ihr Appetit auch war: Ihre Meinung zum Body blieb eindeutig.

    Die Wunde wurde schlimmer. Sie eiterte, begann irgendwann zu bluten und roch unangenehm. Gleichzeitig zeigte Baylee uns weiterhin, wie viel Freude sie an ihrem Alltag hatte. Sie spielte, fraß, erkundete ihre Umgebung und kletterte auf ihr Kratzbrett und auf die Schränke.

    Manchmal lag sie stundenlang auf der Bettkante neben uns und wartete darauf, dass wir endlich aufwachten und ihr Frühstück machten. Wenn ihr das zu lange dauerte, kletterte sie kurzerhand über uns hinweg. Unsere Gesichter waren dabei kein Hindernis.

    Beim Essen hatte sie genaue Vorstellungen. Geschmacklich gehörte sie eher zur „Fraktion Tiefkühlpizza“: Hochwertiges Katzenfutter konnte sie nach einer Weile nicht mehr begeistern, und Abwechslung war Pflicht. Zweimal hintereinander dasselbe Futter kam für sie nicht infrage. Huhn mit Leber, alles mit Käse, Thunfisch oder Lachs – damit konnte man ihr eine Freude machen.

baylee-chapter6Title = Mit erhobenem Schwanz
baylee-chapter6Body =
    Der Tumor wurde größer und nässte zunehmend. Baylee hatte sichtbar Schmerzen. Als diese nachließen, war sie wieder unsere kleine Katze, die mit erhobenem Schwanz durch die Räume zog und ihre Menschen für sich beanspruchte.

    Wir sahen, dass ihre Kräfte langsam weniger wurden. Doch ihr Appetit, ihre Neugier, ihre verschmuste Art und ihr lautes Schnurren waren noch da.

baylee-chapter7Title = Abschied nehmen
baylee-chapter7Body =
    Am Freitagabend, dem 23. Oktober 2026, bekam sie Symptome, die zunächst wie eine Erkältung wirkten. Ihre Nase lief, und sie atmete tief und angestrengt, als müsse sie um Luft ringen.

    Am Samstag wirkte sie wieder lebhafter. Die auffällige Atmung und der Schnupfen blieben, aber sie zeigte noch einmal die Energie und Lebensfreude, die wir von ihr kannten.

    Der Sonntagmorgen war schwer. Ihre Augen tränten, sie atmete mühsam und hatte Schmerzen. Ich gab ihr Schmerzmittel. Nach ungefähr einer Stunde ging es ihr wieder besser, doch ihre Atmung blieb verändert. Für uns war klar, dass wir mit ihr zum Tierarzt mussten.

    Am Montagmorgen verschlechterte sich ihr Zustand erneut. Zum Glück hatte meine Frau an diesem Tag frei. Wir riefen in der Praxis an, bekamen einen Termin und fuhren mit Baylee dorthin.

    Die Röntgenaufnahme zeigte, wie weit sich der Krebs inzwischen ausgebreitet hatte. Uns wurde erklärt, dass ihr nur noch ungefähr ein Drittel ihrer Lunge zum Atmen zur Verfügung stand. Sie weiterleben zu lassen hätte bedeutet, dass sie immer weniger Luft bekommen würde.

    Wir mussten Abschied nehmen.

    Am Montag, dem 26. Oktober 2026, gegen zwölf Uhr wurde unsere liebste Baylee eingeschläfert.

baylee-chapter8Title = Für immer im Sonnenlicht
baylee-chapter8Body =
    Sie war knapp ein Jahr bei uns. In dieser Zeit war sie ein Teil unserer Familie geworden und eine sehr enge Freundin. Sie gehörte zu unseren Arbeitstagen, zu unseren Nächten, zum Aufwachen am Morgen. Zu den Sonnenstunden auf dem Balkon und zu den kleinen Gewohnheiten, aus denen ein gemeinsames Leben besteht.

    Jetzt fehlt sie in all diesen Momenten.

    Die Leere ist noch frisch und tief. Uns fehlen ihr Schnurren, ihr Gewicht auf unseren Beinen, ihr erhobener Schwanz und ihre selbstverständliche Erwartung, dass wir uns endlich um ihr Frühstück kümmern.

    Wenn es etwas nach diesem Leben gibt, was auch immer es sein mag, dann wünsche ich mir, dieser Seele eines Tages wiederzubegegnen.

    Bis dahin tragen wir unser kleines Mäuschen für immer im Herzen.

    Deshalb ist dieses Hobbyprojekt Baylee gewidmet. Ihrem Temperament, ihrer Nähe, ihrer Freude an der Sonne und den Vögeln. Und der Liebe, die in unserer gemeinsamen Zeit gewachsen ist.

    Hier darf sie friedlich auf ihrer Wolke schlafen, im warmen Sonnenlicht, umgeben von Blumen, Tauben und Gartenvögeln.

    Ruhe in Frieden, meine liebste Baylee.

baylee-photoOpen = Foto groß ansehen
baylee-photoClose = Foto schließen
baylee-zoomIn = Vergrößern
baylee-zoomOut = Verkleinern
baylee-fit = Einpassen

baylee-photo6 = Baylee, gemütlich zwischen den Decken.
baylee-photo7 = Geborgen im roten Body.
baylee-photo8 = Ein Sonnenplatz im Karton.
baylee-photo9 = Eingekuschelt und ganz entspannt.
baylee-photo10 = Ihr Lieblingsplatz: ganz nah bei uns.
