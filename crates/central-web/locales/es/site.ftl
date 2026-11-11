# baylee.app's words in English, Project Fluent's format. English decides
# which messages exist and which arguments each takes; build.rs turns them
# into typed functions (crates/central-web/build.rs). The reader is "you";
# a button says what it does.

## The frame of every page

site-name = Baylee
skip = Saltar al contenido
# Read out after a link that leaves the site (the stylesheet carries it).
new-tab = se abre en una pestaña nueva
nav-label = Principal
nav-download = Descargar
nav-gateways = Gateways
nav-faq = Preguntas
nav-contact = Contacto
nav-sign-in = Iniciar sesión
nav-account = Tu cuenta
lang-label = Idioma
theme-label = Esquema de color
theme-system = Colores: como tu dispositivo
theme-light = Colores: claro
theme-dark = Colores: oscuro

foot-label = Acerca de este sitio
foot-play = Jugar
foot-legal = Aviso legal
foot-releases = Todas las versiones
foot-issues = Informar de un error
foot-discord = Comunidad de Discord
foot-central-source = Código fuente de este sitio
foot-dedication = Para Baylee. Siempre bajo el sol.
fan-content = Baylee es contenido de fans no oficial permitido por la <policy>Política de contenido de fans</policy>. No está aprobado ni respaldado por Wizards. Parte de los materiales utilizados es propiedad de Wizards of the Coast. ©Wizards of the Coast LLC
foot-colophon = Tipografías: Literata e Inter, con licencia SIL Open Font License 1.1, servidas desde este sitio. La pintura y el logotipo son propios. <notices>Avisos</notices>

## Titles and descriptions

title-home = Baylee – Construye tus mazos. Reúne a tus amigos. Solo juega.
desc-home = Baylee es un juego de fans gratuito y de código abierto de Magic: The Gathering para Linux, Windows y macOS: un motor de reglas que conoce las cartas, mesas de hasta ocho jugadores y rivales para cuando no hay nadie más.
title-download = Descargar Baylee
desc-download = Descarga el cliente de Baylee para Windows, macOS y Linux: instaladores y archivos de la última versión, con sumas de verificación y firmas.
title-gateways = Gateways: dónde se juega online a Baylee
desc-gateways = El directorio público de los gateways de Baylee, los servidores donde viven las mesas online, y cómo montar el tuyo.
title-faq = Preguntas y respuestas
desc-faq = Qué es Baylee, cuánto cuesta, cómo funcionan el juego online y las cuentas, y qué pasa con tus datos.
title-contact = Contacto
desc-contact = Escribe a la persona que está detrás de Baylee o informa de un error.
title-impressum = Aviso legal
desc-impressum = Quién gestiona baylee.app (§ 5 DDG).
title-privacy = Política de privacidad
desc-privacy = Qué datos tratan baylee.app y Baylee Central sobre ti, por qué, con qué base jurídica y durante cuánto tiempo.
title-terms = Condiciones de uso
desc-terms = Las condiciones de uso de Baylee Central, el servicio de cuentas que hay detrás de baylee.app.
title-fan-content = Contenido de fans no oficial
desc-fan-content = Baylee es un proyecto de fans bajo la Política de contenido de fans de Wizards of the Coast: qué significa y qué no.
title-not-found = Página no encontrada
title-gone = Esta página ya no existe
crumbs-label = Estás aquí
breadcrumb-home = Inicio

## Home

hero-title = <nb>Build your decks.</nb> <nb>Gather your friends.</nb> <glow>Just play.</glow>
hero-lead = Un constructor de mazos completo, partidas de hasta ocho jugadores en duelos, todos contra todos o por equipos, y un motor de reglas que comprueba cada paso, para que cada turno sea justo sin que nadie tenga que consultar nada.
hero-offline = Las partidas en solitario y el constructor de mazos funcionan sin conexión y sin cuenta. Las mesas online viven en gateways; <host>puedes montar el tuyo</host>.
hero-account = ¿Vas a jugar online? <register>Crea una cuenta</register>, gratis y abierta a todos durante la beta abierta, o <signin>inicia sesión</signin>.
hero-memorial = Recuerda a Baylee: abre su álbum de fotos

status-label = Estado del proyecto

cards-eyebrow = Las cartas
cards-title = Cuántas cartas conoce Baylee
cards-lead = Baylee implementa las cartas de Magic una a una, cada una comprobada con su texto impreso. Estas son las cifras de la última versión.
cards-implemented = Totalmente implementadas
cards-implemented-help = Todas las reglas de la carta funcionan y están probadas.
cards-generated = Generadas
cards-generated-help = Implementadas por el generador de Baylee a partir del texto de reglas de la carta, pero aún no probadas a mano. Se pueden jugar; en casos límite poco frecuentes, alguna puede comportarse de otra manera.
cards-partial = Parcialmente implementadas
cards-partial-help = La carta se puede jugar, pero aún falta una de sus habilidades o un caso límite de sus reglas.
cards-total = Cartas en total
cards-total-help = Todas las cartas que tiene Magic, tal como las lista Scryfall; las tres cifras anteriores son las que Baylee puede jugar.
cards-help = Qué significa esto
cards-share = Hoy se pueden jugar { $playable } de { $total } cartas.
cards-as-of = Versión { $tag }, contadas el { $date }.
cards-missing = No se pudieron leer las cifras de las cartas en este momento.

shot-duel-alt = Un duelo contra la IA de la casa: seis criaturas atacan, tres son bloqueadas
shot-duel = <b>Duelo táctico</b> contra la IA de la casa, con líneas de combate claras para cada asignación.
shot-log-alt = Un duelo con el registro de partida abierto, tres hechizos en la pila
shot-log = <b>El registro de partida</b> anota cada paso como un chat; el nombre de una carta abre su vista previa.
shot-ring-alt = Una mesa de cuatro asientos dispuesta en círculo
shot-ring = <b>Las mesas de hasta ocho asientos</b> se disponen solas alrededor del campo de batalla.
shot-lobby-alt = El vestíbulo con el mensaje de una amiga y su invitación a una mesa
shot-lobby = <b>El vestíbulo:</b> mesas abiertas, amigos, invitaciones y un chat cifrado de extremo a extremo.
shot-room-alt = Una sala donde tres sillas vacías las ocupa la IA de la casa
shot-room = <b>Asientos para todos:</b> personas, la IA de la casa en cinco niveles o un modelo de lenguaje.
shot-builder-alt = El constructor de mazos filtrando la reserva en busca de un ángel
shot-builder = <b>El constructor de mazos</b> busca en toda la reserva mientras escribes.

about-eyebrow = Qué es
about-title = Motor de reglas completo y mesa 3D clara
about-engine = Prioridad, pila, capas, efectos de reemplazo, habilidades disparadas, acciones basadas en el estado y combate: el motor en Rust aplica el reglamento completo y calcula las opciones legales exactas en cada paso. El cliente presenta solo esas acciones válidas.
about-table = Campo de batalla, manos, pila, exilio y cementerios en una mesa 3D despejada, con líneas de combate claras y vistas previas instantáneas de las cartas. La ilustración de las cartas llega en streaming desde Scryfall mientras juegas, o puedes cambiar en cualquier momento a una vista nítida con el texto de Oracle.

gw-eyebrow = Online
gw-title = Gateways

open-central = Baylee Central en GitHub

## Download

dl-prerelease = Beta
dl-for = Descargar para { $system }
dl-size = { $size } MB
dl-checksum = SHA-256
dl-signature = Firma
dl-notes = Notas de la versión
dl-unknown = No pudimos reconocer tu sistema a partir de este navegador; elige el tuyo más abajo.
dl-mobile = Todavía no hay versión para móviles ni tabletas. Baylee funciona en un ordenador.
dl-mac-unsure = Este navegador no indica si tu Mac tiene un chip de Apple o uno de Intel. El archivo de arriba es para chips de Apple (M1 y posteriores); el de los Mac con Intel está más abajo.
dl-windows-unsure = En un PC con Windows y procesador ARM (un portátil Snapdragon, por ejemplo), elige el archivo ARM de más abajo.
dl-linux-unsure = Tu navegador no indica tu procesador; se supone x86-64. El archivo ARM está más abajo.
dl-missing = Esta versión no tiene archivo para tu sistema. Las versiones anteriores están en la <releases>página de versiones</releases>.
dl-none = GitHub no respondió en este momento. Todas las versiones están en la <releases>página de versiones</releases>.
dl-yours = Tu sistema
dl-more-files = Más archivos ({ $count })
dl-installer = Instalador
dl-deb = Paquete Debian
dl-archive = Archivo
dl-fallback = Archivo para procesadores antiguos
dl-fallback-help = Para procesadores x86-64 sin AVX2 (anteriores a 2013, más o menos). Las compilaciones principales de Baylee necesitan AVX2 y no arrancan sin él.
plat-windows-x64 = Windows
plat-windows-x64-sub = most PCs: 64‑bit Intel or AMD
plat-windows-arm64 = Windows en ARM
plat-windows-arm64-sub = portátiles Snapdragon y similares
plat-mac-arm64 = macOS
plat-mac-arm64-sub = chips de Apple: M1 y posteriores
plat-mac-x64 = macOS en Intel
plat-mac-x64-sub = Mac con procesador Intel
plat-linux-x64 = Linux
plat-linux-x64-sub = x86-64
plat-linux-arm64 = Linux en ARM
plat-linux-arm64-sub = ARM64 / aarch64
plat-none = Esta versión no tiene archivo para este sistema.
install-title = Primer arranque y requisitos
install-first-title = Primer arranque
install-first = Las compilaciones no están firmadas con código, porque firmar cuesta dinero, así que Windows y macOS avisan la primera vez. En Windows, elige <i>Más información</i> y después <i>Ejecutar de todas formas</i>. En macOS, abre <i>Ajustes del Sistema → Privacidad y seguridad</i> y elige <i>Abrir igualmente</i>.
install-check-title = Comprobar una descarga
install-check = Junto a cada archivo hay un <code>.sha256</code> con su suma de verificación; los archivos comprimidos llevan además un <code>.sig</code>, la firma Ed25519 que el cliente comprueba antes de actualizarse. En Linux, <code>sha256sum -c</code> comprueba un archivo; en macOS, <code>shasum -a 256</code>; en Windows, <code>certutil -hashfile</code>.
install-needs-title = Requisitos
install-windows = <b>Windows</b> 10 u 11, x64 o ARM64.
install-mac = <b>macOS</b> 11 o posterior, con chip de Apple o Intel.
install-linux = <b>Linux</b> con glibc 2.35 o más reciente (Ubuntu 22.04 y posteriores) y un controlador Vulkan; el <code>.deb</code> instala todo lo demás que necesita.
install-cpu = <b>Procesador:</b> las compilaciones x86-64 necesitan AVX2; para procesadores más antiguos hay un archivo aparte.

## Gateways

