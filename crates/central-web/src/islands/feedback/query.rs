//! A list of reports as its address holds it
//! (`/{lang}/account/admin/feedback?…`), so that a list is a link, a saved
//! view (`AdminRequest::SaveReportView` keeps the query string) and the way
//! back from a report's page. The query string is the old administration's
//! exactly (the removed React build's `query.ts`): the same keys in the same
//! order, encoded as `URLSearchParams` encodes them, so the views saved
//! there still open and still match. Every part is read as Central would
//! check it (central-api `ReportFilter::validate`): a part out of its
//! bounds is dropped, never sent.

use central_api::reports::{
    Channel, MAX_QUERY, MAX_REPORTER, MAX_SAID, PAGE, Priority, ReportFilter, ReportKind,
    ReportSort, ReportStatus, valid_label,
};

use super::text::{hex, id_of};

/// The page sizes offered; Central takes at most 200.
pub const PAGE_SIZES: [u32; 4] = [25, 50, 100, 200];

/// Not done: what the inbox and "open" hold.
pub const OPEN: [ReportStatus; 3] = [
    ReportStatus::New,
    ReportStatus::Triaged,
    ReportStatus::InProgress,
];

/// Done (`ReportStatus::done`).
pub const DONE: [ReportStatus; 3] = [
    ReportStatus::Resolved,
    ReportStatus::WontFix,
    ReportStatus::Duplicate,
];

/// Every priority, and none after them.
pub const PRIORITIES: [Option<Priority>; 5] = [
    Some(Priority::Critical),
    Some(Priority::High),
    Some(Priority::Normal),
    Some(Priority::Low),
    None,
];

/// Who looks after the reports of a list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Assignee {
    /// Nobody (`assignee=none`).
    Nobody,
    /// This administrator.
    Admin([u8; 16]),
}

/// The list as the address says it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListQuery {
    /// Any of these, in Central's order; none is no filter.
    pub statuses: Vec<ReportStatus>,
    /// Any of these kinds.
    pub kinds: Vec<ReportKind>,
    /// `None` is no priority, after the four.
    pub priorities: Vec<Option<Priority>>,
    /// The search.
    pub q: Option<String>,
    /// From this UTC day.
    pub from: Option<String>,
    /// To this UTC day.
    pub to: Option<String>,
    /// With a record, or without.
    pub record: Option<bool>,
    /// Through a gateway, or direct.
    pub channel: Option<Channel>,
    /// Filed by the gateway of this name.
    pub gateway: Option<String>,
    /// Looked after by.
    pub assignee: Option<Assignee>,
    /// With this label.
    pub label: Option<String>,
    /// From this reporter.
    pub reporter: Option<String>,
    /// In this crash group.
    pub signature: Option<String>,
    /// Duplicates of this report.
    pub duplicate_of: Option<[u8; 16]>,
    /// In the trash.
    pub trash: bool,
    /// Not opened by the reading administrator.
    pub unseen: bool,
    /// The order.
    pub sort: ReportSort,
    /// Oldest, smallest or first first.
    pub asc: bool,
    /// From 1.
    pub page: u32,
    /// Reports a page.
    pub limit: u32,
}

impl Default for ListQuery {
    fn default() -> Self {
        ListQuery {
            statuses: Vec::new(),
            kinds: Vec::new(),
            priorities: Vec::new(),
            q: None,
            from: None,
            to: None,
            record: None,
            channel: None,
            gateway: None,
            assignee: None,
            label: None,
            reporter: None,
            signature: None,
            duplicate_of: None,
            trash: false,
            unseen: false,
            sort: ReportSort::Received,
            asc: false,
            page: 1,
            limit: PAGE,
        }
    }
}

/// The views above the list: each sets the statuses, the assignee and the
/// trash, and leaves the rest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum View {
    /// New reports.
    Inbox,
    /// Not done.
    Open,
    /// Not done, looked after by the reader.
    Mine,
    /// Not done, looked after by nobody.
    Unassigned,
    /// Done.
    Done,
    /// Every report out of the trash.
    All,
    /// The trash.
    Trash,
}

