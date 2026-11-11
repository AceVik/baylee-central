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

hero-title = <nb>构筑你的套牌。</nb> <nb>召集你的朋友。</nb> <glow>尽情游玩。</glow>
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
plat-windows-x64-sub = 大多数电脑：64位 Intel 或 AMD
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
gws-now = 此刻
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
impressum-write = 写电子邮件
impressum-call = 拨打电话
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
privacy-directory = 我们的服务器从 Baylee Central 读取 Gateway 目录并加以显示；这不涉及关于你的任何信息。Gateway 可以告诉 Central 它此刻有多少玩家、开放牌桌和对局，Central 也会统计其账号数量；本站会显示这些数字。它们只是数量，绝不是名字，而 Gateway 的数字是其运营者自己的陈述。
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
privacy-impressum = 法律声明只有在你的浏览器解开一道谜题之后才会显示；这样可以让地址免受收集地址的程序侵扰。谜题由随机数字组成，不含任何关于你的信息。为此，反馈服务只在内存中保存以下内容，绝不写入磁盘，也绝不写入日志：你的 IP 地址请求了多少道谜题（保存一小时；IPv6 地址只取前半部分）、它是否访问了对人类隐藏的链接（保存 24 小时），以及哪些谜题已被解开（保存五分钟，不含地址）。关于你的浏览器的三个“是或否”的判断会决定谜题的难度，且不会被保存：是否有程序在驱动它、它是否没有声明任何语言，以及它是否在没有显卡的情况下绘图。没有人会被拒绝；看起来像程序的浏览器会得到更长的谜题。法律依据：GDPR 第 6 条第 1 款 (f) 项（我们保护私人地址的合法利益）。
privacy-recipients-title = 接收方和国家
privacy-recipients = 我们不出售数据，也不为广告而向他人提供数据。接收方有：netcup GmbH（德国的托管服务，依据 GDPR 第 28 条作为处理者）；Baylee Central 的管理员，他们为管理之需会看到你的登录名、你的显示名及其标签、你账号的来源、创建时间和上次登录时间，以及你的 Gateways，但绝不会看到你的加密数据；你用账号登录的 Gateway（它会收到你的账号 ID、显示名和标签，不会收到你的密码）；你的联系人和你所在群组的成员（显示名、标签和你的消息）；仅在法律要求我们提供时才提供给主管机关。欧洲经济区以外的国家：只有当你开启通知时，你浏览器的推送服务（Google、Mozilla 或 Apple，美国）才会承载通知。他人运营的 Gateways 可能位于任何国家；是否在那里登录由你决定。除此之外，我们不向欧洲经济区以外的国家传输任何数据。
privacy-rights-title = 你的权利
privacy-rights = 你享有访问权（GDPR 第 15 条）、更正权（第 16 条）、删除权（第 17 条）、限制处理权（第 18 条）和数据可携带权（第 20 条）。对于基于我们的合法利益、依据第 6 条第 1 款 (f) 项进行的处理，你可以随时基于与你的具体情况有关的理由提出反对（第 21 条）。你可以随时撤回同意，对今后生效（第 7 条第 3 款）。你账号的数据可以在你账号的设置中导出，账号也可以在那里删除；其他事项，通过联系表单或发送到法律声明中所列的地址留言即可。你可以向数据保护监管机构投诉（第 77 条），例如你居住地的监管机构，或北莱茵-威斯特法伦州数据保护与信息自由事务专员。没有任何法律或合同要求你向我们提供数据；但是，没有登录名和密码，就没有账号。我们不进行自动化决策，也不进行用户画像（第 22 条）。
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
strip-accounts = Baylee Central 上的账号
strip-live = 各 Gateway 此刻的上报数据
live-accounts = { $n ->
   *[other] { $n } 个账号
}
live-players = { $n ->
   *[other] { $n } 位玩家
}
live-tables = { $n ->
   *[other] { $n } 张开放牌桌
}
live-games = { $n ->
   *[other] { $n } 局对局
}
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
feature-commander = 可以构筑经典 60张牌对决、多人混战或自定义房规赛制的套牌。初始生命值、队伍布局和调度（mulligan）规则都由主持者自由设定。
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
feature-tables = 从节奏明快的 15秒回合到不限时的轻松牌桌，每个房间的节奏都可以单独设置。
shots-eyebrow = 在牌桌上
shots-title = 游戏内部
shots-lead = 直接从 3D 客户端录制的简短实机录像，牌张图像来自 Scryfall（© Wizards of the Coast）——从对决和 4人混战，到加密大厅和套牌构筑器。
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
title-security = Baylee 的安全：密码、密钥和聊天
desc-security = Baylee 如何处理密码、密钥和聊天消息：用四张图逐步说明，并标明每个流程的局限。
nav-security = 安全
sec-eyebrow = 安全
sec-title = 密码、密钥和聊天：什么去了哪里
sec-lead = 四个流程，逐步说明：什么离开了你的设备，Baylee Central 能看到什么、看不到什么，以及每个流程的局限。
sec-toc-label = 本页内容
sec-figure = 分步图示：{ $name }
sec-play = 播放
sec-all = 全部步骤
sec-step = 第 { $n } 步
sec-hint = 图示会自动播放。选择一个步骤，即可停在那里。
sec-hint-still = 选择一个步骤，即可高亮显示它。
sec-in-browser = 虚线框是你的浏览器：这一切都在其中发生。
# The actors' names are the only words inside the drawings, where text does
# not wrap: one or two words each, at most about twice the English width
# (the test `names_fit_with_room_to_grow` measures every language).
sec-actor-device = 你的设备
sec-actor-central = Central
sec-actor-page = 页面
sec-actor-worker = Worker
sec-actor-store = 存储
sec-actor-you = 你
sec-actor-contact = 联系人
sec-actor-website = 网站
sec-actor-client = 客户端