gws-title = Gateways
gws-official = Oficial
gws-online = Online
gws-offline = Sin conexión
gws-version = Versión
gws-now = Right now
gws-website = Sitio web
gws-address = Dirección
gws-copy = Copiar dirección
gws-copied = Dirección copiada.
gws-copied-short = Copiada
gws-copy-failed = No se pudo copiar.
gws-failed = No se pudo leer el directorio en este momento. Inténtalo de nuevo en un minuto.

## Questions

faq-eyebrow = Preguntas
faq-title = Preguntas y respuestas
faq-what-q = ¿Qué es Baylee?
faq-what-a = Un juego de fans gratuito de Magic: The Gathering para tu ordenador. Tiene un constructor de mazos, un motor de reglas que comprueba cada paso, una mesa para hasta ocho jugadores y rivales controlados por el ordenador. No está hecho, aprobado ni respaldado por Wizards of the Coast.
faq-cost-q = ¿Cuánto cuesta?
faq-cost-a = Nada, y eso no va a cambiar: sin muros de pago, sin funciones de pago, sin publicidad.
faq-account-q = ¿Necesito una cuenta?
faq-account-a = No para jugar solo: el constructor de mazos y las partidas contra la IA de la casa funcionan sin conexión. Para las mesas online necesitas una cuenta de Baylee; durante la beta abierta cualquiera puede crear una en este sitio, sin coste.
faq-data-q = ¿Qué pasa con mis datos?
faq-data-a = Lo menos posible; la <privacy>política de privacidad</privacy> dice exactamente qué. Tu contraseña nunca sale de tu navegador; tus datos privados y tus mensajes de chat se cifran en tu dispositivo antes de llegar a nosotros.
faq-password-q = He olvidado mi contraseña. ¿Qué hago?
faq-password-a = Estableces una contraseña nueva con un código de restablecimiento que te da un administrador; todavía no se puede canjear en este sitio web, llegará más adelante. Tus datos cifrados solo se conservan si tienes el código de recuperación que recibiste al crear la cuenta, o si sigues con la sesión iniciada en el cliente de Baylee. Si no, se pierden.
faq-gateway-q = ¿Qué es un gateway?
faq-gateway-a = Un servidor donde viven las mesas online. Cualquiera puede montar uno; el <gateways>directorio</gateways> lista los que Central conoce.
faq-systems-q = ¿En qué sistemas funciona?
faq-systems-a = Windows, macOS y Linux, en x86-64 y ARM64. Todavía no hay versión para móviles ni tabletas.
faq-open-q = ¿Es de código abierto?
faq-open-a = El juego sí, bajo la GNU Affero General Public License. El código de Baylee Central es público para que cualquiera pueda comprobar cómo trata sus datos.
faq-bug-q = He encontrado un error. ¿Dónde lo comunico?
faq-bug-a = En el juego, pulsa <kbd>F8</kbd> para enviar un informe con todo lo necesario. Si no, en el gestor de incidencias de GitHub, en Discord o con el <contact>formulario de contacto</contact>.

## Contact

contact-eyebrow = Contacto
contact-title = Escríbenos
contact-name = Tu nombre
contact-name-hint = Opcional.
contact-email = Tu dirección de correo electrónico
contact-email-hint = Opcional; solo para poder responderte.
contact-message = Tu mensaje
contact-count = { $n } de 5.000 caracteres
contact-send = Enviar mensaje
contact-sending = Enviando …
contact-sent-title = Gracias
contact-sent = Tu mensaje ha llegado. Si indicaste una dirección, te responderemos allí.
contact-error-empty = Escribe primero un mensaje.
contact-error-email = Esto no parece una dirección de correo electrónico.
contact-error-busy = Demasiados mensajes desde tu red en este momento. Inténtalo de nuevo dentro de una hora.
contact-error-refused = No se aceptó el mensaje. Comprueba su longitud e inténtalo de nuevo.
contact-error-network = No se pudo enviar el mensaje. Comprueba tu conexión e inténtalo de nuevo.
contact-privacy = Lo que envías se conserva para poder responderte y se elimina 90 días después de que lo hayamos atendido, o como máximo al cabo de un año. Más en la <privacy>política de privacidad</privacy>.
contact-noscript = El formulario necesita JavaScript. También puedes escribirnos en Discord o en GitHub.

## Legal notice

impressum-title = Aviso legal
impressum-lead = Información exigida por el § 5 DDG (la ley alemana de servicios digitales). Baylee es un proyecto privado y sin fines comerciales.
impressum-show = Mostrar los datos
impressum-why = Los datos están tras un pequeño rompecabezas que tu navegador resuelve en unos segundos, para mantener alejados a los robots que recogen direcciones. No se te pide nada más.
impressum-solving = Resolviendo el rompecabezas … { $percent } %
impressum-name = Nombre
impressum-address = Dirección
impressum-email = Correo electrónico
impressum-phone = Teléfono
impressum-write = Write an e-mail
impressum-call = Call
impressum-failed = No se pudieron cargar los datos. Inténtalo de nuevo o escríbenos con el <contact>formulario de contacto</contact>.
impressum-noscript = Los datos necesitan JavaScript para mostrarse. Sin él, escríbenos por Discord o GitHub.
impressum-responsible = Responsable del contenido: la persona indicada arriba.
impressum-dispute = No estamos obligados ni dispuestos a participar en procedimientos de resolución de litigios ante una junta arbitral de consumo.

## Fan content

fc-title = Contenido de fans no oficial
fc-eyebrow = Contenido de fans
fc-lead = Baylee es un proyecto de fans. Está permitido por la Política de contenido de fans de Wizards of the Coast, que fija las reglas que cumple.
fc-notice-title = El aviso de cada página
fc-means-title = Qué significa
fc-means = Baylee no está hecho, aprobado ni respaldado por Wizards of the Coast. Magic: The Gathering, los nombres de sus cartas y de sus ediciones, las imágenes de las cartas y los símbolos de maná son propiedad de Wizards of the Coast LLC. Baylee es gratuito para todos: no hay muro de pago, ni funciones de pago, ni registro por correo electrónico.
fc-images = El juego muestra imágenes de cartas de Scryfall, sin alterar y con crédito al artista; este sitio no muestra ninguna. Los datos y las imágenes de las cartas los proporciona Scryfall; Baylee no está afiliado a Scryfall.
fc-policy = <policy>Política de contenido de fans</policy> de Wizards of the Coast

## Privacy notice

