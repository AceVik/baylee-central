# baylee.app's words in English, Project Fluent's format. English decides
# which messages exist and which arguments each takes; build.rs turns them
# into typed functions (crates/central-web/build.rs). The reader is "you";
# a button says what it does.

## The frame of every page

site-name = Baylee
skip = Skip to content
nav-label = Main
nav-download = Download
nav-gateways = Gateways
nav-faq = Questions
nav-contact = Contact
nav-sign-in = Sign in
nav-account = Your account
lang-label = Language
theme-label = Colour scheme
theme-system = Colours: as your device
theme-light = Colours: light
theme-dark = Colours: dark

foot-label = About this site
foot-play = Play
foot-legal = Legal
foot-releases = All releases
foot-issues = Report a bug
foot-discord = Discord community
foot-central-source = This site's source
foot-dedication = For Baylee. Forever in the sunlight.
fan-content = Baylee is unofficial Fan Content permitted under the <policy>Fan Content Policy</policy>. Not approved/endorsed by Wizards. Portions of the materials used are property of Wizards of the Coast. ©Wizards of the Coast LLC.
foot-colophon = Fonts: Faustina and Alegreya Sans, SIL Open Font License 1.1, served from this site. The painting and the logo are our own. <notices>Notices</notices>

## Titles and descriptions

title-home = Baylee – Build your decks. Gather your friends. Just play.
desc-home = Baylee is a free, open-source fan game for Magic: The Gathering on Linux, Windows and macOS: a rules engine that knows the cards, tables for up to eight players, and opponents when no one else is around.
title-download = Download Baylee
desc-download = Download Baylee's client for Windows, macOS and Linux: installers and archives of the newest release, with checksums and signatures.
title-gateways = Gateways: where Baylee is played online
desc-gateways = The public directory of Baylee gateways, the servers where online tables live, and how to run your own.
title-faq = Questions and answers
desc-faq = What Baylee is, what it costs, how online play and accounts work, and what happens to your data.
title-contact = Contact
desc-contact = Write to the person behind Baylee, or report a bug.
title-impressum = Legal notice
desc-impressum = Who runs baylee.app (§ 5 DDG).
title-privacy = Privacy notice
desc-privacy = What baylee.app and Baylee Central process about you, why, on which legal basis and for how long.
title-terms = Terms of use
desc-terms = The terms of use of Baylee Central, the account service behind baylee.app.
title-fan-content = Unofficial Fan Content
desc-fan-content = Baylee is a fan project under Wizards of the Coast's Fan Content Policy: what that means and what it does not.
title-not-found = Page not found
title-gone = This page is gone
crumbs-label = You are here
breadcrumb-home = Home

## Home

hero-kicker = Open beta · Free · Open source · Unofficial fan content
hero-title = Build your decks. Gather your friends. <glow>Just play.</glow>
hero-lead = A comprehensive deck builder, matches for up to eight players in duels, free-for-all or teams, and a rules engine that checks every step, so every turn stays fair without anyone having to look anything up.
hero-offline = Solo matches and the deck builder work offline, without an account. Online tables live on gateways; <host>you can run your own</host>.
hero-account = Playing online? <register>Create an account</register>, free and open to everyone in the open beta, or <signin>sign in</signin>.
hero-memorial = Remember Baylee: open her photo album

status-label = Project status

cards-eyebrow = The cards
cards-title = How many cards Baylee knows
cards-lead = Baylee implements Magic's cards one by one, each checked against its printed text. These are the numbers of the newest release.
cards-implemented = Fully implemented
cards-implemented-help = Every rule of the card works and is tested.
cards-generated = Generated
cards-generated-help = Implemented by Baylee's generator from the card's rules text, but not yet tested by hand. They are playable; a rare corner may behave differently.
cards-partial = Partly implemented
cards-partial-help = The card is playable, but one of its abilities or an edge of its rules is still missing.
cards-total = Cards in all
cards-total-help = Every card Magic has, as Scryfall lists them; the three numbers before are those Baylee can play.
cards-help = What this means
cards-share = { $playable } of { $total } cards are playable today.
cards-as-of = Release { $tag }, counted on { $date }.
cards-missing = The card numbers could not be read just now.

shot-duel-alt = A duel against the house AI: six creatures attack, three are blocked
shot-duel = <b>Tactical duel</b> against the house AI, with clear combat lines for every assignment.
shot-log-alt = A duel with the game log open, three spells on the stack
shot-log = <b>The game log</b> records every step like a chat; a card's name opens its preview.
shot-ring-alt = A four-seat table arranged in a ring
shot-ring = <b>Tables with up to eight seats</b> arrange themselves around the battlefield.
shot-lobby-alt = The lobby with a friend's message and her table invitation
shot-lobby = <b>The lobby:</b> open tables, friends, invitations and an end-to-end encrypted chat.
shot-room-alt = A room where three empty chairs go to the house AI
shot-room = <b>Seats for everyone:</b> people, the house AI at five levels, or a language model.
shot-builder-alt = The deck builder filtering the pool for an angel
shot-builder = <b>The deck builder</b> searches the whole pool as you type.

about-eyebrow = What it is
about-title = Complete Rules Engine & Clear 3D Table
about-engine = Priority, the stack, layers, replacement effects, triggers, state-based actions, and combat: the Rust engine applies the complete ruleset and computes the exact legal choices at every step. The client presents only those valid actions.
about-table = Battlefield, hands, stack, exile, and graveyards on an uncluttered 3D table with clear combat lines and instant card previews. Card art streams from Scryfall while you play, or switches to a crisp oracle-text view anytime.

gw-eyebrow = Online
gw-title = Gateways

open-central = Baylee Central on GitHub

## Download

