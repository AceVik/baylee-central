# baylee.app's words in English, Project Fluent's format. English decides
# which messages exist and which arguments each takes; build.rs turns them
# into typed functions (crates/central-web/build.rs). The reader is "you";
# a button says what it does.

## The frame of every page

site-name = Baylee
skip = 跳到正文
# Read out after a link that leaves the site (the stylesheet carries it).
new-tab = 在新标签页中打开
nav-label = 主导航
nav-download = 下载
nav-gateways = Gateways
nav-faq = 常见问题
nav-contact = 联系
nav-sign-in = 登录
nav-account = 你的账号
lang-label = 语言
theme-label = 配色
theme-system = 配色：跟随设备
theme-light = 配色：浅色
theme-dark = 配色：深色

foot-label = 关于本站
foot-play = 游玩
foot-legal = 法律信息
foot-releases = 所有版本
foot-issues = 报告错误
foot-discord = Discord 社区
foot-central-source = 本站源代码
foot-dedication = 献给Baylee。永远沐浴在阳光里。
fan-content = Baylee是非官方的爱好者内容，且遵守<policy>《爱好者内容方针》</policy>。未经威世智认同或许可。本内容中的一部分材料是威世智©Wizards of the Coast LLC的财产。
foot-colophon = 字体：Literata 和 Inter，采用 SIL Open Font License 1.1，由本站提供。绘画和标志均为我们自己的作品。<notices>声明</notices>

## Titles and descriptions

title-home = Baylee – 构筑套牌，召集朋友，尽情游玩。
desc-home = Baylee 是一款免费、开源的《Magic: The Gathering》爱好者游戏，支持 Linux、Windows 和 macOS：规则引擎认识每一张牌，牌桌最多容纳八位玩家，没有其他人时还有对手可以陪你玩。
title-download = 下载 Baylee
desc-download = 下载适用于 Windows、macOS 和 Linux 的 Baylee 客户端：最新版本的安装程序和压缩包，附校验和与签名。
title-gateways = Gateways：在线游玩 Baylee 的地方
desc-gateways = Baylee Gateways 的公开目录。Gateway 是在线牌桌所在的服务器；这里也说明如何自己运行一个。
title-faq = 常见问题与解答
desc-faq = Baylee 是什么、要花多少钱、在线游玩和账号如何运作，以及你的数据会怎样处理。
title-contact = 联系
desc-contact = 写信给 Baylee 背后的那个人，或报告错误。
title-impressum = 法律声明
desc-impressum = baylee.app 的运营者是谁（依据德国《数字服务法》（DDG）第 5 条）。
title-privacy = 隐私声明
desc-privacy = baylee.app 和 Baylee Central 会处理你的哪些信息、为什么处理、法律依据是什么，以及保存多久。
title-terms = 使用条款
desc-terms = Baylee Central（baylee.app 背后的账号服务）的使用条款。
title-fan-content = 非官方爱好者内容
desc-fan-content = Baylee 是遵守威世智《爱好者内容方针》的爱好者项目：这意味着什么，又不意味着什么。
title-not-found = 找不到页面
title-gone = 此页面已不存在
crumbs-label = 你的位置
breadcrumb-home = 首页

## Home

hero-title = <nb>Build your decks.</nb> <nb>Gather your friends.</nb> <glow>Just play.</glow>
hero-lead = 功能完备的套牌构筑器，支持最多八位玩家的对决、混战或组队对局，规则引擎会检查每一步，所以每个回合都公平，没有人需要去查规则。
hero-offline = 单人对局和套牌构筑器无需账号，离线也能用。在线牌桌位于 Gateways 上；<host>你也可以自己运行一个</host>。
hero-account = 想在线游玩？<register>创建账号</register>，公开测试期间免费，人人都可以注册；或者<signin>登录</signin>。
hero-memorial = 缅怀 Baylee：打开她的相册

status-label = 项目状态

cards-eyebrow = 牌张
cards-title = Baylee 认识多少张牌
cards-lead = Baylee 逐张实现 Magic 的牌，每一张都对照印刷的牌面文字检查。以下是最新版本的数字。
cards-implemented = 完整实现
cards-implemented-help = 这张牌的每条规则都能运作，并且经过测试。
cards-generated = 自动生成
cards-generated-help = 由 Baylee 的生成器根据牌面规则文字实现，但尚未经过人工测试。这些牌可以游玩；在少数边缘情况下，表现可能有所不同。
cards-partial = 部分实现
cards-partial-help = 这张牌可以游玩，但它的某个异能或规则中的某个边缘情况仍然缺失。
cards-total = 牌张总数
cards-total-help = Magic 的全部牌张，以 Scryfall 的列表为准；前面三个数字是 Baylee 能够游玩的牌。
cards-help = 这是什么意思
cards-share = 目前 { $total } 张牌中有 { $playable } 张可以游玩。
cards-as-of = 版本 { $tag }，统计于 { $date }。
cards-missing = 暂时无法读取牌张数字。

shot-duel-alt = 与内置 AI 的对决：六个生物攻击，其中三个被阻挡
shot-duel = <b>战术对决</b>：对手是内置 AI，每一次分配都有清晰的战斗连线。
shot-log-alt = 打开了游戏日志的对决，堆叠上有三个咒语
shot-log = <b>游戏日志</b>像聊天一样记录每一步；点击牌名即可打开牌张预览。
shot-ring-alt = 围成一圈的四人牌桌
shot-ring = <b>最多八个座位的牌桌</b>会自动围绕战场排列。
shot-lobby-alt = 大厅，显示一位朋友的消息和她发来的牌桌邀请
shot-lobby = <b>大厅：</b>开放的牌桌、好友、邀请，以及端到端加密的聊天。
shot-room-alt = 一个房间，三张空椅子由内置 AI 入座
shot-room = <b>人人都有座位：</b>真人、五个等级的内置 AI，或者一个语言模型。
shot-builder-alt = 套牌构筑器正在筛选牌池，查找天使
shot-builder = <b>套牌构筑器</b>随着你的输入搜索整个牌池。

about-eyebrow = 它是什么
about-title = 完整的规则引擎与清晰的 3D 牌桌
about-engine = 优先权、堆叠、层、替代效应、触发式异能、状态动作和战斗：这个 Rust 引擎应用完整的规则，并在每一步计算出确切的合法选择。客户端只呈现这些有效的操作。
about-table = 战场、手牌、堆叠、放逐区和坟场都摆放在整洁的 3D 牌桌上，有清晰的战斗连线和即时的牌张预览。牌图在你游玩时从 Scryfall 流式载入，也可以随时切换为清爽的牌面文字视图。

gw-eyebrow = 在线
gw-title = Gateways

open-central = GitHub 上的 Baylee Central

## Download

