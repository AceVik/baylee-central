# baylee.app's words in English, Project Fluent's format. English decides
# which messages exist and which arguments each takes; build.rs turns them
# into typed functions (crates/central-web/build.rs). The reader is "you";
# a button says what it does.

## The frame of every page

site-name = Baylee
skip = Aller au contenu
# Read out after a link that leaves the site (the stylesheet carries it).
new-tab = s’ouvre dans un nouvel onglet
nav-label = Principal
nav-download = Télécharger
nav-gateways = Gateways
nav-faq = Questions
nav-contact = Contact
nav-sign-in = Connexion
nav-account = Votre compte
lang-label = Langue
theme-label = Thème de couleurs
theme-light = Couleurs : clair
theme-dark = Couleurs : sombre

foot-label = À propos de ce site
foot-play = Jouer
foot-legal = Mentions légales
foot-releases = Toutes les versions
foot-issues = Signaler un bug
foot-discord = Communauté Discord
foot-central-source = Source de ce site
foot-dedication = Pour Baylee. À jamais dans la lumière du soleil.
fan-content = Baylee est un contenu de fan non officiel autorisé dans le cadre de la <policy>Politique des contenus de fans</policy>. Ni approuvé, ni promu par Wizards. Certaines parties des matériaux utilisés sont la propriété de Wizards of the Coast. ©Wizards of the Coast LLC.
foot-colophon = Polices : Literata et Inter, licence SIL Open Font 1.1, servies depuis ce site. Le tableau et le logo sont de nous. <notices>Mentions</notices>

## Titles and descriptions

title-home = Baylee – Construisez vos decks. Réunissez vos amis. Jouez, tout simplement.
desc-home = Baylee est un jeu de fan gratuit et open source pour Magic: The Gathering sous Linux, Windows et macOS : un moteur de règles qui connaît les cartes, des tables jusqu’à huit joueurs, et des adversaires quand personne d’autre n’est là.
title-download = Télécharger Baylee
desc-download = Téléchargez le client de Baylee pour Windows, macOS et Linux : installateurs et archives de la dernière version, avec sommes de contrôle et signatures.
title-gateways = Gateways : où l’on joue à Baylee en ligne
desc-gateways = L’annuaire public des gateways de Baylee, les serveurs où vivent les tables en ligne, et comment faire tourner la vôtre.
title-faq = Questions et réponses
desc-faq = Ce qu’est Baylee, ce qu’il coûte, comment fonctionnent le jeu en ligne et les comptes, et ce qu’il advient de vos données.
title-contact = Contact
desc-contact = Écrivez à la personne derrière Baylee, ou signalez un bug.
title-impressum = Mentions légales
desc-impressum = Qui exploite baylee.app (§ 5 DDG).
title-privacy = Politique de confidentialité
desc-privacy = Ce que baylee.app et Baylee Central traitent à votre sujet, pourquoi, sur quelle base légale et pendant combien de temps.
title-terms = Conditions d’utilisation
desc-terms = Les conditions d’utilisation de Baylee Central, le service de comptes derrière baylee.app.
title-fan-content = Contenu de fan non officiel
desc-fan-content = Baylee est un projet de fan relevant de la Politique des contenus de fans de Wizards of the Coast : ce que cela signifie et ce que cela ne signifie pas.
title-not-found = Page introuvable
title-gone = Cette page n’existe plus
crumbs-label = Vous êtes ici
breadcrumb-home = Accueil

## Home

hero-title = <nb>Construisez vos decks.</nb> <nb>Réunissez vos amis.</nb> <glow>Jouez, tout
    simplement.</glow>
hero-lead = Un éditeur de decks complet, des parties jusqu’à huit joueurs en duel, en chacun pour soi ou par équipes, et un moteur de règles qui vérifie chaque étape : chaque tour reste équitable sans que personne ait à chercher quoi que ce soit.
hero-offline = Les parties en solo et l’éditeur de decks fonctionnent hors ligne, sans compte. Les tables en ligne vivent sur des gateways ; <host>vous pouvez faire tourner la vôtre</host>.
hero-account = Vous jouez en ligne ? <register>Créez un compte</register>, gratuit et ouvert à tous pendant la Beta ouverte, ou <signin>connectez-vous</signin>.
hero-memorial = Souvenir de Baylee : ouvrir son album photo

# English for now: to be translated.
status-label = Baylee in numbers

cards-eyebrow = Les cartes
cards-title = Combien de cartes Baylee connaît
cards-lead = Baylee implémente les cartes de Magic une à une, chacune vérifiée d’après son texte imprimé. Voici les chiffres de la dernière version.
cards-implemented = Entièrement implémentées
cards-implemented-help = Toutes les règles de la carte fonctionnent et sont testées.
cards-generated = Générées
cards-generated-help = Implémentées par le générateur de Baylee à partir du texte de règles de la carte, mais pas encore testées à la main. Elles sont jouables ; dans de rares cas limites, l’une peut se comporter autrement.
cards-partial = Partiellement implémentées
cards-partial-help = La carte est jouable, mais une de ses capacités ou un cas limite de ses règles manque encore.
cards-total = Cartes au total
cards-total-help = Toutes les cartes de Magic, telles que Scryfall les répertorie ; les trois nombres précédents sont ceux que Baylee peut jouer.
cards-help = Ce que cela signifie
cards-share = { $playable } cartes sur { $total } sont jouables aujourd’hui.
cards-as-of = Version { $tag }, comptée le { $date }.
cards-missing = Les nombres de cartes n’ont pas pu être lus à l’instant.

shot-duel-alt = Un duel contre l’IA maison : six créatures attaquent, trois sont bloquées
shot-duel = <b>Duel tactique</b> contre l’IA maison, avec des lignes de combat claires pour chaque assignation.
shot-log-alt = Un duel avec le journal de partie ouvert, trois sorts sur la pile
shot-log = <b>Le journal de partie</b> consigne chaque étape comme un chat ; le nom d’une carte en ouvre l’aperçu.
shot-ring-alt = Une table à quatre places disposées en anneau
shot-ring = <b>Des tables jusqu’à huit places</b> s’organisent autour du champ de bataille.
shot-lobby-alt = Le salon avec le message d’un ami et son invitation à une table
shot-lobby = <b>Le salon :</b> tables ouvertes, amis, invitations et un chat chiffré de bout en bout.
shot-room-alt = Une salle où trois chaises vides reviennent à l’IA maison
shot-room = <b>Une place pour chacun :</b> des personnes, l’IA maison à cinq niveaux, ou un modèle de langage.
shot-builder-alt = L’éditeur de decks filtrant la réserve pour un ange
shot-builder = <b>L’éditeur de decks</b> fouille toute la réserve pendant que vous tapez.

about-eyebrow = Ce que c’est
about-title = Un moteur de règles complet et une table 3D claire
about-engine = Priorité, pile, couches, effets de remplacement, capacités déclenchées, actions basées sur l’état et combat : le moteur en Rust applique l’ensemble complet des règles et calcule les choix légaux exacts à chaque étape. Le client ne présente que ces actions valides.
about-table = Champ de bataille, mains, pile, exil et cimetières sur une table 3D épurée, avec des lignes de combat claires et des aperçus de cartes instantanés. Les illustrations des cartes arrivent de Scryfall pendant la partie, ou passent à tout moment à une vue nette du texte Oracle.

gw-eyebrow = En ligne
gw-title = Gateways

open-central = Baylee Central sur GitHub

## Download

dl-prerelease = Beta
dl-for = Télécharger pour { $system }
dl-size = { $size } Mo
dl-checksum = SHA-256
dl-signature = Signature
dl-notes = Notes de version
dl-unknown = Ce navigateur ne nous a pas permis de reconnaître votre système ; choisissez le vôtre ci-dessous.
dl-mobile = Il n’existe pas encore de version pour téléphones et tablettes. Baylee fonctionne sur un ordinateur.
dl-mac-unsure = Ce navigateur ne dit pas si votre Mac a une puce Apple ou Intel. Le fichier ci-dessus est pour les puces Apple (M1 et suivantes) ; le fichier pour les Mac Intel se trouve plus bas.
dl-windows-unsure = Sur un PC Windows à processeur ARM (un portable Snapdragon, par exemple), prenez le fichier ARM ci-dessous.
dl-linux-unsure = Votre navigateur n’indique pas votre processeur ; x86-64 est supposé. Le fichier ARM se trouve plus bas.
dl-missing = Cette version n’a pas de fichier pour votre système. Les versions plus anciennes se trouvent sur la <releases>page des versions</releases>.
dl-none = GitHub n’a pas répondu à l’instant. Toutes les versions se trouvent sur la <releases>page des versions</releases>.
dl-yours = Votre système
dl-more-files = Autres fichiers ({ $count })
dl-installer = Installateur
dl-deb = Paquet Debian
dl-archive = Archive
dl-fallback = Archive pour anciens processeurs
dl-fallback-help = Pour les processeurs x86-64 sans AVX2 (d’avant 2013 environ). Les versions principales de Baylee ont besoin d’AVX2 et ne démarrent pas sans.
plat-windows-x64 = Windows
plat-windows-x64-sub = la plupart des PC : Intel ou AMD 64 bits
plat-windows-arm64 = Windows sur ARM
plat-windows-arm64-sub = portables Snapdragon et similaires
plat-mac-arm64 = macOS
plat-mac-arm64-sub = puces Apple : M1 et suivantes
plat-mac-x64 = macOS sur Intel
plat-mac-x64-sub = Mac avec processeur Intel
plat-linux-x64 = Linux
plat-linux-x64-sub = x86-64
plat-linux-arm64 = Linux sur ARM
plat-linux-arm64-sub = ARM64 / aarch64
plat-none = Pas de fichier pour ce système dans cette version.
install-title = Premier lancement et prérequis
install-first-title = Premier lancement
install-first = Les versions ne sont pas signées par un certificat de signature de code, car signer coûte de l’argent ; Windows et macOS vous avertissent donc la première fois. Sous Windows, choisissez <i>Informations complémentaires</i> puis <i>Exécuter quand même</i>. Sous macOS, ouvrez <i>Réglages Système → Confidentialité et sécurité</i> et choisissez <i>Ouvrir quand même</i>.
install-check-title = Vérifier un téléchargement
install-check = À côté de chaque fichier se trouve un <code>.sha256</code> avec sa somme de contrôle ; les archives portent aussi un <code>.sig</code>, la signature Ed25519 que le client vérifie avant de se mettre à jour. Sous Linux, <code>sha256sum -c</code> vérifie un fichier ; sous macOS <code>shasum -a 256</code> ; sous Windows <code>certutil -hashfile</code>.
install-needs-title = Prérequis
install-windows = <b>Windows</b> 10 ou 11, x64 ou ARM64.
install-mac = <b>macOS</b> 11 ou plus récent, sur puce Apple ou Intel.
install-linux = <b>Linux</b> avec glibc 2.35 ou plus récente (Ubuntu 22.04 et suivantes) et un pilote Vulkan ; le <code>.deb</code> installe tout le reste dont il a besoin.
install-cpu = <b>Processeur :</b> les versions x86-64 ont besoin d’AVX2 ; pour les processeurs plus anciens, il existe une archive séparée.

## Gateways

gws-title = Gateways
gws-online = En ligne
gws-offline = Hors ligne
gws-version = Version
gws-now = En ce moment
gws-website = Site web
gws-address = Adresse
gws-copy = Copier l’adresse
gws-copied = Adresse copiée.
gws-copied-short = Copiée
gws-copy-failed = La copie n’a pas fonctionné.
gws-failed = L’annuaire n’a pas pu être lu à l’instant. Réessayez dans une minute.

## Questions

faq-eyebrow = Questions
faq-title = Questions et réponses
faq-what-q = Qu’est-ce que Baylee ?
faq-what-a = Un jeu de fan gratuit pour Magic: The Gathering sur votre ordinateur. Il comprend un éditeur de decks, un moteur de règles qui vérifie chaque étape, une table jusqu’à huit joueurs et des adversaires contrôlés par l’ordinateur. Il n’est ni créé, ni approuvé, ni promu par Wizards of the Coast.
faq-cost-q = Combien ça coûte ?
faq-cost-a = Rien, et cela ne changera pas : pas de paywall, pas de fonctions payantes, pas de publicité.
faq-account-q = Ai-je besoin d’un compte ?
faq-account-a = Pas pour jouer seul : l’éditeur de decks et les parties contre l’IA maison fonctionnent hors ligne. Pour les tables en ligne, il faut un compte Baylee ; pendant la Beta ouverte, chacun peut en créer un sur ce site, gratuitement.
faq-data-q = Qu’advient-il de mes données ?
faq-data-a = Le moins possible ; la <privacy>politique de confidentialité</privacy> dit exactement quoi. Votre mot de passe ne quitte jamais votre navigateur ; vos données privées et vos messages de chat sont chiffrés sur votre appareil avant de nous parvenir.
faq-password-q = J’ai oublié mon mot de passe. Que faire ?
faq-password-a = Vous définissez un nouveau mot de passe avec un code de réinitialisation, <reset>sur cette page</reset>. Si votre compte a une adresse e-mail confirmée, vous pouvez y faire envoyer le code depuis cette page ; sinon, un administrateur vous en remet un. Vos données chiffrées ne survivent que si vous avez le code de récupération reçu à la création de votre compte. Sans lui, vous repartez avec un coffre vide, et ces données sont perdues.
faq-gateway-q = Qu’est-ce qu’une gateway ?
faq-gateway-a = Un serveur où vivent des tables en ligne. N’importe qui peut en faire tourner une ; l’<gateways>annuaire</gateways> liste celles que Central connaît.
faq-systems-q = Sur quels systèmes fonctionne-t-il ?
faq-systems-a = Windows, macOS et Linux, sur x86-64 et ARM64. Il n’existe pas encore de version pour téléphones ou tablettes.
faq-open-q = Est-ce open source ?
faq-open-a = Le jeu l’est, sous la GNU Affero General Public License. Le code de Baylee Central est public pour que chacun puisse vérifier comment il traite ses données.
faq-bug-q = J’ai trouvé un bug. Où le signaler ?
faq-bug-a = Dans le jeu, appuyez sur <kbd>F8</kbd> pour envoyer un rapport avec tout le nécessaire. Sinon, sur le suivi de tickets de GitHub, sur Discord, ou via le <contact>formulaire de contact</contact>.

