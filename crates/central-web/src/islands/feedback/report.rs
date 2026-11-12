//! `/{lang}/account/admin/feedback/<id in hex>`: one report, read and
//! decided (`docs/reports.md` §"The website's section"): what the player
//! wrote, what the client sent, where it came from, its triage, notes, the
//! lists it belongs with, its history, and the trash; the way back to the
//! list it was opened from (its query rides along in the address) and to
//! that list's previous and next report (`ReportNeighbours`). Opening a
//! report marks it opened for this administrator; every change names the
//! revision it was read at, and a conflict reads the report again and
//! says so.

use std::sync::Arc;

use central_api::admin::AdminRequest;
use central_api::reports::{
    Channel, MAX_CLIENT_GZIP, MAX_LABELS, MAX_NOTE, MAX_RECORD_GZIP, Neighbours, Priority,
    RecordOrigin, ReportChange, ReportDetail, ReportKind, ReportStatus, TRASH_DAYS, valid_label,
};
use leptos::prelude::*;
use serde_json::{Map, Value};

use super::super::account::when;
use super::super::central::Failure;
use super::super::chat::glyph;
use super::super::{clip, focus, session};
use super::api::{self, PartFailure, browser};
use super::client::{self, MAX_CLIENT_JSON, Piece, Refs, SystemRow, Unreadable};
use super::github::{category, issue_url, new_issue_url, parse_issue};
use super::query::{self, ListQuery, list_path, report_path};
use super::shared::{
    Ctx, DOWNLOAD, EXTERNAL, NEXT, PREVIOUS, Purge, Purged, TRASH, kind_pill, lost, notices_view,
    priority_pill, status_pill,
};
use super::text::{
    action_text, alias, detail_text, first_line, hex, kind_word, priority_text, read_report_id,
    short_id, size, status_word,
};
use super::tree::tree;
use crate::words::{self, Lang};

#[derive(Clone, Debug)]
enum Load {
    Loading,
    Failed(&'static str),
    Ready(Box<ReportDetail>),
}

/// The client's details as the page has them.
#[derive(Clone)]
enum Client {
    /// None were announced.
    None,
    /// Announced, not arrived.
    Awaiting,
    Loading,
    Failed(&'static str),
    Ready(Arc<Map<String, Value>>),
}

/// The report page's signals. All of it is `Copy`.
#[derive(Clone, Copy)]
struct Rep {
    c: Ctx,
    id: [u8; 16],
    query: StoredValue<ListQuery>,
    load: RwSignal<Load>,
    busy: RwSignal<bool>,
    error: RwSignal<Option<&'static str>>,
    near: RwSignal<Option<Neighbours>>,
    client: RwSignal<Client>,
    known: RwSignal<Option<Vec<String>>>,
    note: RwSignal<String>,
    label: RwSignal<String>,
    label_bad: RwSignal<bool>,
    dup: RwSignal<String>,
    dup_bad: RwSignal<bool>,
    issue: RwSignal<String>,
    issue_bad: RwSignal<bool>,
    summary: RwSignal<String>,
    category: RwSignal<Option<ReportKind>>,
    recording: RwSignal<bool>,
    shot_full: RwSignal<bool>,
    purge: Purge,
}

impl Rep {
    fn code(self) -> &'static str {
        self.c.lang.code()
    }

    /// `f` of the report, where it is read.
    fn with<T>(self, f: impl FnOnce(&ReportDetail) -> T) -> Option<T> {
        self.load.with(|load| match load {
            Load::Ready(detail) => Some(f(detail)),
            _ => None,
        })
    }

    fn show(self, detail: ReportDetail) {
        let client = match &detail.client {
            None => Some(Client::None),
            Some(part) if !part.received => Some(Client::Awaiting),
            // read once it arrived, and only once
            Some(_) => match self.client.get_untracked() {
                Client::None | Client::Awaiting => None,
                _ => Some(self.client.get_untracked()),
            },
        };
        let read_client = client.is_none();
        let _ = self.load.try_set(Load::Ready(Box::new(detail)));
        match client {
            Some(client) => {
                let _ = self.client.try_set(client);
            }
            None if read_client => self.read_client(),
            None => {}
        }
    }

    /// Reads the report; the first read marks it opened.
    fn read(self, first: bool) {
        let lang = self.c.lang;
        leptos::task::spawn_local(async move {
            match api::report(self.id).await {
                Ok(mut detail) => {
                    if first && !detail.summary.seen {
                        // the mark is a convenience: the next read says how it is
                        detail.summary.seen = true;
                        leptos::task::spawn_local(async move {
                            let _ = api::done(AdminRequest::SetReportsSeen {
                                reports: vec![self.id],
                                seen: true,
                            })
                            .await;
                        });
                    }
                    self.show(detail);
                }
                Err(Failure::Rejected) => {
                    if let Some(why) = self.c.explain().await {
                        let _ = self.load.try_set(Load::Failed(
                            self.c.why_text(why, words::fb_err_report(lang)),
                        ));
                    }
                }
                Err(failure) => {
                    let _ = self.load.try_set(Load::Failed(self.c.read_failed(failure)));
                }
            }
        });
    }

    fn read_near(self) {
        let q = self.query.get_value();
        let asked = query::request_of(&q);
        leptos::task::spawn_local(async move {
            // the page reads without its neighbours
            if let Ok(near) =
                api::neighbours(self.id, asked.filter, asked.sort, asked.descending).await
            {
                let _ = self.near.try_set(Some(near));
            }
        });
    }

    fn read_client(self) {
        let lang = self.c.lang;
        self.client.set(Client::Loading);
        leptos::task::spawn_local(async move {
            let max = usize::try_from(MAX_CLIENT_GZIP).unwrap_or(usize::MAX);
            let read = match api::part(self.id, "client", max).await {
                Ok(part) => match api::inflate(&part.gzip, MAX_CLIENT_JSON).await {
                    Ok(bytes) => client::parse(&bytes),
                    Err(true) => Err(Unreadable::Large),
                    Err(false) => Err(Unreadable::Shape),
                },
                Err(PartFailure::Shape) => Err(Unreadable::Shape),
                Err(PartFailure::Central(failure)) => {
                    let _ = self.client.try_set(Client::Failed(match failure {
                        Failure::Rejected => words::fb_err_report(lang),
                        failure => self.c.read_failed(failure),
                    }));
                    return;
                }
            };
            let _ = self.client.try_set(match read {
                Ok(map) => Client::Ready(Arc::new(map)),
                Err(Unreadable::Large) => Client::Failed(words::fb_err_client_large(lang)),
                Err(Unreadable::Shape) => Client::Failed(words::fb_err_client_shape(lang)),
            });
        });
    }

