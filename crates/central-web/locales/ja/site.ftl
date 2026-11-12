# baylee.app's words in English, Project Fluent's format. English decides
# which messages exist and which arguments each takes; build.rs turns them
# into typed functions (crates/central-web/build.rs). The reader is "you";
# a button says what it does.

## The frame of every page

site-name = Baylee
skip = 本文へ移動
# Read out after a link that leaves the site (the stylesheet carries it).
new-tab = 新しいタブで開きます
nav-label = メイン
nav-download = ダウンロード
nav-gateways = Gateway
nav-faq = よくある質問
nav-contact = お問い合わせ
nav-sign-in = ログイン
nav-account = アカウント
lang-label = 言語
theme-label = 配色
theme-light = 配色：ライト
theme-dark = 配色：ダーク

foot-label = このサイトについて
foot-play = 遊ぶ
foot-legal = 法的情報
foot-releases = すべてのリリース
foot-issues = 不具合を報告
foot-discord = Discord コミュニティ
foot-central-source = このサイトのソース
foot-dedication = Baylee へ。いつまでも日だまりの中で。
fan-content = Baylee は<policy>ファンコンテンツ・ポリシー</policy>に沿った非公式のファンコンテンツです。ウィザーズ社の認可/許諾は得ていません。題材の一部に、ウィザーズ・オブ・ザ・コースト社の財産を含んでいます。©Wizards of the Coast LLC.
foot-colophon = フォント：Literata と Inter（SIL Open Font License 1.1、このサイトから配信）。絵画とロゴは私たちの作品です。<notices>表示事項</notices>

## Titles and descriptions

title-home = Baylee – デッキを組んで、仲間を集めて、さあ遊ぼう。
desc-home = Baylee は、Linux・Windows・macOS で動く、無料でオープンソースの「Magic: The Gathering」ファンゲームです。カードを知っているルールエンジン、最大8人のテーブル、ほかに誰もいないときの対戦相手を備えています。
title-download = Baylee のダウンロード
desc-download = Windows・macOS・Linux 向けの Baylee クライアントをダウンロード。最新リリースのインストーラーとアーカイブを、チェックサムと署名付きで配布しています。
title-gateways = Gateway：Baylee をオンラインで遊ぶ場所
desc-gateways = オンラインのテーブルがある Baylee の Gateway（サーバー）の公開一覧と、自分で運営する方法。
title-faq = よくある質問
desc-faq = Baylee とは何か、費用、オンライン対戦とアカウントの仕組み、データの扱いについて。
title-contact = お問い合わせ
desc-contact = Baylee の作者への連絡、不具合の報告はこちらから。
title-impressum = 法的表示
desc-impressum = baylee.app の運営者（DDG第5条）。
title-privacy = プライバシー通知
desc-privacy = baylee.app と Baylee Central があなたについて何を、なぜ、どの法的根拠で、どれだけの期間処理するか。
title-terms = 利用規約
desc-terms = baylee.app の背後にあるアカウントサービス Baylee Central の利用規約。
title-fan-content = 非公式ファンコンテンツ
desc-fan-content = Baylee はウィザーズ・オブ・ザ・コースト社のファンコンテンツ・ポリシーに沿ったファンプロジェクトです。その意味と、意味しないこと。
title-not-found = ページが見つかりません
title-gone = このページは削除されました
crumbs-label = 現在地
breadcrumb-home = ホーム

## Home

hero-title = <nb>デッキを組もう。</nb> <nb>仲間を集めよう。</nb> <glow>さあ遊ぼう。</glow>
hero-lead = 充実したデッキビルダー、対戦・全員対戦・チーム戦に対応した最大8人の対戦、そしてすべての手順を確認するルールエンジン。調べものをしなくても、毎ターン公正に進みます。
hero-offline = 一人用の対戦とデッキビルダーは、アカウントなしでオフラインでも使えます。オンラインのテーブルは Gateway 上にあり、<host>自分で運営することもできます</host>。
hero-account = オンラインで遊びますか？ オープンベータ中は誰でも無料で作れます。<register>アカウントを作成</register>するか、<signin>ログイン</signin>してください。
hero-memorial = Baylee を偲んで：写真アルバムを開く

status-label = 数字で見る Baylee

cards-eyebrow = カード
cards-title = Baylee が知っているカードの数
cards-lead = Baylee は Magic のカードを一枚ずつ実装し、それぞれ印刷されたテキストと照合しています。数字は最新リリースのものです。
cards-implemented = 完全に実装
cards-implemented-help = カードのすべてのルールが動作し、テスト済みです。
cards-generated = 自動生成
cards-generated-help = カードのルールテキストから Baylee の生成器が実装したもので、手作業のテストはまだです。プレイはできますが、まれな特殊ケースで挙動が異なることがあります。
cards-partial = 一部実装
cards-partial-help = カードはプレイできますが、能力の一つ、またはルール上の特殊ケースがまだ足りません。
cards-total = カード総数
cards-total-help = Scryfall が挙げる、Magic のすべてのカードです。前の三つの数字は、Baylee でプレイできるものです。
cards-help = これは何を意味しますか
cards-share = { $total } 枚中 { $playable } 枚のカードが、現在プレイできます。
cards-as-of = リリース { $tag }、{ $date } 時点の集計。
cards-missing = カードの数をいま読み込めませんでした。

shot-duel-alt = ハウスAIとの対戦：クリーチャー6体が攻撃し、3体がブロックされている
shot-duel = ハウスAIとの<b>戦術的な対戦</b>。どの割り当てにも、わかりやすい戦闘の線が引かれます。
shot-log-alt = ゲームログを開いた対戦。スタックに呪文が3つ
shot-log = <b>ゲームログ</b>はチャットのように、すべての手順を記録します。カード名を押すとプレビューが開きます。
shot-ring-alt = 円形に並んだ4席のテーブル
shot-ring = <b>最大8席のテーブル</b>は、戦場のまわりに自動で並びます。
shot-lobby-alt = 友達からのメッセージとテーブルへの招待が表示されたロビー
shot-lobby = <b>ロビー：</b>公開テーブル、フレンド、招待、そしてエンドツーエンド暗号化のチャット。
shot-room-alt = 空いた3席をハウスAIが埋めているルーム
shot-room = <b>全員の席：</b>人間、5段階のハウスAI、または言語モデル。
shot-builder-alt = 天使を探してプールを絞り込むデッキビルダー
shot-builder = <b>デッキビルダー</b>は、入力するそばからプール全体を検索します。

about-eyebrow = Baylee とは
about-title = 完全なルールエンジンと、見やすい3Dテーブル
about-engine = 優先権、スタック、レイヤー、置換効果、誘発型能力、状況起因処理、戦闘まで。Rust 製のエンジンが完全なルールを適用し、各場面で選べる合法な選択肢を正確に算出します。クライアントが表示するのは、その有効な操作だけです。
about-table = 戦場、手札、スタック、追放領域、墓地を、すっきりした3Dテーブルに表示。戦闘の線はわかりやすく、カードのプレビューは瞬時に出ます。カードの絵はプレイ中に Scryfall から読み込まれ、いつでもオラクルテキスト表示に切り替えられます。

gw-eyebrow = オンライン
gw-title = Gateway

open-central = GitHub の Baylee Central

## Download

dl-prerelease = ベータ
dl-for = { $system } 版をダウンロード
dl-size = { $size } MB
dl-checksum = SHA-256
dl-signature = 署名
dl-notes = リリースノート
dl-unknown = このブラウザーからはお使いのシステムを判別できませんでした。下からお選びください。
dl-mobile = スマートフォンとタブレット向けの版はまだありません。Baylee はパソコンで動きます。
dl-mac-unsure = このブラウザーでは、お使いの Mac が Apple チップか Intel かがわかりません。上のファイルは Apple チップ（M1 以降）用で、Intel Mac 用のファイルは下にあります。
dl-windows-unsure = ARM プロセッサーの Windows パソコン（たとえば Snapdragon 搭載のノートパソコン）では、下の ARM 用ファイルを選んでください。
dl-linux-unsure = ブラウザーがプロセッサーの種類を示さないため、x86-64 と仮定しています。ARM 用のファイルは下にあります。
dl-missing = このリリースには、お使いのシステム用のファイルがありません。以前のリリースは<releases>リリース一覧</releases>にあります。
dl-none = GitHub からいま応答がありませんでした。すべてのリリースは<releases>リリース一覧</releases>にあります。
dl-yours = お使いのシステム
dl-more-files = ほかのファイル（{ $count }）
dl-installer = インストーラー
dl-deb = Debian パッケージ
dl-archive = アーカイブ
dl-fallback = 古いプロセッサー向けアーカイブ
dl-fallback-help = AVX2 のない x86-64 プロセッサー（2013年ごろより前）向けです。Baylee の通常ビルドは AVX2 が必要で、なければ起動しません。
plat-windows-x64 = Windows
plat-windows-x64-sub = 多くのPC：64ビットの Intel または AMD
plat-windows-arm64 = Windows on ARM
plat-windows-arm64-sub = Snapdragon 搭載ノートなど
plat-mac-arm64 = macOS
plat-mac-arm64-sub = Apple チップ：M1 以降
plat-mac-x64 = Intel 版 macOS
plat-mac-x64-sub = Intel プロセッサーの Mac
plat-linux-x64 = Linux
plat-linux-x64-sub = x86-64
plat-linux-arm64 = Linux on ARM
plat-linux-arm64-sub = ARM64 / aarch64
plat-none = このリリースには、このシステム用のファイルがありません。
install-title = 初回起動と動作環境
install-first-title = 初回起動
install-first = ビルドにはコード署名がありません（署名には費用がかかるため）。そのため Windows と macOS では初回に警告が出ます。Windows では<i>詳細情報</i>から<i>実行</i>を選んでください。macOS では<i>システム設定 → プライバシーとセキュリティ</i>を開き、<i>このまま開く</i>を選んでください。
install-check-title = ダウンロードの確認
install-check = 各ファイルの隣に、チェックサムを記した <code>.sha256</code> があります。アーカイブには <code>.sig</code> も付いており、これはクライアントが自己更新の前に確認する Ed25519 署名です。Linux では <code>sha256sum -c</code>、macOS では <code>shasum -a 256</code>、Windows では <code>certutil -hashfile</code> でファイルを確認できます。
install-needs-title = 動作環境
install-windows = <b>Windows</b> 10 または 11（x64 または ARM64）。
install-mac = <b>macOS</b> 11 以降（Apple チップまたは Intel）。
install-linux = <b>Linux</b>：glibc 2.35 以降（Ubuntu 22.04 以降）と Vulkan ドライバーが必要です。そのほか必要なものは <code>.deb</code> が一緒に導入します。
install-cpu = <b>プロセッサー：</b>x86-64 向けビルドには AVX2 が必要です。古いプロセッサー向けには別のアーカイブがあります。

## Gateways

gws-title = Gateway
gws-online = オンライン
gws-offline = オフライン
gws-version = バージョン
gws-now = 現在
gws-website = ウェブサイト
gws-address = アドレス
gws-copy = アドレスをコピー
gws-copied = アドレスをコピーしました。
gws-copied-short = コピー済み
gws-copy-failed = コピーできませんでした。
gws-failed = 一覧をいま読み込めませんでした。しばらくしてからもう一度お試しください。

## Questions

faq-eyebrow = 質問
faq-title = よくある質問
faq-what-q = Baylee とは何ですか？
faq-what-a = パソコンで遊べる、「Magic: The Gathering」の無料のファンゲームです。デッキビルダー、すべての手順を確認するルールエンジン、最大8人のテーブル、コンピューターの対戦相手があります。ウィザーズ・オブ・ザ・コースト社が制作・承認・推奨したものではありません。
faq-cost-q = 費用はかかりますか？
faq-cost-a = かかりません。今後も変わりません。課金も、有料機能も、広告もありません。
faq-account-q = アカウントは必要ですか？
faq-account-a = 一人で遊ぶ分には不要です。デッキビルダーとハウスAIとの対戦はオフラインで使えます。オンラインのテーブルには Baylee のアカウントが必要です。オープンベータ中は、誰でもこのサイトで無料で作成できます。
faq-data-q = 私のデータはどうなりますか？
faq-data-a = できるだけ少なくしています。詳しくは<privacy>プライバシー通知</privacy>をご覧ください。パスワードがブラウザーの外に出ることはなく、非公開のデータとチャットのメッセージは、私たちに届く前にお使いの端末で暗号化されます。
faq-password-q = パスワードを忘れました。どうすればよいですか？
faq-password-a = 新しいパスワードは、リセットコードを使って<reset>このページ</reset>で設定します。アカウントに確認済みのメールアドレスがあれば、そこでコードをそのアドレスに送ってもらえます。ない場合は、管理者から受け取ります。暗号化されたデータが残るのは、アカウント作成時に受け取った回復コードがある場合だけです。ない場合は空の保管庫から始まり、そのデータは失われます。
faq-gateway-q = Gateway とは何ですか？
faq-gateway-a = オンラインのテーブルがあるサーバーです。誰でも運営できます。<gateways>一覧</gateways>には、Central が把握しているものが載っています。
faq-systems-q = どのシステムで動きますか？
faq-systems-a = x86-64 と ARM64 の Windows、macOS、Linux です。スマートフォンとタブレット向けの版はまだありません。
faq-open-q = オープンソースですか？
faq-open-a = ゲームはオープンソースで、GNU Affero General Public License の下にあります。Baylee Central のコードは、データの扱いを誰でも確かめられるよう公開されています。
faq-bug-q = 不具合を見つけました。どこに報告すればよいですか？
faq-bug-a = ゲーム内で <kbd>F8</kbd> を押すと、必要な情報をまとめて送れます。ほかには、GitHub の Issue トラッカー、Discord、<contact>お問い合わせフォーム</contact>からも報告できます。

