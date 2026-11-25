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
    CardPrinting, CardSuggestion, Contact, ContactAction, FriendInviteSummary, MAX_BLOCKS,
    MAX_CARD_FILTER, MAX_CARD_PRINTINGS, MAX_CARD_QUERY, MAX_CARD_RESULTS, MAX_CONTACTS,
    MAX_FRIEND_INVITE_DAYS, MAX_INCOMING, MAX_OUTGOING, Person, Relation, SocialBadge,
    SocialRequest, SocialResponse, valid_friend_invite_code,
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

/// A canonical hyphenated lowercase UUID (e.g. Scryfall card printing ID).
#[must_use]
pub fn uuid_any(id: &str) -> bool {
    let b = id.as_bytes();
    b.len() == 36
        && b.iter().enumerate().all(|(i, &c)| match i {
            8 | 13 | 18 | 23 => c == b'-',
            _ => c.is_ascii_digit() || (b'a'..=b'f').contains(&c),
        })
}

fn clean_opt_filter(raw: Option<&str>) -> Option<String> {
    let trimmed = raw?.trim();
    if trimmed.is_empty()
        || trimmed.chars().count() > MAX_CARD_FILTER
        || trimmed.chars().any(char::is_control)
    {
        return None;
    }
    Some(trimmed.to_owned())
}

pub(super) fn card_printing_valid(p: &CardPrinting) -> bool {
    let id_ok = match (&p.scryfall_id, p.ub) {
        // Defense in depth: Universes Beyond printings must never carry an image ID.
        (Some(_), true) => false,
        (Some(id), false) => uuid_any(id),
        (None, _) => true,
    };
    id_ok
        && (1..=12).contains(&p.set_code.len())
        && p.set_code.bytes().all(|b| b.is_ascii_alphanumeric())
        && p.set_name.chars().count() <= 80
        && (1..=16).contains(&p.collector_number.chars().count())
        && (2..=5).contains(&p.lang.len())
        && p.finishes <= 7
}