    /// Says what failed of a change to this report; an answer that did
    /// not arrive reads the report again, for it may have been made.
    async fn failed(self, failure: Failure) {
        let lang = self.c.lang;
        let said = match failure {
            Failure::Conflict => {
                self.read(false);
                words::fb_conflict(lang)
            }
            Failure::Rejected => match self.c.explain().await {
                Some(why) => self.c.why_text(why, words::fb_err_refused(lang)),
                None => return,
            },
            Failure::Busy => words::error_busy(lang),
            failure if lost(failure) => {
                self.read(false);
                words::fb_lost(lang)
            }
            _ => words::error_failed(lang),
        };
        let _ = self.error.try_set(Some(said));
        focus::after_render("#fb-report-error");
    }

    /// A change to the report's triage, at the revision it was read at;
    /// the keyboard goes back to `at` once it is drawn again.
    fn change(self, change: ReportChange, at: String) {
        if self.busy.get_untracked() || change.validate(false).is_err() {
            return;
        }
        let Some(revision) = self.with(|d| d.summary.revision) else {
            return;
        };
        self.busy.set(true);
        self.error.set(None);
        let lang = self.c.lang;
        leptos::task::spawn_local(async move {
            match api::set_report(self.id, change, revision).await {
                Ok(detail) => {
                    self.show(detail);
                    self.c.notices.say(words::fb_saved(lang));
                    focus::after_render(at);
                }
                Err(failure) => self.failed(failure).await,
            }
            let _ = self.busy.try_set(false);
        });
    }

    /// A request answered with the report: a note added or removed.
    fn note_change(self, request: AdminRequest, said: &'static str, at: &'static str) {
        if self.busy.get_untracked() {
            return;
        }
        self.busy.set(true);
        self.error.set(None);
        let adding = matches!(request, AdminRequest::AddReportNote { .. });
        leptos::task::spawn_local(async move {
            match api::one(request).await {
                Ok(detail) => {
                    self.show(detail);
                    if adding {
                        self.note.set(String::new());
                    }
                    self.c.notices.say(said);
                    focus::after_render(at);
                }
                // the draft stays: a note that may have been saved is read first
                Err(failure) => self.failed(failure).await,
            }
            let _ = self.busy.try_set(false);
        });
    }

    fn trash(self) {
        if self.busy.get_untracked() {
            return;
        }
        self.busy.set(true);
        self.error.set(None);
        let lang = self.c.lang;
        leptos::task::spawn_local(async move {
            match api::done(AdminRequest::TrashReport { report: self.id }).await {
                Ok(()) => {
                    let undo = Callback::new(move |()| self.restore());
                    self.c.notices.say_undo(words::fb_trashed_one(lang), undo);
                    self.read(false);
                    focus::after_render("#fb-report-title");
                }
                Err(failure) => self.failed(failure).await,
            }
            let _ = self.busy.try_set(false);
        });
    }

    fn restore(self) {
        let lang = self.c.lang;
        leptos::task::spawn_local(async move {
            match api::done(AdminRequest::RestoreReport { report: self.id }).await {
                Ok(()) => {
                    self.c.notices.say(words::fb_restored_one(lang));
                    self.read(false);
                    focus::after_render("#fb-report-title");
                }
                Err(failure) => self.failed(failure).await,
            }
        });
    }

    /// The record as Central keeps it, a `.gz` handed to the browser.
    fn download_record(self) {
        if self.recording.get_untracked() {
            return;
        }
        self.recording.set(true);
        leptos::task::spawn_local(async move {
            let max = usize::try_from(MAX_RECORD_GZIP).unwrap_or(usize::MAX);
            match api::part(self.id, "record", max).await {
                Ok(part) => {
                    let name = format!("baylee-report-{}-record.jsonl.gz", short_id(&self.id));
                    browser::save(&name, &part.gzip, "application/gzip");
                }
                Err(PartFailure::Shape) => {
                    self.c.notices.fail(words::fb_err_client_shape(self.c.lang));
                }
                Err(PartFailure::Central(failure)) => {
                    self.c.notices.fail(self.c.read_failed(failure));
                }
            }
            let _ = self.recording.try_set(false);
        });
    }

