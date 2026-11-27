//! AI runner tokens over the real listeners (`docs/ai-players.md` §2): an
//! administrator's website session makes one, a runner trades it for a
//! session of the AI account it lists on the native listener, without a
//! bearer, at most twelve times an hour for the account; the website never
//! takes the exchange; a revocation ends the runner's session at once.
use super::{
    Http,
    browser::Browser,
    pace,
    password::{OLD, browser_sign_in, close, context, moved, sign_in},
    push::dial,
};
use central_api::{
    Bearer, Error, SignedToken, Token,
    admin::{AdminAnswer, AdminRequest, AiPlayerSettings},
    advent,
    ai_runner::{EXCHANGES_PER_HOUR, token_text},
    challenges,
    email::EmailRequest,
    gateway::GatewayMetadata,
    presence::Choice,
    ranked::{
        self,
        ai::{AiKind, Offer, Request as AiRequest, Response as AiResponse, Unseated},
        replica::SeatRequest,
    },
    reset_code::ResetCode,
    social::SocialRequest,
    terms::{TermsSubject, TermsVersion},
    wire::{Identity, Request, Response},
};
use central_client::{Transport, ai_runner::parse_token_text, ai_runner_session};
use central_crypto::opaque::{Enrollment, Login};
use central_identity::Delivery;
use central_postgres::appoint_admin;
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement, Value, prelude::Uuid};
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

/// An AI account as an administrator creates it on the website, switched
/// on: it has no password, so every session it holds is a runner's.
fn ai_account(desk: &Desk<'_>, name: &str) -> Identity {
    let Ok(Response::Admin {
        answer: AdminAnswer::AiPlayerCreated { account, .. },
    }) = desk.ask(AdminRequest::CreateAiPlayer {
        settings: AiPlayerSettings {
            kind: AiKind::House,
            profile: "house:expert".into(),
            label: "Haus-KI · Expert".into(),
            max_games: 1,
            daily_games: None,
            display_name: name.into(),
        },
        tag: None,
    })
    else {
        panic!("an AI player");
    };
    desk.done(AdminRequest::SetAiEnabled {
        account,
        enabled: true,
    });
    Identity { account, epoch: 1 }
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
    let bot = ai_account(&desk, "KI-Runner-SDK");
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

    refusing(http, &desk, runtime, operator, (admin, &person_bearer));
}

/// A request as the wire writes it, for a failure's message.
fn named(request: &Request) -> String {
    String::from_utf8(central_api::encode(request).unwrap()).unwrap()
}

/// A runner session of a new AI account, by hand: its signed bearer.
fn runner_bearer(http: &Http, desk: &Desk<'_>, account: [u8; 16]) -> SignedToken {
    let Ok(Response::Admin {
        answer: AdminAnswer::AiRunnerTokenCreated { token, .. },
    }) = desk.ask(AdminRequest::CreateAiRunnerToken {
        name: "Runner refusals".into(),
        accounts: vec![account],
        days: 1,
    })
    else {
        panic!("a runner token");
    };
    let Ok(Response::AiRunnerSession { token, .. }) =
        http.exchange(&exchange(&token, account), None)
    else {
        panic!("a runner session");
    };
    token
}

/// One statement by the operator.
fn operated(runtime: &Runtime, operator: &DatabaseConnection, sql: &str, values: Vec<Value>) {
    runtime
        .block_on(operator.execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            sql,
            values,
        )))
        .unwrap();
}

/// A runner session of an AI account, and what the operator sees of it.
struct Runner<'a> {
    http: &'a Http,
    runtime: &'a Runtime,
    operator: &'a DatabaseConnection,
    ai: Identity,
    signed: SignedToken,
    /// Its login name, which nobody is shown.
    name: String,
}

impl Runner<'_> {
    fn ask(&self, request: &Request) -> Result<Response, Error> {
        pace();
        self.http.exchange(request, Some((&self.signed).into()))
    }

    fn operated(&self, sql: &str) {
        operated(
            self.runtime,
            self.operator,
            sql,
            vec![Uuid::from_bytes(self.ai.account).into()],
        );
    }

    /// Whether the account or its address holds a reset code.
    fn holds_a_code(&self) -> bool {
        self.runtime
            .block_on(self.operator.query_one_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT (a.reset_digest IS NOT NULL OR e.reset_digest IS NOT NULL) AS held
                 FROM central_identity.accounts a
                 LEFT JOIN central_identity.account_emails e ON e.account_id = a.id
                 WHERE a.id = $1",
                vec![Uuid::from_bytes(self.ai.account).into()],
            )))
            .unwrap()
            .unwrap()
            .try_get::<bool>("", "held")
            .unwrap()
    }
}

fn address() -> Request {
    Request::Email {
        request: EmailRequest::Set {
            address: "ai-runner@example.org".into(),
            lang: "en".into(),
        },
    }
}

