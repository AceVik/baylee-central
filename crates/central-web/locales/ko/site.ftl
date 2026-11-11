# baylee.app's words in English, Project Fluent's format. English decides
# which messages exist and which arguments each takes; build.rs turns them
# into typed functions (crates/central-web/build.rs). The reader is "you";
# a button says what it does.

## The frame of every page

site-name = Baylee
skip = 본문으로 건너뛰기
# Read out after a link that leaves the site (the stylesheet carries it).
new-tab = 새 탭에서 열려요
nav-label = 주 메뉴
nav-download = 다운로드
nav-gateways = Gateways
nav-faq = 질문
nav-contact = 문의
nav-sign-in = 로그인
nav-account = 내 계정
lang-label = 언어
theme-label = 색상 모드
theme-system = 색상: 기기 설정과 같게
theme-light = 색상: 밝게
theme-dark = 색상: 어둡게

foot-label = 이 사이트 소개
foot-play = 플레이
foot-legal = 법적 고지
foot-releases = 모든 릴리스
foot-issues = 버그 신고
foot-discord = Discord 커뮤니티
foot-central-source = 이 사이트의 소스
foot-dedication = Baylee에게. 영원히 햇살 속에서.
fan-content = Baylee는 <policy>팬 콘텐츠 정책</policy>에 따라 허가된 비공식 팬 콘텐츠임. 위저즈에 의해 허가/보증되지 않음. 사용된 자료 중 일부는 위저즈오브더코스트의 자산임. ©Wizards of the Coast LLC.
foot-colophon = 글꼴: Literata와 Inter, SIL Open Font License 1.1, 이 사이트에서 제공해요. 그림과 로고는 직접 만들었어요. <notices>고지</notices>

## Titles and descriptions

title-home = Baylee – 덱을 만들고, 친구를 모으고, 그냥 플레이하세요.
desc-home = Baylee는 Linux, Windows, macOS에서 즐기는 무료 오픈 소스 Magic: The Gathering 팬 게임이에요. 카드를 아는 룰 엔진, 최대 여덟 명이 함께하는 테이블, 아무도 없을 때 상대해 줄 AI까지 갖추었어요.
title-download = Baylee 다운로드
desc-download = Windows, macOS, Linux용 Baylee 클라이언트를 내려받으세요. 최신 릴리스의 설치 파일과 압축 파일을 체크섬 및 서명과 함께 제공해요.
title-gateways = Gateways: Baylee를 온라인으로 즐기는 곳
desc-gateways = 온라인 테이블이 열리는 서버인 Baylee Gateway의 공개 목록과, 직접 운영하는 방법을 안내해요.
title-faq = 자주 묻는 질문
desc-faq = Baylee가 무엇인지, 비용은 얼마인지, 온라인 플레이와 계정은 어떻게 동작하는지, 내 데이터는 어떻게 되는지 알려 드려요.
title-contact = 문의
desc-contact = Baylee를 만든 사람에게 글을 보내거나 버그를 신고하세요.
title-impressum = 법적 고지
desc-impressum = baylee.app을 운영하는 사람 (독일 디지털서비스법 § 5 DDG).
title-privacy = 개인정보 처리방침
desc-privacy = baylee.app과 Baylee Central이 어떤 정보를 왜, 어떤 법적 근거로, 얼마나 오래 처리하는지 안내해요.
title-terms = 이용 약관
desc-terms = baylee.app의 계정 서비스인 Baylee Central의 이용 약관이에요.
title-fan-content = 비공식 팬 콘텐츠
desc-fan-content = Baylee는 Wizards of the Coast의 팬 콘텐츠 정책에 따른 팬 프로젝트예요. 그 의미와 의미하지 않는 것을 설명해요.
title-not-found = 페이지를 찾을 수 없음
title-gone = 삭제된 페이지
crumbs-label = 현재 위치
breadcrumb-home = 홈

## Home

hero-title = <nb>덱을 만들어요.</nb> <nb>친구를 모아요.</nb> <glow>바로 플레이해요.</glow>
hero-lead = 폭넓은 기능의 덱 빌더, 듀얼·난전·팀전으로 최대 여덟 명이 즐기는 대전, 그리고 모든 단계를 확인하는 룰 엔진이 있어서, 아무도 규칙을 찾아볼 필요 없이 모든 턴이 공정하게 진행돼요.
hero-offline = 솔로 대전과 덱 빌더는 계정 없이 오프라인에서 쓸 수 있어요. 온라인 테이블은 Gateway에서 열려요. <host>직접 운영할 수도 있어요</host>.
hero-account = 온라인으로 플레이하나요? <register>계정을 만드세요</register>. 오픈 베타 동안 누구나 무료로 만들 수 있어요. 이미 계정이 있다면 <signin>로그인하세요</signin>.
hero-memorial = Baylee를 기억해요: 사진 앨범 열기

status-label = 프로젝트 현황

cards-eyebrow = 카드
cards-title = Baylee가 아는 카드의 수
cards-lead = Baylee는 Magic의 카드를 하나씩 구현하고, 각각 인쇄된 텍스트와 대조해 확인해요. 아래는 최신 릴리스의 수치예요.
cards-implemented = 완전 구현
cards-implemented-help = 카드의 모든 규칙이 동작하고 테스트를 거쳤어요.
cards-generated = 자동 생성
cards-generated-help = 카드의 규칙 텍스트를 바탕으로 Baylee의 생성기가 구현했지만 아직 손으로 테스트하지는 않았어요. 플레이할 수 있지만, 드문 예외 상황에서는 다르게 동작할 수도 있어요.
cards-partial = 부분 구현
cards-partial-help = 카드를 플레이할 수는 있지만, 능력 하나 또는 규칙의 예외 상황 하나가 아직 빠져 있어요.
cards-total = 전체 카드
cards-total-help = Scryfall이 목록으로 올린 Magic의 모든 카드예요. 앞의 세 수치는 Baylee에서 플레이할 수 있는 카드예요.
cards-help = 무슨 뜻인가요
cards-share = 오늘 현재 { $total }장 중 { $playable }장을 플레이할 수 있어요.
cards-as-of = 릴리스 { $tag }, { $date } 집계.
cards-missing = 카드 수치를 지금은 읽을 수 없었어요.

shot-duel-alt = 하우스 AI와의 듀얼: 생물 여섯 마리가 공격하고 세 마리가 막혀요
shot-duel = 하우스 AI와의 <b>전술 듀얼</b>. 모든 배정에 명확한 전투선이 표시돼요.
shot-log-alt = 게임 로그가 열린 듀얼, 스택에 주문 세 개
shot-log = <b>게임 로그</b>는 모든 단계를 채팅처럼 기록해요. 카드 이름을 누르면 미리보기가 열려요.
shot-ring-alt = 고리 모양으로 배치된 4인 테이블
shot-ring = <b>최대 여덟 자리의 테이블</b>은 전장 둘레에 알아서 자리를 잡아요.
shot-lobby-alt = 친구의 메시지와 테이블 초대가 보이는 로비
shot-lobby = <b>로비:</b> 열린 테이블, 친구, 초대, 종단간 암호화 채팅이 있어요.
shot-room-alt = 빈 의자 세 개를 하우스 AI가 맡는 방
shot-room = <b>모두를 위한 자리:</b> 사람, 다섯 단계의 하우스 AI, 또는 언어 모델.
shot-builder-alt = 천사를 찾으려고 풀을 필터링하는 덱 빌더
shot-builder = <b>덱 빌더</b>는 입력하는 동안 전체 풀을 검색해요.

about-eyebrow = 소개
about-title = 완전한 룰 엔진과 깔끔한 3D 테이블
about-engine = 우선권, 스택, 계층, 대체 효과, 유발 능력, 상태 기반 행동, 전투까지: Rust 엔진이 전체 규칙을 적용하고 매 단계에서 정확한 합법 선택지를 계산해요. 클라이언트는 유효한 행동만 보여 줘요.
about-table = 전장, 손, 스택, 추방 영역, 묘지가 깔끔한 3D 테이블 위에 명확한 전투선과 즉시 뜨는 카드 미리보기와 함께 표시돼요. 카드 일러스트는 플레이하는 동안 Scryfall에서 불러오며, 언제든 선명한 오라클 텍스트 보기로 바꿀 수 있어요.

gw-eyebrow = 온라인
gw-title = Gateways

open-central = GitHub의 Baylee Central

## Download

dl-prerelease = Beta
dl-for = { $system }용 다운로드
dl-size = { $size } MB
dl-checksum = SHA-256
dl-signature = 서명
dl-notes = 릴리스 노트
dl-unknown = 이 브라우저로는 시스템을 알 수 없었어요. 아래에서 직접 골라 주세요.
dl-mobile = 휴대폰과 태블릿용 버전은 아직 없어요. Baylee는 컴퓨터에서 실행돼요.
dl-mac-unsure = 이 브라우저는 Mac이 Apple 칩인지 Intel 칩인지 알려 주지 않아요. 위의 파일은 Apple 칩(M1 이후)용이고, Intel Mac용 파일은 아래에 있어요.
dl-windows-unsure = ARM 프로세서가 들어간 Windows PC(예: Snapdragon 노트북)라면 아래의 ARM 파일을 받으세요.
dl-linux-unsure = 브라우저가 프로세서를 알려 주지 않아서 x86-64로 가정했어요. ARM 파일은 아래에 있어요.
dl-missing = 이 릴리스에는 사용 중인 시스템용 파일이 없어요. 이전 릴리스는 <releases>릴리스 페이지</releases>에 있어요.
dl-none = GitHub가 지금은 응답하지 않았어요. 모든 릴리스는 <releases>릴리스 페이지</releases>에 있어요.
dl-yours = 내 시스템
dl-more-files = 파일 더 보기 ({ $count })
dl-installer = 설치 프로그램
dl-deb = Debian 패키지
dl-archive = 압축 파일
dl-fallback = 구형 프로세서용 압축 파일
dl-fallback-help = AVX2가 없는 x86-64 프로세서(대략 2013년 이전)용이에요. Baylee의 기본 빌드에는 AVX2가 필요하며, 없으면 시작되지 않아요.
plat-windows-x64 = Windows
plat-windows-x64-sub = 대부분의 PC: 64‑bit Intel 또는 AMD
plat-windows-arm64 = Windows on ARM
plat-windows-arm64-sub = Snapdragon 노트북 등
plat-mac-arm64 = macOS
plat-mac-arm64-sub = Apple 칩: M1 이후
plat-mac-x64 = Intel용 macOS
plat-mac-x64-sub = Intel 프로세서를 쓰는 Mac
plat-linux-x64 = Linux
plat-linux-x64-sub = x86-64
plat-linux-arm64 = ARM용 Linux
plat-linux-arm64-sub = ARM64 / aarch64
plat-none = 이 릴리스에는 이 시스템용 파일이 없어요.
install-title = 처음 시작할 때와 필요 사양
install-first-title = 처음 시작할 때
install-first = 코드 서명에는 비용이 들기 때문에 이 빌드는 코드 서명이 되어 있지 않아요. 그래서 Windows와 macOS에서는 처음 실행할 때 경고가 떠요. Windows에서는 <i>추가 정보</i>를 누른 다음 <i>실행</i>을 선택하세요. macOS에서는 <i>시스템 설정 → 개인정보 보호 및 보안</i>을 열고 <i>그래도 열기</i>를 선택하세요.
install-check-title = 내려받은 파일 확인
install-check = 각 파일 옆에는 체크섬이 담긴 <code>.sha256</code> 파일이 있고, 압축 파일에는 클라이언트가 스스로 업데이트하기 전에 확인하는 Ed25519 서명인 <code>.sig</code> 파일도 있어요. Linux에서는 <code>sha256sum -c</code>로, macOS에서는 <code>shasum -a 256</code>으로, Windows에서는 <code>certutil -hashfile</code>로 파일을 확인할 수 있어요.
install-needs-title = 필요 사양
install-windows = <b>Windows</b> 10 또는 11, x64 또는 ARM64.
install-mac = <b>macOS</b> 11 이상, Apple 칩 또는 Intel.
install-linux = <b>Linux</b>: glibc 2.35 이상(Ubuntu 22.04 이후)과 Vulkan 드라이버가 필요해요. <code>.deb</code> 패키지가 나머지 필요한 것을 모두 함께 설치해요.
install-cpu = <b>프로세서:</b> x86-64 빌드에는 AVX2가 필요해요. 더 오래된 프로세서용으로는 별도의 압축 파일이 있어요.

## Gateways

gws-title = Gateways
gws-official = 공식
gws-online = 온라인
gws-offline = 오프라인
gws-version = 버전
gws-now = 지금
gws-website = 웹사이트
gws-address = 주소
gws-copy = 주소 복사
gws-copied = 주소를 복사했어요.
gws-copied-short = 복사됨
gws-copy-failed = 복사하지 못했어요.
gws-failed = 목록을 지금은 읽을 수 없었어요. 잠시 후 다시 시도해 주세요.

## Questions

