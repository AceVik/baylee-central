//! Real PostgreSQL accounting tests. Synthetic packages exercise admission, not engine proofs.
use super::{enroll, login};
use central_api::{Error, Token, challenges::*, wire::Identity};
use central_crypto::opaque::server::ServerKeys;
use central_postgres::{AccountStore, Session};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};
use std::sync::atomic::{AtomicU64, Ordering};

mod boundaries;
mod edges;
mod results;
mod tower;

const HOST: &str = "http://127.0.0.1:28780";
const EXECUTOR: &str = "synthetic-challenge-test-v1";

fn id() -> [u8; 16] {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let mut bytes = [0; 16];
    bytes[6] = 0x70;
    bytes[8..].copy_from_slice(&NEXT.fetch_add(1, Ordering::Relaxed).to_be_bytes());
    bytes[8] |= 0x80;
    bytes
}

fn words(text: &str) -> Words {
    Words {
        en: text.into(),
        de: text.into(),
    }
}

fn publication(kind: Kind, now: i64) -> Publication {
    let mut value = Publication {
        description: Description {
            period: Period::containing(kind, now).unwrap(),
            title: words("Synthetic challenge"),
            instructions: words("Win the synthetic test."),
            deck: "60 Synthetic card".into(),
            executor: EXECUTOR.into(),
            target_turn: 5,
            expected_minutes: 20,
        },
        digest: [0; 32],
        package: b"synthetic-private-execution-package".to_vec(),
        certificate: Certificate {
            validator: "synthetic-test-only".into(),
            witnesses: (1..=3)
                .map(|n| Witness {
                    record_digest: [n; 32],
                    strategy_digest: [n + 10; 32],
                    family: if n == 1 {
                        SolutionFamily::Combat
                    } else {
                        SolutionFamily::DirectDamage
                    },
                })
                .collect(),
        },
        hints: [words("Synthetic hint one"), words("Synthetic hint two")],
    };
    value.digest = value.calculated_digest().unwrap();
    value
}

async fn public(
    store: &AccountStore,
    token: Option<&Token>,
    request: Request,
) -> Result<Response, Error> {
    store
        .challenges(token.map(Session::Native), request, HOST)
        .await
}

async fn host(store: &AccountStore, request: HostRequest) -> Result<HostResponse, Error> {
    store.official_challenges(request, HOST).await
}

async fn publish(store: &AccountStore, value: Publication) -> Challenge {
    let HostResponse::Challenge { challenge } = host(
        store,
        HostRequest::Publish {
            publication: Box::new(value),
        },
    )
    .await
    .unwrap() else {
        panic!("publication response");
    };
    challenge
}

async fn begin(store: &AccountStore, token: &Token, task: &Challenge) -> Attempt {
    let Response::Attempt { attempt } = public(
        store,
        Some(token),
        Request::Begin {
            challenge: task.id,
            request_id: id(),
            listed: true,
        },
    )
    .await
    .unwrap() else {
        panic!("attempt response");
    };
    attempt
}

async fn claim(store: &AccountStore, attempt: Attempt, owner: Identity) -> Attempt {
    let HostResponse::Claimed { attempt, .. } = host(
        store,
        HostRequest::Claim {
            attempt: attempt.id,
            account: owner.account,
            game: id(),
            executor: EXECUTOR.into(),
        },
    )
    .await
    .unwrap() else {
        panic!("claim response");
    };
    attempt
}

fn completion(
    attempt: &Attempt,
    state: AttemptState,
    turn: u32,
    milliseconds: Option<u64>,
) -> Completion {
    Completion {
        attempt: attempt.id,
        game: attempt.game.unwrap(),
        state,
        turn,
        finished_at: i64::try_from(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        )
        .unwrap(),
        milliseconds,
        record_digest: [71; 32],
    }
}

async fn finish(store: &AccountStore, result: Completion) -> Attempt {
    let HostResponse::Attempt { attempt } =
        host(store, HostRequest::Finish { result }).await.unwrap()
    else {
        panic!("result response");
    };
    attempt
}

async fn board(store: &AccountStore, task: &Challenge, kind: Board) -> Vec<Standing> {
    let Response::Detail { standings, .. } = public(
        store,
        None,
        Request::Detail {
            challenge: task.id,
            board: kind,
            offset: 0,
        },
    )
    .await
    .unwrap() else {
        panic!("ranking response");
    };
    standings
}

async fn badges(store: &AccountStore, token: &Token) -> Vec<Badge> {
    let Response::Progress { badges, .. } =
        public(store, Some(token), Request::Progress).await.unwrap()
    else {
        panic!("progress response");
    };
    badges
}

pub(super) async fn run(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    let now: i64 = operator
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT floor(extract(epoch FROM pg_catalog.clock_timestamp()))::bigint AS now",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "now")
        .unwrap();
    let daily = publish(store, publication(Kind::Daily, now)).await;
    let weekly = publish(store, publication(Kind::Weekly, now)).await;
    results::run(store, keys, &daily, &weekly).await;
    edges::run(store, keys, now, &daily, &weekly).await;
    boundaries::run(store, operator, keys, now).await;
    tower::run(store, operator, keys, now).await;
}

pub(super) async fn tower_only(
    store: &AccountStore,
    operator: &DatabaseConnection,
    keys: &ServerKeys,
) {
    let now: i64 = operator
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT floor(extract(epoch FROM pg_catalog.clock_timestamp()))::bigint AS now",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "now")
        .unwrap();
    tower::run(store, operator, keys, now).await;
}
