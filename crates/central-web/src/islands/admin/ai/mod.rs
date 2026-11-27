//! `/{lang}/account/admin/ai`: the AI players (`docs/ai-players.md` §1,
//! §5). Each is listed with whether it is on, its live matches, its games
//! today, its offers and its runner's last sign-in
//! (`AdminRequest::AiPlayers`). An administrator creates one
//! (`CreateAiPlayer`: a new account without a password, named `KI-…`,
//! switched off), changes its label, limits and name (`SetAiPlayer`; the
//! kind and the profile stay), switches it on and off (`SetAiEnabled`),
//! removes it before its first match (`RemoveAiPlayer`), sets what holds
//! for every AI (`AiSettings`, `SetAiSettings`) and stops them all at once
//! (`StopAllAi`). The runner tokens are `ai_tokens`'s part of the page.
//!
//! A button's change goes through its confirmation (`common::Flow`); a
//! form's is checked by Central's own rules before anything is sent, and
//! asks for the password once more where Central needs a sign-in less than
//! 30 minutes old (`AdminRequest::fresh`). Switching off, stopping every AI
//! and revoking a runner token need none.

use std::sync::Arc;

use central_api::Token;
use central_api::admin::{AdminAiPlayer, AdminAnswer, AdminRequest, AiSettings};
use central_api::ai_runner::AiRunnerTokenInfo;
use central_api::ranked::Format;
use central_api::ranked::ai::{AiBadge, AiKind};
use central_api::wire::{Request, Response};
use leptos::prelude::*;

use super::Listing;
use super::common::{self, Asked, Came, Change, Cx, Flow, Note};
use crate::data::id_hex;
use crate::islands::account::when;
use crate::islands::auth::{Problem, problem_text};
use crate::islands::central::{Failure, call};
use crate::islands::chat::glyph;
use crate::islands::field::{self, Labelled};
use crate::islands::focus;
use crate::islands::ui::ai_badge::{ai_badge, kind_key, kind_text};
use crate::words::{self, Lang};

mod fields;
mod tokens;

use fields::{Fields, SettingsFields};
use tokens::TokenFields;

/// The kinds an administrator creates; a kind this build does not know is
/// never offered.
const KINDS: [AiKind; 3] = [AiKind::House, AiKind::Model, AiKind::Net];

/// Which of the page's forms a step belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Which {
    /// The new AI player's.
    Create,
    /// The change of the AI player with this account.
    Edit([u8; 16]),
    /// The settings for every AI.
    Settings,
    /// The new runner token's.
    Token,
}

impl Which {
    /// Its part of element ids.
    fn key(self) -> String {
        match self {
            Which::Create => "new".to_owned(),
            // one AI player is changed at a time
            Which::Edit(_) => "edit".to_owned(),
            Which::Settings => "settings".to_owned(),
            Which::Token => "token".to_owned(),
        }
    }
}

/// Where a form stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    Idle,
    /// The password once more, for this form.
    Password(Which),
    /// Asking Central.
    Working(Which),
}

/// A runner token just made, shown once. Neither `Clone` nor `Debug`:
/// the step shares it, and the `Token` wipes its bytes when it goes.
struct Made {
    token: Token,
    name: String,
}

/// What a failed form means before it is explained: `None` for Central's
/// refusal, explained by what stands afterwards. An answer that did not
/// arrive is no refusal: the change may have been made.
fn at_form(which: Which, failure: Failure) -> Option<Came> {
    match failure {
        Failure::Rejected => None,
        Failure::Busy => Some(Came::Other(Problem::Busy)),
        Failure::Conflict if which == Which::Create => Some(Came::Taken),
        Failure::Conflict | Failure::Forbidden => Some(Came::Other(Problem::Failed)),
        Failure::Unavailable | Failure::Malformed | Failure::Network => Some(Came::Lost),
    }
}

/// The words for what a form came to; `None` where the page asks for the
/// password, says nobody is signed in, or says it was made.
fn form_text(lang: Lang, which: Which, came: Came) -> Option<&'static str> {
    match (which, came) {
        (_, Came::Taken) => Some(words::ai_tag_taken(lang)),
        (Which::Token, Came::Other(Problem::Busy)) => Some(words::ai_token_full(lang)),
        (Which::Token, Came::Lost) => Some(words::ai_token_lost(lang)),
        (_, Came::Lost) => Some(words::admin_change_lost(lang)),
        (_, came) => common::came_text(lang, came),
    }
}

/// Sends a form's `request`, once, after `AccountSummary` said it may be;
/// `proven` where the password was proven just now.
async fn send(
    which: Which,
    request: AdminRequest,
    own: [u8; 16],
    proven: bool,
) -> Result<Response, Came> {
    common::may_send(own, proven).await?;
    match call(&Request::Admin { request }).await {
        Ok(response) => Ok(response),
        Err(failure) => match at_form(which, failure) {
            Some(came) => Err(came),
            None => Err(common::refused(own, proven).await),
        },
    }
}