## Contact

contact-eyebrow = お問い合わせ
contact-title = ご連絡ください
contact-name = お名前
contact-name-hint = 任意です。
contact-email = メールアドレス
contact-email-hint = 任意です。返信のためだけに使います。
contact-message = メッセージ
contact-count = { $n } / 5,000 文字
contact-send = 送信
contact-sending = 送信中 …
contact-sent-title = ありがとうございます
contact-sent = メッセージが届きました。アドレスをご記入いただいた場合は、そちらに返信します。
contact-error-empty = 先にメッセージを書いてください。
contact-error-email = メールアドレスの形式ではないようです。
contact-error-busy = お使いのネットワークからのメッセージが多すぎます。1時間後にもう一度お試しください。
contact-error-refused = メッセージを受け付けられませんでした。長さを確認して、もう一度お試しください。
contact-error-network = メッセージを送信できませんでした。接続を確認して、もう一度お試しください。
contact-privacy = お送りいただいた内容は、返信のために保管し、対応後90日で、遅くとも1年後に削除します。詳しくは<privacy>プライバシー通知</privacy>をご覧ください。
contact-noscript = このフォームには JavaScript が必要です。Discord または GitHub からも連絡できます。

## Legal notice

impressum-title = 法的表示
impressum-lead = DDG（ドイツのデジタルサービス法）第5条が求める情報です。Baylee は個人による非営利のプロジェクトです。
impressum-show = 詳細を表示
impressum-why = 詳細は、住所を収集するロボットを遠ざけるため、ブラウザーが数秒で解く短いパズルの先にあります。それ以外にお願いすることはありません。
impressum-solving = パズルを解いています … { $percent } %
impressum-name = 氏名
impressum-address = 住所
impressum-email = メール
impressum-phone = 電話
impressum-write = メールを書く
impressum-call = 電話をかける
impressum-failed = 詳細を読み込めませんでした。もう一度お試しいただくか、<contact>お問い合わせフォーム</contact>からご連絡ください。
impressum-noscript = 詳細の表示には JavaScript が必要です。使えない場合は、Discord または GitHub からご連絡ください。
impressum-responsible = 内容の責任者：上に記載した人物。
impressum-dispute = 私たちは、消費者仲裁機関での紛争解決手続に参加する義務を負わず、参加する意思もありません。

## Fan content

fc-title = 非公式ファンコンテンツ
fc-eyebrow = ファンコンテンツ
fc-lead = Baylee はファンプロジェクトです。ウィザーズ・オブ・ザ・コースト社のファンコンテンツ・ポリシーに沿ったものであり、そのルールに従っています。
fc-notice-title = すべてのページにある表示
fc-means-title = その意味
fc-means = Baylee は、ウィザーズ・オブ・ザ・コースト社が制作・承認・推奨したものではありません。Magic: The Gathering、そのカード名、セット名、カード画像、マナ・シンボルは、Wizards of the Coast LLC の財産です。Baylee は誰でも無料で使えます。課金も、有料機能も、メールアドレスでの登録もありません。
fc-images = ゲームは Scryfall のカード画像を、加工せず、アーティストのクレジットとともに表示します。このサイトには画像を載せていません。カードのデータと画像は Scryfall が提供しており、Baylee は Scryfall と提携していません。
fc-policy = ウィザーズ・オブ・ザ・コースト社の<policy>ファンコンテンツ・ポリシー</policy>

## Privacy notice

privacy-title = プライバシー通知
privacy-lead = このサイトと Baylee Central があなたについて何を、なぜ、どの法的根拠で、どれだけの期間処理するか。
privacy-updated = 2026年11月12日現在
privacy-controller-title = 責任者
privacy-controller = GDPR（EU一般データ保護規則）上の管理者は、<impressum>法的表示</impressum>に記載した個人です。そこには氏名、郵送先住所、メールアドレスが記されています。プライバシーに関する質問は、お問い合わせフォームからも送れます。データ保護責任者はいません。選任は義務付けられていません。
privacy-short-title = 要点
privacy-short = トラッキング、広告、アクセス解析はなく、ログインしない限りクッキーもなく、ほかのサーバーから読み込むものもありません。フォント、画像、スクリプトはすべて baylee.app から配信されます。パスワードが私たちに届くことはなく、チャットのメッセージと非公開のデータは、送信前にブラウザー内で暗号化されます。
privacy-hosting-title = サイトの閲覧
privacy-hosting = このサイトは、ドイツの netcup GmbH から借りているサーバーで動いています。ページを開くと、ブラウザーはすべてのリクエストに付くもの、つまりIPアドレス、要求したアドレス、時刻、ブラウザーの言語、ブラウザーの識別情報（ユーザーエージェント）を送ります。サーバーは応答のためにこれを必要とします。言語版は言語の情報から選び、ダウンロードページはユーザーエージェントから適切なファイルを提案します。ウェブサーバーはアクセスログを残さないため、これらのリクエストは保存されません。サーバー自身の動作メッセージ（システムジャーナル）には、まれにエラーの際にIPアドレスが含まれることがあり、遅くとも14日後に削除されます。法的根拠：GDPR第6条第1項(f)。私たちの正当な利益は、サイトを安全かつ確実に提供することです。
privacy-views-title = ページビューの集計
privacy-views = このサイトは、ページが開かれた回数を数えています。すべてのページとすべての訪問者をまとめた1つの数だけで、トップページに表示されます。ブラウザでページが開かれると、この数が1増えます。このために、あなたに関することは何も保存しません。IPアドレスも、クッキーも、識別子も、どのページだったかさえ保存しません。
privacy-device-title = ブラウザーが保持するもの
privacy-device = ログインするまで、このサイトはクッキーを設定しません。ブラウザーがお使いの端末に保持するものは次のとおりで、JavaScript を使う場合に限ります。配色を選んだ場合はその選択（<code>baylee-site-theme</code>）。ログイン中は、セッションの終了時刻（<code>baylee-central-session</code>）、チャットの鍵、連絡先の信頼済みの鍵（チャットの項を参照）。通知をオンにした場合は、この端末の乱数（<code>baylee-push-installation-v1</code>、<code>baylee-push</code>）。そして、ログインした、サイトをアプリとしてインストールした、通知をオンにした、のいずれかの場合に限り、サイトの公開ファイル（ページの枠、フォント、スクリプト）の控え。これにより、接続がなくても起動でき、通知を受け取れます。この三つのいずれにも当てはまらなくなった場合、控えは次にサイトを開いたときに削除されます。以下の項に記載がない限り、これらが私たちに送られることはありません。チャットとセッションの項目はログアウト時に削除されます。それ以外は、ブラウザーでサイトのデータを消去すれば削除できます。法的根拠：TDDDG第25条第2項第2号（利用する機能に厳密に必要なもの）。通知の乱数については、同意（TDDDG第25条第1項）。
privacy-device-manager = 回復コードをブラウザーのパスワードマネージャーに保存したり、自分宛てにメールで送ったり、ファイルとして保存したり、表示される手順で印刷したりした場合、コードを保持するのは、ブラウザー、メールソフト、ファイル、紙であり、このサイトではありません。コードはお使いのブラウザー内で作られ、私たちには送られません。サイトのデータを消去しても、それらの場所からは消えません。
privacy-github-title = GitHub
privacy-github = ダウンロードページとトップページには、最新のリリース、テストの状況、カードの数が表示されます。これらはブラウザーではなく私たちのサーバーが GitHub に問い合わせて取得するため、GitHub があなたについて知ることはありません。ダウンロードや GitHub へのリンクを押したときに限り、ブラウザーが GitHub（GitHub, Inc.、米国）に接続し、IPアドレスとすべてのリクエストに付くものを送ります。その後は GitHub 自身のプライバシー通知が適用されます。
privacy-directory-title = Gateway 一覧
privacy-directory = 私たちのサーバーは Baylee Central から Gateway の一覧を読み、表示します。あなたに関することは含まれません。Gateway は、その時点で接続しているプレイヤーの数、そのうちゲーム中の数、開いているテーブルの数、ゲームの数を Central に伝えることがあり、Central はアカウント数を数えます。サイトはこれらの数を表示します。表示されるのは数だけで、名前は含まれません。Gateway の数は、その運営者自身の申告です。
privacy-account-title = アカウント
privacy-account = アカウントについて、Baylee Central は次を保存します。アカウントID、ログイン名、タグ付きの表示名、アカウントがいつどのように作られたか、OPAQUE のパスワード記録（私たちのサーバーがパスワードを知ることなく、パスワードを証明できる仕組み）、パスワードと回復コードで二重に暗号化されたマスターキー、暗号化されたアカウント文書と暗号化されたオブジェクト（私たちには読めず、サイズ、リビジョン、時刻だけが見えます）、同意した利用規約のバージョンと時期、最後にログインした時刻、セッション、管理者の役割（ある場合）、登録した Gateway とそのトークン。ほかのプレイヤーに見えるのは表示名とタグで、ログイン名は見えません。管理者がリセットコードを渡した場合、私たちが保存するのは、コードが使われるか期限が切れるまで、そのダイジェストだけです。管理者がアカウントや Gateway に対して行った操作は、3年間、記録（誰が、何を、どのアカウントまたは Gateway に、いつ）として保管されます。法的根拠：GDPR第6条第1項(b)（ご希望のアカウント）。最後のログイン時刻と記録については、GDPR第6条第1項(f)（安全で追跡可能な運用という私たちの正当な利益）。ログイン名とパスワードがなければアカウントはありません。メールアドレスは不要です。アカウントを作成できるのは16歳からです。アカウントを削除するまで保管します。削除すると、稼働中のシステムからは直ちに、バックアップからは遅くとも12か月以内に削除されます。セッションは、使われないまま30分で、遅くともログインから12時間で終了します。
privacy-email-title = メールアドレス
privacy-email = メールアドレスは任意です。アカウントには必要ありません。メールアドレスを登録した場合、Baylee Central は、そのアドレス、メールを書く言語、確認した日時を保存します。指定されたアドレスのうち、コードの入力を待っているものは、確認または取り下げるまで、ただし遅くともコードの期限が切れてから1日後まで保管されます。送信した各コードについては、コードが使われるか期限が切れるまでダイジェストだけを保存し、24時間は、リセットコードが送信された時刻を保存します。1日に送るリセットコードを3通以内にするためです。アドレスは4つの目的に使います。アドレスを確認するコードの送信、あなたまたはあなたのログイン名を知っている誰かが求めたときのリセットコードの送信、古いアドレスへの置き換えまたは削除の通知、リセットコードで新しいパスワードが設定されたとき、パスワードが変更されたとき、または管理者がアカウントのリセットコードを作ったときの、確認済みのアドレスへの通知です。メールは、同じサーバー上の Baylee 独自のメールサービスから、mailbox.org（Heinlein Hosting GmbH、Berlin）経由で送られます。mailbox.org は処理者として、アドレスとメールを受け取ります。法的根拠：GDPR第6条第1項(b)（ご希望のアカウント）。アドレスを削除するかアカウントを削除するまで保管します。
privacy-cookie-title = セッションのクッキー
privacy-cookie = ログインすると、クッキーを一つ設定します。<code>__Host-central</code> で、セッションを識別する乱数です。baylee.app にだけ送られ、スクリプトからは読めず、セッションとともに、遅くとも12時間後に終了します。ログイン状態を保つために厳密に必要なものであり、同意は要りません。法的根拠：TDDDG第25条第2項第2号およびGDPR第6条第1項(b)。
privacy-chat-title = チャットと連絡先
privacy-chat = メッセージは、受信者の端末ごとに、お使いの端末で暗号化されます。Baylee Central は暗号化された形でのみ保存し、読むことはできません。Central が見られるもの、保存するものは次のとおりです。連絡先、連絡先リクエストとブロック、名前・メンバー・招待を含むグループ、誰が誰にいつどのくらいの大きさのメッセージを送ったか、そしてお使いの端末の公開鍵。連絡先とグループのメンバーには、タグ付きの表示名とあなたのメッセージが見えます。ブラウザーは、ログイン中、チャットの鍵（このブラウザーだけが持つ鍵で封じて、ストレージ <code>baylee-chat</code> に）と、信頼した連絡先の鍵（暗号化して、<code>baylee-chat-trust</code> に）を保持します。メッセージと配信は、アカウントを削除するまで保管されます。これより短い自動削除期間は、まだありません。送信者ごと、端末ごとに保持できるのは最大5,000件で、それを超えると新しいメッセージは拒否されます。法的根拠：GDPR第6条第1項(b)。
privacy-push-title = 通知
privacy-push = ブラウザーで通知をオンにすると、ブラウザーがそのプッシュサービスに登録し、Central は、このブラウザー向けのそのサービスのアドレス、ブラウザーの公開鍵と認証シークレット、言語、この端末の乱数、選んだカテゴリーを保存します。プッシュサービスは、ブラウザーの提供元であるGoogle（Firebase Cloud Messaging）、MozillaまたはAppleのもので、いずれも米国に拠点があります。通知には、暗号化された状態で、種類（たとえば「新着メッセージ」）と言語だけが入り、名前や本文は入りません。プッシュサービスには、アドレス、時刻、サイズなどの接続データが見えます。これらは、この端末で通知をオフにしたとき、ログアウトしたとき、またはアカウントを削除したときに削除されます。90日間更新されないアドレスは使われなくなり、待機中の通知は1日で失効します。法的根拠：GDPR第6条第1項(a)およびTDDDG第25条第1項（同意）。アカウントの設定でいつでも撤回できます。それまでの処理は適法のままです。
privacy-counters-title = 不正利用への対策
privacy-counters = パスワードの推測と過負荷を防ぐため、Central はリクエスト（登録、ログインの試行、パスワードのリセット、ディレクトリの閲覧、およびより一般に、1つのアドレスが1分間に送るリクエストの数）を IP アドレスごとに、メモリ内でのみ数えます。登録と、正しいパスワードで完了しなかったログインの試行は最長で1日、それ以外は長くても1時間です。正しいパスワードでログインした後、Central は、あなたのアカウントがそのアドレス（IPv4 アドレス、または IPv6 アドレスの前半。1アカウントにつき最大8件）からログインしたことを、やはりメモリ内でのみ30日間覚えています。これは、他の人によるあなたのアカウントへの試行で、あなたがそのアドレスから締め出されないようにするためです。同じアドレスを使っていて、あなたのログイン名を知っている人は、Central の応答から、あなたのアカウントがそのアドレスからログインしたことが分かります。新しいパスワードを設定すると、設定したアドレスを除いてこれらのアドレスは忘れられ、アカウントを削除するとすべて忘れられ、サービスを再起動した場合も同様です。登録の前には、ブラウザーがメモリと計算時間を要するパズルも解きます。パズルは乱数でできており、あなたに関する情報は含まず、遅くとも20分後には忘れられます。Central はブラウザーの答えを確認するため、その一部を計算し直します。難しさは、作成中のアカウントの数と、リクエストがプログラムのものに見えるかどうかで決まります。具体的には、すべてのブラウザーが送るヘッダーが付いているか、ブラウザーがプログラムに操作されていると申告しているか、そして直近1時間にあなたのアドレスとその周りのネットワークブロック（IPv4 アドレスの最初の3つの数、IPv6 アドレスの最初の48ビット）が何個のパズルを要求したかです。これについて Central が保持するのは、メモリ内の件数だけで、最長で1時間です。アカウントごとに、Central は連絡の依頼と他のプレイヤーの検索を数えます。誰も他の人に依頼を大量に送りつけないようにするためです。法的根拠：GDPR第6条第1項(f)。私たちの正当な利益は、アカウントとサービスを不正利用から守ることです。
privacy-contact-title = お問い合わせフォーム
privacy-contact = フォームは、メッセージ、ページの言語、時刻を同じサーバー上の Baylee のフィードバックサービスに送り、氏名とメールアドレスは、ご記入いただいた場合に限って送ります。どちらも任意です。サービスは、メッセージに対応済みかどうかも記録します。IPアドレスは保存せず、メッセージの大量送信を防ぐために、1時間だけメモリ内で数えます。メッセージを読むのは法的表示に記載した人物だけで、第三者には渡しません。メッセージは、対応後90日、遅くとも1年後に削除し、ご希望があればそれより早く削除します。法的根拠：GDPR第6条第1項(f)（メッセージに答えるという私たちの正当な利益）。メッセージがアカウントに関するものである場合は(b)。
privacy-impressum-title = 法的表示のパズル
privacy-impressum = 法的表示は、ブラウザーがパズルを解いた後にのみ表示されます。これは、住所を収集するプログラムから住所を守るためです。パズルは乱数でできており、あなたに関する情報は含みません。このためフィードバックサービスは、メモリ内にのみ、ディスクにも記録にも残さずに、次のものを保持します。あなたのIPアドレスが要求したパズルの数（1時間。IPv6アドレスは前半部分で）、人には見えないリンクをたどったかどうか（24時間）、解かれたパズル（5分、アドレスなし）。ブラウザーについての三つのはい・いいえの答えがパズルの難しさを決めますが、保持されません。プログラムが操作しているか、言語を一つも示していないか、グラフィックスカードなしで描画しているか、の三つです。拒否される人はいません。プログラムに見えるブラウザーには、より長いパズルが出ます。法的根拠：GDPR第6条第1項(f)（個人の住所を守るという私たちの正当な利益）。
privacy-recipients-title = 受領者と国
privacy-recipients = データを販売することも、広告のために渡すこともありません。受領者は次のとおりです。netcup GmbH（ドイツでのホスティング、GDPR第28条にいう処理者）。mailbox.org（Heinlein Hosting GmbH、Berlin。あなたが登録したメールアドレスへメールを届ける処理者）。Baylee Central の管理者（管理のため、ログイン名、タグ付きの表示名、アカウントの由来・作成・最後のログイン、Gateway が見えますが、暗号化されたデータは見えません）。アカウントでログインする Gateway（アカウントID、表示名、タグを受け取り、パスワードは受け取りません）。連絡先とグループのメンバー（表示名、タグ、あなたのメッセージ）。法律で義務付けられている場合に限り、当局。欧州経済領域外の国：通知をオンにした場合に限り、ブラウザーのプッシュサービス（Google、Mozilla、Apple、米国）がこれらを運びます。他者が運営する Gateway はどの国にもありえます。そこでログインするかどうかは、あなたが決めます。それ以外に、欧州経済領域外の国へデータを移転することはありません。
privacy-rights-title = あなたの権利
privacy-rights = あなたには、アクセス権（GDPR第15条）、訂正権（第16条）、消去権（第17条）、処理の制限の権利（第18条）、データポータビリティの権利（第20条）があります。ご自身の特別な状況を理由として、私たちの正当な利益に基づく第6条第1項(f)の処理に、いつでも異議を述べることができます（第21条）。同意は、将来に向けていつでも撤回できます（第7条第3項）。アカウントのデータの書き出しとアカウントの削除は、アカウントの設定で行えます。それ以外は、お問い合わせフォームか、法的表示に記載のアドレスへのメッセージで足ります。データ保護監督機関に苦情を申し立てることもできます（第77条）。たとえばお住まいの地域の機関や、ノルトライン＝ヴェストファーレン州のデータ保護・情報自由州コミッショナーです。法律や契約によってデータの提供を義務付けられることはありません。ただし、ログイン名とパスワードがなければアカウントはありません。自動化された意思決定もプロファイリングもありません（第22条）。
privacy-objection-title = 異議を述べる権利
privacy-objection = GDPR第6条第1項(f)に基づいてデータを処理する場合（サイトの提供、不正利用への対策、お問い合わせフォーム、法的表示のパズル、最後のログイン時刻、管理の記録、不具合の報告、利用数の集計）、ご自身の特別な事情を理由に、いつでも異議を述べることができます（GDPR第21条）。その場合、あなたの利益に優先する、やむを得ない正当な理由を示せるとき、または処理が法的請求の主張、行使、防御に役立つときを除き、処理をやめます。お問い合わせフォームまたは法的表示のアドレスへの、形式にとらわれないメッセージで足ります。
privacy-links-title = ほかのサービスへのリンク
privacy-links = このサイトは、GitHub、Discord、WhatsApp などへのリンクを載せています。これらは単なるリンクで、何も埋め込んでおらず、押すまでは、これらのサービスがあなたの訪問を知ることはありません。押したとき、ブラウザーは、どのページから来たかを伝えません（リファラーなし）。その後は各提供元のプライバシー通知が適用され、EU域外でデータが処理されることもあります。そこでコミュニティのグループの他のメンバーにあなたの何が見えるかは、それらのサービスが決めます。
privacy-reports-title = ゲームからの不具合報告と利用数の集計
privacy-reports = ゲームから F8 で送る不具合報告には、種類、あなたのテキスト、クライアントのバージョン、チェックを残した添付が含まれます。内容はお問い合わせページに一覧があります。対応後90日、遅くとも3年後に削除します。法的根拠：GDPR第6条第1項(f)（不具合のないゲームという私たちの正当な利益）。クライアントがクラッシュレポートを送るのは、許可した場合に限ります（GDPR第6条第1項(a)。クライアントの設定でいつでも撤回できます）。オフラインのゲーム中、クライアントはそのゲームのランダムな識別子を1分に1回送り、それ以外は何も送りません。これはメモリ内だけに保持し、最後の反応から150秒後に忘れます（GDPR第6条第1項(f)。「匿名の利用数を送信」でオフにできます）。すべての接続と同様に、IPアドレスは私たちに届きます。

