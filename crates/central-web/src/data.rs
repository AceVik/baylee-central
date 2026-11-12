//! The public data the pages show, read by the server (GitHub, Central's
//! directory) and handed to the views: what a release offers, which file
//! suits the visitor's machine, the card status, the gateways.

use serde::{Deserialize, Serialize};

/// A platform Baylee's client is built for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Platform {
    /// Windows on x86-64.
    WindowsX64,
    /// Windows on ARM64.
    WindowsArm64,
    /// macOS on Apple silicon.
    MacArm64,
    /// macOS on an Intel processor.
    MacX64,
    /// Linux on x86-64.
    LinuxX64,
    /// Linux on ARM64.
    LinuxArm64,
}

impl Platform {
    /// Every platform, in the order the download page lists them.
    pub const ALL: [Platform; 6] = [
        Platform::WindowsX64,
        Platform::WindowsArm64,
        Platform::MacArm64,
        Platform::MacX64,
        Platform::LinuxX64,
        Platform::LinuxArm64,
    ];

    /// A stable identifier for anchors and tests: `windows-x64`, `mac-arm64`.
    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Platform::WindowsX64 => "windows-x64",
            Platform::WindowsArm64 => "windows-arm64",
            Platform::MacArm64 => "mac-arm64",
            Platform::MacX64 => "mac-x64",
            Platform::LinuxX64 => "linux-x64",
            Platform::LinuxArm64 => "linux-arm64",
        }
    }

    /// The operating system's family.
    #[must_use]
    pub const fn os(self) -> Os {
        match self {
            Platform::WindowsX64 | Platform::WindowsArm64 => Os::Windows,
            Platform::MacArm64 | Platform::MacX64 => Os::Mac,
            Platform::LinuxX64 | Platform::LinuxArm64 => Os::Linux,
        }
    }
}

/// An operating system family.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Os {
    /// Windows.
    Windows,
    /// macOS.
    Mac,
    /// Linux.
    Linux,
}

/// What kind of file a release asset is, in the order offered.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Kind {
    /// An installer (`.exe`, `.dmg`, `.AppImage`).
    Installer,
    /// A Debian package.
    Deb,
    /// An archive to unpack (`.zip`, `.tar.gz`).
    Archive,
    /// An archive built for processors without AVX2 (`-v2`), Baylee's
    /// fallback for older x86-64 CPUs (Baylee `docs/releasing.md`).
    Fallback,
}

/// Which platform and kind an asset's file name is, by Baylee's release
/// names (Baylee `docs/releasing.md` §"Installers", `release.rs`); anything
/// else, checksums and signatures included, is `None`.
#[must_use]
pub fn classify(name: &str) -> Option<(Platform, Kind)> {
    let base = |prefix: &str, suffix: &str| {
        name.strip_prefix(prefix)
            .and_then(|rest| rest.strip_suffix(suffix))
            .is_some_and(|middle| !middle.is_empty() && middle.bytes().all(version_byte))
    };
    let rules: [(&str, &str, Platform, Kind); 16] = [
        (
            "Baylee-Setup-",
            "-x64.exe",
            Platform::WindowsX64,
            Kind::Installer,
        ),
        (
            "Baylee-Setup-",
            "-arm64.exe",
            Platform::WindowsArm64,
            Kind::Installer,
        ),
        (
            "Baylee-",
            "-aarch64.dmg",
            Platform::MacArm64,
            Kind::Installer,
        ),
        ("Baylee-", "-x86_64.dmg", Platform::MacX64, Kind::Installer),
        (
            "Baylee-",
            "-x86_64.AppImage",
            Platform::LinuxX64,
            Kind::Installer,
        ),
        (
            "Baylee-",
            "-aarch64.AppImage",
            Platform::LinuxArm64,
            Kind::Installer,
        ),
        ("baylee_", "_amd64.deb", Platform::LinuxX64, Kind::Deb),
        ("baylee_", "_arm64.deb", Platform::LinuxArm64, Kind::Deb),
        (
            "baylee-client-",
            "-x86_64-pc-windows-msvc-v2.zip",
            Platform::WindowsX64,
            Kind::Fallback,
        ),
        (
            "baylee-client-",
            "-x86_64-apple-darwin-v2.zip",
            Platform::MacX64,
            Kind::Fallback,
        ),
        (
            "baylee-client-",
            "-x86_64-unknown-linux-gnu-v2.tar.gz",
            Platform::LinuxX64,
            Kind::Fallback,
        ),
        (
            "baylee-client-",
            "-x86_64-pc-windows-msvc.zip",
            Platform::WindowsX64,
            Kind::Archive,
        ),
        (
            "baylee-client-",
            "-aarch64-pc-windows-msvc.zip",
            Platform::WindowsArm64,
            Kind::Archive,
        ),
        (
            "baylee-client-",
            "-aarch64-apple-darwin.zip",
            Platform::MacArm64,
            Kind::Archive,
        ),
        (
            "baylee-client-",
            "-x86_64-apple-darwin.zip",
            Platform::MacX64,
            Kind::Archive,
        ),
        (
            "baylee-client-",
            "-x86_64-unknown-linux-gnu.tar.gz",
            Platform::LinuxX64,
            Kind::Archive,
        ),
    ];
    if base("baylee-client-", "-aarch64-unknown-linux-gnu.tar.gz") {
        return Some((Platform::LinuxArm64, Kind::Archive));
    }
    rules
        .into_iter()
        .find(|(prefix, suffix, _, _)| base(prefix, suffix))
        .map(|(_, _, platform, kind)| (platform, kind))
}

