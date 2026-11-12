# baylee.app's words in English, Project Fluent's format. English decides
# which messages exist and which arguments each takes; build.rs turns them
# into typed functions (crates/central-web/build.rs). The reader is "you";
# a button says what it does.

## The frame of every page

site-name = Baylee
skip = 跳到內容
# Read out after a link that leaves the site (the stylesheet carries it).
new-tab = 會在新分頁開啟
nav-label = 主選單
nav-download = 下載
nav-gateways = Gateways
nav-faq = 常見問題
nav-contact = 聯絡
nav-sign-in = 登入
nav-account = 你的帳號
lang-label = 語言
theme-label = 色彩配置
theme-light = 色彩：淺色
theme-dark = 色彩：深色

foot-label = 關於本站
foot-play = 遊玩
foot-legal = 法律資訊
foot-releases = 所有版本
foot-issues = 回報錯誤
foot-discord = Discord 社群
foot-central-source = 本站原始碼
foot-dedication = 獻給 Baylee。永遠在陽光裡。
fan-content = Baylee屬於非官方的同好內容，並在<policy>同好內容政策</policy>的允許範圍內。未經威世智核准或背書。此內容的部分材料為威世智的財產。©威世智有限公司。
foot-colophon = 字型：Literata 與 Inter，採用 SIL Open Font License 1.1，由本站提供。畫作與標誌是我們自己的作品。<notices>聲明</notices>

## Titles and descriptions

title-home = Baylee – 組好牌組，找來朋友，開打就對了
desc-home = Baylee 是一款免費、開放原始碼的 Magic: The Gathering 同好遊戲，支援 Linux、Windows 與 macOS：有認得每張牌的規則引擎、最多八位玩家同桌，沒有其他人在時也有對手可玩。
title-download = 下載 Baylee
desc-download = 下載適用於 Windows、macOS 與 Linux 的 Baylee 用戶端：提供最新版本的安裝程式與壓縮檔，附校驗碼與簽章。
title-gateways = Gateways：在線上玩 Baylee 的地方
desc-gateways = Baylee Gateways 的公開目錄。Gateway 是線上牌桌所在的伺服器；這裡也說明如何架設自己的 Gateway。
title-faq = 常見問題與解答
desc-faq = Baylee 是什麼、要花多少錢、線上遊玩與帳號如何運作，以及你的資料會怎麼處理。
title-contact = 聯絡
desc-contact = 寫信給 Baylee 背後的那個人，或回報錯誤。
title-impressum = 法律聲明
desc-impressum = baylee.app 由誰營運（依德國《數位服務法》DDG 第 5 條）。
title-privacy = 隱私權聲明
desc-privacy = baylee.app 與 Baylee Central 會處理你的哪些資料、為什麼、依據什麼法律基礎，以及保存多久。
title-terms = 使用條款
desc-terms = Baylee Central（baylee.app 背後的帳號服務）的使用條款。
title-fan-content = 非官方同好內容
desc-fan-content = Baylee 是依據威世智同好內容政策製作的同好專案：這代表什麼，又不代表什麼。
title-not-found = 找不到頁面
title-gone = 此頁面已不存在
crumbs-label = 你的位置
breadcrumb-home = 首頁

## Home

hero-title = <nb>組好你的牌組。</nb> <nb>找來你的朋友。</nb> <glow>開玩就對了。</glow>
hero-lead = 功能完整的牌組建構器、最多八位玩家的對戰（雙人對決、混戰或團隊），以及會檢查每一步的規則引擎，讓每個回合都公平進行，不必有人去翻規則。
hero-offline = 單人對戰與牌組建構器可離線使用，不需要帳號。線上牌桌在 Gateways 上；<host>你也可以自己架設</host>。
hero-account = 想線上遊玩？<register>建立帳號</register>，公開測試期間免費，人人都可以申請；或<signin>登入</signin>。
hero-memorial = 懷念 Baylee：打開她的相簿

status-label = Baylee 數字一覽

cards-eyebrow = 卡牌
cards-title = Baylee 認得多少張牌
cards-lead = Baylee 一張一張實作 Magic 的牌，每一張都對照印刷的牌面文字檢查。以下是最新版本的數字。
cards-implemented = 完整實作
cards-implemented-help = 這張牌的所有規則都能運作，並且經過測試。
cards-generated = 自動產生
cards-generated-help = 由 Baylee 的產生器依牌面規則文字實作，但還沒有以人工測試。可以正常遊玩；在少數特殊情況下，行為可能不同。
cards-partial = 部分實作
cards-partial-help = 這張牌可以遊玩，但仍缺少其中一項異能，或規則上的某個特殊情況。
cards-total = 牌數總計
cards-total-help = Magic 的每一張牌，依 Scryfall 的列表；前面三個數字是 Baylee 能玩的部分。
cards-help = 這代表什麼
cards-share = 目前有 { $playable } 張牌可以遊玩，共 { $total } 張。
cards-as-of = 版本 { $tag }，於 { $date } 統計。
cards-missing = 暫時無法讀取卡牌數量。

shot-duel-alt = 與內建 AI 的對決：六隻生物攻擊，其中三隻被阻擋
shot-duel = 與內建 AI 的<b>戰術對決</b>，每一次分配都有清楚的戰鬥連線。
shot-log-alt = 開著遊戲紀錄的對決，堆疊上有三個咒語
shot-log = <b>遊戲紀錄</b>像聊天一樣記下每一步；點牌名可開啟預覽。
shot-ring-alt = 圍成一圈的四人牌桌
shot-ring = <b>最多八個座位的牌桌</b>會自行圍繞戰場排列。
shot-lobby-alt = 大廳，裡面有朋友的訊息與她的牌桌邀請
shot-lobby = <b>大廳：</b>開放的牌桌、朋友、邀請，以及端對端加密的聊天。
shot-room-alt = 一個房間，三張空位交給內建 AI
shot-room = <b>人人有位子：</b>真人、五種強度的內建 AI，或語言模型。
shot-builder-alt = 牌組建構器正在篩選牌池中的天使
shot-builder = <b>牌組建構器</b>在你輸入時搜尋整個牌池。

about-eyebrow = 它是什麼
about-title = 完整的規則引擎與清晰的 3D 牌桌
about-engine = 優先權、堆疊、分層、替代效應、觸發、狀態動作與戰鬥：這個 Rust 引擎套用完整的規則集，並在每一步計算出確切的合法選擇。用戶端只呈現這些有效的動作。
about-table = 戰場、手牌、堆疊、放逐區與墳場，都在整潔的 3D 牌桌上，有清楚的戰鬥連線與即時的牌面預覽。牌圖在你遊玩時從 Scryfall 串流載入，也可以隨時切換成清晰的規則文字檢視。

gw-eyebrow = 線上
gw-title = Gateways

open-central = GitHub 上的 Baylee Central

## Download

dl-prerelease = Beta
dl-for = 下載 { $system } 版
dl-size = { $size } MB
dl-checksum = SHA-256
dl-signature = 簽章
dl-notes = 版本說明
dl-unknown = 我們無法從這個瀏覽器判斷你的系統；請在下方自行選擇。
dl-mobile = 目前沒有手機與平板版本。Baylee 在電腦上執行。
dl-mac-unsure = 這個瀏覽器不會告訴我們你的 Mac 是 Apple 晶片還是 Intel 晶片。上方的檔案適用於 Apple 晶片（M1 及之後）；Intel Mac 的檔案在下方。
dl-windows-unsure = 如果你的 Windows 電腦使用 ARM 處理器（例如 Snapdragon 筆電），請選下方的 ARM 檔案。
dl-linux-unsure = 你的瀏覽器沒有說明處理器；我們先假設是 x86-64。ARM 檔案在下方。
dl-missing = 這個版本沒有適用於你系統的檔案。較舊的版本在<releases>版本頁面</releases>。
dl-none = GitHub 暫時沒有回應。所有版本都在<releases>版本頁面</releases>。
dl-yours = 你的系統
dl-more-files = 更多檔案（{ $count }）
dl-installer = 安裝程式
dl-deb = Debian 套件
dl-archive = 壓縮檔
dl-fallback = 適用較舊處理器的壓縮檔
dl-fallback-help = 適用於沒有 AVX2 的 x86-64 處理器（約 2013 年以前）。Baylee 的主要版本需要 AVX2，沒有的話無法啟動。
plat-windows-x64 = Windows
plat-windows-x64-sub = 多數電腦：64 位元 Intel 或 AMD
plat-windows-arm64 = Windows on ARM
plat-windows-arm64-sub = Snapdragon 筆電等
plat-mac-arm64 = macOS
plat-mac-arm64-sub = Apple 晶片：M1 及之後
plat-mac-x64 = macOS（Intel）
plat-mac-x64-sub = 使用 Intel 處理器的 Mac
plat-linux-x64 = Linux
plat-linux-x64-sub = x86-64
plat-linux-arm64 = Linux on ARM
plat-linux-arm64-sub = ARM64 / aarch64
plat-none = 這個版本沒有適用於此系統的檔案。
install-title = 首次啟動與系統需求
install-first-title = 首次啟動
install-first = 這些版本沒有程式碼簽署，因為簽署要花錢，所以 Windows 與 macOS 在第一次執行時會發出警告。在 Windows 上，選擇<i>更多資訊</i>，再選<i>仍要執行</i>。在 macOS 上，開啟<i>系統設定 → 隱私權與安全性</i>，選擇<i>仍要打開</i>。
install-check-title = 檢查下載的檔案
install-check = 每個檔案旁邊都有一個附校驗碼的 <code>.sha256</code>；壓縮檔還附有 <code>.sig</code>，那是用戶端在自我更新前會檢查的 Ed25519 簽章。在 Linux 上可用 <code>sha256sum -c</code> 檢查檔案，在 macOS 用 <code>shasum -a 256</code>，在 Windows 用 <code>certutil -hashfile</code>。
install-needs-title = 系統需求
install-windows = <b>Windows</b> 10 或 11，x64 或 ARM64。
install-mac = <b>macOS</b> 11 或更新版本，Apple 晶片或 Intel 皆可。
install-linux = <b>Linux</b> 需要 glibc 2.35 或更新版本（Ubuntu 22.04 及之後）與 Vulkan 驅動程式；<code>.deb</code> 會一併安裝其餘所需的東西。
install-cpu = <b>處理器：</b>x86-64 版本需要 AVX2；較舊的處理器另有單獨的壓縮檔。

## Gateways

gws-title = Gateways
gws-online = 在線
gws-offline = 離線
gws-version = 版本
gws-now = 目前
gws-website = 網站
gws-address = 位址
gws-copy = 複製位址
gws-copied = 已複製位址。
gws-copied-short = 已複製
gws-copy-failed = 複製失敗。
gws-failed = 暫時無法讀取目錄。請一分鐘後再試。

## Questions

faq-eyebrow = 問答
faq-title = 常見問題與解答
faq-what-q = Baylee 是什麼？
faq-what-a = 一款在你電腦上玩的 Magic: The Gathering 免費同好遊戲。它有牌組建構器、會檢查每一步的規則引擎、最多八位玩家同桌，還有電腦對手。它並非由威世智製作、核准或背書。
faq-cost-q = 要花多少錢？
faq-cost-a = 完全免費，而且以後也不會改變：沒有付費牆、沒有付費功能、沒有廣告。
faq-account-q = 我需要帳號嗎？
faq-account-a = 單人遊玩不需要：牌組建構器與對戰內建 AI 都可離線使用。要上線上牌桌則需要 Baylee 帳號；公開測試期間，任何人都可以在本站免費建立。
faq-data-q = 我的資料會怎麼處理？
faq-data-a = 盡可能越少越好；<privacy>隱私權聲明</privacy>寫得很清楚。你的密碼不會離開你的瀏覽器；你的私人資料與聊天訊息在送到我們這裡之前，就已在你的裝置上加密。
faq-password-q = 我忘記密碼了，怎麼辦？
faq-password-a = 新密碼要用重設碼來設定，就在<reset>這個頁面</reset>上。如果你的帳號有已確認的電子郵件地址，可以在那裡讓重設碼寄到該地址；否則由管理員給你一組。只有你持有建立帳號時拿到的救援碼，加密資料才保得住。沒有救援碼，你就從空保險庫開始，那些資料也就遺失了。
faq-gateway-q = 什麼是 Gateway？
faq-gateway-a = 線上牌桌所在的伺服器。任何人都可以架設；<gateways>目錄</gateways>列出 Central 知道的 Gateways。
faq-systems-q = 它能在哪些系統上執行？
faq-systems-a = Windows、macOS 與 Linux，支援 x86-64 與 ARM64。目前還沒有手機或平板版本。
faq-open-q = 它是開放原始碼嗎？
faq-open-a = 遊戲本身是，採用 GNU Affero General Public License。Baylee Central 的程式碼是公開的，讓任何人都能檢查它如何處理自己的資料。
faq-bug-q = 我發現了錯誤，要在哪裡回報？
faq-bug-a = 在遊戲中按 <kbd>F8</kbd>，就能送出附有所需一切資訊的報告。也可以到 GitHub 的 issue 追蹤器、Discord，或透過<contact>聯絡表單</contact>。

## Contact

contact-eyebrow = 聯絡
contact-title = 寫信給我們
contact-name = 你的名字
contact-name-hint = 選填。
contact-email = 你的電子郵件地址
contact-email-hint = 選填；只是讓我們能回覆。
contact-message = 你的訊息
contact-count = { $n } / 5,000 字元
contact-send = 送出訊息
contact-sending = 傳送中……
contact-sent-title = 謝謝你
contact-sent = 你的訊息已送達。如果你留了地址，我們會回覆到那裡。
contact-error-empty = 請先寫下訊息。
contact-error-email = 這看起來不像電子郵件地址。
contact-error-busy = 你的網路目前送出的訊息太多。請一小時後再試。
contact-error-refused = 訊息未被接受。請檢查長度後再試一次。
contact-error-network = 無法送出訊息。請檢查連線後再試一次。
contact-privacy = 你送出的內容會保存下來以便我們回覆，並在我們處理完畢 90 天後刪除，最遲在一年後刪除。詳情請見<privacy>隱私權聲明</privacy>。
contact-noscript = 這個表單需要 JavaScript。你也可以透過 Discord 或 GitHub 聯絡我們。

## Legal notice

impressum-title = 法律聲明
impressum-lead = 依 DDG 第 5 條（德國《數位服務法》）所需提供的資訊。Baylee 是個人、非商業的專案。
impressum-show = 顯示詳細資料
impressum-why = 詳細資料藏在一道簡短的謎題後面，你的瀏覽器會在幾秒內解開，用來擋掉蒐集地址的機器人。除此之外不會要求你做任何事。
impressum-solving = 正在解謎…… { $percent } %
impressum-name = 姓名
impressum-address = 地址
impressum-email = 電子郵件
impressum-phone = 電話
impressum-write = 寫電子郵件
impressum-call = 撥打電話
impressum-failed = 無法載入詳細資料。請再試一次，或透過<contact>聯絡表單</contact>寫信給我們。
impressum-noscript = 顯示詳細資料需要 JavaScript。沒有的話，請透過 Discord 或 GitHub 聯絡我們。
impressum-responsible = 內容負責人：上述所列之人。
impressum-dispute = 我們既無義務、也無意願參與消費者仲裁機構的爭議解決程序。

## Fan content

