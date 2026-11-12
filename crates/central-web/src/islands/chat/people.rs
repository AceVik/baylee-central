//! Contacts and groups: finding a person by handle, requests and blocks,
//! creating a group, invitations, and a group's members and settings.
//! A step that cannot be undone asks once more in place, naming whom or
//! what it is about.

use central_api::groups::{Change, Group};
use central_api::social::{Contact, ContactAction, Person, Relation};
use leptos::prelude::*;

use super::glyph::{self, glyph};
use super::model::{self, direct, handle, tag_of};
use super::talk::avatar;
use super::{Ctl, wire};
use crate::words::{self, Lang};

/// A step that asks once more: which one, for whom.
type Asking = RwSignal<Option<(u32, &'static str)>>;

fn person_line(p: &Person) -> impl IntoView + use<> {
    let tag = p.tag.to_string();
    view! {
        {avatar(&p.display_name, &tag, false, Some(p.tag))}
        <bdi class="person-name" translate="no">{handle(p)}</bdi>
    }
}

/// The Contacts half.
pub fn contacts_panel(
    lang: Lang,
    own: u32,
    own_handle: String,
    ctl: Ctl,
    choose: Callback<String>,
) -> impl IntoView {
    let asking: Asking = RwSignal::new(None);
    let section = move |relation: Relation, title: &'static str| {
        move || {
            let list: Vec<Contact> = ctl.contacts.with(|c| match c {
                Some(Ok(list)) => list
                    .iter()
                    .filter(|c| c.relation == relation)
                    .cloned()
                    .collect(),
                _ => Vec::new(),
            });
            (!list.is_empty()).then(|| {
                let items = list
                    .into_iter()
                    .map(|c| contact_row(lang, own, &c, ctl, asking, choose))
                    .collect_view();
                view! {
                    <section class="people">
                        <h2>{title}</h2>
                        <ul>{items}</ul>
                    </section>
                }
            })
        }
    };
    let state = move || {
        match ctl.contacts.with(|c| c.as_ref().map(Result::is_ok)) {
        None => Some(view! { <p class="msgr-note" role="status">{words::account_loading(lang)}</p> }.into_any()),
        Some(false) => Some(view! {
            <p class="banner bad" role="alert">
                <span>{words::people_failed(lang)}</span>
                <button type="button" class="button small" on:click=move |_| ctl.reload_contacts()>{words::chat_retry(lang)}</button>
            </p>
        }
        .into_any()),
        Some(true) if ctl.contacts.with(|c| matches!(c, Some(Ok(list)) if list.is_empty())) => {
            Some(view! { <p class="msgr-note">{words::people_empty(lang)}</p> }.into_any())
        }
        Some(true) => None,
    }
    };
    view! {
        <div class="msgr-panel">
            <p class="msgr-own">
                <span>{words::people_your_handle(lang)}</span>" "
                <strong translate="no" class="selectable">{own_handle}</strong>
            </p>
            {find(lang, own, ctl)}
            {state}
            {section(Relation::Incoming, words::people_incoming_title(lang))}
            {section(Relation::Friend, words::people_friends(lang))}
            {section(Relation::Outgoing, words::people_outgoing(lang))}
            {section(Relation::Blocked, words::people_blocked(lang))}
        </div>
    }
}