sec-signin-eyebrow = 登录与注册
sec-signin-title = Central 在不接收密码的情况下检查你的密码
sec-signin-lead = Baylee 用 OPAQUE（RFC 9807）让你登录。你的设备和 Central 交换三条消息；你的密码不在其中任何一条里，连普通哈希也不在。
sec-signin-c1 = 盲化请求
sec-signin-t1 = 你的设备对密码做盲化。
sec-signin-s1 = 它把密码与一个新的随机数结合，只发送结果。从这条消息中无法读出密码。
sec-signin-c2 = Central 的回应
sec-signin-t2 = Central 用自己的密钥作答。
sec-signin-s2 = Central 用只有它持有的密钥对盲化后的值进行计算，并加上它那一半密钥交换。这样做时，它不会得知密码。
sec-signin-c3 = Argon2id · 64 MiB
sec-signin-t3 = 繁重的计算由你的设备完成。
sec-signin-s3 = 它去除盲化，并用 Argon2id（64 MiB 内存，三轮）对结果做拉伸；Central 无法要求更低的参数。密码错误会在这里被发现，在发送其他任何内容之前。
sec-signin-c4 = 证明
sec-signin-t4 = 你的设备证明它拥有正确的密钥。
sec-signin-s4 = 密码正确时，你的设备会得到与 Central 相同的会话密钥，并发送一份简短的证明（64 字节）。
sec-signin-c5 = 会话 + 封装件
sec-signin-t5 = Central 开启会话。
sec-signin-s5 = 返回的是会话（在浏览器中是页面自身无法读取的 Cookie），以及放在封装件中的主密钥；这个封装件只能在你的设备上、凭正确的密码打开。
sec-signin-register = <b>注册</b>使用同一套协议。你的设备随机生成一个主密钥（256 位），分别用你的密码和你的恢复码各封装一次，只发送这两个封装件。Central 存储一条 OPAQUE 记录和这些封装件，不存储密钥本身。
sec-signin-legacy = <b>一个例外：</b>尚未迁移到 Central 的旧官方 Gateway 的账号。在这样的账号迁移之前，Baylee 客户端中的每次登录尝试，都会通过经过验证的 TLS 连接，把输入的密码发送给 Central，由 Central 对照旧的哈希进行检查，既不存储也不记录它。迁移成功后，该账号按上文所示登录。本网站不会让这样的账号登录；它会指向客户端。这个例外在为迁移设定的期限届满时结束。
sec-signin-limit = <b>仍然存在的是：</b>谁拿到 Central 的数据库及其密钥，就可以逐个尝试密码；每次尝试都要花费一次 Argon2id 运算。以这种方式找到的密码，会打开该账号的加密数据及其已存储的聊天。一个很长、且你没有在其他任何地方使用过的密码，依然很重要。