/// What a runner session is refused over the real listeners, and what it
/// keeps (`docs/ai-players.md` §2): first the chain a stolen runner token
/// would have used, an address and then a reset code by mail, link by link;
/// then the terms, which the runner accepts with its own session; then each
/// family of refusals, which a person's session is served; then the
/// runner's own requests.
fn refusing(
    http: &Http,
    desk: &Desk<'_>,
    runtime: &Runtime,
    operator: &DatabaseConnection,
    (admin, person): (Identity, &SignedToken),
) {
    // The person's control needs a name and a tag, which a moved account
    // waits for: social requests ask for both.
    operated(
        runtime,
        operator,
        "UPDATE central_identity.accounts SET display_name = 'RunnerDesk',
           display_name_at = pg_catalog.clock_timestamp(),
           tag = (SELECT min(t) FROM pg_catalog.generate_series(4096, 65535) t
                  WHERE NOT EXISTS (SELECT 1 FROM central_identity.accounts WHERE tag = t))
         WHERE id = $1 AND tag IS NULL",
        vec![Uuid::from_bytes(admin.account).into()],
    );
    let ai = ai_account(desk, "KI-Refused");
    let name: String = runtime
        .block_on(operator.query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT username_key FROM central_identity.accounts WHERE id = $1",
            vec![Uuid::from_bytes(ai.account).into()],
        )))
        .unwrap()
        .unwrap()
        .try_get("", "username_key")
        .unwrap();
    let runner = Runner {
        http,
        runtime,
        operator,
        ai,
        signed: runner_bearer(http, desk, ai.account),
        name,
    };
    mailing(&runner, desk);
    let current = accepting(&runner);
    families(&runner, person, &current);
    keeping(&runner);
}

/// No address, and no reset code by any way: refused as an AI before the
/// terms are asked about; an address the operator planted mails no code, an
/// administrator makes none, and a code the operator planted starts no
/// reset. Its login name is no name to sign in with.
fn mailing(runner: &Runner<'_>, desk: &Desk<'_>) {
    let http = runner.http;
    assert_eq!(runner.ask(&address()).err(), Some(Error::Rejected));
    runner.operated(
        "INSERT INTO central_identity.account_emails (account_id, address, lang)
         VALUES ($1, 'ai-runner@example.org', 'en')",
    );
    assert!(matches!(
        http.from(
            "203.0.113.241",
            &Request::ResetMail {
                username: runner.name.clone(),
                lang: "en".into(),
            }
        ),
        Ok(Response::Done)
    ));
    while let Some(delivery) = runner.runtime.block_on(http.identity.deliver_mail()) {
        assert_ne!(delivery, Delivery::Sent, "no mail for an AI");
    }
    assert!(!runner.holds_a_code());
    assert_eq!(
        desk.ask(AdminRequest::ResetLogin {
            account: runner.ai.account
        })
        .err(),
        Some(Error::Rejected)
    );
    // The desk is fresh: a change that needs it is served.
    desk.done(AdminRequest::SetAiEnabled {
        account: runner.ai.account,
        enabled: true,
    });
    assert!(!runner.holds_a_code());
    let code = ResetCode::generate().unwrap();
    operated(
        runner.runtime,
        runner.operator,
        "UPDATE central_identity.accounts SET reset_digest = $2,
           reset_expires = pg_catalog.clock_timestamp() + interval '1 hour' WHERE id = $1",
        vec![
            Uuid::from_bytes(runner.ai.account).into(),
            code.digest().to_vec().into(),
        ],
    );
    let (_, registration) = Enrollment::start(OLD.into(), context(http, runner.ai)).unwrap();
    assert_eq!(
        http.exchange(
            &Request::ResetStart {
                username: runner.name.clone(),
                code: ResetCode::parse(&code.display()).unwrap(),
                registration,
            },
            None,
        )
        .err(),
        Some(Error::Rejected)
    );
    runner.operated(
        "UPDATE central_identity.accounts SET reset_digest = NULL, reset_expires = NULL
         WHERE id = $1",
    );
    runner.operated("DELETE FROM central_identity.account_emails WHERE account_id = $1");
    let unknown = http
        .exchange(
            &Request::Discover {
                username: "ai-0000000000000000000a".into(),
            },
            None,
        )
        .map(|_| ());
    assert_eq!(
        http.exchange(
            &Request::Discover {
                username: runner.name.clone(),
            },
            None
        )
        .map(|_| ()),
        unknown
    );
}