    fn copy(self, text: String) {
        let lang = self.c.lang;
        leptos::task::spawn_local(async move {
            if clip::write(&text).await {
                self.c.notices.say(words::fb_copied(lang));
            } else {
                self.c.notices.fail(words::fb_copy_failed(lang));
            }
        });
    }
}

/// The site's own origin, for a report's link.
fn origin() -> String {
    leptos::web_sys::window()
        .and_then(|w| w.location().origin().ok())
        .unwrap_or_default()
}

fn steps_view(r: &Rep) -> impl IntoView + use<> {
    let r = *r;
    let lang = r.c.lang;
    let q = r.query.get_value();
    let code = r.code();
    let back = list_path(code, &q);
    let step = move |to: Option<[u8; 16]>, shapes: &'static [glyph::Shape], label: &'static str| {
        match to {
            Some(to) => view! {
                <a class="round-btn" href=report_path(code, &to, &r.query.get_value())>
                    {glyph::glyph(shapes)}<span class="visually-hidden">{label}</span>
                </a>
            }
            .into_any(),
            None => view! { <span class="round-btn fb-off" aria-hidden="true">{glyph::glyph(shapes)}</span> }
                .into_any(),
        }
    };
    let near = move || {
        let near = r.near.get();
        let place = near.as_ref().map(|n| match n.position {
            Some(at) => words::fb_position(
                lang,
                &words::number(lang, at),
                &words::number(lang, n.total),
            ),
            None => words::fb_not_in_list(lang).to_owned(),
        });
        view! {
            {step(near.as_ref().and_then(|n| n.previous), PREVIOUS, words::fb_previous(lang))}
            <span class="meta">{place}</span>
            {step(near.as_ref().and_then(|n| n.next), NEXT, words::fb_next(lang))}
        }
    };
    view! {
        <nav class="fb-steps" aria-label=words::fb_steps(lang)>
            <a class="button button-ghost button-sm" href=back>{glyph::glyph(glyph::BACK)}<span>{words::fb_back_to_list(lang)}</span></a>
            <span class="fb-steps-near">{near}</span>
        </nav>
    }
}

/// A section of the report: a card with its small heading.
fn section(id: &'static str, title: &'static str, body: impl IntoView) -> impl IntoView {
    view! {
        <section class="card fb-section" aria-labelledby=id>
            <h3 id=id class="fb-section-title">{title}</h3>
            {body}
        </section>
    }
}

/// The player's words, each reference the client resolved in its place.
fn text_view(lang: Lang, text: &str, refs: Option<&Refs>) -> AnyView {
    let Some(refs) = refs else {
        return view! { {text.to_owned()} }.into_any();
    };
    client::pieces(text, refs)
        .into_iter()
        .map(|piece| match piece {
            Piece::Text(t) => view! { {t} }.into_any(),
            Piece::Card(i) => {
                let card = &refs.cards[i];
                let mut place: Vec<String> = Vec::new();
                if let Some(zone) = &card.zone {
                    place.push(zone.replace('_', " "));
                }
                if let Some(owner) = card.owner {
                    place.push(words::fb_seat(lang, &owner.to_string()));
                }
                let title = if place.is_empty() {
                    card.name.clone()
                } else {
                    format!("{} · {}", card.name, place.join(", "))
                };
                view! { <span class="fb-chip" title=title>{card.name.clone()}</span> }.into_any()
            }
            Piece::Player(i) => {
                let player = &refs.players[i];
                let title = player.seat.map_or_else(
                    || player.name.clone(),
                    |s| words::fb_seat(lang, &s.to_string()),
                );
                view! { <span class="fb-chip" title=title>{format!("@{}", player.name)}</span> }
                    .into_any()
            }
        })
        .collect_view()
        .into_any()
}

fn wrote_view(r: &Rep) -> impl IntoView + use<> {
    let r = *r;
    let lang = r.c.lang;
    let body = move || {
        let refs = r.client.with(|c| match c {
            Client::Ready(map) => client::refs(map),
            _ => None,
        });
        r.with(|d| {
            let text = if d.text.trim().is_empty() {
                view! { <p class="meta">{words::fb_no_text(lang)}</p> }.into_any()
            } else {
                view! { <p class="fb-text" dir="auto">{text_view(lang, &d.text, refs.as_ref())}</p> }.into_any()
            };
            let crash = d.summary.crash.clone().map(|message| {
                let place = d.crash_location.clone().map(|l| view! { <code class="fb-code">{l}</code> });
                view! {
                    <div class="fb-crash">
                        <p class="fb-legend">{words::fb_crash(lang)}</p>
                        <pre class="fb-pre">{message}</pre>
                        {place}
                    </div>
                }
            });
            view! { {text}{crash} }
        })
    };
    section("fb-wrote", words::fb_wrote(lang), body)
}

/// The window's picture, drawn from its checked bytes into a canvas.
fn shot_view(r: &Rep, shot: client::Screenshot) -> impl IntoView + use<> {
    let r = *r;
    let lang = r.c.lang;
    let canvas: NodeRef<leptos::html::Canvas> = NodeRef::new();
    let png = Arc::new(shot.png);
    Effect::new(move |_| {
        #[cfg(target_arch = "wasm32")]
        if let Some(element) = canvas.get() {
            let png = Arc::clone(&png);
            leptos::task::spawn_local(async move {
                let _ = browser::draw(element, &png).await;
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = (&canvas, &png);
    });
    let alt = words::fb_shot_alt(lang, &shot.width.to_string(), &shot.height.to_string());
    view! {
        <div class="fb-shot" class:fb-shot-full=move || r.shot_full.get()>
            <canvas node_ref=canvas role="img" aria-label=alt width=shot.width.to_string() height=shot.height.to_string()></canvas>
        </div>
        <p>
            <button type="button" class="button button-ghost button-sm" id="fb-shot-size" aria-pressed=move || r.shot_full.get().to_string()
                on:click=move |_| r.shot_full.update(|f| *f = !*f)>{words::fb_shot_full(lang)}</button>
        </p>
    }
}

/// A fold of the client's details.
fn fold(title: String, body: impl IntoView) -> impl IntoView {
    view! {
        <details class="fb-more">
            <summary>{title}</summary>
            {body}
        </details>
    }
}

fn system_word(lang: Lang, row: SystemRow) -> &'static str {
    match row {
        SystemRow::Platform => words::fb_sys_platform(lang),
        SystemRow::Cpus => words::fb_sys_cpus(lang),
        SystemRow::Adapter => words::fb_sys_adapter(lang),
        SystemRow::Backend => words::fb_sys_backend(lang),
        SystemRow::Window => words::fb_sys_window(lang),
        SystemRow::Scale => words::fb_sys_scale(lang),
        SystemRow::Lang => words::fb_sys_lang(lang),
    }
}

fn client_details(r: &Rep, c: &Map<String, Value>) -> AnyView {
    let r = *r;
    let lang = r.c.lang;
    let shot = client::screenshot(c).map_or_else(
        || view! { <p class="meta">{words::fb_no_shot(lang)}</p> }.into_any(),
        |shot| shot_view(&r, shot).into_any(),
    );
    let mut facts: Vec<AnyView> = Vec::new();
    if let Some(build) = client::build(c) {
        let commit = build.commit.map(|c| c.chars().take(12).collect::<String>());
        let text = [build.version, commit]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" · ");
        let text = if text.is_empty() {
            "—".to_owned()
        } else {
            text
        };
        facts.push(view! { <dt>{words::fb_client_build(lang)}</dt><dd><code class="fb-code">{text}</code></dd> }.into_any());
    }
    if let Some(game) = client::game(c) {
        if let Some(moment) = game.when {
            facts.push(view! { <dt>{words::fb_game_when(lang)}</dt><dd>{moment}</dd> }.into_any());
        }
        if let Some(turn) = game.turn {
            let text = match game.active {
                Some(active) => {
                    words::fb_game_turn_of(lang, &turn.to_string(), &active.to_string())
                }
                None => turn.to_string(),
            };
            facts.push(view! { <dt>{words::fb_game_turn(lang)}</dt><dd>{text}</dd> }.into_any());
        }
        if let Some(seat) = game.seat {
            let text = match game.players {
                Some(n) => words::fb_game_seat_of(lang, &seat.to_string(), &n.to_string()),
                None => seat.to_string(),
            };
            facts.push(view! { <dt>{words::fb_game_seat(lang)}</dt><dd>{text}</dd> }.into_any());
        }
        let pending = game.pending.map_or_else(
            || view! { {words::fb_game_nothing(lang)} }.into_any(),
            |p| view! { <code class="fb-code">{client::spaced(&p)}</code> }.into_any(),
        );
        facts.push(view! { <dt>{words::fb_game_pending(lang)}</dt><dd>{pending}</dd> }.into_any());
    }
    let backtrace = client::crash(c).and_then(|c| c.backtrace).map(|trace| {
        fold(
            words::fb_backtrace(lang).to_owned(),
            view! { <pre class="fb-pre">{trace}</pre> },
        )
    });
    let log = client::log(c).map(|log| {
        let roster = (!log.roster.is_empty()).then(|| {
            let text = log
                .roster
                .iter()
                .map(|(seat, name, ai)| {
                    let seat = seat.map_or_else(|| "?".to_owned(), |s| s.to_string());
                    if *ai {
                        format!("{seat}: {name} ({})", words::fb_computer(lang))
                    } else {
                        format!("{seat}: {name}")
                    }
                })
                .collect::<Vec<_>>()
                .join(" · ");
            view! { <p class="meta">{text}</p> }
        });
        let lines = log
            .lines
            .iter()
            .map(|line| {
                let turn = line
                    .turn
                    .map(|t| view! { <span class="meta">{format!("T{t} ")}</span> });
                view! { <li>{turn}{line.text.clone()}</li> }
            })
            .collect_view();
        let title = words::fb_log(lang, &log.lines.len().to_string());
        fold(title, view! { {roster}<ol class="fb-log">{lines}</ol> })
    });
    let system = client::system(c)
        .filter(|rows| !rows.is_empty())
        .map(|rows| {
            let rows = rows
                .into_iter()
                .map(|(row, value)| view! { <dt>{system_word(lang, row)}</dt><dd>{value}</dd> })
                .collect_view();
            fold(
                words::fb_system(lang).to_owned(),
                view! { <dl class="facts">{rows}</dl> },
            )
        });
    let settings = c
        .get("settings")
        .map(|s| fold(words::fb_settings(lang).to_owned(), tree(lang, s, true)));
    let everything = Value::Object(c.clone());
    view! {
        <div class="fb-client">
            {shot}
            {(!facts.is_empty()).then(|| view! { <dl class="facts">{facts}</dl> })}
            {backtrace}
            {log}
            {system}
            {settings}
            {fold(words::fb_everything(lang).to_owned(), tree(lang, &everything, true))}
        </div>
    }
    .into_any()
}

fn sent_view(r: &Rep) -> impl IntoView + use<> {
    let r = *r;
    let lang = r.c.lang;
    let body = move || {
        match r.client.get() {
        Client::None => view! { <p class="meta">{words::fb_no_client_part(lang)}</p> }.into_any(),
        Client::Awaiting => view! { <p class="meta">{words::fb_client_awaiting(lang)}</p> }.into_any(),
        Client::Loading => {
            view! { <p class="meta" role="status">{words::account_loading(lang)}</p> }.into_any()
        }
        Client::Failed(text) => view! {
            <p class="error" role="alert">{text}</p>
            <p><button type="button" class="button button-ghost button-sm" on:click=move |_| r.read_client()>{words::error_again(lang)}</button></p>
        }
        .into_any(),
        Client::Ready(map) => client_details(&r, &map),
    }
    };
    section("fb-sent", words::fb_client_sent(lang), body)
}

fn facts_view(r: &Rep) -> impl IntoView + use<> {
    let r = *r;
    let lang = r.c.lang;
    let body = move || {
        r.with(|d| {
            let s = &d.summary;
            let gateway = if s.channel == Channel::Direct {
                view! { {words::fb_channel_direct(lang)} }.into_any()
            } else {
                let says = s
                    .gateway_says
                    .clone()
                    .filter(|says| *says != s.gateway)
                    .map(|says| {
                        view! { <span class="meta">{format!(" · {} ", words::fb_calls_itself(lang))}<bdi translate="no">{says}</bdi></span> }
                    });
                view! { <bdi translate="no">{s.gateway.clone()}</bdi>{says} }.into_any()
            };
            // the address the gateway gave, as text: never a link
            let url = d
                .gateway_url
                .clone()
                .map(|u| view! { <br /><code class="fb-code" translate="no">{u}</code> });
            let gateway_build = s.gateway_version.clone().map(|v| {
                view! { <dt>{words::fb_gateway_build(lang)}</dt><dd><code class="fb-code">{v}</code></dd> }
            });
            let build = s.build.clone().map_or_else(
                || view! { {"—"} }.into_any(),
                |b| view! { <code class="fb-code">{b}</code> }.into_any(),
            );
            let commit = d.commit.clone().map(|c| {
                view! { " " <code class="fb-code">{c.chars().take(12).collect::<String>()}</code> }
            });
            let (name, short) = alias(&s.reporter);
            let game = d.game_id.clone().map(|g| {
                view! { <dt>{words::fb_game(lang)}</dt><dd><code class="fb-code" translate="no">{g}</code></dd> }
            });
            let record = match &d.record {
                None => view! { {words::fb_no_record(lang)} }.into_any(),
                Some(part) if part.received => {
                    let who = if s.record_origin == Some(RecordOrigin::Client) {
                        words::fb_by_client(lang)
                    } else {
                        words::fb_by_gateway(lang)
                    };
                    let end = if s.record_complete {
                        words::fb_record_complete(lang)
                    } else {
                        words::fb_record_partial(lang)
                    };
                    view! {
                        {words::fb_record_here(lang, &size(lang, u64::from(part.bytes)), who, end)}
                        <br />
                        <button type="button" class="button button-ghost button-sm" id="fb-record"
                            aria-disabled=move || r.recording.get().then_some("true") on:click=move |_| r.download_record()>
                            {glyph::glyph(DOWNLOAD)}<span>{words::fb_download_record(lang)}</span>
                        </button>
                    }
                    .into_any()
                }
                Some(part) => {
                    view! { {words::fb_record_awaited(lang, &size(lang, u64::from(part.bytes)))} }.into_any()
                }
            };
            let client = match &d.client {
                None => words::fb_no_client(lang).to_owned(),
                Some(part) if part.received => words::fb_client_here(lang, &size(lang, u64::from(part.bytes))),
                Some(part) => words::fb_client_awaited(lang, &size(lang, u64::from(part.bytes))),
            };
            let id = hex(&s.id);
            let copied = id.clone();
            view! {
                <dl class="facts">
                    <dt>{words::fb_received(lang)}</dt><dd>{when(lang, s.created_at)}</dd>
                    <dt>{words::fb_changed(lang)}</dt><dd>{when(lang, s.updated_at)}</dd>
                    <dt>{words::fb_gateway(lang)}</dt><dd>{gateway}{url}</dd>
                    {gateway_build}
                    <dt>{words::fb_client_build(lang)}</dt><dd>{build}{commit}</dd>
                    <dt>{words::fb_reporter(lang)}</dt>
                    <dd><span class="fb-reporter" title=s.reporter.clone() translate="no"><span>{name}</span>" "<code>{short}</code></span></dd>
                    {game}
                    <dt>{words::fb_record_part(lang)}</dt><dd>{record}</dd>
                    <dt>{words::fb_client_part(lang)}</dt><dd>{client}</dd>
                    <dt>{words::fb_id(lang)}</dt>
                    <dd class="fb-copyable">
                        <code class="fb-code" translate="no">{id}</code>
                        <button type="button" class="button button-ghost button-sm" on:click=move |_| r.copy(copied.clone())>
                            {glyph::glyph(glyph::COPY)}<span>{words::fb_copy_id(lang)}</span>
                        </button>
                    </dd>
                </dl>
            }
        })
    };
    section("fb-facts", words::fb_facts(lang), body)
}

fn history_view(r: &Rep) -> impl IntoView + use<> {
    let r = *r;
    let lang = r.c.lang;
    let body = move || {
        r.with(|d| {
            if d.audit.is_empty() {
                return view! { <p class="meta">{words::fb_no_history(lang)}</p> }.into_any();
            }
            let c = r.c;
            let name = move |id: Option<[u8; 16]>| c.name(id);
            let rows = d
                .audit
                .iter()
                .map(|row| {
                    let who = row
                        .actor
                        .map_or_else(|| words::fb_h_central(lang).to_owned(), |a| c.name(Some(a)));
                    let what = action_text(lang, &row.action).map_or_else(
                        || view! { <code>{row.action.clone()}</code> }.into_any(),
                        |w| view! { {w} }.into_any(),
                    );
                    let detail = detail_text(lang, &row.action, &row.detail, &name)
                        .map(|d| view! { " " <bdi class="fb-detail">{d}</bdi> });
                    view! {
                        <li>
                            <span class="meta">{when(lang, row.at)}</span>" "
                            <bdi translate="no">{who}</bdi>": "{what}{detail}
                        </li>
                    }
                })
                .collect_view();
            view! { <ol class="fb-history">{rows}</ol> }.into_any()
        })
    };
    section("fb-history", words::fb_history(lang), body)
}

fn chips_view(r: &Rep) -> impl IntoView + use<> {
    let r = *r;
    let lang = r.c.lang;
    move || {
        r.with(|d| {
            let now = d.summary.status;
            let statuses = ReportStatus::ALL
                .iter()
                .filter(|s| **s != ReportStatus::Duplicate)
                .map(|s| {
                    let s = *s;
                    let id = format!("fb-status-{}", s.as_str());
                    let at = format!("#{id}");
                    view! {
                        <button type="button" class="chip-btn" id=id aria-pressed=(now == s).to_string()
                            aria-disabled=(now == s).then_some("true")
                            on:click=move |_| if now != s {
                                r.change(ReportChange { status: Some(s), ..ReportChange::default() }, at.clone());
                            }>{status_word(lang, s)}</button>
                    }
                })
                .collect_view();
            let duplicate = (now == ReportStatus::Duplicate).then(|| {
                view! {
                    <button type="button" class="chip-btn" aria-pressed="true" aria-disabled="true">
                        {status_word(lang, ReportStatus::Duplicate)}
                    </button>
                }
            });
            let current = d.summary.priority;
            let low_first: [Option<Priority>; 5] = [
                None,
                Some(Priority::Low),
                Some(Priority::Normal),
                Some(Priority::High),
                Some(Priority::Critical),
            ];
            let priorities = low_first
                .into_iter()
                .map(|p| {
                    let id = format!("fb-priority-{}", query::priority_word(p));
                    let at = format!("#{id}");
                    view! {
                        <button type="button" class="chip-btn" id=id aria-pressed=(current == p).to_string()
                            aria-disabled=(current == p).then_some("true")
                            on:click=move |_| if current != p {
                                r.change(ReportChange { priority: Some(p), ..ReportChange::default() }, at.clone());
                            }>{priority_text(lang, p)}</button>
                    }
                })
                .collect_view();
            view! {
                <fieldset class="fb-chips">
                    <legend>{words::fb_status(lang)}</legend>
                    <div class="fb-chip-row">{statuses}{duplicate}</div>
                </fieldset>
                <fieldset class="fb-chips">
                    <legend>{words::fb_priority(lang)}</legend>
                    <div class="fb-chip-row">{priorities}</div>
                </fieldset>
            }
        })
    }
}

fn assignee_view(r: &Rep) -> impl IntoView + use<> {
    let r = *r;
    let lang = r.c.lang;
    let now = move || r.with(|d| d.summary.assignee).flatten();
    let options = move || {
        let current = now();
        let mut options = vec![
            view! { <option value="" prop:selected=current.is_none()>{words::fb_nobody(lang)}</option> }.into_any(),
        ];
        for id in r.c.people(current) {
            options.push(
                view! { <option value=hex(&id) prop:selected=current == Some(id)>{r.c.who(id)}</option> }
                    .into_any(),
            );
        }
        options.collect_view()
    };
    let me = r.c.me;
    view! {
        <div class="field fb-field">
            <label for="fb-assignee">{words::fb_assignee(lang)}</label>
            <div class="fb-inline">
                <select id="fb-assignee" aria-disabled=move || r.busy.get().then_some("true") on:change=move |ev| {
                    let v = event_target_value(&ev);
                    let whom = super::text::id_of(&v);
                    if whom != now() && (v.is_empty() || whom.is_some()) {
                        r.change(ReportChange { assignee: Some(whom), ..ReportChange::default() }, "#fb-assignee".into());
                    }
                }>{options}</select>
                {move || (now() != Some(me)).then(|| view! {
                    <button type="button" class="button button-ghost button-sm" on:click=move |_| {
                        r.change(ReportChange { assignee: Some(Some(me)), ..ReportChange::default() }, "#fb-assignee".into());
                    }>{words::fb_assign_me(lang)}</button>
                })}
            </div>
        </div>
    }
}

fn labels_view(r: &Rep) -> impl IntoView + use<> {
    let r = *r;
    let lang = r.c.lang;
    let labels = move || r.with(|d| d.summary.labels.clone()).unwrap_or_default();
    let list = move || {
        let all = labels();
        (!all.is_empty()).then(|| {
            let items = all
                .iter()
                .map(|label| {
                    let href = list_path(r.code(), &ListQuery { label: Some(label.clone()), ..ListQuery::default() });
                    let rest: Vec<String> = all.iter().filter(|l| *l != label).cloned().collect();
                    let remove = words::fb_remove_label(lang, label);
                    view! {
                        <li>
                            <a class="fb-label" href=href translate="no">{label.clone()}</a>
                            <button type="button" class="round-btn" on:click=move |_| {
                                r.change(ReportChange { labels: Some(rest.clone()), ..ReportChange::default() }, "#fb-label-new".into());
                            }>
                                {glyph::glyph(glyph::CLOSE)}<span class="visually-hidden">{remove}</span>
                            </button>
                        </li>
                    }
                })
                .collect_view();
            view! { <ul class="fb-label-list">{items}</ul> }
        })
    };
    let full = move || labels().len() >= MAX_LABELS;
    let add = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let label = r.label.get_untracked().trim().to_ascii_lowercase();
        if !valid_label(&label) {
            r.label_bad.set(true);
            focus::after_render("#fb-label-new");
            return;
        }
        r.label_bad.set(false);
        let mut all = labels();
        if all.contains(&label) || full() {
            r.label.set(String::new());
            return;
        }
        all.push(label);
        all.sort();
        r.label.set(String::new());
        r.change(
            ReportChange {
                labels: Some(all),
                ..ReportChange::default()
            },
            "#fb-label-new".into(),
        );
    };
    // the labels in use, offered once the field is first reached
    let known = move || {
        let mine = labels();
        r.known
            .get()
            .unwrap_or_default()
            .into_iter()
            .filter(|l| !mine.contains(l))
            .map(|l| view! { <option value=l></option> })
            .collect_view()
    };
    let learn = move |_| {
        if r.known.get_untracked().is_none() {
            r.known.set(Some(Vec::new()));
            leptos::task::spawn_local(async move {
                if let Ok(f) = api::facets(central_api::reports::ReportFilter::default()).await {
                    let _ = r
                        .known
                        .try_set(Some(f.labels.into_iter().map(|c| c.value).collect()));
                }
            });
        }
    };
    view! {
        <div class="fb-field">
            <p class="fb-legend">{words::fb_labels(lang)}</p>
            {list}
            <form class="fb-inline" method="post" novalidate=true on:submit=add>
                <label class="visually-hidden" for="fb-label-new">{words::fb_add_label(lang)}</label>
                <input id="fb-label-new" type="text" list="fb-labels-known" autocomplete="off" spellcheck="false" maxlength="32"
                    placeholder=words::fb_label_hint(lang) aria-invalid=move || r.label_bad.get().then_some("true")
                    aria-describedby=move || r.label_bad.get().then_some("fb-label-bad")
                    prop:disabled=full on:focus=learn
                    prop:value=move || r.label.get() on:input=move |ev| {
                        r.label.set(event_target_value(&ev));
                        r.label_bad.set(false);
                    } />
                <datalist id="fb-labels-known">{known}</datalist>
                <button type="submit" class="button button-ghost button-sm">{words::fb_add_label(lang)}</button>
            </form>
            <p class="error" id="fb-label-bad" role="alert">{move || r.label_bad.get().then(|| words::fb_bad_label(lang))}</p>
            {move || full().then(|| view! { <p class="meta">{words::fb_labels_full(lang, &MAX_LABELS.to_string())}</p> })}
        </div>
    }
}

fn duplicate_view(r: &Rep) -> impl IntoView + use<> {
    let r = *r;
    let lang = r.c.lang;
    let original = move || r.with(|d| d.summary.duplicate_of).flatten();
    let mark = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        if let Some(target) = read_report_id(&r.dup.get_untracked()).filter(|found| *found != r.id)
        {
            r.dup_bad.set(false);
            r.dup.set(String::new());
            r.change(
                ReportChange {
                    duplicate_of: Some(Some(target)),
                    ..ReportChange::default()
                },
                "#fb-not-duplicate".into(),
            );
        } else {
            r.dup_bad.set(true);
            focus::after_render("#fb-duplicate");
        }
    };
    move || {
        match original() {
        Some(of) => view! {
            <div class="fb-field">
                <p class="fb-legend">{words::fb_duplicate_of(lang)}</p>
                <div class="fb-inline">
                    <a href=report_path(r.code(), &of, &r.query.get_value())><code>{format!("…{}", short_id(&of))}</code></a>
                    <button type="button" class="button button-ghost button-sm" id="fb-not-duplicate" on:click=move |_| {
                        r.change(ReportChange { duplicate_of: Some(None), ..ReportChange::default() }, "#fb-duplicate".into());
                    }>{words::fb_not_duplicate(lang)}</button>
                </div>
            </div>
        }
        .into_any(),
        None => view! {
            <form class="field fb-field" method="post" novalidate=true on:submit=mark>
                <label for="fb-duplicate">{words::fb_mark_duplicate(lang)}</label>
                <div class="fb-inline">
                    <input id="fb-duplicate" type="text" autocomplete="off" spellcheck="false" maxlength="200"
                        aria-describedby="fb-duplicate-hint" aria-invalid=move || r.dup_bad.get().then_some("true")
                        prop:value=move || r.dup.get() on:input=move |ev| {
                            r.dup.set(event_target_value(&ev));
                            r.dup_bad.set(false);
                        } />
                    <button type="submit" class="button button-ghost button-sm">{words::fb_mark(lang)}</button>
                </div>
                <p class="hint" id="fb-duplicate-hint">{words::fb_duplicate_hint(lang)}</p>
                <p class="error" role="alert">{move || r.dup_bad.get().then(|| words::fb_bad_duplicate(lang))}</p>
            </form>
        }
        .into_any(),
    }
    }
}

