//! The messenger's window: the list beside the open conversation (one of
//! the two at a time on a phone), the conversation's messages by day, the
//! message on its way, the composer and the conversation's details.
//! Every text a person wrote is rendered as text, never as markup.

use central_api::social::{Person, Relation};
use leptos::html;
use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use leptos::web_sys;

use super::glyph::{self, glyph};
use super::model::{self, Conversation, Kind, Message, Review, Sources, initials, tag_text};
use super::people;
use super::time;
use super::{Ctl, Me, Phase, Tab, href, missing_view, remember};
use crate::rich::rich;
use crate::words::{self, Lang};

/// A row's element id, from the conversation's.
fn row_id(conv: &str) -> String {
    format!("conv-{}", conv.replace(':', "-"))
}

fn focus(id: &str) {
    if let Some(element) = document()
        .get_element_by_id(id)
        .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
    {
        let _ = element.focus();
    }
}

/// An avatar: initials or the group sign, in one of six tints.
pub fn avatar(name: &str, seed: &str, group: bool) -> AnyView {
    let face = if group {
        glyph(glyph::USERS)
    } else {
        initials(name).into_any()
    };
    view! { <span class="avatar" data-hue=model::hue(seed).to_string() aria-hidden="true">{face}</span> }.into_any()
}

/// What the lists are made of: the contacts, the groups, every conversation.
#[derive(Clone, Copy)]
struct Lists {
    friends: Memo<Vec<Person>>,
    rows: Memo<Vec<Conversation>>,
    unread: Memo<usize>,
    incoming: Memo<usize>,
}

impl Lists {
    fn new(own: u32, ctl: Ctl) -> Lists {
        let friends = Memo::new(move |_| {
            ctl.contacts.with(|c| match c {
                Some(Ok(list)) => list
                    .iter()
                    .filter(|c| c.relation == Relation::Friend)
                    .map(|c| c.person.clone())
                    .collect::<Vec<Person>>(),
                _ => Vec::new(),
            })
        });
        let groups = Memo::new(move |_| {
            ctl.groups.with(|g| match g {
                Some(Ok(snapshot)) => snapshot.groups.clone(),
                _ => Vec::new(),
            })
        });
        let rows = Memo::new(move |_| {
            ctl.shown.with(|s| {
                friends.with(|f| {
                    groups.with(|g| {
                        model::conversations(&Sources {
                            own,
                            friends: f,
                            known: &s.known,
                            groups: g,
                            messages: &s.messages,
                            seen: s.seen.as_ref(),
                        })
                    })
                })
            })
        });
        let unread = Memo::new(move |_| rows.with(|r| r.iter().map(|c| c.unread).sum::<usize>()));
        let incoming = Memo::new(move |_| {
            ctl.contacts.with(|c| match c {
                Some(Ok(list)) => list
                    .iter()
                    .filter(|c| c.relation == Relation::Incoming)
                    .count(),
                _ => 0,
            })
        });
        Lists {
            friends,
            rows,
            unread,
            incoming,
        }
    }

    /// The page's title carries the unread count.
    fn title(self) {
        let title = StoredValue::new(String::new());
        Effect::new(move |_| {
            let count = self.unread.get();
            if title.with_value(String::is_empty) {
                title.set_value(document().title());
            }
            let base = title.get_value();
            document().set_title(&if count > 0 {
                format!("({count}) {base}")
            } else {
                base
            });
        });
    }
}

fn tabs(
    lang: Lang,
    lists: Lists,
    tab: RwSignal<Tab>,
    selected: RwSignal<Option<String>>,
) -> AnyView {
    let switch = move |to: Tab| {
        move |event: leptos::ev::MouseEvent| {
            event.prevent_default();
            remember(lang, to, None);
            tab.set(to);
            selected.set(None);
        }
    };
    let badge = move |count: Memo<usize>, what: &'static str| {
        move || {
            (count.get() > 0).then(|| {
                view! { <span class="badge">{count.get()}<span class="visually-hidden">{format!(" {what}")}</span></span> }
            })
        }
    };
    view! {
        <nav class="msgr-tabs" aria-label=words::chat_tabs(lang)>
            <a href=href(lang, Tab::Chats, None) aria-current=move || (tab.get() == Tab::Chats).then_some("page")
                on:click=switch(Tab::Chats)>
                {glyph(glyph::MESSAGE)}<span>{words::chat_tab_chats(lang)}</span>
                {badge(lists.unread, words::chat_unread(lang))}
            </a>
            <a href=href(lang, Tab::Contacts, None) aria-current=move || (tab.get() == Tab::Contacts).then_some("page")
                on:click=switch(Tab::Contacts)>
                {glyph(glyph::USERS)}<span>{words::chat_tab_contacts(lang)}</span>
                {badge(lists.incoming, words::people_incoming(lang))}
            </a>
        </nav>
    }
    .into_any()
}

