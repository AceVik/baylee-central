### The website's words in English (web/README.md §"The words").
### English decides which messages exist and which arguments and markup each
### takes; every other language translates them (Baylee's docs/translations.md).
### Markup is a pair of tags the page fills in, `<code>…</code>` or a link such
### as `<notice>…</notice>`: keep each pair, translate the words inside it.
### A `link-…` message is a path in Baylee's repository: point it at that
### language's README where there is one.

## The frame

app-loading = Loading…
app-skip = Skip to content
app-close = Close
nav-cards = Cards
nav-download = Download
nav-gateways = Gateways
nav-contact = Contact
nav-menu = Menu
nav-label = Sections
nav-signIn = Sign in
nav-account = Your account, { $name }
nav-accountUnnamed = Your account
notFound-title = There is no such page
notFound-back = Back to the start page

## The tab's title, per page

title-home = Baylee – a table that knows the rules
title-download = Download – Baylee
title-cards = Cards – Baylee
title-gateways = Gateways – Baylee
title-contact = Contact – Baylee
title-impressum = Legal notice – Baylee
title-privacy = Privacy – Baylee
title-terms = Terms of use – Baylee
title-register = Create an account – Baylee
title-signIn = Sign in – Baylee
title-account = Your account – Baylee
title-notFound = Not found – Baylee

## The footer. The Fan Content Policy's notice is Wizards' own wording: the
## English one stands in every language, and another language shows Wizards'
## official translation of it beside it (Baylee's docs/legal.md §1).

foot-fanContent = Baylee is unofficial Fan Content permitted under the <policy>Fan Content Policy</policy>. Not approved/endorsed by Wizards. Portions of the materials used are property of Wizards of the Coast. ©Wizards of the Coast LLC.
foot-fanContentUrl = https://company.wizards.com/en/legal/fancontentpolicy
foot-wotc = Magic: The Gathering, card names, set names, card images and mana symbols are property of Wizards of the Coast LLC. Every card image names its artist; select a card to see artist and printing on Scryfall.
foot-scryfall = Card data and images provided by Scryfall. Baylee is not affiliated with Scryfall.
foot-label = Footer
foot-source = Source code (AGPL-3.0)
foot-central = Baylee Central (source-available)
foot-siteSource = This site's source
foot-releases = Releases
foot-issues = Issues
foot-contact = Contact
foot-legal = Legal
foot-terms = Terms of use
foot-privacy = Privacy
foot-impressum = Legal notice
foot-colophon = Fonts: Faustina and Alegreya Sans, SIL Open Font License 1.1, served from this site. The painting and the logo are our own.

## Paths in Baylee's repository, under github.com/AceVik/baylee

link-readmeWhat = #what-baylee-is
link-readmeInstall = #installing
link-readmeGateway = #running-your-own-gateway
link-readmeModel = #playing-against-a-language-model

## Times, sizes and the state of CI

time-minutesAgo = { $n } min ago
time-hoursAgo = { $n } h ago
time-daysAgo =
    { $n ->
        [one] { $n } day ago
       *[other] { $n } days ago
    }
size-mb = { $size } MB
ci-passing = passing
ci-failing = failing

## The start page

hero-kicker = Closed beta · free · open source · unofficial fan content
hero-title = A table that <glow>knows the rules.</glow>
hero-lead = Baylee is a rules engine and a 3D table for playing Magic: The Gathering with friends or against the house AI. The engine enforces the Comprehensive Rules, so nobody has to. Free, open source under the AGPL, and it stays that way.
hero-note = Plays offline against the house AI without an account. Online play on the Sanctuary gateway is a closed beta and needs a key.
hero-what = What Baylee is
hero-cards = Where the cards stand
hero-scroll = Scroll down

status-label = Project status
status-build = Latest build
status-releases = see releases
status-ci = CI on main
status-onGitHub = see on GitHub
status-commit = Last commit
status-gateways = Gateways
status-directory = the directory

about-eyebrow = What it is
about-title = Two halves, one game
about-engineTitle = An engine that enforces the game
about-engine = Priority and the stack, the layer system, replacement effects, triggers, state-based actions and combat: the engine applies the Comprehensive Rules and lists, at every step, exactly the choices a player has. The client offers those and nothing else. It never decides what is legal.
about-deterministic = It is deterministic: a seeded random-number generator, no clock and no hash-map order in the rules, so the same game with the same answers always ends the same way. Every hosted game is recorded move by move and can be replayed to the same state, which is also what a bug report carries.
about-tableTitle = A 3D table built with Bevy
about-table = Battlefield, hands, stack and graveyards on a table you can read at a glance, plus a lobby for accounts, decks and rooms. Card images come from Scryfall while you play; a text view draws every card from its rules text instead, and <kbd>T</kbd> switches between the two.
about-keys = You can play with the mouse or entirely from the keyboard: <kbd>Space</kbd> passes, <kbd>Tab</kbd> fast-forwards to the next phase while every decision stays yours, <kbd>L</kbd> opens a game log that reads like a chat. Every key can be rebound.