/// A byte of a `SemVer` version as Baylee names its files.
fn version_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'+')
}

/// One file of a release.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseFile {
    /// The file name.
    pub name: String,
    /// Its platform.
    pub platform: Platform,
    /// Its kind.
    pub kind: Kind,
    /// Its size in bytes.
    pub size: u64,
    /// GitHub's download address (`https://github.com/…`).
    pub url: String,
    /// The `.sha256` beside it, if the release has one.
    pub checksum: Option<String>,
    /// The `.sig` beside it, if the release has one.
    pub signature: Option<String>,
}

/// The newest release of Baylee's client, as the download page shows it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Release {
    /// The tag, `v0.1.1-beta.9b`.
    pub tag: String,
    /// The release page on GitHub.
    pub page: String,
    /// When it was published, `YYYY-MM-DD`.
    pub date: String,
    /// Whether GitHub marks it a pre-release (every beta is one).
    pub prerelease: bool,
    /// Its client files, by platform and kind.
    pub files: Vec<ReleaseFile>,
}

impl Release {
    /// The file to offer first for `platform`: an installer, then a Debian
    /// package, then an archive; a fallback build only when nothing else is.
    #[must_use]
    pub fn best(&self, platform: Platform) -> Option<&ReleaseFile> {
        self.files
            .iter()
            .filter(|file| file.platform == platform)
            .min_by_key(|file| file.kind)
    }

    /// Every file for `platform`, in the order offered.
    #[must_use]
    pub fn for_platform(&self, platform: Platform) -> Vec<&ReleaseFile> {
        let mut files: Vec<&ReleaseFile> = self
            .files
            .iter()
            .filter(|file| file.platform == platform)
            .collect();
        files.sort_by_key(|file| file.kind);
        files
    }
}

/// What the request says about the visitor's machine: the user agent,
/// corrected by client hints where the browser sends them. What cannot be
/// told is said, never guessed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Machine {
    /// The platform to offer first, if one can be named.
    pub platform: Option<Platform>,
    /// The architecture was read (client hints), not assumed.
    pub sure: bool,
    /// A phone or tablet: the client does not run there.
    pub mobile: bool,
}

