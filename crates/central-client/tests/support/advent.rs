//! Synthetic authenticated native HTTP/SDK campaign and cache retrieval.
use super::{Http, password};
use central_api::{
    Token,
    advent::{self as api, Request as Q, Response as A},
    wire,
};
use central_client::{Session, Transport};
use sea_orm::DatabaseConnection;
use tokio::runtime::Runtime;
mod framing;
#[path = "../../../central-postgres/tests/support/advent_material.rs"]
mod material;
mod staging;

const YEAR: u16 = 2106;
fn closed(http: &Http, owner: &Session) {
    let q = Q::Calendar { year: YEAR };
    assert!(
        http.exchange(&wire::Request::Advent { request: q.clone() }, None)
            .is_err()
    );
    let page = owner.advent(http, q).unwrap();
    let encoded = String::from_utf8(central_api::encode(&page).unwrap()).unwrap();
    for internal in [
        "Synthetic reward",
        "Fixture hint",
        "digest",
        "art_id",
        "puzzle",
        "sleeve",
        "playmat",
        "tier",
        "mat_framing",
        "focus",
        "variants",
    ] {
        assert!(!encoded.contains(internal));
    }
    let A::Calendar { hero, doors, .. } = page else {
        panic!("private calendar")
    };
    assert!(hero.is_none() && doors.iter().all(|d| d.state == api::DoorState::Locked));
    assert!(matches!(
        owner.advent(http, Q::CurrentCalendar {}).unwrap(),
        A::Calendar {
            year: YEAR,
            hero: None,
            ..
        }
    ));
    assert!(
        owner
            .advent(
                http,
                Q::Puzzle {
                    year: YEAR,
                    door: 1
                }
            )
            .is_err()
    );
    assert!(
        owner
            .advent_asset(http, api::AssetScope::Calendar { year: YEAR }, None)
            .is_err()
    );
    assert!(central_api::decode::<Q>(br#"{"calendar":{"year":2106,"now":9999999999}}"#).is_err());
}

fn no_static_image(http: &Http, reward: &api::Reward) {
    use std::fmt::Write as _;
    let mut hash = String::new();
    for byte in reward.asset.digest {
        write!(hash, "{byte:02x}").unwrap();
    }
    let (status, _, body) = http.raw(&format!("GET /advent/assets/{hash}.webp"), &[], b"");
    assert_eq!(status, 404);
    assert_ne!(body, material::bytes(1));
}

fn won(http: &Http, owner: &Session) -> api::Reward {
    assert!(matches!(
        owner
            .advent(
                http,
                Q::Submit {
                    year: YEAR,
                    door: 1,
                    line: material::line(0)
                }
            )
            .unwrap(),
        A::Submission { won: false }
    ));
    assert!(
        owner
            .advent(
                http,
                Q::Claim {
                    year: YEAR,
                    door: 1
                }
            )
            .is_err()
    );
    assert!(matches!(
        owner
            .advent(
                http,
                Q::Submit {
                    year: YEAR,
                    door: 1,
                    line: material::line(2)
                }
            )
            .unwrap(),
        A::Submission { won: true }
    ));
    let A::Reward { reward } = owner
        .advent(
            http,
            Q::Claim {
                year: YEAR,
                door: 1,
            },
        )
        .unwrap()
    else {
        panic!("personal reward")
    };
    reward
}

fn range_reads(http: &Http, owner: &Session, outsider: &Session, reward: &api::Reward) {
    let scope = api::AssetScope::Owned { item: reward.id };
    let digest = reward.asset.digest;
    let (meta, bytes) = owner.advent_asset(http, scope, None).unwrap();
    assert_eq!(meta, reward.asset);
    assert_eq!(bytes.as_deref(), Some(material::bytes(1).as_slice()));
    assert_eq!(
        owner.advent_asset(http, scope, Some(digest)).unwrap(),
        (meta, None)
    );
    assert!(outsider.advent_asset(http, scope, Some(digest)).is_err());
    for (offset, cached) in [
        (1, None),
        (api::PAGE_BYTES, Some(digest)),
        (api::PAGE_BYTES * 3, None),
    ] {
        assert!(
            owner
                .advent(
                    http,
                    Q::Asset {
                        scope,
                        offset,
                        cached
                    }
                )
                .is_err()
        );
    }
    let answer = owner
        .advent(
            http,
            Q::Asset {
                scope,
                offset: api::PAGE_BYTES * 2,
                cached: None,
            },
        )
        .unwrap();
    assert!(
        matches!(answer,A::Asset{page} if page.offset==api::PAGE_BYTES*2 && page.bytes==[0xe7;7])
    );
}

fn current(http: &Http, owner: &Session) -> Option<u16> {
    match owner.advent(http, Q::CurrentCalendar {}).unwrap() {
        A::NoCalendar {} => None,
        A::Calendar { year, .. } => Some(year),
        _ => panic!("default calendar response"),
    }
}

pub(crate) fn run(
    http: &Http,
    owner: &Session,
    bridge: &Token,
    runtime: &Runtime,
    db: &DatabaseConnection,
) {
    let preceding = current(http, owner);
    runtime.block_on(material::provision(db, YEAR));
    runtime.block_on(material::elapsed(db, YEAR, -1800));
    closed(http, owner);
    let (id, outsider) = password::moved(http, bridge, "advent-native-peer");
    password::close(http, bridge, "advent-native-peer", id);
    runtime.block_on(material::elapsed(db, YEAR, 60));
    assert!(
        matches!(owner.advent(http,Q::Puzzle{year:YEAR,door:1}).unwrap(),A::Puzzle{puzzle,..}if puzzle==material::board(1))
    );
    let reward = won(http, owner);
    assert!(reward.asset.mat_framing.is_none());
    no_static_image(http, &reward);
    range_reads(http, owner, &outsider, &reward);
    runtime.block_on(material::elapsed(db, YEAR, 86_460));
    framing::run(http, &outsider, owner);
    runtime.block_on(material::elapsed(db, YEAR, 37 * 86_400));
    assert_eq!(current(http, owner), preceding);
    assert!(
        matches!(owner.advent(http,Q::Inventory{after:None}).unwrap(),A::Inventory{items,..}if items==[reward.clone()])
    );
    assert!(
        owner
            .advent(
                http,
                Q::Puzzle {
                    year: YEAR,
                    door: 24
                }
            )
            .is_err()
    );
    assert!(
        owner
            .advent_asset(http, api::AssetScope::Calendar { year: YEAR }, None)
            .is_err()
    );
    range_reads(http, owner, &outsider, &reward);
    if let Some(root) = std::env::var_os("CENTRAL_TEST_PROVISIONED_ADVENT_PACK") {
        staging::verify(
            http,
            owner,
            &outsider,
            runtime,
            db,
            std::path::Path::new(&root),
        );
    }
    outsider.logout(http).unwrap();
}
