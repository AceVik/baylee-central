# baylee.app's words in English, Project Fluent's format. English decides
# which messages exist and which arguments each takes; build.rs turns them
# into typed functions (crates/central-web/build.rs). The reader is "you";
# a button says what it does.

## The frame of every page

site-name = Baylee
skip = Vai al contenuto
# Read out after a link that leaves the site (the stylesheet carries it).
new-tab = si apre in una nuova scheda
nav-label = Principale
nav-download = Scarica
nav-gateways = Gateway
nav-faq = Domande
nav-contact = Contatti
nav-sign-in = Accedi
nav-account = Il tuo account
lang-label = Lingua
theme-label = Combinazione di colori
theme-light = Colori: chiari
theme-dark = Colori: scuri

foot-label = Informazioni sul sito
foot-play = Gioca
foot-legal = Note legali
foot-releases = Tutte le versioni
foot-issues = Segnala un bug
foot-discord = Community su Discord
foot-central-source = Il codice di questo sito
foot-dedication = Per Baylee. Per sempre sotto il sole.
fan-content = Baylee è un contenuto amatoriale non ufficiale consentito dalle <policy>Linee guida sui contenuti amatoriali</policy>. Non è approvato né autorizzato da Wizards. Parte dei materiali utilizzati è proprietà di Wizards of the Coast. © Wizards of the Coast LLC.
foot-colophon = Caratteri: Literata e Inter, SIL Open Font License 1.1, serviti da questo sito. Il dipinto e il logo sono nostri. <notices>Avvisi</notices>

## Titles and descriptions

title-home = Baylee – Costruisci i tuoi mazzi. Riunisci i tuoi amici. Gioca e basta.
desc-home = Baylee è un gioco amatoriale gratuito e open source per Magic: The Gathering su Linux, Windows e macOS: un motore di regole che conosce le carte, tavoli fino a otto giocatori e avversari quando non c'è nessun altro.
title-download = Scarica Baylee
desc-download = Scarica il client di Baylee per Windows, macOS e Linux: installer e archivi dell'ultima versione, con checksum e firme.
title-gateways = Gateway: dove si gioca online con Baylee
desc-gateways = L'elenco pubblico dei gateway di Baylee, i server dove si trovano i tavoli online, e come gestirne uno tuo.
title-faq = Domande e risposte
desc-faq = Che cos'è Baylee, quanto costa, come funzionano il gioco online e gli account, e che cosa succede ai tuoi dati.
title-contact = Contatti
desc-contact = Scrivi alla persona che sta dietro a Baylee, o segnala un bug.
title-impressum = Note legali
desc-impressum = Chi gestisce baylee.app (§ 5 DDG).
title-privacy = Informativa sulla privacy
desc-privacy = Quali dati su di te trattano baylee.app e Baylee Central, perché, su quale base giuridica e per quanto tempo.
title-terms = Condizioni d'uso
desc-terms = Le condizioni d'uso di Baylee Central, il servizio di account dietro baylee.app.
title-fan-content = Contenuto amatoriale non ufficiale
desc-fan-content = Baylee è un progetto amatoriale nel quadro delle Linee guida sui contenuti amatoriali di Wizards of the Coast: che cosa significa e che cosa no.
title-not-found = Pagina non trovata
title-gone = Questa pagina non esiste più
crumbs-label = Sei qui
breadcrumb-home = Home

## Home

hero-title = <nb>Costruisci i tuoi mazzi.</nb> <nb>Riunisci gli amici.</nb> <glow>Gioca e
    basta.</glow>
hero-lead = Un deck builder completo, partite fino a otto giocatori in duelli, tutti contro tutti o a squadre, e un motore di regole che controlla ogni passaggio, così ogni turno resta equo senza che nessuno debba cercare nulla.
hero-offline = Le partite in solitaria e il deck builder funzionano offline, senza account. I tavoli online si trovano sui gateway; <host>puoi gestirne uno tuo</host>.
hero-account = Giochi online? <register>Crea un account</register>, gratuito e aperto a tutti durante la beta aperta, oppure <signin>accedi</signin>.
hero-memorial = Ricorda Baylee: apri il suo album di foto

status-label = Baylee in numeri

cards-eyebrow = Le carte
cards-title = Quante carte conosce Baylee
cards-lead = Baylee implementa le carte di Magic una per una, ognuna controllata sul testo stampato. Questi sono i numeri dell'ultima versione.
cards-implemented = Implementate del tutto
cards-implemented-help = Ogni regola della carta funziona ed è testata.
cards-generated = Generate
cards-generated-help = Implementate dal generatore di Baylee a partire dal testo di regole della carta, ma non ancora testate a mano. Sono giocabili; in rari casi limite una può comportarsi in modo diverso.
cards-partial = Implementate in parte
cards-partial-help = La carta è giocabile, ma manca ancora una delle sue abilità o un caso limite delle sue regole.
cards-total = Carte in totale
cards-total-help = Tutte le carte di Magic, come le elenca Scryfall; i tre numeri precedenti sono quelle che Baylee sa giocare.
cards-help = Che cosa significa
cards-share = { $playable } carte su { $total } sono giocabili oggi.
cards-as-of = Versione { $tag }, conteggio del { $date }.
cards-missing = Al momento non è stato possibile leggere i numeri delle carte.

shot-duel-alt = Un duello contro l'IA della casa: sei creature attaccano, tre vengono bloccate
shot-duel = <b>Duello tattico</b> contro l'IA della casa, con linee di combattimento chiare per ogni assegnazione.
shot-log-alt = Un duello con il registro di gioco aperto, tre magie sulla pila
shot-log = <b>Il registro di gioco</b> annota ogni passaggio come una chat; il nome di una carta apre la sua anteprima.
shot-ring-alt = Un tavolo da quattro posti disposti ad anello
shot-ring = <b>Tavoli fino a otto posti</b> che si dispongono da soli attorno al campo di battaglia.
shot-lobby-alt = La lobby con il messaggio di un amico e il suo invito al tavolo
shot-lobby = <b>La lobby:</b> tavoli aperti, amici, inviti e una chat con crittografia end-to-end.
shot-room-alt = Una stanza in cui tre sedie vuote vanno all'IA della casa
shot-room = <b>Un posto per tutti:</b> persone, l'IA della casa in cinque livelli, o un modello linguistico.
shot-builder-alt = Il deck builder che filtra le carte disponibili per trovare un angelo
shot-builder = <b>Il deck builder</b> cerca in tutte le carte mentre scrivi.

about-eyebrow = Che cos'è
about-title = Motore di regole completo e tavolo 3D chiaro
about-engine = Priorità, pila, livelli, effetti di sostituzione, abilità innescate, azioni basate sullo stato e combattimento: il motore in Rust applica il regolamento completo e calcola le scelte legali esatte a ogni passaggio. Il client mostra solo quelle azioni valide.
about-table = Campo di battaglia, mani, pila, esilio e cimiteri su un tavolo 3D ordinato, con linee di combattimento chiare e anteprime immediate delle carte. Le illustrazioni delle carte arrivano in streaming da Scryfall mentre giochi, oppure puoi passare in qualsiasi momento a una nitida vista con il testo Oracle.

gw-eyebrow = Online
gw-title = Gateway

open-central = Baylee Central su GitHub

## Download

dl-prerelease = Beta
dl-for = Scarica per { $system }
dl-size = { $size } MB
dl-checksum = SHA-256
dl-signature = Firma
dl-notes = Note di rilascio
dl-unknown = Non siamo riusciti a riconoscere il tuo sistema da questo browser; scegli il tuo qui sotto.
dl-mobile = Non esiste ancora una versione per telefoni e tablet. Baylee funziona su un computer.
dl-mac-unsure = Questo browser non dice se il tuo Mac ha un chip Apple o uno Intel. Il file qui sopra è per i chip Apple (M1 e successivi); il file per i Mac Intel è qui sotto.
dl-windows-unsure = Su un PC Windows con processore ARM (per esempio un portatile Snapdragon), scegli il file ARM qui sotto.
dl-linux-unsure = Il tuo browser non indica il processore; si presume x86-64. Il file ARM è qui sotto.
dl-missing = Questa versione non ha un file per il tuo sistema. Le versioni precedenti sono nella <releases>pagina delle versioni</releases>.
dl-none = GitHub non ha risposto al momento. Tutte le versioni sono nella <releases>pagina delle versioni</releases>.
dl-yours = Il tuo sistema
dl-more-files = Altri file ({ $count })
dl-installer = Installer
dl-deb = Pacchetto Debian
dl-archive = Archivio
dl-fallback = Archivio per processori meno recenti
dl-fallback-help = Per processori x86-64 senza AVX2 (precedenti al 2013 circa). Le build principali di Baylee richiedono AVX2 e non si avviano senza.
plat-windows-x64 = Windows
plat-windows-x64-sub = la maggior parte dei PC: Intel o AMD a 64 bit
plat-windows-arm64 = Windows su ARM
plat-windows-arm64-sub = portatili Snapdragon e simili
plat-mac-arm64 = macOS
plat-mac-arm64-sub = chip Apple: M1 e successivi
plat-mac-x64 = macOS su Intel
plat-mac-x64-sub = Mac con processore Intel
plat-linux-x64 = Linux
plat-linux-x64-sub = x86-64
plat-linux-arm64 = Linux su ARM
plat-linux-arm64-sub = ARM64 / aarch64
plat-none = Nessun file per questo sistema in questa versione.
install-title = Primo avvio e requisiti
install-first-title = Primo avvio
install-first = Le build non sono firmate digitalmente, perché la firma costa denaro, quindi Windows e macOS mostrano un avviso la prima volta. Su Windows, scegli <i>Ulteriori informazioni</i> e poi <i>Esegui comunque</i>. Su macOS, apri <i>Impostazioni di Sistema → Privacy e sicurezza</i> e scegli <i>Apri comunque</i>.
install-check-title = Verificare un download
install-check = Accanto a ogni file c'è un <code>.sha256</code> con il suo checksum; gli archivi hanno anche un <code>.sig</code>, la firma Ed25519 che il client controlla prima di aggiornarsi da solo. Su Linux, <code>sha256sum -c</code> verifica un file; su macOS <code>shasum -a 256</code>; su Windows <code>certutil -hashfile</code>.
install-needs-title = Requisiti
install-windows = <b>Windows</b> 10 o 11, x64 o ARM64.
install-mac = <b>macOS</b> 11 o successivo, su chip Apple o Intel.
install-linux = <b>Linux</b> con glibc 2.35 o più recente (Ubuntu 22.04 e successive) e un driver Vulkan; il <code>.deb</code> installa tutto il resto che serve.
install-cpu = <b>Processore:</b> le build x86-64 richiedono AVX2; per i processori meno recenti c'è un archivio a parte.

## Gateways

gws-title = Gateway
gws-online = Online
gws-offline = Offline
gws-version = Versione
gws-now = Adesso
gws-website = Sito web
gws-address = Indirizzo
gws-copy = Copia indirizzo
gws-copied = Indirizzo copiato.
gws-copied-short = Copiato
gws-copy-failed = La copia non è riuscita.
gws-failed = Al momento non è stato possibile leggere l'elenco. Riprova tra un minuto.

## Questions

faq-eyebrow = Domande
faq-title = Domande e risposte
faq-what-q = Che cos'è Baylee?
faq-what-a = Un gioco amatoriale gratuito di Magic: The Gathering per il tuo computer. Ha un deck builder, un motore di regole che controlla ogni passaggio, un tavolo fino a otto giocatori e avversari controllati dal computer. Non è creato, approvato né autorizzato da Wizards of the Coast.
faq-cost-q = Quanto costa?
faq-cost-a = Niente, e non cambierà: nessun accesso a pagamento, nessuna funzione a pagamento, nessuna pubblicità.
faq-account-q = Mi serve un account?
faq-account-a = Non per giocare da solo: il deck builder e le partite contro l'IA della casa funzionano offline. Per i tavoli online serve un account Baylee; durante la beta aperta chiunque può crearne uno su questo sito, gratuitamente.
faq-data-q = Che cosa succede ai miei dati?
faq-data-a = Il meno possibile; l'<privacy>informativa sulla privacy</privacy> dice esattamente che cosa. La tua password non lascia mai il tuo browser; i tuoi dati privati e i tuoi messaggi in chat vengono cifrati sul tuo dispositivo prima di arrivare a noi.
faq-password-q = Ho dimenticato la password. E ora?
faq-password-a = Imposti una nuova password con un codice di reimpostazione, <reset>in questa pagina</reset>. Se il tuo account ha un indirizzo e-mail confermato, puoi farti mandare lì il codice; altrimenti te ne dà uno un amministratore. I tuoi dati cifrati si salvano solo se hai il codice di recupero ricevuto quando hai creato il tuo account. Senza, riparti con una cassaforte vuota, e quei dati sono persi.
faq-gateway-q = Che cos'è un gateway?
faq-gateway-a = Un server dove si trovano i tavoli online. Chiunque può gestirne uno; l'<gateways>elenco</gateways> riporta quelli che Central conosce.
faq-systems-q = Su quali sistemi funziona?
faq-systems-a = Windows, macOS e Linux, su x86-64 e ARM64. Non esiste ancora una versione per telefoni o tablet.
faq-open-q = È open source?
faq-open-a = Il gioco lo è, sotto la GNU Affero General Public License. Il codice di Baylee Central è pubblico, così che chiunque possa verificare come tratta i suoi dati.
faq-bug-q = Ho trovato un bug. Dove lo segnalo?
faq-bug-a = Nel gioco, premi <kbd>F8</kbd> per inviare una segnalazione con tutto il necessario. Altrimenti sul tracker delle issue di GitHub, su Discord o tramite il <contact>modulo di contatto</contact>.

## Contact

contact-eyebrow = Contatti
contact-title = Scrivici
contact-name = Il tuo nome
contact-name-hint = Facoltativo.
contact-email = Il tuo indirizzo e-mail
contact-email-hint = Facoltativo; solo per poterti rispondere.
contact-message = Il tuo messaggio
contact-count = { $n } di 5.000 caratteri
contact-send = Invia messaggio
contact-sending = Invio in corso …
contact-sent-title = Grazie
contact-sent = Il tuo messaggio è arrivato. Se hai indicato un indirizzo, risponderemo lì.
contact-error-empty = Scrivi prima un messaggio.
contact-error-email = Questo non sembra un indirizzo e-mail.
contact-error-busy = Troppi messaggi dalla tua rete al momento. Riprova tra un'ora.
contact-error-refused = Il messaggio non è stato accettato. Controlla la lunghezza e riprova.
contact-error-network = Non è stato possibile inviare il messaggio. Controlla la connessione e riprova.
contact-privacy = Quanto invii viene conservato per poterti rispondere, e cancellato 90 giorni dopo che l'abbiamo gestito, o al più tardi dopo un anno. Altro nell'<privacy>informativa sulla privacy</privacy>.
contact-noscript = Il modulo richiede JavaScript. Puoi anche raggiungerci su Discord o su GitHub.

## Legal notice

