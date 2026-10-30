### The website's words in German. English (locales/en/site.ftl) decides
### which messages exist and which arguments and markup each takes; this file
### translates them by Baylee's glossary (docs/translations.md): Magic's terms
### as German cards print them, the words German players say in English stay
### English (Account, Deck, Gateway, Release, Screenshot …), du, „…“ and – .

## The frame

app-loading = Lädt …
app-skip = Zum Inhalt springen
app-close = Schließen
nav-cards = Karten
nav-download = Download
nav-gateways = Gateways
nav-contact = Kontakt
nav-menu = Menü
nav-label = Bereiche
nav-signIn = Anmelden
nav-account = Dein Account, { $name }
nav-accountUnnamed = Dein Account
notFound-title = Diese Seite gibt es nicht
notFound-back = Zurück zur Startseite

## The tab's title, per page

title-home = Baylee – ein Tisch, der die Regeln kennt
title-download = Download – Baylee
title-cards = Karten – Baylee
title-gateways = Gateways – Baylee
title-contact = Kontakt – Baylee
title-impressum = Impressum – Baylee
title-privacy = Datenschutz – Baylee
title-terms = Nutzungsbedingungen – Baylee
title-register = Account anlegen – Baylee
title-signIn = Anmelden – Baylee
title-account = Dein Account – Baylee
title-accountSecurity = Sicherheit und Sitzungen – Baylee
title-accountGateways = Deine Gateways – Baylee
title-admin = Verwaltung – Baylee
title-adminUser = Ein Account – Verwaltung – Baylee
title-adminGateways = Gateways – Verwaltung – Baylee
title-adminAudit = Protokoll – Verwaltung – Baylee
title-adminLegacy = Alte Konten – Verwaltung – Baylee
title-notFound = Nicht gefunden – Baylee

## The footer. foot-fanContent is Wizards' official German notice, word for
## word; it stands beside the English one.

foot-fanContent = Baylee ist ein inoffizieller Fan-Inhalt im Rahmen der <policy>Richtlinie für Fan-Inhalte</policy>. Nicht von Wizards gefördert/gesponsert. Bestandteile des enthaltenen Materials sind Eigentum von Wizards of the Coast. ©Wizards of the Coast LLC. Baylee ist nicht mit Wizards of the Coast verbunden.
foot-fanContentUrl = https://company.wizards.com/de/legal/fancontentpolicy
foot-wotc = Magic: The Gathering sowie Kartennamen, Setnamen, Kartenbilder und Manasymbole sind Eigentum von Wizards of the Coast LLC. Jedes Kartenbild nennt seine Künstlerin oder seinen Künstler; ein Klick auf eine Karte zeigt Künstler und Druck bei Scryfall.
foot-scryfall = Kartendaten und -bilder bereitgestellt von Scryfall. Baylee ist nicht mit Scryfall verbunden.
foot-label = Fußzeile
foot-play = Spielen
foot-project = Projekt
foot-legalHead = Rechtliches
foot-source = Quellcode (AGPL-3.0)
foot-central = Baylee Central (Quellcode einsehbar)
foot-siteSource = Quellcode dieser Seite
foot-releases = Releases
foot-issues = Issues
foot-contact = Kontakt
foot-legal = Rechtliches
foot-terms = Nutzungsbedingungen
foot-privacy = Datenschutz
foot-impressum = Impressum
foot-colophon = Schriften: Faustina und Alegreya Sans, SIL Open Font License 1.1, von dieser Seite ausgeliefert. Das Gemälde und das Logo sind unsere eigenen.

## Paths in Baylee's repository: the German README's sections

link-readmeWhat = /blob/main/README.de.md#was-baylee-ist
link-readmeInstall = /blob/main/README.de.md#installation
link-readmeGateway = /blob/main/README.de.md#ein-eigenes-gateway-betreiben
link-readmeModel = /blob/main/README.de.md#gegen-ein-sprachmodell-spielen

## Times, sizes and the state of CI

time-minutesAgo = vor { $n } Min.
time-hoursAgo = vor { $n } Std.
time-daysAgo =
    { $n ->
        [one] vor { $n } Tag
       *[other] vor { $n } Tagen
    }
size-mb = { $size } MB
ci-passing = grün
ci-failing = rot

## The start page

hero-kicker = Geschlossene Beta · kostenlos · quelloffen · inoffizieller Fan-Inhalt
hero-title = Ein Tisch, der <glow>die Regeln kennt.</glow>
hero-lead = Baylee ist eine Regel-Engine und ein 3D-Spieltisch für Magic: The Gathering – mit Freunden oder gegen die Haus-KI. Die Engine setzt die Comprehensive Rules durch, damit es niemand am Tisch tun muss. Kostenlos, quelloffen unter der AGPL, und das bleibt so.
hero-note = Offline gegen die Haus-KI geht ohne Account. Online auf dem Sanctuary-Gateway ist eine geschlossene Beta und braucht einen Schlüssel.
hero-what = Was Baylee ist
hero-cards = Stand der Karten
hero-scroll = Nach unten

status-label = Projektstand
status-build = Neuester Build
status-releases = siehe Releases
status-ci = CI auf main
status-onGitHub = auf GitHub ansehen
status-commit = Letzter Commit
status-gateways = Gateways
status-directory = das Verzeichnis

about-eyebrow = Was es ist
about-title = Zwei Hälften, ein Spiel
about-engineTitle = Eine Engine, die das Spiel durchsetzt
about-engine = Priorität und Stapel, das Schichtensystem, Ersetzungseffekte, Trigger, zustandsbasierte Aktionen und Kampf: Die Engine wendet die Comprehensive Rules an und zählt bei jedem Schritt genau die Entscheidungen auf, die ein Spieler hat. Der Client bietet genau diese an und sonst nichts. Er entscheidet nie, was erlaubt ist.
about-deterministic = Sie ist deterministisch: ein geseedeter Zufallsgenerator, keine Uhr und keine Hash-Map-Reihenfolge in den Regeln. Dasselbe Spiel mit denselben Antworten endet immer gleich. Jedes gehostete Spiel wird Zug für Zug aufgezeichnet und lässt sich auf denselben Zustand zurückspielen – das ist auch, was ein Bericht mitbringt.
about-tableTitle = Ein 3D-Tisch auf Bevy
about-table = Spielfeld, Hände, Stapel und Friedhöfe auf einem Tisch, den man auf einen Blick lesen kann, dazu eine Lobby für Accounts, Decks und Räume. Die Kartenbilder kommen beim Spielen von Scryfall; eine Textansicht zeichnet jede Karte stattdessen aus ihrem Regeltext, und <kbd>T</kbd> schaltet um.
about-keys = Mit der Maus oder ganz über die Tastatur: <kbd>Leertaste</kbd> passt, <kbd>Tab</kbd> spult zur nächsten Phase vor, wobei jede Entscheidung bei dir bleibt, <kbd>L</kbd> öffnet ein Protokoll, das sich wie ein Chat liest. Jede Taste lässt sich umbelegen.

feature-hiddenTitle = Verdeckte Information bleibt verdeckt
feature-hidden = Dein Client bekommt nur, was du sehen darfst. Bibliotheken und fremde Hände kommen als Zahlen an, eine verdeckte Karte trägt keine Identität. Ein manipulierter Client kann nicht spicken, weil die Daten ihn nie erreichen.
feature-seatsTitle = Zwei bis acht Plätze
feature-seats = Duelle, Jeder-gegen-jeden und Teamtische. Der Gastgeber verteilt die Plätze auf Seiten, und die Seiten müssen nicht gleich groß sein. Teams haben eigene Züge und Lebenspunkte; Teamkollegen können sich gegenseitig die Hand zeigen.
feature-commanderTitle = Commander und Freiform
feature-commander = Ein Deck mit Commander spielt Commander, jedes andere spielt frei. Banlisten und Formatprüfungen gibt es noch nicht. Das ist eine Lücke, kein Feature.
feature-aiTitle = Eine Haus-KI in fünf Stufen
feature-ai = Anfänger, Locker, Solide, Scharf und Experte. Sie spielt aus derselben Sicht wie ein Mensch und sieht deine Hand genauso wenig.
feature-processTitle = Ein Prozess pro Spiel
feature-process = Jedes Spiel läuft in einem eigenen Engine-Prozess. Das Gateway für Accounts und Räume kennt keine Regeln, also ist ein kaputtes Spiel genau ein kaputtes Spiel.
feature-languagesTitle = Kartentext in 19 Sprachen
feature-languages = Hat das Gateway den Kartenkatalog, lesen sich Karten in deiner Sprache, sonst auf Englisch. Die Oberfläche selbst gibt es auf Englisch und Deutsch.
feature-watchTitle = Einem laufenden Tisch zuschauen
feature-watch = Der Button „Zuschauen“ in der Lobby setzt dich als Zuschauer an den Tisch. Du siehst, was der ganze Tisch sieht, und keine Hand. Ein Gastgeber kann Zuschauer für seinen Raum abschalten.
feature-connectionTitle = Verbindung weg, Spiel nicht verloren
feature-connection = Fällt ein Spieler heraus, zeigt der Tisch einen Countdown, solange sein Platz auf ihn wartet. Läuft die Zeit ab, spielt die Haus-KI für ihn, bis er zurück ist; sitzt sonst niemand am Tisch, pausiert das Spiel einfach.
feature-tablesTitle = Tische nach deinen Regeln
feature-tables = Ein Tisch „Gemischt“ setzt Decks jedes Formats zusammen, und vierzehn Entscheidungsuhren reichen von 15 Sekunden bis ohne Uhr. Startleben und Mulligans legt der Gastgeber fest.