dl-prerelease = Beta
dl-for = 下载{ $system }版
dl-size = { $size } MB
dl-checksum = SHA-256
dl-signature = 签名
dl-notes = 发行说明
dl-unknown = 我们无法从此浏览器判断你的系统，请在下方自行选择。
dl-mobile = 目前还没有适用于手机和平板的版本。Baylee 在电脑上运行。
dl-mac-unsure = 此浏览器没有说明你的 Mac 使用的是苹果芯片还是 Intel 芯片。上面的文件适用于苹果芯片（M1 及更新）；适用于 Intel Mac 的文件在下方。
dl-windows-unsure = 如果你的 Windows 电脑使用 ARM 处理器（例如搭载骁龙的笔记本电脑），请选择下方的 ARM 文件。
dl-linux-unsure = 你的浏览器没有说明处理器型号，因此默认按 x86-64 处理。ARM 文件在下方。
dl-missing = 此版本没有适用于你的系统的文件。更早的版本在<releases>版本页面</releases>。
dl-none = GitHub 刚才没有响应。所有版本都在<releases>版本页面</releases>。
dl-yours = 你的系统
dl-more-files = 更多文件（{ $count }）
dl-installer = 安装程序
dl-deb = Debian 软件包
dl-archive = 压缩包
dl-fallback = 适用于较旧处理器的压缩包
dl-fallback-help = 适用于不支持 AVX2 的 x86-64 处理器（约 2013 年之前）。Baylee 的主要构建需要 AVX2，没有它就无法启动。
plat-windows-x64 = Windows
plat-windows-x64-sub = most PCs: 64‑bit Intel or AMD
plat-windows-arm64 = Windows on ARM
plat-windows-arm64-sub = 骁龙笔记本电脑等
plat-mac-arm64 = macOS
plat-mac-arm64-sub = 苹果芯片：M1 及更新
plat-mac-x64 = macOS（Intel）
plat-mac-x64-sub = 使用 Intel 处理器的 Mac
plat-linux-x64 = Linux
plat-linux-x64-sub = x86-64
plat-linux-arm64 = Linux on ARM
plat-linux-arm64-sub = ARM64 / aarch64
plat-none = 此版本没有适用于该系统的文件。
install-title = 首次启动与运行要求
install-first-title = 首次启动
install-first = 这些构建没有代码签名，因为签名需要花钱，所以 Windows 和 macOS 在第一次运行时会发出警告。在 Windows 上，选择<i>更多信息</i>，然后选择<i>仍要运行</i>。在 macOS 上，打开<i>系统设置 → 隐私与安全性</i>，然后选择<i>仍要打开</i>。
install-check-title = 校验下载的文件
install-check = 每个文件旁边都有一个带校验和的 <code>.sha256</code> 文件；压缩包还附带 <code>.sig</code> 文件，即客户端在自行更新前会检查的 Ed25519 签名。在 Linux 上，可用 <code>sha256sum -c</code> 校验文件；在 macOS 上用 <code>shasum -a 256</code>；在 Windows 上用 <code>certutil -hashfile</code>。
install-needs-title = 运行要求
install-windows = <b>Windows</b> 10 或 11，x64 或 ARM64。
install-mac = <b>macOS</b> 11 或更新版本，苹果芯片或 Intel 均可。
install-linux = <b>Linux</b>，glibc 2.35 或更新版本（Ubuntu 22.04 及更新版本），并需要 Vulkan 驱动；<code>.deb</code> 会一并安装其余所需的组件。
install-cpu = <b>处理器：</b>x86-64 构建需要 AVX2；较旧的处理器另有单独的压缩包。

## Gateways

gws-title = Gateways
gws-official = 官方
gws-online = 在线
gws-offline = 离线
gws-version = 版本
gws-website = 网站
gws-address = 地址
gws-copy = 复制地址
gws-copied = 地址已复制。
gws-copied-short = 已复制
gws-copy-failed = 复制没有成功。
gws-failed = 暂时无法读取目录。请一分钟后再试。

## Questions

faq-eyebrow = 常见问题
faq-title = 常见问题与解答
faq-what-q = Baylee 是什么？
faq-what-a = 一款在电脑上玩的《Magic: The Gathering》免费爱好者游戏。它有套牌构筑器、会检查每一步的规则引擎、最多八位玩家的牌桌，还有电脑对手。它不是由威世智制作、认同或许可的。
faq-cost-q = 要花多少钱？
faq-cost-a = 不花钱，而且这一点不会改变：没有付费墙，没有付费功能，也没有广告。
faq-account-q = 我需要账号吗？
faq-account-a = 单人游玩不需要：套牌构筑器和与内置 AI 的对局都可以离线使用。在线牌桌需要 Baylee 账号；公开测试期间，任何人都可以在本站免费创建。
faq-data-q = 我的数据会怎样处理？
faq-data-a = 尽可能少；<privacy>隐私声明</privacy>里写得很准确。你的密码不会离开你的浏览器；你的私人数据和聊天消息在到达我们之前，就已在你的设备上加密。
faq-password-q = 我忘记了密码，怎么办？
faq-password-a = 你需要用管理员给你的重置码来设置新密码；目前还不能在本网站上使用重置码兑换，之后会支持。只有在你拥有创建账号时收到的恢复码，或者仍在 Baylee 客户端中保持登录时，你的加密数据才能保留。否则数据就丢失了。
faq-gateway-q = 什么是 Gateway？
faq-gateway-a = 在线牌桌所在的服务器。任何人都可以运行一个；<gateways>目录</gateways>列出了 Central 所知道的那些。
faq-systems-q = 它能在哪些系统上运行？
faq-systems-a = Windows、macOS 和 Linux，支持 x86-64 和 ARM64。目前还没有适用于手机或平板的版本。
faq-open-q = 它是开源的吗？
faq-open-a = 游戏本身是开源的，采用 GNU Affero 通用公共许可证。Baylee Central 的代码是公开的，这样任何人都可以检查它如何处理自己的数据。
faq-bug-q = 我发现了一个错误，该向哪里报告？
faq-bug-a = 在游戏中按 <kbd>F8</kbd>，即可发送一份包含所需全部信息的报告。其他方式：在 GitHub 的问题追踪器上、在 Discord 上，或通过<contact>联系表单</contact>。

## Contact

contact-eyebrow = 联系
contact-title = 写信给我们
contact-name = 你的姓名
contact-name-hint = 选填。
contact-email = 你的电子邮件地址
contact-email-hint = 选填；仅用于我们回复你。
contact-message = 你的留言
contact-count = { $n } / 5,000 个字符
contact-send = 发送留言
contact-sending = 正在发送 …
contact-sent-title = 谢谢
contact-sent = 你的留言已送达。如果你留了地址，我们会在那里回复。
contact-error-empty = 请先写点内容。
contact-error-email = 这看起来不像电子邮件地址。
contact-error-busy = 你所在网络的留言暂时过多。请一小时后再试。
contact-error-refused = 留言未被接受。请检查长度后再试。
contact-error-network = 留言无法发送。请检查网络连接后再试。
contact-privacy = 你发送的内容会被保存，以便我们回复；在我们处理完毕 90 天后删除，最迟一年后删除。详见<privacy>隐私声明</privacy>。
contact-noscript = 此表单需要 JavaScript。你也可以通过 Discord 或 GitHub 联系我们。

## Legal notice

impressum-title = 法律声明
impressum-lead = 依据德国《数字服务法》（DDG）第 5 条须提供的信息。Baylee 是一个私人的非商业项目。
impressum-show = 显示详细信息
impressum-why = 详细信息位于一道简短的谜题之后，你的浏览器会在几秒钟内解开它，以阻止收集地址的机器人。除此之外不会要求你做任何事。
impressum-solving = 正在解谜 … { $percent } %
impressum-name = 姓名
impressum-address = 地址
impressum-email = 电子邮件
impressum-phone = 电话
impressum-write = Write an e-mail
impressum-call = Call
impressum-failed = 无法加载详细信息。请重试，或通过<contact>联系表单</contact>写信给我们。
impressum-noscript = 显示详细信息需要 JavaScript。没有它的话，请通过 Discord 或 GitHub 联系我们。
impressum-responsible = 内容负责人：上面所列之人。
impressum-dispute = 我们既没有义务、也不愿意参加消费者仲裁机构的争议解决程序。

## Fan content

fc-title = 非官方爱好者内容
fc-eyebrow = 爱好者内容
fc-lead = Baylee 是一个爱好者项目。它遵守威世智的《爱好者内容方针》，并按其规定行事。
fc-notice-title = 每个页面上的声明
fc-means-title = 这意味着什么
fc-means = Baylee 不是由威世智制作、认同或许可的。Magic: The Gathering、其牌名、系列名称、牌张图像和法术力标志均为 Wizards of the Coast LLC 的财产。Baylee 对所有人免费：没有付费墙，没有付费功能，也不需要通过电子邮件注册。
fc-images = 游戏显示来自 Scryfall 的牌张图像，未经改动并附有画师署名；本站不显示任何牌张图像。牌张数据和图像由 Scryfall 提供；Baylee 与 Scryfall 没有关联。
fc-policy = 威世智的<policy>《爱好者内容方针》</policy>

## Privacy notice