impressum-title = Note legali
impressum-lead = Informazioni richieste dal § 5 DDG (la legge tedesca sui servizi digitali). Baylee è un progetto privato e non commerciale.
impressum-show = Mostra i dettagli
impressum-why = I dettagli sono protetti da un breve rompicapo che il tuo browser risolve in pochi secondi, per tenere lontani i robot che raccolgono indirizzi. Non ti viene chiesto altro.
impressum-solving = Risoluzione del rompicapo … { $percent } %
impressum-name = Nome
impressum-address = Indirizzo
impressum-email = E-mail
impressum-phone = Telefono
impressum-write = Scrivi un'e-mail
impressum-call = Chiama
impressum-failed = Non è stato possibile caricare i dettagli. Riprova, oppure scrivici tramite il <contact>modulo di contatto</contact>.
impressum-noscript = Per mostrare i dettagli serve JavaScript. Senza, raggiungici tramite Discord o GitHub.
impressum-responsible = Responsabile dei contenuti: la persona indicata sopra.
impressum-dispute = Non siamo obbligati né disposti a partecipare a procedimenti di risoluzione delle controversie davanti a un organismo di conciliazione dei consumatori.

## Fan content

fc-title = Contenuto amatoriale non ufficiale
fc-eyebrow = Contenuto amatoriale
fc-lead = Baylee è un progetto amatoriale. È consentito dalle Linee guida sui contenuti amatoriali di Wizards of the Coast, che stabiliscono le regole che rispetta.
fc-notice-title = L'avviso su ogni pagina
fc-means-title = Che cosa significa
fc-means = Baylee non è creato, approvato né autorizzato da Wizards of the Coast. Magic: The Gathering, i nomi delle sue carte e delle sue espansioni, le immagini delle carte e i simboli di mana sono proprietà di Wizards of the Coast LLC. Baylee è gratuito per tutti: non c'è nessun accesso a pagamento, nessuna funzione a pagamento e nessuna registrazione tramite e-mail.
fc-images = Il gioco mostra immagini di carte provenienti da Scryfall, non modificate e con il nome dell'artista; questo sito non ne mostra. I dati e le immagini delle carte sono forniti da Scryfall; Baylee non è affiliato a Scryfall.
fc-policy = <policy>Linee guida sui contenuti amatoriali</policy> di Wizards of the Coast

## Privacy notice

privacy-title = Informativa sulla privacy
privacy-lead = Quali dati su di te trattano questo sito e Baylee Central, perché, su quale base giuridica e per quanto tempo.
privacy-updated = Aggiornata al 12 novembre 2026
privacy-controller-title = Chi è responsabile
privacy-controller = Il titolare del trattamento ai sensi del GDPR (Regolamento generale sulla protezione dei dati) è la persona fisica privata indicata nelle <impressum>note legali</impressum>, che riportano il suo nome, il suo indirizzo postale e il suo indirizzo e-mail. Puoi inviare domande sulla privacy anche tramite il modulo di contatto. Non c'è un responsabile della protezione dei dati; non è richiesto.
privacy-short-title = In breve
privacy-short = Nessun tracciamento, nessuna pubblicità, nessuna analisi statistica, nessun cookie a meno che tu non acceda, e nulla caricato da altri server: caratteri, immagini e script provengono tutti da baylee.app. La tua password non ci raggiunge mai, e i tuoi messaggi in chat e i tuoi dati privati vengono cifrati nel tuo browser prima di essere inviati.
privacy-hosting-title = Visita al sito
privacy-hosting = Il sito gira su un server che prendiamo in affitto da netcup GmbH in Germania. Quando apri una pagina, il tuo browser invia ciò che porta con sé ogni richiesta: il tuo indirizzo IP, l'indirizzo richiesto, l'ora, la lingua del browser e la sua identificazione (user agent). Il server ne ha bisogno per rispondere; sceglie la versione linguistica in base alla lingua, e la pagina di download usa lo user agent per suggerire il file giusto. Il server web non conserva alcun log di accesso, quindi queste richieste non vengono memorizzate. I messaggi di funzionamento del server stesso (il suo journal di sistema) possono, in rari casi di errore, contenere un indirizzo IP; vengono cancellati al più tardi dopo 14 giorni. Base giuridica: art. 6, par. 1, lett. f) del GDPR; il nostro legittimo interesse è fornire il sito in modo sicuro e affidabile.
privacy-views-title = Il conteggio delle visualizzazioni di pagina
privacy-views = Il sito conta quante volte vengono aperte le sue pagine: un solo numero per tutte le pagine e tutti i visitatori insieme, mostrato nella pagina iniziale. Una pagina aperta in un browser lo aumenta di uno. Per questo non viene memorizzato nulla su di te: né indirizzo IP, né cookie, né identificatore, nemmeno quale pagina fosse.
privacy-device-title = Che cosa conserva il tuo browser
privacy-device = Questo sito non imposta alcun cookie finché non accedi. Ciò che il tuo browser conserva sul tuo dispositivo, e solo con JavaScript: la tua scelta dei colori, se ne hai fatta una (<code>baylee-site-theme</code>); finché sei connesso, quando termina la tua sessione (<code>baylee-central-session</code>), la tua chiave di chat e le chiavi fidate dei tuoi contatti (vedi Chat); se hai attivato le notifiche, un numero casuale per questo dispositivo (<code>baylee-push-installation-v1</code>, <code>baylee-push</code>); e, solo dopo che hai effettuato l'accesso, installato il sito come app o attivato le notifiche, una copia dei file pubblici del sito (guscio della pagina, caratteri, script), così che si avvii senza connessione e le notifiche possano arrivare. Se nessuna delle tre condizioni vale più, la copia viene rimossa alla successiva apertura del sito. Nulla di tutto questo ci viene inviato, a meno che una sezione qui sotto non lo dica. Le voci della chat e della sessione vengono cancellate quando esci; tutto il resto lo rimuovi cancellando i dati del sito nel tuo browser. Base giuridica: § 25, comma 2, n. 2 TDDDG (strettamente necessario per la funzione che usi); per il numero casuale delle notifiche, il tuo consenso (§ 25, comma 1 TDDDG).
privacy-device-manager = Se conservi il tuo codice di recupero nel gestore di password del browser, te lo mandi per e-mail, lo salvi come file o lo stampi nel passaggio che lo mostra, a conservarlo sono il tuo browser, il tuo programma di posta, i tuoi file o la tua carta, mai questo sito: il codice nasce nel tuo browser e non ci viene mai inviato. Cancellare i dati del sito non lo rimuove da lì.
privacy-github-title = GitHub
privacy-github = La pagina di download e la pagina iniziale mostrano l'ultima versione, lo stato dei test e i numeri delle carte. È il nostro server a chiederli a GitHub, non il tuo browser; GitHub non apprende nulla su di te da questo. Solo quando fai clic su un download o su un link di GitHub il tuo browser si collega a GitHub (GitHub, Inc., USA) e gli invia il tuo indirizzo IP e ciò che porta con sé ogni richiesta; si applica allora l'informativa sulla privacy di GitHub.
privacy-directory-title = L'elenco dei gateway
privacy-directory = Il nostro server legge l'elenco dei gateway da Baylee Central e lo mostra; non è coinvolto nulla che ti riguardi. Un gateway può comunicare a Central quanti giocatori sono connessi al momento, quanti di loro sono in una partita e quanti tavoli aperti e partite ha, e Central conta i propri account; il sito mostra questi numeri. Sono solo conteggi, mai nomi, e i numeri di un gateway sono una dichiarazione del suo gestore.
privacy-account-title = Il tuo account
privacy-account = Per un account, Baylee Central memorizza: l'ID del tuo account, il tuo nome di accesso, il tuo nome visualizzato con il suo tag, quando e come è stato creato l'account, un record di password OPAQUE (che ti permette di dimostrare la tua password senza che il nostro server la conosca mai), la tua chiave principale cifrata due volte (dalla tua password e dal tuo codice di recupero), il documento cifrato del tuo account e i tuoi oggetti cifrati (illeggibili per noi; vediamo solo dimensione, revisione e ora), quale versione delle condizioni hai accettato e quando, l'ora del tuo ultimo accesso, le tue sessioni, un ruolo di amministratore se ne hai uno, e i gateway che registri con i loro token. Gli altri giocatori vedono il tuo nome visualizzato e il tag, mai il tuo nome di accesso. Se un amministratore ti dà un codice di reimpostazione, ne memorizziamo solo l'impronta (digest) finché il codice non viene usato o scade. Ciò che gli amministratori fanno a un account o a un gateway viene conservato in un registro per tre anni (chi, che cosa, quale account o gateway, quando). Base giuridica: art. 6, par. 1, lett. b) del GDPR (l'account che hai richiesto); per l'ora dell'ultimo accesso e il registro, art. 6, par. 1, lett. f) del GDPR (il nostro legittimo interesse a un funzionamento sicuro e tracciabile). Senza un nome di accesso e una password non c'è alcun account; non ti serve un indirizzo e-mail. Puoi creare un account a partire dai 16 anni. Conservato finché non elimini l'account; viene poi rimosso subito dal sistema in esercizio, e dai backup al più tardi entro 12 mesi. Le sessioni terminano dopo 30 minuti di inutilizzo e al più tardi 12 ore dopo l'accesso.
privacy-email-title = Indirizzo e-mail
privacy-email = Un indirizzo e-mail è facoltativo; un account non ne richiede nessuno. Se ne indichi uno, Baylee Central memorizza l'indirizzo, la lingua in cui sono scritte le sue e-mail e quando lo hai confermato. Un indirizzo che hai indicato e che attende il suo codice viene conservato finché non lo confermi o lo ritiri, e al massimo un giorno dopo la scadenza del suo codice. Di ogni codice che inviamo per e-mail memorizziamo solo l'impronta (digest), finché il codice non viene usato o scade, e per 24 ore gli orari in cui ti sono stati inviati codici di reimpostazione, così che ne partano al massimo tre al giorno. Usiamo l'indirizzo per quattro scopi: inviare il codice che lo conferma, inviare un codice di reimpostazione quando tu o chiunque conosca il tuo nome di accesso ne richiede uno, avvisare il vecchio indirizzo quando viene sostituito o rimosso, e avvisare l'indirizzo confermato quando un codice di reimpostazione è stato usato per impostare una nuova password, quando la password è stata cambiata o quando un amministratore ha creato un codice di reimpostazione per l'account. Le e-mail vengono inviate dal servizio di posta di Baylee sullo stesso server tramite mailbox.org (Heinlein Hosting GmbH, Berlino), che opera come responsabile del trattamento e riceve l'indirizzo e l'e-mail. Base giuridica: art. 6, par. 1, lett. b) del GDPR (l'account che hai richiesto). Conservato finché non rimuovi l'indirizzo o non elimini l'account.
privacy-cookie-title = Il cookie di sessione
privacy-cookie = L'accesso imposta un solo cookie, <code>__Host-central</code>: un numero casuale che identifica la tua sessione. Viene inviato solo a baylee.app, non può essere letto dagli script e termina con la sessione, al più tardi dopo 12 ore. È strettamente necessario per mantenerti connesso; non richiede consenso. Base giuridica: § 25, comma 2, n. 2 TDDDG e art. 6, par. 1, lett. b) del GDPR.
privacy-chat-title = Chat e contatti
privacy-chat = I messaggi vengono cifrati sul tuo dispositivo per i dispositivi di ciascun destinatario; Baylee Central li memorizza solo in forma cifrata e non può leggerli. Ciò che Central può vedere e memorizza: i tuoi contatti, le richieste di contatto e i blocchi, i gruppi con i loro nomi, membri e inviti, chi ha mandato un messaggio a chi, quando e quanto è grande, e le chiavi pubbliche dei tuoi dispositivi. I tuoi contatti e i membri dei tuoi gruppi vedono il tuo nome visualizzato con il suo tag e i tuoi messaggi. Il tuo browser conserva la tua chiave di chat (sigillata con una chiave che solo questo browser possiede, nel suo archivio <code>baylee-chat</code>) e le chiavi dei tuoi contatti di cui ti sei fidato (cifrate, <code>baylee-chat-trust</code>) finché sei connesso. Messaggi e consegne vengono conservati finché non elimini il tuo account; per ora non esiste un periodo di conservazione automatico più breve. Ogni mittente e ogni dispositivo conserva al massimo 5.000 messaggi; dopo, i nuovi vengono rifiutati. Base giuridica: art. 6, par. 1, lett. b) del GDPR.
privacy-push-title = Notifiche
privacy-push = Se attivi le notifiche in un browser, il tuo browser si registra presso il proprio servizio push, e Central memorizza l'indirizzo di quel servizio per questo browser, la chiave pubblica e il segreto di autenticazione del browser, la tua lingua, un numero casuale per questo dispositivo e la tua scelta delle categorie. Il servizio push appartiene al produttore del tuo browser: Google (Firebase Cloud Messaging), Mozilla o Apple, tutte con sede negli USA. Una notifica contiene, cifrati, solo il suo tipo (per esempio «nuovo messaggio») e la tua lingua, mai nomi o testo; il servizio push vede i dati di connessione come indirizzo, ora e dimensione. Questi dati vengono cancellati quando disattivi le notifiche su questo dispositivo, quando esci o quando elimini il tuo account; un indirizzo non rinnovato per 90 giorni non viene più usato, e le notifiche in attesa scadono dopo un giorno. Base giuridica: art. 6, par. 1, lett. a) del GDPR e § 25, comma 1 TDDDG (il tuo consenso). Puoi revocarlo in qualsiasi momento nelle impostazioni del tuo account; ciò che è avvenuto fino ad allora resta lecito.
privacy-counters-title = Protezione dagli abusi
privacy-counters = Per impedire che le password vengano indovinate e per evitare il sovraccarico, Central conta le richieste per indirizzo IP (registrazioni, tentativi di accesso, reimpostazioni della password, lettura dell'elenco e, più in generale, quante richieste invia un indirizzo entro un minuto), solo in memoria: le registrazioni e i tentativi di accesso non conclusi con la password giusta fino a un giorno, tutto il resto per al massimo un'ora. Dopo un accesso con la password giusta, Central ricorda per 30 giorni, di nuovo solo in memoria, che il tuo account ha effettuato l'accesso da quell'indirizzo (un indirizzo IPv4, o la prima metà di un indirizzo IPv6; al massimo otto per account), in modo che i tentativi altrui sul tuo account non ti escludano da lì. Chi usa lo stesso indirizzo e conosce il tuo nome di accesso può capire dalle risposte di Central che il tuo account ha effettuato l'accesso da lì. Una nuova password dimentica questi indirizzi, tranne quello da cui l'hai impostata; l'eliminazione dell'account li dimentica tutti, e lo stesso vale per un riavvio del servizio. Prima di una registrazione il tuo browser risolve inoltre un rompicapo che richiede memoria e tempo di calcolo; è fatto di numeri casuali, non contiene nulla su di te e viene dimenticato al più tardi dopo venti minuti. Central ne ricalcola una piccola parte per verificare la risposta del tuo browser. Quanto è difficile dipende da quanti account vengono creati e da se la richiesta sembra venire da un programma: se contiene le intestazioni che ogni browser invia, se il tuo browser dichiara che un programma lo sta controllando, e quanti rompicapi il tuo indirizzo e il blocco di rete che lo circonda (i primi tre numeri di un indirizzo IPv4, i primi 48 bit di un indirizzo IPv6) hanno richiesto nell'ultima ora. Di questo Central conserva solo i conteggi, in memoria, per al massimo un'ora. Per ogni account Central conta le richieste di contatto e le ricerche di altri giocatori, perché nessuno sommerga gli altri di richieste. Base giuridica: art. 6, par. 1, lett. f) del GDPR; il nostro legittimo interesse è proteggere gli account e il servizio dagli abusi.
privacy-contact-title = Il modulo di contatto
privacy-contact = Il modulo invia il tuo messaggio, la lingua della pagina e l'ora al servizio di feedback di Baylee sullo stesso server, e il tuo nome e il tuo indirizzo e-mail solo se li indichi; entrambi sono facoltativi. Il servizio annota anche se il messaggio è stato gestito. Il tuo indirizzo IP non viene memorizzato; viene solo contato in memoria per un'ora, per fermare un'ondata di messaggi. Solo la persona indicata nelle note legali legge i messaggi; nulla viene trasmesso ad altri. Un messaggio viene cancellato 90 giorni dopo che è stato gestito, al più tardi dopo un anno, o prima se lo chiedi. Base giuridica: art. 6, par. 1, lett. f) del GDPR (il nostro legittimo interesse a rispondere ai messaggi), oppure lett. b) se il tuo messaggio riguarda il tuo account.
privacy-impressum-title = Il rompicapo delle note legali
privacy-impressum = Le note legali vengono mostrate solo dopo che il tuo browser ha risolto un
    rompicapo; questo protegge l'indirizzo dai programmi che raccolgono indirizzi. Il rompicapo è
    composto da numeri casuali e non contiene nulla che ti riguardi. A questo scopo il servizio di
    feedback conserva, solo in memoria, mai su disco e mai in un log: quanti rompicapi ha richiesto
    il tuo indirizzo IP (per un'ora; un indirizzo IPv6 con la sua prima metà), se ha seguito un link
    nascosto alle persone (per 24 ore) e quali rompicapi sono stati risolti (per cinque minuti,
    senza indirizzo). Tre risposte sì-o-no sul tuo browser stabiliscono la difficoltà del rompicapo
    e non vengono conservate: se un programma lo pilota, se non indica alcuna lingua e se disegna
    senza scheda grafica. Nessuno viene rifiutato; un browser che sembra un programma riceve un
    rompicapo più lungo. Base giuridica: art. 6, par. 1, lett. f) del GDPR (il nostro legittimo
    interesse a proteggere l'indirizzo di una persona fisica privata).
privacy-recipients-title = Destinatari e paesi
privacy-recipients = Non vendiamo dati e non ne trasmettiamo per pubblicità. I destinatari sono: netcup GmbH (hosting in Germania, come responsabile del trattamento ai sensi dell'art. 28 del GDPR); mailbox.org (Heinlein Hosting GmbH, Berlino), che recapita le e-mail a un indirizzo e-mail che hai indicato, come responsabile del trattamento; gli amministratori di Baylee Central, che per l'amministrazione vedono il tuo nome di accesso, il tuo nome visualizzato con il suo tag, l'origine, la creazione e l'ultimo accesso del tuo account, e i tuoi gateway, mai i tuoi dati cifrati; un gateway al quale accedi con il tuo account (riceve l'ID del tuo account, il nome visualizzato e il tag, non la tua password); i tuoi contatti e i membri dei tuoi gruppi (nome visualizzato, tag e i tuoi messaggi); le autorità solo dove la legge ce lo impone. Paesi al di fuori dello Spazio economico europeo: solo quando attivi le notifiche il servizio push del tuo browser (Google, Mozilla o Apple, USA) le trasporta. I gateway gestiti da altri possono trovarsi in qualsiasi paese; accedervi o meno è una tua decisione. Per il resto non trasferiamo dati a paesi al di fuori dello Spazio economico europeo.
privacy-rights-title = I tuoi diritti
privacy-rights = Hai il diritto di accesso (art. 15 GDPR), di rettifica (art. 16), di cancellazione
    (art. 17), di limitazione del trattamento (art. 18) e di portabilità dei dati (art. 20). Puoi
    opporti in qualsiasi momento, per motivi connessi alla tua situazione particolare, al
    trattamento basato sui nostri legittimi interessi ai sensi dell'art. 6, par. 1, lett. f) (art.
    21). Puoi revocare un consenso in qualsiasi momento per il futuro (art. 7, par. 3). I dati del
    tuo account possono essere esportati e l'account eliminato nelle impostazioni del tuo account;
    per tutto il resto basta un messaggio tramite il modulo di contatto o all'indirizzo indicato
    nelle note legali. Puoi proporre reclamo a un'autorità di controllo per la protezione dei dati
    (art. 77), per esempio quella del luogo in cui vivi o il Garante regionale per la protezione dei
    dati e la libertà di informazione della Renania Settentrionale-Vestfalia. Nessuna legge o
    contratto ti obbliga a fornirci dati; senza un nome di accesso e una password, tuttavia, non c'è
    alcun account. Non esiste alcun processo decisionale automatizzato né profilazione (art. 22).
privacy-objection-title = Il tuo diritto di opposizione
privacy-objection = Quando trattiamo dati sulla base dell'art. 6, par. 1, lett. f) del GDPR (fornitura del sito, protezione dagli abusi, modulo di contatto, rompicapo delle note legali, ora del tuo ultimo accesso, registro dell'amministrazione, segnalazioni di bug e conteggio d'uso), puoi opporti in qualsiasi momento per motivi connessi alla tua situazione particolare (art. 21 del GDPR). Cesseremo allora di trattare i dati, a meno che non possiamo dimostrare motivi legittimi cogenti che prevalgano sui tuoi interessi, o il trattamento serva ad accertare, esercitare o difendere diritti in sede giudiziaria. Basta un messaggio informale tramite il modulo di contatto o all'indirizzo indicato nelle note legali.
privacy-links-title = Link ad altri servizi
privacy-links = Il sito rimanda, tra l'altro, a GitHub, Discord e WhatsApp. Sono semplici link: non è incorporato nulla, e finché non ne apri uno, nessuno di questi servizi sa della tua visita. Quando fai clic, il tuo browser non indica da quale pagina provieni (nessun referrer). Da quel momento si applica l'informativa sulla privacy di ciascun fornitore, che può trattare dati anche fuori dall'UE; ciò che gli altri membri dei gruppi della community vedono di te lì è deciso da quei servizi.
privacy-reports-title = Segnalazioni di bug e conteggio d'uso dal gioco
privacy-reports = Una segnalazione di bug che invii dal gioco con F8 contiene il suo tipo, il tuo testo, la versione del client e gli allegati che lasci selezionati; la pagina dei contatti li elenca. Viene cancellata 90 giorni dopo che è stata gestita, al più tardi dopo tre anni. Base giuridica: art. 6, par. 1, lett. f) del GDPR (il nostro legittimo interesse a un gioco senza bug). Il client invia segnalazioni di arresto anomalo solo se lo hai consentito (art. 6, par. 1, lett. a) del GDPR; puoi revocarlo in qualsiasi momento nelle impostazioni del client). Durante una partita offline il client invia una volta al minuto un identificatore casuale di quella partita e nient'altro; lo conserviamo solo in memoria e lo dimentichiamo 150 secondi dopo il suo ultimo segno di vita (art. 6, par. 1, lett. f) del GDPR; si può disattivare in «Send anonymous usage count»). Come con ogni connessione, il tuo indirizzo IP ci raggiunge.