dl-prerelease = Beta
dl-for = Download for { $system }
dl-size = { $size } MB
dl-checksum = SHA-256
dl-signature = Signature
dl-notes = Release notes
dl-unknown = We could not tell your system from this browser; pick yours below.
dl-mobile = There is no version for phones and tablets yet. Baylee runs on a computer.
dl-mac-unsure = This browser does not say whether your Mac has an Apple chip or an Intel one. The file above is for Apple chips (M1 and later); an Intel Mac takes its file below.
dl-windows-unsure = On a Windows PC with an ARM processor (a Snapdragon laptop, say), take the ARM file below.
dl-linux-unsure = Your browser does not name your processor; x86-64 is assumed. The ARM file is below.
dl-missing = This release has no file for your system. Older releases are on the <releases>releases page</releases>.
dl-none = GitHub did not answer just now. Every release is on the <releases>releases page</releases>.
dl-yours = Your system
dl-installer = Installer
dl-deb = Debian package
dl-archive = Archive
dl-fallback = Archive for older processors
dl-fallback-help = For x86-64 processors without AVX2 (before about 2013). Baylee's main builds need AVX2 and do not start without it.
plat-windows-x64 = Windows
plat-windows-x64-sub = most PCs: 64-bit Intel or AMD
plat-windows-arm64 = Windows on ARM
plat-windows-arm64-sub = Snapdragon laptops and the like
plat-mac-arm64 = macOS
plat-mac-arm64-sub = Apple chips: M1 and later
plat-mac-x64 = macOS on Intel
plat-mac-x64-sub = Macs with an Intel processor
plat-linux-x64 = Linux
plat-linux-x64-sub = x86-64
plat-linux-arm64 = Linux on ARM
plat-linux-arm64-sub = ARM64 / aarch64
plat-none = No file for this system in this release.
install-title = First start and what it needs
install-first-title = First start
install-first = The builds are not code-signed, because signing costs money, so Windows and macOS warn the first time. On Windows, choose <i>More info</i> and then <i>Run anyway</i>. On macOS, open <i>System Settings → Privacy & Security</i> and choose <i>Open Anyway</i>.
install-check-title = Checking a download
install-check = Beside each file is a <code>.sha256</code> with its checksum; the archives also carry a <code>.sig</code>, the Ed25519 signature the client checks before it updates itself. On Linux, <code>sha256sum -c</code> checks a file; on macOS <code>shasum -a 256</code>; on Windows <code>certutil -hashfile</code>.
install-needs-title = What it needs
install-windows = <b>Windows</b> 10 or 11, x64 or ARM64.
install-mac = <b>macOS</b> 11 or later, on an Apple chip or Intel.
install-linux = <b>Linux</b> with glibc 2.35 or newer (Ubuntu 22.04 and later) and a Vulkan driver; the <code>.deb</code> brings what else it needs.
install-cpu = <b>Processor:</b> x86-64 builds need AVX2; an older processor takes the archive for older processors.

## Gateways

gws-title = Gateways
gws-official = Official
gws-online = Online
gws-offline = Offline
gws-version = Version
gws-website = Website
gws-address = Address
gws-copy = Copy address
gws-copied = Address copied.
gws-copied-short = Copied
gws-copy-failed = Copying did not work.
gws-failed = The directory could not be read just now. Try again in a minute.

## Questions

faq-eyebrow = Questions
faq-title = Questions and answers
faq-what-q = What is Baylee?
faq-what-a = A free fan game for Magic: The Gathering on your computer. It has a deck builder, a rules engine that checks every step, a table for up to eight players and computer opponents. It is not made, approved or endorsed by Wizards of the Coast.
faq-cost-q = What does it cost?
faq-cost-a = Nothing, and that will not change: no paywall, no paid features, no advertising.
faq-account-q = Do I need an account?
faq-account-a = Not to play alone: the deck builder and matches against the house AI work offline. For online tables you need a Baylee account; during the open beta anyone can create one on this site, free of charge.
faq-data-q = What happens to my data?
faq-data-a = As little as possible, and it is written down in the <privacy>privacy notice</privacy>. Your password never leaves your browser; your private data and your chat messages are encrypted on your device before they reach us.
faq-password-q = I forgot my password. What now?
faq-password-a = With the recovery code you received when you created your account you can set a new one. Without it, an administrator can give you a reset code, but your encrypted data cannot be recovered without the code.
faq-gateway-q = What is a gateway?
faq-gateway-a = A server where online tables live. Anyone can run one; the <gateways>directory</gateways> lists those Central knows.
faq-systems-q = Which systems does it run on?
faq-systems-a = Windows, macOS and Linux, on x86-64 and ARM64. There is no version for phones or tablets yet.
faq-open-q = Is it open source?
faq-open-a = The game is, under the GNU Affero General Public License. Baylee Central's code is public so that anyone can check how it handles their data.
faq-bug-q = I found a bug. Where do I report it?
faq-bug-a = In the game, press <kbd>F8</kbd> to send a report with what it needs. Otherwise on GitHub's issue tracker, on Discord, or through the <contact>contact form</contact>.

## Contact

contact-eyebrow = Contact
contact-title = Write to us
contact-name = Your name
contact-name-hint = Optional.
contact-email = Your e-mail address
contact-email-hint = Optional; only so that we can answer.
contact-message = Your message
contact-count = { $n } of 5,000 characters
contact-send = Send message
contact-sending = Sending …
contact-sent-title = Thank you
contact-sent = Your message arrived. If you gave an address, we will answer there.
contact-error-empty = Write a message first.
contact-error-email = This does not look like an e-mail address.
contact-error-busy = Too many messages from your network just now. Try again in an hour.
contact-error-refused = The message was not accepted. Check its length and try again.
contact-error-network = The message could not be sent. Check your connection and try again.
contact-privacy = What you send is kept for answering it and deleted 90 days after we handled it, after a year at the latest. More in the <privacy>privacy notice</privacy>.
contact-noscript = The form needs JavaScript. You can also reach us on Discord or on GitHub.

## Legal notice

impressum-title = Legal notice
impressum-lead = Information according to § 5 DDG. Baylee is a private, non-commercial project.
impressum-show = Show the details
impressum-why = The details are behind a short puzzle that your browser solves in a few seconds, to keep address-collecting robots away. Nothing else is asked of you.
impressum-solving = Solving the puzzle … { $percent } %
impressum-name = Name
impressum-address = Address
impressum-email = E-mail
impressum-phone = Phone
impressum-failed = The details could not be loaded. Try again, or write to us through the <contact>contact form</contact>.
impressum-noscript = The details need JavaScript to show. Without it, reach us through Discord or GitHub.
impressum-responsible = Responsible for the content: the person named above.
impressum-dispute = We are neither obliged nor willing to take part in dispute resolution proceedings before a consumer arbitration board.

## Fan content

