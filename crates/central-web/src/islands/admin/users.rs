//! `/{lang}/account/admin/users`: the accounts, newest first, 50 a page, or
//! those whose name holds a search's words, or the one with a `#tag`
//! (`AdminRequest::Accounts`, `AccountQuery`). As the old build's page did,
//! the list follows the field once typing pauses (the newest answer
//! winning), Escape empties the field, and Ctrl+K or ⌘K reaches it from
//! anywhere on the page. Unlike it, the search stays out of the address
//! (decided for the reset code's page, `docs/security-assurance.md`). A
//! search Central would refuse is said and never sent.

use std::time::Duration;

use central_api::admin::{AdminAnswer, AdminRequest};
use leptos::prelude::*;

use super::common::{self, Cx};
use super::{Listing, Row, query};
use crate::islands::account::when;
use crate::islands::auth::{Problem, problem_text};
use crate::islands::chat::glyph;
use crate::islands::focus;
use crate::words;

/// How long typing pauses before the list follows the field.
const PAUSE: Duration = Duration::from_millis(300);

/// The page's signals. All of it is `Copy`.
#[derive(Clone, Copy)]
struct Users {
    cx: Cx,
    rows: RwSignal<Vec<Row>>,
    more: RwSignal<bool>,
    listing: RwSignal<Listing>,
    adding: RwSignal<bool>,
    /// The search the list shows.
    asked: StoredValue<Option<String>>,
    /// The newest list request: an older one's answer is not drawn.
    generation: StoredValue<u32>,
    typed: RwSignal<String>,
    search_error: RwSignal<Option<&'static str>>,
    /// The pause after typing, until the list follows.
    pause: StoredValue<Option<TimeoutHandle>>,
}

impl Users {
    /// Reads a page of accounts: the first of the search `asked`, or the
    /// one after `before`.
    fn read(self, before: Option<[u8; 16]>) {
        let Some(generation) = self.generation.try_get_value() else {
            return;
        };
        let generation = if before.is_none() {
            generation.wrapping_add(1)
        } else {
            generation
        };
        self.generation.set_value(generation);
        let query = self.asked.get_value();
        let lang = self.cx.lang;
        leptos::task::spawn_local(async move {
            let answer = common::read(AdminRequest::Accounts { query, before }).await;
            // a newer search, or a page that is gone, draws its own
            if self.generation.try_get_value() != Some(generation) {
                return;
            }
            match answer {
                Ok(AdminAnswer::Accounts { accounts, more }) => {
                    let found: Vec<Row> = accounts.into_iter().map(Row::from).collect();
                    let next = found.first().map(Row::key);
                    if before.is_some() {
                        self.rows.update(|rows| rows.extend(found));
                    } else {
                        self.rows.set(found);
                    }
                    self.more.set(more);
                    self.listing.set(Listing::Ready);
                    // after "Show more" the keyboard goes on at the first new account
                    if before.is_some()
                        && let Some(key) = next
                    {
                        focus::after_render(format!("#admin-user-{key}"));
                    }
                }
                Ok(_) => self
                    .listing
                    .set(Listing::Failed(problem_text(lang, Problem::Failed))),
                Err(unread) => {
                    // the search was checked: a refusal is the role or the session
                    let missing = problem_text(lang, Problem::Failed);
                    if let Some(text) = self.cx.unread_text(unread, missing) {
                        self.listing.set(Listing::Failed(text));
                    }
                }
            }
            self.adding.set(false);
        });
    }

    /// Lists what `asked` finds, from its first page.
    fn ask(self, asked: Option<String>) {
        self.asked.set_value(asked);
        self.rows.set(Vec::new());
        self.more.set(false);
        self.listing.set(Listing::Loading);
        self.read(None);
    }

    /// The field as typed now, as Central would read it: a search it would
    /// refuse is said, and not sent.
    fn checked(self) -> Result<Option<String>, ()> {
        let checked = query(&self.typed.get_untracked());
        self.search_error.set(
            checked
                .is_err()
                .then(|| words::admin_search_error(self.cx.lang)),
        );
        checked
    }

    /// Typing paused: the list follows the field, unless it shows that
    /// search already.
    fn follow(self) {
        let _ = self.pause.try_set_value(None);
        if let Ok(asked) = self.checked()
            && (asked != self.asked.get_value()
                || matches!(self.listing.get_untracked(), Listing::Failed(_)))
        {
            self.ask(asked);
        }
    }

    /// A key typed into the field: the pause starts again.
    fn typed(self, value: String) {
        self.typed.set(value);
        if let Some(Some(pause)) = self.pause.try_get_value() {
            pause.clear();
        }
        let handle = set_timeout(move || self.follow(), PAUSE).ok();
        self.pause.set_value(handle);
    }

    /// Escape in the field empties it, and the list shows every account.
    fn escape(self, event: &leptos::ev::KeyboardEvent) {
        if event.key() != "Escape" || self.typed.get_untracked().is_empty() {
            return;
        }
        event.prevent_default();
        if let Some(Some(pause)) = self.pause.try_get_value() {
            pause.clear();
        }
        self.typed.set(String::new());
        self.follow();
    }

    /// The search's submit: at once, whatever the pause.
    fn search(self, event: &leptos::ev::SubmitEvent) {
        event.prevent_default();
        if let Some(Some(pause)) = self.pause.try_get_value() {
            pause.clear();
        }
        let _ = self.pause.try_set_value(None);
        match self.checked() {
            Ok(asked) => self.ask(asked),
            Err(()) => focus::after_render("#admin-users-search"),
        }
    }