privacy-title = Política de privacidad
privacy-lead = Qué datos tratan este sitio y Baylee Central sobre ti, por qué, con qué base jurídica y durante cuánto tiempo.
privacy-updated = A fecha de 11 de noviembre de 2026
privacy-controller-title = Quién es el responsable
privacy-controller = El responsable del tratamiento en el sentido del RGPD (Reglamento General de Protección de Datos) es la persona física indicada en el <impressum>aviso legal</impressum>, que da su nombre, su dirección postal y su correo electrónico. También puedes enviar preguntas sobre privacidad mediante el formulario de contacto. No hay delegado de protección de datos; no es obligatorio.
privacy-short-title = En resumen
privacy-short = Sin seguimiento, sin publicidad, sin analítica, sin cookies salvo que inicies sesión, y nada se carga desde otros servidores: las tipografías, las imágenes y los scripts vienen todos de baylee.app. Tu contraseña nunca nos llega, y tus mensajes de chat y tus datos privados se cifran en tu navegador antes de enviarse.
privacy-hosting-title = Visitar el sitio
privacy-hosting = El sitio funciona en un servidor que alquilamos a netcup GmbH, en Alemania. Cuando abres una página, tu navegador envía lo que lleva toda petición: tu dirección IP, la dirección solicitada, la hora, el idioma de tu navegador y su identificación (user agent). El servidor lo necesita para responder; elige la versión de idioma a partir del idioma, y la página de descarga usa el user agent para sugerir el archivo adecuado. El servidor web no guarda registro de accesos, de modo que estas peticiones no se almacenan. Los mensajes de funcionamiento del propio servidor (su diario del sistema) pueden, en casos raros de error, contener una dirección IP; se eliminan como máximo a los 14 días. Base jurídica: art. 6, apdo. 1, letra f) del RGPD; nuestro interés legítimo es ofrecer el sitio de forma segura y fiable.
privacy-device-title = Lo que guarda tu navegador
privacy-device = Este sitio no instala ninguna cookie hasta que inicias sesión. Lo que tu navegador guarda en tu dispositivo, y solo con JavaScript: tu elección de color, si la hiciste (<code>baylee-site-theme</code>); mientras tienes la sesión iniciada, cuándo termina tu sesión (<code>baylee-central-session</code>), tu clave de chat y las claves de confianza de tus contactos (véase Chat); si activaste las notificaciones, un número aleatorio de este dispositivo (<code>baylee-push-installation-v1</code>, <code>baylee-push</code>); y, solo una vez que hayas iniciado sesión, instalado el sitio como aplicación o activado las notificaciones, una copia de los archivos públicos del sitio (estructura de la página, tipografías, scripts), para que arranque sin conexión y puedan llegar las notificaciones. Si ya no se da ninguna de las tres circunstancias, la copia se elimina la próxima vez que abras el sitio. Nada de esto se nos envía, salvo que un apartado siguiente diga lo contrario. Las entradas del chat y de la sesión se eliminan al cerrar sesión; todo lo demás lo eliminas borrando los datos del sitio en tu navegador. Base jurídica: § 25, apdo. 2, n.º 2 TDDDG (estrictamente necesario para la función que usas); para el número aleatorio de las notificaciones, tu consentimiento (§ 25, apdo. 1 TDDDG).
privacy-github-title = GitHub
privacy-github = La página de descarga y la página de inicio muestran la última versión, el estado de las pruebas y las cifras de las cartas. Es nuestro servidor quien se las pide a GitHub, no tu navegador; GitHub no averigua nada sobre ti con ello. Solo cuando haces clic en una descarga o en un enlace de GitHub se conecta tu navegador a GitHub (GitHub, Inc., EE. UU.) y le envía tu dirección IP y lo que lleva toda petición; a partir de ahí se aplica la política de privacidad del propio GitHub.
privacy-directory-title = El directorio de gateways
privacy-directory = Our server reads the directory of gateways from Baylee Central and shows it; nothing about you is involved. A gateway may tell Central how many players, open tables and games it has at the moment, and Central counts its accounts; the site shows these numbers. They are counts only, never names, and a gateway's numbers are its operator's own statement.
privacy-account-title = Tu cuenta
privacy-account = Para una cuenta, Baylee Central almacena: el identificador de tu cuenta, tu nombre de inicio de sesión, tu nombre visible con su etiqueta, cuándo y cómo se creó la cuenta, un registro de contraseña OPAQUE (que te permite demostrar tu contraseña sin que nuestro servidor llegue a conocerla), tu clave maestra cifrada dos veces (con tu contraseña y con tu código de recuperación), tu documento de cuenta cifrado y tus objetos cifrados (ilegibles para nosotros; solo vemos tamaño, revisión y hora), qué versión de las condiciones aceptaste y cuándo, la hora de tu último inicio de sesión, tus sesiones, un rol de administrador si lo tienes, y los gateways que registras con sus tokens. Los demás jugadores ven tu nombre visible y tu etiqueta, nunca tu nombre de inicio de sesión. Si un administrador te da un código de restablecimiento, solo guardamos su huella hasta que se use el código o caduque. Lo que los administradores hacen con una cuenta o un gateway se conserva en un registro durante tres años (quién, qué, qué cuenta o gateway, cuándo). Base jurídica: art. 6, apdo. 1, letra b) del RGPD (la cuenta que pediste); para la hora del último inicio de sesión y el registro, art. 6, apdo. 1, letra f) del RGPD (nuestro interés legítimo en un funcionamiento seguro y trazable). Sin nombre de inicio de sesión y sin contraseña no hay cuenta; no necesitas correo electrónico. Puedes crear una cuenta a partir de los 16 años. Se conserva hasta que eliminas la cuenta; entonces se borra del sistema en funcionamiento de inmediato y de las copias de seguridad como máximo en 12 meses. Las sesiones terminan tras 30 minutos sin uso y, como máximo, 12 horas después de iniciar sesión.
privacy-cookie-title = La cookie de sesión
privacy-cookie = Al iniciar sesión se instala una cookie, <code>__Host-central</code>: un número aleatorio que identifica tu sesión. Solo se envía a baylee.app, los scripts no pueden leerla y termina con la sesión, como máximo a las 12 horas. Es estrictamente necesaria para mantener tu sesión iniciada; no requiere consentimiento. Base jurídica: § 25, apdo. 2, n.º 2 TDDDG y art. 6, apdo. 1, letra b) del RGPD.
privacy-chat-title = Chat y contactos
privacy-chat = Los mensajes se cifran en tu dispositivo para los dispositivos de cada destinatario; Baylee Central solo los almacena cifrados y no puede leerlos. Lo que Central puede ver y almacena: tus contactos, las solicitudes de contacto y los bloqueos, los grupos con sus nombres, miembros e invitaciones, quién envió un mensaje a quién, cuándo y de qué tamaño, y las claves públicas de tus dispositivos. Tus contactos y los miembros de tus grupos ven tu nombre visible con su etiqueta y tus mensajes. Tu navegador guarda tu clave de chat (sellada con una clave que solo posee este navegador, en su almacenamiento <code>baylee-chat</code>) y las claves de tus contactos en las que confiaste (cifradas, <code>baylee-chat-trust</code>) mientras tienes la sesión iniciada. Los mensajes y las entregas se conservan hasta que eliminas tu cuenta; todavía no hay un plazo de conservación automático más corto. Cada remitente y cada dispositivo guardan como máximo 5.000 mensajes; después se rechazan los nuevos. Base jurídica: art. 6, apdo. 1, letra b) del RGPD.
privacy-push-title = Notificaciones
privacy-push = Si activas las notificaciones en un navegador, tu navegador se registra en su servicio push, y Central almacena la dirección de ese servicio para este navegador, la clave pública y el secreto de autenticación del navegador, tu idioma, un número aleatorio de este dispositivo y tu elección de categorías. El servicio push pertenece al fabricante de tu navegador: Google (Firebase Cloud Messaging), Mozilla o Apple, todos con sede en EE. UU. Una notificación lleva, cifrado, solo su tipo (por ejemplo, «mensaje nuevo») y tu idioma, nunca nombres ni texto; el servicio push ve datos de conexión como la dirección, la hora y el tamaño. Estos datos se eliminan cuando desactivas las notificaciones en este dispositivo, cuando cierras sesión o cuando eliminas tu cuenta; una dirección que no se renueva en 90 días deja de usarse, y las notificaciones en espera caducan al cabo de un día. Base jurídica: art. 6, apdo. 1, letra a) del RGPD y § 25, apdo. 1 TDDDG (tu consentimiento). Puedes retirarlo en cualquier momento en los ajustes de tu cuenta; lo ocurrido hasta entonces sigue siendo lícito.
privacy-counters-title = Protección contra abusos
privacy-counters = Para impedir que se adivinen contraseñas y la sobrecarga, Central cuenta ciertas peticiones por dirección IP (registros, intentos de inicio de sesión, restablecimientos de contraseña, lectura del directorio), solo en memoria: los registros durante un día como máximo, todo lo demás durante una hora como máximo. Antes de un registro, tu navegador también resuelve un rompecabezas; consiste en números aleatorios, no contiene nada sobre ti y se olvida como máximo a los cinco minutos. Por cuenta, Central cuenta las solicitudes de contacto y las búsquedas de otros jugadores, para que nadie inunde a otros de solicitudes. Base jurídica: art. 6, apdo. 1, letra f) del RGPD; nuestro interés legítimo es proteger las cuentas y el servicio contra abusos.
privacy-contact-title = El formulario de contacto
privacy-contact = El formulario envía tu mensaje, el idioma de la página y la hora al servicio de comentarios de Baylee en el mismo servidor, y tu nombre y tu correo electrónico solo si los indicas; ambos son opcionales. El servicio también anota si el mensaje ya se ha atendido. Tu dirección IP no se almacena; solo se cuenta en memoria durante una hora, para impedir una avalancha de mensajes. Solo la persona indicada en el aviso legal lee los mensajes; no se transmite nada. Un mensaje se elimina 90 días después de haberse atendido, como máximo al cabo de un año, o antes si lo pides. Base jurídica: art. 6, apdo. 1, letra f) del RGPD (nuestro interés legítimo en responder a los mensajes), o letra b) cuando tu mensaje se refiere a tu cuenta.
privacy-impressum-title = El rompecabezas del aviso legal
privacy-impressum = The legal notice is shown only after your browser has solved a puzzle; that protects the address from programs that collect addresses. The puzzle consists of random numbers and holds nothing about you. For this, the feedback service keeps in memory only, never on disk and never in a log: how many puzzles your IP address asked for (for one hour; an IPv6 address by its first half), whether it followed a link that is hidden from people (for 24 hours), and which puzzles were solved (for five minutes, without an address). Three yes-or-no answers about your browser set the puzzle's difficulty and are not kept: whether a program drives it, whether it names no languages, and whether it draws without a graphics card. Nobody is refused; a browser that looks like a program gets a longer puzzle. Legal basis: Art. 6(1)(f) GDPR (our legitimate interest in protecting a private person's address).
privacy-recipients-title = Destinatarios y países
privacy-recipients = No vendemos datos ni los cedemos con fines publicitarios. Los destinatarios son: netcup GmbH (alojamiento en Alemania, como encargado del tratamiento conforme al art. 28 del RGPD); los administradores de Baylee Central, que para la administración ven tu nombre de inicio de sesión, tu nombre visible con su etiqueta, el origen, la creación y el último inicio de sesión de tu cuenta, y tus gateways, nunca tus datos cifrados; un gateway en el que inicias sesión con tu cuenta (recibe el identificador de tu cuenta, tu nombre visible y tu etiqueta, no tu contraseña); tus contactos y los miembros de tus grupos (nombre visible, etiqueta y tus mensajes); las autoridades solo cuando la ley nos obliga. Países fuera del Espacio Económico Europeo: solo cuando activas las notificaciones las transporta el servicio push de tu navegador (Google, Mozilla o Apple, EE. UU.). Los gateways gestionados por terceros pueden estar en cualquier país; iniciar sesión en ellos es decisión tuya. Por lo demás, no transferimos datos a países fuera del Espacio Económico Europeo.
privacy-rights-title = Tus derechos
privacy-rights = You have the right of access (Art. 15 GDPR), rectification (Art. 16), erasure (Art. 17), restriction of processing (Art. 18) and data portability (Art. 20). You can object at any time, on grounds relating to your particular situation, to processing based on our legitimate interests under Art. 6(1)(f) (Art. 21). You can withdraw a consent at any time for the future (Art. 7(3)). Your account's data can be exported and the account deleted in your account's settings; for everything else, a message through the contact form or to the address in the legal notice is enough. You may lodge a complaint with a data protection supervisory authority (Art. 77), for example the one where you live or the State Commissioner for Data Protection and Freedom of Information of North Rhine-Westphalia. No law or contract obliges you to give us data; without a sign-in name and a password, however, there is no account. There is no automated decision-making and no profiling (Art. 22).
privacy-objection-title = Tu derecho de oposición
privacy-objection = Cuando tratamos datos sobre la base del art. 6, apdo. 1, letra f) del RGPD (ofrecer el sitio, protección contra abusos, el formulario de contacto, el rompecabezas del aviso legal, la hora de tu último inicio de sesión, el registro de administración, los informes de errores y el recuento de uso), puedes oponerte en cualquier momento por motivos relacionados con tu situación particular (art. 21 del RGPD). Dejaremos entonces de tratar los datos, salvo que podamos acreditar motivos legítimos imperiosos que prevalezcan sobre tus intereses, o que el tratamiento sirva para formular, ejercer o defender reclamaciones. Basta un mensaje informal mediante el formulario de contacto o a la dirección del aviso legal.
privacy-links-title = Enlaces a otros servicios
privacy-links = El sitio enlaza, entre otros, a GitHub, Discord y WhatsApp. Son enlaces simples: no se incrusta nada, y hasta que haces clic en uno, ninguno de estos servicios se entera de tu visita. Cuando haces clic, tu navegador no indica de qué página vienes (sin referrer). Después se aplica la política de privacidad de cada proveedor, y es posible que traten datos fuera de la UE; lo que los demás miembros de los grupos de la comunidad ven de ti allí lo deciden esos servicios.
privacy-reports-title = Informes de errores y recuento de uso desde el juego
privacy-reports = Un informe de error que envías desde el juego con F8 contiene su tipo, tu texto, la versión del cliente y los adjuntos que dejes marcados; la página de contacto los enumera. Se elimina 90 días después de haberse atendido, como máximo a los tres años. Base jurídica: art. 6, apdo. 1, letra f) del RGPD (nuestro interés legítimo en un juego sin errores). El cliente envía informes de fallos solo si los permitiste (art. 6, apdo. 1, letra a) del RGPD; puedes retirarlo en cualquier momento en los ajustes del cliente). Durante una partida sin conexión, el cliente envía una vez por minuto un identificador aleatorio de esa partida y nada más; lo guardamos solo en memoria y lo olvidamos 150 segundos después de su última señal de vida (art. 6, apdo. 1, letra f) del RGPD; se puede desactivar en «Send anonymous usage count»). Como en toda conexión, tu dirección IP nos llega.

## Terms

terms-title = Condiciones de uso
terms-draft = Estas condiciones son un borrador y todavía no están en vigor.
terms-version = Versión { $version }, actualizada el { $date }.
# Shown only in the languages a machine translated: every one but German and English.
machine-note = Una máquina ha traducido este sitio y todavía no lo ha leído ningún hablante de tu idioma. El juego en sí, por ahora, solo está en inglés y alemán. <link>Cuéntanos si ves un error.</link>
legal-machine = Esta página es una traducción automática, ofrecida como ayuda. Si difiere del texto <de>alemán</de> o del <en>inglés</en>, prevalecen esos.
terms-machine = Las condiciones de Central existen solo en alemán y en inglés. A continuación está el texto en inglés; también hay una versión <de>en alemán</de>.

