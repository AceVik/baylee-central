//! Owner gateway and token manager: searchable, filterable, sortable,
//! scrollable compact list with incremental loading, live gateway statistics,
//! deeplink, inline rename, soft-delete, and token mint/revoke.

use std::fmt::Write as _;

use central_api::gateway::GatewayState;
use central_api::owner::{GatewayScopes, MyGateway, OwnerAnswer, OwnerRequest};
use central_api::wire::{Request, Response};
use leptos::prelude::*;

use crate::words::{self, Lang};

use super::account::{Problem, problem_text, reauth};
use super::central::{Failure, call};
use super::chat::glyph::{self, glyph};
use super::clip;
use super::time;

const PAGE_STEP: usize = 12;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Filter {
    All,
    Online,
    Offline,
    Ranked,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SortBy {
    Name,
    Players,
    Status,
    Created,
}

fn gateway_key(id: &[u8; 16]) -> String {
    id.iter().fold(String::with_capacity(32), |mut out, byte| {
        let _ = write!(out, "{byte:02x}");
        out
    })
}

/// The client's link that proposes `origin` (Baylee `docs/client.md`, "Links
/// from the website"): `baylee://gateway/add?origin=` and the origin,
/// percent-encoded. The client drops any other spelling without a word,
/// `baylee://gateway?origin=` among them.
fn deeplink_for(origin: &str) -> String {
    let encoded: String = origin
        .bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
                char::from(b).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect();
    format!("baylee://gateway/add?origin={encoded}")
}

#[derive(Clone, Copy)]
struct GatewayActions {
    revoke: Callback<([u8; 16], [u8; 16])>,
    mint: Callback<([u8; 16], String, u16, GatewayScopes)>,
    rename: Callback<([u8; 16], String)>,
    delete: Callback<[u8; 16]>,
}

fn filter_and_sort_gateways<'a>(
    all: &'a [MyGateway],
    query: &str,
    mode: Filter,
    order: SortBy,
) -> Vec<&'a MyGateway> {
    let mut filtered: Vec<&MyGateway> = all
        .iter()
        .filter(|gw| {
            let ok_filter = match mode {
                Filter::All => true,
                Filter::Online => gw.online,
                Filter::Offline => !gw.online,
                Filter::Ranked => gw.ranked_verified,
            };
            let ok_query = query.is_empty()
                || gw.metadata.name.to_lowercase().contains(query)
                || gw.origin.to_lowercase().contains(query);
            ok_filter && ok_query
        })
        .collect();

    filtered.sort_by(|a, b| match order {
        SortBy::Name => a
            .metadata
            .name
            .to_lowercase()
            .cmp(&b.metadata.name.to_lowercase()),
        SortBy::Players => {
            let pa = a.activity.as_ref().map_or(0, |act| act.players);
            let pb = b.activity.as_ref().map_or(0, |act| act.players);
            pb.cmp(&pa).then_with(|| b.online.cmp(&a.online))
        }
        SortBy::Status => b
            .online
            .cmp(&a.online)
            .then_with(|| a.metadata.name.cmp(&b.metadata.name)),
        SortBy::Created => b.created_at.cmp(&a.created_at),
    });
    filtered
}

fn gateway_empty_state(lang: Lang, show_add: RwSignal<bool>) -> AnyView {
    view! {
        <div class="gw-empty">
            <span class="gw-empty-icon">{glyph(glyph::SERVER)}</span>
            <h3>{words::gw_empty_title(lang)}</h3>
            <p class="meta">{words::gw_empty_lead(lang)}</p>
            <button
                type="button"
                class="button primary"
                on:click=move |_| show_add.set(true)
            >
                {glyph(glyph::PLUS)}
                <span>{words::gateways_add(lang)}</span>
            </button>
        </div>
    }
    .into_any()
}

type GatewayControls = (
    RwSignal<String>,
    RwSignal<Filter>,
    RwSignal<SortBy>,
    RwSignal<usize>,
    RwSignal<bool>,
);