shots-eyebrow = Am Tisch
shots-title = So sieht es aus
shots-lead = Die Bilder sind kurze Aufnahmen des Spiels, mit den Kartenbildern von Scryfall (© Wizards of the Coast). Entstanden sind sie mit 0.1.0-beta.8b auf einem lokalen Test-Gateway, mit erfundenen Spielern.
shots-credit = Kartenbilder: Scryfall · © Wizards of the Coast
shots-showLarge = Groß zeigen: { $title }
shots-dialog = Screenshot
shots-previous = Zurück
shots-next = Weiter
shot-duelAlt = Ein Duell gegen die Haus-KI: Sechs Kreaturen greifen an, drei werden geblockt, und die geblockten gehen auf den Friedhof
shot-duelTitle = Ein Duell
shot-duelCaption = gegen die Haus-KI. Sechs Kreaturen greifen an, drei werden geblockt; die Linien zeigen, wer gegen wen kämpft.
shot-logAlt = Ein Duell mit offenem Protokoll: ein Land, Raging Goblin und Storm Seeker auf dem Stapel, ein Spieler als Ziel gewählt, das Protokoll füllt sich
shot-logTitle = Das Protokoll
shot-logCaption = (L) liest sich wie ein Chat und schreibt jeden Zug mit; jeder Kartenname öffnet die Vorschau.
shot-ringAlt = Ein Tisch mit vier Plätzen im Ring um den Filz; je eine Karte auf drei anderen Spielfeldern wird groß gezeigt
shot-ringTitle = Vier Plätze.
shot-ringCaption = Größere Tische sitzen im Ring um den Filz; zeigst du auf eine Karte, siehst du sie groß.
shot-lobbyAlt = Die Lobby: Ein Flüstern und eine Tischeinladung einer Freundin kommen an, dann öffnet sich der private Chat
shot-lobbyTitle = Die Lobby.
shot-lobbyCaption = Offene Tische, deine Freunde und wo sie sind; ein Flüstern und eine Einladung kommen an, und der verschlüsselte Chat öffnet sich.
shot-roomAlt = Ein Raum wird eingerichtet: Drei freie Plätze gehen an die Haus-KI in drei Stufen
shot-roomTitle = Einen Tisch einrichten.
shot-roomCaption = Der Gastgeber gibt freie Plätze der Haus-KI, jeden in einer eigenen Stufe, oder einem anderen Spieler oder einem Sprachmodell.
shot-builderAlt = Der Deckbauer: Die Eingabe angel serra engt den Pool ein, Serra Angel wird groß gezeigt und kommt ins Deck
shot-builderTitle = Der Deckbauer.
shot-builderCaption = Eine Suche über den ganzen Pool wird beim Tippen enger; zeigst du auf eine Karte, siehst du sie groß, und sie kommt ins Deck, nach Typ gruppiert.

ai-eyebrow = Gegner
ai-title = Gegner: das Haus, oder ein Sprachmodell
ai-houseTitle = Die Haus-KI
ai-house = Fünf Stufen von Anfänger bis Experte, offline und an jedem Online-Tisch. Sie spielt aus derselben Sicht wie ein Mensch, verdeckte Information bleibt also auch vor ihr verdeckt. Sie spielt heuristisch, nicht trainiert: gut genug, um ein Deck dagegen zu lernen, kein Turniergegner.
ai-modelTitle = Ein Platz für ein Sprachmodell
ai-model = Eine Seat-Bridge lässt ein Modell als gewöhnlichen Spieler am Tisch sitzen: ein lokales Modell in LM Studio, eines hinter einer OpenAI-kompatiblen oder Anthropic-API oder ein Agent-CLI mit deinem eigenen Login. API-Schlüssel stehen nie in einer Datei; die Einstellungsdatei nennt die Umgebungsvariable, aus der sie kommen, und Tages- und Monatslimits gelten über alle Spiele. Wenn das Modell nicht antwortet oder etwas Unerlaubtes versucht, antwortet das Haus für diese Frage, und das Protokoll sagt es.
ai-gateway = An einem Gateway-Tisch setzt der Gastgeber es im Raum hin: Jeder freie Platz bietet im Desktop-Client <i>Sprachmodell</i> an, und der Client startet die Seat-Bridge (sie liegt dem Spiel bei) mit deinen eigenen Einstellungen und deinem Schlüssel. Wo der Betreiber des Gateways gehostete Modelle anbietet, kann ein registrierter Gastgeber stattdessen eines davon wählen; es spielt auf dem Rechner des Betreibers, auf deinem braucht es dann weder Schlüssel noch Seat-Bridge.
ai-setup = So richtest du es ein

beta-eyebrow = Online
beta-title = Die geschlossene Beta
beta-sanctuary = Das öffentliche Gateway <b>Baylee Sanctuary</b> ist der Ort für Online-Tische. Es ist eine geschlossene Beta: Du spielst dort mit einem Baylee-Account, den du hier auf der Website mit einem Schlüssel der Form <code>CENTRAL-XXXX-XXXX-XXXX-XXXX</code> anlegst, den der Projektinhaber vergibt.
beta-signup = Die Registrierung fragt nach keiner E-Mail-Adresse: Ein Account ist ein Benutzername und ein Passwort, und den Benutzernamen siehst nur du. Auf einem Gateway sehen andere Spieler einen Anzeigenamen und ein kurzes Kürzel. Löschen lässt sich ein Account vorerst auf Anfrage über den Kontakt im Impressum.
beta-keys = Schlüssel kommen vom Projektinhaber, <owner>@AceVik</owner> auf GitHub. Ohne Schlüssel spielt Baylee trotzdem offline gegen die Haus-KI, mit Deckbauer und eigenen Decks, und du kannst für deine Runde ein eigenes Gateway betreiben.
beta-listTitle = Jedes Gateway in einer Liste
beta-list = Sanctuary und die Gateways, die andere für ihre Runde betreiben: Das Verzeichnis zeigt von jedem Adresse, Version und ob es gerade online ist.
beta-open = Verzeichnis öffnen
beta-own = Selbst betreiben

source-eyebrow = Quellcode
source-title = Quelloffen, und das bleibt so
source-promise = Baylee bleibt immer quelloffen und kostenlos. Keine Bezahlschranke, keine kostenpflichtigen Funktionen, keine Werbung. Nicht jetzt, nicht später. Der Code steht unter der GNU Affero General Public License: Wer ein verändertes Gateway für andere betreibt, muss auch ihnen den Quellcode anbieten.
source-rust = Geschrieben in Rust, das Projekt einer Person mit viel Hilfe von Coding-Agenten, und jede Änderung geht durch dasselbe Gate: Formatierung, Lints, die Testsuite und eine Prüfung jeder Karte gegen ihren gedruckten Text. Issues und Pull Requests sind willkommen; <agents>AGENTS.md</agents> und <claude>CLAUDE.md</claude> sind die Arbeitsregeln für Menschen und Agenten.
source-discord = Community auf Discord
source-architecture = Architektur
source-repoTitle = Das Repository gerade jetzt
source-stars = Sterne
source-issues = Offene Issues und Pull Requests
source-note = Beim Laden aus GitHubs öffentlicher API gelesen und in diesem Tab eine Weile gemerkt. Über dich geht nichts irgendwohin außer zu GitHub.

## Download

dl-button = Baylee herunterladen
dl-fallbackMeta = Windows · macOS (Apple Silicon) · Linux. Das neueste Release liegt auf GitHub.
dl-for = Download für { $system }
dl-signature = Signatur
dl-notes = Release-Notizen
dl-others = andere Systeme
dl-archive = Dieses Release kommt als Archiv: entpacken und Baylee starten. Installer kommen mit dem nächsten Release.
dl-noteUnknown = Dein System ließ sich aus diesem Browser nicht erkennen; unter Download sind alle Dateien aufgeführt.
dl-noteMobile = Für Handy und Tablet gibt es noch keine Builds. Der Button führt zum Release für deinen Rechner.
dl-noteMacUnsure = Dieser Browser verrät nicht, ob dein Mac einen Apple- oder einen Intel-Chip hat. Die Datei oben ist für Apple Silicon (M1 und neuer); ein Intel-Mac findet seine Datei bei den anderen Systemen.
dl-noteMacIntel = Dein Mac meldet einen Intel-Chip, und dieses Release hat keinen Intel-Build. Der Button führt zur Release-Seite.
dl-noteWindowsUnsure = Wenn das ein Windows-auf-ARM-PC ist (etwa ein Snapdragon-Laptop), nimm stattdessen die ARM-Datei unter Download.
dl-noteLinuxUnsure = Dein Browser sagt nicht, welche CPU das ist; x86-64 ist angenommen. Die ARM-Datei steht unter Download.

dlPage-eyebrow = Hol es dir
dlPage-title = Download
dlPage-lead = Jeder Download geht direkt zur Datei auf GitHub, nichts dazwischen. Wo ein Release Installer hat, sind sie der einfache Weg; die Archive enthalten dasselbe Spiel zum Entpacken an beliebiger Stelle. Ein installiertes Baylee aktualisiert sich selbst aus den signierten Archiven, wo es seinen eigenen Ordner beschreiben darf. Jedes Paket enthält neben dem Spiel die Seat-Bridge für Sprachmodelle.
dlPage-filesEyebrow = Jede Datei
dlPage-filesTitle = Alle Systeme

release-loading = Lade das neueste Release von GitHub …
release-noAnswer = GitHub hat gerade nicht geantwortet; einer Seite wie dieser erlaubt es 60 Anfragen pro Stunde. <releases>Die Releases-Seite hat jede Datei.</releases>
release-pre = Vorabversion
release-notes = Release-Notizen und Prüfsummen
release-all = Alle Releases