impl View {
    /// Every view, in its order above the list.
    pub const ALL: [View; 7] = [
        View::Inbox,
        View::Open,
        View::Mine,
        View::Unassigned,
        View::Done,
        View::All,
        View::Trash,
    ];

    /// The parts of a list this view decides, for the administrator `me`.
    #[must_use]
    pub fn query(self, me: [u8; 16]) -> ListQuery {
        let mut q = ListQuery::default();
        match self {
            View::Inbox => q.statuses = vec![ReportStatus::New],
            View::Open => q.statuses = OPEN.to_vec(),
            View::Mine => {
                q.statuses = OPEN.to_vec();
                q.assignee = Some(Assignee::Admin(me));
            }
            View::Unassigned => {
                q.statuses = OPEN.to_vec();
                q.assignee = Some(Assignee::Nobody);
            }
            View::Done => q.statuses = DONE.to_vec(),
            View::All => {}
            View::Trash => q.trash = true,
        }
        q
    }

    /// This view, with `query`'s order and page size kept.
    #[must_use]
    pub fn applied(self, query: &ListQuery, me: [u8; 16]) -> ListQuery {
        let mut next = self.query(me);
        next.sort = query.sort;
        next.asc = query.asc;
        next.limit = query.limit;
        next
    }
}

// ---------------------------------------------------------------- the address

/// `text` as `URLSearchParams` writes a name or a value: letters, digits
/// and `*-._` as they are, a space as `+`, every other byte `%XX`.
#[must_use]
pub fn encode(text: &str) -> String {
    const DIGITS: &[u8; 16] = b"0123456789ABCDEF";
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        match byte {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'*' | b'-' | b'.' | b'_' => {
                out.push(char::from(byte));
            }
            b' ' => out.push('+'),
            _ => {
                out.push('%');
                out.push(char::from(DIGITS[usize::from(byte >> 4)]));
                out.push(char::from(DIGITS[usize::from(byte & 15)]));
            }
        }
    }
    out
}

/// `text` as `URLSearchParams` reads it: `+` a space, `%XX` a byte, a `%`
/// without two hex digits as it is, then UTF-8 with replacement.
#[must_use]
pub fn decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let digit = |i: usize| {
        bytes
            .get(i)
            .and_then(|b| char::from(*b).to_digit(16))
            .and_then(|d| u8::try_from(d).ok())
    };
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        match bytes[at] {
            b'+' => {
                out.push(b' ');
                at += 1;
            }
            b'%' => {
                if let (Some(high), Some(low)) = (digit(at + 1), digit(at + 2)) {
                    out.push(high * 16 + low);
                    at += 3;
                } else {
                    out.push(b'%');
                    at += 1;
                }
            }
            byte => {
                out.push(byte);
                at += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// The pairs of a query string, a leading `?` ignored.
fn pairs(search: &str) -> Vec<(String, String)> {
    search
        .trim_start_matches('?')
        .split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            (decode(key), decode(value))
        })
        .collect()
}

/// Several of `all`, in `all`'s order, from `a,b,c`.
fn several<T: Copy>(all: &[T], word: impl Fn(T) -> &'static str, text: &str) -> Vec<T> {
    let asked: Vec<&str> = text.split(',').collect();
    all.iter()
        .copied()
        .filter(|value| asked.contains(&word(*value)))
        .collect()
}

/// A priority as the address writes it.
#[must_use]
pub fn priority_word(priority: Option<Priority>) -> &'static str {
    priority.map_or("none", Priority::as_str)
}

/// central-api's `invisible`: marks that change how the text around them reads.
const fn invisible(c: char) -> bool {
    matches!(
        c,
        '\u{061C}'
            | '\u{200B}'
            | '\u{200E}'
            | '\u{200F}'
            | '\u{202A}'..='\u{202E}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{2060}'..='\u{2064}'
            | '\u{2066}'..='\u{2069}'
            | '\u{FEFF}'
    )
}