faq-eyebrow = 질문
faq-title = 자주 묻는 질문
faq-what-q = Baylee가 무엇인가요?
faq-what-a = 컴퓨터에서 즐기는 Magic: The Gathering 무료 팬 게임이에요. 덱 빌더, 모든 단계를 확인하는 룰 엔진, 최대 여덟 명이 앉는 테이블, 컴퓨터 상대가 있어요. Wizards of the Coast가 만들거나 허가하거나 보증한 것이 아니에요.
faq-cost-q = 비용은 얼마인가요?
faq-cost-a = 무료이고, 앞으로도 바뀌지 않아요. 유료 장벽도, 유료 기능도, 광고도 없어요.
faq-account-q = 계정이 필요한가요?
faq-account-a = 혼자 플레이할 때는 필요 없어요. 덱 빌더와 하우스 AI와의 대전은 오프라인에서 동작해요. 온라인 테이블에는 Baylee 계정이 필요하며, 오픈 베타 동안에는 누구나 이 사이트에서 무료로 만들 수 있어요.
faq-data-q = 내 데이터는 어떻게 되나요?
faq-data-a = 가능한 한 최소한만 처리해요. 자세한 내용은 <privacy>개인정보 처리방침</privacy>에 있어요. 비밀번호는 브라우저를 떠나지 않으며, 개인 데이터와 채팅 메시지는 우리에게 도착하기 전에 기기에서 암호화돼요.
faq-password-q = 비밀번호를 잊어버렸어요. 어떻게 하나요?
faq-password-a = 관리자가 알려 주는 재설정 코드로 새 비밀번호를 설정해요. 이 코드는 아직 이 웹사이트에서 사용할 수 없고, 추후 지원할 예정이에요. 암호화된 데이터는 계정을 만들 때 받은 복구 코드가 있거나 Baylee 클라이언트에 아직 로그인되어 있을 때만 남아 있어요. 그렇지 않으면 사라져요.
faq-gateway-q = Gateway가 무엇인가요?
faq-gateway-a = 온라인 테이블이 열리는 서버예요. 누구나 운영할 수 있으며, <gateways>목록</gateways>에는 Central이 아는 Gateway가 올라와 있어요.
faq-systems-q = 어떤 시스템에서 실행되나요?
faq-systems-a = x86-64와 ARM64의 Windows, macOS, Linux에서 실행돼요. 휴대폰이나 태블릿용 버전은 아직 없어요.
faq-open-q = 오픈 소스인가요?
faq-open-a = 게임은 GNU Affero 일반 공중 사용 허가서에 따른 오픈 소스예요. Baylee Central의 코드도 누구나 데이터 처리 방식을 확인할 수 있도록 공개되어 있어요.
faq-bug-q = 버그를 발견했어요. 어디에 신고하나요?
faq-bug-a = 게임에서 <kbd>F8</kbd>을 누르면 필요한 정보를 모두 담아 신고를 보낼 수 있어요. 그 밖에는 GitHub 이슈 트래커, Discord, <contact>문의 양식</contact>으로 알려 주세요.

## Contact

contact-eyebrow = 문의
contact-title = 글 보내기
contact-name = 이름
contact-name-hint = 선택 사항이에요.
contact-email = 이메일 주소
contact-email-hint = 선택 사항이며, 답장을 드리는 데만 써요.
contact-message = 메시지
contact-count = 5,000자 중 { $n }자
contact-send = 메시지 보내기
contact-sending = 보내는 중 …
contact-sent-title = 감사합니다
contact-sent = 메시지가 도착했어요. 주소를 남기셨다면 그 주소로 답장드릴게요.
contact-error-empty = 먼저 메시지를 적어 주세요.
contact-error-email = 이메일 주소처럼 보이지 않아요.
contact-error-busy = 지금 이 네트워크에서 보낸 메시지가 너무 많아요. 한 시간 뒤에 다시 시도해 주세요.
contact-error-refused = 메시지가 접수되지 않았어요. 길이를 확인하고 다시 시도해 주세요.
contact-error-network = 메시지를 보내지 못했어요. 연결을 확인하고 다시 시도해 주세요.
contact-privacy = 보내 주신 내용은 답장을 드리기 위해 보관하며, 처리를 마친 뒤 90일 후에, 늦어도 1년 후에는 삭제해요. 자세한 내용은 <privacy>개인정보 처리방침</privacy>에 있어요.
contact-noscript = 이 양식에는 JavaScript가 필요해요. Discord나 GitHub로도 연락할 수 있어요.

## Legal notice

impressum-title = 법적 고지
impressum-lead = 독일 디지털서비스법(DDG) § 5에 따라 필요한 정보예요. Baylee는 개인이 운영하는 비영리 프로젝트예요.
impressum-show = 상세 정보 보기
impressum-why = 상세 정보는 브라우저가 몇 초 안에 푸는 짧은 퍼즐 뒤에 있어요. 주소를 수집하는 로봇을 막기 위해서예요. 그 밖에 요구하는 것은 없어요.
impressum-solving = 퍼즐을 푸는 중 … { $percent } %
impressum-name = 이름
impressum-address = 주소
impressum-email = 이메일
impressum-phone = 전화
impressum-write = 이메일 쓰기
impressum-call = 전화하기
impressum-failed = 상세 정보를 불러오지 못했어요. 다시 시도하거나 <contact>문의 양식</contact>으로 글을 보내 주세요.
impressum-noscript = 상세 정보를 표시하려면 JavaScript가 필요해요. 사용할 수 없다면 Discord나 GitHub로 연락해 주세요.
impressum-responsible = 콘텐츠 책임자: 위에 적힌 사람.
impressum-dispute = 우리는 소비자 중재 기구의 분쟁 해결 절차에 참여할 의무도 의사도 없어요.

## Fan content

fc-title = 비공식 팬 콘텐츠
fc-eyebrow = 팬 콘텐츠
fc-lead = Baylee는 팬 프로젝트예요. Wizards of the Coast의 팬 콘텐츠 정책에 따라 허가되었으며, 그 정책이 정한 규칙을 따라요.
fc-notice-title = 모든 페이지에 있는 고지
fc-means-title = 그 의미
fc-means = Baylee는 Wizards of the Coast가 만들거나 허가하거나 보증한 것이 아니에요. Magic: The Gathering과 그 카드 이름, 세트 이름, 카드 이미지, 마나 기호는 Wizards of the Coast LLC의 자산이에요. Baylee는 누구에게나 무료예요. 유료 장벽도, 유료 기능도, 이메일 등록도 없어요.
fc-images = 게임은 Scryfall의 카드 이미지를 변형 없이 아티스트 표기와 함께 보여 줘요. 이 사이트에는 카드 이미지가 없어요. 카드 데이터와 이미지는 Scryfall이 제공하며, Baylee는 Scryfall과 제휴 관계가 아니에요.
fc-policy = Wizards of the Coast의 <policy>팬 콘텐츠 정책</policy>

## Privacy notice