fn gateway_list_view(
    lang: Lang,
    all: &[MyGateway],
    controls: GatewayControls,
    actions: GatewayActions,
) -> AnyView {
    let (search, filter, sort_by, visible_limit, show_add) = controls;
    if all.is_empty() {
        return gateway_empty_state(lang, show_add);
    }
    let query = search.get().trim().to_lowercase();
    let filtered = filter_and_sort_gateways(all, &query, filter.get(), sort_by.get());
    let total = filtered.len();
    let limit = visible_limit.get();
    let has_more = total > limit;
    let shown_cards = filtered
        .into_iter()
        .take(limit)
        .map(|gw| gateway_row(lang, gw, actions))
        .collect_view();

    let on_scroll = move |ev: leptos::ev::Event| {
        #[cfg(target_arch = "wasm32")]
        if let Some(target) = ev
            .target()
            .and_then(|t| wasm_bindgen::JsCast::dyn_into::<web_sys::HtmlElement>(t).ok())
            && target.scroll_top() + target.client_height() + 96 >= target.scroll_height()
        {
            visible_limit.update(|n| *n = n.saturating_add(PAGE_STEP));
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = ev;
    };

    view! {
        <div>
            <div class="gw-scroll-list" on:scroll=on_scroll>
                {if total == 0 {
                    view! { <p class="meta">{words::gateways_none(lang)}</p> }.into_any()
                } else {
                    view! { <div class="stack">{shown_cards}</div> }.into_any()
                }}
            </div>
            {has_more.then(|| view! {
                <div class="gw-more-row">
                    <button
                        type="button"
                        class="button"
                        on:click=move |_| visible_limit.update(|n| *n = n.saturating_add(PAGE_STEP))
                    >
                        {glyph(glyph::REFRESH_CW)}
                        <span>{words::gw_load_more(lang)}</span>
                    </button>
                </div>
            })}
        </div>
    }
    .into_any()
}

fn gateways_toolbar(
    lang: Lang,
    search: RwSignal<String>,
    filter: RwSignal<Filter>,
    sort_by: RwSignal<SortBy>,
    visible_limit: RwSignal<usize>,
    show_add: RwSignal<bool>,
) -> AnyView {
    let filter_btn = move |mode: Filter, label: &'static str| {
        let active = move || filter.get() == mode;
        view! {
            <button
                type="button"
                class=move || if active() { "gw-chip is-active" } else { "gw-chip" }
                aria-pressed=move || if active() { "true" } else { "false" }
                on:click=move |_| {
                    filter.set(mode);
                    visible_limit.set(PAGE_STEP);
                }
            >
                {label}
            </button>
        }
    };
    view! {
        <div>
            <div class="gw-toolbar">
                <div class="inset gw-search-box">
                    <span class="inset-mark">{glyph(glyph::SEARCH)}</span>
                    <input
                        id="gw-owner-search"
                        type="search"
                        autocomplete="off"
                        spellcheck="false"
                        placeholder=words::gw_search_placeholder(lang)
                        prop:value=move || search.get()
                        on:input=move |ev| {
                            search.set(event_target_value(&ev));
                            visible_limit.set(PAGE_STEP);
                        }
                    />
                </div>
                <div class="gw-sort-box">
                    <span class="gw-sort-icon">{glyph(glyph::ARROW_UP_DOWN)}</span>
                    <select
                        aria-label=words::gw_sort(lang)
                        on:change=move |ev| {
                            sort_by.set(match event_target_value(&ev).as_str() {
                                "players" => SortBy::Players,
                                "name" => SortBy::Name,
                                "created" => SortBy::Created,
                                _ => SortBy::Status,
                            });
                        }
                    >
                        <option value="status" selected=true>{words::gw_sort_status(lang)}</option>
                        <option value="players">{words::gw_sort_players(lang)}</option>
                        <option value="name">{words::gw_sort_name(lang)}</option>
                        <option value="created">{words::gw_sort_created(lang)}</option>
                    </select>
                </div>
                <button
                    type="button"
                    class="button primary"
                    aria-expanded=move || if show_add.get() { "true" } else { "false" }
                    on:click=move |_| show_add.update(|s| *s = !*s)
                >
                    {glyph(glyph::PLUS)}
                    <span>{words::gateways_add(lang)}</span>
                </button>
            </div>
            <div class="gw-filter-row" role="group">
                {filter_btn(Filter::All, words::gw_filter_all(lang))}
                {filter_btn(Filter::Online, words::gw_filter_online(lang))}
                {filter_btn(Filter::Offline, words::gw_filter_offline(lang))}
                {filter_btn(Filter::Ranked, words::gw_filter_ranked(lang))}
            </div>
        </div>
    }
    .into_any()
}