## Terms

terms-title = 利用規約
terms-draft = この規約は草案であり、まだ効力を持ちません。
terms-version = バージョン { $version }、{ $date } 更新。
# Shown only in the languages a machine translated: every one but German and English.
machine-note = このサイトは機械翻訳されたもので、日本語を話す方による確認はまだ受けていません。ゲーム自体は今のところ英語とドイツ語です。<link>誤りを知らせる。</link>
legal-machine = このページは参考として提供する機械翻訳です。<de>ドイツ語</de>版または<en>英語</en>版と異なる場合は、それらが優先します。
terms-machine = Central の利用規約は、ドイツ語と英語でのみ提供されています。以下は英語版です。<de>ドイツ語版</de>もあります。

## Errors

not-found-title = このページは存在しません
not-found-lead = アドレスの入力が間違っているか、ページが移動した可能性があります。
gone-title = このページは完全に削除されました
gone-lead = ここにあったものは削除されました。次に進む先はトップページに載っています。
error-home = トップページへ

## Baylee's remembrance (approved by her family on 27 October 2026; keep word for word)
baylee-dedication = Baylee へ。いつまでも日だまりの中で。
baylee-invitation = この世界の向こうにいる小さな魂に会いに
baylee-open = Baylee を偲んで · 写真アルバムを開く
baylee-eyebrow = このプロジェクトの心
baylee-date = 愛をこめて偲んで · 2026年10月26日
baylee-close = Baylee の世界へ戻る
baylee-previous = 前の写真
baylee-next = 次の写真
baylee-photo1 = やわらかな白い掛け布団の中で丸くなる Baylee。
baylee-photo2 = 花柄の掛け布団からのぞいている Baylee。
baylee-photo3 = 丸いグレーのベッドで休んでいます。
baylee-photo4 = ぴたりと丸まって眠る Baylee。
baylee-photo5 = 掛け布団にくるまった、静かなひととき。

baylee-storyTitle = Baylee へ。いつまでも日だまりの中で。
baylee-together = 私たちとともに · 2025年11月17日 – 2026年10月26日
baylee-chapters = Baylee の物語
baylee-chapterPrevious = 前の章
baylee-chapterNext = 次の章
baylee-photos = 私たちの思い出
baylee-read = スクロールして読む

baylee-chapter1Title = Baylee の家
baylee-chapter1Body =
    Baylee は2025年11月17日に、私たちの家に来ました。

    妻と私は、クレーフェルトの動物保護施設から彼女を迎えました。推定で13歳ほどでした。悪性の乳腺腫瘍があり、がんはすでに肝臓にまで広がっていました。Baylee は緩和ケアの猫でした。ともに過ごせる時間が限られていることは、わかっていました。私たちのもとで、愛される家を持ってほしいと思ったのです。

    彼女がその家をあっという間に自分のものにすることを、私たちはすぐに知りました。

    来たその日に、仕事中の私の膝にするりと乗ってきました。最初の夜には、両親の猫のために注文しておいたキャットフードの袋に穴を開け、勝手に食べてしまいました。もちろん彼女には専用のごはんがあり、その晩もしっかり食べていました。でも Baylee は、この備蓄はまだ詳しく調べる価値があると考えたようです。

    私たちの小さなネズミさんは、驚くほどの食欲と、信じられないほどの元気を持っていました。

baylee-chapter2Title = 深まる信頼
baylee-chapter2Body =
    水をたくさん飲むことに気づき、獣医に連れて行きました。血液検査で甲状腺の病気がわかり、旺盛な食欲と活発さの理由もそこにありました。甲状腺の薬と、炎症の数値が高かったため一時的な抗生物質が処方されました。

    突然あらゆるものに体をこすりつけ、よく鳴き、いつになく甘えてくっつくようになったとき、新しい薬のどれかが合わないのではと最初は心配しました。別の診察で理由がわかりました。うちの小さな子は発情していたのです。避妊手術を受けていませんでした。

    その時期も、ともに乗り越えました。Baylee は私たちをますます信頼するようになり、私たちの距離もどんどん近くなっていきました。

baylee-chapter3Title = 私たちの小さなお姫さま
baylee-chapter3Body =
    キャットツリーが届いたときの、彼女の喜びようを今もはっきり覚えています。すぐいちばん上の台に座り、満足そうに部屋を見渡していました。不思議なことに、その場所を使ったのは、それきりでした。理由は彼女だけの秘密です。

    ほかの場所は、空いていることがほとんどありませんでした。私の膝、私の脚、私たちのベッド。ベッドでは、たいてい私か妻の上で眠りました。いちばんいいのは、二人の上に同時に寝ることでした。

    しっぽをぴんと立てて部屋を歩く姿は、自分がここのお姫さまだと、よくわかっているかのようでした。気が強く、自分の境界をはっきり示すことができました。私たちのもとでは、ありのままでいられました。くせも、要求も、ちょっと頑固なところも、みんな愛されていました。

    動物とこれほど深い絆を結んだことは、それまでありませんでした。Baylee は、妻と私の心のとても深いところに居場所を見つけました。

baylee-chapter4Title = 日なたで過ごす時間
baylee-chapter4Body =
    暖かくなると、彼女はバルコニーを見つけました。その日最初の日差しを毛皮に受けるのが大好きでした。まもなく、ほとんど一日中そこで過ごすようになりました。

    ハトや庭の鳥たちは、特別な楽しみでした。じっと見つめ、待ち伏せし、何度も驚かせようとしました。日向ぼっこと鳥の観察のあいだで、うちの小さなお姫さまの一日はとても忙しいものでした。