/// The machine from a request's `User-Agent` and its client hints
/// (`Sec-CH-UA-Platform`, `Sec-CH-UA-Arch`, `Sec-CH-UA-Bitness`), each as
/// sent (hints are quoted strings).
#[must_use]
pub fn machine(
    agent: &str,
    platform: Option<&str>,
    arch: Option<&str>,
    bitness: Option<&str>,
) -> Machine {
    let unquote =
        |value: Option<&str>| value.map(|v| v.trim().trim_matches('"').to_ascii_lowercase());
    let (hint_os, hint_arch, hint_bits) = (unquote(platform), unquote(arch), unquote(bitness));
    let ua = agent.to_ascii_lowercase();
    let mobile = ua.contains("android")
        || ua.contains("iphone")
        || ua.contains("ipad")
        || ua.contains("ipod");
    let os = match hint_os.as_deref() {
        Some("windows") => Some(Os::Windows),
        Some("macos") => Some(Os::Mac),
        Some("linux") => Some(Os::Linux),
        Some("android" | "ios" | "chrome os" | "chromeos") => None,
        _ if mobile || ua.contains("cros") => None,
        _ if ua.contains("windows") => Some(Os::Windows),
        _ if ua.contains("mac os x") || ua.contains("macintosh") => Some(Os::Mac),
        _ if ua.contains("linux") || ua.contains("x11") => Some(Os::Linux),
        _ => None,
    };
    let hinted = match (hint_arch.as_deref(), hint_bits.as_deref()) {
        (Some("arm"), _) => Some(true),
        (Some("x86"), Some("64")) => Some(false),
        _ => None,
    };
    let ua_arm = ua.contains("arm64") || ua.contains("aarch64");
    let mobile = mobile || matches!(hint_os.as_deref(), Some("android" | "ios"));
    let (platform, sure) = match (os, hinted) {
        (Some(Os::Windows), Some(arm)) => (
            Some(if arm {
                Platform::WindowsArm64
            } else {
                Platform::WindowsX64
            }),
            true,
        ),
        (Some(Os::Windows), None) => (
            Some(if ua_arm {
                Platform::WindowsArm64
            } else {
                Platform::WindowsX64
            }),
            false,
        ),
        (Some(Os::Mac), Some(arm)) => (
            Some(if arm {
                Platform::MacArm64
            } else {
                Platform::MacX64
            }),
            true,
        ),
        // Safari and Firefox say "Intel Mac OS X" on every Mac.
        (Some(Os::Mac), None) => (Some(Platform::MacArm64), false),
        (Some(Os::Linux), Some(arm)) => (
            Some(if arm {
                Platform::LinuxArm64
            } else {
                Platform::LinuxX64
            }),
            true,
        ),
        (Some(Os::Linux), None) => (
            Some(if ua_arm {
                Platform::LinuxArm64
            } else {
                Platform::LinuxX64
            }),
            ua.contains("x86_64") || ua_arm,
        ),
        (None, _) => (None, false),
    };
    Machine {
        platform: if mobile { None } else { platform },
        sure,
        mobile,
    }
}

/// The card status of the newest release (Baylee's `sets.json` totals).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardStats {
    /// Cards implemented and tested.
    pub implemented: u64,
    /// Cards implemented by the generator, not yet tested.
    pub generated: u64,
    /// Cards that work in part.
    pub partial: u64,
    /// Every card there is (the ledger's rows).
    pub total: u64,
}

/// Where the card status was read and of when.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardStatus {
    /// The numbers.
    pub stats: CardStats,
    /// The release tag the numbers belong to.
    pub tag: String,
    /// When Baylee's tool wrote them, `YYYY-MM-DD`.
    pub date: String,
}

/// A gateway of Central's public directory, checked as `central-client`
/// checks it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Gateway {
    /// Its canonical origin.
    pub origin: String,
    /// The name its operator gave it.
    pub name: String,
    /// The version it reports.
    pub version: String,
    /// Its website, on its own origin.
    pub website: String,
    /// A heartbeat arrived within the last 90 seconds.
    pub online: bool,
    /// The origin Central's API calls official (`is_official_origin`):
    /// the server lists it first, and the pages do not mark it (owner,
    /// 12.11.2026: there are no official gateways).
    pub official: bool,
    /// What it says is happening on it right now, if it reports that
    /// (Central's public figures): absent for a gateway that does not.
    #[serde(default)]
    pub activity: Option<Activity>,
}

/// A gateway's own counts as Central passes them on: its claim, which
/// nobody can check, each bounded by Central (`GatewayActivity::MOST`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Activity {
    /// Players connected.
    pub players: u32,
    /// Of those, the players seated in a running game.
    pub playing: u32,
    /// Tables open to join.
    pub tables: u32,
    /// Games running.
    pub games: u32,
}

/// The gateways' reports added up, for the start page's strip: players
/// connected, those of them in a game, open tables and games, in that
/// order. `None` while no online gateway reports anything (a number nobody
/// reported is not shown as zero).
#[must_use]
pub fn activity_of(gateways: &[Gateway]) -> Option<(u64, u64, u64, u64)> {
    let mut reported = gateways
        .iter()
        .filter(|gateway| gateway.online)
        .filter_map(|gateway| gateway.activity)
        .peekable();
    reported.peek()?;
    Some(reported.fold((0, 0, 0, 0), |sum, activity| {
        (
            sum.0 + u64::from(activity.players),
            sum.1 + u64::from(activity.playing),
            sum.2 + u64::from(activity.tables),
            sum.3 + u64::from(activity.games),
        )
    }))
}

