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

hero-title = <nb>Crea tus mazos.</nb> <nb>Reúne a tus amigos.</nb> <glow>Solo juega.</glow>
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
plat-windows-x64-sub = la mayoría de los PC: Intel o AMD de 64 bits
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
gws-now = Ahora mismo
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
# English for now: to be translated.
faq-password-a = You set a new password with a reset code, <reset>on this page</reset>. If your account has a confirmed e-mail address, you can have the code sent to it there; otherwise an administrator gives you one. Your encrypted data survives only if you have the recovery code you received when you created your account. Without it you start with an empty vault, and that data is lost.
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
impressum-write = Escribir un correo
impressum-call = Llamar
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
privacy-updated = A fecha de 12 de noviembre de 2026
privacy-controller-title = Quién es el responsable
privacy-controller = El responsable del tratamiento en el sentido del RGPD (Reglamento General de Protección de Datos) es la persona física indicada en el <impressum>aviso legal</impressum>, que da su nombre, su dirección postal y su correo electrónico. También puedes enviar preguntas sobre privacidad mediante el formulario de contacto. No hay delegado de protección de datos; no es obligatorio.
privacy-short-title = En resumen
privacy-short = Sin seguimiento, sin publicidad, sin analítica, sin cookies salvo que inicies sesión, y nada se carga desde otros servidores: las tipografías, las imágenes y los scripts vienen todos de baylee.app. Tu contraseña nunca nos llega, y tus mensajes de chat y tus datos privados se cifran en tu navegador antes de enviarse.
privacy-hosting-title = Visitar el sitio
privacy-hosting = El sitio funciona en un servidor que alquilamos a netcup GmbH, en Alemania. Cuando abres una página, tu navegador envía lo que lleva toda petición: tu dirección IP, la dirección solicitada, la hora, el idioma de tu navegador y su identificación (user agent). El servidor lo necesita para responder; elige la versión de idioma a partir del idioma, y la página de descarga usa el user agent para sugerir el archivo adecuado. El servidor web no guarda registro de accesos, de modo que estas peticiones no se almacenan. Los mensajes de funcionamiento del propio servidor (su diario del sistema) pueden, en casos raros de error, contener una dirección IP; se eliminan como máximo a los 14 días. Base jurídica: art. 6, apdo. 1, letra f) del RGPD; nuestro interés legítimo es ofrecer el sitio de forma segura y fiable.
privacy-views-title = El recuento de páginas vistas
privacy-views = El sitio cuenta cuántas veces se abren sus páginas: un único número para todas las páginas y todos los visitantes juntos, que se muestra en la página de inicio. Una página abierta en un navegador le suma uno. Para ello no se almacena nada sobre ti: ni dirección IP, ni cookie, ni identificador, ni siquiera de qué página se trataba.
privacy-device-title = Lo que guarda tu navegador
privacy-device = Este sitio no instala ninguna cookie hasta que inicias sesión. Lo que tu navegador guarda en tu dispositivo, y solo con JavaScript: tu elección de color, si la hiciste (<code>baylee-site-theme</code>); mientras tienes la sesión iniciada, cuándo termina tu sesión (<code>baylee-central-session</code>), tu clave de chat y las claves de confianza de tus contactos (véase Chat); si activaste las notificaciones, un número aleatorio de este dispositivo (<code>baylee-push-installation-v1</code>, <code>baylee-push</code>); y, solo una vez que hayas iniciado sesión, instalado el sitio como aplicación o activado las notificaciones, una copia de los archivos públicos del sitio (estructura de la página, tipografías, scripts), para que arranque sin conexión y puedan llegar las notificaciones. Si ya no se da ninguna de las tres circunstancias, la copia se elimina la próxima vez que abras el sitio. Nada de esto se nos envía, salvo que un apartado siguiente diga lo contrario. Las entradas del chat y de la sesión se eliminan al cerrar sesión; todo lo demás lo eliminas borrando los datos del sitio en tu navegador. Base jurídica: § 25, apdo. 2, n.º 2 TDDDG (estrictamente necesario para la función que usas); para el número aleatorio de las notificaciones, tu consentimiento (§ 25, apdo. 1 TDDDG).
privacy-github-title = GitHub
privacy-github = La página de descarga y la página de inicio muestran la última versión, el estado de las pruebas y las cifras de las cartas. Es nuestro servidor quien se las pide a GitHub, no tu navegador; GitHub no averigua nada sobre ti con ello. Solo cuando haces clic en una descarga o en un enlace de GitHub se conecta tu navegador a GitHub (GitHub, Inc., EE. UU.) y le envía tu dirección IP y lo que lleva toda petición; a partir de ahí se aplica la política de privacidad del propio GitHub.
privacy-directory-title = El directorio de gateways
privacy-directory = Nuestro servidor lee el directorio de gateways de Baylee Central y lo muestra; no interviene nada sobre ti. Un gateway puede decirle a Central cuántos jugadores están conectados en este momento, cuántos de ellos están en una partida y cuántas mesas abiertas y partidas tiene, y Central cuenta sus cuentas; el sitio muestra estas cifras. Son solo recuentos, nunca nombres, y las cifras de un gateway son una declaración de su propio operador.
privacy-account-title = Tu cuenta
privacy-account = Para una cuenta, Baylee Central almacena: el identificador de tu cuenta, tu nombre de inicio de sesión, tu nombre visible con su etiqueta, cuándo y cómo se creó la cuenta, un registro de contraseña OPAQUE (que te permite demostrar tu contraseña sin que nuestro servidor llegue a conocerla), tu clave maestra cifrada dos veces (con tu contraseña y con tu código de recuperación), tu documento de cuenta cifrado y tus objetos cifrados (ilegibles para nosotros; solo vemos tamaño, revisión y hora), qué versión de las condiciones aceptaste y cuándo, la hora de tu último inicio de sesión, tus sesiones, un rol de administrador si lo tienes, y los gateways que registras con sus tokens. Los demás jugadores ven tu nombre visible y tu etiqueta, nunca tu nombre de inicio de sesión. Si un administrador te da un código de restablecimiento, solo guardamos su huella hasta que se use el código o caduque. Lo que los administradores hacen con una cuenta o un gateway se conserva en un registro durante tres años (quién, qué, qué cuenta o gateway, cuándo). Base jurídica: art. 6, apdo. 1, letra b) del RGPD (la cuenta que pediste); para la hora del último inicio de sesión y el registro, art. 6, apdo. 1, letra f) del RGPD (nuestro interés legítimo en un funcionamiento seguro y trazable). Sin nombre de inicio de sesión y sin contraseña no hay cuenta; no necesitas correo electrónico. Puedes crear una cuenta a partir de los 16 años. Se conserva hasta que eliminas la cuenta; entonces se borra del sistema en funcionamiento de inmediato y de las copias de seguridad como máximo en 12 meses. Las sesiones terminan tras 30 minutos sin uso y, como máximo, 12 horas después de iniciar sesión.
# English for now: to be translated.
privacy-email-title = E-mail address
# English for now: to be translated.
privacy-email = An e-mail address is optional; an account needs none. If you give one, Baylee Central stores the address and whether and when you confirmed it. Of every code we mail, we store only a digest, until the code is used or has expired. We use the address for three things: to send the code that confirms it, to send a reset code when you ask for one, and to tell the old address when it is replaced or removed. The mail is sent by Baylee's own mail service on the same server through mailbox.org (Heinlein Hosting GmbH, Berlin), which acts as a processor. Legal basis: Art. 6(1)(b) GDPR (the account you asked for). Kept until you remove the address or delete the account.
privacy-cookie-title = La cookie de sesión
privacy-cookie = Al iniciar sesión se instala una cookie, <code>__Host-central</code>: un número aleatorio que identifica tu sesión. Solo se envía a baylee.app, los scripts no pueden leerla y termina con la sesión, como máximo a las 12 horas. Es estrictamente necesaria para mantener tu sesión iniciada; no requiere consentimiento. Base jurídica: § 25, apdo. 2, n.º 2 TDDDG y art. 6, apdo. 1, letra b) del RGPD.
privacy-chat-title = Chat y contactos
privacy-chat = Los mensajes se cifran en tu dispositivo para los dispositivos de cada destinatario; Baylee Central solo los almacena cifrados y no puede leerlos. Lo que Central puede ver y almacena: tus contactos, las solicitudes de contacto y los bloqueos, los grupos con sus nombres, miembros e invitaciones, quién envió un mensaje a quién, cuándo y de qué tamaño, y las claves públicas de tus dispositivos. Tus contactos y los miembros de tus grupos ven tu nombre visible con su etiqueta y tus mensajes. Tu navegador guarda tu clave de chat (sellada con una clave que solo posee este navegador, en su almacenamiento <code>baylee-chat</code>) y las claves de tus contactos en las que confiaste (cifradas, <code>baylee-chat-trust</code>) mientras tienes la sesión iniciada. Los mensajes y las entregas se conservan hasta que eliminas tu cuenta; todavía no hay un plazo de conservación automático más corto. Cada remitente y cada dispositivo guardan como máximo 5.000 mensajes; después se rechazan los nuevos. Base jurídica: art. 6, apdo. 1, letra b) del RGPD.
privacy-push-title = Notificaciones
privacy-push = Si activas las notificaciones en un navegador, tu navegador se registra en su servicio push, y Central almacena la dirección de ese servicio para este navegador, la clave pública y el secreto de autenticación del navegador, tu idioma, un número aleatorio de este dispositivo y tu elección de categorías. El servicio push pertenece al fabricante de tu navegador: Google (Firebase Cloud Messaging), Mozilla o Apple, todos con sede en EE. UU. Una notificación lleva, cifrado, solo su tipo (por ejemplo, «mensaje nuevo») y tu idioma, nunca nombres ni texto; el servicio push ve datos de conexión como la dirección, la hora y el tamaño. Estos datos se eliminan cuando desactivas las notificaciones en este dispositivo, cuando cierras sesión o cuando eliminas tu cuenta; una dirección que no se renueva en 90 días deja de usarse, y las notificaciones en espera caducan al cabo de un día. Base jurídica: art. 6, apdo. 1, letra a) del RGPD y § 25, apdo. 1 TDDDG (tu consentimiento). Puedes retirarlo en cualquier momento en los ajustes de tu cuenta; lo ocurrido hasta entonces sigue siendo lícito.
privacy-counters-title = Protección contra abusos
privacy-counters = Para impedir que se adivinen contraseñas y para evitar la sobrecarga, Central cuenta solicitudes por dirección IP (registros, intentos de inicio de sesión, restablecimientos de contraseña, lectura del directorio y, en general, cuántas solicitudes envía una dirección en un minuto), solo en memoria: los registros y los intentos de inicio de sesión que no se completaron con la contraseña correcta, hasta un día; todo lo demás, como máximo una hora. Tras un inicio de sesión con la contraseña correcta, Central recuerda durante 30 días, de nuevo solo en memoria, que tu cuenta inició sesión desde esa dirección (una dirección IPv4, o la primera mitad de una dirección IPv6; como máximo ocho por cuenta), para que los intentos de otras personas sobre tu cuenta no te dejen fuera desde allí. Quien use la misma dirección y conozca tu nombre de inicio de sesión puede saber por las respuestas de Central que tu cuenta inició sesión desde ella. Una contraseña nueva olvida esas direcciones salvo aquella desde la que la estableciste; eliminar la cuenta las olvida todas, y un reinicio del servicio también. Antes de un registro, tu navegador resuelve además un rompecabezas que requiere memoria y tiempo de cálculo; consiste en números aleatorios, no contiene nada sobre ti y se olvida como máximo a los veinte minutos. Central vuelve a calcular una pequeña parte para comprobar la respuesta de tu navegador. Su dificultad depende de cuántas cuentas se estén creando y de si la solicitud parece de un programa: si lleva las cabeceras que envía todo navegador, si tu navegador indica que lo controla un programa, y cuántos rompecabezas han pedido tu dirección y el bloque de red que la rodea (los tres primeros números de una dirección IPv4, los primeros 48 bits de una dirección IPv6) en la última hora. De esto Central conserva solo los recuentos, en memoria, como máximo una hora. Por cuenta, Central cuenta las solicitudes de contacto y las búsquedas de otros jugadores, para que nadie inunde a los demás de solicitudes. Base jurídica: art. 6, apdo. 1, letra f) del RGPD; nuestro interés legítimo es proteger las cuentas y el servicio frente al abuso.
privacy-contact-title = El formulario de contacto
privacy-contact = El formulario envía tu mensaje, el idioma de la página y la hora al servicio de comentarios de Baylee en el mismo servidor, y tu nombre y tu correo electrónico solo si los indicas; ambos son opcionales. El servicio también anota si el mensaje ya se ha atendido. Tu dirección IP no se almacena; solo se cuenta en memoria durante una hora, para impedir una avalancha de mensajes. Solo la persona indicada en el aviso legal lee los mensajes; no se transmite nada. Un mensaje se elimina 90 días después de haberse atendido, como máximo al cabo de un año, o antes si lo pides. Base jurídica: art. 6, apdo. 1, letra f) del RGPD (nuestro interés legítimo en responder a los mensajes), o letra b) cuando tu mensaje se refiere a tu cuenta.
privacy-impressum-title = El rompecabezas del aviso legal
privacy-impressum = El aviso legal solo se muestra después de que tu navegador haya resuelto un rompecabezas; eso protege la dirección de los programas que recogen direcciones. El rompecabezas consiste en números aleatorios y no contiene nada sobre ti. Para ello, el servicio de comentarios guarda únicamente en memoria, nunca en disco ni en un registro: cuántos rompecabezas ha pedido tu dirección IP (durante una hora; una dirección IPv6, por su primera mitad), si ha seguido un enlace oculto para las personas (durante 24 horas) y qué rompecabezas se resolvieron (durante cinco minutos, sin dirección). Tres respuestas de sí o no sobre tu navegador fijan la dificultad del rompecabezas y no se guardan: si lo maneja un programa, si no declara ningún idioma y si dibuja sin tarjeta gráfica. No se rechaza a nadie; un navegador que parece un programa recibe un rompecabezas más largo. Base jurídica: art. 6, apdo. 1, letra f) del RGPD (nuestro interés legítimo en proteger la dirección de una persona física).
privacy-recipients-title = Destinatarios y países
privacy-recipients = No vendemos datos ni los cedemos con fines publicitarios. Los destinatarios son: netcup GmbH (alojamiento en Alemania, como encargado del tratamiento conforme al art. 28 del RGPD); los administradores de Baylee Central, que para la administración ven tu nombre de inicio de sesión, tu nombre visible con su etiqueta, el origen, la creación y el último inicio de sesión de tu cuenta, y tus gateways, nunca tus datos cifrados; un gateway en el que inicias sesión con tu cuenta (recibe el identificador de tu cuenta, tu nombre visible y tu etiqueta, no tu contraseña); tus contactos y los miembros de tus grupos (nombre visible, etiqueta y tus mensajes); las autoridades solo cuando la ley nos obliga. Países fuera del Espacio Económico Europeo: solo cuando activas las notificaciones las transporta el servicio push de tu navegador (Google, Mozilla o Apple, EE. UU.). Los gateways gestionados por terceros pueden estar en cualquier país; iniciar sesión en ellos es decisión tuya. Por lo demás, no transferimos datos a países fuera del Espacio Económico Europeo.
privacy-rights-title = Tus derechos
privacy-rights = Tienes derecho de acceso (art. 15 del RGPD), rectificación (art. 16), supresión (art. 17), limitación del tratamiento (art. 18) y portabilidad de los datos (art. 20). Puedes oponerte en cualquier momento, por motivos relacionados con tu situación particular, al tratamiento basado en nuestros intereses legítimos conforme al art. 6, apdo. 1, letra f) (art. 21). Puedes retirar un consentimiento en cualquier momento con efecto futuro (art. 7, apdo. 3). Los datos de tu cuenta pueden exportarse y la cuenta puede eliminarse en los ajustes de tu cuenta; para todo lo demás basta un mensaje mediante el formulario de contacto o a la dirección del aviso legal. Puedes presentar una reclamación ante una autoridad de control de protección de datos (art. 77), por ejemplo la de tu lugar de residencia o el Comisionado Estatal para la Protección de Datos y la Libertad de Información de Renania del Norte-Westfalia. Ninguna ley ni contrato te obliga a darnos datos; sin embargo, sin un nombre de inicio de sesión y una contraseña no hay cuenta. No hay decisiones automatizadas ni elaboración de perfiles (art. 22).
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

