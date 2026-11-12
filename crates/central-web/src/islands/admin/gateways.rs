//! `/{lang}/account/admin/gateways`: every registered gateway, newest first,
//! 50 a page, by state (`AdminRequest::Gateways`), with its owner and, on
//! request, its tokens (`AdminRequest::Gateway`). An administrator approves
//! a gateway after checking outside Central who runs its address
//! (`docs/gateway-registry.md`), disables one (it leaves the list, its
//! credentials and tokens stop, its players' sign-ins there end), enables
//! one again, and revokes a token; each after its confirmation
//! (`common::Flow`). After a change the list is read again and the keyboard
//! goes on at the gateway in the acted one's place, so an administrator
//! works down the queue with Tab and Enter.
//!
//! The filter stands in the address's fragment (`#state=approved`, as the
//! overview links to it): it stays when the page is loaded again and never
//! reaches the site.

use central_api::admin::{AdminAnswer, AdminGateway, AdminOverview, AdminRequest};
use central_api::gateway::GatewayState;
use central_api::owner::TokenInfo;
use leptos::prelude::*;

use super::Listing;
use super::common::{self, Asked, Came, Change, Cx, Flow, Note};
use crate::data::id_hex;
use crate::islands::account::when;
use crate::islands::auth::{Problem, problem_text};
use crate::islands::{focus, fragment};
use crate::words::{self, Lang};

/// One registered gateway, as the page shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Gateway {
    id: [u8; 16],
    origin: String,
    name: String,
    official: bool,
    owner: Option<[u8; 16]>,
    owner_name: Option<String>,
    state: GatewayState,
    online: bool,
    last_seen: Option<i64>,
    created_at: i64,
    live_tokens: u32,
    credential_pair: bool,
}

impl From<AdminGateway> for Gateway {
    fn from(gateway: AdminGateway) -> Self {
        Gateway {
            id: gateway.id,
            origin: gateway.origin,
            name: gateway.metadata.name,
            official: gateway.official,
            owner: gateway.owner,
            owner_name: common::handle(gateway.owner_name.as_deref(), gateway.owner_tag),
            state: gateway.state,
            online: gateway.online,
            last_seen: gateway.last_seen,
            created_at: gateway.created_at,
            live_tokens: gateway.live_tokens,
            credential_pair: gateway.credential_pair,
        }
    }
}

/// One of a gateway's tokens: its name, what it allows and its times.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Token {
    id: [u8; 16],
    name: String,
    sso: bool,
    listing: bool,
    expires_at: i64,
    revoked_at: Option<i64>,
    last_used_at: Option<i64>,
}

impl From<TokenInfo> for Token {
    fn from(token: TokenInfo) -> Self {
        Token {
            id: token.id,
            name: token.name,
            sso: token.scopes.sso,
            listing: token.scopes.listing,
            expires_at: token.expires_at,
            revoked_at: token.revoked_at,
            last_used_at: token.last_used_at,
        }
    }
}

/// The filters in their order: awaiting approval first (the queue), then
/// approved, disabled, and every gateway.
const FILTERS: [Option<GatewayState>; 4] = [
    Some(GatewayState::Pending),
    Some(GatewayState::Approved),
    Some(GatewayState::Disabled),
    None,
];

/// The filter's part of the address.
fn fragment_of(filter: Option<GatewayState>) -> &'static str {
    match filter {
        Some(GatewayState::Pending) => "#state=pending",
        Some(GatewayState::Approved) => "#state=approved",
        Some(GatewayState::Disabled) => "#state=disabled",
        None => "#state=all",
    }
}

/// The filter the address's fragment names; the queue where it names none.
fn filter_in(hash: &str) -> Option<GatewayState> {
    FILTERS
        .into_iter()
        .find(|filter| fragment_of(*filter) == hash)
        .unwrap_or(Some(GatewayState::Pending))
}