feature-hiddenTitle = Hidden information stays hidden
feature-hidden = Your client is sent only what you may see. Libraries and other hands arrive as counts, a face-down card carries no identity. A modified client cannot peek, because the data never reaches it.
feature-seatsTitle = Two to eight seats
feature-seats = Duels, free-for-all and team tables. The host puts chairs on sides, and the sides need not be even. Teams keep separate turns and life totals; teammates can choose to show each other their hands.
feature-commanderTitle = Commander and freeform
feature-commander = A deck that names a commander plays Commander; any other deck plays freeform. Banlists and format-legality checks do not exist yet. That is a gap, not a feature.
feature-aiTitle = A house AI at five levels
feature-ai = Novice, casual, steady, sharp and expert. It plays from the same view a human gets, so it cannot see your hand either.
feature-processTitle = One process per game
feature-process = Each game runs in its own engine process. The gateway that handles accounts and rooms runs no rules at all, so one broken game is one broken game.
feature-languagesTitle = Card text in 19 languages
feature-languages = When the gateway carries the card catalog, cards read in your language with English as the fallback. The interface itself is in English and German.
feature-watchTitle = Watch a running table
feature-watch = The lobby's Watch button seats you as a spectator. You see what the whole table sees and nobody's hand. A host can switch spectators off for a room.
feature-connectionTitle = A lost connection is not a lost game
feature-connection = When a player drops, the table shows a countdown while their chair waits for them. If time runs out, the house AI plays for them until they are back; with nobody else at the table, the game simply pauses.
feature-tablesTitle = Tables your way
feature-tables = A Mixed table seats decks of any format together, and fourteen decision clocks run from a 15-second bullet to untimed. Starting life and mulligans are the host's to set.

shots-eyebrow = At the table
shots-title = What it looks like
shots-lead = These pictures show the text view, which draws each card from its rules text rather than the printed card image, because we do not redistribute Scryfall's scans. In the game you see the real card images. The screenshots were taken with 0.1.0-beta.2 on a local test gateway with made-up players; the current build looks a little different.
shots-showLarge = Show large: { $title }
shots-dialog = Screenshot
shots-previous = Previous
shots-next = Next
shot-duelAlt = A duel against the house AI; a hovered card is shown large in the text view
shot-duelTitle = A duel
shot-duelCaption = against the house AI. Hovering a card shows it large.
shot-logAlt = The game log beside the table, each card name a link
shot-logTitle = The game log
shot-logCaption = (L) reads like a chat; every card name opens its preview.
shot-ringAlt = A four-seat table, the seats in a ring around the felt
shot-ringTitle = Four seats.
shot-ringCaption = Larger tables sit in a ring around the felt.
shot-lobbyAlt = The lobby: decks on the left, open tables on the right
shot-lobbyTitle = The lobby.
shot-lobbyCaption = Your decks, the open tables, and Play the house for a quick game.
shot-roomAlt = A room being arranged: seats, AI levels, starting life
shot-roomTitle = Arranging a table.
shot-roomCaption = Chairs, starting life, mulligans, and a level for each AI chair.
shot-builderAlt = The deck builder with statistics and a search over the pool
shot-builderTitle = The deck builder.
shot-builderCaption = Mana curve, land statistics, a search over the whole pool, and a warning for cards that will not play as printed.

ai-eyebrow = Opponents
ai-title = Opponents: the house, or a language model
ai-houseTitle = The house AI
ai-house = Five levels from novice to expert, available offline and at any online table. It plays from the same view a human gets, so hidden information stays hidden from it too. It is a heuristic player, not a trained one: good enough to learn a deck against, not a tournament opponent.
ai-modelTitle = A chair for a language model
ai-model = A seat bridge lets a model sit at the table as an ordinary player: a local model in LM Studio, one behind an OpenAI-compatible or Anthropic API, or an agent CLI on your own login. API keys never go in a file; a settings file names the environment variable they come from, and daily and monthly spending caps hold across games. When the model times out or answers something illegal, the house answers that question and the transcript says so.
ai-gateway = At a gateway table the host seats one from the room: any open chair offers <i>language model</i> on a desktop client, which starts the bridge (it ships beside the game) with your own settings and key. Where the gateway's operator runs hosted models, a registered host can also pick one of those instead, and it plays on the operator's machine, so no key or bridge is needed on yours.
ai-setup = How to set it up

beta-eyebrow = Online
beta-title = The closed beta
beta-sanctuary = The public gateway, <b>Baylee Sanctuary</b>, is where online tables live. It is a closed beta: a new account and a new guest each need a key of the form <code>BAYLEE-XXXX-XXXX-XXXX-XXXX</code>, handed out by the project owner. There is no public sign-up form, and we are not pretending otherwise.
beta-signup = Signing up asks for no e-mail address: an account is a username and a password. Other players see only your display name and a short tag. A guest needs just a display name and is deleted when it signs out or after 30 days without use. You can delete your account from the client at any time.
beta-keys = Keys come from the project owner, <owner>@AceVik</owner> on GitHub. Without one, Baylee still plays offline against the house AI, with the deck builder and your own decks, and you can run a gateway of your own for your group.
beta-listTitle = Every gateway, in one list
beta-list = Sanctuary and the gateways other people run for their groups: the directory shows each one's address, version and whether it is online right now.
beta-open = Open the directory
beta-own = Run your own

