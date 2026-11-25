//! The messenger's window: the list beside the open conversation (one of
//! the two at a time on a phone), the conversation's messages by day, the
//! message on its way, the composer and the conversation's details.
//! Every text a person wrote is rendered as text, never as markup.

use std::time::Duration;

use central_api::picture::PictureKey;
use central_api::social::{
    CardPrinting, CardSuggestion, FINISH_ETCHED, FINISH_FOIL, FINISH_NONFOIL, Person, Relation,
};
use leptos::html;
use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use leptos::web_sys;

use super::super::field;
use super::super::ui::copy_handle::handle_with_copy;
use super::super::ui::rank_avatar::{rank_canvas, rank_of, rank_scale_class};
use super::cache;
use super::glyph::{self, glyph};
use super::mentions::{self, CardFinish, Trigger, TriggerKind};
use super::model::{self, Conversation, Kind, Message, Review, Sources, initials, tag_text};
use super::people;
use super::preview::PreviewCtl;
use super::time;
use super::{Ctl, Me, Phase, Tab, href, missing_view, remember, wire};
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

/// Opens a native `<dialog>` element modally via `showModal()` once mounted
/// (SSR-safe, without requiring extra `web-sys` feature flags).
pub(in super::super) fn open_modal_dialog(id: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        use js_sys::{Function, Reflect};
        use wasm_bindgen::JsValue;
        let id = id.to_owned();
        let _ = request_animation_frame(move || {
            let Some(el) = document().get_element_by_id(&id) else {
                return;
            };
            let already_open = Reflect::get(&el, &JsValue::from_str("open"))
                .ok()
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if already_open {
                return;
            }
            if let Some(show) = Reflect::get(&el, &JsValue::from_str("showModal"))
                .ok()
                .and_then(|f| f.dyn_into::<Function>().ok())
            {
                let _ = show.call0(&el);
            }
        });
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = id;
    }
}

/// An avatar: the person's picture where `tag` has one
/// (`docs/profile-pictures.md`), else initials or the group sign, in one of
/// six tints.
/// Always a mark of the same size: the initials stand again whenever the
/// picture goes. Its name stands beside it, so it is hidden from screen
/// readers and its picture has no text of its own.
pub fn avatar(name: &str, seed: &str, group: bool, tag: Option<u32>) -> AnyView {
    let face = if group {
        glyph(glyph::USERS)
    } else {
        let name = name.to_owned();
        (move || match tag.and_then(crate::islands::picture::face_of) {
            Some((key, moving)) => {
                crate::islands::picture::face(&key, moving, String::new()).into_any()
            }
            None => initials(&name).into_any(),
        })
        .into_any()
    };
    let hue = model::hue(seed).to_string();
    if group {
        return view! { <span class="avatar" data-hue=hue aria-hidden="true">{face}</span> }
            .into_any();
    }
    let plate_hue = hue.clone();
    let class = move || match tag.and_then(rank_of) {
        Some(rank) => format!(
            "avatar rank-avatar {} has-rank",
            rank_scale_class(Some(&rank))
        ),
        None => "avatar".to_owned(),
    };
    let frame = move || {
        tag.and_then(rank_of)
            .map(|rank| rank_canvas("frame", rank, false, 40))
    };
    let presence_dot = move || {
        tag.and_then(super::socket::presence_of).map(|state| {
            let attr = state.attr();
            view! { <span class="presence-dot" data-state=attr aria-hidden="true">{""}</span> }
        })
    };
    view! {
        <span class=class data-hue=hue aria-hidden="true">
            <span class="rank-avatar-plate" data-hue=plate_hue>{face}</span>
            {frame}
            {presence_dot}
        </span>
    }
    .into_any()
}