## Contact

contact-eyebrow = Contact
contact-title = Écrivez-nous
contact-name = Votre nom
contact-name-hint = Facultatif.
contact-email = Votre adresse e-mail
contact-email-hint = Facultatif ; uniquement pour pouvoir vous répondre.
contact-message = Votre message
contact-count = { $n } caractères sur 5 000
contact-send = Envoyer
contact-sending = Envoi …
contact-sent-title = Merci
contact-sent = Votre message nous est parvenu. Si vous avez indiqué une adresse, nous y répondrons.
contact-error-empty = Écrivez d’abord un message.
contact-error-email = Cela ne ressemble pas à une adresse e-mail.
contact-error-busy = Trop de messages depuis votre réseau à l’instant. Réessayez dans une heure.
contact-error-refused = Le message n’a pas été accepté. Vérifiez sa longueur et réessayez.
contact-error-network = Le message n’a pas pu être envoyé. Vérifiez votre connexion et réessayez.
contact-privacy = Ce que vous envoyez est conservé pour que nous puissions répondre, et supprimé 90 jours après que nous l’avons traité, ou au plus tard après un an. Plus de détails dans la <privacy>politique de confidentialité</privacy>.
contact-noscript = Le formulaire nécessite JavaScript. Vous pouvez aussi nous joindre sur Discord ou sur GitHub.

## Legal notice

impressum-title = Mentions légales
impressum-lead = Informations requises par le § 5 DDG (la loi allemande sur les services numériques). Baylee est un projet privé et non commercial.
impressum-show = Afficher les coordonnées
impressum-why = Les coordonnées sont derrière une courte énigme que votre navigateur résout en quelques secondes, pour tenir à l’écart les robots qui collectent les adresses. Rien d’autre n’est demandé de votre part.
impressum-solving = Résolution de l’énigme … { $percent } %
impressum-name = Nom
impressum-address = Adresse
impressum-email = E-mail
impressum-phone = Téléphone
impressum-write = Écrire un e-mail
impressum-call = Appeler
impressum-failed = Les coordonnées n’ont pas pu être chargées. Réessayez, ou écrivez-nous via le <contact>formulaire de contact</contact>.
impressum-noscript = L’affichage des coordonnées nécessite JavaScript. Sans lui, joignez-nous via Discord ou GitHub.
impressum-responsible = Responsable du contenu : la personne nommée ci-dessus.
impressum-dispute = Nous ne sommes ni tenus ni disposés à participer à une procédure de règlement des litiges devant un organisme de conciliation des consommateurs.

## Fan content

fc-title = Contenu de fan non officiel
fc-eyebrow = Contenu de fan
fc-lead = Baylee est un projet de fan. Il est autorisé dans le cadre de la Politique des contenus de fans de Wizards of the Coast, qui fixe les règles qu’il suit.
fc-notice-title = La mention sur chaque page
fc-means-title = Ce que cela signifie
fc-means = Baylee n’est ni créé, ni approuvé, ni promu par Wizards of the Coast. Magic: The Gathering, ses noms de cartes, noms d’extensions, images de cartes et symboles de mana sont la propriété de Wizards of the Coast LLC. Baylee est gratuit pour tous : il n’y a ni paywall, ni fonction payante, ni inscription par e-mail.
fc-images = Le jeu affiche des images de cartes provenant de Scryfall, non modifiées et avec mention de l’artiste ; ce site n’en affiche aucune. Les données et images de cartes sont fournies par Scryfall ; Baylee n’est pas affilié à Scryfall.
fc-policy = La <policy>Politique des contenus de fans</policy> de Wizards of the Coast

## Privacy notice

privacy-title = Politique de confidentialité
privacy-lead = Ce que ce site et Baylee Central traitent à votre sujet, pourquoi, sur quelle base légale et pendant combien de temps.
privacy-updated = En date du 12 novembre 2026
privacy-controller-title = Qui est responsable
privacy-controller = Le responsable du traitement au sens du RGPD (règlement général sur la protection des données) est la personne physique nommée dans les <impressum>mentions légales</impressum>, qui indiquent son nom, son adresse postale et son adresse e-mail. Vous pouvez aussi envoyer vos questions sur la protection des données via le formulaire de contact. Il n’y a pas de délégué à la protection des données ; il n’est pas requis.
privacy-short-title = En bref
privacy-short = Pas de suivi, pas de publicité, pas d’analyse d’audience, pas de cookie tant que vous ne vous connectez pas, et rien de chargé depuis d’autres serveurs : polices, images et scripts viennent tous de baylee.app. Votre mot de passe ne nous parvient jamais, et vos messages de chat et vos données privées sont chiffrés dans votre navigateur avant d’être envoyés.
privacy-hosting-title = Visite du site
privacy-hosting = Le site tourne sur un serveur que nous louons à netcup GmbH en Allemagne. Quand vous ouvrez une page, votre navigateur envoie ce que porte toute requête : votre adresse IP, l’adresse demandée, l’heure, la langue de votre navigateur et son identification (user agent). Le serveur en a besoin pour répondre ; il choisit la version linguistique d’après la langue, et la page de téléchargement utilise le user agent pour suggérer le bon fichier. Le serveur web ne tient aucun journal d’accès : ces requêtes ne sont donc pas enregistrées. Les messages de fonctionnement propres au serveur (son journal système) peuvent, dans de rares cas d’erreur, contenir une adresse IP ; ils sont supprimés au plus tard après 14 jours. Base légale : art. 6, par. 1, point f) du RGPD ; notre intérêt légitime est de fournir le site de façon sûre et fiable.
privacy-views-title = Le comptage des pages vues
privacy-views = Le site compte combien de fois ses pages sont ouvertes : un seul nombre pour toutes les pages et tous les visiteurs ensemble, affiché sur la page d’accueil. Une page ouverte dans un navigateur y ajoute un. Rien sur vous n’est enregistré pour cela : ni adresse IP, ni cookie, ni identifiant, pas même la page dont il s’agissait.
privacy-device-title = Ce que conserve votre navigateur
privacy-device = Ce site ne dépose aucun cookie tant que vous ne vous connectez pas. Ce que votre navigateur conserve sur votre appareil, et seulement avec JavaScript : votre choix de couleurs si vous en avez fait un (<code>baylee-site-theme</code>) ; pendant que vous êtes connecté, la fin de votre session (<code>baylee-central-session</code>), votre clé de chat et les clés de confiance de vos contacts (voir Chat) ; si vous avez activé les notifications, un nombre aléatoire pour cet appareil (<code>baylee-push-installation-v1</code>, <code>baylee-push</code>) ; et, uniquement une fois que vous vous êtes connecté, que vous avez installé le site comme application ou activé les notifications, une copie des fichiers publics du site (structure des pages, polices, scripts), pour qu’il démarre sans connexion et que les notifications puissent arriver. Si aucun de ces trois cas ne s’applique plus, la copie est supprimée à la prochaine ouverture du site. Rien de tout cela ne nous est envoyé, sauf si une section ci-dessous le dit. Les entrées du chat et de la session sont supprimées quand vous vous déconnectez ; vous supprimez tout le reste en effaçant les données du site dans votre navigateur. Base légale : § 25, par. 2, n° 2 TDDDG (strictement nécessaire à la fonction que vous utilisez) ; pour le nombre aléatoire des notifications, votre consentement (§ 25, par. 1 TDDDG).
privacy-github-title = GitHub
privacy-github = La page de téléchargement et la page d’accueil affichent la dernière version, l’état des tests et les nombres de cartes. Notre serveur les demande à GitHub, pas votre navigateur ; GitHub n’apprend rien sur vous par ce biais. Ce n’est que lorsque vous cliquez sur un lien de téléchargement ou de GitHub que votre navigateur se connecte à GitHub (GitHub, Inc., États-Unis) et lui envoie votre adresse IP et ce que porte toute requête ; la politique de confidentialité propre à GitHub s’applique alors.
privacy-directory-title = L’annuaire des gateways
privacy-directory = Notre serveur lit l’annuaire des gateways auprès de Baylee Central et l’affiche ; rien qui vous concerne n’y intervient. Une gateway peut indiquer à Central combien de joueurs sont connectés en ce moment, combien d’entre eux sont dans une partie, et combien de tables ouvertes et de parties elle compte, et Central compte ses comptes ; le site affiche ces nombres. Ce ne sont que des nombres, jamais des noms, et les chiffres d’une gateway sont la déclaration de son propre opérateur.
privacy-account-title = Votre compte
privacy-account = Pour un compte, Baylee Central enregistre : l’identifiant de votre compte, votre nom de connexion, votre nom d’affichage avec son tag, la date et le mode de création du compte, un enregistrement de mot de passe OPAQUE (qui vous permet de prouver votre mot de passe sans que notre serveur ne l’apprenne jamais), votre clé maîtresse chiffrée deux fois (par votre mot de passe et par votre code de récupération), votre document de compte chiffré et vos objets chiffrés (illisibles pour nous ; nous ne voyons que la taille, la révision et l’heure), la version des conditions que vous avez acceptée et la date, l’heure de votre dernière connexion, vos sessions, un rôle d’administrateur si vous en avez un, et les gateways que vous enregistrez avec leurs jetons. Les autres joueurs voient votre nom d’affichage et votre tag, jamais votre nom de connexion. Si un administrateur vous remet un code de réinitialisation, nous n’en conservons que l’empreinte jusqu’à ce que le code soit utilisé ou ait expiré. Ce que les administrateurs font sur un compte ou une gateway est consigné dans un registre pendant trois ans (qui, quoi, quel compte ou quelle gateway, quand). Base légale : art. 6, par. 1, point b) du RGPD (le compte que vous avez demandé) ; pour l’heure de la dernière connexion et le registre, art. 6, par. 1, point f) du RGPD (notre intérêt légitime à un fonctionnement sûr et traçable). Sans nom de connexion et sans mot de passe, il n’y a pas de compte ; vous n’avez besoin d’aucune adresse e-mail. Vous pouvez créer un compte à partir de 16 ans. Conservé jusqu’à la suppression du compte ; il est alors retiré immédiatement du système en fonctionnement, et des sauvegardes au plus tard dans les 12 mois. Les sessions prennent fin après 30 minutes sans utilisation et au plus tard 12 heures après la connexion.
privacy-email-title = Adresse e-mail
privacy-email = Une adresse e-mail est facultative ; un compte n’en exige aucune. Si vous en indiquez une, Baylee Central enregistre l’adresse, la langue dans laquelle ses e-mails sont rédigés et la date à laquelle vous l’avez confirmée. Une adresse que vous avez indiquée et qui attend son code est conservée jusqu’à ce que vous la confirmiez ou la retiriez, et au plus un jour après l’expiration de son code. De chaque code que nous envoyons par e-mail, nous n’enregistrons que l’empreinte, jusqu’à ce que le code soit utilisé ou ait expiré, et pendant 24 heures les moments où des codes de réinitialisation vous ont été envoyés, afin que trois au plus partent par jour. Nous utilisons l’adresse pour trois choses : envoyer le code qui la confirme, envoyer un code de réinitialisation lorsque vous ou toute personne qui connaît votre nom de connexion en demandez un, et prévenir l’ancienne adresse lorsqu’elle est remplacée ou retirée. L’e-mail est envoyé par le service de messagerie propre à Baylee, sur le même serveur, via mailbox.org (Heinlein Hosting GmbH, Berlin), qui agit en tant que sous-traitant et reçoit l’adresse et l’e-mail. Base légale : art. 6, par. 1, point b) du RGPD (le compte que vous avez demandé). Conservée jusqu’à ce que vous retiriez l’adresse ou supprimiez le compte.
privacy-cookie-title = Le cookie de session
privacy-cookie = La connexion dépose un seul cookie, <code>__Host-central</code> : un nombre aléatoire qui identifie votre session. Il n’est envoyé qu’à baylee.app, ne peut pas être lu par des scripts et prend fin avec la session, après 12 heures au plus tard. Il est strictement nécessaire pour vous garder connecté ; il ne requiert aucun consentement. Base légale : § 25, par. 2, n° 2 TDDDG et art. 6, par. 1, point b) du RGPD.
privacy-chat-title = Chat et contacts
privacy-chat = Les messages sont chiffrés sur votre appareil pour les appareils de chaque destinataire ; Baylee Central ne les conserve que sous forme chiffrée et ne peut pas les lire. Ce que Central peut voir et enregistre : vos contacts, demandes de contact et blocages, les groupes avec leurs noms, membres et invitations, qui a envoyé un message à qui, quand et de quelle taille, et les clés publiques de vos appareils. Vos contacts et les membres de vos groupes voient votre nom d’affichage avec son tag et vos messages. Votre navigateur conserve votre clé de chat (scellée sous une clé que seul ce navigateur détient, dans son stockage <code>baylee-chat</code>) et les clés de vos contacts auxquelles vous avez fait confiance (chiffrées, <code>baylee-chat-trust</code>) pendant que vous êtes connecté. Les messages et les remises sont conservés jusqu’à la suppression de votre compte ; il n’existe pas encore de durée de conservation automatique plus courte. Chaque expéditeur et chaque appareil détient au plus 5 000 messages ; au-delà, les nouveaux sont refusés. Base légale : art. 6, par. 1, point b) du RGPD.
privacy-push-title = Notifications
privacy-push = Si vous activez les notifications dans un navigateur, votre navigateur s’enregistre auprès de son service push, et Central enregistre l’adresse de ce service pour ce navigateur, la clé publique et le secret d’authentification du navigateur, votre langue, un nombre aléatoire pour cet appareil et votre choix de catégories. Le service push appartient au fabricant de votre navigateur : Google (Firebase Cloud Messaging), Mozilla ou Apple, tous établis aux États-Unis. Une notification ne porte, chiffrés, que son genre (par exemple « nouveau message ») et votre langue, jamais de noms ni de texte ; le service push voit des données de connexion telles que l’adresse, l’heure et la taille. Ces données sont supprimées quand vous désactivez les notifications sur cet appareil, quand vous vous déconnectez ou quand vous supprimez votre compte ; une adresse non renouvelée pendant 90 jours n’est plus utilisée, et les notifications en attente expirent après un jour. Base légale : art. 6, par. 1, point a) du RGPD et § 25, par. 1 TDDDG (votre consentement). Vous pouvez le retirer à tout moment dans les réglages de votre compte ; ce qui s’est passé jusque-là reste licite.
privacy-counters-title = Protection contre les abus
privacy-counters = Pour empêcher qu’on devine les mots de passe et pour éviter la surcharge, Central compte les requêtes par adresse IP (inscriptions, tentatives de connexion, réinitialisations de mot de passe, lecture de l’annuaire et, plus généralement, le nombre de requêtes qu’une adresse envoie en une minute), uniquement en mémoire : les inscriptions et les tentatives de connexion qui n’ont pas abouti avec le bon mot de passe, jusqu’à un jour, tout le reste une heure au plus. Après une connexion avec le bon mot de passe, Central retient pendant 30 jours, là aussi uniquement en mémoire, que votre compte s’est connecté depuis cette adresse (une adresse IPv4, ou la première moitié d’une adresse IPv6 ; huit au plus par compte), afin que les tentatives d’autres personnes sur votre compte ne vous en excluent pas depuis cette adresse. Quelqu’un qui utilise la même adresse et connaît votre nom de connexion peut déduire des réponses de Central que votre compte s’est connecté depuis celle-ci. Un nouveau mot de passe oublie ces adresses, sauf celle depuis laquelle vous l’avez défini ; la suppression du compte les oublie toutes, tout comme un redémarrage du service. Avant une inscription, votre navigateur résout en outre une énigme qui demande de la mémoire et du temps de calcul ; elle est faite de nombres aléatoires, ne contient rien sur vous et est oubliée au bout de vingt minutes au plus tard. Central en recalcule une petite partie pour vérifier la réponse de votre navigateur. Sa difficulté dépend du nombre de comptes en cours de création et de l’apparence de la requête, selon qu’elle vient d’un programme : si elle porte les en-têtes que tout navigateur envoie, si votre navigateur indique qu’un programme le pilote, et combien d’énigmes votre adresse et le bloc réseau qui l’entoure (les trois premiers nombres d’une adresse IPv4, les 48 premiers bits d’une adresse IPv6) ont demandées durant la dernière heure. De cela, Central ne conserve que les décomptes, en mémoire, une heure au plus. Par compte, Central compte les demandes de contact et les recherches d’autres joueurs, afin que personne n’inonde les autres de demandes. Base légale : art. 6, par. 1, point f) du RGPD ; notre intérêt légitime est de protéger les comptes et le service contre les abus.
privacy-contact-title = Le formulaire de contact
privacy-contact = Le formulaire envoie votre message, la langue de la page et l’heure au service de retours de Baylee sur le même serveur, et votre nom et votre adresse e-mail uniquement si vous les indiquez ; les deux sont facultatifs. Le service note aussi si le message a été traité. Votre adresse IP n’est pas enregistrée ; elle est seulement comptée en mémoire pendant une heure, pour empêcher une avalanche de messages. Seule la personne nommée dans les mentions légales lit les messages ; rien n’est transmis. Un message est supprimé 90 jours après son traitement, après un an au plus tard, ou plus tôt si vous le demandez. Base légale : art. 6, par. 1, point f) du RGPD (notre intérêt légitime à répondre aux messages), ou point b) lorsque votre message concerne votre compte.
privacy-impressum-title = L’énigme des mentions légales
privacy-impressum = Les mentions légales ne s’affichent qu’une fois que votre navigateur a résolu une énigme ; cela
    protège l’adresse des programmes qui collectent les adresses. L’énigme se compose de nombres
    aléatoires et ne contient rien sur vous. Pour cela, le service de retours conserve uniquement en
    mémoire, jamais sur disque ni dans un journal : combien d’énigmes votre adresse IP a demandées
    (pendant une heure ; une adresse IPv6 par sa première moitié), si elle a suivi un lien caché aux
    personnes (pendant 24 heures), et quelles énigmes ont été résolues (pendant cinq minutes, sans
    adresse). Trois réponses par oui ou par non sur votre navigateur fixent la difficulté de
    l’énigme et ne sont pas conservées : si un programme le pilote, s’il n’indique aucune langue, et
    s’il dessine sans carte graphique. Personne n’est refusé ; un navigateur qui ressemble à un
    programme reçoit une énigme plus longue. Base légale : art. 6, par. 1, point f) du RGPD (notre
    intérêt légitime à protéger l’adresse d’une personne privée).