fc-title = Unofficial Fan Content
fc-eyebrow = Fan content
fc-lead = Baylee is a fan project. It is permitted under Wizards of the Coast's Fan Content Policy, which sets the rules it follows.
fc-notice-title = The notice on every page
fc-means-title = What this means
fc-means = Baylee is not made, approved or endorsed by Wizards of the Coast. Magic: The Gathering, its card names, set names, card images and mana symbols are property of Wizards of the Coast LLC. Baylee is free for everyone: there is no paywall, no paid feature and no registration by e-mail.
fc-images = The game shows card images from Scryfall, unaltered and with their artist; this site shows none. Card data and images are provided by Scryfall; Baylee is not affiliated with Scryfall.
fc-policy = Wizards of the Coast's <policy>Fan Content Policy</policy>

## Privacy notice

privacy-title = Privacy notice
privacy-lead = What this site and Baylee Central process about you, why, on which legal basis and for how long. Draft, reviewed against the code on 10 November 2026; it is checked by a lawyer before it is final.
privacy-updated = As of 10 November 2026
privacy-controller-title = Who is responsible
privacy-controller = The private person named in the <impressum>legal notice</impressum>. There is no data protection officer; none is required.
privacy-short-title = In short
privacy-short = No tracking, no advertising, no analytics, no cookie unless you sign in, and nothing loaded from other servers: fonts, pictures and scripts all come from baylee.app. Your password never reaches us, and your chat messages and private data are encrypted in your browser before they are sent.
privacy-hosting-title = Visiting the site
privacy-hosting = The site runs on a server rented from netcup GmbH in Germany. When you open a page, your browser sends what every request carries: your IP address, the address asked for, the time, your browser's language and what it says about itself. The server needs this to answer, and the download page uses your browser's description to suggest the right file. The web server keeps no access log, so these requests are not written down. The server's own operating messages (its system journal) can, in rare error cases, contain an IP address; they are kept only as long as the journal keeps them. Legal basis: Art. 6(1)(f) GDPR (delivering the site safely).
privacy-device-title = What your browser keeps
privacy-device = This site sets no cookie until you sign in. What your browser keeps on your device, and only with JavaScript: your colour choice if you made one (<code>baylee-site-theme</code>); while you are signed in, when your session ends (<code>baylee-central-session</code>), your chat key and your contacts' trusted keys (see Chat); if you turned on notifications, a random number for this device (<code>baylee-push-installation-v1</code>, <code>baylee-push</code>); and, if you install the site as an app, a copy of its public files for offline use. None of this is sent to us unless a section below says so; the chat and session entries are deleted when you sign out. Legal basis: § 25(2) no. 2 TDDDG.
privacy-github-title = GitHub
privacy-github = The download page and the start page show the newest release, the test status and the card numbers. Our server asks GitHub for them, not your browser; GitHub learns nothing about you. A download link leads to GitHub (GitHub Inc., USA) only when you click it; GitHub's own privacy notice then applies.
privacy-directory-title = The gateway directory
privacy-directory = Our server reads the directory of gateways from Baylee Central and shows it; nothing about you is involved.
privacy-account-title = Your account
privacy-account = For an account, Baylee Central stores: your sign-in name and display name with its tag, an OPAQUE password record (which lets you prove your password without our server ever learning it), your master key twice encrypted (by your password and by your recovery code), your encrypted account document, which version of the terms you accepted, the times of your sign-ins, your sessions, and the digest of the beta key you used. Legal basis: Art. 6(1)(b) GDPR (the account you asked for). Kept until you delete the account; sessions end after 30 minutes without use and at the latest 12 hours after signing in.
privacy-cookie-title = The session cookie
privacy-cookie = Signing in sets one cookie, <code>__Host-central</code>: a random number that identifies your session, sent only to baylee.app, never readable by scripts, ending with the session. It is strictly necessary for being signed in. Legal basis: § 25(2) no. 2 TDDDG and Art. 6(1)(b) GDPR.
privacy-chat-title = Chat and contacts
privacy-chat = Messages are encrypted on your device for each recipient's devices; Baylee Central stores them only in encrypted form and cannot read them. What it can see and stores: who is in contact with whom, the members of groups and their names, when a message was sent, how large it is, and your devices' public keys. Your browser keeps your chat key (sealed under a key only this browser holds, in its storage <code>baylee-chat</code>) and the keys of your contacts that you trusted (encrypted, <code>baylee-chat-trust</code>) while you are signed in. Messages and deliveries are kept until you delete your account; each device keeps at most 5,000. Legal basis: Art. 6(1)(b) GDPR.
privacy-push-title = Notifications
privacy-push = If you turn on notifications in a browser, your browser registers with its push service, and Central stores that address and a random number for this device. The push service belongs to your browser's maker: Google (Firebase Cloud Messaging), Mozilla or Apple. A notification carries only its kind (for example "new message") and your language, never names or text. It is deleted when you turn notifications off on this device, when you sign out, or when you delete your account; an address not renewed for 90 days is no longer used. Legal basis: Art. 6(1)(a) GDPR (your consent, which you can withdraw at any time in your account's settings).
privacy-counters-title = Protection against abuse
privacy-counters = To stop guessing and flooding, Central counts certain requests per IP address (registrations, sign-in attempts, password resets, reading the directory), only in memory and for at most one hour. Legal basis: Art. 6(1)(f) GDPR.
privacy-contact-title = The contact form
privacy-contact = The form sends your message, your name and e-mail address only if you give them, the page's language and the time to Baylee's feedback service on the same server; it also notes whether the message was dealt with. Your IP address is not stored; for an hour it is counted in memory only, to stop a flood of messages. Only the person named in the legal notice reads the messages; nothing is passed on. A message is deleted 90 days after it was dealt with, after one year at the latest, or earlier when you ask. Legal basis: Art. 6(1)(f) GDPR (answering messages), or (b) where your message aims at a contract.
privacy-impressum-title = The legal notice's puzzle
privacy-impressum = To show the legal notice, the feedback service counts puzzles per IP address in memory for one hour and keeps a solved puzzle for five minutes. Legal basis: Art. 6(1)(f) GDPR.
privacy-recipients-title = Recipients and countries
privacy-recipients = We sell nothing and share nothing. Processors: netcup GmbH (hosting, Germany). Only when you turn on notifications does your browser's push service (Google, Mozilla or Apple) carry them, possibly from outside the EU.
privacy-rights-title = Your rights
privacy-rights = You have the right of access (Art. 15), rectification (16), erasure (17), restriction (18), data portability (20) and to object (21). Your account's data can be exported and the account deleted in your account's settings. You may complain to a supervisory authority, for example the State Commissioner for Data Protection of North Rhine-Westphalia. There is no automated decision-making (Art. 22).

## Terms

