//! `/{lang}/account/admin/overview`: the administration's first view, as
//! the old build had it. A few calm tiles with the counts Central reads in
//! one statement (`AdminRequest::Overview`), each leading to the page where
//! the work is; the ones that ask for attention (gateways awaiting
//! approval, new crashes) say so in words and in the state's colour. The
//! open reports (`AdminRequest::ReportFacets`) and what is left of the
//! legacy store (`AdminRequest::LegacyAccounts`) come from their own reads,
//! and a tile whose read gives nothing is left out.

use central_api::admin::{AdminAnswer, AdminOverview, AdminRequest};
use central_api::reports::{ReportFacets, ReportFilter, ReportKind, ReportStatus};
use leptos::prelude::*;

use super::common::{self, Cx};
use crate::data::Section;
use crate::islands::account::day;
use crate::islands::auth::{Problem, problem_text};
use crate::words::{self, Lang};

/// The reports at a glance, from the facets under "new": the statuses are
/// counted without that filter (so the open ones: new, triaged, in
/// progress), the kinds under it (so the new crashes).
fn glance(facets: &ReportFacets) -> (u64, u64) {
    let open = facets
        .statuses
        .iter()
        .filter(|count| !count.value.done())
        .map(|count| count.count)
        .sum();
    let crashes = facets
        .kinds
        .iter()
        .filter(|count| count.value == ReportKind::Crash)
        .map(|count| count.count)
        .sum();
    (open, crashes)
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Counts {
    Loading,
    Ready(AdminOverview),
    Failed(&'static str),
}

/// One tile: what it counts, the number, a line, and the way to the work.
struct Tile {
    label: &'static str,
    n: u64,
    warn: bool,
    sub: String,
    go: &'static str,
    href: String,
}

fn tile(lang: Lang, tile: Tile) -> impl IntoView + use<> {
    let Tile {
        label,
        n,
        warn,
        sub,
        go,
        href,
    } = tile;
    view! {
        <li class="card admin-stat" class:admin-stat-warn=warn>
            <h2 class="admin-stat-label">{label}</h2>
            <p class="admin-stat-n">{words::number(lang, n)}</p>
            <p class="admin-stat-sub">{sub}</p>
            <p><a class="admin-stat-go" href=href>{go}</a></p>
        </li>
    }
}

/// The page: the tiles.
pub fn page(cx: Cx) -> impl IntoView {
    let lang = cx.lang;
    let counts = RwSignal::new(Counts::Loading);
    // `None` until read, and where nothing can be read
    let legacy = RwSignal::new(None::<(usize, Option<i64>)>);
    let reports = RwSignal::new(None::<(u64, u64)>);
    let count = move || {
        leptos::task::spawn_local(async move {
            let found = match common::read(AdminRequest::Overview).await {
                Ok(AdminAnswer::Overview { overview }) => Counts::Ready(overview),
                Ok(_) => Counts::Failed(words::admin_ov_failed(lang)),
                Err(common::Unread::Other(_) | common::Unread::Missing) => {
                    Counts::Failed(words::admin_ov_failed(lang))
                }
                Err(unread) => match cx.unread_text(unread, problem_text(lang, Problem::Failed)) {
                    Some(text) => Counts::Failed(text),
                    None => return,
                },
            };
            let _ = counts.try_set(found);
        });
    };
    // Effects run in the browser only: the server renders "loading".
    Effect::new(move |_| {
        count();
        leptos::task::spawn_local(async move {
            if let Ok(AdminAnswer::LegacyAccounts { store }) =
                common::read(AdminRequest::LegacyAccounts).await
            {
                let _ = legacy.try_set(Some((store.accounts.len(), store.deadline)));
            }
        });
        leptos::task::spawn_local(async move {
            let filter = ReportFilter {
                statuses: vec![ReportStatus::New],
                ..ReportFilter::default()
            };
            if let Ok(AdminAnswer::ReportFacets { facets }) =
                common::read(AdminRequest::ReportFacets { filter }).await
            {
                let _ = reports.try_set(Some(glance(&facets)));
            }
        });
    });
    view! {
        <p class="admin-lead">{words::admin_ov_lead(lang)}</p>
        {move || match counts.get() {
            Counts::Loading => view! { <p role="status" class="meta">{words::account_loading(lang)}</p> }.into_any(),
            Counts::Failed(text) => view! {
                <p class="error" role="alert">{text}</p>
                <p class="actions">
                    <button type="button" class="button" on:click=move |_| {
                        counts.set(Counts::Loading);
                        count();
                    }>{words::error_again(lang)}</button>
                </p>
            }
            .into_any(),
            Counts::Ready(overview) => tiles(lang, &overview, legacy, reports).into_any(),
        }}
    }
}

/// The tiles for the counts `overview`, with those of the reports and the
/// legacy store where they were read.
fn tiles(
    lang: Lang,
    overview: &AdminOverview,
    legacy: RwSignal<Option<(usize, Option<i64>)>>,
    reports: RwSignal<Option<(u64, u64)>>,
) -> impl IntoView + use<> {
    let code = lang.code();
    let path = move |section: Section| format!("/{code}/{}", section.slug());
    {
        let accounts = tile(
            lang,
            Tile {
                label: words::admin_accounts(lang),
                n: overview.accounts,
                warn: false,
                sub: words::admin_ov_accounts(
                    lang,
                    &words::number(lang, overview.accounts_week),
                    &words::number(lang, overview.moved),
                ),
                go: words::admin_ov_accounts_go(lang),
                href: path(Section::AdminUsers),
            },
        );
        let pending = tile(
            lang,
            Tile {
                label: words::gateways_pending(lang),
                n: overview.pending,
                warn: overview.pending > 0,
                sub: if overview.pending > 0 {
                    words::admin_ov_pending(lang)
                } else {
                    words::admin_ov_pending_none(lang)
                }
                .to_owned(),
                go: words::admin_ov_pending_go(lang),
                href: format!("{}#state=pending", path(Section::AdminGateways)),
            },
        );
        let gateways = tile(
            lang,
            Tile {
                label: words::admin_ov_gateways(lang),
                n: overview.approved,
                warn: false,
                sub: words::admin_ov_gateways_sub(
                    lang,
                    &words::number(lang, overview.online),
                    &words::number(lang, overview.disabled),
                    &words::number(lang, overview.live_tokens),
                ),
                go: words::admin_ov_gateways_go(lang),
                href: format!("{}#state=approved", path(Section::AdminGateways)),
            },
        );
        let admins = tile(
            lang,
            Tile {
                label: words::admin_ov_admins(lang),
                n: overview.admins,
                warn: false,
                sub: words::admin_ov_admins_sub(lang).to_owned(),
                go: words::admin_audit(lang),
                href: path(Section::AdminAudit),
            },
        );
        view! {
            <ul class="admin-stats">
                {accounts}
                {pending}
                {reports_tile(lang, reports)}
                {gateways}
                {admins}
                {legacy_tile(lang, legacy)}
            </ul>
        }
    }
}

/// The open reports' tile, once read; the feedback's page is another
/// island's, and its filters are its own.
fn reports_tile(lang: Lang, reports: RwSignal<Option<(u64, u64)>>) -> impl IntoView + use<> {
    let code = lang.code();
    move || {
        reports.get().map(|(open, crashes)| {
            tile(
                lang,
                Tile {
                    label: words::admin_ov_reports(lang),
                    n: open,
                    warn: crashes > 0,
                    sub: if crashes > 0 {
                        words::admin_ov_crashes(lang, &words::number(lang, crashes))
                    } else {
                        words::admin_ov_calm(lang).to_owned()
                    },
                    go: words::admin_ov_reports_go(lang),
                    href: format!("/{code}/account/admin/feedback"),
                },
            )
        })
    }
}

/// What is left of the legacy store, once read.
fn legacy_tile(
    lang: Lang,
    legacy: RwSignal<Option<(usize, Option<i64>)>>,
) -> impl IntoView + use<> {
    move || {
        legacy.get().map(|(left, deadline)| {
            tile(
                lang,
                Tile {
                    label: words::admin_ov_legacy(lang),
                    n: left as u64,
                    warn: false,
                    sub: deadline.map_or_else(
                        || words::admin_legacy_no_deadline(lang).to_owned(),
                        |at| words::admin_ov_legacy_sub(lang, &day(lang, at)),
                    ),
                    go: words::admin_legacy(lang),
                    href: format!("/{}/{}", lang.code(), Section::AdminLegacy.slug()),
                },
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use central_api::reports::Count;

    /// The open reports are the statuses not done; the new crashes are
    /// the crashes among the new.
    #[test]
    fn the_reports_at_a_glance() {
        let facets = ReportFacets {
            statuses: vec![
                Count {
                    value: ReportStatus::New,
                    count: 4,
                },
                Count {
                    value: ReportStatus::Triaged,
                    count: 2,
                },
                Count {
                    value: ReportStatus::InProgress,
                    count: 1,
                },
                Count {
                    value: ReportStatus::Resolved,
                    count: 9,
                },
                Count {
                    value: ReportStatus::WontFix,
                    count: 3,
                },
                Count {
                    value: ReportStatus::Duplicate,
                    count: 5,
                },
            ],
            kinds: vec![
                Count {
                    value: ReportKind::Bug,
                    count: 2,
                },
                Count {
                    value: ReportKind::Crash,
                    count: 2,
                },
            ],
            priorities: vec![],
            assignees: vec![],
            labels: vec![],
            gateways: vec![],
            channels: vec![],
            reporters: vec![],
            total: 4,
        };
        assert_eq!(glance(&facets), (7, 2));
    }
}