privacy-title = 隐私声明
privacy-lead = 本站和 Baylee Central 会处理你的哪些信息、为什么处理、法律依据是什么，以及保存多久。
privacy-updated = 截至 2026 年 11 月 11 日
privacy-controller-title = 谁是责任方
privacy-controller = GDPR（《通用数据保护条例》）所称的控制者，是<impressum>法律声明</impressum>中列出的那位自然人，声明中给出了其姓名、邮寄地址和电子邮件地址。你也可以通过联系表单发送隐私方面的问题。我们没有数据保护官；也没有法律要求设置。
privacy-short-title = 简而言之
privacy-short = 没有追踪，没有广告，没有分析，除非你登录否则没有 Cookie，也不从其他服务器加载任何内容：字体、图片和脚本全部来自 baylee.app。你的密码不会到达我们这里，你的聊天消息和私人数据在发送之前已在你的浏览器中加密。
privacy-hosting-title = 访问本站
privacy-hosting = 本站运行在我们向德国 netcup GmbH 租用的服务器上。当你打开页面时，你的浏览器会发送每个请求都带有的信息：你的 IP 地址、所请求的地址、时间、浏览器的语言及其标识（user agent）。服务器需要这些信息来作出响应；它根据语言选择语言版本，下载页面则根据 user agent 推荐合适的文件。网络服务器不保留访问日志，因此这些请求不会被存储。服务器自身的运行信息（系统日志）在少数出错的情况下可能包含 IP 地址；它们最迟在 14 天后删除。法律依据：GDPR 第 6 条第 1 款 (f) 项；我们的合法利益是安全、可靠地提供本站。
privacy-device-title = 你的浏览器保存什么
privacy-device = 在你登录之前，本站不设置任何 Cookie。你的浏览器会在你的设备上保存以下内容，并且仅在启用 JavaScript 时：如果你做过选择，则保存你的配色选择（<code>baylee-site-theme</code>）；在你登录期间，保存会话何时结束（<code>baylee-central-session</code>）、你的聊天密钥以及联系人的受信任密钥（见“聊天”）；如果你开启了通知，则保存本设备的一个随机数（<code>baylee-push-installation-v1</code>、<code>baylee-push</code>）；并且，仅当你已登录、将本站安装为应用或开启了通知时，保存本站公开文件（页面外壳、字体、脚本）的一份副本，使本站无需网络连接即可启动，也使通知能够送达。如果这三种情况都不再适用，该副本会在你下次打开本站时被移除。除非下文某一节另有说明，这些内容都不会发送给我们。聊天和会话相关的条目会在你退出登录时删除；其余所有内容，你可以通过清除浏览器中本站的数据来移除。法律依据：TDDDG 第 25 条第 2 款第 2 项（对你所使用的功能而言是严格必需的）；对于通知所用的随机数，则是你的同意（TDDDG 第 25 条第 1 款）。
privacy-github-title = GitHub
privacy-github = 下载页面和起始页面会显示最新版本、测试状态和牌张数字。是我们的服务器向 GitHub 请求这些信息，而不是你的浏览器；GitHub 不会因此得知任何关于你的信息。只有当你点击下载链接或 GitHub 链接时，你的浏览器才会连接到 GitHub（GitHub, Inc.，美国），并向它发送你的 IP 地址以及每个请求都带有的信息；此后适用 GitHub 自己的隐私声明。
privacy-directory-title = Gateway 目录
privacy-directory = 我们的服务器从 Baylee Central 读取 Gateways 的目录并显示出来；不涉及任何关于你的信息。
privacy-account-title = 你的账号
privacy-account = 对于一个账号，Baylee Central 会存储：你的账号 ID、你的登录名、你的显示名及其标签、账号创建的时间和方式、一条 OPAQUE 密码记录（它让你可以证明自己知道密码，而我们的服务器始终不会得知密码）、被你的密码和你的恢复码各加密一次的主密钥、你的加密账号文档和你的加密对象（我们无法读取；我们只看得到大小、修订号和时间）、你接受了哪个版本的条款以及接受的时间、你上次登录的时间、你的会话、管理员角色（如果你有的话），以及你注册的 Gateways 及其令牌。其他玩家看到的是你的显示名和标签，而不是你的登录名。如果管理员给你一个重置码，我们只存储它的摘要，直到该重置码被使用或已过期。管理员对某个账号或 Gateway 所做的操作会记录在案并保存三年（谁、做了什么、涉及哪个账号或 Gateway、何时）。法律依据：GDPR 第 6 条第 1 款 (b) 项（你所请求的账号）；对于上次登录的时间和该记录，则是 GDPR 第 6 条第 1 款 (f) 项（我们在安全、可追溯的运营方面的合法利益）。没有登录名和密码就没有账号；你不需要电子邮件地址。你从 16 岁起可以创建账号。保存到你删除账号为止；删除后，数据会立即从运行中的系统中移除，并最迟在 12 个月内从备份中移除。会话在 30 分钟未使用后结束，最迟在登录 12 小时后结束。
privacy-cookie-title = 会话 Cookie
privacy-cookie = 登录会设置一个 Cookie：<code>__Host-central</code>，它是用来标识你的会话的随机数。它只会发送给 baylee.app，脚本无法读取，并随会话结束而失效，最迟 12 小时后失效。它是让你保持登录所严格必需的，无需同意。法律依据：TDDDG 第 25 条第 2 款第 2 项以及 GDPR 第 6 条第 1 款 (b) 项。
privacy-chat-title = 聊天和联系人
privacy-chat = 消息在你的设备上针对每位收件人的设备加密；Baylee Central 只以加密形式存储它们，无法读取。Central 能看到并存储的内容：你的联系人、联系请求和屏蔽、群组及其名称、成员和邀请、谁在何时向谁发送了消息以及消息有多大，还有你各设备的公钥。你的联系人和你所在群组的成员会看到你的显示名及其标签以及你的消息。在你登录期间，你的浏览器会保存你的聊天密钥（由只有此浏览器持有的密钥封存，存放在其存储空间 <code>baylee-chat</code> 中）以及你已信任的联系人的密钥（已加密，<code>baylee-chat-trust</code>）。消息和投递记录保存到你删除账号为止；目前还没有更短的自动保留期限。每个发送者和每台设备最多保存 5,000 条消息；超过之后，新消息会被拒收。法律依据：GDPR 第 6 条第 1 款 (b) 项。
privacy-push-title = 通知
privacy-push = 如果你在浏览器中开启通知，你的浏览器会向其推送服务注册，Central 会存储该服务针对此浏览器的地址、浏览器的公钥和身份验证密钥、你的语言、本设备的一个随机数以及你选择的类别。推送服务属于你的浏览器厂商：Google（Firebase Cloud Messaging）、Mozilla 或 Apple，均位于美国。一条通知经加密后只携带其种类（例如“新消息”）和你的语言，绝不包含姓名或文字；推送服务能看到地址、时间和大小等连接数据。当你在本设备上关闭通知、退出登录或删除账号时，这些信息会被删除；90 天未更新的地址不再使用，等待中的通知在一天后过期。法律依据：GDPR 第 6 条第 1 款 (a) 项和 TDDDG 第 25 条第 1 款（你的同意）。你可以随时在账号设置中撤回同意；在此之前所做的处理仍然合法。
privacy-counters-title = 防止滥用
privacy-counters = 为了阻止猜测密码和过载，Central 会按 IP 地址统计某些请求（注册、登录尝试、密码重置、读取目录），且仅保存在内存中：注册最长统计一天，其余最多统计一小时。在注册之前，你的浏览器还会解开一道谜题；它由随机数组成，不含任何关于你的信息，最迟五分钟后即被遗忘。对于每个账号，Central 会统计联系请求和对其他玩家的搜索，以免有人用请求淹没其他人。法律依据：GDPR 第 6 条第 1 款 (f) 项；我们的合法利益是保护账号和服务不被滥用。
privacy-contact-title = 联系表单
privacy-contact = 表单会把你的留言、页面的语言和时间发送到同一台服务器上的 Baylee 反馈服务，仅在你填写时才会发送你的姓名和电子邮件地址；两者都是选填。该服务还会记录这条留言是否已处理。你的 IP 地址不会被存储；它只在内存中计数一小时，以阻止大量留言涌入。只有法律声明中所列之人会阅读这些留言；不会转交给任何人。留言在处理完毕 90 天后删除，最迟一年后删除，如果你要求，也可以更早删除。法律依据：GDPR 第 6 条第 1 款 (f) 项（我们回复留言的合法利益），或者在你的留言涉及你的账号时，为 (b) 项。
privacy-impressum-title = 法律声明的谜题
privacy-impressum = The legal notice is shown only after your browser has solved a puzzle; that protects the address from programs that collect addresses. The puzzle consists of random numbers and holds nothing about you. For this, the feedback service keeps in memory only, never on disk and never in a log: how many puzzles your IP address asked for (for one hour; an IPv6 address by its first half), whether it followed a link that is hidden from people (for 24 hours), and which puzzles were solved (for five minutes, without an address). Three yes-or-no answers about your browser set the puzzle's difficulty and are not kept: whether a program drives it, whether it names no languages, and whether it draws without a graphics card. Nobody is refused; a browser that looks like a program gets a longer puzzle. Legal basis: Art. 6(1)(f) GDPR (our legitimate interest in protecting a private person's address).
privacy-recipients-title = 接收方和国家
privacy-recipients = 我们不出售数据，也不为广告而向他人提供数据。接收方有：netcup GmbH（德国的托管服务，依据 GDPR 第 28 条作为处理者）；Baylee Central 的管理员，他们为管理之需会看到你的登录名、你的显示名及其标签、你账号的来源、创建时间和上次登录时间，以及你的 Gateways，但绝不会看到你的加密数据；你用账号登录的 Gateway（它会收到你的账号 ID、显示名和标签，不会收到你的密码）；你的联系人和你所在群组的成员（显示名、标签和你的消息）；仅在法律要求我们提供时才提供给主管机关。欧洲经济区以外的国家：只有当你开启通知时，你浏览器的推送服务（Google、Mozilla 或 Apple，美国）才会承载通知。他人运营的 Gateways 可能位于任何国家；是否在那里登录由你决定。除此之外，我们不向欧洲经济区以外的国家传输任何数据。
privacy-rights-title = 你的权利
privacy-rights = 你有权访问（GDPR 第 15 条）、更正（第 16 条）、删除（第 17 条）、限制处理（第 18 条）和数据可携（第 20 条）。你可以随时基于与你的具体情况有关的理由，反对基于我们的合法利益（第 6 条第 1 款 (f) 项）进行的处理（第 21 条）。你可以随时撤回同意，对未来生效（第 7 条第 3 款）。你可以在账号设置中导出账号数据并删除账号；对于其他一切，通过联系表单或发送到法律声明中的地址的一条消息就足够了。你有权向数据保护监管机关投诉（第 77 条），例如你居住地的监管机关，或北莱茵-威斯特法伦州数据保护和信息自由事务专员。没有任何法律或合同要求你向我们提供数据；但是，没有登录名和密码就没有账号。不存在自动化决策，也不存在画像（第 22 条）。
privacy-objection-title = 你的反对权
privacy-objection = 凡是我们基于 GDPR 第 6 条第 1 款 (f) 项处理数据的情形（提供本站、防止滥用、联系表单、法律声明的谜题、你上次登录的时间、管理记录、错误报告和使用计数），你都可以随时基于与你的具体情况有关的理由提出反对（GDPR 第 21 条）。此后我们会停止处理这些数据，除非我们能证明存在压倒你的利益的、令人信服的合法理由，或者该处理是为了确立、行使或抗辩法律上的主张。通过联系表单或发送到法律声明中的地址的一条非正式消息就足够了。
privacy-links-title = 指向其他服务的链接
privacy-links = 本站链接到 GitHub、Discord 和 WhatsApp 等。这些都是普通链接：没有嵌入任何内容，在你点击之前，这些服务都不会得知你的访问。当你点击时，你的浏览器不会告知你来自哪个页面（没有 referrer）。此后适用各提供商自己的隐私声明，他们可能在欧盟以外处理数据；你在那里的社区群组中，其他成员能看到你的哪些信息，由这些服务决定。
privacy-reports-title = 来自游戏的错误报告和使用计数
privacy-reports = 你在游戏中用 F8 发送的错误报告包含其种类、你的文字、客户端版本和你保持勾选的附件；联系页面列出了这些内容。报告在处理完毕 90 天后删除，最迟三年后删除。法律依据：GDPR 第 6 条第 1 款 (f) 项（我们让游戏没有错误的合法利益）。客户端仅在你允许时才发送崩溃报告（GDPR 第 6 条第 1 款 (a) 项；你可以随时在客户端的设置中撤回）。离线游戏期间，客户端每分钟发送一次该局游戏的随机标识符，除此之外不发送任何内容；我们只在内存中保存它，并在它最后一次发出活动迹象 150 秒后将其忘记（GDPR 第 6 条第 1 款 (f) 项；可在“Send anonymous usage count”下关闭）。与每一次连接一样，你的 IP 地址会到达我们这里。

