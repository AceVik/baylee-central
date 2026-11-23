//! Offline import of a private, operator-owned campaign. Diagnostics contain no private material.
use central_api::advent::{DOORS, Kind, MAX_ART_BYTES, Text, framing::MatFraming, puzzle::Puzzle};
use central_postgres::{
    DevConfig,
    advent_import::{Art, Import},
    connect_dev, connect_server,
};
use serde::Deserialize;
use std::{
    fs::File,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    process::{Command, ExitCode, Stdio},
};
use zeroize::{Zeroize, ZeroizeOnDrop};

pub(crate) mod ranked;
#[cfg(test)]
mod tests;

#[derive(Deserialize, Zeroize, ZeroizeOnDrop)]
#[serde(deny_unknown_fields)]
struct Local {
    port: u16,
    user: String,
    password: String,
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Connection {
    Tls(super::production::Database),
    LoopbackFixture(Local),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Settings {
    database: Connection,
    pack_root: PathBuf,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Pack {
    year: u16,
    hero: Image,
    doors: Vec<Entry>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Image {
    #[serde(default)]
    mat_framing: Option<MatFraming>,
    path: PathBuf,
    sha256: String,
    title: Text,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    door: u8,
    kind: Kind,
    tier: String,
    art: Image,
    challenge: PathBuf,
    challenge_sha256: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Challenge {
    puzzle: Puzzle,
    hints: [Text; 2],
}

/// Files come from an operator-owned, private directory. Check each component before opening;
/// reject links, pipes, special files, oversized input and absolute paths.
fn read(root: &Path, relative: &Path, cap: usize) -> Result<Vec<u8>, ()> {
    if relative.as_os_str().is_empty() || relative.as_os_str().len() > 1024 {
        return Err(());
    }
    let mut path = root.to_owned();
    for part in relative.components() {
        let Component::Normal(name) = part else {
            return Err(());
        };
        path.push(name);
        if path.symlink_metadata().map_err(|_| ())?.is_symlink() {
            return Err(());
        }
    }
    let meta = path.symlink_metadata().map_err(|_| ())?;
    if !meta.is_file()
        || meta.len() > cap as u64
        || !path.canonicalize().map_err(|_| ())?.starts_with(root)
    {
        return Err(());
    }
    let mut bytes = Vec::with_capacity(usize::try_from(meta.len()).map_err(|_| ())?);
    File::open(path)
        .map_err(|_| ())?
        .take(cap as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| ())?;
    if bytes.len() > cap {
        return Err(());
    }
    Ok(bytes)
}

fn inspect(worker: &Path, bytes: &[u8]) -> Result<[u32; 2], ()> {
    let mut child = Command::new(worker)
        .arg("--inspect-advent-webp")
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| ())?;
    let sent = child
        .stdin
        .take()
        .ok_or(())
        .and_then(|mut stdin| stdin.write_all(bytes).map_err(|_| ()));
    let output = child.wait_with_output().map_err(|_| ())?;
    sent?;
    if !output.status.success() {
        return Err(());
    }
    let raw: [u8; 8] = output.stdout.try_into().map_err(|_| ())?;
    Ok([
        u32::from_be_bytes(raw[..4].try_into().map_err(|_| ())?),
        u32::from_be_bytes(raw[4..].try_into().map_err(|_| ())?),
    ])
}

fn verified(root: &Path, relative: &Path, expected: &str, limit: usize) -> Result<Vec<u8>, ()> {
    let bytes = read(root, relative, limit)?;
    let digest = central_picture::digest(&bytes, None);
    let hex = b"0123456789abcdef";
    if expected.len() != 64
        || !expected
            .as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .zip(digest)
            .all(|(pair, byte)| *pair == [hex[usize::from(byte >> 4)], hex[usize::from(byte & 15)]])
    {
        return Err(());
    }
    Ok(bytes)
}

fn artwork(
    root: &Path,
    worker: &Path,
    image: Image,
    kind: Option<Kind>,
    tier: String,
) -> Result<Art, ()> {
    let bytes = verified(root, &image.path, &image.sha256, MAX_ART_BYTES as usize)?;
    let dimensions = inspect(worker, &bytes)?;
    Ok(Art {
        mat_framing: image.mat_framing,
        kind,
        tier,
        title: image.title,
        dimensions,
        bytes,
    })
}

/// Both offline imports use the same confined file reader, decoder and restricted connection.
fn settings() -> Result<(Connection, PathBuf, PathBuf), ()> {
    let input = super::production::read_settings()?;
    let Settings {
        database,
        pack_root,
    } = serde_json::from_slice(&input).map_err(|_| ())?;
    if !pack_root.is_absolute() || !pack_root.symlink_metadata().map_err(|_| ())?.is_dir() {
        return Err(());
    }
    let root = pack_root.canonicalize().map_err(|_| ())?;
    let worker = std::env::current_exe()
        .map_err(|_| ())?
        .with_file_name(format!(
            "central-picture-worker{}",
            std::env::consts::EXE_SUFFIX
        ));
    Ok((database, root, worker))
}

async fn connect(mut database: Connection) -> Result<central_postgres::DatabaseConnection, ()> {
    match &mut database {
        Connection::Tls(config) => connect_server(config.server()?).await,
        Connection::LoopbackFixture(config) => {
            connect_dev(
                DevConfig::new(
                    config.port,
                    &config.user,
                    std::mem::take(&mut config.password),
                    &config.name,
                )
                .map_err(|_| ())?,
            )
            .await
        }
    }
    .map_err(|_| ())
}

async fn import() -> Result<(), ()> {
    let (database, root, worker) = settings()?;
    let mut pack: Pack =
        serde_json::from_slice(&read(&root, Path::new("campaign.json"), 64 * 1024)?)
            .map_err(|_| ())?;
    pack.doors.sort_by_key(|door| door.door);
    if !pack.doors.iter().map(|d| d.door).eq(1..=DOORS) {
        return Err(());
    }
    let hero = artwork(&root, &worker, pack.hero, None, "standard".into())?;
    let db = connect(database).await?;
    let mut publication = Import::begin(&db, pack.year, hero).await.map_err(|_| ())?;
    for door in pack.doors {
        let Challenge { puzzle, hints } = serde_json::from_slice(&verified(
            &root,
            &door.challenge,
            &door.challenge_sha256,
            40 * 1024,
        )?)
        .map_err(|_| ())?;
        let art = artwork(&root, &worker, door.art, Some(door.kind), door.tier)?;
        publication
            .door(door.door, &puzzle, hints, art)
            .await
            .map_err(|_| ())?;
    }
    publication.finish().await.map_err(|_| ())
}

pub(super) async fn run() -> ExitCode {
    if import().await.is_ok() {
        println!("Advent campaign verified and stored (24 doors, 1 calendar image).");
        ExitCode::SUCCESS
    } else {
        eprintln!("Advent import refused. Verify the private package, worker and staging role.");
        ExitCode::FAILURE
    }
}
