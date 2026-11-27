//! The AI players' page's runner tokens (`docs/ai-players.md` §2): the live
//! ones first, then those revoked or expired in the last 30 days
//! (`AdminRequest::AiRunnerTokens`), each with the AI players it plays,
//! who made it (an administrator, or the operator's command), when it ends
//! and when a runner last signed in with it. An administrator makes one
//! (`CreateAiRunnerToken`: a name, 1 to 16 AI players, its days) and
//! revokes one after its confirmation, which needs no fresh sign-in.
//!
//! A new token is shown once, as the report tokens' page shows one: held
//! by the step that shows it, which goes when it is left (by its button,
//! by leaving the page, or by the page being put away into the browser's
//! back-forward cache). The `Token` wipes its own bytes; it never goes into
//! an address, a log, the console, storage or a field with a name.

use central_api::admin::AdminRequest;
use central_api::ai_runner::{AiRunnerTokenInfo, MAX_ACCOUNTS, token_text, validate_new};
use central_api::owner::{MAX_TOKEN_DAYS, MAX_TOKEN_NAME};
use leptos::prelude::*;

use super::super::common::{self, Asked, Change};
use super::{Made, Page, Which, idle, name_of, password_step, wrong_line};
use crate::data::id_hex;
use crate::islands::account::when;
use crate::islands::chat::glyph;
use crate::islands::field::{self, Labelled};
use crate::islands::{clip, focus};
use crate::words::{self, Lang};

/// How long a token is offered to live, in days.
const DAYS: [u16; 4] = [30, 90, 180, 366];
const _: () = assert!(DAYS[3] <= MAX_TOKEN_DAYS);
const _: () = assert!(MAX_TOKEN_NAME == 64, "the name's field takes as many");

/// A new runner token as typed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct TokenFields {
    pub(super) name: String,
    /// The AI players it plays, in the order they were chosen.
    accounts: Vec<[u8; 16]>,
    days: u16,
}

impl Default for TokenFields {
    fn default() -> Self {
        TokenFields {
            name: String::new(),
            accounts: Vec::new(),
            days: DAYS[1],
        }
    }
}

/// Where a token stands at a moment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Standing {
    Live,
    Expired,
    Revoked,
}

fn standing(token: &AiRunnerTokenInfo, now: i64) -> Standing {
    if token.revoked_at.is_some() {
        Standing::Revoked
    } else if token.expires_at <= now {
        Standing::Expired
    } else {
        Standing::Live
    }
}

fn standing_text(lang: Lang, standing: Standing) -> (&'static str, &'static str) {
    match standing {
        Standing::Live => (words::rt_live(lang), "ok"),
        Standing::Expired => (words::rt_expired(lang), "off"),
        Standing::Revoked => (words::tokens_revoked(lang), "bad"),
    }
}

/// A person a token names: their handle, else the end of their id.
fn person(id: &[u8; 16], name: Option<&str>, tag: Option<u32>) -> String {
    common::handle(name, tag).unwrap_or_else(|| common::id_end(id))
}

/// The request the form asks for, checked by Central's rules first; else
/// the words that say why not, and where the keyboard goes.
pub(super) fn request(p: &Page) -> Result<AdminRequest, (&'static str, String)> {
    let fields = p.token.get_untracked();
    let name = fields.name.trim().to_owned();
    validate_new(&name, &fields.accounts, fields.days).map_err(|_| {
        (
            words::ai_token_invalid(p.cx.lang),
            "#ai-token-name".to_owned(),
        )
    })?;
    Ok(AdminRequest::CreateAiRunnerToken {
        name,
        accounts: fields.accounts,
        days: fields.days,
    })
}

/// A token's "Revoke": its confirmation.
fn revoke(p: &Page, token: &AiRunnerTokenInfo) {
    let lang = p.cx.lang;
    p.flow.ask(Asked {
        change: Change::RevokeRunner { token: token.id },
        title: words::ai_token_revoke_title(lang, &token.name),
        text: words::ai_token_revoke_text(lang).to_owned(),
        action: words::tokens_revoke(lang),
        danger: true,
        from: format!("#ai-token-revoke-{}", id_hex(&token.id)),
    });
}

