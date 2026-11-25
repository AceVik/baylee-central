//! Contacts and groups: finding a person by name or handle, requests and
//! blocks, creating a group, invitations, and WhatsApp-style group profiles,
//! admin roles, and autocomplete invites.
//! A step that cannot be undone asks once more in place, naming whom or
//! what it is about.

use std::time::Duration;

use central_api::groups::{Change, Group, Member};
use central_api::social::{Contact, ContactAction, Person, Relation};
use leptos::prelude::*;

use super::super::field;
use super::super::ui::action_menu::{ActionItem, action_menu};
use super::super::ui::copy_handle::handle_with_copy;
use super::super::ui::invite_dialog::invite_dialog;
use super::glyph::{self, Shape, glyph};
use super::model::{self, direct, handle, tag_of};
use super::talk::{avatar, group_avatar};
use super::{Ctl, wire};
use crate::words::{self, Lang};

/// A step that asks once more: which one, for whom.
type Asking = RwSignal<Option<(u32, &'static str)>>;

const SEARCH_DEBOUNCE: Duration = Duration::from_millis(250);

/// A person's face and copyable handle (`Name#1a2b`).
fn person_line(lang: Lang, p: &Person) -> impl IntoView + use<> {
    let tag = p.tag.to_string();
    let tag_hex = model::tag_text(p.tag);
    view! {
        {avatar(&p.display_name, &tag, false, Some(p.tag))}
        <span class="person-handle">
            {handle_with_copy(lang, p.display_name.clone(), tag_hex, true)}
        </span>
    }
}

#[allow(clippy::too_many_arguments)]
fn contacts_section(
    lang: Lang,
    own: u32,
    ctl: Ctl,
    asking: Asking,
    choose: Callback<String>,
    shown: RwSignal<usize>,
    relation: Relation,
    title: &'static str,
) -> impl IntoView {
    let kind = match relation {
        Relation::Incoming => "incoming",
        Relation::Friend => "friend",
        Relation::Outgoing => "outgoing",
        Relation::Blocked => "blocked",
    };
    move || {
        let limit = shown.get();
        let list: Vec<Contact> = ctl.contacts.with(|c| match c {
            Some(Ok(list)) => list
                .iter()
                .filter(|c| c.relation == relation)
                .take(limit)
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
                <section class="people" data-relation=kind>
                    <h2>{title}</h2>
                    <ul>{items}</ul>
                </section>
            }
        })
    }
}

fn contacts_state(lang: Lang, ctl: Ctl) -> impl IntoView {
    move || {
        match ctl.contacts.with(|c| c.as_ref().map(Result::is_ok)) {
        None => Some(
            view! { <p class="msgr-note" role="status">{words::account_loading(lang)}</p> }
                .into_any(),
        ),
        Some(false) => Some(
            view! {
                <p class="banner bad" role="alert">
                    {glyph(glyph::INFO)}<span>{words::people_failed(lang)}</span>
                    <button type="button" class="button small quiet" on:click=move |_| ctl.reload_contacts()>
                        {glyph(glyph::REFRESH_CW)}
                        <span>{words::chat_retry(lang)}</span>
                    </button>
                </p>
            }
            .into_any(),
        ),
        Some(true)
            if ctl
                .contacts
                .with(|c| matches!(c, Some(Ok(list)) if list.is_empty())) =>
        {
            Some(
                view! {
                    <div class="msgr-empty">
                        <span class="msgr-empty-icon">{glyph(glyph::USERS)}</span>
                        <p class="msgr-empty-title">{words::chat_quiet(lang)}</p>
                        <p>{words::people_empty(lang)}</p>
                    </div>
                }
                .into_any(),
            )
        }
        Some(true) => None,
    }
    }
}

fn redeem_feedback_view(
    lang: Lang,
    own: u32,
    outcome: RwSignal<Option<Result<Person, ()>>>,
    choose: Callback<String>,
) -> AnyView {
    view! {
        {move || match outcome.get() {
            Some(Ok(inviter)) => {
                let tag = inviter.tag;
                let text = words::invite_page_accepted(lang, &handle(&inviter));
                Some(
                    view! {
                        <div class="invite-redeem-ok" role="status">
                            <span>{text}</span>
                            <button
                                type="button"
                                class="button small quiet"
                                on:click=move |_| choose.run(direct(own, tag))
                            >
                                {glyph(glyph::MESSAGE)}
                                <span>{words::invite_page_open_chat(lang)}</span>
                            </button>
                        </div>
                    }
                    .into_any(),
                )
            }
            Some(Err(())) => {
                Some(view! { <p class="error" role="alert">{words::invite_redeem_failed(lang)}</p> }.into_any())
            }
            None => None,
        }}
    }
    .into_any()
}

fn invite_bar(
    lang: Lang,
    own: u32,
    ctl: Ctl,
    invite_open: RwSignal<bool>,
    choose: Callback<String>,
) -> AnyView {
    let code_input = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let outcome = RwSignal::new(None::<Result<Person, ()>>);

    let submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        if busy.get_untracked() {
            return;
        }
        let raw = code_input.get_untracked();
        if wire::extract_invite_code(&raw).is_none() {
            outcome.set(Some(Err(())));
            return;
        }
        busy.set(true);
        outcome.set(None);
        let lang_code = lang.code();
        leptos::task::spawn_local(async move {
            let res = wire::redeem_friend_invite(&raw, false, lang_code).await;
            busy.set(false);
            match res {
                Ok(redeemed) => {
                    code_input.set(String::new());
                    outcome.set(Some(Ok(redeemed.inviter)));
                    ctl.reload_contacts();
                    ctl.bump_invites();
                }
                Err(_) => outcome.set(Some(Err(()))),
            }
        });
    };

    view! {
        <section class="people-invites-bar" aria-label=words::invite_open_btn(lang)>
            <div class="people-invite-top">
                <button
                    type="button"
                    id="people-invite-btn"
                    class="button small primary"
                    aria-expanded=move || invite_open.get().to_string()
                    aria-controls="friend-invite-dialog"
                    on:click=move |_| invite_open.set(true)
                >
                    {glyph(glyph::USER_PLUS)}
                    <span>{words::invite_open_btn(lang)}</span>
                </button>
            </div>
            <form class="invite-redeem-form" on:submit=submit>
                <label class="invite-redeem-label" for="invite-redeem-input">
                    {words::invite_redeem_box_title(lang)}
                </label>
                <div class="invite-redeem-row">
                    <div class="inset">
                        {field::mark(glyph::LINK)}
                        <input
                            id="invite-redeem-input"
                            type="text"
                            autocomplete="off"
                            spellcheck="false"
                            autocapitalize="none"
                            placeholder=words::invite_redeem_placeholder(lang)
                            prop:value=move || code_input.get()
                            on:input=move |ev| {
                                code_input.set(event_target_value(&ev));
                                outcome.set(None);
                            }
                        />
                    </div>
                    <button
                        type="submit"
                        class="button small quiet"
                        aria-disabled=move || busy.get().then_some("true")
                    >
                        {glyph(glyph::KEY)}
                        <span>{words::invite_redeem_submit(lang)}</span>
                    </button>
                </div>
                {redeem_feedback_view(lang, own, outcome, choose)}
            </form>
        </section>
    }
    .into_any()
}