/// central-api's `shown` for one line: within `max` characters, no control
/// and no invisible character.
#[must_use]
pub fn shown(text: &str, max: usize) -> bool {
    text.chars().count() <= max && text.chars().all(|c| !c.is_control() && !invisible(c))
}

/// A search as Central takes it: control and invisible characters made
/// spaces, trimmed, at most 200 characters.
#[must_use]
pub fn clean_query(text: &str) -> Option<String> {
    let plain: String = text
        .chars()
        .map(|c| {
            if c.is_control() || invisible(c) {
                ' '
            } else {
                c
            }
        })
        .collect();
    let cut: String = plain.trim().chars().take(MAX_QUERY).collect();
    let cut = cut.trim();
    (!cut.is_empty() && shown(cut, MAX_QUERY)).then(|| cut.to_owned())
}

/// central-api's `day`: `YYYY-MM-DD`, month 1–12, day 1–31.
#[must_use]
pub fn valid_day(text: &str) -> bool {
    let b = text.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
        && (1..=12).contains(&text[5..7].parse::<u8>().unwrap_or(0))
        && (1..=31).contains(&text[8..10].parse::<u8>().unwrap_or(0))
}

/// A gateway's name as a filter: 1 to 256 bytes of shown text.
#[must_use]
pub fn valid_gateway(name: &str) -> bool {
    !name.is_empty() && name.len() <= MAX_SAID && shown(name, MAX_SAID)
}

/// A reporter's pseudonym: 1 to 128 hex digits.
#[must_use]
pub fn valid_reporter(reporter: &str) -> bool {
    !reporter.is_empty()
        && reporter.len() <= MAX_REPORTER
        && reporter.bytes().all(|b| b.is_ascii_hexdigit())
}