fn find(lang: Lang, own: u32, ctl: Ctl) -> impl IntoView {
    let input = RwSignal::new(String::new());
    let found = RwSignal::new(None::<Result<Option<Person>, &'static str>>);
    let sent = RwSignal::new(false);
    let submit = move |event: leptos::ev::SubmitEvent| {
        event.prevent_default();
        sent.set(false);
        let Some(tag) = tag_of(&input.get_untracked()) else {
            found.set(Some(Err(words::people_handle_hint(lang))));
            return;
        };
        if tag == own {
            found.set(Some(Err(words::people_yourself(lang))));
            return;
        }
        leptos::task::spawn_local(async move {
            let answer = match wire::lookup(tag).await {
                Ok(Some(p)) => Ok(Some(p)),
                Ok(None) => Err(words::people_nobody(lang)),
                Err(_) => Err(words::people_failed(lang)),
            };
            found.set(Some(answer));
        });
    };
    let result = move || {
        found.get().map(|answer| match answer {
            Ok(Some(p)) => {
                let tag = p.tag;
                let relation = ctl.contacts.with(|c| match c {
                    Some(Ok(list)) => list.iter().find(|c| c.person.tag == tag).map(|c| c.relation),
                    _ => None,
                });
                let action = match relation {
                    None => view! {
                        <button type="button" class="button small primary" aria-disabled=move || ctl.social_busy.get().then_some("true")
                            on:click=move |_| if !ctl.social_busy.get_untracked() {
                                ctl.contact(tag, ContactAction::Request, move |ok| sent.set(ok));
                            }>
                            {glyph(glyph::USER_PLUS)}<span>{words::people_request(lang)}</span>
                        </button>
                    }
                    .into_any(),
                    Some(r) => view! { <span class="pill">{relation_name(lang, r)}</span> }.into_any(),
                };
                view! {
                    <div class="person found">
                        <span class="person-line">{person_line(&p)}</span>
                        {action}
                    </div>
                    {move || sent.get().then(|| view! { <p class="ok" role="status">{words::people_requested(lang)}</p> })}
                }
                .into_any()
            }
            Ok(None) => ().into_any(),
            Err(text) => view! { <p class="error" role="alert">{text}</p> }.into_any(),
        })
    };
    view! {
        <form class="msgr-add" on:submit=submit>
            <label for="contact-find">{words::people_find(lang)}</label>
            <div class="with-tool">
                <input id="contact-find" type="text" autocomplete="off" spellcheck="false" autocapitalize="none"
                    placeholder="Name#1a2b" aria-describedby="contact-find-hint"
                    on:input=move |ev| input.set(event_target_value(&ev)) />
                <button type="submit" class="button small">{glyph(glyph::SEARCH)}<span>{words::people_find_button(lang)}</span></button>
            </div>
            <p id="contact-find-hint" class="hint">{words::people_handle_hint(lang)}</p>
            {result}
        </form>
    }
}

fn relation_name(lang: Lang, relation: Relation) -> &'static str {
    match relation {
        Relation::Friend => words::people_is_friend(lang),
        Relation::Incoming => words::people_is_incoming(lang),
        Relation::Outgoing => words::people_is_outgoing(lang),
        Relation::Blocked => words::people_is_blocked(lang),
    }
}

/// A button for one change; `ask` first asks once more in place.
fn act(
    ctl: Ctl,
    asking: Asking,
    tag: u32,
    change: ContactAction,
    label: &'static str,
    ask: bool,
) -> impl IntoView {
    let key = label;
    let run = move || {
        ctl.contact(tag, change, move |ok| {
            if ok {
                asking.set(None);
            }
        });
    };
    let class = if matches!(change, ContactAction::Block | ContactAction::Remove) {
        "button small danger"
    } else {
        "button small"
    };
    view! {
        <button type="button" class=class aria-disabled=move || ctl.social_busy.get().then_some("true")
            on:click=move |_| {
                if ctl.social_busy.get_untracked() {
                    return;
                }
                if ask {
                    asking.set(Some((tag, key)));
                } else {
                    run();
                }
            }>{label}</button>
    }
}