privacy-recipients-title = Destinataires et pays
privacy-recipients = Nous ne vendons aucune donnée et n’en transmettons aucune à des fins publicitaires. Les destinataires sont : netcup GmbH (hébergement en Allemagne, en tant que sous-traitant au titre de l’art. 28 du RGPD) ; mailbox.org (Heinlein Hosting GmbH, Berlin), qui distribue les e-mails à une adresse e-mail que vous avez indiquée, en tant que sous-traitant ; les administrateurs de Baylee Central, qui voient, pour l’administration, votre nom de connexion, votre nom d’affichage avec son tag, l’origine, la création et la dernière connexion de votre compte, et vos gateways, jamais vos données chiffrées ; une gateway à laquelle vous vous connectez avec votre compte (elle reçoit l’identifiant de votre compte, votre nom d’affichage et votre tag, pas votre mot de passe) ; vos contacts et les membres de vos groupes (nom d’affichage, tag et vos messages) ; les autorités uniquement lorsque la loi nous y oblige. Pays hors de l’Espace économique européen : uniquement lorsque vous activez les notifications, le service push de votre navigateur (Google, Mozilla ou Apple, États-Unis) les achemine. Les gateways exploitées par d’autres peuvent se trouver dans n’importe quel pays ; vous décidez si vous vous y connectez. Pour le reste, nous ne transférons aucune donnée vers des pays hors de l’Espace économique européen.
privacy-rights-title = Vos droits
privacy-rights = Vous avez le droit d’accès (art. 15 du RGPD), de rectification (art. 16), d’effacement (art.
    17), de limitation du traitement (art. 18) et à la portabilité des données (art. 20). Vous
    pouvez vous opposer à tout moment, pour des raisons tenant à votre situation particulière, au
    traitement fondé sur nos intérêts légitimes au titre de l’art. 6, par. 1, point f) (art. 21).
    Vous pouvez retirer un consentement à tout moment pour l’avenir (art. 7, par. 3). Les données de
    votre compte peuvent être exportées et le compte supprimé dans les paramètres de votre compte ;
    pour tout le reste, un message via le formulaire de contact ou à l’adresse indiquée dans les
    mentions légales suffit. Vous pouvez introduire une réclamation auprès d’une autorité de
    contrôle de la protection des données (art. 77), par exemple celle du lieu où vous vivez ou le
    Commissaire d’État à la protection des données et à la liberté d’information de
    Rhénanie-du-Nord-Westphalie. Aucune loi ni aucun contrat ne vous oblige à nous donner des
    données ; sans nom de connexion et sans mot de passe, il n’y a toutefois pas de compte. Il n’y a
    ni décision automatisée ni profilage (art. 22).
privacy-objection-title = Votre droit d’opposition
privacy-objection = Lorsque nous traitons des données sur le fondement de l’art. 6, par. 1, point f) du RGPD (fourniture du site, protection contre les abus, formulaire de contact, énigme des mentions légales, heure de votre dernière connexion, registre d’administration, rapports de bugs et comptage d’utilisation), vous pouvez vous opposer à tout moment, pour des raisons tenant à votre situation particulière (art. 21 du RGPD). Nous cessons alors le traitement des données, sauf si nous pouvons démontrer des motifs légitimes impérieux qui prévalent sur vos intérêts, ou si le traitement sert à la constatation, à l’exercice ou à la défense de droits en justice. Un simple message via le formulaire de contact ou à l’adresse indiquée dans les mentions légales suffit.
privacy-links-title = Liens vers d’autres services
privacy-links = Le site renvoie notamment vers GitHub, Discord et WhatsApp. Ce sont de simples liens : rien n’est intégré, et tant que vous n’en cliquez aucun, aucun de ces services ne connaît votre visite. Quand vous cliquez, votre navigateur n’indique pas de quelle page vous venez (pas de référent). Ensuite, la politique de confidentialité propre à chaque fournisseur s’applique, et ils peuvent traiter des données hors de l’UE ; ce que les autres membres des groupes de la communauté voient de vous là-bas est décidé par ces services.
privacy-reports-title = Rapports de bugs et comptage d’utilisation depuis le jeu
privacy-reports = Un rapport de bug que vous envoyez depuis le jeu avec F8 contient son genre, votre texte, la version du client et les pièces jointes que vous laissez cochées ; la page de contact les énumère. Il est supprimé 90 jours après son traitement, au plus tard après trois ans. Base légale : art. 6, par. 1, point f) du RGPD (notre intérêt légitime à un jeu sans bugs). Le client n’envoie de rapports de plantage que si vous l’avez autorisé (art. 6, par. 1, point a) du RGPD ; vous pouvez le retirer à tout moment dans les réglages du client). Pendant une partie hors ligne, le client envoie une fois par minute un identifiant aléatoire de cette partie et rien d’autre ; nous ne le conservons qu’en mémoire et l’oublions 150 secondes après son dernier signe de vie (art. 6, par. 1, point f) du RGPD ; peut être désactivé sous « Send anonymous usage count »). Comme pour toute connexion, votre adresse IP nous parvient.

## Terms

terms-title = Conditions d’utilisation
terms-draft = Ces conditions sont un projet et ne sont pas encore en vigueur.
terms-version = Version { $version }, mise à jour le { $date }.
# Shown only in the languages a machine translated: every one but German and English.
machine-note = Ce site a été traduit par une machine, et aucun locuteur de votre langue ne l’a encore relu. Le jeu lui-même parle pour l’instant anglais et allemand. <link>Signalez-nous une erreur.</link>
legal-machine = Cette page est une traduction automatique, proposée comme aide. En cas de différence avec le texte <de>allemand</de> ou <en>anglais</en>, ceux-ci prévalent.
terms-machine = Les conditions de Central n’existent qu’en allemand et en anglais. Voici le texte anglais ; il existe aussi une <de>version allemande</de>.

## Errors

not-found-title = Cette page n’existe pas
not-found-lead = L’adresse est peut-être mal saisie, ou la page a été déplacée.
gone-title = Cette page a disparu pour de bon
gone-lead = Ce qui se trouvait ici a été supprimé. La page d’accueil montre où aller ensuite.
error-home = Vers la page d’accueil

## Baylee's remembrance (approved by her family on 27 October 2026; keep word for word)
baylee-dedication = Pour Baylee. À jamais dans la lumière du soleil.
baylee-invitation = Faites connaissance avec la petite âme derrière ce monde
baylee-open = Souvenir de Baylee · ouvrir son album photo
baylee-eyebrow = Le cœur de ce projet
baylee-date = En souvenir plein d’amour · 26 octobre 2026
baylee-close = Retour à son monde
baylee-previous = Photo précédente
baylee-next = Photo suivante
baylee-photo1 = Baylee blottie dans la douce couette blanche.
baylee-photo2 = Baylee regardant dehors depuis la couette à fleurs.
baylee-photo3 = Au repos dans son panier rond et gris.
baylee-photo4 = Baylee endormie, tout contre nous.
baylee-photo5 = Un moment paisible, nichée dans la couette.

baylee-storyTitle = Pour Baylee. À jamais dans la lumière du soleil.
baylee-together = Avec nous · 17 novembre 2025 – 26 octobre 2026
baylee-chapters = Son histoire
baylee-chapterPrevious = Chapitre précédent
baylee-chapterNext = Chapitre suivant
baylee-photos = Nos souvenirs
baylee-read = Faites défiler pour lire

baylee-chapter1Title = Un foyer pour Baylee
baylee-chapter1Body =
    Baylee est arrivée chez nous le 17 novembre 2025.

    Ma femme et moi l’avions adoptée au refuge pour animaux de Krefeld. On lui estimait environ treize ans. Elle avait des tumeurs mammaires malignes, et le cancer s’était déjà propagé au foie. Baylee était une chatte en soins palliatifs. Nous savions que notre temps ensemble serait limité. Chez nous, elle devait avoir un foyer où elle serait aimée.

    Elle nous a vite montré à quel point elle allait s’approprier ce foyer.

    Dès son premier jour, elle s’est glissée sur mes genoux pendant que je travaillais. Et dès sa première nuit, elle a déchiré un sac de croquettes que j’avais commandé pour les chats de mes parents et s’est servie. Bien sûr, elle avait eu sa propre nourriture et bien mangé ce soir-là. Mais Baylee avait visiblement décidé que cette réserve méritait quand même un examen plus approfondi.

    Notre petite souris avait un appétit étonnant – et une énergie incroyable.

baylee-chapter2Title = La confiance grandit
baylee-chapter2Body =
    Quand nous avons remarqué qu’elle buvait beaucoup, nous l’avons emmenée chez le vétérinaire. Une prise de sang a révélé un problème de thyroïde, qui expliquait aussi sa grande faim et son tempérament vif. On lui a prescrit un traitement pour la thyroïde et une cure d’antibiotiques de courte durée parce que ses marqueurs d’inflammation étaient élevés.

    Quand elle s’est soudain mise à se frotter contre tout, à miauler beaucoup et à devenir inhabituellement câline et attachée à nous, nous avons d’abord craint qu’elle ne réagisse mal à l’un de ses nouveaux médicaments. Un autre examen a donné l’explication : notre petite chérie était en chaleur. Elle n’avait pas été stérilisée.

    Nous avons aussi traversé cette période ensemble. Baylee nous faisait de plus en plus confiance, et nous nous rapprochions toujours davantage.

baylee-chapter3Title = Notre petite princesse
baylee-chapter3Body =
    Je me souviens encore exactement de sa joie quand elle a eu son arbre à chat. Elle s’est tout de suite installée sur la plateforme tout en haut et a regardé l’appartement d’un air content. Curieusement, elle n’a plus jamais utilisé cette place. Pourquoi, cela restera son secret.

    Les autres endroits restaient rarement libres : mes genoux, mes jambes et notre lit. Elle y dormait le plus souvent sur moi ou sur ma femme. Le mieux, c’était de réussir à s’allonger sur nous deux à la fois.

    Elle se pavanait dans l’appartement, la queue bien haute, comme si elle savait parfaitement qu’elle était la princesse des lieux. Elle avait du caractère et savait faire respecter ses limites. Chez nous, elle pouvait être tout à fait elle-même. Ses manies, ses exigences et son petit côté têtu étaient aimés eux aussi.

    Je n’avais jamais noué un lien aussi profond avec un animal. Baylee a trouvé une place très profonde dans le cœur de ma femme comme dans le mien.

