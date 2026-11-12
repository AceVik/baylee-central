//! `/{lang}/account/admin/audit`: what administrators and the operator did,
//! newest first, 50 a page (`AdminRequest::Audit`). An event records ids,
//! never words (`docs/administration.md`); the read brings today's names of
//! the accounts and gateways a page names, which the table shows (the id's
//! end where a name is gone). An action this page does not know is shown
//! as Central names it.

use central_api::admin::{AdminAnswer, AdminRequest};
use leptos::prelude::*;

use super::Listing;
use super::common::{self, Cx, Event, Names};
use crate::data::id_hex;
use crate::islands::auth::{Problem, problem_text};
use crate::islands::focus;
use crate::words;

/// The page's signals. All of it is `Copy`.
#[derive(Clone, Copy)]
struct Audit {
    cx: Cx,
    events: RwSignal<Vec<Event>>,
    /// The names of every page read so far.
    names: RwSignal<Names>,
    listing: RwSignal<Listing>,
    more: RwSignal<bool>,
    adding: RwSignal<bool>,
}

impl Audit {
    /// Reads a page of events: the first, or the one after `before`.
    fn read(self, before: Option<[u8; 16]>) {
        let lang = self.cx.lang;
        leptos::task::spawn_local(async move {
            let answer = common::read(AdminRequest::Audit {
                account: None,
                before,
            })
            .await;
            match answer {
                Ok(AdminAnswer::Audit {
                    events,
                    more,
                    accounts,
                    gateways,
                }) => {
                    let found: Vec<Event> = events.into_iter().map(Event::from).collect();
                    let next = found.first().map(|event| id_hex(&event.id));
                    let _ = self.names.try_update(|names| {
                        for account in accounts {
                            if let Some(name) =
                                common::handle(account.display_name.as_deref(), account.tag)
                            {
                                names.accounts.insert(account.id, name);
                            }
                        }
                        for gateway in gateways {
                            names
                                .gateways
                                .insert(gateway.id, (gateway.name, gateway.origin));
                        }
                    });
                    if before.is_some() {
                        self.events.update(|events| events.extend(found));
                    } else {
                        self.events.set(found);
                    }
                    self.more.set(more);
                    self.listing.set(Listing::Ready);
                    // after "Show more" the keyboard goes on at the first new event
                    if before.is_some()
                        && let Some(key) = next
                    {
                        focus::after_render(format!("#admin-event-{key}"));
                    }
                }
                Ok(_) => self
                    .listing
                    .set(Listing::Failed(problem_text(lang, Problem::Failed))),
                Err(unread) => {
                    let missing = problem_text(lang, Problem::Failed);
                    if let Some(text) = self.cx.unread_text(unread, missing) {
                        self.listing.set(Listing::Failed(text));
                    }
                }
            }
            let _ = self.adding.try_set(false);
        });
    }

    /// "Show more", and trying again: the page after the last event shown.
    fn more(self) {
        if self.adding.get_untracked() {
            return;
        }
        let last = self
            .events
            .with_untracked(|events| events.last().map(|event| event.id));
        self.adding.set(true);
        self.read(last);
    }
}

/// The page: the log, newest first.
pub fn page(cx: Cx) -> impl IntoView {
    let lang = cx.lang;
    let a = Audit {
        cx,
        events: RwSignal::new(Vec::new()),
        names: RwSignal::new(Names::default()),
        listing: RwSignal::new(Listing::Loading),
        more: RwSignal::new(false),
        adding: RwSignal::new(false),
    };
    // Effects run in the browser only: the server renders "loading".
    Effect::new(move |_| a.read(None));
    let shown = move || {
        let events = a.events.get();
        let table = a
            .names
            .with(|names| common::audit_table(lang, "admin-events-title", &events, names));
        let failed = match a.listing.get() {
            Listing::Failed(text) => Some(view! {
                <p class="error" role="alert">{text}</p>
                <p class="actions">
                    <button type="button" class="button" on:click=move |_| {
                        a.listing.set(Listing::Loading);
                        a.more();
                    }>{words::error_again(lang)}</button>
                </p>
            }),
            _ => None,
        };
        let ready = a.listing.get() == Listing::Ready;
        let more = (ready && a.more.get()).then(|| {
            view! {
                <p class="actions">
                    <button type="button" class="button" aria-disabled=move || a.adding.get().then_some("true")
                        on:click=move |_| a.more()>{words::admin_more(lang)}</button>
                </p>
            }
        });
        match a.listing.get() {
            Listing::Loading if events.is_empty() => {
                view! { <p role="status" class="meta">{words::account_loading(lang)}</p> }
                    .into_any()
            }
            Listing::Failed(_) if events.is_empty() => failed.into_any(),
            _ => view! { {table}{failed}{more} }.into_any(),
        }
    };
    view! {
        <p class="admin-lead">{words::admin_audit_lead(lang)}</p>
        <section class="card">
            <h2 id="admin-events-title">{words::admin_audit_events(lang)}</h2>
            <div aria-busy=move || (a.listing.get() == Listing::Loading || a.adding.get()).then_some("true")>
                {shown}
            </div>
        </section>
    }
}