## Terms

terms-title = Condizioni d'uso
terms-draft = Queste condizioni sono una bozza e non sono ancora in vigore.
terms-version = Versione { $version }, aggiornata il { $date }.
# Shown only in the languages a machine translated: every one but German and English.
machine-note = Questo sito è stato tradotto da una macchina e nessun madrelingua italiano lo ha ancora letto. Il gioco stesso parla per ora inglese e tedesco. <link>Segnalaci un errore.</link>
legal-machine = Questa pagina è una traduzione automatica, offerta come aiuto. Dove differisce dal testo <de>tedesco</de> o <en>inglese</en>, valgono quelli.
terms-machine = Le condizioni di Central esistono solo in tedesco e in inglese. Qui sotto trovi il testo inglese; c'è anche una <de>versione tedesca</de>.

## Errors

not-found-title = Questa pagina non esiste
not-found-lead = L'indirizzo potrebbe contenere un errore, oppure la pagina è stata spostata.
gone-title = Questa pagina non c'è più
gone-lead = Ciò che c'era qui è stato rimosso. La pagina iniziale mostra dove proseguire.
error-home = Alla pagina iniziale

## Baylee's remembrance (approved by her family on 27 October 2026; keep word for word)
baylee-dedication = Per Baylee. Per sempre sotto il sole.
baylee-invitation = Conosci la piccola anima dietro questo mondo
baylee-open = Ricorda Baylee · apri il suo album di foto
baylee-eyebrow = Il cuore di questo progetto
baylee-date = In dolce memoria · 26 ottobre 2026
baylee-close = Torna al suo mondo
baylee-previous = Fotografia precedente
baylee-next = Fotografia successiva
baylee-photo1 = Baylee rannicchiata nel morbido piumone bianco.
baylee-photo2 = Baylee che guarda fuori dal piumone a fiori.
baylee-photo3 = A riposo nel suo letto rotondo e grigio.
baylee-photo4 = Baylee addormentata, rannicchiata vicino a noi.
baylee-photo5 = Un momento di quiete, accoccolata nel piumone.

baylee-storyTitle = Per Baylee. Per sempre sotto il sole.
baylee-together = Con noi · dal 17 novembre 2025 al 26 ottobre 2026
baylee-chapters = La sua storia
baylee-chapterPrevious = Capitolo precedente
baylee-chapterNext = Capitolo successivo
baylee-photos = I nostri ricordi
baylee-read = Scorri per leggere

baylee-chapter1Title = Una casa per Baylee
baylee-chapter1Body =
    Baylee è arrivata a casa da noi il 17 novembre 2025.

    Mia moglie e io l'avevamo adottata dal rifugio per animali di Krefeld. Si stimava avesse circa tredici anni. Aveva tumori mammari maligni, e il cancro si era già diffuso al fegato. Baylee era una gatta in cure palliative. Sapevamo che il nostro tempo insieme sarebbe stato limitato. Con noi, avrebbe avuto una casa in cui essere amata.

    Ben presto ci mostrò con quanta rapidità avrebbe fatto sua quella casa.

    Il primo giorno si infilò sulle mie ginocchia mentre lavoravo. E la prima notte strappò un buco in un sacco di cibo per gatti che avevo ordinato per i gatti dei miei genitori e si servì da sola. Certo, aveva ricevuto il suo cibo e quella sera aveva mangiato bene. Ma a quanto pare Baylee aveva deciso che quella scorta meritava comunque un'ispezione più approfondita.

    La nostra topolina aveva un appetito sorprendente – e un'energia incredibile.

baylee-chapter2Title = La fiducia che cresce
baylee-chapter2Body =
    Quando notammo che beveva moltissima acqua, la portammo dal veterinario. Un esame del sangue rivelò un problema alla tiroide, che spiegava anche la sua grande fame e la sua vivacità. Le diedero un farmaco per la tiroide e una cura antibiotica temporanea perché i valori dell'infiammazione erano elevati.

    Quando improvvisamente cominciò a strofinarsi contro tutto, a miagolare molto e a diventare insolitamente coccolona e attaccata a noi, all'inizio tememmo che stesse reagendo male a una delle sue nuove medicine. Un'altra visita ci diede la spiegazione: la nostra piccola cara era in calore. Non era stata sterilizzata.

    Superammo insieme anche quel periodo. Baylee si fidava sempre di più di noi, e diventavamo sempre più vicini.

baylee-chapter3Title = La nostra piccola principessa
baylee-chapter3Body =
    Ricordo ancora esattamente quanto fu felice quando ebbe il suo tiragraffi ad albero. Si sedette subito sul ripiano più alto e si guardò attorno soddisfatta per l'appartamento. Stranamente, non usò mai più quel posto in particolare. Il perché rimase il suo segreto.

    Gli altri posti restavano raramente liberi: le mie ginocchia, le mie gambe e il nostro letto. Lì dormiva di solito su di me o su mia moglie. Il massimo era riuscire a stendersi su tutti e due insieme.

    Sfilava per l'appartamento con la coda alta, come se sapesse benissimo di essere la principessa di casa. Aveva carattere e sapeva farsi rispettare. Con noi poteva essere del tutto se stessa. Le sue manie, le sue pretese e la sua piccola testardaggine erano amate anche loro.

    Non avevo mai creato un legame così profondo con un animale. Baylee trovò un posto molto in fondo al cuore di mia moglie e al mio.

baylee-chapter4Title = Ore sotto il sole
baylee-chapter4Body =
    Quando il tempo si fece più caldo, scoprì il balcone. Adorava sentire nel pelo i primi raggi di sole del giorno. In poco tempo passava là fuori quasi tutta la giornata.

    I piccioni e gli uccelli del giardino erano un fascino particolare. Li osservava attenta, stava in agguato e cercava ripetutamente di spaventarli. Tra bagni di sole e osservazione degli uccelli, la nostra piccola principessa aveva una giornata molto piena.

baylee-chapter5Title = Un caratterino tutto suo
baylee-chapter5Body =
    Intanto la sua malattia continuava a progredire.

    Il tumore si infiammò, e Baylee continuava a leccare la zona colpita fino ad aprirla. Dal veterinario ricevette la sua prima tutina di recupero, una rossa. Da quel momento dovette indossarne una ogni giorno per impedirle di ferirsi ancora la ferita.

    Si abituò in fretta a portarla. Farsela mettere, invece, le dispiacque fino all'ultimo giorno. Nemmeno una ciotola di cibo bastava allora a convincerla a uscire. Per quanto grande fosse il suo appetito, la sua opinione sulla tutina era inequivocabile.

    La ferita peggiorò. Secerneva pus, alla fine cominciò a sanguinare e prese un cattivo odore. Allo stesso tempo, Baylee continuava a mostrarci quanta gioia trovasse ancora nella vita di tutti i giorni. Giocava, mangiava, esplorava i dintorni e si arrampicava sul suo tiragraffi e sugli armadi.

    A volte restava per ore sul bordo del letto accanto a noi, aspettando che ci svegliassimo finalmente e le preparassimo la colazione. Se ci mettevamo troppo, ci passava semplicemente sopra. Le nostre facce non erano un ostacolo.

    Aveva idee molto precise sul cibo. I suoi gusti erano piuttosto da «pizza surgelata»: dopo un po' il cibo per gatti di alta qualità non riusciva più a entusiasmarla, e la varietà era essenziale. Lo stesso cibo due volte di seguito era fuori discussione. Pollo con fegato, qualsiasi cosa con il formaggio, tonno o salmone – erano cose che la rendevano felice.

baylee-chapter6Title = Con la coda alta
baylee-chapter6Body =
    Il tumore crebbe e lacrimava sempre di più. Baylee soffriva visibilmente. Quando il dolore si attenuava, tornava a essere la nostra gattina, si muoveva per le stanze con la coda alta e rivendicava le sue persone per sé.

    Vedevamo le sue forze svanire piano piano. Ma il suo appetito, la sua curiosità, la sua indole affettuosa e le sue forti fusa c'erano ancora.