baylee-chapter4Title = Des heures au soleil
baylee-chapter4Body =
    Quand le temps s’est réchauffé, elle a découvert le balcon. Elle adorait sentir les premiers rayons du jour dans sa fourrure. Bientôt, elle y passait presque toute la journée.

    Les pigeons et les oiseaux du jardin la fascinaient tout particulièrement. Elle les observait avec attention, les guettait et essayait sans cesse de les effrayer. Entre bains de soleil et observation des oiseaux, notre petite princesse avait un emploi du temps bien rempli.

baylee-chapter5Title = Une petite tête bien à elle
baylee-chapter5Body =
    Entre-temps, sa maladie continuait de progresser.

    La tumeur s’est enflammée, et Baylee n’arrêtait pas de lécher la zone touchée jusqu’à la rouvrir. Chez le vétérinaire, elle a reçu sa première combinaison de récupération, une rouge. Dès lors, elle a dû en porter une chaque jour pour l’empêcher de blesser davantage la plaie.

    Elle s’y est vite habituée. Se la faire enfiler, en revanche, était une chose qu’elle n’a pas aimée jusqu’à son dernier jour. Même une gamelle ne suffisait alors qu’à moitié à l’en faire sortir. Aussi grand que fût son appétit, son avis sur la combinaison était sans équivoque.

    La plaie s’est aggravée. Elle suppurait, a fini par saigner et a dégagé une mauvaise odeur. En même temps, Baylee continuait de nous montrer combien de joie elle trouvait encore dans son quotidien. Elle jouait, mangeait, explorait les environs et grimpait sur son griffoir et sur les placards.

    Parfois, elle restait des heures au bord du lit à côté de nous, à attendre que nous nous réveillions enfin et lui préparions son petit-déjeuner. Si cela prenait trop de temps, elle nous grimpait tout simplement dessus. Nos visages ne l’arrêtaient pas.

    Elle avait des idées très arrêtées sur la nourriture. Son goût allait plutôt du côté de la « pizza surgelée » : au bout d’un moment, la pâtée de qualité ne l’enthousiasmait plus, et la variété était indispensable. Deux fois de suite la même chose, il n’en était pas question. Poulet au foie, tout ce qui contenait du fromage, thon ou saumon – voilà ce qui la rendait heureuse.

baylee-chapter6Title = La queue bien haute
baylee-chapter6Body =
    La tumeur a grossi et suintait de plus en plus. Baylee souffrait visiblement. Quand la douleur s’atténuait, elle redevenait notre petite chatte, parcourant les pièces la queue bien haute et réclamant ses humains pour elle seule.

    Nous voyions ses forces décliner peu à peu. Mais son appétit, sa curiosité, sa tendresse et son ronronnement sonore étaient toujours là.

baylee-chapter7Title = Dire adieu
baylee-chapter7Body =
    Le vendredi soir 23 octobre 2026, elle a présenté des symptômes qui ressemblaient d’abord à un rhume. Son nez coulait, et sa respiration était profonde et pénible, comme si elle luttait pour avoir de l’air.

    Le samedi, elle nous a semblé de nouveau plus vive. La respiration inhabituelle et le nez qui coulait persistaient, mais elle montrait de nouveau l’énergie et l’amour de la vie que nous connaissions si bien.

    Le dimanche matin a été difficile. Ses yeux larmoyaient, elle avait du mal à respirer et elle souffrait. Je lui ai donné un antidouleur. Au bout d’une heure environ, elle allait de nouveau mieux, mais sa respiration restait différente. Nous savions qu’il fallait l’emmener chez le vétérinaire.

    Le lundi matin, son état s’est de nouveau aggravé. Heureusement, ma femme était en congé ce jour-là. Nous avons appelé le cabinet, pris rendez-vous et y avons emmené Baylee.

    La radiographie a montré jusqu’où le cancer s’était étendu. On nous a dit qu’il ne restait qu’environ un tiers de ses poumons pour respirer. La laisser continuer à vivre aurait signifié qu’elle aurait de moins en moins d’air.

    Nous avons dû lui dire adieu.

    Le lundi 26 octobre 2026, vers midi, notre très chère Baylee s’est endormie pour toujours.

baylee-chapter8Title = À jamais dans la lumière du soleil
baylee-chapter8Body =
    Elle est restée avec nous un peu moins d’un an. Pendant ce temps, elle était devenue membre de notre famille et une amie très proche. Elle faisait partie de nos journées de travail, de nos nuits et de nos réveils du matin. Des heures ensoleillées sur le balcon et des petites habitudes qui font une vie à deux.

    Maintenant, elle manque dans tous ces moments.

    Le vide est encore frais et profond. Son ronronnement nous manque, son poids sur nos jambes, sa queue dressée et son attente sans le moindre doute que nous nous occuperions enfin de son petit-déjeuner.

    S’il y a quelque chose après cette vie, quoi que ce puisse être, alors je souhaite retrouver un jour cette âme.

    D’ici là, nous porterons notre petite souris dans nos cœurs à jamais.

    C’est pourquoi ce projet de loisir est dédié à Baylee. À son esprit, à sa proximité, à sa joie du soleil et des oiseaux. Et à l’amour qui a grandi pendant notre temps ensemble.

    Ici, elle peut dormir en paix sur son nuage, dans la chaude lumière du soleil, entourée de fleurs, de pigeons et d’oiseaux du jardin.

    Repose en paix, ma très chère Baylee.

baylee-photoOpen = Voir la photo en entier
baylee-photoClose = Fermer la photo
baylee-zoomIn = Agrandir
baylee-zoomOut = Réduire
baylee-fit = Ajuster

baylee-photo6 = Baylee, bien au chaud parmi les couvertures.
baylee-photo7 = Blottie dans sa combinaison de récupération rouge.
baylee-photo8 = Un coin ensoleillé dans un carton.
baylee-photo9 = Bien bordée et complètement détendue.
baylee-photo10 = Sa place préférée : juste à côté de nous.

## Errors shared by the forms

error-busy = Trop de tentatives. Réessayez plus tard ; après plusieurs mots de passe erronés, cela peut prendre une heure, au plus un jour.
error-unavailable = Baylee Central ne répond pas pour le moment. Réessayez dans un instant.
error-network = Pas de connexion. Vérifiez votre réseau et réessayez.
error-failed = Une erreur s’est produite. Réessayez ; si cela continue, dites-le-nous.
error-again = Réessayer
password-show-label = Afficher le mot de passe

## Signing in

signin-title = Connexion
signin-lead = Votre compte Baylee, pour les tables en ligne, les contacts et le chat.
signin-name = Nom de connexion
signin-password = Mot de passe
signin-submit = Se connecter
signin-missing = Saisissez votre nom de connexion et votre mot de passe.
signin-working = Sécurisation de votre mot de passe … cela prend un instant et se passe uniquement dans votre navigateur.
signin-wrong = Ce nom de connexion et ce mot de passe ne correspondent pas. Vérifiez les deux et réessayez.
signin-not-moved = Ce compte n’a pas encore migré vers Baylee Central. Connectez-vous une fois avec le client Baylee ; il migre votre compte.
signin-no-account = Pas encore de compte ? <register>Créez-en un</register>.
signin-forgot = Mot de passe oublié ? <reset>Définissez-en un nouveau avec un code de réinitialisation</reset>.
register-have-account = Vous avez déjà un compte ? <signin>Connectez-vous</signin>.
dialog-close = Fermer
signin-noscript = La connexion nécessite JavaScript : votre mot de passe est traité dans votre navigateur et ne nous est jamais envoyé.

## Registering

register-title = Créer un compte
register-lead = Pour les tables en ligne, les contacts et le chat. Gratuit et ouvert à tous pendant la Beta ouverte.
register-name = Nom de connexion
register-name-hint = Privé, uniquement pour se connecter : 3 à 24 lettres minuscules, chiffres, points, tirets ou tirets bas.
register-display = Nom d’affichage
register-display-hint = Comment les autres joueurs vous voient : 3 à 16 lettres, chiffres, - et _. Votre tag est attribué pour vous.
register-password = Mot de passe
register-password-hint = Au moins 8 caractères. Une longue phrase est facile à retenir et difficile à deviner.
register-repeat = Répéter le mot de passe
register-repeat-hint = La même chose une fois de plus, pour repérer une faute de frappe.
register-age = J’ai 16 ans ou plus.
register-consent = J’accepte les <terms>conditions d’utilisation</terms>. La façon dont nous traitons vos données figure dans la <privacy>politique de confidentialité</privacy>.
register-submit = Créer le compte
register-error-name = Le nom de connexion doit comporter 3 à 24 lettres minuscules, chiffres, points, tirets ou tirets bas.
register-error-display = Le nom d’affichage doit comporter 3 à 16 lettres ou chiffres ; - et _ seulement à l’intérieur.
register-error-reserved = Ce nom d’affichage est réservé. Choisissez-en un autre.
register-error-short = Le mot de passe doit comporter au moins 8 caractères.
register-error-long = Le mot de passe est trop long.
register-error-repeat = Les deux mots de passe ne sont pas identiques.
register-error-age = Il faut avoir 16 ans ou plus pour avoir un compte.
register-error-consent = Acceptez les conditions d’utilisation pour créer un compte.
register-error-taken = Ce nom de connexion est déjà pris. Choisissez-en un autre.

## The recovery code

code-title = Votre code de récupération
code-handle = Votre identifiant : { $handle }
code-lead = Ce n’est qu’avec ce code que vos données chiffrées survivent si vous oubliez votre mot de passe. Nous ne le connaissons pas et ne pouvons pas le restaurer. Gardez-le en lieu sûr, loin de cet appareil.
code-shown-once = Affiché une seule fois
code-download = Enregistrer en fichier texte
code-kept = J’ai enregistré mon code de récupération
code-not-kept = Enregistrez d’abord le code, puis confirmez.
code-continue = Continuer
code-later = Pas maintenant
code-copy = Copier
code-copied = Copié.
code-copy-failed = La copie n’a pas fonctionné ici. Sélectionnez le code et copiez-le à la main.

## Setting a new password with a reset code

reset-title = Définir un nouveau mot de passe
reset-lead = Pour le cas où vous avez oublié votre mot de passe : avec un code de réinitialisation, vous en définissez ici un nouveau. Le code vous parvient par e-mail, à votre adresse confirmée, ou par un administrateur.
reset-noscript = Définir un nouveau mot de passe nécessite JavaScript : votre mot de passe et votre code de récupération sont traités dans votre navigateur et ne nous sont jamais envoyés.
reset-code = Code de réinitialisation
reset-code-hint = Par e-mail ou par un administrateur. Il commence par RESET et ne sert qu’une fois : pendant une heure s’il est arrivé par e-mail, pendant 24 heures s’il a été établi par un administrateur.
reset-data = Vos données chiffrées
reset-keep = J’ai mon code de récupération. Mes données restent.
reset-new = Je ne l’ai pas. Repartir avec un coffre vide.
reset-recovery = Code de récupération
reset-recovery-hint = Le code que vous avez enregistré à la création de votre compte. Il reste dans votre navigateur et ne nous est jamais envoyé.
reset-warning = Avec un coffre vide, vos données chiffrées sont perdues pour de bon : les réglages enregistrés dans votre compte, vos sauvegardes de decks et votre historique de chat. Personne ne peut les récupérer, nous non plus. Votre compte, votre nom d’affichage et vos contacts restent. Vous recevez un nouveau code de récupération.
reset-warning-confirm = J’ai compris que ces données sont perdues pour de bon.
reset-submit = Définir le nouveau mot de passe
reset-missing = Saisissez votre nom de connexion, le code de réinitialisation et un nouveau mot de passe.
reset-recovery-missing = Saisissez votre code de récupération, ou choisissez le coffre vide.
reset-error-code = Ce n’est pas un code de réinitialisation. Il commence par RESET, suivi de huit groupes de quatre caractères.
reset-error-recovery = Ce code de récupération n’ouvre pas les données de votre compte. Vérifiez-le et réessayez. Rien n’a été modifié, et votre code de réinitialisation reste valable.
reset-error-confirm = Confirmez d’abord que vous savez ce qui est perdu.
reset-refused = Ce nom de connexion et ce code de réinitialisation ne vont pas ensemble, ou le code a expiré ou a déjà servi. Vérifiez les deux. Si rien n’y fait, demandez un nouveau code.
reset-done-title = Votre nouveau mot de passe est défini
reset-done-kept = Vos données et votre code de récupération restent tels quels. Vous êtes déconnecté partout ; connectez-vous avec votre nouveau mot de passe.
reset-done-new = Vous repartez avec un coffre vide et vous êtes déconnecté partout. Votre nouveau code de récupération figure ci-dessous ; l’ancien ne fonctionne plus.
reset-unsure-title = Aucune réponse n’est arrivée
reset-unsure = La connexion s’est interrompue avant la réponse de Baylee Central ; nous ne pouvons donc pas dire si votre nouveau mot de passe a été défini. Essayez de vous connecter avec lui. Si cela échoue, rien n’a été modifié : votre code de réinitialisation reste valable, saisissez-le de nouveau ici.
reset-unsure-code = Si votre nouveau mot de passe fonctionne, ceci est votre nouveau code de récupération et l’ancien ne fonctionne plus ; il ne peut pas être affiché une seconde fois, enregistrez-le donc maintenant. Si votre nouveau mot de passe ne fonctionne pas, ce code ne vaut rien.
reset-to-signin = Se connecter
reset-mail-title = Pas de code de réinitialisation ?
reset-mail-lead = Saisissez votre nom de connexion. Si votre compte a une adresse e-mail confirmée, nous y envoyons un code de réinitialisation.
reset-mail-name-hint = Le nom avec lequel vous vous connectez, pas votre adresse e-mail.
reset-mail-submit = Envoyer un code par e-mail
reset-mail-error-name = Ce n’est pas un nom de connexion. Saisissez le nom avec lequel vous vous connectez, pas votre adresse e-mail.
reset-mail-sent = Si ce compte a une adresse e-mail confirmée, un code de réinitialisation y est en route. Il est valable une heure.
reset-mail-busy = Trop de demandes pour le moment. Réessayez plus tard.
reset-mail-none = Ce serveur n’envoie pas d’e-mails. Demandez un code de réinitialisation à un administrateur.
reset-mail-early = C’était trop rapide. Réessayez dans un instant.