fc-title = 非官方同好內容
fc-eyebrow = 同好內容
fc-lead = Baylee 是同好專案。它在威世智的同好內容政策允許範圍內，並遵守該政策訂下的規則。
fc-notice-title = 每個頁面上的聲明
fc-means-title = 這代表什麼
fc-means = Baylee 並非由威世智製作、核准或背書。Magic: The Gathering、其牌名、系列名稱、牌面圖像與法術力符號，皆為威世智有限公司的財產。Baylee 對所有人免費：沒有付費牆、沒有付費功能，也不用以電子郵件註冊。
fc-images = 遊戲顯示來自 Scryfall 的牌面圖像，未經更動並標示畫家姓名；本站不顯示任何牌面圖像。卡牌資料與圖像由 Scryfall 提供；Baylee 與 Scryfall 沒有隸屬關係。
fc-policy = 威世智的<policy>同好內容政策</policy>

## Privacy notice

privacy-title = 隱私權聲明
privacy-lead = 本站與 Baylee Central 會處理你的哪些資料、為什麼、依據什麼法律基礎，以及保存多久。
privacy-updated = 截至 2026 年 11 月 12 日
privacy-controller-title = 誰負責
privacy-controller = GDPR（《一般資料保護規則》）所稱的控管者，是<impressum>法律聲明</impressum>中列名的自然人，該聲明載有其姓名、郵寄地址與電子郵件地址。你也可以透過聯絡表單送出隱私權方面的問題。我們沒有資料保護長；法律並不要求設置。
privacy-short-title = 簡單來說
privacy-short = 沒有追蹤、沒有廣告、沒有分析，除非你登入否則沒有 Cookie，也不從其他伺服器載入任何東西：字型、圖片與指令碼全都來自 baylee.app。你的密碼不會到達我們這裡，你的聊天訊息與私人資料在送出之前，就已在你的瀏覽器中加密。
privacy-hosting-title = 造訪本站
privacy-hosting = 本站執行在我們向德國 netcup GmbH 租用的伺服器上。你開啟頁面時，瀏覽器會送出每個請求都帶有的資訊：你的 IP 位址、所要求的位址、時間、瀏覽器的語言與其識別資訊（user agent）。伺服器需要這些資訊才能回應；它依語言挑選語言版本，下載頁面則用 user agent 推薦適合的檔案。網頁伺服器不保留存取紀錄，因此這些請求不會被儲存。伺服器本身的運作訊息（系統日誌）在少數錯誤情況下可能含有 IP 位址；這些訊息最遲在 14 天後刪除。法律基礎：GDPR 第 6 條第 1 項 (f) 款；我們的正當利益是安全、可靠地提供本站。
privacy-views-title = 統計頁面瀏覽次數
privacy-views = 本站會統計其頁面被開啟的次數：所有頁面、所有訪客合在一起只有一個數字，顯示在首頁上。在瀏覽器中開啟一個頁面，就會使它加一。為此不會儲存任何關於你的資料：沒有 IP 位址，沒有 Cookie，沒有識別碼，也不儲存是哪個頁面。
privacy-device-title = 你的瀏覽器保存了什麼
privacy-device = 在你登入之前，本站不設定任何 Cookie。你的瀏覽器會在你的裝置上保存以下內容，而且只有在啟用 JavaScript 時才會：你的色彩選擇（若你有選）（<code>baylee-site-theme</code>）；在你登入期間，你的工作階段何時結束（<code>baylee-central-session</code>）、你的聊天金鑰，以及聯絡人的受信任金鑰（見「聊天」）；若你開啟了通知，一組代表這台裝置的隨機數字（<code>baylee-push-installation-v1</code>、<code>baylee-push</code>）；以及，只有在你已登入、將本站安裝為應用程式或開啟通知之後，才會有一份本站公開檔案（頁面外框、字型、指令碼）的副本，讓本站可以在沒有連線時啟動，也讓通知能夠送達。如果這三種情況都不再成立，這份副本會在你下次開啟本站時移除。除非下方某一節另有說明，這些內容都不會傳送給我們。聊天與工作階段的項目會在你登出時刪除；其餘一切，你可以在瀏覽器中清除本站資料來移除。法律基礎：TDDDG（《電信數位服務資料保護法》）第 25 條第 2 項第 2 款（為你所使用的功能所絕對必要）；至於通知所用的隨機數字，則是你的同意（TDDDG 第 25 條第 1 項）。
privacy-device-manager = 如果你把救援碼存在瀏覽器的密碼管理員、用電子郵件寄給自己、存成檔案，或在顯示它的那一步列印出來，保存它的是你的瀏覽器、郵件程式、檔案或紙張，而不是本站：這組碼在你的瀏覽器中產生，絕不會傳送給我們。清除本站資料並不會把它從那些地方移除。
privacy-github-title = GitHub
privacy-github = 下載頁面與首頁會顯示最新版本、測試狀態與卡牌數量。是我們的伺服器向 GitHub 取得這些資料，不是你的瀏覽器；GitHub 不會因此得知任何關於你的事。只有在你點擊下載或 GitHub 連結時，你的瀏覽器才會連線到 GitHub（GitHub, Inc.，美國），並把你的 IP 位址與每個請求都帶有的資訊傳給它；之後適用 GitHub 自己的隱私權聲明。
privacy-directory-title = Gateway 目錄
privacy-directory = 我們的伺服器會從 Baylee Central 讀取 Gateways 的目錄並顯示出來；過程不涉及你的任何資料。Gateway 可以告訴 Central 它目前有多少玩家已連線、其中有多少人在遊戲中，以及有多少開放的牌桌和進行中的遊戲，Central 也會計算帳號數量；本站會顯示這些數字。它們只是數量，絕不含名字，而且 Gateway 的數字只是其營運者自己的說法。
privacy-account-title = 你的帳號
privacy-account = 為了建立帳號，Baylee Central 會儲存：你的帳號 ID、登入名稱、帶標籤的顯示名稱、帳號建立的時間與方式、一筆 OPAQUE 密碼紀錄（讓你能證明自己知道密碼，而我們的伺服器完全不會得知密碼）、經過兩次加密的主金鑰（分別由你的密碼與救援碼加密）、你加密的帳號文件與加密物件（我們無法讀取，只看得到大小、修訂版本與時間）、你接受的是哪一版條款以及何時接受、上次登入的時間、你的工作階段、管理員角色（如果你有的話），以及你註冊的 Gateways 與其權杖。其他玩家看到的是你的顯示名稱與標籤，不會看到你的登入名稱。如果管理員給你一組重設碼，我們只會儲存它的摘要，直到該重設碼被使用或過期為止。管理員對帳號或 Gateway 所做的操作，會記錄保存三年（誰、做了什麼、哪個帳號或 Gateway、何時）。法律基礎：GDPR 第 6 條第 1 項 (b) 款（你要求建立的帳號）；至於上次登入的時間與該紀錄，則是 GDPR 第 6 條第 1 項 (f) 款（我們在安全、可追溯的營運上的正當利益）。沒有登入名稱和密碼就沒有帳號；你不需要電子郵件地址。年滿 16 歲才可以建立帳號。資料保存到你刪除帳號為止；刪除後，會立即從運作中的系統移除，並最遲在 12 個月內從備份中移除。工作階段在閒置 30 分鐘後結束，最遲在登入後 12 小時結束。
privacy-email-title = 電子郵件地址
privacy-email = 電子郵件地址是選填的，帳號並不需要它。如果你提供了地址，Baylee Central 會儲存該地址、郵件所用的語言，以及你確認它的時間。你填寫後正在等待確認碼的地址，會保留到你確認或撤回為止，最晚在其確認碼過期一天後刪除。對於我們寄出的每個碼，我們只儲存其摘要，直到該碼被使用或過期；並在 24 小時內儲存寄出重設碼給你的時間，以確保每天最多寄出三封。我們把這個地址用於四件事：寄出用來確認它的確認碼；在你或任何知道你登入名稱的人要求時寄出重設碼；在地址被取代或移除時通知舊地址；在用重設碼設定了新密碼、密碼被變更或管理員為帳號產生了重設碼時，通知已確認的地址。郵件由 Baylee 在同一台伺服器上的自有郵件服務透過 mailbox.org（Heinlein Hosting GmbH，柏林）寄出，後者擔任受託處理者，接收該地址與郵件。法律基礎：GDPR 第 6 條第 1 項 (b) 款（你要求建立的帳號）。保存到你移除該地址或刪除帳號為止。
privacy-cookie-title = 工作階段 Cookie
privacy-cookie = 登入會設定一個 Cookie：<code>__Host-central</code>，是用來識別你的工作階段的一組隨機數字。它只會傳送給 baylee.app，指令碼無法讀取，並隨工作階段結束，最遲在 12 小時後結束。它是讓你保持登入所絕對必要的，不需要同意。法律基礎：TDDDG 第 25 條第 2 項第 2 款與 GDPR 第 6 條第 1 項 (b) 款。
privacy-chat-title = 聊天與聯絡人
privacy-chat = 訊息在你的裝置上為每位收件人的裝置分別加密；Baylee Central 只以加密形式儲存，無法讀取。Central 看得到並儲存的有：你的聯絡人、聯絡請求與封鎖、群組及其名稱、成員與邀請、誰在何時傳了訊息給誰以及訊息有多大，還有你各裝置的公開金鑰。你的聯絡人與你所在群組的成員會看到你帶標籤的顯示名稱與你的訊息。在你登入期間，你的瀏覽器會保存你的聊天金鑰（以只有這個瀏覽器持有的金鑰封存，放在其儲存空間 <code>baylee-chat</code>）以及你已信任的聯絡人金鑰（加密，<code>baylee-chat-trust</code>）。訊息與傳遞紀錄會保存到你刪除帳號為止；目前還沒有較短的自動保存期限。每位寄件者與每台裝置最多持有 5,000 則訊息；超過之後，新的訊息會被拒收。法律基礎：GDPR 第 6 條第 1 項 (b) 款。
privacy-push-title = 通知
privacy-push = 如果你在瀏覽器中開啟通知，你的瀏覽器會向其推播服務註冊，而 Central 會儲存該服務針對這個瀏覽器的位址、瀏覽器的公開金鑰與驗證密鑰、你的語言、一組代表這台裝置的隨機數字，以及你選擇的類別。推播服務屬於你瀏覽器的開發商：Google（Firebase Cloud Messaging）、Mozilla 或 Apple，皆設於美國。通知經加密後，只帶有它的種類（例如「新訊息」）與你的語言，絕不含姓名或內文；推播服務會看到位址、時間與大小等連線資料。當你在這台裝置上關閉通知、登出，或刪除帳號時，這些資料就會刪除；90 天未更新的位址不再使用，等待中的通知在一天後失效。法律基礎：GDPR 第 6 條第 1 項 (a) 款與 TDDDG 第 25 條第 1 項（你的同意）。你可以隨時在帳號設定中撤回同意；在此之前所做的處理仍屬合法。
privacy-counters-title = 防止濫用
privacy-counters = 為了防止猜測密碼和過載，Central 依 IP 位址統計請求（註冊、登入嘗試、密碼重設、讀取目錄，以及更一般地，一個位址一分鐘內送出多少請求），只在記憶體中進行：註冊和未以正確密碼完成的登入嘗試最長保留一天，其餘的最長保留一小時。以正確密碼登入後，Central 會在記憶體中記住 30 天（同樣只在記憶體中）你的帳號曾從該位址登入（一個 IPv4 位址，或 IPv6 位址的前半段；每個帳號最多八個），這樣別人對你帳號的嘗試就不會把你擋在那裡。使用同一位址且知道你登入名稱的人，可以從 Central 的回應看出你的帳號曾從該位址登入。設定新密碼會忘記這些位址，只保留你設定密碼時所用的那個；刪除帳號會全部忘記，重新啟動服務也一樣。註冊之前，你的瀏覽器還會解一道需要記憶體和運算時間的謎題；它由隨機數字構成，不含任何關於你的資料，最遲二十分鐘後即被遺忘。Central 會重新計算其中一小部分，以檢查你瀏覽器的答案。難度取決於正在建立多少個帳號，以及請求是否像來自程式：是否帶有每個瀏覽器都會送出的標頭，你的瀏覽器是否表明有程式在控制它，以及你的位址及其周圍的網段（IPv4 位址的前三個數字，IPv6 位址的前 48 位元）在過去一小時內要求了多少道謎題。對此，Central 只保留計數，存放在記憶體中，最長一小時。Central 依帳號統計聯絡請求和對其他玩家的搜尋，以免有人用請求淹沒別人。法律基礎：GDPR 第 6 條第 1 項 (f) 款；我們的正當利益是保護帳號和服務免遭濫用。
privacy-contact-title = 聯絡表單
privacy-contact = 表單會把你的訊息、頁面的語言和時間，送到同一台伺服器上 Baylee 的意見回饋服務；你的名字與電子郵件地址只有在你填寫時才會送出，兩者都是選填。該服務也會記下這則訊息是否已處理。你的 IP 位址不會儲存；它只在記憶體中計數一小時，用來阻止訊息氾濫。只有法律聲明中列名的人會讀這些訊息；不會轉交給任何人。訊息在處理完畢 90 天後刪除，最遲一年後刪除，或在你要求時提早刪除。法律基礎：GDPR 第 6 條第 1 項 (f) 款（我們回覆訊息的正當利益），或在你的訊息與你的帳號有關時，為 (b) 款。
privacy-impressum-title = 法律聲明的謎題
privacy-impressum = 法律聲明要在你的瀏覽器解開一道謎題之後才會顯示；這是為了保護地址不被蒐集地址的程式取得。謎題由隨機數字構成，不含任何關於你的資料。為此，回饋服務只會在記憶體中保存下列資料，絕不寫入磁碟，也絕不寫入日誌：你的 IP 位址要求了多少道謎題（保存一小時；IPv6 位址只取前半段）、它是否點了人看不到的隱藏連結（保存 24 小時），以及哪些謎題已被解開（保存五分鐘，不含位址）。關於你瀏覽器的三個「是或否」答案會決定謎題的難度，且不會被保存：是否有程式在操控它、它是否沒有指定任何語言，以及它是否不靠顯示卡繪圖。沒有人會被拒絕；看起來像程式的瀏覽器會得到較長的謎題。法律基礎：GDPR 第 6 條第 1 項 (f) 款（我們為保護自然人的地址而有的正當利益）。
privacy-recipients-title = 接收者與國家
privacy-recipients = 我們不出售資料，也不為了廣告而轉交任何資料。接收者有：netcup GmbH（位於德國的主機代管，依 GDPR 第 28 條擔任受託處理者）；mailbox.org（Heinlein Hosting GmbH，柏林），它擔任受託處理者，把郵件遞送到你提供的電子郵件地址；Baylee Central 的管理員，他們為了管理而看得到你的登入名稱、帶標籤的顯示名稱、帳號的來源、建立與上次登入，以及你的 Gateways，但看不到你的加密資料；你以帳號登入的 Gateway（它會收到你的帳號 ID、顯示名稱與標籤，不會收到你的密碼）；你的聯絡人與你所在群組的成員（顯示名稱、標籤與你的訊息）；以及僅在法律要求我們提供時的主管機關。歐洲經濟區以外的國家：只有在你開啟通知時，你瀏覽器的推播服務（Google、Mozilla 或 Apple，美國）才會傳輸資料。他人營運的 Gateways 可能位於任何國家；是否登入由你決定。除此之外，我們不會將資料傳輸到歐洲經濟區以外的國家。
privacy-rights-title = 你的權利
privacy-rights = 你有權請求查閱（GDPR 第 15 條）、更正（第 16 條）、刪除（第 17 條）、限制處理（第 18 條）以及資料可攜（第 20 條）。對於基於我們正當利益（第 6 條第 1 項 (f) 款）所進行的處理，你可以隨時基於與你個人特殊情況有關的理由提出異議（第 21 條）。你可以隨時撤回同意，效力及於未來（第 7 條第 3 項）。你帳號的資料可以在帳號設定中匯出，帳號也可以在那裡刪除；其他事項，只要透過聯絡表單或法律聲明中的地址傳訊息給我們即可。你可以向資料保護主管機關提出申訴（第 77 條），例如你居住地的主管機關，或北萊茵-西伐利亞邦資料保護暨資訊自由專員。沒有任何法律或合約要求你提供資料給我們；不過，沒有登入名稱和密碼，就沒有帳號。我們不進行自動化決策，也不進行剖析（第 22 條）。
privacy-objection-title = 你的異議權
privacy-objection = 對於我們依 GDPR 第 6 條第 1 項 (f) 款處理的資料（提供本站、防止濫用、聯絡表單、法律聲明的謎題、你上次登入的時間、管理紀錄、錯誤報告與使用計數），你可以隨時基於與你的特定情況有關的理由提出異議（GDPR 第 21 條）。屆時我們會停止處理這些資料，除非我們能證明有壓倒你的利益的令人信服的正當理由，或該處理是為了主張、行使或抗辯法律上的請求。透過聯絡表單或寄到法律聲明中的地址送出一則不拘形式的訊息即可。
privacy-links-title = 其他服務的連結
privacy-links = 本站連結到 GitHub、Discord 與 WhatsApp 等服務。這些都只是單純的連結：沒有嵌入任何內容，在你點擊之前，這些服務都不會得知你來訪。點擊時，你的瀏覽器不會說明你來自哪個頁面（沒有 referrer）。之後適用各供應商自己的隱私權聲明，它們也可能在歐盟以外處理資料；社群群組中其他成員在那裡能看到你的哪些資訊，由那些服務決定。
privacy-reports-title = 來自遊戲的錯誤報告與使用計數
privacy-reports = 你在遊戲中按 F8 送出的錯誤報告，包含它的種類、你的文字、用戶端版本，以及你保留勾選的附件；聯絡頁面列出了這些項目。報告在處理完畢 90 天後刪除，最遲三年後刪除。法律基礎：GDPR 第 6 條第 1 項 (f) 款（我們讓遊戲沒有錯誤的正當利益）。只有在你允許時，用戶端才會送出當機報告（GDPR 第 6 條第 1 項 (a) 款；你可以隨時在用戶端的設定中撤回）。在離線遊戲期間，用戶端每分鐘會送出該場遊戲的一組隨機識別碼，此外不送出任何東西；我們只將它保存在記憶體中，並在其最後一次活動跡象的 150 秒後遺忘（GDPR 第 6 條第 1 項 (f) 款；可在「Send anonymous usage count」下關閉）。如同每一次連線，你的 IP 位址會到達我們這裡。