source-eyebrow = Source
source-title = Open source, and it stays that way
source-promise = Baylee will always be open source and free. No paywall, no paid features, no ads. Not now, not later. The code is under the GNU Affero General Public License, which means anyone who runs a modified gateway for other people must offer them its source too.
source-rust = It is written in Rust, one person's project with a lot of help from coding agents, and every change goes through the same gate: formatting, lints, the test suite, and a validation of every card against its printed text. Issues and pull requests are welcome; <agents>AGENTS.md</agents> and <claude>CLAUDE.md</claude> are the working rules for people and agents alike.
source-discord = Community on Discord
source-architecture = Architecture
source-repoTitle = The repository right now
source-stars = Stars
source-issues = Open issues and pull requests
source-note = Read from GitHub's public API when the page loads, cached in this tab for a while. Nothing about you is sent anywhere but to GitHub.

## Download

dl-button = Download Baylee
dl-fallbackMeta = Windows · macOS (Apple silicon) · Linux. The latest release is on GitHub.
dl-for = Download for { $system }
dl-signature = signature
dl-notes = release notes
dl-others = other systems
dl-archive = This release ships as an archive: unpack it and start Baylee. Installers come with the next release.
dl-noteUnknown = We could not tell your system from this browser; every file is listed under Download.
dl-noteMobile = There are no builds for phones and tablets yet. The button takes you to the release for your computer.
dl-noteMacUnsure = This browser does not say whether your Mac has an Apple chip or an Intel one. The file above is for Apple silicon (M1 and later); an Intel Mac finds its file under the other systems.
dl-noteMacIntel = Your Mac reports an Intel chip, and this release has no Intel build. The button takes you to the release page.
dl-noteWindowsUnsure = If this is a Windows on ARM PC (a Snapdragon laptop, say), take the ARM file under Download instead.
dl-noteLinuxUnsure = Your browser does not say which CPU this is; x86-64 is assumed. The ARM file is under Download.

dlPage-eyebrow = Get it
dlPage-title = Download
dlPage-lead = Every download goes straight to the file on GitHub, nothing in between. Where a release has installers, they are the easy way; the archives hold the same game to unpack anywhere. An installed Baylee updates itself from the signed archives wherever it may write its own folder. Every package carries the seat bridge for language models beside the game.
dlPage-filesEyebrow = Every file
dlPage-filesTitle = All systems

release-loading = Loading the latest release from GitHub …
release-noAnswer = GitHub did not answer just now; it lets a page like this ask 60 times an hour. <releases>The releases page has every file.</releases>
release-pre = pre-release
release-notes = Release notes and checksums
release-all = All releases

plat-empty = An installer and an archive for Windows (x64 and ARM64), macOS (Apple silicon) and Linux (x86-64 and ARM64), each with a .sha256 beside it, on the <releases>releases page</releases>.
plat-asking = Asking GitHub for the files … They are also on the <releases>releases page</releases>.
plat-yours = your system
plat-noIntel = This release has no build for Intel Macs. The one for Apple chips needs an M1 or later.
plat-missing = This release has no file for: { $systems }. Older releases are on the releases page.
plat-windows = Windows
plat-windowsSub = most PCs: 64-bit Intel or AMD
plat-windowsArm = Windows on ARM
plat-windowsArmSub = Snapdragon laptops and the like
plat-mac = macOS
plat-macSub = Apple silicon: M1 and later
plat-macIntel = macOS on Intel
plat-linux = Linux
plat-linuxSub = x86-64
plat-linuxArm = Linux on ARM
plat-linuxArmSub = ARM64 / aarch64
file-setup = Setup
file-diskImage = Disk image
file-appImage = AppImage
file-deb = Debian / Ubuntu package
file-zip = Archive (zip)
file-tarGz = Archive (tar.gz)

install-eyebrow = Installing
install-title = First start and what it needs
install-firstTitle = First start
install-first = The builds are not code-signed, because signing costs money. Windows and macOS therefore warn the first time. On Windows, click <i>More info</i> and <i>Run anyway</i>. On macOS, open <i>System Settings → Privacy & Security</i> and click <i>Open Anyway</i>. The <notes>install notes</notes> have every step and the Linux packages.
install-checkTitle = Checking a download
install-check = Beside each file there is a <code>.sha256</code>; the archives also carry a <code>.sig</code>, the Ed25519 signature the client checks before it installs an update by itself. <code>sha256sum -c</code> on Linux, <code>shasum -a 256 -c</code> on macOS, <code>certutil -hashfile … SHA256</code> on Windows.
install-needsTitle = What it needs
install-windows = <b>Windows</b> 10 or 11, x64 or ARM64. The x64 build needs a CPU with x86-64-v2 (SSE4.2), as Windows 11 24H2 does.
install-mac = <b>macOS</b> 11 or later, on Apple silicon (M1 and later) or Intel.
install-linux = <b>Linux</b> with glibc 2.35 or newer (Ubuntu 22.04 and up), a Vulkan driver, and ALSA, udev, X11/Wayland and xkbcommon; the <code>.deb</code> pulls them in.
install-browser = <b>Browser:</b> the browser version is paused for this beta and comes back later; until then, take the desktop client above.
install-phones = <b>Phones and tablets:</b> no builds to download yet. Android runs but cannot be used by touch; iOS runs only in the simulator.
install-releases = Every release on GitHub

