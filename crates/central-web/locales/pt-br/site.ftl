# baylee.app's words in English, Project Fluent's format. English decides
# which messages exist and which arguments each takes; build.rs turns them
# into typed functions (crates/central-web/build.rs). The reader is "you";
# a button says what it does.

## The frame of every page

site-name = Baylee
skip = Ir para o conteúdo
# Read out after a link that leaves the site (the stylesheet carries it).
new-tab = abre em uma nova aba
nav-label = Principal
nav-download = Baixar
nav-gateways = Gateways
nav-faq = Perguntas
nav-contact = Contato
nav-sign-in = Entrar
nav-account = Sua conta
lang-label = Idioma
theme-label = Esquema de cores
theme-light = Cores: claras
theme-dark = Cores: escuras

foot-label = Sobre este site
foot-play = Jogar
foot-legal = Legal
foot-releases = Todas as versões
foot-issues = Informar um erro
foot-discord = Comunidade no Discord
foot-central-source = Código-fonte deste site
foot-dedication = Para a Baylee. Para sempre sob o sol.
fan-content = Baylee é um conteúdo de fã não oficial permitido sob a <policy>política de conteúdo de fãs</policy>. Não aprovado/apoiado pela Wizards. Porções do material usado são propriedade da Wizards of the Coast. ©Wizards of the Coast LLC.
foot-colophon = Fontes: Literata e Inter, SIL Open Font License 1.1, servidas por este site. A pintura e o logotipo são nossos. <notices>Avisos</notices>

## Titles and descriptions

title-home = Baylee – Monte seus decks. Reúna seus amigos. Só jogue.
desc-home = Baylee é um jogo de fã gratuito e de código aberto de Magic: The Gathering para Linux, Windows e macOS: um motor de regras que conhece os cards, mesas para até oito jogadores e oponentes para quando não houver mais ninguém por perto.
title-download = Baixar o Baylee
desc-download = Baixe o cliente do Baylee para Windows, macOS e Linux: instaladores e arquivos da versão mais recente, com somas de verificação e assinaturas.
title-gateways = Gateways: onde o Baylee é jogado online
desc-gateways = O diretório público dos gateways do Baylee, os servidores onde ficam as mesas online, e como executar o seu próprio.
title-faq = Perguntas e respostas
desc-faq = O que é o Baylee, quanto custa, como funcionam o jogo online e as contas, e o que acontece com os seus dados.
title-contact = Contato
desc-contact = Escreva para a pessoa por trás do Baylee ou informe um erro.
title-impressum = Aviso legal
desc-impressum = Quem opera o baylee.app (§ 5 DDG).
title-privacy = Aviso de privacidade
desc-privacy = O que o baylee.app e o Baylee Central processam sobre você, por quê, com qual base legal e por quanto tempo.
title-terms = Termos de uso
desc-terms = Os termos de uso do Baylee Central, o serviço de contas por trás do baylee.app.
title-fan-content = Conteúdo de fã não oficial
desc-fan-content = O Baylee é um projeto de fã sob a política de conteúdo de fãs da Wizards of the Coast: o que isso significa e o que não significa.
title-not-found = Página não encontrada
title-gone = Esta página não existe mais
crumbs-label = Você está aqui
breadcrumb-home = Início

## Home

hero-title = <nb>Monte seus decks.</nb> <nb>Reúna seus amigos.</nb> <glow>É só jogar.</glow>
hero-lead = Um montador de decks completo, partidas para até oito jogadores em duelos, todos contra todos ou em equipes, e um motor de regras que confere cada passo, para que cada turno seja justo sem ninguém precisar consultar nada.
hero-offline = Partidas solo e o montador de decks funcionam offline, sem conta. As mesas online ficam em gateways; <host>você pode executar o seu próprio</host>.
hero-account = Vai jogar online? <register>Crie uma conta</register>, gratuita e aberta a todos no beta aberto, ou <signin>entre</signin>.
hero-memorial = Lembre-se da Baylee: abra o álbum de fotos dela

status-label = O Baylee em números

cards-eyebrow = Os cards
cards-title = Quantos cards o Baylee conhece
cards-lead = O Baylee implementa os cards de Magic um a um, cada um conferido com o texto impresso. Estes são os números da versão mais recente.
cards-implemented = Totalmente implementados
cards-implemented-help = Todas as regras do card funcionam e estão testadas.
cards-generated = Gerados
cards-generated-help = Implementados pelo gerador do Baylee a partir do texto de regras do card, mas ainda não testados à mão. Dá para jogar com eles; em casos extremos raros, um deles pode se comportar de outra forma.
cards-partial = Parcialmente implementados
cards-partial-help = O card é jogável, mas ainda falta uma de suas habilidades ou um caso extremo de suas regras.
cards-total = Total de cards
cards-total-help = Todos os cards que Magic tem, conforme a lista do Scryfall; os três números anteriores são os que o Baylee consegue jogar.
cards-help = O que isso significa
cards-share = { $playable } de { $total } cards são jogáveis hoje.
cards-as-of = Versão { $tag }, contados em { $date }.
cards-missing = Não foi possível ler os números dos cards agora.

shot-duel-alt = Um duelo contra a IA da casa: seis criaturas atacam, três são bloqueadas
shot-duel = <b>Duelo tático</b> contra a IA da casa, com linhas de combate claras para cada atribuição.
shot-log-alt = Um duelo com o registro da partida aberto, três mágicas na pilha
shot-log = <b>O registro da partida</b> anota cada passo como um chat; o nome de um card abre a pré-visualização dele.
shot-ring-alt = Uma mesa de quatro lugares disposta em anel
shot-ring = <b>Mesas com até oito lugares</b> se organizam em volta do campo de batalha.
shot-lobby-alt = O lobby com a mensagem de um amigo e o convite dela para uma mesa
shot-lobby = <b>O lobby:</b> mesas abertas, amigos, convites e um chat com criptografia de ponta a ponta.
shot-room-alt = Uma sala onde três cadeiras vazias vão para a IA da casa
shot-room = <b>Lugares para todos:</b> pessoas, a IA da casa em cinco níveis ou um modelo de linguagem.
shot-builder-alt = O montador de decks filtrando o conjunto de cards por um anjo
shot-builder = <b>O montador de decks</b> pesquisa todo o conjunto de cards enquanto você digita.

about-eyebrow = O que é
about-title = Motor de regras completo e mesa 3D clara
about-engine = Prioridade, pilha, camadas, efeitos de substituição, habilidades disparadas, ações baseadas em estado e combate: o motor em Rust aplica o conjunto completo de regras e calcula as escolhas legais exatas a cada passo. O cliente apresenta somente essas ações válidas.
about-table = Campo de batalha, mãos, pilha, exílio e cemitérios em uma mesa 3D organizada, com linhas de combate claras e pré-visualizações instantâneas de cards. A arte dos cards vem do Scryfall enquanto você joga, ou muda a qualquer momento para uma exibição nítida do texto de oráculo.

gw-eyebrow = Online
gw-title = Gateways

open-central = Baylee Central no GitHub

## Download

dl-prerelease = Beta
dl-for = Baixar para { $system }
dl-size = { $size } MB
dl-checksum = SHA-256
dl-signature = Assinatura
dl-notes = Notas da versão
dl-unknown = Não conseguimos identificar o seu sistema por este navegador; escolha o seu abaixo.
dl-mobile = Ainda não há versão para celulares e tablets. O Baylee roda em um computador.
dl-mac-unsure = Este navegador não informa se o seu Mac tem um chip Apple ou um Intel. O arquivo acima é para chips Apple (M1 e posteriores); o arquivo para Macs Intel está abaixo.
dl-windows-unsure = Em um PC com Windows e processador ARM (um notebook Snapdragon, por exemplo), use o arquivo ARM abaixo.
dl-linux-unsure = O seu navegador não informa o seu processador; presumimos x86-64. O arquivo ARM está abaixo.
dl-missing = Esta versão não tem arquivo para o seu sistema. As versões mais antigas estão na <releases>página de versões</releases>.
dl-none = O GitHub não respondeu agora. Todas as versões estão na <releases>página de versões</releases>.
dl-yours = Seu sistema
dl-more-files = Mais arquivos ({ $count })
dl-installer = Instalador
dl-deb = Pacote Debian
dl-archive = Arquivo
dl-fallback = Arquivo para processadores mais antigos
dl-fallback-help = Para processadores x86-64 sem AVX2 (anteriores a cerca de 2013). As versões principais do Baylee precisam de AVX2 e não iniciam sem ele.
plat-windows-x64 = Windows
plat-windows-x64-sub = a maioria dos PCs: Intel ou AMD de 64 bits
plat-windows-arm64 = Windows em ARM
plat-windows-arm64-sub = notebooks Snapdragon e similares
plat-mac-arm64 = macOS
plat-mac-arm64-sub = chips Apple: M1 e posteriores
plat-mac-x64 = macOS em Intel
plat-mac-x64-sub = Macs com processador Intel
plat-linux-x64 = Linux
plat-linux-x64-sub = x86-64
plat-linux-arm64 = Linux em ARM
plat-linux-arm64-sub = ARM64 / aarch64
plat-none = Nenhum arquivo para este sistema nesta versão.
install-title = Primeira inicialização e requisitos
install-first-title = Primeira inicialização
install-first = As versões não têm assinatura de código, porque assinar custa dinheiro, então o Windows e o macOS avisam na primeira vez. No Windows, escolha <i>Mais informações</i> e depois <i>Executar assim mesmo</i>. No macOS, abra <i>Ajustes do Sistema → Privacidade e Segurança</i> e escolha <i>Abrir Mesmo Assim</i>.
install-check-title = Verificando um download
install-check = Ao lado de cada arquivo há um <code>.sha256</code> com a soma de verificação; os arquivos compactados também trazem um <code>.sig</code>, a assinatura Ed25519 que o cliente confere antes de se atualizar. No Linux, <code>sha256sum -c</code> verifica um arquivo; no macOS, <code>shasum -a 256</code>; no Windows, <code>certutil -hashfile</code>.
install-needs-title = Requisitos
install-windows = <b>Windows</b> 10 ou 11, x64 ou ARM64.
install-mac = <b>macOS</b> 11 ou posterior, em chip Apple ou Intel.
install-linux = <b>Linux</b> com glibc 2.35 ou mais recente (Ubuntu 22.04 e posteriores) e um driver Vulkan; o <code>.deb</code> instala tudo o mais de que precisa.
install-cpu = <b>Processador:</b> as versões x86-64 precisam de AVX2; para processadores mais antigos há um arquivo separado.

## Gateways

gws-title = Gateways
gws-online = Online
gws-offline = Offline
gws-version = Versão
gws-now = Agora
gws-website = Site
gws-address = Endereço
gws-copy = Copiar endereço
gws-copied = Endereço copiado.
gws-copied-short = Copiado
gws-copy-failed = Não foi possível copiar.
gws-failed = Não foi possível ler o diretório agora. Tente de novo em um minuto.

## Questions

faq-eyebrow = Perguntas
faq-title = Perguntas e respostas
faq-what-q = O que é o Baylee?
faq-what-a = Um jogo de fã gratuito de Magic: The Gathering para o seu computador. Tem um montador de decks, um motor de regras que confere cada passo, uma mesa para até oito jogadores e oponentes controlados pelo computador. Não é feito, aprovado nem apoiado pela Wizards of the Coast.
faq-cost-q = Quanto custa?
faq-cost-a = Nada, e isso não vai mudar: sem acesso pago, sem recursos pagos, sem anúncios.
faq-account-q = Preciso de uma conta?
faq-account-a = Não para jogar sozinho: o montador de decks e as partidas contra a IA da casa funcionam offline. Para as mesas online você precisa de uma conta Baylee; durante o beta aberto, qualquer pessoa pode criar uma neste site, sem custo.
faq-data-q = O que acontece com os meus dados?
faq-data-a = O mínimo possível; o <privacy>aviso de privacidade</privacy> diz exatamente o quê. Sua senha nunca sai do seu navegador; seus dados privados e suas mensagens de chat são criptografados no seu dispositivo antes de chegarem até nós.
faq-password-q = Esqueci minha senha. E agora?
faq-password-a = Você define uma nova senha com um código de redefinição, <reset>nesta página</reset>. Se a sua conta tiver um endereço de e-mail confirmado, você pode pedir lá que o código seja enviado a ele; caso contrário, um administrador lhe dá um. Seus dados criptografados só sobrevivem se você tiver o código de recuperação que recebeu ao criar a conta. Sem ele, você recomeça com um cofre vazio, e esses dados se perdem.
faq-gateway-q = O que é um gateway?
faq-gateway-a = Um servidor onde ficam as mesas online. Qualquer pessoa pode executar um; o <gateways>diretório</gateways> lista os que o Central conhece.
faq-systems-q = Em quais sistemas roda?
faq-systems-a = Windows, macOS e Linux, em x86-64 e ARM64. Ainda não há versão para celulares nem tablets.
faq-open-q = É de código aberto?
faq-open-a = O jogo é, sob a GNU Affero General Public License. O código do Baylee Central é público para que qualquer pessoa possa conferir como ele trata os dados.
faq-bug-q = Encontrei um erro. Onde informo?
faq-bug-a = No jogo, pressione <kbd>F8</kbd> para enviar um relatório com tudo o que é necessário. Ou então no rastreador de issues do GitHub, no Discord ou pelo <contact>formulário de contato</contact>.

## Contact

contact-eyebrow = Contato
contact-title = Escreva para nós
contact-name = Seu nome
contact-name-hint = Opcional.
contact-email = Seu endereço de e-mail
contact-email-hint = Opcional; somente para que possamos responder.
contact-message = Sua mensagem
contact-count = { $n } de 5.000 caracteres
contact-send = Enviar mensagem
contact-sending = Enviando …
contact-sent-title = Obrigado
contact-sent = Sua mensagem chegou. Se você informou um endereço, responderemos por lá.
contact-error-empty = Escreva uma mensagem primeiro.
contact-error-email = Isto não parece um endereço de e-mail.
contact-error-busy = Mensagens demais da sua rede agora. Tente de novo em uma hora.
contact-error-refused = A mensagem não foi aceita. Confira o tamanho e tente de novo.
contact-error-network = Não foi possível enviar a mensagem. Verifique sua conexão e tente de novo.
contact-privacy = O que você envia é guardado para que possamos responder, e apagado 90 dias depois de o tratarmos, ou após um ano, no máximo. Mais no <privacy>aviso de privacidade</privacy>.
contact-noscript = O formulário precisa de JavaScript. Você também pode falar conosco no Discord ou no GitHub.

## Legal notice