/// A group's avatar: its uploaded picture when present, else the users glyph
/// in its seeded tint.
pub fn group_avatar(seed: &str, picture: Option<(PictureKey, bool)>) -> AnyView {
    let hue = model::hue(seed).to_string();
    let face = match picture {
        Some((key, moving)) => {
            crate::islands::picture::face(&key, moving, String::new()).into_any()
        }
        None => glyph(glyph::USERS),
    };
    view! { <span class="avatar" data-hue=hue aria-hidden="true">{face}</span> }.into_any()
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

fn msgr_header(lang: Lang, me: &Me, making: RwSignal<bool>) -> impl IntoView {
    let own = me.tag;
    view! {
        <header class="msgr-me">
            {avatar(&me.name, &own.to_string(), false, Some(own))}
            <span class="msgr-me-name">
                {handle_with_copy(lang, me.name.clone(), tag_text(own), true)}
            </span>
            <button
                type="button"
                id="msgr-new-group-btn"
                class="icon-button msgr-me-action"
                title=words::chat_new_group(lang)
                aria-label=words::chat_new_group(lang)
                aria-expanded=move || making.get().to_string()
                aria-controls="new-group-dialog"
                on:click=move |_| making.set(true)
            >
                {glyph(glyph::PLUS)}
            </button>
        </header>
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MainView {
    Missing,
    Ended,
    Ready,
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
    let making = RwSignal::new(false);
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
    let add_contact = Callback::new(move |()| {
        remember(lang, Tab::Contacts, None);
        tab.set(Tab::Contacts);
        selected.set(None);
        crate::islands::focus::after_render("#contact-find");
    });
    let me_store = StoredValue::new(me.clone());
    let side = move || match tab.get() {
        Tab::Chats => {
            chats_panel(lang, own, ctl, lists.rows, selected, choose, add_contact).into_any()
        }
        Tab::Contacts => people::contacts_panel(lang, own, ctl, choose).into_any(),
    };
    let current = Memo::new(move |_| {
        selected
            .get()
            .filter(|id| lists.rows.with(|r| r.iter().any(|c| c.id == *id)))
    });
    let phase = Memo::new(move |_| match ctl.shown.with(|s| s.phase) {
        Phase::Missing => MainView::Missing,
        Phase::Ended => MainView::Ended,
        Phase::Opening | Phase::Ready | Phase::Failed => MainView::Ready,
    });
    let main = move || {
        match phase.get() {
        MainView::Missing => {
            missing_view(lang, worker.get_value(), me_store.get_value(), ctl).into_any()
        }
        MainView::Ended => view! {
            <div class="msgr-welcome" role="alert">
                <h2>{words::chat_ended(lang)}</h2>
                <p><a class="button primary" href=format!("/{}/signin", lang.code())>{words::nav_sign_in(lang)}</a></p>
            </div>
        }
        .into_any(),
        MainView::Ready => match current.get() {
            Some(id) => conversation(lang, &id, own, ctl, lists, selected),
            None => welcome(lang, ctl).into_any(),
        },
    }
    };
    let new_group_modal = move || {
        making.get().then(|| {
            people::new_group_dialog(
                lang,
                own,
                ctl,
                lists.friends,
                Callback::new(move |id: String| {
                    making.set(false);
                    choose.run(format!("grp:{id}"));
                }),
                Callback::new(move |()| {
                    making.set(false);
                    focus("msgr-new-group-btn");
                }),
            )
        })
    };
    view! {
        <div class="messenger" data-pane=pane>
            <section class="msgr-side" aria-label=words::chat_side(lang)>
                {msgr_header(lang, me, making)}
                {tabs(lang, lists, tab, selected)}
                {side}
            </section>
            <section class="msgr-main" aria-label=words::chat_main(lang)>{main}</section>
            {new_group_modal}
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

/// What the list waits for before it may say that it is empty.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Known {
    /// The contacts, the groups or the inbox's first read are still to come.
    Waiting,
    /// All three are here.
    Loaded,
    /// The contacts or the groups could not be read.
    Failed,
}

impl Known {
    /// An empty list is said to be empty only once everything it is made
    /// of arrived; a failure says so, never an empty list in its place.
    /// The inbox is waited for only while the chat opens or is open: one
    /// that could not be opened, or whose session ended, reads nothing.
    fn watch(ctl: Ctl) -> Memo<Known> {
        Memo::new(move |_| {
            let contacts = ctl.contacts.with(|c| c.as_ref().map(Result::is_ok));
            let groups = ctl.groups.with(|g| g.as_ref().map(Result::is_ok));
            let reading = ctl
                .shown
                .with(|s| s.loading && matches!(s.phase, Phase::Opening | Phase::Ready));
            match (contacts, groups) {
                (Some(false), _) | (_, Some(false)) => Known::Failed,
                (Some(true), Some(true)) if !reading => Known::Loaded,
                _ => Known::Waiting,
            }
        })
    }
}

/// The list's failure, above whatever it could show, with a way to try again.
fn list_failed(lang: Lang, ctl: Ctl, known: Memo<Known>) -> impl IntoView {
    move || {
        (known.get() == Known::Failed).then(|| {
            view! {
                <p class="banner bad" role="alert">
                    {glyph(glyph::INFO)}<span>{words::chat_failed(lang)}</span>
                    <button type="button" class="button small quiet" on:click=move |_| {
                        ctl.reload_contacts();
                        ctl.reload_groups();
                    }>{words::chat_retry(lang)}</button>
                </p>
            }
        })
    }
}

/// No conversation at all: still loading, or a new account's first step.
fn list_empty(lang: Lang, known: Known, add_contact: Callback<()>) -> AnyView {
    match known {
        Known::Failed => ().into_any(),
        Known::Waiting => {
            view! { <p class="msgr-note" role="status">{words::account_loading(lang)}</p> }
                .into_any()
        }
        Known::Loaded => view! {
            <div class="msgr-empty">
                <span class="msgr-empty-icon">{glyph(glyph::MESSAGE)}</span>
                <p class="msgr-empty-title">{words::chat_quiet(lang)}</p>
                <p>{words::chat_no_conversations(lang)}</p>
                <a class="button small" href=href(lang, Tab::Contacts, None)
                    on:click=move |event: leptos::ev::MouseEvent| {
                        if !(event.ctrl_key() || event.meta_key() || event.shift_key()) {
                            event.prevent_default();
                            add_contact.run(());
                        }
                    }>
                    {glyph(glyph::USER_PLUS)}<span>{words::chat_add_contact(lang)}</span>
                </a>
            </div>
        }
        .into_any(),
    }
}

/// Ctrl/Cmd+K finds a conversation while the Chats half is shown.
fn find_shortcut() {
    let find = window_event_listener(leptos::ev::keydown, move |event| {
        if (event.ctrl_key() || event.meta_key()) && event.key().eq_ignore_ascii_case("k") {
            event.prevent_default();
            focus("chat-search");
        }
    });
    on_cleanup(move || find.remove());
}

fn chat_search_bar(lang: Lang, query: RwSignal<String>) -> impl IntoView {
    view! {
        <div class="msgr-bar">
            <div class="inset msgr-find">
                <label class="visually-hidden" for="chat-search">{words::chat_search(lang)}</label>
                {field::mark(glyph::SEARCH)}
                <input id="chat-search" type="search" autocomplete="off" spellcheck="false"
                    placeholder=words::chat_search_hint(lang)
                    on:input=move |ev| query.set(event_target_value(&ev)) prop:value=move || query.get() />
                <Show when=move || !query.with(String::is_empty)>
                    <button
                        type="button"
                        class="inset-end"
                        title=words::chat_search_clear(lang)
                        aria-label=words::chat_search_clear(lang)
                        on:click=move |_| {
                            query.set(String::new());
                            focus("chat-search");
                        }
                    >
                        {glyph(glyph::CLOSE)}
                    </button>
                </Show>
            </div>
        </div>
    }
}

fn conv_row_key(
    c: &Conversation,
) -> (
    String,
    Option<String>,
    usize,
    String,
    bool,
    Option<PictureKey>,
    bool,
) {
    let (pic, moving) = match &c.kind {
        Kind::Group(g) => (g.picture_key, g.picture_moving),
        _ => (None, false),
    };
    (
        c.id.clone(),
        c.last.as_ref().map(|m| m.id.clone()),
        c.unread,
        c.title.clone(),
        c.open,
        pic,
        moving,
    )
}

fn conv_list_keys(
    event: &leptos::ev::KeyboardEvent,
    visible: Memo<Vec<Conversation>>,
    anchor: Memo<Option<String>>,
    rove: RwSignal<Option<String>>,
) {
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
}

/// The Chats half: search, a new group, invitations, the list.
fn chats_panel(
    lang: Lang,
    own: u32,
    ctl: Ctl,
    rows: Memo<Vec<Conversation>>,
    selected: RwSignal<Option<String>>,
    choose: Callback<String>,
    add_contact: Callback<()>,
) -> impl IntoView {
    let query = RwSignal::new(String::new());
    let shown = RwSignal::new(40_usize);
    let panel: NodeRef<html::Div> = NodeRef::new();
    let visible = Memo::new(move |_| {
        let needle = query.get().trim().to_lowercase();
        rows.with(|r| {
            r.iter()
                .filter(|c| needle.is_empty() || c.title.to_lowercase().contains(&needle))
                .cloned()
                .collect::<Vec<_>>()
        })
    });
    Effect::new(move |_| {
        query.track();
        shown.set(40);
    });
    let total = Memo::new(move |_| visible.with(Vec::len));
    let paged = Memo::new(move |_| {
        let limit = shown.get();
        visible.with(|v| v.iter().take(limit).cloned().collect::<Vec<_>>())
    });
    let rove = RwSignal::new(None::<String>);
    let anchor = Memo::new(move |_| {
        paged.with(|v| {
            let has = |id: &String| v.iter().any(|c| c.id == *id);
            rove.get()
                .filter(has)
                .or_else(|| selected.get().filter(has))
                .or_else(|| v.first().map(|c| c.id.clone()))
        })
    });
    let pick = Pick {
        selected,
        anchor,
        rove,
        choose,
    };
    find_shortcut();
    let known = Known::watch(ctl);
    let instead = Memo::new(move |_| {
        visible
            .with(Vec::is_empty)
            .then(|| query.with(|q| q.trim().is_empty()).then(|| known.get()))
    });
    let on_scroll = move |_| {
        let Some(node) = panel.get_untracked() else {
            return;
        };
        if node.scroll_height() - node.scroll_top() - node.client_height() < 160
            && shown.get_untracked() < total.get_untracked()
        {
            shown.update(|n| *n += 40);
        }
    };
    let list = move || {
        match instead.get() {
        Some(None) => view! { <p class="msgr-note">{words::chat_no_match(lang)}</p> }.into_any(),
        Some(Some(known)) => list_empty(lang, known, add_contact),
        None => view! {
            <nav class="conv-nav" aria-label=words::chat_conversations(lang)>
                <ul class="conv-list" on:keydown=move |ev| conv_list_keys(&ev, paged, anchor, rove)>
                    <For each=move || paged.get() key=conv_row_key
                        children=move |c| row(lang, &c, own, ctl, pick) />
                </ul>
            </nav>
            {move || (total.get() > shown.get()).then(|| view! {
                <button type="button" class="button small conv-more" on:click=move |_| shown.update(|n| *n += 40)>
                    {words::admin_more(lang)}
                </button>
            })}
        }
        .into_any(),
    }
    };
    view! {
        <div class="msgr-panel" node_ref=panel on:scroll=on_scroll>
            {chat_search_bar(lang, query)}
            {people::invites(lang, own, ctl, choose)}
            {list_failed(lang, ctl, known)}
            {list}
        </div>
    }
}

/// The list's choosing: the open conversation, the row that holds the one
/// tab stop, the row last focused, and opening one.
#[derive(Clone, Copy)]
struct Pick {
    selected: RwSignal<Option<String>>,
    anchor: Memo<Option<String>>,
    rove: RwSignal<Option<String>>,
    choose: Callback<String>,
}

fn row(lang: Lang, c: &Conversation, own: u32, ctl: Ctl, pick: Pick) -> AnyView {
    let Pick {
        selected,
        anchor,
        rove,
        choose,
    } = pick;
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
    // the last line, and who wrote it where that is not plain: oneself, or
    // a member of a group
    let who = match (&c.last, &c.kind) {
        (Some(m), _) if m.sender == own => Some(
            view! { <span class="conv-who">{format!("{}: ", words::chat_you(lang))}</span> }
                .into_any(),
        ),
        (Some(m), Kind::Group(_) | Kind::Former) => ctl
            .shown
            .with_untracked(|s| s.known.get(&m.sender).cloned())
            .map(|name| {
                view! { <span class="conv-who"><bdi translate="no">{name}</bdi>{": "}</span> }
                    .into_any()
            }),
        _ => None,
    };
    let preview = match (&c.last, &c.kind) {
        (Some(m), _) => mentions::plain_preview(&m.text),
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
                {match &c.kind {
                    Kind::Direct(p) => avatar(&name, &id, false, Some(p.tag)),
                    Kind::Group(g) => {
                        group_avatar(&id, g.picture_key.map(|k| (k, g.picture_moving)))
                    }
                    Kind::Former => group_avatar(&id, None),
                }}
                <span class="conv-main">
                    <span class="conv-top">
                        <bdi class="conv-name" translate="no">{name.clone()}</bdi>{tag}{when}
                    </span>
                    <span class="conv-bottom">
                        <span class="conv-preview">{who}{preview}</span>{unread}
                    </span>
                </span>
            </a>
        </li>
    }
    .into_any()
}
/// A message, whether it starts a run of one sender's lines, and whether it
/// ends one.
type Line = (Message, bool, bool);

/// A conversation's messages by day.
type Days = Vec<(String, String, Vec<Line>)>;

fn days(lang: Lang, messages: &[Message]) -> Days {
    let now = time::now();
    let mut out: Days = Vec::new();
    let mut previous: Option<&Message> = None;
    for (m, (first, last)) in messages.iter().zip(model::runs(messages, time::same_day)) {
        if previous.is_none_or(|p| !time::same_day(p.at, m.at)) {
            let key = time::machine(m.at).chars().take(10).collect();
            out.push((key, time::day_label(lang, m.at, now), Vec::new()));
        }
        if let Some(day) = out.last_mut() {
            day.2.push((m.clone(), first, last));
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
    mention_people: Memo<Vec<Person>>,
    waiting: Memo<Option<(String, bool, bool)>>,
    trouble: Memo<(bool, bool)>,
    preview: PreviewCtl,
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
    crate::islands::focus::after_render("#conv-title");
    view! {
        <div class="conv">
            {open.head()}
            {move || open.details_view()}
            {move || open.banners()}
            {open.scroll()}
            {move || open.footer()}
            {open.preview.view()}
            {open.announcer()}
        </div>
    }
    .into_any()
}

fn mention_candidates(
    conv: Memo<Option<Conversation>>,
    friends: Memo<Vec<Person>>,
) -> Memo<Vec<Person>> {
    Memo::new(move |_| {
        let mut out: Vec<Person> = Vec::new();
        let mut seen = std::collections::BTreeSet::new();
        conv.with(|c| match c.as_ref().map(|c| &c.kind) {
            Some(Kind::Group(g)) => {
                for m in &g.members {
                    if seen.insert(m.person.tag) {
                        out.push(m.person.clone());
                    }
                }
            }
            Some(Kind::Direct(p)) if seen.insert(p.tag) => {
                out.push(p.clone());
            }
            _ => {}
        });
        friends.with(|list| {
            for p in list {
                if seen.insert(p.tag) {
                    out.push(p.clone());
                }
            }
        });
        out
    })
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
        let mention_people = mention_candidates(conv, friends);
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
            mention_people,
            waiting,
            trouble,
            preview: PreviewCtl::new(lang),
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
            visible: RwSignal::new(60),
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

    /// The other person's tag in a direct conversation.
    fn direct_tag(self) -> Option<u32> {
        self.conv
            .with_untracked(|c| match c.as_ref().map(|c| &c.kind) {
                Some(Kind::Direct(p)) => Some(p.tag),
                _ => None,
            })
    }

    fn group(self) -> bool {
        self.conv.with_untracked(|c| {
            c.as_ref()
                .is_some_and(|c| !matches!(c.kind, Kind::Direct(_)))
        })
    }

    fn head(self) -> AnyView {
        let lang = self.lang;
        let direct_tag = self
            .conv
            .with_untracked(|c| match c.as_ref().map(|c| &c.kind) {
                Some(Kind::Direct(p)) => Some(tag_text(p.tag)),
                _ => None,
            });
        let direct_num = self.direct_tag();
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
            // the conversation goes; its row in the list takes the focus
            let row = self.id.with_value(|id| row_id(id));
            crate::islands::focus::after_render(format!("#{row}"));
        };
        let name = self.name();
        let id = self.id.get_value();
        let head_face = self.head_face(id, name.clone());
        let group_title = {
            let fallback = name.clone();
            let group_now = self.group_now;
            move || {
                group_now.with(|g| {
                    g.as_ref()
                        .map_or_else(|| fallback.clone(), |g| g.name.clone())
                })
            }
        };
        view! {
            <header class="conv-head">
                <button type="button" class="icon-button conv-back" aria-label=words::chat_back(lang) on:click=back>
                    {glyph(glyph::BACK)}
                </button>
                {head_face}
                <div class="conv-head-text">
                    <h2 id="conv-title" tabindex="-1">
                        {match direct_tag {
                            Some(tag_hex) => handle_with_copy(lang, name.clone(), tag_hex, true),
                            None => view! { <bdi class="conv-head-name" translate="no">{group_title}</bdi> }.into_any(),
                        }}
                    </h2>
                    <p class="conv-head-state">
                        {glyph(glyph::LOCK)}
                        <span>{subtitle}</span>
                        {move || self.head_presence(direct_num)}
                    </p>
                </div>
                {self.head_actions()}
            </header>
        }
        .into_any()
    }

    fn head_presence(self, direct_num: Option<u32>) -> impl IntoView {
        let lang = self.lang;
        direct_num
            .and_then(super::socket::presence_of)
            .map(|p_state| {
                let attr = p_state.attr();
                let label = p_state.label(lang);
                view! {
                    <span class="conv-head-presence" data-state=attr>
                        <span class="conv-head-sep" aria-hidden="true">{"·"}</span>
                        <span class="presence-dot" data-state=attr aria-hidden="true">{""}</span>
                        <span>{label}</span>
                    </span>
                }
            })
    }

    fn head_actions(self) -> impl IntoView {
        let lang = self.lang;
        let (changed, details, preview) = (self.changed, self.details, self.preview);
        let previews_on = preview.previews_enabled;
        view! {
            <div class="conv-head-actions">
                <button
                    type="button"
                    id="chat-card-previews-toggle"
                    class="icon-button"
                    aria-pressed=move || previews_on.get().to_string()
                    data-active=move || previews_on.get().then_some("true")
                    title=move || if previews_on.get() { words::chat_card_previews_on(lang) } else { words::chat_card_previews_off(lang) }
                    aria-label=move || if previews_on.get() { words::chat_card_previews_on(lang) } else { words::chat_card_previews_off(lang) }
                    on:click=move |_| preview.toggle_previews()
                >
                    {move || if previews_on.get() { glyph(glyph::EYE) } else { glyph(glyph::EYE_OFF) }}
                </button>
                <button type="button" id="conv-details-toggle" class="icon-button" data-warn=move || changed.get().then_some("true")
                    aria-expanded=move || details.get().to_string() aria-controls="conv-details"
                    aria-label=words::chat_details(lang) on:click=move |_| details.update(|d| *d = !*d)>
                    {move || if changed.get() { glyph(glyph::SHIELD_ALERT) } else { glyph(glyph::INFO) }}
                </button>
            </div>
        }
    }

    fn head_face(self, id: String, name: String) -> impl IntoView + 'static {
        let is_group = self.group();
        let direct = self.direct_tag();
        let group_now = self.group_now;
        move || {
            if is_group {
                let pic = group_now.with(|g| {
                    g.as_ref()
                        .and_then(|g| g.picture_key.map(|k| (k, g.picture_moving)))
                });
                group_avatar(&id, pic)
            } else {
                avatar(&name, &id, false, direct)
            }
        }
    }

    fn details_view(self) -> Option<AnyView> {
        let lang = self.lang;
        if !self.details.get() {
            return None;
        }
        open_modal_dialog("conv-details");
        let backdrop_down = RwSignal::new(false);
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
                <dialog
                    id="conv-details"
                    class="conv-details-dialog"
                    aria-labelledby="conv-details-title"
                    on:pointerdown=move |ev: leptos::ev::PointerEvent| {
                        backdrop_down.set(ev.target() == ev.current_target());
                    }
                    on:click=move |ev: leptos::ev::MouseEvent| {
                        if backdrop_down.get_untracked() && ev.target() == ev.current_target() {
                            self.details.set(false);
                            focus("conv-details-toggle");
                        }
                    }
                    on:close=move |_| {
                        if self.details.get_untracked() {
                            self.details.set(false);
                        }
                        focus("conv-details-toggle");
                    }
                >
                    <div class="conv-details-card">
                        <header class="conv-details-head">
                            <h3 id="conv-details-title">{words::chat_details(lang)}</h3>
                            <button
                                type="button"
                                class="dialog-close"
                                title=words::chat_close(lang)
                                aria-label=words::chat_close(lang)
                                on:click=close
                            >
                                {glyph(glyph::CLOSE)}
                            </button>
                        </header>
                        <div class="conv-details-body">
                            {group}
                            {move || self.direct_tag().and_then(|tag| {
                                crate::islands::picture::key_of(tag).map(|key| crate::islands::picture::report(lang, tag, key))
                            })}
                            {safety(lang, self.reviews, self.ctl)}
                            <p class="conv-fan">
                                {glyph(glyph::INFO)}
                                <span>{rich(words::chat_fan(lang), move |tag| (tag == "more").then(|| format!("/{}/fan-content", lang.code())))}</span>
                            </p>
                        </div>
                    </div>
                </dialog>
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
            {refused.then(|| view! {
                <p class="banner" role="status">{glyph(glyph::INFO)}<span>{words::chat_refused(lang)}</span></p>
            })}
            {error.then(|| view! {
                <p class="banner bad" role="alert">
                    {glyph(glyph::INFO)}<span>{words::chat_failed(lang)}</span>
                    <button type="button" class="button small quiet" on:click=move |_| ctl.poll()>{words::chat_retry(lang)}</button>
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
                <button type="button" class="button small quiet" aria-disabled=busy.then_some("true")
                    on:click=move |_| if !busy { ctl.retry() }>{words::chat_retry(lang)}</button>
                <button type="button" class="button small danger" aria-disabled=busy.then_some("true")
                    on:click=move |_| if !busy { ctl.discard() }>{words::chat_discard(lang)}</button>
            </span>
        });
        Some(
            view! {
                <div class="msg pending" data-own="true" data-first="true" data-last="true"
                    data-failed=failed.then_some("true")>
                    <div class="msg-bubble">
                        <p class="msg-text">{mentions::render_message_with(lang, &text, self.own, Some(self.preview))}</p>
                    </div>
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
            Some(self.start())
        } else {
            None
        }
    }

    /// A conversation without a message: its face, and where writing is
    /// possible, a way to the composer.
    fn start(self) -> AnyView {
        let lang = self.lang;
        let id = self.id.get_value();
        let name = self.name();
        let is_group = self.group();
        let direct = self.direct_tag();
        let group_now = self.group_now;
        let face = move || {
            if is_group {
                let pic = group_now.with(|g| {
                    g.as_ref()
                        .and_then(|g| g.picture_key.map(|k| (k, g.picture_moving)))
                });
                group_avatar(&id, pic)
            } else {
                avatar(&name, &id, false, direct)
            }
        };
        if self.can_send.get() {
            view! {
                <div class="conv-start">
                    {face}
                    <p class="conv-start-title">{words::chat_say_hello(lang)}</p>
                    <p>{words::chat_choose_lead(lang)}</p>
                    <button type="button" class="button small quiet" on:click=move |_| focus("composer-text")>
                        {glyph(glyph::MESSAGE)}<span>{words::chat_write_first(lang)}</span>
                    </button>
                </div>
            }
            .into_any()
        } else {
            view! {
                <div class="conv-start">
                    {face}
                    <p class="conv-start-title">{words::chat_empty(lang)}</p>
                </div>
            }
            .into_any()
        }
    }

    fn day(self, (key, label, list): (String, String, Vec<Line>)) -> AnyView {
        let (lang, own, ctl, group, preview) =
            (self.lang, self.own, self.ctl, self.group(), self.preview);
        let known = move |tag: u32| {
            ctl.shown
                .with_untracked(|s| s.known.get(&tag).cloned())
                .unwrap_or_else(|| format!("#{}", tag_text(tag)))
        };
        let items = list
            .into_iter()
            .map(|(m, first, last)| {
                let mine = m.sender == own;
                let who = if mine { words::chat_you(lang).to_owned() } else { known(m.sender) };
                // in a group, others' lines stand beside their face, which
                // the run's first line shows with the name; a screen reader
                // hears the name once, in the line's own words
                let beside = group && !mine;
                let face = (beside && first).then(|| {
                    view! {
                        <span class="msg-face">{avatar(&who, &m.sender.to_string(), false, Some(m.sender))}</span>
                    }
                });
                let sender = (beside && first).then(|| {
                    view! { <bdi class="msg-sender" translate="no" aria-hidden="true">{who.clone()}</bdi> }
                });
                let prefix = format!("{who}, {}: ", time::clock(m.at));
                view! {
                    <li class="msg" data-own=mine.then_some("true") data-first=first.then_some("true")
                        data-last=last.then_some("true") data-beside=beside.then_some("true")>
                        {face}
                        <div class="msg-bubble">
                            {sender}
                            <span class="visually-hidden">{prefix}</span>
                            <p class="msg-text" class:emote=m.emote>{mentions::render_message_with(lang, &m.text, own, Some(preview))}</p>
                            <time datetime=time::machine(m.at) aria-hidden="true">{time::clock(m.at)}</time>
                        </div>
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
        let (lang, messages, visible, preview) =
            (self.lang, self.messages, self.visible, self.preview);
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
            let _ = request_animation_frame(move || {
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
            preview.dismiss_hover();
            if let Some(node) = scroller.get_untracked() {
                follow.set_value(
                    node.scroll_height() - node.scroll_top() - node.client_height() < 80,
                );
                if node.scroll_top() < 120
                    && older_height.get_value().is_none()
                    && messages.with_untracked(Vec::len) > visible.get_untracked()
                {
                    older_height.set_value(Some(node.scroll_height()));
                    visible.update(|v| *v += 50);
                }
            }
        };
        let older = move |_| {
            if let Some(node) = scroller.get_untracked() {
                older_height.set_value(Some(node.scroll_height()));
            }
            visible.update(|v| *v += 50);
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
            composer(
                lang,
                self.id.get_value(),
                self.members,
                self.mention_people,
                self.preview,
                self.ctl,
            )
            .into_any()
        } else {
            let text = self.conv.with(|c| match c.as_ref().map(|c| &c.kind) {
                Some(Kind::Direct(_)) => words::chat_friend_required(lang),
                _ => words::chat_group_left(lang),
            });
            view! { <p class="composer-closed">{glyph(glyph::INFO)}<span>{text}</span></p> }
                .into_any()
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
                    let tag = u32::from_str_radix(&r.tag, 16).ok();
                    let who = match r.snapshot.handle.rsplit_once('#') {
                        Some((name, hex)) => {
                            handle_with_copy(lang, name.to_owned(), hex.to_owned(), true)
                        }
                        None => view! { <bdi translate="no">{r.snapshot.handle.clone()}</bdi> }.into_any(),
                    };
                    view! {
                        <li class="safety-item" data-warn=r.changed.then_some("true")>
                            <span class="safety-who">
                                {avatar(&r.snapshot.handle, &r.tag, false, tag)}
                                {who}
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

#[cfg(target_arch = "wasm32")]
fn utf16_to_byte(text: &str, utf16_offset: usize) -> usize {
    let mut seen = 0_usize;
    for (idx, ch) in text.char_indices() {
        if seen >= utf16_offset {
            return idx;
        }
        seen += ch.len_utf16();
    }
    text.len()
}

fn composer_caret_bytes(text: &str) -> usize {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsValue;
        let pos = document()
            .get_element_by_id("composer-text")
            .and_then(|el| js_sys::Reflect::get(&el, &JsValue::from_str("selectionStart")).ok())
            .and_then(|v| v.as_f64())
            .and_then(|n| format!("{:.0}", n.max(0.0).trunc()).parse::<usize>().ok());
        pos.map_or(text.len(), |utf16| utf16_to_byte(text, utf16))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        text.len()
    }
}

fn set_composer_caret(text: &str, byte_index: usize) {
    #[cfg(target_arch = "wasm32")]
    {
        use js_sys::{Function, Reflect};
        use wasm_bindgen::JsValue;
        let end = byte_index.min(text.len());
        let utf16 = u32::try_from(text[..end].encode_utf16().count()).unwrap_or(u32::MAX);
        let _ = request_animation_frame(move || {
            let Some(el) = document().get_element_by_id("composer-text") else {
                return;
            };
            if let Ok(html_el) = el.clone().dyn_into::<web_sys::HtmlElement>() {
                let _ = html_el.focus();
            }
            if let Some(func) = Reflect::get(&el, &JsValue::from_str("setSelectionRange"))
                .ok()
                .and_then(|f| f.dyn_into::<Function>().ok())
            {
                let pos = JsValue::from_f64(f64::from(utf16));
                let _ = func.call2(&el, &pos, &pos);
            }
        });
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (text, byte_index);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum MentionChoice {
    Card(CardSuggestion),
    User(Person),
}

#[derive(Clone, Copy)]
struct ComposerMentions {
    lang: Lang,
    text: RwSignal<String>,
    people: Memo<Vec<Person>>,
    preview: PreviewCtl,
    trigger: RwSignal<Option<Trigger>>,
    parsed_query: RwSignal<Option<mentions::CardTriggerQuery>>,
    cards: RwSignal<Vec<CardSuggestion>>,
    cards_loading: RwSignal<bool>,
    active: RwSignal<usize>,
    printings: RwSignal<Vec<CardPrinting>>,
    printings_loading: RwSignal<bool>,
    selected_print_idx: RwSignal<usize>,
    selected_finish: RwSignal<CardFinish>,
    print_explicit: RwSignal<bool>,
    pause: StoredValue<Option<TimeoutHandle>>,
    card_gen: StoredValue<u32>,
    print_gen: StoredValue<u32>,
}

fn warm_printing_images(printings: &[CardPrinting], enabled: bool, limit: usize) {
    if !enabled {
        return;
    }
    for p in printings.iter().filter(|p| !p.ub).take(limit) {
        if let Some(uuid) = p.scryfall_id.as_deref()
            && let Some(img_url) = mentions::scryfall_image_url(uuid)
        {
            cache::warm_card_image(&img_url);
        }
    }
}

fn best_finish(printing: Option<&CardPrinting>, preferred: CardFinish) -> CardFinish {
    let Some(p) = printing else {
        return preferred;
    };
    if p.finishes == 0 || (p.finishes & preferred.bit()) != 0 {
        return preferred;
    }
    if (p.finishes & FINISH_NONFOIL) != 0 {
        return CardFinish::Normal;
    }
    if (p.finishes & FINISH_FOIL) != 0 {
        return CardFinish::Foil;
    }
    if (p.finishes & FINISH_ETCHED) != 0 {
        return CardFinish::Etched;
    }
    preferred
}

impl ComposerMentions {
    fn new(
        lang: Lang,
        text: RwSignal<String>,
        people: Memo<Vec<Person>>,
        preview: PreviewCtl,
    ) -> Self {
        let pause = StoredValue::new(None::<TimeoutHandle>);
        on_cleanup(move || {
            if let Some(Some(handle)) = pause.try_get_value() {
                handle.clear();
            }
        });
        Self {
            lang,
            text,
            people,
            preview,
            trigger: RwSignal::new(None),
            parsed_query: RwSignal::new(None),
            cards: RwSignal::new(Vec::new()),
            cards_loading: RwSignal::new(false),
            active: RwSignal::new(0),
            printings: RwSignal::new(Vec::new()),
            printings_loading: RwSignal::new(false),
            selected_print_idx: RwSignal::new(0),
            selected_finish: RwSignal::new(CardFinish::Normal),
            print_explicit: RwSignal::new(false),
            pause,
            card_gen: StoredValue::new(0),
            print_gen: StoredValue::new(0),
        }
    }

    fn clear_pause(self) {
        if let Some(Some(handle)) = self.pause.try_get_value() {
            handle.clear();
        }
        let _ = self.pause.try_set_value(None);
    }

    fn dismiss(self) {
        self.clear_pause();
        self.trigger.set(None);
        self.parsed_query.set(None);
        self.cards.set(Vec::new());
        self.cards_loading.set(false);
        self.printings.set(Vec::new());
        self.printings_loading.set(false);
        self.selected_print_idx.set(0);
        self.selected_finish.set(CardFinish::Normal);
        self.print_explicit.set(false);
        self.active.set(0);
    }

    fn load_active_printings(self) {
        let items = self.cards.get_untracked();
        let idx = self.active.get_untracked();
        let Some(item) = items.get(idx).cloned() else {
            self.printings.set(Vec::new());
            self.printings_loading.set(false);
            self.selected_print_idx.set(0);
            return;
        };
        let previews_on = self.preview.previews_enabled.get_untracked();
        let pq = self.parsed_query.get_untracked();
        let seed_printings: Vec<CardPrinting> = item.preview.iter().cloned().collect();
        warm_printing_images(&seed_printings, previews_on, 1);
        let pref = pq
            .as_ref()
            .and_then(|q| q.preferred_finish)
            .unwrap_or_else(|| self.selected_finish.get_untracked());
        self.selected_finish
            .set(best_finish(seed_printings.first(), pref));
        self.selected_print_idx.set(0);
        self.printings.set(seed_printings);

        let next_pg = self.print_gen.get_value().wrapping_add(1);
        self.print_gen.set_value(next_pg);

        let lookup_name = item.name.clone();
        let lookup_lang = if item.matched_lang.trim().is_empty() {
            self.lang.code().to_owned()
        } else {
            item.matched_lang.clone()
        };
        let set_f = pq.as_ref().and_then(|q| q.set_filter.clone());
        let col_f = pq.as_ref().and_then(|q| q.collector_filter.clone());
        let fin_f = pq.as_ref().and_then(|q| q.finish_filter.clone());
        let cache_key = cache::card_cache_key(
            &lookup_name,
            &lookup_lang,
            set_f.as_deref(),
            col_f.as_deref(),
            fin_f.as_deref(),
        );

        if let Some((_, cached)) = cache::get_card_printings(&cache_key)
            && !cached.is_empty()
        {
            self.printings_loading.set(false);
            warm_printing_images(&cached, previews_on, 4);
            self.selected_finish.set(best_finish(
                cached.first(),
                self.selected_finish.get_untracked(),
            ));
            self.selected_print_idx.set(0);
            self.printings.set(cached);
            return;
        }

        self.printings_loading.set(true);
        leptos::task::spawn_local(async move {
            let mut res = wire::card_printings_lang(
                &lookup_name,
                &lookup_lang,
                set_f.as_deref(),
                col_f.as_deref(),
                fin_f.as_deref(),
            )
            .await;
            if res.as_ref().is_ok_and(|(_, list)| list.is_empty())
                && (set_f.is_some() || col_f.is_some() || fin_f.is_some())
            {
                res = wire::card_printings_lang(&lookup_name, &lookup_lang, None, None, None).await;
            }
            let list = match res {
                Ok((card_opt, list)) => {
                    if !list.is_empty() {
                        cache::put_card_printings(cache_key, card_opt, list.clone());
                    }
                    list
                }
                Err(_) => Vec::new(),
            };
            if self.print_gen.try_get_value() != Some(next_pg) {
                return;
            }
            self.printings_loading.set(false);
            if !list.is_empty() {
                warm_printing_images(&list, previews_on, 4);
                self.selected_finish.set(best_finish(
                    list.first(),
                    self.selected_finish.get_untracked(),
                ));
                self.selected_print_idx.set(0);
                self.printings.set(list);
            }
        });
    }

    fn select_index(self, idx: usize) {
        self.active.set(idx);
        if self
            .trigger
            .with_untracked(|t| t.as_ref().is_some_and(|t| t.kind == TriggerKind::Card))
        {
            self.load_active_printings();
        }
    }

    fn refresh_trigger(self, current: &str) {
        let caret = composer_caret_bytes(current);
        let found = mentions::active_trigger(current, caret);
        self.trigger.set(found.clone());
        self.active.set(0);
        self.clear_pause();
        let Some(Trigger {
            kind: TriggerKind::Card,
            query,
            ..
        }) = found
        else {
            self.print_gen
                .set_value(self.print_gen.get_value().wrapping_add(1));
            self.parsed_query.set(None);
            self.cards.set(Vec::new());
            self.printings.set(Vec::new());
            self.cards_loading.set(false);
            self.printings_loading.set(false);
            return;
        };
        let parsed = mentions::parse_card_trigger(&query);
        self.parsed_query.set(Some(parsed.clone()));
        self.selected_finish
            .set(parsed.preferred_finish.unwrap_or(CardFinish::Normal));
        self.print_explicit
            .set(parsed.set_filter.is_some() || parsed.collector_filter.is_some());
        if parsed.name_query.trim().chars().count() < 2 {
            self.print_gen
                .set_value(self.print_gen.get_value().wrapping_add(1));
            self.cards.set(Vec::new());
            self.printings.set(Vec::new());
            self.cards_loading.set(false);
            self.printings_loading.set(false);
            return;
        }
        self.print_gen
            .set_value(self.print_gen.get_value().wrapping_add(1));
        self.printings.set(Vec::new());
        self.printings_loading.set(false);

        let next_gen = self.card_gen.get_value().wrapping_add(1);
        self.card_gen.set_value(next_gen);
        let lang = self.lang;
        let suggest_key = cache::card_cache_key(
            &parsed.name_query,
            lang.code(),
            parsed.set_filter.as_deref(),
            parsed.collector_filter.as_deref(),
            parsed.finish_filter.as_deref(),
        );
        if let Some(cached) = cache::get_card_suggestions(&suggest_key) {
            self.cards.set(cached);
            self.cards_loading.set(false);
            self.active.set(0);
            self.load_active_printings();
            return;
        }

        self.cards_loading.set(true);
        let handle = set_timeout(
            move || {
                let _ = self.pause.try_set_value(None);
                leptos::task::spawn_local(async move {
                    let list = match wire::suggest_cards_rich(
                        &parsed.name_query,
                        lang,
                        parsed.set_filter.as_deref(),
                        parsed.collector_filter.as_deref(),
                        parsed.finish_filter.as_deref(),
                    )
                    .await
                    {
                        Ok(list) => {
                            cache::put_card_suggestions(suggest_key, &list);
                            list
                        }
                        Err(_) => Vec::new(),
                    };
                    if self.card_gen.try_get_value() == Some(next_gen) {
                        self.cards.set(list);
                        self.cards_loading.set(false);
                        self.active.set(0);
                        self.load_active_printings();
                    }
                });
            },
            Duration::from_millis(220),
        )
        .ok();
        self.pause.set_value(handle);
    }

    fn choices(self) -> Vec<MentionChoice> {
        let Some(trig) = self.trigger.get() else {
            return Vec::new();
        };
        match trig.kind {
            TriggerKind::Card => self
                .cards
                .get()
                .into_iter()
                .map(MentionChoice::Card)
                .collect(),
            TriggerKind::User => {
                let needle = trig.query.trim().to_lowercase();
                self.people.with(|list| {
                    list.iter()
                        .filter(|p| {
                            needle.is_empty()
                                || p.display_name.to_lowercase().contains(&needle)
                                || model::handle(p).to_lowercase().contains(&needle)
                        })
                        .take(8)
                        .cloned()
                        .map(MentionChoice::User)
                        .collect()
                })
            }
        }
    }

    fn choices_untracked(self) -> Vec<MentionChoice> {
        let Some(trig) = self.trigger.get_untracked() else {
            return Vec::new();
        };
        match trig.kind {
            TriggerKind::Card => self
                .cards
                .get_untracked()
                .into_iter()
                .map(MentionChoice::Card)
                .collect(),
            TriggerKind::User => {
                let needle = trig.query.trim().to_lowercase();
                self.people.with_untracked(|list| {
                    list.iter()
                        .filter(|p| {
                            needle.is_empty()
                                || p.display_name.to_lowercase().contains(&needle)
                                || model::handle(p).to_lowercase().contains(&needle)
                        })
                        .take(8)
                        .cloned()
                        .map(MentionChoice::User)
                        .collect()
                })
            }
        }
    }

    fn handle_key(self, event: &leptos::ev::KeyboardEvent) -> bool {
        if self.trigger.with_untracked(Option::is_none) {
            return false;
        }
        let items = self.choices_untracked();
        match event.key().as_str() {
            "ArrowDown" if !items.is_empty() => {
                event.prevent_default();
                let len = items.len();
                let next = (self.active.get_untracked() + 1) % len;
                self.select_index(next);
                true
            }
            "ArrowUp" if !items.is_empty() => {
                event.prevent_default();
                let len = items.len();
                let cur = self.active.get_untracked();
                let next = if cur == 0 { len - 1 } else { cur - 1 };
                self.select_index(next);
                true
            }
            "Enter" | "Tab" if !items.is_empty() && !event.shift_key() && !event.is_composing() => {
                event.prevent_default();
                let idx = self.active.get_untracked().min(items.len() - 1);
                self.apply(&items[idx]);
                true
            }
            "Escape" => {
                event.prevent_default();
                self.dismiss();
                true
            }
            _ => false,
        }
    }

    fn apply(self, choice: &MentionChoice) {
        let Some(trig) = self.trigger.get_untracked() else {
            return;
        };
        let replacement = match choice {
            MentionChoice::Card(card) => {
                let finish = self.selected_finish.get_untracked();
                let explicit = self.print_explicit.get_untracked();
                let active_card = self
                    .cards
                    .with_untracked(|list| list.get(self.active.get_untracked()).cloned());
                let printing = if explicit {
                    if active_card
                        .as_ref()
                        .is_some_and(|a| a.en_name == card.en_name)
                    {
                        self.printings
                            .with_untracked(|p| {
                                p.get(self.selected_print_idx.get_untracked()).cloned()
                            })
                            .or_else(|| card.preview.clone())
                    } else {
                        card.preview.clone()
                    }
                } else {
                    None
                };
                mentions::card_replacement(&card.name, &card.en_name, printing.as_ref(), finish)
            }
            MentionChoice::User(person) => mentions::user_replacement(person),
        };
        let (next, caret) =
            mentions::replace_trigger(&self.text.get_untracked(), &trig, &replacement);
        self.dismiss();
        self.text.set(next.clone());
        set_composer_caret(&next, caret);
    }

    fn card_preview_visual(
        self,
        card: &CardSuggestion,
        active_printing: Option<&CardPrinting>,
        active_finish: CardFinish,
    ) -> AnyView {
        let lang = self.lang;
        if !self.preview.previews_enabled.get() {
            return view! {
                <div class="card-preview-notice compact">
                    {glyph(glyph::EYE_OFF)}
                    <p>{words::chat_card_previews_disabled(lang)}</p>
                </div>
            }
            .into_any();
        }
        if let Some(p) = active_printing
            && !p.ub
            && let Some(uuid) = p.scryfall_id.as_deref()
            && let Some(img_url) = mentions::scryfall_image_url(uuid)
        {
            let finish_attr = active_finish.attr();
            let foil_w = "200";
            let foil_h = "278";
            return view! {
                <div class="card-preview-frame" data-finish=finish_attr>
                    <img
                        class="card-preview-img"
                        src=img_url
                        alt=card.name.clone()
                        referrerpolicy="no-referrer"
                        decoding="async"
                    />
                    {finish_attr.map(|f| view! {
                        <canvas
                            class="card-foil-canvas"
                            width=foil_w
                            height=foil_h
                            aria-hidden="true"
                            data-card-finish=f
                        ></canvas>
                    })}
                </div>
            }
            .into_any();
        }
        let msg = if active_printing.is_some_and(|p| p.ub) {
            words::chat_card_ub_notice(lang)
        } else {
            words::chat_card_credit(lang)
        };
        view! {
            <div class="card-preview-notice compact">
                {glyph(glyph::LAYERS)}
                <p>{msg}</p>
            </div>
        }
        .into_any()
    }

    fn card_preview_pane(self, card: &CardSuggestion) -> AnyView {
        let lang = self.lang;
        let all_prints = self.printings.get();
        let print_idx = self
            .selected_print_idx
            .get()
            .min(all_prints.len().saturating_sub(1));
        let active_printing = all_prints.get(print_idx).or(card.preview.as_ref());
        let active_finish = self.selected_finish.get();
        let visual = self.card_preview_visual(card, active_printing, active_finish);

        let finishes_mask = active_printing.map_or(7, |p| p.finishes);
        let finish_pills = [
            (
                CardFinish::Normal,
                FINISH_NONFOIL,
                words::chat_card_finish_normal(lang),
            ),
            (
                CardFinish::Foil,
                FINISH_FOIL,
                words::chat_card_finish_foil(lang),
            ),
            (
                CardFinish::Etched,
                FINISH_ETCHED,
                words::chat_card_finish_etched(lang),
            ),
        ]
        .into_iter()
        .map(|(f, bit, label)| {
            let active = active_finish == f;
            let supported = finishes_mask == 0 || (finishes_mask & bit) != 0;
            view! {
                <button
                    type="button"
                    class="pill-button"
                    data-active=active.then_some("true")
                    data-unsupported=(!supported).then_some("true")
                    on:mousedown=|ev: leptos::ev::MouseEvent| ev.prevent_default()
                    on:click=move |_| self.selected_finish.set(f)
                >
                    {label}
                </button>
            }
        })
        .collect_view();

        let print_select = (!all_prints.is_empty()).then(|| {
            let opts = all_prints
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    let label = format!(
                        "{} #{} ({}) · {}",
                        p.set_code.to_ascii_uppercase(),
                        p.collector_number,
                        p.lang.to_ascii_uppercase(),
                        p.set_name
                    );
                    view! {
                        <option value=i.to_string() selected=i == print_idx>
                            {label}
                        </option>
                    }
                })
                .collect_view();
            view! {
                <label class="preview-print-row">
                    <span class="visually-hidden">{words::chat_card_prints(lang)}</span>
                    <select
                        class="mention-print-select"
                        on:change=move |ev| {
                            if let Ok(i) = event_target_value(&ev).parse::<usize>() {
                                self.selected_print_idx.set(i);
                                self.print_explicit.set(true);
                                let finish_now = self.selected_finish.get_untracked();
                                let next_f = self.printings.with_untracked(|list| best_finish(list.get(i), finish_now));
                                self.selected_finish.set(next_f);
                            }
                        }
                    >
                        {opts}
                    </select>
                </label>
            }
        });

        let credit = words::chat_card_credit(lang);
        view! {
            <div class="mention-ac-preview">
                {visual}
                {print_select}
                <div class="card-finish-pills" role="group">{finish_pills}</div>
                <p class="preview-card-credit">{credit}</p>
            </div>
        }
        .into_any()
    }

    fn choice_row(self, idx: usize, active_idx: usize, item: MentionChoice) -> AnyView {
        let selected = idx == active_idx;
        let pick = item.clone();
        let label = match item {
            MentionChoice::Card(c) => {
                let show_en = !c.en_name.eq_ignore_ascii_case(&c.name);
                let set_tag = c.preview.as_ref().map(|p| p.set_code.to_ascii_uppercase());
                view! {
                    {glyph(glyph::LAYERS)}
                    <span class="ac-card-titles" translate="no">
                        <span>{c.name}</span>
                        {show_en.then(|| view! { <small class="ac-card-sub">{c.en_name}</small> })}
                    </span>
                    {set_tag.map(|s| view! { <span class="ac-card-set">{s}</span> })}
                }
                .into_any()
            }
            MentionChoice::User(p) => view! {
                {avatar(&p.display_name, &p.tag.to_string(), false, Some(p.tag))}
                <span translate="no">
                    <bdi>{p.display_name}</bdi>
                    <span class="tag">{"#"}{tag_text(p.tag)}</span>
                </span>
            }
            .into_any(),
        };
        view! {
            <li role="presentation">
                <button
                    type="button"
                    class="ac-item"
                    role="option"
                    aria-selected=selected.to_string()
                    on:mouseenter=move |_| {
                        if self.active.get_untracked() != idx {
                            self.select_index(idx);
                        }
                    }
                    on:mousedown=|ev: leptos::ev::MouseEvent| ev.prevent_default()
                    on:click=move |_| self.apply(&pick)
                >
                    {label}
                </button>
            </li>
        }
        .into_any()
    }

    fn popover(self) -> impl IntoView {
        let lang = self.lang;
        move || {
            let trig = self.trigger.get()?;
            let items = self.choices();
            let loading = trig.kind == TriggerKind::Card && self.cards_loading.get();
            if items.is_empty() && !loading {
                return None;
            }
            let is_cards = trig.kind == TriggerKind::Card;
            let active_idx = self.active.get().min(items.len().saturating_sub(1));
            let active_card = match items.get(active_idx) {
                Some(MentionChoice::Card(c)) => Some(c.clone()),
                _ => None,
            };
            let (head_icon, head_label) = match trig.kind {
                TriggerKind::Card => (glyph(glyph::HASH), words::chat_mention_cards(lang)),
                TriggerKind::User => (glyph(glyph::AT_SIGN), words::chat_mention_people(lang)),
            };
            let rows = items
                .into_iter()
                .enumerate()
                .map(|(idx, item)| self.choice_row(idx, active_idx, item))
                .collect_view();
            let status = loading.then(|| {
                view! { <p class="ac-empty" role="status">{words::chat_group_invite_searching(lang)}</p> }
            });
            let preview_pane = active_card.as_ref().map(|c| self.card_preview_pane(c));
            Some(view! {
                <div class=if is_cards { "mention-popover is-cards" } else { "mention-popover" }>
                    <div class="mention-ac-main">
                        <p class="ac-heading">{head_icon}<span>{head_label}</span></p>
                        <ul class="ac-list" role="listbox">{rows}</ul>
                        {status}
                    </div>
                    {preview_pane}
                </div>
            })
        }
    }
}

fn focus_current_row() {
    if let Some(row) = document()
        .query_selector(".conv-row[aria-current]")
        .ok()
        .flatten()
        && let Ok(row) = row.dyn_into::<web_sys::HtmlElement>()
    {
        let _ = row.focus();
    }
}

fn composer(
    lang: Lang,
    conv: String,
    members: Memo<Vec<u32>>,
    mention_people: Memo<Vec<Person>>,
    preview: PreviewCtl,
    ctl: Ctl,
) -> impl IntoView {
    let text = RwSignal::new(String::new());
    let mentions = ComposerMentions::new(lang, text, mention_people, preview);
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
        mentions.dismiss();
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
    let keys = move |event: leptos::ev::KeyboardEvent| {
        if mentions.handle_key(&event) {
            return;
        }
        match event.key().as_str() {
            "Enter" if !event.shift_key() && !event.is_composing() => {
                if event.ctrl_key() || event.meta_key() || !coarse() {
                    event.prevent_default();
                    send();
                }
            }
            "Escape" => focus_current_row(),
            _ => {}
        }
    };
    let count = move || {
        let n = text.with(|t| t.chars().count());
        (n >= 900)
            .then(|| words::chat_count(lang, &words::number(lang, u64::try_from(n).unwrap_or(0))))
    };
    view! {
        <div class="composer-wrap">
            {mentions.popover()}
            <form class="composer" on:submit=submit>
                <label class="visually-hidden" for="composer-text">{words::chat_write(lang)}</label>
                <div class="composer-field">
                    {field::mark(glyph::MESSAGE)}
                    <textarea id="composer-text" rows="1" maxlength="1000" placeholder=words::chat_write(lang)
                        aria-describedby="composer-hint" prop:value=move || text.get()
                        on:input=move |ev| {
                            let val = event_target_value(&ev);
                            text.set(val.clone());
                            mentions.refresh_trigger(&val);
                        }
                        on:click=move |_| {
                            let val = text.get_untracked();
                            mentions.refresh_trigger(&val);
                        }
                        on:keydown=keys></textarea>
                    <button type="submit" class="send composer-send" title=words::chat_send(lang)
                        aria-label=words::chat_send(lang)
                        aria-disabled=move || (blocked() || text.with(|t| t.trim().is_empty())).then_some("true")>
                        {glyph(glyph::SEND)}
                    </button>
                </div>
                <p id="composer-hint" class="composer-hint">
                    <span>{format!("{} · {}", words::chat_enter_hint(lang), words::chat_composer_mentions_hint(lang))}</span>
                    <span class="composer-count" aria-live="polite">{count}</span>
                </p>
            </form>
        </div>
    }
}