## The card status

cards-eyebrow = Cards
cards-title = The cards, honestly
cards-intro = Every card in Baylee is code: one file per card, written in a small rules language and played in a test. There is no "close enough" interpreter guessing at card text. That is why the pool grows batch by batch instead of all at once.
cards-counted = Counted from the repository by Baylee's own tools (<pool>the pool list</pool>, <readme>the README</readme>). The deck builder can show only playable cards, and it warns when a deck holds cards that will not play as printed.
cards-planTitle = The plan
cards-plan = Sets go in release order, starting with Alpha, because every set introduces its mechanics on top of the older ones. Alpha is complete: every playable card of it plays as printed. Arabian Nights and Antiquities come next, and the rest follows set by set. Bugs found at the table come before new cards, every time.
cards-modern = That means a modern deck will often hold cards Baylee does not know yet. We would rather tell you that in the deck builder than let a card resolve wrongly. Cards that cannot be played at all (ante, dexterity, subgames) are on a named skip list and never silently dropped.
cards-implemented = cards that play exactly as printed
cards-partial = more that are partly implemented and marked as such
cards-corpus = cards in the whole corpus we are working towards

sets-title = Sets and card status
sets-lead = Every set in release order, each card counted under the set it first appeared in. Open a set to see its cards: whether each plays as printed, plays but is not yet tested, plays in part, is not playable yet, or will never be built. Rest the pointer on a card, or focus it, to see it large with the reason; a click opens its page on Scryfall, with artist and printing. On a touch screen a tap shows it large first.
sets-legend = Legend
sets-legendFull = Full: plays as printed
sets-legendUntested = Untested: playable, every sentence is read by the rules engine, but no test plays it yet
sets-legendPartial = Partial: plays, one part missing (the reason is shown with the card)
sets-legendNone = Not yet: not playable yet
sets-legendNever = Never: ante, dexterity or a subgame; a set counts as complete without it
sets-percent = The percentage is playable cards (full and untested) out of all cards of the set that will ever be built. Partial cards are the third part of the bar and are not counted as done.
sets-loading = Loading the set list …
sets-failed = The set list did not load just now. Reload the page to try again.
sets-hide = Hide the full list
sets-showAll =
    { $n ->
        [one] Show the { $n } set
       *[other] Show all { $n } sets
    }
sets-imagesNote = Card images are loaded from Scryfall only when you open a set or show a card large, unaltered. Card data and images provided by Scryfall; Baylee is not affiliated with Scryfall. The counts are measured from the repository by <code>baylee-tools site card-status</code>.
sets-ub = Card images © Wizards of the Coast, shown unaltered from Scryfall. Cards not yet in Baylee's pool, and Universes Beyond cards, are listed by name only.
sets-total =
    { $play } of { $all ->
        [one] { $all } card
       *[other] { $all } cards
    } playable ({ $pct } %): { $full } full, { $untested } untested. Besides: { $partial } partial, { $none } not yet, { $never } never; { $sets ->
        [one] { $sets } set
       *[other] { $sets } sets
    }, { $corpus ->
        [one] { $corpus } card
       *[other] { $corpus } cards
    }.
sets-atRelease = Release { $release }, commit { $rev }.
sets-atCommit = Measured at commit { $rev }.
sets-finished = Finished
sets-next = Next, in release order
sets-find = Find a set
sets-onlyBegun = only sets that have begun
sets-foundNone = No set matches.
sets-foundMany =
    { $n ->
        [one] { $n } set matches; the first { $max } are shown. Type more to narrow it.
       *[other] { $n } sets match; the first { $max } are shown. Type more to narrow it.
    }
sets-found =
    { $n ->
        [one] { $n } set matches.
       *[other] { $n } sets match.
    }
sets-eraMeta =
    { $sets ->
        [one] { $sets } set
       *[other] { $sets } sets
    } · { $cards ->
        [one] { $cards } card
       *[other] { $cards } cards
    }
sets-cards =
    { $n ->
        [one] { $n } card
       *[other] { $n } cards
    }
sets-cardsFailed = The card list did not load. Close and open the set to try again.
sets-onScryfall = This set on Scryfall
sets-images = Images: Scryfall · © Wizards of the Coast
sets-pct = { $pct } %
sets-pctPlayable = { $pct } percent playable

count-playable = { $n } playable:
count-full = { $n } full
count-untested = { $n } untested
count-partial = { $n } partial
count-none = { $n } not yet
count-never = { $n } never

filter-label = Show cards
filter-all = All
status-full = Full
status-untested = Untested
status-partial = Partial
status-none = Not yet
status-never = Never