impressum-title = Aviso legal
impressum-lead = Informações exigidas pelo § 5 DDG (a lei alemã de serviços digitais). O Baylee é um projeto privado e sem fins comerciais.
impressum-show = Mostrar os dados
impressum-why = Os dados ficam atrás de um pequeno quebra-cabeça que o seu navegador resolve em alguns segundos, para manter afastados os robôs que coletam endereços. Nada mais é pedido a você.
impressum-solving = Resolvendo o quebra-cabeça … { $percent } %
impressum-name = Nome
impressum-address = Endereço
impressum-email = E-mail
impressum-phone = Telefone
impressum-write = Escrever um e-mail
impressum-call = Ligar
impressum-failed = Não foi possível carregar os dados. Tente de novo ou escreva para nós pelo <contact>formulário de contato</contact>.
impressum-noscript = Os dados precisam de JavaScript para aparecer. Sem ele, fale conosco pelo Discord ou pelo GitHub.
impressum-responsible = Responsável pelo conteúdo: a pessoa indicada acima.
impressum-dispute = Não somos obrigados nem dispostos a participar de procedimentos de resolução de conflitos perante uma junta de arbitragem de consumo.

## Fan content

fc-title = Conteúdo de fã não oficial
fc-eyebrow = Conteúdo de fã
fc-lead = O Baylee é um projeto de fã. Ele é permitido sob a política de conteúdo de fãs da Wizards of the Coast, que define as regras que ele segue.
fc-notice-title = O aviso em todas as páginas
fc-means-title = O que isso significa
fc-means = O Baylee não é feito, aprovado nem apoiado pela Wizards of the Coast. Magic: The Gathering, os nomes de seus cards, os nomes de suas coleções, as imagens dos cards e os símbolos de mana são propriedade da Wizards of the Coast LLC. O Baylee é gratuito para todos: não há acesso pago, nem recurso pago, nem cadastro por e-mail.
fc-images = O jogo mostra imagens de cards do Scryfall, sem alterações e com o crédito do artista; este site não mostra nenhuma. Os dados e as imagens dos cards são fornecidos pelo Scryfall; o Baylee não é afiliado ao Scryfall.
fc-policy = <policy>Política de conteúdo de fãs</policy> da Wizards of the Coast

## Privacy notice

privacy-title = Aviso de privacidade
privacy-lead = O que este site e o Baylee Central processam sobre você, por quê, com qual base legal e por quanto tempo.
privacy-updated = Em 12 de novembro de 2026
privacy-controller-title = Quem é o responsável
privacy-controller = O controlador nos termos do RGPD (Regulamento Geral sobre a Proteção de Dados da UE, o GDPR) é a pessoa física indicada no <impressum>aviso legal</impressum>, que informa seu nome, endereço postal e endereço de e-mail. Você também pode enviar dúvidas sobre privacidade pelo formulário de contato. Não há encarregado de proteção de dados; nenhum é exigido.
privacy-short-title = Em resumo
privacy-short = Sem rastreamento, sem publicidade, sem análise de uso, sem cookie a menos que você entre, e nada carregado de outros servidores: fontes, imagens e scripts vêm todos do baylee.app. Sua senha nunca chega até nós, e suas mensagens de chat e seus dados privados são criptografados no seu navegador antes de serem enviados.
privacy-hosting-title = Visitando o site
privacy-hosting = O site roda em um servidor que alugamos da netcup GmbH, na Alemanha. Quando você abre uma página, seu navegador envia o que toda requisição carrega: seu endereço IP, o endereço solicitado, a hora, o idioma do navegador e a identificação dele (user agent). O servidor precisa disso para responder; ele escolhe a versão de idioma a partir do idioma, e a página de download usa o user agent para sugerir o arquivo certo. O servidor web não guarda registro de acessos, então essas requisições não são armazenadas. As mensagens de operação do próprio servidor (seu diário do sistema) podem, em casos raros de erro, conter um endereço IP; elas são apagadas após 14 dias, no máximo. Base legal: art. 6.º, n.º 1, alínea f), do RGPD; nosso interesse legítimo é entregar o site de forma segura e confiável.
privacy-views-title = A contagem de páginas vistas
privacy-views = O site conta quantas vezes suas páginas são abertas: um único número para todas as páginas e todos os visitantes juntos, exibido na página inicial. Uma página aberta em um navegador soma um a ele. Nada sobre você é armazenado para isso: nem endereço IP, nem cookie, nem identificador, nem mesmo qual era a página.
privacy-device-title = O que o seu navegador guarda
privacy-device = Este site não define nenhum cookie até você entrar. O que o seu navegador guarda no seu dispositivo, e somente com JavaScript: sua escolha de cores, se você fez uma (<code>baylee-site-theme</code>); enquanto você está conectado, quando a sua sessão termina (<code>baylee-central-session</code>), sua chave de chat e as chaves confiáveis dos seus contatos (veja Chat); se você ativou as notificações, um número aleatório para este dispositivo (<code>baylee-push-installation-v1</code>, <code>baylee-push</code>); e, somente depois de você ter entrado, instalado o site como aplicativo ou ativado as notificações, uma cópia dos arquivos públicos do site (estrutura das páginas, fontes, scripts), para que ele inicie sem conexão e as notificações possam chegar. Se nenhuma das três situações valer mais, a cópia é removida na próxima vez que você abrir o site. Nada disso é enviado a nós, a menos que uma seção abaixo diga isso. Os registros de chat e de sessão são apagados quando você sai; você remove todo o resto limpando os dados do site no seu navegador. Base legal: § 25, n.º 2, item 2, da TDDDG (estritamente necessário para a função que você usa); para o número aleatório das notificações, o seu consentimento (§ 25, n.º 1, da TDDDG).
privacy-device-manager = Se você guarda seu código de recuperação no gerenciador de senhas do navegador, o envia por e-mail para si mesmo, o salva como arquivo ou o imprime no passo que o mostra, quem o guarda é o seu navegador, o seu programa de e-mail, os seus arquivos ou o seu papel, nunca este site: o código é gerado no seu navegador e nunca é enviado a nós. Limpar os dados do site não o remove de lá.
privacy-github-title = GitHub
privacy-github = A página de download e a página inicial mostram a versão mais recente, o estado dos testes e os números dos cards. Nosso servidor os pede ao GitHub, não o seu navegador; com isso, o GitHub não fica sabendo nada sobre você. Somente quando você clica em um link de download ou do GitHub o seu navegador se conecta ao GitHub (GitHub, Inc., EUA) e lhe envia seu endereço IP e o que toda requisição carrega; a partir daí vale o aviso de privacidade do próprio GitHub.
privacy-directory-title = O diretório de gateways
privacy-directory = Nosso servidor lê o diretório de gateways do Baylee Central e o exibe; nada sobre você está envolvido. Um gateway pode informar ao Central quantos jogadores estão conectados no momento, quantos deles estão em uma partida e quantas mesas abertas e partidas tem, e o Central conta as suas contas; o site mostra esses números. São apenas contagens, nunca nomes, e os números de um gateway são uma declaração do próprio operador.
privacy-account-title = Sua conta
privacy-account = Para uma conta, o Baylee Central armazena: o ID da sua conta, seu nome de entrada, seu nome de exibição com a sua tag, quando e como a conta foi criada, um registro de senha OPAQUE (que permite que você prove sua senha sem que nosso servidor jamais a conheça), sua chave mestra criptografada duas vezes (pela sua senha e pelo seu código de recuperação), o documento criptografado da sua conta e seus objetos criptografados (ilegíveis para nós; vemos apenas o tamanho, a revisão e a hora), qual versão dos termos você aceitou e quando, a hora da sua última entrada, suas sessões, uma função de administrador, se você tiver uma, e os gateways em que você se registra, com seus tokens. Outros jogadores veem seu nome de exibição e sua tag, nunca o seu nome de entrada. Se um administrador lhe entregar um código de redefinição, armazenamos somente o resumo (digest) dele até que o código seja usado ou expire. O que os administradores fazem em uma conta ou em um gateway é mantido em um registro por três anos (quem, o quê, qual conta ou gateway, quando). Base legal: art. 6.º, n.º 1, alínea b), do RGPD (a conta que você pediu); para a hora da última entrada e para o registro, art. 6.º, n.º 1, alínea f), do RGPD (nosso interesse legítimo em uma operação segura e rastreável). Sem um nome de entrada e uma senha não há conta; você não precisa de endereço de e-mail. Você pode criar uma conta a partir dos 16 anos. Mantida até você excluir a conta; ela é então removida do sistema em funcionamento de imediato, e dos backups em 12 meses, no máximo. As sessões terminam após 30 minutos sem uso e, no máximo, 12 horas depois da entrada.
privacy-email-title = Endereço de e-mail
privacy-email = Um endereço de e-mail é opcional; uma conta não precisa de nenhum. Se você informar um, o Baylee Central armazena o endereço, o idioma em que seus e-mails são escritos e quando você o confirmou. Um endereço que você indicou e que aguarda o seu código é mantido até você confirmá-lo ou retirá-lo e, no máximo, um dia depois de o código ter expirado. De cada código que enviamos por e-mail, armazenamos apenas um resumo (digest), até o código ser usado ou expirar, e, por 24 horas, os horários em que códigos de redefinição foram enviados a você, para que no máximo três por dia sejam enviados. Usamos o endereço para quatro finalidades: enviar o código que o confirma, enviar um código de redefinição quando você ou qualquer pessoa que conheça o seu nome de entrada pedir um, avisar o endereço antigo quando ele é substituído ou removido, e avisar o endereço confirmado quando um código de redefinição foi usado para definir uma nova senha, quando a senha foi alterada ou quando um administrador gerou um código de redefinição para a conta. O e-mail é enviado pelo serviço de e-mail próprio do Baylee, no mesmo servidor, por meio da mailbox.org (Heinlein Hosting GmbH, Berlin), que atua como operadora e recebe o endereço e o e-mail. Base legal: art. 6.º, n.º 1, alínea b), do RGPD (a conta que você pediu). Mantido até você remover o endereço ou excluir a conta.
privacy-cookie-title = O cookie de sessão
privacy-cookie = Ao entrar, é definido um cookie, <code>__Host-central</code>: um número aleatório que identifica a sua sessão. Ele é enviado somente ao baylee.app, não pode ser lido por scripts e termina com a sessão, após 12 horas, no máximo. Ele é estritamente necessário para manter você conectado; não precisa de consentimento. Base legal: § 25, n.º 2, item 2, da TDDDG e art. 6.º, n.º 1, alínea b), do RGPD.
privacy-chat-title = Chat e contatos
privacy-chat = As mensagens são criptografadas no seu dispositivo para os dispositivos de cada destinatário; o Baylee Central as armazena somente de forma criptografada e não consegue lê-las. O que o Central pode ver e armazena: seus contatos, pedidos de contato e bloqueios, os grupos com seus nomes, membros e convites, quem enviou uma mensagem a quem, quando e de que tamanho, e as chaves públicas dos seus dispositivos. Seus contatos e os membros dos seus grupos veem seu nome de exibição com a sua tag e suas mensagens. Seu navegador guarda a sua chave de chat (selada sob uma chave que somente este navegador possui, em seu armazenamento <code>baylee-chat</code>) e as chaves dos seus contatos que você marcou como confiáveis (criptografadas, <code>baylee-chat-trust</code>) enquanto você está conectado. As mensagens e as entregas são mantidas até você excluir a conta; ainda não há um prazo automático de retenção mais curto. Cada remetente e cada dispositivo guarda no máximo 5.000 mensagens; depois disso, as novas são recusadas. Base legal: art. 6.º, n.º 1, alínea b), do RGPD.
privacy-push-title = Notificações
privacy-push = Se você ativar as notificações em um navegador, o seu navegador se registra no serviço de push dele, e o Central armazena o endereço desse serviço para este navegador, a chave pública e o segredo de autenticação do navegador, o seu idioma, um número aleatório para este dispositivo e a sua escolha de categorias. O serviço de push pertence ao fabricante do seu navegador: Google (Firebase Cloud Messaging), Mozilla ou Apple, todos com sede nos EUA. Uma notificação leva, criptografados, apenas o seu tipo (por exemplo, "nova mensagem") e o seu idioma, nunca nomes nem texto; o serviço de push vê dados de conexão como endereço, hora e tamanho. Esses dados são apagados quando você desativa as notificações neste dispositivo, quando você sai ou quando você exclui a conta; um endereço não renovado por 90 dias deixa de ser usado, e as notificações em espera expiram após um dia. Base legal: art. 6.º, n.º 1, alínea a), do RGPD e § 25, n.º 1, da TDDDG (o seu consentimento). Você pode retirá-lo a qualquer momento nas configurações da sua conta; o que ocorreu até então continua lícito.
privacy-counters-title = Proteção contra abuso
privacy-counters = Para impedir que senhas sejam adivinhadas e evitar sobrecarga, o Central conta requisições por endereço IP (registros, tentativas de entrada, redefinições de senha, leitura do diretório e, de modo geral, quantas requisições um endereço envia em um minuto), somente na memória: os registros e as tentativas de entrada que não foram concluídas com a senha correta, por até um dia; todo o resto, por no máximo uma hora. Depois de uma entrada com a senha correta, o Central lembra por 30 dias, de novo somente na memória, que a sua conta entrou a partir desse endereço (um endereço IPv4, ou a primeira metade de um endereço IPv6; no máximo oito por conta), para que tentativas de outras pessoas na sua conta não deixem você de fora desse endereço. Quem usa o mesmo endereço e sabe o seu nome de entrada pode perceber, pelas respostas do Central, que a sua conta entrou a partir dele. Uma nova senha esquece esses endereços, exceto aquele a partir do qual você a definiu; excluir a conta os esquece todos, e uma reinicialização do serviço também. Antes de um registro, o seu navegador também resolve um quebra-cabeça que consome memória e tempo de processamento; ele consiste em números aleatórios, não contém nada sobre você e é esquecido no máximo depois de vinte minutos. O Central recalcula uma pequena parte dele para conferir a resposta do seu navegador. A dificuldade depende de quantas contas estão sendo criadas e de a requisição parecer de um programa: se ela traz os cabeçalhos que todo navegador envia, se o seu navegador informa que um programa o está controlando e quantos quebra-cabeças o seu endereço e o bloco de rede ao redor dele (os três primeiros números de um endereço IPv4, os primeiros 48 bits de um endereço IPv6) pediram na última hora. Disso o Central guarda apenas as contagens, na memória, por no máximo uma hora. Por conta, o Central conta pedidos de contato e buscas por outros jogadores, para que ninguém inunde os outros de pedidos. Base legal: art. 6.º, n.º 1, alínea f), do RGPD; nosso interesse legítimo é proteger as contas e o serviço contra abuso.
privacy-contact-title = O formulário de contato
privacy-contact = O formulário envia a sua mensagem, o idioma da página e a hora ao serviço de feedback do Baylee, no mesmo servidor, e o seu nome e endereço de e-mail somente se você os informar; ambos são opcionais. O serviço também anota se a mensagem já foi tratada. Seu endereço IP não é armazenado; ele é apenas contado na memória por uma hora, para impedir uma enxurrada de mensagens. Somente a pessoa indicada no aviso legal lê as mensagens; nada é repassado. Uma mensagem é apagada 90 dias depois de ser tratada, após um ano, no máximo, ou antes, se você pedir. Base legal: art. 6.º, n.º 1, alínea f), do RGPD (nosso interesse legítimo em responder às mensagens), ou alínea b), quando a sua mensagem diz respeito à sua conta.
privacy-impressum-title = O quebra-cabeça do aviso legal
privacy-impressum = O aviso legal só é exibido depois que o seu navegador resolve um quebra-cabeça; isso protege o endereço de programas que coletam endereços. O quebra-cabeça consiste em números aleatórios e não contém nada sobre você. Para isso, o serviço de feedback guarda somente na memória, nunca em disco e nunca em um registro: quantos quebra-cabeças o seu endereço IP pediu (por uma hora; um endereço IPv6 pela sua primeira metade), se ele seguiu um link escondido das pessoas (por 24 horas) e quais quebra-cabeças foram resolvidos (por cinco minutos, sem endereço). Três respostas de sim ou não sobre o seu navegador definem a dificuldade do quebra-cabeça e não são guardadas: se um programa o controla, se ele não informa nenhum idioma e se ele desenha sem uma placa de vídeo. Ninguém é recusado; um navegador que parece um programa recebe um quebra-cabeça mais longo. Base legal: art. 6.º, n.º 1, alínea f), do RGPD (nosso interesse legítimo em proteger o endereço de uma pessoa física).
privacy-recipients-title = Destinatários e países
privacy-recipients = Não vendemos dados nem repassamos nenhum para publicidade. Os destinatários são: a netcup GmbH (hospedagem na Alemanha, como operadora nos termos do art. 28 do RGPD); a mailbox.org (Heinlein Hosting GmbH, Berlin), que entrega os e-mails a um endereço de e-mail que você informou, como operadora; os administradores do Baylee Central, que, para a administração, veem seu nome de entrada, seu nome de exibição com a sua tag, a origem, a criação e a última entrada da sua conta, e seus gateways, nunca os seus dados criptografados; um gateway em que você entra com a sua conta (ele recebe o ID da sua conta, o nome de exibição e a tag, não a sua senha); seus contatos e os membros dos seus grupos (nome de exibição, tag e suas mensagens); autoridades somente quando a lei nos obriga. Países fora do Espaço Econômico Europeu: somente quando você ativa as notificações é que o serviço de push do seu navegador (Google, Mozilla ou Apple, EUA) as transporta. Gateways operados por terceiros podem estar em qualquer país; entrar ou não em um deles é uma decisão sua. Fora isso, não transferimos dados para países fora do Espaço Econômico Europeu.
privacy-rights-title = Seus direitos
privacy-rights = Você tem o direito de acesso (art. 15 do RGPD), de retificação (art. 16), de apagamento (art. 17), de limitação do tratamento (art. 18) e de portabilidade dos dados (art. 20). Você pode se opor a qualquer momento, por motivos relacionados à sua situação particular, ao tratamento baseado em nossos interesses legítimos nos termos do art. 6.º, n.º 1, alínea f) (art. 21). Você pode retirar um consentimento a qualquer momento, para o futuro (art. 7.º, n.º 3). Os dados da sua conta podem ser exportados e a conta pode ser excluída nas configurações da sua conta; para todo o resto, basta uma mensagem pelo formulário de contato ou para o endereço do aviso legal. Você pode apresentar uma reclamação a uma autoridade de supervisão de proteção de dados (art. 77), por exemplo, a do lugar onde você mora ou o Comissário Estadual para a Proteção de Dados e a Liberdade de Informação da Renânia do Norte-Vestfália. Nenhuma lei ou contrato obriga você a nos fornecer dados; sem um nome de entrada e uma senha, porém, não há conta. Não há decisões automatizadas nem definição de perfis (art. 22).
privacy-objection-title = Seu direito de oposição
privacy-objection = Quando tratamos dados com base no art. 6.º, n.º 1, alínea f), do RGPD (entrega do site, proteção contra abuso, formulário de contato, quebra-cabeça do aviso legal, hora da sua última entrada, registro de administração, relatórios de erro e contagem de uso), você pode se opor a qualquer momento por motivos relacionados à sua situação particular (art. 21 do RGPD). Nesse caso, deixamos de tratar os dados, a menos que possamos demonstrar razões legítimas imperiosas que prevaleçam sobre os seus interesses, ou que o tratamento sirva para declarar, exercer ou defender direitos em processos judiciais. Basta uma mensagem informal pelo formulário de contato ou para o endereço do aviso legal.
privacy-links-title = Links para outros serviços
privacy-links = O site tem links para o GitHub, o Discord e o WhatsApp, entre outros. São links simples: nada é incorporado e, até você clicar em um deles, nenhum desses serviços fica sabendo da sua visita. Quando você clica, seu navegador não informa de qual página você veio (sem referenciador). Depois disso, vale o aviso de privacidade de cada provedor, e eles podem tratar dados fora da UE; o que os outros membros dos grupos da comunidade veem de você ali é decidido por esses serviços.
privacy-reports-title = Relatórios de erro e a contagem de uso do jogo
privacy-reports = Um relatório de erro que você envia do jogo com F8 contém o seu tipo, o seu texto, a versão do cliente e os anexos que você deixa marcados; a página de contato os lista. Ele é apagado 90 dias depois de ser tratado, após três anos, no máximo. Base legal: art. 6.º, n.º 1, alínea f), do RGPD (nosso interesse legítimo em um jogo sem erros). O cliente envia relatórios de falha somente se você os permitiu (art. 6.º, n.º 1, alínea a), do RGPD; você pode retirar isso a qualquer momento nas configurações do cliente). Durante um jogo offline, o cliente envia uma vez por minuto um identificador aleatório desse jogo e nada mais; nós o guardamos somente na memória e o esquecemos 150 segundos após o seu último sinal de vida (art. 6.º, n.º 1, alínea f), do RGPD; pode ser desativado em "Send anonymous usage count" [Enviar contagem de uso anônima]). Como em toda conexão, o seu endereço IP chega até nós.