baylee-chapter5Title = 自分だけの考え
baylee-chapter5Body =
    その間にも、病気は進んでいきました。

    腫瘍は炎症を起こし、Baylee は患部をなめて開いてしまいました。獣医で最初の術後服をもらいました。赤い服でした。それからは、傷をこれ以上傷つけないよう、毎日着なければなりませんでした。

    すぐに着慣れました。ただ、着せられるのは最後の日まで嫌がりました。ごはんの皿を見せても、そのときはあまり効きませんでした。食欲は旺盛でも、服についての意見ははっきりしていました。

    傷は悪くなっていきました。膿が出て、やがて出血し、いやなにおいがするようになりました。それでも Baylee は、毎日の暮らしにまだどれほど喜びを見いだしているかを、私たちに見せ続けました。遊び、食べ、まわりを探検し、爪とぎの板や戸棚に登りました。

    ときには何時間も私たちの隣のベッドの端で、私たちがやっと目を覚まして朝ごはんを用意するのを待っていました。時間がかかりすぎると、ただ私たちの上を乗り越えていきました。顔だって邪魔になりませんでした。

    食べ物にはとてもはっきりした考えがありました。好みはむしろ「冷凍ピザ」派でした。しばらくすると、高級なキャットフードでは興奮しなくなり、バリエーションが欠かせませんでした。同じものが二回続くなど、もってのほかでした。レバー入りのチキン、チーズ入りのもの、ツナやサーモン、そういったものが彼女を幸せにしました。

baylee-chapter6Title = しっぽをぴんと立てて
baylee-chapter6Body =
    腫瘍は大きくなり、ますます液がにじむようになりました。Baylee は目に見えて痛がっていました。痛みがやわらぐと、また私たちの小さな猫に戻り、しっぽをぴんと立てて部屋を歩き、自分の人間たちを独り占めしていました。

    彼女の力が少しずつ弱まっていくのが、わかりました。それでも、食欲も、好奇心も、甘えん坊なところも、大きなゴロゴロも、まだそこにありました。

baylee-chapter7Title = お別れ
baylee-chapter7Body =
    2026年10月23日の金曜日の夜、最初は風邪のように見える症状が出ました。鼻水が出て、呼吸は深く苦しそうで、息をするのに苦労しているようでした。

    土曜日は、また元気になったように見えました。いつもと違う呼吸と鼻水は残っていましたが、私たちのよく知る元気さと生きる喜びを、また見せてくれました。

    日曜日の朝はつらいものでした。目から涙が出て、息をするのが苦しく、痛がっていました。痛み止めをあげました。1時間ほどで楽になりましたが、呼吸はやはり違ったままでした。獣医に連れて行かなければならないと、私たちにはわかりました。

    月曜日の朝、容態はまた悪くなりました。幸い、妻はその日が休みでした。病院に電話をして予約を取り、Baylee を連れて行きました。

    レントゲンで、がんがどこまで広がっているかがわかりました。呼吸に使えるのは肺の約3分の1しか残っていない、と告げられました。このまま生きさせることは、彼女がだんだん息ができなくなることを意味していました。

    お別れしなければなりませんでした。

    2026年10月26日の月曜日、正午ごろ、いちばん愛しい Baylee は安らかに眠りにつきました。

baylee-chapter8Title = いつまでも日だまりの中で
baylee-chapter8Body =
    彼女が私たちのもとにいたのは、1年に少し足りない間でした。その間に、彼女は私たちの家族になり、とても近しい友になりました。仕事をする日々にも、夜にも、朝の目覚めにも、彼女はいました。バルコニーの日だまりの時間にも、ともに暮らすことをかたちづくる小さな習慣にも。

    今は、そのどの瞬間にも彼女がいません。

    空白は、まだ生々しく、深いままです。ゴロゴロという声も、脚にかかる重みも、ぴんと立ったしっぽも、私たちがやっと朝ごはんを用意すると信じて疑わない顔も、恋しいです。

    この人生のあとに何かがあるなら、それがどんなものであっても、いつかあの魂にまた会いたいと願います。

    それまでは、私たちの小さなネズミさんを、いつまでも心の中に抱いていきます。

    だから、この趣味のプロジェクトは Baylee に捧げられています。彼女の魂、彼女のぬくもり、日差しと鳥たちを喜ぶ心に。そして、ともに過ごした時間に育った愛に。

    ここで彼女が、雲の上で、暖かな日差しの中、花とハトと庭の鳥たちに囲まれて、安らかに眠れますように。

    安らかに眠って、いちばん愛しい Baylee。

baylee-photoOpen = 写真を大きく表示
baylee-photoClose = 写真を閉じる
baylee-zoomIn = 拡大
baylee-zoomOut = 縮小
baylee-fit = 全体表示

baylee-photo6 = ブランケットの中で心地よさそうな Baylee。
baylee-photo7 = 赤い術後服を着て身を寄せています。
baylee-photo8 = 段ボール箱の日だまり。
baylee-photo9 = くるまれて、すっかりくつろいでいます。
baylee-photo10 = お気に入りの場所：私たちのすぐ隣。

## Errors shared by the forms

error-busy = 試行回数が多すぎます。しばらくしてからもう一度お試しください。誤ったパスワードが何度か続いた後は、1時間、長くても1日かかることがあります。
error-unavailable = Baylee Central がいま応答していません。しばらくしてからもう一度お試しください。
error-network = 接続がありません。ネットワークを確認して、もう一度お試しください。
error-failed = 問題が発生しました。もう一度お試しください。何度も起きる場合はお知らせください。
error-again = もう一度試す
password-show-label = パスワードを表示

## Signing in

signin-title = ログイン
signin-lead = オンラインのテーブル、連絡先、チャットのための Baylee アカウント。
signin-name = ログイン名
signin-password = パスワード
signin-submit = ログイン
signin-missing = ログイン名とパスワードを入力してください。
signin-working = パスワードを保護しています … 少し時間がかかりますが、処理はブラウザー内だけで行われます。
signin-wrong = ログイン名とパスワードが一致しません。両方を確認して、もう一度お試しください。
signin-not-moved = このアカウントはまだ Baylee Central に移行していません。Baylee クライアントで一度ログインしてください。アカウントが移行されます。
signin-no-account = まだアカウントがありませんか？ <register>作成する</register>。
signin-forgot = パスワードを忘れましたか？ <reset>リセットコードで新しいパスワードを設定する</reset>。
register-have-account = すでにアカウントをお持ちですか？ <signin>ログイン</signin>。
dialog-close = 閉じる
signin-noscript = ログインには JavaScript が必要です。パスワードはブラウザー内で処理され、私たちに送られることはありません。

## Registering

register-title = アカウントを作成
register-lead = オンラインのテーブル、連絡先、チャットのために。オープンベータ中は誰でも無料です。
register-name = ログイン名
register-name-hint = 非公開で、ログインにだけ使います。小文字の英字、数字、ドット、ハイフン、アンダースコアで3〜24文字。
register-display = 表示名
register-display-hint = ほかのプレイヤーに見える名前です。英字、数字、- と _ で3〜16文字。タグは自動で付きます。
register-password = パスワード
register-password-hint = 8文字以上。長い文は覚えやすく、推測されにくいです。
register-repeat = パスワードを再入力
register-repeat-hint = 入力ミスを防ぐため、もう一度同じものを入力してください。
register-age = 16歳以上です。
register-consent = <terms>利用規約</terms>に同意します。データの扱いは<privacy>プライバシー通知</privacy>に記載されています。
register-submit = アカウントを作成
register-error-name = ログイン名は、小文字の英字、数字、ドット、ハイフン、アンダースコアで3〜24文字にしてください。
register-error-display = 表示名は英字または数字で3〜16文字にしてください。- と _ は途中だけに使えます。
register-error-reserved = この表示名は予約されています。別の名前を選んでください。
register-error-short = パスワードは8文字以上にしてください。
register-error-long = パスワードが長すぎます。
register-error-repeat = 2つのパスワードが同じではありません。
register-error-age = アカウントを持つには16歳以上である必要があります。
register-error-consent = アカウントを作成するには、利用規約に同意してください。
register-error-taken = このログイン名は使われています。別の名前を選んでください。

## The recovery code

code-title = 回復コード
code-handle = あなたのハンドル：{ $handle }
code-lead = パスワードを忘れたとき、暗号化されたデータが残るのは、このコードがある場合だけです。私たちはこのコードを知らず、復元もできません。この端末から離れた安全な場所に保管してください。
code-shown-once = 表示は今だけです
code-download = テキストファイルとして保存
code-kept = 回復コードを保存しました
code-not-kept = 先にコードを保存してから、確認してください。
code-continue = 続ける
code-later = 今はしない
code-copy = コピー
code-copied = コピーしました。
code-copy-failed = ここではコピーできませんでした。コードを選択して、手でコピーしてください。
code-print = 印刷
code-manager = パスワードマネージャーに保存
code-manager-id = Baylee 回復コード（{ $name }）
code-manager-offered = ブラウザーにコードを渡しました。確認を求められた場合は、保存されたかどうか確かめてください。
code-manager-failed = ブラウザーはコードを受け取りませんでした。ほかの方法で保管してください。
code-mail = 自分宛てにメールで送る
code-mail-warning = ご注意：コードがメールボックスに残ります。そのメールを読める人は、そこにリセットコードも送らせることができ、両方があればログインをリセットして、暗号化されたデータを開けます。
code-mail-subject = Baylee 回復コード
code-sheet-title = Baylee 回復コード
code-sheet-name = ログイン名：{ $name }
code-sheet-what = このコードがあれば、パスワードを忘れても、暗号化された Baylee のデータが残ります。リセットページで、リセットコードと一緒に入力します。
code-sheet-keep = あなただけが手に取れる場所に保管してください。Baylee の誰もこのコードを尋ねることはありません。
code-email-label = メールアドレス（任意）
code-email-hint = パスワードを忘れたときのリセットコード用です。この手順のあと、アドレスを確認するコードをメールでお送りします。アドレスはあとからアカウントで追加することもできます。
code-email-step-title = メールアドレスの確認
code-email-step-lead = メールに届いたコードをここに入力するか、あとでアカウントのセキュリティページで入力してください。
code-email-step-done = アカウントへ進む

## Setting a new password with a reset code

reset-title = 新しいパスワードの設定
reset-lead = パスワードを忘れたときのために。リセットコードがあれば、ここで新しいパスワードを設定できます。コードは、確認済みのメールアドレスに届くか、管理者から渡されます。
reset-noscript = 新しいパスワードの設定には JavaScript が必要です。パスワードと回復コードはお使いのブラウザー内で処理され、当方には送信されません。
reset-code = リセットコード
reset-code-hint = メールまたは管理者から届きます。RESET で始まり、1回だけ使えます。メールで届いたものは1時間、管理者が作ったものは24時間有効です。
reset-data = 暗号化されたデータ
reset-keep = 回復コードがあります。データは残します。
reset-new = 回復コードがありません。空の保管庫から始めます。
reset-recovery = 回復コード
reset-recovery-hint = アカウント作成時に保存したコードです。ブラウザー内にとどまり、当方には送信されません。
reset-warning = 空の保管庫から始めると、暗号化されたデータは完全に失われます。アカウントに保存された設定、デッキのバックアップ、チャット履歴が対象です。誰にも元に戻せず、私たちにもできません。アカウント、表示名、連絡先は残ります。新しい回復コードが発行されます。
reset-warning-confirm = このデータが完全に失われることを理解しました。
reset-submit = 新しいパスワードを設定
reset-missing = ログイン名、リセットコード、新しいパスワードを入力してください。
reset-recovery-missing = 回復コードを入力するか、空の保管庫を選んでください。
reset-error-code = これはリセットコードではありません。RESET で始まり、4文字ずつ8つのグループが続きます。
reset-error-recovery = この回復コードでは、アカウントのデータを開けません。確認して、もう一度お試しください。何も変更されておらず、リセットコードもそのまま使えます。
reset-error-confirm = 先に、何が失われるかを理解したことを確認してください。
reset-refused = このログイン名とリセットコードは対応していないか、コードの期限が切れているか、すでに使われています。両方を確認してください。それでも解決しない場合は、新しいコードを依頼してください。
reset-done-title = 新しいパスワードを設定しました
reset-done-kept = データと回復コードはそのままです。すべての場所からログアウトされました。新しいパスワードでログインしてください。
reset-done-new = 空の保管庫から始まり、すべての場所からログアウトされました。下に新しい回復コードを表示します。古い回復コードはもう使えません。
reset-unsure-title = 応答がありませんでした
reset-unsure = Baylee Central が応答する前に接続が切れたため、新しいパスワードが設定されたかどうか分かりません。新しいパスワードでログインしてみてください。ログインできなければ、何も変更されていません。リセットコードはそのまま使えるので、ここでもう一度入力してください。
reset-unsure-code = 新しいパスワードでログインできた場合は、これが新しい回復コードで、古い回復コードはもう使えません。再表示はできないので、今すぐ保存してください。新しいパスワードでログインできない場合、このコードは無効です。
reset-to-signin = ログイン
reset-mail-title = リセットコードが届きませんか？
reset-mail-lead = ログイン名を入力してください。アカウントに確認済みのメールアドレスがあれば、そこにリセットコードを送ります。
reset-mail-name-hint = ログインに使う名前です。メールアドレスではありません。
reset-mail-submit = コードをメールで送る
reset-mail-error-name = これはログイン名ではありません。メールアドレスではなく、ログインに使う名前を入力してください。
reset-mail-sent = このアカウントに確認済みのメールアドレスがあれば、リセットコードがそこへ送られます。有効期間は1時間です。
reset-mail-busy = ただいまリクエストが多すぎます。しばらくしてからもう一度お試しください。
reset-mail-none = このサーバーはメールを送信しません。リセットコードは管理者にお問い合わせください。
reset-mail-early = 操作が早すぎます。少し待ってからもう一度お試しください。