baylee-chapter7Title = Dirsi addio
baylee-chapter7Body =
    Venerdì sera, 23 ottobre 2026, ebbe dei sintomi che all'inizio sembravano un raffreddore. Le colava il naso e il respiro era profondo e faticoso, come se lottasse per avere aria.

    Sabato sembrava di nuovo più vivace. Il respiro insolito e il naso che colava restavano, ma mostrava ancora l'energia e l'amore per la vita che conoscevamo così bene.

    La domenica mattina fu difficile. Aveva gli occhi che lacrimavano, faticava a respirare e soffriva. Le diedi un antidolorifico. Dopo circa un'ora stava di nuovo meglio, ma il respiro restava diverso. Sapevamo che dovevamo portarla dal veterinario.

    Lunedì mattina le sue condizioni peggiorarono di nuovo. Per fortuna mia moglie era a casa dal lavoro. Chiamammo lo studio, prendemmo un appuntamento e portammo Baylee lì.

    La radiografia mostrò fin dove si era esteso il cancro. Ci dissero che solo circa un terzo dei suoi polmoni era ancora disponibile per respirare. Lasciarla continuare a vivere avrebbe significato che avrebbe avuto sempre meno aria.

    Dovemmo dirle addio.

    Lunedì 26 ottobre 2026, verso mezzogiorno, la nostra carissima Baylee fu addormentata per sempre.

baylee-chapter8Title = Per sempre sotto il sole
baylee-chapter8Body =
    È stata con noi per poco meno di un anno. In quel tempo era diventata parte della nostra famiglia e un'amica molto vicina. Faceva parte dei nostri giorni di lavoro, delle nostre notti e dei nostri risvegli al mattino. Delle ore di sole sul balcone e delle piccole abitudini che compongono una vita insieme.

    Ora manca in tutti quei momenti.

    Il vuoto è ancora fresco e profondo. Ci mancano le sue fusa, il suo peso sulle nostre gambe, la sua coda dritta e la sua certezza incrollabile che ci saremmo finalmente occupati della sua colazione.

    Se c'è qualcosa dopo questa vita, qualunque cosa sia, allora desidero incontrare un giorno quell'anima.

    Fino ad allora, porteremo la nostra topolina nel cuore per sempre.

    Per questo motivo questo progetto nato per hobby è dedicato a Baylee. Al suo spirito, alla sua vicinanza, alla sua gioia per il sole e per gli uccelli. E all'amore che è cresciuto nel tempo trascorso insieme.

    Qui possa dormire in pace sulla sua nuvola, nel caldo sole, circondata da fiori, piccioni e uccelli del giardino.

    Riposa in pace, mia carissima Baylee.

baylee-photoOpen = Vedi la fotografia intera
baylee-photoClose = Chiudi la fotografia
baylee-zoomIn = Ingrandisci
baylee-zoomOut = Riduci
baylee-fit = Adatta

baylee-photo6 = Baylee, a suo agio tra le coperte.
baylee-photo7 = Accoccolata nella sua tutina di recupero rossa.
baylee-photo8 = Un angolino di sole in una scatola di cartone.
baylee-photo9 = Ben rimboccata e completamente rilassata.
baylee-photo10 = Il suo posto preferito: proprio accanto a noi.

## Errors shared by the forms

error-busy = Troppi tentativi. Riprova più tardi; dopo diverse password errate può volerci un'ora, al massimo un giorno.
error-unavailable = Baylee Central non risponde al momento. Riprova tra poco.
error-network = Nessuna connessione. Controlla la rete e riprova.
error-failed = Qualcosa è andato storto. Riprova; se continua, faccelo sapere.
error-again = Riprova
password-show-label = Mostra la password

## Signing in

signin-title = Accedi
signin-lead = Il tuo account Baylee, per i tavoli online, i contatti e la chat.
signin-name = Nome di accesso
signin-password = Password
signin-submit = Accedi
signin-missing = Inserisci il tuo nome di accesso e la password.
signin-working = Protezione della tua password in corso … ci vuole un momento e avviene solo nel tuo browser.
signin-wrong = Questo nome di accesso e questa password non corrispondono. Controlla entrambi e riprova.
signin-not-moved = Questo account non è ancora passato a Baylee Central. Accedi una volta con il client di Baylee; sposta il tuo account.
signin-no-account = Non hai ancora un account? <register>Creane uno</register>.
signin-forgot = Password dimenticata? <reset>Impostane una nuova con un codice di reimpostazione</reset>.
register-have-account = Hai già un account? <signin>Accedi</signin>.
dialog-close = Chiudi
signin-noscript = Per accedere serve JavaScript: la tua password viene elaborata nel tuo browser e non ci viene mai inviata.

## Registering

register-title = Crea un account
register-lead = Per i tavoli online, i contatti e la chat. Gratuito e aperto a tutti durante la beta aperta.
register-name = Nome di accesso
register-name-hint = Privato, solo per accedere: da 3 a 24 lettere minuscole, cifre, punti, trattini o trattini bassi.
register-display = Nome visualizzato
register-display-hint = Come ti vedono gli altri giocatori: da 3 a 16 lettere, cifre, - e _. Il tuo tag viene assegnato per te.
register-password = Password
register-password-hint = Almeno 8 caratteri. Una frase lunga è facile da ricordare e difficile da indovinare.
register-repeat = Ripeti la password
register-repeat-hint = La stessa ancora una volta, per scovare un errore di battitura.
register-age = Ho 16 anni o più.
register-consent = Accetto le <terms>condizioni d'uso</terms>. Come trattiamo i tuoi dati è spiegato nell'<privacy>informativa sulla privacy</privacy>.
register-submit = Crea account
register-error-name = Il nome di accesso richiede da 3 a 24 lettere minuscole, cifre, punti, trattini o trattini bassi.
register-error-display = Il nome visualizzato richiede da 3 a 16 lettere o cifre; - e _ solo all'interno.
register-error-reserved = Questo nome visualizzato è riservato. Scegline un altro.
register-error-short = La password richiede almeno 8 caratteri.
register-error-long = La password è troppo lunga.
register-error-repeat = Le due password non coincidono.
register-error-age = Devi avere 16 anni o più per avere un account.
register-error-consent = Accetta le condizioni d'uso per creare un account.
register-error-taken = Questo nome di accesso è già in uso. Scegline un altro.

## The recovery code

code-title = Il tuo codice di recupero
code-handle = Il tuo handle: { $handle }
code-lead = Solo con questo codice i tuoi dati cifrati si salvano se dimentichi la password. Noi non lo conosciamo e non possiamo ripristinarlo. Conservalo in un posto sicuro, lontano da questo dispositivo.
code-shown-once = Mostrato solo ora
code-download = Salva come file di testo
code-kept = Ho salvato il mio codice di recupero
code-not-kept = Salva prima il codice, poi confermalo.
code-continue = Continua
code-later = Non ora
code-copy = Copia
code-copied = Copiato.
code-copy-failed = Qui la copia non ha funzionato. Seleziona il codice e copialo a mano.
code-print = Stampa
code-manager = Salva nel gestore di password
code-manager-id = Codice di recupero Baylee ({ $name })
code-manager-offered = Il codice è stato proposto al tuo browser. Se te lo ha chiesto, controlla che l'abbia salvato.
code-manager-failed = Il tuo browser non ha preso il codice. Conservalo in un altro modo.
code-mail = Mandamelo per e-mail
code-mail-warning = Attenzione: il codice resta così nella tua casella di posta. Chi può leggere la tua posta può anche far inviare lì un codice di reimpostazione, e con entrambi reimpostare il tuo accesso e aprire i tuoi dati cifrati.
code-mail-subject = Il mio codice di recupero Baylee
code-sheet-title = Codice di recupero Baylee
code-sheet-name = Nome di accesso: { $name }
code-sheet-what = Con questo codice i tuoi dati cifrati di Baylee si salvano se dimentichi la password: lo inserisci nella pagina di reimpostazione, insieme a un codice di reimpostazione.
code-sheet-keep = Conservalo dove solo tu puoi arrivarci. Nessuno di Baylee te lo chiederà mai.
code-email-label = Indirizzo e-mail (facoltativo)
code-email-hint = Per un codice di reimpostazione se dimentichi la password. Dopo questo passaggio ti mandiamo per e-mail un codice per confermare l'indirizzo; puoi anche aggiungerne uno più tardi nel tuo account.
code-email-step-title = Conferma il tuo indirizzo e-mail
code-email-step-lead = Inserisci qui il codice ricevuto per e-mail, oppure più tardi nella pagina di sicurezza del tuo account.
code-email-step-done = Continua verso il tuo account

## Setting a new password with a reset code

reset-title = Imposta una nuova password
reset-lead = Per quando hai dimenticato la password: con un codice di reimpostazione ne imposti qui una nuova. Il codice arriva per e-mail al tuo indirizzo confermato, oppure da un amministratore.
reset-noscript = Per impostare una nuova password serve JavaScript: la tua password e il tuo codice di recupero vengono elaborati nel tuo browser e non ci vengono mai inviati.
reset-code = Codice di reimpostazione
reset-code-hint = Per e-mail o da un amministratore. Inizia con RESET e vale una volta sola: per un'ora se è arrivato per e-mail, per 24 ore se l'ha creato un amministratore.
reset-data = I tuoi dati cifrati
reset-keep = Ho il mio codice di recupero. I miei dati restano.
reset-new = Non ce l'ho. Ricominciare con una cassaforte vuota.
reset-recovery = Codice di recupero
reset-recovery-hint = Il codice che hai salvato quando hai creato l'account. Resta nel tuo browser e non ci viene mai inviato.
reset-warning = Con una cassaforte vuota i tuoi dati cifrati sono persi per sempre: le impostazioni salvate nel tuo account, i backup dei tuoi mazzi e la cronologia della chat. Nessuno può recuperarli, nemmeno noi. Il tuo account, il tuo nome visualizzato e i tuoi contatti restano. Ricevi un nuovo codice di recupero.
reset-warning-confirm = Ho capito che questi dati sono persi per sempre.
reset-submit = Imposta la nuova password
reset-missing = Inserisci il tuo nome di accesso, il codice di reimpostazione e una nuova password.
reset-recovery-missing = Inserisci il tuo codice di recupero, oppure scegli la cassaforte vuota.
reset-error-code = Questo non è un codice di reimpostazione. Inizia con RESET, seguito da otto gruppi di quattro caratteri.
reset-error-recovery = Questo codice di recupero non apre i dati del tuo account. Controllalo e riprova. Non è stato cambiato nulla, e il tuo codice di reimpostazione vale ancora.
reset-error-confirm = Prima conferma di sapere che cosa va perso.
reset-refused = Questo nome di accesso e questo codice di reimpostazione non vanno insieme, oppure il codice è scaduto o è già stato usato. Controlla entrambi. Se non serve a nulla, chiedi un nuovo codice.
reset-done-title = La tua nuova password è impostata
reset-done-kept = I tuoi dati e il tuo codice di recupero restano com'erano. Sei disconnesso ovunque; accedi con la nuova password.
reset-done-new = Riparti con una cassaforte vuota e sei disconnesso ovunque. Qui sotto c'è il tuo nuovo codice di recupero; quello vecchio non vale più.
reset-unsure-title = Non è arrivata nessuna risposta
reset-unsure = La connessione si è interrotta prima che Baylee Central rispondesse, quindi non sappiamo dire se la tua nuova password è stata impostata. Prova ad accedere con quella. Se non funziona, non è stato cambiato nulla: il tuo codice di reimpostazione vale ancora, inseriscilo di nuovo qui.
reset-unsure-code = Se la tua nuova password funziona, questo è il tuo nuovo codice di recupero e quello vecchio non vale più; non può essere mostrato di nuovo, quindi salvalo adesso. Se la tua nuova password non funziona, questo codice non vale nulla.
reset-to-signin = Accedi
reset-mail-title = Nessun codice di reimpostazione?
reset-mail-lead = Inserisci il tuo nome di accesso. Se il tuo account ha un indirizzo e-mail confermato, ti mandiamo un codice di reimpostazione.
reset-mail-name-hint = Il nome con cui accedi, non il tuo indirizzo e-mail.
reset-mail-submit = Invia un codice per e-mail
reset-mail-error-name = Questo non è un nome di accesso. Inserisci il nome con cui accedi, non il tuo indirizzo e-mail.
reset-mail-sent = Se questo account ha un indirizzo e-mail confermato, un codice di reimpostazione è in arrivo lì. Vale un'ora.
reset-mail-busy = Troppe richieste per ora. Riprova più tardi.
reset-mail-none = Questo server non invia e-mail. Chiedi un codice di reimpostazione a un amministratore.
reset-mail-early = Troppo in fretta. Riprova tra un attimo.

## Changing the password

password-new = Nuova password
password-new-repeat = Ripeti la nuova password
password-current = Password attuale
password-change-submit = Cambia password
password-missing = Inserisci la password attuale e una nuova.
password-error-current = La password attuale non è corretta. Controllala e riprova.
password-changed = La tua password è cambiata. Qui resti connesso, in ogni altro posto sei disconnesso, anche nel client di Baylee; lì accedi con la nuova password.
password-unsure = Non è arrivata nessuna risposta, quindi non sappiamo dire se la tua password è stata cambiata. Al prossimo accesso prova prima la nuova password, poi quella vecchia.

## An account's e-mail address

email-title = Indirizzo e-mail
email-lead = Un indirizzo e-mail è facoltativo. Lo usiamo per mandarti un codice di reimpostazione quando ne chiedi uno, e per avvisarti quando l'indirizzo viene sostituito o rimosso.
email-warning = Chiunque possa leggere questa casella di posta può reimpostare il tuo accesso. I tuoi dati cifrati sopravvivono a una reimpostazione solo con il tuo codice di recupero.
email-no-mail = Questo server non invia e-mail, quindi qui non si può né inserire né confermare un indirizzo e-mail.
email-unavailable = Al momento non è possibile mostrare l'indirizzo e-mail.
email-password-hint = La tua password attuale, perché nessun altro davanti a questo browser decida chi può reimpostare il tuo accesso.
email-submit = Invia il codice di conferma
email-missing = Inserisci un indirizzo e-mail e la tua password.
email-error-address = Non possiamo scrivere a questo indirizzo. Inserisci un indirizzo semplice, come nome@example.org; le lettere accentate e le dieresi non funzionano.
email-current = Il tuo indirizzo
email-confirmed-at = Confermato
email-waiting = Attende il suo codice
email-change = Cambia
email-change-lead = Il tuo indirizzo attuale resta in uso finché non hai confermato quello nuovo.
email-change-cancel = Tieni l'indirizzo attuale
email-remove = Rimuovi
email-remove-ask = Rimuovere questo indirizzo? Avvisiamo l'indirizzo che è stato rimosso. Dopo non potrai più ricevere un codice di reimpostazione per e-mail.
email-remove-ask-no-mail = Rimuovere questo indirizzo? Finché questo server non invia e-mail, non puoi inserirne uno nuovo.
email-remove-confirm = Rimuovi indirizzo
email-remove-keep = Tienilo
email-pending = Abbiamo inviato un codice a <code>{ $address }</code>. Vale 15 minuti.
email-pending-keeps = Finché non lo confermi, <code>{ $address }</code> resta il tuo indirizzo.
email-code = Codice di conferma
email-code-hint = Otto caratteri, come in XXXX-XXXX. Maiuscole o minuscole, con o senza trattino.
email-confirm = Conferma
email-resend = Invia di nuovo
email-other = Usa un altro indirizzo
email-withdraw = Ritira
email-error-code = Questo non è un codice di conferma. Ha otto caratteri, come in XXXX-XXXX.
email-wrong-code = Quel codice non è giusto. Controllalo e riprova. Dopo diversi codici sbagliati devi inserire di nuovo l'indirizzo.
email-code-dropped = Neanche quel codice era giusto, quindi l'indirizzo non è più in attesa. Inseriscilo di nuovo per ottenere un nuovo codice.
email-now-confirmed = Il tuo indirizzo e-mail è confermato.
email-resent = Un nuovo codice è in arrivo. Quello precedente non vale più.
email-withdrawn = L'indirizzo è ritirato. Non si attende più nessun codice.
email-removed = Il tuo indirizzo e-mail è rimosso.
email-busy = Troppe e-mail per ora. Riprova più tardi.
email-same = È già il tuo indirizzo.
email-unsure = Non è arrivata nessuna risposta, quindi non sappiamo se è andata a buon fine. Guarda cosa è mostrato qui e riprova se non è andata.
email-again = Per inviare il codice, inserisci ancora una volta la tua password.