/// The AI players a new token plays, as boxes to tick.
fn accounts_view(p: &Page) -> impl IntoView + use<> {
    let p = *p;
    let lang = p.cx.lang;
    move || {
        let players = p.players.get();
        if players.is_empty() {
            return view! { <p class="notice">{words::ai_token_no_ai(lang)}</p> }.into_any();
        }
        let boxes = players
            .iter()
            .map(|player| {
                let account = player.account;
                let id = format!("ai-token-for-{}", id_hex(&account));
                view! {
                    <div class="check">
                        <input id=id.clone() type="checkbox" aria-describedby="ai-token-accounts-hint"
                            prop:checked=move || p.token.with(|fields| fields.accounts.contains(&account))
                            on:change=move |ev| {
                                let held = event_target_checked(&ev);
                                p.token.update(|fields| {
                                    fields.accounts.retain(|at| *at != account);
                                    if held {
                                        fields.accounts.push(account);
                                    }
                                });
                            } />
                        <label for=id><bdi translate="no">{name_of(player)}</bdi></label>
                    </div>
                }
            })
            .collect_view();
        view! {
            <fieldset class="choice">
                <legend>{words::ai_token_accounts(lang)}</legend>
                {boxes}
                <p class="hint" id="ai-token-accounts-hint">{words::ai_token_accounts_hint(lang)}</p>
            </fieldset>
        }
        .into_any()
    }
}

fn form_view(p: &Page) -> AnyView {
    let p = *p;
    let lang = p.cx.lang;
    let which = Which::Token;
    let spec = Labelled {
        id: "ai-token-name",
        label: words::tokens_name(lang),
        hint: Some(words::ai_token_name_hint(lang)),
        mark: glyph::KEY,
        auto: "off",
        max: "64",
    };
    let days = DAYS
        .into_iter()
        .map(|days| {
            view! {
                <option value=days.to_string() prop:selected=move || p.token.with(|fields| fields.days == days)>
                    {words::number(lang, u64::from(days))}
                </option>
            }
        })
        .collect_view();
    view! {
        <form class="form" method="post" novalidate=true on:submit=move |event: leptos::ev::SubmitEvent| {
            event.prevent_default();
            p.submit(which, None);
        }>
            {field::labelled(lang, spec, move || p.token.with(|fields| fields.name.clone()), move |value| p.token.update(|fields| fields.name = value))}
            {accounts_view(&p)}
            <div class="field">
                <label for="ai-token-days">{words::tokens_days(lang)}</label>
                <select id="ai-token-days" on:change=move |ev| {
                    let days = event_target_value(&ev).parse().unwrap_or(DAYS[1]);
                    p.token.update(|fields| fields.days = days);
                }>
                    {days}
                </select>
            </div>
            {wrong_line(&p, which)}
            <p class="actions" hidden=move || !idle(&p, which)()>
                <button type="submit" class="button" id="ai-submit-token">{words::tokens_make(lang)}</button>
            </p>
        </form>
        {password_step(&p, which, words::tokens_make(lang))}
    }
    .into_any()
}

fn shown_view(p: &Page, made: &Made) -> AnyView {
    let p = *p;
    let lang = p.cx.lang;
    let text = token_text(&made.token).as_str().to_owned();
    let (copy, copied) = clip::copier(lang, text.clone());
    view! {
        <div class="secret-box" role="group" aria-labelledby="ai-token-shown">
            <h3 id="ai-token-shown" tabindex="-1">{words::tokens_shown_title(lang)}</h3>
            <p class="meta"><bdi translate="no">{made.name.clone()}</bdi></p>
            <p class="eyebrow">{words::code_shown_once(lang)}</p>
            <p class="secret" translate="no"><code>{text}</code></p>
            <p>{words::ai_token_shown_lead(lang)}</p>
            <p class="actions">
                {copy}
                <button type="button" class="button primary" on:click=move |_| {
                    p.shown.set(None);
                    focus::after_render("#ai-token-new");
                }>{words::tokens_kept(lang)}</button>
            </p>
            {copied}
        </div>
    }
    .into_any()
}