fn build_gateway_actions(
    lang: Lang,
    notice: RwSignal<Option<&'static str>>,
    shown: RwSignal<Option<String>>,
    needs_reauth: RwSignal<bool>,
    minting: StoredValue<Option<[u8; 16]>>,
    reload_focus: Callback<Option<String>>,
) -> GatewayActions {
    let revoke = Callback::new(move |(gateway, token): ([u8; 16], [u8; 16])| {
        leptos::task::spawn_local(async move {
            match call(&Request::Owner {
                request: OwnerRequest::RevokeToken { token },
            })
            .await
            {
                Ok(_) => {
                    notice.set(Some(words::tokens_revoked_now(lang)));
                    reload_focus.run(Some(format!("#token-{}-title", gateway_key(&gateway))));
                }
                Err(failure) => notice.set(Some(problem_text(lang, failure.into()))),
            }
        });
    });

    let mint = Callback::new(
        move |(gateway, name, days, scopes): ([u8; 16], String, u16, GatewayScopes)| {
            if central_api::owner::token_name(&name).is_err() || scopes.validate().is_err() {
                notice.set(Some(words::tokens_invalid(lang)));
                return;
            }
            minting.set_value(Some(gateway));
            leptos::task::spawn_local(async move {
                match call(&Request::Owner {
                    request: OwnerRequest::CreateToken {
                        gateway,
                        name,
                        scopes,
                        days,
                    },
                })
                .await
                {
                    Ok(Response::Owner {
                        answer: OwnerAnswer::TokenCreated { token, .. },
                    }) => {
                        shown.set(Some(
                            central_api::owner::token_text(&token).as_str().to_owned(),
                        ));
                        needs_reauth.set(false);
                        notice.set(None);
                        reload_focus.run(Some("#token-shown".to_owned()));
                    }
                    Ok(_) => notice.set(Some(problem_text(lang, Problem::Failed))),
                    Err(Failure::Rejected | Failure::Forbidden) => needs_reauth.set(true),
                    Err(failure) => notice.set(Some(problem_text(lang, failure.into()))),
                }
            });
        },
    );

    let rename = Callback::new(move |(gateway, name): ([u8; 16], String)| {
        let trimmed = name.trim().to_owned();
        if trimmed.is_empty() {
            notice.set(Some(words::gateways_invalid(lang)));
            return;
        }
        leptos::task::spawn_local(async move {
            match super::central::update_gateway(gateway, trimmed).await {
                Ok(()) => {
                    notice.set(None);
                    reload_focus.run(None);
                }
                Err(Failure::Rejected | Failure::Forbidden) => needs_reauth.set(true),
                Err(failure) => notice.set(Some(problem_text(lang, failure.into()))),
            }
        });
    });

    let delete = Callback::new(move |gateway: [u8; 16]| {
        leptos::task::spawn_local(async move {
            match super::central::delete_gateway(gateway).await {
                Ok(()) => {
                    notice.set(None);
                    reload_focus.run(None);
                }
                Err(Failure::Rejected | Failure::Forbidden) => needs_reauth.set(true),
                Err(failure) => notice.set(Some(problem_text(lang, failure.into()))),
            }
        });
    });

    GatewayActions {
        revoke,
        mint,
        rename,
        delete,
    }
}