/// A crash group: 32 lowercase hex digits.
#[must_use]
pub fn valid_signature(signature: &str) -> bool {
    signature.len() == 32
        && signature
            .bytes()
            .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// The list `search` (a query string) asks for; anything unreadable is left out.
#[must_use]
pub fn read(search: &str) -> ListQuery {
    let all = pairs(search);
    let get = |key: &str| {
        all.iter()
            .find(|(k, _)| k == key)
            .map(|(_, value)| value.as_str())
    };
    let mut q = ListQuery::default();
    if let Some(text) = get("status") {
        q.statuses = several(ReportStatus::ALL, ReportStatus::as_str, text);
    }
    if let Some(text) = get("kind") {
        q.kinds = several(ReportKind::ALL, ReportKind::as_str, text);
    }
    if let Some(text) = get("priority") {
        q.priorities = several(&PRIORITIES, priority_word, text);
    }
    q.q = clean_query(get("q").unwrap_or(""));
    q.from = get("from").filter(|d| valid_day(d)).map(str::to_owned);
    q.to = get("to").filter(|d| valid_day(d)).map(str::to_owned);
    q.record = match get("record") {
        Some("yes") => Some(true),
        Some("no") => Some(false),
        _ => None,
    };
    q.channel = match get("channel") {
        Some("gateway") => Some(Channel::Gateway),
        Some("direct") => Some(Channel::Direct),
        _ => None,
    };
    q.gateway = get("gateway")
        .filter(|g| valid_gateway(g))
        .map(str::to_owned);
    q.assignee = match get("assignee") {
        Some("none") => Some(Assignee::Nobody),
        Some(text) => id_of(text).map(Assignee::Admin),
        None => None,
    };
    q.label = get("label").filter(|l| valid_label(l)).map(str::to_owned);
    q.reporter = get("reporter")
        .filter(|r| valid_reporter(r))
        .map(str::to_owned);
    q.signature = get("signature")
        .filter(|s| valid_signature(s))
        .map(str::to_owned);
    q.duplicate_of = get("duplicate_of").and_then(id_of);
    q.trash = get("trash") == Some("1");
    q.unseen = get("unseen") == Some("1");
    if let Some(sort) = get("sort").and_then(|s| ReportSort::ALL.iter().find(|x| x.as_str() == s)) {
        q.sort = *sort;
    }
    q.asc = get("dir") == Some("asc");
    if let Some(limit) = get("limit").and_then(|l| l.parse::<u32>().ok())
        && PAGE_SIZES.contains(&limit)
    {
        q.limit = limit;
    }
    if let Some(page) = get("page").and_then(|p| p.parse::<u32>().ok())
        && page > 1
    {
        q.page = page;
    }
    q
}

/// `q` as a query string, its parts in the old administration's fixed
/// order; empty for the whole list.
#[must_use]
pub fn write(q: &ListQuery) -> String {
    let mut parts: Vec<(&str, String)> = Vec::new();
    if q.trash {
        parts.push(("trash", "1".into()));
    }
    let joined = |words: Vec<&str>| (!words.is_empty()).then(|| words.join(","));
    let lists = [
        (
            "status",
            joined(q.statuses.iter().map(|s| s.as_str()).collect()),
        ),
        ("kind", joined(q.kinds.iter().map(|k| k.as_str()).collect())),
        (
            "priority",
            joined(q.priorities.iter().map(|p| priority_word(*p)).collect()),
        ),
        (
            "assignee",
            q.assignee.map(|a| match a {
                Assignee::Nobody => "none".to_owned(),
                Assignee::Admin(id) => hex(&id),
            }),
        ),
        ("label", q.label.clone()),
        ("gateway", q.gateway.clone()),
        (
            "channel",
            q.channel.map(|c| {
                match c {
                    Channel::Gateway => "gateway",
                    Channel::Direct => "direct",
                }
                .to_owned()
            }),
        ),
        ("reporter", q.reporter.clone()),
        ("signature", q.signature.clone()),
        ("duplicate_of", q.duplicate_of.map(|id| hex(&id))),
        ("q", q.q.clone()),
        ("from", q.from.clone()),
        ("to", q.to.clone()),
        (
            "record",
            q.record.map(|r| if r { "yes" } else { "no" }.to_owned()),
        ),
    ];
    for (key, value) in lists {
        if let Some(value) = value.filter(|v| !v.is_empty()) {
            parts.push((key, value));
        }
    }
    if q.unseen {
        parts.push(("unseen", "1".into()));
    }
    if q.sort != ReportSort::Received {
        parts.push(("sort", q.sort.as_str().to_owned()));
    }
    if q.asc {
        parts.push(("dir", "asc".into()));
    }
    if q.limit != PAGE {
        parts.push(("limit", q.limit.to_string()));
    }
    if q.page > 1 {
        parts.push(("page", q.page.to_string()));
    }
    parts
        .into_iter()
        .map(|(key, value)| format!("{}={}", encode(key), encode(&value)))
        .collect::<Vec<_>>()
        .join("&")
}

/// `q` without its page: what a saved view and a report's way back keep.
#[must_use]
pub fn without_page(q: &ListQuery) -> ListQuery {
    ListQuery {
        page: 1,
        ..q.clone()
    }
}

/// `q` after `change`, on its first page, as the address can say it.
#[must_use]
pub fn changed(q: &ListQuery, change: impl FnOnce(&mut ListQuery)) -> ListQuery {
    let mut next = q.clone();
    change(&mut next);
    next.page = 1;
    read(&write(&next))
}

/// `values` with `value` in or out, in `order`.
#[must_use]
pub fn toggled<T: Copy + PartialEq>(values: &[T], value: T, order: &[T]) -> Vec<T> {
    let on = values.contains(&value);
    order
        .iter()
        .copied()
        .filter(|v| if *v == value { !on } else { values.contains(v) })
        .collect()
}

/// The filter Central reads for `q`.
#[must_use]
pub fn filter_of(q: &ListQuery) -> ReportFilter {
    ReportFilter {
        statuses: q.statuses.clone(),
        kinds: q.kinds.clone(),
        priorities: q.priorities.clone(),
        q: q.q.clone(),
        from: q.from.clone(),
        to: q.to.clone(),
        has_record: q.record,
        channel: q.channel,
        gateway: q.gateway.clone(),
        assignee: match q.assignee {
            Some(Assignee::Admin(id)) => Some(id),
            _ => None,
        },
        unassigned: q.assignee == Some(Assignee::Nobody),
        label: q.label.clone(),
        reporter: q.reporter.clone(),
        signature: q.signature.clone(),
        duplicate_of: q.duplicate_of,
        trash: q.trash,
        unseen: q.unseen,
    }
}

/// What the facets and the crash groups count over: `q` without its page,
/// its order and its size.
#[must_use]
pub fn scope(q: &ListQuery) -> ListQuery {
    ListQuery {
        sort: ReportSort::Received,
        asc: false,
        page: 1,
        limit: PAGE,
        ..q.clone()
    }
}

/// The sorts read best from the top down start ascending; the others
/// newest or most first.
#[must_use]
pub const fn starts_ascending(sort: ReportSort) -> bool {
    matches!(
        sort,
        ReportSort::Status | ReportSort::Kind | ReportSort::Reporter | ReportSort::Gateway
    )
}

/// One page of a list as Central is asked for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Asked {
    /// Which.
    pub filter: ReportFilter,
    /// In what order.
    pub sort: ReportSort,
    /// Newest, largest or last first.
    pub descending: bool,
    /// Skipped rows.
    pub offset: u32,
    /// At most.
    pub limit: u32,
}