/// The whole messenger.
pub fn messenger(
    lang: Lang,
    me: &Me,
    ctl: Ctl,
    tab: RwSignal<Tab>,
    selected: RwSignal<Option<String>>,
    worker: StoredValue<String>,
) -> AnyView {
    let own = me.tag;
    let lists = Lists::new(own, ctl);
    lists.title();
    let choose = Callback::new(move |id: String| {
        remember(lang, Tab::Chats, Some(&id));
        tab.set(Tab::Chats);
        selected.set(Some(id));
    });
    let pane = move || {
        if tab.get() == Tab::Chats && selected.get().is_some() {
            "conversation"
        } else {
            "list"
        }
    };
    let me_store = StoredValue::new(me.clone());
    let own_handle = central_api::profile::handle(&me.name, own);
    let side = move || match tab.get() {
        Tab::Chats => chats_panel(lang, own, ctl, lists.rows, selected, choose).into_any(),
        Tab::Contacts => {
            people::contacts_panel(lang, own, own_handle.clone(), ctl, choose).into_any()
        }
    };
    let current = Memo::new(move |_| {
        selected
            .get()
            .filter(|id| lists.rows.with(|r| r.iter().any(|c| c.id == *id)))
    });
    let phase = Memo::new(move |_| ctl.shown.with(|s| s.phase));
    let main = move || {
        match phase.get() {
        Phase::Missing => missing_view(lang, worker.get_value(), me_store.get_value(), ctl).into_any(),
        Phase::Ended => view! {
            <div class="msgr-welcome" role="alert">
                <h2>{words::chat_ended(lang)}</h2>
                <p><a class="button primary" href=format!("/{}/signin", lang.code())>{words::nav_sign_in(lang)}</a></p>
            </div>
        }
        .into_any(),
        _ => match current.get() {
            Some(id) => conversation(lang, &id, own, ctl, lists, selected),
            None => welcome(lang, ctl).into_any(),
        },
    }
    };
    view! {
        <div class="messenger" data-pane=pane>
            <section class="msgr-side" aria-label=words::chat_side(lang)>
                <header class="msgr-me">
                    {avatar(&me.name, &own.to_string(), false)}
                    <span class="msgr-me-name">
                        <bdi translate="no">{me.name.clone()}</bdi>
                        <span class="tag" translate="no">{format!("#{}", tag_text(own))}</span>
                    </span>
                </header>
                {tabs(lang, lists, tab, selected)}
                {side}
            </section>
            <section class="msgr-main" aria-label=words::chat_main(lang)>{main}</section>
        </div>
    }
    .into_any()
}

fn welcome(lang: Lang, ctl: Ctl) -> impl IntoView {
    let phase = Memo::new(move |_| ctl.shown.with(|s| s.phase));
    move || match phase.get() {
        Phase::Opening => view! {
            <div class="msgr-welcome"><p role="status">{words::chat_opening(lang)}</p></div>
        }
        .into_any(),
        Phase::Failed => view! {
            <div class="msgr-welcome">
                <p class="error" role="alert">{words::chat_failed_open(lang)}</p>
            </div>
        }
        .into_any(),
        _ => view! {
            <div class="msgr-welcome">
                <span class="msgr-welcome-icon">{glyph(glyph::MESSAGE)}</span>
                <h2>{words::chat_choose(lang)}</h2>
                <p>{words::chat_choose_lead(lang)}</p>
                <p class="meta">{glyph(glyph::LOCK)}" "{words::chat_storage(lang)}</p>
            </div>
        }
        .into_any(),
    }
}