pub(super) fn card_suggestion_valid(s: &CardSuggestion) -> bool {
    !s.name.is_empty()
        && s.name.chars().count() <= 128
        && !s.en_name.is_empty()
        && s.en_name.chars().count() <= 128
        && (2..=5).contains(&s.matched_lang.len())
        && s.preview.as_ref().is_none_or(card_printing_valid)
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
        && central_api::groups::valid_description(&g.description)
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
                && central_api::groups::valid_description(&i.description)
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

/// Searches public enrolled players by display name or tag (`Name#tag`, `#tag`,
/// or display-name substring).
///
/// # Errors
/// The refusal, or an answer exceeding bounds.
pub async fn search_people(query: &str) -> Result<Vec<Person>, Failure> {
    let trimmed = query.trim();
    if trimmed.is_empty() || trimmed.chars().count() > central_api::social::MAX_SEARCH_QUERY {
        return Ok(Vec::new());
    }
    match social(SocialRequest::Search {
        query: trimmed.to_owned(),
    })
    .await?
    {
        SocialResponse::People { people }
            if people.len() <= central_api::social::MAX_SEARCH_RESULTS
                && people.iter().all(person) =>
        {
            Ok(people)
        }
        _ => Err(Failure::Malformed),
    }
}

/// Percent-encodes a UTF-8 query parameter component (RFC 3986 unreserved set).
#[must_use]
pub fn encode_query_component(text: &str) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(char::from(byte));
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

/// A `UUIDv7` as Central writes it: lowercase, hyphenated.
#[must_use]
pub fn uuid_v7(id: &str) -> bool {
    let b = id.as_bytes();
    uuid_any(id) && b[14] == b'7' && matches!(b[19], b'8' | b'9' | b'a' | b'b')
}

/// Searches Baylee Central's authenticated card catalog for localized and
/// English card matches (strictly scoped to `lang` and `en`), optionally
/// filtered by set code/name, collector number, or finish.
///
/// # Errors
/// The refusal, or an answer exceeding protocol bounds.
pub async fn suggest_cards_rich(
    query: &str,
    lang: crate::words::Lang,
    set_filter: Option<&str>,
    collector_filter: Option<&str>,
    finish_filter: Option<&str>,
) -> Result<Vec<CardSuggestion>, Failure> {
    let trimmed = query.trim();
    if trimmed.is_empty() || trimmed.chars().count() > MAX_CARD_QUERY {
        return Ok(Vec::new());
    }
    match social(SocialRequest::CardSuggest {
        query: trimmed.to_owned(),
        lang: lang.code().to_owned(),
        set_filter: clean_opt_filter(set_filter),
        collector_filter: clean_opt_filter(collector_filter),
        finish_filter: clean_opt_filter(finish_filter),
    })
    .await?
    {
        SocialResponse::CardSuggestions { items }
            if items.len() <= MAX_CARD_RESULTS && items.iter().all(card_suggestion_valid) =>
        {
            Ok(items)
        }
        _ => Err(Failure::Malformed),
    }
}

/// Fetches printings and metadata for one card name (scoped by `lang_tag` and
/// English fallback) over the authenticated Central RPC channel.
///
/// # Errors
/// The refusal, or an answer exceeding bounds.
pub async fn card_printings_lang(
    name: &str,
    lang_tag: &str,
    set_filter: Option<&str>,
    collector_filter: Option<&str>,
    finish_filter: Option<&str>,
) -> Result<(Option<CardSuggestion>, Vec<CardPrinting>), Failure> {
    let trimmed = name.trim();
    if trimmed.is_empty() || trimmed.chars().count() > 128 {
        return Ok((None, Vec::new()));
    }
    let clean_lang = clean_opt_filter(Some(lang_tag)).unwrap_or_else(|| "en".to_owned());
    match social(SocialRequest::CardPrintings {
        name: trimmed.to_owned(),
        lang: clean_lang,
        set_filter: clean_opt_filter(set_filter),
        collector_filter: clean_opt_filter(collector_filter),
        finish_filter: clean_opt_filter(finish_filter),
    })
    .await?
    {
        SocialResponse::CardPrintings { card, printings }
            if card.as_ref().is_none_or(card_suggestion_valid)
                && printings.len() <= MAX_CARD_PRINTINGS
                && printings.iter().all(card_printing_valid) =>
        {
            Ok((card, printings))
        }
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

/// Creator-owned friend invites list and capabilities.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FriendInvitesList {
    /// Invites newest-first.
    pub invites: Vec<FriendInviteSummary>,
    /// Maximum active friend invites allowed per account.
    pub active_limit: u32,
    /// Whether outbound invitation email delivery is configured on Central.
    pub mail_available: bool,
    /// Recruiter medal progress for the caller, when returned.
    pub recruiter_badge: Option<SocialBadge>,
}

/// Newly minted single-use friend invite returned once by Central.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreatedFriendInvite {
    /// Summary record for immediate list/metadata display.
    pub invite: FriendInviteSummary,
    /// Raw single-use 22-character base64url invite code.
    pub code: String,
    /// Whether Central handed an email to its mailer.
    pub mail_sent: bool,
}

/// Outcome of redeeming a friend invite while already signed in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RedeemedFriendInvite {
    /// Inviter who is now a mutual contact.
    pub inviter: Person,
    /// Whether an email confirmation code was also dispatched.
    pub email_pending: bool,
}

/// Extracts a canonical 128-bit friend invite code from a raw code, an
/// `/invite/{code}` URL, or a `/register?invite={code}` URL.
#[must_use]
pub fn extract_invite_code(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    if valid_friend_invite_code(trimmed) {
        return Some(trimmed.to_owned());
    }
    if let Some((_, query)) = trimmed.split_once('?') {
        let query = query.split('#').next().unwrap_or(query);
        for pair in query.split('&') {
            if let Some(val) = pair.strip_prefix("invite=")
                && valid_friend_invite_code(val)
            {
                return Some(val.to_owned());
            }
        }
    }
    let path = trimmed
        .split(['?', '#'])
        .next()
        .unwrap_or(trimmed)
        .trim_end_matches('/');
    let last = path.rsplit('/').next()?;
    valid_friend_invite_code(last).then(|| last.to_owned())
}

fn invite_summary_valid(item: &FriendInviteSummary) -> bool {
    item.created_at <= item.expires_at && item.redeemed_by.as_ref().is_none_or(person)
}