plat-empty = Ein Installer und ein Archiv für Windows (x64 und ARM64), macOS (Apple Silicon) und Linux (x86-64 und ARM64), jeweils mit einer .sha256 daneben, auf der <releases>Releases-Seite</releases>.
plat-asking = Frage GitHub nach den Dateien … Sie liegen auch auf der <releases>Releases-Seite</releases>.
plat-yours = dein System
plat-noIntel = Dieses Release hat keinen Build für Intel-Macs. Der für Apple-Chips braucht einen M1 oder neuer.
plat-missing = Dieses Release hat keine Datei für: { $systems }. Ältere Releases liegen auf der Releases-Seite.
plat-windows = Windows
plat-windowsSub = die meisten PCs: 64-Bit Intel oder AMD
plat-windowsArm = Windows auf ARM
plat-windowsArmSub = Snapdragon-Laptops und ähnliche
plat-mac = macOS
plat-macSub = Apple Silicon: M1 und neuer
plat-macIntel = macOS auf Intel
plat-linux = Linux
plat-linuxSub = x86-64
plat-linuxArm = Linux auf ARM
plat-linuxArmSub = ARM64 / aarch64
file-setup = Setup
file-diskImage = Disk-Image
file-appImage = AppImage
file-deb = Debian-/Ubuntu-Paket
file-zip = Archiv (zip)
file-tarGz = Archiv (tar.gz)

install-eyebrow = Installieren
install-title = Erster Start und was es braucht
install-firstTitle = Erster Start
install-first = Die Builds sind nicht signiert, weil Signieren Geld kostet. Windows und macOS warnen deshalb beim ersten Mal. Unter Windows: <i>Weitere Informationen</i> und <i>Trotzdem ausführen</i>. Unter macOS: <i>Systemeinstellungen → Datenschutz & Sicherheit</i> und <i>Dennoch öffnen</i>. Die <notes>Installationshinweise</notes> haben jeden Schritt und die Linux-Pakete.
install-checkTitle = Einen Download prüfen
install-check = Neben jeder Datei liegt eine <code>.sha256</code>; die Archive tragen zusätzlich eine <code>.sig</code>, die Ed25519-Signatur, die der Client prüft, bevor er ein Update selbst installiert. <code>sha256sum -c</code> unter Linux, <code>shasum -a 256 -c</code> unter macOS, <code>certutil -hashfile … SHA256</code> unter Windows.
install-needsTitle = Was es braucht
install-windows = <b>Windows</b> 10 oder 11, x64 oder ARM64. Der x64-Build braucht eine CPU mit x86-64-v2 (SSE4.2), wie Windows 11 24H2 auch.
install-mac = <b>macOS</b> 11 oder neuer, auf Apple Silicon (M1 und neuer) oder Intel.
install-linux = <b>Linux</b> mit glibc 2.35 oder neuer (ab Ubuntu 22.04), einem Vulkan-Treiber sowie ALSA, udev, X11/Wayland und xkbcommon; das <code>.deb</code> zieht sie mit.
install-browser = <b>Browser:</b> Die Browser-Version pausiert in dieser Beta und kommt später wieder; bis dahin nimm den Desktop-Client oben.
install-phones = <b>Handy und Tablet:</b> noch keine Builds zum Herunterladen. Android läuft, ist aber per Touch nicht bedienbar; iOS läuft nur im Simulator.
install-releases = Jedes Release auf GitHub

## The card status

cards-eyebrow = Karten
cards-title = Die Karten, ehrlich
cards-intro = Jede Karte in Baylee ist Code: eine Datei pro Karte, in einer kleinen Regelsprache geschrieben und in einem Test gespielt. Es gibt keinen Interpreter, der Kartentext ungefähr errät. Deshalb wächst der Pool in Schüben statt auf einmal.
cards-counted = Gezählt im Repository von Baylees eigenen Werkzeugen (<pool>die Pool-Liste</pool>, <readme>die README</readme>). Der Deckbauer kann nur spielbare Karten zeigen und warnt, wenn ein Deck Karten enthält, die nicht wie gedruckt spielen.
cards-planTitle = Der Plan
cards-plan = Die Sets kommen in Erscheinungsreihenfolge, beginnend mit Alpha, weil jedes Set seine Mechaniken auf die älteren setzt. Alpha ist vollständig: Jede spielbare Karte daraus spielt wie gedruckt. Als Nächstes kommen Arabian Nights und Antiquities, der Rest folgt Set für Set. Fehler, die am Tisch auffallen, gehen immer vor neuen Karten.
cards-modern = Das heißt: Ein modernes Deck enthält oft Karten, die Baylee noch nicht kennt. Wir sagen dir das lieber im Deckbauer, als eine Karte falsch verrechnen zu lassen. Karten, die gar nicht spielbar sind (Ante, Geschicklichkeit, Teilspiele), stehen auf einer benannten Auslassungsliste und fallen nie stillschweigend weg.
cards-implemented = Karten, die genau wie gedruckt spielen
cards-partial = weitere, teilweise umgesetzt und so markiert
cards-corpus = Karten im gesamten Korpus, auf den wir hinarbeiten

sets-title = Sets und Kartenstatus
sets-lead = Jedes Set in Erscheinungsreihenfolge, jede Karte unter dem Set gezählt, in dem sie zuerst erschien. Öffne ein Set, um seine Karten zu sehen: ob jede wie gedruckt spielt, spielt, aber noch ungetestet ist, teilweise spielt, noch nicht spielbar ist oder nie gebaut wird. Lass den Zeiger auf einer Karte ruhen oder fokussiere sie, um sie groß mit dem Grund zu sehen; ein Klick öffnet ihre Seite bei Scryfall, mit Künstler und Druck. Auf einem Touchscreen zeigt ein Tippen sie zuerst groß.
sets-legend = Legende
sets-legendFull = Voll: spielt wie gedruckt
sets-legendUntested = Ungetestet: spielbar, jeder Satz wird von der Regel-Engine gelesen, aber noch kein Test spielt sie
sets-legendPartial = Teilweise: spielt, ein Teil fehlt (der Grund steht bei der Karte)
sets-legendNone = Noch nicht: noch nicht spielbar
sets-legendNever = Nie: Ante, Geschicklichkeit oder ein Teilspiel; ein Set gilt ohne sie als vollständig
sets-percent = Der Prozentwert ist der Anteil spielbarer Karten (voll und ungetestet) an allen Karten des Sets, die je gebaut werden. Teilweise umgesetzte Karten sind der dritte Teil des Balkens und zählen nicht als fertig.
sets-loading = Lade die Set-Liste …
sets-failed = Die Set-Liste wurde gerade nicht geladen. Lade die Seite neu, um es noch einmal zu versuchen.
sets-hide = Vollständige Liste ausblenden
sets-showAll =
    { $n ->
        [one] Das { $n } Set anzeigen
       *[other] Alle { $n } Sets anzeigen
    }
sets-imagesNote = Kartenbilder werden erst beim Öffnen eines Sets oder beim Großzeigen einer Karte und unverändert von Scryfall geladen. Kartendaten und -bilder bereitgestellt von Scryfall; Baylee ist nicht mit Scryfall verbunden. Die Zahlen misst <code>baylee-tools site card-status</code> im Repository.
sets-ub = Kartenbilder © Wizards of the Coast, unverändert von Scryfall geladen. Karten, die noch nicht in Baylees Pool sind, und Universes-Beyond-Karten stehen nur mit Namen hier.
sets-total =
    { $play } von { $all ->
        [one] { $all } Karte
       *[other] { $all } Karten
    } spielbar ({ $pct } %): { $full } voll, { $untested } ungetestet. Außerdem: { $partial } teilweise, { $none } noch nicht, { $never } nie; { $sets ->
        [one] { $sets } Set
       *[other] { $sets } Sets
    }, { $corpus ->
        [one] { $corpus } Karte
       *[other] { $corpus } Karten
    }.
sets-atRelease = Release { $release }, Commit { $rev }.
sets-atCommit = Gemessen an Commit { $rev }.
sets-finished = Fertig
sets-next = Als Nächstes, in Erscheinungsreihenfolge
sets-find = Set suchen
sets-onlyBegun = nur begonnene Sets
sets-foundNone = Kein Set passt.
sets-foundMany =
    { $n ->
        [one] { $n } Set passt; die ersten { $max } stehen hier. Tippe mehr, um einzugrenzen.
       *[other] { $n } Sets passen; die ersten { $max } stehen hier. Tippe mehr, um einzugrenzen.
    }
sets-found =
    { $n ->
        [one] { $n } Set passt.
       *[other] { $n } Sets passen.
    }
sets-eraMeta =
    { $sets ->
        [one] { $sets } Set
       *[other] { $sets } Sets
    } · { $cards ->
        [one] { $cards } Karte
       *[other] { $cards } Karten
    }
sets-cards =
    { $n ->
        [one] { $n } Karte
       *[other] { $n } Karten
    }
sets-cardsFailed = Die Kartenliste wurde nicht geladen. Schließe das Set und öffne es erneut.
sets-onScryfall = Dieses Set bei Scryfall
sets-images = Bilder: Scryfall · © Wizards of the Coast
sets-pct = { $pct } %
sets-pctPlayable = { $pct } Prozent spielbar

count-playable = { $n } spielbar:
count-full = { $n } voll
count-untested = { $n } ungetestet
count-partial = { $n } teilweise
count-none = { $n } noch nicht
count-never = { $n } nie