/// The Contacts half.
pub fn contacts_panel(lang: Lang, own: u32, ctl: Ctl, choose: Callback<String>) -> impl IntoView {
    let asking: Asking = RwSignal::new(None);
    let shown = RwSignal::new(40_usize);
    let invite_open = RwSignal::new(false);
    let panel: NodeRef<leptos::html::Div> = NodeRef::new();
    let has_more = Memo::new(move |_| {
        let limit = shown.get();
        ctl.contacts.with(|c| match c {
            Some(Ok(list)) => [
                Relation::Incoming,
                Relation::Friend,
                Relation::Outgoing,
                Relation::Blocked,
            ]
            .into_iter()
            .any(|rel| list.iter().filter(|x| x.relation == rel).count() > limit),
            _ => false,
        })
    });
    let on_scroll = move |_| {
        let Some(node) = panel.get_untracked() else {
            return;
        };
        if node.scroll_height() - node.scroll_top() - node.client_height() < 160
            && has_more.get_untracked()
        {
            shown.update(|n| *n += 40);
        }
    };
    view! {
        <div class="msgr-panel" node_ref=panel on:scroll=on_scroll>
            {invite_bar(lang, own, ctl, invite_open, choose)}
            {find(lang, own, ctl)}
            {contacts_state(lang, ctl)}
            {contacts_section(lang, own, ctl, asking, choose, shown, Relation::Incoming, words::people_incoming_title(lang))}
            {contacts_section(lang, own, ctl, asking, choose, shown, Relation::Friend, words::people_friends(lang))}
            {contacts_section(lang, own, ctl, asking, choose, shown, Relation::Outgoing, words::people_outgoing(lang))}
            {contacts_section(lang, own, ctl, asking, choose, shown, Relation::Blocked, words::people_blocked(lang))}
            {move || has_more.get().then(|| view! {
                <button type="button" class="button small conv-more" on:click=move |_| shown.update(|n| *n += 40)>
                    {glyph(glyph::CHEVRON_RIGHT)}
                    <span>{words::admin_more(lang)}</span>
                </button>
            })}
            {invite_dialog(lang, ctl, invite_open)}
        </div>
    }
}

fn found_person_row(lang: Lang, ctl: Ctl, p: &Person, sent: RwSignal<Option<u32>>) -> AnyView {
    let tag = p.tag;
    let relation = ctl.contacts.with(|c| match c {
        Some(Ok(list)) => list
            .iter()
            .find(|c| c.person.tag == tag)
            .map(|c| c.relation),
        _ => None,
    });
    let action = match relation {
        None => view! {
            <button
                type="button"
                class="button small primary"
                aria-disabled=move || ctl.social_busy.get().then_some("true")
                on:click=move |_| {
                    if !ctl.social_busy.get_untracked() {
                        ctl.contact(tag, ContactAction::Request, move |ok| {
                            if ok {
                                sent.set(Some(tag));
                            }
                        });
                    }
                }
            >
                {glyph(glyph::USER_PLUS)}
                <span>{words::people_request(lang)}</span>
            </button>
        }
        .into_any(),
        Some(r) => view! { <span class="pill">{relation_name(lang, r)}</span> }.into_any(),
    };
    view! {
        <li class="person found">
            <span class="person-line">{person_line(lang, p)}</span>
            {action}
        </li>
    }
    .into_any()
}

fn run_contact_search(
    lang: Lang,
    own: u32,
    raw: &str,
    generation: u32,
    search_gen: StoredValue<u32>,
    found: RwSignal<Option<Result<Vec<Person>, &'static str>>>,
    searching: RwSignal<bool>,
) {
    let trimmed = raw.trim().to_owned();
    if let Some(tag) = tag_of(&trimmed) {
        if tag == own {
            searching.set(false);
            found.set(Some(Err(words::people_yourself(lang))));
            return;
        }
        searching.set(true);
        leptos::task::spawn_local(async move {
            let answer = match wire::lookup(tag).await {
                Ok(Some(p)) => Ok(vec![p]),
                Ok(None) => Err(words::people_nobody(lang)),
                Err(_) => Err(words::people_failed(lang)),
            };
            if search_gen.try_get_value() == Some(generation) {
                searching.set(false);
                found.set(Some(answer));
            }
        });
        return;
    }
    if trimmed.chars().count() < 2 {
        searching.set(false);
        found.set(Some(Err(words::people_handle_hint(lang))));
        return;
    }
    searching.set(true);
    leptos::task::spawn_local(async move {
        let answer = match wire::search_people(&trimmed).await {
            Ok(list) if list.is_empty() => Err(words::people_nobody(lang)),
            Ok(list) => Ok(list),
            Err(_) => Err(words::people_failed(lang)),
        };
        if search_gen.try_get_value() == Some(generation) {
            searching.set(false);
            found.set(Some(answer));
        }
    });
}

fn find_results_view(
    lang: Lang,
    ctl: Ctl,
    found: RwSignal<Option<Result<Vec<Person>, &'static str>>>,
    searching: RwSignal<bool>,
    sent: RwSignal<Option<u32>>,
) -> impl IntoView {
    move || {
        if searching.get() {
            return Some(
                view! { <p class="meta" role="status">{words::chat_group_invite_searching(lang)}</p> }
                    .into_any(),
            );
        }
        found.get().map(|answer| match answer {
            Ok(list) => {
                let rows = list
                    .iter()
                    .map(|p| found_person_row(lang, ctl, p, sent))
                    .collect_view();
                view! {
                    <ul class="people-list">{rows}</ul>
                    {move || sent.get().map(|_| view! { <p class="ok" role="status">{words::people_requested(lang)}</p> })}
                }
                .into_any()
            }
            Err(text) => view! { <p class="error" role="alert">{text}</p> }.into_any(),
        })
    }
}

