//! Optional operator release check. Only a specially marked disposable database clone can move its clock.
use super::{A, Http, Q, Session, api, material};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use tokio::runtime::Runtime;

#[derive(Deserialize)]
struct Pack {
    year: u16,
    hero: Image,
    doors: Vec<Entry>,
}
#[derive(Deserialize)]
struct Entry {
    door: u8,
    art: Image,
}
#[derive(Deserialize)]
struct Image {
    path: PathBuf,
}

fn compare(http: &Http, owner: &Session, scope: api::AssetScope, path: &Path, root: &Path) {
    let path = root.join(path).canonicalize().unwrap();
    assert!(path.starts_with(root));
    let expected = std::fs::read(path).unwrap();
    let (metadata, download) = owner.advent_asset(http, scope, None).unwrap();
    metadata.verify(&expected).unwrap();
    assert!(matches!(download.as_deref(),Some(bytes) if bytes==expected));
}

pub(super) fn verify(
    http: &Http,
    owner: &Session,
    peer: &Session,
    runtime: &Runtime,
    db: &DatabaseConnection,
    root: &Path,
) {
    let gate = runtime
        .block_on(db.query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT 1 WHERE current_database()='central_native_test' AND EXISTS(
         SELECT 1 FROM pg_database WHERE datname=current_database()
         AND shobj_description(oid,'pg_database')='advent-disposable-clone-v1')",
        )))
        .unwrap();
    assert!(
        gate.is_some(),
        "requires an isolated marked clone of the staging database"
    );
    let root = root.canonicalize().unwrap();
    let pack: Pack =
        central_api::decode(&std::fs::read(root.join("campaign.json")).unwrap()).unwrap();
    let year = pack.year;
    assert_eq!(pack.doors.len(), usize::from(api::DOORS));
    let hero = api::AssetScope::Calendar { year };
    runtime.block_on(material::elapsed(db, year, -1800));
    assert!(owner.advent_asset(http, hero, None).is_err());
    assert!(matches!(
        owner.advent(http, Q::CurrentCalendar {}).unwrap(),
        A::Calendar { year:current, hero:None, doors, .. }
            if current==year && doors.iter().all(|entry|entry.state==api::DoorState::Locked)
    ));
    runtime.block_on(material::elapsed(db, year, 24 * 86_400));
    assert!(
        matches!(owner.advent(http,Q::CurrentCalendar{}).unwrap(),A::Calendar{year:current,hero:Some(_),..} if current==year)
    );
    compare(http, owner, hero, &pack.hero.path, &root);
    for entry in &pack.doors {
        let door = entry.door;
        let A::Puzzle { puzzle, .. } = owner.advent(http, Q::Puzzle { year, door }).unwrap() else {
            panic!("missing puzzle");
        };
        let line = puzzle.study(100_000).unwrap().witness.unwrap();
        assert!(matches!(
            owner.advent(http, Q::Submit { year, door, line }),
            Ok(A::Submission { won: true })
        ));
        let A::Inventory { items, .. } = owner.advent(http, Q::Inventory { after: None }).unwrap()
        else {
            panic!("missing inventory");
        };
        let reward = items
            .iter()
            .find(|item| item.year == year && item.door == door)
            .expect("atomic reward delivery");
        let scope = api::AssetScope::Owned { item: reward.id };
        assert!(
            peer.advent_asset(http, scope, Some(reward.asset.digest))
                .is_err()
        );
        compare(http, owner, scope, &entry.art.path, &root);
    }
    let A::Inventory { items, .. } = owner.advent(http, Q::Inventory { after: None }).unwrap()
    else {
        panic!("missing inventory");
    };
    assert_eq!(
        items.iter().filter(|item| item.year == year).count(),
        usize::from(api::DOORS)
    );
    runtime.block_on(material::elapsed(db, year, 37 * 86_400));
    assert!(matches!(
        owner.advent(http, Q::CurrentCalendar {}).unwrap(),
        A::NoCalendar {}
    ));
    assert!(
        matches!(owner.advent(http,Q::Claim{year,door:24}).unwrap(),A::Reward{reward} if items.contains(&reward))
    );
    println!(
        "Staged native HTTP delivery verified: calendar and 24 owned images equal their private source, other session denied."
    );
}