/// The Chats half: search, a new group, invitations, the list.
fn chats_panel(
    lang: Lang,
    own: u32,
    ctl: Ctl,
    rows: Memo<Vec<Conversation>>,
    selected: RwSignal<Option<String>>,
    choose: Callback<String>,
) -> impl IntoView {
    let query = RwSignal::new(String::new());
    let making = RwSignal::new(false);
    let visible = Memo::new(move |_| {
        let needle = query.get().trim().to_lowercase();
        rows.with(|r| {
            r.iter()
                .filter(|c| needle.is_empty() || c.title.to_lowercase().contains(&needle))
                .cloned()
                .collect::<Vec<_>>()
        })
    });
    let rove = RwSignal::new(None::<String>);
    let anchor = Memo::new(move |_| {
        visible.with(|v| {
            let has = |id: &String| v.iter().any(|c| c.id == *id);
            rove.get()
                .filter(has)
                .or_else(|| selected.get().filter(has))
                .or_else(|| v.first().map(|c| c.id.clone()))
        })
    });
    let keys = move |event: leptos::ev::KeyboardEvent| {
        let ids: Vec<String> = visible.with_untracked(|v| v.iter().map(|c| c.id.clone()).collect());
        let Some(last) = ids.len().checked_sub(1) else {
            return;
        };
        let at = anchor
            .get_untracked()
            .and_then(|a| ids.iter().position(|id| *id == a))
            .unwrap_or(0);
        let next = match event.key().as_str() {
            "ArrowDown" => (at + 1).min(last),
            "ArrowUp" => at.saturating_sub(1),
            "Home" => 0,
            "End" => last,
            _ => return,
        };
        event.prevent_default();
        rove.set(Some(ids[next].clone()));
        focus(&row_id(&ids[next]));
    };
    // Ctrl/Cmd+K: find a conversation
    let find = window_event_listener(leptos::ev::keydown, move |event| {
        if (event.ctrl_key() || event.meta_key()) && event.key().eq_ignore_ascii_case("k") {
            event.prevent_default();
            focus("chat-search");
        }
    });
    on_cleanup(move || find.remove());
    let empty = Memo::new(move |_| {
        visible.with(Vec::is_empty).then(|| {
            if query.with(|q| q.trim().is_empty()) {
                words::chat_no_conversations(lang)
            } else {
                words::chat_no_match(lang)
            }
        })
    });
    let list = move || {
        if let Some(text) = empty.get() {
            return view! { <p class="msgr-empty">{text}</p> }.into_any();
        }
        view! {
            <nav class="conv-nav" aria-label=words::chat_conversations(lang)>
                <ul class="conv-list" on:keydown=keys>
                    <For each=move || visible.get()
                        key=|c| (c.id.clone(), c.last.as_ref().map(|m| m.id.clone()), c.unread, c.title.clone(), c.open)
                        children=move |c| row(lang, &c, own, selected, anchor, rove, choose) />
                </ul>
            </nav>
        }
        .into_any()
    };
    view! {
        <div class="msgr-panel">
            <div class="msgr-find">
                <label class="visually-hidden" for="chat-search">{words::chat_search(lang)}</label>
                {glyph(glyph::SEARCH)}
                <input id="chat-search" type="search" autocomplete="off" spellcheck="false"
                    placeholder=words::chat_search_hint(lang)
                    on:input=move |ev| query.set(event_target_value(&ev)) prop:value=move || query.get() />
            </div>
            <div class="msgr-tools">
                <button type="button" class="button small" aria-expanded=move || making.get().to_string()
                    aria-controls="new-group" on:click=move |_| making.update(|m| *m = !*m)>
                    {glyph(glyph::PLUS)}<span>{words::chat_new_group(lang)}</span>
                </button>
            </div>
            {move || making.get().then(|| people::new_group(lang, ctl, Callback::new(move |id: String| {
                making.set(false);
                choose.run(format!("grp:{id}"));
            })))}
            {people::invites(lang, own, ctl, choose)}
            {list}
        </div>
    }
}

fn row(
    lang: Lang,
    c: &Conversation,
    own: u32,
    selected: RwSignal<Option<String>>,
    anchor: Memo<Option<String>>,
    rove: RwSignal<Option<String>>,
    choose: Callback<String>,
) -> AnyView {
    let id = c.id.clone();
    let name = match &c.kind {
        Kind::Former => words::chat_former_group(lang).to_owned(),
        _ => c.title.clone(),
    };
    let tag = match &c.kind {
        Kind::Direct(p) => Some(
            view! { <span class="tag" translate="no">{format!("#{}", tag_text(p.tag))}</span> },
        ),
        _ => None,
    };
    let preview = match (&c.last, &c.kind) {
        (Some(m), _) if m.sender == own => format!("{}: {}", words::chat_you(lang), m.text),
        (Some(m), _) => m.text.clone(),
        (None, Kind::Group(g)) => {
            words::chat_members(lang, u64::try_from(g.members.len()).unwrap_or(0))
        }
        (None, Kind::Former) => words::chat_former_lead(lang).to_owned(),
        (None, Kind::Direct(_)) if c.open => words::chat_say_hello(lang).to_owned(),
        (None, Kind::Direct(_)) => words::chat_history_only(lang).to_owned(),
    };
    let when = c.last.as_ref().map(|m| {
        view! { <time datetime=time::machine(m.at)>{time::short(lang, m.at, time::now())}</time> }
    });
    let unread = (c.unread > 0).then(|| {
        view! {
            <span class="badge">{c.unread}<span class="visually-hidden">{format!(" {}", words::chat_unread(lang))}</span></span>
        }
    });
    let is_current = {
        let id = id.clone();
        move || selected.get().as_deref() == Some(id.as_str())
    };
    let tabindex = {
        let id = id.clone();
        move || {
            if anchor.get().as_deref() == Some(id.as_str()) {
                "0"
            } else {
                "-1"
            }
        }
    };
    let (to, focused) = (id.clone(), id.clone());
    view! {
        <li>
            <a id=row_id(&id) class="conv-row" href=href(lang, Tab::Chats, Some(&id))
                aria-current=move || is_current().then_some("true") tabindex=tabindex
                data-unread=(c.unread > 0).then_some("true")
                on:focus=move |_| rove.set(Some(focused.clone()))
                on:click=move |event: leptos::ev::MouseEvent| {
                    if !(event.ctrl_key() || event.meta_key() || event.shift_key()) {
                        event.prevent_default();
                        choose.run(to.clone());
                    }
                }>
                {avatar(&name, &id, !matches!(c.kind, Kind::Direct(_)))}
                <span class="conv-main">
                    <span class="conv-top">
                        <bdi class="conv-name" translate="no">{name.clone()}</bdi>{tag}{when}
                    </span>
                    <span class="conv-bottom">
                        <span class="conv-preview">{preview}</span>{unread}
                    </span>
                </span>
            </a>
        </li>
    }
    .into_any()
}
/// A conversation's messages by day: each with whether it starts a run of
/// one sender's.
type Days = Vec<(String, String, Vec<(Message, bool)>)>;

