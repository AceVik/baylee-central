//! Central's social surface for the messenger: contacts, groups and the
//! encrypted chat, as `central-api` types. The types refuse unknown fields;
//! the checks here add what they cannot say (the previous site's
//! `chat/api.ts`, `social/api.ts` and `chat/groups.ts`): lengths and
//! alphabets, bounds, uniqueness, an inbox that only moves forward, keys
//! only for the tags asked about, and an acceptance naming exactly the
//! message that was sent.

use std::collections::BTreeSet;

use central_api::chat::{
    AccountKeys, ChatRequest, ChatResponse, Delivery, DeviceKey, DeviceRegistration, Header,
    PAGE_SIZE, Sealed,
};
use central_api::groups::{
    Change, Group, GroupRequest, GroupResponse, Groups, MAX_GROUPS, MAX_MEMBERS,
};
use central_api::social::{
    Contact, ContactAction, MAX_BLOCKS, MAX_CONTACTS, MAX_INCOMING, MAX_OUTGOING, Person, Relation,
    SocialRequest, SocialResponse,
};
use central_api::wire::{Request, Response};

use super::super::b64;
use super::super::central::{Failure, call};
use super::model::{tag_of, tag_text, valid_tag};

/// The largest integer a page's number keeps exactly, as Central bounds them.
const SAFE: u64 = (1 << 53) - 1;