filter-label = Karten zeigen
filter-all = Alle
status-full = Voll
status-untested = Ungetestet
status-partial = Teilweise
status-none = Noch nicht
status-never = Nie

grid-hint = Pfeiltasten wechseln die Karte; die Leertaste zeigt sie groß, Enter öffnet sie bei Scryfall.
grid-label = Karten aus { $set }
grid-tile = { $status }. { $why }
grid-scryfall = Künstler und Druck bei Scryfall

card-credit = Kartenbild bereitgestellt von Scryfall, unverändert. Baylee ist weder mit Scryfall noch mit Wizards of the Coast verbunden.
card-illustrated = Illustriert von { $artist }
card-clickHint = Klick: Künstler und Druck bei Scryfall.
card-implemented = Spielt wie gedruckt.
card-generated = Spielbar: jeder Satz wird von der Regel-Engine gelesen, aber noch kein Test spielt sie.
card-partial = Teilweise umgesetzt. Es fehlt (Notiz auf Englisch): { $note }
card-unimplemented = Im Kartenpool, aber ihre Fähigkeiten sind noch nicht geschrieben: nicht spielbar.
card-absent = Noch nicht im Kartenpool.
card-never = Wird nie gebaut: { $reason }
card-neverAnte = sie spielt um Ante (CR 407), und kein Tisch hier spielt um Ante
card-neverDexterity = sie verlangt eine körperliche Handlung, die kein Spielzustand beschreibt
card-neverSubgame = sie startet ein Spiel im Spiel (CR 728), das die Engine nicht ausrichtet

## The gateway directory

gw-eyebrow = Online
gw-title = Gateways
gw-lead = Ein Gateway ist ein Server für Online-Tische: Accounts, Decks, Räume und ein Engine-Prozess pro Spiel. Hier stehen die bei Baylee Central registrierten; dessen Betreiber hat bei jedem geprüft, wer es betreibt, bevor er es aufgenommen hat. Baylee betreibt Sanctuary; die anderen betreiben andere für ihre Runde, und dort gelten deren eigene Regeln.
gw-howTo = Um auf einem zu spielen, kopiere seine Adresse in die Eingangstür des Clients. Online heißt, Central hat in den letzten anderthalb Minuten von dem Gateway gehört; ein Versprechen, dass es antwortet, ist das nicht. Namen und Versionen sind die Angaben der Betreiber.
gw-count =
    { $n ->
        [one] { $n } Gateway gelistet, { $online } online.
       *[other] { $n } Gateways gelistet, { $online } online.
    }
gw-none = Noch ist kein Gateway gelistet.
gw-next = Die nächsten { $n } zeigen
gw-tryAgain = Erneut versuchen
gw-unavailable = Baylee Central kann die Gateways gerade nicht auflisten.
gw-busy = Baylee Central ist ausgelastet; bitte versuch es gleich noch einmal.
gw-refused = Baylee Central hat die Anfrage dieser Seite nicht beantwortet.
gw-malformed = Die Antwort von Baylee Central hat die Prüfungen dieser Seite nicht bestanden, deshalb wird nichts davon gezeigt.
gw-network = Baylee Central war nicht erreichbar.
gw-online = online
gw-notSeen = zuletzt nicht gesehen
gw-official = Offiziell
gw-address = Adresse
gw-version = Version
gw-copy = Adresse kopieren
gw-website = Website
gw-copyImpossible = Kopieren geht in diesem Browser nicht.
gw-copied = Adresse kopiert.
gw-copyFailed = Kopieren hat nicht geklappt.
gw-open = In Baylee öffnen
gw-openHint = Öffnet den Baylee-Client auf diesem Gateway, sobald dein Client Gateway-Links kennt. Bis dahin kopiere die Adresse in seine Eingangstür.
gw-copyShort = Kopieren
gw-cardLabel = Gateway { $name }

selfhost-eyebrow = Selbst hosten
selfhost-title = Ein eigenes Gateway betreiben
selfhost-processes = Du brauchst unseren Server nicht. Ein Gateway sind drei Arten von Prozessen: <code>baylee-gateway</code> verwaltet Accounts, Decks, Räume und das Routing und braucht PostgreSQL; <code>baylee-agent</code> startet pro Spiel einen Engine-Prozess; <code>baylee-engine-server</code> ist ein Spiel. Die ersten beiden kennen keine Regeln.
selfhost-proxy = Setz einen TLS-Proxy davor, setz <code>BAYLEE_REGISTRATION=invite</code>, wenn du einen eigenen geschlossenen Kreis willst, erzeuge Schlüssel mit <code>baylee-gateway invite create</code>, und die Eingangstür des Clients nimmt deine Adresse wie jede andere. Kartentext in 19 Sprachen ist ein optionaler Import von Scryfalls Bulk-Daten in dieselbe Datenbank.
selfhost-guide = <guide>Die Anleitung</guide> geht alles durch; <protocol>docs/protocol.md</protocol> ist die Referenz.
selfhost-source = Jedes Gateway beantwortet <code>GET /source</code> mit Lizenz, Version und Commit, ohne Anmeldung. Die AGPL verlangt, dass du den Leuten, die deinen Server nutzen, den Quellcode anbietest; diese Route erledigt das für dich.

## Contact

contact-eyebrow = Kontakt
contact-title = Schreib uns
contact-intro = Eine Frage, eine Idee, ein Spiel, das du gern sehen würdest, oder etwas, das du lieber nicht öffentlich schreibst: Dieses Formular geht direkt an die Leute hinter Baylee. Lass eine E-Mail-Adresse da, wenn du eine Antwort willst; ohne können wir es nur lesen.
contact-f8 = Eine Karte, die falsch verrechnet wurde, meldest du am besten im Spiel mit <kbd>F8</kbd>, weil der Bericht das ganze Spiel mitnehmen kann.
contact-discord = reden, Gegner finden, von Builds hören
contact-issues = GitHub-Issues
contact-issuesSub = Fehler und Ideen, öffentlich
contact-operator = Wer diese Seite betreibt, mit Postanschrift: das <impressum>Impressum</impressum>.

form-name = Name
form-optional = freiwillig
form-email = E-Mail
form-emailOptional = freiwillig, nur wenn du eine Antwort willst
form-message = Nachricht
form-privacy = Ich speichere deine Nachricht, und Namen und E-Mail-Adresse nur, wenn du sie angibst, um sie zu lesen und zu beantworten: keine IP-Adresse, kein Cookie, kein Tracker. Sie wird 90 Tage, nachdem ich sie erledigt habe, gelöscht, spätestens nach einem Jahr. <more>Mehr in der Datenschutzerklärung</more>.
form-unreachable = Das Formular hat uns gerade nicht erreicht. Bitte versuch es später noch einmal, oder schreib auf Discord oder GitHub.
form-empty = Bitte schreib zuerst eine Nachricht.
form-badEmail = Diese E-Mail-Adresse sieht nicht richtig aus. Lass sie leer, wenn du keine Antwort willst.
form-sending = Wird gesendet …
form-sent = Gesendet.
form-tooMany = Zu viele Nachrichten von hier in der letzten Stunde. Bitte versuch es später noch einmal.
form-refused = Die Nachricht wurde nicht angenommen. Ist sie kürzer als { $max } Zeichen und die Adresse eine echte?
send-send = Senden
send-sent = Gesendet
send-retry = Erneut senden
thanks-title = Danke, sie ist unterwegs.
thanks-answer = Wenn du eine Adresse dagelassen hast, kommt die Antwort von einem Menschen, per Mail.
thanks-again = Noch eine schreiben

reports-eyebrow = Berichte
reports-title = Etwas gefunden? Sag es uns
reports-f8 = Drück <kbd>F8</kbd> am Tisch oder in der Lobby. Ein Bericht enthält seine Art, deinen Text, die Client-Version und am Tisch die Spiel-ID. Alles Weitere hat ein eigenes Kästchen, und jedes Kästchen ist anfangs angehakt; nimm vor dem Senden das Häkchen weg, wo du etwas lieber behältst: System und Hardware, der Tisch wie du ihn siehst, dein Protokoll (andere Spieler als „Player A“, „Player B“, …), deine Einstellungen, ein Bild nur des Baylee-Fensters und am Tisch die ganze Aufzeichnung des Spiels (jeder Zug und die Karten jedes Platzes, keine Namen). <i>Zeigen, was gesendet wird</i> zeigt jedes Byte. Sitzungs-Tokens werden nie gesendet.
reports-usage = Beim Offline-Spielen sagt der Client einmal pro Minute, dass ein Spiel läuft: kein Account, keine Geräte-ID, nichts über das Spiel. <i>Anonyme Nutzungszahl senden</i> unter Einstellungen › Datenschutz & Daten schaltet das ab.
reports-discord = Lust zu reden, Gegner zu finden oder als Erstes von neuen Builds zu erfahren? Komm in die <discord>Baylee-Community auf Discord</discord>.
reports-issues = Regelfragen, falsches Kartenverhalten, Ideen: <issues>eröffne ein Issue auf GitHub</issues>. Eine Karte, die falsch verrechnet wurde, ist der nützlichste Bericht, den wir bekommen können, denn jede Karte hat einen Test, und deine wird einer.
reports-privacy = <docs>docs/privacy.md</docs> listet alles, was Client, Gateway und Feedback-Dienst speichern. Diese Seite speichert nichts über dich: keine Cookies, keine Analyse, keine Schriften von fremden Servern. <notice>Die Datenschutzerklärung</notice> nennt jeden anderen Host, mit dem sie spricht, und wann.

## The legal notice (Impressum)