fn days(lang: Lang, messages: &[Message]) -> Days {
    let now = time::now();
    let mut out: Days = Vec::new();
    let mut previous: Option<&Message> = None;
    for m in messages {
        let new_day = previous.is_none_or(|p| !time::same_day(p.at, m.at));
        if new_day {
            let key = time::machine(m.at).chars().take(10).collect();
            out.push((key, time::day_label(lang, m.at, now), Vec::new()));
        }
        let first = new_day
            || previous
                .is_some_and(|p| p.sender != m.sender || m.at.saturating_sub(p.at) > 5 * 60_000);
        if let Some(day) = out.last_mut() {
            day.2.push((m.clone(), first));
        }
        previous = Some(m);
    }
    out
}

/// Whether the page is used with a touch keyboard: there Enter breaks the
/// line and the button sends.
fn coarse() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        use js_sys::{Function, Reflect};
        use wasm_bindgen::JsValue;
        let window: JsValue = window().into();
        Reflect::get(&window, &JsValue::from_str("matchMedia"))
            .ok()
            .and_then(|f| f.dyn_into::<Function>().ok())
            .and_then(|f| {
                f.call1(&window, &JsValue::from_str("(pointer: coarse)"))
                    .ok()
            })
            .and_then(|list| Reflect::get(&list, &JsValue::from_str("matches")).ok())
            .and_then(|m| m.as_bool())
            .unwrap_or(false)
    }
    #[cfg(not(target_arch = "wasm32"))]
    false
}

/// One open conversation's signals, shared by its parts.
#[derive(Clone, Copy)]
struct Open {
    lang: Lang,
    own: u32,
    ctl: Ctl,
    id: StoredValue<String>,
    conv: Memo<Option<Conversation>>,
    members: Memo<Vec<u32>>,
    messages: Memo<Vec<Message>>,
    reviews: Memo<Vec<Review>>,
    changed: Memo<bool>,
    can_send: Memo<bool>,
    details: RwSignal<bool>,
    visible: RwSignal<usize>,
    friends: Memo<Vec<Person>>,
    selected: RwSignal<Option<String>>,
    group_now: Memo<Option<central_api::groups::Group>>,
    waiting: Memo<Option<(String, bool, bool)>>,
    trouble: Memo<(bool, bool)>,
}

fn conversation(
    lang: Lang,
    id: &str,
    own: u32,
    ctl: Ctl,
    lists: Lists,
    selected: RwSignal<Option<String>>,
) -> AnyView {
    let open = Open::new(lang, id, own, ctl, lists, selected);
    open.watch();
    // a conversation opened from the list: its heading takes the focus
    request_animation_frame(|| focus("conv-title"));
    let escape = move |event: leptos::ev::KeyboardEvent| {
        if event.key() == "Escape" && open.details.get_untracked() {
            open.details.set(false);
            focus("conv-details-toggle");
        }
    };
    view! {
        <div class="conv" on:keydown=escape>
            {open.head()}
            {move || open.details_view()}
            {move || open.banners()}
            {open.scroll()}
            {move || open.footer()}
            {open.announcer()}
        </div>
    }
    .into_any()
}

