//! The messenger's data, without the browser: what the Worker opened, the
//! conversations the page knows of, and how they are named, ordered and
//! counted. Conversation ids are the protocol's (`docs/central-chat.md`):
//! `dm:<lo>:<hi>` with each tag as at least four hex digits, `grp:<uuid>`.

use std::collections::{BTreeMap, BTreeSet};

use central_api::chat::AccountKeys;
use central_api::groups::Group;
use central_api::social::Person;
use serde::Deserialize;

/// The newest messages kept, as the previous site and the protocol bound them.
pub const KEPT: usize = central_api::chat::MAX_DELIVERIES;

/// A message the Worker authenticated and opened. Its text is plain text
/// and is only ever rendered as text.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Message {
    /// The sender's signed correlation id.
    pub id: String,
    /// The signed conversation.
    pub conv: String,
    /// The sender's public tag.
    pub sender: u32,
    /// The text.
    pub text: String,
    /// Sent as `/me` from the game.
    pub emote: bool,
    /// Central's acceptance, Unix milliseconds.
    pub at: u64,
}

/// A peer's keys as the Worker judged them: the safety number and whether
/// they changed since they were trusted.
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Review {
    /// The peer's tag in hex.
    pub tag: String,
    /// Sixty digits to compare.
    pub number: String,
    /// The keys changed; sending is blocked until confirmed.
    pub changed: bool,
    /// Exactly the keys a confirmation is for.
    pub snapshot: AccountKeys,
}

impl PartialEq for Review {
    /// The same judgement of the same keys: the number is derived from the
    /// keys, so it changes when they do.
    fn eq(&self, other: &Self) -> bool {
        self.tag == other.tag && self.number == other.number && self.changed == other.changed
    }
}

impl Review {
    /// The peer's tag.
    #[must_use]
    pub fn peer(&self) -> Option<u32> {
        u32::from_str_radix(&self.tag, 16).ok()
    }

    /// The safety number in groups of five digits.
    #[must_use]
    pub fn grouped(&self) -> String {
        let digits: Vec<char> = self.number.chars().collect();
        digits
            .chunks(5)
            .map(|group| group.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// A tag as the protocol writes it: lowercase hex, at least four digits.
#[must_use]
pub fn tag_text(tag: u32) -> String {
    format!("{tag:04x}")
}

/// A public handle: the display name and the tag.
#[must_use]
pub fn handle(person: &Person) -> String {
    central_api::profile::handle(&person.display_name, person.tag)
}

/// Whether `tag` can be an account's public tag.
#[must_use]
pub fn valid_tag(tag: u32) -> bool {
    (1..=0x7fff_ffff).contains(&tag)
}

/// The tag in a full handle (`Name#1a2b`) or a bare tag, written exactly as
/// the protocol writes tags; a name alone never matches anyone.
#[must_use]
pub fn tag_of(input: &str) -> Option<u32> {
    let text = input.trim().rsplit('#').next()?;
    let canonical = match text.len() {
        4 => text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        5..=8 => {
            !text.starts_with('0')
                && text
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        }
        _ => false,
    };
    let tag = u32::from_str_radix(text, 16).ok().filter(|_| canonical)?;
    valid_tag(tag).then_some(tag)
}

/// The direct conversation of two tags.
#[must_use]
pub fn direct(a: u32, b: u32) -> String {
    let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
    format!("dm:{}:{}", tag_text(lo), tag_text(hi))
}

/// The other side of a direct conversation of `own`'s.
#[must_use]
pub fn partner(conv: &str, own: u32) -> Option<u32> {
    let mut tags = conv.strip_prefix("dm:")?.split(':');
    let (a, b) = (tags.next()?, tags.next()?);
    if tags.next().is_some() {
        return None;
    }
    let (a, b) = (tag_of(a)?, tag_of(b)?);
    if direct(a, b) != conv {
        return None;
    }
    match (a == own, b == own) {
        (true, false) => Some(b),
        (false, true) => Some(a),
        _ => None,
    }
}

/// What kind of conversation a row is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    /// With one person.
    Direct(Person),
    /// A group this account is a member of.
    Group(Group),
    /// A group this account left or that was deleted: its history only.
    Former,
}

/// One row of the conversation list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Conversation {
    /// The protocol's id.
    pub id: String,
    /// The name shown: a display name, a group's name, or "a former group".
    pub title: String,
    /// Who or what.
    pub kind: Kind,
    /// The newest message.
    pub last: Option<Message>,
    /// Messages from others since the conversation was last looked at.
    pub unread: usize,
    /// Messages can be sent: a contact, or a group of which this account
    /// is a member.
    pub open: bool,
}

impl Conversation {
    /// The tags a message here goes to, this account's included.
    #[must_use]
    pub fn members(&self, own: u32) -> Vec<u32> {
        match &self.kind {
            Kind::Direct(person) => vec![own, person.tag],
            Kind::Group(group) => group.members.iter().map(|m| m.person.tag).collect(),
            Kind::Former => Vec::new(),
        }
    }
}

/// What the list is made from.
pub struct Sources<'a> {
    /// This account's tag.
    pub own: u32,
    /// Accepted contacts.
    pub friends: &'a [Person],
    /// Every sender the inbox named, by tag.
    pub known: &'a BTreeMap<u32, String>,
    /// The groups this account is a member of.
    pub groups: &'a [Group],
    /// Every message, oldest first.
    pub messages: &'a [Message],
    /// The newest message time looked at, by conversation; `None` until the
    /// inbox was first read through, so history is not "unread".
    pub seen: Option<&'a BTreeMap<String, u64>>,
}