## Terms

terms-title = Termos de uso
terms-draft = Estes termos são um rascunho e ainda não estão em vigor.
terms-version = Versão { $version }, atualizada em { $date }.
# Shown only in the languages a machine translated: every one but German and English.
machine-note = Este site foi traduzido por uma máquina, e nenhum falante do seu idioma o leu ainda. O jogo em si fala somente inglês e alemão por enquanto. <link>Avise-nos sobre um erro.</link>
legal-machine = Esta página é uma tradução automática, oferecida como ajuda. Onde ela diferir do texto em <de>alemão</de> ou em <en>inglês</en>, valem esses textos.
terms-machine = Os termos do Central existem somente em alemão e inglês. Abaixo está o texto em inglês; há também um <de>em alemão</de>.

## Errors

not-found-title = Esta página não existe
not-found-lead = O endereço pode ter sido digitado errado, ou a página mudou de lugar.
gone-title = Esta página foi removida definitivamente
gone-lead = O que estava aqui foi removido. A página inicial mostra por onde continuar.
error-home = Para a página inicial

## Baylee's remembrance (approved by her family on 27 October 2026; keep word for word)
baylee-dedication = Para a Baylee. Para sempre sob o sol.
baylee-invitation = Conheça a alminha por trás deste mundo
baylee-open = Lembre-se da Baylee · abra o álbum de fotos dela
baylee-eyebrow = O coração deste projeto
baylee-date = Em amorosa memória · 26 de outubro de 2026
baylee-close = De volta ao mundo dela
baylee-previous = Foto anterior
baylee-next = Próxima foto
baylee-photo1 = Baylee aninhada no macio edredom branco.
baylee-photo2 = Baylee olhando para fora do edredom de flores.
baylee-photo3 = Descansando em sua caminha redonda e cinza.
baylee-photo4 = Baylee dormindo, toda enroladinha.
baylee-photo5 = Um momento de calma, aconchegada no edredom.

baylee-storyTitle = Para a Baylee. Para sempre sob o sol.
baylee-together = Conosco · 17 de novembro de 2025 – 26 de outubro de 2026
baylee-chapters = A história dela
baylee-chapterPrevious = Capítulo anterior
baylee-chapterNext = Próximo capítulo
baylee-photos = Nossas lembranças
baylee-read = Role para ler

baylee-chapter1Title = Um lar para a Baylee
baylee-chapter1Body =
    A Baylee chegou em casa conosco em 17 de novembro de 2025.

    Minha esposa e eu a adotamos do abrigo de animais de Krefeld. Estimavam que ela tivesse cerca de treze anos. Tinha tumores mamários malignos, e o câncer já havia se espalhado para o fígado. A Baylee era uma gata em cuidados paliativos. Sabíamos que o nosso tempo juntos seria limitado. Conosco, ela teria um lar onde seria amada.

    Logo ela nos mostrou com que rapidez tomaria aquele lar como seu.

    No primeiro dia, ela se esgueirou para o meu colo enquanto eu trabalhava. E na primeira noite, rasgou um saco de ração que eu havia encomendado para os gatos dos meus pais e se serviu. É claro que ela tinha recebido a própria comida e comido bem naquela noite. Mas a Baylee, ao que parece, havia decidido que aquele estoque ainda merecia uma investigação mais de perto.

    Nossa ratinha tinha um apetite impressionante – e uma energia incrível.

baylee-chapter2Title = Confiança crescente
baylee-chapter2Body =
    Quando notamos que ela bebia muita água, a levamos ao veterinário. Um exame de sangue revelou um problema na tireoide, o que também explicava a grande fome e o jeito agitado. Ela recebeu remédio para a tireoide e um curso temporário de antibióticos, porque os marcadores de inflamação estavam elevados.

    Quando, de repente, ela começou a se esfregar em tudo, a miar muito e a ficar incomumente carinhosa e apegada a nós, primeiro nos preocupamos que estivesse reagindo mal a um dos remédios novos. Outro exame trouxe a explicação: nossa queridinha estava no cio. Ela não havia sido castrada.

    Passamos por esse período juntos também. A Baylee confiava cada vez mais em nós, e nos aproximamos cada vez mais uns dos outros.

baylee-chapter3Title = Nossa princesinha
baylee-chapter3Body =
    Ainda me lembro exatamente de como ela ficou feliz quando ganhou o arranhador de árvore. Ela se sentou de imediato na plataforma mais alta e olhou satisfeita em volta do apartamento. Curiosamente, nunca mais usou aquele ponto. O motivo continuou sendo um segredo dela.

    Outros lugares raramente ficavam vagos: o meu colo, as minhas pernas e a nossa cama. Ali ela costumava dormir em cima de mim ou da minha esposa. O melhor de tudo era conseguir se deitar nos dois ao mesmo tempo.

    Ela desfilava pelo apartamento com o rabo erguido, como se soubesse muito bem que era a princesa dali. Tinha personalidade e sabia deixar claros os seus limites. Conosco, podia ser inteiramente ela mesma. Suas manias, suas exigências e sua teimosia também eram amadas.

    Eu nunca havia criado um vínculo tão profundo com um animal. A Baylee encontrou um lugar bem fundo no coração da minha esposa e no meu.

baylee-chapter4Title = Horas ao sol
baylee-chapter4Body =
    Quando o tempo esquentou, ela descobriu a varanda. Adorava sentir nos pelos os primeiros raios de sol do dia. Em pouco tempo, passava quase o dia inteiro lá fora.

    Os pombos e os passarinhos do jardim eram um fascínio à parte. Ela os observava com atenção, ficava à espreita e tentava, uma vez atrás da outra, assustá-los. Entre banhos de sol e observação de pássaros, nossa princesinha tinha uma agenda diária bem cheia.

baylee-chapter5Title = Uma mentezinha só dela
baylee-chapter5Body =
    Enquanto isso, a doença dela continuava a avançar.

    O tumor inflamou, e a Baylee não parava de lamber e abrir a área afetada. No veterinário, ela ganhou sua primeira roupa de recuperação, uma vermelha. Dali em diante, tinha de usar uma todos os dias para não machucar mais a ferida.

    Ela se acostumou rápido a usá-la. Ser vestida, porém, foi algo de que ela não gostou até o último dia. Nem mesmo uma tigela de comida conseguia atraí-la muito nessa hora. Por maior que fosse o apetite, a opinião dela sobre a roupa era inequívoca.

    A ferida piorou. Soltava pus, acabou por sangrar e passou a ter um cheiro desagradável. Ao mesmo tempo, a Baylee continuava nos mostrando quanta alegria ainda encontrava no dia a dia. Brincava, comia, explorava os arredores e subia no arranhador e nos armários.

    Às vezes ela ficava deitada por horas na beira da cama, ao nosso lado, esperando que finalmente acordássemos e fizéssemos o café da manhã dela. Se demorava demais, ela simplesmente passava por cima de nós. Nossos rostos não eram obstáculo.

    Ela tinha ideias bem definidas sobre comida. O gosto dela era mais do tipo “pizza congelada”: depois de um tempo, ração de alta qualidade já não a empolgava, e variedade era essencial. A mesma comida duas vezes seguidas estava fora de cogitação. Frango com fígado, qualquer coisa com queijo, atum ou salmão – era isso que a deixava feliz.

baylee-chapter6Title = Com o rabo erguido
baylee-chapter6Body =
    O tumor cresceu e passou a verter cada vez mais. A Baylee estava visivelmente com dor. Quando a dor diminuía, ela voltava a ser a nossa gatinha, andando pelos cômodos com o rabo erguido e reivindicando as suas pessoas só para si.

    Víamos a força dela se esvaindo aos poucos. Mas o apetite, a curiosidade, o jeito carinhoso e o ronronar alto ainda estavam lá.

baylee-chapter7Title = Dizendo adeus
baylee-chapter7Body =
    Na noite de sexta-feira, 23 de outubro de 2026, ela teve sintomas que no início pareciam um resfriado. O nariz escorria, e a respiração estava funda e difícil, como se ela lutasse por ar.

    No sábado ela pareceu mais animada de novo. A respiração incomum e o nariz escorrendo continuaram, mas ela voltou a mostrar a energia e o amor pela vida que conhecíamos tão bem.

    A manhã de domingo foi difícil. Os olhos dela lacrimejavam, ela tinha dificuldade para respirar e sentia dor. Dei-lhe um remédio para a dor. Depois de cerca de uma hora ela se sentiu melhor, mas a respiração continuava diferente. Sabíamos que precisávamos levá-la ao veterinário.

    Na segunda-feira de manhã o estado dela piorou de novo. Felizmente, minha esposa estava de folga naquele dia. Ligamos para a clínica, marcamos uma consulta e levamos a Baylee.

    A radiografia mostrou até onde o câncer havia se espalhado. Disseram-nos que apenas cerca de um terço dos pulmões dela ainda estava disponível para respirar. Deixá-la continuar vivendo significaria que ela receberia cada vez menos ar.

    Tivemos de nos despedir.

    Na segunda-feira, 26 de outubro de 2026, por volta do meio-dia, nossa querida Baylee foi posta para dormir.