terms-title = Terms of use
terms-draft = These terms are a draft and not yet in force.
terms-version = Version { $version }, updated { $date }.

## Errors

not-found-title = This page does not exist
not-found-lead = The address may be mistyped, or the page has moved.
gone-title = This page is gone for good
gone-lead = What was here has been removed. The start page shows where to go on.
error-home = To the start page

## Baylee's remembrance (approved by her family on 27 October 2026; keep word for word)
baylee-dedication = For Baylee. Forever in the sunlight.
baylee-invitation = Meet the little soul behind this world
baylee-open = Remember Baylee · open her photo album
baylee-eyebrow = The heart of this project
baylee-date = In loving memory · 26 October 2026
baylee-close = Back to her world
baylee-previous = Previous photograph
baylee-next = Next photograph
baylee-photo1 = Baylee curled up in the soft white duvet.
baylee-photo2 = Baylee looking out from the flowered duvet.
baylee-photo3 = Resting in her round, grey bed.
baylee-photo4 = Baylee asleep, curled up close.
baylee-photo5 = A quiet moment, nestled in the duvet.

baylee-storyTitle = For Baylee. Forever in the sunlight.
baylee-together = With us · 17 November 2025 – 26 October 2026
baylee-chapters = Her story
baylee-chapterPrevious = Previous chapter
baylee-chapterNext = Next chapter
baylee-photos = Our memories
baylee-read = Scroll to read

baylee-chapter1Title = A home for Baylee
baylee-chapter1Body =
    Baylee came home to us on 17 November 2025.

    My wife and I had adopted her from the animal shelter in Krefeld. She was estimated to be about thirteen years old. She had malignant mammary tumours, and the cancer had already spread to her liver. Baylee was a palliative-care cat. We knew our time together would be limited. With us, she was to have a home where she was loved.

    She soon showed us just how quickly she would claim that home as her own.

    On her very first day, she slipped onto my lap while I was working. And on her first night, she tore a hole in a bag of cat food I had ordered for my parents’ cats and helped herself. Of course she had been given her own food and had eaten well that evening. But Baylee had apparently decided that this supply still deserved a closer investigation.

    Our little mouse had an astonishing appetite – and an incredible amount of energy.

baylee-chapter2Title = Growing trust
baylee-chapter2Body =
    When we noticed that she was drinking a lot of water, we took her to the vet. A blood test revealed a thyroid condition, which also explained her great hunger and lively nature. She was given thyroid medication and a temporary course of antibiotics because her inflammation markers were elevated.

    When she suddenly began rubbing herself against everything, meowing a lot and becoming unusually cuddly and attached to us, we initially worried that she was reacting badly to one of her new medicines. Another examination revealed the explanation: our little darling was in heat. She had not been spayed.

    We got through that time together, too. Baylee trusted us more and more, and we grew ever closer to one another.

baylee-chapter3Title = Our little princess
baylee-chapter3Body =
    I still remember exactly how happy she was when she got her cat tree. She immediately sat on the very top perch and looked contentedly around the flat. Strangely enough, she never used that particular spot again. Why remained her secret.

    Other places were rarely left unoccupied: my lap, my legs and our bed. There she usually slept on me or on my wife. Best of all was managing to lie on both of us at once.

    She strutted through the flat with her tail held high, as though she knew perfectly well that she was the princess here. She had spirit and knew how to make her boundaries clear. With us, she could be entirely herself. Her quirks, her demands and her little stubborn streak were all loved, too.

    I had never formed such a deep bond with an animal before. Baylee found a place very deep in both my wife’s heart and mine.

baylee-chapter4Title = Hours in the sunshine
baylee-chapter4Body =
    When the weather grew warmer, she discovered the balcony. She loved feeling the day’s first rays of sunshine in her fur. Before long, she was spending almost the entire day out there.

    The pigeons and garden birds were a particular fascination. She watched them intently, lay in wait and repeatedly tried to startle them. Between sunbathing and birdwatching, our little princess had a very full daily schedule.

baylee-chapter5Title = Her own little mind
baylee-chapter5Body =
    Meanwhile, her illness continued to progress.

    The tumour became inflamed, and Baylee kept licking the affected area open. At the vet’s she was given her first recovery suit, a red one. From then on, she had to wear one every day to keep her from injuring the wound further.

    She quickly got used to wearing it. Having it put on, however, was something she disliked right up to her last day. Even a bowl of food could only do so much to coax her out then. Big as her appetite was, her opinion of the suit was unmistakable.

    The wound grew worse. It discharged pus, eventually began to bleed and developed an unpleasant smell. At the same time, Baylee kept showing us how much joy she still found in her everyday life. She played, ate, explored her surroundings and climbed her scratching board and the cupboards.

    Sometimes she lay for hours on the edge of the bed beside us, waiting for us to finally wake up and make her breakfast. If that took too long, she simply climbed over us. Our faces were no obstacle.

    She had very definite ideas about food. Her taste was rather in the “frozen pizza” camp: after a while, high-quality cat food could no longer excite her, and variety was essential. The same food twice in a row was out of the question. Chicken with liver, anything with cheese, tuna or salmon – those were things that made her happy.

baylee-chapter6Title = With her tail held high
baylee-chapter6Body =
    The tumour grew larger and wept increasingly. Baylee was visibly in pain. When the pain eased, she was once again our little cat, moving through the rooms with her tail held high and claiming her people for herself.

    We could see her strength slowly fading. But her appetite, her curiosity, her affectionate nature and her loud purr were still there.

baylee-chapter7Title = Saying goodbye
baylee-chapter7Body =
    On Friday evening, 23 October 2026, she developed symptoms that at first seemed like a cold. Her nose was running, and her breathing was deep and laboured, as though she were struggling for air.

    On Saturday she seemed livelier again. The unusual breathing and runny nose remained, but she once more showed the energy and love of life we knew so well.

    Sunday morning was difficult. Her eyes were watering, she was struggling to breathe and she was in pain. I gave her pain medication. After about an hour she felt better again, but her breathing remained different. We knew we needed to take her to the vet.

    On Monday morning her condition worsened again. Fortunately, my wife had the day off. We called the practice, made an appointment and took Baylee there.

    The X-ray showed how far the cancer had spread. We were told that only about a third of her lungs remained available for breathing. Letting her continue to live would have meant that she would get less and less air.

    We had to say goodbye.

    On Monday, 26 October 2026, at around noon, our dearest Baylee was put to sleep.