grid-hint = Arrow keys move between cards; Space shows one large, Enter opens it on Scryfall.
grid-label = Cards of { $set }
grid-tile = { $status }. { $why }
grid-scryfall = Artist and printing on Scryfall

card-credit = Card image provided by Scryfall, unaltered. Baylee is not affiliated with Scryfall or Wizards of the Coast.
card-illustrated = Illustrated by { $artist }
card-clickHint = Click for the artist and printing on Scryfall.
card-implemented = Plays as printed.
card-generated = Playable: every sentence is read by the rules engine, but no test plays it yet.
card-partial = Partly implemented. Missing: { $note }
card-unimplemented = In the card pool, but its abilities are not written yet: not playable.
card-absent = Not in the card pool yet.
card-never = Never built: { $reason }
card-neverAnte = it plays for ante (CR 407), and no table here plays for ante
card-neverDexterity = it asks a physical act of the player, which no game state describes
card-neverSubgame = it starts a game within the game (CR 728), which the engine does not host

## The gateway directory

gw-eyebrow = Online
gw-title = Gateways
gw-lead = A gateway is a server where online tables live: accounts, decks, rooms and one engine process per game. These are the ones registered with Baylee Central, whose operator checked who runs each before listing it. Baylee runs Sanctuary; the others are run by other people for their groups, and their own rules apply there.
gw-howTo = To play on one, copy its address into the client's front door. Online means Central heard from the gateway within the last minute and a half; it is no promise that it answers. Names and versions are what each operator reports.
gw-count =
    { $n ->
        [one] { $n } gateway listed, { $online } online.
       *[other] { $n } gateways listed, { $online } online.
    }
gw-none = No gateway is listed yet.
gw-next = Show the next { $n }
gw-tryAgain = Try again
gw-unavailable = Baylee Central cannot list gateways just now.
gw-busy = Baylee Central is busy; please try again in a moment.
gw-refused = Baylee Central did not answer this page's request. The directory may not be open to browsers yet.
gw-malformed = Baylee Central's answer did not pass this page's checks, so nothing of it is shown.
gw-network = Baylee Central could not be reached.
gw-online = online
gw-notSeen = not seen lately
gw-address = Address
gw-version = Version
gw-copy = Copy address
gw-website = Website
gw-copyImpossible = Copying is not possible in this browser.
gw-copied = Address copied.
gw-copyFailed = Copying did not work.

selfhost-eyebrow = Self-hosting
selfhost-title = Run your own gateway
selfhost-processes = You do not need our server. A gateway is three kinds of process: <code>baylee-gateway</code> handles accounts, decks, rooms and routing and needs PostgreSQL; <code>baylee-agent</code> starts one engine process per game; <code>baylee-engine-server</code> is one game. The first two run no rules at all.
selfhost-proxy = Put a TLS proxy in front, set <code>BAYLEE_REGISTRATION=invite</code> if you want your own closed circle, create keys with <code>baylee-gateway invite create</code>, and the client's front door takes your address like any other. Card text in 19 languages is an optional ingest of Scryfall's bulk data into the same database.
selfhost-guide = <guide>The self-hosting guide</guide> walks through it; <protocol>docs/protocol.md</protocol> is the reference.
selfhost-source = Every gateway answers <code>GET /source</code> with its licence, version and commit, without sign-in. The AGPL asks you to offer your source to the people who use your server; that route does it for you.

## Contact

contact-eyebrow = Contact
contact-title = Write to us
contact-intro = A question, an idea, a game you would like to see, or something you would rather not post in public: this form goes straight to the people behind Baylee. Leave an e-mail address if you want an answer; without one we can only read it.
contact-f8 = A card that resolved wrongly is best reported from the game with <kbd>F8</kbd>, because the report can carry the whole game.
contact-discord = talk, find opponents, hear about builds
contact-issues = GitHub issues
contact-issuesSub = bugs and ideas, in public
contact-operator = Who runs this site, with a postal address: the <impressum>legal notice</impressum>.

form-name = Name
form-optional = optional
form-email = E-mail
form-emailOptional = optional, only if you want an answer
form-message = Message
form-privacy = I keep your message, and your name and e-mail address only if you give them, to read and answer it: no IP address, no cookie, no tracker. It is deleted 90 days after I have dealt with it, after a year at the latest. <more>More in the privacy notice</more>.
form-unreachable = The form could not reach us just now. Please try again later, or write on Discord or GitHub.
form-empty = Please write a message first.
form-badEmail = That e-mail address does not look right. Leave it empty if you want no answer.
form-sending = Sending…
form-sent = Sent.
form-tooMany = Too many messages from here in the last hour. Please try again later.
form-refused = The message was not accepted. Is it shorter than { $max } characters, and the address a real one?
send-send = Send
send-sent = Sent
send-retry = Try again
thanks-title = Thank you, it is on its way.
thanks-answer = If you left an address, the answer comes from a person, by mail.
thanks-again = Write another