## Changing the password

password-new = 新しいパスワード
password-new-repeat = 新しいパスワードを再入力
password-current = 現在のパスワード
password-change-submit = パスワードを変更
password-missing = 現在のパスワードと新しいパスワードを入力してください。
password-error-current = 現在のパスワードが正しくありません。確認して、もう一度お試しください。
password-changed = パスワードを変更しました。この場所ではログインしたままですが、それ以外のすべての場所（Baylee クライアントを含む）ではログアウトされます。そちらでは新しいパスワードでログインしてください。
password-unsure = 応答がなかったため、パスワードが変更されたかどうか分かりません。次回のログインでは、まず新しいパスワードを試し、だめなら古いパスワードを試してください。

## An account's e-mail address

email-title = メールアドレス
email-lead = メールアドレスは任意です。リセットコードを求められたときの送信と、アドレスが置き換えられたり削除されたりしたときの通知に使います。
email-warning = このメールボックスを読める人は、あなたのログインをリセットできます。暗号化されたデータがリセット後も残るのは、回復コードがある場合だけです。
email-no-mail = このサーバーはメールを送信しないため、ここではメールアドレスを設定も確認もできません。
email-unavailable = 現在、メールアドレスを表示できません。
email-password-hint = 現在のパスワードです。このブラウザーを使っている他の人が、誰がログインをリセットできるかを決められないようにするためです。
email-submit = 確認コードを送る
email-missing = メールアドレスとパスワードを入力してください。
email-error-address = このアドレスにはメールを送れません。name@example.org のような、1つの通常のアドレスを入力してください。アクセント付きの文字やウムラウトは使えません。
email-current = あなたのアドレス
email-confirmed-at = 確認済み
email-waiting = コード待ち
email-change = 変更
email-change-lead = 新しいアドレスを確認するまで、現在のアドレスが使われます。
email-change-cancel = 現在のアドレスを使い続ける
email-remove = 削除
email-remove-ask = このアドレスを削除しますか？削除したことを、そのアドレスに通知します。その後は、リセットコードをメールで受け取れなくなります。
email-remove-ask-no-mail = このアドレスを削除しますか？このサーバーがメールを送信しない間は、アドレスを再び設定できません。
email-remove-confirm = アドレスを削除
email-remove-keep = 削除しない
email-pending = <code>{ $address }</code> にコードを送りました。有効期間は15分です。
email-pending-keeps = 確認するまでは、<code>{ $address }</code> があなたのアドレスのままです。
email-code = 確認コード
email-code-hint = XXXX-XXXX のように8文字です。大文字と小文字のどちらでも、ハイフンの有無も問いません。
email-confirm = 確認
email-resend = 再送
email-other = 別のアドレスを使う
email-withdraw = 取り下げ
email-error-code = これは確認コードではありません。XXXX-XXXX のように8文字です。
email-wrong-code = コードが違います。確認して、もう一度お試しください。何度か間違えると、アドレスを入力し直す必要があります。
email-code-dropped = このコードも違っていたため、アドレスはコード待ちではなくなりました。もう一度入力すると、新しいコードが届きます。
email-now-confirmed = メールアドレスを確認しました。
email-resent = 新しいコードを送信しています。前のコードはもう使えません。
email-withdrawn = アドレスを取り下げました。コードはもう待っていません。
email-removed = メールアドレスを削除しました。
email-busy = 現在メールが多すぎます。しばらくしてからもう一度お試しください。
email-same = すでにこのアドレスです。
email-unsure = 応答がなかったため、処理が完了したかどうか分かりません。ここに表示されている内容を確認し、完了していなければもう一度お試しください。
email-again = コードを送るには、もう一度パスワードを入力してください。

## Central's terms

terms-step-title = 新しい利用規約
terms-step-lead = Baylee Central の利用規約が変わりました。アカウントを使うには、読んで同意してください。同意しない場合は、ログアウトもできます。
terms-accept = 同意する
terms-decline = 同意せずログアウト

## The signed-in area

account-lead = あなたの Baylee アカウント：プロフィール、セキュリティ、プライバシー、Gateway。
account-nav = アカウント
account-home = 概要
account-profile = プロフィール
account-profile-lead = ハンドル、表示名、回復コード。
account-security = セキュリティ
account-security-lead = ログイン中の場所と、ログアウト。
account-privacy = プライバシーとデータ
account-privacy-lead = データのエクスポートと、アカウントの削除。
account-gateways = Gateway とトークン
account-gateways-lead = 登録した Gateway と、そのログインに使うトークン。
account-loading = 読み込み中 …
account-signed-out = ログインしていません。
account-unnamed = アカウント
account-signin-name = ログイン名：{ $name }
account-until = { $time } までログイン中
account-needs-title = 対応が必要です
account-needs-code = 回復コードを作成
account-needs-name = 表示名を選ぶ
account-admin = 管理

reauth-lead = この操作には最近のログインが必要です。ログイン名とパスワードをもう一度入力してください。
reauth-submit = もう一度ログイン

profile-who-title = あなたについて
profile-handle = ハンドル
profile-signin-name = ログイン名
profile-source = アカウント
profile-source-moved = 公式 Gateway から移行
profile-source-registered = このサイトで登録
profile-choose-title = 表示名を選ぶ
profile-choose-submit = 表示名を保存
profile-name-taken = このアカウントには、すでに表示名があります。
profile-code-title = 回復コード
profile-code-kept = アカウントには回復コードがあります。大切に保管してください。パスワードがないとき、暗号化されたデータに戻る唯一の方法です。
profile-code-missing = アカウントには、まだ回復コードがありません。作成してください。パスワードをもう一度入力する必要があります。
profile-code-make = 回復コードを作成

security-sessions = ログイン中の場所
security-lead = Baylee Central は、このウェブサイト用に一つ、ゲームクライアント用に一つのセッションを保持します。個々の端末は区別しません。
security-kind-browser = このウェブサイト
security-kind-client = Baylee クライアント
security-this = これ
security-signed-in = ログイン
security-ends = 終了
security-gateways = Gateway にて
security-end-client = クライアントをログアウト
security-client-ended = クライアントはログアウトしました。
security-sign-out = ここをログアウト
security-password-title = パスワード
security-password-lead = 暗号化されたデータと回復コードはそのままです。パスワードを変更すると、この場所以外のすべての場所からログアウトされます。

privacy-export-title = データのエクスポート
privacy-export-lead = Baylee Central があなたのアカウントについて把握していることを、JSON ファイルにまとめます。基本データ、セッション、同意した規約、Gateway、あなたのアカウントに関する監査記録です。暗号化された内容を復号してエクスポートできるのは、Baylee クライアントだけです。エクスポートには、30分以内のログインが必要です。
privacy-export-make = エクスポートを準備
privacy-export-save = エクスポートを保存
privacy-export-no-email = メールアドレスを読み取れませんでした。このファイルには含まれていません。
privacy-delete-title = アカウントを削除
privacy-delete-lead = アカウント、そのデータ、セッション、連絡先、あなたが所有する Gateway は、稼働中のシステムから直ちに完全に削除されます。バックアップからは、遅くとも12か月以内に消えます。Central は、あなたが送ったメッセージも削除します。ほかの人がすでに端末に保存したものは、そこに残ることがあります。
privacy-delete-start = アカウントを削除する
privacy-delete-password = 確認のためパスワードを入力
privacy-delete-prove = 続ける
privacy-delete-last = 最後の手順です。取り消せません。時間は1分です。
privacy-delete-gateways = あなたが所有する次の Gateway も一緒に削除されます：
privacy-delete-confirm = アカウントを完全に削除
privacy-delete-cancel = アカウントを残す
privacy-deleted = アカウントを削除しました。

gateways-none = Gateway を登録していません。ほとんどのプレイヤーには必要ありません。
gateways-add-title = Gateway を登録
gateways-origin = Gateway のアドレス
gateways-origin-hint = Gateway の https アドレス。例：https://play.example.org
gateways-name = Gateway の名前
gateways-add = Gateway を登録
gateways-added = Gateway を登録しました。管理者の承認を待っています。
gateways-taken = このアドレスはすでに登録されています。
gateways-invalid = パスのない https アドレスと、名前を入力してください。
gateways-approved = 承認済み
gateways-pending = 承認待ち
gateways-disabled = 無効
tokens-title = トークン
tokens-name = トークン名
tokens-days = 有効期間（日）
tokens-sso = プレイヤーをログインさせる
tokens-listing = Gateway を一覧に載せる
tokens-both = プレイヤーをログインさせ、Gateway を一覧に載せる
tokens-make = トークンを作成
tokens-revoke = 失効させる
tokens-revoked = 失効済み
tokens-revoked-now = トークンを失効させました。
tokens-until = { $time } まで
tokens-invalid = トークンに名前を付け、許可することを少なくとも一つ選んでください。
tokens-shown-title = 新しいトークン
tokens-kept = 保存しました
tokens-again = 再ログインしました。もう一度トークンを作成してください。

## The messenger

account-chat = チャット
account-contacts = 連絡先
chat-side = 会話と連絡先
chat-tabs = メッセンジャー
chat-tab-chats = チャット
chat-tab-contacts = 連絡先
chat-main = 会話
chat-unread = 未読
chat-conversations = 会話
chat-search = 会話を探す
chat-search-hint = 検索 (Ctrl+K)
chat-new-group = 新しいグループ
chat-no-conversations = まだ会話がありません。連絡先を追加して、メッセージを送りましょう。
chat-no-match = その名前の会話はありません。
chat-former-group = 以前のグループ
chat-former-lead = このグループにはもういません。
chat-you = あなた
chat-members = { $n ->
   *[other] メンバー { $n } 人
}
chat-say-hello = あいさつしましょう。
chat-history-only = もう連絡先ではありません。
chat-today = 今日
chat-yesterday = 昨日
chat-encrypted = エンドツーエンド暗号化
chat-opening-lead = あなたの会話と連絡先です。メッセージは端末を出る前に暗号化され、Central は読めないまま中継します。
chat-new-from = { $who } からの新着メッセージ。
chat-new-many = 新着メッセージ { $n } 件。
chat-retry = もう一度試す
chat-discard = 破棄
chat-not-sent = 未送信。
chat-sending = 送信中 …
chat-keys-changed = この会話の誰かの鍵が変わりました。もう一度書く前に、安全番号を比べてください。
chat-refused = 検証できなかったメッセージがあり、表示していません。
chat-failed = チャットがいま Central に接続できませんでした。
chat-failed-open = チャットを開けませんでした。ページを再読み込みして、もう一度お試しください。
chat-empty = まだメッセージがありません。
chat-older = 古いメッセージを表示
chat-compare-first = 鍵が変わりました。先に安全番号を比べてから、書いてください。
chat-compare = 安全番号を比べる
chat-friend-required = 再び連絡先になると、書き込めます。
chat-group-left = このグループにはもういません。
chat-details = 詳細
chat-close = 閉じる
chat-back = 会話に戻る
chat-messages = メッセージ
chat-fan = Baylee は非公式のファンコンテンツで、ウィザーズ社の承認や推奨を受けたものではありません。<more>その意味</more>
chat-safety = 安全番号
chat-safety-lead = 番号を相手と比べてください。直接会うか、信頼できる別の手段で。一致すれば、あなたたちの間に誰かが気づかれず入り込んではいません。
chat-changed = 変更あり
chat-confirm-keys = 一致しました：新しい鍵を信頼する
chat-count = { $n } / 1,000 文字
chat-write = メッセージを書く
chat-send = 送信
chat-enter-hint = Enter で送信、Shift+Enter で改行します。
chat-choose = 会話を選んでください
chat-choose-lead = メッセージはこのブラウザーで暗号化され、Central は中継するだけです。
chat-storage = ログインしている間、チャットの鍵はこのブラウザーに残ります。
chat-opening = チャットを開いています …
chat-open = チャットを開く
chat-ended = ログインが終了しました。
chat-unnamed = 先に表示名を選んでください。他の人が書き込むときのハンドルになります。
chat-missing-title = チャットを開くには、もう一度ログインしてください
chat-missing-lead = このブラウザーにはチャットの鍵がありません。パスワードでここで開けます。パスワードがこのブラウザーの外に出ることはありません。
chat-invites = グループへの招待
chat-invited-by = { $who } から
chat-group-name = グループ名
chat-group-name-hint = 1〜48文字の名前。
chat-group-create = 作成
chat-group-failed = グループをいま変更できませんでした。
chat-group-members = { $n ->
   *[other] メンバー { $n } 人
}
chat-group-owner = オーナー
chat-group-rename = 名前を変更
chat-group-transfer = オーナーにする
chat-group-remove = 外す
chat-group-transfer-ask = { $who } をこのグループのオーナーにしますか？ あなたはメンバーのままです。
chat-group-remove-ask = { $who } をこのグループから外しますか？
chat-group-invite = 連絡先を招待
chat-group-choose = 連絡先を選ぶ
chat-group-invite-button = 招待
chat-group-invited = 招待済み
chat-group-delete = グループを削除
chat-group-leave = グループを抜ける
chat-group-delete-ask = 「{ $name }」を全員分削除しますか？ 受信済みのメッセージは読めるままです。
chat-group-leave-ask = 「{ $name }」を抜けますか？ 戻るには新しい招待が必要です。