fn contact_row(
    lang: Lang,
    own: u32,
    c: &Contact,
    ctl: Ctl,
    asking: Asking,
    choose: Callback<String>,
) -> AnyView {
    let tag = c.person.tag;
    let who = handle(&c.person);
    let actions = match c.relation {
        Relation::Friend => view! {
            <a class="button small primary" href=format!("/{}/account/chat?c={}", lang.code(), direct(own, tag))
                on:click=move |event: leptos::ev::MouseEvent| {
                    event.prevent_default();
                    choose.run(direct(own, tag));
                }>{glyph(glyph::MESSAGE)}<span>{words::people_write(lang)}</span></a>
            {act(ctl, asking, tag, ContactAction::Remove, words::people_remove(lang), true)}
            {act(ctl, asking, tag, ContactAction::Block, words::people_block(lang), true)}
        }
        .into_any(),
        Relation::Incoming => view! {
            {act(ctl, asking, tag, ContactAction::Accept, words::people_accept(lang), false)}
            {act(ctl, asking, tag, ContactAction::Remove, words::people_decline(lang), false)}
            {act(ctl, asking, tag, ContactAction::Block, words::people_block(lang), true)}
        }
        .into_any(),
        Relation::Outgoing => act(ctl, asking, tag, ContactAction::Remove, words::people_cancel(lang), false).into_any(),
        Relation::Blocked => act(ctl, asking, tag, ContactAction::Unblock, words::people_unblock(lang), false).into_any(),
    };
    let question = move || {
        asking.get().filter(|(t, _)| *t == tag).map(|(_, which)| {
            let block = which == words::people_block(lang);
            let (text, change) = if block {
                (words::people_confirm_block(lang, &who), ContactAction::Block)
            } else {
                (words::people_confirm_remove(lang, &who), ContactAction::Remove)
            };
            view! {
                <div class="ask" role="group" aria-label=which>
                    <p>{text}</p>
                    <button type="button" class="button small danger"
                        on:click=move |_| ctl.contact(tag, change, move |ok| if ok { asking.set(None) })>{which}</button>
                    <button type="button" class="button small" on:click=move |_| asking.set(None)>{words::people_keep(lang)}</button>
                </div>
            }
        })
    };
    view! {
        <li class="person">
            <span class="person-line">{person_line(&c.person)}</span>
            <span class="person-actions">{actions}</span>
            {question}
        </li>
    }
    .into_any()
}