## Terms

terms-title = 使用条款
terms-draft = 这些条款是草案，尚未生效。
terms-version = 版本 { $version }，更新于 { $date }。
# Shown only in the languages a machine translated: every one but German and English.
machine-note = 本站由机器翻译，尚无使用你所用语言的人通读过。游戏本身目前只有英文和德文。<link>请告诉我们哪里有错。</link>
legal-machine = 本页是机器翻译，仅作辅助。如与<de>德文</de>或<en>英文</en>文本有出入，以后者为准。
terms-machine = Central 的条款只有德文和英文版本。下面是英文文本；另有<de>德文版</de>。

## Errors

not-found-title = 此页面不存在
not-found-lead = 地址可能输错了，或者页面已经移走。
gone-title = 此页面已永久移除
gone-lead = 这里原有的内容已被删除。起始页面会告诉你接下来去哪里。
error-home = 回到起始页面

## Baylee's remembrance (approved by her family on 27 October 2026; keep word for word)
baylee-dedication = 献给Baylee。永远沐浴在阳光里。
baylee-invitation = 来认识这个世界背后的小小灵魂
baylee-open = 缅怀 Baylee · 打开她的相册
baylee-eyebrow = 这个项目的心
baylee-date = 深切怀念 · 2026 年 10 月 26 日
baylee-close = 回到她的世界
baylee-previous = 上一张照片
baylee-next = 下一张照片
baylee-photo1 = Baylee 蜷缩在柔软的白色被子里。
baylee-photo2 = Baylee 从带花的被子里向外张望。
baylee-photo3 = 在她圆圆的灰色小床里休息。
baylee-photo4 = Baylee 睡着了，紧紧地蜷成一团。
baylee-photo5 = 安静的一刻，依偎在被子里。

baylee-storyTitle = 献给Baylee。永远沐浴在阳光里。
baylee-together = 与我们在一起 · 2025 年 11 月 17 日 – 2026 年 10 月 26 日
baylee-chapters = 她的故事
baylee-chapterPrevious = 上一章
baylee-chapterNext = 下一章
baylee-photos = 我们的回忆
baylee-read = 向下滚动阅读

baylee-chapter1Title = Baylee 的家
baylee-chapter1Body =
    2025 年 11 月 17 日，Baylee 来到了我们家。

    我和妻子是从克雷费尔德的动物收容所领养她的。她被估计大约十三岁。她患有恶性乳腺肿瘤，癌细胞已经扩散到肝脏。Baylee 是一只接受安宁照护的猫。我们知道，我们共度的时间有限。在我们身边，她要有一个被爱着的家。

    很快她就让我们看到，她多么迅速地把这个家当成了自己的。

    在她到家的第一天，我工作的时候，她就悄悄跳上了我的腿。而在她的第一个夜晚，她撕开了一袋我为父母的猫订购的猫粮，自己吃了起来。当然，她有自己的猫粮，那天晚上也吃得很好。但 Baylee 显然认为，这份存货仍然值得她更仔细地调查一番。

    我们的小老鼠胃口惊人，精力也不可思议。

baylee-chapter2Title = 渐渐信任
baylee-chapter2Body =
    当我们注意到她喝了很多水时，就带她去看了兽医。血液检查显示她有甲状腺疾病，这也解释了她为什么那么饿、那么活泼。她开始服用甲状腺药物，又因为炎症指标升高而服了一个疗程的抗生素。

    有一天，她忽然开始在所有东西上蹭来蹭去，叫个不停，变得异常粘人、依恋我们，我们起初担心她对某种新药产生了不良反应。再次检查后得到了解释：我们的小宝贝正在发情。她没有做过绝育手术。

    那段时间我们也一起熬了过来。Baylee 越来越信任我们，我们彼此也越来越亲近。

baylee-chapter3Title = 我们的小公主
baylee-chapter3Body =
    我至今清楚地记得，她得到猫爬架时有多高兴。她立刻坐到最顶上的那个平台，满意地环顾整个屋子。说来奇怪，她再也没有用过那个位置。为什么，始终是她的秘密。

    别的地方很少空着：我的腿上、我的双腿之间，还有我们的床。她通常睡在我身上或妻子身上。最好的是同时躺在我们两个人身上。

    她翘着尾巴在屋子里昂首阔步，仿佛她清楚得很，自己就是这里的公主。她有脾气，也懂得表明自己的底线。在我们身边，她可以完全做自己。她的怪癖、她的要求和她那点小小的倔强，我们也全都爱着。

    我从未与一只动物建立过如此深的联系。Baylee 在我和妻子的心里，都占据了很深很深的位置。

baylee-chapter4Title = 阳光下的时光
baylee-chapter4Body =
    天气变暖之后，她发现了阳台。她喜欢让一天里的第一缕阳光洒在毛上。没过多久，她几乎整天都待在那里。

    鸽子和庭院里的鸟儿尤其让她着迷。她专注地盯着它们，埋伏着，一次又一次地想把它们吓飞。在晒太阳和看鸟之间，我们的小公主每天的日程排得满满的。