privacy-title = 개인정보 처리방침
privacy-lead = 이 사이트와 Baylee Central이 어떤 정보를 왜, 어떤 법적 근거로, 얼마나 오래 처리하는지 안내해요.
privacy-updated = 2026년 11월 11일 기준
privacy-controller-title = 책임자
privacy-controller = GDPR(EU 일반 개인정보 보호법)에 따른 처리 책임자는 <impressum>법적 고지</impressum>에 이름, 우편 주소, 이메일 주소가 적힌 개인이에요. 개인정보 관련 문의는 문의 양식으로도 보낼 수 있어요. 개인정보 보호책임자는 없으며, 두어야 할 의무도 없어요.
privacy-short-title = 요약
privacy-short = 추적도, 광고도, 분석도 없고, 로그인하지 않으면 쿠키도 없으며, 다른 서버에서 불러오는 것도 없어요. 글꼴, 그림, 스크립트가 모두 baylee.app에서 제공돼요. 비밀번호는 우리에게 도달하지 않으며, 채팅 메시지와 개인 데이터는 전송되기 전에 브라우저에서 암호화돼요.
privacy-hosting-title = 사이트 방문
privacy-hosting = 이 사이트는 독일 netcup GmbH에서 임대한 서버에서 운영돼요. 페이지를 열면 브라우저가 모든 요청에 담기는 정보를 보내요. 곧 IP 주소, 요청한 주소, 시각, 브라우저 언어, 브라우저 식별 정보(user agent)예요. 서버는 응답하기 위해 이 정보가 필요해요. 서버는 언어 정보로 언어 버전을 고르고, 다운로드 페이지는 user agent로 알맞은 파일을 제안해요. 웹 서버는 접속 로그를 남기지 않으므로 이 요청들은 저장되지 않아요. 서버 자체의 운영 메시지(시스템 저널)에는 드문 오류 상황에서 IP 주소가 들어갈 수 있으며, 늦어도 14일 후에 삭제돼요. 법적 근거: GDPR 제6조 제1항 (f)호. 우리의 정당한 이익은 사이트를 안전하고 안정적으로 제공하는 것이에요.
privacy-views-title = Counting page views
privacy-views = The website counts how often its pages are opened: one single number for all pages and all visitors together, shown on the start page. A page opened in a browser adds one to it. Nothing about you is stored for this: no IP address, no cookie, no identifier, not even which page it was.
privacy-device-title = 브라우저에 저장되는 것
privacy-device = 이 사이트는 로그인하기 전에는 쿠키를 설정하지 않아요. 브라우저가 기기에 저장하는 것은 다음과 같고, JavaScript를 쓸 때만 해당돼요. 색상을 선택했다면 그 선택(<code>baylee-site-theme</code>), 로그인한 동안에는 세션이 끝나는 시각(<code>baylee-central-session</code>), 채팅 키, 연락처의 신뢰한 키(채팅 항목 참고), 알림을 켰다면 이 기기의 임의 번호(<code>baylee-push-installation-v1</code>, <code>baylee-push</code>), 그리고 로그인했거나 사이트를 앱으로 설치했거나 알림을 켠 경우에 한해 사이트의 공개 파일(페이지 틀, 글꼴, 스크립트)의 사본이에요. 이 사본이 있으면 연결 없이도 시작되고 알림을 받을 수 있어요. 세 가지가 모두 더는 해당하지 않으면 다음에 사이트를 열 때 사본이 제거돼요. 아래 항목에서 달리 말하지 않는 한, 이 중 어떤 것도 우리에게 전송되지 않아요. 채팅과 세션 항목은 로그아웃하면 삭제되며, 나머지는 브라우저에서 사이트 데이터를 지우면 제거할 수 있어요. 법적 근거: TDDDG § 25 제2항 제2호(사용하는 기능에 꼭 필요한 경우). 알림의 임의 번호에 대해서는 동의(TDDDG § 25 제1항).
privacy-github-title = GitHub
privacy-github = 다운로드 페이지와 시작 페이지는 최신 릴리스, 테스트 상태, 카드 수치를 보여 줘요. 이 정보는 브라우저가 아니라 우리 서버가 GitHub에 요청하므로, GitHub는 이를 통해 여러분에 대해 아무것도 알 수 없어요. 다운로드나 GitHub 링크를 누를 때만 브라우저가 GitHub(GitHub, Inc., 미국)에 연결해 IP 주소와 모든 요청에 담기는 정보를 보내며, 그 뒤에는 GitHub의 개인정보 처리방침이 적용돼요.
privacy-directory-title = Gateway 목록
privacy-directory = 우리 서버가 Baylee Central에서 Gateway 목록을 읽어 와 보여 줘요. 여러분에 관한 정보는 관여하지 않아요. Gateway는 지금 플레이어, 열린 테이블, 게임이 몇 개인지 Central에 알릴 수 있고, Central은 계정 수를 세요. 사이트는 이 숫자들을 보여 줘요. 이 숫자들은 개수일 뿐 이름이 아니며, Gateway의 숫자는 그 운영자 본인의 진술이에요.
privacy-account-title = 내 계정
privacy-account = 계정을 위해 Baylee Central은 다음을 저장해요. 계정 ID, 로그인 이름, 태그가 붙은 표시 이름, 계정을 만든 시기와 방법, OPAQUE 비밀번호 기록(우리 서버가 비밀번호를 알지 못한 채 여러분이 비밀번호를 증명할 수 있게 해 줌), 비밀번호와 복구 코드로 각각 한 번씩 암호화된 마스터 키, 암호화된 계정 문서와 암호화된 객체(우리는 읽을 수 없고 크기, 리비전, 시각만 볼 수 있음), 동의한 약관의 버전과 시기, 마지막 로그인 시각, 세션, 관리자 역할(있는 경우), 등록한 Gateway와 그 토큰. 다른 플레이어에게는 태그가 붙은 표시 이름이 보이고, 로그인 이름은 절대 보이지 않아요. 관리자가 재설정 코드를 주면, 코드를 사용하거나 만료될 때까지 그 다이제스트만 저장해요. 관리자가 계정이나 Gateway에 한 작업은 누가, 무엇을, 어느 계정이나 Gateway에, 언제 했는지를 3년 동안 기록으로 남겨요. 법적 근거: GDPR 제6조 제1항 (b)호(요청하신 계정). 마지막 로그인 시각과 기록에 대해서는 GDPR 제6조 제1항 (f)호(안전하고 추적 가능한 운영에 대한 우리의 정당한 이익). 로그인 이름과 비밀번호가 없으면 계정이 없으며, 이메일 주소는 필요 없어요. 계정은 만 16세부터 만들 수 있어요. 계정을 삭제할 때까지 보관하며, 삭제하면 운영 중인 시스템에서는 즉시, 백업에서는 늦어도 12개월 안에 제거돼요. 세션은 사용하지 않은 지 30분 후에 끝나고, 늦어도 로그인 후 12시간이 지나면 끝나요.
privacy-cookie-title = 세션 쿠키
privacy-cookie = 로그인하면 쿠키 하나, <code>__Host-central</code>이 설정돼요. 세션을 식별하는 임의 번호예요. baylee.app에만 전송되고, 스크립트가 읽을 수 없으며, 세션과 함께 늦어도 12시간 후에 끝나요. 로그인 상태를 유지하는 데 꼭 필요하므로 동의가 필요하지 않아요. 법적 근거: TDDDG § 25 제2항 제2호 및 GDPR 제6조 제1항 (b)호.
privacy-chat-title = 채팅과 연락처
privacy-chat = 메시지는 각 수신자의 기기에 맞게 여러분의 기기에서 암호화돼요. Baylee Central은 암호화된 형태로만 저장하며 읽을 수 없어요. Central이 볼 수 있고 저장하는 것은 연락처, 연락처 요청과 차단, 이름·구성원·초대가 있는 그룹, 누가 누구에게 언제 얼마 크기의 메시지를 보냈는지, 기기의 공개 키예요. 연락처와 그룹 구성원에게는 태그가 붙은 표시 이름과 여러분의 메시지가 보여요. 브라우저는 로그인해 있는 동안 이 브라우저만 가진 키로 봉인한 채팅 키(저장소 <code>baylee-chat</code>)와 신뢰한 연락처의 키(암호화됨, <code>baylee-chat-trust</code>)를 보관해요. 메시지와 전달 기록은 계정을 삭제할 때까지 보관하며, 아직 더 짧은 자동 보관 기간은 없어요. 보낸 사람마다, 기기마다 최대 5,000개의 메시지를 보관하고, 그 이후의 새 메시지는 거부돼요. 법적 근거: GDPR 제6조 제1항 (b)호.
privacy-push-title = 알림
privacy-push = 브라우저에서 알림을 켜면 브라우저가 푸시 서비스에 등록되고, Central은 이 브라우저에 대한 그 서비스의 주소, 브라우저의 공개 키와 인증 비밀값, 언어, 이 기기의 임의 번호, 선택한 범주를 저장해요. 푸시 서비스는 브라우저 제조사의 서비스로, Google(Firebase Cloud Messaging), Mozilla 또는 Apple이며 모두 미국에 있어요. 알림에는 암호화된 상태로 종류(예: "새 메시지")와 언어만 담기며, 이름이나 본문은 절대 담기지 않아요. 푸시 서비스는 주소, 시각, 크기 같은 연결 데이터를 볼 수 있어요. 이 정보는 이 기기에서 알림을 끄거나 로그아웃하거나 계정을 삭제하면 삭제돼요. 90일 동안 갱신되지 않은 주소는 더는 쓰이지 않고, 대기 중인 알림은 하루 후에 만료돼요. 법적 근거: GDPR 제6조 제1항 (a)호 및 TDDDG § 25 제1항(동의). 계정 설정에서 언제든 동의를 철회할 수 있으며, 그때까지 이루어진 처리는 적법하게 남아요.
privacy-counters-title = 남용 방지
privacy-counters = 비밀번호 추측과 과부하를 막기 위해 Central은 특정 요청(가입, 로그인 시도, 비밀번호 재설정, 디렉터리 읽기)을 IP 주소별로 메모리에서만 세요. 가입과 올바른 비밀번호로 끝나지 않은 로그인 시도는 최대 하루 동안, 나머지는 길어야 한 시간 동안 세요. 올바른 비밀번호로 로그인한 뒤에는 Central이 여러분의 계정이 그 주소(IPv4 주소 또는 IPv6 주소의 앞 절반, 계정당 최대 여덟 개)에서 로그인했다는 사실을 마찬가지로 메모리에서만 30일 동안 기억해요. 다른 사람이 여러분의 계정에 시도하더라도 여러분이 그 주소에서 차단되지 않게 하기 위해서예요. 같은 주소를 쓰면서 여러분의 로그인 이름을 아는 사람은 Central의 응답으로 여러분의 계정이 그 주소에서 로그인했다는 것을 알 수 있어요. 새 비밀번호를 설정하면 설정한 주소를 뺀 이 주소들을 잊고, 계정을 삭제하면 모두 잊으며, 서비스를 다시 시작해도 마찬가지예요. 가입 전에 브라우저는 메모리와 계산 시간이 드는 퍼즐도 풀어요. 퍼즐은 임의의 숫자로 이루어져 있고 여러분에 관해 아무것도 담지 않으며, 늦어도 20분 뒤에는 잊혀요. Central은 브라우저의 답을 확인하려고 그 일부를 다시 계산해요. 얼마나 어려운지는 몇 개의 계정이 만들어지고 있는지, 그리고 요청이 프로그램의 것처럼 보이는지에 따라 달라요. 모든 브라우저가 보내는 헤더가 있는지, 브라우저가 프로그램이 자신을 조종하고 있다고 알리는지, 그리고 지난 한 시간 동안 여러분의 주소와 그 주변 네트워크 블록(IPv4 주소의 처음 세 숫자, IPv6 주소의 처음 48비트)이 퍼즐을 몇 개 요청했는지가 기준이에요. 이 가운데 Central이 보관하는 것은 메모리 속의 횟수뿐이며, 길어야 한 시간이에요. 계정별로 Central은 연락 요청과 다른 플레이어 검색을 세요. 아무도 다른 사람에게 요청을 쏟아붓지 못하게 하기 위해서예요. 법적 근거: GDPR 제6조 제1항 (f)호. 우리의 정당한 이익은 계정과 서비스를 악용으로부터 보호하는 것이에요.
privacy-contact-title = 문의 양식
privacy-contact = 이 양식은 메시지, 페이지의 언어, 시각을 같은 서버에 있는 Baylee의 피드백 서비스로 보내며, 이름과 이메일 주소는 입력했을 때만 보내요. 둘 다 선택 사항이에요. 이 서비스는 메시지를 처리했는지도 기록해요. IP 주소는 저장하지 않으며, 메시지 폭주를 막기 위해 한 시간 동안 메모리에서만 세요. 법적 고지에 이름이 적힌 사람만 메시지를 읽으며, 누구에게도 전달하지 않아요. 메시지는 처리한 지 90일 후에, 늦어도 1년 후에, 또는 요청하시면 더 일찍 삭제돼요. 법적 근거: GDPR 제6조 제1항 (f)호(메시지에 답하는 것에 대한 우리의 정당한 이익), 메시지가 계정에 관한 것이면 (b)호.
privacy-impressum-title = 법적 고지의 퍼즐
privacy-impressum = 법적 고지는 브라우저가 퍼즐을 푼 뒤에만 표시돼요. 주소를 수집하는 프로그램으로부터 주소를 보호하기 위해서예요. 퍼즐은 임의의 숫자로 이루어져 있고 여러분에 관해 아무것도 담지 않아요. 이를 위해 피드백 서비스는 다음을 메모리에만 보관하며, 디스크에도 로그에도 남기지 않아요. 여러분의 IP 주소가 퍼즐을 몇 번 요청했는지(한 시간 동안, IPv6 주소는 앞 절반으로), 사람에게는 보이지 않게 숨겨진 링크를 따라갔는지(24시간 동안), 어떤 퍼즐이 풀렸는지(5분 동안, 주소 없이). 브라우저에 관한 세 가지 예/아니요 답은 퍼즐의 난이도를 정하며 보관되지 않아요. 프로그램이 브라우저를 조종하는지, 브라우저가 언어를 하나도 알리지 않는지, 그래픽 카드 없이 그리는지예요. 거부되는 사람은 없어요. 프로그램처럼 보이는 브라우저는 더 긴 퍼즐을 받을 뿐이에요. 법적 근거: GDPR(EU 일반 개인정보 보호법) 제6조 제1항 (f)호(개인의 주소를 보호하려는 우리의 정당한 이익).
privacy-recipients-title = 수신자와 국가
privacy-recipients = 우리는 데이터를 판매하지 않으며 광고를 위해 넘기지도 않아요. 수신자는 다음과 같아요. netcup GmbH(독일의 호스팅, GDPR 제28조에 따른 수탁자), Baylee Central의 관리자(관리를 위해 로그인 이름, 태그가 붙은 표시 이름, 계정의 출처·생성·마지막 로그인, Gateway를 볼 수 있으나 암호화된 데이터는 절대 볼 수 없음), 계정으로 로그인하는 Gateway(계정 ID, 표시 이름, 태그를 받으며 비밀번호는 받지 않음), 연락처와 그룹 구성원(표시 이름, 태그, 메시지), 법이 요구하는 경우에 한한 당국. 유럽경제지역 밖의 국가: 알림을 켰을 때만 브라우저의 푸시 서비스(Google, Mozilla 또는 Apple, 미국)가 알림을 전달해요. 다른 사람이 운영하는 Gateway는 어느 나라에 있을 수도 있으며, 그곳에 로그인할지는 여러분이 결정해요. 그 밖에는 유럽경제지역 밖의 국가로 데이터를 전송하지 않아요.
privacy-rights-title = 내 권리
privacy-rights = 여러분에게는 열람권(GDPR 제15조), 정정권(제16조), 삭제권(제17조), 처리 제한권(제18조), 데이터 이동권(제20조)이 있어요. 제6조 제1항 (f)호에 따른 우리의 정당한 이익에 근거한 처리에 대해서는 특별한 상황에 근거한 이유로 언제든 반대할 수 있어요(제21조). 동의는 언제든 장래를 향해 철회할 수 있어요(제7조 제3항). 계정의 데이터는 계정 설정에서 내보낼 수 있고 계정도 삭제할 수 있어요. 그 밖의 모든 일에는 문의 양식이나 법적 고지의 주소로 보내는 메시지로 충분해요. 개인정보 감독 기관에 민원을 제기할 수 있어요(제77조). 예를 들어 사는 곳의 기관이나 노르트라인베스트팔렌주 정보보호·정보자유 담당관이에요. 어떤 법이나 계약도 여러분에게 데이터를 제공하라고 의무를 지우지 않아요. 다만 로그인 이름과 비밀번호가 없으면 계정이 없어요. 자동화된 의사 결정이나 프로파일링은 없어요(제22조).
privacy-objection-title = 반대할 권리
privacy-objection = GDPR 제6조 제1항 (f)호에 근거해 데이터를 처리하는 경우(사이트 제공, 남용 방지, 문의 양식, 법적 고지의 퍼즐, 마지막 로그인 시각, 관리 기록, 버그 신고, 사용 횟수 집계), 특별한 상황에 근거한 이유로 언제든 반대할 수 있어요(GDPR 제21조). 그러면 우리는 여러분의 이익보다 우선하는 설득력 있는 정당한 사유를 제시할 수 있거나 그 처리가 법적 청구권의 확립, 행사 또는 방어에 쓰이는 경우가 아니라면 해당 데이터의 처리를 중단해요. 문의 양식이나 법적 고지의 주소로 보내는 형식 없는 메시지로 충분해요.
privacy-links-title = 다른 서비스로 가는 링크
privacy-links = 이 사이트에는 GitHub, Discord, WhatsApp 등으로 가는 링크가 있어요. 단순한 링크일 뿐이며 아무것도 삽입되어 있지 않아서, 링크를 누르기 전에는 이 서비스들 중 어느 곳도 여러분의 방문을 알지 못해요. 링크를 누르면 브라우저는 어느 페이지에서 왔는지 알리지 않아요(리퍼러 없음). 그 뒤에는 각 제공자의 개인정보 처리방침이 적용되고, 제공자가 EU 밖에서 데이터를 처리할 수도 있어요. 그곳의 커뮤니티 그룹에서 다른 구성원에게 여러분의 어떤 정보가 보이는지는 해당 서비스가 정해요.
privacy-reports-title = 게임의 버그 신고와 사용 횟수 집계
privacy-reports = 게임에서 F8로 보내는 버그 신고에는 종류, 입력한 텍스트, 클라이언트 버전, 체크해 둔 첨부가 담기며, 문의 페이지에 그 목록이 있어요. 처리한 지 90일 후에, 늦어도 3년 후에 삭제돼요. 법적 근거: GDPR 제6조 제1항 (f)호(버그 없는 게임에 대한 우리의 정당한 이익). 클라이언트는 허용한 경우에만 충돌 보고서를 보내요(GDPR 제6조 제1항 (a)호, 클라이언트 설정에서 언제든 철회할 수 있음). 오프라인 게임 중에 클라이언트는 그 게임의 임의 식별자를 1분에 한 번 보내며 그 밖에는 아무것도 보내지 않아요. 우리는 이를 메모리에만 보관하고 마지막 생존 신호 후 150초가 지나면 잊어요(GDPR 제6조 제1항 (f)호, "익명 사용 횟수 보내기"에서 끌 수 있음). 모든 연결과 마찬가지로 IP 주소는 우리에게 도달해요.

## Terms

terms-title = 이용 약관
terms-draft = 이 약관은 초안이며 아직 효력이 없어요.
terms-version = 버전 { $version }, { $date } 업데이트.
# Shown only in the languages a machine translated: every one but German and English.
machine-note = 이 사이트는 기계가 번역했으며, 아직 한국어를 쓰는 사람이 읽고 확인하지 않았어요. 게임 자체는 아직 영어와 독일어로만 제공돼요. <link>오류를 알려 주세요.</link>
legal-machine = 이 페이지는 도움을 위해 제공하는 기계 번역이에요. <de>독일어</de> 또는 <en>영어</en> 텍스트와 다를 경우 그쪽이 적용돼요.
terms-machine = Central의 약관은 독일어와 영어로만 있어요. 아래는 영어 텍스트이며, <de>독일어 약관</de>도 있어요.

## Errors

not-found-title = 이 페이지는 없어요
not-found-lead = 주소가 잘못 입력되었거나 페이지가 이동했을 수 있어요.
gone-title = 이 페이지는 완전히 사라졌어요
gone-lead = 여기 있던 내용은 삭제되었어요. 시작 페이지에서 다음에 갈 곳을 찾을 수 있어요.
error-home = 시작 페이지로