fn issue_view(r: &Rep) -> impl IntoView + use<> {
    let r = *r;
    let lang = r.c.lang;
    let linked = move || r.with(|d| d.summary.issue).flatten();
    let link = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        if let Some(n) = parse_issue(&r.issue.get_untracked()) {
            r.issue_bad.set(false);
            r.issue.set(String::new());
            r.change(
                ReportChange {
                    issue: Some(Some(n)),
                    ..ReportChange::default()
                },
                "#fb-unlink".into(),
            );
        } else {
            r.issue_bad.set(true);
            focus::after_render("#fb-issue");
        }
    };
    let now = move || {
        match linked().and_then(|n| issue_url(n).map(|href| (n, href))) {
        Some((n, href)) => view! {
            <div class="fb-inline">
                <a class="fb-issue" href=href target="_blank" rel="noopener noreferrer">
                    {format!("#{n}")}{glyph::glyph(EXTERNAL)}<span class="visually-hidden">{" "}{words::fb_new_tab(lang)}</span>
                </a>
                <button type="button" class="button button-ghost button-sm" id="fb-unlink" on:click=move |_| {
                    r.change(ReportChange { issue: Some(None), ..ReportChange::default() }, "#fb-issue".into());
                }>{words::fb_unlink(lang)}</button>
            </div>
        }
        .into_any(),
        None => view! {
            <form class="field" method="post" novalidate=true on:submit=link>
                <label for="fb-issue">{words::fb_issue_label(lang)}</label>
                <div class="fb-inline">
                    <input id="fb-issue" type="text" autocomplete="off" spellcheck="false" maxlength="200"
                        aria-describedby="fb-issue-hint" aria-invalid=move || r.issue_bad.get().then_some("true")
                        prop:value=move || r.issue.get() on:input=move |ev| {
                            r.issue.set(event_target_value(&ev));
                            r.issue_bad.set(false);
                        } />
                    <button type="submit" class="button button-ghost button-sm">{words::fb_link(lang)}</button>
                </div>
                <p class="hint" id="fb-issue-hint">{words::fb_issue_hint(lang)}</p>
                <p class="error" role="alert">{move || r.issue_bad.get().then(|| words::fb_bad_issue(lang))}</p>
            </form>
        }
        .into_any(),
    }
    };
    // GitHub's form for a new issue, filled with the administrator's own neutral words
    let kind = move || r.with(|d| d.summary.kind).unwrap_or(ReportKind::Other);
    let build = move || {
        r.with(|d| {
            d.summary
                .build
                .clone()
                .or_else(|| d.summary.gateway_version.clone())
        })
        .flatten()
    };
    let chosen = move || r.category.get().unwrap_or_else(kind);
    let url = move || new_issue_url(&r.summary.get(), chosen(), build().as_deref());
    let categories = ReportKind::ALL
        .iter()
        .map(|k| {
            let k = *k;
            view! { <option value=k.as_str() prop:selected=move || chosen() == k>{category(k)}</option> }
        })
        .collect_view();
    let new_issue = move || {
        linked().is_none().then(|| {
            view! {
                <details class="fb-more">
                    <summary>{words::fb_new_issue(lang)}</summary>
                    <div class="field">
                        <label for="fb-issue-summary">{words::fb_summary_label(lang)}</label>
                        <textarea id="fb-issue-summary" rows="3" aria-describedby="fb-issue-summary-hint"
                            prop:value=move || r.summary.get() on:input=move |ev| r.summary.set(event_target_value(&ev))></textarea>
                        <p class="hint" id="fb-issue-summary-hint">{words::fb_summary_hint(lang)}</p>
                    </div>
                    <div class="field fb-field">
                        <label for="fb-issue-category">{words::fb_category(lang)}</label>
                        <select id="fb-issue-category" on:change=move |ev| {
                            let v = event_target_value(&ev);
                            r.category.set(ReportKind::ALL.iter().copied().find(|k| k.as_str() == v));
                        }>{categories.clone()}</select>
                    </div>
                    {move || match url() {
                        None => view! { <p class="meta">{words::fb_summary_first(lang)}</p> }.into_any(),
                        Some(href) => view! {
                            <a class="button button-ghost" href=href target="_blank" rel="noopener noreferrer">
                                <span>{words::fb_open_issue(lang)}</span>{glyph::glyph(EXTERNAL)}
                                <span class="visually-hidden">{" "}{words::fb_new_tab(lang)}</span>
                            </a>
                        }
                        .into_any(),
                    }}
                </details>
            }
        })
    };
    view! {
        <div class="fb-field">
            <p class="fb-legend">{words::fb_issue(lang)}</p>
            {now}
            {new_issue}
        </div>
    }
}

