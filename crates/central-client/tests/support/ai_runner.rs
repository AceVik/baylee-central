//! AI runner tokens over the real listeners (`docs/ai-players.md` §2): an
//! administrator's website session makes one, a runner trades it for a
//! session of the AI account it lists on the native listener, without a
//! bearer, at most twelve times an hour for the account; the website never
//! takes the exchange; a revocation ends the runner's session at once.
use super::{
    Http,
    browser::Browser,
    password::{OLD, browser_sign_in, close, context, moved, sign_in},
};
use central_api::{
    Bearer, Error, Token,
    admin::{AdminAnswer, AdminRequest, AiPlayerSettings},
    ai_runner::{EXCHANGES_PER_HOUR, token_text},
    ranked::ai::{AiKind, Request as AiRequest, Response as AiResponse},
    wire::{Identity, Request, Response},
};
use central_client::{Transport, ai_runner::parse_token_text, ai_runner_session};
use central_postgres::appoint_admin;
use sea_orm::DatabaseConnection;
use tokio::runtime::Runtime;

/// The administrator's website session.
struct Desk<'a> {
    browser: &'a Browser,
    cookie: Token,
}

impl Desk<'_> {
    fn ask(&self, request: AdminRequest) -> Result<Response, Error> {
        self.browser
            .answer(&Request::Admin { request }, Some(&self.cookie))
    }

    fn done(&self, request: AdminRequest) {
        assert!(matches!(self.ask(request), Ok(Response::Done)));
    }
}

/// An AI account, made the one way there is until the creation path lands:
/// a moved account an administrator flags before it ever played. Its own
/// session ends, so it holds runner sessions alone.
fn ai_account(http: &Http, bridge: &Token, desk: &Desk<'_>, legacy: &str) -> Identity {
    let (identity, session) = moved(http, bridge, legacy);
    close(http, bridge, legacy, identity);
    session.logout(http).unwrap();
    desk.done(AdminRequest::SetAiPlayer {
        account: identity.account,
        settings: Some(AiPlayerSettings {
            kind: AiKind::House,
            profile: "house:expert".into(),
            label: "Haus-KI · Expert".into(),
            max_games: 1,
            daily_games: None,
            display_name: "KI-Runner-SDK".into(),
        }),
    });
    desk.done(AdminRequest::SetAiEnabled {
        account: identity.account,
        enabled: true,
    });
    identity
}

fn exchange(token: &Token, account: [u8; 16]) -> Request {
    Request::AiRunnerSession {
        token: Token::from_secret_bytes(token.to_secret_bytes()),
        account,
    }
}

pub(crate) fn run(
    http: &Http,
    browser: &Browser,
    bridge: &Token,
    runtime: &Runtime,
    operator: &DatabaseConnection,
) {
    let browser = Browser {
        address: browser.address,
        client: "198.51.100.44",
    };
    let (admin, session) = moved(http, bridge, "runner-tokens-web");
    close(http, bridge, "runner-tokens-web", admin);
    drop(session);
    runtime
        .block_on(appoint_admin(operator, "runner-tokens-web"))
        .unwrap();
    let desk = Desk {
        cookie: browser_sign_in(&browser, admin, context(http, admin), OLD),
        browser: &browser,
    };
    let bot = ai_account(http, bridge, &desk, "runner-tokens-bot");
    let Ok(Response::Admin {
        answer: AdminAnswer::AiRunnerTokenCreated { id, token, .. },
    }) = desk.ask(AdminRequest::CreateAiRunnerToken {
        name: "Runner SDK".into(),
        accounts: vec![bot.account],
        days: 90,
    })
    else {
        panic!("a runner token");
    };
    // The runner reads it from the text the website shows.
    let token = parse_token_text(&token_text(&token)).unwrap();

    let runner = ai_runner_session(http, &token, bot.account).unwrap();
    assert!(runner.is_runner());
    assert_eq!(runner.identity().account, bot.account);
    runner.check_active(http).unwrap();
    runner.renew(http).unwrap();
    let AiResponse::Work { settings, .. } = runner.ai(http, AiRequest::Work).unwrap() else {
        panic!("work");
    };
    assert_eq!(settings.profile, "house:expert");
    // Nothing that needs the master key, and nothing that needs a restore.
    assert_eq!(runner.chat_identity().err(), Some(Error::Rejected));
    assert!(runner.tower_token(http).is_err());

    // Not with a bearer of either kind, not on the website's listener.
    let (person_bearer, _) = sign_in(http, admin, OLD);
    let bearers: [Bearer<'_>; 2] = [(&person_bearer).into(), bridge.into()];
    for bearer in bearers {
        assert_eq!(
            http.exchange(&exchange(&token, bot.account), Some(bearer))
                .err(),
            Some(Error::Rejected)
        );
    }
    assert!(
        browser
            .answer(&exchange(&token, bot.account), None)
            .is_err()
    );
    // Neither for another account nor with another token.
    for (presented, account) in [
        (&token, admin.account),
        (&Token::generate().unwrap(), bot.account),
    ] {
        assert_eq!(
            ai_runner_session(http, presented, account).err(),
            Some(Error::Rejected)
        );
    }
    // Twelve sessions an hour for the account; the thirteenth waits, and
    // the refusals before spent none of them.
    let held: Vec<_> = (1..EXCHANGES_PER_HOUR)
        .map(|_| ai_runner_session(http, &token, bot.account).unwrap())
        .collect();
    assert_eq!(
        ai_runner_session(http, &token, bot.account).err(),
        Some(Error::Busy)
    );
    // The first was ended by the fifth: at most four live at once.
    assert_eq!(runner.renew(http).err(), Some(Error::Rejected));
    let live = &held[held.len() - 4..];
    for session in live {
        session.renew(http).unwrap();
    }

    // A revocation ends the runner's sessions, each at its very next request.
    desk.done(AdminRequest::RevokeAiRunnerToken { token: id });
    for session in live {
        assert_eq!(session.renew(http).err(), Some(Error::Rejected));
        assert!(session.check_active(http).is_err());
    }
    let Ok(Response::Admin {
        answer: AdminAnswer::AiRunnerTokens { tokens },
    }) = desk.ask(AdminRequest::AiRunnerTokens)
    else {
        panic!("the tokens");
    };
    let listed = tokens.iter().find(|t| t.id == id).unwrap();
    assert!(listed.revoked_at.is_some() && listed.last_used_at.is_some());
    assert_eq!(listed.accounts.len(), 1);
}