## Changing the password

password-new = Nouveau mot de passe
password-new-repeat = Répéter le nouveau mot de passe
password-current = Mot de passe actuel
password-change-submit = Changer le mot de passe
password-missing = Saisissez votre mot de passe actuel et un nouveau.
password-error-current = Le mot de passe actuel n’est pas correct. Vérifiez-le et réessayez.
password-changed = Votre mot de passe est changé. Vous restez connecté ici et vous êtes déconnecté partout ailleurs, y compris dans le client Baylee ; là, connectez-vous avec le nouveau mot de passe.
password-unsure = Aucune réponse n’est arrivée ; nous ne pouvons donc pas dire si votre mot de passe a été changé. À votre prochaine connexion, essayez d’abord le nouveau mot de passe, puis l’ancien.

## An account's e-mail address

email-title = Adresse e-mail
email-lead = Une adresse e-mail est facultative. Nous nous en servons pour vous envoyer un code de réinitialisation quand vous en demandez un, et pour vous prévenir quand l’adresse est remplacée ou retirée.
email-warning = Quiconque peut lire cette boîte mail peut réinitialiser votre connexion. Vos données chiffrées ne survivent à une réinitialisation qu’avec votre code de récupération.
email-no-mail = Ce serveur n’envoie pas d’e-mails ; une adresse e-mail ne peut donc être ni enregistrée ni confirmée ici.
email-unavailable = L’adresse e-mail ne peut pas être affichée pour le moment.
email-password-hint = Votre mot de passe actuel, pour que personne d’autre devant ce navigateur ne décide qui peut réinitialiser votre connexion.
email-submit = Envoyer le code de confirmation
email-missing = Saisissez une adresse e-mail et votre mot de passe.
email-error-address = Nous ne pouvons pas écrire à cette adresse. Saisissez une adresse simple, comme nom@example.org ; les lettres accentuées et les trémas ne fonctionnent pas.
email-current = Votre adresse
email-confirmed-at = Confirmée
email-waiting = Attend son code
email-change = Modifier
email-change-lead = Votre adresse actuelle reste en usage jusqu’à ce que vous ayez confirmé la nouvelle.
email-change-cancel = Garder l’adresse actuelle
email-remove = Retirer
email-remove-ask = Retirer cette adresse ? Nous prévenons l’adresse qu’elle a été retirée. Ensuite, vous ne pourrez plus recevoir de code de réinitialisation par e-mail.
email-remove-ask-no-mail = Retirer cette adresse ? Tant que ce serveur n’envoie pas d’e-mails, vous ne pouvez pas en enregistrer une nouvelle.
email-remove-confirm = Retirer l’adresse
email-remove-keep = La garder
email-pending = Nous avons envoyé un code à <code>{ $address }</code>. Il est valable 15 minutes.
email-pending-keeps = Tant que vous ne l’avez pas confirmée, <code>{ $address }</code> reste votre adresse.
email-code = Code de confirmation
email-code-hint = Huit caractères, comme dans XXXX-XXXX. Majuscules ou minuscules, avec ou sans trait d’union.
email-confirm = Confirmer
email-resend = Renvoyer
email-other = Utiliser une autre adresse
email-withdraw = Annuler
email-error-code = Ce n’est pas un code de confirmation. Il comporte huit caractères, comme dans XXXX-XXXX.
email-wrong-code = Ce code n’est pas le bon. Vérifiez-le et réessayez. Après plusieurs codes erronés, vous devez saisir à nouveau l’adresse.
email-code-dropped = Ce code n’était pas le bon non plus, l’adresse n’attend donc plus de code. Saisissez-la à nouveau pour obtenir un nouveau code.
email-now-confirmed = Votre adresse e-mail est confirmée.
email-resent = Un nouveau code est en route. Le précédent ne fonctionne plus.
email-withdrawn = L’adresse en attente est annulée. Plus aucun code n’est attendu.
email-removed = Votre adresse e-mail est retirée.
email-busy = Trop d’e-mails pour le moment. Réessayez plus tard.
email-same = C’est déjà votre adresse.
email-unsure = Aucune réponse n’est revenue, nous ne pouvons donc pas savoir si l’action a abouti. Regardez ce qui s’affiche ici et réessayez si elle n’a pas abouti.

## Central's terms

terms-step-title = Nouvelles conditions d’utilisation
terms-step-lead = Les conditions de Baylee Central ont changé. Lisez-les et acceptez-les pour utiliser votre compte ; vous pouvez aussi refuser et vous déconnecter.
terms-accept = Accepter
terms-decline = Refuser et se déconnecter

## The signed-in area

account-lead = Votre compte Baylee : profil, sécurité, confidentialité et gateways.
account-nav = Votre compte
account-home = Vue d’ensemble
account-profile = Profil
account-profile-lead = Votre identifiant, votre nom d’affichage et votre code de récupération.
account-security = Sécurité
account-security-lead = Où vous êtes connecté, et la déconnexion.
account-privacy = Confidentialité et données
account-privacy-lead = Exportez vos données ou supprimez votre compte.
account-gateways = Gateways et jetons
account-gateways-lead = Les gateways que vous avez enregistrées et les jetons avec lesquels elles se connectent.
account-loading = Chargement …
account-signed-out = Vous n’êtes pas connecté.
account-unnamed = Votre compte
account-signin-name = Nom de connexion : { $name }
account-until = Connecté jusqu’à { $time }
account-needs-title = Demande votre attention
account-needs-code = Créer un code de récupération
account-needs-name = Choisir votre nom d’affichage
account-admin = Administration

reauth-lead = Cela demande une connexion récente. Saisissez encore une fois votre nom de connexion et votre mot de passe.
reauth-submit = Se reconnecter

profile-who-title = Qui vous êtes
profile-handle = Identifiant
profile-signin-name = Nom de connexion
profile-source = Compte
profile-source-moved = Migré depuis la gateway officielle
profile-source-registered = Créé sur ce site
profile-choose-title = Choisir votre nom d’affichage
profile-choose-submit = Enregistrer le nom d’affichage
profile-name-taken = Ce compte a déjà un nom d’affichage.
profile-code-title = Code de récupération
profile-code-kept = Votre compte a un code de récupération. Gardez-le en lieu sûr ; sans votre mot de passe, c’est le seul moyen de retrouver vos données chiffrées.
profile-code-missing = Votre compte n’a pas encore de code de récupération. Créez-en un ; vous aurez de nouveau besoin de votre mot de passe.
profile-code-make = Créer un code de récupération

security-sessions = Où vous êtes connecté
security-lead = Baylee Central conserve une session pour ce site et une pour le client de jeu ; il ne distingue pas les appareils individuels.
security-kind-browser = Ce site
security-kind-client = Le client Baylee
security-this = Celle-ci
security-signed-in = Connectée
security-ends = Fin
security-gateways = Sur les gateways
security-end-client = Déconnecter le client
security-client-ended = Le client est déconnecté.
security-sign-out = Se déconnecter ici
security-password-title = Mot de passe
security-password-lead = Vos données chiffrées et votre code de récupération restent tels quels. Changer le mot de passe vous déconnecte partout sauf ici.

privacy-export-title = Exporter vos données
privacy-export-lead = Ce que Baylee Central sait de votre compte, sous forme de fichier JSON : ses données de base, les sessions, les conditions acceptées, vos gateways et les entrées du journal d’audit qui nomment votre compte. Vos contenus chiffrés ne peuvent être déchiffrés et exportés que par votre client Baylee. L’export demande une connexion de moins de 30 minutes.
privacy-export-make = Préparer l’export
privacy-export-save = Enregistrer l’export
privacy-export-no-email = L’adresse e-mail n’a pas pu être lue ; elle ne figure pas dans ce fichier.
privacy-delete-title = Supprimer votre compte
privacy-delete-lead = Votre compte, ses données, ses sessions, vos contacts et les gateways dont vous êtes propriétaire sont supprimés immédiatement et définitivement du système en fonctionnement ; ils quittent les sauvegardes dans les 12 mois au plus tard. Central supprime aussi les messages que vous avez envoyés ; ce que d’autres ont déjà enregistré sur leurs appareils peut y rester.
privacy-delete-start = Supprimer mon compte
privacy-delete-password = Votre mot de passe, pour confirmer
privacy-delete-prove = Continuer
privacy-delete-last = Dernière étape : cela ne peut pas être annulé. Vous avez une minute.
privacy-delete-gateways = Ces gateways dont vous êtes propriétaire sont supprimées avec lui :
privacy-delete-confirm = Supprimer le compte définitivement
privacy-delete-cancel = Garder mon compte
privacy-deleted = Votre compte est supprimé.

gateways-none = Vous n’avez enregistré aucune gateway. La plupart des joueurs n’en ont jamais besoin.
gateways-add-title = Enregistrer une gateway
gateways-origin = Adresse de la gateway
gateways-origin-hint = L’adresse https de la gateway, comme https://play.example.org.
gateways-name = Nom de la gateway
gateways-add = Enregistrer la gateway
gateways-added = La gateway est enregistrée et attend l’approbation d’un administrateur.
gateways-taken = Cette adresse est déjà enregistrée.
gateways-invalid = Saisissez une adresse https sans chemin, et un nom.
gateways-approved = Approuvée
gateways-pending = En attente d’approbation
gateways-disabled = Désactivée
tokens-title = Jetons
tokens-name = Nom du jeton
tokens-days = Valable (jours)
tokens-sso = Connecte les joueurs
tokens-listing = Référence la gateway
tokens-both = connecte les joueurs et référence la gateway
tokens-make = Créer un jeton
tokens-revoke = Révoquer
tokens-revoked = Révoqué
tokens-revoked-now = Le jeton est révoqué.
tokens-until = Jusqu’au { $time }
tokens-invalid = Donnez un nom au jeton et au moins une chose qu’il autorise.
tokens-shown-title = Votre nouveau jeton
tokens-kept = Je l’ai enregistré
tokens-again = Reconnecté. Créez de nouveau le jeton.

## The messenger

account-chat = Chat
account-contacts = Contacts
chat-side = Conversations et contacts
chat-tabs = Messagerie
chat-tab-chats = Chats
chat-tab-contacts = Contacts
chat-main = Conversation
chat-unread = non lus
chat-conversations = Vos conversations
chat-search = Trouver une conversation
chat-search-hint = Chercher (Ctrl+K)
chat-new-group = Nouveau groupe
chat-no-conversations = Pas encore de conversation. Ajoutez un contact, puis écrivez-lui.
chat-no-match = Aucune conversation ne porte ce nom.
chat-former-group = Un ancien groupe
chat-former-lead = Vous n’êtes plus dans ce groupe.
chat-you = Vous
chat-members = { $n ->
    [one] { $n } membre
   *[other] { $n } membres
}
chat-say-hello = Dites bonjour.
chat-history-only = N’est plus un contact.
chat-today = Aujourd’hui
chat-yesterday = Hier
chat-encrypted = Chiffré de bout en bout
chat-opening-lead = Vos conversations et vos contacts. Les messages sont chiffrés sur vos appareils avant de les quitter, et Central les transmet sans pouvoir les lire.
chat-new-from = Nouveau message de { $who }.
chat-new-many = { $n } nouveaux messages.
chat-retry = Réessayer
chat-discard = Abandonner
chat-not-sent = Non envoyé.
chat-sending = Envoi …
chat-keys-changed = Les clés de quelqu’un dans cette conversation ont changé. Comparez le numéro de sécurité avant d’écrire de nouveau.
chat-refused = Certains messages n’ont pas pu être vérifiés et ne sont pas affichés.
chat-failed = Le chat n’a pas pu joindre Central à l’instant.
chat-failed-open = Le chat n’a pas pu être ouvert. Rechargez la page pour réessayer.
chat-empty = Pas encore de message.
chat-older = Afficher les anciens messages
chat-compare-first = Les clés ont changé. Comparez d’abord le numéro de sécurité, puis écrivez.
chat-compare = Comparer le numéro de sécurité
chat-friend-required = Vous pourrez écrire quand vous serez de nouveau contacts.
chat-group-left = Vous n’êtes plus dans ce groupe.
chat-details = Détails
chat-close = Fermer
chat-back = Retour aux conversations
chat-messages = Messages
chat-fan = Baylee est un contenu de fan non officiel, ni approuvé ni promu par Wizards. <more>Ce que cela signifie</more>
chat-safety = Numéros de sécurité
chat-safety-lead = Comparez le numéro avec l’autre personne, en personne ou sur un autre canal de confiance. S’il correspond, personne ne s’est glissé entre vous à votre insu.
chat-changed = Modifié
chat-confirm-keys = Il correspond : faire confiance aux nouvelles clés
chat-count = { $n } caractères sur 1 000
chat-write = Écrire un message
chat-send = Envoyer
chat-enter-hint = Entrée envoie, Maj+Entrée passe à la ligne.
chat-choose = Choisir une conversation
chat-choose-lead = Vos messages sont chiffrés dans ce navigateur ; Central ne fait que les transmettre.
chat-storage = Votre clé de chat reste dans ce navigateur tant que vous êtes connecté.
chat-opening = Ouverture de vos chats …
chat-open = Ouvrir mes chats
chat-ended = Votre connexion a pris fin.
chat-unnamed = Choisissez d’abord votre nom d’affichage : il vous donne l’identifiant auquel les autres écrivent.
chat-missing-title = Reconnectez-vous pour ouvrir vos chats
chat-missing-lead = Ce navigateur ne détient pas votre clé de chat. Votre mot de passe l’ouvre ici ; il ne quitte jamais ce navigateur.
chat-invites = Invitations à des groupes
chat-invited-by = de { $who }
chat-group-name = Nom du groupe
chat-group-name-hint = Un nom de 1 à 48 caractères.
chat-group-create = Créer
chat-group-failed = Le groupe n’a pas pu être modifié à l’instant.
chat-group-members = { $n ->
    [one] { $n } membre
   *[other] { $n } membres
}
chat-group-owner = Propriétaire
chat-group-rename = Renommer
chat-group-transfer = Nommer propriétaire
chat-group-remove = Retirer
chat-group-transfer-ask = Nommer { $who } propriétaire de ce groupe ? Vous restez membre.
chat-group-remove-ask = Retirer { $who } de ce groupe ?
chat-group-invite = Inviter un contact
chat-group-choose = Choisir un contact
chat-group-invite-button = Inviter
chat-group-invited = Invité
chat-group-delete = Supprimer le groupe
chat-group-leave = Quitter le groupe
chat-group-delete-ask = Supprimer « { $name } » pour tout le monde ? Les messages déjà reçus restent lisibles.
chat-group-leave-ask = Quitter « { $name } » ? Vous ne pourrez revenir qu’avec une nouvelle invitation.