## Errors

not-found-title = Esta página no existe
not-found-lead = Puede que la dirección tenga un error, o que la página se haya movido.
gone-title = Esta página ha desaparecido para siempre
gone-lead = Lo que había aquí se ha eliminado. La página de inicio te indica adónde ir.
error-home = A la página de inicio

## Baylee's remembrance (approved by her family on 27 October 2026; keep word for word)
baylee-dedication = Para Baylee. Siempre bajo el sol.
baylee-invitation = Conoce al alma pequeña detrás de este mundo
baylee-open = Recuerda a Baylee · abre su álbum de fotos
baylee-eyebrow = El corazón de este proyecto
baylee-date = En cariñoso recuerdo · 26 de octubre de 2026
baylee-close = Volver a su mundo
baylee-previous = Fotografía anterior
baylee-next = Fotografía siguiente
baylee-photo1 = Baylee hecha un ovillo en el suave edredón blanco.
baylee-photo2 = Baylee asomándose desde el edredón de flores.
baylee-photo3 = Descansando en su cama redonda y gris.
baylee-photo4 = Baylee dormida, hecha un ovillo.
baylee-photo5 = Un momento tranquilo, acurrucada en el edredón.

baylee-storyTitle = Para Baylee. Siempre bajo el sol.
baylee-together = Con nosotros · 17 de noviembre de 2025 – 26 de octubre de 2026
baylee-chapters = Su historia
baylee-chapterPrevious = Capítulo anterior
baylee-chapterNext = Capítulo siguiente
baylee-photos = Nuestros recuerdos
baylee-read = Desplázate para leer

baylee-chapter1Title = Un hogar para Baylee
baylee-chapter1Body =
    Baylee llegó a casa el 17 de noviembre de 2025.

    Mi mujer y yo la habíamos adoptado del refugio de animales de Krefeld. Se calculaba que tenía unos trece años. Tenía tumores mamarios malignos y el cáncer ya se había extendido al hígado. Baylee era una gata en cuidados paliativos. Sabíamos que nuestro tiempo juntos sería limitado. Con nosotros iba a tener un hogar donde la querrían.

    Pronto nos demostró con qué rapidez iba a hacer suyo ese hogar.

    El primer día se me subió al regazo mientras trabajaba. Y la primera noche rompió una bolsa de comida para gatos que yo había pedido para los gatos de mis padres y se sirvió ella misma. Por supuesto, ya le habían dado su propia comida y había cenado bien. Pero al parecer Baylee había decidido que esa reserva todavía merecía una inspección más de cerca.

    Nuestra ratoncita tenía un apetito asombroso, y una energía increíble.

baylee-chapter2Title = Una confianza que crece
baylee-chapter2Body =
    Cuando notamos que bebía mucha agua, la llevamos al veterinario. Un análisis de sangre reveló un problema de tiroides, que también explicaba su gran hambre y su carácter vivaz. Le dieron medicación para la tiroides y un ciclo temporal de antibióticos porque tenía elevados los marcadores de inflamación.

    Cuando de repente empezó a frotarse contra todo, a maullar mucho y a ponerse inusualmente cariñosa y apegada a nosotros, al principio nos preocupó que estuviera reaccionando mal a alguna de sus medicinas nuevas. Otra revisión dio la explicación: nuestra pequeña estaba en celo. No estaba esterilizada.

    También superamos juntos aquella época. Baylee confiaba cada vez más en nosotros, y cada vez estábamos más unidos.

baylee-chapter3Title = Nuestra pequeña princesa
baylee-chapter3Body =
    Aún recuerdo exactamente lo feliz que se puso cuando le montamos su rascador. Se sentó enseguida en la plataforma más alta y miró satisfecha alrededor del piso. Curiosamente, nunca volvió a usar ese sitio. El porqué siguió siendo su secreto.

    Los demás lugares rara vez quedaban libres: mi regazo, mis piernas y nuestra cama. Allí solía dormir sobre mí o sobre mi mujer. Lo mejor era conseguir tumbarse sobre los dos a la vez.

    Se paseaba por el piso con la cola bien alta, como si supiera perfectamente que allí la princesa era ella. Tenía carácter y sabía dejar claros sus límites. Con nosotros podía ser del todo ella misma. Sus manías, sus exigencias y su pequeña vena testaruda también se querían.

    Nunca antes había creado un vínculo tan profundo con un animal. Baylee encontró un lugar muy hondo en el corazón de mi mujer y en el mío.

baylee-chapter4Title = Horas al sol
baylee-chapter4Body =
    Cuando el tiempo se volvió más cálido, descubrió el balcón. Le encantaba sentir en el pelaje los primeros rayos de sol del día. Enseguida pasaba allí casi el día entero.

    Las palomas y los pájaros del jardín la fascinaban de manera especial. Los observaba con atención, se quedaba al acecho y probaba una y otra vez a sobresaltarlos. Entre tomar el sol y observar pájaros, nuestra princesita tenía una agenda diaria muy completa.

baylee-chapter5Title = Su propia cabecita
baylee-chapter5Body =
    Mientras tanto, su enfermedad seguía avanzando.

    El tumor se inflamó y Baylee no dejaba de lamerse la zona hasta abrirla. En el veterinario le pusieron su primer body de recuperación, uno rojo. Desde entonces tuvo que llevar uno cada día para que no se hiciera más daño en la herida.

    Se acostumbró enseguida a llevarlo. Pero que se lo pusieran fue algo que no le gustó hasta su último día. Ni siquiera un cuenco de comida conseguía convencerla mucho entonces. Por grande que fuera su apetito, su opinión sobre el body era inequívoca.

    La herida empeoró. Supuraba pus, acabó sangrando y desprendía un olor desagradable. Al mismo tiempo, Baylee seguía mostrándonos cuánta alegría encontraba aún en su día a día. Jugaba, comía, exploraba su entorno y trepaba por su rascador y por los armarios.

    A veces se quedaba tumbada horas en el borde de la cama, a nuestro lado, esperando a que por fin nos despertáramos y le preparáramos el desayuno. Si tardábamos demasiado, simplemente nos pasaba por encima. Nuestras caras no eran obstáculo.

    Tenía ideas muy claras sobre la comida. Su gusto era más bien de «pizza congelada»: pasado un tiempo, la comida de calidad para gatos ya no la entusiasmaba y la variedad era imprescindible. Dos veces seguidas lo mismo, ni pensarlo. Pollo con hígado, cualquier cosa con queso, atún o salmón: eso era lo que la hacía feliz.

baylee-chapter6Title = Con la cola bien alta
baylee-chapter6Body =
    El tumor creció y supuraba cada vez más. Baylee sufría visiblemente. Cuando el dolor remitía, volvía a ser nuestra gatita, recorriendo las habitaciones con la cola bien alta y reclamando a sus personas para sí.

    Veíamos cómo sus fuerzas se apagaban poco a poco. Pero su apetito, su curiosidad, su cariño y su sonoro ronroneo seguían ahí.

baylee-chapter7Title = La despedida
baylee-chapter7Body =
    El viernes por la tarde, 23 de octubre de 2026, le aparecieron unos síntomas que al principio parecían un resfriado. Tenía mocos y su respiración era profunda y trabajosa, como si le costara tomar aire.

    El sábado pareció más animada otra vez. La respiración inusual y los mocos persistían, pero volvió a mostrar la energía y las ganas de vivir que tan bien conocíamos.

    La mañana del domingo fue difícil. Le lloraban los ojos, le costaba respirar y tenía dolor. Le di analgésicos. Al cabo de una hora se encontró mejor, pero su respiración seguía siendo distinta. Supimos que teníamos que llevarla al veterinario.

    El lunes por la mañana su estado volvió a empeorar. Por suerte, mi mujer tenía el día libre. Llamamos a la clínica, pedimos cita y llevamos allí a Baylee.

    La radiografía mostró hasta dónde se había extendido el cáncer. Nos dijeron que solo quedaba disponible para respirar aproximadamente un tercio de sus pulmones. Dejarla seguir viviendo habría significado que cada vez le llegaría menos aire.

    Tuvimos que despedirnos.

    El lunes 26 de octubre de 2026, hacia el mediodía, nuestra queridísima Baylee se durmió para siempre.

baylee-chapter8Title = Siempre bajo el sol
baylee-chapter8Body =
    Estuvo con nosotros algo menos de un año. En ese tiempo se había convertido en parte de nuestra familia y en una amiga muy cercana. Pertenecía a nuestros días de trabajo, a nuestras noches y a nuestros despertares por la mañana. A las horas de sol en el balcón y a las pequeñas costumbres que forman una vida en común.

    Ahora falta en todos esos momentos.

    El vacío es todavía reciente y hondo. Echamos de menos su ronroneo, su peso sobre nuestras piernas, su cola erguida y su esperanza incuestionable de que por fin le atendiéramos el desayuno.

    Si hay algo después de esta vida, sea lo que sea, deseo volver a encontrarme algún día con esa alma.

    Hasta entonces, llevaremos a nuestra ratoncita en el corazón para siempre.

    Por eso este proyecto de afición está dedicado a Baylee. A su espíritu, a su cercanía, a su alegría por el sol y los pájaros. Y al amor que creció durante el tiempo que pasamos juntos.

    Aquí puede dormir en paz sobre su nube, bajo el cálido sol, rodeada de flores, palomas y pájaros del jardín.

    Descansa en paz, mi queridísima Baylee.

baylee-photoOpen = Ver la fotografía completa
baylee-photoClose = Cerrar la fotografía
baylee-zoomIn = Acercar
baylee-zoomOut = Alejar
baylee-fit = Ajustar

baylee-photo6 = Baylee, a gusto entre las mantas.
baylee-photo7 = Acurrucada en su body de recuperación rojo.
baylee-photo8 = Un rincón soleado en una caja de cartón.
baylee-photo9 = Bien arropada y completamente relajada.
baylee-photo10 = Su lugar favorito: justo a nuestro lado.

## Errors shared by the forms

error-busy = Demasiados intentos en este momento. Espera un minuto e inténtalo de nuevo.
error-unavailable = Baylee Central no responde ahora mismo. Inténtalo de nuevo en un momento.
error-network = Sin conexión. Comprueba tu red e inténtalo de nuevo.
error-failed = Algo ha salido mal. Inténtalo de nuevo; si sigue ocurriendo, avísanos.
error-again = Reintentar
password-show = Mostrar

## Signing in

signin-title = Iniciar sesión
signin-lead = Tu cuenta de Baylee, para mesas online, contactos y el chat.
signin-name = Nombre de inicio de sesión
signin-password = Contraseña
signin-submit = Iniciar sesión
signin-missing = Introduce tu nombre de inicio de sesión y tu contraseña.
signin-working = Protegiendo tu contraseña … tarda un momento y ocurre solo en tu navegador.
signin-wrong = Ese nombre de inicio de sesión y esa contraseña no coinciden. Revisa ambos e inténtalo de nuevo.
signin-not-moved = Esta cuenta todavía no se ha trasladado a Baylee Central. Inicia sesión una vez con el cliente de Baylee; él traslada tu cuenta.
signin-no-account = ¿Aún no tienes cuenta? <register>Crea una</register>.
signin-noscript = Iniciar sesión necesita JavaScript: tu contraseña se procesa en tu navegador y nunca se nos envía.

