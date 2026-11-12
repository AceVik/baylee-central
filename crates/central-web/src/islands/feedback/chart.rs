//! The last 30 days of reports (`AdminRequest::ReportStats`): a column a
//! day, the crashes stacked on the other reports, a legend in words with
//! the totals, each day's numbers in its `<title>` and in a table. Plain
//! inline SVG sized by its own attributes, no script library; read only
//! when it is unfolded, so the list never waits for it.

use leptos::prelude::*;

use super::super::central::Failure;
use super::api;
use super::text::{Day, last_days};
use crate::format;
use crate::words::{self, Lang};

/// A column's width and its bar's, and the plot's height, in the
/// viewBox's units.
const W: u64 = 10;
const BAR: u64 = 7;
const H: u64 = 100;

#[derive(Clone, Debug)]
enum Load {
    Idle,
    Loading,
    Failed(&'static str),
    Ready(Vec<Day>),
}

fn columns(lang: Lang, days: &[Day]) -> impl IntoView + use<> {
    let max = days
        .iter()
        .map(|d| d.crashes + d.others)
        .max()
        .unwrap_or(0)
        .max(1);
    // in hundredths of the viewBox's units, so nothing needs a float
    let scale = |n: u64| n * (H - 2) * 100 / max;
    let at = |hundredths: u64| format!("{}.{:02}", hundredths / 100, hundredths % 100);
    let n = u64::try_from(days.len()).unwrap_or(0);
    let bars = days
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let i = u64::try_from(i).unwrap_or(0);
            let others = scale(d.others);
            let crashes = scale(d.crashes);
            // the bar sits in the middle of its column
            let x = at(i * W * 100 + (W - BAR) * 50);
            let title = words::fb_chart_day(
                lang,
                &format::date(lang, &d.date),
                &words::number(lang, d.others + d.crashes),
                &words::number(lang, d.crashes),
            );
            let other_bar = (d.others > 0).then(|| {
                view! {
                    <rect class="fb-chart-reports" x=x.clone() y=at(H * 100 - others)
                        width=BAR.to_string() height=at(others) />
                }
            });
            // a gap of the surface between the two, so they part without colour too
            let gap = if d.others > 0 { 100 } else { 0 };
            let crash_bar = (d.crashes > 0).then(|| {
                view! {
                    <rect class="fb-chart-crashes" x=x.clone() y=at(H * 100 - others - crashes)
                        width=BAR.to_string() height=at(crashes.saturating_sub(gap).max(50)) />
                }
            });
            view! {
                <g class="fb-chart-day">
                    <title>{title}</title>
                    <rect class="fb-chart-hit" x=(i * W).to_string() y="0" width=W.to_string() height=H.to_string() />
                    {other_bar}
                    {crash_bar}
                </g>
            }
        })
        .collect_view();
    let first = days
        .first()
        .map(|d| format::date(lang, &d.date))
        .unwrap_or_default();
    let last = days
        .last()
        .map(|d| format::date(lang, &d.date))
        .unwrap_or_default();
    view! {
        <div class="fb-chart-plot">
            <span class="fb-chart-max meta" aria-hidden="true">{words::number(lang, max)}</span>
            <svg class="fb-chart-svg" viewBox=format!("0 0 {} {H}", n * W) preserveAspectRatio="none"
                aria-hidden="true" focusable="false">
                <line class="fb-chart-base" x1="0" x2=(n * W).to_string() y1="99.5" y2="99.5" />
                {bars}
            </svg>
            <span class="fb-chart-axis meta" aria-hidden="true"><span>{first}</span><span>{last}</span></span>
        </div>
    }
}

fn figure(lang: Lang, days: &[Day]) -> impl IntoView + use<> {
    let others: u64 = days.iter().map(|d| d.others).sum();
    let crashes: u64 = days.iter().map(|d| d.crashes).sum();
    let rows = days
        .iter()
        .filter(|d| d.crashes + d.others > 0)
        .map(|d| {
            view! {
                <tr>
                    <th scope="row">{format::date(lang, &d.date)}</th>
                    <td class="num">{words::number(lang, d.others)}</td>
                    <td class="num">{words::number(lang, d.crashes)}</td>
                </tr>
            }
        })
        .collect_view();
    view! {
        <figure class="fb-chart-figure" aria-describedby="fb-chart-days">
            <figcaption class="fb-chart-legend">
                <span><span class="fb-swatch fb-chart-reports" aria-hidden="true">{""}</span>" "{words::fb_chart_others(lang, &words::number(lang, others))}</span>
                <span><span class="fb-swatch fb-chart-crashes" aria-hidden="true">{""}</span>" "{words::fb_chart_crashes(lang, &words::number(lang, crashes))}</span>
            </figcaption>
            {columns(lang, days)}
            <details class="fb-more" id="fb-chart-days">
                <summary>{words::fb_chart_table(lang)}</summary>
                <div class="fb-table-box" role="region" aria-label=words::fb_chart_table(lang) tabindex="0">
                    <table class="fb-table">
                        <thead>
                            <tr>
                                <th scope="col">{words::fb_chart_day_head(lang)}</th>
                                <th scope="col">{words::fb_chart_others(lang, "")}</th>
                                <th scope="col">{words::fb_chart_crashes(lang, "")}</th>
                            </tr>
                        </thead>
                        <tbody>{rows}</tbody>
                    </table>
                </div>
            </details>
        </figure>
    }
}

/// The chart, folded until it is opened.
pub fn view(lang: Lang) -> impl IntoView {
    let load = RwSignal::new(Load::Idle);
    let read = move || {
        load.set(Load::Loading);
        leptos::task::spawn_local(async move {
            let now = i64::try_from(super::super::pow::now_ms() / 1000).unwrap_or(0);
            let next = match api::stats().await {
                Ok(counts) => Load::Ready(last_days(&counts, now)),
                Err(Failure::Rejected) => Load::Failed(words::fb_err_chart(lang)),
                Err(failure) => Load::Failed(super::super::auth::problem_text(
                    lang,
                    super::super::auth::Problem::outside_sign_in(failure),
                )),
            };
            let _ = load.try_set(next);
        });
    };
    let body = move || {
        match load.get() {
        Load::Idle => ().into_any(),
        Load::Loading => {
            view! { <p class="meta" role="status">{words::account_loading(lang)}</p> }.into_any()
        }
        Load::Failed(text) => view! {
            <p class="error" role="alert">{text}</p>
            <p><button type="button" class="button button-ghost button-sm" on:click=move |_| read()>{words::error_again(lang)}</button></p>
        }
        .into_any(),
        Load::Ready(days) => figure(lang, &days).into_any(),
    }
    };
    view! {
        <details class="fb-more fb-chart" on:toggle=move |ev| {
            let open = event_target::<leptos::web_sys::HtmlDetailsElement>(&ev).open();
            if open && matches!(load.get_untracked(), Load::Idle) {
                read();
            }
        }>
            <summary>{words::fb_chart(lang)}</summary>
            {body}
        </details>
    }
}