people-incoming = demandes
people-failed = Vos contacts n’ont pas pu être chargés à l’instant.
people-empty = Pas encore de contact. Trouvez quelqu’un par son identifiant, ou donnez-lui le vôtre.
people-your-handle = Votre identifiant :
people-incoming-title = Demandes qui vous sont adressées
people-friends = Contacts
people-outgoing = Demandes envoyées
people-blocked = Bloqués
people-find = Ajouter un contact par son identifiant
people-find-button = Chercher
people-handle-hint = Un identifiant complet avec son tag, comme Name#1a2b.
people-yourself = C’est votre propre identifiant.
people-nobody = Personne n’a cet identifiant.
people-request = Envoyer une demande de contact
people-requested = Demande envoyée.
people-is-friend = Un contact
people-is-incoming = A demandé à devenir votre contact
people-is-outgoing = Demande envoyée
people-is-blocked = Bloqué
people-write = Écrire
people-remove = Retirer
people-block = Bloquer
people-accept = Accepter
people-decline = Refuser
people-cancel = Retirer la demande
people-unblock = Débloquer
people-keep = Garder
people-confirm-remove = Retirer { $who } de vos contacts ? Vos messages restent.
people-confirm-block = Bloquer { $who } ? Cette personne ne pourra plus demander à devenir votre contact ni vous écrire.

## Offline, updates, installing

title-offline = Hors ligne
offline-title = Vous êtes hors ligne
offline-lead = Cette page nécessite une connexion. Central garde vos messages jusqu’à votre retour.
offline-again = Réessayer
update-ready = Une nouvelle version de baylee.app est prête.
update-unsent = Vous avez ici du texte pas encore envoyé. Recharger quand même ?
update-reload = Recharger
install-app = Installer comme application

## Notifications

push-title = Notifications
push-lead = Choisissez ce dont Baylee peut vous notifier. Les notifications n’affichent ni texte de message, ni noms, ni contenu de rapports.
push-device = Notifications sur cet appareil
push-categories = Me notifier pour
push-chat = Messages de chat
push-contacts = Demandes de contact
push-updates = Nouvelles versions
push-feedback = Rapports de retours (administrateurs)
push-gateways = Demandes de gateways (administrateurs)
push-on = Activées sur cet appareil. Seules les catégories choisies sont envoyées.
push-off = Désactivées sur cet appareil. Vos choix de catégories sont conservés.
push-scope = Les choix de catégories s’appliquent à votre compte. L’autorisation de l’appareil est distincte sur chaque navigateur. La déconnexion déconnecte cet appareil.
push-failed = La modification n’a pas pu être confirmée. Réessayez.
push-denied = Les notifications sont bloquées. Vous pouvez les autoriser dans les réglages de votre navigateur ou de votre appareil, puis réessayer.
push-unsupported = Ce navigateur ne propose pas Web Push ici. Sur iPhone et iPad, ajoutez Baylee à l’écran d’accueil et ouvrez-le depuis là.
push-unconfigured = Les notifications en arrière-plan ne sont pas encore disponibles sur ce serveur. Vous pouvez déjà enregistrer vos préférences.

register-closed = Les inscriptions sont suspendues pour le moment. Réessayez plus tard.
register-refused = L’inscription n’a pas été acceptée. Réessayez.
admin-registration = Inscription
admin-registration-open = Ouverte : chacun peut créer un compte après avoir résolu la preuve de travail.
admin-registration-closed = Fermée : personne ne peut créer de compte tant que vous ne la rouvrez pas.
admin-registration-lead = Fermez les inscriptions pendant une attaque et rouvrez-les à la main. Chaque changement est consigné dans le journal d’audit.
admin-registration-close = Fermer les inscriptions
admin-registration-reopen = Rouvrir les inscriptions
hero-discord = Communauté Discord
hero-whatsapp = Communauté WhatsApp
hero-github = GitHub
nav-menu = Menu
foot-project = Projet
foot-source = Code source (AGPL-3.0)
foot-central = Baylee Central (code source disponible)
foot-summary = Jeu de cartes de table gratuit et open source, conçu en Rust et Bevy, avec un moteur de règles déterministe, un mode hors ligne complet, une IA maison, un pont vers les modèles de langage et des gateways communautaires.
strip-ci = CI
strip-commit = Commit
strip-gateways = Gateways
strip-online = en ligne
strip-offline = hors ligne
strip-directory = l’annuaire
strip-cards = Cartes jouables
strip-cards-unit = cartes
strip-accounts = Comptes sur Baylee Central
strip-live = En ce moment sur les gateways, selon leurs déclarations
strip-views = Pages vues depuis le { $date }
live-views = { $n ->
    [one] { $n } vue
   *[other] { $n } vues
}
live-accounts = { $n ->
    [one] { $n } compte
   *[other] { $n } comptes
}
live-players = { $n ->
    [one] { $n } joueur
   *[other] { $n } joueurs
}
live-playing = { $n ->
    [one] { $n } en partie
   *[other] { $n } en partie
}
live-tables = { $n ->
    [one] { $n } table ouverte
   *[other] { $n } tables ouvertes
}
live-games = { $n ->
    [one] { $n } partie
   *[other] { $n } parties
}
hero-badges = En un coup d’œil
hero-badge-beta = Beta ouverte
hero-badge-free = Gratuit
hero-badge-open = Open source
hero-badge-fan = Contenu de fan non officiel
gws-open-client = Ouvrir dans Baylee
gws-open-client-help = « Open in Baylee » [Ouvrir dans Baylee] lance le client installé (0.1.1-beta.9b ou plus récent). Il affiche l’adresse et demande avant d’ajouter la gateway.
dl-menu-title = Tous les systèmes
dl-old-cpu = anciens processeurs (sans AVX2)
ci-passing = réussi
ci-failing = échoué
dl-button = Télécharger Baylee
dl-quick = Autres systèmes et architectures
dl-others = autres systèmes

# The start page, as the old site wrote it (owner, 11.11.2026)
link-readmeModel = #playing-against-a-language-model
about-engineTitle = Un moteur qui impose chaque règle
about-deterministic = Alimentée par un générateur aléatoire à graine, sans aucune dépendance à l’horloge ni à l’ordre de hachage, chaque partie est strictement déterministe et reproductible coup par coup.
about-tableTitle = Une table 3D épurée construite avec Bevy
about-keys = Depuis le salon intégré, vous passez sans effort de l’éditeur de decks local aux parties en solo et aux tables multijoueurs sur les gateways communautaires.
feature-hiddenTitle = Les informations cachées restent privées
feature-hidden = Votre client ne reçoit que les données que votre place a le droit de voir. Les bibliothèques et les mains des adversaires n’arrivent que sous forme de nombres—même un client modifié ne peut pas inspecter les cartes cachées.
feature-seatsTitle = Jusqu’à huit joueurs à une même table
feature-seats = Duels, chacun pour soi et parties par équipes (comme le 2 contre 2) jusqu’à 8 places. Les équipiers partagent les tours et peuvent se montrer les cartes de leur main.
feature-commanderTitle = Formats souples et règles maison personnalisées
feature-commander = Composez des decks pour des duels classiques à 60 cartes, des parties multijoueurs en cercle ou
    des formats maison sur mesure. Les points de vie de départ, la composition des équipes et les
    règles de mulligan sont fixés librement par l’hôte.
feature-aiTitle = Rediffusions déterministes et rapports F8
feature-ai = Chaque partie est enregistrée coup par coup. Si vous repérez un problème d’interaction de règles en jouant, appuyez sur F8 directement à la table pour envoyer un rapport d’état reproductible.
feature-processTitle = Un processus serveur isolé par partie
feature-process = Chaque partie en cours tourne dans son propre processus de moteur isolé sur la gateway. La gateway elle-même ne gère que le salon et les salles—une erreur n’affecte jamais les autres tables.
feature-languagesTitle = Texte des cartes en 19 langues
feature-languages = Chaque fois qu’une gateway fournit le catalogue de cartes multilingue, les cartes se lisent dans votre langue préférée, avec l’anglais en repli. L’interface du client est disponible en allemand et en anglais.
feature-watchTitle = Regarder des parties en direct
feature-watch = Le bouton Watch du salon vous place à la table comme spectateur en direct : suivez le champ de bataille public en temps réel pendant que toutes les mains des joueurs restent cachées.
feature-connectionTitle = Reconnexion transparente en cas de coupure
feature-connection = Si une connexion tombe brièvement, la table garde la place avec un compte à rebours et peut mettre la partie en pause ou laisser l’IA maison intervenir temporairement.
feature-tablesTitle = Quatorze minuteurs de décision
feature-tables = Des tours rapides de 15 secondes aux tables détendues sans limite de temps, réglez le rythme
    individuellement pour chaque salle.
shots-eyebrow = À la table
shots-title = Dans le jeu
shots-lead = De courts enregistrements en direct, capturés directement dans le client 3D, avec des images de
    cartes de Scryfall (© Wizards of the Coast)—des duels et des parties à 4 joueurs jusqu’au salon
    chiffré et à l’éditeur de decks.
shots-credit = Images de cartes : Scryfall · © Wizards of the Coast
shots-showLarge = Agrandir : { $title }
shots-dialog = Capture d’écran
shots-previous = Précédent
shots-next = Suivant
shots-close = Fermer
ai-eyebrow = Adversaires IA et modèles de langage
ai-title = IA maison, pont LLM et IA native de Baylee
ai-houseTitle = IA maison heuristique en cinq niveaux
ai-house = Prête à jouer hors ligne et à toute table en ligne : choisissez entre Novice, Occasionnel, Régulier, Aiguisé et Expert. L’IA maison décide localement sans aucune attente et ne voit que le même état public qu’un joueur humain.
ai-modelTitle = Des modèles de langage à la table via Seat-Bridge
ai-model = Installez <b>Google Gemini, OpenAI ChatGPT, Anthropic Claude, DeepSeek</b> ou des modèles locaux via <b>LM Studio</b> directement à votre table. Les clés d’API restent protégées dans des variables d’environnement avec des plafonds de dépenses ; en cas de dépassement de délai, l’IA maison prend le relais sans accroc.
ai-setup = Guide de configuration LLM et Seat-Bridge
home-gwLead = Les tables en ligne vivent sur des gateways, des serveurs que Baylee Central référence une fois que son administrateur les a approuvés. Voici les premières ; l’annuaire les contient toutes.
beta-open = Ouvrir l’annuaire
beta-own = Faire tourner la vôtre
source-eyebrow = Source
source-title = Open source, et cela le reste
source-promise = Baylee sera toujours open source et gratuit. Pas de paywall, pas de fonctions payantes, pas de publicité. Ni maintenant, ni plus tard. Le code est sous la GNU Affero General Public License, ce qui signifie que quiconque fait tourner une gateway modifiée pour d’autres personnes doit aussi leur en offrir la source.
source-rust = Il est écrit en Rust, c’est le projet d’une seule personne avec beaucoup d’aide d’agents de programmation, et chaque modification passe par la même barrière : formatage, lints, suite de tests et validation de chaque carte d’après son texte imprimé. Les tickets et les pull requests sont les bienvenus ; <agents>AGENTS.md</agents> et <claude>CLAUDE.md</claude> sont les règles de travail pour les personnes comme pour les agents.
source-discord = Communauté sur Discord
source-whatsapp = Communauté sur WhatsApp
source-architecture = Architecture
source-repoTitle = Le dépôt en ce moment
source-stars = Étoiles
source-issues = Tickets et pull requests ouverts
source-note = Lu depuis l’API publique de GitHub par le serveur de ce site toutes les dix minutes. Votre navigateur ne demande rien à GitHub.
gw-count =
    { $n ->
        [one] { $n } gateway listée, { $online } en ligne.
       *[other] { $n } gateways listées, { $online } en ligne.
    }
gw-none = Aucune gateway n’est encore listée.
contact-intro = Une question, une idée, un jeu que vous aimeriez voir, ou quelque chose que vous préférez ne pas publier : ce formulaire va directement à la personne derrière Baylee. Laissez une adresse e-mail si vous voulez une réponse ; sans elle, nous ne pouvons que le lire.
contact-f8 = Une carte mal résolue se signale au mieux depuis le jeu avec <kbd>F8</kbd>, car le rapport peut contenir toute la partie.
contact-discord = discuter, trouver des adversaires, être au courant des versions
contact-issues = Tickets GitHub
contact-issuesSub = bugs et idées, en public
contact-operator = Qui exploite ce site, avec une adresse postale : les <impressum>mentions légales</impressum>.
contact-whatsappSub = Groupe communautaire et parties rapides
contact-reportsTitle = Rapports et confidentialité
contact-reportsSub = Détails sur les rapports F8 et les données
ai-housePill = Hors ligne et en ligne
ai-providers = Modèles de langage pris en charge
ai-hostsPill = Pour les hébergeurs de gateways
ai-hostsTitle = Proposez des places LLM hébergées sur votre gateway
ai-hosts = En tant qu’exploitant de gateway, vous pouvez fournir des modèles de langage préconfigurés directement sur votre serveur. Les invités d’une table choisissent simplement l’un de vos modèles hébergés comme adversaire—sans clé d’API.
ai-trainingPill = En entraînement · IA native du moteur
ai-trainingTitle = IA native de Baylee — entraînée à jouer directement sur le moteur comme un vrai joueur
ai-training = À côté de l’IA maison et du pont LLM externe, une IA native de Baylee dédiée est entraînée directement sur le moteur de règles déterministe. Elle apprend à évaluer les états du plateau, les fenêtres de priorité, les calculs de combat et les synergies de deck comme un joueur humain expérimenté—sans aucune latence d’API externe.
ai-hostingLink = Hébergement de gateway et configuration LLM