## Terms

terms-title = 使用條款
terms-draft = 這些條款目前是草稿，尚未生效。
terms-version = 版本 { $version }，更新於 { $date }。
# Shown only in the languages a machine translated: every one but German and English.
machine-note = 本站由機器翻譯，目前還沒有使用你語言的人讀過。遊戲本身目前只有英文與德文。<link>告訴我們哪裡有錯。</link>
legal-machine = 本頁是機器翻譯，僅供參考。若與<de>德文</de>或<en>英文</en>內容有出入，以該兩者為準。
terms-machine = Central 的條款只有德文與英文版。以下是英文原文；另有<de>德文版</de>。

## Errors

not-found-title = 此頁面不存在
not-found-lead = 可能是地址打錯了，或是頁面已搬走。
gone-title = 此頁面已永久消失
gone-lead = 原本在這裡的內容已被移除。首頁會告訴你接下來可以去哪裡。
error-home = 回到首頁

## Baylee's remembrance (approved by her family on 27 October 2026; keep word for word)
baylee-dedication = 獻給 Baylee。永遠在陽光裡。
baylee-invitation = 來認識這個世界背後的小小靈魂
baylee-open = 懷念 Baylee · 打開她的相簿
baylee-eyebrow = 這個專案的心
baylee-date = 深切懷念 · 2026 年 10 月 26 日
baylee-close = 回到她的世界
baylee-previous = 上一張照片
baylee-next = 下一張照片
baylee-photo1 = Baylee 蜷縮在柔軟的白色被子裡。
baylee-photo2 = Baylee 從花朵圖案的被子裡向外張望。
baylee-photo3 = 在她圓圓的灰色床上休息。
baylee-photo4 = Baylee 睡著了，緊緊蜷成一團。
baylee-photo5 = 安靜的片刻，窩在被子裡。

baylee-storyTitle = 獻給 Baylee。永遠在陽光裡。
baylee-together = 與我們在一起 · 2025 年 11 月 17 日 – 2026 年 10 月 26 日
baylee-chapters = 她的故事
baylee-chapterPrevious = 上一章
baylee-chapterNext = 下一章
baylee-photos = 我們的回憶
baylee-read = 向下捲動閱讀

baylee-chapter1Title = Baylee 的家
baylee-chapter1Body =
    Baylee 在 2025 年 11 月 17 日來到我們家。

    我和妻子從克雷費爾德的動物收容所領養了她。她估計約十三歲。她有惡性乳腺腫瘤，癌細胞已經轉移到肝臟。Baylee 是一隻安寧照護的貓。我們知道相處的時間有限。在我們身邊，她要有一個被愛的家。

    她很快就讓我們看見，她多快就把這個家當成了自己的。

    第一天，我在工作時，她就溜到我的腿上。第一個晚上，她在我為父母的貓訂的一包貓糧上咬破了洞，自己吃了起來。當然，她那天晚上已經有自己的食物，也吃得很飽。但 Baylee 顯然認為，這批存糧還值得再仔細調查一番。

    我們的小老鼠食慾驚人，精力也驚人。

baylee-chapter2Title = 逐漸建立的信任
baylee-chapter2Body =
    我們發現她喝了很多水，就帶她去看獸醫。驗血發現她有甲狀腺疾病，這也解釋了她的大食量和活潑的個性。因為她的發炎指數偏高，獸醫開了甲狀腺藥物，以及一個短期的抗生素療程。

    後來她突然開始拿身體蹭所有東西，常常喵喵叫，變得格外愛撒嬌、黏著我們，我們起初擔心她對某種新藥反應不好。再次檢查後才找到原因：我們的小寶貝發情了。她沒有結紮。

    那段時間我們也一起撐過來了。Baylee 越來越信任我們，我們彼此也越來越親近。

baylee-chapter3Title = 我們的小公主
baylee-chapter3Body =
    我至今還清楚記得，她得到貓跳台時有多開心。她立刻坐上最頂端的平台，滿意地環顧整間公寓。奇怪的是，她後來再也沒有用過那個位置。為什麼，成了她的祕密。

    其他地方很少空著：我的腿上、我的雙腿，還有我們的床。她通常睡在我或妻子身上。最棒的，是同時躺在我們兩個人身上。

    她翹著尾巴，昂首闊步地走過整間公寓，彷彿她很清楚自己是這裡的公主。她有個性，也懂得清楚表明自己的界線。在我們身邊，她可以完全做自己。她的怪癖、她的要求，還有她那一點小小的固執，我們也都愛。

    我從未和一隻動物建立過這麼深的連結。Baylee 在我妻子與我的心裡，都占了很深很深的位置。

baylee-chapter4Title = 陽光下的時光
baylee-chapter4Body =
    天氣變暖之後，她發現了陽台。她喜歡讓一天中最早的陽光灑在毛上。沒多久，她幾乎整天都待在那裡。

    鴿子和庭院裡的小鳥特別吸引她。她專注地看著牠們，埋伏等候，一次又一次想嚇牠們一跳。在做日光浴和賞鳥之間，我們的小公主每天的行程排得滿滿的。

baylee-chapter5Title = 她自己的小主意
baylee-chapter5Body =
    與此同時，她的病情持續惡化。

    腫瘤發炎了，Baylee 不停舔，把患處舔開。在獸醫那裡，她拿到了第一件術後護理服，是紅色的。從那以後，她每天都得穿著，免得傷口被她弄得更糟。

    她很快就習慣穿著它。但直到最後一天，她都不喜歡被人幫她穿上。就算有一碗食物，也只能勉強把她哄出來。她的食慾再大，對這件衣服的看法也再明白不過。

    傷口越來越嚴重。它流出膿液，後來開始出血，還發出難聞的氣味。同時，Baylee 一直讓我們看見，她在日常生活中仍然找得到多少快樂。她玩耍、吃東西、探索周圍，爬上她的貓抓板和櫃子。

    有時她會在床邊躺上好幾個小時，陪著我們，等我們終於醒來，替她準備早餐。如果等太久，她就直接從我們身上爬過去。我們的臉對她來說根本不是障礙。

    她對食物有很明確的想法。她的口味比較屬於「冷凍披薩」那一派：過一陣子，高品質的貓糧就再也無法讓她興奮，變化是必要的。連續兩次吃同樣的東西是不可能的。雞肉加肝、任何有起司的東西、鮪魚或鮭魚——這些都能讓她開心。

baylee-chapter6Title = 尾巴高高翹起
baylee-chapter6Body =
    腫瘤越長越大，也越來越常滲液。Baylee 明顯很痛。疼痛減輕時，她又變回我們的小貓，翹著尾巴穿梭在房間裡，把她的人據為己有。

    我們看得出她的力氣正慢慢消退。但她的食慾、她的好奇心、她親人的天性，還有她響亮的呼嚕聲，都還在。

baylee-chapter7Title = 道別
baylee-chapter7Body =
    2026 年 10 月 23 日星期五晚上，她出現了一開始看起來像感冒的症狀。她流鼻水，呼吸深而吃力，像是在拚命喘氣。

    星期六她看起來又有活力了些。不尋常的呼吸和流鼻水還在，但她再次展現出我們熟悉的那種活力和對生命的熱愛。

    星期日早上很難熬。她的眼睛一直流淚，呼吸困難，而且很痛。我給她吃了止痛藥。大約一小時後她又覺得好些了，但呼吸仍然和平常不一樣。我們知道必須帶她去看獸醫。

    星期一早上，她的狀況又惡化了。幸好我妻子那天休假。我們打電話給診所、約好時間，帶 Baylee 過去。

    X 光顯示癌症已擴散到什麼程度。我們被告知，她的肺只剩大約三分之一還能用來呼吸。讓她繼續活下去，意味著她能吸到的空氣會越來越少。

    我們必須道別。

    2026 年 10 月 26 日星期一中午前後，我們最親愛的 Baylee 被安樂死了。

baylee-chapter8Title = 永遠在陽光裡
baylee-chapter8Body =
    她和我們在一起不到一年。在這段時間裡，她成了我們家庭的一分子，也是非常親近的朋友。她屬於我們工作的日子、我們的夜晚，和我們早上醒來的時刻。屬於陽台上陽光燦爛的時光，也屬於構成共同生活的那些小小習慣。

    現在，所有這些時刻都少了她。

    空虛感仍然新鮮而深刻。我們想念她的呼嚕聲、她壓在我們腿上的重量、她豎起的尾巴，以及她理所當然地期待我們終於來張羅她早餐的樣子。

    如果這一生之後還有些什麼，不管那是什麼，我希望有一天能再遇見那個靈魂。

    在那之前，我們會永遠把我們的小老鼠放在心裡。

    因此，這個興趣專案是獻給 Baylee 的。獻給她的靈魂、她的親近、她對陽光與小鳥的喜悅。也獻給我們在一起的時光裡長出來的愛。

    願她在這裡，在她的雲上安穩地睡著，沐浴在溫暖的陽光裡，身邊有花朵、鴿子和庭院裡的小鳥。

    安息吧，我最親愛的 Baylee。

baylee-photoOpen = 檢視完整照片
baylee-photoClose = 關閉照片
baylee-zoomIn = 放大
baylee-zoomOut = 縮小
baylee-fit = 符合視窗

baylee-photo6 = Baylee 舒服地窩在毯子之間。
baylee-photo7 = 依偎在她紅色的術後護理服裡。
baylee-photo8 = 紙箱裡一處陽光灑落的角落。
baylee-photo9 = 被蓋得好好的，完全放鬆。
baylee-photo10 = 她最愛的位置：就在我們身邊。

## Errors shared by the forms

error-busy = 嘗試次數過多。請稍後再試；連續輸錯幾次密碼後，可能需要一小時，最長一天。
error-unavailable = Baylee Central 目前沒有回應。請稍後再試。
error-network = 沒有連線。請檢查網路後再試一次。
error-failed = 發生了問題。請再試一次；若持續發生，請告訴我們。
error-again = 再試一次
password-show-label = 顯示密碼

## Signing in

signin-title = 登入
signin-lead = 你的 Baylee 帳號，用於線上牌桌、聯絡人與聊天。
signin-name = 登入名稱
signin-password = 密碼
signin-submit = 登入
signin-missing = 請輸入你的登入名稱與密碼。
signin-working = 正在保護你的密碼……需要一點時間，而且只在你的瀏覽器中進行。
signin-wrong = 這個登入名稱與密碼不相符。請檢查兩者後再試一次。
signin-not-moved = 這個帳號還沒有移轉到 Baylee Central。請先用 Baylee 用戶端登入一次；它會幫你移轉帳號。
signin-no-account = 還沒有帳號？<register>建立一個</register>。
signin-forgot = 忘記密碼了？<reset>用重設碼設定新密碼</reset>。
register-have-account = 已經有帳號了？<signin>登入</signin>。
dialog-close = 關閉
signin-noscript = 登入需要 JavaScript：你的密碼在你的瀏覽器中處理，絕不會傳送給我們。

## Registering

register-title = 建立帳號
register-lead = 用於線上牌桌、聯絡人與聊天。公開測試期間免費，人人都可以申請。
register-name = 登入名稱
register-name-hint = 私人使用，只用來登入：3 到 24 個小寫字母、數字、句點、連字號或底線。
register-display = 顯示名稱
register-display-hint = 其他玩家看到你的名字：3 到 16 個字母、數字、- 與 _。你的標籤會幫你產生。
register-password = 密碼
register-password-hint = 至少 8 個字元。一句長長的話容易記，也很難猜。
register-repeat = 再輸入一次密碼
register-repeat-hint = 再輸入一次相同的密碼，以免打錯。
register-age = 我已年滿 16 歲。
register-consent = 我接受<terms>使用條款</terms>。我們如何處理你的資料，請見<privacy>隱私權聲明</privacy>。
register-submit = 建立帳號
register-error-name = 登入名稱需要 3 到 24 個小寫字母、數字、句點、連字號或底線。
register-error-display = 顯示名稱需要 3 到 16 個字母或數字；- 與 _ 只能放在中間。
register-error-reserved = 這個顯示名稱已被保留。請選另一個。
register-error-short = 密碼至少需要 8 個字元。
register-error-long = 密碼太長。
register-error-repeat = 兩次輸入的密碼不一樣。
register-error-age = 你需要年滿 16 歲才能擁有帳號。
register-error-consent = 請接受使用條款才能建立帳號。
register-error-taken = 這個登入名稱已被使用。請選另一個。

## The recovery code