    /// "Show more": the page after the last account shown.
    fn more(self) {
        if self.adding.get_untracked() {
            return;
        }
        let Some(last) = self
            .rows
            .with_untracked(|rows| rows.last().map(|row| row.id))
        else {
            return;
        };
        self.adding.set(true);
        self.read(Some(last));
    }
}

fn row_view(cx: Cx, row: &Row) -> impl IntoView + use<> {
    let lang = cx.lang;
    let pill = row
        .admin
        .then(|| view! { " " <span class="pill info">{words::admin_pill(lang)}</span> });
    let source = if row.moved {
        words::profile_source_moved(lang)
    } else {
        words::profile_source_registered(lang)
    };
    let last = row
        .last_sign_in_at
        .map_or_else(|| "–".to_owned(), |at| when(lang, at));
    view! {
        <tr>
            <th scope="row">
                <a id=format!("admin-user-{}", row.key()) href=common::user_path(lang, &row.id)><bdi translate="no">{row.name()}</bdi></a>
                {pill}
            </th>
            <td><code translate="no">{row.username.clone()}</code></td>
            <td>{source}</td>
            <td class="num">{when(lang, row.created_at)}</td>
            <td class="num">{last}</td>
        </tr>
    }
}

fn results(u: Users) -> impl IntoView {
    let lang = u.cx.lang;
    move || {
        let rows = u.rows.get();
        match u.listing.get() {
            Listing::Loading => {
                view! { <p role="status" class="meta">{words::account_loading(lang)}</p> }
                    .into_any()
            }
            Listing::Failed(text) => view! {
                <p class="error" role="alert">{text}</p>
                <p class="actions">
                    <button type="button" class="button" on:click=move |_| {
                        u.listing.set(Listing::Loading);
                        u.read(None);
                    }>{words::error_again(lang)}</button>
                </p>
            }
            .into_any(),
            Listing::Ready if rows.is_empty() => {
                view! { <p role="status">{words::admin_none(lang)}</p> }.into_any()
            }
            Listing::Ready => {
                let count = rows.len().to_string();
                let body = rows.iter().map(|row| row_view(u.cx, row)).collect_view();
                let more = u.more.get().then(|| {
                    view! {
                        <p class="actions">
                            <button type="button" class="button" aria-disabled=move || u.adding.get().then_some("true")
                                on:click=move |_| u.more()>{words::admin_more(lang)}</button>
                        </p>
                    }
                });
                view! {
                    <p role="status" class="meta">{words::admin_count(lang, &count)}</p>
                    <div class="admin-table" role="region" aria-label=words::admin_accounts(lang) tabindex="0">
                        <table>
                            <thead>
                                <tr>
                                    <th scope="col">{words::profile_handle(lang)}</th>
                                    <th scope="col">{words::profile_signin_name(lang)}</th>
                                    <th scope="col">{words::profile_source(lang)}</th>
                                    <th scope="col">{words::admin_created(lang)}</th>
                                    <th scope="col">{words::admin_last_sign_in(lang)}</th>
                                </tr>
                            </thead>
                            <tbody>{body}</tbody>
                        </table>
                    </div>
                    {more}
                }
                .into_any()
            }
        }
    }
}

/// The page: the search and the accounts it found.
pub fn page(cx: Cx) -> impl IntoView {
    let lang = cx.lang;
    let u = Users {
        cx,
        rows: RwSignal::new(Vec::new()),
        more: RwSignal::new(false),
        listing: RwSignal::new(Listing::Loading),
        adding: RwSignal::new(false),
        asked: StoredValue::new(None),
        generation: StoredValue::new(0),
        typed: RwSignal::new(String::new()),
        search_error: RwSignal::new(None),
        pause: StoredValue::new(None),
    };
    // Effects run in the browser only: the newest accounts first.
    Effect::new(move |_| u.read(None));
    // Ctrl+K or ⌘K from anywhere on the page reaches the search
    #[cfg(target_arch = "wasm32")]
    {
        let keys = window_event_listener(leptos::ev::keydown, move |event| {
            if (event.ctrl_key() || event.meta_key()) && event.key().eq_ignore_ascii_case("k") {
                event.prevent_default();
                focus::after_render("#admin-users-search");
            }
        });
        on_cleanup(move || keys.remove());
    }
    on_cleanup(move || {
        if let Some(Some(pause)) = u.pause.try_get_value() {
            pause.clear();
        }
    });
    view! {
        <div class="card">
            <form class="form admin-search" role="search" method="post" novalidate=true on:submit=move |event| u.search(&event)>
                <div class="field">
                    <label for="admin-users-search">{words::admin_search(lang)}</label>
                    <div class="with-tool">
                        <input id="admin-users-search" type="search" autocomplete="off" autocapitalize="none" spellcheck="false" maxlength="64"
                            aria-describedby="admin-users-search-hint" aria-keyshortcuts="Control+K Meta+K"
                            on:input=move |ev| u.typed(event_target_value(&ev)) on:keydown=move |ev| u.escape(&ev)
                            prop:value=move || u.typed.get() />
                        <button type="submit" class="button">{glyph::glyph(glyph::SEARCH)}<span>{words::admin_search_submit(lang)}</span></button>
                    </div>
                    <p class="hint" id="admin-users-search-hint">{words::admin_search_hint(lang)}</p>
                </div>
                <p class="error" role="alert">{move || u.search_error.get()}</p>
            </form>
        </div>
        <div class="admin-results" aria-busy=move || (u.listing.get() == Listing::Loading || u.adding.get()).then_some("true")>
            {results(u)}
        </div>
    }
}