baylee-chapter8Title = Para sempre sob o sol
baylee-chapter8Body =
    Ela ficou conosco por pouco menos de um ano. Nesse tempo, tornou-se parte da nossa família e uma amiga muito próxima. Ela pertencia aos nossos dias de trabalho, às nossas noites e ao nosso acordar de manhã. Às horas de sol na varanda e aos pequenos hábitos que formam uma vida em comum.

    Agora ela faz falta em todos esses momentos.

    O vazio ainda é recente e profundo. Sentimos falta do ronronar dela, do peso dela sobre as nossas pernas, do rabo erguido e da expectativa inquestionável de que finalmente cuidaríamos do café da manhã dela.

    Se existe algo depois desta vida, seja o que for, então desejo reencontrar essa alma um dia.

    Até lá, vamos carregar a nossa ratinha no coração para sempre.

    Por isso este projeto de hobby é dedicado à Baylee. Ao espírito dela, à sua proximidade, à sua alegria com o sol e com os pássaros. E ao amor que cresceu durante o nosso tempo juntos.

    Aqui ela pode dormir em paz na sua nuvem, ao sol quentinho, cercada de flores, pombos e passarinhos do jardim.

    Descanse em paz, minha querida Baylee.

baylee-photoOpen = Ver a foto inteira
baylee-photoClose = Fechar a foto
baylee-zoomIn = Aumentar
baylee-zoomOut = Diminuir
baylee-fit = Ajustar

baylee-photo6 = Baylee, aconchegada entre os cobertores.
baylee-photo7 = Toda aninhada na sua roupa de recuperação vermelha.
baylee-photo8 = Um cantinho ensolarado numa caixa de papelão.
baylee-photo9 = Bem acomodada e totalmente relaxada.
baylee-photo10 = O lugar favorito dela: bem ao nosso lado.

## Errors shared by the forms

error-busy = Tentativas demais. Tente de novo mais tarde; depois de várias senhas erradas, isso pode levar uma hora, no máximo um dia.
error-unavailable = O Baylee Central não está respondendo agora. Tente de novo daqui a pouco.
error-network = Sem conexão. Verifique sua rede e tente de novo.
error-failed = Algo deu errado. Tente de novo; se continuar acontecendo, avise-nos.
error-again = Tentar de novo
password-show-label = Mostrar a senha

## Signing in

signin-title = Entrar
signin-lead = Sua conta Baylee, para mesas online, contatos e o chat.
signin-name = Nome de entrada
signin-password = Senha
signin-submit = Entrar
signin-missing = Informe seu nome de entrada e sua senha.
signin-working = Protegendo sua senha … isso leva um instante e acontece somente no seu navegador.
signin-wrong = Esse nome de entrada e essa senha não combinam. Confira os dois e tente de novo.
signin-not-moved = Esta conta ainda não foi migrada para o Baylee Central. Entre uma vez com o cliente do Baylee; ele migra a sua conta.
signin-no-account = Ainda sem conta? <register>Crie uma</register>.
signin-forgot = Esqueceu a senha? <reset>Defina uma nova com um código de redefinição</reset>.
register-have-account = Já tem uma conta? <signin>Entrar</signin>.
dialog-close = Fechar
signin-noscript = Entrar precisa de JavaScript: sua senha é processada no seu navegador e nunca é enviada a nós.

## Registering

register-title = Criar uma conta
register-lead = Para mesas online, contatos e o chat. Gratuita e aberta a todos no beta aberto.
register-name = Nome de entrada
register-name-hint = Privado, só para entrar: de 3 a 24 letras minúsculas, dígitos, pontos, hifens ou sublinhados.
register-display = Nome de exibição
register-display-hint = Como os outros jogadores veem você: de 3 a 16 letras, dígitos, - e _. Sua tag é sorteada para você.
register-password = Senha
register-password-hint = Pelo menos 8 caracteres. Uma frase longa é fácil de lembrar e difícil de adivinhar.
register-repeat = Repita a senha
register-repeat-hint = A mesma mais uma vez, para pegar um erro de digitação.
register-age = Tenho 16 anos ou mais.
register-consent = Aceito os <terms>termos de uso</terms>. Como tratamos seus dados está no <privacy>aviso de privacidade</privacy>.
register-submit = Criar conta
register-error-name = O nome de entrada precisa ter de 3 a 24 letras minúsculas, dígitos, pontos, hifens ou sublinhados.
register-error-display = O nome de exibição precisa ter de 3 a 16 letras ou dígitos; - e _ somente no meio.
register-error-reserved = Este nome de exibição é reservado. Escolha outro.
register-error-short = A senha precisa ter pelo menos 8 caracteres.
register-error-long = A senha é longa demais.
register-error-repeat = As duas senhas não são iguais.
register-error-age = Você precisa ter 16 anos ou mais para ter uma conta.
register-error-consent = Aceite os termos de uso para criar uma conta.
register-error-taken = Este nome de entrada já está em uso. Escolha outro.

## The recovery code

code-title = Seu código de recuperação
code-handle = Seu identificador: { $handle }
code-lead = Somente com este código seus dados criptografados sobrevivem se você esquecer a senha. Nós não o conhecemos e não podemos restaurá-lo. Guarde-o em um lugar seguro, longe deste dispositivo.
code-shown-once = Mostrado só agora
code-download = Salvar como arquivo de texto
code-kept = Salvei meu código de recuperação
code-not-kept = Salve o código primeiro e depois confirme.
code-continue = Continuar
code-later = Agora não
code-copy = Copiar
code-copied = Copiado.
code-copy-failed = Copiar não funcionou aqui. Selecione o código e copie-o à mão.
code-print = Imprimir
code-manager = Salvar no gerenciador de senhas
code-manager-id = Código de recuperação do Baylee ({ $name })
code-manager-offered = O código foi oferecido ao seu navegador. Se ele perguntou a você, confira se o salvou.
code-manager-failed = Seu navegador não aceitou o código. Guarde-o de outra forma.
code-mail = Enviar a mim mesmo por e-mail
code-mail-warning = Cuidado: o código fica então na sua caixa de e-mail. Quem puder ler o seu e-mail também pode pedir que um código de redefinição seja enviado para lá e, com os dois, redefinir a sua entrada e abrir os seus dados criptografados.
code-mail-subject = Meu código de recuperação do Baylee
code-sheet-title = Código de recuperação do Baylee
code-sheet-name = Nome de entrada: { $name }
code-sheet-what = Com este código, seus dados criptografados do Baylee sobrevivem a uma senha esquecida: você o informa na página de redefinição, junto com um código de redefinição.
code-sheet-keep = Guarde-o onde só você possa alcançá-lo. Ninguém do Baylee jamais vai pedi-lo.
code-email-label = Endereço de e-mail (opcional)
code-email-hint = Para um código de redefinição, caso você esqueça a senha. Depois deste passo, enviamos um código por e-mail para confirmar o endereço; você também pode adicionar um depois, na sua conta.
code-email-step-title = Confirme seu endereço de e-mail
code-email-step-lead = Digite aqui o código do e-mail, ou depois, na página de segurança da sua conta.
code-email-step-done = Continuar para a sua conta

## Setting a new password with a reset code

reset-title = Definir uma nova senha
reset-lead = Para quando você esqueceu a senha: com um código de redefinição, você define aqui uma nova. O código chega por e-mail ao seu endereço confirmado ou vem de um administrador.
reset-noscript = Definir uma nova senha precisa de JavaScript: sua senha e seu código de recuperação são processados no seu navegador e nunca são enviados a nós.
reset-code = Código de redefinição
reset-code-hint = Por e-mail ou de um administrador. Começa com RESET e só vale uma vez: por uma hora, se veio por e-mail; por 24 horas, se um administrador o gerou.
reset-data = Seus dados criptografados
reset-keep = Tenho meu código de recuperação. Meus dados ficam.
reset-new = Não tenho. Recomeçar com um cofre vazio.
reset-recovery = Código de recuperação
reset-recovery-hint = O código que você salvou ao criar sua conta. Ele fica no seu navegador e nunca é enviado a nós.
reset-warning = Com um cofre vazio, seus dados criptografados se perdem de vez: as configurações salvas na sua conta, seus backups de decks e seu histórico de chat. Ninguém pode trazê-los de volta, nem nós. Sua conta, seu nome de exibição e seus contatos ficam. Você recebe um novo código de recuperação.
reset-warning-confirm = Entendi que esses dados se perdem de vez.
reset-submit = Definir nova senha
reset-missing = Informe seu nome de entrada, o código de redefinição e uma nova senha.
reset-recovery-missing = Informe seu código de recuperação ou escolha o cofre vazio.
reset-error-code = Isso não é um código de redefinição. Ele começa com RESET, seguido de oito grupos de quatro caracteres.
reset-error-recovery = Este código de recuperação não abre os dados da sua conta. Confira e tente de novo. Nada foi alterado, e seu código de redefinição continua valendo.
reset-error-confirm = Confirme primeiro que você sabe o que se perde.
reset-refused = Esse nome de entrada e esse código de redefinição não combinam, ou o código expirou ou já foi usado. Confira os dois. Se nada adiantar, peça um novo código.
reset-done-title = Sua nova senha está definida
reset-done-kept = Seus dados e seu código de recuperação ficam como estavam. Você foi desconectado em todos os lugares; entre com a nova senha.
reset-done-new = Você recomeça com um cofre vazio e foi desconectado em todos os lugares. Abaixo está seu novo código de recuperação; o antigo não vale mais.
reset-unsure-title = Nenhuma resposta chegou
reset-unsure = A conexão caiu antes de o Baylee Central responder, então não sabemos dizer se sua nova senha foi definida. Tente entrar com ela. Se não der certo, nada foi alterado: seu código de redefinição continua valendo, informe-o aqui de novo.
reset-unsure-code = Se sua nova senha funcionar, este é seu novo código de recuperação e o antigo não vale mais; ele não pode ser mostrado de novo, então salve-o agora. Se sua nova senha não funcionar, este código não vale nada.
reset-to-signin = Entrar
reset-mail-title = Sem código de redefinição?
reset-mail-lead = Informe seu nome de entrada. Se a sua conta tiver um endereço de e-mail confirmado, enviaremos um código de redefinição para ele.
reset-mail-name-hint = O nome com que você entra, não o seu endereço de e-mail.
reset-mail-submit = Enviar um código por e-mail
reset-mail-error-name = Isso não é um nome de entrada. Informe o nome com que você entra, não o seu endereço de e-mail.
reset-mail-sent = Se esta conta tiver um endereço de e-mail confirmado, um código de redefinição está a caminho dele. Ele vale por uma hora.
reset-mail-busy = Muitas solicitações por enquanto. Tente de novo mais tarde.
reset-mail-none = Este servidor não envia e-mails. Peça um código de redefinição a um administrador.
reset-mail-early = Foi rápido demais. Tente de novo em instantes.

## Changing the password

password-new = Nova senha
password-new-repeat = Repita a nova senha
password-current = Senha atual
password-change-submit = Alterar senha
password-missing = Informe sua senha atual e uma nova.
password-error-current = A senha atual não está certa. Confira e tente de novo.
password-changed = Sua senha foi alterada. Você continua conectado aqui e foi desconectado em todos os outros lugares, inclusive no cliente do Baylee; entre lá com a nova senha.
password-unsure = Nenhuma resposta chegou, então não sabemos dizer se sua senha foi alterada. Na próxima vez que entrar, tente primeiro a nova senha, depois a antiga.

## An account's e-mail address

email-title = Endereço de e-mail
email-lead = Um endereço de e-mail é opcional. Nós o usamos para enviar um código de redefinição quando você pedir e para avisar quando o endereço for substituído ou removido.
email-warning = Quem puder ler esta caixa de e-mail pode redefinir a sua entrada. Seus dados criptografados só sobrevivem a uma redefinição com o seu código de recuperação.
email-no-mail = Este servidor não envia e-mails, então aqui não é possível definir nem confirmar um endereço de e-mail.
email-unavailable = O endereço de e-mail não pode ser exibido agora.
email-password-hint = Sua senha atual, para que ninguém mais neste navegador decida quem pode redefinir a sua entrada.
email-submit = Enviar código de confirmação
email-missing = Informe um endereço de e-mail e a sua senha.
email-error-address = Não conseguimos escrever para este endereço. Informe um único endereço simples, como name@example.org; letras acentuadas e tremas não funcionam.
email-current = Seu endereço
email-confirmed-at = Confirmado
email-waiting = Aguarda o código
email-change = Alterar
email-change-lead = Seu endereço atual continua em uso até você confirmar o novo.
email-change-cancel = Manter o endereço atual
email-remove = Remover
email-remove-ask = Remover este endereço? Avisaremos o endereço de que ele foi removido. Depois disso, você não poderá mais receber um código de redefinição por e-mail.
email-remove-ask-no-mail = Remover este endereço? Enquanto este servidor não enviar e-mails, você não poderá definir um de novo.
email-remove-confirm = Remover endereço
email-remove-keep = Manter
email-pending = Enviamos um código para <code>{ $address }</code>. Ele vale por 15 minutos.
email-pending-keeps = Até você confirmá-lo, <code>{ $address }</code> continua sendo o seu endereço.
email-code = Código de confirmação
email-code-hint = Oito caracteres, como em XXXX-XXXX. Maiúsculas ou minúsculas, com ou sem o hífen.
email-confirm = Confirmar
email-resend = Enviar de novo
email-other = Usar outro endereço
email-withdraw = Retirar
email-error-code = Isso não é um código de confirmação. Ele tem oito caracteres, como em XXXX-XXXX.
email-wrong-code = Esse código não está certo. Confira e tente de novo. Depois de vários códigos errados, você precisa informar o endereço de novo.
email-code-dropped = Esse código também não estava certo, então o endereço não aguarda mais. Informe-o de novo para receber um novo código.
email-now-confirmed = Seu endereço de e-mail está confirmado.
email-resent = Um novo código está a caminho. O anterior não vale mais.
email-withdrawn = O endereço foi retirado. Nenhum código é mais aguardado.
email-removed = Seu endereço de e-mail foi removido.
email-busy = Muitos e-mails por enquanto. Tente de novo mais tarde.
email-same = Este já é o seu endereço.
email-unsure = Nenhuma resposta voltou, então não sabemos dizer se isso deu certo. Veja o que é mostrado aqui e tente de novo se não deu.
email-again = Para enviar o código, informe sua senha mais uma vez.

## Central's terms

terms-step-title = Novos termos de uso
terms-step-lead = Os termos do Baylee Central mudaram. Leia-os e aceite-os para usar sua conta; você também pode recusar e sair.
terms-accept = Aceitar
terms-decline = Recusar e sair

## The signed-in area

account-lead = Sua conta Baylee: perfil, segurança, privacidade e gateways.
account-nav = Sua conta
account-home = Visão geral
account-profile = Perfil
account-profile-lead = Seu identificador, seu nome de exibição e seu código de recuperação.
account-security = Segurança
account-security-lead = Onde você está conectado, e como sair.
account-privacy = Privacidade e dados
account-privacy-lead = Exporte seus dados ou exclua sua conta.
account-gateways = Gateways e tokens
account-gateways-lead = Gateways que você registrou e os tokens com que eles entram.
account-loading = Carregando …
account-signed-out = Você não está conectado.
account-unnamed = Sua conta
account-signin-name = Nome de entrada: { $name }
account-until = Conectado até { $time }
account-needs-title = Precisa da sua atenção
account-needs-code = Criar um código de recuperação
account-needs-name = Escolher seu nome de exibição
account-admin = Administração

reauth-lead = Isto exige uma entrada recente. Informe seu nome de entrada e sua senha mais uma vez.
reauth-submit = Entrar de novo