fn triage_view(r: &Rep) -> impl IntoView + use<> {
    let r = *r;
    let lang = r.c.lang;
    let trashed = move || r.with(|d| d.summary.deleted_at.is_some()).unwrap_or(false);
    let body = move || {
        if trashed() {
            view! { <p class="meta">{words::fb_trash_no_triage(lang)}</p> }.into_any()
        } else {
            view! {
                {chips_view(&r)}
                {assignee_view(&r)}
                {labels_view(&r)}
                {duplicate_view(&r)}
                {issue_view(&r)}
            }
            .into_any()
        }
    };
    section("fb-triage", words::fb_triage(lang), body)
}

fn notes_view(r: &Rep) -> impl IntoView + use<> {
    let r = *r;
    let lang = r.c.lang;
    let me = r.c.me;
    let list = move || {
        r.with(|d| {
            if d.notes.is_empty() {
                return view! { <p class="meta">{words::fb_no_notes(lang)}</p> }.into_any();
            }
            let items = d
                .notes
                .iter()
                .map(|n| {
                    let mine = n.author == me;
                    let at = when(lang, n.at);
                    let note = n.id;
                    let remove = mine.then(|| {
                        let label = words::fb_remove_note(lang, &at);
                        view! {
                            <button type="button" class="round-btn" aria-disabled=move || r.busy.get().then_some("true")
                                on:click=move |_| r.note_change(
                                    AdminRequest::RemoveReportNote { report: r.id, note },
                                    words::fb_note_removed(lang), "#fb-note")>
                                {glyph::glyph(glyph::CLOSE)}<span class="visually-hidden">{label}</span>
                            </button>
                        }
                    });
                    view! {
                        <li class="fb-note">
                            <p class="fb-note-head meta">
                                <bdi translate="no">{r.c.who(n.author)}</bdi>" · "{at.clone()}
                                {remove}
                            </p>
                            <p class="fb-note-text" dir="auto">{n.text.clone()}</p>
                        </li>
                    }
                })
                .collect_view();
            view! { <ol class="fb-notes">{items}</ol> }.into_any()
        })
    };
    let left = move || {
        let used = r.note.with(|n| n.chars().count());
        i64::try_from(MAX_NOTE).unwrap_or(0) - i64::try_from(used).unwrap_or(0)
    };
    let add = move || {
        let text = r.note.get_untracked().trim().to_owned();
        if text.is_empty() || left() < 0 {
            return;
        }
        r.note_change(
            AdminRequest::AddReportNote { report: r.id, text },
            words::fb_note_saved(lang),
            "#fb-note",
        );
    };
    let title = move || {
        let n = r.with(|d| d.notes.len()).unwrap_or(0);
        if n > 0 {
            words::fb_notes_n(lang, &n.to_string())
        } else {
            words::fb_notes(lang).to_owned()
        }
    };
    view! {
        <section class="card fb-section" aria-labelledby="fb-notes">
            <h3 id="fb-notes" class="fb-section-title">{title}</h3>
            {list}
            <form class="field fb-note-form" method="post" novalidate=true on:submit=move |ev| {
                ev.prevent_default();
                add();
            }>
                <label for="fb-note">{words::fb_add_note(lang)}</label>
                <textarea id="fb-note" rows="3" aria-describedby="fb-note-hint"
                    aria-invalid=move || (left() < 0).then_some("true")
                    prop:value=move || r.note.get() on:input=move |ev| r.note.set(event_target_value(&ev))
                    on:keydown=move |ev: leptos::ev::KeyboardEvent| if ev.key() == "Enter" && (ev.ctrl_key() || ev.meta_key()) {
                        ev.prevent_default();
                        add();
                    }></textarea>
                <div class="fb-note-foot">
                    <p class="hint" id="fb-note-hint">
                        {words::fb_note_hint(lang)}" "
                        <span class:fb-near=move || left() < 200>{move || words::fb_chars_left(lang, &left().to_string())}</span>
                    </p>
                    <button type="submit" class="button button-ghost button-sm" aria-disabled=move || r.busy.get().then_some("true")>
                        {words::fb_save_note(lang)}
                    </button>
                </div>
            </form>
        </section>
    }
}