sec-memory-eyebrow = 内存中
sec-memory-title = 登录之后内存中还留着什么
sec-memory-lead = 密码只需要一次：用来打开你的主密钥。图示跟随它在浏览器中的去向；Baylee 客户端的不同之处写在下面。
sec-memory-c1 = 密码
sec-memory-t1 = 页面把密码交给 Worker。
sec-memory-s1 = 表单把它传给一个专属的后台线程（Worker），清空输入框，并释放自己的那份副本。
sec-memory-c2 = OPAQUE + Argon2id
sec-memory-t2 = Worker 计算，然后覆写。
sec-memory-s2 = 它的 Rust 代码只在 OPAQUE 步骤结束之前持有密码，随后将其覆写，同时覆写途中派生出的、由它自己持有的密钥，以及 Argon2id 的 64 MiB 工作内存。
sec-memory-c3 = 主密钥：校验后擦除
sec-memory-t3 = 主密钥被打开、使用，然后擦除。
sec-memory-s3 = Worker 打开它以检查登录；如果你的账号有聊天，就从中派生出聊天密钥；然后再次将其覆写。
sec-memory-c4 = 聊天密钥，已封存
sec-memory-t4 = 只有聊天密钥留下来。
sec-memory-s4 = 为了让聊天在重新加载后无需你的密码就能打开，Worker 把聊天密钥用 AES-256-GCM 加密后存入浏览器（IndexedDB）。用于此加密的密钥是不可导出的：脚本可以使用它，但无法把它读出来。聊天页面打开期间，它自己的一个 Worker 会把未封存的聊天密钥保存在内存中。退出登录会删除这条记录；会话自行结束之后，本站下一个加载的页面会删除它。
sec-memory-c5 = Worker 已结束
sec-memory-t5 = 登录用的 Worker 被结束。
sec-memory-s5 = 登录完成时，登录页面会结束它的 Worker，浏览器随之丢弃该 Worker 的内存。
sec-memory-limit = <b>浏览器不作承诺的部分：</b>输入时的密码、它交给 Worker 的过程，以及 JavaScript 的字符串，都由浏览器管理。Baylee 会释放它们；它不会覆写页面自己的副本，也无法覆写浏览器的副本。这块内存何时真正被重新使用，由浏览器决定。而且，只要有人在你登录期间能在本站运行脚本，或能读取你的浏览器配置文件，就能接触到封存的聊天密钥。
sec-memory-client = <b>在 Baylee 客户端中</b>，输入的密码位于释放时会被覆写的内存中，并在登录后立即丢弃；它从不被存储。主密钥在你登录期间保留在客户端的内存中，因为客户端要用它打开你的加密数据。

sec-chat-eyebrow = 聊天
sec-chat-title = 消息在你的设备上加密
sec-chat-lead = 每条消息都有自己的密钥。Central 只存储和转发加密后的内容；要读取它，需要某个被封装对象账号的聊天密钥。
sec-chat-c1 = 文本 + 一次性密钥
sec-chat-t1 = 每条消息一个新密钥。
sec-chat-s1 = 你的设备随机生成一个消息密钥，并用它加密文本（XChaCha20-Poly1305）。
sec-chat-c2 = 按聊天密钥封装，已签名
sec-chat-t2 = 为每位收件人的聊天密钥分别封装，并用你的聊天密钥签名。
sec-chat-s2 = 它为收件人账号登记的每一个聊天密钥分别封装这个消息密钥（X25519），并用你自己的聊天密钥对消息头和密文签名（Ed25519）。目前一个账号只有一个聊天密钥，它由主密钥派生而来，在该账号登录的每个浏览器和客户端中都相同。
sec-chat-c3 = 密文 + 封装
sec-chat-t3 = Central 只收到密文。
sec-chat-s3 = Central 检查签名和收件人列表，并把密文连同这些封装一起存储。它既没有消息密钥，也没有你账号的密钥。
sec-chat-c4 = 密文 + 一份封装
sec-chat-t4 = 每个聊天密钥有自己的封装。
sec-chat-s4 = 当你的联系人取回消息时，Central 交出密文，以及只为该聊天密钥制作的那一份封装。
sec-chat-c5 = 已校验，已打开
sec-chat-t5 = 校验、解封、阅读。
sec-chat-s5 = 你的联系人的设备校验你的签名，用它的聊天密钥解开消息密钥的封装，并解密文本。
sec-chat-meta = <b>Central 仍然能看到的：</b>谁与谁有联系、带名称和成员的群组、显示名、公开的聊天密钥，以及每条消息的发送者、所在对话、发送时间和大小。和任何服务器一样，它也会看到请求来自哪个地址。
sec-chat-limit = <b>局限：</b>没有双棘轮，因此没有前向保密：如果某位收件人的聊天密钥日后被窃取，它也能打开之前为它封装的消息。你的聊天密钥由你账号的主密钥派生而来，并且 Central 在投递之后仍保留加密的消息：谁拿到 Central 的数据库及其密钥并猜出你的密码，或拥有你的恢复码，就可以读取它们。初次联系时，你的设备信任 Central 给出的密钥；要核对它们，请通过另一个渠道比对安全码。如果联系人的密钥之后发生变化，只有在你确认之后，Baylee 才会再次发送。群组由谁组成，由 Central 决定。