/// The page of `q`'s list Central is asked for.
#[must_use]
pub fn request_of(q: &ListQuery) -> Asked {
    Asked {
        filter: filter_of(q),
        sort: q.sort,
        descending: !q.asc,
        offset: (q.page.max(1) - 1).saturating_mul(q.limit),
        limit: q.limit,
    }
}

/// The view `q` is in, if any.
#[must_use]
pub fn view_of(q: &ListQuery, me: [u8; 16]) -> Option<View> {
    View::ALL.into_iter().find(|view| {
        let v = view.query(me);
        let same = |a: &[ReportStatus], b: &[ReportStatus]| {
            a.len() == b.len() && a.iter().all(|s| b.contains(s))
        };
        same(&q.statuses, &v.statuses) && q.assignee == v.assignee && q.trash == v.trash
    })
}

/// One active filter beyond the view's own, as a pill that takes it out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pill {
    /// A status.
    Status(ReportStatus),
    /// A kind.
    Kind(ReportKind),
    /// A priority, or none.
    Priority(Option<Priority>),
    /// The assignee.
    Assignee(Assignee),
    /// The label.
    Label(String),
    /// The gateway.
    Gateway(String),
    /// The channel.
    Channel(Channel),
    /// The reporter.
    Reporter(String),
    /// The crash group.
    Signature(String),
    /// The original of duplicates.
    DuplicateOf([u8; 16]),
    /// The search.
    Search(String),
    /// From a day.
    From(String),
    /// To a day.
    To(String),
    /// With a record or without.
    Record(bool),
    /// Only unread.
    Unseen,
    /// In the trash.
    Trash,
}