fn related_view(r: &Rep) -> impl IntoView + use<> {
    let r = *r;
    let lang = r.c.lang;
    let body = move || {
        r.with(|d| {
            let s = &d.summary;
            let code = r.code();
            let path = |q: ListQuery| list_path(code, &q);
            let crash = s.signature.clone().map(|signature| {
                let href = path(ListQuery {
                    signature: Some(signature),
                    ..ListQuery::default()
                });
                view! { <li><a href=href>{words::fb_same_crash(lang)}</a></li> }
            });
            let duplicates = s.duplicate_of.is_none().then(|| {
                let href = path(ListQuery {
                    duplicate_of: Some(s.id),
                    ..ListQuery::default()
                });
                view! { <li><a href=href>{words::fb_its_duplicates(lang)}</a></li> }
            });
            let reporter = path(ListQuery {
                reporter: Some(s.reporter.clone()),
                ..ListQuery::default()
            });
            let kind = path(ListQuery {
                kinds: vec![s.kind],
                ..ListQuery::default()
            });
            view! {
                <ul class="fb-related">
                    {crash}
                    {duplicates}
                    <li><a href=reporter>{words::fb_same_reporter(lang)}</a></li>
                    <li><a href=kind>{words::fb_same_kind(lang, kind_word(lang, s.kind))}</a></li>
                </ul>
            }
        })
    };
    section("fb-related", words::fb_related(lang), body)
}