code-title = 你的救援碼
code-handle = 你的代號：{ $handle }
code-lead = 如果你忘記密碼，只有靠這組救援碼，你的加密資料才保得住。我們不知道它，也無法恢復。請把它存放在安全的地方，遠離這台裝置。
code-shown-once = 只顯示這一次
code-download = 儲存為文字檔
code-kept = 我已儲存我的救援碼
code-not-kept = 請先儲存救援碼，再確認。
code-continue = 繼續
code-later = 稍後再說
code-copy = 複製
code-copied = 已複製。
code-copy-failed = 這裡無法複製。請選取代碼，手動複製。
code-print = 列印
code-manager = 存入密碼管理員
code-manager-id = Baylee 救援碼（{ $name }）
code-manager-offered = 已將這組碼交給你的瀏覽器。如果它有詢問你，請檢查是否已儲存。
code-manager-failed = 你的瀏覽器沒有接收這組碼。請用其他方式保存。
code-mail = 用電子郵件寄給自己
code-mail-warning = 請注意：這組碼之後會留在你的信箱裡。能讀取你信件的人，也可以讓重設碼寄到那裡；有了兩者，就能重設你的登入並打開你的加密資料。
code-mail-subject = 我的 Baylee 救援碼
code-sheet-title = Baylee 救援碼
code-sheet-name = 登入名稱：{ $name }
code-sheet-what = 有了這組碼，你的 Baylee 加密資料在忘記密碼後仍能保住：請在重設頁面上連同重設碼一起輸入。
code-sheet-keep = 請存放在只有你拿得到的地方。Baylee 的任何人都不會向你索取它。
code-email-label = 電子郵件地址（選填）
code-email-hint = 用於在你忘記密碼時寄送重設碼。完成這一步後，我們會寄一組碼到該地址以確認；你也可以稍後在帳號中新增。
code-email-step-title = 確認你的電子郵件地址
code-email-step-lead = 請在這裡輸入郵件中的碼，或稍後到帳號的安全頁面輸入。
code-email-step-done = 前往你的帳號

## Setting a new password with a reset code

reset-title = 設定新密碼
reset-lead = 給忘記密碼的你：用重設碼就能在這裡設定新密碼。重設碼會以電子郵件寄到你已確認的地址，或由管理員提供。
reset-noscript = 設定新密碼需要 JavaScript：你的密碼和救援碼在你的瀏覽器中處理，絕不會傳送給我們。
reset-code = 重設碼
reset-code-hint = 以電子郵件寄來，或由管理員提供。它以 RESET 開頭，只能使用一次：以電子郵件寄來的有效一小時，管理員產生的有效 24 小時。
reset-data = 你的加密資料
reset-keep = 我有救援碼。保留我的資料。
reset-new = 我沒有救援碼。從空保險庫開始。
reset-recovery = 救援碼
reset-recovery-hint = 建立帳號時你儲存的那組碼。它只留在你的瀏覽器中，絕不會傳送給我們。
reset-warning = 使用空保險庫後，你的加密資料將永久遺失：儲存在帳號中的設定、你的牌組備份和你的聊天記錄。沒有人能把它們找回來，我們也不能。你的帳號、顯示名稱和聯絡人會保留。你會得到一組新的救援碼。
reset-warning-confirm = 我明白這些資料將永久遺失。
reset-submit = 設定新密碼
reset-missing = 請輸入你的登入名稱、重設碼和新密碼。
reset-recovery-missing = 請輸入你的救援碼，或選擇空保險庫。
reset-error-code = 這不是重設碼。它以 RESET 開頭，後面是八組、每組四個字元。
reset-error-recovery = 這組救援碼打不開你帳號的資料。請檢查後再試一次。沒有任何變更，你的重設碼仍然有效。
reset-error-confirm = 請先確認你知道會遺失什麼。
reset-refused = 這個登入名稱和重設碼不相符，或者該碼已過期，或者已被使用。請檢查兩者。如果仍然不行，請索取新的碼。
reset-done-title = 你的新密碼已設定
reset-done-kept = 你的資料和救援碼維持原樣。你已在所有地方登出；請用新密碼登入。
reset-done-new = 你將從空保險庫開始，並已在所有地方登出。下面是你的新救援碼；舊的已不能使用。
reset-unsure-title = 沒有收到回應
reset-unsure = 在 Baylee Central 回應之前連線中斷了，所以我們無法確定你的新密碼是否已設定。請嘗試用它登入。如果登入失敗，表示沒有任何變更：你的重設碼仍然有效，請在這裡重新輸入。
reset-unsure-code = 如果你的新密碼可用，這就是你的新救援碼，舊的已不能使用；它無法再次顯示，所以請現在就儲存。如果你的新密碼不可用，這組碼就一文不值。
reset-to-signin = 登入
reset-mail-title = 沒有重設碼？
reset-mail-lead = 輸入你的登入名稱。如果你的帳號有已確認的電子郵件地址，我們會寄一組重設碼到該地址。
reset-mail-name-hint = 是你用來登入的名稱，不是你的電子郵件地址。
reset-mail-submit = 以電子郵件寄送重設碼
reset-mail-error-name = 這不是登入名稱。請輸入你用來登入的名稱，而不是電子郵件地址。
reset-mail-sent = 如果這個帳號有已確認的電子郵件地址，重設碼正在寄往該地址。它的有效期限為一小時。
reset-mail-busy = 目前請求過多。請稍後再試。
reset-mail-none = 這台伺服器不寄送郵件。請向管理員索取重設碼。
reset-mail-early = 太快了。請稍等片刻再試。

## Changing the password

password-new = 新密碼
password-new-repeat = 再輸入一次新密碼
password-current = 目前的密碼
password-change-submit = 變更密碼
password-missing = 請輸入你目前的密碼和新密碼。
password-error-current = 目前的密碼不正確。請檢查後再試一次。
password-changed = 你的密碼已變更。你在這裡維持登入，在其他所有地方（包括 Baylee 用戶端）都已登出；請在那裡用新密碼登入。
password-unsure = 沒有收到回應，所以我們無法確定你的密碼是否已變更。下次登入時，先試新密碼，再試舊密碼。

## An account's e-mail address

email-title = 電子郵件地址
email-lead = 電子郵件地址是選填的。當你要求時，我們用它寄送重設碼；地址被取代或移除時，也用它通知你。
email-warning = 能讀取這個信箱的人，都可以重設你的登入。只有憑救援碼，你的加密資料才能在重設後保留下來。
email-no-mail = 這台伺服器不寄送郵件，因此無法在這裡設定或確認電子郵件地址。
email-unavailable = 目前無法顯示電子郵件地址。
email-password-hint = 你目前的密碼，這樣使用這個瀏覽器的其他人就無法決定誰能重設你的登入。
email-submit = 寄送確認碼
email-missing = 請輸入電子郵件地址和你的密碼。
email-error-address = 我們無法寄信到這個地址。請輸入像 name@example.org 這樣的單一一般地址；帶重音的字母和變音字母無法使用。
email-current = 你的地址
email-confirmed-at = 已確認
email-waiting = 等待確認碼
email-change = 變更
email-change-lead = 在你確認新地址之前，目前的地址會繼續使用。
email-change-cancel = 保留目前的地址
email-remove = 移除
email-remove-ask = 要移除這個地址嗎？我們會通知該地址它已被移除。之後你就無法再用電子郵件取得重設碼了。
email-remove-ask-no-mail = 要移除這個地址嗎？只要這台伺服器不寄送郵件，你就無法再次設定地址。
email-remove-confirm = 移除地址
email-remove-keep = 保留
email-pending = 我們已寄了一組碼到 <code>{ $address }</code>。它的有效期限為 15 分鐘。
email-pending-keeps = 在你確認之前，<code>{ $address }</code> 仍是你的地址。
email-code = 確認碼
email-code-hint = 八個字元，如 XXXX-XXXX。大小寫皆可，有無連字號皆可。
email-confirm = 確認
email-resend = 再寄一次
email-other = 使用其他地址
email-withdraw = 撤回
email-error-code = 這不是確認碼。確認碼有八個字元，如 XXXX-XXXX。
email-wrong-code = 這組碼不對。請檢查後再試。輸錯幾次之後，你需要重新輸入地址。
email-code-dropped = 這組碼也不對，所以該地址不再等待確認。請重新輸入地址以取得新的碼。
email-now-confirmed = 你的電子郵件地址已確認。
email-resent = 新的碼正在寄出。先前的碼已失效。
email-withdrawn = 該地址已撤回。不再等待任何碼。
email-removed = 你的電子郵件地址已移除。
email-busy = 目前郵件過多。請稍後再試。
email-same = 這已經是你的地址了。
email-unsure = 沒有收到回應，所以我們無法確定剛才的操作是否成功。請看一下這裡顯示的內容；如果沒有成功，請再試一次。
email-again = 要寄送這組碼，請再輸入一次你的密碼。

## Central's terms

terms-step-title = 新的使用條款
terms-step-lead = Baylee Central 的條款已變更。請閱讀並接受，才能使用你的帳號；你也可以拒絕並登出。
terms-accept = 接受
terms-decline = 拒絕並登出

## The signed-in area

account-lead = 你的 Baylee 帳號：個人資料、安全性、隱私權與 Gateways。
account-nav = 你的帳號
account-home = 總覽
account-profile = 個人資料
account-profile-lead = 你的代號、顯示名稱與救援碼。
account-security = 安全性
account-security-lead = 你在哪裡登入，以及登出。
account-privacy = 隱私權與資料
account-privacy-lead = 匯出你的資料，或刪除你的帳號。
account-gateways = Gateways 與權杖
account-gateways-lead = 你註冊的 Gateways，以及它們用來登入的權杖。
account-loading = 載入中……
account-signed-out = 你尚未登入。
account-unnamed = 你的帳號
account-signin-name = 登入名稱：{ $name }
account-until = 登入有效至 { $time }
account-needs-title = 需要你注意
account-needs-code = 建立救援碼
account-needs-name = 選擇你的顯示名稱
account-admin = 管理

reauth-lead = 這個操作需要最近一次的登入。請再輸入一次登入名稱與密碼。
reauth-submit = 重新登入

profile-who-title = 你是誰
profile-handle = 代號
profile-signin-name = 登入名稱
profile-source = 帳號
profile-source-moved = 從官方 Gateway 移轉而來
profile-source-registered = 在本站註冊
profile-choose-title = 選擇你的顯示名稱
profile-choose-submit = 儲存顯示名稱
profile-name-taken = 這個帳號已經有顯示名稱了。
profile-code-title = 救援碼
profile-code-kept = 你的帳號有一組救援碼。請妥善保管；沒有密碼時，它是找回你加密資料的唯一方法。
profile-code-missing = 你的帳號還沒有救援碼。請建立一組；你會需要再輸入一次密碼。
profile-code-make = 建立救援碼

security-sessions = 你在哪裡登入
security-lead = Baylee Central 為這個網站保留一個工作階段，也為遊戲用戶端保留一個；它不會區分個別裝置。
security-kind-browser = 這個網站
security-kind-client = Baylee 用戶端
security-this = 目前這個
security-signed-in = 登入於
security-ends = 結束於
security-gateways = 在 Gateways
security-end-client = 登出用戶端
security-client-ended = 用戶端已登出。
security-sign-out = 在此登出
security-password-title = 密碼
security-password-lead = 你的加密資料和救援碼維持原樣。變更密碼會讓你在此處以外的所有地方登出。

privacy-export-title = 匯出你的資料
privacy-export-lead = Baylee Central 所知道的關於你帳號的資料，以 JSON 檔案提供：基本資料、工作階段、已接受的條款、你的 Gateways，以及提到你帳號的稽核紀錄。你的加密內容只能由你的 Baylee 用戶端解密並匯出。匯出需要不到 30 分鐘前的登入。
privacy-export-make = 準備匯出
privacy-export-save = 儲存匯出檔
privacy-export-no-email = 無法讀取電子郵件地址；它不在此檔案中。
privacy-delete-title = 刪除你的帳號
privacy-delete-lead = 你的帳號、其資料、工作階段、聯絡人，以及你擁有的 Gateways，會立即從運作中的系統永久刪除；它們最遲在 12 個月內從備份中移除。Central 也會刪除你傳送的訊息；別人已儲存在他們裝置上的內容可能仍會留在那裡。
privacy-delete-start = 刪除我的帳號
privacy-delete-password = 你的密碼，用於確認
privacy-delete-prove = 繼續
privacy-delete-last = 最後一步：這無法復原。你有一分鐘。
privacy-delete-gateways = 你擁有的這些 Gateways 會一併刪除：
privacy-delete-confirm = 永久刪除帳號
privacy-delete-cancel = 保留我的帳號
privacy-deleted = 你的帳號已刪除。

gateways-none = 你還沒有註冊 Gateway。多數玩家都用不到。
gateways-add-title = 註冊 Gateway
gateways-origin = Gateway 位址
gateways-origin-hint = Gateway 的 https 位址，例如 https://play.example.org。
gateways-name = Gateway 名稱
gateways-add = 註冊 Gateway
gateways-added = Gateway 已註冊，正在等待管理員核准。
gateways-taken = 這個位址已經註冊過了。
gateways-invalid = 請輸入不含路徑的 https 位址，以及一個名稱。
gateways-approved = 已核准
gateways-pending = 等待核准
gateways-disabled = 已停用
tokens-title = 權杖
tokens-name = 權杖名稱
tokens-days = 有效天數
tokens-sso = 讓玩家登入
tokens-listing = 列出 Gateway
tokens-both = 讓玩家登入並列出 Gateway
tokens-make = 建立權杖
tokens-revoke = 撤銷
tokens-revoked = 已撤銷
tokens-revoked-now = 權杖已撤銷。
tokens-until = 至 { $time }
tokens-invalid = 請為權杖取個名稱，並至少選擇一項它允許的功能。
tokens-shown-title = 你的新權杖
tokens-kept = 我已儲存
tokens-again = 已重新登入。請再建立一次權杖。

## The messenger

account-chat = 聊天
account-contacts = 聯絡人
chat-side = 對話與聯絡人
chat-tabs = 通訊
chat-tab-chats = 聊天
chat-tab-contacts = 聯絡人
chat-main = 對話
chat-unread = 未讀
chat-conversations = 你的對話
chat-search = 尋找對話
chat-search-hint = 尋找（Ctrl+K）
chat-new-group = 新群組
chat-no-conversations = 還沒有對話。新增一位聯絡人，再寫訊息給他。
chat-no-match = 沒有對話符合這個名稱。
chat-former-group = 已離開的群組
chat-former-lead = 你已不在這個群組中。
chat-you = 你
chat-members = { $n ->
   *[other] { $n } 位成員
}
chat-say-hello = 打個招呼吧。
chat-history-only = 已不是聯絡人。
chat-today = 今天
chat-yesterday = 昨天
chat-encrypted = 端對端加密
chat-opening-lead = 你的對話與聯絡人。訊息在離開你的裝置之前就已加密，Central 只負責轉送，無法讀取。
chat-new-from = 來自 { $who } 的新訊息。
chat-new-many = { $n } 則新訊息。
chat-retry = 再試一次
chat-discard = 捨棄
chat-not-sent = 未送出。
chat-sending = 傳送中……
chat-keys-changed = 這個對話中有人的金鑰變了。再次寫訊息之前，請先比對安全碼。
chat-refused = 有些訊息無法驗證，因此不予顯示。
chat-failed = 聊天暫時無法連上 Central。
chat-failed-open = 無法開啟聊天。請重新載入頁面再試一次。
chat-empty = 還沒有訊息。
chat-older = 顯示較早的訊息
chat-compare-first = 金鑰變了。請先比對安全碼，再寫訊息。
chat-compare = 比對安全碼
chat-friend-required = 重新成為聯絡人之後，你才能寫訊息。
chat-group-left = 你已不在這個群組中。
chat-details = 詳細資料
chat-close = 關閉
chat-back = 回到對話列表
chat-messages = 訊息
chat-fan = Baylee 屬於非官方同好內容，未經威世智核准或背書。<more>這代表什麼</more>
chat-safety = 安全碼
chat-safety-lead = 請和對方比對這組號碼，可以當面比對，或用另一個你信任的管道。如果相符，就沒有人在你們之間悄悄插入。
chat-changed = 已變更
chat-confirm-keys = 相符：信任新的金鑰
chat-count = { $n } / 1,000 字元
chat-write = 寫一則訊息
chat-send = 送出
chat-enter-hint = 按 Enter 送出，按 Shift+Enter 換行。
chat-choose = 選擇一個對話
chat-choose-lead = 你的訊息在這個瀏覽器中加密；Central 只負責轉送。
chat-storage = 在你登入期間，你的聊天金鑰會保留在這個瀏覽器中。
chat-opening = 正在開啟你的聊天……
chat-open = 開啟我的聊天
chat-ended = 你的登入已結束。
chat-unnamed = 請先選擇你的顯示名稱：它會給你一個別人用來寫訊息給你的代號。
chat-missing-title = 重新登入以開啟你的聊天
chat-missing-lead = 這個瀏覽器沒有你的聊天金鑰。你的密碼可在這裡將它開啟；密碼絕不會離開這個瀏覽器。
chat-invites = 群組邀請
chat-invited-by = 來自 { $who }
chat-group-name = 群組名稱
chat-group-name-hint = 1 到 48 個字元的名稱。
chat-group-create = 建立
chat-group-failed = 暫時無法變更群組。
chat-group-members = { $n ->
   *[other] { $n } 位成員
}
chat-group-owner = 擁有者
chat-group-rename = 重新命名
chat-group-transfer = 設為擁有者
chat-group-remove = 移除
chat-group-transfer-ask = 要讓 { $who } 成為這個群組的擁有者嗎？你仍會留在群組中。
chat-group-remove-ask = 要將 { $who } 從這個群組中移除嗎？
chat-group-invite = 邀請一位聯絡人
chat-group-choose = 選擇一位聯絡人
chat-group-invite-button = 邀請
chat-group-invited = 已邀請
chat-group-delete = 刪除群組
chat-group-leave = 離開群組
chat-group-delete-ask = 要為所有人刪除「{ $name }」嗎？已收到的訊息仍可閱讀。
chat-group-leave-ask = 要離開「{ $name }」嗎？你只能靠新的邀請才能回來。