## Baylee's remembrance (approved by her family on 27 October 2026; keep word for word)
baylee-dedication = Baylee에게. 영원히 햇살 속에서.
baylee-invitation = 이 세계 뒤에 있는 작은 영혼을 만나 보세요
baylee-open = Baylee를 기억해요 · 사진 앨범 열기
baylee-eyebrow = 이 프로젝트의 마음
baylee-date = 사랑으로 기억하며 · 2026년 10월 26일
baylee-close = Baylee의 세계로 돌아가기
baylee-previous = 이전 사진
baylee-next = 다음 사진
baylee-photo1 = 부드러운 흰 이불 속에 웅크린 Baylee.
baylee-photo2 = 꽃무늬 이불 너머로 내다보는 Baylee.
baylee-photo3 = 동그란 회색 침대에서 쉬고 있어요.
baylee-photo4 = 가까이 웅크리고 잠든 Baylee.
baylee-photo5 = 이불에 폭 안긴 고요한 한때.

baylee-storyTitle = Baylee에게. 영원히 햇살 속에서.
baylee-together = 우리와 함께 · 2025년 11월 17일 – 2026년 10월 26일
baylee-chapters = Baylee의 이야기
baylee-chapterPrevious = 이전 장
baylee-chapterNext = 다음 장
baylee-photos = 우리의 추억
baylee-read = 스크롤해서 읽기

baylee-chapter1Title = Baylee의 집
baylee-chapter1Body =
    Baylee는 2025년 11월 17일에 우리 집으로 왔어요.

    아내와 나는 크레펠트의 동물 보호소에서 Baylee를 입양했어요. 나이는 열세 살쯤으로 추정되었어요. 악성 유선 종양이 있었고, 암은 이미 간까지 퍼져 있었어요. Baylee는 완화 치료를 받는 고양이였어요. 우리가 함께할 시간이 한정되어 있다는 걸 알았어요. 우리와 함께 있는 동안만큼은 사랑받는 집이 있기를 바랐어요.

    Baylee는 곧 그 집을 얼마나 빨리 자기 것으로 만드는지 보여 주었어요.

    첫날부터 내가 일하는 동안 무릎 위로 슬며시 올라왔어요. 첫날 밤에는 내가 부모님의 고양이들을 위해 주문해 둔 사료 봉지를 찢어 구멍을 내고는 마음껏 먹었어요. 물론 Baylee는 자기 사료를 받았고 그날 저녁도 잘 먹었어요. 하지만 그 봉지는 좀 더 조사해 볼 가치가 있다고 생각했나 봐요.

    우리 작은 생쥐는 놀라운 식욕과 엄청난 에너지를 가지고 있었어요.

baylee-chapter2Title = 쌓여 가는 신뢰
baylee-chapter2Body =
    물을 아주 많이 마시는 것을 알아채고 수의사에게 데려갔어요. 혈액 검사에서 갑상선 질환이 나왔고, 그것이 엄청난 식욕과 활발함도 설명해 주었어요. 염증 수치가 높아서 갑상선 약과 일시적인 항생제 치료를 받았어요.

    갑자기 온갖 것에 몸을 비비고, 많이 울고, 평소와 달리 우리에게 달라붙어 어리광을 부리기 시작했을 때는 새 약이 잘 맞지 않는 건가 걱정했어요. 다시 검사해 보니 이유가 나왔어요. 우리 아가가 발정기였던 거예요. 중성화 수술을 받지 않았거든요.

    그 시간도 우리는 함께 이겨 냈어요. Baylee는 점점 더 우리를 믿었고, 우리는 서로 더 가까워졌어요.

baylee-chapter3Title = 우리의 작은 공주님
baylee-chapter3Body =
    캣타워를 받았을 때 얼마나 기뻐했는지 지금도 정확히 기억나요. 곧바로 맨 꼭대기 칸에 앉아 만족스럽게 집 안을 둘러보았어요. 신기하게도 그 자리는 그 뒤로 다시는 쓰지 않았어요. 왜 그랬는지는 Baylee만의 비밀로 남았어요.

    다른 자리는 비어 있는 일이 드물었어요. 내 무릎, 내 다리, 그리고 우리 침대였어요. 침대에서는 보통 나나 아내 위에서 잤어요. 가장 좋은 건 우리 둘 위에 한꺼번에 눕는 데 성공하는 일이었어요.

    꼬리를 높이 세우고 집 안을 당당하게 걸어 다녔어요. 자기가 이 집의 공주라는 것을 아주 잘 안다는 듯이요. 기개가 있었고, 자신의 선을 분명히 알릴 줄 알았어요. 우리와 함께라면 온전히 자기 자신일 수 있었어요. 별난 버릇도, 요구도, 작은 고집도 모두 사랑받았어요.

    동물과 이렇게 깊은 유대를 맺어 본 적은 처음이었어요. Baylee는 아내의 마음에도, 내 마음에도 아주 깊은 자리를 차지했어요.

baylee-chapter4Title = 햇볕 아래의 시간
baylee-chapter4Body =
    날씨가 따뜻해지자 Baylee는 발코니를 발견했어요. 하루의 첫 햇살이 털에 닿는 느낌을 좋아했어요. 얼마 지나지 않아 거의 온종일을 그곳에서 보냈어요.

    비둘기와 정원의 새들은 특별한 매력이었어요. 열심히 지켜보고, 숨어서 기다리고, 몇 번이고 놀래 주려 했어요. 일광욕과 새 구경 사이에서 우리 작은 공주님의 하루 일정은 아주 빽빽했어요.

baylee-chapter5Title = 자기만의 작은 생각
baylee-chapter5Body =
    그러는 동안 병은 계속 진행되었어요.

    종양에 염증이 생겼고, Baylee는 그 부위를 계속 핥아서 터뜨렸어요. 수의사에게서 처음으로 빨간색 회복 슈트를 받았어요. 그때부터 상처가 더 다치지 않도록 매일 입어야 했어요.

    금세 입는 데 익숙해졌어요. 하지만 입히는 일은 마지막 날까지 싫어했어요. 그럴 때는 밥 한 그릇으로도 어르기가 힘들었어요. 식욕이 아무리 커도 슈트에 대한 의견은 분명했어요.

    상처는 더 나빠졌어요. 고름이 나고, 결국 피가 나기 시작했으며, 불쾌한 냄새도 났어요. 그러면서도 Baylee는 일상에서 여전히 얼마나 큰 기쁨을 찾는지 계속 보여 주었어요. 놀고, 먹고, 주변을 탐험하고, 스크래처와 수납장에 올라갔어요.

    가끔은 우리 곁 침대 가장자리에 몇 시간이고 누워서 우리가 마침내 일어나 아침을 차려 주기를 기다렸어요. 너무 오래 걸리면 그냥 우리 위를 넘어 다녔어요. 우리 얼굴도 장애물이 아니었어요.

    음식에 대한 생각이 아주 뚜렷했어요. 입맛은 오히려 "냉동 피자" 쪽이었어요. 얼마 지나면 고급 고양이 사료에는 더는 흥분하지 않았고, 다양함이 꼭 필요했어요. 같은 사료가 두 번 연달아 나오는 건 말도 안 됐어요. 간이 든 닭고기, 치즈가 든 것은 무엇이든, 참치나 연어, 그런 것들이 Baylee를 행복하게 했어요.

baylee-chapter6Title = 꼬리를 높이 세우고
baylee-chapter6Body =
    종양은 점점 커지고 진물도 늘었어요. Baylee는 눈에 띄게 아파했어요. 통증이 가라앉으면 다시 우리의 작은 고양이로 돌아와, 꼬리를 높이 세우고 방 사이를 오가며 자기 사람들을 차지했어요.

    기력이 서서히 줄어드는 것이 보였어요. 그래도 식욕도, 호기심도, 애교도, 우렁찬 갸르릉 소리도 여전히 그대로였어요.

baylee-chapter7Title = 작별
baylee-chapter7Body =
    2026년 10월 23일 금요일 저녁, Baylee에게 처음에는 감기처럼 보이는 증상이 나타났어요. 콧물이 흐르고, 숨이 깊고 힘들어 보였어요. 마치 공기를 얻으려고 애쓰는 것처럼요.

    토요일에는 다시 활기차 보였어요. 평소와 다른 호흡과 콧물은 그대로였지만, 우리가 잘 아는 에너지와 삶에 대한 사랑을 다시 보여 주었어요.

    일요일 아침은 힘들었어요. 눈에서 눈물이 났고, 숨쉬기가 힘들었고, 아파했어요. 진통제를 주었어요. 한 시간쯤 지나자 다시 나아졌지만 호흡은 여전히 달랐어요. 수의사에게 데려가야 한다는 것을 알았어요.

    월요일 아침에 상태가 다시 나빠졌어요. 다행히 아내가 그날 쉬는 날이었어요. 병원에 전화해 예약을 잡고 Baylee를 데려갔어요.

    엑스레이는 암이 얼마나 퍼졌는지 보여 주었어요. 폐의 약 3분의 1만 숨쉬는 데 쓸 수 있다고 들었어요. 계속 살게 하는 것은 점점 더 적은 공기만 쉬게 한다는 뜻이었어요.

    우리는 작별해야 했어요.

    2026년 10월 26일 월요일 정오 무렵, 가장 사랑하는 Baylee는 잠들었어요.

baylee-chapter8Title = 영원히 햇살 속에서
baylee-chapter8Body =
    Baylee는 채 1년이 안 되는 시간 동안 우리와 함께했어요. 그동안 우리 가족의 일부이자 아주 가까운 친구가 되었어요. 우리의 일하는 낮과 밤과 아침에 깨어나는 일에 함께했어요. 발코니의 햇살 가득한 시간들과, 함께 사는 삶을 이루는 작은 습관들에도요.

    이제 그 모든 순간에 Baylee가 없어요.

    빈자리는 아직 생생하고 깊어요. 갸르릉 소리, 다리 위의 무게, 곧게 세운 꼬리, 우리가 드디어 아침을 챙겨 줄 거라는 의심 없는 기대가 그리워요.

    이 삶 뒤에 무언가 있다면, 그것이 무엇이든, 언젠가 그 영혼을 다시 만나기를 바라요.

    그때까지 우리는 작은 생쥐를 마음속에 영원히 품고 갈 거예요.

    그래서 이 취미 프로젝트를 Baylee에게 바쳐요. Baylee의 영혼, 가까움, 햇살과 새들에 대한 기쁨에게. 그리고 우리가 함께한 시간 동안 자란 사랑에게.

    이곳에서 Baylee가 구름 위에서 편히 잠들기를, 따뜻한 햇살 속에서 꽃과 비둘기와 정원의 새들에 둘러싸여.

    편히 쉬렴, 가장 사랑하는 Baylee.

baylee-photoOpen = 사진 전체 보기
baylee-photoClose = 사진 닫기
baylee-zoomIn = 확대
baylee-zoomOut = 축소
baylee-fit = 맞춤

baylee-photo6 = 담요 사이에 포근히 자리 잡은 Baylee.
baylee-photo7 = 빨간 회복 슈트를 입고 폭 안겨 있어요.
baylee-photo8 = 종이 상자 속 햇살 드는 자리.
baylee-photo9 = 폭 감싸여 완전히 느긋해요.
baylee-photo10 = 가장 좋아하는 자리: 바로 우리 곁.

## Errors shared by the forms

error-busy = 시도가 너무 많아요. 나중에 다시 해 보세요. 비밀번호를 여러 번 잘못 입력하면 한 시간, 길어야 하루가 걸릴 수 있어요.
error-unavailable = Baylee Central이 지금은 응답하지 않아요. 잠시 후 다시 시도해 주세요.
error-network = 연결되지 않았어요. 네트워크를 확인하고 다시 시도해 주세요.
error-failed = 문제가 생겼어요. 다시 시도해 보고, 계속되면 알려 주세요.
error-again = 다시 시도
password-show-label = Show the password

## Signing in

signin-title = 로그인
signin-lead = 온라인 테이블, 연락처, 채팅을 위한 Baylee 계정이에요.
signin-name = 로그인 이름
signin-password = 비밀번호
signin-submit = 로그인
signin-missing = 로그인 이름과 비밀번호를 입력해 주세요.
signin-working = 비밀번호를 안전하게 처리하는 중 … 잠시 걸리며, 브라우저 안에서만 이루어져요.
signin-wrong = 로그인 이름과 비밀번호가 일치하지 않아요. 둘 다 확인하고 다시 시도해 주세요.
signin-not-moved = 이 계정은 아직 Baylee Central로 옮겨지지 않았어요. Baylee 클라이언트로 한 번 로그인해 주세요. 그러면 계정이 옮겨져요.
signin-no-account = 아직 계정이 없나요? <register>계정 만들기</register>.
register-have-account = Already have an account? <signin>Sign in</signin>.
dialog-close = Close
signin-noscript = 로그인하려면 JavaScript가 필요해요. 비밀번호는 브라우저에서 처리되며 우리에게 전송되지 않아요.

## Registering

register-title = 계정 만들기
register-lead = 온라인 테이블, 연락처, 채팅을 위한 계정이에요. 오픈 베타 동안 누구나 무료로 만들 수 있어요.
register-name = 로그인 이름
register-name-hint = 비공개이며 로그인에만 써요. 소문자, 숫자, 점, 대시, 밑줄 3~24자.
register-display = 표시 이름
register-display-hint = 다른 플레이어에게 보이는 이름이에요. 글자, 숫자, -, _ 3~16자. 태그는 자동으로 붙어요.
register-password = 비밀번호
register-password-hint = 8자 이상. 긴 문장은 기억하기 쉽고 추측하기 어려워요.
register-repeat = 비밀번호 확인
register-repeat-hint = 오타를 막기 위해 한 번 더 입력해 주세요.
register-age = 만 16세 이상이에요.
register-consent = <terms>이용 약관</terms>에 동의해요. 데이터 처리 방식은 <privacy>개인정보 처리방침</privacy>에 있어요.
register-submit = 계정 만들기
register-error-name = 로그인 이름은 소문자, 숫자, 점, 대시, 밑줄 3~24자여야 해요.
register-error-display = 표시 이름은 글자 또는 숫자 3~16자여야 하며, -와 _는 중간에만 쓸 수 있어요.
register-error-reserved = 예약된 표시 이름이에요. 다른 이름을 골라 주세요.
register-error-short = 비밀번호는 8자 이상이어야 해요.
register-error-long = 비밀번호가 너무 길어요.
register-error-repeat = 두 비밀번호가 같지 않아요.
register-error-age = 계정을 만들려면 만 16세 이상이어야 해요.
register-error-consent = 계정을 만들려면 이용 약관에 동의해 주세요.
register-error-taken = 이미 사용 중인 로그인 이름이에요. 다른 이름을 골라 주세요.