/// An offer waits for Central's terms, and the runner accepts them with its
/// own session; accepted, it still names no address. Answers the version.
fn accepting(runner: &Runner<'_>) -> TermsVersion {
    let offer = || Request::Ranked {
        request: ranked::Request::Ai {
            request: AiRequest::Offer {
                offers: vec![Offer {
                    format: ranked::Format::Standard,
                    deck: [7; 32],
                }],
            },
        },
    };
    let carousel = || Request::Ranked {
        request: ranked::Request::Ai {
            request: AiRequest::Carousel {
                format: ranked::Format::Standard,
            },
        },
    };
    assert_eq!(runner.ask(&offer()).err(), Some(Error::TermsRequired));
    assert_eq!(runner.ask(&carousel()).err(), Some(Error::TermsRequired));
    let Ok(Response::TermsStatus {
        current: Some(current),
        accepted: false,
    }) = runner.ask(&Request::TermsStatus)
    else {
        panic!("terms in force, not accepted");
    };
    assert!(matches!(
        runner.ask(&Request::AcceptTerms {
            subject: TermsSubject::Central,
            version: current.version.clone(),
            digest: current.digest,
        }),
        Ok(Response::Done)
    ));
    assert!(matches!(
        runner.ask(&offer()),
        Ok(Response::Ranked {
            response: ranked::Response::Ai {
                response: AiResponse::Offered { until: Some(_) }
            }
        })
    ));
    // The carousel, as installed (`carousel_games` 0), tells the runner so.
    assert!(matches!(
        runner.ask(&carousel()),
        Ok(Response::Ranked {
            response: ranked::Response::Ai {
                response: AiResponse::Unseated {
                    reason: Unseated::Off
                }
            }
        })
    ));
    assert_eq!(runner.ask(&address()).err(), Some(Error::Rejected));
    current
}

/// Each family: what a person's session is served, an AI's is refused.
/// What would change the AI's own account has no person's control here and
/// is refused as well; a few of those are refused at more than one place
/// (a person's `Join`, a sealed credential's proof). No push socket.
fn families(runner: &Runner<'_>, person: &SignedToken, current: &TermsVersion) {
    let http = runner.http;
    pace();
    assert!(matches!(
        http.exchange(
            &Request::AcceptTerms {
                subject: TermsSubject::Central,
                version: current.version.clone(),
                digest: current.digest,
            },
            Some(person.into()),
        ),
        Ok(Response::Done)
    ));
    for request in &[
        Request::KeyCheckStatus,
        Request::Presence,
        Request::SetPresence {
            choice: Choice::Online,
        },
        Request::AccountExport,
        Request::Email {
            request: EmailRequest::Status,
        },
        Request::TowerProgress {},
        Request::Challenges {
            request: challenges::Request::Progress,
        },
        Request::Social {
            request: SocialRequest::Contacts,
        },
        Request::ReadDocument,
        Request::ListObjects { after: None },
        Request::CreateObject,
        Request::Ranked {
            request: ranked::Request::Export {
                table: ranked::export::Table::ALL[0],
                after: None,
            },
        },
    ] {
        pace();
        let served = http.exchange(request, Some(person.into()));
        assert!(served.is_ok(), "{}: {:?}", named(request), served.err());
        let refused = runner.ask(request).err();
        assert_eq!(refused, Some(Error::Rejected), "{}", named(request));
    }
    let (_, ke1) = Login::start(OLD.into(), context(http, runner.ai)).unwrap();
    let (_, registration) = Enrollment::start(OLD.into(), context(http, runner.ai)).unwrap();
    for request in [
        Request::SetKeyCheck { check: vec![1; 32] },
        Request::ConfirmRestore { revision: 1 },
        Request::WriteDocument {
            expected_revision: 1,
            document: [[1].as_slice(), &[0; 40]].concat(),
        },
        Request::TowerToken {},
        Request::Advent {
            request: advent::Request::CurrentCalendar {},
        },
        Request::RegisterGateway {
            origin: "https://ai-runner.example".into(),
            metadata: GatewayMetadata {
                name: "Runner".into(),
                version: "1".into(),
                website: "https://ai-runner.example/".into(),
            },
        },
        // Refused at more than one place.
        Request::ChangePasswordStart {
            login: ke1.clone(),
            registration,
        },
        Request::DeleteAccountStart { login: ke1 },
        Request::Ranked {
            request: ranked::Request::Replicate {
                request: SeatRequest::Join {
                    key: [9; 16],
                    format: ranked::Format::Standard,
                    deck: [7; 32],
                    ai: ranked::ai::AiOpponents::None,
                },
            },
        },
    ] {
        let refused = runner.ask(&request).err();
        assert_eq!(refused, Some(Error::Rejected), "{}", named(&request));
    }
    pace();
    let refused = dial(http, Bearer::from(&runner.signed).header().as_str())
        .err()
        .unwrap();
    assert_eq!(refused.status, 401, "{}", refused.head);
}

/// What a runner session keeps, and its end.
fn keeping(runner: &Runner<'_>) {
    for request in [
        Request::SessionStatus,
        Request::RenewSession,
        Request::TermsStatus,
        Request::Terms,
        Request::Ranked {
            request: ranked::Request::Status,
        },
        Request::Ranked {
            request: ranked::Request::OwnProfile { season: None },
        },
        Request::Ranked {
            request: ranked::Request::Ai {
                request: AiRequest::Work,
            },
        },
    ] {
        assert!(runner.ask(&request).is_ok(), "{}", named(&request));
    }
    assert!(matches!(runner.ask(&Request::Logout), Ok(Response::Done)));
    assert_eq!(
        runner.ask(&Request::SessionStatus).err(),
        Some(Error::Rejected)
    );
}