/// Every conversation the page knows of: contacts, groups and what the
/// inbox replayed; the newest talk first, then by name.
#[must_use]
pub fn conversations(from: &Sources<'_>) -> Vec<Conversation> {
    let mut last: BTreeMap<&str, &Message> = BTreeMap::new();
    let mut unread: BTreeMap<&str, usize> = BTreeMap::new();
    for m in from.messages {
        last.insert(&m.conv, m);
        if let Some(seen) = from.seen
            && m.sender != from.own
            && m.at > seen.get(&m.conv).copied().unwrap_or(0)
        {
            *unread.entry(&m.conv).or_default() += 1;
        }
    }
    let mut out = Vec::new();
    let mut add = |id: String, title: String, kind: Kind, open: bool| {
        let newest = last.get(id.as_str()).map(|m| (*m).clone());
        let count = unread.get(id.as_str()).copied().unwrap_or(0);
        out.push(Conversation {
            id,
            title,
            kind,
            last: newest,
            unread: count,
            open,
        });
    };
    let mut people: BTreeSet<u32> = BTreeSet::new();
    for person in from.friends {
        people.insert(person.tag);
        add(
            direct(from.own, person.tag),
            person.display_name.clone(),
            Kind::Direct(person.clone()),
            true,
        );
    }
    for conv in last.keys() {
        let Some(tag) = partner(conv, from.own) else {
            continue;
        };
        if !people.insert(tag) {
            continue;
        }
        let name = from.known.get(&tag).cloned().unwrap_or_else(|| "?".into());
        let person = Person {
            tag,
            display_name: name.clone(),
        };
        add((*conv).to_owned(), name, Kind::Direct(person), false);
    }
    for group in from.groups {
        let open = group.members.iter().any(|m| m.person.tag == from.own);
        add(
            format!("grp:{}", group.id),
            group.name.clone(),
            Kind::Group(group.clone()),
            open,
        );
    }
    for conv in last.keys() {
        if conv.starts_with("grp:") && !from.groups.iter().any(|g| format!("grp:{}", g.id) == *conv)
        {
            add((*conv).to_owned(), String::new(), Kind::Former, false);
        }
    }
    out.sort_by(|a, b| {
        let at = |c: &Conversation| c.last.as_ref().map_or(0, |m| m.at);
        at(b).cmp(&at(a)).then_with(|| a.title.cmp(&b.title))
    });
    out
}

/// The longest pause within a run of one sender's lines.
pub const RUN_PAUSE: u64 = 5 * 60_000;

/// For each of `messages` (oldest first), whether it starts a run of one
/// sender's lines and whether it ends one. A run breaks where the sender
/// changes, where a day ends (`same_day`, in the reader's calendar) and
/// after a pause longer than [`RUN_PAUSE`].
#[must_use]
pub fn runs(messages: &[Message], same_day: impl Fn(u64, u64) -> bool) -> Vec<(bool, bool)> {
    let starts: Vec<bool> = messages
        .iter()
        .enumerate()
        .map(|(at, m)| {
            at.checked_sub(1)
                .and_then(|before| messages.get(before))
                .is_none_or(|p| {
                    p.sender != m.sender
                        || !same_day(p.at, m.at)
                        || m.at.saturating_sub(p.at) > RUN_PAUSE
                })
        })
        .collect();
    starts
        .iter()
        .enumerate()
        .map(|(at, first)| (*first, starts.get(at + 1).copied().unwrap_or(true)))
        .collect()
}