/// How often the site's pages were opened: one number for all of them and
/// every visitor together, and the day the count began.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Views {
    /// The pages opened since `since`.
    pub count: u64,
    /// The day the count began, `YYYY-MM-DD`.
    pub since: String,
}

/// A page of the signed-in area (`/{lang}/account/…`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Section {
    /// `/account`: the handle and what needs the person.
    Home,
    /// `/account/chat`: the messenger's conversations.
    Chat,
    /// `/account/contacts`: the messenger's contacts.
    Contacts,
    /// `/account/profile`: the display name and the recovery code.
    Profile,
    /// `/account/security`: the sessions, signing out.
    Security,
    /// `/account/privacy`: the export and the deletion.
    Privacy,
    /// `/account/gateways`: registered gateways and their tokens.
    Gateways,
    /// `/account/notifications`: Web Push on this device.
    Notifications,
    /// `/account/admin`: an administrator makes a reset code for an
    /// account (`docs/account-reset.md`). In no navigation: the overview
    /// leads an administrator to it, and it shows nothing to anyone else.
    Admin,
    /// `/account/admin/overview`: the administration's counts, each leading
    /// to the page where the work is.
    AdminOverview,
    /// `/account/admin/users`: every account, newest first, and a search.
    AdminUsers,
    /// `/account/admin/users/<its id in hex>`: one account and its changes.
    /// The only address with a part the visitor chooses: [`Section::slug`]
    /// gives its list's, and `Page::path` adds the id.
    AdminUser([u8; 16]),
    /// `/account/admin/gateways`: the registered gateways, their approval
    /// and their tokens.
    AdminGateways,
    /// `/account/admin/audit`: what administrators and the operator did.
    AdminAudit,
    /// `/account/admin/legacy`: what is left of the legacy store.
    AdminLegacy,
    /// `/account/admin/pictures`: the reported profile pictures, an
    /// account's case and their moderation (`docs/profile-pictures.md`).
    AdminPictures,
    /// `/account/admin/feedback`: bug and crash reports, triaged
    /// (`docs/reports.md`). As `Admin`: in no navigation, and nothing for
    /// anyone without the role.
    Feedback,
    /// `/account/admin/feedback/<id in hex>`: one report. Every report's
    /// address is this one page (`Page::from_slug`); the island reads the
    /// id from the address, so neither the page nor its cache holds one.
    /// Its own path is the list's.
    Report,
}

impl Section {
    /// Every section in the navigation, in its order. The administration
    /// is not among them: the navigation is the same for everyone.
    pub const ALL: [Section; 8] = [
        Section::Home,
        Section::Chat,
        Section::Contacts,
        Section::Profile,
        Section::Security,
        Section::Privacy,
        Section::Notifications,
        Section::Gateways,
    ];

    /// Every section with a fixed address: the navigation's and the
    /// administration's. One account's page (`AdminUser`) has an address
    /// per account and is read by [`Section::account_page`] apart.
    pub const ROUTED: [Section; 17] = [
        Section::Home,
        Section::Chat,
        Section::Contacts,
        Section::Profile,
        Section::Security,
        Section::Privacy,
        Section::Notifications,
        Section::Gateways,
        Section::Admin,
        Section::AdminOverview,
        Section::AdminUsers,
        Section::AdminGateways,
        Section::AdminAudit,
        Section::AdminLegacy,
        Section::AdminPictures,
        Section::Feedback,
        Section::Report,
    ];

