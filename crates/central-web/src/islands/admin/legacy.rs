//! `/{lang}/account/admin/legacy`: what is left of the legacy store
//! (`docs/native-login.md` §"Legacy login", `AdminRequest::LegacyAccounts`):
//! the deadline after which the sweep deletes every row, and each account
//! of the official gateway that has not moved to Central yet, with how far
//! its move got and what holds it back (a ban, a password the gateway
//! wanted changed, a lockout). Names, times and counters only: Central
//! never answers a hash or a snapshot. Central refuses the read to an
//! administrator when it has no legacy store, and the page says so.

use central_api::admin::{AdminAnswer, AdminRequest};
use central_api::legacy::{LegacyAccount, LegacyProgress};
use leptos::prelude::*;

use super::common::{self, Cx};
use crate::islands::account::{day, when};
use crate::islands::auth::{Problem, problem_text};
use crate::words::{self, Lang};

/// One account left in the store, as the page shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Left {
    username: String,
    display_name: Option<String>,
    tag: Option<u32>,
    progress: LegacyProgress,
    imported_at: i64,
    failures: u32,
    locked_until: Option<i64>,
    must_change_password: bool,
    banned: bool,
}

impl From<LegacyAccount> for Left {
    fn from(account: LegacyAccount) -> Self {
        Left {
            username: account.username,
            display_name: account.display_name,
            tag: account.tag,
            progress: account.progress,
            imported_at: account.imported_at,
            failures: account.failures,
            locked_until: account.locked_until,
            must_change_password: account.must_change_password,
            banned: account.banned,
        }
    }
}

/// What holds an account's move back at `now`, in the store's own order: a
/// ban, a password the gateway wanted changed, a lockout (with its end).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Held {
    Banned,
    MustChange,
    Locked(i64),
}

fn held(left: &Left, now: i64) -> Vec<Held> {
    let mut out = Vec::new();
    if left.banned {
        out.push(Held::Banned);
    }
    if left.must_change_password {
        out.push(Held::MustChange);
    }
    if let Some(until) = left.locked_until
        && until > now
    {
        out.push(Held::Locked(until));
    }
    out
}

fn progress_text(lang: Lang, progress: LegacyProgress) -> (&'static str, &'static str) {
    match progress {
        LegacyProgress::Unstarted => (words::admin_legacy_unstarted(lang), "off"),
        LegacyProgress::Reserved => (words::admin_legacy_reserved(lang), "warn"),
        LegacyProgress::Enrolled => (words::admin_legacy_enrolled(lang), "warn"),
        LegacyProgress::Restored => (words::admin_legacy_restored(lang), "warn"),
    }
}