/// An avatar's letters: the first of a name, or of its first two words.
#[must_use]
pub fn initials(name: &str) -> String {
    let words: Vec<&str> = name
        .split(|c: char| c.is_whitespace() || matches!(c, '_' | '-' | '.'))
        .filter(|w| !w.is_empty())
        .collect();
    let letters: String = match words.as_slice() {
        [] => "?".into(),
        [one] => one.chars().take(1).collect(),
        [first, second, ..] => first
            .chars()
            .take(1)
            .chain(second.chars().take(1))
            .collect(),
    };
    letters.to_uppercase()
}

/// One of six avatar tints, the same for the same seed everywhere.
#[must_use]
pub fn hue(seed: &str) -> u32 {
    seed.chars()
        .fold(0u32, |h, c| h.wrapping_mul(31).wrapping_add(u32::from(c)))
        % 6
}

/// Whether the inbox's messages, newest kept, change by `incoming`: new
/// ones are added; one that differs from the same id already shown is
/// refused (counted), never replaced.
pub fn merge(messages: &mut Vec<Message>, incoming: Vec<Message>) -> usize {
    let mut by_id: BTreeMap<String, Message> =
        messages.drain(..).map(|m| (m.id.clone(), m)).collect();
    let mut refused = 0;
    for m in incoming {
        match by_id.get(&m.id) {
            Some(old) if *old != m => refused += 1,
            Some(_) => {}
            None => {
                by_id.insert(m.id.clone(), m);
            }
        }
    }
    let mut all: Vec<Message> = by_id.into_values().collect();
    all.sort_by(|a, b| a.at.cmp(&b.at).then_with(|| a.id.cmp(&b.id)));
    let skip = all.len().saturating_sub(KEPT);
    messages.extend(all.into_iter().skip(skip));
    refused
}

#[cfg(test)]
mod tests {
    use super::*;
    use central_api::groups::Member;

    fn person(tag: u32, name: &str) -> Person {
        Person {
            tag,
            display_name: name.into(),
        }
    }

    fn message(id: &str, conv: &str, sender: u32, at: u64) -> Message {
        Message {
            id: id.into(),
            conv: conv.into(),
            sender,
            text: format!("text {id}"),
            emote: false,
            at,
        }
    }

    #[test]
    fn conversation_ids() {
        assert_eq!(direct(0x2b, 0x1a), "dm:001a:002b");
        assert_eq!(direct(0x1_0000, 0xff), "dm:00ff:10000");
        assert_eq!(partner("dm:001a:002b", 0x1a), Some(0x2b));
        assert_eq!(partner("dm:00ff:10000", 0xff), Some(0x1_0000));
        assert_eq!(partner("dm:001a:002b", 0x3c), None);
        assert_eq!(partner("dm:002b:001a", 0x1a), None, "not canonical");
        assert_eq!(partner("dm:1a:2b", 0x1a), None);
        assert_eq!(partner("dm:001a:001a", 0x1a), None);
        assert_eq!(partner("grp:x", 0x1a), None);
    }

    #[test]
    fn tags_from_handles() {
        assert_eq!(tag_of("Erika#1a2b"), Some(0x1a2b));
        assert_eq!(tag_of(" 1a2b "), Some(0x1a2b));
        assert_eq!(tag_of("Erika#1a2bc"), Some(0x1a2bc));
        assert_eq!(
            tag_of("Erika#01a2b"),
            None,
            "five digits never start with 0"
        );
        assert_eq!(tag_of("Erika#1A2B"), None);
        assert_eq!(tag_of("Erika"), None);
        assert_eq!(tag_of("0000"), None);
        assert_eq!(tag_of("80000000"), None);
    }