/// The active filters of `q` beyond what its view decides.
#[must_use]
pub fn pills_of(q: &ListQuery, view: Option<View>) -> Vec<Pill> {
    let owned = view.is_some();
    let mut pills = Vec::new();
    if !owned {
        pills.extend(q.statuses.iter().map(|s| Pill::Status(*s)));
    }
    pills.extend(q.kinds.iter().map(|k| Pill::Kind(*k)));
    pills.extend(q.priorities.iter().map(|p| Pill::Priority(*p)));
    if !owned && let Some(assignee) = q.assignee {
        pills.push(Pill::Assignee(assignee));
    }
    pills.extend(q.label.clone().map(Pill::Label));
    pills.extend(q.gateway.clone().map(Pill::Gateway));
    pills.extend(q.channel.map(Pill::Channel));
    pills.extend(q.reporter.clone().map(Pill::Reporter));
    pills.extend(q.signature.clone().map(Pill::Signature));
    pills.extend(q.duplicate_of.map(Pill::DuplicateOf));
    pills.extend(q.q.clone().map(Pill::Search));
    pills.extend(q.from.clone().map(Pill::From));
    pills.extend(q.to.clone().map(Pill::To));
    pills.extend(q.record.map(Pill::Record));
    if q.unseen {
        pills.push(Pill::Unseen);
    }
    if !owned && q.trash {
        pills.push(Pill::Trash);
    }
    pills
}

/// `q` without the filter `pill`, on its first page.
#[must_use]
pub fn without(q: &ListQuery, pill: &Pill) -> ListQuery {
    changed(q, |n| match pill {
        Pill::Status(s) => n.statuses.retain(|x| x != s),
        Pill::Kind(k) => n.kinds.retain(|x| x != k),
        Pill::Priority(p) => n.priorities.retain(|x| x != p),
        Pill::Assignee(_) => n.assignee = None,
        Pill::Label(_) => n.label = None,
        Pill::Gateway(_) => n.gateway = None,
        Pill::Channel(_) => n.channel = None,
        Pill::Reporter(_) => n.reporter = None,
        Pill::Signature(_) => n.signature = None,
        Pill::DuplicateOf(_) => n.duplicate_of = None,
        Pill::Search(_) => n.q = None,
        Pill::From(_) => n.from = None,
        Pill::To(_) => n.to = None,
        Pill::Record(_) => n.record = None,
        Pill::Unseen => n.unseen = false,
        Pill::Trash => n.trash = false,
    })
}

/// How many filters the panel holds beyond what the view decides.
#[must_use]
pub fn panel_count(q: &ListQuery, view: Option<View>) -> usize {
    let mut n = [
        !q.kinds.is_empty(),
        !q.priorities.is_empty(),
        q.label.is_some(),
        q.gateway.is_some(),
        q.channel.is_some(),
        q.record.is_some(),
        q.reporter.is_some(),
        q.from.is_some(),
        q.to.is_some(),
        q.unseen,
        q.signature.is_some(),
        q.duplicate_of.is_some(),
    ]
    .into_iter()
    .filter(|on| *on)
    .count();
    if view.is_none() {
        n += [!q.statuses.is_empty(), q.assignee.is_some(), q.trash]
            .into_iter()
            .filter(|on| *on)
            .count();
    }
    n
}

/// The address of the list `q` in the language `code`.
#[must_use]
pub fn list_path(code: &str, q: &ListQuery) -> String {
    let search = write(q);
    if search.is_empty() {
        format!("/{code}/account/admin/feedback")
    } else {
        format!("/{code}/account/admin/feedback?{search}")
    }
}

/// The address of report `id` opened from the list `q`, which its page
/// steps through.
#[must_use]
pub fn report_path(code: &str, id: &[u8; 16], q: &ListQuery) -> String {
    let search = write(&without_page(q));
    let hex = hex(id);
    if search.is_empty() {
        format!("/{code}/account/admin/feedback/{hex}")
    } else {
        format!("/{code}/account/admin/feedback/{hex}?{search}")
    }
}