/// The store as read, and when (against which a lockout is shown).
#[derive(Clone, Debug, PartialEq, Eq)]
enum Load {
    Loading,
    Ready {
        deadline: Option<i64>,
        left: Vec<Left>,
        now: i64,
    },
    /// This Central keeps no legacy store.
    None,
    Failed(&'static str),
}

async fn fetch(cx: Cx) -> Option<Load> {
    let lang = cx.lang;
    match common::read(AdminRequest::LegacyAccounts).await {
        Ok(AdminAnswer::LegacyAccounts { store }) => Some(Load::Ready {
            deadline: store.deadline,
            left: store.accounts.into_iter().map(Left::from).collect(),
            now: common::now(),
        }),
        Ok(_) => Some(Load::Failed(problem_text(lang, Problem::Failed))),
        Err(common::Unread::Missing) => Some(Load::None),
        Err(unread) => cx
            .unread_text(unread, problem_text(lang, Problem::Failed))
            .map(Load::Failed),
    }
}

fn row(lang: Lang, left: &Left, now: i64) -> impl IntoView + use<> {
    let (progress, tone) = progress_text(lang, left.progress);
    let name = left.display_name.as_ref().map(|name| {
        let shown = left.tag.map_or_else(
            || name.clone(),
            |tag| central_api::profile::handle(name, tag),
        );
        view! { " " <bdi translate="no">{shown}</bdi> }
    });
    let holds = held(left, now);
    let holds = if holds.is_empty() {
        "–".into_any()
    } else {
        holds
            .into_iter()
            .map(|hold| {
                let text = match hold {
                    Held::Banned => words::admin_legacy_banned(lang).to_owned(),
                    Held::MustChange => words::admin_legacy_must_change(lang).to_owned(),
                    Held::Locked(until) => words::admin_legacy_locked(lang, &when(lang, until)),
                };
                view! { <span class="pill bad">{text}</span> }
            })
            .collect_view()
            .into_any()
    };
    view! {
        <tr>
            <th scope="row"><code translate="no">{left.username.clone()}</code>{name}</th>
            <td><span class=format!("pill {tone}")>{progress}</span></td>
            <td class="admin-holds">{holds}</td>
            <td class="num">{words::number(lang, u64::from(left.failures))}</td>
            <td class="num">{day(lang, left.imported_at)}</td>
        </tr>
    }
}

fn store(lang: Lang, deadline: Option<i64>, left: &[Left], now: i64) -> AnyView {
    let said = match deadline {
        None => words::admin_legacy_no_deadline(lang).to_owned(),
        Some(at) => words::admin_legacy_deadline(
            lang,
            &words::number(lang, left.len() as u64),
            &day(lang, at),
        ),
    };
    let table = if left.is_empty() {
        view! { <p>{words::admin_legacy_none(lang)}</p> }.into_any()
    } else {
        let rows = left.iter().map(|left| row(lang, left, now)).collect_view();
        view! {
            <div class="admin-table" role="region" aria-labelledby="admin-legacy-title" tabindex="0">
                <table>
                    <thead>
                        <tr>
                            <th scope="col">{words::profile_source(lang)}</th>
                            <th scope="col">{words::admin_legacy_move(lang)}</th>
                            <th scope="col">{words::admin_legacy_held(lang)}</th>
                            <th scope="col">{words::admin_legacy_failures(lang)}</th>
                            <th scope="col">{words::admin_legacy_imported(lang)}</th>
                        </tr>
                    </thead>
                    <tbody>{rows}</tbody>
                </table>
            </div>
        }
        .into_any()
    };
    view! {
        <p>{said}</p>
        {table}
    }
    .into_any()
}

/// The page: the deadline and what is left.
pub fn page(cx: Cx) -> impl IntoView {
    let lang = cx.lang;
    let load = RwSignal::new(Load::Loading);
    let read = move || {
        leptos::task::spawn_local(async move {
            if let Some(found) = fetch(cx).await {
                let _ = load.try_set(found);
            }
        });
    };
    // Effects run in the browser only: the server renders "loading".
    Effect::new(move |_| read());
    view! {
        <p class="admin-lead">{words::admin_legacy_lead(lang)}</p>
        <section class="card">
            <h2 id="admin-legacy-title">{words::admin_ov_legacy(lang)}</h2>
            {move || match load.get() {
                Load::Loading => view! { <p role="status" class="meta">{words::account_loading(lang)}</p> }.into_any(),
                Load::None => view! { <p>{words::admin_legacy_no_store(lang)}</p> }.into_any(),
                Load::Failed(text) => view! {
                    <p class="error" role="alert">{text}</p>
                    <p class="actions">
                        <button type="button" class="button" on:click=move |_| {
                            load.set(Load::Loading);
                            read();
                        }>{words::error_again(lang)}</button>
                    </p>
                }
                .into_any(),
                Load::Ready { deadline, left, now } => store(lang, deadline, &left, now),
            }}
        </section>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn left() -> Left {
        Left {
            username: "oldtimer".into(),
            display_name: None,
            tag: None,
            progress: LegacyProgress::Unstarted,
            imported_at: 1_790_000_000,
            failures: 0,
            locked_until: None,
            must_change_password: false,
            banned: false,
        }
    }

    /// What holds a move back, in the store's order; a lockout only while
    /// it lasts.
    #[test]
    fn what_holds_a_move_back() {
        let now = 1_800_000_000;
        assert_eq!(held(&left(), now), vec![]);
        let all = Left {
            banned: true,
            must_change_password: true,
            locked_until: Some(now + 60),
            ..left()
        };
        assert_eq!(
            held(&all, now),
            vec![Held::Banned, Held::MustChange, Held::Locked(now + 60)]
        );
        let over = Left {
            locked_until: Some(now),
            ..left()
        };
        assert_eq!(held(&over, now), vec![]);
    }
}