## Central's terms

terms-step-title = Nuove condizioni d'uso
terms-step-lead = Le condizioni di Baylee Central sono cambiate. Leggile e accettale per usare il tuo account; puoi anche rifiutarle e uscire.
terms-accept = Accetta
terms-decline = Rifiuta ed esci

## The signed-in area

account-lead = Il tuo account Baylee: profilo, sicurezza, privacy e gateway.
account-nav = Il tuo account
account-home = Panoramica
account-profile = Profilo
account-profile-lead = Il tuo handle, il tuo nome visualizzato e il tuo codice di recupero.
account-security = Sicurezza
account-security-lead = Dove sei connesso, e come uscire.
account-privacy = Privacy e dati
account-privacy-lead = Esporta i tuoi dati o elimina il tuo account.
account-gateways = Gateway e token
account-gateways-lead = I gateway che hai registrato e i token con cui accedono.
account-loading = Caricamento …
account-signed-out = Non sei connesso.
account-unnamed = Il tuo account
account-signin-name = Nome di accesso: { $name }
account-until = Connesso fino a { $time }
account-needs-title = Richiede la tua attenzione
account-needs-code = Crea un codice di recupero
account-needs-name = Scegli il tuo nome visualizzato
account-admin = Amministrazione

reauth-lead = Serve un accesso recente. Inserisci ancora una volta il tuo nome di accesso e la password.
reauth-submit = Accedi di nuovo

profile-who-title = Chi sei
profile-handle = Handle
profile-signin-name = Nome di accesso
profile-source = Account
profile-source-moved = Trasferito dal gateway ufficiale
profile-source-registered = Registrato su questo sito
profile-choose-title = Scegli il tuo nome visualizzato
profile-choose-submit = Salva nome visualizzato
profile-name-taken = Questo account ha già un nome visualizzato.
profile-code-title = Codice di recupero
profile-code-kept = Il tuo account ha un codice di recupero. Conservalo con cura; senza la tua password è l'unica via per tornare ai tuoi dati cifrati.
profile-code-missing = Il tuo account non ha ancora un codice di recupero. Creane uno; ti servirà di nuovo la password.
profile-code-make = Crea un codice di recupero

security-sessions = Dove sei connesso
security-lead = Baylee Central tiene una sessione per questo sito web e una per il client del gioco; non distingue i singoli dispositivi.
security-kind-browser = Questo sito web
security-kind-client = Il client di Baylee
security-this = Questa
security-signed-in = Connesso
security-ends = Termina
security-gateways = Sui gateway
security-end-client = Disconnetti il client
security-client-ended = Il client è disconnesso.
security-sign-out = Esci da qui
security-password-title = Password
security-password-lead = I tuoi dati cifrati e il tuo codice di recupero restano come sono. Cambiare la password ti disconnette ovunque tranne qui.

privacy-export-title = Esporta i tuoi dati
privacy-export-lead = Ciò che Baylee Central sa del tuo account, come file JSON: i suoi dati di base, le sessioni, le condizioni accettate, i tuoi gateway e le voci di audit che nominano il tuo account. I tuoi contenuti cifrati possono essere decifrati ed esportati solo dal tuo client di Baylee. L'esportazione richiede un accesso effettuato meno di 30 minuti fa.
privacy-export-make = Prepara l'esportazione
privacy-export-save = Salva l'esportazione
privacy-export-no-email = L'indirizzo e-mail non ha potuto essere letto; non è in questo file.
privacy-delete-title = Elimina il tuo account
privacy-delete-lead = Il tuo account, i suoi dati, le sue sessioni, i tuoi contatti e i gateway di cui sei proprietario vengono eliminati subito e definitivamente dal sistema in esercizio; escono dai backup al più tardi entro 12 mesi. Central elimina anche i messaggi che hai inviato; ciò che gli altri hanno già memorizzato sui loro dispositivi può restarvi.
privacy-delete-start = Elimina il mio account
privacy-delete-password = La tua password, per confermare
privacy-delete-prove = Continua
privacy-delete-last = Ultimo passaggio: non si può annullare. Hai un minuto.
privacy-delete-gateways = Questi gateway di cui sei proprietario vengono eliminati insieme all'account:
privacy-delete-confirm = Elimina l'account definitivamente
privacy-delete-cancel = Tieni il mio account
privacy-deleted = Il tuo account è stato eliminato.

gateways-none = Non hai registrato alcun gateway. La maggior parte dei giocatori non ne ha mai bisogno.
gateways-add-title = Registra un gateway
gateways-origin = Indirizzo del gateway
gateways-origin-hint = L'indirizzo https del gateway, come https://play.example.org.
gateways-name = Nome del gateway
gateways-add = Registra gateway
gateways-added = Il gateway è registrato e attende l'approvazione di un amministratore.
gateways-taken = Questo indirizzo è già registrato.
gateways-invalid = Inserisci un indirizzo https senza percorso, e un nome.
gateways-approved = Approvato
gateways-pending = In attesa di approvazione
gateways-disabled = Disattivato
tokens-title = Token
tokens-name = Nome del token
tokens-days = Validità (giorni)
tokens-sso = Fa accedere i giocatori
tokens-listing = Inserisce il gateway nell'elenco
tokens-both = fa accedere i giocatori e inserisce il gateway nell'elenco
tokens-make = Crea token
tokens-revoke = Revoca
tokens-revoked = Revocato
tokens-revoked-now = Il token è stato revocato.
tokens-until = Fino a { $time }
tokens-invalid = Dai un nome al token e almeno una cosa che consente.
tokens-shown-title = Il tuo nuovo token
tokens-kept = L'ho salvato
tokens-again = Accesso eseguito di nuovo. Crea di nuovo il token.

## The messenger

account-chat = Chat
account-contacts = Contatti
chat-side = Conversazioni e contatti
chat-tabs = Messenger
chat-tab-chats = Chat
chat-tab-contacts = Contatti
chat-main = Conversazione
chat-unread = da leggere
chat-conversations = Le tue conversazioni
chat-search = Cerca una conversazione
chat-search-hint = Cerca (Ctrl+K)
chat-new-group = Nuovo gruppo
chat-no-conversations = Ancora nessuna conversazione. Aggiungi un contatto, poi scrivigli.
chat-no-match = Nessuna conversazione ha questo nome.
chat-former-group = Un ex gruppo
chat-former-lead = Non fai più parte di questo gruppo.
chat-you = Tu
chat-members = { $n ->
    [one] { $n } membro
   *[other] { $n } membri
}
chat-say-hello = Saluta.
chat-history-only = Non è più un contatto.
chat-today = Oggi
chat-yesterday = Ieri
chat-encrypted = Crittografia end-to-end
chat-opening-lead = Le tue conversazioni e i tuoi contatti. I messaggi vengono cifrati sui tuoi dispositivi prima di lasciarli, e Central li inoltra senza poterli leggere.
chat-new-from = Nuovo messaggio da { $who }.
chat-new-many = { $n } nuovi messaggi.
chat-retry = Riprova
chat-discard = Scarta
chat-not-sent = Non inviato.
chat-sending = Invio in corso …
chat-keys-changed = Le chiavi di qualcuno in questa conversazione sono cambiate. Confronta il numero di sicurezza prima di scrivere ancora.
chat-refused = Alcuni messaggi non hanno potuto essere verificati e non vengono mostrati.
chat-failed = Al momento la chat non ha potuto raggiungere Central.
chat-failed-open = Non è stato possibile aprire la chat. Ricarica la pagina per riprovare.
chat-empty = Ancora nessun messaggio.
chat-older = Mostra messaggi più vecchi
chat-compare-first = Le chiavi sono cambiate. Confronta prima il numero di sicurezza, poi scrivi.
chat-compare = Confronta il numero di sicurezza
chat-friend-required = Potrai scrivere quando sarete di nuovo contatti.
chat-group-left = Non fai più parte di questo gruppo.
chat-details = Dettagli
chat-close = Chiudi
chat-back = Torna alle conversazioni
chat-messages = Messaggi
chat-fan = Baylee è un contenuto amatoriale non ufficiale, non approvato né autorizzato da Wizards. <more>Che cosa significa</more>
chat-safety = Numeri di sicurezza
chat-safety-lead = Confronta il numero con l'altra persona, di persona o su un altro canale di cui ti fidi. Se coincide, nessuno si è inserito tra voi senza che ve ne accorgeste.
chat-changed = Cambiato
chat-confirm-keys = Coincide: fidati delle nuove chiavi
chat-count = { $n } di 1.000 caratteri
chat-write = Scrivi un messaggio
chat-send = Invia
chat-enter-hint = Invio invia, Maiusc+Invio va a capo.
chat-choose = Scegli una conversazione
chat-choose-lead = I tuoi messaggi sono cifrati in questo browser; Central li inoltra soltanto.
chat-storage = La tua chiave di chat resta in questo browser finché sei connesso.
chat-opening = Apertura delle tue chat …
chat-open = Apri le mie chat
chat-ended = Il tuo accesso è terminato.
chat-unnamed = Scegli prima il tuo nome visualizzato: ti dà l'handle a cui gli altri scrivono.
chat-missing-title = Accedi di nuovo per aprire le tue chat
chat-missing-lead = Questo browser non possiede la tua chiave di chat. La tua password la apre qui; non lascia mai questo browser.
chat-invites = Inviti ai gruppi
chat-invited-by = da { $who }
chat-group-name = Nome del gruppo
chat-group-name-hint = Un nome da 1 a 48 caratteri.
chat-group-create = Crea
chat-group-failed = Al momento non è stato possibile modificare il gruppo.
chat-group-members = { $n ->
    [one] { $n } membro
   *[other] { $n } membri
}
chat-group-owner = Proprietario
chat-group-rename = Rinomina
chat-group-transfer = Rendi proprietario
chat-group-remove = Rimuovi
chat-group-transfer-ask = Rendere { $who } proprietario di questo gruppo? Resti un membro.
chat-group-remove-ask = Rimuovere { $who } da questo gruppo?
chat-group-invite = Invita un contatto
chat-group-choose = Scegli un contatto
chat-group-invite-button = Invita
chat-group-invited = Invitato
chat-group-delete = Elimina gruppo
chat-group-leave = Lascia il gruppo
chat-group-delete-ask = Eliminare «{ $name }» per tutti? I messaggi già ricevuti restano leggibili.
chat-group-leave-ask = Lasciare «{ $name }»? Potrai tornare solo con un nuovo invito.

people-incoming = richieste
people-failed = Al momento non è stato possibile caricare i tuoi contatti.
people-empty = Ancora nessun contatto. Cerca qualcuno tramite il suo handle, o dagli il tuo.
people-your-handle = Il tuo handle:
people-incoming-title = Richieste per te
people-friends = Contatti
people-outgoing = Richieste inviate
people-blocked = Bloccati
people-find = Aggiungi un contatto tramite il suo handle
people-find-button = Cerca
people-handle-hint = Un handle completo con il suo tag, come Name#1a2b.
people-yourself = Questo è il tuo stesso handle.
people-nobody = Nessuno ha questo handle.
people-request = Invia una richiesta di contatto
people-requested = Richiesta inviata.
people-is-friend = Un contatto
people-is-incoming = Ha chiesto di essere tuo contatto
people-is-outgoing = Richiesta inviata
people-is-blocked = Bloccato
people-write = Scrivi
people-remove = Rimuovi
people-block = Blocca
people-accept = Accetta
people-decline = Rifiuta
people-cancel = Ritira
people-unblock = Sblocca
people-keep = Mantieni
people-confirm-remove = Rimuovere { $who } dai tuoi contatti? I tuoi messaggi restano.
people-confirm-block = Bloccare { $who }? Non potrà più chiedere di essere tuo contatto né scriverti.

## Offline, updates, installing

title-offline = Offline
offline-title = Sei offline
offline-lead = Questa pagina richiede una connessione. Central conserva i messaggi per te finché non sei di nuovo online.
offline-again = Riprova
update-ready = È pronta una nuova versione di baylee.app.
update-unsent = Hai qui del testo non ancora inviato. Ricaricare comunque?
update-reload = Ricarica
install-app = Installa come app

## Notifications

push-title = Notifiche
push-lead = Scegli di che cosa Baylee può avvisarti. Le notifiche non mostrano testo dei messaggi, nomi né contenuti delle segnalazioni.
push-device = Notifiche su questo dispositivo
push-categories = Avvisami su
push-chat = Messaggi in chat
push-contacts = Richieste di contatto
push-updates = Nuove versioni
push-feedback = Segnalazioni di feedback (amministratori)
push-gateways = Richieste di gateway (amministratori)
push-on = Attivate su questo dispositivo. Vengono consegnate solo le categorie selezionate.
push-off = Disattivate su questo dispositivo. Le tue scelte di categoria vengono conservate.
push-scope = Le scelte di categoria valgono per il tuo account. Il permesso del dispositivo è separato su ogni browser. Uscendo, questo dispositivo viene scollegato.
push-failed = Non è stato possibile confermare la modifica. Riprova.
push-denied = Le notifiche sono bloccate. Puoi consentirle nelle impostazioni del browser o del dispositivo e riprovare.
push-unsupported = Qui questo browser non offre Web Push. Su iPhone e iPad, aggiungi Baylee alla schermata Home e aprilo da lì.
push-unconfigured = Le notifiche in background non sono ancora disponibili su questo server. Le tue preferenze possono già essere salvate.