imp-eyebrow = Impressum
imp-title = Impressum
imp-ddg = Angaben gemäß § 5 DDG
imp-private = Diese Website ist ein privates, nicht kommerzielles Projekt einer Privatperson. Baylee ist kostenlos und quelloffen; es gibt keine Werbung und nichts zu kaufen.
imp-write = Du erreichst mich außerdem über das <contact>Kontaktformular</contact>.
imp-disputeTitle = Streitbeilegung
imp-dispute = Dies ist ein kostenloses, nicht kommerzielles Angebot einer Privatperson. Ich bin weder verpflichtet noch bereit, an Streitbeilegungsverfahren vor einer Verbraucherschlichtungsstelle teilzunehmen.
imp-data = Wie diese Seite mit deinen Daten umgeht: <notice>die Datenschutzerklärung</notice>. Was Client, Gateway und Feedback-Dienst über dich speichern: <docs>docs/privacy.md</docs>.
imp-why = Damit Adresssammler leer ausgehen, werden die Angaben erst auf deinen Wunsch geladen; dein Browser löst dafür ein kleines Rätsel, das einen Moment dauert.
imp-show = Impressum anzeigen
imp-puzzle = Rätsel
imp-working = Einen Moment, dein Browser löst ein kleines Rätsel …
imp-failed = Die Angaben konnten nicht geladen werden. Bitte lade die Seite neu oder nutze das <contact>Kontaktformular</contact>.
imp-drawn = Die Anschrift ist so gezeichnet, dass Adresssammler sie nicht lesen können, und deshalb auch kein Screenreader. Der Button darunter kopiert sie in die Zwischenablage; oder schreib mir über das Kontaktformular.
imp-copyAddress = Anschrift kopieren
imp-contact = Kontakt
imp-email = E-Mail
imp-writeEmail = E-Mail schreiben
imp-copyEmail = E-Mail-Adresse kopieren
imp-phone = Telefon
imp-call = Anrufen
imp-copyNumber = Nummer kopieren
imp-copyImpossible = Kopieren geht in diesem Browser nicht; bitte nutze das Kontaktformular.
imp-copied = Kopiert.
imp-copyFailed = Kopieren hat nicht geklappt; bitte nutze das Kontaktformular.

## The terms of use (their text is legal/terms.de.md)

terms-eyebrow = Nutzungsbedingungen
terms-draft = Entwurf: Diese Bedingungen gelten noch nicht. Sie treten erst mit ihrer Veröffentlichung in Kraft, und du wirst gefragt, ob du sie annimmst.
terms-version = Fassung { $version }.
terms-updated = Stand: { $date }.

## The privacy notice: a draft for the owner's lawyer (src/privacy/)

privacy-eyebrow = Datenschutz
privacy-title = Datenschutzerklärung dieser Website
privacy-lead = Diese Seite sagt, was mit deinen Daten geschieht, wenn du diese Website besuchst. Das Spiel selbst (Client und Online-Gateway) führt eigene Aufzeichnungen; die vollständige Liste steht in <docs>docs/privacy.md</docs>.
privacy-contents = Inhalt
privacy-date = Stand: 21. Oktober 2026.
privacy-controllerTitle = Verantwortlich
privacy-controller = Verantwortlich ist die im <impressum>Impressum</impressum> genannte Privatperson. Du erreichst mich dort oder über das <contact>Kontaktformular</contact>.
privacy-hostingTitle = Hosting und Server
privacy-hosting = Die Website läuft auf einem bei der netcup GmbH gemieteten Server in Deutschland. Wenn du eine Seite öffnest, schickt dein Browser dem Server, was jede Anfrage mitbringt: deine IP-Adresse, die angefragte Adresse, die Uhrzeit und was dein Browser über sich sagt. Der Server braucht das, um zu antworten. Der Webserver führt für diese Website <b>kein Zugriffsprotokoll</b>; diese Anfragen werden also nicht aufgeschrieben. Die Betriebsmeldungen des Servers (sein Systemjournal) können in seltenen Fehlerfällen technische Verbindungsdaten wie eine IP-Adresse enthalten; sie bleiben nur so lange, wie das Journal sie aufbewahrt.
privacy-hostingBasis = Rechtsgrundlage: Art. 6 Abs. 1 lit. f DSGVO, mein berechtigtes Interesse an einer sicheren Auslieferung der Website.
privacy-storageTitle = Kein Tracking
privacy-storage = Diese Website nutzt keine Analyse, kein Tracking und keine Werbung, und sie setzt kein Cookie, außer du meldest dich an (siehe das Sitzungs-Cookie unten). Die Schriften kommen von diesem Server; kein Schriftendienst wird gefragt. Dein Browser speichert ein paar Dinge auf deinem Gerät, die es nie verlassen: die gewählte Sprache (bis du sie löschst); zehn Minuten lang in diesem Tab GitHubs Antworten zum neuesten Release, damit die Seite nicht bei jedem Besuch neu fragt; und solange du angemeldet bist, wann deine Sitzung endet.
privacy-storageBasis = Rechtsgrundlage für diese Speicherung: § 25 Abs. 2 Nr. 2 TDDDG; sie ist für das, was du von der Seite willst, unbedingt erforderlich.
privacy-githubTitle = GitHub: Releases und Projektstand
privacy-github = Um das neueste Release, seine Dateien und den Stand des Projekts zu zeigen, fragt dein Browser beim Laden der Startseite oder der Download-Seite GitHubs öffentliche API (<code>api.github.com</code>), und ein Download kommt direkt von GitHub. Dein Browser schickt dabei deine IP-Adresse und die Anfrage an GitHub, Inc. in den USA. Siehe <statement>GitHubs Datenschutzerklärung</statement>.
privacy-githubBasis = Rechtsgrundlage: Art. 6 Abs. 1 lit. f DSGVO, mein berechtigtes Interesse, aktuelle Downloads zu zeigen, ohne sie selbst zu hosten.
privacy-scryfallTitle = Kartenbilder von Scryfall
privacy-scryfall = Kartenbilder liegen nicht hier. Erst wenn du im Kartenstatus ein Set öffnest oder eine Karte groß zeigst, lädt dein Browser ihre Bilder direkt von Scryfalls Bildserver (<code>cards.scryfall.io</code>) und schickt dabei deine IP-Adresse und die Anfrage an Scryfall, LLC in den USA. Ein Referrer wird nicht mitgeschickt. Siehe <policy>Scryfalls Datenschutzerklärung</policy>.
privacy-scryfallBasis = Rechtsgrundlage: Art. 6 Abs. 1 lit. f DSGVO, mein berechtigtes Interesse, die Karten zu zeigen, von denen der Status spricht.
privacy-centralTitle = Das Gateway-Verzeichnis
privacy-central = Wenn du das <directory>Gateway-Verzeichnis</directory> öffnest, fragt dein Browser Baylee Central, den Account-Dienst des Projekts, über die eigene Adresse dieser Website nach der öffentlichen Liste der Gateways. Die Anfrage trägt kein Cookie, keinen Account und keine Kennung von dir; Central antwortet allen mit derselben Liste. Sonst spricht die Website nur mit Central, wenn du dich registrierst, anmeldest oder deinen Account öffnest (unten).
privacy-centralBasis = Rechtsgrundlage: Art. 6 Abs. 1 lit. f DSGVO, mein berechtigtes Interesse, zu zeigen, wo man Baylee online spielen kann.
privacy-contactTitle = Das Kontaktformular
privacy-contact = Was du sendest, geht an Baylees Feedback-Dienst (<code>{ $host }</code>) und wird dort gespeichert: deine Nachricht, Name und E-Mail-Adresse nur, wenn du sie angibst, die Sprache der Seite, die Uhrzeit und ob ich sie erledigt habe. Deine IP-Adresse wird nicht gespeichert; eine Stunde lang wird sie nur im Arbeitsspeicher gezählt, um eine Flut von Nachrichten zu verhindern. Nur ich lese die Nachrichten; nichts wird weitergegeben. Eine Nachricht wird 90 Tage, nachdem ich sie erledigt habe, gelöscht, spätestens ein Jahr nach ihrem Eingang, oder früher, wenn du darum bittest.
privacy-contactBasis = Rechtsgrundlage: Art. 6 Abs. 1 lit. f DSGVO, mein berechtigtes Interesse, Nachrichten zu beantworten; zielt deine Nachricht auf einen Vertrag mit mir, Art. 6 Abs. 1 lit. b DSGVO.
privacy-impressumTitle = Wie das Impressum geschützt ist
privacy-impressum = Damit Adresssammler meine Angaben nicht einsammeln, zeigt das Impressum sie, nachdem dein Browser ein kleines Rechenrätsel gelöst hat. Dafür hält der Feedback-Dienst nur im Arbeitsspeicher, nie auf der Festplatte und nie in einem Protokoll: wie viele Rätsel deine IP-Adresse angefragt hat (eine Stunde lang; eine IPv6-Adresse nach ihrer ersten Hälfte), ob sie einem für Menschen versteckten Link gefolgt ist (24 Stunden), und welche Rätsel gelöst wurden (fünf Minuten, ohne Adresse). Drei Ja/Nein-Antworten über deinen Browser bestimmen die Schwierigkeit und werden nicht aufbewahrt. Niemand wird gesperrt: Im ungünstigsten Fall dauert das Rätsel länger.
privacy-impressumBasis = Rechtsgrundlage: Art. 6 Abs. 1 lit. f DSGVO, mein berechtigtes Interesse, meine Angaben vor Missbrauch zu schützen.
privacy-linksTitle = Links zu anderen Seiten
privacy-links = Discord, GitHub, Scryfall, Wizards of the Coast und die Websites gelisteter Gateways sind verlinkt, nicht eingebettet: An sie geht nichts, bis du einem Link folgst; dann gelten deren eigene Datenschutzbestimmungen.
privacy-rightsTitle = Deine Rechte
privacy-rights = Du hast das Recht auf Auskunft (Art. 15 DSGVO), Berichtigung (Art. 16), Löschung (Art. 17), Einschränkung der Verarbeitung (Art. 18) und Datenübertragbarkeit (Art. 20), und du kannst einer Verarbeitung nach Art. 6 Abs. 1 lit. f jederzeit widersprechen (Art. 21). Schreib mir dafür über das Kontaktformular oder an die Anschrift im Impressum.
privacy-complain = Außerdem kannst du dich bei einer Datenschutz-Aufsichtsbehörde beschweren (Art. 77 DSGVO), etwa bei der an deinem Wohnort oder bei der für mich zuständigen: der <ldi>Landesbeauftragten für Datenschutz und Informationsfreiheit Nordrhein-Westfalen (LDI NRW)</ldi>.
privacy-automated = Eine automatisierte Entscheidungsfindung im Sinne von Art. 22 DSGVO findet nicht statt: Die Schwierigkeit des Rätsels passt sich selbst an, entscheidet aber nichts über dich.