    /// The path after `/{lang}/`; one account's page gives its list's.
    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Section::Home => "account",
            Section::Chat => "account/chat",
            Section::Contacts => "account/contacts",
            Section::Profile => "account/profile",
            Section::Security => "account/security",
            Section::Privacy => "account/privacy",
            Section::Gateways => "account/gateways",
            Section::Notifications => "account/notifications",
            Section::Admin => "account/admin",
            Section::AdminOverview => "account/admin/overview",
            Section::AdminUsers | Section::AdminUser(_) => "account/admin/users",
            Section::AdminGateways => "account/admin/gateways",
            Section::AdminAudit => "account/admin/audit",
            Section::AdminLegacy => "account/admin/legacy",
            Section::AdminPictures => "account/admin/pictures",
            Section::Feedback | Section::Report => "account/admin/feedback",
        }
    }

    /// Whether it is a page of the administration: for administrators
    /// only, which the island decides, since the server cannot know.
    #[must_use]
    pub const fn admin(self) -> bool {
        matches!(
            self,
            Section::Admin
                | Section::AdminOverview
                | Section::AdminUsers
                | Section::AdminUser(_)
                | Section::AdminGateways
                | Section::AdminAudit
                | Section::AdminLegacy
                | Section::AdminPictures
        )
    }

    /// One account's page, at `slug` (after `/{lang}/`): the account's id
    /// as 32 lowercase hex digits after the list's path, and nothing else.
    #[must_use]
    pub fn account_page(slug: &str) -> Option<Section> {
        let hex = slug.strip_prefix("account/admin/users/")?;
        id_from_hex(hex).map(Section::AdminUser)
    }
}

/// An id as the addresses write it: 32 lowercase hex digits.
#[must_use]
pub fn id_hex(id: &[u8; 16]) -> String {
    use std::fmt::Write as _;
    id.iter().fold(String::with_capacity(32), |mut out, byte| {
        let _ = write!(out, "{byte:02x}");
        out
    })
}

/// The id that `text` writes as 32 lowercase hex digits, and only then: no
/// other length, case or character, so that one id has one address.
#[must_use]
pub fn id_from_hex(text: &str) -> Option<[u8; 16]> {
    let bytes = text.as_bytes();
    if bytes.len() != 32 || !bytes.iter().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
        return None;
    }
    let mut id = [0_u8; 16];
    for (at, pair) in bytes.as_chunks::<2>().0.iter().enumerate() {
        let digits = std::str::from_utf8(pair).ok()?;
        id[at] = u8::from_str_radix(digits, 16).ok()?;
    }
    Some(id)
}