impl Open {
    fn new(
        lang: Lang,
        id: &str,
        own: u32,
        ctl: Ctl,
        lists: Lists,
        selected: RwSignal<Option<String>>,
    ) -> Open {
        let rows = lists.rows;
        let friends = lists.friends;
        let id = StoredValue::new(id.to_owned());
        let conv = Memo::new(move |_| {
            id.with_value(|id| rows.with(|r| r.iter().find(|c| c.id == *id).cloned()))
        });
        let members = Memo::new(move |_| {
            conv.with(|c| c.as_ref().map(|c| c.members(own)).unwrap_or_default())
        });
        let messages = Memo::new(move |_| {
            id.with_value(|id| {
                ctl.shown.with(|s| {
                    s.messages
                        .iter()
                        .filter(|m| m.conv == *id)
                        .cloned()
                        .collect::<Vec<_>>()
                })
            })
        });
        let reviews = Memo::new(move |_| {
            let tags = members.get();
            ctl.shown.with(|s| {
                s.reviews
                    .iter()
                    .filter(|(tag, _)| **tag != own && tags.contains(tag))
                    .map(|(_, r)| r.clone())
                    .collect::<Vec<_>>()
            })
        });
        let changed = Memo::new(move |_| reviews.with(|r| r.iter().any(|r| r.changed)));
        let can_send = Memo::new(move |_| {
            conv.with(|c| {
                c.as_ref().is_some_and(|c| {
                    c.open
                        && match &c.kind {
                            Kind::Direct(p) => friends.with(|f| f.iter().any(|f| f.tag == p.tag)),
                            Kind::Group(_) => true,
                            Kind::Former => false,
                        }
                })
            })
        });
        let group_now = Memo::new(move |_| {
            conv.with(|c| match c.as_ref().map(|c| &c.kind) {
                Some(Kind::Group(g)) => Some(g.clone()),
                _ => None,
            })
        });
        let waiting = Memo::new(move |_| {
            id.with_value(|id| {
                ctl.shown.with(|s| {
                    s.pending
                        .as_ref()
                        .filter(|p| p.conv == *id)
                        .map(|p| (p.text.clone(), p.failed, s.busy))
                })
            })
        });
        let trouble = Memo::new(move |_| ctl.shown.with(|s| (s.refused > 0, s.error)));
        Open {
            group_now,
            waiting,
            trouble,
            lang,
            own,
            ctl,
            id,
            conv,
            members,
            messages,
            reviews,
            changed,
            can_send,
            details: RwSignal::new(false),
            visible: RwSignal::new(100),
            friends,
            selected,
        }
    }

    /// Reads the members' keys while the chat is open, and counts what is
    /// shown as read.
    fn watch(self) {
        let ready = Memo::new(move |_| self.ctl.shown.with(|s| s.phase == Phase::Ready));
        Effect::new(move |_| {
            let tags = self.members.get();
            if !tags.is_empty() && ready.get() {
                self.ctl.keys(tags);
            }
        });
        Effect::new(move |_| {
            self.messages.with(|m| m.last().map(|m| m.at));
            self.id.with_value(|id| self.ctl.seen(id));
        });
    }

    fn name(self) -> String {
        self.conv
            .with_untracked(|c| {
                c.as_ref().map(|c| match c.kind {
                    Kind::Former => words::chat_former_group(self.lang).to_owned(),
                    _ => c.title.clone(),
                })
            })
            .unwrap_or_default()
    }

    fn group(self) -> bool {
        self.conv.with_untracked(|c| {
            c.as_ref()
                .is_some_and(|c| !matches!(c.kind, Kind::Direct(_)))
        })
    }

    fn head(self) -> AnyView {
        let lang = self.lang;
        let tag = self
            .conv
            .with_untracked(|c| match c.as_ref().map(|c| &c.kind) {
                Some(Kind::Direct(p)) => Some(format!("#{}", tag_text(p.tag))),
                _ => None,
            });
        let subtitle = move || {
            self.conv.with(|c| match c.as_ref().map(|c| &c.kind) {
                Some(Kind::Group(g)) => {
                    words::chat_members(lang, u64::try_from(g.members.len()).unwrap_or(0))
                }
                Some(Kind::Former) => words::chat_former_lead(lang).to_owned(),
                _ => words::chat_encrypted(lang).to_owned(),
            })
        };
        let back = move |_| {
            remember(lang, Tab::Chats, None);
            self.selected.set(None);
            let row = self.id.with_value(|id| row_id(id));
            request_animation_frame(move || focus(&row));
        };
        let (changed, details) = (self.changed, self.details);
        let name = self.name();
        let id = self.id.get_value();
        view! {
            <header class="conv-head">
                <button type="button" class="icon-button conv-back" aria-label=words::chat_back(lang) on:click=back>
                    {glyph(glyph::BACK)}
                </button>
                {avatar(&name, &id, self.group())}
                <div class="conv-head-text">
                    <h2 id="conv-title" tabindex="-1"><bdi translate="no">{name.clone()}</bdi>
                        {tag.map(|t| view! { <span class="tag" translate="no">{t}</span> })}</h2>
                    <p>{glyph(glyph::LOCK)}<span>{subtitle}</span></p>
                </div>
                <button type="button" id="conv-details-toggle" class="icon-button" data-warn=move || changed.get().then_some("true")
                    aria-expanded=move || details.get().to_string() aria-controls="conv-details"
                    aria-label=words::chat_details(lang) on:click=move |_| details.update(|d| *d = !*d)>
                    {move || if changed.get() { glyph(glyph::SHIELD_ALERT) } else { glyph(glyph::INFO) }}
                </button>
            </header>
        }
        .into_any()
    }