fn base64url(text: &str, max: usize) -> bool {
    !text.is_empty()
        && text.len() <= max
        && text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

fn exactly(text: &str, len: usize) -> bool {
    text.len() == len && base64url(text, len)
}

fn fingerprint(id: &str) -> bool {
    id.len() == 16
        && id
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// A `UUIDv7` as Central writes it: lowercase, hyphenated.
#[must_use]
pub fn uuid_v7(id: &str) -> bool {
    let b = id.as_bytes();
    b.len() == 36
        && b.iter().enumerate().all(|(i, &c)| match i {
            8 | 13 | 18 | 23 => c == b'-',
            _ => c.is_ascii_digit() || (b'a'..=b'f').contains(&c),
        })
        && b[14] == b'7'
        && matches!(b[19], b'8' | b'9' | b'a' | b'b')
}

/// A public person: a tag and a display name as Central allows them.
#[must_use]
pub fn person(p: &Person) -> bool {
    valid_tag(p.tag) && central_api::profile::valid_display_name(&p.display_name)
}

fn device(d: &DeviceKey) -> bool {
    fingerprint(&d.id)
        && exactly(&d.ik, 43)
        && exactly(&d.dk, 43)
        && exactly(&d.dsig, 86)
        && (1..=24).contains(&d.label.len())
        && d.label.bytes().all(|b| (0x20..=0x7e).contains(&b))
        && d.label.trim() == d.label
        && d.added_at <= SAFE
}

fn account_keys(k: &AccountKeys) -> bool {
    let canonical = tag_of(&k.tag).is_some_and(|tag| tag_text(tag) == k.tag);
    let ids: BTreeSet<&str> = k.devices.iter().map(|d| d.id.as_str()).collect();
    canonical
        && k.handle.chars().count() <= 25
        && k.handle.ends_with(&format!("#{}", k.tag))
        && k.devices.len() <= central_api::chat::MAX_DEVICES
        && ids.len() == k.devices.len()
        && k.devices.iter().all(device)
}

fn sealed(s: &Sealed, max_wraps: usize) -> bool {
    base64url(&s.hdr, 684)
        && exactly(&s.n, 32)
        && base64url(&s.ct, 5526)
        && exactly(&s.epk, 43)
        && exactly(&s.sig, 86)
        && s.wraps.len() <= max_wraps
        && s.wraps
            .iter()
            .all(|w| fingerprint(&w.device) && exactly(&w.k, 64))
}

/// One delivery as an inbox may hold it: one wrap, this device's.
#[must_use]
pub fn delivery(d: &Delivery) -> bool {
    (1..=SAFE).contains(&d.seq)
        && person(&d.sender)
        && device(&d.device)
        && sealed(&d.sealed, 1)
        && d.at <= SAFE
}

/// An inbox page after `after`: at most a page, each delivery valid, the
/// cursor strictly increasing, and `more` only after a full page.
#[must_use]
pub fn inbox_page(messages: &[Delivery], more: bool, after: u64) -> bool {
    let mut previous = after;
    messages.len() <= PAGE_SIZE
        && (!more || messages.len() == PAGE_SIZE)
        && messages.iter().all(|m| {
            let forward = m.seq > previous;
            previous = m.seq;
            forward && delivery(m)
        })
}

/// Keys for some of the tags asked about, each once.
#[must_use]
pub fn keys_answer(keys: &[AccountKeys], asked: &[u32]) -> bool {
    let tags: BTreeSet<&str> = keys.iter().map(|k| k.tag.as_str()).collect();
    keys.len() <= asked.len()
        && tags.len() == keys.len()
        && keys
            .iter()
            .all(|k| account_keys(k) && tag_of(&k.tag).is_some_and(|tag| asked.contains(&tag)))
}

/// Central's acceptance names the message in the signed header: its id and
/// its conversation.
#[must_use]
pub fn accepted(id: &str, envelope: &Sealed, conv: &str) -> bool {
    let header = b64::decode_url(&envelope.hdr)
        .filter(|bytes| bytes.len() <= 512)
        .and_then(|bytes| serde_json::from_slice::<Header>(&bytes).ok());
    uuid_v7(id) && header.is_some_and(|h| h.id == id && h.conv == conv)
}

/// A contacts list: valid people, each once, within each relation's bound.
#[must_use]
pub fn contacts_list(list: &[Contact]) -> bool {
    let mut seen = BTreeSet::new();
    let mut counts = [0usize; 4];
    list.iter().all(|c| {
        let (slot, cap) = match c.relation {
            Relation::Friend => (0, MAX_CONTACTS),
            Relation::Incoming => (1, MAX_INCOMING),
            Relation::Outgoing => (2, MAX_OUTGOING),
            Relation::Blocked => (3, MAX_BLOCKS),
        };
        counts[slot] += 1;
        person(&c.person) && seen.insert(c.person.tag) && counts[slot] <= cap
    })
}

fn group(g: &Group) -> bool {
    let people: Vec<&Person> = g
        .members
        .iter()
        .map(|m| &m.person)
        .chain(g.invited.iter())
        .collect();
    let tags: BTreeSet<u32> = people.iter().map(|p| p.tag).collect();
    uuid_v7(&g.id)
        && central_api::groups::valid_name(&g.name)
        && people.len() <= MAX_MEMBERS
        && tags.len() == people.len()
        && people.iter().all(|p| person(p))
        && g.members.iter().filter(|m| m.owner).count() == 1
}

/// A groups snapshot: each group and invitation valid, each id once.
#[must_use]
pub fn groups_snapshot(s: &Groups) -> bool {
    let ids: BTreeSet<&str> = s
        .groups
        .iter()
        .map(|g| g.id.as_str())
        .chain(s.invites.iter().map(|i| i.group.as_str()))
        .collect();
    s.groups.len() + s.invites.len() <= MAX_GROUPS
        && ids.len() == s.groups.len() + s.invites.len()
        && s.groups.iter().all(group)
        && s.invites.iter().all(|i| {
            uuid_v7(&i.group)
                && central_api::groups::valid_name(&i.name)
                && person(&i.inviter)
                && i.at <= SAFE
        })
}

async fn social(request: SocialRequest) -> Result<SocialResponse, Failure> {
    match call(&Request::Social { request }).await? {
        Response::Social { response } => Ok(response),
        _ => Err(Failure::Malformed),
    }
}

async fn chat(request: ChatRequest) -> Result<ChatResponse, Failure> {
    match social(SocialRequest::Chat { request }).await? {
        SocialResponse::Chat { response } => Ok(response),
        _ => Err(Failure::Malformed),
    }
}

async fn groups_call(request: GroupRequest) -> Result<GroupResponse, Failure> {
    match social(SocialRequest::Group { request }).await? {
        SocialResponse::Group { response } => Ok(response),
        _ => Err(Failure::Malformed),
    }
}

/// Registers this browser's device; Central's fingerprint of it.
///
/// # Errors
/// The refusal, or an answer of another shape.
pub async fn register(device: DeviceRegistration) -> Result<String, Failure> {
    match chat(ChatRequest::Register { device }).await? {
        ChatResponse::Registered { id } if fingerprint(&id) => Ok(id),
        _ => Err(Failure::Malformed),
    }
}

/// The current devices of `tags` that this account may write to.
///
/// # Errors
/// The refusal, or an answer of another shape.
pub async fn keys(tags: &[u32]) -> Result<Vec<AccountKeys>, Failure> {
    match chat(ChatRequest::Keys {
        tags: tags.to_vec(),
    })
    .await?
    {
        ChatResponse::Keys { keys } if keys_answer(&keys, tags) => Ok(keys),
        _ => Err(Failure::Malformed),
    }
}

/// Sends a sealed message to `conv`.
///
/// # Errors
/// The refusal (`Conflict`: this envelope was not accepted), or an
/// acceptance of another message.
pub async fn send(conv: &str, envelope: &Sealed) -> Result<(), Failure> {
    match chat(ChatRequest::Send {
        conv: conv.to_owned(),
        sealed: envelope.clone(),
    })
    .await?
    {
        ChatResponse::Sent { id, at } if at <= SAFE && accepted(&id, envelope, conv) => Ok(()),
        _ => Err(Failure::Malformed),
    }
}

/// One inbox page of `device` after the cursor `after`.
///
/// # Errors
/// The refusal, or a page that breaks the cursor's rules.
pub async fn inbox(device: &str, after: u64) -> Result<(Vec<Delivery>, bool), Failure> {
    match chat(ChatRequest::Inbox {
        device: device.to_owned(),
        after,
    })
    .await?
    {
        ChatResponse::Inbox { messages, more } if inbox_page(&messages, more, after) => {
            Ok((messages, more))
        }
        _ => Err(Failure::Malformed),
    }
}

/// This account's contacts.
///
/// # Errors
/// The refusal, or a list that breaks its bounds.
pub async fn contacts() -> Result<Vec<Contact>, Failure> {
    match social(SocialRequest::Contacts).await? {
        SocialResponse::Contacts { contacts } if contacts_list(&contacts) => Ok(contacts),
        _ => Err(Failure::Malformed),
    }
}

/// The public profile of exactly `tag`, if there is one.
///
/// # Errors
/// The refusal, or another person.
pub async fn lookup(tag: u32) -> Result<Option<Person>, Failure> {
    match social(SocialRequest::Lookup { tag }).await? {
        SocialResponse::Person { person: None } => Ok(None),
        SocialResponse::Person { person: Some(p) } if p.tag == tag && person(&p) => Ok(Some(p)),
        _ => Err(Failure::Malformed),
    }
}

/// Changes the relation with `tag`.
///
/// # Errors
/// The refusal.
pub async fn contact(tag: u32, change: ContactAction) -> Result<(), Failure> {
    match social(SocialRequest::Contact { tag, change }).await? {
        SocialResponse::Done => Ok(()),
        _ => Err(Failure::Malformed),
    }
}

/// This account's groups and invitations.
///
/// # Errors
/// The refusal, or a snapshot that breaks its bounds.
pub async fn groups() -> Result<Groups, Failure> {
    match groups_call(GroupRequest::List).await? {
        GroupResponse::Groups { snapshot } if groups_snapshot(&snapshot) => Ok(snapshot),
        _ => Err(Failure::Malformed),
    }
}

/// Creates a group; its id.
///
/// # Errors
/// The refusal.
pub async fn create_group(name: &str) -> Result<String, Failure> {
    match groups_call(GroupRequest::Create {
        name: name.to_owned(),
    })
    .await?
    {
        GroupResponse::Created { id } if uuid_v7(&id) => Ok(id),
        _ => Err(Failure::Malformed),
    }
}

/// Changes group `id`.
///
/// # Errors
/// The refusal.
pub async fn change_group(id: &str, change: Change) -> Result<(), Failure> {
    match groups_call(GroupRequest::Change {
        id: id.to_owned(),
        change,
    })
    .await?
    {
        GroupResponse::Done => Ok(()),
        _ => Err(Failure::Malformed),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use central_api::chat::Wrap;

    fn key(id: &str) -> DeviceKey {
        DeviceKey {
            id: id.into(),
            ik: "a".repeat(43),
            dk: "b".repeat(43),
            dsig: "c".repeat(86),
            label: "Linux".into(),
            added_at: 1,
        }
    }

    fn envelope(header: &str) -> Sealed {
        Sealed {
            hdr: b64::encode(header.as_bytes())
                .trim_end_matches('=')
                .replace('+', "-")
                .replace('/', "_"),
            n: "n".repeat(32),
            ct: "c".repeat(100),
            epk: "e".repeat(43),
            sig: "s".repeat(86),
            wraps: vec![Wrap {
                device: "0123456789abcdef".into(),
                k: "k".repeat(64),
            }],
        }
    }

    fn delivered(seq: u64) -> Delivery {
        Delivery {
            seq,
            sender: Person {
                tag: 0x1a2b,
                display_name: "Erika".into(),
            },
            device: key("0123456789abcdef"),
            sealed: envelope("{}"),
            at: 1,
        }
    }

    #[test]
    fn devices_and_keys() {
        assert!(device(&key("0123456789abcdef")));
        assert!(!device(&key("0123456789ABCDEF")));
        let mut spaced = key("0123456789abcdef");
        spaced.label = " Linux".into();
        assert!(!device(&spaced));
        let keys = AccountKeys {
            tag: "1a2b".into(),
            handle: "Erika#1a2b".into(),
            devices: vec![key("0123456789abcdef")],
        };
        assert!(keys_answer(std::slice::from_ref(&keys), &[0x1a2b]));
        assert!(
            !keys_answer(std::slice::from_ref(&keys), &[0x1a2c]),
            "not asked for"
        );
        assert!(
            !keys_answer(&[keys.clone(), keys.clone()], &[0x1a2b, 1]),
            "twice"
        );
        let mut twice = keys.clone();
        twice.devices.push(key("0123456789abcdef"));
        assert!(!keys_answer(&[twice], &[0x1a2b]), "a device twice");
        let mut other = keys;
        other.handle = "Erika#1a2c".into();
        assert!(!keys_answer(&[other], &[0x1a2b]), "handle of another tag");
    }

    #[test]
    fn inbox_moves_forward_only() {
        assert!(inbox_page(&[delivered(3), delivered(4)], false, 2));
        assert!(
            !inbox_page(&[delivered(2)], false, 2),
            "not after the cursor"
        );
        assert!(!inbox_page(&[delivered(4), delivered(3)], false, 0));
        assert!(
            !inbox_page(&[delivered(1)], true, 0),
            "more after a short page"
        );
        let full: Vec<Delivery> = (1..=50).map(delivered).collect();
        assert!(inbox_page(&full, true, 0));
        let mut two = delivered(1);
        two.sealed.wraps.push(two.sealed.wraps[0].clone());
        assert!(!inbox_page(&[two], false, 0), "one wrap per delivery");
    }

    #[test]
    fn acceptance_names_the_sent_message() {
        let id = "0190f3a0-1234-7abc-8def-0123456789ab";
        let header = format!(
            r#"{{"v":1,"id":"{id}","conv":"dm:001a:002b","from":"001a","dev":"0123456789abcdef","ts":1,"kind":"text"}}"#
        );
        let sent = envelope(&header);
        assert!(accepted(id, &sent, "dm:001a:002b"));
        assert!(!accepted(id, &sent, "dm:001a:002c"), "another conversation");
        assert!(!accepted(
            "0190f3a0-1234-7abc-8def-0123456789ac",
            &sent,
            "dm:001a:002b"
        ));
        assert!(!accepted(id, &envelope("{}"), "dm:001a:002b"));
        assert!(uuid_v7(id));
        assert!(!uuid_v7("0190f3a0-1234-4abc-8def-0123456789ab"));
    }

    #[test]
    fn contacts_and_groups_are_bounded() {
        let erika = Person {
            tag: 0x1a2b,
            display_name: "Erika".into(),
        };
        let friend = Contact {
            person: erika.clone(),
            relation: Relation::Friend,
        };
        assert!(contacts_list(std::slice::from_ref(&friend)));
        assert!(!contacts_list(&[friend.clone(), friend]), "a person twice");
        let outgoing: Vec<Contact> = (1..=51)
            .map(|tag| Contact {
                person: Person {
                    tag,
                    display_name: "Erika".into(),
                },
                relation: Relation::Outgoing,
            })
            .collect();
        assert!(!contacts_list(&outgoing));
        let g = Group {
            id: "0190f3a0-1234-7abc-8def-0123456789ab".into(),
            name: "Table".into(),
            members: vec![central_api::groups::Member {
                person: erika.clone(),
                owner: true,
            }],
            invited: vec![erika],
        };
        let snapshot = Groups {
            groups: vec![g],
            invites: Vec::new(),
        };
        assert!(!groups_snapshot(&snapshot), "member and invited at once");
    }
}
