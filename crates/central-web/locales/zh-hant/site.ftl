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
theme-system = 色彩：跟隨裝置
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

hero-title = <nb>Build your decks.</nb> <nb>Gather your friends.</nb> <glow>Just play.</glow>
hero-lead = 功能完整的牌組建構器、最多八位玩家的對戰（雙人對決、混戰或團隊），以及會檢查每一步的規則引擎，讓每個回合都公平進行，不必有人去翻規則。
hero-offline = 單人對戰與牌組建構器可離線使用，不需要帳號。線上牌桌在 Gateways 上；<host>你也可以自己架設</host>。
hero-account = 想線上遊玩？<register>建立帳號</register>，公開測試期間免費，人人都可以申請；或<signin>登入</signin>。
hero-memorial = 懷念 Baylee：打開她的相簿

status-label = 專案狀態

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
plat-windows-x64-sub = most PCs: 64‑bit Intel or AMD
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
gws-official = 官方
gws-online = 在線
gws-offline = 離線
gws-version = 版本
gws-now = Right now
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
faq-password-a = 你可以用管理員給你的重設碼設定新密碼；目前還不能在這個網站上兌換重設碼，之後會提供。只有在你有建立帳號時拿到的救援碼，或仍在 Baylee 用戶端保持登入時，你的加密資料才保得住。否則就會遺失。
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
impressum-write = Write an e-mail
impressum-call = Call
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
privacy-updated = 截至 2026 年 11 月 11 日
privacy-controller-title = 誰負責
privacy-controller = GDPR（《一般資料保護規則》）所稱的控管者，是<impressum>法律聲明</impressum>中列名的自然人，該聲明載有其姓名、郵寄地址與電子郵件地址。你也可以透過聯絡表單送出隱私權方面的問題。我們沒有資料保護長；法律並不要求設置。
privacy-short-title = 簡單來說
privacy-short = 沒有追蹤、沒有廣告、沒有分析，除非你登入否則沒有 Cookie，也不從其他伺服器載入任何東西：字型、圖片與指令碼全都來自 baylee.app。你的密碼不會到達我們這裡，你的聊天訊息與私人資料在送出之前，就已在你的瀏覽器中加密。
privacy-hosting-title = 造訪本站
privacy-hosting = 本站執行在我們向德國 netcup GmbH 租用的伺服器上。你開啟頁面時，瀏覽器會送出每個請求都帶有的資訊：你的 IP 位址、所要求的位址、時間、瀏覽器的語言與其識別資訊（user agent）。伺服器需要這些資訊才能回應；它依語言挑選語言版本，下載頁面則用 user agent 推薦適合的檔案。網頁伺服器不保留存取紀錄，因此這些請求不會被儲存。伺服器本身的運作訊息（系統日誌）在少數錯誤情況下可能含有 IP 位址；這些訊息最遲在 14 天後刪除。法律基礎：GDPR 第 6 條第 1 項 (f) 款；我們的正當利益是安全、可靠地提供本站。
privacy-device-title = 你的瀏覽器保存了什麼
privacy-device = 在你登入之前，本站不設定任何 Cookie。你的瀏覽器會在你的裝置上保存以下內容，而且只有在啟用 JavaScript 時才會：你的色彩選擇（若你有選）（<code>baylee-site-theme</code>）；在你登入期間，你的工作階段何時結束（<code>baylee-central-session</code>）、你的聊天金鑰，以及聯絡人的受信任金鑰（見「聊天」）；若你開啟了通知，一組代表這台裝置的隨機數字（<code>baylee-push-installation-v1</code>、<code>baylee-push</code>）；以及，只有在你已登入、將本站安裝為應用程式或開啟通知之後，才會有一份本站公開檔案（頁面外框、字型、指令碼）的副本，讓本站可以在沒有連線時啟動，也讓通知能夠送達。如果這三種情況都不再成立，這份副本會在你下次開啟本站時移除。除非下方某一節另有說明，這些內容都不會傳送給我們。聊天與工作階段的項目會在你登出時刪除；其餘一切，你可以在瀏覽器中清除本站資料來移除。法律基礎：TDDDG（《電信數位服務資料保護法》）第 25 條第 2 項第 2 款（為你所使用的功能所絕對必要）；至於通知所用的隨機數字，則是你的同意（TDDDG 第 25 條第 1 項）。
privacy-github-title = GitHub
privacy-github = 下載頁面與首頁會顯示最新版本、測試狀態與卡牌數量。是我們的伺服器向 GitHub 取得這些資料，不是你的瀏覽器；GitHub 不會因此得知任何關於你的事。只有在你點擊下載或 GitHub 連結時，你的瀏覽器才會連線到 GitHub（GitHub, Inc.，美國），並把你的 IP 位址與每個請求都帶有的資訊傳給它；之後適用 GitHub 自己的隱私權聲明。
privacy-directory-title = Gateway 目錄
privacy-directory = Our server reads the directory of gateways from Baylee Central and shows it; nothing about you is involved. A gateway may tell Central how many players, open tables and games it has at the moment, and Central counts its accounts; the site shows these numbers. They are counts only, never names, and a gateway's numbers are its operator's own statement.
privacy-account-title = 你的帳號
privacy-account = 為了建立帳號，Baylee Central 會儲存：你的帳號 ID、登入名稱、帶標籤的顯示名稱、帳號建立的時間與方式、一筆 OPAQUE 密碼紀錄（讓你能證明自己知道密碼，而我們的伺服器完全不會得知密碼）、經過兩次加密的主金鑰（分別由你的密碼與救援碼加密）、你加密的帳號文件與加密物件（我們無法讀取，只看得到大小、修訂版本與時間）、你接受的是哪一版條款以及何時接受、上次登入的時間、你的工作階段、管理員角色（如果你有的話），以及你註冊的 Gateways 與其權杖。其他玩家看到的是你的顯示名稱與標籤，不會看到你的登入名稱。如果管理員給你一組重設碼，我們只會儲存它的摘要，直到該重設碼被使用或過期為止。管理員對帳號或 Gateway 所做的操作，會記錄保存三年（誰、做了什麼、哪個帳號或 Gateway、何時）。法律基礎：GDPR 第 6 條第 1 項 (b) 款（你要求建立的帳號）；至於上次登入的時間與該紀錄，則是 GDPR 第 6 條第 1 項 (f) 款（我們在安全、可追溯的營運上的正當利益）。沒有登入名稱和密碼就沒有帳號；你不需要電子郵件地址。年滿 16 歲才可以建立帳號。資料保存到你刪除帳號為止；刪除後，會立即從運作中的系統移除，並最遲在 12 個月內從備份中移除。工作階段在閒置 30 分鐘後結束，最遲在登入後 12 小時結束。
privacy-cookie-title = 工作階段 Cookie
privacy-cookie = 登入會設定一個 Cookie：<code>__Host-central</code>，是用來識別你的工作階段的一組隨機數字。它只會傳送給 baylee.app，指令碼無法讀取，並隨工作階段結束，最遲在 12 小時後結束。它是讓你保持登入所絕對必要的，不需要同意。法律基礎：TDDDG 第 25 條第 2 項第 2 款與 GDPR 第 6 條第 1 項 (b) 款。
privacy-chat-title = 聊天與聯絡人
privacy-chat = 訊息在你的裝置上為每位收件人的裝置分別加密；Baylee Central 只以加密形式儲存，無法讀取。Central 看得到並儲存的有：你的聯絡人、聯絡請求與封鎖、群組及其名稱、成員與邀請、誰在何時傳了訊息給誰以及訊息有多大，還有你各裝置的公開金鑰。你的聯絡人與你所在群組的成員會看到你帶標籤的顯示名稱與你的訊息。在你登入期間，你的瀏覽器會保存你的聊天金鑰（以只有這個瀏覽器持有的金鑰封存，放在其儲存空間 <code>baylee-chat</code>）以及你已信任的聯絡人金鑰（加密，<code>baylee-chat-trust</code>）。訊息與傳遞紀錄會保存到你刪除帳號為止；目前還沒有較短的自動保存期限。每位寄件者與每台裝置最多持有 5,000 則訊息；超過之後，新的訊息會被拒收。法律基礎：GDPR 第 6 條第 1 項 (b) 款。
privacy-push-title = 通知
privacy-push = 如果你在瀏覽器中開啟通知，你的瀏覽器會向其推播服務註冊，而 Central 會儲存該服務針對這個瀏覽器的位址、瀏覽器的公開金鑰與驗證密鑰、你的語言、一組代表這台裝置的隨機數字，以及你選擇的類別。推播服務屬於你瀏覽器的開發商：Google（Firebase Cloud Messaging）、Mozilla 或 Apple，皆設於美國。通知經加密後，只帶有它的種類（例如「新訊息」）與你的語言，絕不含姓名或內文；推播服務會看到位址、時間與大小等連線資料。當你在這台裝置上關閉通知、登出，或刪除帳號時，這些資料就會刪除；90 天未更新的位址不再使用，等待中的通知在一天後失效。法律基礎：GDPR 第 6 條第 1 項 (a) 款與 TDDDG 第 25 條第 1 項（你的同意）。你可以隨時在帳號設定中撤回同意；在此之前所做的處理仍屬合法。
privacy-counters-title = 防止濫用
privacy-counters = 為了阻止猜測密碼與過載，Central 會依 IP 位址計算某些請求（註冊、登入嘗試、密碼重設、讀取目錄），而且只在記憶體中計算：註冊最長計算一天，其他一律最長一小時。註冊之前，你的瀏覽器還會解一道謎題；它由隨機數字組成，不含任何關於你的資料，最遲在五分鐘後就被遺忘。Central 也依帳號計算聯絡請求與搜尋其他玩家的次數，以免有人用請求灌爆別人。法律基礎：GDPR 第 6 條第 1 項 (f) 款；我們的正當利益是保護帳號與服務免受濫用。
privacy-contact-title = 聯絡表單
privacy-contact = 表單會把你的訊息、頁面的語言和時間，送到同一台伺服器上 Baylee 的意見回饋服務；你的名字與電子郵件地址只有在你填寫時才會送出，兩者都是選填。該服務也會記下這則訊息是否已處理。你的 IP 位址不會儲存；它只在記憶體中計數一小時，用來阻止訊息氾濫。只有法律聲明中列名的人會讀這些訊息；不會轉交給任何人。訊息在處理完畢 90 天後刪除，最遲一年後刪除，或在你要求時提早刪除。法律基礎：GDPR 第 6 條第 1 項 (f) 款（我們回覆訊息的正當利益），或在你的訊息與你的帳號有關時，為 (b) 款。
privacy-impressum-title = 法律聲明的謎題
privacy-impressum = The legal notice is shown only after your browser has solved a puzzle; that protects the address from programs that collect addresses. The puzzle consists of random numbers and holds nothing about you. For this, the feedback service keeps in memory only, never on disk and never in a log: how many puzzles your IP address asked for (for one hour; an IPv6 address by its first half), whether it followed a link that is hidden from people (for 24 hours), and which puzzles were solved (for five minutes, without an address). Three yes-or-no answers about your browser set the puzzle's difficulty and are not kept: whether a program drives it, whether it names no languages, and whether it draws without a graphics card. Nobody is refused; a browser that looks like a program gets a longer puzzle. Legal basis: Art. 6(1)(f) GDPR (our legitimate interest in protecting a private person's address).
privacy-recipients-title = 接收者與國家
privacy-recipients = 我們不出售資料，也不為了廣告而轉交任何資料。接收者有：netcup GmbH（位於德國的主機代管，依 GDPR 第 28 條擔任受託處理者）；Baylee Central 的管理員，他們為了管理而看得到你的登入名稱、帶標籤的顯示名稱、帳號的來源、建立與上次登入，以及你的 Gateways，但看不到你的加密資料；你以帳號登入的 Gateway（它會收到你的帳號 ID、顯示名稱與標籤，不會收到你的密碼）；你的聯絡人與你所在群組的成員（顯示名稱、標籤與你的訊息）；以及僅在法律要求我們提供時的主管機關。歐洲經濟區以外的國家：只有在你開啟通知時，你瀏覽器的推播服務（Google、Mozilla 或 Apple，美國）才會傳輸資料。他人營運的 Gateways 可能位於任何國家；是否登入由你決定。除此之外，我們不會將資料傳輸到歐洲經濟區以外的國家。
privacy-rights-title = 你的權利
privacy-rights = 你有權請求查閱（GDPR 第 15 條）、更正（第 16 條）、刪除（第 17 條）、限制處理（第 18 條）以及資料可攜（第 20 條）。對於基於我們正當利益（第 6 條第 1 項 (f) 款）的處理，你可以隨時基於與你的特定情況有關的理由提出異議（第 21 條）。你可以隨時撤回同意，效力及於未來（第 7 條第 3 項）。你帳號的資料可以在帳號設定中匯出，帳號也可以刪除；其他一切事項，透過聯絡表單或寄到法律聲明中的地址送出一則訊息即可。你可以向資料保護主管機關提出申訴（第 77 條），例如你居住地的主管機關，或北萊茵-西發里亞邦資料保護與資訊自由專員。沒有任何法律或契約要求你提供資料給我們；不過沒有登入名稱和密碼就沒有帳號。我們沒有自動化決策，也沒有剖析（第 22 條）。
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