    #[test]
    fn list_orders_counts_and_keeps_history() {
        let own = 0x10;
        let friends = [person(0x20, "Bea"), person(0x30, "Ada")];
        let mut known = BTreeMap::new();
        known.insert(0x40, "Old".to_owned());
        let group = Group {
            id: "0190f3a0-0000-7000-8000-000000000001".into(),
            name: "Table".into(),
            members: vec![Member {
                person: person(own, "Me"),
                owner: true,
            }],
            invited: Vec::new(),
        };
        let messages = [
            message("1", &direct(own, 0x20), 0x20, 10),
            message("2", &direct(own, 0x40), 0x40, 20),
            message("3", "grp:0190f3a0-0000-7000-8000-00000000dead", 0x20, 30),
            message("4", &direct(own, 0x20), own, 40),
            message("5", &direct(own, 0x20), 0x20, 50),
        ];
        let mut seen = BTreeMap::new();
        seen.insert(direct(own, 0x20), 10);
        let list = conversations(&Sources {
            own,
            friends: &friends,
            known: &known,
            groups: std::slice::from_ref(&group),
            messages: &messages,
            seen: Some(&seen),
        });
        let ids: Vec<&str> = list.iter().map(|c| c.title.as_str()).collect();
        assert_eq!(ids, ["Bea", "", "Old", "Ada", "Table"]);
        assert_eq!(list[0].unread, 1, "own messages and seen ones do not count");
        assert!(list[0].open);
        assert!(matches!(list[1].kind, Kind::Former));
        assert!(!list[2].open, "no longer a contact: history only");
        assert_eq!(list[4].members(own), [own]);
        let quiet = conversations(&Sources {
            own,
            friends: &friends,
            known: &known,
            groups: &[],
            messages: &messages,
            seen: None,
        });
        assert!(quiet.iter().all(|c| c.unread == 0), "history is not unread");
    }

    #[test]
    fn merging_refuses_changed_messages_and_bounds() {
        let mut shown = vec![message("a", "dm:0010:0020", 0x20, 5)];
        let mut changed = message("a", "dm:0010:0020", 0x20, 5);
        changed.text = "other".into();
        let refused = merge(
            &mut shown,
            vec![changed, message("b", "dm:0010:0020", 0x20, 1)],
        );
        assert_eq!(refused, 1);
        assert_eq!(shown.len(), 2);
        assert_eq!(shown[0].id, "b", "oldest first");
        assert_eq!(shown[1].text, "text a", "never replaced");
        let many: Vec<Message> = (0..KEPT + 3)
            .map(|i| {
                message(
                    &format!("{i:06}"),
                    "dm:0010:0020",
                    0x20,
                    u64::try_from(i).unwrap_or_default(),
                )
            })
            .collect();
        merge(&mut shown, many);
        assert_eq!(shown.len(), KEPT);
        assert_eq!(shown[KEPT - 1].id, format!("{:06}", KEPT + 2));
    }

    #[test]
    fn runs_break_at_a_sender_a_pause_and_a_day() {
        let minute = 60_000;
        let day = 86_400_000;
        let conv = "dm:0010:0020";
        let lines = [
            message("1", conv, 0x20, 0),
            message("2", conv, 0x20, minute),
            message("3", conv, 0x20, 2 * minute),
            message("4", conv, 0x10, 3 * minute),
            message("5", conv, 0x10, 3 * minute + RUN_PAUSE + 1),
            message("6", conv, 0x10, 4 * minute + RUN_PAUSE),
            message("7", conv, 0x10, day + 1),
        ];
        let same_day = |a: u64, b: u64| a / day == b / day;
        assert_eq!(
            runs(&lines, same_day),
            [
                (true, false),
                (false, false),
                (false, true),
                (true, true),
                (true, false),
                (false, true),
                (true, true),
            ]
        );
        assert_eq!(runs(&lines[..1], same_day), [(true, true)]);
        assert_eq!(runs(&[], same_day), []);
    }

    #[test]
    fn faces_and_numbers() {
        assert_eq!(initials("chat_alice"), "CA");
        assert_eq!(initials("Bea"), "B");
        assert_eq!(initials("éva"), "É");
        assert_eq!(initials(""), "?");
        assert_eq!(hue("dm:0010:0020"), hue("dm:0010:0020"));
        assert!(hue("x") < 6);
        let review = Review {
            tag: "1a2b".into(),
            number: "123456789012".into(),
            changed: false,
            snapshot: AccountKeys {
                tag: "1a2b".into(),
                handle: "Erika#1a2b".into(),
                devices: Vec::new(),
            },
        };
        assert_eq!(review.grouped(), "12345 67890 12");
        assert_eq!(review.peer(), Some(0x1a2b));
    }
}