profile-who-title = Quem é você
profile-handle = Identificador
profile-signin-name = Nome de entrada
profile-source = Conta
profile-source-moved = Migrada do gateway oficial
profile-source-registered = Registrada neste site
profile-choose-title = Escolha seu nome de exibição
profile-choose-submit = Salvar nome de exibição
profile-name-taken = Esta conta já tem um nome de exibição.
profile-code-title = Código de recuperação
profile-code-kept = Sua conta tem um código de recuperação. Guarde-o bem; sem a sua senha, ele é o único caminho de volta aos seus dados criptografados.
profile-code-missing = Sua conta ainda não tem código de recuperação. Crie um; você precisará da sua senha de novo.
profile-code-make = Criar um código de recuperação

security-sessions = Onde você está conectado
security-lead = O Baylee Central mantém uma sessão para este site e uma para o cliente do jogo; ele não distingue dispositivos individuais.
security-kind-browser = Este site
security-kind-client = O cliente do Baylee
security-this = Esta aqui
security-signed-in = Conectado
security-ends = Termina
security-gateways = Em gateways
security-end-client = Desconectar o cliente
security-client-ended = O cliente foi desconectado.
security-sign-out = Sair aqui
security-password-title = Senha
security-password-lead = Seus dados criptografados e seu código de recuperação ficam como estão. Alterar a senha desconecta você de todos os lugares, menos daqui.

privacy-export-title = Exportar seus dados
privacy-export-lead = O que o Baylee Central sabe sobre a sua conta, como um arquivo JSON: os dados básicos, as sessões, os termos aceitos, seus gateways e as entradas de auditoria que citam a sua conta. Seus conteúdos criptografados só podem ser descriptografados e exportados pelo seu cliente do Baylee. A exportação exige uma entrada com menos de 30 minutos.
privacy-export-make = Preparar a exportação
privacy-export-save = Salvar a exportação
privacy-export-no-email = O endereço de e-mail não pôde ser lido; ele não está neste arquivo.
privacy-delete-title = Excluir sua conta
privacy-delete-lead = Sua conta, seus dados, suas sessões, seus contatos e os gateways que você possui são excluídos do sistema em funcionamento de imediato e em definitivo; eles saem dos backups em 12 meses, no máximo. O Central também exclui as mensagens que você enviou; o que outros já guardaram nos próprios dispositivos pode continuar lá.
privacy-delete-start = Excluir minha conta
privacy-delete-password = Sua senha, para confirmar
privacy-delete-prove = Continuar
privacy-delete-last = Último passo: isto não pode ser desfeito. Você tem um minuto.
privacy-delete-gateways = Estes gateways que você possui serão excluídos junto:
privacy-delete-confirm = Excluir a conta em definitivo
privacy-delete-cancel = Manter minha conta
privacy-deleted = Sua conta foi excluída.

gateways-none = Você não registrou nenhum gateway. A maioria dos jogadores nunca precisa de um.
gateways-add-title = Registrar um gateway
gateways-origin = Endereço do gateway
gateways-origin-hint = O endereço https do gateway, como https://play.example.org.
gateways-name = Nome do gateway
gateways-add = Registrar gateway
gateways-added = O gateway foi registrado e aguarda a aprovação de um administrador.
gateways-taken = Este endereço já está registrado.
gateways-invalid = Informe um endereço https sem caminho e um nome.
gateways-approved = Aprovado
gateways-pending = Aguardando aprovação
gateways-disabled = Desativado
tokens-title = Tokens
tokens-name = Nome do token
tokens-days = Válido por (dias)
tokens-sso = Faz os jogadores entrarem
tokens-listing = Lista o gateway
tokens-both = faz os jogadores entrarem e lista o gateway
tokens-make = Criar token
tokens-revoke = Revogar
tokens-revoked = Revogado
tokens-revoked-now = O token foi revogado.
tokens-until = Até { $time }
tokens-invalid = Dê um nome ao token e marque pelo menos uma coisa que ele permite.
tokens-shown-title = Seu novo token
tokens-kept = Eu o salvei
tokens-again = Você entrou de novo. Crie o token outra vez.

## The messenger

account-chat = Chat
account-contacts = Contatos
chat-side = Conversas e contatos
chat-tabs = Mensagens
chat-tab-chats = Chats
chat-tab-contacts = Contatos
chat-main = Conversa
chat-unread = não lidas
chat-conversations = Suas conversas
chat-search = Procurar uma conversa
chat-search-hint = Procurar (Ctrl+K)
chat-new-group = Novo grupo
chat-no-conversations = Ainda não há conversas. Adicione um contato e escreva para ele.
chat-no-match = Nenhuma conversa tem esse nome.
chat-former-group = Um antigo grupo
chat-former-lead = Você não está mais neste grupo.
chat-you = Você
chat-members = { $n ->
    [one] { $n } membro
   *[other] { $n } membros
}
chat-say-hello = Diga olá.
chat-history-only = Não é mais um contato.
chat-today = Hoje
chat-yesterday = Ontem
chat-encrypted = Criptografia de ponta a ponta
chat-opening-lead = Suas conversas e contatos. As mensagens são criptografadas nos seus dispositivos antes de saírem deles, e o Central as repassa sem poder lê-las.
chat-new-from = Nova mensagem de { $who }.
chat-new-many = { $n } novas mensagens.
chat-retry = Tentar de novo
chat-discard = Descartar
chat-not-sent = Não enviada.
chat-sending = Enviando …
chat-keys-changed = As chaves de alguém nesta conversa mudaram. Compare o número de segurança antes de escrever de novo.
chat-refused = Algumas mensagens não puderam ser verificadas e não são mostradas.
chat-failed = O chat não conseguiu falar com o Central agora.
chat-failed-open = Não foi possível abrir o chat. Recarregue a página para tentar de novo.
chat-empty = Ainda não há mensagens.
chat-older = Mostrar mensagens mais antigas
chat-compare-first = As chaves mudaram. Compare primeiro o número de segurança e depois escreva.
chat-compare = Comparar o número de segurança
chat-friend-required = Você poderá escrever quando voltarem a ser contatos.
chat-group-left = Você não está mais neste grupo.
chat-details = Detalhes
chat-close = Fechar
chat-back = Voltar às conversas
chat-messages = Mensagens
chat-fan = Baylee é um conteúdo de fã não oficial, não aprovado nem apoiado pela Wizards. <more>O que isso significa</more>
chat-safety = Números de segurança
chat-safety-lead = Compare o número com a outra pessoa, pessoalmente ou em outro canal em que você confie. Se for igual, ninguém se colocou entre vocês sem ser notado.
chat-changed = Alteradas
chat-confirm-keys = É igual: confiar nas novas chaves
chat-count = { $n } de 1.000 caracteres
chat-write = Escrever uma mensagem
chat-send = Enviar
chat-enter-hint = Enter envia, Shift+Enter inicia uma nova linha.
chat-choose = Escolha uma conversa
chat-choose-lead = Suas mensagens são criptografadas neste navegador; o Central apenas as repassa.
chat-storage = Sua chave de chat fica neste navegador enquanto você está conectado.
chat-opening = Abrindo seus chats …
chat-open = Abrir meus chats
chat-ended = Sua sessão terminou.
chat-unnamed = Escolha primeiro seu nome de exibição: ele dá a você o identificador para o qual os outros escrevem.
chat-missing-title = Entre de novo para abrir seus chats
chat-missing-lead = Este navegador não guarda a sua chave de chat. Sua senha a abre aqui; ela nunca sai deste navegador.
chat-invites = Convites para grupos
chat-invited-by = de { $who }
chat-group-name = Nome do grupo
chat-group-name-hint = Um nome de 1 a 48 caracteres.
chat-group-create = Criar
chat-group-failed = Não foi possível alterar o grupo agora.
chat-group-members = { $n ->
    [one] { $n } membro
   *[other] { $n } membros
}
chat-group-owner = Dono
chat-group-rename = Renomear
chat-group-transfer = Tornar dono
chat-group-remove = Remover
chat-group-transfer-ask = Tornar { $who } o dono deste grupo? Você continua como membro.
chat-group-remove-ask = Remover { $who } deste grupo?
chat-group-invite = Convidar um contato
chat-group-choose = Escolha um contato
chat-group-invite-button = Convidar
chat-group-invited = Convidado
chat-group-delete = Excluir grupo
chat-group-leave = Sair do grupo
chat-group-delete-ask = Excluir “{ $name }” para todos? As mensagens já recebidas continuam legíveis.
chat-group-leave-ask = Sair de “{ $name }”? Você só poderá voltar com um novo convite.

people-incoming = pedidos
people-failed = Não foi possível carregar seus contatos agora.
people-empty = Ainda não há contatos. Encontre alguém pelo identificador, ou dê o seu a ele.
people-your-handle = Seu identificador:
people-incoming-title = Pedidos para você
people-friends = Contatos
people-outgoing = Pedidos enviados
people-blocked = Bloqueados
people-find = Adicionar um contato pelo identificador
people-find-button = Buscar
people-handle-hint = Um identificador completo com a tag, como Nome#1a2b.
people-yourself = Esse é o seu próprio identificador.
people-nobody = Ninguém tem esse identificador.
people-request = Enviar um pedido de contato
people-requested = Pedido enviado.
people-is-friend = Um contato
people-is-incoming = Pediu para ser seu contato
people-is-outgoing = Pedido enviado
people-is-blocked = Bloqueado
people-write = Escrever
people-remove = Remover
people-block = Bloquear
people-accept = Aceitar
people-decline = Recusar
people-cancel = Retirar
people-unblock = Desbloquear
people-keep = Manter
people-confirm-remove = Remover { $who } dos seus contatos? Suas mensagens permanecem.
people-confirm-block = Bloquear { $who }? Essa pessoa não poderá mais pedir para ser seu contato nem escrever para você.

## Offline, updates, installing

title-offline = Offline
offline-title = Você está offline
offline-lead = Esta página precisa de conexão. O Central guarda as mensagens para você até você voltar.
offline-again = Tentar de novo
update-ready = Uma nova versão do baylee.app está pronta.
update-unsent = Você tem aqui um texto que ainda não foi enviado. Recarregar mesmo assim?
update-reload = Recarregar
install-app = Instalar como aplicativo

## Notifications

push-title = Notificações
push-lead = Escolha sobre o que o Baylee pode notificar você. As notificações não mostram texto de mensagens, nomes nem o conteúdo de relatórios.
push-device = Notificações neste dispositivo
push-categories = Notificar-me sobre
push-chat = Mensagens de chat
push-contacts = Pedidos de contato
push-updates = Novas versões
push-feedback = Relatórios de feedback (administradores)
push-gateways = Pedidos de gateways (administradores)
push-on = Ativadas neste dispositivo. Somente as categorias selecionadas são entregues.
push-off = Desativadas neste dispositivo. Suas escolhas de categorias são mantidas.
push-scope = As escolhas de categorias valem para a sua conta. A permissão do dispositivo é separada em cada navegador. Sair desconecta este dispositivo.
push-failed = Não foi possível confirmar a alteração. Tente de novo.
push-denied = As notificações estão bloqueadas. Você pode permiti-las nas configurações do navegador ou do dispositivo e tentar de novo.
push-unsupported = Este navegador não oferece Web Push aqui. No iPhone e no iPad, adicione o Baylee à Tela de Início e abra-o por lá.
push-unconfigured = As notificações em segundo plano ainda não estão disponíveis neste servidor. Suas preferências já podem ser salvas.

register-closed = O cadastro está pausado agora. Tente de novo mais tarde.
register-refused = O cadastro não foi aceito. Tente de novo.
admin-registration = Cadastro
admin-registration-open = Aberto: qualquer pessoa pode criar uma conta depois de resolver a prova de trabalho.
admin-registration-closed = Fechado: ninguém pode criar uma conta até você abrir de novo.
admin-registration-lead = Feche o cadastro durante um ataque e abra-o de novo à mão. Cada alteração é registrada no log de auditoria.
admin-registration-close = Fechar o cadastro
admin-registration-reopen = Abrir o cadastro de novo
hero-discord = Comunidade no Discord
hero-whatsapp = Comunidade no WhatsApp
hero-github = GitHub
nav-menu = Menu
foot-project = Projeto
foot-source = Código-fonte (AGPL-3.0)
foot-central = Baylee Central (código disponível)
foot-summary = Jogo de cartas de mesa gratuito e de código aberto, feito em Rust e Bevy — com um motor de regras determinístico, modo offline completo, IA da casa, ponte para LLMs e gateways da comunidade.
strip-ci = CI
strip-commit = Commit
strip-gateways = Gateways
strip-online = online
strip-offline = offline
strip-directory = o diretório
strip-cards = Cards jogáveis
strip-cards-unit = cards
strip-accounts = Contas no Baylee Central
strip-live = Agora nos gateways, como eles informam
strip-views = Páginas vistas desde { $date }
live-views = { $n ->
    [one] { $n } visualização
   *[other] { $n } visualizações
}
live-accounts = { $n ->
    [one] { $n } conta
   *[other] { $n } contas
}
live-players = { $n ->
    [one] { $n } jogador
   *[other] { $n } jogadores
}
live-playing = { $n ->
    [one] { $n } em partida
   *[other] { $n } em partida
}
live-tables = { $n ->
    [one] { $n } mesa aberta
   *[other] { $n } mesas abertas
}
live-games = { $n ->
    [one] { $n } partida
   *[other] { $n } partidas
}
hero-badges = Resumo
hero-badge-beta = Beta aberto
hero-badge-free = Gratuito
hero-badge-open = Código aberto
hero-badge-fan = Conteúdo de fã não oficial
gws-open-client = Abrir no Baylee
gws-open-client-help = “Abrir no Baylee” inicia o cliente instalado (0.1.1-beta.9b ou mais recente). Ele mostra o endereço e pergunta antes de adicionar o gateway.
dl-menu-title = Todos os sistemas
dl-old-cpu = processadores mais antigos (sem AVX2)
ci-passing = passando
ci-failing = falhando
dl-button = Baixar o Baylee
dl-quick = Outros sistemas e arquiteturas
dl-others = outros sistemas