fn find(lang: Lang, own: u32, ctl: Ctl) -> impl IntoView {
    let input = RwSignal::new(String::new());
    let found = RwSignal::new(None::<Result<Vec<Person>, &'static str>>);
    let searching = RwSignal::new(false);
    let sent = RwSignal::new(None::<u32>);
    let pause = StoredValue::new(None::<TimeoutHandle>);
    let search_gen = StoredValue::new(0_u32);
    on_cleanup(move || {
        if let Some(Some(handle)) = pause.try_get_value() {
            handle.clear();
        }
    });
    let clear_pause = move || {
        if let Some(Some(handle)) = pause.try_get_value() {
            handle.clear();
        }
        let _ = pause.try_set_value(None);
    };
    let on_typed = move |value: String| {
        input.set(value.clone());
        sent.set(None);
        clear_pause();
        let trimmed = value.trim();
        if trimmed.is_empty() {
            search_gen.set_value(search_gen.get_value().wrapping_add(1));
            searching.set(false);
            found.set(None);
            return;
        }
        if trimmed.chars().count() < 2 && tag_of(trimmed).is_none() {
            searching.set(false);
            return;
        }
        let next = search_gen.get_value().wrapping_add(1);
        search_gen.set_value(next);
        let handle = set_timeout(
            move || {
                let _ = pause.try_set_value(None);
                run_contact_search(lang, own, &value, next, search_gen, found, searching);
            },
            SEARCH_DEBOUNCE,
        )
        .ok();
        pause.set_value(handle);
    };
    let submit = move |event: leptos::ev::SubmitEvent| {
        event.prevent_default();
        sent.set(None);
        clear_pause();
        let next = search_gen.get_value().wrapping_add(1);
        search_gen.set_value(next);
        run_contact_search(
            lang,
            own,
            &input.get_untracked(),
            next,
            search_gen,
            found,
            searching,
        );
    };
    view! {
        <form class="msgr-add" on:submit=submit>
            <div class="inset">
                {field::mark(glyph::USER_PLUS)}
                <input id="contact-find" type="text" autocomplete="off" spellcheck="false" autocapitalize="none"
                    aria-label=words::people_find(lang)
                    placeholder=words::chat_contact_placeholder(lang)
                    aria-describedby="contact-find-hint"
                    prop:value=move || input.get()
                    on:input=move |ev| on_typed(event_target_value(&ev)) />
                <button type="submit" class="inset-end"
                    title=words::chat_contact_find_tooltip(lang)
                    aria-label=words::chat_contact_find_tooltip(lang)>
                    {glyph(glyph::SEARCH)}
                </button>
            </div>
            <p id="contact-find-hint" class="hint">{words::people_handle_hint(lang)}</p>
            {find_results_view(lang, ctl, found, searching, sent)}
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
    icon: &'static [Shape],
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
    let class = match change {
        ContactAction::Block | ContactAction::Remove => "button small danger",
        ContactAction::Accept => "button small",
        _ => "button small quiet",
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
            }>{glyph(icon)}<span>{label}</span></button>
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
    let menu_label = words::people_more_actions(lang).to_owned();
    let ask_item = move |label: &'static str, icon: &'static [Shape], danger: bool| ActionItem {
        label: label.to_owned(),
        icon,
        danger,
        on_select: Callback::new(move |()| asking.set(Some((tag, label)))),
    };
    let actions = match c.relation {
        Relation::Friend => view! {
            <a class="button small quiet" href=format!("/{}/account/chat?c={}", lang.code(), direct(own, tag))
                on:click=move |event: leptos::ev::MouseEvent| {
                    event.prevent_default();
                    choose.run(direct(own, tag));
                }>{glyph(glyph::MESSAGE)}<span>{words::people_write(lang)}</span></a>
            {action_menu(
                menu_label,
                vec![
                    ask_item(words::people_remove(lang), glyph::USER_X, true),
                    ask_item(words::people_block(lang), glyph::BAN, true),
                ],
            )}
        }
        .into_any(),
        Relation::Incoming => view! {
            {act(ctl, asking, tag, ContactAction::Accept, words::people_accept(lang), glyph::USER_CHECK, false)}
            {act(ctl, asking, tag, ContactAction::Remove, words::people_decline(lang), glyph::USER_X, false)}
            {action_menu(
                menu_label,
                vec![ask_item(words::people_block(lang), glyph::BAN, true)],
            )}
        }
        .into_any(),
        Relation::Outgoing => {
            act(ctl, asking, tag, ContactAction::Remove, words::people_cancel(lang), glyph::USER_X, false)
                .into_any()
        }
        Relation::Blocked => {
            act(ctl, asking, tag, ContactAction::Unblock, words::people_unblock(lang), glyph::USER_CHECK, false)
                .into_any()
        }
    };
    let question = move || {
        asking.get().filter(|(t, _)| *t == tag).map(|(_, which)| {
            let block = which == words::people_block(lang);
            let (text, change, confirm_icon) = if block {
                (words::people_confirm_block(lang, &who), ContactAction::Block, glyph::BAN)
            } else {
                (words::people_confirm_remove(lang, &who), ContactAction::Remove, glyph::USER_X)
            };
            view! {
                <div class="ask" role="group" aria-label=which>
                    <p>{text}</p>
                    <button type="button" class="button small danger"
                        on:click=move |_| ctl.contact(tag, change, move |ok| if ok { asking.set(None) })>
                        {glyph(confirm_icon)}<span>{which}</span>
                    </button>
                    <button type="button" class="button small quiet" on:click=move |_| asking.set(None)>
                        {glyph(glyph::ROTATE_CCW)}<span>{words::people_keep(lang)}</span>
                    </button>
                </div>
            }
        })
    };
    view! {
        <li class="person">
            <span class="person-line">{person_line(lang, &c.person)}</span>
            <span class="person-actions">{actions}</span>
            {question}
        </li>
    }
    .into_any()
}

#[derive(Clone, Copy)]
struct NewGroupAc {
    lang: Lang,
    own: u32,
    friends: Memo<Vec<Person>>,
    queued: RwSignal<Vec<Person>>,
    query: RwSignal<String>,
    open: RwSignal<bool>,
    others: RwSignal<Vec<Person>>,
    searching: RwSignal<bool>,
    active: RwSignal<usize>,
    pause: StoredValue<Option<TimeoutHandle>>,
    search_gen: StoredValue<u32>,
}

impl NewGroupAc {
    fn new(
        lang: Lang,
        own: u32,
        friends: Memo<Vec<Person>>,
        queued: RwSignal<Vec<Person>>,
    ) -> Self {
        let pause = StoredValue::new(None::<TimeoutHandle>);
        on_cleanup(move || {
            if let Some(Some(handle)) = pause.try_get_value() {
                handle.clear();
            }
        });
        Self {
            lang,
            own,
            friends,
            queued,
            query: RwSignal::new(String::new()),
            open: RwSignal::new(false),
            others: RwSignal::new(Vec::new()),
            searching: RwSignal::new(false),
            active: RwSignal::new(0),
            pause,
            search_gen: StoredValue::new(0),
        }
    }

    fn clear_pause(self) {
        if let Some(Some(handle)) = self.pause.try_get_value() {
            handle.clear();
        }
        let _ = self.pause.try_set_value(None);
    }

    fn is_excluded(self, tag: u32) -> bool {
        tag == self.own || self.queued.with(|list| list.iter().any(|p| p.tag == tag))
    }

    fn is_excluded_untracked(self, tag: u32) -> bool {
        tag == self.own
            || self
                .queued
                .with_untracked(|list| list.iter().any(|p| p.tag == tag))
    }

    fn filtered_contacts(self, needle: &str) -> Vec<Person> {
        let low = needle.trim().to_lowercase();
        self.friends.with(|list| {
            list.iter()
                .filter(|p| !self.is_excluded(p.tag))
                .filter(|p| {
                    low.is_empty()
                        || p.display_name.to_lowercase().contains(&low)
                        || handle(p).to_lowercase().contains(&low)
                })
                .cloned()
                .collect()
        })
    }

    fn filtered_others(self, contacts: &[Person]) -> Vec<Person> {
        self.others.with(|list| {
            list.iter()
                .filter(|p| !self.is_excluded(p.tag) && !contacts.iter().any(|c| c.tag == p.tag))
                .cloned()
                .collect()
        })
    }

    fn on_typed(self, value: &str) {
        self.query.set(value.to_owned());
        self.open.set(true);
        self.active.set(0);
        self.clear_pause();
        let trimmed = value.trim();
        let next = self.search_gen.get_value().wrapping_add(1);
        self.search_gen.set_value(next);
        if trimmed.chars().count() < 2 && tag_of(trimmed).is_none() {
            self.others.set(Vec::new());
            self.searching.set(false);
            return;
        }
        self.searching.set(true);
        let q = trimmed.to_owned();
        let handle = set_timeout(
            move || {
                let _ = self.pause.try_set_value(None);
                leptos::task::spawn_local(async move {
                    let list = wire::search_people(&q).await.unwrap_or_default();
                    if self.search_gen.try_get_value() == Some(next) {
                        self.others.set(list);
                        self.searching.set(false);
                    }
                });
            },
            SEARCH_DEBOUNCE,
        )
        .ok();
        self.pause.set_value(handle);
    }

    fn add_person(self, person: Person) {
        let cap = central_api::groups::MAX_MEMBERS.saturating_sub(1);
        if !self.is_excluded_untracked(person.tag) && self.queued.with_untracked(Vec::len) < cap {
            self.queued.update(|list| list.push(person));
        }
        self.clear_pause();
        self.query.set(String::new());
        self.others.set(Vec::new());
        self.searching.set(false);
        self.open.set(false);
        self.active.set(0);
    }

    fn add_current(self) {
        let q = self.query.get_untracked();
        let contacts = self.filtered_contacts(&q);
        let others = self.filtered_others(&contacts);
        let all: Vec<Person> = contacts.into_iter().chain(others).collect();
        if let Some(person) = all
            .get(self.active.get_untracked())
            .or_else(|| all.first())
            .cloned()
        {
            self.add_person(person);
            return;
        }
        if let Some(tag) = tag_of(&q)
            && !self.is_excluded_untracked(tag)
        {
            leptos::task::spawn_local(async move {
                if let Ok(Some(person)) = wire::lookup(tag).await {
                    self.add_person(person);
                }
            });
        }
    }

    fn handle_keydown(self, event: &leptos::ev::KeyboardEvent) {
        let open = self.open.get_untracked();
        let q = self.query.get_untracked();
        if !open {
            if event.key() == "Enter" && !q.trim().is_empty() {
                event.prevent_default();
                event.stop_propagation();
                self.add_current();
            }
            return;
        }
        let contacts = self.filtered_contacts(&q);
        let others = self.filtered_others(&contacts);
        let len = contacts.len() + others.len();
        match event.key().as_str() {
            "ArrowDown" if len > 0 => {
                event.prevent_default();
                self.active.update(|i| *i = (*i + 1) % len);
            }
            "ArrowUp" if len > 0 => {
                event.prevent_default();
                self.active
                    .update(|i| *i = if *i == 0 { len - 1 } else { *i - 1 });
            }
            "Enter" => {
                event.prevent_default();
                event.stop_propagation();
                self.add_current();
            }
            "Escape" => {
                event.prevent_default();
                event.stop_propagation();
                self.open.set(false);
            }
            _ => {}
        }
    }

    fn popover(self) -> impl IntoView {
        let lang = self.lang;
        move || {
            if !self.open.get() {
                return None;
            }
            let q = self.query.get();
            let contacts = self.filtered_contacts(&q);
            let others = self.filtered_others(&contacts);
            let loading = self.searching.get();
            let total = contacts.len() + others.len();
            if total == 0 && !loading && q.trim().is_empty() {
                return None;
            }
            let active_idx = self.active.get().min(total.saturating_sub(1));
            let render_item = move |idx: usize, p: Person| {
                let selected = idx == active_idx;
                let pick = p.clone();
                view! {
                    <li role="presentation">
                        <button
                            type="button"
                            class="ac-item"
                            role="option"
                            aria-selected=selected.to_string()
                            on:mousedown=|ev: leptos::ev::MouseEvent| ev.prevent_default()
                            on:click=move |_| self.add_person(pick.clone())
                        >
                            {avatar(&p.display_name, &p.tag.to_string(), false, Some(p.tag))}
                            <span translate="no">
                                <bdi>{p.display_name}</bdi>
                                <span class="tag">{"#"}{model::tag_text(p.tag)}</span>
                            </span>
                        </button>
                    </li>
                }
            };
            let contact_len = contacts.len();
            let contacts_sec = (!contacts.is_empty()).then(|| {
                let items = contacts
                    .into_iter()
                    .enumerate()
                    .map(|(i, p)| render_item(i, p))
                    .collect_view();
                view! {
                    <p class="ac-heading">{words::chat_group_invite_contacts(lang)}</p>
                    <ul class="ac-list">{items}</ul>
                }
            });
            let others_sec = (!others.is_empty() || loading).then(|| {
                let items = others
                    .into_iter()
                    .enumerate()
                    .map(|(i, p)| render_item(contact_len + i, p))
                    .collect_view();
                let wait = loading.then(|| {
                    view! { <p class="ac-empty" role="status">{words::chat_group_invite_searching(lang)}</p> }
                });
                view! {
                    <p class="ac-heading">{words::chat_group_invite_others(lang)}</p>
                    <ul class="ac-list">{items}</ul>
                    {wait}
                }
            });
            let empty_msg = (total == 0 && !loading).then(|| {
                view! { <p class="ac-empty">{words::chat_group_invite_empty(lang)}</p> }
            });
            Some(view! {
                <div id="new-group-invite-list" class="ac-popover" role="listbox">
                    {contacts_sec}
                    {others_sec}
                    {empty_msg}
                </div>
            })
        }
    }
}

fn new_group_invite_section(lang: Lang, ac: NewGroupAc, queued: RwSignal<Vec<Person>>) -> AnyView {
    let chips = move || {
        let list = queued.get();
        (!list.is_empty()).then(|| {
            let items = list
                .into_iter()
                .map(|p| {
                    let tag = p.tag;
                    let label = handle(&p);
                    view! {
                        <li class="new-group-chip">
                            {avatar(&p.display_name, &tag.to_string(), false, Some(tag))}
                            <bdi translate="no">{label}</bdi>
                            <button
                                type="button"
                                class="icon-button"
                                title=words::chat_group_remove(lang)
                                aria-label=words::chat_group_remove(lang)
                                on:click=move |_| queued.update(|all| all.retain(|x| x.tag != tag))
                            >
                                {glyph(glyph::CLOSE)}
                            </button>
                        </li>
                    }
                })
                .collect_view();
            view! { <ul class="new-group-chips">{items}</ul> }
        })
    };
    view! {
        <section class="group-section">
            <label for="new-group-invite">{words::chat_group_invite(lang)}</label>
            {chips}
            <div class="ac-wrap">
                <div class="inset">
                    {field::mark(glyph::USER_PLUS)}
                    <input
                        id="new-group-invite"
                        type="text"
                        role="combobox"
                        autocomplete="off"
                        spellcheck="false"
                        aria-autocomplete="list"
                        aria-controls="new-group-invite-list"
                        aria-expanded=move || ac.open.get().to_string()
                        placeholder=words::chat_group_invite_placeholder(lang)
                        prop:value=move || ac.query.get()
                        on:focus=move |_| ac.open.set(true)
                        on:blur=move |_| ac.open.set(false)
                        on:input=move |ev| ac.on_typed(&event_target_value(&ev))
                        on:keydown=move |ev| ac.handle_keydown(&ev)
                    />
                    <button
                        type="button"
                        class="inset-end"
                        title=words::chat_group_invite_button(lang)
                        aria-label=words::chat_group_invite_button(lang)
                        on:click=move |_| ac.add_current()
                    >
                        {glyph(glyph::PLUS)}
                    </button>
                </div>
                {ac.popover()}
            </div>
        </section>
    }
    .into_any()
}

fn new_group_identity_section(
    lang: Lang,
    name: RwSignal<String>,
    description: RwSignal<String>,
) -> AnyView {
    view! {
        <section class="group-section">
            <label for="new-group-name">{words::chat_group_name(lang)}</label>
            <div class="inset">
                {field::mark(glyph::USERS)}
                <input
                    id="new-group-name"
                    type="text"
                    maxlength="96"
                    autocomplete="off"
                    prop:value=move || name.get()
                    on:input=move |ev| name.set(event_target_value(&ev))
                />
            </div>
            <label for="new-group-description">{words::chat_group_description(lang)}</label>
            <div class="inset">
                {field::mark(glyph::FILE_TEXT)}
                <input
                    id="new-group-description"
                    type="text"
                    maxlength="280"
                    placeholder=words::chat_group_description_placeholder(lang)
                    prop:value=move || description.get()
                    on:input=move |ev| description.set(event_target_value(&ev))
                />
            </div>
        </section>
    }
    .into_any()
}

/// Centered hierarchical modal dialog for creating a new group with optional
/// description and initial player invites.
pub fn new_group_dialog(
    lang: Lang,
    own: u32,
    ctl: Ctl,
    friends: Memo<Vec<Person>>,
    created: Callback<String>,
    close: Callback<()>,
) -> AnyView {
    super::talk::open_modal_dialog("new-group-dialog");
    let backdrop_down = RwSignal::new(false);
    let name = RwSignal::new(String::new());
    let description = RwSignal::new(String::new());
    let queued = RwSignal::new(Vec::<Person>::new());
    let problem = RwSignal::new(None::<&'static str>);
    let ac = NewGroupAc::new(lang, own, friends, queued);
    let submit = move |event: leptos::ev::SubmitEvent| {
        event.prevent_default();
        let text = name.get_untracked();
        if !central_api::groups::valid_name(&text) {
            problem.set(Some(words::chat_group_name_hint(lang)));
            return;
        }
        let desc = description.get_untracked();
        if !central_api::groups::valid_description(&desc) {
            problem.set(Some(words::chat_group_description_hint(lang)));
            return;
        }
        if ctl.social_busy.get_untracked() {
            return;
        }
        let invitees = queued.get_untracked();
        problem.set(None);
        ctl.create_group(text, move |id| match id {
            Some(id) => {
                if !desc.trim().is_empty() {
                    ctl.group(
                        id.clone(),
                        Change::SetDescription { description: desc },
                        |_| {},
                    );
                }
                for person in invitees {
                    ctl.group(id.clone(), Change::Invite { tag: person.tag }, |_| {});
                }
                created.run(id);
            }
            None => problem.set(Some(words::chat_group_failed(lang))),
        });
    };
    view! {
        <dialog
            id="new-group-dialog"
            class="conv-details-dialog new-group-dialog"
            aria-labelledby="new-group-title"
            on:pointerdown=move |ev: leptos::ev::PointerEvent| {
                backdrop_down.set(ev.target() == ev.current_target());
            }
            on:click=move |ev: leptos::ev::MouseEvent| {
                if backdrop_down.get_untracked() && ev.target() == ev.current_target() {
                    close.run(());
                }
            }
            on:close=move |_| close.run(())
        >
            <div class="conv-details-card">
                <header class="conv-details-head">
                    <h3 id="new-group-title">{words::chat_new_group(lang)}</h3>
                    <button type="button" class="dialog-close" title=words::chat_close(lang)
                        aria-label=words::chat_close(lang) on:click=move |_| close.run(())>
                        {glyph(glyph::CLOSE)}
                    </button>
                </header>
                <form id="new-group" class="conv-details-body new-group-form" on:submit=submit>
                    {new_group_identity_section(lang, name, description)}
                    {new_group_invite_section(lang, ac, queued)}
                    <p class="error" role="alert">{move || problem.get()}</p>
                    <div class="new-group-actions">
                        <button type="button" class="button button-ghost" on:click=move |_| close.run(())>
                            {words::picture_cancel(lang)}
                        </button>
                        <button type="submit" class="button primary"
                            aria-disabled=move || ctl.social_busy.get().then_some("true")>
                            {words::chat_group_create(lang)}
                        </button>
                    </div>
                </form>
            </div>
        </dialog>
    }
    .into_any()
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
                    let inviter = handle_with_copy(
                        lang,
                        invite.inviter.display_name.clone(),
                        model::tag_text(invite.inviter.tag),
                        true,
                    );
                    let invited_by = words::chat_invited_by(lang, "\0");
                    let (before, after) = match invited_by.split_once('\0') {
                        Some((head, tail)) => (head.to_owned(), tail.to_owned()),
                        None => (invited_by, String::new()),
                    };
                    let pic = invite.picture_key.map(|k| (k, invite.picture_moving));
                    let desc = (!invite.description.is_empty()).then(|| {
                        view! { <small class="group-desc">{invite.description.clone()}</small> }
                    });
                    view! {
                        <li class="person">
                            <span class="person-line">
                                {group_avatar(&format!("grp:{}", invite.group), pic)}
                                <span class="person-handle">
                                    <bdi class="person-name">{invite.name.clone()}</bdi>
                                    {desc}
                                    <small>{before}{inviter}{after}</small>
                                </span>
                            </span>
                            <span class="person-actions">
                                <button type="button" class="button small primary" on:click=move |_| {
                                    let id = join.clone();
                                    ctl.group(id.clone(), Change::Join, move |ok| if ok { choose.run(format!("grp:{id}")) });
                                }>{glyph(glyph::USER_CHECK)}<span>{words::people_accept(lang)}</span></button>
                                <button type="button" class="button small quiet" on:click=move |_| {
                                    ctl.group(decline.clone(), Change::Withdraw { tag: own }, |_| {});
                                }>{glyph(glyph::USER_X)}<span>{words::people_decline(lang)}</span></button>
                            </span>
                        </li>
                    }
                })
                .collect_view();
            view! {
                <section class="people" data-relation="invited">
                    <h2>{words::chat_invites(lang)}</h2>
                    <ul>{items}</ul>
                </section>
            }
        })
    }
}