people-incoming = リクエスト
people-failed = 連絡先をいま読み込めませんでした。
people-empty = まだ連絡先がありません。ハンドルで相手を探すか、自分のハンドルを伝えましょう。
people-your-handle = あなたのハンドル：
people-incoming-title = あなた宛てのリクエスト
people-friends = 連絡先
people-outgoing = 送信したリクエスト
people-blocked = ブロック中
people-find = ハンドルで連絡先を追加
people-find-button = 探す
people-handle-hint = タグ付きの完全なハンドル。例：Name#1a2b
people-yourself = それはあなた自身のハンドルです。
people-nobody = このハンドルの人はいません。
people-request = 連絡先リクエストを送る
people-requested = リクエストを送りました。
people-is-friend = 連絡先
people-is-incoming = 連絡先になりたいとのリクエスト
people-is-outgoing = リクエスト送信済み
people-is-blocked = ブロック中
people-write = 書く
people-remove = 削除
people-block = ブロック
people-accept = 承認
people-decline = 断る
people-cancel = 取り下げ
people-unblock = ブロック解除
people-keep = 残す
people-confirm-remove = { $who } を連絡先から削除しますか？ メッセージは残ります。
people-confirm-block = { $who } をブロックしますか？ 相手は連絡先のリクエストも、あなたへの書き込みもできなくなります。

## Offline, updates, installing

title-offline = オフライン
offline-title = オフラインです
offline-lead = このページには接続が必要です。Central は、接続が戻るまでメッセージを預かります。
offline-again = もう一度試す
update-ready = baylee.app の新しいバージョンが用意できました。
update-unsent = ここに未送信の文章があります。それでも再読み込みしますか？
update-reload = 再読み込み
install-app = アプリとしてインストール

## Notifications

push-title = 通知
push-lead = Baylee からの通知の内容を選びます。通知には、メッセージの本文、名前、報告の内容は表示されません。
push-device = この端末の通知
push-categories = 通知する内容
push-chat = チャットのメッセージ
push-contacts = 連絡先リクエスト
push-updates = 新しいバージョン
push-feedback = フィードバックの報告（管理者）
push-gateways = Gateway の申請（管理者）
push-on = この端末で有効です。選んだカテゴリーだけが届きます。
push-off = この端末で無効です。カテゴリーの選択は保持されます。
push-scope = カテゴリーの選択はアカウントに適用されます。端末の許可は、ブラウザーごとに別です。ログアウトすると、この端末は切断されます。
push-failed = 変更を確認できませんでした。もう一度お試しください。
push-denied = 通知がブロックされています。ブラウザーまたは端末の設定で許可してから、もう一度お試しください。
push-unsupported = このブラウザーでは、ここで Web Push を使えません。iPhone と iPad では、Baylee をホーム画面に追加して、そこから開いてください。
push-unconfigured = バックグラウンド通知は、このサーバーではまだ使えません。設定は、今から保存できます。

register-closed = 登録はいま一時停止しています。しばらくしてからもう一度お試しください。
register-refused = 登録を受け付けられませんでした。もう一度お試しください。
admin-registration = 登録
admin-registration-open = 開放中：proof of work を解けば、誰でもアカウントを作成できます。
admin-registration-closed = 閉鎖中：もう一度開放するまで、誰もアカウントを作成できません。
admin-registration-lead = 攻撃を受けている間は登録を閉じ、手動でもう一度開きます。変更はすべて監査ログに記録されます。
admin-registration-close = 登録を閉じる
admin-registration-reopen = 登録をもう一度開く
hero-discord = Discord コミュニティ
hero-whatsapp = WhatsApp コミュニティ
hero-github = GitHub
nav-menu = メニュー
foot-project = プロジェクト
foot-source = ソースコード (AGPL-3.0)
foot-central = Baylee Central（ソース公開）
foot-summary = Rust と Bevy で作られた、無料でオープンソースのテーブルトップ・カードゲーム。決定論的なルールエンジン、完全なオフラインモード、ハウスAI、LLMブリッジ、コミュニティ Gateway を備えています。
strip-ci = CI
strip-commit = コミット
strip-gateways = Gateway
strip-online = オンライン
strip-offline = オフライン
strip-directory = 一覧
strip-cards = プレイ可能なカード
strip-cards-unit = 枚
strip-accounts = Baylee Central のアカウント
strip-live = Gateway が報告する、今の数
strip-views = { $date } からのページビュー
live-views = { $n ->
   *[other] { $n } ビュー
}
live-accounts = { $n ->
    [one] { $n } アカウント
   *[other] { $n } アカウント
}
live-players = { $n ->
    [one] { $n } 人のプレイヤー
   *[other] { $n } 人のプレイヤー
}
live-playing = { $n ->
   *[other] { $n } 人がゲーム中
}
live-tables = { $n ->
    [one] 開いているテーブル { $n } 件
   *[other] 開いているテーブル { $n } 件
}
live-games = { $n ->
    [one] { $n } ゲーム
   *[other] { $n } ゲーム
}
hero-badges = 概要
hero-badge-beta = オープンベータ
hero-badge-free = 無料
hero-badge-open = オープンソース
hero-badge-fan = 非公式ファンコンテンツ
gws-open-client = Baylee で開く
gws-open-client-help = 「Baylee で開く」は、インストール済みのクライアント（0.1.1-beta.9b 以降）を起動します。アドレスを表示し、Gateway を追加する前に確認します。
dl-menu-title = すべてのシステム
dl-old-cpu = 古いプロセッサー（AVX2 なし）
ci-passing = 成功
ci-failing = 失敗
dl-button = Baylee をダウンロード
dl-quick = ほかのシステムとアーキテクチャ
dl-others = ほかのシステム

# The start page, as the old site wrote it (owner, 11.11.2026)
link-readmeModel = #playing-against-a-language-model
about-engineTitle = すべてのルールを守らせるエンジン
about-deterministic = 実時間にもハッシュの順序にも依存しない、シード付き乱数生成器で動くため、すべての対戦は厳密に決定論的で、一手ずつ再現できます。
about-tableTitle = Bevy で作った、すっきりした3Dテーブル
about-keys = 統合されたロビーから、ローカルのデッキビルダー、一人用の対戦、コミュニティ Gateway 上のマルチプレイヤーのテーブルへ、気軽に行き来できます。
feature-hiddenTitle = 非公開の情報は非公開のまま
feature-hidden = クライアントに届くのは、あなたの席が見てよいデータだけです。ライブラリーと相手の手札は枚数としてだけ届くので、改造したクライアントでも、隠されたカードは見られません。
feature-seatsTitle = 一つのテーブルに最大8人
feature-seats = 最大8席までの対戦、全員対戦、チーム戦（2対2など）。チームメイトはターンを共有し、手札をお互いに公開できます。
feature-commanderTitle = 柔軟なフォーマットと自由なハウスルール
feature-commander = 60枚の定番の対戦、多人数の円卓、自分たちのハウスフォーマットまで、デッキを組めます。開始時のライフ、チームの配置、マリガンのルールは、ホストが自由に設定できます。
feature-aiTitle = 決定論的なリプレイと F8 報告
feature-ai = すべての対戦は一手ずつ記録されます。プレイ中にルール上の相互作用の問題に気づいたら、テーブルでそのまま F8 を押すと、再現できる状態の報告を送れます。
feature-processTitle = 対戦ごとに分離されたサーバープロセス
feature-process = 進行中のゲームはすべて、Gateway 上のサンドボックス化された専用のエンジンプロセスで動きます。Gateway 自体が管理するのはロビーとルームだけなので、エラーがほかのテーブルに及ぶことはありません。
feature-languagesTitle = 19言語のカードテキスト
feature-languages = Gateway が多言語のカードカタログを提供していれば、カードは希望の言語で表示され、英語が代わりに使われます。クライアントのUIはドイツ語と英語で使えます。
feature-watchTitle = ライブ対戦を観戦
feature-watch = ロビーの観戦ボタンで、ライブの観戦者としてテーブルに着けます。公開された戦場をリアルタイムで追え、プレイヤーの手札は隠されたままです。
feature-connectionTitle = 切断してもスムーズに再接続
feature-connection = 接続が一時的に切れても、テーブルはカウントダウンつきで席を確保し、対戦を一時停止するか、一時的にハウスAIに代わらせることができます。
feature-tablesTitle = 14段階の持ち時間
feature-tables = テンポのよい15秒ターンから、のんびりした時間無制限のテーブルまで、ペースは部屋ごとに設定できます。
shots-eyebrow = テーブルにて
shots-title = ゲームの中
shots-lead = Scryfall のカード画像（© Wizards of the Coast）を使い、3Dクライアントから直接録画した短い映像です。対戦や4人の円卓から、暗号化されたロビーとデッキビルダーまで。
shots-credit = カード画像：Scryfall · © Wizards of the Coast
shots-showLarge = 大きく表示：{ $title }
shots-dialog = スクリーンショット
shots-previous = 前へ
shots-next = 次へ
shots-close = 閉じる
ai-eyebrow = AI の対戦相手と言語モデル
ai-title = ハウスAI、LLMブリッジ、Baylee 独自のAI
ai-houseTitle = 5段階の強さのヒューリスティック・ハウスAI
ai-house = オフラインでも、どのオンラインのテーブルでもすぐに遊べます。初心者、カジュアル、堅実、鋭い、エキスパートから選べます。ハウスAIは待ち時間なしでローカルに判断し、人間のプレイヤーと同じ公開状態しか見ません。
ai-modelTitle = Seat-Bridge で、言語モデルがテーブルに
ai-model = <b>Google Gemini、OpenAI ChatGPT、Anthropic Claude、DeepSeek</b>、または <b>LM Studio</b> のローカルモデルを、テーブルにそのまま座らせられます。API キーは、利用上限つきの環境変数で保護されます。タイムアウトしたときは、ハウスAIが途切れなく代わります。
ai-setup = LLM と Seat-Bridge のセットアップガイド
home-gwLead = オンラインのテーブルは Gateway 上にあります。Gateway は、管理者が承認した後に、Baylee Central が一覧に載せるサーバーです。ここには最初のいくつかを載せています。全部は一覧にあります。
beta-open = 一覧を開く
beta-own = 自分で運営する
source-eyebrow = ソース
source-title = オープンソースです。これからも
source-promise = Baylee はこれからもずっとオープンソースで、無料です。課金も、有料機能も、広告もありません。今も、この先も。コードは GNU Affero General Public License の下にあり、改変した Gateway を他の人のために運営する人は、そのソースも提供しなければなりません。
source-rust = Rust で書かれた、一人のプロジェクトで、コーディングエージェントの大きな助けを借りています。すべての変更は同じ関門を通ります。書式、リント、テストスイート、そしてすべてのカードを印刷されたテキストと照らす検証です。Issue とプルリクエストを歓迎します。<agents>AGENTS.md</agents> と <claude>CLAUDE.md</claude> は、人間とエージェントの両方のための作業ルールです。
source-discord = Discord のコミュニティ
source-whatsapp = WhatsApp のコミュニティ
source-architecture = アーキテクチャ
source-repoTitle = リポジトリの現在の状況
source-stars = スター
source-issues = 未解決の Issue とプルリクエスト
source-note = このサイトのサーバーが GitHub の公開 API から10分ごとに読み取っています。ブラウザーが GitHub に問い合わせることはありません。
gw-count =
    { $n ->
       *[other] 登録されている Gateway は { $n } 件で、{ $online } 件がオンラインです。
    }
gw-none = 登録されている Gateway はまだありません。
contact-intro = 質問、アイデア、遊んでみたいゲーム、公開の場には書きたくないこと。このフォームは、Baylee の作者に直接届きます。返信が必要な場合はメールアドレスを残してください。なければ、読むことしかできません。
contact-f8 = 誤って解決されたカードは、ゲーム内で <kbd>F8</kbd> から報告するのがいちばんです。報告にはゲーム全体を添えられるからです。
contact-discord = 雑談、対戦相手探し、新しいビルドの情報
contact-issues = GitHub の Issue
contact-issuesSub = 不具合とアイデア（公開）
contact-operator = このサイトの運営者と郵送先住所：<impressum>法的表示</impressum>。
contact-whatsappSub = コミュニティグループとクイックマッチ
contact-reportsTitle = 報告とプライバシー
contact-reportsSub = F8 報告とデータの詳細
ai-housePill = オフラインとオンライン
ai-providers = 対応している言語モデル
ai-hostsPill = Gateway ホスト向け
ai-hostsTitle = Gateway でホスト型の LLM 席を提供する
ai-hosts = Gateway の運営者は、設定済みの言語モデルをサーバー上で直接提供できます。テーブルのゲストは、ホストされているモデルを対戦相手として選ぶだけです。API キーは要りません。
ai-trainingPill = 訓練中 · ネイティブなエンジンAI
ai-trainingTitle = Baylee 独自のAI — 本物のプレイヤーのように、エンジン上で直接プレイするよう訓練中
ai-training = ハウスAIと外部の LLM ブリッジのほかに、決定論的なルールエンジン上で、専用の Baylee 独自のAIを直接訓練しています。盤面、優先権の窓、戦闘の計算、デッキのシナジーを、経験豊富な人間のプレイヤーのように評価することを学びます。外部APIの遅延はゼロです。
ai-hostingLink = Gateway のホスティングと LLM の設定