## The recovery code

code-title = 복구 코드
code-handle = 내 핸들: { $handle }
code-lead = 비밀번호를 잊어버렸을 때 이 코드가 있어야만 암호화된 데이터가 남아 있어요. 우리는 이 코드를 알지 못하며 복원할 수도 없어요. 이 기기와 떨어진 안전한 곳에 보관해 주세요.
code-shown-once = 지금만 표시돼요
code-download = 텍스트 파일로 저장
code-kept = 복구 코드를 저장했어요
code-not-kept = 먼저 코드를 저장한 다음 확인해 주세요.
code-continue = 계속
code-later = 나중에

## Central's terms

terms-step-title = 새 이용 약관
terms-step-lead = Baylee Central의 약관이 바뀌었어요. 계정을 계속 쓰려면 읽고 동의해 주세요. 거부하고 로그아웃할 수도 있어요.
terms-accept = 동의
terms-decline = 거부하고 로그아웃

## The signed-in area

account-lead = 내 Baylee 계정: 프로필, 보안, 개인정보, Gateways.
account-nav = 내 계정
account-home = 개요
account-profile = 프로필
account-profile-lead = 핸들, 표시 이름, 복구 코드예요.
account-security = 보안
account-security-lead = 로그인되어 있는 곳과 로그아웃이에요.
account-privacy = 개인정보와 데이터
account-privacy-lead = 데이터를 내보내거나 계정을 삭제해요.
account-gateways = Gateways와 토큰
account-gateways-lead = 등록한 Gateway와 그 로그인에 쓰이는 토큰이에요.
account-loading = 불러오는 중 …
account-signed-out = 로그인되어 있지 않아요.
account-unnamed = 내 계정
account-signin-name = 로그인 이름: { $name }
account-until = { $time }까지 로그인됨
account-needs-title = 확인이 필요해요
account-needs-code = 복구 코드 만들기
account-needs-name = 표시 이름 정하기
account-admin = 관리

reauth-lead = 최근에 로그인한 기록이 필요해요. 로그인 이름과 비밀번호를 한 번 더 입력해 주세요.
reauth-submit = 다시 로그인

profile-who-title = 나에 대해
profile-handle = 핸들
profile-signin-name = 로그인 이름
profile-source = 계정
profile-source-moved = 공식 Gateway에서 옮겨 옴
profile-source-registered = 이 사이트에서 가입함
profile-choose-title = 표시 이름 정하기
profile-choose-submit = 표시 이름 저장
profile-name-taken = 이 계정에는 이미 표시 이름이 있어요.
profile-code-title = 복구 코드
profile-code-kept = 계정에 복구 코드가 있어요. 안전하게 보관하세요. 비밀번호가 없을 때 암호화된 데이터로 돌아갈 수 있는 유일한 방법이에요.
profile-code-missing = 계정에 아직 복구 코드가 없어요. 하나 만들어 주세요. 비밀번호를 다시 입력해야 해요.
profile-code-make = 복구 코드 만들기

security-sessions = 로그인되어 있는 곳
security-lead = Baylee Central은 이 웹사이트용 세션 하나와 게임 클라이언트용 세션 하나를 유지하며, 개별 기기를 구분하지는 않아요.
security-kind-browser = 이 웹사이트
security-kind-client = Baylee 클라이언트
security-this = 현재 세션
security-signed-in = 로그인
security-ends = 종료
security-gateways = Gateways에서
security-end-client = 클라이언트 로그아웃
security-client-ended = 클라이언트가 로그아웃되었어요.
security-sign-out = 여기서 로그아웃
security-password-title = 비밀번호
security-password-later = 웹사이트에서 비밀번호를 바꾸는 기능은 추후 지원할 예정이에요. 그때까지는 Baylee 클라이언트에서 바꿔 주세요.

privacy-export-title = 데이터 내보내기
privacy-export-lead = Baylee Central이 계정에 대해 아는 내용을 JSON 파일로 내보내요. 기본 데이터, 세션, 동의한 약관, 내 Gateways, 계정이 언급된 감사 기록이 들어 있어요. 암호화된 내용은 Baylee 클라이언트에서만 복호화하고 내보낼 수 있어요. 내보내려면 30분 이내에 로그인해야 해요.
privacy-export-make = 내보내기 준비
privacy-export-save = 내보낸 파일 저장
privacy-delete-title = 계정 삭제
privacy-delete-lead = 계정, 데이터, 세션, 연락처, 소유한 Gateways가 운영 중인 시스템에서 즉시, 영구적으로 삭제돼요. 백업에서는 늦어도 12개월 안에 사라져요. Central은 보낸 메시지도 삭제하지만, 다른 사람이 이미 자기 기기에 저장한 것은 그대로 남을 수 있어요.
privacy-delete-start = 내 계정 삭제
privacy-delete-password = 확인을 위한 비밀번호
privacy-delete-prove = 계속
privacy-delete-last = 마지막 단계예요. 되돌릴 수 없어요. 1분 안에 진행해 주세요.
privacy-delete-gateways = 소유한 다음 Gateways도 함께 삭제돼요:
privacy-delete-confirm = 계정을 영구 삭제
privacy-delete-cancel = 계정 유지
privacy-deleted = 계정이 삭제되었어요.

gateways-none = 등록한 Gateway가 없어요. 대부분의 플레이어는 필요하지 않아요.
gateways-add-title = Gateway 등록
gateways-origin = Gateway 주소
gateways-origin-hint = Gateway의 https 주소예요. 예: https://play.example.org.
gateways-name = Gateway 이름
gateways-add = Gateway 등록
gateways-added = Gateway가 등록되었으며 관리자의 승인을 기다리고 있어요.
gateways-taken = 이미 등록된 주소예요.
gateways-invalid = 경로 없는 https 주소와 이름을 입력해 주세요.
gateways-approved = 승인됨
gateways-pending = 승인 대기 중
gateways-disabled = 비활성화됨
tokens-title = 토큰
tokens-name = 토큰 이름
tokens-days = 유효 기간(일)
tokens-sso = 플레이어 로그인
tokens-listing = Gateway 등록
tokens-both = 플레이어를 로그인시키고 Gateway를 목록에 올려요
tokens-make = 토큰 만들기
tokens-revoke = 폐기
tokens-revoked = 폐기됨
tokens-revoked-now = 토큰이 폐기되었어요.
tokens-until = { $time }까지
tokens-invalid = 토큰 이름을 정하고 허용할 항목을 하나 이상 골라 주세요.
tokens-shown-title = 새 토큰
tokens-kept = 저장했어요
tokens-again = 다시 로그인되었어요. 토큰을 다시 만들어 주세요.

## The messenger

account-chat = 채팅
account-contacts = 연락처
chat-side = 대화와 연락처
chat-tabs = 메신저
chat-tab-chats = 채팅
chat-tab-contacts = 연락처
chat-main = 대화
chat-unread = 읽지 않음
chat-conversations = 내 대화
chat-search = 대화 찾기
chat-search-hint = 찾기 (Ctrl+K)
chat-new-group = 새 그룹
chat-no-conversations = 아직 대화가 없어요. 연락처를 추가한 다음 글을 보내 보세요.
chat-no-match = 그 이름의 대화가 없어요.
chat-former-group = 이전 그룹
chat-former-lead = 더는 이 그룹에 속해 있지 않아요.
chat-you = 나
chat-members = { $n ->
   *[other] 구성원 { $n }명
}
chat-say-hello = 인사해 보세요.
chat-history-only = 더는 연락처가 아니에요.
chat-today = 오늘
chat-yesterday = 어제
chat-encrypted = 종단간 암호화
chat-opening-lead = 대화와 연락처예요. 메시지는 기기를 떠나기 전에 기기에서 암호화되며, Central은 내용을 읽지 못한 채 전달만 해요.
chat-new-from = { $who }의 새 메시지.
chat-new-many = 새 메시지 { $n }개.
chat-retry = 다시 시도
chat-discard = 삭제
chat-not-sent = 보내지 못했어요.
chat-sending = 보내는 중 …
chat-keys-changed = 이 대화에서 누군가의 키가 바뀌었어요. 다시 글을 쓰기 전에 안전 번호를 비교해 주세요.
chat-refused = 일부 메시지는 확인할 수 없어서 표시되지 않아요.
chat-failed = 채팅이 지금은 Central에 닿지 못했어요.
chat-failed-open = 채팅을 열 수 없었어요. 페이지를 새로 고쳐 다시 시도해 주세요.
chat-empty = 아직 메시지가 없어요.
chat-older = 이전 메시지 보기
chat-compare-first = 키가 바뀌었어요. 먼저 안전 번호를 비교한 다음 글을 쓰세요.
chat-compare = 안전 번호 비교
chat-friend-required = 다시 연락처가 되면 글을 쓸 수 있어요.
chat-group-left = 더는 이 그룹에 속해 있지 않아요.
chat-details = 상세
chat-close = 닫기
chat-back = 대화 목록으로
chat-messages = 메시지
chat-fan = Baylee는 Wizards가 허가하거나 보증하지 않은 비공식 팬 콘텐츠예요. <more>그 의미</more>
chat-safety = 안전 번호
chat-safety-lead = 상대방과 직접 만나거나 믿을 수 있는 다른 채널로 번호를 비교하세요. 일치하면 아무도 몰래 끼어들지 않은 거예요.
chat-changed = 변경됨
chat-confirm-keys = 일치해요: 새 키를 신뢰
chat-count = 1,000자 중 { $n }자
chat-write = 메시지 쓰기
chat-send = 보내기
chat-enter-hint = Enter는 보내기, Shift+Enter는 줄바꿈이에요.
chat-choose = 대화를 선택하세요
chat-choose-lead = 메시지는 이 브라우저에서 암호화되고, Central은 전달만 해요.
chat-storage = 로그인해 있는 동안 채팅 키는 이 브라우저에 머물러요.
chat-opening = 채팅을 여는 중 …
chat-open = 내 채팅 열기
chat-ended = 로그인이 종료되었어요.
chat-unnamed = 먼저 표시 이름을 정해 주세요. 다른 사람이 글을 보낼 핸들이 만들어져요.
chat-missing-title = 채팅을 열려면 다시 로그인하세요
chat-missing-lead = 이 브라우저에는 채팅 키가 없어요. 비밀번호로 여기서 열 수 있으며, 비밀번호는 이 브라우저를 떠나지 않아요.
chat-invites = 그룹 초대
chat-invited-by = { $who }님이 초대
chat-group-name = 그룹 이름
chat-group-name-hint = 1~48자의 이름.
chat-group-create = 만들기
chat-group-failed = 그룹을 지금은 바꿀 수 없었어요.
chat-group-members = { $n ->
   *[other] 구성원 { $n }명
}
chat-group-owner = 소유자
chat-group-rename = 이름 바꾸기
chat-group-transfer = 소유자로 지정
chat-group-remove = 내보내기
chat-group-transfer-ask = { $who }님을 이 그룹의 소유자로 지정할까요? 나는 구성원으로 남아요.
chat-group-remove-ask = { $who }님을 이 그룹에서 내보낼까요?
chat-group-invite = 연락처 초대
chat-group-choose = 연락처 선택
chat-group-invite-button = 초대
chat-group-invited = 초대함
chat-group-delete = 그룹 삭제
chat-group-leave = 그룹 나가기
chat-group-delete-ask = “{ $name }” 그룹을 모두에게서 삭제할까요? 이미 받은 메시지는 계속 읽을 수 있어요.
chat-group-leave-ask = “{ $name }” 그룹에서 나갈까요? 새 초대를 받아야만 다시 들어올 수 있어요.

people-incoming = 요청
people-failed = 연락처를 지금은 불러올 수 없었어요.
people-empty = 아직 연락처가 없어요. 핸들로 상대를 찾거나 내 핸들을 알려 주세요.
people-your-handle = 내 핸들:
people-incoming-title = 받은 요청
people-friends = 연락처
people-outgoing = 보낸 요청
people-blocked = 차단함
people-find = 핸들로 연락처 추가
people-find-button = 찾기
people-handle-hint = 태그가 붙은 전체 핸들이에요. 예: Name#1a2b.
people-yourself = 내 핸들이에요.
people-nobody = 이 핸들의 사용자는 없어요.
people-request = 연락처 요청 보내기
people-requested = 요청을 보냈어요.
people-is-friend = 연락처
people-is-incoming = 연락처를 요청했어요
people-is-outgoing = 요청 보냄
people-is-blocked = 차단함
people-write = 글쓰기
people-remove = 삭제
people-block = 차단
people-accept = 수락
people-decline = 거절
people-cancel = 취소
people-unblock = 차단 해제
people-keep = 유지
people-confirm-remove = { $who }님을 연락처에서 삭제할까요? 메시지는 그대로 남아요.
people-confirm-block = { $who }님을 차단할까요? 이후 연락처 요청도, 글도 보낼 수 없어요.