fn group_hero(
    lang: Lang,
    group: &Group,
    is_admin: bool,
    ctl: Ctl,
    change: impl Fn(Change) + Copy + Send + Sync + 'static,
) -> AnyView {
    let pic = group.picture_key.map(|k| (k, group.picture_moving));
    let count = words::chat_members(lang, u64::try_from(group.members.len()).unwrap_or(0));
    let desc = if group.description.is_empty() {
        view! { <p class="group-desc is-empty">{words::chat_group_description_empty(lang)}</p> }
            .into_any()
    } else {
        view! { <p class="group-desc">{group.description.clone()}</p> }.into_any()
    };
    let cropper = is_admin.then(|| {
        crate::islands::picture::group_cropper(
            lang,
            group.id.clone(),
            group.picture_key.is_some(),
            Callback::new(move |()| ctl.reload_groups()),
            Callback::new(move |()| change(Change::RemovePicture)),
        )
    });
    view! {
        <div class="group-hero">
            <div class="group-avatar-ring">
                {group_avatar(&format!("grp:{}", group.id), pic)}
            </div>
            <h4 id="group-panel-title" class="group-hero-name"><bdi translate="no">{group.name.clone()}</bdi></h4>
            <p class="meta">{count}</p>
            {desc}
            {cropper}
        </div>
    }
    .into_any()
}