/// The form that creates a group.
pub fn new_group(lang: Lang, ctl: Ctl, created: Callback<String>) -> impl IntoView {
    let name = RwSignal::new(String::new());
    let problem = RwSignal::new(None::<&'static str>);
    let submit = move |event: leptos::ev::SubmitEvent| {
        event.prevent_default();
        let text = name.get_untracked();
        if !central_api::groups::valid_name(&text) {
            problem.set(Some(words::chat_group_name_hint(lang)));
            return;
        }
        if ctl.social_busy.get_untracked() {
            return;
        }
        problem.set(None);
        ctl.create_group(text, move |id| match id {
            Some(id) => created.run(id),
            None => problem.set(Some(words::chat_group_failed(lang))),
        });
    };
    view! {
        <form id="new-group" class="msgr-add" on:submit=submit>
            <label for="new-group-name">{words::chat_group_name(lang)}</label>
            <div class="with-tool">
                <input id="new-group-name" type="text" maxlength="96" autocomplete="off"
                    on:input=move |ev| name.set(event_target_value(&ev)) />
                <button type="submit" class="button small primary">{words::chat_group_create(lang)}</button>
            </div>
            <p class="error" role="alert">{move || problem.get()}</p>
        </form>
    }
}

/// Invitations to groups, to join or decline.
pub fn invites(lang: Lang, own: u32, ctl: Ctl, choose: Callback<String>) -> impl IntoView {
    move || {
        let list = ctl.groups.with(|g| match g {
            Some(Ok(snapshot)) => snapshot.invites.clone(),
            _ => Vec::new(),
        });
        (!list.is_empty()).then(|| {
            let items = list
                .into_iter()
                .map(|invite| {
                    let (join, decline) = (invite.group.clone(), invite.group.clone());
                    view! {
                        <li class="person">
                            <span class="person-line">
                                {avatar(&invite.name, &format!("grp:{}", invite.group), true, None)}
                                <span><bdi class="person-name">{invite.name.clone()}</bdi>
                                    <small>{words::chat_invited_by(lang, &handle(&invite.inviter))}</small></span>
                            </span>
                            <span class="person-actions">
                                <button type="button" class="button small primary" on:click=move |_| {
                                    let id = join.clone();
                                    ctl.group(id.clone(), Change::Join, move |ok| if ok { choose.run(format!("grp:{id}")) });
                                }>{words::people_accept(lang)}</button>
                                <button type="button" class="button small" on:click=move |_| {
                                    ctl.group(decline.clone(), Change::Withdraw { tag: own }, |_| {});
                                }>{words::people_decline(lang)}</button>
                            </span>
                        </li>
                    }
                })
                .collect_view();
            view! {
                <section class="people">
                    <h2>{words::chat_invites(lang)}</h2>
                    <ul>{items}</ul>
                </section>
            }
        })
    }
}

/// A group's members and settings, in its conversation's details.
pub fn group_panel(
    lang: Lang,
    group: &Group,
    own: u32,
    ctl: Ctl,
    friends: Memo<Vec<Person>>,
    selected: RwSignal<Option<String>>,
) -> AnyView {
    let owner = group.members.iter().any(|m| m.owner && m.person.tag == own);
    let asking: Asking = RwSignal::new(None);
    let id = StoredValue::new(group.id.clone());
    let change = move |what: Change| {
        ctl.group(id.get_value(), what, move |ok| {
            if ok {
                asking.set(None);
            }
        });
    };
    let rename = owner.then(|| {
        let name = RwSignal::new(group.name.clone());
        view! {
            <form class="msgr-add" on:submit=move |event: leptos::ev::SubmitEvent| {
                event.prevent_default();
                let text = name.get_untracked();
                if central_api::groups::valid_name(&text) {
                    change(Change::Rename { name: text });
                }
            }>
                <label for="group-rename">{words::chat_group_name(lang)}</label>
                <div class="with-tool">
                    <input id="group-rename" type="text" maxlength="96" prop:value=move || name.get()
                        on:input=move |ev| name.set(event_target_value(&ev)) />
                    <button type="submit" class="button small">{words::chat_group_rename(lang)}</button>
                </div>
            </form>
        }
    });
    let leave = Leave {
        lang,
        own,
        owner,
        name: group.name.clone(),
        asking,
        ctl,
        id,
        selected,
    };
    view! {
        <section class="group-panel" aria-labelledby="group-panel-title">
            <h4 id="group-panel-title">{glyph(glyph::USERS)}<span>{words::chat_group_members(lang, u64::try_from(group.members.len()).unwrap_or(0))}</span></h4>
            {rename}
            {members(lang, group, own, owner, asking, change)}
            {invite(lang, group, friends, change)}
            {invited(lang, group, owner, change)}
            <p class="error" role="alert">{move || ctl.social_failed.get().then(|| words::chat_group_failed(lang))}</p>
            <div class="group-leave">{move || leave.clone().view()}</div>
        </section>
    }
    .into_any()
}

fn members(
    lang: Lang,
    group: &Group,
    own: u32,
    owner: bool,
    asking: Asking,
    change: impl Fn(Change) + Copy + Send + Sync + 'static,
) -> AnyView {
    let items = group
        .members
        .iter()
        .map(|m| {
            let tag = m.person.tag;
            let who = handle(&m.person);
            let tools = (owner && tag != own).then(|| view! {
                <span class="person-actions">
                    <button type="button" class="button small" on:click=move |_| asking.set(Some((tag, "transfer")))>
                        {words::chat_group_transfer(lang)}</button>
                    <button type="button" class="button small danger" on:click=move |_| asking.set(Some((tag, "remove")))>
                        {words::chat_group_remove(lang)}</button>
                </span>
            });
            let question = move || {
                asking.get().filter(|(t, _)| *t == tag).map(|(_, which)| {
                    let (text, label, what): (String, &'static str, fn(u32) -> Change) = if which == "transfer" {
                        (words::chat_group_transfer_ask(lang, &who), words::chat_group_transfer(lang), |tag| Change::Transfer { tag })
                    } else {
                        (words::chat_group_remove_ask(lang, &who), words::chat_group_remove(lang), |tag| Change::Remove { tag })
                    };
                    view! {
                        <div class="ask" role="group" aria-label=label>
                            <p>{text}</p>
                            <button type="button" class="button small danger" on:click=move |_| change(what(tag))>{label}</button>
                            <button type="button" class="button small" on:click=move |_| asking.set(None)>{words::people_keep(lang)}</button>
                        </div>
                    }
                })
            };
            view! {
                <li class="person">
                    <span class="person-line">{person_line(&m.person)}
                        {m.owner.then(|| view! { <span class="pill">{words::chat_group_owner(lang)}</span> })}</span>
                    {tools}
                    {question}
                </li>
            }
        })
        .collect_view();
    view! { <ul class="people-list">{items}</ul> }.into_any()
}

fn invite<F: Fn(Change) + Copy + Send + Sync + 'static>(
    lang: Lang,
    group: &Group,
    friends: Memo<Vec<Person>>,
    change: F,
) -> impl IntoView + use<F> {
    let taken: Vec<u32> = group
        .members
        .iter()
        .map(|m| m.person.tag)
        .chain(group.invited.iter().map(|p| p.tag))
        .collect();
    let full = taken.len() >= central_api::groups::MAX_MEMBERS;
    let choice = RwSignal::new(String::new());
    move || {
        let free: Vec<Person> = friends.with(|f| {
            f.iter()
                .filter(|p| !taken.contains(&p.tag))
                .cloned()
                .collect()
        });
        (!free.is_empty() && !full).then(|| {
            let options = free
                .iter()
                .map(|p| view! { <option value=model::tag_text(p.tag)>{handle(p)}</option> })
                .collect_view();
            view! {
                <form class="msgr-add" on:submit=move |event: leptos::ev::SubmitEvent| {
                    event.prevent_default();
                    if let Some(tag) = tag_of(&choice.get_untracked()) {
                        change(Change::Invite { tag });
                        choice.set(String::new());
                    }
                }>
                    <label for="group-invite">{words::chat_group_invite(lang)}</label>
                    <div class="with-tool">
                        <select id="group-invite" on:change=move |ev| choice.set(event_target_value(&ev)) prop:value=move || choice.get()>
                            <option value="">{words::chat_group_choose(lang)}</option>
                            {options}
                        </select>
                        <button type="submit" class="button small">{words::chat_group_invite_button(lang)}</button>
                    </div>
                </form>
            }
        })
    }
}