reports-eyebrow = Reports
reports-title = Found something wrong? Tell us
reports-f8 = Press <kbd>F8</kbd> at the table or in the lobby. A report carries its kind, your text, the client version and, at a table, the game's id. Everything more has a checkbox of its own, and every box starts ticked, so untick what you would rather keep before you press Send: system and hardware, the table as you see it, your log (other players renamed "Player A", "Player B", …), your settings, a picture of the Baylee window only, and at a table the game's whole record (every move and every seat's cards, no names). <i>Show what is sent</i> shows every byte. Session tokens are never sent.
reports-usage = While you play offline, the client says once a minute that a game is running: no account, no device id, nothing about the game. <i>Send anonymous usage count</i> under Settings › Privacy & Data switches it off.
reports-discord = Want to talk, find opponents or hear first about new builds? Join the <discord>Baylee community on Discord</discord>.
reports-issues = Rules questions, wrong card behaviour, ideas: <issues>open an issue on GitHub</issues>. A card that resolved wrongly is the most useful report we can get, because every card has a test and yours becomes one.
reports-privacy = <docs>docs/privacy.md</docs> lists everything the client, the gateway and the feedback service keep. This site keeps nothing about you: no cookies, no analytics, no fonts from anyone else's server. <notice>The privacy notice</notice> names every other host it talks to, and when.

## The legal notice (Impressum)

imp-eyebrow = Legal notice
imp-title = Legal notice
imp-ddg = Information pursuant to § 5 DDG
imp-private = This website is a private, non-commercial project of a private individual. Baylee is free and open source; there is no advertising and nothing to buy.
imp-write = You can also write to me with the <contact>contact form</contact>.
imp-disputeTitle = Dispute resolution
imp-dispute = This is a free, non-commercial offer of a private individual. I am neither obliged nor willing to take part in dispute resolution proceedings before a consumer arbitration board.
imp-data = How this site handles your data: <notice>the privacy notice</notice>. What the client, the gateway and the feedback service keep about you: <docs>docs/privacy.md</docs>.
imp-why = To keep address harvesters away, the details load only when you ask for them; your browser works out a small puzzle for it, which takes a moment.
imp-show = Show the details
imp-puzzle = Puzzle
imp-working = One moment, your browser is working out a small puzzle …
imp-failed = The details could not be loaded. Please reload the page or use the <contact>contact form</contact>.
imp-drawn = The address is drawn so that address harvesters cannot read it, and a screen reader cannot either. The button below copies it to the clipboard; or write to me with the contact form.
imp-copyAddress = Copy address
imp-contact = Contact
imp-email = E-mail
imp-writeEmail = Write an e-mail
imp-copyEmail = Copy e-mail address
imp-phone = Telephone
imp-call = Call
imp-copyNumber = Copy number
imp-copyImpossible = Copying is not possible in this browser; please use the contact form.
imp-copied = Copied.
imp-copyFailed = Copying did not work; please use the contact form.

## The terms of use (their text is legal/terms.<lang>.md)

terms-eyebrow = Terms of use
terms-draft = Draft: these terms are not in force yet. They take effect only once published, and you will be asked to accept them.
terms-version = Version { $version }.
terms-updated = Updated { $date }.

## The privacy notice: a draft for the owner's lawyer (src/privacy/)

privacy-eyebrow = Privacy
privacy-title = Privacy notice for this website
privacy-lead = This page says what happens with your data when you visit this website. The game itself (the client and the online gateway) keeps its own records, listed in full in <docs>docs/privacy.md</docs>.
privacy-contents = Contents
privacy-date = As of 21 October 2026.
privacy-controllerTitle = Who is responsible
privacy-controller = The controller is the private individual named in the <impressum>legal notice</impressum>. You can reach me there or with the <contact>contact form</contact>.
privacy-hostingTitle = Hosting and server
privacy-hosting = The website runs on a server rented from netcup GmbH in Germany. When you open a page, your browser sends the server what every request carries: your IP address, the address asked for, the time and what your browser says about itself. The server needs this to answer. The web server keeps <b>no access log</b> for this website, so these requests are not written down. The server's own operating messages (its system journal) can, in rare error cases, contain technical connection data such as an IP address; they are kept only as long as the journal keeps them.
privacy-hostingBasis = Legal basis: Art. 6(1)(f) GDPR, my legitimate interest in delivering the website safely.
privacy-storageTitle = No tracking
privacy-storage = This website uses no analytics, no tracking and no advertising, and it sets no cookie unless you sign in (see the session cookie below). The fonts are served from this server; no font service is asked. Your browser keeps a few things on your device, which never leave it: the language you chose (until you delete it); for ten minutes in this tab, GitHub's answers about the latest release, so the page does not ask again on every visit; and while you are signed in, when your session ends.
privacy-storageBasis = Legal basis for this storage: § 25(2) no. 2 TDDDG; it is strictly necessary for what you asked the page to do.
privacy-githubTitle = GitHub: releases and project status
privacy-github = To show the latest release, its files and the project's state, your browser asks GitHub's public API (<code>api.github.com</code>) when the start page or the download page loads, and a download comes straight from GitHub. Your browser then sends your IP address and the request to GitHub, Inc. in the USA. See <statement>GitHub's privacy statement</statement>.
privacy-githubBasis = Legal basis: Art. 6(1)(f) GDPR, my legitimate interest in showing current downloads without hosting them myself.
privacy-scryfallTitle = Card images from Scryfall
privacy-scryfall = Card images are not stored here. Only when you open a set in the card status, or show a card large, your browser loads its images directly from Scryfall's image server (<code>cards.scryfall.io</code>) and sends your IP address and the request to Scryfall, LLC in the USA. No referrer is sent. See <policy>Scryfall's privacy policy</policy>.
privacy-scryfallBasis = Legal basis: Art. 6(1)(f) GDPR, my legitimate interest in showing the cards the status speaks of.
privacy-centralTitle = The gateway directory
privacy-central = When you open the <directory>gateway directory</directory>, your browser asks Baylee Central, the project's account service, for the public list of gateways, through this website's own address. The request carries no cookie, no account and no identifier of yours; Central answers everyone the same list. Otherwise the website talks to Central only when you register, sign in or open your account (below).
privacy-centralBasis = Legal basis: Art. 6(1)(f) GDPR, my legitimate interest in showing where Baylee can be played online.
privacy-contactTitle = The contact form
privacy-contact = What you send goes to Baylee's feedback service (<code>{ $host }</code>) and is stored there: your message, your name and e-mail address only if you give them, the page's language, the time and whether I have dealt with it. Your IP address is not stored; for an hour it is counted in memory only, to stop a flood of messages. Only I read the messages; nothing is passed on. A message is deleted 90 days after I have dealt with it and a year after it came at the latest, or earlier when you ask.
privacy-contactBasis = Legal basis: Art. 6(1)(f) GDPR, my legitimate interest in answering messages; where your message aims at a contract with me, Art. 6(1)(b) GDPR.
privacy-impressumTitle = How the legal notice is protected
privacy-impressum = So that address harvesters do not collect my details, the legal notice shows them after your browser has solved a small computing puzzle. For that, the feedback service keeps in memory only, never on disk and never in a log: how many puzzles your IP address asked for (for an hour; an IPv6 address by its first half), whether it followed a link hidden from people (for 24 hours), and which puzzles were solved (five minutes, without an address). Three yes/no answers about your browser set the puzzle's difficulty and are not kept. Nobody is blocked: at worst the puzzle takes longer.
privacy-impressumBasis = Legal basis: Art. 6(1)(f) GDPR, my legitimate interest in protecting my details from misuse.
privacy-linksTitle = Links to other sites
privacy-links = Discord, GitHub, Scryfall, Wizards of the Coast and the websites of listed gateways are linked, not embedded: nothing is sent to them until you follow a link, and then their own privacy terms apply.
privacy-rightsTitle = Your rights
privacy-rights = You have the right to access (Art. 15 GDPR), rectification (Art. 16), erasure (Art. 17), restriction of processing (Art. 18) and data portability (Art. 20), and you may object at any time to processing based on Art. 6(1)(f) (Art. 21). Write to me through the contact form or the address in the legal notice.
privacy-complain = You may also complain to a data protection supervisory authority (Art. 77 GDPR), for example the one where you live, or the one responsible for me: the <ldi>Landesbeauftragte für Datenschutz und Informationsfreiheit Nordrhein-Westfalen (LDI NRW)</ldi>.
privacy-automated = There is no automated decision-making within the meaning of Art. 22 GDPR: the puzzle's difficulty adapts by itself, but it decides nothing about you.

## Accounts at Baylee Central (src/account/)

acct-eyebrow = Baylee Central
acct-username = Username
acct-password = Password
acct-errBusy = Too many attempts from here. Please wait a while and try again.
acct-errUnreachable = Baylee Central did not answer. Please try again in a moment.
acct-errCrypto = This browser could not run the account's encryption: it needs WebAssembly and Web Workers. Please try a current browser.
acct-errMismatch = Baylee Central's answer did not check out, so nothing was kept. Please try again; if it happens again, please tell us.
register-title = Create your account
register-lead = One account for Baylee: you sign in with it in the Baylee client and on the gateways that use Baylee Central. While Baylee is in beta, you need a beta key to register.
register-betaKey = Beta key
register-betaKeyHint = As you got it, for example CENTRAL-XXXX-XXXX-XXXX-XXXX.
register-usernameRules = 3 to 24 characters: letters a–z, digits, and . _ - between them, never two in a row. Capitals are read as small letters. Other players see this name.
register-password = Password
register-passwordAgain = Password again
register-passwordHint = At least 8 characters, and no rules about which. A passphrase of several unrelated words is strong and easy to remember. Your password never leaves this browser.
register-strengthShort = Too short
register-strengthWeak = Weak: easy to guess
register-strengthFair = Fair: longer would be stronger
register-strengthStrong = Strong
register-age = I am at least 16 years old.
register-consent = I accept the <terms>terms of use</terms> of Baylee Central and have read the <privacy>privacy notice</privacy>.
register-submit = Create account
register-submitting = Creating…
register-haveAccount = Already have an account? <signin>Sign in</signin>.
register-progressKey = Checking the beta key…
register-progressSecuring = Securing your password. This takes a few seconds…
register-progressSaving = Saving your account…
register-errKey = Please enter your beta key.
register-errUsername = That username does not follow the rules above.
register-errPassword = The password needs at least 8 characters (and at most { $max } bytes).
register-errMismatch = The two passwords differ.
register-errAge = Please confirm that you are at least 16 years old.
register-errConsent = Please accept the terms of use.
register-errBadKey = That beta key is not valid, used up or expired.
register-errTaken = That username is taken. Please choose another.
recovery-title = Your recovery code
recovery-lead = Your account is ready. If you ever lose your password, this code opens your encrypted data. It is shown only now: nobody, not even Baylee Central, can show it to you again.
recovery-label = Recovery code
recovery-copy = Copy
recovery-copied = Copied.
recovery-copyFailed = Could not copy. Please select the code and copy it by hand.
recovery-download = Download as a text file
recovery-fileName = baylee-recovery-code.txt
recovery-file =
    Baylee Central: the recovery code for { $name }

    { $code }

    If you lose your password, this code opens your account's encrypted data.
    Keep it somewhere safe and private, not next to your password.
    Nobody, not even Baylee Central, can show it to you again.
recovery-keep = Write it down, or keep the file somewhere safe and offline, not next to your password.
recovery-confirm = I have kept my recovery code in a safe place.
recovery-continue = Continue
accept-title = Accept the terms of use
accept-lead = Version { $version } of Baylee Central's terms of use applies to your account. Please read them and accept them to use it.
accept-loading = Loading the terms of use…
accept-accept = I accept
accept-later = Not now (sign out)
accept-retry = Try again
accept-mismatch = The terms Baylee Central sent do not match their fingerprint, so they cannot be accepted here. Please try again later.
accept-signedOut = Your session has ended. <signin>Sign in again</signin> to accept the terms.
accept-declined = You are signed out. <signin>Sign in again</signin> whenever you want to accept the terms.
signin-title = Sign in
signin-submit = Sign in
signin-submitting = Signing in…
signin-progress = Signing in. This takes a few seconds…
signin-errName = Please enter your username.
signin-errPassword = Please enter your password.
signin-errWrong = The username or the password is wrong.
signin-legacy = Played on the official gateway before? Then sign in there once with the Baylee client: that moves your account to Baylee Central, and from then on it works here too.
signin-noAccount = No account yet? <register>Create one</register>.
signin-already = You are signed in as <b>{ $name }</b>. <account>To your account</account>
signin-alreadyUnnamed = You are signed in. <account>To your account</account>
signin-registered = Your account is ready. Please sign in.
account-title = Your account
account-signedInAs = Signed in as
account-signedIn = Signed in
account-session = In this browser you stay signed in until { $time }, unless you sign out first.
account-client = To play, sign in with the same name and password in the Baylee client (<download>download</download>).
account-recovery = Keep your recovery code safe: without your password, it is the only way into your encrypted data.
account-later = Changing your password and deleting your account here come later. Until then, write to us through the <contact>contact form</contact> to have your account deleted.
account-signOut = Sign out
account-signOutUnconfirmed = Baylee Central could not be reached to end the session, so this browser keeps it until { $time }. On a shared computer, close the browser now.
account-notSignedIn = You are not signed in. <signin>Sign in</signin> or <register>create an account</register>.
privacy-accountTitle = Your account at Baylee Central
privacy-account = If you create an account, Baylee Central, the project's account service, keeps: your username; a credential record of the OPAQUE protocol, from which your password cannot be read and which never contains it; your account's master key, encrypted once under your password and once under your recovery code, neither of which Central can decrypt; your account document, encrypted in your browser before it is sent; which version of Central's terms of use you accepted, and when; and your sessions while they last. Your password and your recovery code never leave your browser. Registering uses up one use of a beta key; Central stores beta keys only as a digest. Nothing of this goes to anyone else: there are no third parties.
privacy-accountBasis = Legal basis: Art. 6(1)(b) GDPR, the account you asked for. It is kept until the account is deleted; until you can do that yourself, write to me.
privacy-cookieTitle = The session cookie
privacy-cookie = When you sign in, Baylee Central sets one cookie, <code>__Host-central</code>: a random session identifier, for this site only, sent only over HTTPS and only with requests from this site, unreadable for the page's scripts, and gone when you sign out or after 30 minutes. It is the only cookie, and only signing in sets it. So that the bar at the top can show that you are signed in after a reload, your browser also keeps when the session ends (local storage, on your device only); the page then asks Baylee Central whose session it is, and signing out forgets it.
privacy-cookieBasis = Legal basis: § 25(2) no. 2 TDDDG, strictly necessary for signing in, which you asked for; Art. 6(1)(b) GDPR.
privacy-countersTitle = Limits on attempts
privacy-counters = To stop the guessing of beta keys and passwords, Baylee Central counts registrations (five an hour) and sign-in attempts (thirty a minute) per IP address. These counters live in its memory only: never written to disk, gone after an hour at the latest, and lost whenever the service restarts.
privacy-countersBasis = Legal basis: Art. 6(1)(f) GDPR, my legitimate interest in keeping accounts safe.