/// The repository's state for the home page's status strip.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Repo {
    /// The newest run of the CI workflow on `main` passed, failed, or is unknown.
    pub ci: Option<bool>,
    /// The newest commit on `main`: its short id and date.
    pub commit: Option<(String, String)>,
    /// That commit's subject, its first line, at most 120 characters: text.
    #[serde(default)]
    pub subject: Option<String>,
    /// How many stars the repository has.
    #[serde(default)]
    pub stars: Option<u64>,
    /// How many issues are open.
    #[serde(default)]
    pub issues: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One account's page has one address: 32 lowercase hex digits, read
    /// back to the same id; anything else is no page.
    #[test]
    fn an_account_page_has_one_address() {
        let id = [
            0x01, 0x9a, 0x2b, 0x3c, 0x4d, 0x5e, 0x70, 0x80, 0x90, 0xa0, 0xb0, 0xc0, 0xd0, 0xe0,
            0xf0, 0xff,
        ];
        let hex = id_hex(&id);
        assert_eq!(hex, "019a2b3c4d5e708090a0b0c0d0e0f0ff");
        assert_eq!(id_from_hex(&hex), Some(id));
        assert_eq!(
            Section::account_page(&format!("account/admin/users/{hex}")),
            Some(Section::AdminUser(id))
        );
        for bad in [
            String::new(),
            hex.to_uppercase(),
            hex[..31].to_owned(),
            format!("{hex}0"),
            format!("{}g", &hex[..31]),
            format!("+{}", &hex[..31]),
            format!("{hex}/"),
            format!(" {}", &hex[..31]),
        ] {
            assert_eq!(id_from_hex(&bad), None, "{bad}");
            let slug = format!("account/admin/users/{bad}");
            assert_eq!(Section::account_page(&slug), None, "{bad}");
        }
        assert_eq!(
            Section::account_page(&format!("account/admin/user/{hex}")),
            None
        );
        assert_eq!(Section::account_page(&format!("account/users/{hex}")), None);
        // the administration's pages say so; the area's own do not
        let admin = Section::ROUTED.iter().filter(|s| s.admin()).count();
        assert_eq!(admin, 7);
        assert!(Section::AdminUser(id).admin());
        assert!(Section::ALL.iter().all(|s| !s.admin()));
        assert_eq!(Section::AdminUser(id).slug(), Section::AdminUsers.slug());
    }

    #[test]
    fn release_names() {
        let cases = [
            (
                "Baylee-Setup-0.1.1-beta.9b-x64.exe",
                Some((Platform::WindowsX64, Kind::Installer)),
            ),
            (
                "Baylee-0.2.0-aarch64.dmg",
                Some((Platform::MacArm64, Kind::Installer)),
            ),
            (
                "Baylee-0.2.0-x86_64.AppImage",
                Some((Platform::LinuxX64, Kind::Installer)),
            ),
            (
                "baylee_0.2.0-beta.o1_arm64.deb",
                Some((Platform::LinuxArm64, Kind::Deb)),
            ),
            (
                "baylee-client-0.2.0-x86_64-pc-windows-msvc.zip",
                Some((Platform::WindowsX64, Kind::Archive)),
            ),
            (
                "baylee-client-0.2.0-x86_64-pc-windows-msvc-v2.zip",
                Some((Platform::WindowsX64, Kind::Fallback)),
            ),
            (
                "baylee-client-0.2.0-aarch64-unknown-linux-gnu.tar.gz",
                Some((Platform::LinuxArm64, Kind::Archive)),
            ),
            (
                "baylee-client-0.2.0-x86_64-apple-darwin-v2.zip",
                Some((Platform::MacX64, Kind::Fallback)),
            ),
            (
                "baylee-client-0.2.0-x86_64-pc-windows-msvc.zip.sha256",
                None,
            ),
            ("Baylee-Setup--x64.exe", None),
            ("Baylee-Setup-1 2-x64.exe", None),
            ("evil-Baylee-0.2.0-aarch64.dmg", None),
        ];
        for (name, want) in cases {
            assert_eq!(classify(name), want, "{name}");
        }
    }

    #[test]
    fn machines() {
        let mac_safari = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Safari/605.1.15";
        assert_eq!(
            machine(mac_safari, None, None, None),
            Machine {
                platform: Some(Platform::MacArm64),
                sure: false,
                mobile: false
            }
        );
        assert_eq!(
            machine(
                mac_safari,
                Some("\"macOS\""),
                Some("\"x86\""),
                Some("\"64\"")
            )
            .platform,
            Some(Platform::MacX64)
        );
        let win = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0 Safari/537.36";
        assert_eq!(
            machine(win, Some("\"Windows\""), Some("\"arm\""), Some("\"64\"")),
            Machine {
                platform: Some(Platform::WindowsArm64),
                sure: true,
                mobile: false
            }
        );
        assert_eq!(
            machine(win, None, None, None).platform,
            Some(Platform::WindowsX64)
        );
        let android = "Mozilla/5.0 (Linux; Android 15; Pixel 7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0 Mobile Safari/537.36";
        assert_eq!(
            machine(android, None, None, None),
            Machine {
                platform: None,
                sure: false,
                mobile: true
            }
        );
        let linux = "Mozilla/5.0 (X11; Linux x86_64; rv:140.0) Gecko/20100101 Firefox/140.0";
        assert_eq!(
            machine(linux, None, None, None),
            Machine {
                platform: Some(Platform::LinuxX64),
                sure: true,
                mobile: false
            }
        );
    }

    #[test]
    fn best_file_prefers_installers_and_never_a_fallback_first() {
        let file = |name: &str| {
            let (platform, kind) = classify(name).expect("a release name");
            ReleaseFile {
                name: name.into(),
                platform,
                kind,
                size: 1,
                url: String::new(),
                checksum: None,
                signature: None,
            }
        };
        let release = Release {
            tag: "v1".into(),
            page: String::new(),
            date: String::new(),
            prerelease: false,
            files: vec![
                file("baylee-client-1-x86_64-unknown-linux-gnu-v2.tar.gz"),
                file("baylee-client-1-x86_64-unknown-linux-gnu.tar.gz"),
                file("baylee_1_amd64.deb"),
                file("Baylee-1-x86_64.AppImage"),
            ],
        };
        assert_eq!(
            release.best(Platform::LinuxX64).map(|f| f.kind),
            Some(Kind::Installer)
        );
        let kinds: Vec<Kind> = release
            .for_platform(Platform::LinuxX64)
            .iter()
            .map(|f| f.kind)
            .collect();
        assert_eq!(
            kinds,
            [Kind::Installer, Kind::Deb, Kind::Archive, Kind::Fallback]
        );
        assert!(release.best(Platform::MacArm64).is_none());
    }
}
