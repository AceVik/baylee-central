//! Two real signed-in native SDK clients, two verified registered operators and a real issuer.
use super::super::{Http, password};
use central_api::{
    Error, Token,
    gateway::GatewayMetadata,
    ranked::{self as api, replica as r},
    wire::{self, GatewayPurpose},
};
use central_client::{GatewayRegistration, Session, Transport, ranked};
use central_crypto::jwt::{GatewayClaims, Purpose};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement, prelude::Uuid};
use tokio::runtime::Runtime;

struct Fixture<'a> {
    http: &'a Http,
    runtime: &'a Runtime,
    db: &'a DatabaseConnection,
    players: [&'a Session; 2],
    operators: [GatewayRegistration; 2],
}

/// Carry the same signed native session to the issuer that services the relay.
/// The public listener is exercised independently with this exact request.
struct Issuer<'a> {
    fixture: &'a Fixture<'a>,
    fence: r::Fence,
    public: bool,
}

impl Transport for Issuer<'_> {
    fn endpoint(&self) -> &central_api::Endpoint {
        self.fixture.http.endpoint()
    }
    fn exchange(
        &self,
        request: &wire::Request,
        bearer: Option<central_api::Bearer<'_>>,
    ) -> Result<wire::Response, Error> {
        let wire::Request::GatewayToken {
            gateway,
            purpose: GatewayPurpose::Ranked,
        } = request
        else {
            return Err(Error::Rejected);
        };
        let scoped = wire::Request::RankedToken {
            gateway: gateway.clone(),
            fence: self.fence,
        };
        if self.public {
            return self.fixture.http.exchange(&scoped, bearer);
        }
        let service = &self.fixture.http.identity;
        let header = bearer.ok_or(Error::Rejected)?.header();
        let native = service.present(&header, None)?;
        self.fixture
            .runtime
            .block_on(service.handle(scoped, Some(&native)))
    }
}

fn register(http: &Http, session: &Session, origin: &str) -> GatewayRegistration {
    session
        .register_gateway(
            http,
            origin.into(),
            GatewayMetadata {
                name: "Replicated native fixture".into(),
                version: "1".into(),
                website: format!("{origin}/status"),
            },
        )
        .unwrap()
}