sec-pair-eyebrow = 通过链接登录
sec-pair-title = 从浏览器进入客户端，无需再次输入密码
sec-pair-lead = 在网站上已登录的人，应当能够以已登录状态启动 Baylee 客户端。为此，页面把主密钥送到客户端：经过封存，并且 Central 既无法打开它，也无法塞进它自己的一份。
sec-pair-status = <b>目前的状态：</b>Central 和 Baylee 的客户端库如今已能做到这一点，并且已相互测试。它还不能在本网站或已发布的客户端中使用。
sec-pair-c1 = 开始配对
sec-pair-t1 = 页面发起一次配对。
sec-pair-s1 = Central 为此签发一个有效期五分钟的句柄。同时，页面生成一个一次性秘密，并且从不向 Central 显示它。
sec-pair-c2 = 带秘密的链接
sec-pair-t2 = 一个链接打开客户端。
sec-pair-s2 = 链接 <code>baylee://pair</code> 绕过 Central，在你自己的设备上把句柄和秘密从页面带到客户端。
sec-pair-c3 = 公钥
sec-pair-t3 = 客户端以一对新的密钥作答。
sec-pair-s3 = 它把公钥和一个用该秘密算出的校验值（HMAC-SHA-256）发给 Central。页面验证这个值；两个屏幕上显示同一个简短的代码，由你在页面上批准。
sec-pair-c4 = 主密钥，已封存
sec-pair-t4 = 页面为客户端封存主密钥。
sec-pair-s4 = 使用 HPKE（RFC 9180），面向客户端的公钥，并且同样以该秘密为条件。Central 存储这份封存件一次；它无法打开它，并且因为不知道该秘密，也无法制作出自己的一份。
sec-pair-c5 = 会话 + 封存件
sec-pair-t5 = 客户端兑换一次。
sec-pair-s5 = 它收到一个会话和这份封存件，用自己的私钥和该秘密打开封存件，随即完成登录。此后该配对会被删除，兑换失败时也一样。
sec-pair-note = <b>密码不参与这次交换。</b>页面只在刚刚用你的密码打开主密钥的那一刻持有它。以这种方式配对的会话，其时效不比它所来自页面上的密码登录更新：配对从不会为它续期。

sec-limits-eyebrow = 局限
sec-limits-title = 这一切做不到的事
sec-limit-device = <b>被入侵的设备。</b>恶意软件、会读取页面的浏览器扩展，或被篡改的客户端，都能看到你所看到和输入的内容。通往服务器途中的任何加密，都无助于应对这种情况。
sec-limit-site = <b>网站的代码来自它的服务器。</b>谁接管了我们的服务器，就可能下发在你输入密码时捕获它的代码；完整性哈希和严格的 Content Security Policy 防范的是来自外部的改动，而不是这种情况。客户端是你机器上的程序，安装更新之前会检查签名。
sec-limit-central = <b>被接管的 Central。</b>它可能泄露元数据、拒绝服务并攻击登录。它可能在初次联系时给出虚假的密钥，或向群组添加成员，从而读到此后发送的消息。它所存储的你的数据和聊天都是加密的；要打开它们，需要你的密码或你的恢复码。
sec-limit-recovery = <b>丢失密钥。</b>没有你的密码，也没有你的恢复码，任何人都无法找回你的加密数据，我们也不能：Central 只有这些封装件。
sec-limit-audit = <b>没有独立审计。</b>登录遵循 OPAQUE（RFC 9807）与 Argon2id（RFC 9106），配对使用 HPKE（RFC 9180）封存：它们是 IRTF 发布的规范，不是 IETF 标准。配对各步骤的顺序和聊天格式，是 Baylee 自己对标准构件（Ed25519、X25519、XChaCha20-Poly1305）的组合。到目前为止，只有构建它的人审查过这套实现。代码是公开的；如果你发现了问题，<contact>请告诉我们</contact>。
sec-docs-title = 延伸阅读
sec-docs = 详情见项目文档：<protocol>密码学协议</protocol>、<chat>聊天</chat>、<accounts>账号与配对</accounts>和<threats>威胁模型</threats>。你的数据在法律上如何处理，见<privacy>隐私声明</privacy>。

# The page's two links from elsewhere: the FAQ and the sign-in page.
faq-security-q = Baylee 如何处理我的密码和聊天？
faq-security-a = 登录时，你的设备通过 OPAQUE 协议证明它知道密码，而无需发送密码；聊天消息在你的设备上加密。<security>安全页面</security>用图示展示了每一步，包括那个例外和各项局限。
sec-signin-link = 登录时你的密码会怎样：<security>安全</security>
sec-teaser-lead = 逐步说明：什么离开了你的设备，Baylee Central 能看到什么、看不到什么，以及每个流程的局限。
sec-teaser-more = 在图示中查看每一步
# ==== end of the security page =================================================
# ==== registration: the proof of work, while it takes long (English until translated) ====
register-solving = Your browser is solving a puzzle that keeps robots out. When many accounts are being created, this can take a few minutes … { $percent } %