baylee-chapter5Title = 她自己的小主意
baylee-chapter5Body =
    与此同时，她的病情仍在发展。

    肿瘤发炎了，Baylee 不停地舔，把患处舔破。在兽医那里，她拿到了第一件术后康复服，是红色的。从那以后，她每天都得穿着，免得再伤到伤口。

    她很快就习惯了穿它。但是被人套上它，直到最后一天她都不喜欢。那时就连一碗食物也只能起到有限的哄劝作用。她的胃口再大，对这件衣服的看法也明明白白。

    伤口越来越严重。它流脓，最后开始出血，并且有了难闻的气味。与此同时，Baylee 一直向我们展示她在日常生活中仍然能找到多少快乐。她玩耍、吃饭、探索周围，爬她的猫抓板和柜子。

    有时她会在我们旁边的床沿上一躺就是几个小时，等着我们终于醒来，给她准备早餐。如果我们拖得太久，她就干脆从我们身上爬过去。我们的脸也挡不住她。

    她对食物有非常明确的想法。她的口味更接近“冷冻披萨”那一派：过了一阵子，优质的猫粮就再也打动不了她，花样变换必不可少。同样的食物连着吃两次，想都别想。鸡肉配肝、任何带奶酪的东西、金枪鱼或三文鱼——这些才能让她高兴。

baylee-chapter6Title = 高高翘着尾巴
baylee-chapter6Body =
    肿瘤越长越大，渗液也越来越多。Baylee 明显很痛。疼痛缓解的时候，她又变回了我们的小猫，翘着尾巴穿过一个个房间，把她的人据为己有。

    我们看得出她的力气在慢慢消退。但她的胃口、她的好奇心、她亲昵的天性和她响亮的呼噜声依然都在。

baylee-chapter7Title = 告别
baylee-chapter7Body =
    2026 年 10 月 23 日星期五的晚上，她出现了起初看上去像感冒的症状。她流鼻涕，呼吸深而吃力，仿佛在喘不过气来。

    星期六她似乎又精神了一些。反常的呼吸和流鼻涕还在，但她再次显露出我们熟悉的那种活力和对生活的热爱。

    星期日早晨很难熬。她的眼睛在流泪，呼吸困难，而且很痛。我给她吃了止痛药。大约一小时后她又好受了些，但她的呼吸仍然和以前不同。我们知道必须带她去看兽医了。

    星期一早晨，她的状况又恶化了。幸好妻子那天休息。我们打电话给诊所，约好了时间，带 Baylee 去了那里。

    X 光片显示了癌症扩散到什么程度。我们被告知，她的肺里只剩大约三分之一还能用来呼吸。让她继续活下去，意味着她能呼吸到的空气会越来越少。

    我们不得不告别。

    2026 年 10 月 26 日星期一，大约中午时分，我们最亲爱的 Baylee 被安乐死了。

baylee-chapter8Title = 永远沐浴在阳光里
baylee-chapter8Body =
    她和我们在一起不到一年。在这段时间里，她成了我们家庭的一员，也成了一位非常亲近的朋友。她属于我们工作的日子，属于我们的夜晚，属于我们早晨醒来的时刻。属于阳台上阳光明媚的时光，也属于那些构成共同生活的小习惯。

    如今，在所有这些时刻里都没有她了。

    空落落的感觉依然新鲜而深重。我们想念她的呼噜声，想念她压在我们腿上的重量，想念她竖起的尾巴，还有她理所当然地期待我们终于去准备她早餐的样子。

    如果这一生之后还有什么，不论那是什么，我希望有一天能再次遇见那个灵魂。

    在那之前，我们会把我们的小老鼠永远放在心里。

    正因如此，这个业余项目是献给 Baylee 的。献给她的灵魂，她的陪伴，她对阳光和小鸟的喜爱。也献给我们共度时光里滋长的爱。

    愿她在这里安睡在她的云朵上，在温暖的阳光里，被鲜花、鸽子和庭院里的鸟儿环绕。

    安息吧，我最亲爱的 Baylee。

baylee-photoOpen = 查看完整照片
baylee-photoClose = 关闭照片
baylee-zoomIn = 放大
baylee-zoomOut = 缩小
baylee-fit = 适应窗口

baylee-photo6 = Baylee 舒舒服服地窝在毯子里。
baylee-photo7 = 裹在她的红色康复服里依偎着。
baylee-photo8 = 纸板箱里的一处阳光。
baylee-photo9 = 被裹得严严实实，完全放松。
baylee-photo10 = 她最喜欢的地方：就在我们身旁。

## Errors shared by the forms

error-busy = 刚才尝试的次数过多。请等一分钟后再试。
error-unavailable = Baylee Central 暂时没有响应。请稍后再试。
error-network = 没有网络连接。请检查网络后再试。
error-failed = 出了点问题。请重试；如果问题持续出现，请告诉我们。
error-again = 重试
password-show = 显示

## Signing in

signin-title = 登录
signin-lead = 你的 Baylee 账号，用于在线牌桌、联系人和聊天。
signin-name = 登录名
signin-password = 密码
signin-submit = 登录
signin-missing = 请输入你的登录名和密码。
signin-working = 正在保护你的密码 …… 这需要片刻，并且只在你的浏览器中进行。
signin-wrong = 该登录名和密码不匹配。请检查两者后再试。
signin-not-moved = 此账号尚未迁移到 Baylee Central。请先用 Baylee 客户端登录一次；它会迁移你的账号。
signin-no-account = 还没有账号？<register>创建一个</register>。
signin-noscript = 登录需要 JavaScript：你的密码在你的浏览器中处理，绝不会发送给我们。

## Registering

register-title = 创建账号
register-lead = 用于在线牌桌、联系人和聊天。公开测试期间免费，人人都可以注册。
register-name = 登录名
register-name-hint = 私密，仅用于登录：3 到 24 个小写字母、数字、点、短横线或下划线。
register-display = 显示名
register-display-hint = 其他玩家看到的你的名字：3 到 16 个字母、数字、- 和 _。你的标签会自动生成。
register-password = 密码
register-password-hint = 至少 8 个字符。一句较长的话容易记住，也很难被猜到。
register-repeat = 再次输入密码
register-repeat-hint = 再输入一遍，以便发现输错的地方。
register-age = 我已年满 16 岁。
register-consent = 我接受<terms>使用条款</terms>。我们如何处理你的数据，见<privacy>隐私声明</privacy>。
register-submit = 创建账号
register-error-name = 登录名需要 3 到 24 个小写字母、数字、点、短横线或下划线。
register-error-display = 显示名需要 3 到 16 个字母或数字；- 和 _ 只能放在中间。
register-error-reserved = 此显示名已被保留。请选择别的。
register-error-short = 密码至少需要 8 个字符。
register-error-long = 密码太长。
register-error-repeat = 两次输入的密码不一致。
register-error-age = 你需要年满 16 岁才能拥有账号。
register-error-consent = 请接受使用条款以创建账号。
register-error-taken = 此登录名已被占用。请选择别的。

## The recovery code

code-title = 你的恢复码
code-handle = 你的用户标识：{ $handle }
code-lead = 只有凭此恢复码，你的加密数据才能在你忘记密码时保留下来。我们不知道它，也无法恢复它。请把它保存在安全的地方，远离这台设备。
code-shown-once = 仅此一次显示
code-download = 保存为文本文件
code-kept = 我已保存恢复码
code-not-kept = 请先保存恢复码，再确认。
code-continue = 继续
code-later = 暂不

## Central's terms

terms-step-title = 新的使用条款
terms-step-lead = Baylee Central 的条款已更改。请阅读并接受，才能使用你的账号；你也可以拒绝并退出登录。
terms-accept = 接受
terms-decline = 拒绝并退出登录

## The signed-in area

account-lead = 你的 Baylee 账号：个人资料、安全、隐私和 Gateways。
account-nav = 你的账号
account-home = 概览
account-profile = 个人资料
account-profile-lead = 你的用户标识、显示名和恢复码。
account-security = 安全
account-security-lead = 你在哪些地方登录，以及退出登录。
account-privacy = 隐私与数据
account-privacy-lead = 导出你的数据或删除账号。
account-gateways = Gateways 与令牌
account-gateways-lead = 你注册的 Gateways，以及它们用来登录的令牌。
account-loading = 正在加载 …
account-signed-out = 你尚未登录。
account-unnamed = 你的账号
account-signin-name = 登录名：{ $name }
account-until = 登录有效期至 { $time }
account-needs-title = 需要你处理
account-needs-code = 创建恢复码
account-needs-name = 选择你的显示名
account-admin = 管理