impl Fixture<'_> {
    fn assign(&self) -> api::Assignment {
        for (i, owner) in self.players.iter().enumerate() {
            let request = r::SeatRequest::Join {
                key: self
                    .runtime
                    .block_on(self.db.query_one_raw(Statement::from_string(
                        DbBackend::Postgres,
                        "SELECT pg_catalog.uuidv7() AS id",
                    )))
                    .unwrap()
                    .unwrap()
                    .try_get::<Uuid>("", "id")
                    .unwrap()
                    .into_bytes(),
                format: api::Format::Commander,
                deck: [u8::try_from(i + 1).unwrap(); 32],
            };
            let r::SeatResponse::Queue { state } = owner.replica(self.http, request).unwrap()
            else {
                panic!("native queue response")
            };
            match state {
                api::QueueState::Assigned { assignment, .. } => {
                    assert_eq!(i, 1);
                    return assignment;
                }
                api::QueueState::Waiting { .. } => assert_eq!(i, 0),
                _ => panic!("unexpected queue state"),
            }
        }
        panic!("expected a replicated game")
    }

    fn approve(&self, owner: &Session, id: api::Id) {
        let r::SeatResponse::Hosts { fence, hosts } = owner
            .replica(self.http, r::SeatRequest::Hosts { match_id: id })
            .unwrap()
        else {
            panic!("native consent")
        };
        assert_eq!(hosts.len(), 2);
        owner
            .replica(
                self.http,
                r::SeatRequest::ApproveHosts {
                    fence,
                    hosts: hosts.into_iter().map(|offer| offer.host).collect(),
                },
            )
            .unwrap();
    }

    fn expire(&self, id: api::Id) {
        self.runtime.block_on(self.db.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
            "UPDATE central_identity.ranked_matches SET lease_until=now()-interval '1 second' WHERE id=$1",[Uuid::from_bytes(id).into()]))).unwrap();
        let request = r::SeatRequest::Status { match_id: id };
        let r::SeatResponse::Status { status } =
            self.players[1].replica(self.http, request).unwrap()
        else {
            panic!("durable terminal state")
        };
        assert_eq!(status.phase, r::Phase::Void);
    }

    fn signed(&self, assigned: &api::Assignment) -> GatewayClaims {
        let client = self.players[1];
        let r::SeatResponse::Status { status } = client
            .replica(
                self.http,
                r::SeatRequest::Status {
                    match_id: assigned.match_id,
                },
            )
            .unwrap()
        else {
            panic!("named current operator")
        };
        let issuer = Issuer {
            fixture: self,
            fence: status.fence,
            public: false,
        };
        let token = |purpose| client.gateway_token(&issuer, assigned.origin.clone(), purpose);
        assert!(token(GatewayPurpose::Ranked).is_err());
        self.approve(self.players[0], assigned.match_id);
        assert!(token(GatewayPurpose::Ranked).is_err());
        self.approve(client, assigned.match_id);
        assert!(
            client
                .replica(
                    self.http,
                    r::SeatRequest::RelayRoute {
                        fence: status.fence
                    }
                )
                .is_err()
        );
        let exported = Issuer {
            public: true,
            ..issuer
        };
        assert!(
            client
                .gateway_token(&exported, assigned.origin.clone(), GatewayPurpose::Ranked)
                .is_err()
        );
        assert!(
            client
                .gateway_token(self.http, assigned.origin.clone(), GatewayPurpose::Ranked)
                .is_err()
        );
        let (ranked, _) = token(GatewayPurpose::Ranked).unwrap();
        let claims = self.http.gateway_claims(&ranked, &assigned.origin);
        assert!(claims.purpose == Purpose::Ranked);
        assert_eq!(claims.ranked_match, Some(assigned.match_id));
        assert!(claims.ranked_generation.is_some());
        assert!(claims.name.is_none() && claims.tag.is_none() && claims.terms.is_none());
        assert!(claims.legacy_id.is_none() && claims.legacy_deck_source.is_none());
        assert!(
            !self
                .players
                .iter()
                .any(|p| Some(p.identity().account) == claims.account())
        );
        let (refresh, _) = token(GatewayPurpose::Ranked).unwrap();
        let renewed = self.http.gateway_claims(&refresh, &assigned.origin);
        assert_eq!((&renewed.sub, &renewed.sid), (&claims.sub, &claims.sid));
        let (lobby, _) = client
            .gateway_token(self.http, assigned.origin.clone(), GatewayPurpose::Admit)
            .unwrap();
        let lobby = self.http.gateway_claims(&lobby, &assigned.origin);
        assert_eq!(lobby.account(), Some(client.identity().account));
        assert_ne!(lobby.sid, claims.sid);
        claims
    }

    fn round_trip(&self) -> api::Id {
        let a = self.assign();
        let credential = &self
            .operators
            .iter()
            .find(|op| op.gateway == a.host_id)
            .unwrap()
            .credential;
        let request = r::SeatRequest::Hosts {
            match_id: a.match_id,
        };
        let wire = wire::Request::Ranked {
            request: api::Request::Replicate { request },
        };
        assert!(self.http.exchange(&wire, None).is_err());
        assert!(self.http.exchange(&wire, Some(credential.into())).is_err());
        let claims = self.signed(&a);
        let alias = claims.account().unwrap();
        let claim = |account| api::HostRequest::Claim {
            match_id: a.match_id,
            account,
            deck: a.deck,
            proof: api::SeatProof::from_sid(&claims.sid).unwrap(),
        };
        assert!(
            ranked::host(
                self.http,
                credential,
                claim(self.players[1].identity().account)
            )
            .is_err()
        );
        for _ in 0..2 {
            let api::HostResponse::Assignment { assignment } =
                ranked::host(self.http, credential, claim(alias)).unwrap()
            else {
                panic!("revalidated host assignment")
            };
            assert_eq!(assignment.accounts[usize::from(a.seat)], alias);
            assert!(
                assignment
                    .accounts
                    .iter()
                    .all(|id| self.players.iter().all(|p| p.identity().account != *id))
            );
        }
        let other = self
            .operators
            .iter()
            .find(|op| op.gateway != a.host_id)
            .unwrap();
        let r::HostResponse::Assignment { work } = ranked::replica_host(
            self.http,
            &other.credential,
            r::HostRequest::Inspect {
                match_id: a.match_id,
            },
        )
        .unwrap() else {
            panic!("standby assignment")
        };
        assert!(!work.accounts.contains(&alias));
        self.expire(a.match_id);
        assert!(
            self.players[1]
                .gateway_token(self.http, a.origin, GatewayPurpose::Ranked)
                .is_err()
        );
        alias
    }
}

pub(crate) fn run(
    http: &Http,
    first: &Session,
    bridge: &Token,
    runtime: &Runtime,
    db: &DatabaseConnection,
) {
    let (identity, second) = password::moved(http, bridge, "replica-seat");
    password::close(http, bridge, "replica-seat", identity);
    let operators = [
        register(http, first, "http://127.0.0.1:28792"),
        register(http, &second, "http://127.0.0.1:28793"),
    ];
    for node in &operators {
        let ready = r::HostRequest::Ready {
            capacity: 2,
            compatibility: [91; 32],
        };
        assert_eq!(
            ranked::replica_host(http, &node.credential, ready.clone()).unwrap_err(),
            Error::Rejected
        );
        runtime.block_on(db.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,
          "INSERT INTO central_identity.gateway_approvals(gateway_id,enabled,ranked_verified) VALUES($1,true,true)",[Uuid::from_bytes(node.gateway).into()]))).unwrap();
        ranked::replica_host(http, &node.credential, ready).unwrap();
    }
    let fixture = Fixture {
        http,
        runtime,
        db,
        players: [first, &second],
        operators,
    };
    let a = fixture.round_trip();
    let b = fixture.round_trip();
    assert_ne!(a, b);
    for node in &fixture.operators {
        runtime
            .block_on(db.execute_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "DELETE FROM central_identity.gateways WHERE id=$1",
                [Uuid::from_bytes(node.gateway).into()],
            )))
            .unwrap();
    }
    second.logout(http).unwrap();
}