# The start page, as the old site wrote it (owner, 11.11.2026)
link-readmeModel = #playing-against-a-language-model
about-engineTitle = Um motor que faz cumprir todas as regras
about-deterministic = Movido por um gerador aleatório com semente e sem nenhuma dependência do relógio nem da ordem de hash, cada partida é estritamente determinística e reproduzível jogada a jogada.
about-tableTitle = Uma mesa 3D limpa feita com Bevy
about-keys = Pelo lobby integrado você passa sem esforço entre o montador de decks local, as partidas solo e as mesas multijogador em gateways da comunidade.
feature-hiddenTitle = A informação oculta continua privada
feature-hidden = Seu cliente recebe apenas os dados que o seu lugar pode ver. Grimórios e mãos dos oponentes chegam estritamente como contagens—nem mesmo um cliente modificado consegue inspecionar cards ocultos.
feature-seatsTitle = Até oito jogadores em uma mesa
feature-seats = Duelos, todos contra todos e partidas em equipe (como 2v2) para até 8 lugares. Os companheiros de equipe dividem os turnos e podem mostrar uns aos outros os cards da mão.
feature-commanderTitle = Formatos flexíveis e regras da casa personalizadas
feature-commander = Monte decks para duelos clássicos de 60 cards, rodadas multijogador ou formatos da casa personalizados. Os pontos de vida iniciais, a formação das equipes e as regras de mulligan são definidos livremente pelo anfitrião.
feature-aiTitle = Replays determinísticos e relatórios F8
feature-ai = Cada partida é gravada jogada a jogada. Se você notar um problema em uma interação de regras durante o jogo, pressione F8 na própria mesa para enviar um relatório de estado reproduzível.
feature-processTitle = Um processo de servidor isolado por partida
feature-process = Cada jogo ativo roda em seu próprio processo de motor em sandbox no gateway. O próprio gateway gerencia apenas o lobby e as salas—um erro nunca afeta as outras mesas.
feature-languagesTitle = Texto dos cards em 19 idiomas
feature-languages = Sempre que um gateway oferece o catálogo multilíngue de cards, os cards são lidos no seu idioma preferido, com o inglês como alternativa. A interface do cliente está disponível em alemão e inglês.
feature-watchTitle = Assista a partidas ao vivo
feature-watch = O botão Assistir do lobby coloca você na mesa como espectador ao vivo: acompanhe o campo de batalha público em tempo real enquanto todas as mãos dos jogadores permanecem ocultas.
feature-connectionTitle = Reconexão contínua em caso de queda
feature-connection = Se a conexão cair por pouco tempo, a mesa guarda o lugar com uma contagem regressiva e pode pausar a partida ou deixar a IA da casa assumir temporariamente.
feature-tablesTitle = Catorze relógios de decisão
feature-tables = De turnos ágeis de 15 segundos a mesas tranquilas sem limite de tempo, configure o ritmo individualmente para cada sala.
shots-eyebrow = Na mesa
shots-title = Dentro do jogo
shots-lead = Pequenas gravações ao vivo feitas diretamente no cliente 3D, com imagens de cards do Scryfall (© Wizards of the Coast), de duelos e rodadas de 4 jogadores até o lobby criptografado e o montador de decks.
shots-credit = Imagens de cards: Scryfall · © Wizards of the Coast
shots-showLarge = Mostrar em tamanho grande: { $title }
shots-dialog = Captura de tela
shots-previous = Anterior
shots-next = Próxima
shots-close = Fechar
ai-eyebrow = Oponentes de IA e modelos de linguagem
ai-title = IA da casa, ponte para LLMs e IA nativa do Baylee
ai-houseTitle = IA da casa heurística em cinco níveis de habilidade
ai-house = Pronta para jogar offline e em qualquer mesa online: escolha entre Novato, Casual, Constante, Afiado e Especialista. A IA da casa decide localmente, sem tempo de espera, e vê somente o mesmo estado público que um jogador humano.
ai-modelTitle = Modelos de linguagem na mesa via Seat-Bridge
ai-model = Coloque o <b>Google Gemini, o OpenAI ChatGPT, o Anthropic Claude, o DeepSeek</b> ou modelos locais via <b>LM Studio</b> na sua própria mesa. As chaves de API ficam protegidas em variáveis de ambiente com limites de gasto; em caso de tempo esgotado, a IA da casa assume sem atrito.
ai-setup = Guia de configuração de LLM e Seat-Bridge
home-gwLead = As mesas online ficam em gateways, servidores que o Baylee Central lista depois que o administrador deles os aprova. Aqui estão os primeiros; o diretório tem todos.
beta-open = Abrir o diretório
beta-own = Executar o seu próprio
source-eyebrow = Código-fonte
source-title = Código aberto, e continua assim
source-promise = O Baylee será sempre de código aberto e gratuito. Sem acesso pago, sem recursos pagos, sem anúncios. Nem agora, nem depois. O código está sob a GNU Affero General Public License, o que significa que quem executa um gateway modificado para outras pessoas também deve oferecer a elas o código-fonte dele.
source-rust = Ele é escrito em Rust, o projeto de uma só pessoa com muita ajuda de agentes de programação, e toda alteração passa pelo mesmo portão: formatação, lints, a suíte de testes e uma validação de cada card contra o seu texto impresso. Issues e pull requests são bem-vindos; <agents>AGENTS.md</agents> e <claude>CLAUDE.md</claude> são as regras de trabalho tanto para pessoas quanto para agentes.
source-discord = Comunidade no Discord
source-whatsapp = Comunidade no WhatsApp
source-architecture = Arquitetura
source-repoTitle = O repositório agora
source-stars = Estrelas
source-issues = Issues e pull requests abertos
source-note = Lido da API pública do GitHub pelo servidor deste site a cada dez minutos. Seu navegador não pergunta nada ao GitHub.
gw-count =
    { $n ->
        [one] { $n } gateway listado, { $online } online.
       *[other] { $n } gateways listados, { $online } online.
    }
gw-none = Nenhum gateway está listado ainda.
contact-intro = Uma pergunta, uma ideia, um jogo que você gostaria de ver, ou algo que você prefere não postar em público: este formulário vai direto para a pessoa por trás do Baylee. Deixe um endereço de e-mail se quiser uma resposta; sem ele, só podemos ler.
contact-f8 = Um card que foi resolvido errado é melhor informado pelo jogo com <kbd>F8</kbd>, porque o relatório pode levar a partida inteira.
contact-discord = conversar, achar oponentes, saber das builds
contact-issues = Issues do GitHub
contact-issuesSub = erros e ideias, em público
contact-operator = Quem opera este site, com endereço postal: o <impressum>aviso legal</impressum>.
contact-whatsappSub = Grupo da comunidade e partidas rápidas
contact-reportsTitle = Relatórios e privacidade
contact-reportsSub = Detalhes sobre relatórios F8 e dados
ai-housePill = Offline e online
ai-providers = Modelos de linguagem compatíveis
ai-hostsPill = Para quem hospeda gateways
ai-hostsTitle = Ofereça lugares de LLM hospedados no seu gateway
ai-hosts = Como operador de gateway, você pode oferecer modelos de linguagem pré-configurados diretamente no seu servidor. Os convidados de uma mesa simplesmente escolhem um dos seus modelos hospedados como oponente—sem precisar de chave de API.
ai-trainingPill = Em treinamento · IA nativa do motor
ai-trainingTitle = IA nativa do Baylee — treinada para jogar diretamente no motor como um jogador de verdade
ai-training = Ao lado da IA da casa e da ponte externa para LLMs, uma IA nativa do Baylee dedicada está sendo treinada diretamente no motor de regras determinístico. Ela aprende a avaliar estados do tabuleiro, janelas de prioridade, contas de combate e sinergias de deck como um jogador humano experiente—sem latência de API externa.
ai-hostingLink = Hospedagem de gateway e configuração de LLM