## Registering

register-title = Crear una cuenta
register-lead = Para mesas online, contactos y el chat. Gratis y abierta a todos durante la beta abierta.
register-name = Nombre de inicio de sesión
register-name-hint = Privado, solo para iniciar sesión: de 3 a 24 letras minúsculas, dígitos, puntos, guiones o guiones bajos.
register-display = Nombre visible
register-display-hint = Cómo te ven los demás jugadores: de 3 a 16 letras, dígitos, - y _. Tu etiqueta se genera por ti.
register-password = Contraseña
register-password-hint = Al menos 8 caracteres. Una frase larga es fácil de recordar y difícil de adivinar.
register-repeat = Repite la contraseña
register-repeat-hint = Lo mismo otra vez, para detectar una errata.
register-age = Tengo 16 años o más.
register-consent = Acepto las <terms>condiciones de uso</terms>. Cómo tratamos tus datos está en la <privacy>política de privacidad</privacy>.
register-submit = Crear cuenta
register-error-name = El nombre de inicio de sesión necesita de 3 a 24 letras minúsculas, dígitos, puntos, guiones o guiones bajos.
register-error-display = El nombre visible necesita de 3 a 16 letras o dígitos; - y _ solo en el interior.
register-error-reserved = Este nombre visible está reservado. Elige otro.
register-error-short = La contraseña necesita al menos 8 caracteres.
register-error-long = La contraseña es demasiado larga.
register-error-repeat = Las dos contraseñas no son iguales.
register-error-age = Necesitas tener 16 años o más para tener una cuenta.
register-error-consent = Acepta las condiciones de uso para crear una cuenta.
register-error-taken = Este nombre de inicio de sesión ya está en uso. Elige otro.

## The recovery code

code-title = Tu código de recuperación
code-handle = Tu identificador: { $handle }
code-lead = Solo con este código se conservan tus datos cifrados si olvidas tu contraseña. No lo conocemos y no podemos restaurarlo. Guárdalo en un lugar seguro, lejos de este dispositivo.
code-shown-once = Se muestra solo ahora
code-download = Guardar como archivo de texto
code-kept = He guardado mi código de recuperación
code-not-kept = Guarda primero el código y luego confírmalo.
code-continue = Continuar
code-later = Ahora no

## Central's terms

terms-step-title = Nuevas condiciones de uso
terms-step-lead = Las condiciones de Baylee Central han cambiado. Léelas y acéptalas para usar tu cuenta; también puedes rechazarlas y cerrar sesión.
terms-accept = Aceptar
terms-decline = Rechazar y cerrar sesión

## The signed-in area

account-lead = Tu cuenta de Baylee: perfil, seguridad, privacidad y gateways.
account-nav = Tu cuenta
account-home = Resumen
account-profile = Perfil
account-profile-lead = Tu identificador, tu nombre visible y tu código de recuperación.
account-security = Seguridad
account-security-lead = Dónde tienes la sesión iniciada, y cerrar sesión.
account-privacy = Privacidad y datos
account-privacy-lead = Exporta tus datos o elimina tu cuenta.
account-gateways = Gateways y tokens
account-gateways-lead = Los gateways que registraste y los tokens con los que inician sesión.
account-loading = Cargando …
account-signed-out = No has iniciado sesión.
account-unnamed = Tu cuenta
account-signin-name = Nombre de inicio de sesión: { $name }
account-until = Sesión iniciada hasta { $time }
account-needs-title = Requiere tu atención
account-needs-code = Crea un código de recuperación
account-needs-name = Elige tu nombre visible
account-admin = Administración

reauth-lead = Esto requiere un inicio de sesión reciente. Introduce otra vez tu nombre de inicio de sesión y tu contraseña.
reauth-submit = Iniciar sesión de nuevo

profile-who-title = Quién eres
profile-handle = Identificador
profile-signin-name = Nombre de inicio de sesión
profile-source = Cuenta
profile-source-moved = Trasladada desde el gateway oficial
profile-source-registered = Registrada en este sitio
profile-choose-title = Elige tu nombre visible
profile-choose-submit = Guardar nombre visible
profile-name-taken = Esta cuenta ya tiene un nombre visible.
profile-code-title = Código de recuperación
profile-code-kept = Tu cuenta tiene un código de recuperación. Guárdalo bien; sin tu contraseña es la única vía para volver a tus datos cifrados.
profile-code-missing = Tu cuenta aún no tiene código de recuperación. Crea uno; necesitarás tu contraseña otra vez.
profile-code-make = Crear un código de recuperación

security-sessions = Dónde tienes la sesión iniciada
security-lead = Baylee Central mantiene una sesión para este sitio web y otra para el cliente del juego; no distingue entre dispositivos concretos.
security-kind-browser = Este sitio web
security-kind-client = El cliente de Baylee
security-this = Esta
security-signed-in = Sesión iniciada
security-ends = Termina
security-gateways = En gateways
security-end-client = Cerrar la sesión del cliente
security-client-ended = La sesión del cliente está cerrada.
security-sign-out = Cerrar sesión aquí
security-password-title = Contraseña
security-password-later = Cambiar la contraseña en el sitio web llegará más adelante; hasta entonces, cámbiala en el cliente de Baylee.

privacy-export-title = Exportar tus datos
privacy-export-lead = Lo que Baylee Central sabe de tu cuenta, como archivo JSON: sus datos básicos, sesiones, condiciones aceptadas, tus gateways y las entradas del registro de auditoría que nombran tu cuenta. Tus contenidos cifrados solo puede descifrarlos y exportarlos tu cliente de Baylee. La exportación necesita un inicio de sesión de hace menos de 30 minutos.
privacy-export-make = Preparar la exportación
privacy-export-save = Guardar la exportación
privacy-delete-title = Eliminar tu cuenta
privacy-delete-lead = Tu cuenta, sus datos, sus sesiones, tus contactos y los gateways de tu propiedad se eliminan del sistema en funcionamiento de inmediato y para siempre; salen de las copias de seguridad como máximo en 12 meses. Central elimina también los mensajes que enviaste; lo que otros ya hayan guardado en sus dispositivos puede quedarse allí.
privacy-delete-start = Eliminar mi cuenta
privacy-delete-password = Tu contraseña, para confirmar
privacy-delete-prove = Continuar
privacy-delete-last = Último paso: esto no se puede deshacer. Tienes un minuto.
privacy-delete-gateways = Estos gateways de tu propiedad se eliminan con ella:
privacy-delete-confirm = Eliminar la cuenta para siempre
privacy-delete-cancel = Conservar mi cuenta
privacy-deleted = Tu cuenta está eliminada.

gateways-none = No has registrado ningún gateway. La mayoría de los jugadores nunca necesita uno.
gateways-add-title = Registrar un gateway
gateways-origin = Dirección del gateway
gateways-origin-hint = La dirección https del gateway, como https://play.example.org.
gateways-name = Nombre del gateway
gateways-add = Registrar gateway
gateways-added = El gateway está registrado y espera la aprobación de un administrador.
gateways-taken = Esta dirección ya está registrada.
gateways-invalid = Introduce una dirección https sin ruta y un nombre.
gateways-approved = Aprobado
gateways-pending = Pendiente de aprobación
gateways-disabled = Desactivado
tokens-title = Tokens
tokens-name = Nombre del token
tokens-days = Válido durante (días)
tokens-sso = Inicia la sesión de los jugadores
tokens-listing = Lista el gateway
tokens-both = inicia la sesión de los jugadores y lista el gateway
tokens-make = Crear token
tokens-revoke = Revocar
tokens-revoked = Revocado
tokens-revoked-now = El token está revocado.
tokens-until = Hasta { $time }
tokens-invalid = Ponle un nombre al token y al menos una cosa que permita.
tokens-shown-title = Tu nuevo token
tokens-kept = Lo he guardado
tokens-again = Sesión iniciada de nuevo. Crea el token otra vez.

## The messenger

account-chat = Chat
account-contacts = Contactos
chat-side = Conversaciones y contactos
chat-tabs = Mensajería
chat-tab-chats = Chats
chat-tab-contacts = Contactos
chat-main = Conversación
chat-unread = sin leer
chat-conversations = Tus conversaciones
chat-search = Buscar una conversación
chat-search-hint = Buscar (Ctrl+K)
chat-new-group = Grupo nuevo
chat-no-conversations = Aún no hay conversaciones. Añade un contacto y escríbele.
chat-no-match = Ninguna conversación tiene ese nombre.
chat-former-group = Un grupo anterior
chat-former-lead = Ya no estás en este grupo.
chat-you = Tú
chat-members = { $n ->
    [one] { $n } miembro
   *[other] { $n } miembros
}
chat-say-hello = Saluda.
chat-history-only = Ya no es un contacto.
chat-today = Hoy
chat-yesterday = Ayer
chat-encrypted = Cifrado de extremo a extremo
chat-opening-lead = Tus conversaciones y contactos. Los mensajes se cifran en tus dispositivos antes de salir de ellos, y Central los transmite sin poder leerlos.
chat-new-from = Mensaje nuevo de { $who }.
chat-new-many = { $n } mensajes nuevos.
chat-retry = Reintentar
chat-discard = Descartar
chat-not-sent = No enviado.
chat-sending = Enviando …
chat-keys-changed = Las claves de alguien en esta conversación han cambiado. Compara el número de seguridad antes de volver a escribir.
chat-refused = Algunos mensajes no se pudieron verificar y no se muestran.
chat-failed = El chat no pudo conectar con Central en este momento.
chat-failed-open = No se pudo abrir el chat. Recarga la página para intentarlo de nuevo.
chat-empty = Aún no hay mensajes.
chat-older = Mostrar mensajes anteriores
chat-compare-first = Las claves han cambiado. Compara primero el número de seguridad y luego escribe.
chat-compare = Comparar el número de seguridad
chat-friend-required = Podrás escribir cuando volváis a ser contactos.
chat-group-left = Ya no estás en este grupo.
chat-details = Detalles
chat-close = Cerrar
chat-back = Volver a las conversaciones
chat-messages = Mensajes
chat-fan = Baylee es contenido de fans no oficial, no aprobado ni respaldado por Wizards. <more>Qué significa</more>
chat-safety = Números de seguridad
chat-safety-lead = Compara el número con la otra persona, en persona o por otro canal de confianza. Si coincide, nadie se ha colado entre vosotros sin que se note.
chat-changed = Cambiado
chat-confirm-keys = Coincide: confiar en las claves nuevas
chat-count = { $n } de 1.000 caracteres
chat-write = Escribe un mensaje
chat-send = Enviar
chat-enter-hint = Intro envía, Mayús+Intro inicia una línea nueva.
chat-choose = Elige una conversación
chat-choose-lead = Tus mensajes se cifran en este navegador; Central solo los transmite.
chat-storage = Tu clave de chat permanece en este navegador mientras tienes la sesión iniciada.
chat-opening = Abriendo tus chats …
chat-open = Abrir mis chats
chat-ended = Tu sesión ha terminado.
chat-unnamed = Elige primero tu nombre visible: te da el identificador al que escriben los demás.
chat-missing-title = Inicia sesión de nuevo para abrir tus chats
chat-missing-lead = Este navegador no tiene tu clave de chat. Tu contraseña la abre aquí; nunca sale de este navegador.
chat-invites = Invitaciones a grupos
chat-invited-by = de { $who }
chat-group-name = Nombre del grupo
chat-group-name-hint = Un nombre de 1 a 48 caracteres.
chat-group-create = Crear
chat-group-failed = No se pudo modificar el grupo en este momento.
chat-group-members = { $n ->
    [one] { $n } miembro
   *[other] { $n } miembros
}
chat-group-owner = Propietario
chat-group-rename = Renombrar
chat-group-transfer = Hacer propietario
chat-group-remove = Quitar
chat-group-transfer-ask = ¿Hacer a { $who } propietario de este grupo? Tú sigues siendo miembro.
chat-group-remove-ask = ¿Quitar a { $who } de este grupo?
chat-group-invite = Invitar a un contacto
chat-group-choose = Elige un contacto
chat-group-invite-button = Invitar
chat-group-invited = Invitado
chat-group-delete = Eliminar grupo
chat-group-leave = Salir del grupo
chat-group-delete-ask = ¿Eliminar «{ $name }» para todos? Los mensajes ya recibidos siguen siendo legibles.
chat-group-leave-ask = ¿Salir de «{ $name }»? Solo podrás volver con una invitación nueva.