# The gateways and the contact page, as the old site wrote them (owner, 11.11.2026)
gw-lead = Gateway は、オンラインのテーブルがあるサーバーです。アカウント、デッキ、ルーム、ゲームごとに1つのエンジンプロセスがあります。ここに載っているのは、Baylee Central に登録され、その管理者が承認したものです。それぞれ運営者が自分たちのグループのために運営しており、そこでは運営者自身のルールが適用されます。一覧に載っていることは、Gateway の動作についての約束ではありません。
gw-howTo = Gateway で遊ぶには、そのアドレスをクライアントの入口にコピーします。オンラインとは、Central が直近1分半以内にその Gateway から連絡を受けたという意味で、応答する約束ではありません。名前とバージョンは、各運営者が報告したものです。
gw-find = Gateway を探す
gw-findHint = 名前またはアドレス
gw-shown = { $all } 件中 { $n } 件を表示
gw-noMatch = 読み込み済みの Gateway には、一致するものがありません。
gw-showAll = すべて表示
gw-stats = ディレクトリの数字
gw-stat-listed = 掲載中の Gateway
gw-stat-online = 現在オンライン
gw-stat-players = オンラインのプレイヤー
gw-stat-playing = うちゲーム中
gw-stat-tables = 開いているテーブル
gw-stat-games = 進行中のゲーム
gw-show = 表示
gw-show-every = すべて
gw-show-online = オンライン
gw-show-tables = 開いているテーブルあり
gw-sort = 並べ替え
gw-sort-best = おすすめ
gw-sort-name = 名前順
gw-sort-players = プレイヤー数が多い順
gw-sort-tables = 開いているテーブルが多い順
gw-sort-version = 新しいバージョン順
gw-view = 表示形式
gw-view-cards = タイル
gw-view-list = リスト
link-readmeGateway = #running-your-own-gateway
selfhost-eyebrow = セルフホストと Central 連携
selfhost-title = 自分の Gateway を運営する
selfhost-processes = セルフホストの Gateway は、PostgreSQL 18+ のほかに、軽量なプロセスを三つ動かします。<code>baylee-gateway</code> がルーム、デッキ、ルーティングを管理し、<code>baylee-agent</code> が対戦ごとに分離された <code>baylee-engine-server</code> プロセスを一つ起動するので、どのテーブルもサンドボックス化されます。
selfhost-proxy = HTTPS のオリジン（例：<code>https://play.example.org</code>）の前に TLS リバースプロキシを置きます。19言語のカードテキストが必要なら、Scryfall のバルクカタログを取り込むこともできます。これで、プレイヤーはクライアントからあなたのアドレスで直接接続できます。
selfhost-central = <b>Baylee Central と連携し、プライバシーを設定する：</b>HTTPS のオリジンを<mine>あなたの Gateway</mine>に登録し、<code>bgt_…</code> トークンを発行します（スコープ：シングルサインオンは <code>sso</code>、一覧へのハートビートとライブのテレメトリは <code>listing</code>）。Gateway のコンソールで、テレメトリのプライバシーレベルを選びます（進行中のゲーム、テーブルの設定、同意したプレイヤーまで含める <code>full</code>、合計だけの <code>counts_only</code>、稼働状態だけの <code>private</code>）。
selfhost-guide = <guide>セルフホストの完全なガイドを読む</guide> · <protocol>プロトコルとハートビートのリファレンス（docs/protocol.md）</protocol>
selfhost-source = すべての Gateway は、<code>GET /source</code> に、ライセンス、バージョン、コミットを、ログインなしで返します。これにより、AGPL のソース開示の要件が自動的に満たされます。
reports-eyebrow = 報告
reports-title = おかしなところを見つけましたか？ お知らせください
reports-f8 = テーブルまたはロビーで <kbd>F8</kbd> を押します。報告には、種類、あなたのテキスト、クライアントのバージョン、テーブルではゲームのIDが入ります。それ以外はそれぞれ専用のチェックボックスがあり、どれも最初はチェックされています。送信を押す前に、残したくないものはチェックを外してください。システムとハードウェア、見ているとおりのテーブル、ログ（ほかのプレイヤーは「Player A」「Player B」…に置き換えられます）、設定、Baylee のウィンドウだけのスクリーンショット、そしてテーブルではゲームの全記録（すべての手と、全席のカード。名前は含みません）です。<i>送信内容を表示</i>で、すべてのバイトを確認できます。セッショントークンは送信されません。
reports-usage = オフラインで遊んでいる間、クライアントは1分に1回、ゲームが動いていることだけを伝えます。アカウントも、端末IDも、ゲームの内容も含みません。設定 › プライバシーとデータの<i>匿名の利用数を送信</i>でオフにできます。
reports-discord = 雑談したい、対戦相手を探したい、新しいビルドをいち早く知りたいという方は、<discord>Discord の Baylee コミュニティ</discord>にどうぞ。
reports-issues = ルールの質問、カードの誤った挙動、アイデアは<issues>GitHub で Issue を開いて</issues>ください。誤って解決されたカードは、いちばん役に立つ報告です。すべてのカードにテストがあり、あなたの報告がそのテストになるからです。
reports-privacy = <docs>docs/privacy.md</docs> に、クライアント、Gateway、フィードバックサービスが保持するものがすべて載っています。このサイトはログインしない限りクッキーを設定せず、アクセス解析を行わず、他者のサーバーからフォントもスクリプトも読み込みません。<notice>プライバシー通知</notice>には、このサイトと Baylee Central が保持するもの、通信するすべてのホストとその時期が記されています。

# The download page, as the old site set it (owner, 11.11.2026)
dlPage-eyebrow = 入手する
dlPage-filesEyebrow = すべてのファイル
dlPage-filesTitle = すべてのシステム
install-eyebrow = インストール
install-releases = GitHub のすべてのリリース
release-notes = リリースノートとチェックサム
release-all = すべてのリリース

# The download page's head, as the old site wrote it
dlPage-title = ダウンロード
dlPage-lead = ダウンロードはすべて、GitHub 上のファイルに直接つながり、間に何も挟みません。インストーラーがあるリリースでは、それがいちばん簡単な方法です。アーカイブは同じゲームで、好きな場所に展開できます。インストールした Baylee は、自分のフォルダーに書き込める場所なら、署名付きのアーカイブから自己更新します。どのパッケージにも、ゲームの隣に言語モデル用の Seat-Bridge が入っています。

# The reading pages' eyebrows, as the old site set them
imp-eyebrow = 法的表示
terms-eyebrow = 利用規約
privacy-eyebrow = プライバシー

# The privacy notice's contents, as the old site named them
privacy-contents = 目次

# The account pages' eyebrow, as the old site set it
acct-eyebrow = Baylee Central

# The account area's navigation, as the old site named it
app-account = アカウント

# ==== The security page (/security) ====================================
# Every sentence and every arrow of this page maps to a row of the claim
# table in docs/security-page.md; a claim changes
# there first. No "safe", "unbreakable" or compliance words: mechanisms only.
title-security = Baylee のセキュリティ：パスワード、鍵、チャット
desc-security = Baylee がパスワード、鍵、チャットのメッセージをどう扱うかを、四つの図で手順ごとに説明します。それぞれの限界も添えて。
nav-security = セキュリティ
sec-eyebrow = セキュリティ
sec-title = パスワード、鍵、チャット：何がどこへ行くか
sec-lead = 四つのフローを手順ごとに：何が端末から出ていくか、Baylee Central に何が見え、何が見えないか。それぞれの限界も添えて。
sec-toc-label = このページの内容
sec-figure = 手順の図：{ $name }
sec-play = 再生
sec-all = 全手順
sec-step = 手順 { $n }
sec-hint = 図は自動で再生されます。手順を選ぶと、そこで止まります。
sec-hint-still = 手順を選ぶと強調されます。
sec-in-browser = 破線の枠はブラウザーです。これらはすべてその中で行われます。
# The actors' names are the only words inside the drawings, where text does
# not wrap: one or two words each, at most about twice the English width
# (the test `names_fit_with_room_to_grow` measures every language).
sec-actor-device = あなたの端末
sec-actor-central = Central
sec-actor-page = ページ
sec-actor-worker = Worker
sec-actor-store = 保存先
sec-actor-you = あなた
sec-actor-contact = 連絡先
sec-actor-website = サイト
sec-actor-client = クライアント

sec-signin-eyebrow = ログインと登録
sec-signin-title = Central は、パスワードを受け取らずに確認します
sec-signin-lead = Baylee は OPAQUE（RFC 9807）でログインします。端末と Central は三つのメッセージをやり取りしますが、パスワードはそのどれにも含まれません。単なるハッシュとしても含まれません。
sec-signin-c1 = 目隠しした要求
sec-signin-t1 = 端末がパスワードに目隠しをします。
sec-signin-s1 = パスワードに新しい乱数を組み合わせ、その結果だけを送ります。そのメッセージからパスワードを読み取ることはできません。
sec-signin-c2 = Central の応答
sec-signin-t2 = Central が自分の鍵で応答します。
sec-signin-s2 = Central は、自分だけが持つ鍵で目隠しされた値を計算し、鍵交換の自分の半分を加えます。その際、パスワードが Central に伝わることはありません。
sec-signin-c3 = Argon2id · 64 MiB
sec-signin-t3 = 重い処理は端末が行います。
sec-signin-s3 = 目隠しを外し、結果を Argon2id（メモリ 64 MiB、3回）で引き延ばします。Central がより軽い設定を求めることはできません。パスワードが違っていれば、ほかに何かを送る前に、ここで分かります。
sec-signin-c4 = 証明
sec-signin-t4 = 端末が、正しい鍵を持つことを証明します。
sec-signin-s4 = パスワードが正しければ、端末は Central と同じセッション鍵に行き着き、そのことの短い証明（64バイト）を送ります。
sec-signin-c5 = セッション＋ラッパー
sec-signin-t5 = Central がセッションを開きます。
sec-signin-s5 = 返ってくるのは、セッション（ブラウザーでは、ページ自身は読めないクッキー）と、ラッパーに入ったマスターキーです。ラッパーは、正しいパスワードを使った、あなたの端末でしか開きません。
sec-signin-register = <b>登録</b>も同じプロトコルで行います。端末が乱数のマスターキー（256ビット）を作り、パスワードで1回、回復コードで1回ラップして、ラッパーだけを送ります。Central が保存するのは OPAQUE レコードとラッパーで、鍵そのものは保存しません。
sec-signin-legacy = <b>一つの例外：</b>旧公式 Gateway のアカウントで、まだ Central に移っていないもの。そのようなアカウントが移行を終えるまで、Baylee クライアントでのログインの試行ごとに、入力したパスワードが、検証済みの TLS 接続で Central に送られます。Central はそれを旧ハッシュと照合し、保存も記録もしません。移行が成功すれば、そのアカウントは上に示したとおりにログインします。このウェブサイトはそのようなアカウントをログインさせず、クライアントを案内します。この例外は、移行に設定された期限で終わります。
sec-signin-limit = <b>残ること：</b>Central のデータベースをその鍵ごと手に入れた者は、パスワードを一つずつ試せます。試行のたびに Argon2id の実行が必要です。そうして見つかったパスワードは、アカウントの暗号化されたデータと、保存されたチャットを開きます。長く、ほかで使っていないパスワードは、それでも意味があります。
sec-signin-guess = <b>ログインを使った推測</b>は、遅くなるだけで、止まりません。Central は、正しいパスワードで完了しなかった試行を数えます。1つのアカウントに対しては、そのアカウントがログインしたことのないアドレスから1日最大100回、1つのアドレスからは、すべてのアカウントを合わせて1日最大100回です。それを超えると、Central は試行が多すぎたと応答し、その1時間または1日が終わるまで続きます。ありふれたパスワードは、それでも見つかります。また、あなたのログイン名を知っている人は、あなたのアカウントの試行回数を使い切らせることができます。その場合、その日が終わるまで、ログインできるのは、あなたのアカウントが過去30日間にログインしたことのあるアドレスからだけです（Central はこれらのアドレスを再起動するまでしか覚えていません）。

sec-memory-eyebrow = メモリ内
sec-memory-title = ログイン後もメモリに残るもの
sec-memory-lead = パスワードが必要なのは一度だけ、マスターキーを開くときです。図はブラウザーでの経過を追います。Baylee クライアントでの違いは下に書きます。
sec-memory-c1 = パスワード
sec-memory-t1 = ページがパスワードを Worker に渡します。
sec-memory-s1 = フォームはそれを専用のバックグラウンドスレッド（Worker）に渡し、入力欄を空にして、自分の持つコピーを解放します。
sec-memory-c2 = OPAQUE＋Argon2id
sec-memory-t2 = Worker が計算し、その後で上書きします。
sec-memory-s2 = Worker の Rust コードは、パスワードを OPAQUE の手順が終わるまでだけ保持し、その後で上書きします。途中で導き出して自分で保持した鍵と、Argon2id の 64 MiB の作業メモリも同様です。
sec-memory-c3 = マスターキー：確認して消去
sec-memory-t3 = マスターキーは開かれ、使われ、消去されます。
sec-memory-s3 = Worker はログインの確認のためにそれを開き、アカウントにチャットがあればそこからチャット鍵を導き、再び上書きします。
sec-memory-c4 = チャット鍵、封印
sec-memory-t4 = 残るのはチャット鍵だけです。
sec-memory-s4 = パスワードなしでも再読み込み後にチャットが開くよう、Worker はそれらを AES-256-GCM で暗号化して、ブラウザー（IndexedDB）に保存します。そのための鍵は抽出不可です。スクリプトは使えますが、読み出せません。チャットのページを開いている間は、専用の Worker が、封印を解いたチャット鍵をメモリに保持します。ログアウトするとこのレコードを削除します。セッションが自然に終了した場合は、次に読み込まれるこのサイトのページが削除します。
sec-memory-c5 = Worker 終了
sec-memory-t5 = ログインの Worker は終了します。
sec-memory-s5 = ログインが終わると、ログインページが Worker を終了させ、ブラウザーはその Worker のメモリを破棄します。
sec-memory-limit = <b>ブラウザーが約束しないこと：</b>入力されたままのパスワード、Worker への受け渡し、JavaScript の文字列は、ブラウザーが管理します。Baylee はそれらを解放しますが、ページ自身のコピーは上書きしませんし、ブラウザーのコピーは上書きできません。そのメモリが実際にいつ再利用されるかは、ブラウザーが決めます。また、ログイン中にこのサイトでスクリプトを実行できる人や、ブラウザーのプロフィールを読める人は、封印されたチャット鍵に手が届きます。
sec-memory-client = <b>Baylee クライアントでは</b>、入力したパスワードは、解放時に上書きされるメモリに置かれ、ログイン直後に破棄されます。保存されることはありません。マスターキーは、ログイン中はクライアントのメモリに残ります。クライアントがそれで暗号化されたデータを開くためです。