register-closed = La registrazione è in pausa al momento. Riprova più tardi.
register-refused = La registrazione non è stata accettata. Riprova.
admin-registration = Registrazione
admin-registration-open = Aperta: chiunque può creare un account dopo aver risolto la proof of work.
admin-registration-closed = Chiusa: nessuno può creare un account finché non la riapri.
admin-registration-lead = Chiudi la registrazione durante un attacco e riaprila a mano. Ogni modifica viene annotata nel registro di audit.
admin-registration-close = Chiudi la registrazione
admin-registration-reopen = Riapri la registrazione
hero-discord = Community su Discord
hero-whatsapp = Community su WhatsApp
hero-github = GitHub
nav-menu = Menu
foot-project = Progetto
foot-source = Codice sorgente (AGPL-3.0)
foot-central = Baylee Central (codice disponibile)
foot-summary = Gioco di carte da tavolo gratuito e open source, scritto in Rust e Bevy — con un motore di regole deterministico, modalità offline completa, IA della casa, ponte per LLM e gateway della community.
strip-ci = CI
strip-commit = Commit
strip-gateways = Gateway
strip-online = online
strip-offline = offline
strip-directory = l'elenco
strip-cards = Carte giocabili
strip-cards-unit = carte
strip-accounts = Account su Baylee Central
strip-live = Adesso sui gateway, come riferito da loro
strip-views = Pagine viste dal { $date }
live-views = { $n ->
    [one] { $n } visualizzazione
   *[other] { $n } visualizzazioni
}
live-accounts = { $n ->
    [one] { $n } account
   *[other] { $n } account
}
live-players = { $n ->
    [one] { $n } giocatore
   *[other] { $n } giocatori
}
live-playing = { $n ->
    [one] { $n } in partita
   *[other] { $n } in partita
}
live-tables = { $n ->
    [one] { $n } tavolo aperto
   *[other] { $n } tavoli aperti
}
live-games = { $n ->
    [one] { $n } partita
   *[other] { $n } partite
}
hero-badges = In breve
hero-badge-beta = Beta aperta
hero-badge-free = Gratuito
hero-badge-open = Open source
hero-badge-fan = Contenuto amatoriale non ufficiale
gws-open-client = Apri in Baylee
gws-open-client-help = «Open in Baylee» (Apri in Baylee) avvia il client installato (0.1.1-beta.9b o successivo). Mostra l'indirizzo e chiede conferma prima di aggiungere il gateway.
dl-menu-title = Tutti i sistemi
dl-old-cpu = processori meno recenti (senza AVX2)
ci-passing = riuscito
ci-failing = fallito
dl-button = Scarica Baylee
dl-quick = Altri sistemi e architetture
dl-others = altri sistemi

# The start page, as the old site wrote it (owner, 11.11.2026)
link-readmeModel = #playing-against-a-language-model
about-engineTitle = Un motore che applica ogni regola
about-deterministic = Basata su un generatore casuale con seme e senza alcuna dipendenza dall'orologio di sistema o dall'ordine di hash, ogni partita è rigorosamente deterministica e riproducibile mossa per mossa.
about-tableTitle = Un tavolo 3D pulito costruito con Bevy
about-keys = Dalla lobby integrata passi con facilità tra il deck builder locale, le partite in solitaria e i tavoli multigiocatore sui gateway della community.
feature-hiddenTitle = Le informazioni nascoste restano private
feature-hidden = Il tuo client riceve solo i dati che il tuo posto può vedere. Grimori e mani degli avversari arrivano rigorosamente come semplici quantità—nemmeno un client modificato può ispezionare le carte nascoste.
feature-seatsTitle = Fino a otto giocatori a un tavolo
feature-seats = Duelli, tutti contro tutti e partite a squadre (come il 2 contro 2) fino a 8 posti. I compagni di squadra condividono i turni e possono mostrarsi a vicenda le carte in mano.
feature-commanderTitle = Formati flessibili e regole della casa personalizzate
feature-commander = Costruisci mazzi per i classici duelli da 60 carte, per tavoli multigiocatore o
    per formati personalizzati della casa. Punti vita iniziali, disposizione delle squadre e regole
    del mulligan sono impostati liberamente dall'host.
feature-aiTitle = Replay deterministici e segnalazioni F8
feature-ai = Ogni partita viene registrata mossa per mossa. Se noti un problema in un'interazione di regole durante il gioco, premi F8 direttamente al tavolo per inviare una segnalazione di stato riproducibile.
feature-processTitle = Un processo server isolato per partita
feature-process = Ogni partita attiva gira nel suo processo del motore in sandbox sul gateway. Il gateway stesso gestisce solo la lobby e le stanze—un errore non tocca mai gli altri tavoli.
feature-languagesTitle = Testo delle carte in 19 lingue
feature-languages = Quando un gateway fornisce il catalogo multilingue delle carte, le carte si leggono nella tua lingua preferita, con l'inglese come ripiego. L'interfaccia del client è disponibile in tedesco e in inglese.
feature-watchTitle = Guarda le partite dal vivo
feature-watch = Il pulsante Watch della lobby ti mette al tavolo come spettatore dal vivo: segui il campo di battaglia pubblico in tempo reale mentre le mani di tutti i giocatori restano nascoste.
feature-connectionTitle = Riconnessione senza interruzioni
feature-connection = Se una connessione cade per poco, il tavolo tiene il posto con un conto alla rovescia e può mettere in pausa la partita o far subentrare temporaneamente l'IA della casa.
feature-tablesTitle = Quattordici timer per le decisioni
feature-tables = Dai turni rapidi da 15 secondi ai tavoli rilassati senza limite di tempo, configura
    il ritmo singolarmente per ogni stanza.
shots-eyebrow = Al tavolo
shots-title = Dentro il gioco
shots-lead = Brevi registrazioni dal vivo catturate direttamente dal client 3D con immagini delle
    carte da Scryfall (© Wizards of the Coast), da duelli e tavoli da 4 giocatori fino alla lobby
    cifrata e al deck builder.
shots-credit = Immagini delle carte: Scryfall · © Wizards of the Coast
shots-showLarge = Mostra in grande: { $title }
shots-dialog = Schermata
shots-previous = Precedente
shots-next = Successivo
shots-close = Chiudi
ai-eyebrow = Avversari IA e modelli linguistici
ai-title = IA della casa, ponte LLM e IA nativa di Baylee
ai-houseTitle = IA euristica della casa in cinque livelli di abilità
ai-house = Pronta per giocare offline e a qualsiasi tavolo online: scegli tra Novice, Casual, Steady, Sharp ed Expert. L'IA della casa decide in locale senza tempi di attesa e vede solo lo stesso stato pubblico di un giocatore umano.
ai-modelTitle = Modelli linguistici al tavolo tramite Seat-Bridge
ai-model = Fai sedere <b>Google Gemini, OpenAI ChatGPT, Anthropic Claude, DeepSeek</b> o modelli locali tramite <b>LM Studio</b> direttamente al tuo tavolo. Le chiavi API restano protette in variabili d'ambiente con limiti di spesa; in caso di timeout, l'IA della casa subentra senza interruzioni.
ai-setup = Guida alla configurazione di LLM e Seat-Bridge
home-gwLead = I tavoli online si trovano sui gateway, server che Baylee Central elenca una volta che il suo amministratore li ha approvati. Ecco i primi; l'elenco li ha tutti.
beta-open = Apri l'elenco
beta-own = Gestisci il tuo
source-eyebrow = Codice sorgente
source-title = Open source, e lo resta
source-promise = Baylee sarà sempre open source e gratuito. Nessun accesso a pagamento, nessuna funzione a pagamento, nessuna pubblicità. Né ora né in futuro. Il codice è sotto la GNU Affero General Public License, il che significa che chiunque gestisca un gateway modificato per altre persone deve offrire anche a loro il suo codice sorgente.
source-rust = È scritto in Rust, il progetto di una sola persona con molto aiuto da agenti di programmazione, e ogni modifica passa per lo stesso controllo: formattazione, lint, la suite di test e la validazione di ogni carta sul suo testo stampato. Issue e pull request sono benvenute; <agents>AGENTS.md</agents> e <claude>CLAUDE.md</claude> sono le regole di lavoro per persone e agenti allo stesso modo.
source-discord = Community su Discord
source-whatsapp = Community su WhatsApp
source-architecture = Architettura
source-repoTitle = Il repository in questo momento
source-stars = Stelle
source-issues = Issue e pull request aperte
source-note = Letto dall'API pubblica di GitHub dal server di questo sito ogni dieci minuti. Il tuo browser non chiede nulla a GitHub.
gw-count =
    { $n ->
        [one] { $n } gateway in elenco, { $online } online.
       *[other] { $n } gateway in elenco, { $online } online.
    }
gw-none = Nessun gateway è ancora in elenco.
contact-intro = Una domanda, un'idea, un gioco che vorresti vedere, o qualcosa che preferisci non pubblicare: questo modulo va direttamente alla persona che sta dietro a Baylee. Lascia un indirizzo e-mail se vuoi una risposta; senza, possiamo solo leggerlo.
contact-f8 = Una carta risolta in modo errato è meglio segnalarla dal gioco con <kbd>F8</kbd>, perché la segnalazione può contenere l'intera partita.
contact-discord = chiacchiera, trova avversari, scopri le novità sulle build
contact-issues = Issue su GitHub
contact-issuesSub = bug e idee, in pubblico
contact-operator = Chi gestisce questo sito, con un indirizzo postale: le <impressum>note legali</impressum>.
contact-whatsappSub = Gruppo della community e partite veloci
contact-reportsTitle = Segnalazioni e privacy
contact-reportsSub = Dettagli su segnalazioni F8 e dati
ai-housePill = Offline e online
ai-providers = Modelli linguistici supportati
ai-hostsPill = Per chi gestisce gateway
ai-hostsTitle = Offri posti LLM ospitati sul tuo gateway
ai-hosts = Come gestore di un gateway puoi mettere a disposizione modelli linguistici già configurati direttamente sul tuo server. Gli ospiti a un tavolo scelgono semplicemente uno dei tuoi modelli ospitati come avversario—senza bisogno di una chiave API.
ai-trainingPill = In addestramento · IA nativa del motore
ai-trainingTitle = IA nativa di Baylee — addestrata a giocare direttamente sul motore come un vero giocatore
ai-training = Accanto all'IA della casa e al ponte LLM esterno, una IA nativa di Baylee dedicata viene addestrata direttamente sul motore di regole deterministico. Impara a valutare gli stati del tavolo, le finestre di priorità, i calcoli di combattimento e le sinergie dei mazzi come un giocatore umano esperto—senza alcuna latenza di API esterne.
ai-hostingLink = Hosting di gateway e configurazione LLM