# The gateways and the contact page, as the old site wrote them (owner, 11.11.2026)
gw-lead = Um gateway é um servidor onde ficam as mesas online: contas, decks, salas e um processo do motor por partida. Estes são os registrados no Baylee Central e aprovados pelo administrador dele. Cada um é executado por seus operadores para os grupos deles, e valem as regras de cada um. Estar na lista não é uma promessa sobre o que um gateway faz.
gw-howTo = Para jogar em um, copie o endereço dele para a porta de entrada do cliente. Online significa que o Central ouviu o gateway no último minuto e meio; não é promessa de que ele responda. Os nomes e as versões são o que cada operador informa.
gw-find = Encontrar um gateway
gw-findHint = Nome ou endereço
gw-shown = { $n } de { $all } exibidos
gw-noMatch = Nenhum gateway carregado até agora corresponde.
gw-showAll = Mostrar todos
gw-stats = O diretório em números
gw-stat-listed = Gateways listados
gw-stat-online = Online agora
gw-stat-players = Jogadores online
gw-stat-playing = Deles, em partida
gw-stat-tables = Mesas abertas
gw-stat-games = Partidas em andamento
gw-show = Mostrar
gw-show-every = Todos
gw-show-online = Online
gw-show-tables = Com mesas abertas
gw-sort = Ordenar por
gw-sort-best = Recomendados
gw-sort-name = Nome (A–Z)
gw-sort-players = Mais jogadores
gw-sort-tables = Mais mesas abertas
gw-sort-version = Versão mais recente
gw-view = Exibição
gw-view-cards = Mosaico
gw-view-list = Lista
link-readmeGateway = #running-your-own-gateway
selfhost-eyebrow = Auto-hospedagem e integração com o Central
selfhost-title = Execute o seu próprio gateway
selfhost-processes = Um gateway auto-hospedado executa três processos leves ao lado do PostgreSQL 18+: o <code>baylee-gateway</code> gerencia salas, decks e roteamento; o <code>baylee-agent</code> inicia um processo <code>baylee-engine-server</code> isolado por partida, de modo que cada mesa fica em sandbox.
selfhost-proxy = Coloque um proxy reverso TLS na frente da sua origem HTTPS (p. ex. <code>https://play.example.org</code>), importe opcionalmente o catálogo em massa do Scryfall para textos de cards em 19 idiomas, e os jogadores podem se conectar diretamente pelo seu endereço no cliente.
selfhost-central = <b>Vincule ao Baylee Central e configure a privacidade:</b> Registre a sua origem HTTPS em <mine>Seus gateways</mine> e gere um token <code>bgt_…</code> (escopos: <code>sso</code> para login único, <code>listing</code> para heartbeats do diretório e telemetria ao vivo). No console do seu gateway, escolha o nível de privacidade da telemetria (<code>full</code> para jogos em andamento, configurações de mesa e jogadores que optaram por participar, <code>counts_only</code> somente para totais ou <code>private</code> somente para disponibilidade).
selfhost-guide = <guide>Leia o guia completo de auto-hospedagem</guide> · <protocol>Referência de protocolo e heartbeat (docs/protocol.md)</protocol>
selfhost-source = Todo gateway responde a <code>GET /source</code> com a sua licença, versão e commit sem exigir login, cumprindo automaticamente a exigência de divulgação do código-fonte da AGPL.
reports-eyebrow = Relatórios
reports-title = Encontrou algo errado? Conte para nós
reports-f8 = Pressione <kbd>F8</kbd> na mesa ou no lobby. Um relatório leva o seu tipo, o seu texto, a versão do cliente e, em uma mesa, o ID do jogo. Tudo o que vai além tem uma caixa de seleção própria, e toda caixa começa marcada, então desmarque o que preferir guardar antes de pressionar Enviar: sistema e hardware, a mesa como você a vê, o seu log (outros jogadores renomeados para "Jogador A", "Jogador B", …), suas configurações, uma imagem somente da janela do Baylee e, em uma mesa, o registro completo do jogo (cada jogada e os cards de cada lugar, sem nomes). <i>Show what is sent</i> [Mostrar o que é enviado] mostra cada byte. Tokens de sessão nunca são enviados.
reports-usage = Enquanto você joga offline, o cliente avisa uma vez por minuto que um jogo está em andamento: sem conta, sem ID de dispositivo, nada sobre o jogo. <i>Send anonymous usage count</i> [Enviar contagem de uso anônima], em Settings › Privacy & Data, desativa isso.
reports-discord = Quer conversar, achar oponentes ou saber primeiro das novas builds? Entre na <discord>comunidade do Baylee no Discord</discord>.
reports-issues = Dúvidas de regras, comportamento errado de cards, ideias: <issues>abra uma issue no GitHub</issues>. Um card que foi resolvido errado é o relatório mais útil que podemos receber, porque todo card tem um teste e o seu se torna um.
reports-privacy = <docs>docs/privacy.md</docs> lista tudo o que o cliente, o gateway e o serviço de feedback guardam. Este site não define cookie a menos que você entre, não executa análise de uso e não carrega fontes nem scripts do servidor de ninguém mais. <notice>O aviso de privacidade</notice> diz o que o site e o Baylee Central guardam, e nomeia todos os outros hosts com que eles se comunicam, e quando.

# The download page, as the old site set it (owner, 11.11.2026)
dlPage-eyebrow = Obtenha
dlPage-filesEyebrow = Todos os arquivos
dlPage-filesTitle = Todos os sistemas
install-eyebrow = Instalação
install-releases = Todas as versões no GitHub
release-notes = Notas da versão e somas de verificação
release-all = Todas as versões

# The download page's head, as the old site wrote it
dlPage-title = Baixar
dlPage-lead = Cada download vai direto ao arquivo no GitHub, sem nada no meio. Onde uma versão tem instaladores, eles são o jeito mais fácil; os arquivos compactados trazem o mesmo jogo para descompactar em qualquer lugar. Um Baylee instalado se atualiza sozinho a partir dos arquivos assinados, onde quer que possa escrever a própria pasta. Todo pacote traz a ponte de lugares para modelos de linguagem ao lado do jogo.

# The reading pages' eyebrows, as the old site set them
imp-eyebrow = Aviso legal
terms-eyebrow = Termos de uso
privacy-eyebrow = Privacidade

# The privacy notice's contents, as the old site named them
privacy-contents = Conteúdo

# The account pages' eyebrow, as the old site set it
acct-eyebrow = Baylee Central

# The account area's navigation, as the old site named it
app-account = Sua conta

# ==== The security page (/security) ====================================
# Every sentence and every arrow of this page maps to a row of the claim
# table in docs/security-page.md; a claim changes
# there first. No "safe", "unbreakable" or compliance words: mechanisms only.
title-security = Segurança no Baylee: senhas, chaves e chats
desc-security = Como o Baylee trata senhas, chaves e mensagens de chat, passo a passo em quatro diagramas, com os limites de cada um.
nav-security = Segurança
sec-eyebrow = Segurança
sec-title = Senhas, chaves e chats: o que vai para onde
sec-lead = Quatro fluxos, passo a passo: o que sai do seu dispositivo, o que o Baylee Central vê e o que ele não vê. Com os limites de cada um.
sec-toc-label = Nesta página
sec-figure = Diagrama em passos: { $name }
sec-play = Reproduzir
sec-all = Todos os passos
sec-step = Passo { $n }
sec-hint = O diagrama avança sozinho. Escolha um passo para parar nele.
sec-hint-still = Escolha um passo para destacá-lo.
sec-in-browser = A moldura tracejada é o seu navegador: tudo isto acontece dentro dele.
# The actors' names are the only words inside the drawings, where text does
# not wrap: one or two words each, at most about twice the English width
# (the test `names_fit_with_room_to_grow` measures every language).
sec-actor-device = Seu dispositivo
sec-actor-central = Central
sec-actor-page = Página
sec-actor-worker = Worker
sec-actor-store = Armazém
sec-actor-you = Você
sec-actor-contact = Contato
sec-actor-website = Site
sec-actor-client = Cliente

sec-signin-eyebrow = Entrar e se cadastrar
sec-signin-title = O Central confere a sua senha sem recebê-la
sec-signin-lead = O Baylee faz a sua entrada com OPAQUE (RFC 9807). Seu dispositivo e o Central trocam três mensagens; sua senha não está em nenhuma delas, nem mesmo como um hash simples.
sec-signin-c1 = Pedido cegado
sec-signin-t1 = Seu dispositivo cega a senha.
sec-signin-s1 = Ele combina a senha com um número aleatório novo e envia somente o resultado. A senha não pode ser lida a partir dessa mensagem.
sec-signin-c2 = Resposta do Central
sec-signin-t2 = O Central responde com uma chave própria.
sec-signin-s2 = O Central calcula sobre o valor cegado com uma chave que só ele tem e acrescenta a sua metade de uma troca de chaves. Ao fazer isso, ele não fica sabendo a senha.
sec-signin-c3 = Argon2id · 64 MiB
sec-signin-t3 = Seu dispositivo faz o trabalho pesado.
sec-signin-s3 = Ele remove o cegamento e alonga o resultado com Argon2id (64 MiB de memória, três passagens); o Central não pode pedir configurações mais baratas. Uma senha errada é percebida aqui, antes de qualquer outra coisa ser enviada.
sec-signin-c4 = Prova
sec-signin-t4 = Seu dispositivo prova que tem as chaves certas.
sec-signin-s4 = Com a senha certa, seu dispositivo chega à mesma chave de sessão que o Central e envia uma prova curta disso (64 bytes).
sec-signin-c5 = Sessão + envoltório
sec-signin-t5 = O Central abre a sessão.
sec-signin-s5 = De volta vêm a sessão (no navegador, um cookie que a própria página não consegue ler) e a sua chave mestra em um envoltório que só abre no seu dispositivo, com a senha certa.
sec-signin-register = O <b>cadastro</b> usa o mesmo protocolo. Seu dispositivo sorteia uma chave mestra aleatória (256 bits), envolve-a uma vez com a sua senha e uma vez com o seu código de recuperação, e envia somente os envoltórios. O Central armazena um registro OPAQUE e os envoltórios, nunca a chave.
sec-signin-legacy = <b>Uma exceção:</b> contas do antigo gateway oficial que ainda não passaram para o Central. Até uma dessas contas ser transferida, cada tentativa de entrada no cliente Baylee envia a senha digitada, por uma conexão TLS verificada, ao Central, que a confere com o hash antigo e não a armazena nem a registra. Depois que a transferência der certo, a conta entra como mostrado acima. Este site não faz a entrada dessas contas; ele aponta para o cliente. A exceção termina com o prazo definido para a transferência.
sec-signin-limit = <b>O que permanece:</b> quem levar o banco de dados do Central junto com as suas chaves pode tentar senhas uma a uma; cada tentativa custa uma execução do Argon2id. Uma senha descoberta dessa forma abre os dados criptografados da conta e os seus chats armazenados. Uma senha longa que você não usa em nenhum outro lugar continua importando.
sec-signin-guess = <b>Adivinhar senhas pela entrada</b> é retardado, não impedido. O Central conta as tentativas que não são concluídas com a senha correta: no máximo 100 por dia em uma conta, a partir de endereços de onde ela não entrou, e no máximo 100 por dia a partir de um endereço em todas as contas juntas. Passado esse limite, ele responde que houve tentativas demais, até a hora ou o dia terminar. Uma senha comum ainda é encontrada assim. E quem sabe o seu nome de entrada pode esgotar as tentativas da sua conta: até o dia terminar, você só entra a partir de um endereço de onde a sua conta entrou nos últimos 30 dias (o Central lembra esses endereços apenas até ser reiniciado).

sec-memory-eyebrow = Na memória
sec-memory-title = O que ainda está na memória depois que você entra
sec-memory-lead = A senha é necessária uma vez: para abrir a sua chave mestra. O diagrama a acompanha pelo navegador; o que o cliente Baylee faz de diferente está abaixo.
sec-memory-c1 = Senha
sec-memory-t1 = A página entrega a senha a um Worker.
sec-memory-s1 = O formulário a passa a uma thread de segundo plano própria (um Worker), esvazia o campo de entrada e libera a sua cópia.
sec-memory-c2 = OPAQUE + Argon2id
sec-memory-t2 = O Worker calcula e depois sobrescreve.
sec-memory-s2 = O código Rust dele guarda a senha só até o passo OPAQUE terminar e então a sobrescreve, junto com as chaves que derivou no caminho e que ele mesmo guarda, e com os 64 MiB de memória de trabalho do Argon2id.
sec-memory-c3 = Chave mestra: conferida, apagada
sec-memory-t3 = A chave mestra é aberta, usada e apagada.
sec-memory-s3 = O Worker a abre para conferir a entrada, deriva dela as chaves de chat se a sua conta tem um chat, e a sobrescreve de novo.
sec-memory-c4 = Chaves de chat, seladas
sec-memory-t4 = Só as chaves de chat ficam.
sec-memory-s4 = Para que o chat abra depois de um recarregamento sem a sua senha, o Worker as guarda no navegador (IndexedDB), criptografadas com AES-256-GCM. A chave usada para isso não é extraível: scripts podem usá-la, mas não conseguem lê-la. Enquanto uma página de chat está aberta, um Worker próprio mantém as chaves de chat sem selo na memória. Sair apaga o registro; depois que uma sessão termina por conta própria, a próxima página deste site que carregar o apaga.
sec-memory-c5 = Worker encerrado
sec-memory-t5 = O Worker da entrada é encerrado.
sec-memory-s5 = Quando a entrada termina, a página de entrada encerra o seu Worker, e o navegador descarta a memória desse Worker.
sec-memory-limit = <b>O que um navegador não promete:</b> a senha como digitada, a sua entrega ao Worker e as strings do JavaScript são gerenciadas pelo navegador. O Baylee as libera; ele não sobrescreve as cópias da própria página e não consegue sobrescrever as do navegador. Quando essa memória é de fato reutilizada é decisão do navegador. E quem puder executar script neste site ou ler o perfil do seu navegador enquanto você está conectado consegue chegar às chaves de chat seladas.
sec-memory-client = <b>No cliente Baylee</b>, a senha digitada fica em uma memória que é sobrescrita quando liberada, e é descartada logo depois da entrada; ela nunca é armazenada. A chave mestra permanece na memória do cliente enquanto você está conectado, porque o cliente abre com ela os seus dados criptografados.

sec-chat-eyebrow = Chat
sec-chat-title = As mensagens são criptografadas no seu dispositivo
sec-chat-lead = Cada mensagem recebe uma chave própria. O Central armazena e repassa somente o que está criptografado; lê-lo exige a chave de chat de uma conta para a qual ele foi envolvido.
sec-chat-c1 = Texto + chave única
sec-chat-t1 = Uma chave nova para cada mensagem.
sec-chat-s1 = Seu dispositivo sorteia uma chave de mensagem aleatória e criptografa o texto com ela (XChaCha20-Poly1305).
sec-chat-c2 = Envelope por chave, assinado
sec-chat-t2 = Envolvida para a chave de chat de cada destinatário, assinada com a sua.
sec-chat-s2 = Ele envolve a chave da mensagem separadamente para cada chave de chat que a conta do destinatário registrou (X25519) e assina o cabeçalho e o texto cifrado com a sua própria chave de chat (Ed25519). Hoje uma conta tem uma só chave de chat, derivada da sua chave mestra e igual em todos os navegadores e clientes em que ela está conectada.
sec-chat-c3 = Texto cifrado + envelopes
sec-chat-t3 = O Central recebe somente texto cifrado.
sec-chat-s3 = O Central confere a assinatura e a lista de destinatários e armazena o texto cifrado com os envelopes. Ele não tem as chaves das mensagens nem as chaves da sua conta.
sec-chat-c4 = Texto cifrado + um envelope
sec-chat-t4 = Cada chave de chat recebe o seu envelope.
sec-chat-s4 = Quando o seu contato busca as mensagens, o Central entrega o texto cifrado e somente o envelope feito para aquela chave de chat.
sec-chat-c5 = Conferida, aberta
sec-chat-t5 = Conferir, desembrulhar, ler.
sec-chat-s5 = O dispositivo do seu contato confere a sua assinatura, desembrulha a chave da mensagem com a sua chave de chat e descriptografa o texto.
sec-chat-meta = <b>O que o Central vê mesmo assim:</b> quem está em contato com quem, grupos com seus nomes e membros, nomes de exibição, as chaves públicas de chat e, de cada mensagem, quem a enviou, em qual conversa, quando e de que tamanho. Como qualquer servidor, ele também vê o endereço de onde vem uma requisição.
sec-chat-limit = <b>Limites:</b> não há double ratchet e, por isso, não há sigilo futuro: se a chave de chat de um destinatário for roubada depois, ela também abre as mensagens anteriores que foram envolvidas para ela. Sua chave de chat é derivada da chave mestra da sua conta, e o Central guarda as mensagens criptografadas depois da entrega: quem levar o banco de dados do Central com as suas chaves e adivinhar a sua senha, ou tiver o seu código de recuperação, pode lê-las. No primeiro contato, seu dispositivo confia nas chaves que o Central indica; para conferi-las, compare o número de segurança por outro canal. Se as chaves de um contato mudarem depois, o Baylee só volta a enviar depois que você as confirmar. Quem pertence a um grupo é decidido pelo Central.

sec-pair-eyebrow = Entrar por link
sec-pair-title = Do navegador para o cliente sem digitar a senha de novo
sec-pair-lead = Quem entrou pelo site deve poder iniciar o cliente Baylee já conectado. Para isso a página leva a chave mestra até o cliente: selada, e de um modo em que o Central não consegue nem abri-la nem colocar uma própria no lugar.
sec-pair-status = <b>Em que pé isto está:</b> o Central e a biblioteca de cliente do Baylee já fazem isto hoje e foram testados um contra o outro. Ainda não pode ser usado a partir deste site nem de um cliente lançado.
sec-pair-c1 = Iniciar o pareamento
sec-pair-t1 = A página inicia um pareamento.
sec-pair-s1 = O Central emite para ele uma referência válida por cinco minutos. Ao mesmo tempo, a página sorteia um segredo de uso único que nunca mostra ao Central.
sec-pair-c2 = Link com o segredo
sec-pair-t2 = Um link abre o cliente.
sec-pair-s2 = O link <code>baylee://pair</code> leva a referência e o segredo da página até o cliente, no seu próprio dispositivo, sem passar pelo Central.
sec-pair-c3 = Chave pública
sec-pair-t3 = O cliente responde com um novo par de chaves.
sec-pair-s3 = Ele envia ao Central a metade pública e um valor de verificação calculado com o segredo (HMAC-SHA-256). A página confere esse valor; as duas telas mostram o mesmo código curto, e você aprova na página.
sec-pair-c4 = Chave mestra, selada
sec-pair-t4 = A página sela a chave mestra para o cliente.
sec-pair-s4 = Com HPKE (RFC 9180), para a chave pública do cliente e também sob o segredo. O Central guarda o selo uma vez; ele não consegue abri-lo e, sem conhecer o segredo, não consegue fazer um próprio.
sec-pair-c5 = Sessão + selo
sec-pair-t5 = O cliente resgata uma vez.
sec-pair-s5 = Ele recebe uma sessão junto com o selo, abre o selo com a sua chave privada e o segredo, e fica conectado. O pareamento é apagado depois, também quando o resgate falha.
sec-pair-note = <b>A senha não participa desta troca.</b> A página só tem a chave mestra no momento em que acabou de abri-la com a sua senha. Uma sessão pareada assim conta como não mais recente do que a entrada com senha na página de onde veio: o pareamento nunca a renova.

sec-limits-eyebrow = Limites
sec-limits-title = O que nada disto consegue fazer
sec-limit-device = <b>Um dispositivo comprometido.</b> Um malware, uma extensão do navegador que lê páginas ou um cliente adulterado vê o que você vê e digita. Nenhuma criptografia no caminho até um servidor ajuda contra isso.
sec-limit-site = <b>O código de um site vem do seu servidor.</b> Quem tomar o nosso servidor poderia entregar um código que captura a sua senha enquanto você a digita; hashes de integridade e uma Content Security Policy rígida protegem contra alterações vindas de fora, não contra isso. O cliente é um programa na sua máquina e confere uma assinatura antes de instalar uma atualização.
sec-limit-central = <b>Um Central que foi tomado.</b> Ele poderia revelar metadados, recusar o serviço e atacar entradas. Poderia indicar chaves falsas num primeiro contato ou acrescentar um membro a um grupo e, assim, ler as mensagens enviadas depois disso. O que ele guarda dos seus dados e chats está criptografado; abri-lo exige a sua senha ou o seu código de recuperação.
sec-limit-recovery = <b>Chaves perdidas.</b> Sem a sua senha e sem o seu código de recuperação, ninguém pode trazer de volta os seus dados criptografados, nem nós: o Central só tem os envoltórios.
sec-limit-audit = <b>Nenhuma auditoria independente.</b> A entrada segue o OPAQUE (RFC 9807) com Argon2id (RFC 9106), e o pareamento sela com HPKE (RFC 9180): especificações publicadas do IRTF, não padrões do IETF. A ordem dos passos do pareamento e o formato do chat são combinações próprias do Baylee de blocos de construção padrão (Ed25519, X25519, XChaCha20-Poly1305). Até agora, somente as pessoas que a construíram revisaram a implementação. O código é aberto; se você encontrar algo, <contact>avise-nos</contact>.
sec-docs-title = Para ler mais
sec-docs = Os detalhes estão nos documentos do projeto: <protocol>protocolo de criptografia</protocol>, <chat>chat</chat>, <accounts>contas e pareamento</accounts> e <threats>modelo de ameaças</threats>. O que acontece com os seus dados em termos jurídicos está no <privacy>aviso de privacidade</privacy>.

# The page's two links from elsewhere: the FAQ and the sign-in page.
faq-security-q = Como o Baylee trata a minha senha e os meus chats?
faq-security-a = Quando você entra, seu dispositivo usa o protocolo OPAQUE para provar que sabe a senha sem enviá-la; as mensagens de chat são criptografadas no seu dispositivo. A <security>página de segurança</security> mostra cada passo em um diagrama, com a única exceção e os limites.
sec-signin-link = O que acontece com a sua senha quando você entra: <security>Segurança</security>
sec-teaser-lead = Passo a passo: o que sai do seu dispositivo, o que o Baylee Central vê e o que ele não vê, e os limites de cada fluxo.
sec-teaser-more = Veja cada passo em um diagrama
# ==== end of the security page =================================================
# ==== registration: the proof of work, while it takes long =====================
register-solving = O seu navegador está resolvendo um quebra-cabeça que mantém os robôs de fora. Quando muitas contas estão sendo criadas, isso pode levar alguns minutos … { $percent } %

## An administrator makes a reset code (/account/admin)
admin-reset-title = Redefinir uma entrada
admin-reset-lead = Encontre a conta da pessoa que pediu a você e gere um código de redefinição para ela. Com o código, ela define uma nova senha na página de redefinição.
admin-find = Encontrar a conta
admin-only = Esta página é só para administradores.
admin-search = Buscar contas
admin-search-hint = Parte de um nome de entrada ou de um nome de exibição, ou # e uma tag. Deixe vazio para listar todas as contas, das mais novas às mais antigas.
admin-search-submit = Buscar
admin-search-error = Uma busca tem de 1 a 64 caracteres simples, sem acentos, ou # e uma tag como #1a2b.
admin-count = Contas exibidas: { $count }
admin-none = Nenhuma conta corresponde a esta busca.
admin-more = Mostrar mais
admin-open = Abrir
admin-back = Voltar
admin-pill = Administrador
admin-state = Estado
admin-state-pending = Reservada, nunca configurada
admin-state-enrolled = Configurada
admin-state-restored = Em uso
admin-created = Criada em
admin-last-sign-in = Última entrada
admin-recovery = Código de recuperação
admin-yes = Sim
admin-no = Não
admin-reset-outstanding = Um código de redefinição não usado vale até { $time }. Se um administrador o gerou ou se ele foi por e-mail, o Central não informa. Um novo código o substitui.
admin-reset = Redefinir a entrada
admin-refused-admin = Esta conta tem a função de administrador, e o Central não gera código de redefinição para a conta de um administrador. Remova a função primeiro.
admin-refused-own = Esta é a sua própria conta, e ninguém gera um código de redefinição para a própria conta. Altere sua senha em Segurança.
admin-refused-pending = Esta conta foi reservada e nunca configurada, então não tem entrada para redefinir.
admin-refused-moving = A mudança desta conta a partir do gateway oficial não terminou. Até terminar, a entrada dela é redefinida lá, não aqui.
admin-refused-stale = O Baylee Central considera sua entrada mais antiga que 30 minutos. Tente de novo; você informa sua senha mais uma vez.
admin-refused-role = Sua conta não tem mais a função de administrador.
admin-refused-gone = Esta conta não existe mais.
admin-switched = Este navegador agora está conectado como outra conta, então nenhum código foi pedido. Carregue a página de novo.
admin-lost = Nenhuma resposta voltou, então não sabemos dizer se um código foi gerado. Se foi, ninguém pode mostrá-lo de novo, e ele já substituiu qualquer código anterior. Gere um novo: ele substitui aquele por sua vez.
admin-warning-title = Antes de gerar um código
admin-warning-who = Confira fora deste site que a pessoa que pede realmente tem esta conta, por exemplo por uma ligação ou por um canal em que você já a conheça. Quem tiver o código pode definir uma nova senha.
admin-warning-ends = Gerar um código não encerra nada: a senha antiga continua funcionando até o código ser usado, e todos os dispositivos continuam conectados.
admin-warning-data = Os dados criptografados da pessoa sobrevivem à redefinição só com o código de recuperação dela ou em um dispositivo que ainda esteja conectado. Sem nenhum dos dois, ela recomeça com um cofre vazio.
admin-warning-replaces = Um novo código substitui qualquer anterior, inclusive um enviado por e-mail. Vale por 24 horas e funciona uma única vez.
admin-warning-takeover = Um código de redefinição responde a uma senha esquecida, não a uma conta tomada: as sessões de um invasor e a senha antiga ficam até o código ser usado, e para sempre se ele nunca for.
admin-warning-continue = Conferi quem está pedindo
admin-password-hint = Gerar um código exige uma entrada com menos de 30 minutos, então você informa sua senha mais uma vez. Ela fica neste navegador.
admin-password-missing = Informe sua senha.
admin-make = Gerar o código de redefinição
admin-code-title = O código de redefinição
admin-code-until = Vale até { $time }, e uma única vez.
admin-code-lead = Entregue-o à pessoa que pediu, por um canal em que você confia. Ele é mostrado só agora: nem o Baylee Central nem esta página podem mostrá-lo de novo. Se ele se perder, gere um novo.
admin-code-done = Concluído

# English for now: to be translated.

## The feedback moderation (/account/admin/feedback)

fb-title = Feedback and crash reports
fb-report-title = Report
fb-to-overview = Administration overview
fb-reports = Reports
fb-refresh = Refresh
fb-views = Views
fb-view-inbox = Inbox
fb-view-open = Open
fb-view-mine = Mine
fb-view-unassigned = Unassigned
fb-view-done = Done
fb-view-all = All
fb-view-trash = Trash
fb-search = Search reports
fb-search-hint = Words of the text or of a crash's message, or part of a report's id.
fb-search-submit = Search
fb-filters = Filters
fb-filters-n = Filters ({ $n })
fb-status = Status
fb-kind = Kind
fb-priority = Priority
fb-assignee = Assignee
fb-label = Label
fb-labels = Labels
fb-gateway = Gateway
fb-channel = Channel
fb-record = Game record
fb-reporter = Reporter
fb-from = From
fb-to = To
fb-sort = Order
fb-only-unread = Only reports I have not opened
fb-any = Any
fb-anyone = Anyone
fb-nobody = Nobody
fb-me = Me
fb-with-record = With a record
fb-without-record = Without a record
fb-channel-gateway = Through a gateway
fb-channel-direct = Direct, through no gateway
fb-ascending = { $sort }, ascending
fb-descending = { $sort }, descending
fb-sort-received = Received
fb-sort-changed = Last change
fb-sort-priority = Priority
fb-sort-status = Status
fb-sort-kind = Kind
fb-sort-reporter = Reporter
fb-sort-gateway = Gateway
fb-kind-bug = Bug
fb-kind-improvement = Improvement
fb-kind-feedback = Feedback
fb-kind-crash = Crash
fb-kind-other = Other
fb-status-new = New
fb-status-triaged = Triaged
fb-status-in-progress = In progress
fb-status-resolved = Resolved
fb-status-wont-fix = Won't fix
fb-status-duplicate = Duplicate
fb-prio-critical = Critical
fb-prio-high = High
fb-prio-normal = Normal
fb-prio-low = Low
fb-prio-none = No priority
fb-active-filters = Active filters
fb-remove-filter = Remove the filter { $what }
fb-clear-filters = Clear the filters
fb-crash-group = Crash group { $id }
fb-duplicates-of = Duplicates of …{ $id }
fb-search-pill = Search: { $text }
fb-saved-views = Saved lists
fb-view-name = Name of this list
fb-save-view = Save this list
fb-no-views = No saved lists yet.
fb-remove-view = Remove the saved list { $name }
fb-views-full = You have saved { $n } lists, the most there can be. Remove one to save another.
fb-view-saved = Saved the list "{ $name }".
fb-view-removed = Removed the list "{ $name }".
fb-err-views = The saved lists could not be read.
fb-chart = The last 30 days
fb-chart-others = Reports: { $n }
fb-chart-crashes = Crashes: { $n }
fb-chart-day = { $day }: { $n } reports, of them { $crashes } crashes
fb-chart-table = The days as a table
fb-chart-day-head = Day
fb-chart-crashes-head = Crashes
fb-err-chart = The numbers of the last 30 days could not be read.
fb-trash-hint = Reports in the trash are deleted for good after { $days } days.
fb-range = { $from } to { $to } of { $total }
fb-unread-here = { $n } not opened
fb-none = No report matches this list.
fb-export-all = Export all as CSV ({ $n })
fb-shown-as = Shown as
fb-as-list = List
fb-as-groups = Crash groups
fb-select-all = Select every report on this page
fb-select-one = Select: { $what }
fb-col-report = Report
fb-unread = Not opened
fb-no-text = (no text)
fb-notes-n = Notes: { $n }
fb-dup-of = Duplicate of …{ $id }
fb-record-mark = Record, { $size }
fb-by-client-short = written by the client
fb-new-tab = (opens in a new tab)
fb-pages = Pages
fb-prev-page = Previous page
fb-next-page = Next page
fb-page-of = Page { $n } of { $total }
fb-page-size = Reports per page
fb-sort-by = Order by { $column }
fb-only-label = Only the label { $label }
fb-only-assignee = Only reports looked after by { $name }
fb-only-gateway = Only the gateway { $name }
fb-only-reporter = Only reports by { $name }
fb-selection = The selected reports
fb-selected = Selected: { $n }
fb-deselect = Clear the selection
fb-set-status = Set the status …
fb-set-priority = Set the priority …
fb-assign-to = Assign to …
fb-add-label = Add a label
fb-remove-label-all = Take a label off …
fb-merge-into = Merge into …
fb-to-trash = Move to the trash
fb-restore = Restore
fb-purge = Delete for good
fb-mark-read = Mark as opened
fb-mark-unread = Mark as not opened
fb-copy-ids = Copy the ids
fb-export-selected = Export as CSV ({ $n })
fb-changed-n = Reports changed: { $n }
fb-trashed-n = Moved to the trash: { $n }
fb-restored-n = Restored from the trash: { $n }
fb-purged-n = Deleted for good: { $n }
fb-marked-read = Marked as opened: { $n }
fb-marked-unread = Marked as not opened: { $n }
fb-ids-copied = Ids copied: { $n }
fb-copy-failed = Copying did not work here.
fb-exported = Reports exported: { $n }
fb-undo = Undo
fb-dismiss = Dismiss
fb-purge-title = Delete for good?
fb-purge-text = This removes { $n } reports with their notes, history and parts from Baylee Central for good. Nothing can bring them back.
fb-purge-go = Delete them for good
fb-cancel = Cancel
fb-purge-password = Deleting for good needs a sign-in less than 30 minutes old, so you enter your password once more. It stays in this browser.
fb-purge-switched = This browser is signed in as another account now, so nothing was deleted. Load the page again.
fb-purge-refused = Baylee Central deleted nothing: a report is no longer in the trash, or no longer exists.
fb-purge-lost = No answer came back, so this page cannot tell whether the reports were deleted. It shows what Baylee Central holds now.
fb-groups = Crash groups
fb-col-crash = Crash
fb-col-count = Reports
fb-col-open = Open
fb-col-first = First
fb-col-last = Last
fb-show-group = Show its reports
fb-no-groups = No crash in this list.
fb-steps = The list this report is in
fb-back-to-list = Back to the list
fb-previous = Previous report
fb-next = Next report
fb-position = { $n } of { $total }
fb-not-in-list = Not in this list
fb-copy-link = Copy the link
fb-copy-id = Copy the id
fb-copied = Copied.
fb-in-trash-since = In the trash since { $time }. It is deleted for good { $days } days after that.
fb-wrote = What they wrote
fb-client-sent = What the client sent
fb-facts = Facts
fb-history = History
fb-triage = Triage
fb-notes = Notes
fb-related = Related lists
fb-trash = Trash
fb-received = Received
fb-changed = Last change
fb-calls-itself = calls itself
fb-gateway-build = Gateway build
fb-client-build = Client build
fb-game = Game
fb-record-part = Game record
fb-no-record = None
fb-record-here = { $size }, { $who }, { $end }
fb-record-awaited = { $size } announced, not arrived yet
fb-by-client = written by the client, unverified
fb-by-gateway = written by the gateway
fb-record-complete = the game's end is in it
fb-record-partial = the game's end is not in it
fb-download-record = Download the record (.gz)
fb-client-part = Client details
fb-no-client = None
fb-client-here = { $size }
fb-client-awaited = { $size } announced, not arrived yet
fb-id = Id
fb-crash = Crash
fb-trash-no-triage = A report in the trash is not triaged. Restore it first.
fb-assign-me = Assign to me
fb-label-hint = Lowercase letters, digits and dashes, up to 32.
fb-bad-label = A label is 1 to 32 lowercase letters, digits and dashes.
fb-remove-label = Remove the label { $label }
fb-labels-full = A report holds at most { $n } labels.
fb-duplicate-of = Duplicate of
fb-mark-duplicate = Mark as a duplicate of
fb-duplicate-hint = The other report's id or its page's address.
fb-mark = Mark
fb-bad-duplicate = That is not another report's id.
fb-not-duplicate = Not a duplicate
fb-issue = GitHub issue
fb-issue-label = Link an issue
fb-issue-hint = Its number, such as 311, or its address in Baylee's repository.
fb-link = Link
fb-unlink = Unlink
fb-bad-issue = That is no issue of Baylee's repository.
fb-new-issue = File a new issue on GitHub
fb-summary-label = A neutral summary for the public issue
fb-summary-hint = In your own words: never the player's words, their pseudonym or anything the client sent. GitHub opens with it filled in, and you file it there.
fb-category = Category
fb-summary-first = Write the summary first.
fb-open-issue = Open GitHub's form
fb-saved = Saved.
fb-conflict = Another administrator changed this report meanwhile. It shows the report as it is now: decide again.
fb-err-refused = Baylee Central refused this change.
fb-err-report = This report does not exist, or no longer exists.
fb-err-list = Baylee Central refused this list.
fb-lost-list = No answer came back, so this page cannot tell whether that was done. It shows the list as Baylee Central holds it now.
fb-lost = No answer came back, so this page cannot tell whether that was done. It shows the report as Baylee Central holds it now: check it before you try again.
fb-trashed-one = Moved to the trash.
fb-restored-one = Restored from the trash.
fb-purged-one = Deleted for good.
fb-no-notes = No notes yet.
fb-add-note = Add a note
fb-note-hint = Notes are internal: only administrators read them.
fb-chars-left = Characters left: { $n }
fb-save-note = Save the note
fb-remove-note = Remove your note of { $time }
fb-note-saved = Note saved.
fb-note-removed = Note removed.
fb-no-history = No history yet.
fb-h-central = Baylee Central
fb-h-filed = filed
fb-h-status = status
fb-h-issue = issue
fb-h-assignee = assignee
fb-h-priority = priority
fb-h-labels = labels
fb-h-duplicate = duplicate of
fb-h-trashed = moved to the trash
fb-h-restored = restored from the trash
fb-h-purged = deleted for good
fb-h-note = note
fb-h-swept = deleted by the sweep
fb-h-none = none
fb-h-original-gone = a report deleted since
fb-h-note-added = added
fb-h-note-removed = removed
fb-same-crash = The same crash
fb-its-duplicates = Its duplicates
fb-same-reporter = The same reporter
fb-same-kind = Every report of the kind { $kind }
fb-trash-lead = A report in the trash leaves every list but the trash's, and is deleted for good after { $days } days unless it is restored.
fb-no-client-part = The client sent no details with this report.
fb-client-awaiting = The client's details have not arrived yet.
fb-err-client-large = The client's details are too large to show here.
fb-err-client-shape = The client's details are not in a shape this page can read.
fb-no-shot = No screenshot.
fb-shot-alt = The client's window when the report was written, { $width } × { $height } pixels
fb-shot-full = Show at full size
fb-shot-fit = Fit to the page
fb-game-when = Moment
fb-game-turn = Turn
fb-game-turn-of = { $n }, seat { $seat }'s turn
fb-game-seat = Seat
fb-game-seat-of = { $seat } of { $n }
fb-game-pending = The game was asking
fb-game-nothing = Nothing
fb-backtrace = Backtrace
fb-log = The game's log ({ $n } lines)
fb-computer = computer
fb-system = System
fb-sys-platform = Platform
fb-sys-cpus = Processors
fb-sys-adapter = Graphics adapter
fb-sys-backend = Graphics backend
fb-sys-window = Window
fb-sys-scale = Scale
fb-sys-lang = Language
fb-settings = Settings
fb-everything = Everything, as a tree
fb-seat = seat { $n }
fb-json-all = Show all { $n } characters
fb-json-base64 = { $n } characters of base64
fb-json-more = Show { $n } more ({ $left } left)
fb-json-items = { $n } items
fb-json-keys = { $n } keys