people-incoming = solicitudes
people-failed = No se pudieron cargar tus contactos en este momento.
people-empty = Aún no hay contactos. Busca a alguien por su identificador, o dale el tuyo.
people-your-handle = Tu identificador:
people-incoming-title = Solicitudes para ti
people-friends = Contactos
people-outgoing = Solicitudes enviadas
people-blocked = Bloqueados
people-find = Añadir un contacto por su identificador
people-find-button = Buscar
people-handle-hint = Un identificador completo con su etiqueta, como Name#1a2b.
people-yourself = Ese es tu propio identificador.
people-nobody = Nadie tiene este identificador.
people-request = Enviar solicitud de contacto
people-requested = Solicitud enviada.
people-is-friend = Un contacto
people-is-incoming = Quiere ser tu contacto
people-is-outgoing = Solicitud enviada
people-is-blocked = Bloqueado
people-write = Escribir
people-remove = Quitar
people-block = Bloquear
people-accept = Aceptar
people-decline = Rechazar
people-cancel = Retirar
people-unblock = Desbloquear
people-keep = Conservar
people-confirm-remove = ¿Quitar a { $who } de tus contactos? Tus mensajes se quedan.
people-confirm-block = ¿Bloquear a { $who }? Ya no podrá pedir ser tu contacto ni escribirte.

## Offline, updates, installing

title-offline = Sin conexión
offline-title = Estás sin conexión
offline-lead = Esta página necesita conexión. Central guarda tus mensajes hasta que vuelvas.
offline-again = Reintentar
update-ready = Hay una versión nueva de baylee.app lista.
update-unsent = Tienes aquí texto que aún no se ha enviado. ¿Recargar de todos modos?
update-reload = Recargar
install-app = Instalar como aplicación

## Notifications

push-title = Notificaciones
push-lead = Elige sobre qué puede avisarte Baylee. Las notificaciones no muestran texto de mensajes, nombres ni contenido de informes.
push-device = Notificaciones en este dispositivo
push-categories = Avisarme de
push-chat = Mensajes de chat
push-contacts = Solicitudes de contacto
push-updates = Versiones nuevas
push-feedback = Informes de comentarios (administradores)
push-gateways = Solicitudes de gateways (administradores)
push-on = Activadas en este dispositivo. Solo se entregan las categorías seleccionadas.
push-off = Desactivadas en este dispositivo. Se conservan tus elecciones de categorías.
push-scope = Las elecciones de categorías se aplican a tu cuenta. El permiso del dispositivo es independiente en cada navegador. Al cerrar sesión se desconecta este dispositivo.
push-failed = No se pudo confirmar el cambio. Inténtalo de nuevo.
push-denied = Las notificaciones están bloqueadas. Puedes permitirlas en los ajustes de tu navegador o dispositivo e intentarlo de nuevo.
push-unsupported = Este navegador no ofrece Web Push aquí. En iPhone y iPad, añade Baylee a la pantalla de inicio y ábrelo desde allí.
push-unconfigured = Las notificaciones en segundo plano todavía no están disponibles en este servidor. Tus preferencias ya se pueden guardar.

register-closed = El registro está en pausa ahora mismo. Inténtalo más tarde.
register-refused = No se aceptó el registro. Inténtalo de nuevo.
admin-registration = Registro
admin-registration-open = Abierto: cualquiera puede crear una cuenta tras resolver la prueba de trabajo.
admin-registration-closed = Cerrado: nadie puede crear una cuenta hasta que lo abras de nuevo.
admin-registration-lead = Cierra el registro durante un ataque y ábrelo de nuevo a mano. Cada cambio queda anotado en el registro de auditoría.
admin-registration-close = Cerrar el registro
admin-registration-reopen = Abrir el registro de nuevo
hero-discord = Comunidad de Discord
hero-whatsapp = Comunidad de WhatsApp
hero-github = GitHub
nav-menu = Menú
foot-project = Proyecto
foot-source = Código fuente (AGPL-3.0)
foot-central = Baylee Central (código disponible)
foot-summary = Juego de cartas de mesa gratuito y de código abierto, creado en Rust y Bevy, con un motor de reglas determinista, modo sin conexión completo, IA de la casa, puente con LLM y gateways de la comunidad.
strip-build = Compilación
strip-ci = CI
strip-commit = Commit
strip-gateways = Gateways
strip-online = online
strip-offline = sin conexión
strip-releases = ver versiones
strip-on-github = ver en GitHub
strip-directory = el directorio
strip-cards = Cartas jugables
strip-cards-unit = cartas
strip-stars = Estrellas en GitHub
strip-accounts = Accounts at Baylee Central
strip-live = Right now on the gateways, as they report it
live-accounts = { $n ->
    [one] { $n } account
   *[other] { $n } accounts
}
live-players = { $n ->
    [one] { $n } player
   *[other] { $n } players
}
live-tables = { $n ->
    [one] { $n } open table
   *[other] { $n } open tables
}
live-games = { $n ->
    [one] { $n } game
   *[other] { $n } games
}
hero-badges = De un vistazo
hero-badge-beta = Beta abierta
hero-badge-free = Gratis
hero-badge-open = Código abierto
hero-badge-fan = Contenido de fans no oficial
gws-open-client = Abrir en Baylee
gws-open-client-help = «Abrir en Baylee» inicia el cliente instalado (0.1.1-beta.9b o posterior). Muestra la dirección y pregunta antes de añadir el gateway.
dl-menu-title = Todos los sistemas
dl-old-cpu = procesadores antiguos (sin AVX2)
ci-passing = correcto
ci-failing = con fallos
dl-button = Descargar Baylee
dl-quick = Otros sistemas y arquitecturas
dl-others = otros sistemas

# The start page, as the old site wrote it (owner, 11.11.2026)
link-readmeModel = #playing-against-a-language-model
about-engineTitle = Un motor que hace cumplir cada regla
about-deterministic = Gracias a un generador aleatorio con semilla y sin dependencias del reloj ni del orden de los hashes, cada partida es estrictamente determinista y reproducible jugada a jugada.
about-tableTitle = Una mesa 3D limpia creada con Bevy
about-keys = Desde el vestíbulo integrado pasas sin esfuerzo entre el constructor de mazos local, las partidas en solitario y las mesas multijugador en los gateways de la comunidad.
feature-hiddenTitle = La información oculta sigue siendo privada
feature-hidden = Tu cliente recibe solo los datos que tu asiento puede ver. Las bibliotecas y las manos de los rivales llegan estrictamente como cantidades; ni siquiera un cliente modificado puede inspeccionar las cartas ocultas.
feature-seatsTitle = Hasta ocho jugadores en una mesa
feature-seats = Duelos, todos contra todos y partidas por equipos (como 2 contra 2) con hasta 8 asientos. Los compañeros comparten turnos y pueden mostrarse las cartas de la mano.
feature-commanderTitle = Formatos flexibles y reglas de la casa personalizadas
feature-commander = Assemble decks for classic 60‑card duels, multiplayer rings, or custom house formats. Starting life totals, team layouts, and mulligan rules are freely set by the host.
feature-aiTitle = Repeticiones deterministas e informes con F8
feature-ai = Cada partida se registra jugada a jugada. Si durante el juego notas un problema en una interacción de reglas, pulsa F8 en la propia mesa para enviar un informe de estado reproducible.
feature-processTitle = Un proceso de servidor aislado por partida
feature-process = Cada partida activa se ejecuta en su propio proceso de motor aislado en el gateway. El gateway solo gestiona el vestíbulo y las salas; un error nunca afecta a otras mesas.
feature-languagesTitle = Texto de las cartas en 19 idiomas
feature-languages = Cuando un gateway ofrece el catálogo multilingüe de cartas, las cartas se leen en tu idioma preferido con el inglés como alternativa. La interfaz del cliente está disponible en alemán y en inglés.
feature-watchTitle = Presencia en partidas en directo
feature-watch = El botón Watch del vestíbulo te sienta a la mesa como espectador en directo: sigue el campo de batalla público en tiempo real mientras las manos de todos los jugadores permanecen ocultas.
feature-connectionTitle = Reconexión fluida si se corta la conexión
feature-connection = Si una conexión se corta un momento, la mesa mantiene el asiento con una cuenta atrás y puede pausar la partida o dejar que la IA de la casa entre temporalmente.
feature-tablesTitle = Catorce relojes de decisión
feature-tables = From brisk 15‑second turns to relaxed untimed tables, configure the pace individually for every room.
shots-eyebrow = En la mesa
shots-title = Dentro del juego
shots-lead = Short live recordings captured directly from the 3D client with card images from Scryfall (© Wizards of the Coast)—from duels and 4‑player rings to the encrypted lobby and deck builder.
shots-credit = Imágenes de cartas: Scryfall · © Wizards of the Coast
shots-showLarge = Ampliar: { $title }
shots-dialog = Captura de pantalla
shots-previous = Anterior
shots-next = Siguiente
shots-close = Cerrar
ai-eyebrow = Rivales de IA y modelos de lenguaje
ai-title = IA de la casa, puente con LLM e IA nativa de Baylee
ai-houseTitle = IA heurística de la casa en cinco niveles de habilidad
ai-house = Lista para jugar sin conexión y en cualquier mesa online: elige entre Novato, Casual, Constante, Afilado y Experto. La IA de la casa decide en local sin tiempo de espera y ve solo el mismo estado público que un jugador humano.
ai-modelTitle = Modelos de lenguaje en la mesa mediante Seat-Bridge
ai-model = Sienta en tu mesa a <b>Google Gemini, OpenAI ChatGPT, Anthropic Claude, DeepSeek</b> o a modelos locales con <b>LM Studio</b>. Las claves de API quedan protegidas en variables de entorno con límites de gasto; si se agota el tiempo, la IA de la casa interviene sin cortes.
ai-setup = Guía de configuración de LLM y Seat-Bridge
home-gwLead = Las mesas online viven en gateways, servidores que Baylee Central lista una vez que su administrador los ha aprobado. Aquí están los primeros; el directorio los tiene todos.
beta-open = Abrir el directorio
beta-own = Monta el tuyo
source-eyebrow = Código fuente
source-title = Código abierto, y así seguirá
source-promise = Baylee será siempre de código abierto y gratuito. Sin muros de pago, sin funciones de pago, sin anuncios. Ni ahora ni más adelante. El código está bajo la GNU Affero General Public License, lo que significa que quien ejecute un gateway modificado para otras personas también debe ofrecerles su código fuente.
source-rust = Está escrito en Rust, es el proyecto de una sola persona con mucha ayuda de agentes de programación, y cada cambio pasa por la misma puerta: formato, lints, el conjunto de pruebas y una validación de cada carta con su texto impreso. Las incidencias y las pull requests son bienvenidas; <agents>AGENTS.md</agents> y <claude>CLAUDE.md</claude> son las normas de trabajo tanto para personas como para agentes.
source-discord = Comunidad en Discord
source-whatsapp = Comunidad en WhatsApp
source-architecture = Arquitectura
source-repoTitle = El repositorio ahora mismo
source-stars = Estrellas
source-issues = Incidencias y pull requests abiertas
source-note = El servidor de este sitio lo lee de la API pública de GitHub cada diez minutos. Tu navegador no le pregunta nada a GitHub.
gw-count =
    { $n ->
        [one] { $n } gateway listado, { $online } online.
       *[other] { $n } gateways listados, { $online } online.
    }