people-incoming = 請求
people-failed = 暫時無法載入你的聯絡人。
people-empty = 還沒有聯絡人。用代號尋找某個人，或把你的代號給他。
people-your-handle = 你的代號：
people-incoming-title = 給你的請求
people-friends = 聯絡人
people-outgoing = 已送出的請求
people-blocked = 已封鎖
people-find = 用代號新增聯絡人
people-find-button = 尋找
people-handle-hint = 帶標籤的完整代號，例如 Name#1a2b。
people-yourself = 這是你自己的代號。
people-nobody = 沒有人使用這個代號。
people-request = 送出聯絡請求
people-requested = 請求已送出。
people-is-friend = 聯絡人
people-is-incoming = 請求成為你的聯絡人
people-is-outgoing = 請求已送出
people-is-blocked = 已封鎖
people-write = 寫訊息
people-remove = 移除
people-block = 封鎖
people-accept = 接受
people-decline = 拒絕
people-cancel = 收回
people-unblock = 解除封鎖
people-keep = 保留
people-confirm-remove = 要將 { $who } 從你的聯絡人中移除嗎？你的訊息會保留。
people-confirm-block = 要封鎖 { $who } 嗎？對方將不能再請求成為你的聯絡人，也不能寫訊息給你。

## Offline, updates, installing

title-offline = 離線
offline-title = 你已離線
offline-lead = 這個頁面需要連線。Central 會替你保留訊息，直到你回到線上。
offline-again = 再試一次
update-ready = baylee.app 有新版本了。
update-unsent = 你在這裡有尚未送出的文字。仍要重新載入嗎？
update-reload = 重新載入
install-app = 安裝為應用程式

## Notifications

push-title = 通知
push-lead = 選擇 Baylee 可以通知你哪些事。通知不會顯示訊息內文、姓名或報告內容。
push-device = 這台裝置上的通知
push-categories = 通知我
push-chat = 聊天訊息
push-contacts = 聯絡請求
push-updates = 新版本
push-feedback = 意見回饋報告（管理員）
push-gateways = Gateway 申請（管理員）
push-on = 已在這台裝置上啟用。只會送出所選的類別。
push-off = 已在這台裝置上停用。你選擇的類別會保留。
push-scope = 類別的選擇適用於你的帳號。裝置的權限在每個瀏覽器上各自獨立。登出會中斷這台裝置的連線。
push-failed = 無法確認這項變更。請再試一次。
push-denied = 通知已被封鎖。你可以在瀏覽器或裝置設定中允許，然後再試一次。
push-unsupported = 這個瀏覽器在此不提供 Web Push。在 iPhone 與 iPad 上，請將 Baylee 加入主畫面，並從那裡開啟。
push-unconfigured = 這台伺服器目前還不提供背景通知。你的偏好設定仍然可以先儲存。

register-closed = 註冊目前暫停。請稍後再試。
register-refused = 註冊未被接受。請再試一次。
admin-registration = 註冊
admin-registration-open = 開放：解開工作量證明後，任何人都可以建立帳號。
admin-registration-closed = 已關閉：在你再次開放之前，沒有人能建立帳號。
admin-registration-lead = 遭受攻擊時關閉註冊，之後再手動重新開放。每次變更都會記錄在稽核紀錄中。
admin-registration-close = 關閉註冊
admin-registration-reopen = 重新開放註冊
hero-discord = Discord 社群
hero-whatsapp = WhatsApp 社群
hero-github = GitHub
nav-menu = 選單
foot-project = 專案
foot-source = 原始碼（AGPL-3.0）
foot-central = Baylee Central（原始碼可取得）
foot-summary = 以 Rust 與 Bevy 打造的免費開放原始碼桌上卡牌遊戲——具備確定性的規則引擎、完整的離線模式、內建 AI、LLM 橋接與社群 Gateways。
strip-ci = CI
strip-commit = 提交
strip-gateways = Gateways
strip-online = 在線
strip-offline = 離線
strip-directory = 目錄
strip-cards = 可玩的牌
strip-cards-unit = 張
strip-accounts = Baylee Central 的帳號
strip-live = 各 Gateway 回報的目前數字
strip-views = 自 { $date } 起的頁面瀏覽次數
live-views = { $n ->
   *[other] { $n } 次瀏覽
}
live-accounts = { $n ->
   *[other] { $n } 個帳號
}
live-players = { $n ->
   *[other] { $n } 位玩家
}
live-playing = { $n ->
   *[other] { $n } 位遊戲中
}
live-tables = { $n ->
   *[other] { $n } 張開放牌桌
}
live-games = { $n ->
   *[other] { $n } 場遊戲
}
hero-badges = 一覽
hero-badge-beta = 公開測試
hero-badge-free = 免費
hero-badge-open = 開放原始碼
hero-badge-fan = 非官方同好內容
gws-open-client = 在 Baylee 中開啟
gws-open-client-help = 「在 Baylee 中開啟」會啟動已安裝的用戶端（0.1.1-beta.9b 或更新版本）。它會顯示位址，並在新增 Gateway 之前先詢問你。
dl-menu-title = 所有系統
dl-old-cpu = 較舊的處理器（無 AVX2）
ci-passing = 通過
ci-failing = 失敗
dl-button = 下載 Baylee
dl-quick = 其他系統與架構
dl-others = 其他系統

# The start page, as the old site wrote it (owner, 11.11.2026)
link-readmeModel = #playing-against-a-language-model
about-engineTitle = 嚴格執行每一條規則的引擎
about-deterministic = 由種子隨機數產生器驅動，完全不依賴系統時鐘或雜湊順序，每場對戰都是嚴格確定的，並可一步一步重現。
about-tableTitle = 以 Bevy 打造的清爽 3D 牌桌
about-keys = 從內建的大廳，你可以輕鬆地在本機牌組建構器、單人對戰，以及社群 Gateways 上的多人牌桌之間切換。
feature-hiddenTitle = 隱藏資訊保持私密
feature-hidden = 你的用戶端只會收到你的座位被允許看到的資料。牌庫與對手的手牌只會以張數送達——即使是被修改過的用戶端，也無法查看隱藏的牌。
feature-seatsTitle = 同一桌最多八位玩家
feature-seats = 雙人對決、混戰與團隊對戰（例如 2v2），最多 8 個座位。隊友共用回合，並可以彼此公開手牌。
feature-commanderTitle = 彈性的賽制與自訂家規
feature-commander = 可組出經典的 60 張牌對戰、多人混戰，或自訂家規賽制的牌組。起始生命、隊伍編排與調度規則，都由主持人自由設定。
feature-aiTitle = 確定性回放與 F8 報告
feature-ai = 每場對戰都逐步記錄。如果你在遊戲中發現規則互動有問題，直接在牌桌上按 F8，就能送出可重現的狀態報告。
feature-processTitle = 每場對戰各自獨立的伺服器程序
feature-process = 每場進行中的遊戲，都在 Gateway 上自己的沙盒引擎程序中執行。Gateway 本身只管理大廳與房間——出錯絕不會影響其他牌桌。
feature-languagesTitle = 19 種語言的牌面文字
feature-languages = 只要 Gateway 提供多語言卡牌目錄，牌面就會以你偏好的語言顯示，並以英文作為後備。用戶端介面提供德文與英文。
feature-watchTitle = 觀戰進行中的對戰
feature-watch = 大廳的 Watch 按鈕會讓你以即時觀戰者的身分入座：即時追蹤公開的戰場，同時所有玩家的手牌仍保持隱藏。
feature-connectionTitle = 斷線後順利重連
feature-connection = 如果連線短暫中斷，牌桌會保留座位並開始倒數，也可以暫停對戰，或讓內建 AI 暫時接手。
feature-tablesTitle = 十四種決策計時
feature-tables = 從明快的 15 秒回合，到悠閒不限時的牌桌，每個房間的節奏都能個別設定。
shots-eyebrow = 在牌桌上
shots-title = 遊戲裡面
shots-lead = 這些簡短的即時錄影直接取自 3D 用戶端，牌面圖片來自 Scryfall（© Wizards of the Coast）——從對戰、4 人混戰，到加密大廳與牌組建構器。
shots-credit = 牌面圖像：Scryfall · © Wizards of the Coast
shots-showLarge = 放大顯示：{ $title }
shots-dialog = 截圖
shots-previous = 上一張
shots-next = 下一張
shots-close = 關閉
ai-eyebrow = AI 對手與語言模型
ai-title = 內建 AI、LLM 橋接與原生 Baylee AI
ai-houseTitle = 分五種強度的啟發式內建 AI
ai-house = 離線和任何線上牌桌都能直接使用：可選擇 Novice、Casual、Steady、Sharp 與 Expert。內建 AI 在本機決策，完全不需等待，而且只看得到與真人玩家相同的公開狀態。
ai-modelTitle = 透過 Seat-Bridge 讓語言模型上桌
ai-model = 讓 <b>Google Gemini、OpenAI ChatGPT、Anthropic Claude、DeepSeek</b>，或透過 <b>LM Studio</b> 執行的本機模型，直接入座你的牌桌。API 金鑰受環境變數與花費上限保護；逾時時，內建 AI 會無縫接手。
ai-setup = LLM 與 Seat-Bridge 設定指南
home-gwLead = 線上牌桌在 Gateways 上，這些伺服器經管理員核准後，由 Baylee Central 列出。這裡是前幾個；完整名單在目錄中。
beta-open = 開啟目錄
beta-own = 架設你自己的
source-eyebrow = 原始碼
source-title = 開放原始碼，而且會一直如此
source-promise = Baylee 將永遠開放原始碼、永遠免費。沒有付費牆、沒有付費功能、沒有廣告。現在沒有，以後也不會有。程式碼採用 GNU Affero General Public License，這表示任何人若為他人運行修改過的 Gateway，也必須向他們提供其原始碼。
source-rust = 它以 Rust 寫成，是一個人的專案，並得到程式碼代理的大量協助，而每次變更都通過同一道關卡：格式、程式碼檢查、測試套件，以及將每張牌對照其印刷文字的驗證。歡迎提出 issue 與 pull request；<agents>AGENTS.md</agents> 與 <claude>CLAUDE.md</claude> 是人和代理共同遵守的工作規則。
source-discord = Discord 上的社群
source-whatsapp = WhatsApp 上的社群
source-architecture = 架構
source-repoTitle = 儲存庫目前的狀況
source-stars = 星數
source-issues = 未解決的 issue 與 pull request
source-note = 由本站伺服器每十分鐘從 GitHub 的公開 API 讀取。你的瀏覽器不會向 GitHub 查詢任何東西。
gw-count =
    { $n ->
       *[other] 已列出 { $n } 個 Gateways，其中 { $online } 個在線。
    }
gw-none = 目前還沒有列出任何 Gateway。
contact-intro = 一個問題、一個想法、一款你想看到的遊戲，或是你不想公開張貼的事：這個表單會直接送到 Baylee 背後的那個人手上。如果想要回覆，請留下電子郵件地址；沒有的話，我們只能閱讀。
contact-f8 = 結算錯誤的牌，最好在遊戲中按 <kbd>F8</kbd> 回報，因為報告可以附上整場遊戲。
contact-discord = 聊天、找對手、搶先知道新版本
contact-issues = GitHub issues
contact-issuesSub = 錯誤與想法，公開討論
contact-operator = 誰經營本站，以及郵寄地址：見<impressum>法律聲明</impressum>。
contact-whatsappSub = 社群群組與快速對戰
contact-reportsTitle = 報告與隱私權
contact-reportsSub = F8 報告與資料的詳細說明
ai-housePill = 離線與線上
ai-providers = 支援的語言模型
ai-hostsPill = 給 Gateway 主持人
ai-hostsTitle = 在你的 Gateway 上提供託管的 LLM 座位
ai-hosts = 身為 Gateway 營運者，你可以直接在自己的伺服器上提供預先設定好的語言模型。牌桌上的客人只要挑選你託管的其中一個模型當對手就行——不需要 API 金鑰。
ai-trainingPill = 訓練中 · 原生引擎 AI
ai-trainingTitle = 原生 Baylee AI——訓練成像真正的玩家一樣，直接在引擎上遊玩
ai-training = 除了內建 AI 與外部 LLM 橋接之外，一個專屬的原生 Baylee AI 正直接在確定性的規則引擎上接受訓練。它學著像經驗豐富的玩家一樣，評估盤面狀態、優先權時機、戰鬥計算與牌組的綜效——而且完全沒有外部 API 延遲。
ai-hostingLink = Gateway 託管與 LLM 設定