## Accounts bei Baylee Central (src/account/)

acct-eyebrow = Baylee Central
acct-username = Benutzername
acct-password = Passwort
acct-errBusy = Zu viele Versuche von hier. Bitte warte eine Weile und versuch es dann noch einmal.
acct-errUnreachable = Baylee Central hat nicht geantwortet. Bitte versuch es gleich noch einmal.
acct-errCrypto = Dieser Browser konnte die Verschlüsselung des Accounts nicht ausführen: Sie braucht WebAssembly und Web Workers. Bitte nimm einen aktuellen Browser.
acct-errMismatch = Die Antwort von Baylee Central war nicht stimmig, darum wurde nichts übernommen. Bitte versuch es noch einmal; passiert es wieder, sag uns bitte Bescheid.
register-title = Leg deinen Account an
register-lead = Ein Account für Baylee: Mit ihm meldest du dich im Baylee-Client an und auf den Gateways, die Baylee Central nutzen. Solange Baylee in der Beta ist, brauchst du zum Registrieren einen Beta-Key.
register-betaKey = Beta-Key
register-betaKeyHint = So, wie du ihn bekommen hast, zum Beispiel CENTRAL-XXXX-XXXX-XXXX-XXXX.
register-usernameRules = 3 bis 24 Zeichen: Buchstaben a–z, Ziffern und dazwischen . _ -, nie zwei davon hintereinander. Großbuchstaben werden als kleine gelesen. Nur du siehst ihn: Mit ihm meldest du dich an. Auf einem Gateway sehen andere Spieler einen Anzeigenamen und ein kurzes Kürzel.
register-displayName = Anzeigename
register-displayNameRules = So sehen dich andere Spieler: 3 bis 16 Buchstaben, Ziffern, - und _. Dein Kürzel wird für dich ausgelost.
register-password = Passwort
register-passwordAgain = Passwort wiederholen
register-passwordHint = Mindestens 8 Zeichen, und keine Regeln, welche. Eine Passphrase aus mehreren Wörtern, die nichts miteinander zu tun haben, ist stark und leicht zu merken. Dein Passwort verlässt diesen Browser nie.
register-strengthShort = Zu kurz
register-strengthWeak = Schwach: leicht zu erraten
register-strengthFair = Mittel: länger wäre stärker
register-strengthStrong = Stark
register-age = Ich bin mindestens 16 Jahre alt.
register-consent = Ich nehme die <terms>Nutzungsbedingungen</terms> von Baylee Central an und habe die <privacy>Datenschutzerklärung</privacy> gelesen.
register-submit = Account anlegen
register-submitting = Wird angelegt …
register-haveAccount = Schon einen Account? <signin>Anmelden</signin>.
register-progressKey = Der Beta-Key wird geprüft …
register-progressSecuring = Dein Passwort wird abgesichert. Das dauert ein paar Sekunden …
register-progressSaving = Dein Account wird gespeichert …
register-errKey = Bitte gib deinen Beta-Key ein.
register-errUsername = Dieser Benutzername folgt den Regeln oben nicht.
register-errDisplayName = Dieser Anzeigename folgt den Regeln oben nicht.
register-errDisplayNameReserved = Dieser Anzeigename ist Baylee selbst vorbehalten. Bitte wähl einen anderen.
register-errPassword = Das Passwort braucht mindestens 8 Zeichen (und höchstens { $max } Bytes).
register-errMismatch = Die beiden Passwörter sind verschieden.
register-errAge = Bitte bestätige, dass du mindestens 16 Jahre alt bist.
register-errConsent = Bitte nimm die Nutzungsbedingungen an.
register-errBadKey = Dieser Beta-Key ist ungültig, aufgebraucht oder abgelaufen.
register-errTaken = Dieser Benutzername ist vergeben. Bitte wähl einen anderen.
recovery-title = Dein Wiederherstellungscode
recovery-lead = Dein Account ist bereit. Falls du je dein Passwort verlierst, öffnet dieser Code deine verschlüsselten Daten. Er wird nur jetzt gezeigt: Niemand, auch Baylee Central nicht, kann ihn dir noch einmal zeigen.
recovery-label = Wiederherstellungscode
recovery-copy = Kopieren
recovery-copied = Kopiert.
recovery-copyFailed = Kopieren hat nicht geklappt. Bitte markier den Code und kopier ihn von Hand.
recovery-download = Als Textdatei herunterladen
recovery-fileName = baylee-wiederherstellungscode.txt
recovery-file =
    Baylee Central: der Wiederherstellungscode für { $name }

    { $code }

    Wenn du dein Passwort verlierst, öffnet dieser Code die verschlüsselten Daten deines Accounts.
    Bewahr ihn sicher und privat auf, nicht neben deinem Passwort.
    Niemand, auch Baylee Central nicht, kann ihn dir noch einmal zeigen.