error-busy = Demasiados intentos. Inténtalo más tarde; tras varias contraseñas incorrectas puede tardar una hora, como máximo un día.
error-unavailable = Baylee Central no responde ahora mismo. Inténtalo de nuevo en un momento.
error-network = Sin conexión. Comprueba tu red e inténtalo de nuevo.
error-failed = Algo ha salido mal. Inténtalo de nuevo; si sigue ocurriendo, avísanos.
error-again = Reintentar
password-show-label = Mostrar la contraseña

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
signin-forgot = ¿Olvidaste tu contraseña? <reset>Establece una nueva con un código de restablecimiento</reset>.
register-have-account = ¿Ya tienes una cuenta? <signin>Inicia sesión</signin>.
dialog-close = Cerrar
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
code-copy = Copiar
code-copied = Copiado.
code-copy-failed = Aquí no se pudo copiar. Selecciona el código y cópialo a mano.

## Setting a new password with a reset code

reset-title = Establecer una contraseña nueva
# English for now: to be translated.
reset-lead = For when you forgot your password: with a reset code you set a new one here. The code comes by e-mail to your confirmed address, or from an administrator.
reset-noscript = Establecer una contraseña nueva requiere JavaScript: tu contraseña y tu código de recuperación se procesan en tu navegador y nunca se nos envían.
reset-code = Código de restablecimiento
# English for now: to be translated.
reset-code-hint = By e-mail or from an administrator. It starts with RESET and works once: for one hour if it came by e-mail, for 24 hours if an administrator made it.
reset-data = Tus datos cifrados
reset-keep = Tengo mi código de recuperación. Mis datos se conservan.
reset-new = No lo tengo. Empezar con una bóveda vacía.
reset-recovery = Código de recuperación
reset-recovery-hint = El código que guardaste al crear tu cuenta. Se queda en tu navegador y nunca se nos envía.
reset-warning = Con una bóveda vacía tus datos cifrados se pierden para siempre: los ajustes guardados en tu cuenta, tus copias de seguridad de mazos y tu historial del chat. Nadie puede recuperarlos, ni siquiera nosotros. Tu cuenta, tu nombre visible y tus contactos se conservan. Recibes un código de recuperación nuevo.
reset-warning-confirm = Entiendo que estos datos se pierden para siempre.
reset-submit = Establecer contraseña nueva
reset-missing = Introduce tu nombre de inicio de sesión, el código de restablecimiento y una contraseña nueva.
reset-recovery-missing = Introduce tu código de recuperación o elige la bóveda vacía.
reset-error-code = Eso no es un código de restablecimiento. Empieza por RESET, seguido de ocho grupos de cuatro caracteres.
reset-error-recovery = Este código de recuperación no abre los datos de tu cuenta. Compruébalo e inténtalo de nuevo. No se cambió nada y tu código de restablecimiento sigue sirviendo.
reset-error-confirm = Confirma primero que sabes qué se pierde.
reset-refused = Este nombre de inicio de sesión y este código de restablecimiento no corresponden, o el código ha caducado o ya se usó. Comprueba ambos. Si nada funciona, pide un código nuevo.
reset-done-title = Tu contraseña nueva está establecida
reset-done-kept = Tus datos y tu código de recuperación siguen como estaban. Has cerrado sesión en todas partes; inicia sesión con tu contraseña nueva.
reset-done-new = Empiezas con una bóveda vacía y has cerrado sesión en todas partes. Abajo está tu código de recuperación nuevo; el anterior ya no sirve.
reset-unsure-title = No llegó ninguna respuesta
reset-unsure = La conexión se cortó antes de que Baylee Central respondiera, así que no podemos saber si se estableció tu contraseña nueva. Intenta iniciar sesión con ella. Si no funciona, no se cambió nada: tu código de restablecimiento sigue sirviendo, introdúcelo aquí otra vez.
reset-unsure-code = Si tu contraseña nueva funciona, este es tu código de recuperación nuevo y el anterior ya no sirve; no se puede mostrar otra vez, así que guárdalo ahora. Si tu contraseña nueva no funciona, este código no vale nada.
reset-to-signin = Iniciar sesión
# English for now: to be translated.
reset-mail-title = No reset code?
# English for now: to be translated.
reset-mail-lead = Enter your sign-in name. If your account has a confirmed e-mail address, we send a reset code to it.
# English for now: to be translated.
reset-mail-name-hint = The name you sign in with, not your e-mail address.
# English for now: to be translated.
reset-mail-submit = Send a code by e-mail
# English for now: to be translated.
reset-mail-error-name = That is not a sign-in name. Enter the name you sign in with, not your e-mail address.
# English for now: to be translated.
reset-mail-sent = If this account has a confirmed e-mail address, a reset code is on its way to it. It is valid for one hour.
# English for now: to be translated.
reset-mail-busy = Too many requests for now. Try again later.
# English for now: to be translated.
reset-mail-none = This server sends no mail. Ask an administrator for a reset code.
# English for now: to be translated.
reset-mail-early = That was too quick. Try again in a moment.