fn making_view(p: &Page) -> impl IntoView + use<> {
    let p = *p;
    let lang = p.cx.lang;
    let body = move || match p.shown.get() {
        Some(made) => shown_view(&p, &made),
        None => form_view(&p),
    };
    view! {
        <section class="card rt-making" aria-labelledby="ai-token-new">
            <h3 id="ai-token-new" tabindex="-1">{words::ai_token_new(lang)}</h3>
            {body}
        </section>
    }
}

fn card(p: &Page, token: &AiRunnerTokenInfo, now: i64) -> impl IntoView + use<> {
    let p = *p;
    let lang = p.cx.lang;
    let id = token.id;
    let key = id_hex(&id);
    let heading = format!("ai-token-{key}");
    let at = standing(token, now);
    let (state, tone) = standing_text(lang, at);
    let accounts = token
        .accounts
        .iter()
        .map(|account| {
            let name = person(&account.id, account.display_name.as_deref(), account.tag);
            view! { <li><a href=common::user_path(lang, &account.id)><bdi translate="no">{name}</bdi></a></li> }
        })
        .collect_view();
    let maker = match token.created_by {
        Some(admin) => {
            let name = person(
                &admin,
                token.created_by_name.as_deref(),
                token.created_by_tag,
            );
            view! { <a href=common::user_path(lang, &admin)><bdi translate="no">{name}</bdi></a> }
                .into_any()
        }
        None => words::admin_audit_operator(lang).into_any(),
    };
    let used = token
        .last_used_at
        .map_or_else(|| words::rt_never(lang).to_owned(), |at| when(lang, at));
    let revoked = token.revoked_at.map(|time| {
        let time = when(lang, time);
        let text = match token.revoked_by {
            Some(admin) => {
                let name = person(
                    &admin,
                    token.revoked_by_name.as_deref(),
                    token.revoked_by_tag,
                );
                words::rt_revoked_by(lang, &time, &name)
            }
            None => words::rt_revoked_at(lang, &time),
        };
        view! { <p class="meta"><bdi>{text}</bdi></p> }
    });
    let action = (at == Standing::Live).then(|| {
        let asked = token.clone();
        view! {
            <p class="actions">
                <button type="button" class="button danger" id=format!("ai-token-revoke-{key}") aria-describedby=heading.clone()
                    on:click=move |_| revoke(&p, &asked)>{words::tokens_revoke(lang)}</button>
            </p>
        }
    });
    view! {
        <li class="card admin-gw" data-standing=tone>
            <div class="admin-gw-head">
                <h4 class="admin-gw-name" id=heading tabindex="-1"><bdi translate="no">{token.name.clone()}</bdi></h4>
                <p class="pills"><span class=format!("pill {tone}")>{state}</span></p>
            </div>
            <dl class="facts">
                <dt>{words::admin_id(lang)}</dt>
                <dd class="admin-copyable">
                    <code class="address" translate="no">{key.clone()}</code>
                    {common::copy_button(lang, key, words::admin_id(lang))}
                </dd>
                <dt>{words::ai_token_accounts(lang)}</dt>
                <dd><ul class="admin-owned">{accounts}</ul></dd>
                <dt>{words::rt_by(lang)}</dt>
                <dd>{maker}</dd>
                <dt>{words::admin_created(lang)}</dt>
                <dd>{when(lang, token.created_at)}</dd>
                <dt>{words::rt_expires(lang)}</dt>
                <dd>{when(lang, token.expires_at)}</dd>
                <dt>{words::rt_used(lang)}</dt>
                <dd>{used}</dd>
            </dl>
            {revoked}
            {action}
            {p.flow.panel(move |change| *change == Change::RevokeRunner { token: id })}
        </li>
    }
}