baylee-chapter8Title = Forever in the sunlight
baylee-chapter8Body =
    She was with us for just under a year. In that time, she had become part of our family and a very close friend. She belonged to our working days, our nights and our waking up in the morning. To sunny hours on the balcony and to the little habits that make up a life together.

    Now she is missing from all those moments.

    The emptiness is still fresh and deep. We miss her purr, her weight on our legs, her upright tail and her unquestioning expectation that we would finally see to her breakfast.

    If there is something after this life, whatever it may be, then I wish to meet that soul again one day.

    Until then, we will carry our little mouse in our hearts forever.

    That is why this hobby project is dedicated to Baylee. To her spirit, her closeness, her joy in the sun and the birds. And to the love that grew during our time together.

    Here she may sleep peacefully on her cloud, in the warm sunshine, surrounded by flowers, pigeons and garden birds.

    Rest in peace, my dearest Baylee.

baylee-photoOpen = View full photograph
baylee-photoClose = Close photograph
baylee-zoomIn = Zoom in
baylee-zoomOut = Zoom out
baylee-fit = Fit

baylee-photo6 = Baylee, cosy among the blankets.
baylee-photo7 = Snuggled up in her red recovery suit.
baylee-photo8 = A sunny spot in a cardboard box.
baylee-photo9 = Tucked in and completely relaxed.
baylee-photo10 = Her favourite place: right beside us.

## Errors shared by the forms

error-busy = Too many attempts just now. Wait a minute and try again.
error-unavailable = Baylee Central does not answer just now. Try again in a moment.
error-network = No connection. Check your network and try again.
error-failed = Something went wrong. Try again; if it keeps happening, tell us.
error-again = Try again
password-show = Show

## Signing in

signin-title = Sign in
signin-lead = Your Baylee account, for online tables, contacts and the chat.
signin-name = Sign-in name
signin-password = Password
signin-submit = Sign in
signin-missing = Enter your sign-in name and password.
signin-working = Securing your password … this takes a moment and happens only in your browser.
signin-wrong = That sign-in name and password do not match. Check both and try again.
signin-not-moved = This account has not moved to Baylee Central yet. Sign in once with the Baylee client; it moves your account.
signin-no-account = No account yet? <register>Create one</register>.
signin-noscript = Signing in needs JavaScript: your password is processed in your browser and never sent to us.

## Registering

register-title = Create an account
register-lead = For online tables, contacts and the chat. Free and open to everyone in the open beta.
register-name = Sign-in name
register-name-hint = Private, only for signing in: 3 to 24 lowercase letters, digits, dots, dashes or underscores.
register-display = Display name
register-display-hint = How other players see you: 3 to 16 letters, digits, - and _. Your tag is drawn for you.
register-password = Password
register-password-hint = 8 characters at least. A long sentence is easy to remember and hard to guess.
register-repeat = Password again
register-repeat-hint = The same once more, to catch a typo.
register-age = I am 16 or older.
register-consent = I accept the <terms>terms of use</terms> and have read the <privacy>privacy notice</privacy>.
register-submit = Create account
register-error-name = The sign-in name needs 3 to 24 lowercase letters, digits, dots, dashes or underscores.
register-error-display = The display name needs 3 to 16 letters or digits; - and _ only inside.
register-error-reserved = This display name is reserved. Choose another.
register-error-short = The password needs 8 characters at least.
register-error-long = The password is too long.
register-error-repeat = The two passwords are not the same.
register-error-age = You need to be 16 or older to have an account.
register-error-consent = Accept the terms of use to create an account.
register-error-taken = This sign-in name is taken. Choose another.

## The recovery code

code-title = Your recovery code
code-handle = Your handle: { $handle }
code-lead = This code is your only way back in if you forget your password. We cannot reset your password for you. Keep it somewhere safe, away from this device.
code-shown-once = Shown only now
code-download = Save as a text file
code-kept = I have saved my recovery code
code-not-kept = Save the code first, then confirm it.
code-continue = Continue
code-later = Not now

## Central's terms

terms-step-title = New terms of use
terms-step-lead = Baylee Central's terms changed. Read them and accept them to use your account; you can also decline and sign out.
terms-accept = Accept
terms-decline = Decline and sign out

## The signed-in area

account-lead = Your Baylee account: profile, security, privacy and gateways.
account-nav = Your account
account-home = Overview
account-profile = Profile
account-profile-lead = Your handle, your display name and your recovery code.
account-security = Security
account-security-lead = Where you are signed in, and signing out.
account-privacy = Privacy and data
account-privacy-lead = Export your data or delete your account.
account-gateways = Gateways and tokens
account-gateways-lead = Gateways you registered and the tokens they sign in with.
account-loading = Loading …
account-signed-out = You are not signed in.
account-unnamed = Your account
account-signin-name = Sign-in name: { $name }
account-until = Signed in until { $time }
account-needs-title = Needs you
account-needs-code = Make a recovery code
account-needs-name = Choose your display name
account-admin = Administration

reauth-lead = This needs a recent sign-in. Enter your sign-in name and password once more.
reauth-submit = Sign in again

profile-who-title = Who you are
profile-handle = Handle
profile-signin-name = Sign-in name
profile-source = Account
profile-source-moved = Moved from the official gateway
profile-source-registered = Registered on this site
profile-choose-title = Choose your display name
profile-choose-submit = Save display name
profile-name-taken = This account has a display name already.
profile-code-title = Recovery code
profile-code-kept = Your account has a recovery code. Keep it safe; it is the only way back in without your password.
profile-code-missing = Your account has no recovery code yet. Make one: it needs your password once more.
profile-code-make = Make a recovery code

security-sessions = Where you are signed in
security-lead = Baylee Central keeps one session for this website and one for the game client; it knows no devices.
security-kind-browser = This website
security-kind-client = The Baylee client
security-this = This one
security-signed-in = Signed in
security-ends = Ends
security-gateways = At gateways
security-end-client = Sign the client out
security-client-ended = The client is signed out.
security-sign-out = Sign out here
security-password-title = Password
security-password-later = Changing your password on the website comes with the next step; until then, change it in the Baylee client.