fn invited(
    lang: Lang,
    group: &Group,
    owner: bool,
    change: impl Fn(Change) + Copy + Send + Sync + 'static,
) -> Option<AnyView> {
    (!group.invited.is_empty()).then(|| {
        let items = group
            .invited
            .iter()
            .map(|p| {
                let tag = p.tag;
                let withdraw = owner.then(|| view! {
                    <button type="button" class="button small" on:click=move |_| change(Change::Withdraw { tag })>
                        {words::people_cancel(lang)}</button>
                });
                view! { <li class="person"><span class="person-line">{person_line(p)}</span>{withdraw}</li> }
            })
            .collect_view();
        view! { <h4>{words::chat_group_invited(lang)}</h4><ul class="people-list">{items}</ul> }.into_any()
    })
}

/// Leaving a group, or deleting it as its owner: asked once more.
#[derive(Clone)]
struct Leave {
    lang: Lang,
    own: u32,
    owner: bool,
    name: String,
    asking: Asking,
    ctl: Ctl,
    id: StoredValue<String>,
    selected: RwSignal<Option<String>>,
}

impl Leave {
    fn view(self) -> AnyView {
        let Leave {
            lang,
            own,
            owner,
            name,
            asking,
            ctl,
            id,
            selected,
        } = self;
        let label = if owner {
            words::chat_group_delete(lang)
        } else {
            words::chat_group_leave(lang)
        };
        if asking.get().is_none_or(|(t, _)| t != 0) {
            return view! {
                <button type="button" class="button small danger" on:click=move |_| asking.set(Some((0, "leave")))>{label}</button>
            }
            .into_any();
        }
        let question = if owner {
            words::chat_group_delete_ask(lang, &name)
        } else {
            words::chat_group_leave_ask(lang, &name)
        };
        view! {
            <div class="ask" role="group" aria-label=label>
                <p>{question}</p>
                <button type="button" class="button small danger" on:click=move |_| {
                    let what = if owner { Change::Delete } else { Change::Remove { tag: own } };
                    ctl.group(id.get_value(), what, move |ok| if ok { selected.set(None) });
                }>{label}</button>
                <button type="button" class="button small" on:click=move |_| asking.set(None)>{words::people_keep(lang)}</button>
            </div>
        }
        .into_any()
    }
}