## Changing the password

password-new = Contraseña nueva
password-new-repeat = Repite la contraseña nueva
password-current = Contraseña actual
password-change-submit = Cambiar contraseña
password-missing = Introduce tu contraseña actual y una nueva.
password-error-current = La contraseña actual no es correcta. Compruébala e inténtalo de nuevo.
password-changed = Tu contraseña se ha cambiado. Aquí sigues con la sesión iniciada y en todas las demás partes has cerrado sesión, también en el cliente de Baylee; inicia sesión allí con la contraseña nueva.
password-unsure = No llegó ninguna respuesta, así que no podemos saber si se cambió tu contraseña. La próxima vez que inicies sesión, prueba primero la contraseña nueva y después la anterior.

## An account's e-mail address

# English for now: to be translated.
email-title = E-mail address
# English for now: to be translated.
email-lead = An e-mail address is optional. We use it to send you a reset code when you ask for one, and to tell you when the address is replaced or removed.
# English for now: to be translated.
email-warning = Whoever can read this mailbox can reset your sign-in. Your encrypted data survives a reset only with your recovery code.
# English for now: to be translated.
email-no-mail = This server sends no mail, so an e-mail address cannot be set or confirmed here.
# English for now: to be translated.
email-unavailable = The e-mail address cannot be shown right now.
# English for now: to be translated.
email-password-hint = Your current password, so that nobody else at this browser decides who can reset your sign-in.
# English for now: to be translated.
email-submit = Send confirmation code
# English for now: to be translated.
email-missing = Enter an e-mail address and your password.
# English for now: to be translated.
email-error-address = We cannot write to this address. Enter one plain address such as name@example.org; accented letters and umlauts do not work.
# English for now: to be translated.
email-current = Your address
# English for now: to be translated.
email-confirmed-at = Confirmed
# English for now: to be translated.
email-waiting = Waits for its code
# English for now: to be translated.
email-change = Change
# English for now: to be translated.
email-change-lead = Your current address stays in use until you have confirmed the new one.
# English for now: to be translated.
email-change-cancel = Keep the current address
# English for now: to be translated.
email-remove = Remove
# English for now: to be translated.
email-remove-ask = Remove this address? We tell the address that it was removed. After that you can no longer get a reset code by e-mail.
# English for now: to be translated.
email-remove-ask-no-mail = Remove this address? While this server sends no mail, you cannot set one again.
# English for now: to be translated.
email-remove-confirm = Remove address
# English for now: to be translated.
email-remove-keep = Keep it
# English for now: to be translated.
email-pending = We sent a code to <code>{ $address }</code>. It is valid for 15 minutes.
# English for now: to be translated.
email-pending-keeps = Until you confirm it, <code>{ $address }</code> stays your address.
# English for now: to be translated.
email-code = Confirmation code
# English for now: to be translated.
email-code-hint = Eight characters, as in XXXX-XXXX. Upper or lower case, with or without the hyphen.
# English for now: to be translated.
email-confirm = Confirm
# English for now: to be translated.
email-resend = Send again
# English for now: to be translated.
email-other = Use another address
# English for now: to be translated.
email-withdraw = Withdraw
# English for now: to be translated.
email-error-code = That is not a confirmation code. It has eight characters, as in XXXX-XXXX.
# English for now: to be translated.
email-wrong-code = That code is not right. Check it and try again. After several wrong codes you have to enter the address again.
# English for now: to be translated.
email-now-confirmed = Your e-mail address is confirmed.
# English for now: to be translated.
email-resent = A new code is on its way. The earlier one no longer works.
# English for now: to be translated.
email-withdrawn = The address is withdrawn. No code is waited for any more.
# English for now: to be translated.
email-removed = Your e-mail address is removed.
# English for now: to be translated.
email-busy = Too many mails for now. Try again later.
# English for now: to be translated.
email-same = This is your address already.
# English for now: to be translated.
email-unsure = No answer came back, so we cannot tell whether that went through. Look at what is shown here, and try again if it did not.

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
security-password-lead = Tus datos cifrados y tu código de recuperación se quedan como están. Cambiar la contraseña cierra tu sesión en todas partes menos aquí.