gw-none = Todavía no hay ningún gateway listado.
contact-intro = Una pregunta, una idea, un juego que te gustaría ver o algo que prefieres no publicar en abierto: este formulario va directo a la persona que está detrás de Baylee. Deja una dirección de correo electrónico si quieres respuesta; sin ella solo podemos leerlo.
contact-f8 = Lo mejor para informar de una carta que se resolvió mal es hacerlo desde el juego con <kbd>F8</kbd>, porque el informe puede llevar toda la partida.
contact-discord = charlar, encontrar rivales, enterarte de las compilaciones
contact-issues = Incidencias de GitHub
contact-issuesSub = errores e ideas, en público
contact-operator = Quién gestiona este sitio, con dirección postal: el <impressum>aviso legal</impressum>.
contact-whatsappSub = Grupo de la comunidad y partidas rápidas
contact-reportsTitle = Informes y privacidad
contact-reportsSub = Detalles sobre los informes F8 y los datos
ai-housePill = Sin conexión y online
ai-providers = Modelos de lenguaje compatibles
ai-hostsPill = Para anfitriones de gateways
ai-hostsTitle = Ofrece asientos de LLM alojados en tu gateway
ai-hosts = Como operador de un gateway, puedes ofrecer modelos de lenguaje preconfigurados directamente en tu servidor. Los invitados de una mesa simplemente eligen uno de tus modelos alojados como rival, sin necesidad de clave de API.
ai-trainingPill = En entrenamiento · IA nativa del motor
ai-trainingTitle = IA nativa de Baylee: entrenada para jugar directamente sobre el motor como un jugador de verdad
ai-training = Junto a la IA de la casa y el puente externo con LLM, se está entrenando una IA nativa de Baylee propia directamente sobre el motor de reglas determinista. Aprende a evaluar estados del tablero, ventanas de prioridad, cálculos de combate y sinergias de mazo como un jugador humano con experiencia, sin latencia de ninguna API externa.
ai-hostingLink = Alojamiento de gateways y configuración de LLM