recovery-keep = Schreib ihn auf oder leg die Datei sicher und offline ab, nicht neben deinem Passwort.
recovery-confirm = Ich habe meinen Wiederherstellungscode sicher aufbewahrt.
recovery-continue = Weiter
recovery-leadMoved = Hier ist ein Wiederherstellungscode für deinen Account, der noch keinen hat. Falls du je dein Passwort verlierst, öffnet dieser Code deine verschlüsselten Daten. Er wird nur jetzt gezeigt und gilt erst, wenn du ihn aufbewahrt und auf Weiter gedrückt hast: Niemand, auch Baylee Central nicht, kann ihn dir noch einmal zeigen.
recovery-handle = Andere Spieler sehen dich als <b>{ $handle }</b>.
recovery-later = Jetzt nicht
recovery-saving = Der Code wird für deinen Account gespeichert …
recovery-errSave = Baylee Central hat nicht geantwortet, darum ist der Code vielleicht noch nicht gespeichert. Behalte ihn und drück auf Weiter, um es noch einmal zu versuchen: Es wird derselbe Code geschickt.
recovery-errRefused = Baylee Central hat diesen Code nicht angenommen, darum gilt er nicht: Wirf ihn bitte weg. Ein Code muss innerhalb von zehn Minuten nach der Anmeldung aufbewahrt werden; melde dich noch einmal an, um einen neuen zu erstellen.
recovery-errTaken = Dein Account hat inzwischen einen anderen Wiederherstellungscode, darum gilt dieser nicht: Wirf ihn bitte weg.
recovery-errUnsure = Deine Sitzung ist abgelaufen, bevor Baylee Central den Code bestätigt hat. Melde dich wieder an: Deine Account-Seite sagt dir dann, ob dein Account einen Wiederherstellungscode hat. Hat er keinen, gilt dieser nicht.
accept-title = Nutzungsbedingungen annehmen
accept-lead = Für deinen Account gilt Fassung { $version } der Nutzungsbedingungen von Baylee Central. Bitte lies sie und nimm sie an, um ihn zu nutzen.
accept-loading = Die Nutzungsbedingungen werden geladen …
accept-accept = Ich nehme sie an
accept-later = Nicht jetzt (abmelden)
accept-retry = Noch einmal versuchen
accept-mismatch = Die Bedingungen, die Baylee Central geschickt hat, passen nicht zu ihrem Fingerabdruck; darum lassen sie sich hier nicht annehmen. Bitte versuch es später noch einmal.
accept-signedOut = Deine Sitzung ist abgelaufen. <signin>Melde dich wieder an</signin>, um die Bedingungen anzunehmen.
accept-declined = Du bist abgemeldet. <signin>Melde dich wieder an</signin>, sobald du die Bedingungen annehmen möchtest.
signin-title = Anmelden
signin-submit = Anmelden
signin-submitting = Wird angemeldet …
signin-progress = Du wirst angemeldet. Das dauert ein paar Sekunden …
signin-errName = Bitte gib deinen Benutzernamen ein.
signin-errPassword = Bitte gib dein Passwort ein.
signin-errWrong = Benutzername oder Passwort stimmt nicht.
signin-legacy = Schon auf dem offiziellen Gateway gespielt? Dann melde dich dort einmal mit dem Baylee-Client an: Das zieht deinen Account zu Baylee Central um, und ab dann klappt es auch hier.
signin-noAccount = Noch keinen Account? <register>Leg einen an</register>.
signin-already = Du bist als <b>{ $name }</b> angemeldet. <account>Zu deinem Account</account>
signin-alreadyUnnamed = Du bist angemeldet. <account>Zu deinem Account</account>
signin-registered = Dein Account ist bereit. Bitte melde dich an.
account-title = Dein Account
account-signedInAs = Angemeldet als
account-signedIn = Angemeldet
account-signInName = Anmeldename: { $name }
account-chooseName = Wähl deinen Anzeigenamen
account-chooseNameLead = Andere Spieler sehen ihn auf den Gateways neben deinem Kürzel, #{ $tag }: 3 bis 16 Buchstaben, Ziffern, - und _, an beiden Enden ein Buchstabe oder eine Ziffer. Du wählst ihn einmal; ändern kommt später.
account-chooseNameSubmit = Namen speichern
account-chooseNameSaving = Der Name wird gespeichert …
account-errNameRefused = Baylee Central hat diesen Namen nicht angenommen. Bitte wähl einen anderen; passiert es wieder, melde dich noch einmal an.
account-errNameTaken = Dein Account hat inzwischen einen Anzeigenamen, darum wurde dieser nicht gespeichert.
account-errNameTerms = Bitte nimm zuerst die Nutzungsbedingungen an.
account-nameFromGateway = Dein Anzeigename und dein Kürzel kommen vom offiziellen Gateway mit: Melde dich dort einmal mit dem Baylee-Client an, dann erscheinen sie hier.
account-session = In diesem Browser bleibst du bis { $time } angemeldet, wenn du dich nicht vorher abmeldest.
account-client = Zum Spielen meldest du dich mit demselben Namen und Passwort im Baylee-Client an (<download>Download</download>).
account-recovery = Du hast einen Wiederherstellungscode. Mit ihm überstehen deine verschlüsselten Daten (Einstellungen, Deck-Sicherungen, Chatverlauf) ein vergessenes Passwort, sobald es das Zurücksetzen gibt. Bewahre ihn so auf, dass nur du ihn erreichst: Wir können ihn dir nicht noch einmal zeigen.
account-noRecovery = Dein Account hat noch keinen Wiederherstellungscode. Er ist vom offiziellen Gateway hierher umgezogen, das keinen kannte. Vergisst du ohne Code dein Passwort, sind deine verschlüsselten Daten verloren.
account-sourceRegistered = Du hast diesen Account hier auf baylee.app angelegt.
account-sourceLegacy = Dieser Account ist vom offiziellen Gateway hierher umgezogen.
account-makeRecovery = Wiederherstellungscode erstellen
account-recoverySignIn = Um den Code zu erstellen, melde dich noch einmal an
account-recoveryCancel = Abbrechen
account-later = Passwort ändern und Account löschen kommen hier später. Bis dahin schreib uns über das <contact>Kontaktformular</contact>, wenn wir deinen Account löschen sollen.
account-signOut = Abmelden
account-signOutUnconfirmed = Baylee Central war nicht erreichbar, um die Sitzung zu beenden; dieser Browser behält sie darum bis { $time }. An einem geteilten Rechner schließ jetzt den Browser.
account-notSignedIn = Du bist nicht angemeldet. <signin>Melde dich an</signin> oder <register>leg einen Account an</register>.
privacy-accountTitle = Dein Account bei Baylee Central
privacy-account = Wenn du einen Account anlegst, speichert Baylee Central, der Account-Dienst des Projekts: deinen Benutzernamen; den Anzeigenamen, den du wählst, und das für ihn ausgeloste Kürzel, die andere Spieler auf den Gateways sehen, bei denen du dich anmeldest; einen Anmeldedatensatz des OPAQUE-Protokolls, aus dem sich dein Passwort nicht lesen lässt und der es nie enthält; den Hauptschlüssel deines Accounts, einmal unter deinem Passwort und einmal unter deinem Wiederherstellungscode verschlüsselt, beides für Central nicht entschlüsselbar; dein Account-Dokument, das schon in deinem Browser verschlüsselt wird, bevor es gesendet wird; welche Fassung der Nutzungsbedingungen von Central du wann angenommen hast; und deine Sitzungen, solange sie laufen. Dein Passwort und dein Wiederherstellungscode verlassen deinen Browser nie. Das Registrieren verbraucht eine Nutzung eines Beta-Keys; Central speichert Beta-Keys nur als Prüfsumme. Nichts davon geht an andere: Es gibt keine Dritten.
privacy-accountBasis = Rechtsgrundlage: Art. 6 Abs. 1 lit. b DSGVO, der Account, den du angelegt hast. Gespeichert wird bis zur Löschung des Accounts; bis du die selbst auslösen kannst, schreib mir.
privacy-cookieTitle = Das Sitzungs-Cookie
privacy-cookie = Wenn du dich anmeldest, setzt Baylee Central ein Cookie, <code>__Host-central</code>: eine zufällige Sitzungskennung, nur für diese Website, nur über HTTPS und nur mit Anfragen dieser Website gesendet, für die Skripte der Seite nicht lesbar, und weg, sobald du dich abmeldest oder nach 30 Minuten. Es ist das einzige Cookie, und nur das Anmelden setzt es. Damit die Leiste oben auch nach dem Neuladen zeigen kann, dass du angemeldet bist, speichert dein Browser außerdem, wann die Sitzung endet (Local Storage, nur auf deinem Gerät); die Seite fragt dann Baylee Central, wessen Sitzung es ist, und beim Abmelden wird es vergessen.
privacy-cookieBasis = Rechtsgrundlage: § 25 Abs. 2 Nr. 2 TDDDG, unbedingt erforderlich für die Anmeldung, die du wünschst; Art. 6 Abs. 1 lit. b DSGVO.
privacy-countersTitle = Grenzen für Versuche
privacy-counters = Damit niemand Beta-Keys oder Passwörter durchprobiert, zählt Baylee Central Registrierungen (fünf pro Stunde) und Anmeldeversuche (dreißig pro Minute) je IP-Adresse. Diese Zähler liegen nur im Arbeitsspeicher: nie auf der Festplatte, spätestens nach einer Stunde weg und bei jedem Neustart des Dienstes verloren.
privacy-countersBasis = Rechtsgrundlage: Art. 6 Abs. 1 lit. f DSGVO, mein berechtigtes Interesse, Accounts zu schützen.

# Baylee, the little soul this project is dedicated to.
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

# Baylee’s story, approved by her family on 27 October 2026.
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
app-nav = Account und Verwaltung
app-account = Dein Account
app-admin = Verwaltung
app-profile = Profil
app-security = Sicherheit und Sitzungen
app-gateways = Deine Gateways
app-users = Accounts
app-adminGateways = Gateways
app-audit = Protokoll
app-legacy = Alte Konten
app-signedOut = Melde dich bei Baylee Central an, um diese Seite zu sehen.
app-cancel = Abbrechen
app-reauthTitle = Mit deinem Passwort bestätigen
app-reauthWhy = Diese Änderung braucht eine Anmeldung aus den letzten 30 Minuten. Gib dein Passwort noch einmal ein; es verlässt diese Seite nie.
app-reauthGo = Bestätigen
app-secretOnce = Nur jetzt sichtbar. Kopiere es und bewahre es sicher auf: Baylee Central behält nur einen Fingerabdruck davon und kann es dir nicht noch einmal zeigen.
app-secretCopied = Kopiert.
sec-title = Sicherheit und Sitzungen
sec-lead = Baylee Central kennt keine Geräte: Ein Account hat höchstens eine Sitzung auf dieser Website und eine im Client. Hier sind sie, mit den Gateways, bei denen dein Client gerade angemeldet ist.
sec-sessions = Deine Sitzungen
sec-website = Diese Website
sec-client = Der Baylee-Client
sec-current = diese Sitzung
sec-since = Angemeldet { $time }
sec-until = endet um { $time }, wenn sie nicht verlängert wird
sec-gateways = Angemeldet bei:
sec-noGateway = Gerade bei keinem Gateway angemeldet.
sec-clientNone = Der Client ist gerade nicht angemeldet.
sec-endClient = Client abmelden
sec-endTitle = Den Client abmelden?
sec-endText = Die Sitzung des Clients endet jetzt und mit ihr seine Anmeldungen bei allen Gateways. Deine Einstellungen, Decks und Chats bleiben; der Client fragt beim nächsten Mal nach deinem Passwort.
sec-ended = Der Client ist abgemeldet.
sec-signOutTitle = Hier abmelden
sec-signOutText = Beendet die Sitzung dieser Website. Der Client bleibt angemeldet.
gwo-title = Deine Gateways
gwo-lead = Betreibe ein Gateway für deine eigene Runde, lass es Spieler über Baylee Central anmelden und in der öffentlichen Liste erscheinen.
gwo-step1 = Registriere hier die Adresse deines Gateways.
gwo-step2 = Ein Admin von Baylee Central prüft, wer es betreibt, und gibt es frei.
gwo-step3 = Erzeuge ein Token und füge es in der Admin-Konsole deines Gateways unter Einstellungen → Baylee Central ein.
gwo-addTitle = Gateway registrieren
gwo-origin = Adresse
gwo-originHint = Seine https-Adresse ohne Pfad, etwa https://play.example.org
gwo-name = Name in der Liste
gwo-add = Registrieren
gwo-added = Registriert. Es wartet auf die Freigabe durch einen Admin.
gwo-errOrigin = Das ist keine Adresse. Schreib sie als https://dein.server
gwo-errName = Gib deinem Gateway einen Namen.
gwo-errTaken = Diese Adresse ist schon registriert.
gwo-errMany = Du hast schon acht Gateways, mehr kann ein Account nicht registrieren.
gwo-errRefused = Baylee Central hat abgelehnt: bitte eine https-Adresse ohne Pfad und einen Namen ohne Steuerzeichen.
gwo-none = Du hast noch kein Gateway registriert.
gwo-pending = Wartet auf Freigabe
gwo-approved = Freigegeben
gwo-disabled = Gesperrt
gwo-pendingText = Bis ein Admin es freigibt, wird dein Gateway nicht gelistet und seine Tokens melden niemanden an. Seine Konsole kann ein Token aber schon prüfen.
gwo-disabledText = Ein Admin hat dieses Gateway gesperrt: Es wird nicht gelistet und seine Tokens melden niemanden an. Schreib uns über die Kontaktseite, wenn du das für einen Irrtum hältst.
gwo-registered = Registriert
gwo-lastSeen = Letzter Heartbeat
tok-make = Token erzeugen
tok-title = Ein Token für { $gateway }
tok-title2 = Tokens
tok-name = Name
tok-nameHint = Nur für dich, etwa der Rechner, auf den es kommt.
tok-scopes = Erlaubt
tok-sso = Spieler über Baylee Central anmelden
tok-listing = Seinen Eintrag in der öffentlichen Gateway-Liste pflegen
tok-ssoShort = Anmeldung
tok-listingShort = Liste
tok-lifetime = Gültig für
tok-days =
    { $n ->
        [one] einen Tag
       *[other] { $n } Tage
    }