reauth-lead = 此操作需要你最近登录过。请再次输入登录名和密码。
reauth-submit = 重新登录

profile-who-title = 你是谁
profile-handle = 用户标识
profile-signin-name = 登录名
profile-source = 账号
profile-source-moved = 从官方 Gateway 迁移而来
profile-source-registered = 在本站注册
profile-choose-title = 选择你的显示名
profile-choose-submit = 保存显示名
profile-name-taken = 此账号已经有显示名了。
profile-code-title = 恢复码
profile-code-kept = 你的账号有一个恢复码。请妥善保管；没有密码时，它是找回加密数据的唯一途径。
profile-code-missing = 你的账号还没有恢复码。请创建一个；你需要再次输入密码。
profile-code-make = 创建恢复码

security-sessions = 你在哪些地方登录
security-lead = Baylee Central 为本网站保留一个会话，为游戏客户端保留一个会话；它不区分单个设备。
security-kind-browser = 本网站
security-kind-client = Baylee 客户端
security-this = 当前会话
security-signed-in = 登录于
security-ends = 结束于
security-gateways = 在 Gateways
security-end-client = 让客户端退出登录
security-client-ended = 客户端已退出登录。
security-sign-out = 在此退出登录
security-password-title = 密码
security-password-later = 在网站上更改密码的功能将会推出；在此之前，请在 Baylee 客户端中更改。

privacy-export-title = 导出你的数据
privacy-export-lead = Baylee Central 所掌握的关于你账号的信息，以 JSON 文件形式提供：其基本数据、会话、已接受的条款、你的 Gateways，以及点名你账号的审计条目。你的加密内容只能由你的 Baylee 客户端解密并导出。导出需要 30 分钟以内的登录。
privacy-export-make = 准备导出
privacy-export-save = 保存导出文件
privacy-delete-title = 删除你的账号
privacy-delete-lead = 你的账号、其数据、其会话、你的联系人以及你拥有的 Gateways，会立即并永久地从运行中的系统删除；它们最迟在 12 个月内从备份中消失。Central 也会删除你发送的消息；其他人已经存储在其设备上的内容可能仍会保留在那里。
privacy-delete-start = 删除我的账号
privacy-delete-password = 你的密码，用于确认
privacy-delete-prove = 继续
privacy-delete-last = 最后一步：此操作无法撤销。你有一分钟时间。
privacy-delete-gateways = 你拥有的这些 Gateways 会随之删除：
privacy-delete-confirm = 永久删除账号
privacy-delete-cancel = 保留我的账号
privacy-deleted = 你的账号已删除。

gateways-none = 你还没有注册 Gateway。大多数玩家用不着。
gateways-add-title = 注册 Gateway
gateways-origin = Gateway 地址
gateways-origin-hint = Gateway 的 https 地址，例如 https://play.example.org。
gateways-name = Gateway 名称
gateways-add = 注册 Gateway
gateways-added = 该 Gateway 已注册，正在等待管理员批准。
gateways-taken = 此地址已被注册。
gateways-invalid = 请输入不含路径的 https 地址，以及一个名称。
gateways-approved = 已批准
gateways-pending = 等待批准
gateways-disabled = 已停用
tokens-title = 令牌
tokens-name = 令牌名称
tokens-days = 有效期（天）
tokens-sso = 让玩家登录
tokens-listing = 列出该 Gateway
tokens-both = 让玩家登录并列出该 Gateway
tokens-make = 创建令牌
tokens-revoke = 吊销
tokens-revoked = 已吊销
tokens-revoked-now = 令牌已吊销。
tokens-until = 至 { $time }
tokens-invalid = 请给令牌起个名字，并至少选择一项它允许的功能。
tokens-shown-title = 你的新令牌
tokens-kept = 我已保存
tokens-again = 已重新登录。请再次创建令牌。

## The messenger

account-chat = 聊天
account-contacts = 联系人
chat-side = 会话和联系人
chat-tabs = 即时通讯
chat-tab-chats = 聊天
chat-tab-contacts = 联系人
chat-main = 会话
chat-unread = 未读
chat-conversations = 你的会话
chat-search = 查找会话
chat-search-hint = 查找（Ctrl+K）
chat-new-group = 新建群组
chat-no-conversations = 还没有会话。添加一位联系人，然后给他们写消息。
chat-no-match = 没有叫这个名字的会话。
chat-former-group = 一个已退出的群组
chat-former-lead = 你已不在这个群组中。
chat-you = 你
chat-members = { $n ->
   *[other] { $n } 位成员
}
chat-say-hello = 打个招呼吧。
chat-history-only = 已不是联系人。
chat-today = 今天
chat-yesterday = 昨天
chat-encrypted = 端到端加密
chat-opening-lead = 你的会话和联系人。消息在离开你的设备之前就已在设备上加密，Central 只负责转交，无法读取。
chat-new-from = 来自 { $who } 的新消息。
chat-new-many = { $n } 条新消息。
chat-retry = 重试
chat-discard = 丢弃
chat-not-sent = 未发送。
chat-sending = 正在发送 …
chat-keys-changed = 此会话中有人的密钥发生了变化。请先比对安全码，再继续写消息。
chat-refused = 有些消息无法验证，因此不予显示。
chat-failed = 聊天暂时无法连接到 Central。
chat-failed-open = 无法打开聊天。请刷新页面后再试。
chat-empty = 还没有消息。
chat-older = 显示更早的消息
chat-compare-first = 密钥已变化。请先比对安全码，再写消息。
chat-compare = 比对安全码
chat-friend-required = 重新成为联系人之后，你才能写消息。
chat-group-left = 你已不在这个群组中。
chat-details = 详情
chat-close = 关闭
chat-back = 返回会话列表
chat-messages = 消息
chat-fan = Baylee 是非官方爱好者内容，未经威世智认同或许可。<more>这意味着什么</more>
chat-safety = 安全码
chat-safety-lead = 请当面或通过另一个你信任的渠道，与对方比对这串数字。如果一致，就没有人悄悄插到你们之间。
chat-changed = 已变化
chat-confirm-keys = 一致：信任新密钥
chat-count = { $n } / 1,000 个字符
chat-write = 写消息
chat-send = 发送
chat-enter-hint = 按 Enter 发送，按 Shift+Enter 换行。
chat-choose = 选择一个会话
chat-choose-lead = 你的消息在此浏览器中加密；Central 只负责转交。
chat-storage = 你登录期间，聊天密钥保存在此浏览器中。
chat-opening = 正在打开你的聊天 …
chat-open = 打开我的聊天
chat-ended = 你的登录已结束。
chat-unnamed = 请先选择你的显示名：它给你一个别人用来给你写消息的用户标识。
chat-missing-title = 请重新登录以打开你的聊天
chat-missing-lead = 此浏览器中没有你的聊天密钥。你的密码可以在这里把它打开；密码不会离开此浏览器。
chat-invites = 群组邀请
chat-invited-by = 来自 { $who }
chat-group-name = 群组名称
chat-group-name-hint = 1 到 48 个字符的名称。
chat-group-create = 创建
chat-group-failed = 暂时无法更改该群组。
chat-group-members = { $n ->
   *[other] { $n } 位成员
}
chat-group-owner = 群主
chat-group-rename = 重命名
chat-group-transfer = 设为群主
chat-group-remove = 移除
chat-group-transfer-ask = 让 { $who } 成为此群组的群主吗？你仍是成员。
chat-group-remove-ask = 将 { $who } 移出此群组吗？
chat-group-invite = 邀请联系人
chat-group-choose = 选择联系人
chat-group-invite-button = 邀请
chat-group-invited = 已邀请
chat-group-delete = 删除群组
chat-group-leave = 退出群组
chat-group-delete-ask = 为所有人删除“{ $name }”吗？已收到的消息仍可阅读。
chat-group-leave-ask = 退出“{ $name }”吗？只有收到新的邀请才能回来。