/// How many gateways a filter holds, as the overview counts them.
fn count(counts: &AdminOverview, filter: Option<GatewayState>) -> u64 {
    match filter {
        Some(GatewayState::Pending) => counts.pending,
        Some(GatewayState::Approved) => counts.approved,
        Some(GatewayState::Disabled) => counts.disabled,
        None => counts.pending + counts.approved + counts.disabled,
    }
}

fn state_text(lang: Lang, state: GatewayState) -> (&'static str, &'static str) {
    match state {
        GatewayState::Pending => (words::gateways_pending(lang), "warn"),
        GatewayState::Approved => (words::gateways_approved(lang), "ok"),
        GatewayState::Disabled => (words::gateways_disabled(lang), "bad"),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Tokens {
    Loading,
    Ready(Vec<Token>),
    Failed(&'static str),
}

/// What a read of the list does once it is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Then {
    /// Nothing more.
    Nothing,
    /// "Show more": the keyboard goes on at the first new gateway.
    More,
    /// A change ended: the keyboard goes on at the gateway in this place,
    /// where it was made, else to the line that says what came of it.
    Ended(Came, Option<usize>),
}

/// The page's signals. All of it is `Copy`.
#[derive(Clone, Copy)]
struct Page {
    cx: Cx,
    filter: RwSignal<Option<GatewayState>>,
    items: RwSignal<Vec<Gateway>>,
    listing: RwSignal<Listing>,
    more: RwSignal<bool>,
    adding: RwSignal<bool>,
    /// The newest list request: an older one's answer is not drawn.
    generation: StoredValue<u32>,
    counts: RwSignal<Option<AdminOverview>>,
    /// The gateway whose tokens are shown.
    open: RwSignal<Option<[u8; 16]>>,
    tokens: RwSignal<Tokens>,
    note: RwSignal<Option<Note>>,
    flow: Flow,
}

impl Page {
    /// The filters' counts, from the overview; none where it cannot be read.
    fn recount(self) {
        leptos::task::spawn_local(async move {
            if let Ok(AdminAnswer::Overview { overview }) =
                common::read(AdminRequest::Overview).await
            {
                let _ = self.counts.try_set(Some(overview));
            }
        });
    }

    /// Reads a page of the filter's gateways: the first, or the one after
    /// `before`.
    fn read(self, before: Option<[u8; 16]>, then: Then) {
        let Some(generation) = self.generation.try_get_value() else {
            return;
        };
        let generation = if before.is_none() {
            generation.wrapping_add(1)
        } else {
            generation
        };
        self.generation.set_value(generation);
        let state = self.filter.get_untracked();
        let lang = self.cx.lang;
        leptos::task::spawn_local(async move {
            let answer = common::read(AdminRequest::Gateways { state, before }).await;
            if self.generation.try_get_value() != Some(generation) {
                return;
            }
            let read = match answer {
                Ok(AdminAnswer::Gateways { gateways, more }) => {
                    let found: Vec<Gateway> = gateways.into_iter().map(Gateway::from).collect();
                    let next = found.first().map(|gateway| id_hex(&gateway.id));
                    if before.is_some() {
                        self.items.update(|items| items.extend(found));
                    } else {
                        self.items.set(found);
                    }
                    self.more.set(more);
                    self.listing.set(Listing::Ready);
                    if then == Then::More
                        && let Some(key) = next
                    {
                        focus::after_render(format!("#admin-gw-{key}"));
                    }
                    true
                }
                Ok(_) => {
                    self.listing
                        .set(Listing::Failed(problem_text(lang, Problem::Failed)));
                    false
                }
                Err(unread) => {
                    let missing = problem_text(lang, Problem::Failed);
                    match self.cx.unread_text(unread, missing) {
                        Some(text) => self.listing.set(Listing::Failed(text)),
                        None => return,
                    }
                    false
                }
            };
            self.adding.set(false);
            if let Then::Ended(came, place) = then {
                if came == Came::Lost {
                    self.note.set(Some(common::lost(lang, read)));
                }
                match place {
                    Some(at) if came == Came::Done && read => {
                        focus::after_render_first(vec![
                            format!("#admin-gws > li:nth-child({}) .admin-gw-name", at + 1),
                            "#admin-gws > li:last-child .admin-gw-name".to_owned(),
                            "#admin-done".to_owned(),
                        ]);
                    }
                    _ => common::focus_note(self.note),
                }
            }
        });
    }

    /// Reads the tokens of `gateway`.
    fn read_tokens(self, gateway: [u8; 16]) {
        let lang = self.cx.lang;
        leptos::task::spawn_local(async move {
            let found = match common::read(AdminRequest::Gateway { gateway }).await {
                Ok(AdminAnswer::Gateway { tokens, .. }) => {
                    Tokens::Ready(tokens.into_iter().map(Token::from).collect())
                }
                Ok(_) => Tokens::Failed(problem_text(lang, Problem::Failed)),
                Err(unread) => match self
                    .cx
                    .unread_text(unread, words::admin_refused_gateway(lang))
                {
                    Some(text) => Tokens::Failed(text),
                    None => return,
                },
            };
            // only for the gateway still open
            if self.open.try_get_untracked().flatten() == Some(gateway) {
                self.tokens.set(found);
            }
        });
    }

    /// "Show tokens" and "Hide tokens".
    fn toggle(self, gateway: [u8; 16]) {
        if self.open.get_untracked() == Some(gateway) {
            self.open.set(None);
        } else {
            self.open.set(Some(gateway));
            self.tokens.set(Tokens::Loading);
            self.read_tokens(gateway);
        }
    }

    /// A filter's button: the list of that state, its first page.
    fn show(self, filter: Option<GatewayState>) {
        if self.filter.get_untracked() == filter {
            return;
        }
        self.filter.set(filter);
        fragment::replace(fragment_of(filter));
        self.items.set(Vec::new());
        self.more.set(false);
        self.open.set(None);
        self.listing.set(Listing::Loading);
        self.read(None, Then::Nothing);
    }

    /// "Show more": the page after the last gateway shown.
    fn more(self) {
        if self.adding.get_untracked() {
            return;
        }
        let Some(last) = self
            .items
            .with_untracked(|items| items.last().map(|g| g.id))
        else {
            return;
        };
        self.adding.set(true);
        self.read(Some(last), Then::More);
    }

    /// A change ended: the list (and the open tokens) are read again.
    fn ended(self, change: Change, came: Came) {
        let place = match change {
            Change::Approval { gateway, .. } => self
                .items
                .with_untracked(|items| items.iter().position(|g| g.id == gateway)),
            _ => None,
        };
        if let Change::Revoke { gateway, .. } = change
            && self.open.get_untracked() == Some(gateway)
        {
            self.read_tokens(gateway);
        }
        self.recount();
        self.read(None, Then::Ended(came, place));
    }

    /// An approval's or a disabling's button: its confirmation.
    fn approval(self, gateway: &Gateway, enabled: bool) {
        let lang = self.cx.lang;
        let key = id_hex(&gateway.id);
        let asked = if enabled {
            Asked {
                change: Change::Approval {
                    gateway: gateway.id,
                    enabled,
                },
                title: words::admin_gw_approve_title(lang, &gateway.name),
                text: words::admin_gw_approve_text(lang, &gateway.origin),
                action: if gateway.state == GatewayState::Disabled {
                    words::admin_gw_enable(lang)
                } else {
                    words::admin_gw_approve(lang)
                },
                danger: false,
                from: format!("#admin-approve-{key}"),
            }
        } else {
            Asked {
                change: Change::Approval {
                    gateway: gateway.id,
                    enabled,
                },
                title: words::admin_gw_disable_title(lang, &gateway.name),
                text: words::admin_gw_disable_text(lang, &gateway.origin),
                action: words::admin_gw_disable(lang),
                danger: true,
                from: format!("#admin-disable-{key}"),
            }
        };
        self.flow.ask(asked);
    }

    /// A token's "Revoke": its confirmation.
    fn revoke(self, gateway: [u8; 16], token: &Token) {
        let lang = self.cx.lang;
        self.flow.ask(Asked {
            change: Change::Revoke {
                gateway,
                token: token.id,
            },
            title: words::admin_gw_revoke_title(lang, &token.name),
            text: words::admin_gw_revoke_text(lang).to_owned(),
            action: words::tokens_revoke(lang),
            danger: true,
            from: format!("#admin-revoke-{}", id_hex(&token.id)),
        });
    }
}

fn tokens_view(p: &Page, gateway: [u8; 16]) -> impl IntoView + use<> {
    let p = *p;
    let lang = p.cx.lang;
    move || match p.tokens.get() {
        Tokens::Loading => {
            view! { <p role="status" class="meta">{words::account_loading(lang)}</p> }.into_any()
        }
        Tokens::Failed(text) => view! { <p class="error" role="alert">{text}</p> }.into_any(),
        Tokens::Ready(tokens) if tokens.is_empty() => {
            view! { <p class="meta">{words::admin_gw_no_tokens(lang)}</p> }.into_any()
        }
        Tokens::Ready(tokens) => {
            let now = common::now();
            let items = tokens
                .into_iter()
                .map(|token| {
                    let live = token.revoked_at.is_none() && token.expires_at > now;
                    let what = match (token.sso, token.listing) {
                        (true, true) => words::tokens_both(lang),
                        (true, false) => words::tokens_sso(lang),
                        _ => words::tokens_listing(lang),
                    };
                    let standing = if token.revoked_at.is_some() {
                        words::tokens_revoked(lang).to_owned()
                    } else {
                        words::tokens_until(lang, &when(lang, token.expires_at))
                    };
                    let used = token
                        .last_used_at
                        .map(|at| view! { <p class="meta">{words::admin_gw_used(lang, &when(lang, at))}</p> });
                    let action = live.then(|| {
                        let asked = token.clone();
                        view! {
                            <p><button type="button" class="button danger" id=format!("admin-revoke-{}", id_hex(&token.id))
                                on:click=move |_| p.revoke(gateway, &asked)>{words::tokens_revoke(lang)}<span class="visually-hidden">{" "}<bdi>{asked.name.clone()}</bdi></span></button></p>
                        }
                    });
                    view! {
                        <li class="token">
                            <p><strong><bdi>{token.name.clone()}</bdi></strong>" · "{what}</p>
                            <p class="meta">{standing}</p>
                            {used}
                            {action}
                        </li>
                    }
                })
                .collect_view();
            view! { <ul class="tokens">{items}</ul> }.into_any()
        }
    }
}

fn card(p: &Page, gateway: &Gateway) -> impl IntoView + use<> {
    let p = *p;
    let lang = p.cx.lang;
    let id = gateway.id;
    let key = id_hex(&id);
    let heading = format!("admin-gw-{key}");
    let (state, tone) = state_text(lang, gateway.state);
    let official = gateway
        .official
        .then(|| view! { <span class="pill gold">{words::admin_gw_official(lang)}</span> });
    let online = if gateway.online {
        view! { <span class="pill ok">{words::gws_online(lang)}</span> }
    } else {
        view! { <span class="pill off">{words::gws_offline(lang)}</span> }
    };
    let owner = match gateway.owner {
        Some(owner) => {
            let name = gateway
                .owner_name
                .clone()
                .unwrap_or_else(|| common::id_end(&owner));
            view! { <a href=common::user_path(lang, &owner)><bdi translate="no">{name}</bdi></a> }
                .into_any()
        }
        None => words::admin_gw_bridge(lang).into_any(),
    };
    let seen = gateway
        .last_seen
        .map_or_else(|| "–".to_owned(), |at| when(lang, at));
    let tokens = if gateway.credential_pair {
        format!(
            "{} · {}",
            words::number(lang, u64::from(gateway.live_tokens)),
            words::admin_gw_pair(lang)
        )
    } else {
        words::number(lang, u64::from(gateway.live_tokens))
    };
    let approve = (gateway.state != GatewayState::Approved).then(|| {
        let label = if gateway.state == GatewayState::Disabled {
            words::admin_gw_enable(lang)
        } else {
            words::admin_gw_approve(lang)
        };
        let asked = gateway.clone();
        view! {
            <button type="button" class="button primary" id=format!("admin-approve-{key}") aria-describedby=heading.clone()
                on:click=move |_| p.approval(&asked, true)>{label}</button>
        }
    });
    let disable = (gateway.state != GatewayState::Disabled).then(|| {
        let asked = gateway.clone();
        view! {
            <button type="button" class="button danger" id=format!("admin-disable-{key}") aria-describedby=heading.clone()
                on:click=move |_| p.approval(&asked, false)>{words::admin_gw_disable(lang)}</button>
        }
    });
    let shown = move || p.open.get() == Some(id);
    view! {
        <li class="card admin-gw">
            <div class="admin-gw-head">
                <div class="admin-gw-who">
                    <h2 class="admin-gw-name" id=heading.clone() tabindex="-1"><bdi>{gateway.name.clone()}</bdi></h2>
                    <p class="admin-copyable">
                        <code class="address" translate="no">{gateway.origin.clone()}</code>
                        {common::copy_button(lang, gateway.origin.clone(), words::gws_address(lang))}
                    </p>
                </div>
                <p class="pills">{official}<span class=format!("pill {tone}")>{state}</span>{online}</p>
            </div>
            <dl class="facts">
                <dt>{words::admin_gw_owner(lang)}</dt>
                <dd>{owner}</dd>
                <dt>{words::admin_created(lang)}</dt>
                <dd>{when(lang, gateway.created_at)}</dd>
                <dt>{words::admin_gw_seen(lang)}</dt>
                <dd>{seen}</dd>
                <dt>{words::admin_gw_tokens(lang)}</dt>
                <dd>{tokens}</dd>
            </dl>
            <p class="actions">
                {approve}
                {disable}
                <button type="button" class="button quiet" id=format!("admin-tokens-{key}") aria-describedby=heading
                    aria-expanded=move || if shown() { "true" } else { "false" }
                    on:click=move |_| p.toggle(id)>
                    {move || if shown() { words::admin_gw_hide_tokens(lang) } else { words::admin_gw_show_tokens(lang) }}
                </button>
            </p>
            {p.flow.panel(move |change| match change {
                Change::Approval { gateway, .. } | Change::Revoke { gateway, .. } => *gateway == id,
                _ => false,
            })}
            {move || shown().then(|| view! { <div class="admin-tokens">{tokens_view(&p, id)}</div> })}
        </li>
    }
}

fn results(p: &Page) -> impl IntoView + use<> {
    let p = *p;
    let lang = p.cx.lang;
    move || {
        let items = p.items.get();
        match p.listing.get() {
            Listing::Loading => {
                view! { <p role="status" class="meta">{words::account_loading(lang)}</p> }
                    .into_any()
            }
            Listing::Failed(text) => view! {
                <p class="error" role="alert">{text}</p>
                <p class="actions">
                    <button type="button" class="button" on:click=move |_| {
                        p.listing.set(Listing::Loading);
                        p.read(None, Then::Nothing);
                    }>{words::error_again(lang)}</button>
                </p>
            }
            .into_any(),
            Listing::Ready if items.is_empty() => {
                view! { <p role="status">{words::admin_gw_none(lang)}</p> }.into_any()
            }
            Listing::Ready => {
                let cards = items.iter().map(|gateway| card(&p, gateway)).collect_view();
                let more = p.more.get().then(|| {
                    view! {
                        <p class="actions">
                            <button type="button" class="button" aria-disabled=move || p.adding.get().then_some("true")
                                on:click=move |_| p.more()>{words::admin_more(lang)}</button>
                        </p>
                    }
                });
                view! {
                    <ul class="admin-gws" id="admin-gws">{cards}</ul>
                    {more}
                }
                .into_any()
            }
        }
    }
}

fn chips(p: &Page) -> impl IntoView + use<> {
    let p = *p;
    let lang = p.cx.lang;
    let buttons = FILTERS
        .into_iter()
        .map(|filter| {
            let label = match filter {
                Some(state) => state_text(lang, state).0,
                None => words::gw_show_every(lang),
            };
            let shown = move || {
                p.counts.with(|counts| {
                    counts.as_ref().map(|counts| {
                        view! { " " <span class="chip-n">{words::number(lang, count(counts, filter))}</span> }
                    })
                })
            };
            view! {
                <button type="button" class="chip-btn" aria-pressed=move || if p.filter.get() == filter { "true" } else { "false" }
                    on:click=move |_| p.show(filter)>{label}{shown}</button>
            }
        })
        .collect_view();
    view! {
        <fieldset class="admin-chips">
            <legend class="visually-hidden">{words::gw_show(lang)}</legend>
            {buttons}
        </fieldset>
    }
}

/// The page: what approval means, the filters, the gateways.
pub fn page(cx: Cx) -> impl IntoView {
    let lang = cx.lang;
    let note = RwSignal::new(None::<Note>);
    let ended = StoredValue::new(None::<Page>);
    let finished = Callback::new(move |(change, came): (Change, Came)| {
        if let Some(Some(p)) = ended.try_get_value() {
            p.ended(change, came);
        }
    });
    let p = Page {
        cx,
        filter: RwSignal::new(Some(GatewayState::Pending)),
        items: RwSignal::new(Vec::new()),
        listing: RwSignal::new(Listing::Loading),
        more: RwSignal::new(false),
        adding: RwSignal::new(false),
        generation: StoredValue::new(0),
        counts: RwSignal::new(None),
        open: RwSignal::new(None),
        tokens: RwSignal::new(Tokens::Loading),
        note,
        flow: Flow::new(cx, note, finished),
    };
    ended.set_value(Some(p));
    // Effects run in the browser only: the filter the address names, its
    // first page, and the counts
    Effect::new(move |_| {
        let filter = filter_in(&fragment::take());
        p.filter.set(filter);
        p.read(None, Then::Nothing);
        p.recount();
    });
    view! {
        <p class="admin-lead">{words::admin_gw_lead(lang)}</p>
        {chips(&p)}
        {p.flow.notes()}
        <div class="admin-results" aria-busy=move || (p.listing.get() == Listing::Loading || p.adding.get()).then_some("true")>
            {results(&p)}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The filter the overview's link and a reload name; the queue for
    /// anything else.
    #[test]
    fn the_filter_stands_in_the_fragment() {
        for filter in FILTERS {
            assert_eq!(filter_in(fragment_of(filter)), filter);
        }
        for other in [
            "",
            "#",
            "#state=",
            "#state=APPROVED",
            "#approved",
            "#state=approved&x",
        ] {
            assert_eq!(filter_in(other), Some(GatewayState::Pending), "{other}");
        }
        let counts = AdminOverview {
            accounts: 0,
            accounts_week: 0,
            moved: 0,
            admins: 0,
            pending: 2,
            approved: 5,
            disabled: 1,
            online: 3,
            live_tokens: 9,
        };
        assert_eq!(count(&counts, Some(GatewayState::Pending)), 2);
        assert_eq!(count(&counts, Some(GatewayState::Approved)), 5);
        assert_eq!(count(&counts, Some(GatewayState::Disabled)), 1);
        assert_eq!(count(&counts, None), 8);
    }
}