# The gateways and the contact page, as the old site wrote them (owner, 11.11.2026)
gw-lead = Gateway 是線上牌桌所在的伺服器：帳號、牌組、房間，以及每場遊戲各一個引擎程序。這裡列出的是已向 Baylee Central 註冊並經其管理員核准的 Gateway。每一個都由其營運者為自己的群組經營，那裡適用他們自己的規則。列出並不代表對 Gateway 的作為作出任何保證。
gw-howTo = 想在其中一個上面遊玩，請把它的位址複製到用戶端的入口畫面。在線表示 Central 在最近一分半鐘內收到過該 Gateway 的訊息；並不保證它會回應。名稱與版本由各營運者自行回報。
gw-find = 尋找 Gateway
gw-findHint = 名稱或位址
gw-shown = 顯示 { $n } / { $all }
gw-noMatch = 目前已載入的 Gateways 中沒有符合的。
gw-showAll = 顯示全部
gw-stats = 目錄概況
gw-stat-listed = 已列出的 Gateway
gw-stat-online = 目前線上
gw-stat-players = 線上玩家
gw-stat-playing = 其中遊戲中
gw-stat-tables = 開放牌桌
gw-stat-games = 進行中的遊戲
gw-show = 顯示
gw-show-every = 全部
gw-show-online = 線上
gw-show-tables = 有開放牌桌
gw-sort = 排序
gw-sort-best = 推薦
gw-sort-name = 依名稱
gw-sort-players = 玩家最多
gw-sort-tables = 開放牌桌最多
gw-sort-version = 最新版本
gw-view = 檢視
gw-view-cards = 格狀
gw-view-list = 列表
link-readmeGateway = #running-your-own-gateway
selfhost-eyebrow = 自行架設與 Central 整合
selfhost-title = 架設你自己的 Gateway
selfhost-processes = 自行架設的 Gateway 會在 PostgreSQL 18+ 旁邊執行三個輕量的程序：<code>baylee-gateway</code> 管理房間、牌組與路由；<code>baylee-agent</code> 為每場對戰啟動一個獨立的 <code>baylee-engine-server</code> 程序，讓每張牌桌都在沙盒中。
selfhost-proxy = 在你的 HTTPS 來源（例如 <code>https://play.example.org</code>）前面放一個 TLS 反向代理，可選擇匯入 Scryfall 的大量目錄，取得 19 種語言的牌面文字，之後玩家就能直接在用戶端中透過你的位址連線。
selfhost-central = <b>連結 Baylee Central 並設定隱私：</b>在<mine>你的 Gateways</mine>下註冊你的 HTTPS 來源，並產生一組 <code>bgt_…</code> 權杖（範圍：<code>sso</code> 用於單一登入，<code>listing</code> 用於目錄心跳與即時遙測）。在你 Gateway 的主控台中，選擇遙測的隱私等級（<code>full</code> 包含進行中的遊戲、牌桌設定與選擇加入的玩家，<code>counts_only</code> 只含總數，<code>private</code> 只表示存活狀態）。
selfhost-guide = <guide>閱讀完整的自行架設指南</guide> · <protocol>通訊協定與心跳參考（docs/protocol.md）</protocol>
selfhost-source = 每個 Gateway 都會在不需登入的情況下，以 <code>GET /source</code> 回覆它的授權條款、版本與提交，自動滿足 AGPL 的原始碼揭露要求。
reports-eyebrow = 報告
reports-title = 發現有問題？告訴我們
reports-f8 = 在牌桌或大廳中按 <kbd>F8</kbd>。報告包含它的種類、你的文字、用戶端版本，以及在牌桌上時該場遊戲的 ID。其他一切都有各自的核取方塊，而且每個方塊一開始都是勾選的，所以在按下送出之前，請取消勾選你想保留的項目：系統與硬體、你所見的牌桌、你的紀錄檔（其他玩家改名為「Player A」、「Player B」……）、你的設定、僅限 Baylee 視窗的畫面，以及在牌桌上時整場遊戲的完整紀錄（每一步與每個座位的牌，不含姓名）。<i>Show what is sent</i> 會顯示每一個位元組。工作階段權杖絕不會送出。
reports-usage = 你離線遊玩時，用戶端每分鐘會說一次有一場遊戲正在進行：沒有帳號、沒有裝置 ID、沒有任何關於該遊戲的資訊。在 Settings › Privacy & Data 下的 <i>Send anonymous usage count</i> 可以把它關掉。
reports-discord = 想聊天、找對手，或最先知道新版本？加入 <discord>Discord 上的 Baylee 社群</discord>。
reports-issues = 規則問題、牌的行為錯誤、想法：<issues>到 GitHub 開一個 issue</issues>。結算錯誤的牌是我們能收到的最有用的報告，因為每張牌都有一個測試，而你的回報就會變成其中一個。
reports-privacy = <docs>docs/privacy.md</docs> 列出了用戶端、Gateway 與意見回饋服務保留的一切。除非你登入，否則本站不設定 Cookie，不做分析，也不從別人的伺服器載入字型或指令碼。<notice>隱私權聲明</notice>說明了本站與 Baylee Central 保留什麼，並指出它們連線的每一個其他主機，以及何時連線。

# The download page, as the old site set it (owner, 11.11.2026)
dlPage-eyebrow = 取得
dlPage-filesEyebrow = 所有檔案
dlPage-filesTitle = 所有系統
install-eyebrow = 安裝
install-releases = GitHub 上的所有版本
release-notes = 版本說明與校驗碼
release-all = 所有版本

# The download page's head, as the old site wrote it
dlPage-title = 下載
dlPage-lead = 每個下載都直接連到 GitHub 上的檔案，中間沒有任何東西。如果某個版本有安裝程式，那是最簡單的方式；壓縮檔裝的是同一個遊戲，可以解壓縮到任何地方。只要 Baylee 能寫入自己的資料夾，安裝後的 Baylee 就會依簽署過的壓縮檔自我更新。每個套件都在遊戲旁附有給語言模型用的 Seat-Bridge。

# The reading pages' eyebrows, as the old site set them
imp-eyebrow = 法律聲明
terms-eyebrow = 使用條款
privacy-eyebrow = 隱私權

# The privacy notice's contents, as the old site named them
privacy-contents = 目錄

# The account pages' eyebrow, as the old site set it
acct-eyebrow = Baylee Central

# The account area's navigation, as the old site named it
app-account = 你的帳號

# ==== The security page (/security) ====================================
# Every sentence and every arrow of this page maps to a row of the claim
# table in docs/security-page.md; a claim changes
# there first. No "safe", "unbreakable" or compliance words: mechanisms only.
title-security = Baylee 的安全性：密碼、金鑰與聊天
desc-security = 以四張圖逐步說明 Baylee 如何處理密碼、金鑰與聊天訊息，以及每個流程的限制。
nav-security = 安全性
sec-eyebrow = 安全性
sec-title = 密碼、金鑰與聊天：什麼去了哪裡
sec-lead = 四個流程，逐步說明：什麼會離開你的裝置，Baylee Central 看得到什麼、看不到什麼，以及每個流程的限制。
sec-toc-label = 本頁內容
sec-figure = 分步圖示：{ $name }
sec-play = 播放
sec-all = 全部步驟
sec-step = 步驟 { $n }
sec-hint = 圖示會自動播放。選一個步驟即可停在該處。
sec-hint-still = 選一個步驟即可標示它。
sec-in-browser = 虛線框是你的瀏覽器：這一切都發生在其中。
# The actors' names are the only words inside the drawings, where text does
# not wrap: one or two words each, at most about twice the English width
# (the test `names_fit_with_room_to_grow` measures every language).
sec-actor-device = 你的裝置
sec-actor-central = Central
sec-actor-page = 頁面
sec-actor-worker = Worker
sec-actor-store = 儲存空間
sec-actor-you = 你
sec-actor-contact = 聯絡人
sec-actor-website = 網站
sec-actor-client = 用戶端

sec-signin-eyebrow = 登入與註冊
sec-signin-title = Central 在不收到密碼的情況下檢查密碼
sec-signin-lead = Baylee 以 OPAQUE（RFC 9807）讓你登入。你的裝置與 Central 交換三則訊息；你的密碼不在其中任何一則裡，連單純的雜湊值也沒有。
sec-signin-c1 = 盲化請求
sec-signin-t1 = 你的裝置將密碼盲化。
sec-signin-s1 = 它把密碼與一個全新的隨機數結合，只送出結果。從這則訊息無法讀出密碼。
sec-signin-c2 = Central 的回應
sec-signin-t2 = Central 以自己的金鑰回應。
sec-signin-s2 = Central 用只有它持有的金鑰對盲化後的值進行運算，並附上它那一半的金鑰交換。這個過程中它不會得知密碼。
sec-signin-c3 = Argon2id · 64 MiB
sec-signin-t3 = 繁重的運算在你的裝置上進行。
sec-signin-s3 = 裝置移除盲化，並用 Argon2id（64 MiB 記憶體、三輪）延展結果；Central 無法要求較省力的設定。密碼若有誤，會在這裡就被發現，那時還沒有送出其他任何東西。
sec-signin-c4 = 證明
sec-signin-t4 = 你的裝置證明自己擁有正確的金鑰。
sec-signin-s4 = 密碼正確時，你的裝置會得出與 Central 相同的工作階段金鑰，並送出一份簡短的證明（64 位元組）。
sec-signin-c5 = 工作階段 + 包裝
sec-signin-t5 = Central 開啟工作階段。
sec-signin-s5 = 回來的有工作階段（在瀏覽器中是頁面本身讀不到的 Cookie），以及放在包裝中的主金鑰；這個包裝只在你的裝置上、輸入正確密碼時才能打開。
sec-signin-register = <b>註冊</b>使用同一套協定。你的裝置抽出一把隨機的主金鑰（256 位元），用你的密碼包裝一次、用你的救援碼再包裝一次，並只送出這些包裝。Central 儲存的是一筆 OPAQUE 紀錄和這些包裝，絕不儲存金鑰本身。
sec-signin-legacy = <b>一個例外：</b>尚未移轉到 Central 的舊官方 Gateway 帳號。在這類帳號移轉完成之前，Baylee 用戶端中的每次登入嘗試，都會把輸入的密碼透過已驗證的 TLS 連線送到 Central，由 Central 對照舊的雜湊值檢查，並且既不儲存也不記錄。移轉成功後，該帳號就會如上所示登入。本網站不會讓這類帳號登入，而是引導你使用用戶端。這個例外隨移轉所訂的期限結束。
sec-signin-limit = <b>仍然存在的情況：</b>任何人只要取得 Central 的資料庫連同它的金鑰，就可以一個一個嘗試密碼；每次嘗試都要花一次 Argon2id 運算。以這種方式找出的密碼，能打開該帳號的加密資料與已儲存的聊天。一組很長、而且你在其他地方都沒用過的密碼仍然很重要。
sec-signin-guess = <b>透過登入來猜測密碼</b>只會被減慢，不會被阻止。Central 統計未以正確密碼完成的嘗試：對同一個帳號，來自該帳號未曾登入過的位址的嘗試，每天最多 100 次；來自同一個位址、針對所有帳號合計的嘗試，每天最多 100 次。超過後，Central 會回應嘗試次數過多，直到這一小時或這一天結束。常見的密碼這樣仍會被找到。而且，知道你登入名稱的人可以用光你帳號的嘗試次數：此後到當天結束為止，你只能從你的帳號在最近 30 天內登入過的位址登入（Central 只會記住這些位址，直到它重新啟動）。

sec-memory-eyebrow = 在記憶體中
sec-memory-title = 登入後，記憶體中還留著什麼
sec-memory-lead = 密碼只需用一次：用來打開你的主金鑰。圖示追蹤它在瀏覽器中的去向；Baylee 用戶端的做法有何不同，見下方。
sec-memory-c1 = 密碼
sec-memory-t1 = 頁面把密碼交給 Worker。
sec-memory-s1 = 表單把密碼交給自己專屬的背景執行緒（Worker），清空輸入欄位，並釋放它自己持有的那份複本。
sec-memory-c2 = OPAQUE + Argon2id
sec-memory-t2 = Worker 運算，然後覆寫。
sec-memory-s2 = 它的 Rust 程式碼只在 OPAQUE 步驟結束前持有密碼，之後就把它覆寫，連同過程中衍生出且由它自己持有的金鑰，以及 Argon2id 的 64 MiB 工作記憶體。
sec-memory-c3 = 主金鑰：檢查後清除
sec-memory-t3 = 主金鑰被打開、使用，然後清除。
sec-memory-s3 = Worker 打開主金鑰以檢查登入，若你的帳號有聊天，就由它衍生出聊天金鑰，然後再次覆寫主金鑰。
sec-memory-c4 = 聊天金鑰，已封存
sec-memory-t4 = 只有聊天金鑰會留下。
sec-memory-s4 = 為了讓聊天在重新載入後不需要你的密碼就能開啟，Worker 會把聊天金鑰以 AES-256-GCM 加密後存在瀏覽器中（IndexedDB）。用來加密的金鑰是不可匯出的：指令碼可以使用它，但無法把它讀出來。聊天頁面開啟期間，另有一個專屬的 Worker 把聊天金鑰以未加密的形式放在記憶體中。登出會刪除該紀錄；工作階段自行結束後，本站下一個載入的頁面會刪除它。
sec-memory-c5 = Worker 已結束
sec-memory-t5 = 登入用的 Worker 已結束。
sec-memory-s5 = 登入完成後，登入頁面會結束它的 Worker，瀏覽器則會丟棄該 Worker 的記憶體。
sec-memory-limit = <b>瀏覽器不承諾的事：</b>輸入時的密碼、它交給 Worker 的過程，以及 JavaScript 的字串，都由瀏覽器管理。Baylee 會釋放它們；但不會覆寫頁面自己的複本，也無法覆寫瀏覽器的複本。那塊記憶體何時真正被重複使用，由瀏覽器決定。而且，在你登入期間，任何能在本站執行指令碼、或能讀取你瀏覽器設定檔的人，都可以取得已封存的聊天金鑰。
sec-memory-client = <b>在 Baylee 用戶端中，</b>輸入的密碼放在釋放時會被覆寫的記憶體裡，並在登入後立即丟棄；它絕不會被儲存。主金鑰在你登入期間留在用戶端的記憶體中，因為用戶端要用它打開你的加密資料。

sec-chat-eyebrow = 聊天
sec-chat-title = 訊息在你的裝置上加密
sec-chat-lead = 每則訊息都有自己的金鑰。Central 只儲存與轉送加密後的內容；要讀取，需要有一個收件帳號的聊天金鑰，因為訊息金鑰就是為它包裝的。
sec-chat-c1 = 文字 + 一次性金鑰
sec-chat-t1 = 每則訊息一把新金鑰。
sec-chat-s1 = 你的裝置抽出一把隨機的訊息金鑰，並用它加密文字（XChaCha20-Poly1305）。
sec-chat-c2 = 逐聊天金鑰包裝，已簽章
sec-chat-t2 = 為每位收件人的聊天金鑰包裝，並用你的聊天金鑰簽章。
sec-chat-s2 = 裝置為收件人帳號登記的每一把聊天金鑰，各自單獨包裝一次訊息金鑰（X25519），並用你自己的聊天金鑰對標頭與密文簽章（Ed25519）。目前一個帳號只有一把聊天金鑰，由其主金鑰衍生，在它登入的每個瀏覽器和用戶端中都相同。
sec-chat-c3 = 密文 + 包裝
sec-chat-t3 = Central 只收到密文。
sec-chat-s3 = Central 檢查簽章與收件人名單，並把密文連同這些包裝一起儲存。它既沒有訊息金鑰，也沒有你帳號的金鑰。
sec-chat-c4 = 密文 + 一份包裝
sec-chat-t4 = 每把聊天金鑰各得一份包裝。
sec-chat-s4 = 你的聯絡人擷取他們的訊息時，Central 交出密文，以及只為那把聊天金鑰做的那一份包裝。
sec-chat-c5 = 已檢查、已開啟
sec-chat-t5 = 檢查、解開包裝、閱讀。
sec-chat-s5 = 你聯絡人的裝置檢查你的簽章，用它的聊天金鑰解開訊息金鑰的包裝，並解密文字。
sec-chat-meta = <b>Central 仍然看得到的：</b>誰和誰有聯絡、群組及其名稱與成員、顯示名稱、公開的聊天金鑰，以及每則訊息是誰傳的、在哪個對話、何時傳的、有多大。和任何伺服器一樣，它也看得到請求來自哪個位址。
sec-chat-limit = <b>限制：</b>這裡沒有雙棘輪，因此沒有前向保密：如果收件人的聊天金鑰日後被竊，它也能打開先前為它包裝的舊訊息。你的聊天金鑰衍生自你帳號的主金鑰，而 Central 在送達後仍保留加密的訊息：任何人只要取得 Central 的資料庫連同它的金鑰並猜出你的密碼，或握有你的救援碼，就能讀取這些訊息。第一次接觸時，你的裝置會信任 Central 所指的金鑰；若要檢查，請透過另一個管道比對安全碼。如果聯絡人的金鑰日後改變，要等你確認之後，Baylee 才會再次傳送。誰屬於某個群組，由 Central 決定。