    fn details_view(self) -> Option<AnyView> {
        let lang = self.lang;
        if !self.details.get() {
            return None;
        }
        let group = move || {
            self.group_now.get().map(|g| {
                people::group_panel(lang, &g, self.own, self.ctl, self.friends, self.selected)
            })
        };
        let close = move |_| {
            self.details.set(false);
            focus("conv-details-toggle");
        };
        Some(
            view! {
                <aside id="conv-details" class="conv-details" aria-labelledby="conv-details-title">
                    <header class="conv-details-head">
                        <h3 id="conv-details-title">{words::chat_details(lang)}</h3>
                        <button type="button" class="icon-button" aria-label=words::chat_close(lang) on:click=close>
                            {glyph(glyph::CLOSE)}
                        </button>
                    </header>
                    {group}
                    {safety(lang, self.reviews, self.ctl)}
                    <p class="conv-fan">
                        {glyph(glyph::INFO)}
                        <span>{rich(words::chat_fan(lang), move |tag| (tag == "more").then(|| format!("/{}/fan-content", lang.code())))}</span>
                    </p>
                </aside>
            }
            .into_any(),
        )
    }

    fn banners(self) -> AnyView {
        let lang = self.lang;
        let ctl = self.ctl;
        let (refused, error) = self.trouble.get();
        view! {
            {self.changed.get().then(|| view! {
                <p class="banner bad" role="alert">{glyph(glyph::SHIELD_ALERT)}<span>{words::chat_keys_changed(lang)}</span></p>
            })}
            {refused.then(|| view! { <p class="banner" role="status">{words::chat_refused(lang)}</p> })}
            {error.then(|| view! {
                <p class="banner bad" role="alert">
                    <span>{words::chat_failed(lang)}</span>
                    <button type="button" class="button small" on:click=move |_| ctl.poll()>{words::chat_retry(lang)}</button>
                </p>
            })}
        }
        .into_any()
    }

    fn pending(self) -> Option<AnyView> {
        let (lang, ctl) = (self.lang, self.ctl);
        let (text, failed, busy) = self.waiting.get()?;
        let actions = failed.then(|| view! {
            <span class="msg-actions">
                <button type="button" class="button small" aria-disabled=busy.then_some("true")
                    on:click=move |_| if !busy { ctl.retry() }>{words::chat_retry(lang)}</button>
                <button type="button" class="button small danger" aria-disabled=busy.then_some("true")
                    on:click=move |_| if !busy { ctl.discard() }>{words::chat_discard(lang)}</button>
            </span>
        });
        Some(
            view! {
                <div class="msg pending" data-own="true" data-first="true">
                    <p class="msg-text">{text}</p>
                    <span class="msg-state" role=failed.then_some("alert")>
                        {if failed { words::chat_not_sent(lang) } else { words::chat_sending(lang) }}
                    </span>
                    {actions}
                </div>
            }
            .into_any(),
        )
    }

    fn note(self) -> Option<AnyView> {
        let lang = self.lang;
        let loading = Memo::new(move |_| self.ctl.shown.with(|s| s.loading));
        if loading.get() {
            Some(
                view! { <p class="msgr-note" role="status">{words::account_loading(lang)}</p> }
                    .into_any(),
            )
        } else if self.messages.with(Vec::is_empty) && self.waiting.with(Option::is_none) {
            Some(view! { <p class="msgr-note">{words::chat_empty(lang)}</p> }.into_any())
        } else {
            None
        }
    }