fn bin_view(r: &Rep) -> impl IntoView + use<> {
    let r = *r;
    let lang = r.c.lang;
    let days = words::number(lang, u64::from(TRASH_DAYS));
    move || {
        let trashed = r.with(|d| d.summary.deleted_at.is_some()).unwrap_or(true);
        (!trashed).then(|| {
            section(
                "fb-bin",
                words::fb_trash(lang),
                view! {
                    <p class="meta">{words::fb_trash_lead(lang, &days)}</p>
                    <p>
                        <button type="button" class="button button-danger" id="fb-to-trash"
                            aria-disabled=move || r.busy.get().then_some("true") on:click=move |_| r.trash()>
                            {glyph::glyph(TRASH)}<span>{words::fb_to_trash(lang)}</span>
                        </button>
                    </p>
                },
            )
        })
    }
}

fn head_view(r: &Rep) -> impl IntoView + use<> {
    let r = *r;
    let lang = r.c.lang;
    move || {
        r.with(|d| {
            let s = &d.summary;
            let line = first_line(&d.text, 120);
            let heading = if line.is_empty() {
                words::fb_no_text(lang).to_owned()
            } else {
                line
            };
            let notes = (s.notes > 0).then(|| {
                view! { <span class="pill">{words::fb_notes_n(lang, &words::number(lang, u64::from(s.notes)))}</span> }
            });
            let address = format!("{}{}", origin(), report_path(r.code(), &s.id, &ListQuery::default()));
            let trashed = s.deleted_at.map(|at| {
                let days = words::number(lang, u64::from(TRASH_DAYS));
                view! {
                    <div class="card fb-trashed" role="note">
                        <p>{words::fb_in_trash_since(lang, &when(lang, at), &days)}</p>
                        <p class="actions">
                            <button type="button" class="button" id="fb-restore" on:click=move |_| r.restore()>{words::fb_restore(lang)}</button>
                            <button type="button" class="button button-danger" id="fb-purge"
                                on:click=move |_| r.purge.open(vec![r.id])>{glyph::glyph(TRASH)}<span>{words::fb_purge(lang)}</span></button>
                        </p>
                    </div>
                }
            });
            view! {
                <header class="fb-report-head">
                    <p class="fb-report-pills">
                        {kind_pill(lang, s.kind)}{status_pill(lang, s.status)}{priority_pill(lang, s.priority)}{notes}
                    </p>
                    <h2 id="fb-report-title" class="fb-report-title" dir="auto" tabindex="-1">{heading}</h2>
                    <p class="meta fb-copyable">
                        <span><code class="fb-code">{format!("…{}", short_id(&s.id))}</code>" · "{when(lang, s.created_at)}</span>
                        <button type="button" class="button button-ghost button-sm" on:click=move |_| r.copy(address.clone())>
                            {glyph::glyph(glyph::COPY)}<span>{words::fb_copy_link(lang)}</span>
                        </button>
                    </p>
                </header>
                {trashed}
            }
        })
    }
}