tok-create = Token erzeugen
tok-errName = Gib dem Token einen Namen mit höchstens 64 Zeichen.
tok-errScope = Ein Token muss etwas erlauben.
tok-errBusy = Dieses Gateway hat acht gültige Tokens oder hat heute schon viele bekommen. Widerrufe eins oder versuch es morgen.
tok-errRefused = Baylee Central hat das Token abgelehnt.
tok-madeTitle = Dein neues Token
tok-madeText = Es gilt für { $gateway } bis { $date }, sobald das Gateway freigegeben ist.
tok-label = Gateway-Token
tok-where1 = Öffne die Admin-Konsole deines Gateways.
tok-where2 = Füge das Token unter Einstellungen → Baylee Central ein und speichere.
tok-where3 = Die Konsole zeigt, ob Baylee Central es angenommen hat und was es erlaubt.
tok-kept = Ich habe es gesichert
tok-none = Noch kein Token.
tok-expires = Gültig bis
tok-lastUsed = Zuletzt benutzt
tok-state = Status
tok-actions = Aktionen
tok-live = Gültig
tok-expired = Abgelaufen
tok-revoked = Widerrufen
tok-revoke = Widerrufen
tok-revokeTitle = „{ $name }“ widerrufen?
tok-revokeText = Das Token hört sofort auf zu funktionieren. Ein Gateway, das es benutzt, kann keine Spieler mehr anmelden und seinen Listeneintrag nicht mehr pflegen, bis es ein anderes bekommt.
tok-revokedSaid = Das Token ist widerrufen.
adm-noAccess = Dieser Teil ist für die Admins von Baylee Central.
adm-errConflict = Jemand anderes hat das inzwischen geändert, oder der Tag ist vergeben. Lade neu und versuch es noch einmal.
adm-errRejected = Baylee Central hat die Änderung abgelehnt.
adm-usersTitle = Accounts
adm-usersLead = Alle Accounts, die neuesten zuerst. Suche nach Namen oder mit # nach dem Tag.
adm-accounts = Accounts
adm-search = Suchen
adm-searchHint = Name oder #3fa2
adm-errSearch = Baylee Central konnte nicht suchen. Eine Suche hat 1 bis 64 einfache Zeichen, oder # und einen Tag.
adm-noAccounts = Kein Account passt.
adm-handle = Name
adm-username = Anmeldename
adm-source = Herkunft
adm-legacy = Vom offiziellen Gateway umgezogen
adm-registered = Hier registriert
adm-created = Angelegt
adm-lastSignIn = Letzte Anmeldung
adm-adminPill = Admin
adm-member = Mitglied
adm-more = Mehr zeigen
adm-userTitle = Ein Account
adm-errAccount = Dieser Account ließ sich nicht lesen.
adm-back = Zurück zu den Accounts
adm-tag = Tag
adm-state = Status
adm-state_pending = Reserviert, nicht eingerichtet
adm-state_enrolled = Eingerichtet
adm-state_restored = In Benutzung
adm-recovery = Wiederherstellungscode
adm-yes = Ja
adm-no = Nein
adm-role = Rolle
adm-tagTitle = Tag
adm-tagText = Ein Tag ist bei Baylee Central eindeutig, also auch Name#Tag. Schreib ihn hexadezimal, etwa #1a2b. Jede Änderung steht mit altem und neuem Tag im Protokoll.
adm-tagSave = Tag setzen
adm-tagConfirm = { $name } den Tag #{ $tag } geben?
adm-tagWarn = Gateways übernehmen den neuen Tag bei der nächsten Anmeldung dort. Flüstern wird auf einem Gateway nach Tag geführt, frühere Flüster-Verläufe erscheinen unter dem neuen Tag deshalb unter Umständen nicht.
adm-tagSaved = Der Tag ist gesetzt.
adm-errTag = Ein Tag hat ein bis acht Hex-Ziffern, ab #0001.
adm-actions = Aktionen
adm-makeAdmin = Zum Admin machen
adm-removeAdmin = Admin-Rolle entziehen
adm-makeAdminTitle = { $name } zum Admin machen?
adm-removeAdminTitle = { $name } die Admin-Rolle entziehen?
adm-makeAdminText = Admins sehen alle Accounts und Gateways und können sie ändern.
adm-removeAdminText = Ihre nächste Anfrage als Admin wird abgelehnt.
adm-adminGranted = Der Account ist jetzt Admin.
adm-adminRemoved = Die Admin-Rolle ist entzogen.
adm-reset = Reset-Code erzeugen
adm-resetTitle = Einen Reset-Code für { $name } erzeugen?
adm-resetText = Mit dem Code setzt die Person ein neues Passwort. Gib ihn nur weiter, wenn du sicher bist, wer fragt. Ein neuer Code ersetzt einen älteren.
adm-resetOpen = Ein unbenutzter Reset-Code gilt bis { $time }.
adm-codeTitle = Der Reset-Code
adm-codeText = Gültig bis { $time }, einmal.
adm-ownedGateways = Registrierte Gateways
adm-auditTitle = Letzte Protokolleinträge
agw-title = Gateways
agw-lead = Gib ein Gateway erst frei, wenn du außerhalb von Baylee Central geprüft hast, dass die Person seine Adresse betreibt. Sperren nimmt es aus der Liste und beendet die Anmeldungen seiner Spieler dort.
agw-listTitle = Registrierte Gateways
agw-filter = Zeigen
agw-all = Alle
agw-none = Kein Gateway in diesem Zustand.
agw-owner = Inhaber
agw-noOwner = die Bridge (offiziell)
agw-tokens = Gültige Tokens
agw-pair = und ein Credential-Paar
agw-approve = Freigeben
agw-enable = Wieder freigeben
agw-disable = Sperren
agw-showTokens = Tokens zeigen
agw-hideTokens = Tokens ausblenden
agw-approveTitle = { $name } freigeben?
agw-approveText = Gib nur frei, wenn du geprüft hast, dass die Person { $origin } betreibt. Freigegeben erscheint es in der öffentlichen Liste und meldet mit seinen Tokens Spieler an.
agw-disableTitle = { $name } sperren?
agw-disableText = { $origin } verschwindet sofort aus der öffentlichen Liste, seine Tokens hören auf zu funktionieren, und alle dort angemeldeten Spieler werden bei der nächsten Prüfung abgemeldet.
agw-approvedSaid = Das Gateway ist freigegeben.
agw-disabledSaid = Das Gateway ist gesperrt.
leg-pageTitle = Alte Konten
leg-lead = Konten des offiziellen Gateways aus der Zeit vor Baylee Central, die noch nicht umgezogen sind. Jedes zieht bei seiner ersten Anmeldung im Client mit seinem alten Passwort um, und seine Zeile wird gelöscht, sobald der Umzug fertig ist. Nur Namen, Zeiten und Zähler: nie ein Passwort-Hash.
leg-title = Noch nicht umgezogen
leg-none = Alle Konten sind umgezogen. Nichts ist übrig.
leg-noStore = Dieses Central führt keine alten Konten.
leg-noDeadline = Keine Frist gesetzt: Die Zeilen bleiben, bis der Betreiber eine setzt.
leg-deadline = Übrig: { $count }. Nach dem { $day } löscht die tägliche Bereinigung jede Zeile, die dann noch da ist.
leg-account = Konto
leg-progress = Umzug
leg-held = Aufgehalten
leg-failures = Fehlversuche
leg-imported = Übernommen
leg-unstarted = Nicht begonnen
leg-reserved = Name reserviert
leg-enrolled = Eingerichtet
leg-restored = Wiederhergestellt
leg-banned = Gesperrt
leg-mustChange = Neues Passwort verlangt
leg-locked = Ausgesperrt bis { $until }
leg-retry = Erneut versuchen
aud-pageTitle = Protokoll
aud-lead = Was Admins und der Betreiber geändert haben, das Neueste zuerst. Ein Eintrag nennt IDs, nie Worte.
aud-title = Einträge
aud-none = Noch nichts protokolliert.
aud-when = Wann
aud-what = Was
aud-by = Von
aud-subject = Betrifft
aud-operator = der Betreiber
aud-adminGranted = Zum Admin gemacht
aud-adminRemoved = Admin-Rolle entzogen
aud-resetIssued = Reset-Code erzeugt
aud-tagChanged = Tag geändert
aud-gatewayApproved = Gateway freigegeben
aud-gatewayDisabled = Gateway gesperrt
aud-tokenRevoked = Gateway-Token widerrufen