people-incoming = 请求
people-failed = 暂时无法加载你的联系人。
people-empty = 还没有联系人。通过用户标识找到某人，或把你的用户标识告诉他们。
people-your-handle = 你的用户标识：
people-incoming-title = 给你的请求
people-friends = 联系人
people-outgoing = 已发送的请求
people-blocked = 已屏蔽
people-find = 通过用户标识添加联系人
people-find-button = 查找
people-handle-hint = 带标签的完整用户标识，例如 Name#1a2b。
people-yourself = 这是你自己的用户标识。
people-nobody = 没有人使用这个用户标识。
people-request = 发送联系请求
people-requested = 请求已发送。
people-is-friend = 联系人
people-is-incoming = 请求成为你的联系人
people-is-outgoing = 请求已发送
people-is-blocked = 已屏蔽
people-write = 写消息
people-remove = 移除
people-block = 屏蔽
people-accept = 接受
people-decline = 拒绝
people-cancel = 撤回
people-unblock = 取消屏蔽
people-keep = 保留
people-confirm-remove = 将 { $who } 从你的联系人中移除吗？你的消息会保留。
people-confirm-block = 屏蔽 { $who } 吗？对方将无法再请求成为你的联系人，也无法给你写消息。

## Offline, updates, installing

title-offline = 离线
offline-title = 你已离线
offline-lead = 此页面需要网络连接。在你重新上线之前，Central 会为你保存消息。
offline-again = 重试
update-ready = baylee.app 有新版本了。
update-unsent = 这里有尚未发送的文字。仍要刷新吗？
update-reload = 刷新
install-app = 安装为应用

## Notifications

push-title = 通知
push-lead = 选择 Baylee 可以就哪些事项通知你。通知不会显示消息文字、姓名或报告内容。
push-device = 本设备上的通知
push-categories = 通知我以下事项
push-chat = 聊天消息
push-contacts = 联系请求
push-updates = 新版本
push-feedback = 反馈报告（管理员）
push-gateways = Gateway 申请（管理员）
push-on = 已在本设备上启用。只会送达所选的类别。
push-off = 已在本设备上停用。你选择的类别会保留。
push-scope = 类别选择适用于你的账号。设备权限在每个浏览器上各自独立。退出登录会断开此设备。
push-failed = 无法确认此更改。请重试。
push-denied = 通知已被屏蔽。你可以在浏览器或设备设置中允许通知，然后重试。
push-unsupported = 此浏览器在这里不提供 Web Push。在 iPhone 和 iPad 上，请把 Baylee 添加到主屏幕，并从那里打开。
push-unconfigured = 此服务器暂时还不支持后台通知。你的偏好设置已经可以保存。

register-closed = 注册目前已暂停。请稍后再试。
register-refused = 注册未被接受。请重试。
admin-registration = 注册
admin-registration-open = 开放：任何人在完成工作量证明后都可以创建账号。
admin-registration-closed = 关闭：在你重新开放之前，没有人能创建账号。
admin-registration-lead = 遭受攻击时关闭注册，之后手动重新开放。每次更改都会记录在审计日志中。
admin-registration-close = 关闭注册
admin-registration-reopen = 重新开放注册
hero-discord = Discord 社区
hero-whatsapp = WhatsApp 社区
hero-github = GitHub
nav-menu = 菜单
foot-project = 项目
foot-source = 源代码（AGPL-3.0）
foot-central = Baylee Central（源码可用）
foot-summary = 免费、开源的桌面卡牌游戏，使用 Rust 和 Bevy 构建——具备确定性的规则引擎、完整的离线模式、内置 AI、LLM 桥接，以及社区 Gateways。
strip-build = 构建
strip-ci = CI
strip-commit = 提交
strip-gateways = Gateways
strip-online = 在线
strip-offline = 离线
strip-releases = 查看版本
strip-on-github = 在 GitHub 查看
strip-directory = 目录
strip-cards = 可玩牌张
strip-cards-unit = 张牌
strip-stars = GitHub 星标
hero-badges = 一览
hero-badge-beta = 公开测试
hero-badge-free = 免费
hero-badge-open = 开源
hero-badge-fan = 非官方爱好者内容
gws-open-client = 在 Baylee 中打开
gws-open-client-help = “Open in Baylee”（在 Baylee 中打开）会启动已安装的客户端（0.1.1-beta.9b 或更新版本）。它会显示地址，并在添加该 Gateway 之前征求你的同意。
dl-menu-title = 所有系统
dl-old-cpu = 较旧的处理器（无 AVX2）
ci-passing = 通过
ci-failing = 失败
dl-button = 下载 Baylee
dl-quick = 其他系统和架构
dl-others = 其他系统

# The start page, as the old site wrote it (owner, 11.11.2026)
link-readmeModel = #playing-against-a-language-model
about-engineTitle = 强制执行每条规则的引擎
about-deterministic = 由带种子的随机数生成器驱动，不依赖任何挂钟时间或哈希顺序，每一局对局都是严格确定的，可以逐步重现。
about-tableTitle = 用 Bevy 构建的清爽 3D 牌桌
about-keys = 在内置大厅中，你可以轻松地在本地套牌构筑器、单人对局和社区 Gateways 上的多人牌桌之间切换。
feature-hiddenTitle = 隐藏信息保持私密
feature-hidden = 你的客户端只会收到你的座位被允许看到的数据。牌库和对手的手牌只以数量的形式送达——即使是被修改过的客户端也无法查看隐藏的牌。
feature-seatsTitle = 一张牌桌最多八位玩家
feature-seats = 对决、混战以及组队对局（例如 2v2），最多 8 个座位。队友共享回合，并且可以互相展示手牌。
feature-commanderTitle = 灵活的赛制与自定义房规
feature-commander = Assemble decks for classic 60‑card duels, multiplayer rings, or custom house formats. Starting life totals, team layouts, and mulligan rules are freely set by the host.
feature-aiTitle = 确定性回放与 F8 报告
feature-ai = 每一局对局都逐步记录。如果你在游玩时发现规则交互方面的问题，就在牌桌上按 F8，发送一份可重现的状态报告。
feature-processTitle = 每局对局一个独立的服务器进程
feature-process = 每一局进行中的游戏都在 Gateway 上自己的沙盒引擎进程里运行。Gateway 本身只管理大厅和房间——出错绝不会影响其他牌桌。
feature-languagesTitle = 19 种语言的牌张文字
feature-languages = 只要 Gateway 提供多语言牌张目录，牌张就会以你偏好的语言显示，并以英语作为后备。客户端界面提供德语和英语。
feature-watchTitle = 观看实时对局
feature-watch = 大厅里的“观战”按钮会让你以实时观众的身份坐到牌桌旁：实时观看公开的战场，而所有玩家的手牌始终保持隐藏。
feature-connectionTitle = 断线后无缝重连
feature-connection = 如果连接短暂中断，牌桌会以倒计时为你保留座位，并可以暂停对局，或让内置 AI 暂时代打。
feature-tablesTitle = 十四种决策计时
feature-tables = From brisk 15‑second turns to relaxed untimed tables, configure the pace individually for every room.
shots-eyebrow = 在牌桌上
shots-title = 游戏内部
shots-lead = Short live recordings captured directly from the 3D client with card images from Scryfall (© Wizards of the Coast)—from duels and 4‑player rings to the encrypted lobby and deck builder.
shots-credit = 牌张图像：Scryfall · © Wizards of the Coast
shots-showLarge = 放大显示：{ $title }
shots-dialog = 截图
shots-previous = 上一个
shots-next = 下一个
shots-close = 关闭
ai-eyebrow = AI 对手与语言模型
ai-title = 内置 AI、LLM 桥接与 Baylee 原生 AI
ai-houseTitle = 五个技能等级的启发式内置 AI
ai-house = 离线和任何在线牌桌上都可直接使用：可在新手、休闲、稳健、犀利和专家之间选择。内置 AI 在本地决策，毫无等待时间，并且只能看到与真人玩家相同的公开状态。
ai-modelTitle = 通过 Seat-Bridge 让语言模型坐上牌桌
ai-model = 让 <b>Google Gemini、OpenAI ChatGPT、Anthropic Claude、DeepSeek</b>，或通过 <b>LM Studio</b> 运行的本地模型直接坐到你的牌桌上。API 密钥保存在带有消费上限的环境变量中，受到保护；一旦超时，内置 AI 会无缝接手。
ai-setup = LLM 与 Seat-Bridge 设置指南
home-gwLead = 在线牌桌位于 Gateways 上，这些服务器在管理员批准之后由 Baylee Central 列出。这里是最前面的几个；目录里有全部。
beta-open = 打开目录
beta-own = 自己运行一个
source-eyebrow = 源代码
source-title = 开源，并且会一直开源
source-promise = Baylee 将永远开源、永远免费。没有付费墙，没有付费功能，没有广告。现在如此，将来也如此。代码采用 GNU Affero 通用公共许可证，这意味着任何为他人运行修改版 Gateway 的人，也必须向他们提供其源代码。
source-rust = 它用 Rust 编写，是一个人的项目，并得到编码智能体的大量帮助，每一次更改都要通过同一道关卡：格式化、lint、测试套件，以及对照印刷文字对每张牌的验证。欢迎提交问题和拉取请求；<agents>AGENTS.md</agents> 和 <claude>CLAUDE.md</claude> 是针对人和智能体的工作规则。
source-discord = Discord 上的社区
source-whatsapp = WhatsApp 上的社区
source-architecture = 架构
source-repoTitle = 仓库的当前状态
source-stars = 星标
source-issues = 未解决的问题和拉取请求
source-note = 由本站的服务器每十分钟从 GitHub 的公开 API 读取。你的浏览器不会向 GitHub 请求任何内容。
gw-count =
    { $n ->
       *[other] 已列出 { $n } 个 Gateway，其中 { $online } 个在线。
    }