# The gateways and the contact page, as the old site wrote them (owner, 11.11.2026)
# English for now: to be translated.
gw-lead = A gateway is a server where online tables live: accounts, decks, rooms and one engine process per game. These are the ones registered with Baylee Central and approved by its administrator. Each is run by its operators for their groups, and their own rules apply there. A listing is not a promise about what a gateway does.
gw-howTo = Pour jouer sur l’une d’elles, copiez son adresse dans l’écran d’accueil du client. En ligne signifie que Central a eu des nouvelles de la gateway au cours de la dernière minute et demie ; ce n’est pas une promesse qu’elle répond. Les noms et les versions sont ceux que chaque exploitant indique.
gw-find = Trouver une gateway
gw-findHint = Nom ou adresse
gw-shown = { $n } sur { $all } affichées
gw-noMatch = Aucune gateway chargée jusqu’ici ne correspond.
gw-showAll = Tout afficher
gw-stats = L’annuaire en chiffres
gw-stat-listed = Gateways listées
gw-stat-online = En ligne maintenant
gw-stat-players = Joueurs en ligne
gw-stat-playing = Dont en partie
gw-stat-tables = Tables ouvertes
gw-stat-games = Parties en cours
gw-show = Afficher
gw-show-every = Toutes
gw-show-online = En ligne
gw-show-tables = Avec tables ouvertes
gw-sort = Trier par
gw-sort-best = Recommandées
gw-sort-name = Nom (A–Z)
gw-sort-players = Plus de joueurs
gw-sort-tables = Plus de tables ouvertes
gw-sort-version = Version la plus récente
gw-view = Vue
gw-view-cards = Tuiles
gw-view-list = Liste
link-readmeGateway = #running-your-own-gateway
selfhost-eyebrow = Auto-hébergement et intégration à Central
selfhost-title = Faites tourner votre propre gateway
selfhost-processes = Une gateway auto-hébergée fait tourner trois processus légers à côté de PostgreSQL 18+ : <code>baylee-gateway</code> gère les salles, les decks et le routage ; <code>baylee-agent</code> lance un processus <code>baylee-engine-server</code> isolé par partie, de sorte que chaque table est confinée.
selfhost-proxy = Placez un proxy inverse TLS devant votre origine HTTPS (par ex. <code>https://play.example.org</code>), importez éventuellement le catalogue en masse de Scryfall pour le texte des cartes en 19 langues, et les joueurs peuvent se connecter directement via votre adresse dans le client.
selfhost-central = <b>Lier à Baylee Central et configurer la confidentialité :</b> enregistrez votre origine HTTPS sous <mine>Vos gateways</mine> et générez un jeton <code>bgt_…</code> (portées : <code>sso</code> pour l’authentification unique, <code>listing</code> pour les signaux de présence dans l’annuaire et la télémétrie en direct). Dans la console de votre gateway, choisissez votre niveau de confidentialité de la télémétrie (<code>full</code> pour les parties en cours, les réglages de table et les joueurs ayant accepté, <code>counts_only</code> pour les totaux seulement, ou <code>private</code> pour la seule disponibilité).
selfhost-guide = <guide>Lire le guide complet d’auto-hébergement</guide> · <protocol>Référence du protocole et des signaux de présence (docs/protocol.md)</protocol>
selfhost-source = Chaque gateway répond à <code>GET /source</code> avec sa licence, sa version et son commit sans connexion, remplissant automatiquement l’obligation de divulgation du code source de l’AGPL.
reports-eyebrow = Rapports
reports-title = Vous avez trouvé un problème ? Dites-le-nous
reports-f8 = Appuyez sur <kbd>F8</kbd> à la table ou dans le salon. Un rapport contient son genre, votre texte, la version du client et, à une table, l’identifiant de la partie. Tout le reste a sa propre case, et chaque case est cochée au départ : décochez donc ce que vous préférez garder avant d’appuyer sur Send : système et matériel, la table telle que vous la voyez, votre journal (les autres joueurs renommés « Player A », « Player B », …), vos réglages, une image de la seule fenêtre Baylee, et, à une table, l’enregistrement complet de la partie (chaque coup et les cartes de chaque place, sans noms). <i>Show what is sent</i> montre chaque octet. Les jetons de session ne sont jamais envoyés.
reports-usage = Pendant que vous jouez hors ligne, le client indique une fois par minute qu’une partie est en cours : pas de compte, pas d’identifiant d’appareil, rien sur la partie. <i>Send anonymous usage count</i> sous Settings › Privacy & Data le désactive.
reports-discord = Envie de discuter, de trouver des adversaires ou d’être le premier à connaître les nouvelles versions ? Rejoignez la <discord>communauté Baylee sur Discord</discord>.
reports-issues = Questions de règles, comportement incorrect d’une carte, idées : <issues>ouvrez un ticket sur GitHub</issues>. Une carte mal résolue est le rapport le plus utile que nous puissions recevoir, car chaque carte a un test et la vôtre en devient un.
reports-privacy = <docs>docs/privacy.md</docs> énumère tout ce que conservent le client, la gateway et le service de retours. Ce site ne dépose aucun cookie tant que vous ne vous connectez pas, n’utilise aucune analyse d’audience et ne charge ni polices ni scripts depuis le serveur de quelqu’un d’autre. <notice>La politique de confidentialité</notice> dit ce que conservent le site et Baylee Central, et nomme chaque autre hôte avec lequel ils communiquent, et quand.

# The download page, as the old site set it (owner, 11.11.2026)
dlPage-eyebrow = Pour l’obtenir
dlPage-filesEyebrow = Chaque fichier
dlPage-filesTitle = Tous les systèmes
install-eyebrow = Installation
install-releases = Toutes les versions sur GitHub
release-notes = Notes de version et sommes de contrôle
release-all = Toutes les versions

# The download page's head, as the old site wrote it
dlPage-title = Télécharger
dlPage-lead = Chaque téléchargement mène directement au fichier sur GitHub, sans intermédiaire. Quand une version propose des installateurs, c’est la voie facile ; les archives contiennent le même jeu, à décompresser n’importe où. Un Baylee installé se met à jour à partir des archives signées partout où il peut écrire dans son propre dossier. Chaque paquet contient le Seat-Bridge pour les modèles de langage à côté du jeu.

# The reading pages' eyebrows, as the old site set them
imp-eyebrow = Mentions légales
terms-eyebrow = Conditions d’utilisation
privacy-eyebrow = Confidentialité

# The privacy notice's contents, as the old site named them
privacy-contents = Sommaire

# The account pages' eyebrow, as the old site set it
acct-eyebrow = Baylee Central

# The account area's navigation, as the old site named it
app-account = Votre compte

# ==== The security page (/security) ====================================
# Every sentence and every arrow of this page maps to a row of the claim
# table in docs/security-page.md; a claim changes
# there first. No "safe", "unbreakable" or compliance words: mechanisms only.
title-security = La sécurité dans Baylee : mots de passe, clés et chats
desc-security = Comment Baylee traite les mots de passe, les clés et les messages de chat, pas à pas en quatre
    schémas, avec les limites de chacun.
nav-security = Sécurité
sec-eyebrow = Sécurité
sec-title = Mots de passe, clés et chats : ce qui va où
sec-lead = Quatre flux, pas à pas : ce qui quitte votre appareil, ce que Baylee Central voit et ce qu’il ne
    voit pas. Avec les limites de chacun.
sec-toc-label = Sur cette page
sec-figure = Schéma en étapes : { $name }
sec-play = Lecture
sec-all = Toutes les étapes
sec-step = Étape { $n }
sec-hint = Le schéma se déroule tout seul. Choisissez une étape pour l’arrêter dessus.
sec-hint-still = Choisissez une étape pour la mettre en évidence.
sec-in-browser = Le cadre en pointillés est votre navigateur : tout cela se passe à l’intérieur.
# The actors' names are the only words inside the drawings, where text does
# not wrap: one or two words each, at most about twice the English width
# (the test `names_fit_with_room_to_grow` measures every language).
sec-actor-device = Votre appareil
sec-actor-central = Central
sec-actor-page = Page
sec-actor-worker = Worker
sec-actor-store = Stockage
sec-actor-you = Vous
sec-actor-contact = Contact
sec-actor-website = Site web
sec-actor-client = Client

sec-signin-eyebrow = Connexion et inscription
sec-signin-title = Central vérifie votre mot de passe sans le recevoir
sec-signin-lead = Baylee vous connecte avec OPAQUE (RFC 9807). Votre appareil et Central échangent trois
    messages ; votre mot de passe ne figure dans aucun d’eux, pas même sous forme de simple hachage.
sec-signin-c1 = Requête masquée
sec-signin-t1 = Votre appareil masque le mot de passe.
sec-signin-s1 = Il combine le mot de passe avec un nombre aléatoire neuf et n’envoie que le résultat. Le mot de
    passe ne peut pas être lu dans ce message.
sec-signin-c2 = Réponse de Central
sec-signin-t2 = Central répond avec une clé qui lui est propre.
sec-signin-s2 = Central calcule sur la valeur masquée avec une clé que lui seul détient et ajoute sa moitié d’un
    échange de clés. Il n’apprend pas le mot de passe ce faisant.
sec-signin-c3 = Argon2id · 64 MiB
sec-signin-t3 = Votre appareil fait le gros du travail.
sec-signin-s3 = Il retire le masque et étire le résultat avec Argon2id (64 MiB de mémoire, trois passes) ;
    Central ne peut pas demander de réglages moins coûteux. Un mot de passe erroné est détecté ici,
    avant que quoi que ce soit d’autre soit envoyé.
sec-signin-c4 = Preuve
sec-signin-t4 = Votre appareil prouve qu’il a les bonnes clés.
sec-signin-s4 = Avec le bon mot de passe, votre appareil aboutit à la même clé de session que Central et en
    envoie une courte preuve (64 octets).
sec-signin-c5 = Session + enveloppe
sec-signin-t5 = Central ouvre la session.
sec-signin-s5 = En retour arrivent la session (dans le navigateur, un cookie que la page elle-même ne peut pas
    lire) et votre clé maîtresse dans une enveloppe qui ne s’ouvre que sur votre appareil, avec le
    bon mot de passe.
sec-signin-register = <b>L’inscription</b> suit le même protocole. Votre appareil tire une clé maîtresse aléatoire
    (256 bits), l’enveloppe une fois sous votre mot de passe et une fois sous votre code de
    récupération, et n’envoie que les enveloppes. Central stocke un enregistrement OPAQUE et les
    enveloppes, jamais la clé.
sec-signin-legacy = <b>Une exception :</b> les comptes de l’ancienne gateway officielle qui n’ont pas encore migré
    vers Central. Tant qu’un tel compte n’a pas migré, chaque tentative de connexion dans le client
    Baylee envoie le mot de passe saisi, par une connexion TLS vérifiée, à Central, qui le compare à
    l’ancien hachage sans le stocker ni le consigner dans un journal. Une fois la migration réussie,
    le compte se connecte comme indiqué plus haut. Ce site web ne connecte pas de tels comptes ; il
    renvoie vers le client. L’exception prend fin à l’échéance fixée pour la migration.
sec-signin-limit = <b>Ce qui reste :</b> quiconque prend la base de données de Central avec ses clés peut essayer
    les mots de passe un par un ; chaque essai coûte un calcul Argon2id. Un mot de passe trouvé
    ainsi ouvre les données chiffrées du compte et ses chats stockés. Un mot de passe long que vous
    n’utilisez nulle part ailleurs compte malgré tout.
sec-signin-guess = <b>Deviner un mot de passe via la connexion</b> est ralenti, pas arrêté. Central compte les tentatives qui n’aboutissent pas avec le bon mot de passe : au plus 100 par jour sur un compte depuis des adresses d’où il ne s’est pas connecté, et au plus 100 par jour depuis une adresse sur l’ensemble des comptes. Au-delà, Central répond qu’il y a eu trop de tentatives, jusqu’à la fin de l’heure ou du jour. Un mot de passe courant est donc quand même trouvé. Et quiconque connaît votre nom de connexion peut épuiser les tentatives de votre compte : jusqu’à la fin du jour, vous ne vous connectez alors que depuis une adresse depuis laquelle votre compte s’est connecté au cours des 30 derniers jours (Central ne retient ces adresses que jusqu’à son redémarrage).

sec-memory-eyebrow = En mémoire
sec-memory-title = Ce qui reste en mémoire après la connexion
sec-memory-lead = Le mot de passe n’est nécessaire qu’une fois : pour ouvrir votre clé maîtresse. Le schéma le
    suit dans le navigateur ; ce que le client Baylee fait différemment figure plus bas.
sec-memory-c1 = Mot de passe
sec-memory-t1 = La page confie le mot de passe à un Worker.
sec-memory-s1 = Le formulaire le transmet à un fil d’exécution d’arrière-plan qui lui est propre (un Worker),
    vide le champ de saisie et libère sa copie.
sec-memory-c2 = OPAQUE + Argon2id
sec-memory-t2 = Le Worker calcule, puis écrase.
sec-memory-s2 = Son code Rust ne garde le mot de passe que jusqu’à la fin de l’étape OPAQUE, puis l’écrase, de
    même que les clés qu’il a dérivées en chemin et qu’il détient lui-même, et les 64 MiB de mémoire
    de travail d’Argon2id.
sec-memory-c3 = Clé maîtresse : vérifiée, effacée
sec-memory-t3 = La clé maîtresse est ouverte, utilisée et effacée.
sec-memory-s3 = Le Worker l’ouvre pour vérifier la connexion, en dérive les clés de chat si votre compte a un
    chat, et l’écrase de nouveau.
sec-memory-c4 = Clés de chat, scellées
sec-memory-t4 = Seules les clés de chat restent.
sec-memory-s4 = Pour que le chat s’ouvre après un rechargement sans votre mot de passe, le Worker les stocke
    dans le navigateur (IndexedDB), chiffrées avec AES-256-GCM. La clé qui sert à cela est non
    extractible : les scripts peuvent l’utiliser mais pas la lire. Tant qu’une page de chat est
    ouverte, un Worker propre à cette page détient les clés de chat non scellées en mémoire. La
    déconnexion supprime l’enregistrement ; lorsqu’une session a pris fin d’elle-même, la prochaine
    page de ce site qui se charge le supprime.
sec-memory-c5 = Worker terminé
sec-memory-t5 = Le Worker de la connexion est terminé.
sec-memory-s5 = Quand la connexion est faite, la page de connexion met fin à son Worker, et le navigateur jette
    la mémoire de ce Worker.
sec-memory-limit = <b>Ce qu’un navigateur ne promet pas :</b> le mot de passe tel que saisi, sa transmission au
    Worker et les chaînes de JavaScript sont gérés par le navigateur. Baylee les libère ; il
    n’écrase pas les copies propres de la page et ne peut pas écraser celles du navigateur. Le
    moment où cette mémoire est réellement réutilisée est décidé par le navigateur. Et quiconque
    peut exécuter un script sur ce site ou lire votre profil de navigateur pendant que vous êtes
    connecté peut accéder aux clés de chat scellées.
sec-memory-client = <b>Dans le client Baylee,</b> le mot de passe saisi se trouve dans une mémoire qui est écrasée
    lorsqu’elle est libérée, et il est abandonné juste après la connexion ; il n’est jamais stocké.
    La clé maîtresse reste dans la mémoire du client tant que vous êtes connecté, car le client
    ouvre vos données chiffrées avec elle.

sec-chat-eyebrow = Chat
sec-chat-title = Les messages sont chiffrés sur votre appareil
sec-chat-lead = Chaque message reçoit sa propre clé. Central ne stocke et ne transmet que ce qui est chiffré ;
    le lire demande la clé de chat d’un compte pour lequel il a été enveloppé.
sec-chat-c1 = Texte + clé à usage unique
sec-chat-t1 = Une nouvelle clé pour chaque message.
sec-chat-s1 = Votre appareil tire une clé de message aléatoire et y chiffre le texte (XChaCha20-Poly1305).
sec-chat-c2 = Enveloppé par clé de chat, signé
sec-chat-t2 = Enveloppé pour la clé de chat de chaque destinataire, signé avec la vôtre.
sec-chat-s2 = Il enveloppe la clé de message séparément pour chaque clé de chat que le compte du destinataire
    a enregistrée (X25519) et signe l’en-tête et le texte chiffré avec votre propre clé de chat
    (Ed25519). Aujourd’hui, un compte a une seule clé de chat, dérivée de sa clé maîtresse et
    identique dans chaque navigateur et client où il est connecté.
sec-chat-c3 = Chiffré + enveloppes
sec-chat-t3 = Central ne reçoit que du texte chiffré.
sec-chat-s3 = Central vérifie la signature et la liste des destinataires et stocke le texte chiffré avec les
    enveloppes. Il n’a ni les clés de message ni les clés de votre compte.
sec-chat-c4 = Chiffré + une enveloppe
sec-chat-t4 = Chaque clé de chat a sa propre enveloppe.
sec-chat-s4 = Quand votre contact récupère ses messages, Central remet le texte chiffré et seulement
    l’enveloppe faite pour cette clé de chat.
sec-chat-c5 = Vérifié, ouvert
sec-chat-t5 = Vérifier, ouvrir l’enveloppe, lire.
sec-chat-s5 = L’appareil de votre contact vérifie votre signature, ouvre l’enveloppe de la clé de message avec
    sa clé de chat et déchiffre le texte.
sec-chat-meta = <b>Ce que Central voit malgré tout :</b> qui est en contact avec qui, les groupes avec leurs
    noms et leurs membres, les noms d’affichage, les clés de chat publiques, et pour chaque message
    qui l’a envoyé, dans quelle conversation, quand, et sa taille. Comme tout serveur, il voit aussi
    l’adresse d’où vient une requête.
sec-chat-limit = <b>Limites :</b> il n’y a pas de double ratchet et donc pas de confidentialité persistante : si
    la clé de chat d’un destinataire est volée plus tard, elle ouvre aussi les messages antérieurs
    qui avaient été enveloppés pour elle. Votre clé de chat est dérivée de la clé maîtresse de votre
    compte, et Central conserve les messages chiffrés après la remise : quiconque prend la base de
    données de Central avec ses clés et devine votre mot de passe, ou possède votre code de
    récupération, peut les lire. Lors du premier contact, votre appareil fait confiance aux clés que
    Central désigne ; pour les vérifier, comparez le numéro de sécurité par un autre canal. Si les
    clés d’un contact changent plus tard, Baylee n’envoie de nouveau qu’après que vous les avez
    confirmées. L’appartenance à un groupe est décidée par Central.

sec-pair-eyebrow = Connexion par lien
sec-pair-title = Du navigateur au client sans retaper le mot de passe
sec-pair-lead = Une personne connectée sur le site web devrait pouvoir lancer le client Baylee déjà connecté.
    Pour cela, la page transmet la clé maîtresse au client : scellée, et de telle sorte que Central
    ne puisse ni l’ouvrir ni en glisser une de son cru.
sec-pair-status = <b>Où en est-on :</b> Central et la bibliothèque cliente de Baylee savent le faire aujourd’hui
    et ont été testés l’un contre l’autre. Cela ne peut pas encore être utilisé depuis ce site web
    ni depuis un client publié.
sec-pair-c1 = Lancer l’appairage
sec-pair-t1 = La page lance un appairage.
sec-pair-s1 = Central lui remet un identifiant valable cinq minutes. En même temps, la page tire un secret à
    usage unique qu’elle ne montre jamais à Central.
sec-pair-c2 = Lien avec le secret
sec-pair-t2 = Un lien ouvre le client.
sec-pair-s2 = Le lien <code>baylee://pair</code> porte l’identifiant et le secret de la page jusqu’au client,
    sur votre propre appareil, sans passer par Central.
sec-pair-c3 = Clé publique
sec-pair-t3 = Le client répond avec une paire de clés neuve.
sec-pair-s3 = Il envoie à Central la moitié publique et une valeur de contrôle calculée avec le secret
    (HMAC-SHA-256). La page vérifie cette valeur ; les deux écrans affichent le même court code, et
    vous approuvez sur la page.
sec-pair-c4 = Clé maîtresse, scellée
sec-pair-t4 = La page scelle la clé maîtresse pour le client.
sec-pair-s4 = Avec HPKE (RFC 9180), vers la clé publique du client et aussi sous le secret. Central stocke le
    sceau une fois ; il ne peut pas l’ouvrir et, ne connaissant pas le secret, ne peut pas en
    fabriquer un de son cru.
sec-pair-c5 = Session + sceau
sec-pair-t5 = Le client l’utilise une seule fois.
sec-pair-s5 = Il reçoit une session avec le sceau, ouvre le sceau avec sa clé privée et le secret, et est
    connecté. L’appairage est supprimé ensuite, y compris lorsque l’utilisation échoue.
sec-pair-note = <b>Le mot de passe ne joue aucun rôle dans cet échange.</b> La page n’a la clé maîtresse qu’au
    moment où elle vient de l’ouvrir avec votre mot de passe. Une session appairée ainsi ne compte
    pas comme plus récente que la connexion par mot de passe sur la page d’où elle vient :
    l’appairage ne la renouvelle jamais.

sec-limits-eyebrow = Limites
sec-limits-title = Ce que rien de tout cela ne peut faire
sec-limit-device = <b>Un appareil compromis.</b> Un logiciel malveillant, une extension de navigateur qui lit les
    pages ou un client altéré voit ce que vous voyez et tapez. Aucun chiffrement en chemin vers un
    serveur n’aide contre cela.
sec-limit-site = <b>Le code d’un site web vient de son serveur.</b> Quiconque prend le contrôle de notre serveur
    pourrait livrer un code qui capte votre mot de passe pendant que vous le tapez ; les empreintes
    d’intégrité et une Content Security Policy stricte protègent contre des modifications venues de
    l’extérieur, pas contre cela. Le client est un programme sur votre machine et vérifie une
    signature avant d’installer une mise à jour.
sec-limit-central = <b>Un Central dont on a pris le contrôle.</b> Il pourrait révéler des métadonnées, refuser le
    service et attaquer les connexions. Il pourrait désigner de fausses clés lors d’un premier
    contact ou ajouter un membre à un groupe, et ainsi lire les messages envoyés après cela. Ce
    qu’il stocke de vos données et de vos chats est chiffré ; l’ouvrir demande votre mot de passe ou
    votre code de récupération.
sec-limit-recovery = <b>Des clés perdues.</b> Sans votre mot de passe et sans votre code de récupération, personne ne
    peut récupérer vos données chiffrées, pas même nous : Central n’a que les enveloppes.
sec-limit-audit = <b>Pas d’audit indépendant.</b> La connexion suit OPAQUE (RFC 9807) avec Argon2id (RFC 9106), et
    l’appairage scelle avec HPKE (RFC 9180) : des spécifications publiées de l’IRTF, pas des normes
    de l’IETF. L’ordre des étapes de l’appairage et le format du chat sont des combinaisons propres
    à Baylee de briques standard (Ed25519, X25519, XChaCha20-Poly1305). Jusqu’ici, seules les
    personnes qui l’ont construite ont examiné l’implémentation. Le code est ouvert ; si vous
    trouvez quelque chose, <contact>dites-le-nous</contact>.
sec-docs-title = Pour aller plus loin
sec-docs = Les détails se trouvent dans les documents du projet : <protocol>protocole
    cryptographique</protocol>, <chat>chat</chat>, <accounts>comptes et appairage</accounts> et
    <threats>modèle de menaces</threats>. Ce qu’il advient de vos données sur le plan juridique
    figure dans la <privacy>politique de confidentialité</privacy>.

# The page's two links from elsewhere: the FAQ and the sign-in page.
faq-security-q = Comment Baylee traite-t-il mon mot de passe et mes chats ?
faq-security-a = Quand vous vous connectez, votre appareil utilise le protocole OPAQUE pour prouver qu’il connaît
    le mot de passe sans l’envoyer ; les messages de chat sont chiffrés sur votre appareil. La
    <security>page Sécurité</security> montre chaque étape sous forme de schéma, avec l’unique
    exception et les limites.
sec-signin-link = Ce qu’il advient de votre mot de passe à la connexion : <security>Sécurité</security>
sec-teaser-lead = Pas à pas : ce qui quitte votre appareil, ce que Baylee Central voit et ce qu’il ne voit pas, et
    les limites de chaque flux.
sec-teaser-more = Voir chaque étape dans un schéma
# ==== end of the security page =================================================
# ==== registration: the proof of work, while it takes long =====================
register-solving = Votre navigateur résout une énigme qui tient les robots à l’écart. Quand beaucoup de comptes sont en cours de création, cela peut prendre quelques minutes … { $percent } %

## An administrator makes a reset code (/account/admin)
# English for now: to be translated.
admin-reset-title = Reset a login
admin-reset-lead = Find the account of the person who asked you, and make a reset code for it. With the code they set a new password on the reset page.
admin-find = Find the account
admin-only = This page is only for administrators.
admin-search = Search accounts
admin-search-hint = Part of a sign-in name or a display name, or # and a tag. Leave it empty to list every account, newest first.
admin-search-submit = Search
admin-search-error = A search is 1 to 64 plain characters without accents, or # and a tag such as #1a2b.
admin-count = Accounts shown: { $count }
admin-none = No account matches this search.
admin-more = Show more
admin-open = Open
admin-back = Back
admin-pill = Administrator
admin-state = State
admin-state-pending = Reserved, never set up
admin-state-enrolled = Set up
admin-state-restored = In use
admin-created = Created
admin-last-sign-in = Last sign-in
admin-recovery = Recovery code
admin-yes = Yes
admin-no = No
admin-reset-outstanding = An unused reset code is valid until { $time }. Whether an administrator made it or it went by e-mail, Central does not say. A new code replaces it.
admin-reset = Reset login
admin-refused-admin = This account holds the administrator role, and Central makes no reset code for an administrator's account. Remove the role first.
admin-refused-own = This is your own account, and nobody makes a reset code for their own. Change your password under Security instead.
admin-refused-pending = This account was reserved and never set up, so it has no sign-in to reset.
admin-refused-moving = This account's move from the official gateway is not finished. Until it is, its sign-in is reset there, not here.
admin-refused-stale = Baylee Central counts your sign-in as older than 30 minutes. Try again; you enter your password once more.
admin-refused-role = Your account no longer holds the administrator role.
admin-refused-gone = This account no longer exists.
admin-switched = This browser is signed in as another account now, so no code was asked for. Load the page again.
admin-lost = No answer came back, so we cannot tell whether a code was made. If one was, nobody can show it again, and it already replaced any earlier code. Make a new one: it replaces that one in turn.
admin-warning-title = Before you make a code
admin-warning-who = Check outside this site that the person asking really holds this account, for example by a call or on a channel you already know them by. Whoever has the code can set a new password.
admin-warning-ends = Making a code ends nothing: the old password keeps working until the code is used, and every device stays signed in.
admin-warning-data = The person's encrypted data survives the reset only with their recovery code or on a device that is still signed in. Without either, they start with an empty vault.
admin-warning-replaces = A new code replaces any earlier one, one sent by e-mail included. It is valid for 24 hours and works once.
admin-warning-takeover = A reset code answers a forgotten password, not a taken-over account: an intruder's sessions and the old password stay until the code is used, and for good if it never is.
admin-warning-continue = I have checked who is asking
admin-password-hint = Making a code needs a sign-in less than 30 minutes old, so you enter your password once more. It stays in this browser.
admin-password-missing = Enter your password.
admin-make = Make the reset code
admin-code-title = The reset code
admin-code-until = Valid until { $time }, and only once.
admin-code-lead = Hand it to the person who asked, on a channel you trust. It is shown only now: neither Baylee Central nor this page can show it again. If it gets lost, make a new one.
admin-code-done = Done
