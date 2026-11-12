//! `/{lang}/account/admin/users/<id in hex>`: one account as an
//! administrator sees it (`AdminRequest::Account`), with its changes as the
//! old build's page had them: another tag (unique at Central, so `Name#tag`
//! stays unique; the audit keeps both), the administrator role (neither for
//! a reserved account nor for one's own), and the way to a reset code,
//! which is the reset code's own page opened at this account. Below, the
//! gateways it registered and its latest twenty audit events.

use central_api::admin::{AccountState, AdminAnswer, AdminRequest};
use leptos::prelude::*;

use super::common::{self, Asked, Came, Change, Cx, Event, Flow, Names, Note};
use super::{Row, title};
use crate::data::id_hex;
use crate::islands::account::when;
use crate::islands::auth::{Problem, problem_text};
use crate::islands::chat::glyph;
use crate::islands::focus;
use crate::words::{self, Lang};

/// A gateway the account registered.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Owned {
    origin: String,
    approved: bool,
}

/// The account as Central answered it.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Detail {
    row: Row,
    recovery: bool,
    gateways: Vec<Owned>,
    audit: Vec<Event>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Load {
    Loading,
    Ready(Detail),
    Failed(&'static str),
}

/// A tag as an administrator types it: one to eight hex digits, a `#`
/// before them allowed, 1 to `MAX_TAG` (Central's own rule).
fn read_tag(text: &str) -> Option<u32> {
    let text = text.trim();
    let hex = text.strip_prefix('#').unwrap_or(text);
    if hex.is_empty() || hex.len() > 8 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let tag = u32::from_str_radix(hex, 16).ok()?;
    central_api::profile::valid_tag(tag).then_some(tag)
}

/// The account read from Central; `None` once the area was told that
/// nobody is signed in.
async fn fetch(cx: Cx, account: [u8; 16]) -> Option<Load> {
    let lang = cx.lang;
    match common::read(AdminRequest::Account { account }).await {
        Ok(AdminAnswer::Account {
            account,
            recovery,
            gateways,
            audit,
        }) => Some(Load::Ready(Detail {
            row: Row::from(account),
            recovery,
            gateways: gateways
                .into_iter()
                .map(|gateway| Owned {
                    origin: gateway.origin,
                    approved: gateway.approved,
                })
                .collect(),
            audit: audit.into_iter().map(Event::from).collect(),
        })),
        Ok(_) => Some(Load::Failed(problem_text(lang, Problem::Failed))),
        Err(unread) => cx
            .unread_text(unread, words::admin_refused_gone(lang))
            .map(Load::Failed),
    }
}

/// The page's signals and what it was given. All of it is `Copy`.
#[derive(Clone, Copy)]
struct Account {
    cx: Cx,
    id: [u8; 16],
    load: RwSignal<Load>,
    typed: RwSignal<String>,
    tag_error: RwSignal<Option<&'static str>>,
    flow: Flow,
}

impl Account {
    /// Reads the account (again).
    fn read(self) {
        leptos::task::spawn_local(async move {
            if let Some(load) = fetch(self.cx, self.id).await {
                let _ = self.load.try_set(load);
            }
        });
    }

    /// The tag's submit: a tag Central would refuse is said, and nothing
    /// is asked; a good one opens its confirmation.
    fn tag(self, event: &leptos::ev::SubmitEvent, name: &str) {
        event.prevent_default();
        let lang = self.cx.lang;
        let Some(tag) = read_tag(&self.typed.get_untracked()) else {
            self.tag_error.set(Some(words::admin_tag_error(lang)));
            focus::after_render("#admin-tag-field");
            return;
        };
        self.tag_error.set(None);
        self.flow.ask(Asked {
            change: Change::Tag {
                account: self.id,
                tag,
            },
            title: words::admin_tag_confirm(
                lang,
                name,
                &format!("#{}", central_api::profile::tag_text(tag)),
            ),
            text: words::admin_tag_warn(lang).to_owned(),
            action: words::admin_tag_set(lang),
            danger: false,
            from: "#admin-tag-field".to_owned(),
        });
    }

    /// The role's button: its confirmation.
    fn role(self, name: &str, admin: bool) {
        let lang = self.cx.lang;
        let (title, text, action) = if admin {
            (
                words::admin_role_grant_title(lang, name),
                words::admin_role_grant_text(lang),
                words::admin_role_grant(lang),
            )
        } else {
            (
                words::admin_role_remove_title(lang, name),
                words::admin_role_remove_text(lang),
                words::admin_role_remove(lang),
            )
        };
        self.flow.ask(Asked {
            change: Change::Role {
                account: self.id,
                admin,
            },
            title,
            text: text.to_owned(),
            action,
            danger: !admin,
            from: "#admin-role".to_owned(),
        });
    }
}

fn facts(lang: Lang, detail: &Detail) -> impl IntoView + use<> {
    let row = &detail.row;
    let hex = id_hex(&row.id);
    let state = match row.state {
        AccountState::Pending => words::admin_state_pending(lang),
        AccountState::Enrolled => words::admin_state_enrolled(lang),
        AccountState::Restored => words::admin_state_restored(lang),
    };
    let source = if row.moved {
        words::profile_source_moved(lang)
    } else {
        words::profile_source_registered(lang)
    };
    let last = row
        .last_sign_in_at
        .map_or_else(|| "–".to_owned(), |at| when(lang, at));
    let tag = row.tag.map_or_else(
        || "–".to_owned(),
        |tag| format!("#{}", central_api::profile::tag_text(tag)),
    );
    let recovery = if detail.recovery {
        words::admin_yes(lang)
    } else {
        words::admin_no(lang)
    };
    let role = if row.admin {
        words::admin_pill(lang)
    } else {
        words::admin_member(lang)
    };
    view! {
        <dl class="facts">
            <dt>{words::profile_signin_name(lang)}</dt>
            <dd class="admin-copyable">
                <code translate="no">{row.username.clone()}</code>
                {common::copy_button(lang, row.username.clone(), words::profile_signin_name(lang))}
            </dd>
            <dt>{words::admin_id(lang)}</dt>
            <dd class="admin-copyable">
                <code class="address" translate="no">{hex.clone()}</code>
                {common::copy_button(lang, hex, words::admin_id(lang))}
            </dd>
            <dt>{words::admin_tag(lang)}</dt>
            <dd><code translate="no">{tag}</code></dd>
            <dt>{words::profile_source(lang)}</dt>
            <dd>{source}</dd>
            <dt>{words::admin_state(lang)}</dt>
            <dd>{state}</dd>
            <dt>{words::admin_created(lang)}</dt>
            <dd>{when(lang, row.created_at)}</dd>
            <dt>{words::admin_last_sign_in(lang)}</dt>
            <dd>{last}</dd>
            <dt>{words::admin_recovery(lang)}</dt>
            <dd>{recovery}</dd>
            <dt>{words::admin_role(lang)}</dt>
            <dd>{role}</dd>
        </dl>
    }
}

fn tag_card(a: &Account, row: &Row) -> impl IntoView + use<> {
    let a = *a;
    let lang = a.cx.lang;
    let name = row.name();
    let shown = row.tag.map_or_else(
        || "#1a2b".to_owned(),
        |tag| format!("#{}", central_api::profile::tag_text(tag)),
    );
    view! {
        <section class="card" aria-labelledby="admin-tag-title">
            <h2 id="admin-tag-title">{words::admin_tag(lang)}</h2>
            <p>{words::admin_tag_lead(lang)}</p>
            <form class="form admin-search" method="post" novalidate=true on:submit=move |event| a.tag(&event, &name)>
                <div class="field">
                    <label for="admin-tag-field">{words::admin_tag(lang)}</label>
                    <div class="with-tool">
                        <input id="admin-tag-field" type="text" autocomplete="off" autocapitalize="none" spellcheck="false" maxlength="9"
                            placeholder=shown translate="no"
                            on:input=move |ev| a.typed.set(event_target_value(&ev)) prop:value=move || a.typed.get() />
                        <button type="submit" class="button">{words::admin_tag_set(lang)}</button>
                    </div>
                </div>
                <p class="error" role="alert">{move || a.tag_error.get()}</p>
            </form>
            {a.flow.panel(|change| matches!(change, Change::Tag { .. }))}
        </section>
    }
}

fn role_card(a: &Account, row: &Row) -> impl IntoView + use<> {
    let a = *a;
    let lang = a.cx.lang;
    let name = row.name();
    let admin = row.admin;
    let (class, label) = if admin {
        ("button danger", words::admin_role_remove(lang))
    } else {
        ("button", words::admin_role_grant(lang))
    };
    view! {
        <section class="card" aria-labelledby="admin-role-title">
            <h2 id="admin-role-title">{words::admin_role(lang)}</h2>
            <p class="actions">
                <button type="button" class=class id="admin-role" on:click=move |_| a.role(&name, !admin)>{label}</button>
            </p>
            {a.flow.panel(|change| matches!(change, Change::Role { .. }))}
        </section>
    }
}

fn detail_view(a: &Account, detail: &Detail) -> AnyView {
    let a = *a;
    let lang = a.cx.lang;
    let row = &detail.row;
    // nothing to change on a reserved account, nor one's own role
    let pending = row.state == AccountState::Pending;
    let acts = !pending && row.id != a.cx.own;
    let outstanding = row.reset_until.map(|until| {
        view! { <p class="meta">{words::admin_reset_outstanding(lang, &when(lang, until))}</p> }
    });
    // the reset code's own page, opened at this account
    let reset = (acts && !row.admin).then(|| {
        let href = format!("/{}/account/admin#account={}", lang.code(), id_hex(&row.id));
        view! {
            <p class="actions">
                <a class="button" href=href id="admin-reset-link">{glyph::glyph(glyph::KEY)}<span>{words::admin_reset(lang)}</span></a>
            </p>
        }
    });
    let gateways = (!detail.gateways.is_empty()).then(|| {
        let items = detail
            .gateways
            .iter()
            .map(|gateway| {
                let state = if gateway.approved {
                    words::gateways_approved(lang)
                } else {
                    words::gateways_pending(lang)
                };
                view! { <li><code class="address" translate="no">{gateway.origin.clone()}</code>" · "{state}</li> }
            })
            .collect_view();
        view! {
            <section class="card" aria-labelledby="admin-user-gateways">
                <h2 id="admin-user-gateways">{words::admin_user_gateways(lang)}</h2>
                <ul class="admin-owned">{items}</ul>
            </section>
        }
    });
    // the account's own name beside its events; others by their id's end
    let mut names = Names::default();
    names.accounts.insert(row.id, row.name());
    view! {
        <section class="card" aria-labelledby="admin-user-title">
            <h2 id="admin-user-title" class="admin-title" tabindex="-1">{title(lang, row)}</h2>
            {facts(lang, detail)}
            {outstanding}
            {reset}
        </section>
        {(!pending).then(|| tag_card(&a, row))}
        {acts.then(|| role_card(&a, row))}
        {gateways}
        <section class="card">
            <h2 id="admin-user-audit">{words::admin_user_audit(lang)}</h2>
            {common::audit_table(lang, "admin-user-audit", &detail.audit, &names)}
        </section>
    }
    .into_any()
}

/// The page for the account `id`.
pub fn page(cx: Cx, id: [u8; 16]) -> impl IntoView {
    let lang = cx.lang;
    let load = RwSignal::new(Load::Loading);
    let typed = RwSignal::new(String::new());
    let note = RwSignal::new(None::<Note>);
    // a change ended: the account is read again, which says a lost answer
    let finished = Callback::new(move |(change, came): (Change, Came)| {
        leptos::task::spawn_local(async move {
            let Some(fresh) = fetch(cx, id).await else {
                return;
            };
            let read = matches!(fresh, Load::Ready(_));
            if load.try_set(fresh).is_some() {
                return;
            }
            if came == Came::Done && matches!(change, Change::Tag { .. }) {
                typed.set(String::new());
            }
            if came == Came::Lost {
                note.set(Some(common::lost(lang, read)));
            }
            common::focus_note(note);
        });
    });
    let a = Account {
        cx,
        id,
        load,
        typed,
        tag_error: RwSignal::new(None),
        flow: Flow::new(cx, note, finished),
    };
    // Effects run in the browser only: the server renders "loading".
    Effect::new(move |_| a.read());
    view! {
        {a.flow.notes()}
        {move || match load.get() {
            Load::Loading => view! { <p role="status" class="meta">{words::account_loading(lang)}</p> }.into_any(),
            Load::Failed(text) => view! {
                <p class="error" role="alert">{text}</p>
                <p class="actions">
                    <button type="button" class="button" on:click=move |_| {
                        load.set(Load::Loading);
                        a.read();
                    }>{words::error_again(lang)}</button>
                </p>
            }
            .into_any(),
            Load::Ready(detail) => detail_view(&a, &detail),
        }}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tag is typed as Central takes it: one to eight hex digits, `#`
    /// allowed, from 1 to `MAX_TAG`.
    #[test]
    fn a_tag_is_read_as_central_takes_it() {
        assert_eq!(read_tag("#1a2b"), Some(0x1a2b));
        assert_eq!(read_tag(" 1A2B "), Some(0x1a2b));
        assert_eq!(read_tag("#1"), Some(1));
        assert_eq!(read_tag("#7fffffff"), Some(0x7fff_ffff));
        for bad in [
            "",
            "#",
            "#0",
            "#00000000",
            "#80000000",
            "#ffffffff",
            "#123456789",
            "#xyz",
            "##1",
            "1 2",
            "+1",
            "-1",
            "#１",
        ] {
            assert_eq!(read_tag(bad), None, "{bad}");
        }
    }
}