    fn day(self, (key, label, list): (String, String, Vec<(Message, bool)>)) -> AnyView {
        let (lang, own, ctl, group) = (self.lang, self.own, self.ctl, self.group());
        let known = move |tag: u32| {
            ctl.shown
                .with_untracked(|s| s.known.get(&tag).cloned())
                .unwrap_or_else(|| format!("#{}", tag_text(tag)))
        };
        let items = list
            .into_iter()
            .map(|(m, first)| {
                let mine = m.sender == own;
                let who = if mine { words::chat_you(lang).to_owned() } else { known(m.sender) };
                let sender = (group && !mine && first)
                    .then(|| view! { <span class="msg-sender"><bdi translate="no">{who.clone()}</bdi></span> });
                let prefix = format!("{who}, {}: ", time::clock(m.at));
                view! {
                    <li class="msg" data-own=mine.then_some("true") data-first=first.then_some("true")>
                        {sender}
                        <span class="visually-hidden">{prefix}</span>
                        <p class="msg-text" class:emote=m.emote>{m.text}</p>
                        <time datetime=time::machine(m.at) aria-hidden="true">{time::clock(m.at)}</time>
                    </li>
                }
            })
            .collect_view();
        let (heading, labelled) = (format!("day-{key}"), format!("day-{key}"));
        view! {
            <section class="conv-day" aria-labelledby=labelled>
                <h3 id=heading class="conv-day-head"><span>{label}</span></h3>
                <ol class="conv-msgs">{items}</ol>
            </section>
        }
        .into_any()
    }

    fn scroll(self) -> AnyView {
        let (lang, messages, visible) = (self.lang, self.messages, self.visible);
        let by_day = Memo::new(move |_| {
            let count = visible.get();
            messages.with(|m| days(lang, &m[m.len().saturating_sub(count)..]))
        });
        let scroller: NodeRef<html::Div> = NodeRef::new();
        let follow = StoredValue::new(true);
        let older_height = StoredValue::new(None::<i32>);
        Effect::new(move |_| {
            let _ = (
                messages.with(Vec::len),
                self.ctl.shown.with(|s| s.pending.is_some()),
                visible.get(),
            );
            request_animation_frame(move || {
                let Some(node) = scroller.get_untracked() else {
                    return;
                };
                if let Some(before) = older_height.get_value() {
                    node.set_scroll_top(node.scroll_top() + node.scroll_height() - before);
                    older_height.set_value(None);
                } else if follow.get_value() {
                    node.set_scroll_top(node.scroll_height());
                }
            });
        });
        let on_scroll = move |_| {
            if let Some(node) = scroller.get_untracked() {
                follow.set_value(
                    node.scroll_height() - node.scroll_top() - node.client_height() < 80,
                );
            }
        };
        let older = move |_| {
            if let Some(node) = scroller.get_untracked() {
                older_height.set_value(Some(node.scroll_height()));
            }
            visible.update(|v| *v += 100);
        };
        let more = move || {
            (messages.with(Vec::len) > visible.get()).then(|| view! {
                <button type="button" class="button small conv-older" on:click=older>{words::chat_older(lang)}</button>
            })
        };
        view! {
            <div class="conv-scroll" node_ref=scroller on:scroll=on_scroll tabindex="0" role="region"
                aria-label=words::chat_messages(lang)>
                {more}
                {move || self.note()}
                <For each=move || by_day.get() key=|d| (d.0.clone(), d.2.len(), d.2.last().map(|m| m.0.id.clone()))
                    children=move |day| self.day(day) />
                {move || self.pending()}
            </div>
        }
        .into_any()
    }

    fn footer(self) -> AnyView {
        let lang = self.lang;
        if self.changed.get() {
            let details = self.details;
            view! {
                <div class="composer-blocked">
                    <p>{words::chat_compare_first(lang)}</p>
                    <button type="button" class="button" on:click=move |_| details.set(true)>{words::chat_compare(lang)}</button>
                </div>
            }
            .into_any()
        } else if self.can_send.get() {
            composer(lang, self.id.get_value(), self.members, self.ctl).into_any()
        } else {
            let text = self.conv.with(|c| match c.as_ref().map(|c| &c.kind) {
                Some(Kind::Direct(_)) => words::chat_friend_required(lang),
                _ => words::chat_group_left(lang),
            });
            view! { <p class="composer-closed">{text}</p> }.into_any()
        }
    }

    /// One polite announcement for what arrives here, at most every 3 s.
    fn announcer(self) -> AnyView {
        let (lang, own, ctl, messages) = (self.lang, self.own, self.ctl, self.messages);
        let announce = RwSignal::new(String::new());
        let heard = StoredValue::new((messages.with_untracked(Vec::len), 0u64));
        Effect::new(move |_| {
            let (count, fresh) = messages.with(|m| {
                let (before, _) = heard.get_value();
                let fresh: Vec<u32> = m
                    .iter()
                    .skip(before)
                    .filter(|m| m.sender != own)
                    .map(|m| m.sender)
                    .collect();
                (m.len(), fresh)
            });
            let (_, last) = heard.get_value();
            let now = time::now();
            if fresh.is_empty() || now.saturating_sub(last) < 3_000 {
                heard.update_value(|h| h.0 = count);
                return;
            }
            let text = if let [one] = fresh.as_slice() {
                let who = ctl
                    .shown
                    .with_untracked(|s| s.known.get(one).cloned())
                    .unwrap_or_default();
                words::chat_new_from(lang, &who)
            } else {
                words::chat_new_many(
                    lang,
                    &words::number(lang, u64::try_from(fresh.len()).unwrap_or(0)),
                )
            };
            announce.set(text);
            heard.set_value((count, now));
        });
        view! { <p class="visually-hidden" aria-live="polite">{move || announce.get()}</p> }
            .into_any()
    }
}