gw-none = 目前还没有列出任何 Gateway。
contact-intro = 一个问题、一个想法、一款你希望看到的游戏，或者你不想公开发布的事：这份表单会直接送到 Baylee 背后的那个人手里。如果想要回复，请留下电子邮件地址；没有的话，我们只能阅读。
contact-f8 = 结算错误的牌，最好在游戏中用 <kbd>F8</kbd> 报告，因为报告可以包含整局游戏。
contact-discord = 聊天、找对手、了解新构建
contact-issues = GitHub 问题追踪
contact-issuesSub = 错误和想法，公开讨论
contact-operator = 谁在运营本站，附邮寄地址：<impressum>法律声明</impressum>。
contact-whatsappSub = 社区群组与快速对局
contact-reportsTitle = 报告与隐私
contact-reportsSub = F8 报告和数据的详情
ai-housePill = 离线和在线
ai-providers = 支持的语言模型
ai-hostsPill = 面向 Gateway 主机
ai-hostsTitle = 在你的 Gateway 上提供托管的 LLM 座位
ai-hosts = 作为 Gateway 运营者，你可以直接在自己的服务器上提供预先配置好的语言模型。牌桌上的访客只需选择你托管的某个模型作为对手——无需 API 密钥。
ai-trainingPill = 训练中 · 原生引擎 AI
ai-trainingTitle = Baylee 原生 AI——训练它像真人玩家一样直接在引擎上对局
ai-training = 除了内置 AI 和外部 LLM 桥接之外，一个专门的 Baylee 原生 AI 正在直接基于确定性规则引擎训练。它学习像经验丰富的真人玩家那样评估局面、优先权窗口、战斗计算和套牌协同——没有任何外部 API 延迟。
ai-hostingLink = Gateway 托管与 LLM 设置

# The gateways and the contact page, as the old site wrote them (owner, 11.11.2026)
gw-lead = Gateway 是在线牌桌所在的服务器：包括账号、套牌、房间，以及每局游戏一个引擎进程。这里列出的是在 Baylee Central 注册并获得其管理员批准的那些。标有“官方”的由 Baylee 运营；其他的由别人为自己的群体运营，适用他们自己的规则。被列出并不等于对 Gateway 的行为作出承诺。
gw-howTo = 要在某个 Gateway 上游玩，请把它的地址复制到客户端的入口处。在线是指 Central 在最近一分半钟内收到过该 Gateway 的消息；这并不保证它会响应。名称和版本由各运营者自行报告。
gw-find = 查找 Gateway
gw-findHint = 名称或地址
gw-onlineOnly = 仅在线
gw-shown = 显示 { $n } / { $all }
gw-noMatch = 到目前为止加载的 Gateway 中没有匹配项。
gw-showAll = 显示全部
link-readmeGateway = #running-your-own-gateway
selfhost-eyebrow = 自托管与 Central 集成
selfhost-title = 运行你自己的 Gateway
selfhost-processes = 自托管的 Gateway 在 PostgreSQL 18+ 之外运行三个轻量级进程：<code>baylee-gateway</code> 管理房间、套牌和路由；<code>baylee-agent</code> 为每局对局启动一个独立的 <code>baylee-engine-server</code> 进程，使每张牌桌都在沙盒中运行。
selfhost-proxy = 在你的 HTTPS 源前面放一个 TLS 反向代理（例如 <code>https://play.example.org</code>），还可以选择导入 Scryfall 的批量目录以获得 19 种语言的牌张文字，这样玩家就可以在客户端中通过你的地址直接连接。
selfhost-central = <b>与 Baylee Central 关联并配置隐私：</b>在<mine>你的 Gateways</mine>下注册你的 HTTPS 源，并生成一个 <code>bgt_…</code> 令牌（范围：<code>sso</code> 用于单点登录，<code>listing</code> 用于目录心跳和实时遥测）。在你的 Gateway 控制台中选择遥测隐私级别（<code>full</code> 包含进行中的游戏、牌桌设置和已选择加入的玩家，<code>counts_only</code> 只有总数，<code>private</code> 只有存活状态）。
selfhost-guide = <guide>阅读完整的自托管指南</guide> · <protocol>协议与心跳参考（docs/protocol.md）</protocol>
selfhost-source = 每个 Gateway 无需登录即可通过 <code>GET /source</code> 返回它的许可证、版本和提交，自动满足 AGPL 的源代码披露要求。
reports-eyebrow = 报告
reports-title = 发现了问题？告诉我们
reports-f8 = 在牌桌上或大厅里按 <kbd>F8</kbd>。报告包含它的种类、你的文字、客户端版本，以及在牌桌上时该局游戏的 ID。其余每一项都有各自的复选框，而且所有复选框一开始都是勾选的，所以在按“Send”（发送）之前，请取消勾选你不想提供的内容：系统和硬件、你所看到的牌桌、你的日志（其他玩家被改名为“Player A”“Player B”……）、你的设置、仅含 Baylee 窗口的一张图片，以及在牌桌上时这局游戏的完整记录（每一步操作和每个座位的牌，不含姓名）。<i>Show what is sent</i>（显示将发送的内容）会显示每一个字节。会话令牌绝不会发送。
reports-usage = 你离线游玩时，客户端每分钟说一次有一局游戏正在进行：没有账号，没有设备 ID，没有关于这局游戏的任何信息。在 Settings › Privacy & Data 下的 <i>Send anonymous usage count</i> 可以将其关闭。
reports-discord = 想聊天、找对手，或第一时间了解新构建？加入 <discord>Discord 上的 Baylee 社区</discord>。
reports-issues = 规则问题、牌的表现不对、想法：<issues>在 GitHub 上提交问题</issues>。结算错误的牌是我们能收到的最有用的报告，因为每张牌都有一个测试，而你的报告会变成其中一个。
reports-privacy = <docs>docs/privacy.md</docs> 列出了客户端、Gateway 和反馈服务保存的全部内容。除非你登录，本站不设置 Cookie，不做分析，也不从别人的服务器加载字体或脚本。<notice>隐私声明</notice>说明了本站和 Baylee Central 保存哪些内容，并列出它们连接的所有其他主机以及连接的时间。

# The download page, as the old site set it (owner, 11.11.2026)
dlPage-eyebrow = 获取
dlPage-filesEyebrow = 所有文件
dlPage-filesTitle = 所有系统
install-eyebrow = 安装
install-releases = GitHub 上的所有版本
release-notes = 发行说明和校验和
release-all = 所有版本

# The download page's head, as the old site wrote it
dlPage-title = 下载
dlPage-lead = 每个下载都直接指向 GitHub 上的文件，中间没有任何环节。有安装程序的版本，用安装程序最省事；压缩包里是同一个游戏，可以解压到任何位置。已安装的 Baylee 会在它能写入自己文件夹的地方，根据已签名的压缩包自行更新。每个软件包里，游戏旁边都带有用于语言模型的 Seat-Bridge。

# The reading pages' eyebrows, as the old site set them
imp-eyebrow = 法律声明
terms-eyebrow = 使用条款
privacy-eyebrow = 隐私

# The privacy notice's contents, as the old site named them
privacy-contents = 目录

# The account pages' eyebrow, as the old site set it
acct-eyebrow = Baylee Central

# The account area's navigation, as the old site named it
app-account = 你的账号

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
sec-teaser-more = See each step in a diagram
# ==== end of the security page =================================================