# The gateways and the contact page, as the old site wrote them (owner, 11.11.2026)
gw-lead = Un gateway è un server dove vivono i tavoli online: account, mazzi, stanze e un processo del motore per ogni partita. Questi sono quelli registrati presso Baylee Central e approvati dal suo amministratore. Ognuno è gestito dai suoi gestori per i loro gruppi, e lì valgono le loro regole. Un elenco non è una promessa su ciò che fa un gateway.
gw-howTo = Per giocare su uno, copia il suo indirizzo nella schermata iniziale del client. Online significa che Central ha avuto notizie del gateway nell'ultimo minuto e mezzo; non è una promessa che risponda. Nomi e versioni sono quelli che ogni gestore indica.
gw-find = Cerca un gateway
gw-findHint = Nome o indirizzo
gw-shown = { $n } su { $all } mostrati
gw-noMatch = Nessun gateway caricato finora corrisponde.
gw-showAll = Mostra tutti
gw-stats = L'elenco in numeri
gw-stat-listed = Gateway in elenco
gw-stat-online = Online ora
gw-stat-players = Giocatori online
gw-stat-playing = Di cui in partita
gw-stat-tables = Tavoli aperti
gw-stat-games = Partite in corso
gw-show = Mostra
gw-show-every = Tutti
gw-show-online = Online
gw-show-tables = Con tavoli aperti
gw-sort = Ordina per
gw-sort-best = Consigliati
gw-sort-name = Nome (A–Z)
gw-sort-players = Più giocatori
gw-sort-tables = Più tavoli aperti
gw-sort-version = Versione più recente
gw-view = Vista
gw-view-cards = Riquadri
gw-view-list = Lista
link-readmeGateway = #running-your-own-gateway
selfhost-eyebrow = Self-hosting e integrazione con Central
selfhost-title = Gestisci il tuo gateway
selfhost-processes = Un gateway in self-hosting esegue tre processi leggeri accanto a PostgreSQL 18+: <code>baylee-gateway</code> gestisce stanze, mazzi e instradamento; <code>baylee-agent</code> avvia un processo <code>baylee-engine-server</code> isolato per ogni partita, così che ogni tavolo sia in sandbox.
selfhost-proxy = Metti un reverse proxy TLS davanti alla tua origine HTTPS (per esempio <code>https://play.example.org</code>), se vuoi importa il catalogo completo di Scryfall per il testo delle carte in 19 lingue, e i giocatori possono collegarsi direttamente tramite il tuo indirizzo nel client.
selfhost-central = <b>Collega con Baylee Central e configura la privacy:</b> registra la tua origine HTTPS in <mine>I tuoi gateway</mine> e genera un token <code>bgt_…</code> (ambiti: <code>sso</code> per il single sign-on, <code>listing</code> per i heartbeat dell'elenco e la telemetria dal vivo). Nella console del tuo gateway scegli il livello di privacy della telemetria (<code>full</code> per partite in corso, impostazioni dei tavoli e giocatori che hanno acconsentito, <code>counts_only</code> solo per i totali, o <code>private</code> solo per lo stato di attività).
selfhost-guide = <guide>Leggi la guida completa al self-hosting</guide> · <protocol>Riferimento di protocollo e heartbeat (docs/protocol.md)</protocol>
selfhost-source = Ogni gateway risponde a <code>GET /source</code> con la sua licenza, versione e commit senza accesso, soddisfacendo automaticamente l'obbligo AGPL di divulgazione del codice sorgente.
reports-eyebrow = Segnalazioni
reports-title = Hai trovato qualcosa che non va? Dillo a noi
reports-f8 = Premi <kbd>F8</kbd> al tavolo o nella lobby. Una segnalazione contiene il suo tipo, il tuo testo, la versione del client e, a un tavolo, l'ID della partita. Tutto il resto ha una propria casella, e ogni casella parte selezionata, quindi deseleziona ciò che preferisci tenere per te prima di premere Send: sistema e hardware, il tavolo come lo vedi, il tuo log (gli altri giocatori rinominati «Player A», «Player B», …), le tue impostazioni, un'immagine della sola finestra di Baylee, e a un tavolo l'intero registro della partita (ogni mossa e le carte di ogni posto, senza nomi). <i>Show what is sent</i> mostra ogni byte. I token di sessione non vengono mai inviati.
reports-usage = Mentre giochi offline, il client dice una volta al minuto che una partita è in corso: nessun account, nessun ID del dispositivo, nulla sulla partita. <i>Send anonymous usage count</i> in Settings › Privacy & Data lo disattiva.
reports-discord = Vuoi chiacchierare, trovare avversari o sapere per primo delle nuove build? Unisciti alla <discord>community di Baylee su Discord</discord>.
reports-issues = Domande sulle regole, comportamento errato delle carte, idee: <issues>apri una issue su GitHub</issues>. Una carta risolta in modo errato è la segnalazione più utile che possiamo ricevere, perché ogni carta ha un test e la tua ne diventa uno.
reports-privacy = <docs>docs/privacy.md</docs> elenca tutto ciò che conservano il client, il gateway e il servizio di feedback. Questo sito non imposta alcun cookie a meno che tu non acceda, non usa analisi statistiche e non carica caratteri o script dal server di nessun altro. <notice>L'informativa sulla privacy</notice> dice che cosa conservano il sito e Baylee Central, e nomina ogni altro host con cui comunicano, e quando.

# The download page, as the old site set it (owner, 11.11.2026)
dlPage-eyebrow = Scaricalo
dlPage-filesEyebrow = Ogni file
dlPage-filesTitle = Tutti i sistemi
install-eyebrow = Installazione
install-releases = Tutte le versioni su GitHub
release-notes = Note di rilascio e checksum
release-all = Tutte le versioni

# The download page's head, as the old site wrote it
dlPage-title = Scarica
dlPage-lead = Ogni download porta direttamente al file su GitHub, senza passaggi intermedi. Dove una versione ha gli installer, sono la via più facile; gli archivi contengono lo stesso gioco da estrarre ovunque. Un Baylee installato si aggiorna da solo dagli archivi firmati ovunque possa scrivere nella propria cartella. Ogni pacchetto include accanto al gioco il Seat-Bridge per i modelli linguistici.

# The reading pages' eyebrows, as the old site set them
imp-eyebrow = Note legali
terms-eyebrow = Condizioni d'uso
privacy-eyebrow = Privacy

# The privacy notice's contents, as the old site named them
privacy-contents = Indice

# The account pages' eyebrow, as the old site set it
acct-eyebrow = Baylee Central

# The account area's navigation, as the old site named it
app-account = Il tuo account

# ==== The security page (/security) ====================================
# Every sentence and every arrow of this page maps to a row of the claim
# table in docs/security-page.md; a claim changes
# there first. No "safe", "unbreakable" or compliance words: mechanisms only.
title-security = Sicurezza in Baylee: password, chiavi e chat
desc-security = Come Baylee tratta password, chiavi e messaggi della chat, passo dopo passo in
    quattro diagrammi, con i limiti di ciascuno.
nav-security = Sicurezza
sec-eyebrow = Sicurezza
sec-title = Password, chiavi e chat: che cosa va dove
sec-lead = Quattro flussi, passo dopo passo: che cosa lascia il tuo dispositivo, che cosa vede
    Baylee Central e che cosa no. Con i limiti di ciascuno.
sec-toc-label = In questa pagina
sec-figure = Diagramma in passaggi: { $name }
sec-play = Avvia
sec-all = Tutti i passaggi
sec-step = Passaggio { $n }
sec-hint = Il diagramma scorre da solo. Scegli un passaggio per fermarlo lì.
sec-hint-still = Scegli un passaggio per evidenziarlo.
sec-in-browser = La cornice tratteggiata è il tuo browser: tutto questo avviene al suo interno.
# The actors' names are the only words inside the drawings, where text does
# not wrap: one or two words each, at most about twice the English width
# (the test `names_fit_with_room_to_grow` measures every language).
sec-actor-device = Tuo dispositivo
sec-actor-central = Central
sec-actor-page = Pagina
sec-actor-worker = Worker
sec-actor-store = Archivio
sec-actor-you = Tu
sec-actor-contact = Contatto
sec-actor-website = Sito web
sec-actor-client = Client

sec-signin-eyebrow = Accesso e registrazione
sec-signin-title = Central verifica la tua password senza riceverla
sec-signin-lead = Baylee ti fa accedere con OPAQUE (RFC 9807). Il tuo dispositivo e Central si
    scambiano tre messaggi; la tua password non è in nessuno di essi, nemmeno come semplice hash.
sec-signin-c1 = Richiesta oscurata
sec-signin-t1 = Il tuo dispositivo oscura la password.
sec-signin-s1 = Combina la password con un numero casuale nuovo e invia solo il risultato. Da quel
    messaggio la password non si può leggere.
sec-signin-c2 = Risposta di Central
sec-signin-t2 = Central risponde con una chiave propria.
sec-signin-s2 = Central calcola sul valore oscurato con una chiave che solo Central possiede e
    aggiunge la sua metà di uno scambio di chiavi. In questo modo non viene a conoscere la password.
sec-signin-c3 = Argon2id · 64 MiB
sec-signin-t3 = Il tuo dispositivo fa il lavoro pesante.
sec-signin-s3 = Rimuove l'oscuramento e allunga il risultato con Argon2id (64 MiB di memoria, tre
    passaggi); Central non può chiedere impostazioni più leggere. Una password errata viene notata
    qui, prima che venga inviato altro.
sec-signin-c4 = Prova
sec-signin-t4 = Il tuo dispositivo dimostra di avere le chiavi giuste.
sec-signin-s4 = Con la password giusta il tuo dispositivo arriva alla stessa chiave di sessione di
    Central e ne invia una breve prova (64 byte).
sec-signin-c5 = Sessione + involucro
sec-signin-t5 = Central apre la sessione.
sec-signin-s5 = Tornano indietro la sessione (nel browser un cookie che la pagina stessa non può
    leggere) e la tua chiave principale in un involucro che si apre solo sul tuo dispositivo, con la
    password giusta.
sec-signin-register = <b>La registrazione</b> usa lo stesso protocollo. Il tuo dispositivo genera
    una chiave principale casuale (256 bit), la avvolge una volta con la tua password e una volta
    con il tuo codice di recupero, e invia solo gli involucri. Central memorizza un record OPAQUE e
    gli involucri, mai la chiave.
sec-signin-legacy = <b>Un'eccezione:</b> gli account del vecchio gateway ufficiale che non sono
    ancora passati a Central. Finché un account di questo tipo non è passato, ogni tentativo di
    accesso nel client di Baylee invia la password digitata, tramite una connessione TLS verificata,
    a Central, che la confronta con il vecchio hash e non la memorizza né la registra nei log.
    Quando il passaggio è riuscito, l'account accede come mostrato sopra. Questo sito web non fa
    accedere tali account; rimanda al client. L'eccezione termina con la scadenza fissata per il
    passaggio.
sec-signin-limit = <b>Che cosa resta:</b> chi si impossessa del database di Central insieme alle sue
    chiavi può provare le password una per una; ogni tentativo costa un'esecuzione di Argon2id. Una
    password trovata in questo modo apre i dati cifrati dell'account e le sue chat memorizzate.
    Conta ancora una password lunga che non usi altrove.
sec-signin-guess = <b>Indovinare la password tramite l'accesso</b> viene rallentato, non fermato. Central conta i tentativi non conclusi con la password giusta: al massimo 100 al giorno su un account da indirizzi da cui non ha effettuato l'accesso, e al massimo 100 al giorno da un indirizzo su tutti gli account insieme. Oltre questo limite risponde che i tentativi sono stati troppi, finché non finisce l'ora o il giorno. Una password comune viene trovata comunque in questo modo. E chi conosce il tuo nome di accesso può esaurire i tentativi del tuo account: fino alla fine del giorno puoi accedere solo da un indirizzo da cui il tuo account ha effettuato l'accesso negli ultimi 30 giorni (Central ricorda questi indirizzi solo fino al suo riavvio).

sec-memory-eyebrow = In memoria
sec-memory-title = Che cosa resta in memoria dopo l'accesso
sec-memory-lead = La password serve una volta sola: per aprire la tua chiave principale. Il
    diagramma la segue nel browser; ciò che il client di Baylee fa in modo diverso è più sotto.
sec-memory-c1 = Password
sec-memory-t1 = La pagina consegna la password a un Worker.
sec-memory-s1 = Il modulo la passa a un thread in background tutto suo (un Worker), svuota il campo
    di input e libera la propria copia.
sec-memory-c2 = OPAQUE + Argon2id
sec-memory-t2 = Il Worker calcola, poi sovrascrive.
sec-memory-s2 = Il suo codice Rust tiene la password solo fino alla fine del passaggio OPAQUE e poi
    la sovrascrive, insieme alle chiavi che ha derivato per strada e che conserva lui stesso, e ai
    64 MiB di memoria di lavoro di Argon2id.
sec-memory-c3 = Chiave: verificata, azzerata
sec-memory-t3 = La chiave principale viene aperta, usata e cancellata.
sec-memory-s3 = Il Worker la apre per verificare l'accesso, ne deriva le chiavi della chat se il tuo
    account ha una chat, e la sovrascrive di nuovo.
sec-memory-c4 = Chiavi chat, sigillate
sec-memory-t4 = Restano solo le chiavi della chat.
sec-memory-s4 = Perché la chat si apra dopo un ricaricamento senza la tua password, il Worker le
    memorizza nel browser (IndexedDB), cifrate con AES-256-GCM. La chiave usata a questo scopo non è
    estraibile: gli script possono usarla ma non possono leggerla. Finché una pagina della chat è
    aperta, un Worker apposito tiene in memoria le chiavi della chat non sigillate. Uscendo, il
    record viene eliminato; dopo che una sessione è terminata da sola, lo elimina la prossima pagina
    di questo sito che si carica.
sec-memory-c5 = Worker terminato
sec-memory-t5 = Il Worker dell'accesso viene terminato.
sec-memory-s5 = Quando l'accesso è concluso, la pagina di accesso termina il suo Worker e il browser
    scarta la memoria di quel Worker.
sec-memory-limit = <b>Ciò che un browser non promette:</b> la password come digitata, il suo
    passaggio al Worker e le stringhe di JavaScript sono gestiti dal browser. Baylee le libera; non
    sovrascrive le copie della pagina stessa e non può sovrascrivere quelle del browser. Quando
    quella memoria viene davvero riutilizzata lo decide il browser. E chi può eseguire script su
    questo sito o leggere il profilo del tuo browser mentre sei connesso può arrivare alle chiavi
    della chat sigillate.
sec-memory-client = <b>Nel client di Baylee</b> la password digitata sta in una memoria che viene
    sovrascritta quando è rilasciata, e viene scartata subito dopo l'accesso; non viene mai
    memorizzata. La chiave principale resta nella memoria del client finché sei connesso, perché il
    client apre con essa i tuoi dati cifrati.

sec-chat-eyebrow = Chat
sec-chat-title = I messaggi sono cifrati sul tuo dispositivo
sec-chat-lead = Ogni messaggio ha una chiave tutta sua. Central memorizza e inoltra solo ciò che è
    cifrato; per leggerlo serve la chiave della chat di un account per cui è stato avvolto.
sec-chat-c1 = Testo + chiave monouso
sec-chat-t1 = Una chiave nuova per ogni messaggio.
sec-chat-s1 = Il tuo dispositivo genera una chiave casuale del messaggio e ci cifra il testo
    (XChaCha20-Poly1305).
sec-chat-c2 = Avvolta per chiave, firmata
sec-chat-t2 = Avvolta per la chiave della chat di ogni destinatario, firmata con la tua.
sec-chat-s2 = Avvolge la chiave del messaggio separatamente per ogni chiave della chat che l'account
    del destinatario ha registrato (X25519) e firma intestazione e testo cifrato con la tua chiave
    della chat (Ed25519). Oggi un account ha una sola chiave della chat, derivata dalla sua chiave
    principale e uguale in ogni browser e client in cui è connesso.
sec-chat-c3 = Cifrato + involucri
sec-chat-t3 = Central riceve solo il testo cifrato.
sec-chat-s3 = Central controlla la firma e l'elenco dei destinatari e memorizza il testo cifrato con
    gli involucri. Non possiede né le chiavi dei messaggi né le chiavi del tuo account.
sec-chat-c4 = Cifrato + un involucro
sec-chat-t4 = Ogni chiave della chat ha il suo involucro.
sec-chat-s4 = Quando il tuo contatto recupera i suoi messaggi, Central consegna il testo cifrato e
    solo l'involucro fatto per quella chiave della chat.
sec-chat-c5 = Verificato, aperto
sec-chat-t5 = Verifica, apertura, lettura.
sec-chat-s5 = Il dispositivo del tuo contatto controlla la tua firma, apre l'involucro della chiave
    del messaggio con la sua chiave della chat e decifra il testo.
sec-chat-meta = <b>Ciò che Central vede comunque:</b> chi è in contatto con chi, i gruppi con i loro
    nomi e membri, i nomi visualizzati, le chiavi pubbliche della chat e, per ogni messaggio, chi
    l'ha inviato, in quale conversazione, quando e quanto è grande. Come ogni server, vede anche
    l'indirizzo da cui proviene una richiesta.
sec-chat-limit = <b>Limiti:</b> non c'è un double ratchet e quindi nessuna forward secrecy: se la
    chiave della chat di un destinatario viene rubata più tardi, apre anche i messaggi precedenti
    che erano stati avvolti per essa. La tua chiave della chat è derivata dalla chiave principale
    del tuo account, e Central conserva i messaggi cifrati dopo la consegna: chi si impossessa del
    database di Central con le sue chiavi e indovina la tua password, o ha il tuo codice di
    recupero, può leggerli. Al primo contatto il tuo dispositivo si fida delle chiavi che Central
    indica; per verificarle, confronta il numero di sicurezza su un altro canale. Se le chiavi di un
    contatto cambiano più tardi, Baylee invia di nuovo solo dopo che le hai confermate. Chi
    appartiene a un gruppo lo decide Central.

sec-pair-eyebrow = Accesso tramite link
sec-pair-title = Dal browser al client senza digitare di nuovo la password
sec-pair-lead = Chi ha effettuato l'accesso sul sito web dovrebbe poter avviare il client di Baylee
    già connesso. Per questo la pagina porta la chiave principale al client: sigillata, e in modo
    tale che Central non possa né aprirla né inserirne una propria.
sec-pair-status = <b>A che punto siamo:</b> Central e la libreria client di Baylee oggi sono in
    grado di farlo e sono testati l'uno con l'altra. Non si può ancora usare da questo sito web né
    da un client rilasciato.
sec-pair-c1 = Avvia associazione
sec-pair-t1 = La pagina avvia un'associazione.
sec-pair-s1 = Central emette per essa un identificativo valido cinque minuti. Nello stesso momento
    la pagina genera un segreto monouso che non mostra mai a Central.
sec-pair-c2 = Link con il segreto
sec-pair-t2 = Un link apre il client.
sec-pair-s2 = Il link <code>baylee://pair</code> porta l'identificativo e il segreto dalla pagina al
    client sul tuo dispositivo, senza passare da Central.
sec-pair-c3 = Chiave pubblica
sec-pair-t3 = Il client risponde con una coppia di chiavi nuova.
sec-pair-s3 = Invia a Central la metà pubblica e un valore di controllo calcolato con il segreto
    (HMAC-SHA-256). La pagina verifica quel valore; entrambi gli schermi mostrano lo stesso breve
    codice, e tu approvi sulla pagina.
sec-pair-c4 = Chiave sigillata
sec-pair-t4 = La pagina sigilla la chiave principale per il client.
sec-pair-s4 = Con HPKE (RFC 9180), per la chiave pubblica del client e anche sotto il segreto.
    Central memorizza il sigillo una volta; non può aprirlo e, non conoscendo il segreto, non può
    crearne uno proprio.
sec-pair-c5 = Sessione + sigillo
sec-pair-t5 = Il client riscatta una sola volta.
sec-pair-s5 = Riceve una sessione insieme al sigillo, apre il sigillo con la sua chiave privata e il
    segreto, ed è connesso. L'associazione viene poi eliminata, anche quando il riscatto fallisce.
sec-pair-note = <b>La password non partecipa a questo scambio.</b> La pagina ha la chiave principale
    solo nel momento in cui l'ha appena aperta con la tua password. Una sessione associata in questo
    modo vale come non più recente dell'accesso con password sulla pagina da cui proviene:
    l'associazione non la rinnova mai.

sec-limits-eyebrow = Limiti
sec-limits-title = Ciò che nulla di tutto questo può fare
sec-limit-device = <b>Un dispositivo compromesso.</b> Un malware, un'estensione del browser che
    legge le pagine o un client manomesso vedono ciò che vedi e digiti tu. Nessuna cifratura sul
    percorso verso un server aiuta contro questo.
sec-limit-site = <b>Il codice di un sito web viene dal suo server.</b> Chi prende il controllo del
    nostro server potrebbe consegnare un codice che cattura la tua password mentre la digiti; gli
    hash di integrità e una Content Security Policy rigorosa difendono dalle modifiche dall'esterno,
    non da questo. Il client è un programma sul tuo computer e controlla una firma prima di
    installare un aggiornamento.
sec-limit-central = <b>Un Central di cui qualcuno ha preso il controllo.</b> Potrebbe rivelare
    metadati, negare il servizio e attaccare gli accessi. Potrebbe indicare chiavi false a un primo
    contatto o aggiungere un membro a un gruppo, e così leggere i messaggi inviati dopo. Ciò che
    memorizza dei tuoi dati e delle tue chat è cifrato; per aprirlo servono la tua password o il tuo
    codice di recupero.
sec-limit-recovery = <b>Chiavi perse.</b> Senza la tua password e senza il tuo codice di recupero
    nessuno può riportare indietro i tuoi dati cifrati, nemmeno noi: Central ha solo gli involucri.
sec-limit-audit = <b>Nessun audit indipendente.</b> L'accesso segue OPAQUE (RFC 9807) con Argon2id
    (RFC 9106), e l'associazione sigilla con HPKE (RFC 9180): specifiche pubblicate dell'IRTF, non
    standard dell'IETF. L'ordine dei passaggi dell'associazione e il formato della chat sono
    combinazioni proprie di Baylee di elementi standard (Ed25519, X25519, XChaCha20-Poly1305).
    Finora solo chi l'ha costruita ha esaminato l'implementazione. Il codice è aperto; se trovi
    qualcosa, <contact>faccelo sapere</contact>.
sec-docs-title = Per approfondire
sec-docs = I dettagli sono nei documenti del progetto: <protocol>protocollo
    crittografico</protocol>, <chat>chat</chat>, <accounts>account e associazione</accounts> e
    <threats>modello delle minacce</threats>. Che cosa succede ai tuoi dati sul piano giuridico è
    spiegato nell'<privacy>informativa sulla privacy</privacy>.

# The page's two links from elsewhere: the FAQ and the sign-in page.
faq-security-q = Come gestisce Baylee la mia password e le mie chat?
faq-security-a = Quando accedi, il tuo dispositivo usa il protocollo OPAQUE per dimostrare di
    conoscere la password senza inviarla; i messaggi della chat sono cifrati sul tuo dispositivo. La
    <security>pagina sulla sicurezza</security> mostra ogni passaggio in un diagramma, con l'unica
    eccezione e i limiti.
sec-signin-link = Che cosa succede alla tua password quando accedi: <security>Sicurezza</security>
sec-teaser-lead = Passo dopo passo: che cosa lascia il tuo dispositivo, che cosa vede Baylee Central
    e che cosa no, e i limiti di ogni flusso.
sec-teaser-more = Vedi ogni passaggio in un diagramma
# ==== end of the security page =================================================
# ==== registration: the proof of work, while it takes long =====================
register-solving = Il tuo browser sta risolvendo un rompicapo che tiene lontani i robot. Quando vengono creati molti account, può volerci qualche minuto … { $percent } %

## An administrator makes a reset code (/account/admin)
admin-reset-title = Reimposta un accesso
admin-reset-lead = Trova l'account della persona che te lo ha chiesto e crea un codice di reimpostazione per esso. Con il codice la persona imposta una nuova password nella pagina di reimpostazione.
admin-find = Trova l'account
admin-only = Questa pagina è solo per gli amministratori.
admin-search = Cerca account
admin-search-hint = Parte di un nome di accesso o di un nome visualizzato, oppure # e un tag. Lascia vuoto per elencare tutti gli account, dal più recente.
admin-search-submit = Cerca
admin-search-error = Una ricerca è di 1-64 caratteri semplici senza accenti, oppure # e un tag come #1a2b.
admin-count = Account mostrati: { $count }
admin-none = Nessun account corrisponde a questa ricerca.
admin-more = Mostra altri
admin-open = Apri
admin-back = Indietro
admin-pill = Amministratore
admin-state = Stato
admin-state-pending = Riservato, mai configurato
admin-state-enrolled = Configurato
admin-state-restored = In uso
admin-created = Creato
admin-last-sign-in = Ultimo accesso
admin-recovery = Codice di recupero
admin-yes = Sì
admin-no = No
admin-reset-outstanding = Un codice di reimpostazione non usato è valido fino alle { $time }. Central non dice se l'ha creato un amministratore o se è arrivato per e-mail. Un nuovo codice lo sostituisce.
admin-reset = Reimposta l'accesso
admin-refused-admin = Questo account ha il ruolo di amministratore, e Central non crea codici di reimpostazione per l'account di un amministratore. Rimuovi prima il ruolo.
admin-refused-own = Questo è il tuo account, e nessuno crea un codice di reimpostazione per il proprio. Cambia invece la tua password in Sicurezza.
admin-refused-pending = Questo account è stato riservato e mai configurato, quindi non ha un accesso da reimpostare.
admin-refused-moving = Il trasferimento di questo account dal gateway ufficiale non è finito. Finché non lo è, il suo accesso si reimposta lì, non qui.
admin-refused-stale = Baylee Central considera il tuo accesso più vecchio di 30 minuti. Riprova; inserisci ancora una volta la tua password.
admin-refused-role = Il tuo account non ha più il ruolo di amministratore.
admin-refused-gone = Questo account non esiste più.
admin-switched = Questo browser ora è connesso con un altro account, quindi nessun codice è stato richiesto. Ricarica la pagina.
admin-lost = Non è arrivata nessuna risposta, quindi non sappiamo dire se un codice è stato creato. Se lo è stato, nessuno può mostrarlo di nuovo, e ha già sostituito ogni codice precedente. Creane uno nuovo: sostituisce a sua volta quello.
admin-warning-title = Prima di creare un codice
admin-warning-who = Verifica fuori da questo sito che la persona che lo chiede abbia davvero questo account, per esempio con una telefonata o su un canale con cui già la conosci. Chi ha il codice può impostare una nuova password.
admin-warning-ends = Creare un codice non termina nulla: la vecchia password continua a funzionare finché il codice non viene usato, e ogni dispositivo resta connesso.
admin-warning-data = I dati cifrati della persona sopravvivono alla reimpostazione solo con il suo codice di recupero o su un dispositivo ancora connesso. Senza nessuno dei due, riparte con una cassaforte vuota.
admin-warning-replaces = Un nuovo codice sostituisce ogni codice precedente, anche uno inviato per e-mail. Vale 24 ore e funziona una sola volta.
admin-warning-takeover = Un codice di reimpostazione risponde a una password dimenticata, non a un account preso da altri: le sessioni di un intruso e la vecchia password restano finché il codice non viene usato, e per sempre se non lo è mai.
admin-warning-continue = Ho verificato chi lo chiede
admin-password-hint = Per creare un codice serve un accesso di meno di 30 minuti fa, quindi inserisci ancora una volta la tua password. Resta in questo browser.
admin-password-missing = Inserisci la tua password.
admin-make = Crea il codice di reimpostazione
admin-code-title = Il codice di reimpostazione
admin-code-until = Valido fino alle { $time }, e una sola volta.
admin-code-lead = Consegnalo alla persona che lo ha chiesto, su un canale di cui ti fidi. Viene mostrato solo ora: né Baylee Central né questa pagina possono mostrarlo di nuovo. Se va perso, creane uno nuovo.
admin-code-done = Fatto

# ==== the administration's other pages (islands/admin/) ========================
# Translated by a model (Claude Sonnet) on 12.11.2026; no native speaker has read it yet.
admin-accounts = Account
admin-account = Un account
admin-audit = Registro di audit
admin-legacy = Vecchi account
admin-feedback = Feedback
admin-ov-lead = Quanti account e gateway ci sono, e che cosa ti aspetta.
admin-ov-accounts = { $week } nuovi questa settimana, { $moved } trasferiti dal gateway ufficiale.
admin-ov-accounts-go = Tutti gli account
admin-ov-pending = Approva ciascuno solo dopo aver verificato chi gestisce il suo indirizzo.
admin-ov-pending-none = Non ti aspetta nulla.
admin-ov-pending-go = Esamina i gateway
admin-ov-gateways = Gateway nell'elenco
admin-ov-gateways-sub = { $online } online adesso, { $disabled } disattivati, { $tokens } token validi.
admin-ov-gateways-go = Gateway approvati
admin-ov-admins = Amministratori
admin-ov-admins-sub = Ogni loro modifica è nel registro di audit.
admin-ov-legacy = Ancora da trasferire
admin-ov-legacy-sub = Account del gateway ufficiale che non sono ancora stati trasferiti, fino al { $day }.
admin-ov-reports = Segnalazioni aperte
admin-ov-crashes = Nuovi arresti anomali che ti aspettano: { $n }.
admin-ov-calm = Nessun nuovo arresto anomalo. Bug, idee e feedback dei giocatori.
admin-ov-reports-go = Smista le segnalazioni
admin-ov-failed = Baylee Central non ha detto quanti sono.
admin-id = ID
admin-tag = Tag
admin-role = Ruolo
admin-member = Membro
admin-tag-lead = Un tag è unico presso Baylee Central, quindi lo è anche Name#tag. Scrivilo in esadecimale, per esempio #1a2b. Ogni modifica compare nel registro di audit con il vecchio e il nuovo tag.
admin-tag-set = Imposta il tag
admin-tag-confirm = Assegnare a { $name } il tag { $tag }?
admin-tag-warn = Dopo la configurazione della chat di Central, il tag resta fisso per preservare i messaggi firmati e i numeri di sicurezza. Per gli account senza la chat di Central, i gateway assumono un nuovo tag al successivo accesso; i sussurri precedenti legati a un gateway non compaiono più dopo la modifica.
admin-tag-error = Un tag ha da una a otto cifre esadecimali, da #1 a #7fffffff.
admin-tag-saved = Il tag è impostato.
admin-tag-taken = Baylee Central ha rifiutato questo tag: un altro account lo ha, oppure questo account usa la chat di Central, che mantiene il suo tag.
admin-role-grant = Rendi amministratore
admin-role-remove = Rimuovi il ruolo di amministratore
admin-role-grant-title = Rendere { $name } amministratore?
admin-role-remove-title = Rimuovere a { $name } il ruolo di amministratore?
admin-role-grant-text = Gli amministratori vedono tutti gli account e i gateway e possono modificarli.
admin-role-remove-text = La sua prossima richiesta come amministratore viene rifiutata.
admin-role-granted = L'account ora è amministratore.
admin-role-removed = Il ruolo di amministratore è stato rimosso.
admin-user-gateways = Gateway che ha registrato
admin-user-audit = Ultimi eventi di audit
admin-change-password = Questa modifica richiede un accesso di meno di 30 minuti fa, quindi inserisci ancora una volta la tua password. Resta in questo browser.
admin-change-lost = Non è arrivata nessuna risposta, quindi non sappiamo dire se la modifica è stata fatta. La pagina l'ha riletta: controlla che cosa mostra prima di riprovare.
admin-change-unknown = Non è arrivata nessuna risposta, quindi non sappiamo dire se la modifica è stata fatta. Ricarica la pagina prima di riprovare.
admin-change-switched = Questo browser ora è connesso con un altro account, quindi non è stato modificato nulla. Ricarica la pagina.
admin-refused-gateway = Questo gateway non esiste più.
admin-working = Richiesta a Baylee Central in corso …
admin-gw-lead = Approva un gateway per l'elenco pubblico solo dopo aver verificato, fuori da Baylee Central, che il suo proprietario gestisce il suo indirizzo; i suoi token fanno accedere i giocatori anche senza l'approvazione. Disattivarlo lo toglie dall'elenco e termina lì gli accessi dei suoi giocatori.
admin-gw-none = Nessun gateway in questo stato.
admin-gw-official = Ufficiale
admin-gw-owner = Proprietario
admin-gw-bridge = Il ponte (ufficiale)
admin-gw-seen = Ultimo heartbeat
admin-gw-tokens = Token validi
admin-gw-pair = e una coppia di credenziali
admin-gw-approve = Approva
admin-gw-enable = Riattiva
admin-gw-disable = Disattiva
admin-gw-show-tokens = Mostra i token
admin-gw-hide-tokens = Nascondi i token
admin-gw-no-tokens = Ancora nessun token.
admin-gw-used = Ultimo utilizzo: { $time }
admin-gw-approve-title = Approvare { $name }?
admin-gw-approve-text = Approva solo se hai verificato che il proprietario gestisce { $origin }. Una volta approvato, compare nell'elenco pubblico.
admin-gw-disable-title = Disattivare { $name }?
admin-gw-disable-text = { $origin } esce subito dall'elenco pubblico, i suoi token smettono di funzionare e tutti i giocatori che vi hanno effettuato l'accesso vengono disconnessi al controllo successivo.
admin-gw-approved = Il gateway è approvato.
admin-gw-disabled = Il gateway è disattivato.
admin-gw-revoke-title = Revocare il token «{ $name }»?
admin-gw-revoke-text = Il token smette subito di funzionare. Un gateway che lo usa non può più far accedere i giocatori né mantenere la sua voce nell'elenco finché non ne riceve un altro.
admin-audit-lead = Ciò che amministratori e gestore hanno modificato, dal più recente. Il registro conserva gli ID; i nomi accanto sono quelli di oggi.
admin-audit-events = Eventi
admin-audit-none = Ancora nulla di registrato.
admin-audit-when = Quando
admin-audit-what = Cosa
admin-audit-by = Da
admin-audit-about = Riguarda
admin-audit-operator = Il gestore
admin-audit-admin-granted = Reso amministratore
admin-audit-admin-removed = Ruolo di amministratore rimosso
admin-audit-reset-issued = Codice di reimpostazione creato
admin-audit-tag-changed = Tag modificato
admin-audit-gateway-approved = Gateway approvato
admin-audit-gateway-disabled = Gateway disattivato
admin-audit-token-revoked = Token del gateway revocato
admin-audit-registration-closed = Registrazione chiusa
admin-audit-registration-opened = Registrazione aperta
admin-legacy-lead = Account del gateway ufficiale precedenti a Baylee Central che non sono ancora stati trasferiti. Ognuno si trasferisce al suo primo accesso nel client, con la sua vecchia password, e la sua riga viene eliminata a trasferimento concluso. Solo nomi, orari e contatori: mai un hash di password.
admin-legacy-none = Tutti gli account sono stati trasferiti. Non ne resta nessuno.
admin-legacy-no-store = Questo Central non conserva vecchi account.
admin-legacy-no-deadline = Nessuna scadenza impostata: le righe restano finché il gestore non ne imposta una.
admin-legacy-deadline = Rimasti: { $count }. Dopo il { $day }, la pulizia giornaliera elimina ogni riga ancora presente.
admin-legacy-move = Trasferimento
admin-legacy-held = Trattenuto
admin-legacy-failures = Accessi falliti
admin-legacy-imported = Importato
admin-legacy-unstarted = Non iniziato
admin-legacy-reserved = Nome riservato
admin-legacy-enrolled = Configurato
admin-legacy-restored = Ripristinato
admin-legacy-banned = Bannato
admin-legacy-must-change = Nuova password richiesta
admin-legacy-locked = Bloccato fino a { $until }