privacy-export-title = Exportar tus datos
privacy-export-lead = Lo que Baylee Central sabe de tu cuenta, como archivo JSON: sus datos básicos, sesiones, condiciones aceptadas, tus gateways y las entradas del registro de auditoría que nombran tu cuenta. Tus contenidos cifrados solo puede descifrarlos y exportarlos tu cliente de Baylee. La exportación necesita un inicio de sesión de hace menos de 30 minutos.
privacy-export-make = Preparar la exportación
privacy-export-save = Guardar la exportación
# English for now: to be translated.
privacy-export-no-email = The e-mail address could not be read; it is not in this file.
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
strip-accounts = Cuentas en Baylee Central
strip-live = Ahora mismo en los gateways, según informan
strip-views = Páginas vistas desde { $date }
live-views = { $n ->
    [one] { $n } vista
   *[other] { $n } vistas
}
live-accounts = { $n ->
    [one] { $n } cuenta
   *[other] { $n } cuentas
}
live-players = { $n ->
    [one] { $n } jugador
   *[other] { $n } jugadores
}
live-playing = { $n ->
    [one] { $n } en partida
   *[other] { $n } en partida
}
live-tables = { $n ->
    [one] { $n } mesa abierta
   *[other] { $n } mesas abiertas
}
live-games = { $n ->
    [one] { $n } partida
   *[other] { $n } partidas
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
feature-commander = Arma mazos para duelos clásicos de 60 cartas, partidas multijugador o formatos propios de la casa. El anfitrión fija libremente los puntos de vida iniciales, la disposición de los equipos y las reglas de mulligan.
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
feature-tables = Desde turnos ágiles de 15 segundos hasta mesas relajadas sin límite de tiempo, configura el ritmo por separado para cada sala.
shots-eyebrow = En la mesa
shots-title = Dentro del juego
shots-lead = Breves grabaciones en directo tomadas del cliente 3D, con imágenes de cartas de Scryfall (© Wizards of the Coast): desde duelos y partidas de 4 jugadores hasta el vestíbulo cifrado y el constructor de mazos.
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
gw-shown = { $n } de { $all } mostrados
gw-noMatch = Ningún gateway cargado hasta ahora coincide.
gw-showAll = Mostrar todos
gw-stats = El directorio en cifras
gw-stat-listed = Gateways listados
gw-stat-online = Online ahora
gw-stat-players = Jugadores online
gw-stat-playing = De ellos, en partida
gw-stat-tables = Mesas abiertas
gw-stat-games = Partidas en curso
gw-show = Mostrar
gw-show-every = Todos
gw-show-online = Online
gw-show-official = Oficial
gw-show-tables = Con mesas abiertas
gw-sort = Ordenar por
gw-sort-best = Recomendados
gw-sort-name = Nombre (A–Z)
gw-sort-players = Más jugadores
gw-sort-tables = Más mesas abiertas
gw-sort-version = Versión más reciente
gw-view = Vista
gw-view-cards = Mosaico
gw-view-list = Lista
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
title-security = Seguridad en Baylee: contraseñas, claves y chats
desc-security = Cómo trata Baylee las contraseñas, las claves y los mensajes de chat, paso a paso en cuatro diagramas, con los límites de cada uno.
nav-security = Seguridad
sec-eyebrow = Seguridad
sec-title = Contraseñas, claves y chats: qué va adónde
sec-lead = Cuatro flujos, paso a paso: qué sale de tu dispositivo, qué ve Baylee Central y qué no. Con los límites de cada uno.
sec-toc-label = En esta página
sec-figure = Diagrama por pasos: { $name }
sec-play = Reproducir
sec-all = Todos los pasos
sec-step = Paso { $n }
sec-hint = El diagrama se reproduce solo. Elige un paso para detenerlo ahí.
sec-hint-still = Elige un paso para resaltarlo.
sec-in-browser = El marco discontinuo es tu navegador: todo esto ocurre dentro de él.
# The actors' names are the only words inside the drawings, where text does
# not wrap: one or two words each, at most about twice the English width
# (the test `names_fit_with_room_to_grow` measures every language).
sec-actor-device = Tu dispositivo
sec-actor-central = Central
sec-actor-page = Página
sec-actor-worker = Worker
sec-actor-store = Almacén
sec-actor-you = Tú
sec-actor-contact = Contacto
sec-actor-website = Sitio web
sec-actor-client = Cliente

sec-signin-eyebrow = Iniciar sesión y registrarse
sec-signin-title = Central comprueba tu contraseña sin recibirla
sec-signin-lead = Baylee te inicia sesión con OPAQUE (RFC 9807). Tu dispositivo y Central intercambian tres mensajes; tu contraseña no está en ninguno de ellos, ni siquiera como hash simple.
sec-signin-c1 = Petición cegada
sec-signin-t1 = Tu dispositivo ciega la contraseña.
sec-signin-s1 = Combina la contraseña con un número aleatorio nuevo y envía solo el resultado. La contraseña no puede leerse a partir de ese mensaje.
sec-signin-c2 = Respuesta de Central
sec-signin-t2 = Central responde con una clave propia.
sec-signin-s2 = Central calcula sobre el valor cegado con una clave que solo ella tiene y añade su mitad de un intercambio de claves. Al hacerlo, no llega a conocer la contraseña.
sec-signin-c3 = Argon2id · 64 MiB
sec-signin-t3 = Tu dispositivo hace el trabajo pesado.
sec-signin-s3 = Quita el cegado y refuerza el resultado con Argon2id (64 MiB de memoria, tres pasadas); Central no puede pedir ajustes más baratos. Aquí se detecta una contraseña incorrecta, antes de enviar nada más.
sec-signin-c4 = Prueba
sec-signin-t4 = Tu dispositivo demuestra que tiene las claves correctas.
sec-signin-s4 = Con la contraseña correcta, tu dispositivo llega a la misma clave de sesión que Central y envía una breve prueba de ello (64 bytes).
sec-signin-c5 = Sesión + envoltorio
sec-signin-t5 = Central abre la sesión.
sec-signin-s5 = Vuelven la sesión (en el navegador, una cookie que la propia página no puede leer) y tu clave maestra en un envoltorio que solo se abre en tu dispositivo, con la contraseña correcta.
sec-signin-register = <b>El registro</b> usa el mismo protocolo. Tu dispositivo genera una clave maestra aleatoria (256 bits), la envuelve una vez con tu contraseña y otra con tu código de recuperación, y envía solo los envoltorios. Central almacena un registro OPAQUE y los envoltorios, nunca la clave.
sec-signin-legacy = <b>Una excepción:</b> las cuentas del antiguo gateway oficial que aún no se han trasladado a Central. Mientras una cuenta así no se haya trasladado, cada intento de inicio de sesión en el cliente de Baylee envía la contraseña escrita, por una conexión TLS verificada, a Central, que la comprueba con el hash antiguo y ni la almacena ni la registra. Cuando el traslado se haya completado, la cuenta inicia sesión como se muestra arriba. Este sitio web no inicia sesión en esas cuentas; remite al cliente. La excepción termina con el plazo fijado para el traslado.
sec-signin-limit = <b>Lo que queda:</b> quien se haga con la base de datos de Central junto con sus claves puede probar contraseñas una a una; cada intento cuesta una ejecución de Argon2id. Una contraseña encontrada así abre los datos cifrados de la cuenta y sus chats almacenados. Sigue importando una contraseña larga que no uses en ningún otro sitio.
sec-signin-guess = <b>Adivinar contraseñas a través del inicio de sesión</b> se frena, no se detiene. Central cuenta los intentos que no se completan con la contraseña correcta: como máximo 100 al día sobre una cuenta desde direcciones desde las que no ha iniciado sesión, y como máximo 100 al día desde una dirección sobre todas las cuentas juntas. Pasado ese límite, responde que hubo demasiados intentos, hasta que termine la hora o el día. Una contraseña común se encuentra igualmente así. Y quien conozca tu nombre de inicio de sesión puede agotar los intentos de tu cuenta: hasta que termine el día, solo podrás iniciar sesión desde una dirección desde la que tu cuenta haya iniciado sesión en los últimos 30 días (Central recuerda esas direcciones solo hasta que se reinicia).

sec-memory-eyebrow = En memoria
sec-memory-title = Qué sigue en memoria después de iniciar sesión
sec-memory-lead = La contraseña hace falta una vez: para abrir tu clave maestra. El diagrama la sigue por el navegador; lo que hace distinto el cliente de Baylee está más abajo.
sec-memory-c1 = Contraseña
sec-memory-t1 = La página entrega la contraseña a un Worker.
sec-memory-s1 = El formulario se la pasa a un hilo propio en segundo plano (un Worker), vacía el campo de entrada y libera su copia.
sec-memory-c2 = OPAQUE + Argon2id
sec-memory-t2 = El Worker calcula y luego sobrescribe.
sec-memory-s2 = Su código Rust conserva la contraseña solo hasta que termina el paso de OPAQUE y entonces la sobrescribe, junto con las claves que derivó por el camino y custodia él mismo, y con los 64 MiB de memoria de trabajo de Argon2id.
sec-memory-c3 = Clave maestra: comprobada, borrada
sec-memory-t3 = La clave maestra se abre, se usa y se borra.
sec-memory-s3 = El Worker la abre para comprobar el inicio de sesión, deriva de ella las claves de chat si tu cuenta tiene chat, y la vuelve a sobrescribir.
sec-memory-c4 = Claves de chat, selladas
sec-memory-t4 = Solo quedan las claves de chat.
sec-memory-s4 = Para que el chat se abra tras recargar sin tu contraseña, el Worker las guarda en el navegador (IndexedDB), cifradas con AES-256-GCM. La clave para ello no es extraíble: los scripts pueden usarla, pero no leerla. Mientras hay una página de chat abierta, un Worker propio mantiene en memoria las claves de chat sin sellar. Al cerrar sesión se borra el registro; cuando una sesión ha terminado por sí sola, la siguiente página de este sitio que se cargue lo borra.
sec-memory-c5 = Worker terminado
sec-memory-t5 = El Worker del inicio de sesión termina.
sec-memory-s5 = Cuando el inicio de sesión ha terminado, la página de inicio de sesión finaliza su Worker y el navegador descarta la memoria de ese Worker.
sec-memory-limit = <b>Lo que un navegador no promete:</b> la contraseña tal como se escribe, su entrega al Worker y las cadenas de JavaScript las gestiona el navegador. Baylee las libera; no sobrescribe las copias propias de la página y no puede sobrescribir las del navegador. Cuándo se reutiliza realmente esa memoria lo decide el navegador. Y quien pueda ejecutar un script en este sitio o leer el perfil de tu navegador mientras tienes la sesión iniciada puede acceder a las claves de chat selladas.
sec-memory-client = <b>En el cliente de Baylee</b> la contraseña escrita está en una memoria que se sobrescribe al liberarse, y se descarta justo después de iniciar sesión; nunca se almacena. La clave maestra permanece en la memoria del cliente mientras tienes la sesión iniciada, porque el cliente abre con ella tus datos cifrados.

sec-chat-eyebrow = Chat
sec-chat-title = Los mensajes se cifran en tu dispositivo
sec-chat-lead = Cada mensaje recibe una clave propia. Central solo almacena y transmite lo cifrado; leerlo requiere la clave de chat de una cuenta para la que se envolvió.
sec-chat-c1 = Texto + clave de un solo uso
sec-chat-t1 = Una clave nueva para cada mensaje.
sec-chat-s1 = Tu dispositivo genera una clave de mensaje aleatoria y cifra el texto con ella (XChaCha20-Poly1305).
sec-chat-c2 = Envuelta por clave de chat, firmada
sec-chat-t2 = Envuelta para la clave de chat de cada destinatario, firmada con la tuya.
sec-chat-s2 = Envuelve la clave del mensaje por separado para cada clave de chat que la cuenta del destinatario haya registrado (X25519) y firma la cabecera y el texto cifrado con tu propia clave de chat (Ed25519). Hoy una cuenta tiene una sola clave de chat, derivada de su clave maestra e igual en todos los navegadores y clientes en los que ha iniciado sesión.
sec-chat-c3 = Texto cifrado + envoltorios
sec-chat-t3 = Central recibe solo texto cifrado.
sec-chat-s3 = Central comprueba la firma y la lista de destinatarios y almacena el texto cifrado con los envoltorios. No tiene ni las claves de los mensajes ni las claves de tu cuenta.
sec-chat-c4 = Texto cifrado + un envoltorio
sec-chat-t4 = Cada clave de chat recibe su propio envoltorio.
sec-chat-s4 = Cuando tu contacto recoge sus mensajes, Central entrega el texto cifrado y solo el envoltorio hecho para esa clave de chat.
sec-chat-c5 = Comprobado, abierto
sec-chat-t5 = Comprobar, desenvolver, leer.
sec-chat-s5 = El dispositivo de tu contacto comprueba tu firma, desenvuelve la clave del mensaje con su clave de chat y descifra el texto.
sec-chat-meta = <b>Lo que Central ve de todos modos:</b> quién está en contacto con quién, los grupos con sus nombres y miembros, los nombres visibles, las claves públicas de chat y, de cada mensaje, quién lo envió, en qué conversación, cuándo y de qué tamaño es. Como cualquier servidor, también ve la dirección desde la que llega una petición.
sec-chat-limit = <b>Límites:</b> no hay doble ratchet y, por tanto, no hay secreto hacia adelante: si más adelante se roba la clave de chat de un destinatario, también abre los mensajes anteriores que se envolvieron para ella. Tu clave de chat se deriva de la clave maestra de tu cuenta, y Central conserva los mensajes cifrados tras la entrega: quien se haga con la base de datos de Central junto con sus claves y adivine tu contraseña, o tenga tu código de recuperación, puede leerlos. En el primer contacto, tu dispositivo confía en las claves que nombra Central; para comprobarlas, compara el número de seguridad por otro canal. Si las claves de un contacto cambian más adelante, Baylee solo vuelve a enviar después de que las hayas confirmado. Central decide quién pertenece a un grupo.

sec-pair-eyebrow = Iniciar sesión mediante un enlace
sec-pair-title = Del navegador al cliente sin volver a escribir la contraseña
sec-pair-lead = Quien haya iniciado sesión en el sitio web debería poder abrir el cliente de Baylee con la sesión ya iniciada. Para ello la página lleva la clave maestra al cliente: sellada y de tal modo que Central ni puede abrirla ni puede colar una propia.
sec-pair-status = <b>En qué punto está:</b> Central y la biblioteca de cliente de Baylee pueden hacerlo hoy y se han probado entre sí. Todavía no puede usarse desde este sitio web ni desde un cliente publicado.
sec-pair-c1 = Iniciar vinculación
sec-pair-t1 = La página inicia una vinculación.
sec-pair-s1 = Central emite para ella un identificador válido durante cinco minutos. A la vez, la página genera un secreto de un solo uso que nunca muestra a Central.
sec-pair-c2 = Enlace con el secreto
sec-pair-t2 = Un enlace abre el cliente.
sec-pair-s2 = El enlace <code>baylee://pair</code> lleva el identificador y el secreto de la página al cliente, en tu propio dispositivo, sin pasar por Central.
sec-pair-c3 = Clave pública
sec-pair-t3 = El cliente responde con un par de claves nuevo.
sec-pair-s3 = Envía a Central la mitad pública y un valor de comprobación calculado con el secreto (HMAC-SHA-256). La página verifica ese valor; ambas pantallas muestran el mismo código corto, y lo apruebas en la página.
sec-pair-c4 = Clave maestra, sellada
sec-pair-t4 = La página sella la clave maestra para el cliente.
sec-pair-s4 = Con HPKE (RFC 9180), para la clave pública del cliente y también bajo el secreto. Central almacena el sello una vez; no puede abrirlo y, al no conocer el secreto, tampoco puede hacer uno propio.
sec-pair-c5 = Sesión + sello
sec-pair-t5 = El cliente lo canjea una vez.
sec-pair-s5 = Recibe una sesión junto con el sello, abre el sello con su clave privada y el secreto, y queda con la sesión iniciada. Después se elimina la vinculación, también cuando el canje falla.
sec-pair-note = <b>La contraseña no interviene en este intercambio.</b> La página tiene la clave maestra solo en el momento en que acaba de abrirla con tu contraseña. Una sesión vinculada así cuenta como no más reciente que el inicio de sesión con contraseña en la página de la que procede: vincular nunca la renueva.

sec-limits-eyebrow = Límites
sec-limits-title = Lo que nada de esto puede hacer
sec-limit-device = <b>Un dispositivo comprometido.</b> Un malware, una extensión del navegador que lee páginas o un cliente manipulado ven lo que ves y escribes. Ningún cifrado en el camino hacia un servidor ayuda contra eso.
sec-limit-site = <b>El código de un sitio web viene de su servidor.</b> Quien se haga con nuestro servidor podría entregar código que capture tu contraseña mientras la escribes; los hashes de integridad y una Content Security Policy estricta protegen contra cambios desde fuera, no contra eso. El cliente es un programa en tu equipo y comprueba una firma antes de instalar una actualización.
sec-limit-central = <b>Un Central que haya sido tomado.</b> Podría revelar metadatos, negar el servicio y atacar inicios de sesión. Podría nombrar claves falsas en un primer contacto o añadir un miembro a un grupo y así leer los mensajes enviados después. Lo que almacena de tus datos y chats está cifrado; abrirlo requiere tu contraseña o tu código de recuperación.
sec-limit-recovery = <b>Claves perdidas.</b> Sin tu contraseña y sin tu código de recuperación nadie puede recuperar tus datos cifrados, ni siquiera nosotros: Central solo tiene los envoltorios.
sec-limit-audit = <b>Sin auditoría independiente.</b> El inicio de sesión sigue OPAQUE (RFC 9807) con Argon2id (RFC 9106), y la vinculación sella con HPKE (RFC 9180): especificaciones publicadas de la IRTF, no estándares de la IETF. El orden de los pasos de la vinculación y el formato del chat son combinaciones propias de Baylee de bloques de construcción estándar (Ed25519, X25519, XChaCha20-Poly1305). Hasta ahora solo han revisado la implementación las personas que la construyeron. El código es abierto; si encuentras algo, <contact>cuéntanoslo</contact>.
sec-docs-title = Para leer más
sec-docs = Los detalles están en los documentos del proyecto: <protocol>protocolo criptográfico</protocol>, <chat>chat</chat>, <accounts>cuentas y vinculación</accounts> y <threats>modelo de amenazas</threats>. Lo que ocurre con tus datos en términos jurídicos está en la <privacy>política de privacidad</privacy>.

# The page's two links from elsewhere: the FAQ and the sign-in page.
faq-security-q = ¿Cómo trata Baylee mi contraseña y mis chats?
faq-security-a = Al iniciar sesión, tu dispositivo usa el protocolo OPAQUE para demostrar que conoce la contraseña sin enviarla; los mensajes de chat se cifran en tu dispositivo. La <security>página de seguridad</security> muestra cada paso en un diagrama, con la única excepción y los límites.
sec-signin-link = Qué pasa con tu contraseña al iniciar sesión: <security>Seguridad</security>
sec-teaser-lead = Paso a paso: qué sale de tu dispositivo, qué ve Baylee Central y qué no, y los límites de cada flujo.
sec-teaser-more = Ver cada paso en un diagrama
# ==== end of the security page =================================================
# ==== registration: the proof of work, while it takes long =====================
register-solving = Tu navegador está resolviendo un rompecabezas que mantiene fuera a los robots. Cuando se están creando muchas cuentas, puede tardar unos minutos … { $percent } %