/// Owner gateway & token console rendered on `/{lang}/account/gateways`.
pub fn view(lang: Lang, worker: String) -> AnyView {
    let list = RwSignal::new(None::<Result<Vec<MyGateway>, Problem>>);
    let notice = RwSignal::new(None::<&'static str>);
    let shown = RwSignal::new(None::<String>);
    let needs_reauth = RwSignal::new(false);
    let show_add = RwSignal::new(false);
    let search = RwSignal::new(String::new());
    let filter = RwSignal::new(Filter::All);
    let sort_by = RwSignal::new(SortBy::Status);
    let visible_limit = RwSignal::new(PAGE_STEP);
    let minting = StoredValue::new(None::<[u8; 16]>);

    let load = Callback::new(move |then: Option<String>| {
        leptos::task::spawn_local(async move {
            let found = match call(&Request::Owner {
                request: OwnerRequest::Gateways,
            })
            .await
            {
                Ok(Response::Owner {
                    answer: OwnerAnswer::Gateways { gateways },
                }) => Ok(gateways),
                Ok(_) => Err(Problem::Failed),
                Err(failure) => Err(failure.into()),
            };
            list.set(Some(found));
            if let Some(then) = then {
                super::focus::after_render(then);
            }
        });
    });
    Effect::new(move |_| load.run(None));

    let actions = build_gateway_actions(lang, notice, shown, needs_reauth, minting, load);
    let after_reauth = Callback::new(move |()| {
        needs_reauth.set(false);
        notice.set(Some(words::tokens_again(lang)));
    });
    let reload = Callback::new(move |()| {
        show_add.set(false);
        load.run(None);
    });
    let body = move || {
        list.with(|current| match current.as_ref() {
            None => view! { <p class="meta">{words::account_loading(lang)}</p> }.into_any(),
            Some(Err(problem)) => {
                view! { <p class="error" role="alert">{problem_text(lang, *problem)}</p> }
                    .into_any()
            }
            Some(Ok(all)) => gateway_list_view(
                lang,
                all,
                (search, filter, sort_by, visible_limit, show_add),
                actions,
            ),
        })
    };

    view! {
        <section class="card">
            {gateways_toolbar(lang, search, filter, sort_by, visible_limit, show_add)}
            {move || show_add.get().then(|| add_panel(lang, notice, reload))}
            {move || notice.get().map(|text| view! { <p class="meta" role="status">{text}</p> })}
            {token_box(lang, shown, minting)}
            {move || needs_reauth.get().then(|| reauth(lang, worker.clone(), after_reauth))}
            {body}
        </section>
    }
    .into_any()
}

fn gateway_title_control(
    lang: Lang,
    id: [u8; 16],
    title_id: String,
    gw_name: String,
    editing_name: RwSignal<bool>,
    name_draft: RwSignal<String>,
    rename_gw: Callback<([u8; 16], String)>,
) -> impl IntoView {
    move || {
        if editing_name.get() {
            view! {
                <form
                    class="gw-rename-form"
                    on:submit=move |ev: leptos::ev::SubmitEvent| {
                        ev.prevent_default();
                        rename_gw.run((id, name_draft.get_untracked()));
                        editing_name.set(false);
                    }
                >
                    <div class="inset">
                        <span class="inset-mark">{glyph(glyph::SERVER)}</span>
                        <input
                            type="text"
                            maxlength="80"
                            aria-label=words::gateways_name(lang)
                            prop:value=move || name_draft.get()
                            on:input=move |ev| name_draft.set(event_target_value(&ev))
                        />
                    </div>
                    <button
                        type="submit"
                        class="button primary"
                        title=words::gw_rename_save(lang)
                        aria-label=words::gw_rename_save(lang)
                    >
                        {glyph(glyph::CHECK)}
                    </button>
                    <button
                        type="button"
                        class="button quiet"
                        title=words::picture_cancel(lang)
                        aria-label=words::picture_cancel(lang)
                        on:click=move |_| editing_name.set(false)
                    >
                        {glyph(glyph::CLOSE)}
                    </button>
                </form>
            }
            .into_any()
        } else {
            view! {
                <div class="gw-name-row">
                    <h3 id=title_id.clone() tabindex="-1"><bdi>{gw_name.clone()}</bdi></h3>
                    <button
                        type="button"
                        class="icon-btn"
                        title=words::gw_rename(lang)
                        aria-label=words::gw_rename(lang)
                        on:click=move |_| editing_name.set(true)
                    >
                        {glyph(glyph::PENCIL)}
                    </button>
                </div>
            }
            .into_any()
        }
    }
}

fn gateway_actions_col(
    lang: Lang,
    id: [u8; 16],
    deeplink: String,
    confirm_delete: RwSignal<bool>,
    delete_gw: Callback<[u8; 16]>,
) -> impl IntoView {
    view! {
        <div class="gw-actions-col">
            <a class="button" href=deeplink title=words::gw_deeplink(lang)>
                {glyph(glyph::EXTERNAL_LINK)}
                <span>{words::gw_deeplink(lang)}</span>
            </a>
            {move || if confirm_delete.get() {
                view! {
                    <div class="gw-delete-confirm">
                        <button
                            type="button"
                            class="button danger"
                            on:click=move |_| delete_gw.run(id)
                        >
                            {glyph(glyph::TRASH_2)}
                            <span>{words::gw_delete_confirm(lang)}</span>
                        </button>
                        <button
                            type="button"
                            class="button quiet"
                            title=words::picture_cancel(lang)
                            aria-label=words::picture_cancel(lang)
                            on:click=move |_| confirm_delete.set(false)
                        >
                            {glyph(glyph::CLOSE)}
                        </button>
                    </div>
                }
                .into_any()
            } else {
                view! {
                    <button
                        type="button"
                        class="button danger"
                        title=words::gw_delete(lang)
                        aria-label=words::gw_delete(lang)
                        on:click=move |_| confirm_delete.set(true)
                    >
                        {glyph(glyph::TRASH_2)}
                        <span>{words::gw_delete(lang)}</span>
                    </button>
                }
                .into_any()
            }}
        </div>
    }
}

fn token_scope_badges(lang: Lang, scopes: GatewayScopes) -> impl IntoView {
    view! {
        <div class="gw-token-scopes">
            {scopes.sso.then(|| view! {
                <span class="pill info">
                    {glyph(glyph::ID_CARD)}
                    <span>{words::tokens_sso(lang)}</span>
                </span>
            })}
            {scopes.listing.then(|| view! {
                <span class="pill">
                    {glyph(glyph::GLOBE)}
                    <span>{words::tokens_listing(lang)}</span>
                </span>
            })}
            {scopes.ranked.then(|| view! {
                <span class="pill gold">
                    {glyph(glyph::SHIELD_CHECK)}
                    <span>{words::tokens_ranked(lang)}</span>
                </span>
            })}
        </div>
    }
}

fn gateway_token_items(
    lang: Lang,
    id: [u8; 16],
    tokens: &[central_api::owner::TokenInfo],
    revoke: Callback<([u8; 16], [u8; 16])>,
) -> AnyView {
    tokens
        .iter()
        .map(|token| {
            let tid = token.id;
            let live = token.revoked_at.is_none();
            let until = time::format_datetime_local(lang, token.expires_at);
            let action = live.then(|| {
                view! {
                    <div class="gw-token-actions">
                        <button
                            type="button"
                            class="button danger"
                            on:click=move |_| revoke.run((id, tid))
                        >
                            {glyph(glyph::TRASH_2)}
                            <span>{words::tokens_revoke(lang)}</span>
                        </button>
                    </div>
                }
            });
            let meta_text = if live {
                words::tokens_until(lang, &until)
            } else {
                words::tokens_revoked(lang).to_owned()
            };
            view! {
                <li class="token gw-token-row" data-live=if live { "true" } else { "false" }>
                    <div class="gw-token-main">
                        <div class="gw-token-head">
                            <strong class="gw-token-name"><bdi>{token.name.clone()}</bdi></strong>
                            {(!live).then(|| view! { <span class="pill off">{words::tokens_revoked(lang)}</span> })}
                        </div>
                        {token_scope_badges(lang, token.scopes)}
                        <p class="meta gw-token-meta">{meta_text}</p>
                    </div>
                    {action}
                </li>
            }
        })
        .collect_view()
        .into_any()
}

fn token_mint_form(
    lang: Lang,
    id: [u8; 16],
    form_id: &str,
    show_mint: RwSignal<bool>,
    mint: Callback<([u8; 16], String, u16, GatewayScopes)>,
) -> AnyView {
    let name = RwSignal::new(String::new());
    let days = RwSignal::new(90_u16);
    let sso = RwSignal::new(true);
    let listing = RwSignal::new(true);
    let ranked = RwSignal::new(false);
    let make = move |event: leptos::ev::SubmitEvent| {
        event.prevent_default();
        mint.run((
            id,
            name.get_untracked(),
            days.get_untracked(),
            GatewayScopes {
                sso: sso.get_untracked(),
                listing: listing.get_untracked(),
                ranked: ranked.get_untracked(),
            },
        ));
    };
    let field_name = format!("{form_id}-name");
    let field_days = format!("{form_id}-days");
    let field_sso = format!("{form_id}-sso");
    let field_listing = format!("{form_id}-listing");
    let field_ranked = format!("{form_id}-ranked");
    view! {
        <div class="gw-mint-panel" hidden=move || !show_mint.get()>
            <form class="form gw-mint-form" method="post" novalidate=true on:submit=make>
                <div class="gw-mint-grid">
                    <div class="field">
                        <label for=field_name.clone()>{words::tokens_name(lang)}</label>
                        <div class="inset">
                            <span class="inset-mark">{glyph(glyph::KEY)}</span>
                            <input
                                id=field_name
                                type="text"
                                maxlength="64"
                                prop:value=move || name.get()
                                on:input=move |ev| name.set(event_target_value(&ev))
                            />
                        </div>
                    </div>
                    <div class="field">
                        <label for=field_days.clone()>{words::tokens_days(lang)}</label>
                        <select id=field_days on:change=move |ev| days.set(event_target_value(&ev).parse().unwrap_or(90))>
                            <option value="30">{"30"}</option>
                            <option value="90" selected=true>{"90"}</option>
                            <option value="180">{"180"}</option>
                            <option value="365">{"365"}</option>
                        </select>
                    </div>
                </div>
                <div class="gw-mint-scopes">
                    <div class="check">
                        <input id=field_sso.clone() type="checkbox" checked=true on:change=move |ev| sso.set(event_target_checked(&ev)) />
                        <label for=field_sso>{words::tokens_sso(lang)}</label>
                    </div>
                    <div class="check">
                        <input id=field_listing.clone() type="checkbox" checked=true on:change=move |ev| listing.set(event_target_checked(&ev)) />
                        <label for=field_listing>{words::tokens_listing(lang)}</label>
                    </div>
                    <div class="check">
                        <input id=field_ranked.clone() type="checkbox" on:change=move |ev| ranked.set(event_target_checked(&ev)) />
                        <label for=field_ranked>{words::tokens_ranked(lang)}</label>
                    </div>
                </div>
                <div class="button-row">
                    <button type="submit" class="button primary">
                        {glyph(glyph::PLUS)}
                        <span>{words::tokens_make(lang)}</span>
                    </button>
                </div>
            </form>
        </div>
    }
    .into_any()
}

fn gateway_tokens_panel(
    lang: Lang,
    gateway: &MyGateway,
    form_id: &str,
    show_tokens: RwSignal<bool>,
    actions: GatewayActions,
) -> AnyView {
    let id = gateway.id;
    let show_mint = RwSignal::new(false);
    let tokens = (!gateway.tokens.is_empty())
        .then(|| gateway_token_items(lang, id, &gateway.tokens, actions.revoke));
    view! {
        <div class="gw-tokens-panel" hidden=move || !show_tokens.get()>
            {tokens.map(|items| view! { <ul class="tokens gw-tokens-list">{items}</ul> })}
            <div class="gw-mint-bar">
                <button
                    type="button"
                    class="button subtle gw-mint-toggle"
                    aria-expanded=move || if show_mint.get() { "true" } else { "false" }
                    on:click=move |_| show_mint.update(|open| *open = !*open)
                >
                    {move || if show_mint.get() { glyph(glyph::CLOSE) } else { glyph(glyph::PLUS) }}
                    <span>{words::tokens_make(lang)}</span>
                </button>
            </div>
            {token_mint_form(lang, id, form_id, show_mint, actions.mint)}
        </div>
    }
    .into_any()
}

fn gw_stat_item(
    lang: Lang,
    shape: &'static [glyph::Shape],
    label: &'static str,
    value: u32,
) -> impl IntoView {
    view! {
        <div class="gw-stat-cell" class:is-none={value == 0}>
            <dt>{label}</dt>
            <dd>
                {glyph(shape)}
                <span class="num" translate="no">{words::number(lang, u64::from(value))}</span>
            </dd>
        </div>
    }
}

fn gateway_status_pills(lang: Lang, gateway: &MyGateway) -> impl IntoView {
    let (state_class, state) = match gateway.state {
        GatewayState::Approved => ("pill ok", words::gateways_approved(lang)),
        GatewayState::Pending => ("pill warn", words::gateways_pending(lang)),
        GatewayState::Disabled => ("pill bad", words::gateways_disabled(lang)),
    };
    let reach_pill = if gateway.online {
        Some(
            view! {
                <span class="pill ok">
                    <span class="dot ok live" aria-hidden="true">{""}</span>
                    <span>{words::gws_online(lang)}</span>
                </span>
            }
            .into_any(),
        )
    } else if matches!(gateway.state, GatewayState::Approved) {
        Some(
            view! {
                <span class="pill off">
                    <span class="dot off" aria-hidden="true">{""}</span>
                    <span>{words::gws_offline(lang)}</span>
                </span>
            }
            .into_any(),
        )
    } else {
        None
    };
    let version = (!gateway.metadata.version.is_empty()).then(|| gateway.metadata.version.clone());
    view! {
        <p class="pills">
            <span class=state_class>{state}</span>
            {reach_pill}
            {gateway.ranked_verified.then(|| view! {
                <span class="pill gold">
                    {glyph(glyph::SHIELD_CHECK)}
                    <span>{words::gw_ranked_verified(lang)}</span>
                </span>
            })}
            {version.map(|v| view! { <span class="pill gw-version-pill" translate="no">{v}</span> })}
        </p>
    }
}

fn gateway_row(lang: Lang, gateway: &MyGateway, actions: GatewayActions) -> AnyView {
    let id = gateway.id;
    let form_id = format!("token-{}", gateway_key(&id));
    let state_attr = match gateway.state {
        GatewayState::Approved => "approved",
        GatewayState::Pending => "pending",
        GatewayState::Disabled => "disabled",
    };
    let editing_name = RwSignal::new(false);
    let name_draft = RwSignal::new(gateway.metadata.name.clone());
    let confirm_delete = RwSignal::new(false);
    let show_tokens = RwSignal::new(false);
    let token_count = gateway
        .tokens
        .iter()
        .filter(|t| t.revoked_at.is_none())
        .count();

    let stats_line = gateway.activity.as_ref().map(|act| {
        view! {
            <dl class="gw-stats-grid">
                {gw_stat_item(lang, glyph::USERS, words::gw_live_online(lang), act.players)}
                {gw_stat_item(lang, glyph::CROWN, words::gw_live_playing(lang), act.playing)}
                {gw_stat_item(lang, glyph::LAYERS, words::gw_live_games(lang), act.games)}
            </dl>
        }
    });
    let last_seen_line = gateway.last_seen.map(|ts| {
        view! {
            <span class="meta gw-seen" title=words::admin_gw_seen(lang)>
                {glyph(glyph::CALENDAR)}
                <span>{time::format_datetime_local(lang, ts)}</span>
            </span>
        }
    });

    view! {
        <article
            class="card gateway-own gw-compact"
            data-state=state_attr
            data-online=if gateway.online { "true" } else { "false" }
        >
            <div class="gw-head">
                <div class="gw-title-col">
                    {gateway_title_control(lang, id, format!("{form_id}-title"), gateway.metadata.name.clone(), editing_name, name_draft, actions.rename)}
                    {gateway_status_pills(lang, gateway)}
                    <p class="gw-origin-line">
                        <code class="address" translate="no">{gateway.origin.clone()}</code>
                        {last_seen_line}
                    </p>
                </div>
                {gateway_actions_col(lang, id, deeplink_for(&gateway.origin), confirm_delete, actions.delete)}
            </div>
            {stats_line}
            <div class="gw-tokens-toggle">
                <button
                    type="button"
                    class="button subtle"
                    aria-expanded=move || if show_tokens.get() { "true" } else { "false" }
                    on:click=move |_| show_tokens.update(|s| *s = !*s)
                >
                    {glyph(glyph::KEY)}
                    <span>{format!("{} ({token_count})", words::tokens_title(lang))}</span>
                </button>
            </div>
            {gateway_tokens_panel(lang, gateway, &form_id, show_tokens, actions)}
        </article>
    }
    .into_any()
}

fn add_panel(
    lang: Lang,
    notice: RwSignal<Option<&'static str>>,
    reload: Callback<()>,
) -> impl IntoView {
    let origin = RwSignal::new(String::new());
    let label = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let add = move |event: leptos::ev::SubmitEvent| {
        event.prevent_default();
        if busy.get_untracked() {
            return;
        }
        let (origin_value, name) = (
            origin.get_untracked().trim().to_owned(),
            label.get_untracked().trim().to_owned(),
        );
        if central_api::gateway::validate_origin(&origin_value, false).is_err() || name.is_empty() {
            notice.set(Some(words::gateways_invalid(lang)));
            return;
        }
        busy.set(true);
        leptos::task::spawn_local(async move {
            match call(&Request::Owner {
                request: OwnerRequest::AddGateway {
                    origin: origin_value,
                    name,
                },
            })
            .await
            {
                Ok(_) => {
                    notice.set(Some(words::gateways_added(lang)));
                    reload.run(());
                }
                Err(failure) => notice.set(Some(problem_text(lang, failure.into()))),
            }
            busy.set(false);
        });
    };

    view! {
        <form class="form gw-add-form" method="post" novalidate=true on:submit=add>
            <h3>{words::gateways_add_title(lang)}</h3>
            <div class="field">
                <label for="gateway-origin">{words::gateways_origin(lang)}</label>
                <div class="inset">
                    <span class="inset-mark">{glyph(glyph::GLOBE)}</span>
                    <input
                        id="gateway-origin"
                        type="url"
                        inputmode="url"
                        placeholder="https://"
                        aria-describedby="gateway-origin-hint"
                        on:input=move |ev| origin.set(event_target_value(&ev))
                    />
                </div>
                <p class="hint" id="gateway-origin-hint">{words::gateways_origin_hint(lang)}</p>
            </div>
            <div class="field">
                <label for="gateway-name">{words::gateways_name(lang)}</label>
                <div class="inset">
                    <span class="inset-mark">{glyph(glyph::SERVER)}</span>
                    <input
                        id="gateway-name"
                        type="text"
                        maxlength="80"
                        on:input=move |ev| label.set(event_target_value(&ev))
                    />
                </div>
            </div>
            <button type="submit" class="button primary" disabled=move || busy.get()>
                {glyph(glyph::PLUS)}
                <span>{words::gateways_add(lang)}</span>
            </button>
        </form>
    }
}

fn token_box(
    lang: Lang,
    shown: RwSignal<Option<String>>,
    minting: StoredValue<Option<[u8; 16]>>,
) -> impl IntoView {
    let copied = RwSignal::new(false);
    view! {
        <Show when=move || shown.with(Option::is_some)>
            <section class="card secret-box" role="alert" aria-labelledby="token-shown">
                <h2 id="token-shown" tabindex="-1">{words::tokens_shown_title(lang)}</h2>
                <p class="eyebrow">{words::code_shown_once(lang)}</p>
                <p class="secret" translate="no"><code>{move || shown.get().unwrap_or_default()}</code></p>
                <div class="button-row">
                    <button
                        type="button"
                        class="button"
                        on:click=move |_| {
                            if let Some(tok) = shown.get_untracked() {
                                leptos::task::spawn_local(async move {
                                    if clip::write(&tok).await {
                                        copied.set(true);
                                    }
                                });
                            }
                        }
                    >
                        {move || if copied.get() { glyph(glyph::CHECK) } else { glyph(glyph::COPY) }}
                        <span>{move || if copied.get() { words::handle_copied_label(lang) } else { words::code_copy(lang) }}</span>
                    </button>
                    <button
                        type="button"
                        class="button primary"
                        on:click=move |_| {
                            shown.set(None);
                            copied.set(false);
                            if let Some(gateway) = minting.get_value() {
                                super::focus::after_render(format!("#token-{}-title", gateway_key(&gateway)));
                            }
                        }
                    >
                        {glyph(glyph::CIRCLE_CHECK)}
                        <span>{words::tokens_kept(lang)}</span>
                    </button>
                </div>
            </section>
        </Show>
    }
}

#[cfg(test)]
mod tests {
    use super::deeplink_for;

    /// The client reads `baylee://gateway/add?origin=` and drops every
    /// other spelling without a word (Baylee `client-core::deeplink`).
    #[test]
    fn a_gateway_link_is_the_one_the_client_reads() {
        assert_eq!(
            deeplink_for("https://gateway.example.org"),
            "baylee://gateway/add?origin=https%3A%2F%2Fgateway.example.org"
        );
    }
}