fn list_view(p: &Page) -> impl IntoView + use<> {
    let p = *p;
    let lang = p.cx.lang;
    move || {
        let tokens = p.tokens.get();
        if tokens.is_empty() {
            return view! { <p role="status">{words::ai_tokens_none(lang)}</p> }.into_any();
        }
        let now = common::now();
        let cards = tokens
            .iter()
            .map(|token| card(&p, token, now))
            .collect_view();
        view! { <ul class="admin-gws" id="ai-tokens">{cards}</ul> }.into_any()
    }
}

/// The runner tokens' part of the page: what they are, a new one and the
/// tokens.
pub(super) fn section(p: &Page) -> impl IntoView + use<> {
    let lang = p.cx.lang;
    view! {
        <section class="ai-stack" aria-labelledby="ai-tokens-title">
            <h2 id="ai-tokens-title">{words::ai_tokens(lang)}</h2>
            <p>{words::ai_tokens_lead(lang)}</p>
            {making_view(p)}
            {list_view(p)}
        </section>
    }
}

const _: () = assert!(MAX_ACCOUNTS == 16, "the hint says 1 to 16");

#[cfg(test)]
mod tests {
    use super::*;
    use central_api::ai_runner::AiRunnerAccount;

    const NOW: i64 = 1_800_000_000;

    fn token() -> AiRunnerTokenInfo {
        AiRunnerTokenInfo {
            id: [1; 16],
            name: "bigace".into(),
            accounts: vec![AiRunnerAccount {
                id: [3; 16],
                display_name: Some("KI-Haus-Expert".into()),
                tag: Some(0x1a2b),
            }],
            created_by: None,
            created_by_name: None,
            created_by_tag: None,
            created_at: NOW - 100,
            expires_at: NOW + 100,
            revoked_at: None,
            revoked_by: None,
            revoked_by_name: None,
            revoked_by_tag: None,
            last_used_at: None,
        }
    }

    /// A token stands as its worst says: revoked, then over; a token the
    /// operator made has no maker, and still works.
    #[test]
    fn where_a_token_stands() {
        let live = token();
        assert_eq!(standing(&live, NOW), Standing::Live);
        assert_eq!(standing(&live, NOW + 100), Standing::Expired);
        let revoked = AiRunnerTokenInfo {
            revoked_at: Some(NOW),
            ..token()
        };
        assert_eq!(standing(&revoked, NOW), Standing::Revoked);
        assert_eq!(standing(&revoked, NOW + 100), Standing::Revoked);
        for lang in Lang::ALL {
            let texts: Vec<&str> = [Standing::Live, Standing::Expired, Standing::Revoked]
                .into_iter()
                .map(|at| standing_text(lang, at).0)
                .collect();
            for (at, text) in texts.iter().enumerate() {
                assert_ne!(*text, "");
                assert!(!texts[..at].contains(text), "{lang:?}: {text}");
            }
        }
        assert_eq!(
            person(&[3; 16], Some("KI-Haus-Expert"), Some(0x1a2b)),
            "KI-Haus-Expert#1a2b"
        );
        assert_eq!(person(&[3; 16], None, None), "…03030303");
    }

    /// The one request that makes a token, as Central reads it.
    #[test]
    fn the_request_serializes_as_central_reads_it() {
        let request = central_api::wire::Request::Admin {
            request: AdminRequest::CreateAiRunnerToken {
                name: "bigace".into(),
                accounts: vec![[3; 16]],
                days: 90,
            },
        };
        let text = serde_json::to_string(&request).expect("json");
        assert!(
            text.starts_with(
                r#"{"operation":"admin","request":{"action":"create_ai_runner_token","name":"bigace","accounts":[[3,"#
            ),
            "{text}"
        );
        assert!(text.ends_with(r#"]],"days":90}}"#), "{text}");
        assert!(validate_new("bigace", &[[3; 16]], DAYS[3]).is_ok());
        assert!(validate_new("bigace", &[], DAYS[1]).is_err());
        assert_eq!(TokenFields::default().days, 90);
    }
}