/// The report a report page's path names: `…/account/admin/feedback/<hex>`.
#[must_use]
pub fn report_at(path: &str) -> Option<[u8; 16]> {
    let (_, rest) = path.split_once("/account/admin/feedback/")?;
    id_of(rest)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A v7 id, as Central makes every one.
    fn id(n: u8) -> [u8; 16] {
        let mut id = [0x10; 16];
        id[6] = 0x70;
        id[8] = 0x80;
        id[15] = n;
        id
    }

    #[test]
    fn the_old_address_encoding_round_trips() {
        assert_eq!(encode("new,triaged"), "new%2Ctriaged");
        assert_eq!(encode("stack went wrong"), "stack+went+wrong");
        assert_eq!(encode("a*b-c.d_e~f"), "a*b-c.d_e%7Ef");
        assert_eq!(encode("ü"), "%C3%BC");
        assert_eq!(decode("stack+went%20wrong"), "stack went wrong");
        assert_eq!(decode("100%"), "100%");
        assert_eq!(decode("%zz%C3%BC"), "%zzü");
    }

    /// Saved views keep the query strings the old administration wrote:
    /// they must read the same and be written back byte for byte.
    #[test]
    fn saved_views_of_the_old_administration_still_match() {
        let me = hex(&id(1));
        let old = [
            "status=new".to_owned(),
            "status=new%2Ctriaged%2Cin_progress".to_owned(),
            format!("status=new%2Ctriaged%2Cin_progress&assignee={me}"),
            "status=new%2Ctriaged%2Cin_progress&assignee=none".to_owned(),
            "trash=1".to_owned(),
            "status=resolved%2Cwont_fix%2Cduplicate&kind=bug%2Ccrash&priority=critical%2Cnone&label=ui-bug&q=stack+went+wrong&from=2026-10-01&to=2026-11-01&record=yes&unseen=1&sort=priority&dir=asc&limit=100".to_owned(),
            "gateway=Synthetic+gateway&channel=gateway&reporter=5bdc&signature=0123456789abcdef0123456789abcdef".to_owned(),
            format!("duplicate_of={}&q=%C3%BCber+%22quotes%22+%26+%3D", hex(&id(2))),
        ];
        for text in old {
            assert_eq!(write(&read(&text)), text, "{text}");
            assert_eq!(write(&read(&format!("?{text}"))), text);
        }
        assert_eq!(write(&ListQuery::default()), "");
    }

    #[test]
    fn what_central_would_refuse_is_left_out() {
        let q = read(
            "status=new,bogus&kind=chat&priority=none,urgent&from=2026-13-01&to=2026-02-31&label=Bad+Label\
             &reporter=xyz&signature=ABC&assignee=00000000000000000000000000000000&duplicate_of=zz\
             &sort=sql&dir=up&limit=7&page=-2&trash=yes&unseen=true&record=maybe&channel=other\
             &q=%E2%80%AE%E2%80%AE",
        );
        assert_eq!(q.statuses, vec![ReportStatus::New]);
        assert_eq!(q.kinds, Vec::<ReportKind>::new());
        assert_eq!(q.priorities, vec![None]);
        assert_eq!(q.from, None);
        // Central's own rule takes day 31 in any month
        assert_eq!(q.to.as_deref(), Some("2026-02-31"));
        assert_eq!(
            (q.label, q.reporter, q.signature, q.assignee, q.duplicate_of),
            (None, None, None, None, None)
        );
        assert_eq!(
            (q.sort, q.asc, q.limit, q.page),
            (ReportSort::Received, false, PAGE, 1)
        );
        assert!(!q.trash && !q.unseen && q.record.is_none() && q.channel.is_none());
        assert_eq!(q.q, None, "only invisible marks is no search");
        assert!(filter_of(&read("q=x&label=ok")).validate().is_ok());
    }

    #[test]
    fn a_search_is_cleaned_as_central_takes_it() {
        assert_eq!(
            clean_query("  stack\u{202E}went\twrong "),
            Some("stack went wrong".into())
        );
        assert_eq!(clean_query("   "), None);
        let long = "x".repeat(MAX_QUERY + 50);
        assert_eq!(
            clean_query(&long).map(|q| q.chars().count()),
            Some(MAX_QUERY)
        );
        assert!(shown("Erika", 5) && !shown("Erika!", 5) && !shown("a\u{200B}b", 9));
    }

    #[test]
    fn the_views_and_their_parts() {
        let me = id(1);
        for view in View::ALL {
            assert_eq!(view_of(&view.query(me), me), Some(view), "{view:?}");
        }
        let mut q = View::Mine.query(me);
        q.kinds = vec![ReportKind::Crash];
        q.sort = ReportSort::Priority;
        assert_eq!(view_of(&q, me), Some(View::Mine));
        assert_eq!(panel_count(&q, view_of(&q, me)), 1);
        assert_eq!(
            pills_of(&q, view_of(&q, me)),
            vec![Pill::Kind(ReportKind::Crash)]
        );
        let inbox = View::Inbox.applied(&q, me);
        assert_eq!(inbox.sort, ReportSort::Priority);
        assert_eq!(inbox.kinds, Vec::<ReportKind>::new());
        // out of every view, the view's parts are pills too
        let odd = ListQuery {
            statuses: vec![ReportStatus::Resolved],
            trash: true,
            ..ListQuery::default()
        };
        assert_eq!(view_of(&odd, me), None);
        assert_eq!(
            pills_of(&odd, None),
            vec![Pill::Status(ReportStatus::Resolved), Pill::Trash]
        );
        assert_eq!(panel_count(&odd, None), 2);
        assert!(!without(&odd, &Pill::Trash).trash);
    }

    #[test]
    fn a_change_goes_back_to_the_first_page() {
        let mut q = read("status=new&page=3&limit=25");
        assert_eq!((q.page, q.limit), (3, 25));
        q = changed(&q, |n| {
            n.kinds = toggled(&n.kinds, ReportKind::Bug, ReportKind::ALL);
        });
        assert_eq!(q.page, 1);
        assert_eq!(write(&q), "status=new&kind=bug&limit=25");
        assert_eq!(
            toggled(&[ReportKind::Bug], ReportKind::Bug, ReportKind::ALL),
            Vec::<ReportKind>::new()
        );
        let asked = request_of(&read("page=3&limit=25&dir=asc"));
        assert_eq!(
            (asked.sort, asked.descending, asked.offset, asked.limit),
            (ReportSort::Received, false, 50, 25)
        );
        assert_eq!(asked.filter, ReportFilter::default());
    }

    #[test]
    fn the_filter_is_central_s() {
        let me = id(1);
        let q = read(&format!(
            "status=new&priority=high,none&assignee={}&duplicate_of={}&trash=1&unseen=1&record=no&channel=direct",
            hex(&me),
            hex(&id(2))
        ));
        let f = filter_of(&q);
        assert_eq!(f.statuses, vec![ReportStatus::New]);
        assert_eq!(f.priorities, vec![Some(Priority::High), None]);
        assert_eq!(f.assignee, Some(me));
        assert!(!f.unassigned);
        assert_eq!(f.duplicate_of, Some(id(2)));
        assert!(f.trash && f.unseen);
        assert_eq!(f.has_record, Some(false));
        assert_eq!(f.channel, Some(Channel::Direct));
        assert!(f.validate().is_ok());
        assert!(filter_of(&read("assignee=none")).unassigned);
    }

    #[test]
    fn the_addresses() {
        let q = read("status=new&page=2");
        assert_eq!(
            list_path("de", &q),
            "/de/account/admin/feedback?status=new&page=2"
        );
        assert_eq!(
            list_path("en", &ListQuery::default()),
            "/en/account/admin/feedback"
        );
        let path = report_path("en", &id(7), &q);
        assert_eq!(
            path,
            format!("/en/account/admin/feedback/{}?status=new", hex(&id(7)))
        );
        assert_eq!(
            report_at(&format!("/en/account/admin/feedback/{}", hex(&id(7)))),
            Some(id(7))
        );
        assert_eq!(report_at("/en/account/admin/feedback/not-an-id"), None);
        assert_eq!(
            report_at(&format!("/en/account/admin/feedback/{}", "00".repeat(16))),
            None
        );
    }
}