/// Lists all friend invites created by the signed-in account.
///
/// # Errors
/// The refusal, or an answer breaking validation bounds.
pub async fn friend_invites() -> Result<FriendInvitesList, Failure> {
    match social(SocialRequest::FriendInvites).await? {
        SocialResponse::FriendInvites {
            invites,
            active_limit,
            mail_available,
            recruiter_badge,
        } if active_limit > 0 && invites.iter().all(invite_summary_valid) => {
            Ok(FriendInvitesList {
                invites,
                active_limit,
                mail_available,
                recruiter_badge,
            })
        }
        _ => Err(Failure::Malformed),
    }
}

/// Creates a personal single-use friend invite link.
///
/// # Errors
/// The refusal, or an answer breaking validation bounds.
pub async fn create_friend_invite(
    label: Option<String>,
    note: Option<String>,
    email: Option<String>,
    lang: &str,
    expires_days: Option<u8>,
) -> Result<CreatedFriendInvite, Failure> {
    let clean = |s: Option<String>| -> Option<String> {
        s.map(|v| v.trim().to_owned()).filter(|v| !v.is_empty())
    };
    let days = expires_days.map(|d| d.clamp(1, MAX_FRIEND_INVITE_DAYS));
    match social(SocialRequest::CreateFriendInvite {
        label: clean(label),
        note: clean(note),
        email: clean(email),
        lang: Some(lang.to_owned()),
        expires_days: days,
    })
    .await?
    {
        SocialResponse::FriendInviteCreated {
            invite,
            code,
            mail_sent,
        } if valid_friend_invite_code(&code) && invite_summary_valid(&invite) => {
            Ok(CreatedFriendInvite {
                invite,
                code,
                mail_sent,
            })
        }
        _ => Err(Failure::Malformed),
    }
}

/// Revokes an active friend invite owned by the signed-in account.
///
/// # Errors
/// The refusal.
pub async fn revoke_friend_invite(invite: [u8; 16]) -> Result<(), Failure> {
    match social(SocialRequest::RevokeFriendInvite { invite }).await? {
        SocialResponse::Done => Ok(()),
        _ => Err(Failure::Malformed),
    }
}

/// Redeems a friend invite code (or invite link) for the signed-in account.
///
/// # Errors
/// Returns [`Failure::Rejected`] when the input is malformed, or Central's refusal.
pub async fn redeem_friend_invite(
    raw_code: &str,
    link_email: bool,
    lang: &str,
) -> Result<RedeemedFriendInvite, Failure> {
    let Some(code) = extract_invite_code(raw_code) else {
        return Err(Failure::Rejected);
    };
    match social(SocialRequest::RedeemFriendInvite {
        code,
        link_email,
        lang: Some(lang.to_owned()),
    })
    .await?
    {
        SocialResponse::FriendInviteRedeemed {
            inviter,
            email_pending,
        } if person(&inviter) => Ok(RedeemedFriendInvite {
            inviter,
            email_pending,
        }),
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
            description: String::new(),
            picture_key: None,
            picture_moving: false,
            members: vec![central_api::groups::Member {
                person: erika.clone(),
                owner: true,
                admin: true,
            }],
            invited: vec![erika],
        };
        let snapshot = Groups {
            groups: vec![g],
            invites: Vec::new(),
        };
        assert!(!groups_snapshot(&snapshot), "member and invited at once");
        assert_eq!(encode_query_component("Sol Ring"), "Sol%20Ring");
        assert_eq!(encode_query_component("Urza's Saga"), "Urza%27s%20Saga");
    }

    #[test]
    fn friend_invite_code_extraction() {
        let code = "AbCdEfGhIjKlMnOpQrStUg";
        assert!(valid_friend_invite_code(code));
        assert_eq!(extract_invite_code(code).as_deref(), Some(code));
        assert_eq!(
            extract_invite_code(&format!("https://central.baylee.app/de/invite/{code}")).as_deref(),
            Some(code)
        );
        assert_eq!(
            extract_invite_code(&format!("/invite/{code}/?utm=1#top")).as_deref(),
            Some(code)
        );
        assert_eq!(
            extract_invite_code(&format!(
                "https://central.baylee.app/en/register?foo=1&invite={code}#card"
            ))
            .as_deref(),
            Some(code)
        );
        assert!(extract_invite_code("not-a-valid-code").is_none());
        assert!(extract_invite_code("   ").is_none());
    }
}