fn group_admin_forms(
    lang: Lang,
    group: &Group,
    change: impl Fn(Change) + Copy + Send + Sync + 'static,
) -> AnyView {
    let name = RwSignal::new(group.name.clone());
    let desc = RwSignal::new(group.description.clone());
    let desc_saved = RwSignal::new(false);
    view! {
        <form class="msgr-add" on:submit=move |event: leptos::ev::SubmitEvent| {
            event.prevent_default();
            let text = name.get_untracked();
            if central_api::groups::valid_name(&text) {
                change(Change::Rename { name: text });
            }
        }>
            <label for="group-rename">{words::chat_group_name(lang)}</label>
            <div class="inset">
                {field::mark(glyph::PENCIL)}
                <input id="group-rename" type="text" maxlength="96" prop:value=move || name.get()
                    on:input=move |ev| name.set(event_target_value(&ev)) />
                <button type="submit" class="inset-end"
                    title=words::chat_group_rename(lang)
                    aria-label=words::chat_group_rename(lang)>
                    {glyph(glyph::CHECK)}
                </button>
            </div>
        </form>
        <form class="msgr-add" on:submit=move |event: leptos::ev::SubmitEvent| {
            event.prevent_default();
            let text = desc.get_untracked();
            if central_api::groups::valid_description(&text) {
                desc_saved.set(true);
                change(Change::SetDescription { description: text });
            }
        }>
            <label for="group-description">{words::chat_group_description(lang)}</label>
            <div class="inset">
                {field::mark(glyph::FILE_TEXT)}
                <input id="group-description" type="text" maxlength="280"
                    placeholder=words::chat_group_description_placeholder(lang)
                    aria-describedby="group-desc-hint"
                    prop:value=move || desc.get()
                    on:input=move |ev| {
                        desc_saved.set(false);
                        desc.set(event_target_value(&ev));
                    } />
                <button type="submit" class="inset-end"
                    title=words::chat_group_description_save(lang)
                    aria-label=words::chat_group_description_save(lang)>
                    {glyph(glyph::CHECK)}
                </button>
            </div>
            <p id="group-desc-hint" class="hint">{words::chat_group_description_hint(lang)}</p>
            {move || desc_saved.get().then(|| view! { <p class="meta" role="status">{words::chat_group_description_saved(lang)}</p> })}
        </form>
    }
    .into_any()
}