sec-pair-eyebrow = 以連結登入
sec-pair-title = 從瀏覽器進入用戶端，不必再輸入一次密碼
sec-pair-lead = 在網站上已登入的人，應該能以已登入的狀態啟動 Baylee 用戶端。為此，頁面會把主金鑰帶給用戶端：經過封存，而且 Central 既無法打開它，也無法塞入一把它自己的金鑰。
sec-pair-status = <b>目前的進度：</b>Central 與 Baylee 的用戶端函式庫今天已能做到這件事，並且已互相測試。但還不能從這個網站或已發行的用戶端使用。
sec-pair-c1 = 開始配對
sec-pair-t1 = 頁面開始一次配對。
sec-pair-s1 = Central 為它發出一個有效期五分鐘的代碼。同時，頁面抽出一個一次性的祕密，且絕不顯示給 Central。
sec-pair-c2 = 附帶祕密的連結
sec-pair-t2 = 一個連結開啟用戶端。
sec-pair-s2 = 連結 <code>baylee://pair</code> 把代碼與祕密從頁面帶到你自己裝置上的用戶端，繞過 Central。
sec-pair-c3 = 公開金鑰
sec-pair-t3 = 用戶端以一組全新的金鑰對回應。
sec-pair-s3 = 它把公開的那一半，以及用祕密算出的檢查值（HMAC-SHA-256）送給 Central。頁面會驗證這個值；兩個畫面顯示相同的短代碼，由你在頁面上核准。
sec-pair-c4 = 主金鑰，已封存
sec-pair-t4 = 頁面為用戶端封存主金鑰。
sec-pair-s4 = 使用 HPKE（RFC 9180），針對用戶端的公開金鑰，並同樣以祕密封存。Central 只儲存這份封存一次；它無法打開，而且因為不知道祕密，也無法自行做出一份。
sec-pair-c5 = 工作階段 + 封存
sec-pair-t5 = 用戶端兌換一次。
sec-pair-s5 = 它同時收到一個工作階段與這份封存，用自己的私密金鑰和祕密打開封存，然後就登入了。事後配對會被刪除，兌換失敗時也一樣。
sec-pair-note = <b>密碼不參與這次交換。</b>頁面只在剛用你的密碼打開主金鑰的那一刻持有它。以這種方式配對的工作階段，視為不比它所來自頁面上的密碼登入更新：配對絕不會讓它更新。

sec-limits-eyebrow = 限制
sec-limits-title = 以上這一切做不到的事
sec-limit-device = <b>遭入侵的裝置。</b>惡意軟體、會讀取頁面的瀏覽器擴充功能，或被竄改的用戶端，都會看到你所看到、所輸入的內容。傳往伺服器途中的任何加密，對此都沒有幫助。
sec-limit-site = <b>網站的程式碼來自它的伺服器。</b>任何接管我們伺服器的人，都可能送出在你輸入密碼時將其擷取的程式碼；完整性雜湊與嚴格的 Content-Security-Policy能防範來自外部的更動，但防不了這種情況。用戶端是你機器上的程式，並會在安裝更新前檢查簽章。
sec-limit-central = <b>被接管的 Central。</b>它可能洩露中繼資料、拒絕服務，並攻擊登入。它可以在第一次接觸時指出假的金鑰，或把成員加入群組，從而讀到之後傳送的訊息。它所儲存的你的資料與聊天都是加密的；要打開，需要你的密碼或你的救援碼。
sec-limit-recovery = <b>遺失金鑰。</b>沒有你的密碼，也沒有你的救援碼，沒有人能找回你的加密資料，我們也不行：Central 只有包裝。
sec-limit-audit = <b>沒有獨立稽核。</b>登入依循 OPAQUE（RFC 9807）與 Argon2id（RFC 9106），配對則用 HPKE（RFC 9180）封存：這些是 IRTF 發布的規格，並非 IETF 標準。配對步驟的順序與聊天格式，是 Baylee 自己組合標準建構元件（Ed25519、X25519、XChaCha20-Poly1305）而成。到目前為止，只有開發者本人審查過這個實作。程式碼是公開的；如果你發現了什麼，<contact>請告訴我們</contact>。
sec-docs-title = 延伸閱讀
sec-docs = 細節在專案的文件中：<protocol>加密協定</protocol>、<chat>聊天</chat>、<accounts>帳號與配對</accounts>，以及<threats>威脅模型</threats>。你的資料在法律上如何處理，見<privacy>隱私權聲明</privacy>。

# The page's two links from elsewhere: the FAQ and the sign-in page.
faq-security-q = Baylee 如何處理我的密碼和聊天？
faq-security-a = 你登入時，裝置用 OPAQUE 協定證明自己知道密碼，而不必送出密碼；聊天訊息在你的裝置上加密。<security>安全性頁面</security>以圖示呈現每個步驟，包括那個唯一的例外與各項限制。
sec-signin-link = 登入時你的密碼會怎麼處理：<security>安全性</security>
sec-teaser-lead = 逐步說明：什麼會離開你的裝置，Baylee Central 看得到什麼、看不到什麼，以及每個流程的限制。
sec-teaser-more = 以圖示查看每個步驟
# ==== end of the security page =================================================
# ==== registration: the proof of work, while it takes long =====================
register-solving = 你的瀏覽器正在解一道把機器人擋在外面的謎題。建立帳號的人很多時，這可能需要幾分鐘 …… { $percent } %

## An administrator makes a reset code (/account/admin)
admin-reset-title = 重設登入
admin-reset-lead = 找到向你求助的那個人的帳號，為它產生一組重設碼。他們用這組碼在重設頁面上設定新密碼。
admin-find = 尋找帳號
admin-only = 這個頁面只供管理員使用。
admin-search = 搜尋帳號
admin-search-hint = 登入名稱或顯示名稱的一部分，或 # 加上標籤。留空則列出所有帳號，最新的在前。
admin-search-submit = 搜尋
admin-search-error = 搜尋內容須為 1 到 64 個不含重音的一般字元，或是 # 加上標籤，例如 #1a2b。
admin-count = 顯示的帳號：{ $count }
admin-none = 沒有符合此搜尋的帳號。
admin-more = 顯示更多
admin-open = 開啟
admin-back = 返回
admin-pill = 管理員
admin-state = 狀態
admin-state-pending = 已保留，從未設定
admin-state-enrolled = 已設定
admin-state-restored = 使用中
admin-created = 建立時間
admin-last-sign-in = 上次登入
admin-recovery = 救援碼
admin-yes = 是
admin-no = 否
admin-reset-outstanding = 有一組未使用的重設碼，有效期限到 { $time }。它是管理員產生的，還是透過電子郵件寄出的，Central 並不會說明。新的碼會取代它。
admin-reset = 重設登入
admin-refused-admin = 這個帳號擁有管理員角色，而 Central 不會為管理員的帳號產生重設碼。請先移除該角色。
admin-refused-own = 這是你自己的帳號，沒有人可以為自己產生重設碼。請改在「安全性」中變更你的密碼。
admin-refused-pending = 這個帳號已保留但從未設定，所以沒有可重設的登入。
admin-refused-moving = 這個帳號從官方 Gateway 的移轉尚未完成。在完成之前，它的登入要在那裡重設，而不是在這裡。
admin-refused-stale = Baylee Central 認為你的登入已超過 30 分鐘。請再試一次；你需要再輸入一次密碼。
admin-refused-role = 你的帳號已不再擁有管理員角色。
admin-refused-gone = 這個帳號已不存在。
admin-switched = 這個瀏覽器現在已登入另一個帳號，所以沒有要求產生重設碼。請重新載入頁面。
admin-lost = 沒有收到回應，所以我們無法確定是否已產生重設碼。如果已產生，沒有人能再次顯示它，而且它已取代了先前的任何碼。請再產生一組新的：它會反過來取代那一組。
admin-warning-title = 在你產生重設碼之前
admin-warning-who = 請在本站之外確認向你求助的人確實擁有這個帳號，例如打電話，或透過你本來就用來認識他們的管道。持有這組碼的人，可以設定新密碼。
admin-warning-ends = 產生重設碼不會終止任何東西：在碼被使用之前，舊密碼仍然有效，所有裝置也都維持登入。
admin-warning-data = 只有憑他們的救援碼，或在仍保持登入的裝置上，那個人的加密資料才能在重設後保留下來。兩者都沒有的話，他們會從空保險庫開始。
admin-warning-replaces = 新的碼會取代先前的任何碼，包括以電子郵件寄出的那一組。它的有效期限為 24 小時，只能使用一次。
admin-warning-takeover = 重設碼是用來應對忘記密碼，而不是帳號被盜用：入侵者的工作階段與舊密碼在碼被使用之前都會一直存在；如果碼一直沒被使用，它們就永遠存在。
admin-warning-continue = 我已確認是誰在求助
admin-password-hint = 產生重設碼需要不到 30 分鐘前的登入，所以你要再輸入一次密碼。它只會留在這個瀏覽器中。
admin-password-missing = 請輸入你的密碼。
admin-make = 產生重設碼
admin-code-title = 重設碼
admin-code-until = 有效期限到 { $time }，而且只能使用一次。
admin-code-lead = 請透過你信任的管道交給向你求助的人。它只會在現在顯示：Baylee Central 和這個頁面都無法再次顯示。如果弄丟了，請再產生一組新的。
admin-code-done = 完成

# ==== the administration's other pages (islands/admin/) ========================
# Translated by a model (Claude Sonnet) on 12.11.2026; no native speaker has read it yet.
admin-accounts = 帳號
admin-account = 一個帳號
admin-audit = 稽核紀錄
admin-legacy = 舊帳號
admin-feedback = 意見回饋
admin-ov-lead = 共有多少帳號和 Gateway，以及有哪些事在等你處理。
admin-ov-accounts = 本週新增 { $week } 個，從官方 Gateway 移轉了 { $moved } 個。
admin-ov-accounts-go = 所有帳號
admin-ov-pending = 請先確認是誰在營運某個 Gateway 的位址，再核准它。
admin-ov-pending-none = 沒有等你處理的事。
admin-ov-pending-go = 審查 Gateway
admin-ov-gateways = 清單中的 Gateway
admin-ov-gateways-sub = 目前在線 { $online } 個，已停用 { $disabled } 個，有效權杖 { $tokens } 個。
admin-ov-gateways-go = 已核准的 Gateway
admin-ov-admins = 管理員
admin-ov-admins-sub = 他們做的每項變更都記錄在稽核紀錄中。
admin-ov-legacy = 待移轉
admin-ov-legacy-sub = 官方 Gateway 中尚未移轉的帳號，期限為 { $day }。
admin-ov-reports = 未處理的報告
admin-ov-crashes = 等你處理的新當機：{ $n } 個。
admin-ov-calm = 沒有新的當機。來自玩家的錯誤、想法與意見回饋。
admin-ov-reports-go = 分類報告
admin-ov-failed = Baylee Central 沒有告知數量。
admin-id = ID
admin-tag = 標籤
admin-role = 角色
admin-member = 成員
admin-tag-lead = 標籤在 Baylee Central 是唯一的，所以 Name#tag 也是唯一的。請用十六進位書寫，例如 #1a2b。每次變更都會連同舊標籤與新標籤記錄在稽核紀錄中。
admin-tag-set = 設定標籤
admin-tag-confirm = 要把標籤 { $tag } 給 { $name } 嗎？
admin-tag-warn = Central 聊天設定完成之後，標籤會固定不變，以保留已簽署的訊息與安全碼。對於沒有 Central 聊天的帳號，Gateway 會在下次登入時採用新標籤；變更之後，先前與 Gateway 綁定的密語紀錄不會再顯示。
admin-tag-error = 標籤是 1 到 8 位十六進位數字，範圍從 #1 到 #7fffffff。
admin-tag-saved = 標籤已設定。
admin-tag-taken = Baylee Central 拒絕了這個標籤：另一個帳號正在使用，或者這個帳號使用 Central 聊天，而 Central 聊天會保留它的標籤。
admin-role-grant = 設為管理員
admin-role-remove = 移除管理員角色
admin-role-grant-title = 要把 { $name } 設為管理員嗎？
admin-role-remove-title = 要移除 { $name } 的管理員角色嗎？
admin-role-grant-text = 管理員可以查看所有帳號與 Gateway，也可以變更它們。
admin-role-remove-text = 這個帳號下一次以管理員身分送出的要求會被拒絕。
admin-role-granted = 這個帳號現在是管理員。
admin-role-removed = 管理員角色已移除。
admin-user-gateways = 它註冊的 Gateway
admin-user-audit = 最新稽核事件
admin-change-password = 這項變更需要 30 分鐘內的登入，所以你要再輸入一次密碼。它只會留在這個瀏覽器中。
admin-change-lost = 沒有收到回應，所以我們無法確定變更是否已生效。頁面已重新讀取：再試一次之前，請先查看它顯示的內容。
admin-change-unknown = 沒有收到回應，所以我們無法確定變更是否已生效。再試一次之前，請先重新載入頁面。
admin-change-switched = 這個瀏覽器現在登入的是另一個帳號，所以沒有變更任何內容。請重新載入頁面。
admin-refused-gateway = 這個 Gateway 已不存在。
admin-working = 正在詢問 Baylee Central……
admin-gw-lead = 請先在 Baylee Central 之外確認擁有者確實在營運某個 Gateway 的位址，再核准它進入公開清單；即使沒有核准，它的權杖也能讓玩家登入。停用它會把它從清單中移除，並結束其玩家在那裡的登入。
admin-gw-none = 沒有處於這個狀態的 Gateway。
admin-gw-official = 官方
admin-gw-owner = 擁有者
admin-gw-bridge = 橋接（官方）
admin-gw-seen = 上次心跳
admin-gw-tokens = 有效權杖
admin-gw-pair = 以及一組憑證
admin-gw-approve = 核准
admin-gw-enable = 重新啟用
admin-gw-disable = 停用
admin-gw-show-tokens = 顯示權杖
admin-gw-hide-tokens = 隱藏權杖
admin-gw-no-tokens = 還沒有權杖。
admin-gw-used = 上次使用：{ $time }
admin-gw-approve-title = 要核准 { $name } 嗎？
admin-gw-approve-text = 只有在確認擁有者確實在營運 { $origin } 時才核准。核准後，它會出現在公開清單中。
admin-gw-disable-title = 要停用 { $name } 嗎？
admin-gw-disable-text = { $origin } 會立即從公開清單中消失，它的權杖會停止生效，在那裡登入的每位玩家都會在下次檢查時被登出。
admin-gw-approved = 這個 Gateway 已核准。
admin-gw-disabled = 這個 Gateway 已停用。
admin-gw-revoke-title = 要撤銷權杖「{ $name }」嗎？
admin-gw-revoke-text = 權杖會立即停止生效。使用它的 Gateway 在取得另一個權杖之前，無法再讓玩家登入，也無法保留它在清單中的項目。
admin-audit-lead = 管理員與營運者所做的變更，最新的在前。紀錄保存的是 ID；旁邊顯示的名稱是今天的名稱。
admin-audit-events = 事件
admin-audit-none = 還沒有任何紀錄。
admin-audit-when = 時間
admin-audit-what = 內容
admin-audit-by = 操作者
admin-audit-about = 涉及
admin-audit-operator = 營運者
admin-audit-admin-granted = 設為管理員
admin-audit-admin-removed = 移除管理員角色
admin-audit-reset-issued = 產生重設碼
admin-audit-tag-changed = 變更標籤
admin-audit-gateway-approved = 核准 Gateway
admin-audit-gateway-disabled = 停用 Gateway
admin-audit-token-revoked = 撤銷 Gateway 權杖
admin-audit-registration-closed = 關閉註冊
admin-audit-registration-opened = 開放註冊
admin-legacy-lead = 早於 Baylee Central 的官方 Gateway 帳號中，尚未移轉的那些。每個帳號在用戶端首次登入時用舊密碼移轉，移轉完成後它那一列就會被刪除。只有名稱、時間與計數：絕不會有密碼雜湊值。
admin-legacy-none = 所有帳號都已移轉，沒有剩下的。
admin-legacy-no-store = 這個 Central 沒有保存舊帳號。
admin-legacy-no-deadline = 沒有設定期限：這些列會一直保留，直到營運者設定期限。
admin-legacy-deadline = 剩餘：{ $count } 個。{ $day } 之後，每日清理會刪除仍留在這裡的每一列。
admin-legacy-move = 移轉
admin-legacy-held = 暫緩
admin-legacy-failures = 登入失敗
admin-legacy-imported = 已匯入
admin-legacy-unstarted = 未開始
admin-legacy-reserved = 名稱已保留
admin-legacy-enrolled = 已設定
admin-legacy-restored = 已還原
admin-legacy-banned = 已停權
admin-legacy-must-change = 需要新密碼
admin-legacy-locked = 鎖定到 { $until }