fn safety(lang: Lang, reviews: Memo<Vec<Review>>, ctl: Ctl) -> impl IntoView {
    move || {
        let list = reviews.get();
        (!list.is_empty()).then(|| {
            let items = list
                .into_iter()
                .map(|r| {
                    let confirm = r.changed.then(|| {
                        let keys = r.snapshot.clone();
                        view! {
                            <button type="button" class="button small" on:click=move |_| ctl.confirm(keys.clone())>
                                {words::chat_confirm_keys(lang)}
                            </button>
                        }
                    });
                    view! {
                        <li class="safety-item" data-warn=r.changed.then_some("true")>
                            <span class="safety-who"><bdi translate="no">{r.snapshot.handle.clone()}</bdi>
                                {r.changed.then(|| view! { <span class="pill bad">{words::chat_changed(lang)}</span> })}</span>
                            <p class="safety-number" translate="no">{r.grouped()}</p>
                            {confirm}
                        </li>
                    }
                })
                .collect_view();
            view! {
                <section class="safety" aria-labelledby="safety-title">
                    <h4 id="safety-title">{glyph(glyph::SHIELD_CHECK)}<span>{words::chat_safety(lang)}</span></h4>
                    <p class="meta">{words::chat_safety_lead(lang)}</p>
                    <ul>{items}</ul>
                </section>
            }
        })
    }
}

fn composer(lang: Lang, conv: String, members: Memo<Vec<u32>>, ctl: Ctl) -> impl IntoView {
    let text = RwSignal::new(String::new());
    {
        let conv = conv.clone();
        Effect::new(move |_| {
            let back = ctl
                .shown
                .with(|s| s.returned.clone().filter(|(c, _)| *c == conv));
            if let Some((_, returned)) = back {
                text.set(returned);
                ctl.shown.update(|s| s.returned = None);
            }
        });
    }
    let blocked = move || {
        ctl.shown
            .with(|s| s.busy || s.pending.is_some() || s.phase != Phase::Ready)
    };
    let send = move || {
        let body = text.get_untracked();
        if body.trim().is_empty()
            || ctl
                .shown
                .with_untracked(|s| s.busy || s.pending.is_some() || s.phase != Phase::Ready)
        {
            return;
        }
        text.set(String::new());
        ctl.send(conv.clone(), body, members.get_untracked());
    };
    let submit = {
        let send = send.clone();
        move |event: leptos::ev::SubmitEvent| {
            event.prevent_default();
            send();
        }
    };
    let keys = move |event: leptos::ev::KeyboardEvent| match event.key().as_str() {
        "Enter" if !event.shift_key() && !event.is_composing() => {
            if event.ctrl_key() || event.meta_key() || !coarse() {
                event.prevent_default();
                send();
            }
        }
        "Escape" => {
            if let Some(row) = document()
                .query_selector(".conv-row[aria-current]")
                .ok()
                .flatten()
                && let Ok(row) = row.dyn_into::<web_sys::HtmlElement>()
            {
                let _ = row.focus();
            }
        }
        _ => {}
    };
    let count = move || {
        let n = text.with(|t| t.chars().count());
        (n >= 900)
            .then(|| words::chat_count(lang, &words::number(lang, u64::try_from(n).unwrap_or(0))))
    };
    view! {
        <form class="composer" on:submit=submit>
            <label class="visually-hidden" for="composer-text">{words::chat_write(lang)}</label>
            <textarea id="composer-text" rows="1" maxlength="1000" placeholder=words::chat_write(lang)
                aria-describedby="composer-hint" prop:value=move || text.get()
                on:input=move |ev| text.set(event_target_value(&ev)) on:keydown=keys></textarea>
            <button type="submit" class="send" aria-label=words::chat_send(lang)
                aria-disabled=move || (blocked() || text.with(|t| t.trim().is_empty())).then_some("true")>
                {glyph(glyph::SEND)}
            </button>
            <p id="composer-hint" class="composer-hint">
                <span>{words::chat_enter_hint(lang)}</span>
                <span class="composer-count" aria-live="polite">{count}</span>
            </p>
        </form>
    }
}
