//! Synthetic verified operators and native account sessions, against the migrated runtime role.
use super::{
    AccountStore, DatabaseConnection, Error, Registered, ServerKeys, Token, api, enroll, login,
    verify,
};
use api::replica as r;
use central_api::gateway::GatewayMetadata;
use sea_orm::{ConnectionTrait, DbBackend, Statement, prelude::Uuid};

mod agreement;
mod clocks;
mod game;
mod participants;
pub(super) mod privacy;
mod recovery;
const BUILD: r::Digest = [0xa1; 32];

struct Node {
    id: api::Id,
    origin: &'static str,
    token: Token,
}

impl Node {
    async fn host(
        &self,
        store: &AccountStore,
        request: r::HostRequest,
    ) -> Result<r::HostResponse, Error> {
        let response = store
            .ranked_host(
                Registered(&self.token),
                api::HostRequest::Replicate { request },
            )
            .await?;
        let api::HostResponse::Replica { response } = response else {
            panic!("expected host replica response")
        };
        Ok(response)
    }

    async fn ready(&self, store: &AccountStore, capacity: u8, compatibility: r::Digest) {
        self.host(
            store,
            r::HostRequest::Ready {
                capacity,
                compatibility,
            },
        )
        .await
        .unwrap();
    }

    async fn register(
        store: &AccountStore,
        db: &DatabaseConnection,
        owner: &Token,
        origin: &'static str,
    ) -> Self {
        let token = Token::generate().unwrap();
        let heartbeat = Token::generate().unwrap();
        let meta = GatewayMetadata {
            name: "Replica fixture".into(),
            version: "fixture-1".into(),
            website: format!("{origin}/status"),
        };
        let id = store
            .register_gateway(owner, origin, &meta, &token, &heartbeat, false)
            .await
            .unwrap();
        verify(db, id, true).await;
        Self { id, origin, token }
    }
}

struct Table<'a> {
    store: &'a AccountStore,
    db: &'a DatabaseConnection,
    seats: [(api::Id, &'a Token); 2],
    observer: &'a Token,
    first: Node,
    sibling: Node,
    second: Node,
    outsider: Node,
}

impl Table<'_> {
    async fn seat(&self, token: &Token, request: r::SeatRequest) -> Result<r::SeatResponse, Error> {
        let response = self
            .store
            .ranked(Some(token.into()), api::Request::Replicate { request })
            .await?;
        let api::Response::Replica { response } = response else {
            panic!("expected private seat response")
        };
        Ok(response)
    }

    fn host(&self, id: api::Id) -> &Node {
        [&self.first, &self.sibling, &self.second, &self.outsider]
            .into_iter()
            .find(|node| node.id == id)
            .expect("a known fixture gateway")
    }

    async fn start_pair(&self, key: u8) -> api::Assignment {
        for (offset, (_, token)) in self.seats.iter().enumerate() {
            let offset = u8::try_from(offset).unwrap();
            let response = self
                .seat(
                    token,
                    r::SeatRequest::Join {
                        key: super::id(key + offset),
                        format: api::Format::Highlander,
                        deck: [offset + 1; 32],
                        ai: api::ai::AiOpponents::None,
                    },
                )
                .await
                .unwrap();
            let r::SeatResponse::Queue { state } = response else {
                panic!("expected upgraded queue")
            };
            if let api::QueueState::Assigned { assignment, .. } = state {
                assert_eq!(offset, 1);
                return assignment;
            }
            assert!(matches!(state, api::QueueState::Waiting { .. }));
        }
        panic!("expected a verified match")
    }

    async fn status(&self, matched: api::Id) -> r::SeatStatus {
        let r::SeatResponse::Status { status } = self
            .seat(
                self.seats[0].1,
                r::SeatRequest::Status { match_id: matched },
            )
            .await
            .unwrap()
        else {
            panic!("seat status")
        };
        status
    }

    async fn receipt_count(&self, matched: api::Id) -> i64 {
        self.db
            .query_one_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT count(*) AS n FROM central_identity.ranked_receipts WHERE match_id=$1",
                vec![Uuid::from_bytes(matched).into()],
            ))
            .await
            .unwrap()
            .unwrap()
            .try_get("", "n")
            .unwrap()
    }
}

async fn exercise(table: &Table<'_>) {
    table.first.ready(table.store, 2, BUILD).await;
    table.sibling.ready(table.store, 2, BUILD).await;
    table.second.ready(table.store, 2, [0xb2; 32]).await;
    let fallback = table.start_pair(100).await;
    assert_eq!(
        table.status(fallback.match_id).await.phase,
        r::Phase::Single
    );
    // Arrivals after allocation cannot upgrade the original trust contract.
    table.second.ready(table.store, 2, BUILD).await;
    assert_eq!(
        table.status(fallback.match_id).await.phase,
        r::Phase::Single
    );
    let primary = table.host(fallback.host_id);
    table
        .store
        .ranked_host(
            Registered(&primary.token),
            api::HostRequest::Void {
                match_id: fallback.match_id,
            },
        )
        .await
        .unwrap();
    participants::closed(table, fallback.match_id).await;
    table.sibling.ready(table.store, 0, BUILD).await;
    agreement::run(table).await;
    clocks::run(table).await;
    recovery::run(table).await;
}

pub(super) async fn run(
    store: &AccountStore,
    db: &DatabaseConnection,
    keys: &ServerKeys,
    seats: [(api::Id, &Token); 2],
) {
    let identity = enroll(store, keys, "ranked-replica-observer", "rankedobserver").await;
    let observer = login(store, keys, identity).await;
    store.confirm_restore(&observer, 1).await.unwrap();
    store
        .close_legacy("ranked-replica-observer", identity)
        .await
        .unwrap();
    let table = Table {
        store,
        db,
        seats,
        observer: &observer,
        first: Node::register(store, db, seats[0].1, "https://ranked-primary.example").await,
        sibling: Node::register(store, db, seats[0].1, "https://ranked-same-owner.example").await,
        second: Node::register(store, db, seats[1].1, "https://ranked-standby.example").await,
        outsider: Node::register(store, db, &observer, "https://ranked-observer.example").await,
    };
    exercise(&table).await;
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "DELETE FROM central_identity.accounts WHERE id=$1",
        vec![Uuid::from_bytes(identity.account).into()],
    ))
    .await
    .unwrap();
}