## The feedback moderation (/account/admin/feedback)

fb-title = 意見回饋與當機報告
fb-report-title = 報告
fb-reports = 報告
fb-refresh = 重新整理
fb-views = 檢視
fb-view-inbox = 收件匣
fb-view-open = 未完成
fb-view-mine = 我負責的
fb-view-unassigned = 未指派
fb-view-done = 已完成
fb-view-all = 全部
fb-view-trash = 垃圾桶
fb-search = 搜尋報告
fb-search-hint = 內文或當機訊息中的字詞，或報告 ID 的一部分。
fb-search-submit = 搜尋
fb-filters = 篩選
fb-filters-n = 篩選（{ $n }）
fb-status = 狀態
fb-kind = 類型
fb-priority = 優先順序
fb-assignee = 負責人
fb-label = 標籤
fb-labels = 標籤
fb-gateway = Gateway
fb-channel = 管道
fb-record = 對局紀錄
fb-reporter = 回報者
fb-from = 起始
fb-to = 截止
fb-sort = 排序
fb-only-unread = 只顯示我還沒開啟過的報告
fb-any = 不限
fb-anyone = 任何人
fb-nobody = 無人
fb-me = 我
fb-with-record = 有紀錄
fb-without-record = 無紀錄
fb-channel-gateway = 透過 Gateway
fb-channel-direct = 直接送出，不經過 Gateway
fb-ascending = { $sort }，遞增
fb-descending = { $sort }，遞減
fb-sort-received = 收到時間
fb-sort-changed = 最近變更
fb-sort-priority = 優先順序
fb-sort-status = 狀態
fb-sort-kind = 類型
fb-sort-reporter = 回報者
fb-sort-gateway = Gateway
fb-kind-bug = 錯誤
fb-kind-improvement = 改進建議
fb-kind-feedback = 意見回饋
fb-kind-crash = 當機
fb-kind-other = 其他
fb-status-new = 新增
fb-status-triaged = 已分類
fb-status-in-progress = 處理中
fb-status-resolved = 已解決
fb-status-wont-fix = 不予修正
fb-status-duplicate = 重複
fb-prio-critical = 緊急
fb-prio-high = 高
fb-prio-normal = 一般
fb-prio-low = 低
fb-prio-none = 無優先順序
fb-active-filters = 使用中的篩選
fb-remove-filter = 移除篩選「{ $what }」
fb-clear-filters = 清除篩選
fb-crash-group = 當機群組 { $id }
fb-duplicates-of = …{ $id } 的重複項
fb-search-pill = 搜尋：{ $text }
fb-saved-views = 已儲存的清單
fb-view-name = 這份清單的名稱
fb-save-view = 儲存這份清單
fb-no-views = 還沒有已儲存的清單。
fb-remove-view = 移除已儲存的清單「{ $name }」
fb-views-full = 你已儲存 { $n } 份清單，已達上限。移除一份後才能再儲存。
fb-view-saved = 已儲存清單「{ $name }」。
fb-view-removed = 已移除清單「{ $name }」。
fb-err-views = 無法讀取已儲存的清單。
fb-chart = 最近 30 天
fb-chart-others = 報告：{ $n }
fb-chart-crashes = 當機：{ $n }
fb-chart-day = { $day } · 報告：{ $n } · 當機：{ $crashes }
fb-chart-table = 以表格顯示每天
fb-chart-day-head = 日期
fb-chart-crashes-head = 當機
fb-err-chart = 無法讀取最近 30 天的數據。
fb-trash-hint = 垃圾桶中的報告會在 { $days } 天後永久刪除。
fb-range = 第 { $from } 至 { $to } 筆，共 { $total } 筆
fb-unread-here = { $n } 筆未開啟
fb-none = 沒有符合這份清單的報告。
fb-export-all = 全部匯出為 CSV（{ $n }）
fb-shown-as = 顯示方式
fb-as-list = 清單
fb-as-groups = 當機群組
fb-select-all = 選取本頁所有報告
fb-select-one = 選取：{ $what }
fb-col-report = 報告
fb-unread = 未開啟
fb-no-text = （無內文）
fb-notes-n = 備註：{ $n }
fb-dup-of = …{ $id } 的重複項
fb-record-mark = 紀錄，{ $size }
fb-by-client-short = 由用戶端寫入
fb-new-tab = （在新分頁中開啟）
fb-pages = 頁碼
fb-prev-page = 上一頁
fb-next-page = 下一頁
fb-page-of = 第 { $n } 頁，共 { $total } 頁
fb-page-size = 每頁報告數
fb-sort-by = 依「{ $column }」排序
fb-only-label = 只顯示標籤「{ $label }」
fb-only-assignee = 只顯示由 { $name } 負責的報告
fb-only-gateway = 只顯示 Gateway「{ $name }」
fb-only-reporter = 只顯示 { $name } 回報的報告
fb-selection = 所選報告
fb-selected = 已選取：{ $n }
fb-deselect = 取消選取
fb-set-status = 設定狀態…
fb-set-priority = 設定優先順序…
fb-assign-to = 指派給…
fb-add-label = 新增標籤
fb-remove-label-all = 移除標籤…
fb-merge-into = 合併到…
fb-to-trash = 移到垃圾桶
fb-restore = 還原
fb-purge = 永久刪除
fb-mark-read = 標示為已開啟
fb-mark-unread = 標示為未開啟
fb-copy-ids = 複製 ID
fb-export-selected = 匯出為 CSV（{ $n }）
fb-changed-n = 已變更的報告：{ $n }
fb-trashed-n = 已移到垃圾桶：{ $n }
fb-restored-n = 已從垃圾桶還原：{ $n }
fb-purged-n = 已永久刪除：{ $n }
fb-marked-read = 已標示為已開啟：{ $n }
fb-marked-unread = 已標示為未開啟：{ $n }
fb-ids-copied = 已複製的 ID：{ $n }
fb-copy-failed = 在這裡無法複製。
fb-exported = 已匯出的報告：{ $n }
fb-undo = 復原
fb-dismiss = 關閉
fb-purge-title = 永久刪除？
fb-purge-text = 將刪除的報告：{ $n }。它們連同備註、歷程和附件，會從 Baylee Central 永久消失，無法找回。
fb-purge-go = 永久刪除它們
fb-cancel = 取消
fb-purge-password = 永久刪除需要 30 分鐘內的登入，所以你要再輸入一次密碼。它只會留在這個瀏覽器中。
fb-purge-switched = 這個瀏覽器現在已登入另一個帳號，所以沒有刪除任何內容。請重新載入頁面。
fb-purge-refused = Baylee Central 沒有刪除任何內容：有報告已不在垃圾桶中，或已不存在。
fb-purge-lost = 沒有收到回應，所以這個頁面無法確定這些報告是否已被刪除。頁面顯示的是 Baylee Central 現在保存的內容。
fb-groups = 當機群組
fb-col-crash = 當機
fb-col-count = 報告
fb-col-open = 未完成
fb-col-first = 最早
fb-col-last = 最近
fb-show-group = 顯示其報告
fb-no-groups = 這份清單中沒有當機。
fb-steps = 這份報告所在的清單
fb-back-to-list = 回到清單
fb-previous = 上一份報告
fb-next = 下一份報告
fb-position = 第 { $n } 份，共 { $total } 份
fb-not-in-list = 不在這份清單中
fb-copy-link = 複製連結
fb-copy-id = 複製 ID
fb-copied = 已複製。
fb-in-trash-since = 自 { $time } 起在垃圾桶中。此後 { $days } 天將被永久刪除。
fb-wrote = 回報者寫的內容
fb-client-sent = 用戶端送出的內容
fb-facts = 基本資訊
fb-history = 歷程
fb-triage = 分類
fb-notes = 備註
fb-related = 相關清單
fb-trash = 垃圾桶
fb-received = 收到時間
fb-changed = 最近變更
fb-calls-itself = 自稱
fb-gateway-build = Gateway 組建版本
fb-client-build = 用戶端組建版本
fb-game = 對局
fb-record-part = 對局紀錄
fb-no-record = 無
fb-record-here = { $size }，{ $who }，{ $end }
fb-record-awaited = 已宣告 { $size }，尚未送達
fb-by-client = 由用戶端寫入，未經驗證
fb-by-gateway = 由 Gateway 寫入
fb-record-complete = 包含對局的結尾
fb-record-partial = 不包含對局的結尾
fb-download-record = 下載紀錄（.gz）
fb-client-part = 用戶端詳細資料
fb-no-client = 無
fb-client-here = { $size }
fb-client-awaited = 已宣告 { $size }，尚未送達
fb-id = ID
fb-crash = 當機
fb-trash-no-triage = 垃圾桶中的報告無法分類。請先還原。
fb-assign-me = 指派給我
fb-label-hint = 小寫字母、數字和連字號，最多 32 個。
fb-bad-label = 標籤由 1 到 32 個小寫字母、數字和連字號組成。
fb-remove-label = 移除標籤「{ $label }」
fb-labels-full = 一份報告最多有 { $n } 個標籤。
fb-duplicate-of = 重複於
fb-mark-duplicate = 標示為重複於
fb-duplicate-hint = 另一份報告的 ID 或其頁面網址。
fb-mark = 標示
fb-bad-duplicate = 這不是另一份報告的 ID。
fb-not-duplicate = 不是重複項
fb-issue = GitHub Issue
fb-issue-label = 連結 Issue
fb-issue-hint = 其編號（如 311），或其在 Baylee 儲存庫中的網址。
fb-link = 連結
fb-unlink = 取消連結
fb-bad-issue = 這不是 Baylee 儲存庫的 Issue。
fb-new-issue = 在 GitHub 上建立新的 Issue
fb-summary-label = 公開 Issue 的中立摘要
fb-summary-hint = 用你自己的話寫：絕不要使用玩家的原話、暱稱或用戶端送出的任何內容。GitHub 會在摘要已填好的狀態下開啟，你在那裡建立 Issue。
fb-category = 類別
fb-summary-first = 請先寫好摘要。
fb-open-issue = 開啟 GitHub 表單
fb-saved = 已儲存。
fb-conflict = 這段期間另一位管理員變更了這份報告。頁面顯示的是它現在的樣子：請重新決定。
fb-err-refused = Baylee Central 拒絕了這項變更。
fb-err-report = 這份報告不存在，或已不存在。
fb-err-list = Baylee Central 拒絕了這份清單。
fb-lost-list = 沒有收到回應，所以這個頁面無法確定是否已完成。頁面顯示的是 Baylee Central 現在保存的清單。
fb-lost = 沒有收到回應，所以這個頁面無法確定是否已完成。頁面顯示的是 Baylee Central 現在保存的報告：請先確認，再重試。
fb-trashed-one = 已移到垃圾桶。
fb-restored-one = 已從垃圾桶還原。
fb-purged-one = 已永久刪除。
fb-no-notes = 還沒有備註。
fb-add-note = 新增備註
fb-note-hint = 備註僅供內部使用：只有管理員看得到。
fb-chars-left = 剩餘字數：{ $n }
fb-save-note = 儲存備註
fb-remove-note = 移除你在 { $time } 寫的備註
fb-note-saved = 備註已儲存。
fb-note-removed = 備註已移除。
fb-no-history = 還沒有歷程。
fb-h-central = Baylee Central
fb-h-filed = 送出
fb-h-status = 狀態
fb-h-issue = Issue
fb-h-assignee = 負責人
fb-h-priority = 優先順序
fb-h-labels = 標籤
fb-h-duplicate = 重複於
fb-h-trashed = 移到垃圾桶
fb-h-restored = 從垃圾桶還原
fb-h-purged = 永久刪除
fb-h-note = 備註
fb-h-swept = 被自動清理刪除
fb-h-none = 無
fb-h-original-gone = 此後已刪除的報告
fb-h-note-added = 新增
fb-h-note-removed = 移除
fb-same-crash = 同一個當機
fb-its-duplicates = 它的重複項
fb-same-reporter = 同一位回報者
fb-same-kind = 類型為「{ $kind }」的所有報告
fb-trash-lead = 垃圾桶中的報告會從除了垃圾桶以外的所有清單中消失，若不還原，將在 { $days } 天後永久刪除。
fb-no-client-part = 用戶端沒有隨這份報告送出詳細資料。
fb-client-awaiting = 用戶端的詳細資料尚未送達。
fb-err-client-large = 用戶端的詳細資料太大，無法在此顯示。
fb-err-client-shape = 用戶端的詳細資料格式無法被這個頁面讀取。
fb-no-shot = 沒有螢幕截圖。
fb-shot-alt = 寫下報告時用戶端的視窗，{ $width } × { $height } 像素
fb-shot-full = 以原始大小顯示
fb-shot-fit = 符合頁面大小
fb-game-when = 時間點
fb-game-turn = 回合
fb-game-turn-of = { $n }，座位 { $seat } 的回合
fb-game-seat = 座位
fb-game-seat-of = 第 { $seat } 位，共 { $n } 位
fb-game-pending = 對局正在詢問
fb-game-nothing = 無
fb-backtrace = 回溯
fb-log = 對局記錄檔（{ $n } 行）
fb-computer = 電腦
fb-system = 系統
fb-sys-platform = 平台
fb-sys-cpus = 處理器
fb-sys-adapter = 顯示轉接器
fb-sys-backend = 圖形後端
fb-sys-window = 視窗
fb-sys-scale = 縮放
fb-sys-lang = 語言
fb-settings = 設定
fb-everything = 全部，以樹狀顯示
fb-seat = 座位 { $n }
fb-json-all = 顯示全部 { $n } 個字元
fb-json-base64 = base64 的 { $n } 個字元
fb-json-more = 顯示接下來的 { $n } 個（剩餘：{ $left }）
fb-json-items = 項目：{ $n }
fb-json-keys = 鍵：{ $n }