/// A group's profile, members, admin controls, and settings in its details pane.
pub fn group_panel(
    lang: Lang,
    group: &Group,
    own: u32,
    ctl: Ctl,
    friends: Memo<Vec<Person>>,
    selected: RwSignal<Option<String>>,
) -> AnyView {
    let owner = group.members.iter().any(|m| m.owner && m.person.tag == own);
    let is_admin = owner || group.members.iter().any(|m| m.admin && m.person.tag == own);
    let asking: Asking = RwSignal::new(None);
    let id = StoredValue::new(group.id.clone());
    let change = move |what: Change| {
        ctl.group(id.get_value(), what, move |ok| {
            if ok {
                asking.set(None);
            }
        });
    };
    let forms = is_admin.then(|| {
        view! {
            <section class="group-section group-settings">
                {group_admin_forms(lang, group, change)}
            </section>
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
            {group_hero(lang, group, is_admin, ctl, change)}
            {forms}
            <section class="group-section group-members-section">
                <h4>{glyph(glyph::USERS)}<span>{words::chat_group_members(lang, u64::try_from(group.members.len()).unwrap_or(0))}</span></h4>
                {members(lang, group, own, owner, is_admin, asking, change)}
                {invite(lang, group, friends, change)}
                {invited(lang, group, is_admin, change)}
            </section>
            <p class="error" role="alert">{move || ctl.social_failed.get().then(|| words::chat_group_failed(lang))}</p>
            <div class="group-leave">{move || leave.clone().view()}</div>
        </section>
    }
    .into_any()
}

fn member_confirm(
    lang: Lang,
    tag: u32,
    who: String,
    asking: Asking,
    change: impl Fn(Change) + Copy + Send + Sync + 'static,
) -> impl IntoView {
    move || {
        asking.get().filter(|(t, _)| *t == tag).map(|(_, which)| {
            let (text, label, what): (String, &'static str, Change) = match which {
                "transfer" => (
                    words::chat_group_transfer_ask(lang, &who),
                    words::chat_group_transfer(lang),
                    Change::Transfer { tag },
                ),
                "make_admin" => (
                    words::chat_group_make_admin_ask(lang, &who),
                    words::chat_group_make_admin(lang),
                    Change::SetAdmin { tag, admin: true },
                ),
                "remove_admin" => (
                    words::chat_group_remove_admin_ask(lang, &who),
                    words::chat_group_remove_admin(lang),
                    Change::SetAdmin { tag, admin: false },
                ),
                _ => (
                    words::chat_group_remove_ask(lang, &who),
                    words::chat_group_remove(lang),
                    Change::Remove { tag },
                ),
            };
            let btn_class = if which == "make_admin" {
                "button small primary"
            } else {
                "button small danger"
            };
            view! {
                <div class="ask" role="group" aria-label=label>
                    <p>{text}</p>
                    <button type="button" class=btn_class on:click=move |_| change(what.clone())>{label}</button>
                    <button type="button" class="button small quiet" on:click=move |_| asking.set(None)>{words::people_keep(lang)}</button>
                </div>
            }
        })
    }
}

fn member_row(
    lang: Lang,
    m: &Member,
    own: u32,
    owner: bool,
    is_admin: bool,
    asking: Asking,
    change: impl Fn(Change) + Copy + Send + Sync + 'static,
) -> AnyView {
    let tag = m.person.tag;
    let who = handle(&m.person);
    let role_pill = if m.owner {
        Some(
            view! { <span class="pill owner">{glyph(glyph::CROWN)}<span>{words::chat_group_owner(lang)}</span></span> }
                .into_any(),
        )
    } else if m.admin {
        Some(
            view! { <span class="pill admin">{glyph(glyph::SHIELD)}<span>{words::chat_group_admin(lang)}</span></span> }
                .into_any(),
        )
    } else {
        None
    };
    let can_admin = is_admin && !m.owner && tag != own;
    let can_transfer = owner && tag != own;
    let can_remove = tag != own && !m.owner && (owner || (is_admin && !m.admin));
    let tools = (can_admin || can_transfer || can_remove).then(|| {
        let admin_btn = can_admin.then(|| {
            let (kind, label) = if m.admin {
                ("remove_admin", words::chat_group_remove_admin(lang))
            } else {
                ("make_admin", words::chat_group_make_admin(lang))
            };
            view! {
                <button type="button" class="button small quiet" on:click=move |_| asking.set(Some((tag, kind)))>
                    {label}
                </button>
            }
        });
        let transfer_btn = can_transfer.then(|| {
            view! {
                <button type="button" class="button small quiet" on:click=move |_| asking.set(Some((tag, "transfer")))>
                    {words::chat_group_transfer(lang)}
                </button>
            }
        });
        let remove_btn = can_remove.then(|| {
            view! {
                <button type="button" class="button small danger" on:click=move |_| asking.set(Some((tag, "remove")))>
                    {words::chat_group_remove(lang)}
                </button>
            }
        });
        view! {
            <span class="person-actions">
                {admin_btn}
                {transfer_btn}
                {remove_btn}
            </span>
        }
    });
    view! {
        <li class="person">
            <span class="person-line">{person_line(lang, &m.person)}{role_pill}</span>
            {tools}
            {member_confirm(lang, tag, who, asking, change)}
        </li>
    }
    .into_any()
}

fn members(
    lang: Lang,
    group: &Group,
    own: u32,
    owner: bool,
    is_admin: bool,
    asking: Asking,
    change: impl Fn(Change) + Copy + Send + Sync + 'static,
) -> AnyView {
    let items = group
        .members
        .iter()
        .map(|m| member_row(lang, m, own, owner, is_admin, asking, change))
        .collect_view();
    view! { <ul class="people-list">{items}</ul> }.into_any()
}

#[derive(Clone, Copy)]
struct InviteAc {
    lang: Lang,
    taken: StoredValue<Vec<u32>>,
    friends: Memo<Vec<Person>>,
    query: RwSignal<String>,
    open: RwSignal<bool>,
    others: RwSignal<Vec<Person>>,
    searching: RwSignal<bool>,
    active: RwSignal<usize>,
    sent: RwSignal<Option<String>>,
    pause: StoredValue<Option<TimeoutHandle>>,
    search_gen: StoredValue<u32>,
}

impl InviteAc {
    fn new(lang: Lang, taken: Vec<u32>, friends: Memo<Vec<Person>>) -> Self {
        let pause = StoredValue::new(None::<TimeoutHandle>);
        on_cleanup(move || {
            if let Some(Some(handle)) = pause.try_get_value() {
                handle.clear();
            }
        });
        Self {
            lang,
            taken: StoredValue::new(taken),
            friends,
            query: RwSignal::new(String::new()),
            open: RwSignal::new(false),
            others: RwSignal::new(Vec::new()),
            searching: RwSignal::new(false),
            active: RwSignal::new(0),
            sent: RwSignal::new(None),
            pause,
            search_gen: StoredValue::new(0),
        }
    }

    fn clear_pause(self) {
        if let Some(Some(handle)) = self.pause.try_get_value() {
            handle.clear();
        }
        let _ = self.pause.try_set_value(None);
    }

    fn filtered_contacts(self, needle: &str) -> Vec<Person> {
        let low = needle.trim().to_lowercase();
        self.taken.with_value(|taken| {
            self.friends.with(|list| {
                list.iter()
                    .filter(|p| !taken.contains(&p.tag))
                    .filter(|p| {
                        low.is_empty()
                            || p.display_name.to_lowercase().contains(&low)
                            || handle(p).to_lowercase().contains(&low)
                    })
                    .cloned()
                    .collect()
            })
        })
    }

    fn filtered_others(self, contacts: &[Person]) -> Vec<Person> {
        self.taken.with_value(|taken| {
            self.others.with(|list| {
                list.iter()
                    .filter(|p| !taken.contains(&p.tag) && !contacts.iter().any(|c| c.tag == p.tag))
                    .cloned()
                    .collect()
            })
        })
    }

    fn on_typed(self, value: &str) {
        self.query.set(value.to_owned());
        self.open.set(true);
        self.active.set(0);
        self.sent.set(None);
        self.clear_pause();
        let trimmed = value.trim();
        let next = self.search_gen.get_value().wrapping_add(1);
        self.search_gen.set_value(next);
        if trimmed.chars().count() < 2 && tag_of(trimmed).is_none() {
            self.others.set(Vec::new());
            self.searching.set(false);
            return;
        }
        self.searching.set(true);
        let q = trimmed.to_owned();
        let handle = set_timeout(
            move || {
                let _ = self.pause.try_set_value(None);
                leptos::task::spawn_local(async move {
                    let list = wire::search_people(&q).await.unwrap_or_default();
                    if self.search_gen.try_get_value() == Some(next) {
                        self.others.set(list);
                        self.searching.set(false);
                    }
                });
            },
            SEARCH_DEBOUNCE,
        )
        .ok();
        self.pause.set_value(handle);
    }

    fn pick(self, person: &Person, change: impl Fn(Change) + Copy + 'static) {
        self.clear_pause();
        self.query.set(String::new());
        self.others.set(Vec::new());
        self.searching.set(false);
        self.open.set(false);
        self.active.set(0);
        self.sent.set(Some(words::chat_group_invite_sent(
            self.lang,
            &handle(person),
        )));
        change(Change::Invite { tag: person.tag });
    }

    fn handle_submit(self, change: impl Fn(Change) + Copy + 'static) {
        let q = self.query.get_untracked();
        let contacts = self.filtered_contacts(&q);
        let others = self.filtered_others(&contacts);
        let all: Vec<Person> = contacts.into_iter().chain(others).collect();
        if let Some(person) = all.get(self.active.get_untracked()).or_else(|| all.first()) {
            self.pick(person, change);
            return;
        }
        if let Some(tag) = tag_of(&q) {
            let taken_hit = self.taken.with_value(|t| t.contains(&tag));
            if !taken_hit {
                self.query.set(String::new());
                self.open.set(false);
                self.sent.set(Some(words::chat_group_invite_sent(
                    self.lang,
                    &format!("#{}", model::tag_text(tag)),
                )));
                change(Change::Invite { tag });
            }
        }
    }

    fn handle_keydown(self, event: &leptos::ev::KeyboardEvent) {
        if !self.open.get_untracked() {
            return;
        }
        let q = self.query.get_untracked();
        let contacts = self.filtered_contacts(&q);
        let others = self.filtered_others(&contacts);
        let len = contacts.len() + others.len();
        match event.key().as_str() {
            "ArrowDown" if len > 0 => {
                event.prevent_default();
                self.active.update(|i| *i = (*i + 1) % len);
            }
            "ArrowUp" if len > 0 => {
                event.prevent_default();
                self.active
                    .update(|i| *i = if *i == 0 { len - 1 } else { *i - 1 });
            }
            "Escape" => {
                event.prevent_default();
                event.stop_propagation();
                self.open.set(false);
            }
            _ => {}
        }
    }

    fn popover(self, change: impl Fn(Change) + Copy + Send + Sync + 'static) -> impl IntoView {
        let lang = self.lang;
        move || {
            if !self.open.get() {
                return None;
            }
            let q = self.query.get();
            let contacts = self.filtered_contacts(&q);
            let others = self.filtered_others(&contacts);
            let loading = self.searching.get();
            let total = contacts.len() + others.len();
            if total == 0 && !loading && q.trim().is_empty() {
                return None;
            }
            let active_idx = self.active.get().min(total.saturating_sub(1));
            let render_item = move |idx: usize, p: Person| {
                let selected = idx == active_idx;
                let pick = p.clone();
                view! {
                    <li role="presentation">
                        <button
                            type="button"
                            class="ac-item"
                            role="option"
                            aria-selected=selected.to_string()
                            on:mousedown=|ev: leptos::ev::MouseEvent| ev.prevent_default()
                            on:click=move |_| self.pick(&pick, change)
                        >
                            {avatar(&p.display_name, &p.tag.to_string(), false, Some(p.tag))}
                            <span translate="no">
                                <bdi>{p.display_name}</bdi>
                                <span class="tag">{"#"}{model::tag_text(p.tag)}</span>
                            </span>
                        </button>
                    </li>
                }
            };
            let contact_len = contacts.len();
            let contacts_sec = (!contacts.is_empty()).then(|| {
                let items = contacts
                    .into_iter()
                    .enumerate()
                    .map(|(i, p)| render_item(i, p))
                    .collect_view();
                view! {
                    <p class="ac-heading">{words::chat_group_invite_contacts(lang)}</p>
                    <ul class="ac-list">{items}</ul>
                }
            });
            let others_sec = (!others.is_empty() || loading).then(|| {
                let items = others
                    .into_iter()
                    .enumerate()
                    .map(|(i, p)| render_item(contact_len + i, p))
                    .collect_view();
                let wait = loading.then(|| {
                    view! { <p class="ac-empty" role="status">{words::chat_group_invite_searching(lang)}</p> }
                });
                view! {
                    <p class="ac-heading">{words::chat_group_invite_others(lang)}</p>
                    <ul class="ac-list">{items}</ul>
                    {wait}
                }
            });
            let empty_msg = (total == 0 && !loading).then(|| {
                view! { <p class="ac-empty">{words::chat_group_invite_empty(lang)}</p> }
            });
            Some(view! {
                <div id="group-invite-list" class="ac-popover" role="listbox">
                    {contacts_sec}
                    {others_sec}
                    {empty_msg}
                </div>
            })
        }
    }
}

fn invite<F: Fn(Change) + Copy + Send + Sync + 'static>(
    lang: Lang,
    group: &Group,
    friends: Memo<Vec<Person>>,
    change: F,
) -> Option<AnyView> {
    let taken: Vec<u32> = group
        .members
        .iter()
        .map(|m| m.person.tag)
        .chain(group.invited.iter().map(|p| p.tag))
        .collect();
    if taken.len() >= central_api::groups::MAX_MEMBERS {
        return None;
    }
    let ac = InviteAc::new(lang, taken, friends);
    let submit = move |event: leptos::ev::SubmitEvent| {
        event.prevent_default();
        ac.handle_submit(change);
    };
    Some(
        view! {
            <form class="msgr-add" on:submit=submit>
                <label for="group-invite">{words::chat_group_invite(lang)}</label>
                <div class="ac-wrap">
                    <div class="inset">
                        {field::mark(glyph::USER_PLUS)}
                        <input
                            id="group-invite"
                            type="text"
                            role="combobox"
                            autocomplete="off"
                            spellcheck="false"
                            aria-autocomplete="list"
                            aria-controls="group-invite-list"
                            aria-expanded=move || ac.open.get().to_string()
                            placeholder=words::chat_group_invite_placeholder(lang)
                            prop:value=move || ac.query.get()
                            on:focus=move |_| ac.open.set(true)
                            on:blur=move |_| ac.open.set(false)
                            on:input=move |ev| ac.on_typed(&event_target_value(&ev))
                            on:keydown=move |ev| ac.handle_keydown(&ev)
                        />
                        <button
                            type="submit"
                            class="inset-end"
                            title=words::chat_group_invite_button(lang)
                            aria-label=words::chat_group_invite_button(lang)
                        >
                            {glyph(glyph::PLUS)}
                        </button>
                    </div>
                    {ac.popover(change)}
                </div>
                {move || ac.sent.get().map(|msg| view! { <p class="meta" role="status">{msg}</p> })}
            </form>
        }
        .into_any(),
    )
}

fn invited(
    lang: Lang,
    group: &Group,
    is_admin: bool,
    change: impl Fn(Change) + Copy + Send + Sync + 'static,
) -> Option<AnyView> {
    (!group.invited.is_empty()).then(|| {
        let items = group
            .invited
            .iter()
            .map(|p| {
                let tag = p.tag;
                let withdraw = is_admin.then(|| view! {
                    <button type="button" class="button small quiet" on:click=move |_| change(Change::Withdraw { tag })>
                        {words::people_cancel(lang)}</button>
                });
                view! { <li class="person"><span class="person-line">{person_line(lang, p)}</span>{withdraw}</li> }
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
                <button type="button" class="button small quiet" on:click=move |_| asking.set(None)>{words::people_keep(lang)}</button>
            </div>
        }
        .into_any()
    }
}
