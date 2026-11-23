//! Whole-catalog private provisioning through the same isolated static WebP pipeline as Advent.
use super::{Image, Kind, artwork, connect, read, settings};
use central_postgres::advent_import::RankedCatalog;
use serde::Deserialize;
use std::{path::Path, process::ExitCode};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    sets: Vec<Pair>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Pair {
    league: u8,
    division: u8,
    sleeve: Image,
    playmat: Image,
}

async fn import() -> Result<(), ()> {
    let (database, root, worker) = settings()?;
    let bytes = read(&root, Path::new("catalog.json"), 128 * 1024)?;
    let pack: Manifest = serde_json::from_slice(&bytes).map_err(|_| ())?;
    if pack.sets.len() != 28 {
        return Err(());
    }
    let db = connect(database).await?;
    let mut publication = RankedCatalog::begin(&db).await.map_err(|_| ())?;
    for pair in pack.sets {
        let tier = match pair.league {
            0..=7 => "standard",
            8 | 9 => "special",
            10 => "finale",
            _ => return Err(()),
        };
        let sleeve = artwork(&root, &worker, pair.sleeve, Some(Kind::Sleeve), tier.into())?;
        let mat = artwork(
            &root,
            &worker,
            pair.playmat,
            Some(Kind::Playmat),
            tier.into(),
        )?;
        publication
            .pair(pair.league, pair.division, sleeve, mat)
            .await
            .map_err(|_| ())?;
    }
    publication.finish().await.map_err(|_| ())
}

pub(crate) async fn run() -> ExitCode {
    if import().await.is_ok() {
        println!("Ranked cosmetic catalog verified and stored (28 pairs).");
        ExitCode::SUCCESS
    } else {
        eprintln!(
            "Ranked cosmetic import refused. Verify the private package, worker and staging role."
        );
        ExitCode::FAILURE
    }
}