privacy-export-title = Export your data
privacy-export-lead = Everything Baylee Central stores about your account, as a JSON file. It needs a sign-in less than 30 minutes old.
privacy-export-make = Prepare the export
privacy-export-save = Save the export
privacy-delete-title = Delete your account
privacy-delete-lead = Your account, its data, its sessions and the gateways you own are deleted at once and for good. Messages you sent stay with those who received them.
privacy-delete-start = Delete my account
privacy-delete-password = Your password, to confirm
privacy-delete-prove = Continue
privacy-delete-last = Last step: this cannot be undone. You have one minute.
privacy-delete-gateways = These gateways you own are deleted with it:
privacy-delete-confirm = Delete account for good
privacy-delete-cancel = Keep my account
privacy-deleted = Your account is deleted.

gateways-none = You have not registered a gateway. Most players never need one.
gateways-add-title = Register a gateway
gateways-origin = Its address
gateways-origin-hint = The gateway's https address, like https://play.example.org.
gateways-name = Its name
gateways-add = Register gateway
gateways-added = The gateway is registered and waits for an administrator's approval.
gateways-taken = This address is registered already.
gateways-invalid = Enter an https address without a path, and a name.
gateways-approved = Approved
gateways-pending = Awaiting approval
gateways-disabled = Disabled
tokens-title = Tokens
tokens-name = Token name
tokens-days = Days it lives
tokens-sso = Signs players in
tokens-listing = Lists the gateway
tokens-both = signs players in and lists the gateway
tokens-make = Make a token
tokens-revoke = Revoke
tokens-revoked = Revoked
tokens-revoked-now = The token is revoked.
tokens-until = Until { $time }
tokens-invalid = Give the token a name and at least one thing it allows.
tokens-shown-title = Your new token
tokens-kept = I have kept it
tokens-again = Signed in again. Make the token once more.

## The messenger

account-chat = Chat
account-contacts = Contacts
chat-side = Conversations and contacts
chat-tabs = Messenger
chat-tab-chats = Chats
chat-tab-contacts = Contacts
chat-main = Conversation
chat-unread = unread
chat-conversations = Your conversations
chat-search = Find a conversation
chat-search-hint = Find (Ctrl+K)
chat-new-group = New group
chat-no-conversations = No conversations yet. Add a contact, then write to them.
chat-no-match = No conversation has that name.
chat-former-group = A former group
chat-former-lead = You are no longer in this group.
chat-you = You
chat-members = { $n ->
    [one] { $n } member
   *[other] { $n } members
}
chat-say-hello = Say hello.
chat-history-only = No longer a contact.
chat-today = Today
chat-yesterday = Yesterday
chat-encrypted = End-to-end encrypted
chat-opening-lead = Your conversations and contacts. Messages are encrypted on your devices before they leave them, and Central passes them on without being able to read them.
chat-new-from = New message from { $who }.
chat-new-many = { $n } new messages.
chat-retry = Try again
chat-discard = Discard
chat-not-sent = Not sent.
chat-sending = Sending …
chat-keys-changed = Someone's keys in this conversation changed. Compare the safety number before you write again.
chat-refused = Some messages could not be verified and are not shown.
chat-failed = The chat could not reach Central just now.
chat-failed-open = The chat could not be opened. Reload the page to try again.
chat-empty = No messages yet.
chat-older = Show older messages
chat-compare-first = The keys changed. Compare the safety number first, then write.
chat-compare = Compare the safety number
chat-friend-required = You can write once you are contacts again.
chat-group-left = You are no longer in this group.
chat-details = Details
chat-close = Close
chat-back = Back to the conversations
chat-messages = Messages
chat-fan = Baylee is unofficial Fan Content, not approved or endorsed by Wizards. <more>What that means</more>
chat-safety = Safety numbers
chat-safety-lead = Compare the number with the other person, in person or on another channel you trust. If it matches, nobody else is reading along.
chat-changed = Changed
chat-confirm-keys = It matches: trust the new keys
chat-count = { $n } of 1,000 characters
chat-write = Write a message
chat-send = Send
chat-enter-hint = Enter sends, Shift+Enter starts a new line.
chat-choose = Choose a conversation
chat-choose-lead = Your messages are encrypted in this browser; Central only passes them on.
chat-storage = Your chat key stays in this browser while you are signed in.
chat-opening = Opening your chats …
chat-open = Open my chats
chat-ended = Your sign-in has ended.
chat-unnamed = Choose your display name first: it gives you the handle others write to.
chat-missing-title = Sign in again to open your chats
chat-missing-lead = This browser does not hold your chat key. Your password opens it here; it never leaves this browser.
chat-invites = Group invitations
chat-invited-by = from { $who }
chat-group-name = Group name
chat-group-name-hint = A name of 1 to 48 characters.
chat-group-create = Create
chat-group-failed = The group could not be changed just now.
chat-group-members = { $n ->
    [one] { $n } member
   *[other] { $n } members
}
chat-group-owner = Owner
chat-group-rename = Rename
chat-group-transfer = Make owner
chat-group-remove = Remove
chat-group-transfer-ask = Make { $who } the owner of this group? You stay a member.
chat-group-remove-ask = Remove { $who } from this group?
chat-group-invite = Invite a contact
chat-group-choose = Choose a contact
chat-group-invite-button = Invite
chat-group-invited = Invited
chat-group-delete = Delete group
chat-group-leave = Leave group
chat-group-delete-ask = Delete “{ $name }” for everybody? Messages already received stay readable.
chat-group-leave-ask = Leave “{ $name }”? You can only come back with a new invitation.

people-incoming = requests
people-failed = Your contacts could not be loaded just now.
people-empty = No contacts yet. Find someone by their handle, or give them yours.
people-your-handle = Your handle:
people-incoming-title = Requests for you
people-friends = Contacts
people-outgoing = Sent requests
people-blocked = Blocked
people-find = Add a contact by their handle
people-find-button = Find
people-handle-hint = A full handle with its tag, like Name#1a2b.
people-yourself = That is your own handle.
people-nobody = Nobody has this handle.
people-request = Send a contact request
people-requested = Request sent.
people-is-friend = A contact
people-is-incoming = Asked to be your contact
people-is-outgoing = Request sent
people-is-blocked = Blocked
people-write = Write
people-remove = Remove
people-block = Block
people-accept = Accept
people-decline = Decline
people-cancel = Withdraw
people-unblock = Unblock
people-keep = Keep
people-confirm-remove = Remove { $who } from your contacts? Your messages stay.
people-confirm-block = Block { $who }? They can no longer ask to be your contact or write to you.

## Offline, updates, installing