error-busy = 目前嘗試的次數太多。請等一分鐘後再試。
error-unavailable = Baylee Central 目前沒有回應。請稍後再試。
error-network = 沒有連線。請檢查網路後再試一次。
error-failed = 發生了問題。請再試一次；若持續發生，請告訴我們。
error-again = 再試一次
password-show = 顯示

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
security-password-later = 在網站上變更密碼的功能之後會提供；在那之前，請在 Baylee 用戶端變更。

privacy-export-title = 匯出你的資料
privacy-export-lead = Baylee Central 所知道的關於你帳號的資料，以 JSON 檔案提供：基本資料、工作階段、已接受的條款、你的 Gateways，以及提到你帳號的稽核紀錄。你的加密內容只能由你的 Baylee 用戶端解密並匯出。匯出需要不到 30 分鐘前的登入。
privacy-export-make = 準備匯出
privacy-export-save = 儲存匯出檔
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
strip-build = 建置
strip-ci = CI
strip-commit = 提交
strip-gateways = Gateways
strip-online = 在線
strip-offline = 離線
strip-releases = 查看版本
strip-on-github = 在 GitHub 查看
strip-directory = 目錄
strip-cards = 可玩的牌
strip-cards-unit = 張
strip-stars = GitHub 星數
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
feature-commander = Assemble decks for classic 60‑card duels, multiplayer rings, or custom house formats. Starting life totals, team layouts, and mulligan rules are freely set by the host.
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
feature-tables = From brisk 15‑second turns to relaxed untimed tables, configure the pace individually for every room.
shots-eyebrow = 在牌桌上
shots-title = 遊戲裡面
shots-lead = Short live recordings captured directly from the 3D client with card images from Scryfall (© Wizards of the Coast)—from duels and 4‑player rings to the encrypted lobby and deck builder.
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
gw-lead = Gateway 是線上牌桌所在的伺服器：帳號、牌組、房間，以及每場遊戲各一個引擎程序。這裡列的是已向 Baylee Central 註冊並經其管理員核准的 Gateways。標示為官方的由 Baylee 營運；其他的由別人為他們的社群營運，適用他們自己的規則。列在名單上，並不代表對 Gateway 的行為作出任何承諾。
gw-howTo = 想在其中一個上面遊玩，請把它的位址複製到用戶端的入口畫面。在線表示 Central 在最近一分半鐘內收到過該 Gateway 的訊息；並不保證它會回應。名稱與版本由各營運者自行回報。
gw-find = 尋找 Gateway
gw-findHint = 名稱或位址
gw-onlineOnly = 僅顯示在線
gw-shown = 顯示 { $n } / { $all }
gw-noMatch = 目前已載入的 Gateways 中沒有符合的。
gw-showAll = 顯示全部
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