## Offline, updates, installing

title-offline = 오프라인
offline-title = 오프라인 상태예요
offline-lead = 이 페이지는 연결이 필요해요. 다시 연결될 때까지 Central이 메시지를 보관해요.
offline-again = 다시 시도
update-ready = baylee.app의 새 버전이 준비되었어요.
update-unsent = 아직 보내지 않은 텍스트가 있어요. 그래도 새로 고칠까요?
update-reload = 새로 고침
install-app = 앱으로 설치

## Notifications

push-title = 알림
push-lead = Baylee가 알려도 되는 내용을 고르세요. 알림에는 메시지 텍스트, 이름, 신고 내용이 표시되지 않아요.
push-device = 이 기기의 알림
push-categories = 알림 받을 항목
push-chat = 채팅 메시지
push-contacts = 연락처 요청
push-updates = 새 버전
push-feedback = 피드백 신고(관리자)
push-gateways = Gateway 신청(관리자)
push-on = 이 기기에서 켜져 있어요. 선택한 범주만 전달돼요.
push-off = 이 기기에서 꺼져 있어요. 범주 선택은 유지돼요.
push-scope = 범주 선택은 계정에 적용돼요. 기기 권한은 브라우저마다 따로 설정해요. 로그아웃하면 이 기기의 연결이 끊겨요.
push-failed = 변경을 확인하지 못했어요. 다시 시도해 주세요.
push-denied = 알림이 차단되어 있어요. 브라우저나 기기 설정에서 허용한 다음 다시 시도할 수 있어요.
push-unsupported = 이 브라우저는 여기서 Web Push를 지원하지 않아요. iPhone과 iPad에서는 Baylee를 홈 화면에 추가한 다음 거기서 열어 주세요.
push-unconfigured = 이 서버는 아직 백그라운드 알림을 지원하지 않아요. 설정은 미리 저장해 둘 수 있어요.

register-closed = 지금은 가입이 일시 중지되어 있어요. 나중에 다시 시도해 주세요.
register-refused = 가입이 접수되지 않았어요. 다시 시도해 주세요.
admin-registration = 가입
admin-registration-open = 열림: 작업 증명을 푼 누구나 계정을 만들 수 있어요.
admin-registration-closed = 닫힘: 다시 열 때까지 아무도 계정을 만들 수 없어요.
admin-registration-lead = 공격을 받는 동안에는 가입을 닫고 직접 다시 여세요. 모든 변경은 감사 로그에 기록돼요.
admin-registration-close = 가입 닫기
admin-registration-reopen = 가입 다시 열기
hero-discord = Discord 커뮤니티
hero-whatsapp = WhatsApp 커뮤니티
hero-github = GitHub
nav-menu = 메뉴
foot-project = 프로젝트
foot-source = 소스 코드 (AGPL-3.0)
foot-central = Baylee Central (소스 공개)
foot-summary = Rust와 Bevy로 만든 무료 오픈 소스 테이블탑 카드 게임. 결정론적 룰 엔진, 완전한 오프라인 모드, 하우스 AI, LLM 브리지, 커뮤니티 Gateways를 갖췄어요.
strip-build = 빌드
strip-ci = CI
strip-commit = 커밋
strip-gateways = Gateways
strip-online = 온라인
strip-offline = 오프라인
strip-releases = 릴리스 보기
strip-on-github = GitHub에서 보기
strip-directory = 목록
strip-cards = 플레이 가능한 카드
strip-cards-unit = 장
strip-stars = GitHub 스타
strip-accounts = Baylee Central의 계정
strip-live = 지금 Gateways가 보고한 수치
strip-views = Page views since { $date }
live-views = { $n ->
    [one] { $n } view
   *[other] { $n } views
}
live-accounts = { $n ->
   *[other] 계정 { $n }개
}
live-players = { $n ->
   *[other] 플레이어 { $n }명
}
live-tables = { $n ->
   *[other] 열린 테이블 { $n }개
}
live-games = { $n ->
   *[other] 게임 { $n }개
}
hero-badges = 한눈에 보기
hero-badge-beta = 오픈 베타
hero-badge-free = 무료
hero-badge-open = 오픈 소스
hero-badge-fan = 비공식 팬 콘텐츠
gws-open-client = Baylee에서 열기
gws-open-client-help = “Baylee에서 열기”는 설치된 클라이언트(0.1.1-beta.9b 이상)를 실행해요. 클라이언트가 주소를 보여 주고, Gateway를 추가하기 전에 물어봐요.
dl-menu-title = 모든 시스템
dl-old-cpu = 구형 프로세서 (AVX2 없음)
ci-passing = 통과
ci-failing = 실패
dl-button = Baylee 다운로드
dl-quick = 다른 시스템과 아키텍처
dl-others = 다른 시스템

# The start page, as the old site wrote it (owner, 11.11.2026)
link-readmeModel = #playing-against-a-language-model
about-engineTitle = 모든 규칙을 지키게 하는 엔진
about-deterministic = 시드 기반 난수 생성기를 쓰고 실제 시계나 해시 순서에 전혀 의존하지 않아서, 모든 대전이 엄밀히 결정론적이며 한 수 한 수 그대로 재현돼요.
about-tableTitle = Bevy로 만든 깔끔한 3D 테이블
about-keys = 통합 로비에서 로컬 덱 빌더, 솔로 대전, 커뮤니티 Gateway의 멀티플레이 테이블 사이를 편하게 오갈 수 있어요.
feature-hiddenTitle = 숨겨진 정보는 비공개로 유지돼요
feature-hidden = 클라이언트는 내 자리가 볼 수 있는 데이터만 받아요. 서고와 상대의 손은 오직 장수로만 전달되어, 수정된 클라이언트로도 숨겨진 카드를 볼 수 없어요.
feature-seatsTitle = 한 테이블에 최대 여덟 명
feature-seats = 듀얼, 난전, 팀전(예: 2v2)을 최대 8자리까지 지원해요. 팀원은 턴을 공유하고 서로에게 손의 카드를 보여 줄 수 있어요.
feature-commanderTitle = 유연한 포맷과 자유로운 하우스 룰
feature-commander = 클래식한 60‑장 대전, 다인용 난전, 자유로운 하우스 포맷에 맞춰 덱을 짜요. 시작 생명점, 팀 구성, 멀리건 규칙은 호스트가 자유롭게 정해요.
feature-aiTitle = 결정론적 리플레이와 F8 신고
feature-ai = 모든 대전이 한 수 한 수 기록돼요. 플레이 중 규칙 상호작용에 문제가 보이면 테이블에서 바로 F8을 눌러 재현 가능한 상태 보고서를 보내세요.
feature-processTitle = 대전마다 격리된 서버 프로세스
feature-process = 진행 중인 게임마다 Gateway에서 자기만의 샌드박스 엔진 프로세스로 실행돼요. Gateway 자체는 로비와 방만 관리하므로, 오류가 다른 테이블에 영향을 주지 않아요.
feature-languagesTitle = 19개 언어의 카드 텍스트
feature-languages = Gateway가 다국어 카드 카탈로그를 제공하면 카드가 선호하는 언어로 표시되며, 없으면 영어로 돌아가요. 클라이언트 UI는 독일어와 영어로 제공돼요.
feature-watchTitle = 진행 중인 대전 관전
feature-watch = 로비의 관전 버튼을 누르면 실시간 관전자로 테이블에 앉아, 모든 플레이어의 손은 숨겨진 채 공개된 전장을 실시간으로 볼 수 있어요.
feature-connectionTitle = 연결이 끊겨도 매끄러운 재접속
feature-connection = 연결이 잠깐 끊기면 테이블이 카운트다운과 함께 자리를 지키고, 대전을 일시 정지하거나 하우스 AI가 임시로 대신할 수 있어요.
feature-tablesTitle = 열네 가지 결정 시계
feature-tables = 빠른 15‑초 턴부터 느긋한 시간 제한 없는 테이블까지, 방마다 따로 속도를 설정해요.
shots-eyebrow = 테이블에서
shots-title = 게임 속으로
shots-lead = 3D 클라이언트에서 직접 담은 짧은 실시간 영상이에요. 카드 이미지는 Scryfall에서 가져왔어요(© Wizards of the Coast). 대전과 4‑인 난전부터 암호화된 로비, 덱 빌더까지 보여 줘요.
shots-credit = 카드 이미지: Scryfall · © Wizards of the Coast
shots-showLarge = 크게 보기: { $title }
shots-dialog = 스크린샷
shots-previous = 이전
shots-next = 다음
shots-close = 닫기
ai-eyebrow = AI 상대와 언어 모델
ai-title = 하우스 AI, LLM 브리지, 네이티브 Baylee AI
ai-houseTitle = 다섯 단계의 휴리스틱 하우스 AI
ai-house = 오프라인에서도 모든 온라인 테이블에서도 바로 쓸 수 있어요. 초보, 캐주얼, 꾸준함, 날카로움, 전문가 중에서 고르세요. 하우스 AI는 기다림 없이 로컬에서 결정하며, 사람 플레이어와 똑같은 공개 상태만 봐요.
ai-modelTitle = Seat-Bridge로 테이블에 앉는 언어 모델
ai-model = <b>Google Gemini, OpenAI ChatGPT, Anthropic Claude, DeepSeek</b> 또는 <b>LM Studio</b>의 로컬 모델을 테이블에 바로 앉힐 수 있어요. API 키는 지출 한도와 함께 환경 변수에 안전하게 보관되며, 시간이 초과되면 하우스 AI가 매끄럽게 이어받아요.
ai-setup = LLM 및 Seat-Bridge 설정 안내
home-gwLead = 온라인 테이블은 Gateway에서 열려요. Gateway는 관리자가 승인한 뒤 Baylee Central에 올라오는 서버예요. 여기에는 처음 몇 개만 있고, 전체는 목록에서 볼 수 있어요.
beta-open = 목록 열기
beta-own = 직접 운영하기
source-eyebrow = 소스
source-title = 오픈 소스이고, 앞으로도 그래요
source-promise = Baylee는 앞으로도 언제나 오픈 소스이고 무료예요. 유료 장벽도, 유료 기능도, 광고도 없어요. 지금도, 나중에도요. 코드는 GNU Affero 일반 공중 사용 허가서를 따르며, 수정한 Gateway를 다른 사람에게 운영하는 사람은 누구든 그 소스도 제공해야 한다는 뜻이에요.
source-rust = Rust로 작성했으며, 코딩 에이전트의 도움을 많이 받은 한 사람의 프로젝트예요. 모든 변경은 같은 관문을 거쳐요. 포맷, 린트, 테스트 모음, 그리고 모든 카드를 인쇄된 텍스트와 대조하는 검증이에요. 이슈와 풀 리퀘스트를 환영해요. <agents>AGENTS.md</agents>와 <claude>CLAUDE.md</claude>는 사람과 에이전트 모두를 위한 작업 규칙이에요.
source-discord = Discord 커뮤니티
source-whatsapp = WhatsApp 커뮤니티
source-architecture = 아키텍처
source-repoTitle = 저장소 현황
source-stars = 스타
source-issues = 열린 이슈와 풀 리퀘스트
source-note = 이 사이트의 서버가 10분마다 GitHub의 공개 API에서 읽어 와요. 브라우저는 GitHub에 아무것도 요청하지 않아요.
gw-count =
    { $n ->
       *[other] 등록된 Gateway { $n }개, 그중 온라인 { $online }개.
    }
gw-none = 아직 등록된 Gateway가 없어요.
contact-intro = 질문, 아이디어, 보고 싶은 게임, 또는 공개로 올리고 싶지 않은 이야기라면, 이 양식은 Baylee를 만든 사람에게 바로 전달돼요. 답장을 원하면 이메일 주소를 남겨 주세요. 주소가 없으면 읽을 수만 있어요.
contact-f8 = 잘못 처리된 카드는 게임 전체를 보고서에 담을 수 있으므로 게임에서 <kbd>F8</kbd>로 신고하는 것이 가장 좋아요.
contact-discord = 대화하고, 상대를 찾고, 새 빌드 소식 듣기
contact-issues = GitHub 이슈
contact-issuesSub = 버그와 아이디어, 공개로
contact-operator = 이 사이트를 운영하는 사람과 우편 주소: <impressum>법적 고지</impressum>.
contact-whatsappSub = 커뮤니티 그룹과 빠른 대전
contact-reportsTitle = 신고와 개인정보
contact-reportsSub = F8 신고와 데이터에 대한 자세한 내용
ai-housePill = 오프라인과 온라인
ai-providers = 지원하는 언어 모델
ai-hostsPill = Gateway 운영자용
ai-hostsTitle = Gateway에서 호스팅 LLM 자리를 제공하기
ai-hosts = Gateway 운영자는 서버에서 미리 설정된 언어 모델을 바로 제공할 수 있어요. 테이블의 손님은 호스팅된 모델 중 하나를 상대로 고르기만 하면 되고, API 키는 필요 없어요.
ai-trainingPill = 학습 중 · 네이티브 엔진 AI
ai-trainingTitle = 네이티브 Baylee AI — 실제 플레이어처럼 엔진 위에서 직접 플레이하도록 학습 중
ai-training = 하우스 AI와 외부 LLM 브리지 외에도, 결정론적 룰 엔진 위에서 직접 전용 네이티브 Baylee AI를 학습시키고 있어요. 숙련된 사람처럼 판세, 우선권 창, 전투 계산, 덱 시너지를 평가하는 법을 배우며, 외부 API 지연은 전혀 없어요.
ai-hostingLink = Gateway 호스팅과 LLM 설정