title-offline = Offline
offline-title = You are offline
offline-lead = This page needs a connection. Nothing is lost: Central keeps your messages until you are back.
offline-again = Try again
update-ready = A new version of baylee.app is ready.
update-unsent = You have text here that is not sent yet. Reload anyway?
update-reload = Reload
install-app = Install as an app

## Notifications

push-title = Notifications
push-lead = Choose what Baylee may notify you about. Notifications show no message text, names or report contents.
push-device = Notifications on this device
push-categories = Notify me about
push-chat = Chat messages
push-contacts = Contact requests
push-updates = New versions
push-feedback = Feedback reports (administrators)
push-gateways = Gateway applications (administrators)
push-on = Enabled on this device. Only the selected categories are delivered.
push-off = Disabled on this device. Your category choices are kept.
push-scope = Category choices apply to your account. Device permission is separate on each browser. Signing out disconnects this device.
push-failed = The change could not be confirmed. Please try again.
push-denied = Notifications are blocked. You can allow them in your browser or device settings and try again.
push-unsupported = This browser does not offer Web Push here. On iPhone and iPad, add Baylee to the Home Screen and open it there.
push-unconfigured = Background notifications are not available on this server yet. Your preferences can already be saved.

register-closed = Registration is paused right now. Please try again later.
register-refused = The registration was not accepted. Please try once more.
admin-registration = Registration
admin-registration-open = Open: anyone may create an account, with the proof of work.
admin-registration-closed = Closed: nobody can create an account until you open it again.
admin-registration-lead = Close registration during an attack and open it again by hand. Each change is in the audit.
admin-registration-close = Close registration
admin-registration-reopen = Open registration again
hero-discord = Discord Community
hero-whatsapp = WhatsApp Community
hero-github = GitHub
nav-menu = Menu
foot-project = Project
foot-source = Source code (AGPL-3.0)
foot-central = Baylee Central (source-available)
foot-summary = Free, open-source tabletop card game built in Rust & Bevy — with a deterministic rules engine, full offline mode, House AI, LLM bridge, and community gateways.
strip-build = Build
strip-ci = CI
strip-commit = Last commit
strip-gateways = Gateways
strip-online = online
strip-offline = offline
strip-releases = see releases
strip-on-github = see on GitHub
strip-directory = the directory
ci-passing = passing
ci-failing = failing
dl-button = Download Baylee
dl-quick = Other systems and architectures
dl-others = other systems

# The start page, as the old site wrote it (owner, 11.11.2026)
link-readmeModel = #playing-against-a-language-model
about-engineTitle = An engine that enforces every rule
about-deterministic = Powered by a seeded random generator with zero wall-clock or hash-order dependencies, every match is strictly deterministic and reproducible move by move.
about-tableTitle = A clean 3D table built with Bevy
about-keys = From the integrated lobby you move effortlessly between the local deck builder, solo matches, and multiplayer tables on community gateways.
feature-hiddenTitle = Hidden information stays private
feature-hidden = Your client receives only the data your seat is allowed to see. Libraries and opponents' hands arrive strictly as counts—even a modified client cannot inspect hidden cards.
feature-seatsTitle = Up to eight players at one table
feature-seats = Duels, free-for-all, and team matches (such as 2v2) for up to 8 seats. Teammates share turns and can reveal their hand cards to one another.
feature-commanderTitle = Flexible formats & custom house rules
feature-commander = Assemble decks for classic 60-card duels, multiplayer rings, or custom house formats. Starting life totals, team layouts, and mulligan rules are freely set by the host.
feature-aiTitle = Deterministic replays & F8 reports
feature-ai = Every match is recorded move by move. If you spot a rules interaction issue during play, press F8 right at the table to send a reproducible state report.
feature-processTitle = Isolated server process per match
feature-process = Every active game runs in its own sandboxed engine process on the gateway. The gateway itself only manages lobby and rooms—an error never affects other tables.
feature-languagesTitle = Card text in 19 languages
feature-languages = Whenever a gateway provides the multilingual card catalog, cards read in your preferred language with English as fallback. The client UI is available in German and English.
feature-watchTitle = Spectate live matches
feature-watch = The lobby's Watch button seats you at the table as a live spectator: follow the public battlefield in real time while all player hands remain hidden.
feature-connectionTitle = Seamless reconnect on drop
feature-connection = If a connection drops briefly, the table holds the seat with a countdown and can pause the match or let the House AI step in temporarily.
feature-tablesTitle = Fourteen decision clocks
feature-tables = From brisk 15-second turns to relaxed untimed tables, configure the pace individually for every room.
shots-eyebrow = At the table
shots-title = Inside the game
shots-lead = Short live recordings captured directly from the 3D client with card images from Scryfall (© Wizards of the Coast)—from duels and 4-player rings to the encrypted lobby and deck builder.
shots-credit = Card images: Scryfall · © Wizards of the Coast
shots-showLarge = Show large: { $title }
shots-dialog = Screenshot
shots-previous = Previous
shots-next = Next
shots-close = Close
ai-eyebrow = AI Opponents & Language Models
ai-title = House AI, LLM Bridge & Native Baylee AI
ai-houseTitle = Heuristic House AI in five skill levels
ai-house = Ready to play offline and at any online table: choose between Novice, Casual, Steady, Sharp, and Expert. The House AI decides locally with zero wait time and sees only the same public state as a human player.
ai-modelTitle = Language models at the table via Seat-Bridge
ai-model = Seat <b>Google Gemini, OpenAI ChatGPT, Anthropic Claude, DeepSeek</b>, or local models via <b>LM Studio</b> right at your table. API keys stay protected in environment variables with spending caps; on timeouts, the House AI steps in seamlessly.
ai-setup = LLM & Seat-Bridge setup guide
home-gwLead = Online tables live on gateways, servers that Baylee Central lists once its administrator approved them. Here are the first few; the directory has them all.
beta-open = Open the directory
beta-own = Run your own
source-eyebrow = Source
source-title = Open source, and it stays that way
source-promise = Baylee will always be open source and free. No paywall, no paid features, no ads. Not now, not later. The code is under the GNU Affero General Public License, which means anyone who runs a modified gateway for other people must offer them its source too.
source-rust = It is written in Rust, one person's project with a lot of help from coding agents, and every change goes through the same gate: formatting, lints, the test suite, and a validation of every card against its printed text. Issues and pull requests are welcome; <agents>AGENTS.md</agents> and <claude>CLAUDE.md</claude> are the working rules for people and agents alike.
source-discord = Community on Discord
source-whatsapp = Community on WhatsApp
source-architecture = Architecture
source-repoTitle = The repository right now
source-stars = Stars
source-issues = Open issues and pull requests
source-note = Read from GitHub's public API by this site's server every ten minutes. Your browser asks GitHub nothing.
gw-count = 
    { $n ->
        [one] { $n } gateway listed, { $online } online.
       *[other] { $n } gateways listed, { $online } online.
    }