# The gateways and the contact page, as the old site wrote them (owner, 11.11.2026)
gw-lead = Un gateway es un servidor donde viven las mesas online: cuentas, mazos, salas y un proceso de motor por partida. Estos son los registrados en Baylee Central y aprobados por su administrador. Baylee gestiona el marcado como oficial; los demás los gestionan otras personas para sus grupos, y allí se aplican sus propias reglas. Que aparezca en la lista no es una promesa sobre lo que hace un gateway.
gw-howTo = Para jugar en uno, copia su dirección en la puerta de entrada del cliente. Online significa que Central ha tenido noticias del gateway en el último minuto y medio; no es una promesa de que responda. Los nombres y las versiones son los que comunica cada operador.
gw-find = Buscar un gateway
gw-findHint = Nombre o dirección
gw-onlineOnly = Solo online
gw-shown = { $n } de { $all } mostrados
gw-noMatch = Ningún gateway cargado hasta ahora coincide.
gw-showAll = Mostrar todos
link-readmeGateway = #running-your-own-gateway
selfhost-eyebrow = Autoalojamiento e integración con Central
selfhost-title = Monta tu propio gateway
selfhost-processes = Un gateway autoalojado ejecuta tres procesos ligeros junto a PostgreSQL 18+: <code>baylee-gateway</code> gestiona salas, mazos y enrutamiento; <code>baylee-agent</code> lanza un proceso <code>baylee-engine-server</code> aislado por partida, de modo que cada mesa queda en un entorno aislado.
selfhost-proxy = Coloca un proxy inverso TLS delante de tu origen HTTPS (p. ej. <code>https://play.example.org</code>), importa opcionalmente el catálogo masivo de Scryfall para tener el texto de las cartas en 19 idiomas, y los jugadores podrán conectarse directamente con tu dirección en el cliente.
selfhost-central = <b>Vincúlalo con Baylee Central y configura la privacidad:</b> registra tu origen HTTPS en <mine>Tus gateways</mine> y genera un token <code>bgt_…</code> (ámbitos: <code>sso</code> para el inicio de sesión único, <code>listing</code> para los latidos del directorio y la telemetría en directo). En la consola de tu gateway, elige tu nivel de privacidad de la telemetría (<code>full</code> para partidas en curso, ajustes de mesa y jugadores que lo han aceptado, <code>counts_only</code> solo para totales, o <code>private</code> solo para saber que está activo).
selfhost-guide = <guide>Lee la guía completa de autoalojamiento</guide> · <protocol>Referencia del protocolo y de los latidos (docs/protocol.md)</protocol>
selfhost-source = Todos los gateways responden a <code>GET /source</code> con su licencia, versión y commit sin necesidad de iniciar sesión, cumpliendo automáticamente el requisito de divulgación del código fuente de la AGPL.
reports-eyebrow = Informes
reports-title = ¿Has encontrado algo que falla? Cuéntanoslo
reports-f8 = Pulsa <kbd>F8</kbd> en la mesa o en el vestíbulo. Un informe lleva su tipo, tu texto, la versión del cliente y, en una mesa, el identificador de la partida. Todo lo demás tiene su propia casilla, y todas las casillas empiezan marcadas, así que desmarca lo que prefieras no enviar antes de pulsar Send: sistema y hardware, la mesa tal como la ves, tu registro (con los demás jugadores renombrados como «Player A», «Player B», …), tus ajustes, una imagen solo de la ventana de Baylee y, en una mesa, el registro completo de la partida (cada jugada y las cartas de cada asiento, sin nombres). <i>Show what is sent</i> (mostrar lo que se envía) muestra cada byte. Los tokens de sesión nunca se envían.
reports-usage = Mientras juegas sin conexión, el cliente avisa una vez por minuto de que hay una partida en marcha: sin cuenta, sin identificador de dispositivo, nada sobre la partida. <i>Send anonymous usage count</i> (enviar recuento de uso anónimo), en Settings › Privacy & Data (Ajustes › Privacidad y datos), lo desactiva.
reports-discord = ¿Quieres charlar, encontrar rivales o enterarte antes de las compilaciones nuevas? Únete a la <discord>comunidad de Baylee en Discord</discord>.
reports-issues = Dudas de reglas, comportamiento erróneo de cartas, ideas: <issues>abre una incidencia en GitHub</issues>. Una carta que se resolvió mal es el informe más útil que podemos recibir, porque cada carta tiene una prueba y la tuya se convierte en una.
reports-privacy = <docs>docs/privacy.md</docs> enumera todo lo que guardan el cliente, el gateway y el servicio de comentarios. Este sitio no instala ninguna cookie salvo que inicies sesión, no usa analítica y no carga tipografías ni scripts del servidor de nadie más. <notice>La política de privacidad</notice> dice qué guardan el sitio y Baylee Central, y nombra cada otro host con el que se comunican, y cuándo.

# The download page, as the old site set it (owner, 11.11.2026)
dlPage-eyebrow = Consíguelo
dlPage-filesEyebrow = Todos los archivos
dlPage-filesTitle = Todos los sistemas
install-eyebrow = Instalación
install-releases = Todas las versiones en GitHub
release-notes = Notas de la versión y sumas de verificación
release-all = Todas las versiones

# The download page's head, as the old site wrote it
dlPage-title = Descarga
dlPage-lead = Cada descarga va directamente al archivo en GitHub, sin nada de por medio. Cuando una versión tiene instaladores, son el camino fácil; los archivos comprimidos contienen el mismo juego para descomprimirlo donde quieras. Un Baylee instalado se actualiza solo desde los archivos firmados allí donde pueda escribir en su propia carpeta. Cada paquete lleva el puente de asientos para modelos de lenguaje junto al juego.

# The reading pages' eyebrows, as the old site set them
imp-eyebrow = Aviso legal
terms-eyebrow = Condiciones de uso
privacy-eyebrow = Privacidad

# The privacy notice's contents, as the old site named them
privacy-contents = Contenido

# The account pages' eyebrow, as the old site set it
acct-eyebrow = Baylee Central

# The account area's navigation, as the old site named it
app-account = Tu cuenta

# ==== The security page (/security) ====================================
# Every sentence and every arrow of this page maps to a row of the claim
# table in docs/security-page.md; a claim changes
# there first. No "safe", "unbreakable" or compliance words: mechanisms only.
title-security = Security in Baylee: passwords, keys and chats
desc-security = How Baylee handles passwords, keys and chat messages, step by step in four diagrams, with the limits each one has.
nav-security = Security
sec-eyebrow = Security
sec-title = Passwords, keys and chats: what goes where
sec-lead = Four flows, step by step: what leaves your device, what Baylee Central sees and what it does not. With the limits each one has.
sec-toc-label = On this page
sec-figure = Diagram in steps: { $name }
sec-play = Play
sec-all = All steps
sec-step = Step { $n }
sec-hint = The diagram plays by itself. Choose a step to stop it there.
sec-hint-still = Choose a step to highlight it.
sec-in-browser = The dashed frame is your browser: all of this happens inside it.
# The actors' names are the only words inside the drawings, where text does
# not wrap: one or two words each, at most about twice the English width
# (the test `names_fit_with_room_to_grow` measures every language).
sec-actor-device = Your device
sec-actor-central = Central
sec-actor-page = Page
sec-actor-worker = Worker
sec-actor-store = Storage
sec-actor-you = You
sec-actor-contact = Contact
sec-actor-website = Website
sec-actor-client = Client

sec-signin-eyebrow = Signing in and registering
sec-signin-title = Central checks your password without receiving it
sec-signin-lead = Baylee signs you in with OPAQUE (RFC 9807). Your device and Central exchange three messages; your password is in none of them, not even as a plain hash.
sec-signin-c1 = Blinded request
sec-signin-t1 = Your device blinds the password.
sec-signin-s1 = It combines the password with a fresh random number and sends only the result. The password cannot be read from that message.
sec-signin-c2 = Central's answer
sec-signin-t2 = Central answers with a key of its own.
sec-signin-s2 = Central computes on the blinded value with a key only it holds and adds its half of a key exchange. It does not learn the password in doing so.
sec-signin-c3 = Argon2id · 64 MiB
sec-signin-t3 = Your device does the heavy work.
sec-signin-s3 = It removes the blinding and stretches the result with Argon2id (64 MiB of memory, three passes); Central cannot ask for cheaper settings. A wrong password is noticed here, before anything else is sent.
sec-signin-c4 = Proof
sec-signin-t4 = Your device proves it has the right keys.
sec-signin-s4 = With the right password your device arrives at the same session key as Central and sends a short proof of that (64 bytes).
sec-signin-c5 = Session + wrapper
sec-signin-t5 = Central opens the session.
sec-signin-s5 = Back come the session (in the browser a cookie the page itself cannot read) and your master key in a wrapper that opens only on your device, with the right password.
sec-signin-register = <b>Registering</b> runs the same protocol. Your device draws a random master key (256 bits), wraps it once under your password and once under your recovery code, and sends only the wrappers. Central stores an OPAQUE record and the wrappers, never the key.
sec-signin-legacy = <b>One exception:</b> accounts of the old official gateway that have not moved to Central yet. Until such an account has moved, each sign-in attempt in the Baylee client sends the typed password, over a verified TLS connection, to Central, which checks it against the old hash and neither stores nor logs it. Once the move has succeeded, the account signs in as shown above. This website does not sign such accounts in; it points to the client. The exception ends with the deadline set for the move.
sec-signin-limit = <b>What remains:</b> whoever takes Central's database together with its keys can try passwords one by one; every attempt costs an Argon2id run. A password found that way opens the account's encrypted data and its stored chats. A long password that you use nowhere else still matters.

sec-memory-eyebrow = In memory
sec-memory-title = What is still in memory after you sign in
sec-memory-lead = The password is needed once: to open your master key. The diagram follows it through the browser; what the Baylee client does differently is below.
sec-memory-c1 = Password
sec-memory-t1 = The page hands the password to a Worker.
sec-memory-s1 = The form passes it to a background thread of its own (a Worker), empties the input field and frees its copy.
sec-memory-c2 = OPAQUE + Argon2id
sec-memory-t2 = The Worker computes, then overwrites.
sec-memory-s2 = Its Rust code holds the password only until the OPAQUE step ends and then overwrites it, along with the keys it derived on the way and holds itself, and Argon2id's 64 MiB of working memory.
sec-memory-c3 = Master key: checked, wiped
sec-memory-t3 = The master key is opened, used and wiped.
sec-memory-s3 = The Worker opens it to check the sign-in, derives the chat keys from it if your account has a chat, and overwrites it again.
sec-memory-c4 = Chat keys, sealed
sec-memory-t4 = Only the chat keys stay.
sec-memory-s4 = So that the chat opens after a reload without your password, the Worker stores them in the browser (IndexedDB), encrypted with AES-256-GCM. The key for that is non-extractable: scripts can use it but cannot read it out. While a chat page is open, a Worker of its own holds the chat keys unsealed in memory. Signing out deletes the record; after a session has ended by itself, the next page of this site that loads deletes it.
sec-memory-c5 = Worker ended
sec-memory-t5 = The sign-in's Worker is ended.
sec-memory-s5 = When the sign-in is done the sign-in page ends its Worker, and the browser discards that Worker's memory.
sec-memory-limit = <b>What a browser does not promise:</b> the password as typed, its handover to the Worker and JavaScript's strings are managed by the browser. Baylee frees them; it does not overwrite the page's own copies and cannot overwrite the browser's. When that memory is really reused is the browser's decision. And whoever can run script on this site or read your browser profile while you are signed in can get at the sealed chat keys.
sec-memory-client = <b>In the Baylee client</b> the typed password sits in memory that is overwritten when released, and it is dropped right after the sign-in; it is never stored. The master key stays in the client's memory while you are signed in, because the client opens your encrypted data with it.

sec-chat-eyebrow = Chat
sec-chat-title = Messages are encrypted on your device
sec-chat-lead = Every message gets a key of its own. Central stores and passes on only what is encrypted; reading it takes the chat key of an account it was wrapped for.
sec-chat-c1 = Text + one-time key
sec-chat-t1 = A new key for every message.
sec-chat-s1 = Your device draws a random message key and encrypts the text with it (XChaCha20-Poly1305).
sec-chat-c2 = Wrapped per chat key, signed
sec-chat-t2 = Wrapped for each recipient's chat key, signed with yours.
sec-chat-s2 = It wraps the message key separately for every chat key the recipient's account has registered (X25519) and signs header and ciphertext with your own chat key (Ed25519). Today an account has one chat key, derived from its master key and the same in every browser and client it is signed in on.
sec-chat-c3 = Ciphertext + wraps
sec-chat-t3 = Central receives only ciphertext.
sec-chat-s3 = Central checks the signature and the list of recipients and stores the ciphertext with the wraps. It has neither the message keys nor your account's keys.
sec-chat-c4 = Ciphertext + one wrap
sec-chat-t4 = Each chat key gets its own wrap.
sec-chat-s4 = When your contact fetches their messages, Central hands over the ciphertext and only the wrap made for that chat key.
sec-chat-c5 = Checked, opened
sec-chat-t5 = Check, unwrap, read.
sec-chat-s5 = Your contact's device checks your signature, unwraps the message key with its chat key and decrypts the text.
sec-chat-meta = <b>What Central sees all the same:</b> who is in contact with whom, groups with their names and members, display names, the public chat keys, and for every message who sent it, in which conversation, when, and how large it is. Like any server it also sees the address a request comes from.
sec-chat-limit = <b>Limits:</b> there is no double ratchet and so no forward secrecy: if a recipient's chat key is stolen later, it also opens earlier messages that were wrapped for it. Your chat key is derived from your account's master key, and Central keeps the encrypted messages after delivery: whoever takes Central's database with its keys and guesses your password, or has your recovery code, can read them. At first contact your device trusts the keys Central names; to check them, compare the safety number over another channel. If a contact's keys change later, Baylee sends again only after you have confirmed them. Who belongs to a group is decided by Central.

sec-pair-eyebrow = Signing in by link
sec-pair-title = From the browser into the client without typing the password again
sec-pair-lead = Someone signed in on the website should be able to start the Baylee client signed in. For that the page brings the master key to the client: sealed, and in such a way that Central can neither open it nor slip in one of its own.
sec-pair-status = <b>Where this stands:</b> Central and Baylee's client library can do this today and are tested against each other. It cannot be used from this website or from a released client yet.
sec-pair-c1 = Start pairing
sec-pair-t1 = The page starts a pairing.
sec-pair-s1 = Central issues a handle for it that is valid for five minutes. At the same time the page draws a one-time secret that it never shows to Central.
sec-pair-c2 = Link with the secret
sec-pair-t2 = A link opens the client.
sec-pair-s2 = The link <code>baylee://pair</code> carries the handle and the secret from the page to the client on your own device, past Central.
sec-pair-c3 = Public key
sec-pair-t3 = The client answers with a fresh key pair.
sec-pair-s3 = It sends Central the public half and a check value computed with the secret (HMAC-SHA-256). The page verifies that value; both screens show the same short code, and you approve on the page.
sec-pair-c4 = Master key, sealed
sec-pair-t4 = The page seals the master key for the client.
sec-pair-s4 = With HPKE (RFC 9180), to the client's public key and under the secret as well. Central stores the seal once; it cannot open it and, not knowing the secret, cannot make one of its own.
sec-pair-c5 = Session + seal
sec-pair-t5 = The client redeems once.
sec-pair-s5 = It receives a session together with the seal, opens the seal with its private key and the secret, and is signed in. The pairing is deleted afterwards, also when redeeming fails.
sec-pair-note = <b>The password takes no part in this exchange.</b> The page has the master key only at the moment it has just opened it with your password. A session paired this way counts as no newer than the password sign-in on the page it came from: pairing never renews it.

sec-limits-eyebrow = Limits
sec-limits-title = What none of this can do
sec-limit-device = <b>A compromised device.</b> Malware, a browser extension that reads pages, or a tampered client sees what you see and type. No encryption on the way to a server helps against that.
sec-limit-site = <b>A website's code comes from its server.</b> Whoever takes over our server could deliver code that captures your password as you type it; integrity hashes and a strict Content Security Policy guard against changes from outside, not against that. The client is a program on your machine and checks a signature before it installs an update.
sec-limit-central = <b>A Central that has been taken over.</b> It could reveal metadata, refuse service and attack sign-ins. It could name false keys at a first contact or add a member to a group, and so read messages sent after that. What it stores of your data and chats is encrypted; opening it takes your password or your recovery code.
sec-limit-recovery = <b>Lost keys.</b> Without your password and without your recovery code nobody can bring your encrypted data back, not even we: Central has only the wrappers.
sec-limit-audit = <b>No independent audit.</b> Sign-in follows OPAQUE (RFC 9807) with Argon2id (RFC 9106), and pairing seals with HPKE (RFC 9180): published specifications of the IRTF, not IETF standards. The order of the pairing's steps and the chat format are Baylee's own combinations of standard building blocks (Ed25519, X25519, XChaCha20-Poly1305). So far only the people who built it have reviewed the implementation. The code is open; if you find something, <contact>please tell us</contact>.
sec-docs-title = To read further
sec-docs = The details are in the project's documents: <protocol>crypto protocol</protocol>, <chat>chat</chat>, <accounts>accounts and pairing</accounts> and <threats>threat model</threats>. What happens to your data in legal terms is in the <privacy>privacy notice</privacy>.

# The page's two links from elsewhere: the FAQ and the sign-in page.
faq-security-q = How does Baylee handle my password and my chats?
faq-security-a = When you sign in, your device uses the OPAQUE protocol to prove that it knows the password without sending it; chat messages are encrypted on your device. The <security>security page</security> shows each step as a diagram, with the one exception and the limits.
sec-signin-link = What happens to your password when you sign in: <security>Security</security>
sec-teaser-lead = Step by step: what leaves your device, what Baylee Central sees and what it does not, and the limits each flow has.
sec-teaser-more = See each step in a diagram
# ==== end of the security page =================================================