fn article(r: &Rep) -> AnyView {
    let r = *r;
    view! {
        <article class="fb-report" aria-labelledby="fb-report-title">
            {head_view(&r)}
            <p class="error" role="alert" id="fb-report-error" tabindex="-1">{move || r.error.get()}</p>
            {move || r.purge.view()}
            <div class="fb-report-grid">
                <div class="fb-report-col">
                    {wrote_view(&r)}
                    {sent_view(&r)}
                    {facts_view(&r)}
                    {history_view(&r)}
                </div>
                <div class="fb-report-col">
                    {triage_view(&r)}
                    {notes_view(&r)}
                    {related_view(&r)}
                    {bin_view(&r)}
                </div>
            </div>
        </article>
    }
    .into_any()
}

/// Deleting for good ended: gone, the way back is to the trash; else
/// said, and the report read again.
fn purged(r: &Rep) {
    let r = *r;
    let lang = r.c.lang;
    let Some(how) = r.purge.ended.get() else {
        return;
    };
    r.purge.ended.set(None);
    match how {
        Purged::Done(_) => {
            r.c.notices.say(words::fb_purged_one(lang));
            let q = ListQuery {
                trash: true,
                ..r.query.get_value()
            };
            session::go(&list_path(lang.code(), &q));
        }
        Purged::Refused | Purged::Lost => {
            r.error.set(Some(if how == Purged::Refused {
                words::fb_purge_refused(lang)
            } else {
                words::fb_purge_lost(lang)
            }));
            r.read(false);
            focus::after_render("#fb-report-error");
        }
    }
}

/// The page for an administrator.
pub fn page(c: Ctx) -> AnyView {
    let lang = c.lang;
    let location = leptos::web_sys::window().map(|w| w.location());
    let path = location
        .as_ref()
        .and_then(|l| l.pathname().ok())
        .unwrap_or_default();
    let search = location
        .as_ref()
        .and_then(|l| l.search().ok())
        .unwrap_or_default();
    let overview = format!("/{}/account/admin/overview", lang.code());
    let back = view! {
        <p class="fb-back"><a href=overview class="text-link">{glyph::glyph(glyph::BACK)}<span>{words::fb_to_overview(lang)}</span></a></p>
    };
    let Some(id) = query::report_at(&path) else {
        // an address no report has: nothing is asked
        return view! {
            {back}
            <p class="error" role="alert">{words::fb_err_report(lang)}</p>
            <p><a class="button" href=list_path(lang.code(), &ListQuery::default())>{words::fb_back_to_list(lang)}</a></p>
        }
        .into_any();
    };
    let r = Rep {
        c,
        id,
        query: StoredValue::new(query::without_page(&query::read(&search))),
        load: RwSignal::new(Load::Loading),
        busy: RwSignal::new(false),
        error: RwSignal::new(None),
        near: RwSignal::new(None),
        client: RwSignal::new(Client::None),
        known: RwSignal::new(None),
        note: RwSignal::new(String::new()),
        label: RwSignal::new(String::new()),
        label_bad: RwSignal::new(false),
        dup: RwSignal::new(String::new()),
        dup_bad: RwSignal::new(false),
        issue: RwSignal::new(String::new()),
        issue_bad: RwSignal::new(false),
        summary: RwSignal::new(String::new()),
        category: RwSignal::new(None),
        recording: RwSignal::new(false),
        shot_full: RwSignal::new(false),
        purge: Purge::new(c, true, "#fb-purge"),
    };
    c.read_admins();
    // Effects run in the browser only: the report, opened, and its place.
    Effect::new(move |_| {
        r.read(true);
        r.read_near();
    });
    // deleting for good ends here: gone, the way back is to the trash
    Effect::new(move |_| purged(&r));
    let ready = Memo::new(move |_| matches!(r.load.get(), Load::Ready(_)));
    let body = move || {
        if ready.get() {
            return article(&r);
        }
        match r.load.get() {
            Load::Failed(text) => view! {
                <p class="error" role="alert">{text}</p>
                <p><button type="button" class="button" on:click=move |_| {
                    r.load.set(Load::Loading);
                    r.read(true);
                }>{words::error_again(lang)}</button></p>
            }
            .into_any(),
            _ => view! { <p class="meta" role="status">{words::account_loading(lang)}</p> }
                .into_any(),
        }
    };
    view! {
        {back}
        <div class="fb-page">
            {steps_view(&r)}
            {body}
        </div>
        {notices_view(lang, c.notices)}
    }
    .into_any()
}