/// The page's signals. All of it is `Copy`.
#[derive(Clone, Copy)]
struct Page {
    cx: Cx,
    players: RwSignal<Vec<AdminAiPlayer>>,
    settings: RwSignal<Option<(AiSettings, Option<i64>)>>,
    tokens: RwSignal<Vec<AiRunnerTokenInfo>>,
    listing: RwSignal<Listing>,
    /// The newest read: an older one's answer is not drawn.
    generation: StoredValue<u32>,
    stage: RwSignal<Stage>,
    password: RwSignal<String>,
    /// What a form says of its last attempt, and which.
    wrong: RwSignal<Option<(Which, &'static str)>>,
    /// A form is on its way: nothing else is sent.
    busy: StoredValue<bool>,
    new: RwSignal<Fields>,
    /// The AI player being changed, and its fields.
    edit: RwSignal<Option<([u8; 16], Fields)>>,
    set: RwSignal<SettingsFields>,
    token: RwSignal<TokenFields>,
    /// The runner token just made, shown once.
    shown: RwSignal<Option<Arc<Made>>>,
    flow: Flow,
}

impl Page {
    /// Reads the AI players, the settings and the runner tokens; `refill`
    /// puts the settings read into their form. After a change it says what
    /// a lost answer came to.
    fn read(self, refill: bool, ended: Option<Came>) {
        let Some(generation) = self.generation.try_get_value() else {
            return;
        };
        let generation = generation.wrapping_add(1);
        self.generation.set_value(generation);
        let lang = self.cx.lang;
        leptos::task::spawn_local(async move {
            let read = read_all().await;
            if self.generation.try_get_value() != Some(generation) {
                return;
            }
            let ok = match read {
                Ok((players, settings, changed_at, tokens)) => {
                    self.players.set(players);
                    if refill || self.settings.get_untracked().is_none() {
                        self.set.set(SettingsFields::of(&settings));
                    }
                    self.settings.set(Some((settings, changed_at)));
                    self.tokens.set(tokens);
                    self.listing.set(Listing::Ready);
                    true
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
            if let Some(came) = ended {
                if came == Came::Lost {
                    self.flow.note.set(Some(common::lost(lang, ok)));
                }
                common::focus_note(self.flow.note);
            }
        });
    }

    /// The request a form asks for, checked before it is sent; else the
    /// words that say why not, said at the form, with the keyboard on the
    /// field.
    fn request(self, which: Which) -> Result<AdminRequest, (&'static str, String)> {
        let lang = self.cx.lang;
        let field = |name: &str| format!("#ai-{}-{name}", which.key());
        match which {
            Which::Create => {
                let fields = self.new.get_untracked();
                let settings = fields
                    .settings(lang)
                    .map_err(|(text, at)| (text, field(at)))?;
                let tag = fields
                    .tag()
                    .map_err(|()| (words::ai_invalid_tag(lang), field("tag")))?;
                Ok(AdminRequest::CreateAiPlayer { settings, tag })
            }
            Which::Edit(account) => {
                let Some((_, fields)) = self.edit.get_untracked() else {
                    return Err((problem_text(lang, Problem::Failed), field("name")));
                };
                let settings = fields
                    .settings(lang)
                    .map_err(|(text, at)| (text, field(at)))?;
                Ok(AdminRequest::SetAiPlayer { account, settings })
            }
            Which::Settings => {
                let settings = self
                    .set
                    .get_untracked()
                    .settings()
                    .map_err(|id| (words::ai_settings_invalid(lang), format!("#{id}")))?;
                Ok(AdminRequest::SetAiSettings { settings })
            }
            Which::Token => tokens::request(&self),
        }
    }

    /// A form's submit, and its password's: asks once, after the password
    /// where `password` is one.
    fn submit(self, which: Which, password: Option<String>) {
        if self.busy.get_value() {
            return;
        }
        self.flow.note.set(None);
        let request = match self.request(which) {
            Ok(request) => request,
            Err((text, at)) => {
                self.stage.set(Stage::Idle);
                self.wrong.set(Some((which, text)));
                focus::after_render(at);
                return;
            }
        };
        self.busy.set_value(true);
        self.wrong.set(None);
        self.stage.set(Stage::Working(which));
        focus::after_render(format!("#ai-working-{}", which.key()));
        let own = self.cx.own;
        let worker = self.cx.worker.get_value();
        leptos::task::spawn_local(async move {
            let sent = match password {
                Some(password) => match common::prove(own, &worker, password).await {
                    Ok(()) => send(which, request, own, true).await,
                    Err(came) => Err(came),
                },
                None => send(which, request, own, false).await,
            };
            self.ended(which, sent);
        });
    }

    /// What a form came to.
    fn ended(self, which: Which, sent: Result<Response, Came>) {
        // a page that went meanwhile shows nothing more
        if self.busy.try_get_value().is_none() {
            return;
        }
        self.busy.set_value(false);
        let lang = self.cx.lang;
        let sent = sent.and_then(|response| match (which, response) {
            (
                Which::Create,
                Response::Admin {
                    answer: AdminAnswer::AiPlayerCreated { .. },
                },
            )
            | (Which::Edit(_) | Which::Settings, Response::Done) => Ok(None),
            (
                Which::Token,
                Response::Admin {
                    answer: AdminAnswer::AiRunnerTokenCreated { token, .. },
                },
            ) => Ok(Some(token)),
            // an answer of another kind says nothing of what happened
            _ => Err(Came::Lost),
        });
        match sent {
            Ok(token) => {
                self.stage.set(Stage::Idle);
                let done = match which {
                    Which::Create => {
                        self.new.set(Fields::fresh());
                        words::ai_created_done(lang)
                    }
                    Which::Edit(_) => {
                        self.edit.set(None);
                        words::ai_saved(lang)
                    }
                    Which::Settings => words::ai_settings_saved(lang),
                    Which::Token => {
                        let name = self.token.get_untracked().name.trim().to_owned();
                        self.token.set(TokenFields::default());
                        if let Some(token) = token {
                            self.shown.set(Some(Arc::new(Made { token, name })));
                        }
                        focus::after_render("#ai-token-shown");
                        self.read(false, None);
                        return;
                    }
                };
                self.flow.note.set(Some(Note::Done(done)));
                self.read(which == Which::Settings, Some(Came::Done));
            }
            Err(Came::Password) => {
                self.stage.set(Stage::Password(which));
                focus::after_render("#admin-password");
            }
            Err(Came::WrongPassword) => {
                self.wrong
                    .set(form_text(lang, which, Came::WrongPassword).map(|text| (which, text)));
                self.stage.set(Stage::Password(which));
                focus::after_render("#admin-password");
            }
            Err(Came::SignedOut) => self.cx.leave(),
            Err(came) => {
                self.stage.set(Stage::Idle);
                self.wrong
                    .set(form_text(lang, which, came).map(|text| (which, text)));
                focus::after_render(format!("#ai-wrong-{}", which.key()));
                if came == Came::Lost {
                    self.read(false, None);
                }
            }
        }
    }

    /// Steps back from a form's password: nothing is sent.
    fn back(self, which: Which) {
        if self.busy.get_value() {
            return;
        }
        self.password.set(String::new());
        self.wrong.set(None);
        self.stage.set(Stage::Idle);
        focus::after_render(format!("#ai-submit-{}", which.key()));
    }

    /// A page put away takes no token and no password with it.
    #[cfg(target_arch = "wasm32")]
    fn forget(self) {
        let _ = self.shown.try_set(None);
        let _ = self.password.try_set(String::new());
    }

    /// "Switch on" or "Switch off": its confirmation.
    fn switch(self, player: &AdminAiPlayer, enabled: bool) {
        let lang = self.cx.lang;
        let name = name_of(player);
        let (title, text, action) = if enabled {
            (
                words::ai_on_title(lang, &name),
                words::ai_on_text(lang),
                words::ai_switch_on(lang),
            )
        } else {
            (
                words::ai_off_title(lang, &name),
                words::ai_off_text(lang),
                words::ai_switch_off(lang),
            )
        };
        self.flow.ask(Asked {
            change: Change::AiEnabled {
                account: player.account,
                enabled,
            },
            title,
            text: text.to_owned(),
            action,
            danger: !enabled,
            from: format!("#ai-switch-{}", id_hex(&player.account)),
        });
    }

    /// "Remove": its confirmation.
    fn remove(self, player: &AdminAiPlayer) {
        let lang = self.cx.lang;
        self.flow.ask(Asked {
            change: Change::RemoveAi {
                account: player.account,
            },
            title: words::ai_remove_title(lang, &name_of(player)),
            text: words::ai_remove_text(lang).to_owned(),
            action: words::ai_remove(lang),
            danger: true,
            from: format!("#ai-remove-{}", id_hex(&player.account)),
        });
    }

    /// "Stop every AI": its confirmation.
    fn stop_all(self) {
        let lang = self.cx.lang;
        self.flow.ask(Asked {
            change: Change::StopAllAi,
            title: words::ai_stop_title(lang).to_owned(),
            text: words::ai_stop_text(lang).to_owned(),
            action: words::ai_stop_all(lang),
            danger: true,
            from: "#ai-stop-all".to_owned(),
        });
    }

    /// "Change": the AI player's form opens in its card.
    fn change(self, player: &AdminAiPlayer) {
        if self.busy.get_value() {
            return;
        }
        self.wrong.set(None);
        self.stage.set(Stage::Idle);
        self.edit.set(Some((player.account, Fields::of(player))));
        focus::after_render("#ai-edit-name");
    }
}

/// Reads what the page shows: the AI players, the settings, the tokens.
async fn read_all() -> Result<
    (
        Vec<AdminAiPlayer>,
        AiSettings,
        Option<i64>,
        Vec<AiRunnerTokenInfo>,
    ),
    common::Unread,
> {
    let unexpected = common::Unread::Other(Problem::Failed);
    let AdminAnswer::AiPlayers { players } = common::read(AdminRequest::AiPlayers).await? else {
        return Err(unexpected);
    };
    let AdminAnswer::AiSettings {
        settings,
        changed_at,
    } = common::read(AdminRequest::AiSettings).await?
    else {
        return Err(unexpected);
    };
    let AdminAnswer::AiRunnerTokens { tokens } = common::read(AdminRequest::AiRunnerTokens).await?
    else {
        return Err(unexpected);
    };
    Ok((players, settings, changed_at, tokens))
}

/// An AI player as the page names it: its handle, else the end of its id.
fn name_of(player: &AdminAiPlayer) -> String {
    common::handle(player.display_name.as_deref(), player.tag)
        .unwrap_or_else(|| common::id_end(&player.account))
}

/// A format's name, as the game prints it.
fn format_name(format: Format) -> &'static str {
    match format {
        Format::Standard => "Standard",
        Format::Highlander => "Highlander",
        Format::Commander => "Commander",
    }
}

/// A number's field of a form.
struct Number {
    id: String,
    label: &'static str,
    hint: &'static str,
    min: &'static str,
    max: &'static str,
}

fn number_field(
    what: Number,
    value: impl Fn() -> String + Send + Sync + 'static,
    set: impl Fn(String) + 'static,
) -> impl IntoView {
    let Number {
        id,
        label,
        hint,
        min,
        max,
    } = what;
    let hint_id = format!("{id}-hint");
    view! {
        <div class="field">
            <label for=id.clone()>{label}</label>
            <input id=id type="number" inputmode="numeric" step="1" min=min max=max autocomplete="off"
                aria-describedby=hint_id.clone()
                on:input=move |ev| set(event_target_value(&ev)) prop:value=value />
            <p class="hint" id=hint_id>{hint}</p>
        </div>
    }
}

/// The password once more, in a form's card, and the work.
fn password_step(p: &Page, which: Which, action: &'static str) -> impl IntoView + use<> {
    let p = *p;
    let lang = p.cx.lang;
    move || {
        match p.stage.get() {
        Stage::Password(at) if at == which => {
            let spec = Labelled {
                id: "admin-password",
                label: words::privacy_delete_password(lang),
                hint: Some(words::admin_change_password(lang)),
                mark: glyph::LOCK,
                auto: "current-password",
                max: "1024",
            };
            let submit = move |event: leptos::ev::SubmitEvent| {
                event.prevent_default();
                let password = p.password.get_untracked();
                if password.is_empty() {
                    p.wrong
                        .set(Some((which, words::admin_password_missing(lang))));
                    focus::after_render("#admin-password");
                    return;
                }
                p.password.set(String::new());
                p.submit(which, Some(password));
            };
            view! {
                <form class="form admin-confirm" method="post" novalidate=true on:submit=submit>
                    // for a password manager: whose password this is
                    <input type="text" autocomplete="username" readonly=true hidden=true value=p.cx.username.get_value() />
                    {field::labelled(lang, spec, move || p.password.get(), move |value| p.password.set(value))}
                    <p class="actions">
                        <button type="submit" class="button primary">{action}</button>
                        <button type="button" class="button quiet" on:click=move |_| p.back(which)>{words::admin_back(lang)}</button>
                    </p>
                </form>
            }
            .into_any()
        }
        Stage::Working(at) if at == which => view! {
            <p class="working" role="status" tabindex="-1" id=format!("ai-working-{}", which.key())>{words::admin_working(lang)}</p>
        }
        .into_any(),
        _ => ().into_any(),
    }
    }
}

/// What a form says of its last attempt.
fn wrong_line(p: &Page, which: Which) -> impl IntoView + use<> {
    let p = *p;
    view! {
        <p class="error" role="alert" id=format!("ai-wrong-{}", which.key()) tabindex="-1">
            {move || p.wrong.get().and_then(|(at, text)| (at == which).then_some(text))}
        </p>
    }
}

/// Whether a form's own submit button stands: not while its password is
/// asked for or it is on its way.
fn idle(p: &Page, which: Which) -> impl Fn() -> bool + Send + Sync + 'static {
    let p = *p;
    move || !matches!(p.stage.get(), Stage::Password(at) | Stage::Working(at) if at == which)
}

/// What only a new AI player chooses: its tag, its kind and its profile.
fn chosen_view(p: &Page) -> impl IntoView + use<> {
    let p = *p;
    let lang = p.cx.lang;
    let tag = Labelled {
        id: "ai-new-tag",
        label: words::ai_tag(lang),
        hint: Some(words::ai_tag_hint(lang)),
        mark: glyph::HASH,
        auto: "off",
        max: "9",
    };
    let profile = Labelled {
        id: "ai-new-profile",
        label: words::ai_profile(lang),
        hint: Some(words::ai_profile_hint(lang)),
        mark: glyph::BOT,
        auto: "off",
        max: "61",
    };
    let kinds = KINDS
        .into_iter()
        .map(|kind| {
            view! {
                <option value=kind_key(kind)
                    prop:selected=move || p.new.with(|fields| fields.kind == kind)>{kind_text(lang, kind)}</option>
            }
        })
        .collect_view();
    let choose = move |ev: leptos::ev::Event| {
        let typed = event_target_value(&ev);
        let chosen = KINDS
            .into_iter()
            .find(|kind| kind_key(*kind) == typed)
            .unwrap_or(AiKind::House);
        p.new.update(|fields| {
            // the profile's family follows the kind while it is all there is
            let family = |kind: AiKind| format!("{}:", kind.as_str().unwrap_or("house"));
            if fields.profile == family(fields.kind) || fields.profile.is_empty() {
                fields.profile = family(chosen);
            }
            fields.kind = chosen;
        });
    };
    view! {
        {field::labelled(lang, tag, move || p.new.with(|f| f.tag.clone()), move |value| p.new.update(|f| f.tag = value))}
        <div class="field">
            <label for="ai-new-kind">{words::ai_kind(lang)}</label>
            <select id="ai-new-kind" aria-describedby="ai-new-kind-hint" on:change=choose>
                {kinds}
            </select>
            <p class="hint" id="ai-new-kind-hint">{words::ai_kind_hint(lang)}</p>
        </div>
        {field::labelled(lang, profile, move || p.new.with(|f| f.profile.clone()), move |value| p.new.update(|f| f.profile = value))}
    }
}

/// An AI player's fields: the new one's (`edit: None`) or the one being
/// changed, whose kind and profile stay.
fn fields_view(p: &Page, which: Which) -> impl IntoView + use<> {
    let p = *p;
    let lang = p.cx.lang;
    let key = which.key();
    let get = move || match which {
        Which::Edit(_) => p
            .edit
            .with(|edit| edit.as_ref().map(|(_, fields)| fields.clone()))
            .unwrap_or_else(Fields::fresh),
        _ => p.new.get(),
    };
    let put = move |change: &dyn Fn(&mut Fields)| match which {
        Which::Edit(_) => p.edit.update(|edit| {
            if let Some((_, fields)) = edit {
                change(fields);
            }
        }),
        _ => p.new.update(|fields| change(fields)),
    };
    let creating = which == Which::Create;
    let id = |new: &'static str, edit: &'static str| if creating { new } else { edit };
    let name = Labelled {
        id: id("ai-new-name", "ai-edit-name"),
        label: words::ai_name(lang),
        hint: Some(words::ai_name_hint(lang)),
        mark: glyph::ID_CARD,
        auto: "off",
        max: "16",
    };
    let label = Labelled {
        id: id("ai-new-label", "ai-edit-label"),
        label: words::ai_label(lang),
        hint: Some(words::ai_label_hint(lang)),
        mark: glyph::PENCIL,
        auto: "off",
        max: "80",
    };
    let fixed = (!creating).then(|| {
        let fields = get();
        view! {
            <dl class="facts">
                <dt>{words::ai_kind(lang)}</dt>
                <dd>{kind_text(lang, fields.kind)}</dd>
                <dt>{words::ai_profile(lang)}</dt>
                <dd><code translate="no">{fields.profile}</code></dd>
            </dl>
            <p class="meta">{words::ai_edit_lead(lang)}</p>
        }
    });
    let chosen = creating.then(|| chosen_view(&p));
    let games = Number {
        id: format!("ai-{key}-games"),
        label: words::ai_max_games(lang),
        hint: words::ai_max_games_hint(lang),
        min: "1",
        max: "16",
    };
    let daily = Number {
        id: format!("ai-{key}-daily"),
        label: words::ai_daily_games(lang),
        hint: words::ai_daily_games_hint(lang),
        min: "0",
        max: "10000",
    };
    let get_name = get;
    let get_label = get;
    let get_games = get;
    let get_daily = get;
    view! {
        {fixed}
        {field::labelled(lang, name, move || get_name().name, move |value| put(&move |f: &mut Fields| f.name.clone_from(&value)))}
        {chosen}
        {field::labelled(lang, label, move || get_label().label, move |value| put(&move |f: &mut Fields| f.label.clone_from(&value)))}
        <div class="ai-numbers">
            {number_field(games, move || get_games().max_games, move |value| put(&move |f: &mut Fields| f.max_games.clone_from(&value)))}
            {number_field(daily, move || get_daily().daily, move |value| put(&move |f: &mut Fields| f.daily.clone_from(&value)))}
        </div>
    }
}

/// The new AI player's card.
fn create_view(p: &Page) -> impl IntoView + use<> {
    let p = *p;
    let lang = p.cx.lang;
    let which = Which::Create;
    view! {
        <section class="card rt-making" aria-labelledby="ai-new-title">
            <h2 id="ai-new-title" tabindex="-1">{words::ai_new(lang)}</h2>
            <form class="form" method="post" novalidate=true on:submit=move |event: leptos::ev::SubmitEvent| {
                event.prevent_default();
                p.submit(which, None);
            }>
                {fields_view(&p, which)}
                {wrong_line(&p, which)}
                <p class="actions" hidden=move || !idle(&p, which)()>
                    <button type="submit" class="button primary" id="ai-submit-new">{glyph::glyph(glyph::PLUS)}<span>{words::ai_create(lang)}</span></button>
                </p>
            </form>
            {password_step(&p, which, words::ai_create(lang))}
        </section>
    }
}

/// What an AI player is, and what it does now.
fn facts_view(lang: Lang, player: &AdminAiPlayer) -> impl IntoView + use<> {
    let daily = player.settings.daily_games.map_or_else(
        || words::ai_daily_none(lang).to_owned(),
        |games| words::number(lang, u64::from(games)),
    );
    let offers = if player.offers.is_empty() {
        words::ai_offers_none(lang).to_owned()
    } else {
        player
            .offers
            .iter()
            .map(|format| format_name(*format))
            .collect::<Vec<_>>()
            .join(" · ")
    };
    let signed_in = player
        .last_sign_in
        .map_or_else(|| words::ai_never(lang).to_owned(), |at| when(lang, at));
    view! {
        <dl class="facts">
            <dt>{words::ai_kind(lang)}</dt>
            <dd>{kind_text(lang, player.settings.kind)}</dd>
            <dt>{words::ai_profile(lang)}</dt>
            <dd><code translate="no">{player.settings.profile.clone()}</code></dd>
            <dt>{words::ai_max_games(lang)}</dt>
            <dd class="num">{words::number(lang, u64::from(player.settings.max_games))}</dd>
            <dt>{words::ai_daily_games(lang)}</dt>
            <dd class="num">{daily}</dd>
            <dt>{words::ai_live(lang)}</dt>
            <dd class="num">{words::number(lang, u64::from(player.live_matches))}</dd>
            <dt>{words::ai_today(lang)}</dt>
            <dd class="num">{words::number(lang, u64::from(player.games_today))}</dd>
            <dt>{words::ai_offers(lang)}</dt>
            <dd translate="no">{offers}</dd>
            <dt>{words::ai_last_sign_in(lang)}</dt>
            <dd>{signed_in}</dd>
            <dt>{words::admin_created(lang)}</dt>
            <dd>{when(lang, player.set_at)}</dd>
        </dl>
    }
}

/// The form that changes the AI player `account`, in its card.
fn edit_view(p: &Page, account: [u8; 16]) -> impl IntoView + use<> {
    let p = *p;
    let lang = p.cx.lang;
    let which = Which::Edit(account);
    view! {
        <form class="form ai-edit" method="post" novalidate=true aria-labelledby="ai-edit-title"
            on:submit=move |event: leptos::ev::SubmitEvent| {
                event.prevent_default();
                p.submit(which, None);
            }>
            <h3 id="ai-edit-title" tabindex="-1">{words::ai_edit_title(lang)}</h3>
            {fields_view(&p, which)}
            {wrong_line(&p, which)}
            <p class="actions" hidden=move || !idle(&p, which)()>
                <button type="submit" class="button" id=format!("ai-submit-{}", which.key())>{words::ai_save(lang)}</button>
                <button type="button" class="button quiet" on:click=move |_| {
                    p.edit.set(None);
                    p.wrong.set(None);
                    focus::after_render(format!("#ai-change-{}", id_hex(&account)));
                }>{words::admin_back(lang)}</button>
            </p>
        </form>
        {password_step(&p, which, words::ai_save(lang))}
    }
}

/// An AI player's card: its state, its numbers and its changes.
fn card(p: &Page, player: &AdminAiPlayer) -> impl IntoView + use<> {
    let p = *p;
    let lang = p.cx.lang;
    let account = player.account;
    let key = id_hex(&account);
    let heading = format!("ai-player-{key}");
    let badge = AiBadge {
        kind: player.settings.kind,
        label: player.settings.label.clone(),
    };
    let (state, tone) = if player.enabled {
        (words::ai_on(lang), "ok")
    } else {
        (words::ai_off(lang), "off")
    };
    let enabled = player.enabled;
    let switched = player.clone();
    let removed = player.clone();
    let changed = player.clone();
    let (switch_class, switch_label) = if enabled {
        ("button danger", words::ai_switch_off(lang))
    } else {
        ("button", words::ai_switch_on(lang))
    };
    // a memo: typing in the form changes its fields, and must not draw it again
    let editing = Memo::new(move |_| {
        p.edit
            .with(|edit| edit.as_ref().is_some_and(|(at, _)| *at == account))
    });
    let edit_form = move || editing.get().then(|| edit_view(&p, account));
    view! {
        <li class="card admin-gw" data-standing=tone>
            <div class="admin-gw-head">
                <div class="admin-gw-who">
                    <h3 class="admin-gw-name" id=heading.clone() tabindex="-1">
                        <a href=common::user_path(lang, &account)><bdi translate="no">{name_of(player)}</bdi></a>
                    </h3>
                    <p class="pills">{ai_badge(lang, &badge)}</p>
                </div>
                <p class="pills"><span class=format!("pill {tone}")>{state}</span></p>
            </div>
            {facts_view(lang, player)}
            <p class="actions" hidden=move || editing.get()>
                <button type="button" class=switch_class id=format!("ai-switch-{key}") aria-describedby=heading.clone()
                    on:click=move |_| p.switch(&switched, !enabled)>{switch_label}</button>
                <button type="button" class="button quiet" id=format!("ai-change-{key}") aria-describedby=heading.clone()
                    on:click=move |_| p.change(&changed)>{glyph::glyph(glyph::PENCIL)}<span>{words::ai_change(lang)}</span></button>
                <button type="button" class="button danger" id=format!("ai-remove-{key}") aria-describedby=heading
                    on:click=move |_| p.remove(&removed)>{glyph::glyph(glyph::TRASH_2)}<span>{words::ai_remove(lang)}</span></button>
            </p>
            {edit_form}
            {p.flow.panel(move |change| matches!(
                change,
                Change::AiEnabled { account: at, .. } | Change::RemoveAi { account: at } if *at == account
            ))}
        </li>
    }
}

/// The settings for every AI, and the way to stop them all.
fn settings_view(p: &Page) -> impl IntoView + use<> {
    let p = *p;
    let lang = p.cx.lang;
    let which = Which::Settings;
    let field = move |id: &'static str,
                      label: &'static str,
                      hint: &'static str,
                      (min, max): (&'static str, &'static str),
                      get: fn(&SettingsFields) -> String,
                      put: fn(&mut SettingsFields, String)| {
        number_field(
            Number {
                id: id.to_owned(),
                label,
                hint,
                min,
                max,
            },
            move || p.set.with(get),
            move |value| p.set.update(|fields| put(fields, value)),
        )
    };
    let state = move || {
        p.settings.get().map(|(settings, changed_at)| {
            let stopped = (settings.global_seats == 0)
                .then(|| view! { <p class="notice">{words::ai_stopped(lang)}</p> });
            let changed = changed_at.map_or_else(
                || words::ai_settings_never(lang).to_owned(),
                |at| words::ai_settings_changed(lang, &when(lang, at)),
            );
            view! {
                {stopped}
                <p class="meta">{changed}</p>
            }
        })
    };
    view! {
        <section class="card" aria-labelledby="ai-settings-title">
            <h2 id="ai-settings-title">{words::ai_settings(lang)}</h2>
            <p>{words::ai_settings_lead(lang)}</p>
            {state}
            <form class="form" method="post" novalidate=true on:submit=move |event: leptos::ev::SubmitEvent| {
                event.prevent_default();
                p.submit(which, None);
            }>
                <div class="ai-numbers">
                    {field("ai-fill-in-after", words::ai_fill_in_after(lang), words::ai_fill_in_after_hint(lang), ("30", "600"),
                        |f| f.fill_in_after.clone(), |f, v| f.fill_in_after = v)}
                    {field("ai-drain", words::ai_drain(lang), words::ai_drain_hint(lang), ("30", "600"),
                        |f| f.drain_minutes.clone(), |f, v| f.drain_minutes = v)}
                    {field("ai-global-seats", words::ai_global_seats(lang), words::ai_global_seats_hint(lang), ("0", "16"),
                        |f| f.global_seats.clone(), |f, v| f.global_seats = v)}
                    {field("ai-host-reserve", words::ai_host_reserve(lang), words::ai_host_reserve_hint(lang), ("0", "64"),
                        |f| f.host_reserve.clone(), |f, v| f.host_reserve = v)}
                    {field("ai-carousel-games", words::ai_carousel_games(lang), words::ai_carousel_games_hint(lang), ("0", "8"),
                        |f| f.carousel_games.clone(), |f, v| f.carousel_games = v)}
                    {field("ai-carousel-daily", words::ai_carousel_daily(lang), words::ai_carousel_daily_hint(lang), ("0", "10000"),
                        |f| f.carousel_daily.clone(), |f, v| f.carousel_daily = v)}
                </div>
                {wrong_line(&p, which)}
                <p class="actions" hidden=move || !idle(&p, which)()>
                    <button type="submit" class="button" id="ai-submit-settings">{words::ai_settings_save(lang)}</button>
                    <button type="button" class="button danger" id="ai-stop-all" on:click=move |_| p.stop_all()>{words::ai_stop_all(lang)}</button>
                </p>
            </form>
            {password_step(&p, which, words::ai_settings_save(lang))}
            {p.flow.panel(|change| *change == Change::StopAllAi)}
        </section>
    }
}

fn players_view(p: &Page) -> impl IntoView + use<> {
    let p = *p;
    let lang = p.cx.lang;
    move || {
        let players = p.players.get();
        if players.is_empty() {
            return view! { <p role="status">{words::ai_none(lang)}</p> }.into_any();
        }
        let count = words::number(lang, u64::try_from(players.len()).unwrap_or(u64::MAX));
        let cards = players.iter().map(|player| card(&p, player)).collect_view();
        view! {
            <p class="meta" role="status">{words::admin_count(lang, &count)}</p>
            <ul class="admin-gws" id="ai-players">{cards}</ul>
        }
        .into_any()
    }
}

/// While the page is read, or where it could not be: the words and the way
/// to try again. What it shows stands drawn once, hidden until it is read,
/// so that a read after a change keeps the forms and the keyboard where
/// they are.
fn results(p: &Page) -> impl IntoView + use<> {
    let p = *p;
    let lang = p.cx.lang;
    let state = move || match p.listing.get() {
        Listing::Loading => {
            view! { <p role="status" class="meta">{words::account_loading(lang)}</p> }.into_any()
        }
        Listing::Failed(text) => view! {
            <p class="error" role="alert">{text}</p>
            <p class="actions">
                <button type="button" class="button" on:click=move |_| {
                    p.listing.set(Listing::Loading);
                    p.read(true, None);
                }>{words::error_again(lang)}</button>
            </p>
        }
        .into_any(),
        Listing::Ready => ().into_any(),
    };
    view! {
        {state}
        <div class="ai-stack" hidden=move || p.listing.get() != Listing::Ready>
            <section aria-labelledby="ai-players-title" class="ai-stack">
                <h2 id="ai-players-title">{words::ai_players(lang)}</h2>
                {players_view(&p)}
            </section>
            {create_view(&p)}
            {settings_view(&p)}
            {tokens::section(&p)}
        </div>
    }
}

/// The page: what AI players are, the players, a new one, the settings for
/// every AI and the runner tokens.
pub fn page(cx: Cx) -> impl IntoView {
    let lang = cx.lang;
    let note = RwSignal::new(None::<Note>);
    let ended = StoredValue::new(None::<Page>);
    let finished = Callback::new(move |(change, came): (Change, Came)| {
        if let Some(Some(p)) = ended.try_get_value() {
            p.read(change == Change::StopAllAi, Some(came));
        }
    });
    let p = Page {
        cx,
        players: RwSignal::new(Vec::new()),
        settings: RwSignal::new(None),
        tokens: RwSignal::new(Vec::new()),
        listing: RwSignal::new(Listing::Loading),
        generation: StoredValue::new(0),
        stage: RwSignal::new(Stage::Idle),
        password: RwSignal::new(String::new()),
        wrong: RwSignal::new(None),
        busy: StoredValue::new(false),
        new: RwSignal::new(Fields::fresh()),
        edit: RwSignal::new(None),
        set: RwSignal::new(SettingsFields::default()),
        token: RwSignal::new(TokenFields::default()),
        shown: RwSignal::new(None),
        flow: Flow::new(cx, note, finished),
    };
    ended.set_value(Some(p));
    // Effects run in the browser only.
    Effect::new(move |_| p.read(true, None));
    #[cfg(target_arch = "wasm32")]
    {
        let away = window_event_listener_untyped("pagehide", move |_| p.forget());
        on_cleanup(move || away.remove());
    }
    view! {
        <p class="admin-lead">{words::ai_lead(lang)}</p>
        {p.flow.notes()}
        <div class="admin-results ai-stack" aria-busy=move || (p.listing.get() == Listing::Loading).then_some("true")>
            {results(&p)}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn typed() -> Fields {
        Fields {
            name: "KI-Haus-Expert".into(),
            label: "Haus-KI · Expert".into(),
            profile: "house:expert".into(),
            ..Fields::fresh()
        }
    }

    /// A taken tag, a full table and a lost answer each have their words;
    /// Central's refusal is explained, never said as it came.
    #[test]
    fn what_a_failed_form_means() {
        for failure in [Failure::Unavailable, Failure::Malformed, Failure::Network] {
            assert_eq!(at_form(Which::Create, failure), Some(Came::Lost));
        }
        assert_eq!(at_form(Which::Create, Failure::Rejected), None);
        assert_eq!(at_form(Which::Create, Failure::Conflict), Some(Came::Taken));
        assert_eq!(
            at_form(Which::Settings, Failure::Conflict),
            Some(Came::Other(Problem::Failed))
        );
        for lang in Lang::ALL {
            let taken = form_text(lang, Which::Create, Came::Taken);
            let full = form_text(lang, Which::Token, Came::Other(Problem::Busy));
            let lost = form_text(lang, Which::Token, Came::Lost);
            let changed = form_text(lang, Which::Settings, Came::Lost);
            let texts = [taken, full, lost, changed];
            for (at, text) in texts.iter().enumerate() {
                assert!(text.is_some_and(|text| !text.is_empty()), "{lang:?}");
                assert!(!texts[..at].contains(text), "{lang:?}: {text:?}");
            }
            for came in [Came::Password, Came::SignedOut, Came::Done] {
                assert_eq!(form_text(lang, Which::Create, came), None);
            }
        }
    }

    /// The forms' requests, as Central reads them.
    #[test]
    fn the_requests_serialize_as_central_reads_them() {
        let text = |request: AdminRequest| {
            serde_json::to_string(&Request::Admin { request }).expect("json")
        };
        let settings = typed().settings(Lang::En).expect("valid");
        assert_eq!(
            text(AdminRequest::CreateAiPlayer {
                settings: settings.clone(),
                tag: Some(0x1a2b),
            }),
            r#"{"operation":"admin","request":{"action":"create_ai_player","settings":{"kind":"house","profile":"house:expert","label":"Haus-KI · Expert","max_games":1,"daily_games":null,"display_name":"KI-Haus-Expert"},"tag":6699}}"#
        );
        assert!(
            text(AdminRequest::SetAiPlayer {
                account: [2; 16],
                settings,
            })
            .starts_with(
                r#"{"operation":"admin","request":{"action":"set_ai_player","account":[2,"#
            )
        );
        let fresh_needed = [
            AdminRequest::CreateAiPlayer {
                settings: typed().settings(Lang::En).expect("valid"),
                tag: None,
            },
            AdminRequest::SetAiSettings {
                settings: SettingsFields::of(&AiSettings {
                    fill_in: true,
                    fill_in_after: 60,
                    drain_minutes: 120,
                    global_seats: 8,
                    host_reserve: 2,
                    carousel_games: 0,
                    carousel_daily: 0,
                })
                .settings()
                .expect("valid"),
            },
        ];
        assert!(fresh_needed.iter().all(AdminRequest::fresh));
    }

    /// Every word the page says of an AI player's state is its own.
    #[test]
    fn the_states_have_their_own_words() {
        for lang in Lang::ALL {
            assert_ne!(words::ai_on(lang), words::ai_off(lang));
            assert_ne!(words::ai_switch_on(lang), words::ai_switch_off(lang));
            assert_ne!(words::ai_daily_none(lang), "");
            assert_ne!(words::ai_offers_none(lang), "");
        }
        assert_eq!(format_name(Format::Commander), "Commander");
    }
}