gw-none = No gateway is listed yet.
contact-intro = A question, an idea, a game you would like to see, or something you would rather not post in public: this form goes straight to the person behind Baylee. Leave an e-mail address if you want an answer; without one we can only read it.
contact-f8 = A card that resolved wrongly is best reported from the game with <kbd>F8</kbd>, because the report can carry the whole game.
contact-discord = talk, find opponents, hear about builds
contact-issues = GitHub issues
contact-issuesSub = bugs and ideas, in public
contact-operator = Who runs this site, with a postal address: the <impressum>legal notice</impressum>.
contact-whatsappSub = Community group & quick matches
contact-reportsTitle = Reports & privacy
contact-reportsSub = Details on F8 reports & data
ai-housePill = Offline & online
ai-providers = Supported language models
ai-hostsPill = For gateway hosts
ai-hostsTitle = Offer hosted LLM seats on your gateway
ai-hosts = As a gateway operator, you can provide pre-configured language models directly on your server. Guests at a table simply pick one of your hosted models as an opponent—with no API key needed.
ai-trainingPill = In training · native engine AI
ai-trainingTitle = Native Baylee AI — trained to play directly on the engine like a real player
ai-training = Alongside the House AI and external LLM bridge, a dedicated native Baylee AI is being trained directly on the deterministic rules engine. It learns to evaluate board states, priority windows, combat math, and deck synergies like an experienced human player—with zero external API latency.
ai-hostingLink = Gateway hosting & LLM setup

# The gateways and the contact page, as the old site wrote them (owner, 11.11.2026)
gw-lead = A gateway is a server where online tables live: accounts, decks, rooms and one engine process per game. These are the ones registered with Baylee Central and approved by its administrator. Baylee runs the one marked official; the others are run by other people for their groups, and their own rules apply there. A listing is not a promise about what a gateway does.
gw-howTo = To play on one, copy its address into the client's front door. Online means Central heard from the gateway within the last minute and a half; it is no promise that it answers. Names and versions are what each operator reports.
gw-find = Find a gateway
gw-findHint = Name or address
gw-onlineOnly = Online only
gw-shown = { $n } of { $all } shown
gw-noMatch = No gateway loaded so far matches.
gw-showAll = Show all
link-readmeGateway = #running-your-own-gateway
selfhost-eyebrow = Self-hosting & Central Integration
selfhost-title = Run your own gateway
selfhost-processes = A self-hosted gateway runs three lightweight processes alongside PostgreSQL 18+: <code>baylee-gateway</code> manages rooms, decks, and routing; <code>baylee-agent</code> spawns one isolated <code>baylee-engine-server</code> process per match so every table is sandboxed.
selfhost-proxy = Put a TLS reverse proxy in front of your HTTPS origin (e.g. <code>https://play.example.org</code>), optionally import Scryfall's bulk catalog for card text in 19 languages, and players can connect directly via your address in the client.
selfhost-central = <b>Link with Baylee Central & configure privacy:</b> Register your HTTPS origin under <mine>Your gateways</mine> and mint a <code>bgt_…</code> token (scopes: <code>sso</code> for single sign-on, <code>listing</code> for directory heartbeats & live telemetry). In your gateway's console, choose your telemetry privacy level (<code>full</code> for running games, table settings & opted-in players, <code>counts_only</code> for totals only, or <code>private</code> for liveness only).
selfhost-guide = <guide>Read the complete self-hosting guide</guide> · <protocol>Protocol & heartbeat reference (docs/protocol.md)</protocol>
selfhost-source = Every gateway answers <code>GET /source</code> with its licence, version and commit without sign-in, fulfilling the AGPL source-disclosure requirement automatically.
reports-eyebrow = Reports
reports-title = Found something wrong? Tell us
reports-f8 = Press <kbd>F8</kbd> at the table or in the lobby. A report carries its kind, your text, the client version and, at a table, the game's id. Everything more has a checkbox of its own, and every box starts ticked, so untick what you would rather keep before you press Send: system and hardware, the table as you see it, your log (other players renamed "Player A", "Player B", …), your settings, a picture of the Baylee window only, and at a table the game's whole record (every move and every seat's cards, no names). <i>Show what is sent</i> shows every byte. Session tokens are never sent.
reports-usage = While you play offline, the client says once a minute that a game is running: no account, no device id, nothing about the game. <i>Send anonymous usage count</i> under Settings › Privacy & Data switches it off.
reports-discord = Want to talk, find opponents or hear first about new builds? Join the <discord>Baylee community on Discord</discord>.
reports-issues = Rules questions, wrong card behaviour, ideas: <issues>open an issue on GitHub</issues>. A card that resolved wrongly is the most useful report we can get, because every card has a test and yours becomes one.
reports-privacy = <docs>docs/privacy.md</docs> lists everything the client, the gateway and the feedback service keep. This site sets no cookie unless you sign in, runs no analytics and loads no fonts or scripts from anyone else's server. <notice>The privacy notice</notice> says what the site and Baylee Central keep, and names every other host they talk to, and when.

# The download page, as the old site set it (owner, 11.11.2026)
dlPage-eyebrow = Get it
dlPage-filesEyebrow = Every file
dlPage-filesTitle = All systems
install-eyebrow = Installing
install-releases = Every release on GitHub
release-notes = Release notes and checksums
release-all = All releases

# The download page's head, as the old site wrote it
dlPage-title = Download
dlPage-lead = Every download goes straight to the file on GitHub, nothing in between. Where a release has installers, they are the easy way; the archives hold the same game to unpack anywhere. An installed Baylee updates itself from the signed archives wherever it may write its own folder. Every package carries the seat bridge for language models beside the game.

# The reading pages' eyebrows, as the old site set them
imp-eyebrow = Legal notice
terms-eyebrow = Terms of use
privacy-eyebrow = Privacy

# The privacy notice's contents, as the old site named them
privacy-contents = Contents

# The account pages' eyebrow, as the old site set it
acct-eyebrow = Baylee Central

# The account area's navigation, as the old site named it
app-account = Your account