# The gateways and the contact page, as the old site wrote them (owner, 11.11.2026)
gw-lead = Gateway는 온라인 테이블이 열리는 서버예요. 계정, 덱, 방, 게임당 엔진 프로세스 하나가 있어요. 여기 나온 것은 Baylee Central에 등록되고 관리자가 승인한 Gateway예요. 공식 표시가 있는 것은 Baylee가 운영하고, 나머지는 다른 사람들이 자기 그룹을 위해 운영하며 그곳에서는 그들의 규칙이 적용돼요. 목록에 있다고 해서 Gateway가 무엇을 하는지 보증하는 것은 아니에요.
gw-howTo = Gateway에서 플레이하려면 주소를 복사해 클라이언트의 첫 화면에 붙여 넣으세요. 온라인은 Central이 최근 1분 30초 안에 그 Gateway에서 소식을 받았다는 뜻이며, 응답한다는 보장은 아니에요. 이름과 버전은 각 운영자가 알려 준 그대로예요.
gw-find = Gateway 찾기
gw-findHint = 이름 또는 주소
gw-shown = { $all }개 중 { $n }개 표시
gw-noMatch = 지금까지 불러온 Gateway 중 일치하는 것이 없어요.
gw-showAll = 모두 보기
gw-stats = The directory in numbers
gw-stat-listed = Gateways listed
gw-stat-online = Online now
gw-stat-players = Players online
gw-stat-tables = Open tables
gw-stat-games = Games running
gw-show = Show
gw-show-every = All
gw-show-online = Online
gw-show-official = Official
gw-show-tables = With open tables
gw-sort = Sort by
gw-sort-best = Recommended
gw-sort-name = Name (A–Z)
gw-sort-players = Most players
gw-sort-tables = Most open tables
gw-sort-version = Newest version
gw-view = View
gw-view-cards = Cards
gw-view-list = List
link-readmeGateway = #running-your-own-gateway
selfhost-eyebrow = 셀프 호스팅과 Central 연동
selfhost-title = 나만의 Gateway 운영하기
selfhost-processes = 셀프 호스팅 Gateway는 PostgreSQL 18+와 함께 가벼운 프로세스 세 개를 실행해요. <code>baylee-gateway</code>는 방, 덱, 라우팅을 관리하고, <code>baylee-agent</code>는 대전마다 격리된 <code>baylee-engine-server</code> 프로세스를 하나씩 띄워서 모든 테이블이 샌드박스에서 돌아가게 해요.
selfhost-proxy = HTTPS 원본(예: <code>https://play.example.org</code>) 앞에 TLS 리버스 프록시를 두고, 원한다면 19개 언어의 카드 텍스트를 위해 Scryfall의 벌크 카탈로그를 가져오면, 플레이어가 클라이언트에서 내 주소로 바로 접속할 수 있어요.
selfhost-central = <b>Baylee Central 연결과 개인정보 설정:</b> <mine>내 Gateways</mine>에서 HTTPS 원본을 등록하고 <code>bgt_…</code> 토큰을 발급하세요(범위: 싱글 사인온용 <code>sso</code>, 디렉터리 하트비트와 실시간 텔레메트리용 <code>listing</code>). Gateway의 콘솔에서 텔레메트리 공개 수준을 고르세요(<code>full</code>은 진행 중인 게임, 테이블 설정, 동의한 플레이어, <code>counts_only</code>는 합계만, <code>private</code>은 가동 여부만 알려요).
selfhost-guide = <guide>전체 셀프 호스팅 안내 읽기</guide> · <protocol>프로토콜 및 하트비트 참고 자료 (docs/protocol.md)</protocol>
selfhost-source = 모든 Gateway는 로그인 없이 <code>GET /source</code>에 라이선스, 버전, 커밋을 응답하여 AGPL 소스 공개 의무를 자동으로 이행해요.
reports-eyebrow = 신고
reports-title = 잘못된 점을 발견했나요? 알려 주세요
reports-f8 = 테이블이나 로비에서 <kbd>F8</kbd>을 누르세요. 신고에는 종류, 입력한 텍스트, 클라이언트 버전, 테이블에서는 게임 ID가 담겨요. 그 밖의 항목은 각각 체크 상자가 있고 모두 처음에 체크되어 있으니, 보내기를 누르기 전에 남기고 싶지 않은 것은 체크를 해제하세요. 시스템과 하드웨어, 보이는 그대로의 테이블, 로그(다른 플레이어는 "플레이어 A", "플레이어 B" … 로 바뀜), 설정, Baylee 창만 담은 사진, 테이블에서는 게임의 전체 기록(모든 수와 모든 자리의 카드, 이름 없음)이에요. <i>전송 내용 보기</i>를 누르면 모든 바이트를 볼 수 있어요. 세션 토큰은 절대 전송되지 않아요.
reports-usage = 오프라인으로 플레이하는 동안 클라이언트는 게임이 진행 중이라는 사실을 1분에 한 번 알려요. 계정도, 기기 ID도, 게임에 관한 정보도 없어요. 설정 › 개인정보 및 데이터의 <i>익명 사용 횟수 보내기</i>에서 끌 수 있어요.
reports-discord = 대화하고, 상대를 찾고, 새 빌드 소식을 가장 먼저 듣고 싶나요? <discord>Discord의 Baylee 커뮤니티</discord>에 참여하세요.
reports-issues = 규칙 질문, 잘못된 카드 동작, 아이디어는 <issues>GitHub에 이슈를 열어</issues> 주세요. 잘못 처리된 카드가 우리가 받을 수 있는 가장 유용한 신고예요. 모든 카드에는 테스트가 있고, 신고하신 카드가 그 테스트가 되기 때문이에요.
reports-privacy = <docs>docs/privacy.md</docs>에 클라이언트, Gateway, 피드백 서비스가 보관하는 모든 것이 나와 있어요. 이 사이트는 로그인하지 않으면 쿠키를 설정하지 않고, 분석도 하지 않으며, 다른 곳의 서버에서 글꼴이나 스크립트를 불러오지 않아요. <notice>개인정보 처리방침</notice>에는 사이트와 Baylee Central이 보관하는 것, 그리고 통신하는 다른 모든 호스트와 그 시점이 나와 있어요.

# The download page, as the old site set it (owner, 11.11.2026)
dlPage-eyebrow = 받기
dlPage-filesEyebrow = 모든 파일
dlPage-filesTitle = 모든 시스템
install-eyebrow = 설치
install-releases = GitHub의 모든 릴리스
release-notes = 릴리스 노트와 체크섬
release-all = 모든 릴리스

# The download page's head, as the old site wrote it
dlPage-title = 다운로드
dlPage-lead = 모든 다운로드는 중간 단계 없이 GitHub의 파일로 바로 연결돼요. 릴리스에 설치 프로그램이 있다면 그것이 가장 쉬운 방법이고, 압축 파일에는 같은 게임이 들어 있어 어디에든 풀어 쓸 수 있어요. 설치된 Baylee는 자기 폴더에 쓸 수 있는 곳이라면 어디서든 서명된 압축 파일로 스스로 업데이트해요. 모든 패키지에는 게임 옆에 언어 모델용 Seat-Bridge가 들어 있어요.

# The reading pages' eyebrows, as the old site set them
imp-eyebrow = 법적 고지
terms-eyebrow = 이용 약관
privacy-eyebrow = 개인정보

# The privacy notice's contents, as the old site named them
privacy-contents = 목차

# The account pages' eyebrow, as the old site set it
acct-eyebrow = Baylee Central

# The account area's navigation, as the old site named it
app-account = 내 계정

# ==== The security page (/security) ====================================
# Every sentence and every arrow of this page maps to a row of the claim
# table in docs/security-page.md; a claim changes
# there first. No "safe", "unbreakable" or compliance words: mechanisms only.
title-security = Baylee의 보안: 비밀번호, 키, 채팅
desc-security = Baylee가 비밀번호, 키, 채팅 메시지를 어떻게 다루는지 네 개의 도표로 단계별로 보여 주며, 각 흐름의 한계도 함께 알려 드려요.
nav-security = 보안
sec-eyebrow = 보안
sec-title = 비밀번호, 키, 채팅: 무엇이 어디로 가는지
sec-lead = 네 가지 흐름을 단계별로 보여 드려요. 내 기기를 떠나는 것, Baylee Central이 보는 것과 보지 못하는 것, 그리고 각 흐름의 한계예요.
sec-toc-label = 이 페이지의 내용
sec-figure = 단계별 도표: { $name }
sec-play = 재생
sec-all = 모든 단계
sec-step = { $n }단계
sec-hint = 도표는 저절로 재생돼요. 단계를 고르면 그 단계에서 멈춰요.
sec-hint-still = 단계를 고르면 강조돼요.
sec-in-browser = 점선 틀이 내 브라우저예요. 이 모든 일은 그 안에서 일어나요.
# The actors' names are the only words inside the drawings, where text does
# not wrap: one or two words each, at most about twice the English width
# (the test `names_fit_with_room_to_grow` measures every language).
sec-actor-device = 내 기기
sec-actor-central = Central
sec-actor-page = 페이지
sec-actor-worker = Worker
sec-actor-store = 저장소
sec-actor-you = 나
sec-actor-contact = 연락처
sec-actor-website = 웹사이트
sec-actor-client = 클라이언트

sec-signin-eyebrow = 로그인과 가입
sec-signin-title = Central은 비밀번호를 받지 않고 확인해요
sec-signin-lead = Baylee는 OPAQUE(RFC 9807)로 로그인해요. 내 기기와 Central은 메시지 세 개를 주고받으며, 비밀번호는 그 어디에도 들어 있지 않아요. 평범한 해시로도 들어 있지 않아요.
sec-signin-c1 = 가려진 요청
sec-signin-t1 = 내 기기가 비밀번호를 가려요.
sec-signin-s1 = 비밀번호를 새 난수와 섞어 그 결과만 보내요. 그 메시지에서 비밀번호를 읽어 낼 수 없어요.
sec-signin-c2 = Central의 응답
sec-signin-t2 = Central이 자기만의 키로 응답해요.
sec-signin-s2 = Central은 자신만 가진 키로 가려진 값을 계산하고, 키 교환의 자기 몫을 덧붙여요. 이 과정에서 비밀번호를 알게 되지 않아요.
sec-signin-c3 = Argon2id · 64 MiB
sec-signin-t3 = 무거운 계산은 내 기기가 해요.
sec-signin-s3 = 가림을 풀고 그 결과를 Argon2id로 늘려요(메모리 64 MiB, 세 번 반복). Central은 더 가벼운 설정을 요구할 수 없어요. 비밀번호가 틀렸다면 다른 무언가를 보내기 전에 여기서 알게 돼요.
sec-signin-c4 = 증명
sec-signin-t4 = 내 기기가 올바른 키를 가졌음을 증명해요.
sec-signin-s4 = 비밀번호가 맞으면 내 기기는 Central과 같은 세션 키에 이르고, 그 사실을 보여 주는 짧은 증명(64바이트)을 보내요.
sec-signin-c5 = 세션 + 래퍼
sec-signin-t5 = Central이 세션을 열어요.
sec-signin-s5 = 돌아오는 것은 세션(브라우저에서는 페이지 자신이 읽을 수 없는 쿠키)과, 올바른 비밀번호를 가진 내 기기에서만 열리는 래퍼에 담긴 마스터 키예요.
sec-signin-register = <b>가입</b>도 같은 프로토콜로 이루어져요. 내 기기가 임의의 마스터 키(256비트)를 만들어 비밀번호로 한 번, 복구 코드로 한 번 감싸고, 래퍼만 보내요. Central은 OPAQUE 기록과 래퍼를 저장하며, 키는 저장하지 않아요.
sec-signin-legacy = <b>한 가지 예외:</b> 아직 Central로 옮겨지지 않은 예전 공식 Gateway의 계정이에요. 이런 계정이 옮겨질 때까지는 Baylee 클라이언트에서 로그인을 시도할 때마다 입력한 비밀번호가 확인된 TLS 연결을 통해 Central로 전송돼요. Central은 그것을 예전 해시와 대조할 뿐 저장도 로그 기록도 하지 않아요. 이전이 성공하면 그 계정은 위에서 본 대로 로그인해요. 이 웹사이트는 그런 계정을 로그인시키지 않고 클라이언트로 안내해요. 이 예외는 이전을 위해 정한 기한에 끝나요.
sec-signin-limit = <b>남는 것:</b> Central의 데이터베이스를 그 키와 함께 가져간 사람은 비밀번호를 하나씩 시도해 볼 수 있어요. 시도할 때마다 Argon2id를 한 번 실행해야 해요. 그렇게 찾아낸 비밀번호는 계정의 암호화된 데이터와 저장된 채팅을 열어요. 다른 곳에서 쓰지 않는 긴 비밀번호는 여전히 중요해요.
sec-signin-guess = <b>로그인을 통한 추측</b>은 느려질 뿐, 멈추지는 않아요. Central은 올바른 비밀번호로 끝나지 않은 시도를 세요. 한 계정에 대해서는 그 계정이 로그인한 적 없는 주소에서 하루 최대 100번, 한 주소에서는 모든 계정을 합쳐 하루 최대 100번이에요. 이를 넘으면 시도가 너무 많았다고 응답하며, 그 시간이나 그날이 끝날 때까지 이어져요. 흔한 비밀번호는 이렇게 해도 찾아내요. 그리고 여러분의 로그인 이름을 아는 사람은 여러분 계정의 시도 횟수를 다 써 버릴 수 있어요. 그러면 그날이 끝날 때까지는 계정이 지난 30일 안에 로그인한 적 있는 주소에서만 로그인할 수 있어요(Central은 이 주소들을 다시 시작할 때까지만 기억해요).

sec-memory-eyebrow = 메모리 속
sec-memory-title = 로그인한 뒤에도 메모리에 남는 것
sec-memory-lead = 비밀번호는 마스터 키를 여는 데 한 번 필요해요. 도표는 브라우저에서 비밀번호가 거치는 길을 따라가요. Baylee 클라이언트가 다르게 하는 점은 아래에 있어요.
sec-memory-c1 = 비밀번호
sec-memory-t1 = 페이지가 비밀번호를 Worker에 넘겨요.
sec-memory-s1 = 양식은 비밀번호를 별도의 백그라운드 스레드(Worker)에 넘기고, 입력란을 비우고, 자기 사본을 해제해요.
sec-memory-c2 = OPAQUE + Argon2id
sec-memory-t2 = Worker가 계산한 뒤 덮어써요.
sec-memory-s2 = Worker의 Rust 코드는 OPAQUE 단계가 끝날 때까지만 비밀번호를 가지고 있다가 덮어써요. 그 과정에서 파생해 스스로 보유한 키와 Argon2id의 작업 메모리 64 MiB도 함께 덮어써요.
sec-memory-c3 = 마스터 키: 확인 후 삭제
sec-memory-t3 = 마스터 키는 열리고, 쓰이고, 지워져요.
sec-memory-s3 = Worker는 로그인을 확인하려고 마스터 키를 열고, 계정에 채팅이 있으면 거기서 채팅 키를 파생한 다음, 다시 덮어써요.
sec-memory-c4 = 채팅 키, 봉인됨
sec-memory-t4 = 채팅 키만 남아요.
sec-memory-s4 = 새로고침한 뒤에도 비밀번호 없이 채팅이 열리도록, Worker는 채팅 키를 AES-256-GCM으로 암호화해 브라우저(IndexedDB)에 저장해요. 그 키는 추출할 수 없어요. 스크립트가 쓸 수는 있지만 읽어 낼 수는 없어요. 채팅 페이지가 열려 있는 동안에는 별도의 Worker가 채팅 키를 봉인되지 않은 상태로 메모리에 가지고 있어요. 로그아웃하면 그 기록이 삭제돼요. 세션이 저절로 끝난 경우에는 이 사이트의 다음 페이지가 로드될 때 삭제해요.
sec-memory-c5 = Worker 종료
sec-memory-t5 = 로그인의 Worker가 종료돼요.
sec-memory-s5 = 로그인이 끝나면 로그인 페이지가 Worker를 종료하고, 브라우저가 그 Worker의 메모리를 버려요.
sec-memory-limit = <b>브라우저가 약속하지 않는 것:</b> 입력된 그대로의 비밀번호, 그것을 Worker에 넘기는 과정, JavaScript의 문자열은 브라우저가 관리해요. Baylee는 이것들을 해제하지만, 페이지 자신의 사본을 덮어쓰지는 않으며 브라우저의 사본은 덮어쓸 수 없어요. 그 메모리가 실제로 다시 쓰이는 때는 브라우저가 정해요. 그리고 로그인해 있는 동안 이 사이트에서 스크립트를 실행할 수 있는 사람이나 브라우저 프로필을 읽을 수 있는 사람은 봉인된 채팅 키에 접근할 수 있어요.
sec-memory-client = <b>Baylee 클라이언트</b>에서는 입력한 비밀번호가 해제될 때 덮어써지는 메모리에 있고, 로그인 직후 버려져요. 저장되지는 않아요. 마스터 키는 로그인해 있는 동안 클라이언트의 메모리에 남아요. 클라이언트가 그 키로 암호화된 데이터를 열기 때문이에요.

sec-chat-eyebrow = 채팅
sec-chat-title = 메시지는 내 기기에서 암호화돼요
sec-chat-lead = 메시지마다 자기만의 키가 있어요. Central은 암호화된 것만 저장하고 전달해요. 읽으려면 그 메시지를 감싸 준 계정의 채팅 키가 필요해요.
sec-chat-c1 = 텍스트 + 일회용 키
sec-chat-t1 = 메시지마다 새 키.
sec-chat-s1 = 내 기기가 임의의 메시지 키를 만들어 그 키로 텍스트를 암호화해요(XChaCha20-Poly1305).
sec-chat-c2 = 채팅 키별 래핑, 서명됨
sec-chat-t2 = 수신자의 채팅 키마다 감싸고 내 키로 서명해요.
sec-chat-s2 = 수신자 계정에 등록된 채팅 키마다 메시지 키를 따로 감싸고(X25519), 헤더와 암호문에 내 채팅 키로 서명해요(Ed25519). 지금은 계정에 채팅 키가 하나이며, 마스터 키에서 파생되고 로그인한 모든 브라우저와 클라이언트에서 같아요.
sec-chat-c3 = 암호문 + 래핑
sec-chat-t3 = Central은 암호문만 받아요.
sec-chat-s3 = Central은 서명과 수신자 목록을 확인하고, 암호문을 래핑과 함께 저장해요. 메시지 키도, 내 계정의 키도 갖고 있지 않아요.
sec-chat-c4 = 암호문 + 래핑 하나
sec-chat-t4 = 채팅 키마다 자기 래핑이 있어요.
sec-chat-s4 = 연락처가 메시지를 가져가면 Central은 암호문과, 그 채팅 키를 위해 만든 래핑만 넘겨 줘요.
sec-chat-c5 = 확인 후 열림
sec-chat-t5 = 확인, 풀기, 읽기.
sec-chat-s5 = 연락처의 기기가 내 서명을 확인하고, 자기 채팅 키로 메시지 키를 풀어 텍스트를 복호화해요.
sec-chat-meta = <b>그래도 Central이 보는 것:</b> 누가 누구와 연락하는지, 이름과 구성원이 있는 그룹, 표시 이름, 공개 채팅 키, 그리고 메시지마다 누가 보냈는지, 어느 대화인지, 언제인지, 크기가 얼마인지예요. 다른 서버와 마찬가지로 요청이 오는 주소도 봐요.
sec-chat-limit = <b>한계:</b> 더블 래칫이 없으므로 순방향 비밀성도 없어요. 수신자의 채팅 키를 나중에 도난당하면 그 키로 감싼 이전 메시지도 열려요. 내 채팅 키는 계정의 마스터 키에서 파생되고, Central은 전달 뒤에도 암호화된 메시지를 보관해요. Central의 데이터베이스를 그 키와 함께 가져가 내 비밀번호를 알아맞히거나 내 복구 코드를 가진 사람은 그 메시지를 읽을 수 있어요. 처음 연락할 때 내 기기는 Central이 알려 주는 키를 믿어요. 확인하려면 다른 경로로 안전 번호를 비교하세요. 연락처의 키가 나중에 바뀌면, Baylee는 내가 그 키를 확인한 뒤에야 다시 보내요. 누가 그룹에 속하는지는 Central이 정해요.

sec-pair-eyebrow = 링크로 로그인
sec-pair-title = 브라우저에서 클라이언트로, 비밀번호를 다시 입력하지 않고
sec-pair-lead = 웹사이트에 로그인한 사람이 로그인된 상태로 Baylee 클라이언트를 시작할 수 있어야 해요. 그러려면 페이지가 마스터 키를 클라이언트에 전달해요. 봉인한 채로, 그리고 Central이 그것을 열 수도, 자기 것을 끼워 넣을 수도 없는 방식으로요.
sec-pair-status = <b>현재 상태:</b> Central과 Baylee의 클라이언트 라이브러리는 지금 이 일을 할 수 있고, 서로 테스트했어요. 아직 이 웹사이트나 배포된 클라이언트에서는 쓸 수 없어요.
sec-pair-c1 = 페어링 시작
sec-pair-t1 = 페이지가 페어링을 시작해요.
sec-pair-s1 = Central이 5분 동안 유효한 핸들을 발급해요. 동시에 페이지는 Central에 절대 보여 주지 않는 일회용 비밀값을 만들어요.
sec-pair-c2 = 비밀값이 든 링크
sec-pair-t2 = 링크가 클라이언트를 열어요.
sec-pair-s2 = <code>baylee://pair</code> 링크가 핸들과 비밀값을 Central을 거치지 않고 페이지에서 내 기기의 클라이언트로 전달해요.
sec-pair-c3 = 공개 키
sec-pair-t3 = 클라이언트가 새 키 쌍으로 응답해요.
sec-pair-s3 = 클라이언트는 공개 키 절반과, 비밀값으로 계산한 확인 값(HMAC-SHA-256)을 Central에 보내요. 페이지가 그 값을 검증하고, 두 화면에 같은 짧은 코드가 표시되며, 페이지에서 승인해요.
sec-pair-c4 = 마스터 키, 봉인됨
sec-pair-t4 = 페이지가 마스터 키를 클라이언트용으로 봉인해요.
sec-pair-s4 = HPKE(RFC 9180)로, 클라이언트의 공개 키에 대해, 비밀값까지 함께 써서 봉인해요. Central은 봉인된 것을 한 번 저장해요. 그것을 열 수 없고, 비밀값을 모르므로 자기 것을 만들 수도 없어요.
sec-pair-c5 = 세션 + 봉인
sec-pair-t5 = 클라이언트가 한 번 받아 가요.
sec-pair-s5 = 클라이언트는 봉인과 함께 세션을 받고, 자기 개인 키와 비밀값으로 봉인을 열어 로그인돼요. 페어링은 그 뒤에 삭제돼요. 받아 가기가 실패한 경우에도 삭제돼요.
sec-pair-note = <b>비밀번호는 이 과정에 관여하지 않아요.</b> 페이지가 마스터 키를 가지는 것은 방금 내 비밀번호로 열었을 때뿐이에요. 이렇게 페어링한 세션은 그 세션이 나온 페이지의 비밀번호 로그인보다 새롭지 않은 것으로 쳐요. 페어링이 세션을 갱신하는 일은 없어요.

sec-limits-eyebrow = 한계
sec-limits-title = 이 모든 것으로도 할 수 없는 일
sec-limit-device = <b>침해된 기기.</b> 악성코드, 페이지를 읽는 브라우저 확장 프로그램, 변조된 클라이언트는 내가 보고 입력하는 것을 봐요. 서버로 가는 길의 암호화는 이런 경우에 도움이 되지 않아요.
sec-limit-site = <b>웹사이트의 코드는 그 서버에서 와요.</b> 우리 서버를 장악한 사람은 비밀번호를 입력하는 대로 가로채는 코드를 전달할 수 있어요. 무결성 해시와 엄격한 Content Security Policy는 외부로부터의 변경을 막을 뿐, 이런 경우를 막지는 못해요. 클라이언트는 내 컴퓨터에서 실행되는 프로그램이며, 업데이트를 설치하기 전에 서명을 확인해요.
sec-limit-central = <b>장악된 Central.</b> 메타데이터를 드러내고, 서비스를 거부하고, 로그인을 공격할 수 있어요. 처음 연락할 때 가짜 키를 알려 주거나 그룹에 구성원을 추가해서, 그 뒤에 보낸 메시지를 읽을 수도 있어요. 내 데이터와 채팅 중 Central이 저장한 것은 암호화되어 있어요. 그것을 열려면 내 비밀번호나 복구 코드가 필요해요.
sec-limit-recovery = <b>잃어버린 키.</b> 비밀번호도 복구 코드도 없으면 아무도 암호화된 데이터를 되살릴 수 없어요. 우리도 마찬가지예요. Central에는 래퍼만 있어요.
sec-limit-audit = <b>독립적인 감사 없음.</b> 로그인은 Argon2id(RFC 9106)를 쓰는 OPAQUE(RFC 9807)를 따르고, 페어링은 HPKE(RFC 9180)로 봉인해요. 이들은 IETF 표준이 아니라 IRTF가 공개한 사양이에요. 페어링 단계의 순서와 채팅 형식은 표준 구성 요소(Ed25519, X25519, XChaCha20-Poly1305)를 Baylee가 직접 조합한 것이에요. 지금까지 구현을 검토한 사람은 만든 사람들뿐이에요. 코드는 공개되어 있어요. 무언가를 발견하면 <contact>알려 주세요</contact>.
sec-docs-title = 더 읽어 보기
sec-docs = 자세한 내용은 프로젝트 문서에 있어요. <protocol>암호 프로토콜</protocol>, <chat>채팅</chat>, <accounts>계정과 페어링</accounts>, <threats>위협 모델</threats>이에요. 내 데이터가 법적으로 어떻게 다뤄지는지는 <privacy>개인정보 처리방침</privacy>에 있어요.

# The page's two links from elsewhere: the FAQ and the sign-in page.
faq-security-q = Baylee는 내 비밀번호와 채팅을 어떻게 다루나요?
faq-security-a = 로그인할 때 내 기기는 OPAQUE 프로토콜로 비밀번호를 보내지 않고 알고 있음을 증명해요. 채팅 메시지는 내 기기에서 암호화돼요. <security>보안 페이지</security>에서 각 단계를 도표로 보여 주며, 한 가지 예외와 한계도 함께 알려 드려요.
sec-signin-link = 로그인할 때 비밀번호는 어떻게 되나요: <security>보안</security>
sec-teaser-lead = 단계별로 보여 드려요. 내 기기를 떠나는 것, Baylee Central이 보는 것과 보지 못하는 것, 그리고 각 흐름의 한계예요.
sec-teaser-more = 각 단계를 도표로 보기
# ==== end of the security page =================================================
# ==== registration: the proof of work, while it takes long =====================
register-solving = 브라우저가 로봇을 막는 퍼즐을 풀고 있어요. 많은 계정이 만들어지고 있을 때는 몇 분 걸릴 수 있어요 … { $percent } %