sec-chat-eyebrow = チャット
sec-chat-title = メッセージは端末で暗号化されます
sec-chat-lead = メッセージごとに専用の鍵が作られます。Central が保存して中継するのは暗号化されたものだけで、読むには、それをラップした宛先アカウントのチャット鍵が必要です。
sec-chat-c1 = テキスト＋使い捨ての鍵
sec-chat-t1 = メッセージごとに新しい鍵。
sec-chat-s1 = 端末が乱数のメッセージ鍵を作り、それでテキストを暗号化します（XChaCha20-Poly1305）。
sec-chat-c2 = 鍵ごとにラップ、署名
sec-chat-t2 = 受信者のチャット鍵ごとにラップし、あなたの鍵で署名します。
sec-chat-s2 = 受信者のアカウントが登録したチャット鍵ごとに、メッセージ鍵を別々にラップし（X25519）、ヘッダーと暗号文にあなた自身のチャット鍵で署名します（Ed25519）。現在、アカウントのチャット鍵は一つで、マスターキーから導かれ、ログインしているどのブラウザー・クライアントでも同じです。
sec-chat-c3 = 暗号文＋ラップ
sec-chat-t3 = Central が受け取るのは暗号文だけです。
sec-chat-s3 = Central は署名と宛先の一覧を確認し、暗号文をラップとともに保存します。メッセージ鍵も、アカウントの鍵も持っていません。
sec-chat-c4 = 暗号文＋ラップ1つ
sec-chat-t4 = チャット鍵ごとに専用のラップ。
sec-chat-s4 = 連絡先がメッセージを取得すると、Central は暗号文と、そのチャット鍵のために作られたラップだけを渡します。
sec-chat-c5 = 確認、開封
sec-chat-t5 = 確認、ラップ解除、読む。
sec-chat-s5 = 連絡先の端末があなたの署名を確認し、自分のチャット鍵でメッセージ鍵のラップを解いて、テキストを復号します。
sec-chat-meta = <b>それでも Central に見えるもの：</b>誰が誰と連絡を取っているか、名前とメンバーつきのグループ、表示名、公開のチャット鍵、そして各メッセージについて、誰が送ったか、どの会話か、いつか、どれくらいの大きさか。ほかのサーバーと同じく、リクエストの送信元のアドレスも見えます。
sec-chat-limit = <b>限界：</b>ダブルラチェットはなく、したがって前方秘匿性もありません。受信者のチャット鍵があとで盗まれると、その鍵宛てにラップされた過去のメッセージも開きます。チャット鍵はアカウントのマスターキーから導かれ、Central は配信後も暗号化されたメッセージを保持します。Central のデータベースをその鍵ごと手に入れてパスワードを推測した人、または回復コードを持つ人は、それらを読めます。最初の接触では、端末は Central が示す鍵を信頼します。確認するには、別の経路で安全番号を比べてください。連絡先の鍵があとで変わった場合、Baylee はあなたが確認したあとにだけ、再び送信します。グループに誰が属するかは、Central が決めます。

sec-pair-eyebrow = リンクでのログイン
sec-pair-title = パスワードをもう一度入力せずに、ブラウザーからクライアントへ
sec-pair-lead = ウェブサイトでログインした人が、ログイン済みの状態で Baylee クライアントを起動できるようにするための仕組みです。そのためにページは、マスターキーをクライアントに渡します。封印した状態で、Central が開くことも、自分の鍵をすり替えることもできない形で。
sec-pair-status = <b>現状：</b>Central と Baylee のクライアントライブラリは、今日これを実行でき、互いに対してテスト済みです。このウェブサイトからも、公開済みのクライアントからも、まだ使えません。
sec-pair-c1 = ペアリング開始
sec-pair-t1 = ページがペアリングを開始します。
sec-pair-s1 = Central は、5分間有効なハンドルを発行します。同時にページは使い捨ての秘密を作り、それを Central には見せません。
sec-pair-c2 = 秘密つきリンク
sec-pair-t2 = リンクがクライアントを開きます。
sec-pair-s2 = リンク <code>baylee://pair</code> が、ハンドルと秘密を、Central を経由せずに、ページからあなた自身の端末のクライアントへ運びます。
sec-pair-c3 = 公開鍵
sec-pair-t3 = クライアントが新しい鍵ペアで応答します。
sec-pair-s3 = クライアントは Central に、公開側の鍵と、秘密で計算した確認値（HMAC-SHA-256）を送ります。ページがその値を検証します。両方の画面に同じ短いコードが表示され、あなたがページで承認します。
sec-pair-c4 = マスターキー、封印
sec-pair-t4 = ページがクライアント向けにマスターキーを封印します。
sec-pair-s4 = HPKE（RFC 9180）で、クライアントの公開鍵宛てに、秘密も加えて封印します。Central は封印を一度だけ保存します。開くことはできず、秘密を知らないので、自分で封印を作ることもできません。
sec-pair-c5 = セッション＋封印
sec-pair-t5 = クライアントは一度だけ引き換えます。
sec-pair-s5 = セッションと封印を受け取り、秘密鍵と秘密で封印を開き、ログイン済みになります。ペアリングは、その後、引き換えが失敗した場合も含めて削除されます。
sec-pair-note = <b>この手順にパスワードは関与しません。</b>ページがマスターキーを持つのは、あなたのパスワードで開いた、まさにその瞬間だけです。この方法でペアリングされたセッションは、元のページでのパスワードによるログインより新しいものとは見なされません。ペアリングがそれを更新することはありません。

sec-limits-eyebrow = 限界
sec-limits-title = これらのどれにもできないこと
sec-limit-device = <b>侵害された端末。</b>マルウェア、ページを読むブラウザー拡張、改ざんされたクライアントは、あなたが見るもの、入力するものを見ます。サーバーまでの道筋での暗号化は、それには役立ちません。
sec-limit-site = <b>ウェブサイトのコードは、そのサーバーから来ます。</b>私たちのサーバーを乗っ取った者は、入力中のパスワードを捕らえるコードを配信できます。整合性ハッシュと厳格な Content Security Policy は外部からの改ざんに備えるもので、それには備えません。クライアントはあなたのマシン上のプログラムで、更新をインストールする前に署名を確認します。
sec-limit-central = <b>乗っ取られた Central。</b>メタデータを明かし、サービスを拒み、ログインを攻撃できます。最初の接触で偽の鍵を示したり、グループにメンバーを加えたりでき、そうすればその後に送られたメッセージを読めます。あなたのデータとチャットとして保存しているものは暗号化されています。開くにはパスワードか回復コードが必要です。
sec-limit-recovery = <b>失われた鍵。</b>パスワードも回復コードもなければ、暗号化されたデータを元に戻せる人はいません。私たちも同様です。Central にはラッパーしかありません。
sec-limit-audit = <b>独立した監査はありません。</b>ログインは OPAQUE（RFC 9807）と Argon2id（RFC 9106）に従い、ペアリングは HPKE（RFC 9180）で封印します。これらは IRTF が公開した仕様で、IETF の標準ではありません。ペアリングの手順の順序とチャットの形式は、標準の部品（Ed25519、X25519、XChaCha20-Poly1305）を組み合わせた Baylee 独自のものです。実装を確認したのは、今のところ作った人たちだけです。コードは公開されています。何か見つけたら、<contact>お知らせください</contact>。
sec-docs-title = さらに読む
sec-docs = 詳細はプロジェクトの文書にあります。<protocol>暗号プロトコル</protocol>、<chat>チャット</chat>、<accounts>アカウントとペアリング</accounts>、<threats>脅威モデル</threats>。データの法的な扱いは、<privacy>プライバシー通知</privacy>にあります。

# The page's two links from elsewhere: the FAQ and the sign-in page.
faq-security-q = Baylee はパスワードとチャットをどう扱いますか？
faq-security-a = ログインのとき、端末は OPAQUE プロトコルで、パスワードを送らずにそれを知っていることを証明します。チャットのメッセージは端末で暗号化されます。<security>セキュリティのページ</security>では、各手順を図で示し、一つの例外と限界も載せています。
sec-signin-link = ログイン時にパスワードがどうなるか：<security>セキュリティ</security>
sec-teaser-lead = 手順ごとに：何が端末から出ていくか、Baylee Central に何が見え何が見えないか、そして各フローの限界。
sec-teaser-more = 各手順を図で見る
# ==== end of the security page =================================================
# ==== registration: the proof of work, while it takes long =====================
register-solving = ブラウザーがロボットを寄せつけないパズルを解いています。多くのアカウントが作成されているときは、数分かかることがあります … { $percent } %

## An administrator makes a reset code (/account/admin)
admin-reset-title = ログインのリセット
admin-reset-lead = 依頼した人のアカウントを探し、そのリセットコードを作ります。そのコードで、本人がリセットページで新しいパスワードを設定します。
admin-find = アカウントを探す
admin-only = このページは管理者専用です。
admin-search = アカウントを検索
admin-search-hint = ログイン名または表示名の一部、または # とタグを入力します。空のままにすると、新しい順にすべてのアカウントを表示します。
admin-search-submit = 検索
admin-search-error = 検索語は、アクセント記号のない通常の文字1～64文字、または # と #1a2b のようなタグです。
admin-count = 表示中のアカウント：{ $count }
admin-none = この検索に一致するアカウントはありません。
admin-more = さらに表示
admin-open = 開く
admin-back = 戻る
admin-pill = 管理者
admin-state = 状態
admin-state-pending = 予約済み・未設定
admin-state-enrolled = 設定済み
admin-state-restored = 使用中
admin-created = 作成日時
admin-last-sign-in = 最後のログイン
admin-recovery = 回復コード
admin-yes = はい
admin-no = いいえ
admin-reset-outstanding = 未使用のリセットコードが{ $time }まで有効です。管理者が作ったものか、メールで送られたものかは、Central には分かりません。新しいコードを作ると、置き換わります。
admin-reset = ログインをリセット
admin-refused-admin = このアカウントは管理者のロールを持っており、Central は管理者のアカウントのリセットコードを作りません。先にロールを外してください。
admin-refused-own = これはご自分のアカウントです。自分のリセットコードは、誰も作れません。代わりに、セキュリティでパスワードを変更してください。
admin-refused-pending = このアカウントは予約されただけで設定されていないため、リセットするログインがありません。
admin-refused-moving = このアカウントの公式 Gateway からの移動は終わっていません。終わるまでは、ログインのリセットはここではなく、そちらで行います。
admin-refused-stale = Baylee Central は、あなたのログインが30分より前のものと見なしています。もう一度お試しください。パスワードをもう一度入力します。
admin-refused-role = あなたのアカウントは、もう管理者のロールを持っていません。
admin-refused-gone = このアカウントは、もう存在しません。
admin-switched = このブラウザーは、いま別のアカウントでログインしているため、コードは求められませんでした。ページを読み込み直してください。
admin-lost = 応答がなかったため、コードが作られたかどうか分かりません。作られていた場合、それをもう一度表示できる人はおらず、以前のコードはすでに置き換わっています。新しいコードを作ってください。それが、そのコードを置き換えます。
admin-warning-title = コードを作る前に
admin-warning-who = 依頼した人が本当にこのアカウントの持ち主であることを、このサイトの外で確認してください。たとえば、電話や、すでにその人と知り合っている連絡手段を使います。コードを持つ人は、新しいパスワードを設定できます。
admin-warning-ends = コードを作っても、何も終わりません。コードが使われるまで古いパスワードは使え続け、すべての端末はログインしたままです。
admin-warning-data = 本人の暗号化されたデータがリセット後も残るのは、回復コードがあるか、まだログインしている端末がある場合だけです。どちらもなければ、空の保管庫から始まります。
admin-warning-replaces = 新しいコードは、メールで送られたものを含め、以前のコードを置き換えます。有効期間は24時間で、1回だけ使えます。
admin-warning-takeover = リセットコードは、パスワードを忘れた場合のためのもので、乗っ取られたアカウントへの対処ではありません。侵入者のセッションと古いパスワードは、コードが使われるまで残り、使われなければそのまま残り続けます。
admin-warning-continue = 依頼した人を確認しました
admin-password-hint = コードを作るには、30分以内のログインが必要なため、パスワードをもう一度入力します。パスワードはこのブラウザーの中にとどまります。
admin-password-missing = パスワードを入力してください。
admin-make = リセットコードを作る
admin-code-title = リセットコード
admin-code-until = { $time }まで有効で、1回だけ使えます。
admin-code-lead = 依頼した人に、信頼できる連絡手段で渡してください。表示されるのは今だけです。Baylee Central もこのページも、もう一度表示することはできません。なくした場合は、新しいコードを作ってください。
admin-code-done = 完了
