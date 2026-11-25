//! Local development runner for Baylee Central and the website.
//!
//! Builds the native services and website assets, stages runtime copies under
//! `target/dev-site/` (so subsequent Cargo runs never collide with Windows file
//! locks), initializes and migrates a dedicated `PostgreSQL` cluster on
//! `127.0.0.1:55433`, and supervises `central-service`, `central-web`, and the
//! loopback reverse proxy on `http://127.0.0.1:28794`.

use std::collections::BTreeSet;
use std::env;
use std::fmt::Write as _;
use std::fs::{self, OpenOptions};
use std::io::{Read as _, Write as _};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use crate::fail::{Fail, OrFail, Result};
use crate::{process, random, workspace};

const PG_PORT: u16 = 55433;
const ENDPOINT_PORT: u16 = 28790;
const BROWSER_PORT: u16 = 28791;
const SITE_PORT: u16 = 28792;
const FRONT_PORT: u16 = 28794;
const MAIL_PORT: u16 = 28795;
const MAX_MAIL_HEADER_BYTES: usize = 16 * 1024;
const MAX_MAIL_BODY_BYTES: usize = 2 * 1024 * 1024;
const GATEWAY_URL: &str = "http://127.0.0.1:28766";
const INITIAL_VIEWS_JSON: &[u8] = b"{\"views\":1248,\"since\":\"2026-11-01\"}\n";
const DEV_OWNER_ID: &str = "019b0000-0000-7000-8000-000000000001";
const DEV_GATEWAY_ONLINE_IDS: [&str; 3] = [
    "019b0000-0000-7000-8000-000000000101",
    "019b0000-0000-7000-8000-000000000102",
    "019b0000-0000-7000-8000-000000000103",
];
const DEV_GATEWAY_OFFLINE_ID: &str = "019b0000-0000-7000-8000-000000000104";
const DEV_GATEWAY_PULSE_INTERVAL: Duration = Duration::from_secs(40);

struct DevGateway {
    id: &'static str,
    origin: &'static str,
    name: &'static str,
    version: &'static str,
    website: &'static str,
    activity: Option<(u32, u32, u32, u32)>,
}

const DEV_GATEWAYS: [DevGateway; 4] = [
    DevGateway {
        id: DEV_GATEWAY_ONLINE_IDS[0],
        origin: "http://127.0.0.1:29761",
        name: "Sanctuary Dev",
        version: "0.2.0-beta.o1",
        website: "http://127.0.0.1:29761/",
        activity: Some((24, 14, 3, 7)),
    },
    DevGateway {
        id: DEV_GATEWAY_ONLINE_IDS[1],
        origin: "http://127.0.0.1:29762",
        name: "Tavern of the Seven Lands",
        version: "0.2.0-beta.o1",
        website: "http://127.0.0.1:29762/",
        activity: Some((11, 6, 2, 3)),
    },
    DevGateway {
        id: DEV_GATEWAY_ONLINE_IDS[2],
        origin: "http://127.0.0.1:29763",
        name: "Mondschein-Runde",
        version: "0.2.0-beta.o1",
        website: "http://127.0.0.1:29763/",
        activity: Some((5, 2, 1, 1)),
    },
    DevGateway {
        id: DEV_GATEWAY_OFFLINE_ID,
        origin: "http://127.0.0.1:29764",
        name: "Atelier Archive",
        version: "0.1.1-beta.9b",
        website: "http://127.0.0.1:29764/",
        activity: None,
    },
];

struct StackGuard {
    children: Vec<Child>,
    pgdata: PathBuf,
    stop_pg_on_drop: bool,
}

impl StackGuard {
    fn stop_all(&mut self) {
        for child in &mut self.children {
            let _ = process::stop_tree(child);
        }
        self.children.clear();
        if self.stop_pg_on_drop {
            let _ = pg_ctl(&self.pgdata, "stop", &["-m", "fast"]);
        }
    }
}

impl Drop for StackGuard {
    fn drop(&mut self) {
        self.stop_all();
    }
}

pub(crate) fn run(root: &Path) -> Result<()> {
    let clean_root = clean_path(root);
    let dev_dir = clean_root.join("target/dev-site");
    let bin_dir = dev_dir.join("bin");
    let site_dir = dev_dir.join("site");
    let pgdata = dev_dir.join("pgdata");
    let pglog = dev_dir.join("pg.log");
    let state_path = dev_dir.join("state.json");
    let views_path = dev_dir.join("views.json");

    fs::create_dir_all(&bin_dir).or_fail("Could not create dev-site runtime folder")?;
    ensure_views_file(&views_path)?;
    build_and_stage(&clean_root, &bin_dir, &site_dir)?;
    let started_pg = ensure_postgres(&pgdata, &pglog)?;
    let service_bin = bin_dir
        .join("central-service")
        .with_extension(env::consts::EXE_EXTENSION);
    let state = load_or_create_state(&state_path, &service_bin)?;
    ensure_roles_and_db(&state)?;
    let migrator_pw = state["db_passwords"]["migrator"]
        .as_str()
        .or_fail("Dev-site state missing migrator credential")?;
    apply_migrations_and_grants(&clean_root, migrator_pw)?;
    seed_dev_gateways(migrator_pw)?;

    let mail_token = random::hex(32)?;
    let mail_log = dev_dir.join("dev-mail.jsonl");
    let _mail_guard = start_dev_mailer(&mail_token, &mail_log)?;

    let children = spawn_stack(
        &clean_root,
        &bin_dir,
        &site_dir,
        &views_path,
        &state,
        &mail_token,
    )?;
    let mut guard = StackGuard {
        children,
        pgdata,
        stop_pg_on_drop: started_pg,
    };
    wait_for_ports(&mut guard.children)?;
    println!("dev-site ready: http://127.0.0.1:{FRONT_PORT}/de/");
    println!(
        "dev-mailer ready: http://127.0.0.1:{MAIL_PORT}/v1/send (spool: {})",
        mail_log.display()
    );
    supervise(&mut guard.children, migrator_pw)
}

fn ensure_views_file(views_path: &Path) -> Result<()> {
    if views_path.is_file() {
        return Ok(());
    }
    fs::write(views_path, INITIAL_VIEWS_JSON).or_fail("Writing dev-site views file failed")
}

fn clean_path(path: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        let text = path.to_string_lossy();
        if let Some(stripped) = text.strip_prefix(r"\\?\")
            && stripped.as_bytes().get(1) == Some(&b':')
        {
            return PathBuf::from(stripped);
        }
    }
    path.to_path_buf()
}

fn find_ffmpeg() -> Option<PathBuf> {
    if let Some(path) = env::var_os("PICTURE_FFMPEG").map(PathBuf::from)
        && path.is_file()
    {
        return Some(path);
    }
    if let Some(paths) = env::var_os("PATH") {
        for dir in env::split_paths(&paths) {
            let candidate = dir
                .join("ffmpeg")
                .with_extension(env::consts::EXE_EXTENSION);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    for candidate in [
        "/usr/bin/ffmpeg",
        "/opt/homebrew/bin/ffmpeg",
        "/usr/local/bin/ffmpeg",
        r"C:\ProgramData\chocolatey\bin\ffmpeg.exe",
    ] {
        let path = PathBuf::from(candidate);
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

fn pg_tool(name: &str) -> PathBuf {
    if let Some(dir) = env::var_os("BAYLEE_PG_BIN") {
        let candidate = PathBuf::from(dir)
            .join(name)
            .with_extension(env::consts::EXE_EXTENSION);
        if candidate.is_file() {
            return candidate;
        }
    }
    PathBuf::from(name)
}

fn build_and_stage(root: &Path, bin_dir: &Path, site_dir: &Path) -> Result<()> {
    process::succeed(
        Command::new(workspace::cargo())
            .args([
                "build",
                "--locked",
                "--offline",
                "-p",
                "central-service",
                "-p",
                "central-picture-codec",
                "-p",
                "central-web-server",
            ])
            .current_dir(root),
        "Building central services for dev-site failed",
    )?;
    process::succeed(
        Command::new(workspace::cargo())
            .args(["xtask", "site"])
            .env(
                "BAYLEE_CENTRAL_DEV_ENDPOINT",
                format!("http://127.0.0.1:{ENDPOINT_PORT}"),
            )
            .env("BAYLEE_CENTRAL_DEV_GATEWAY", GATEWAY_URL)
            .current_dir(root),
        "Building website assets for dev-site failed",
    )?;
    let debug_dir = root.join("target/debug");
    for binary in ["central-service", "central-picture-worker", "central-web"] {
        let src = debug_dir
            .join(binary)
            .with_extension(env::consts::EXE_EXTENSION);
        let dst = bin_dir
            .join(binary)
            .with_extension(env::consts::EXE_EXTENSION);
        fs::copy(&src, &dst).or_fail("Staging dev-site service binary failed")?;
    }
    if site_dir.exists() {
        fs::remove_dir_all(site_dir).or_fail("Clearing staged dev-site folder failed")?;
    }
    copy_dir_all(&root.join("target/site"), site_dir)
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst).or_fail("Creating staged asset directory failed")?;
    let entries = fs::read_dir(src).or_fail("Reading built site directory failed")?;
    for entry in entries {
        let entry = entry.or_fail("Reading site asset entry failed")?;
        let target = dst.join(entry.file_name());
        if entry
            .file_type()
            .or_fail("Inspecting site asset entry failed")?
            .is_dir()
        {
            copy_dir_all(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target).or_fail("Copying site asset file failed")?;
        }
    }
    Ok(())
}

fn pg_ctl(pgdata: &Path, subcmd: &str, extra: &[&str]) -> Result<bool> {
    let status = Command::new(pg_tool("pg_ctl"))
        .arg("-D")
        .arg(pgdata)
        .arg(subcmd)
        .args(extra)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .or_fail("Running pg_ctl failed")?;
    Ok(status.success())
}

fn ensure_postgres(pgdata: &Path, pglog: &Path) -> Result<bool> {
    if !pgdata.join("PG_VERSION").is_file() {
        process::succeed(
            Command::new(pg_tool("initdb")).arg("-D").arg(pgdata).args([
                "-U",
                "postgres",
                "--auth=trust",
                "--encoding=UTF8",
                "--no-locale",
            ]),
            "Initializing local PostgreSQL cluster failed",
        )?;
    }
    if pg_ctl(pgdata, "status", &[])? && tcp_open(PG_PORT) {
        return Ok(false);
    }
    let opts = format!("-p {PG_PORT} -h 127.0.0.1");
    let log_str = pglog
        .to_str()
        .or_fail("PostgreSQL log path is not valid UTF-8")?;
    let started = pg_ctl(pgdata, "start", &["-l", log_str, "-o", &opts])?;
    if !started {
        return Err(Fail::new("Starting local PostgreSQL cluster failed"));
    }
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        if tcp_open(PG_PORT) {
            return Ok(true);
        }
        thread::sleep(Duration::from_millis(100));
    }
    Err(Fail::new(
        "Local PostgreSQL cluster did not accept connections",
    ))
}

fn load_or_create_state(state_path: &Path, service_bin: &Path) -> Result<Value> {
    if state_path.is_file()
        && let Ok(text) = fs::read_to_string(state_path)
        && let Ok(parsed) = serde_json::from_str::<Value>(&text)
        && parsed["db_passwords"]["migrator"].is_string()
        && parsed["db_passwords"]["runtime"].is_string()
        && parsed["secrets"]["opaque_setup"].is_array()
        && parsed["secrets"]["bridge"].is_array()
        && parsed["secrets"]["token_keys"]["active"].is_array()
    {
        return Ok(parsed);
    }
    let output = process::capture(
        Command::new(service_bin).arg("--generate-secrets"),
        "Generating local Central secrets failed",
    )?;
    if !output.status.success() {
        return Err(Fail::new("Generating local Central secrets failed"));
    }
    let secrets: Value =
        serde_json::from_slice(&output.stdout).or_fail("Parsing generated secrets failed")?;
    let state = json!({
        "db_passwords": {
            "migrator": random::hex(32)?,
            "runtime": random::hex(32)?,
            "legacy": random::hex(32)?,
            "advent_importer": random::hex(32)?,
        },
        "secrets": {
            "opaque_setup": secrets["opaque_setup"],
            "bridge": secrets["bridge"],
            "token_keys": secrets["token_keys"],
        }
    });
    let encoded = serde_json::to_vec_pretty(&state).or_fail("Serializing dev-site state failed")?;
    fs::write(state_path, encoded).or_fail("Writing dev-site state failed")?;
    Ok(state)
}

fn ensure_roles_and_db(state: &Value) -> Result<()> {
    let pw = &state["db_passwords"];
    let mut sql = String::new();
    for (role, key) in [
        ("central_migrator", "migrator"),
        ("central_runtime", "runtime"),
        ("central_legacy", "legacy"),
        ("central_advent_importer", "advent_importer"),
    ] {
        let pass = pw[key]
            .as_str()
            .or_fail("Dev-site database credential missing")?;
        let _ = writeln!(
            sql,
            "DO $$ BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_catalog.pg_roles WHERE rolname = '{role}') THEN
    CREATE ROLE {role} LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS PASSWORD '{pass}';
  ELSE
    ALTER ROLE {role} WITH PASSWORD '{pass}';
  END IF;
END $$;"
        );
    }
    let failure = "Bootstrapping dev-site database roles failed";
    let role_out = process::feed(
        &mut psql_base("postgres", "postgres", None),
        sql.as_bytes(),
        failure,
    )?;
    if !role_out.status.success() {
        return Err(Fail::new(failure));
    }
    let check_db = process::capture(
        psql_base("postgres", "postgres", None).args([
            "-At",
            "-c",
            "SELECT 1 FROM pg_database WHERE datname = 'central_dev'",
        ]),
        "Checking dev-site database existence failed",
    )?;
    if String::from_utf8_lossy(&check_db.stdout).trim() != "1" {
        let create_out = process::feed(
            &mut psql_base("postgres", "postgres", None),
            b"CREATE DATABASE central_dev OWNER central_migrator;\n",
            "Creating central_dev database failed",
        )?;
        if !create_out.status.success() {
            return Err(Fail::new("Creating central_dev database failed"));
        }
        let revoke_out = process::feed(
            &mut psql_base("postgres", "central_dev", None),
            b"REVOKE CREATE, TEMP ON DATABASE central_dev FROM PUBLIC;\nREVOKE CREATE ON SCHEMA public FROM PUBLIC;\n",
            "Tightening central_dev database permissions failed",
        )?;
        if !revoke_out.status.success() {
            return Err(Fail::new(
                "Tightening central_dev database permissions failed",
            ));
        }
    }
    Ok(())
}

fn apply_migrations_and_grants(root: &Path, migrator_pw: &str) -> Result<()> {
    let init_history = process::feed(
        &mut psql_base("central_migrator", "central_dev", Some(migrator_pw)),
        br#"CREATE TABLE IF NOT EXISTS public."seaql_migrations" ( "version" varchar NOT NULL PRIMARY KEY, "applied_at" bigint NOT NULL );"#,
        "Ensuring migration history table failed",
    )?;
    if !init_history.status.success() {
        return Err(Fail::new("Ensuring migration history table failed"));
    }
    let recorded_out = process::capture(
        psql_base("central_migrator", "central_dev", Some(migrator_pw)).args([
            "-At",
            "-c",
            r#"SELECT version FROM public."seaql_migrations""#,
        ]),
        "Reading applied migration list failed",
    )?;
    if !recorded_out.status.success() {
        return Err(Fail::new("Reading applied migration list failed"));
    }
    let recorded: BTreeSet<String> = String::from_utf8_lossy(&recorded_out.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect();

    let mut migrations: Vec<(String, PathBuf)> =
        fs::read_dir(root.join("crates/central-migration/src"))
            .or_fail("Listing central migrations failed")?
            .filter_map(|entry| entry.ok().map(|e| e.path()))
            .filter(|path| path.extension().is_some_and(|ext| ext == "sql"))
            .filter_map(|path| {
                let stem = path.file_stem()?.to_str()?.to_owned();
                stem.starts_with('m').then_some((stem, path))
            })
            .collect();
    migrations.sort_by(|a, b| a.0.cmp(&b.0));

    for (stem, path) in migrations {
        if recorded.contains(&stem) {
            continue;
        }
        let mut script = fs::read_to_string(&path).or_fail("Reading migration SQL failed")?;
        let _ = writeln!(
            script,
            "\nINSERT INTO public.\"seaql_migrations\" (version, applied_at) VALUES ('{stem}', floor(extract(epoch FROM pg_catalog.clock_timestamp()))::bigint);"
        );
        let applied = process::feed(
            psql_base("central_migrator", "central_dev", Some(migrator_pw))
                .arg("--single-transaction"),
            script.as_bytes(),
            "Applying central database migration failed",
        )?;
        if !applied.status.success() {
            return Err(Fail::new(format!("Applying migration {stem} failed")));
        }
    }

    let grants = fs::read(root.join("ops/grants.sql")).or_fail("Reading ops/grants.sql failed")?;
    let grant_out = process::feed(
        &mut psql_base("central_migrator", "central_dev", Some(migrator_pw)),
        &grants,
        "Applying ops/grants.sql failed",
    )?;
    if !grant_out.status.success() {
        return Err(Fail::new("Applying ops/grants.sql failed"));
    }
    Ok(())
}

fn seed_dev_gateways(migrator_pw: &str) -> Result<()> {
    let mut sql = format!(
        "INSERT INTO central_identity.accounts (id, legacy_id, username_key, source, public_profile)\n\
         VALUES ('{DEV_OWNER_ID}'::uuid, 'dev-seed-owner', 'dev-seed-owner', 'legacy', false)\n\
         ON CONFLICT (id) DO NOTHING;\n"
    );
    for gw in &DEV_GATEWAYS {
        let (last_seen, players, playing, tables, games, activity_at) = match gw.activity {
            Some((players, playing, tables, games)) => (
                "pg_catalog.clock_timestamp()",
                players.to_string(),
                playing.to_string(),
                tables.to_string(),
                games.to_string(),
                "pg_catalog.clock_timestamp()",
            ),
            None => (
                "pg_catalog.clock_timestamp() - interval '2 hours'",
                "NULL".to_owned(),
                "NULL".to_owned(),
                "NULL".to_owned(),
                "NULL".to_owned(),
                "NULL",
            ),
        };
        let _ = writeln!(
            sql,
            "INSERT INTO central_identity.gateways (\
                id, owner_id, official, origin, name, version, website, \
                last_seen, deleted_at, \
                activity_players, activity_playing, activity_tables, activity_games, activity_at\
            ) VALUES (\
                '{}'::uuid, '{DEV_OWNER_ID}'::uuid, false, '{}', '{}', '{}', '{}', \
                {last_seen}, NULL, {players}, {playing}, {tables}, {games}, {activity_at}\
            ) ON CONFLICT (id) DO UPDATE SET \
                origin = EXCLUDED.origin, name = EXCLUDED.name, version = EXCLUDED.version, \
                website = EXCLUDED.website, last_seen = EXCLUDED.last_seen, deleted_at = NULL, \
                activity_players = EXCLUDED.activity_players, activity_playing = EXCLUDED.activity_playing, \
                activity_tables = EXCLUDED.activity_tables, activity_games = EXCLUDED.activity_games, \
                activity_at = EXCLUDED.activity_at;\n\
            INSERT INTO central_identity.gateway_approvals (gateway_id, enabled) \
            VALUES ('{}'::uuid, true) ON CONFLICT (gateway_id) DO UPDATE SET enabled = true;",
            gw.id, gw.origin, gw.name, gw.version, gw.website, gw.id,
        );
    }
    let seeded = process::feed(
        psql_base("central_migrator", "central_dev", Some(migrator_pw)).arg("--single-transaction"),
        sql.as_bytes(),
        "Seeding dev-site gateways failed",
    )?;
    if !seeded.status.success() {
        return Err(Fail::new("Seeding dev-site gateways failed"));
    }
    Ok(())
}

fn refresh_dev_gateway_heartbeats(migrator_pw: &str) -> Result<()> {
    let [online_1, online_2, online_3] = DEV_GATEWAY_ONLINE_IDS;
    let sql = format!(
        "UPDATE central_identity.gateways\n\
         SET last_seen = pg_catalog.clock_timestamp(),\n\
             activity_at = CASE\n\
                 WHEN activity_players IS NOT NULL THEN pg_catalog.clock_timestamp()\n\
                 ELSE NULL\n\
             END\n\
         WHERE id IN ('{online_1}'::uuid, '{online_2}'::uuid, '{online_3}'::uuid)\n\
           AND deleted_at IS NULL;\n"
    );
    let refreshed = process::feed(
        &mut psql_base("central_migrator", "central_dev", Some(migrator_pw)),
        sql.as_bytes(),
        "Refreshing dev-site gateway heartbeats failed",
    )?;
    if !refreshed.status.success() {
        return Err(Fail::new("Refreshing dev-site gateway heartbeats failed"));
    }
    Ok(())
}

fn psql_base(user: &str, db: &str, password: Option<&str>) -> Command {
    let mut cmd = Command::new(pg_tool("psql"));
    cmd.args(["-X", "-v", "ON_ERROR_STOP=1", "-h", "127.0.0.1"])
        .args(["-p", &PG_PORT.to_string(), "-U", user, "-d", db]);
    if let Some(pass) = password {
        cmd.env("PGPASSWORD", pass);
    }
    cmd
}

fn spawn_with_stdin(command: &mut Command, input: &[u8], failure: &str) -> Result<Child> {
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .or_fail(failure)?;
    let wrote = child
        .stdin
        .take()
        .ok_or_else(|| Fail::new(failure))
        .and_then(|mut stdin| {
            stdin
                .write_all(input)
                .and_then(|()| stdin.flush())
                .or_fail(failure)
        });
    if let Err(err) = wrote {
        let _ = process::stop_tree(&mut child);
        return Err(err);
    }
    Ok(child)
}

fn spawn_stack(
    root: &Path,
    bin_dir: &Path,
    site_dir: &Path,
    views_path: &Path,
    state: &Value,
    mail_token: &str,
) -> Result<Vec<Child>> {
    let service_bin = bin_dir
        .join("central-service")
        .with_extension(env::consts::EXE_EXTENSION);
    let worker_bin = bin_dir
        .join("central-picture-worker")
        .with_extension(env::consts::EXE_EXTENSION);
    let web_bin = bin_dir
        .join("central-web")
        .with_extension(env::consts::EXE_EXTENSION);

    let mut service_cfg = json!({
        "endpoint": format!("http://127.0.0.1:{ENDPOINT_PORT}"),
        "gateway": GATEWAY_URL,
        "database_port": PG_PORT,
        "database_user": "central_runtime",
        "database_name": "central_dev",
        "database_password": state["db_passwords"]["runtime"],
        "opaque_setup": state["secrets"]["opaque_setup"],
        "bridge": state["secrets"]["bridge"],
        "token_keys": state["secrets"]["token_keys"],
        "browser": {
            "listen": format!("127.0.0.1:{BROWSER_PORT}"),
            "origin": format!("http://127.0.0.1:{FRONT_PORT}"),
        },
        "mail": {
            "endpoint": format!("http://127.0.0.1:{MAIL_PORT}"),
            "site": format!("http://127.0.0.1:{FRONT_PORT}"),
            "token": mail_token,
            "logo": false,
        },
        "picture_worker": worker_bin,
    });
    if let Some(ffmpeg) = find_ffmpeg() {
        service_cfg["picture_ffmpeg"] = json!(ffmpeg);
    }
    let service_json =
        serde_json::to_vec(&service_cfg).or_fail("Serializing central-service settings failed")?;
    let mut central_child = spawn_with_stdin(
        Command::new(&service_bin)
            .arg("--dev-accounts-stdin")
            .current_dir(root),
        &service_json,
        "Starting central-service failed",
    )?;
    if let Err(err) = wait_for_service(&mut central_child) {
        let _ = process::stop_tree(&mut central_child);
        return Err(err);
    }

    let web_cfg = json!({
        "listen": format!("127.0.0.1:{SITE_PORT}"),
        "origin": format!("http://127.0.0.1:{FRONT_PORT}"),
        "site": site_dir,
        "central": format!("127.0.0.1:{BROWSER_PORT}"),
        "github": false,
        "fallback": root.join("crates/central-web-server/tests/data"),
        "views": views_path,
    });
    let web_json =
        serde_json::to_vec(&web_cfg).or_fail("Serializing central-web settings failed")?;
    let web_child = spawn_with_stdin(
        Command::new(&web_bin)
            .arg("--config-stdin")
            .current_dir(root),
        &web_json,
        "Starting central-web failed",
    )?;

    let front_script = root.join("crates/central-web/browser/e2e-real/front.ts");
    let front_child = Command::new("node")
        .arg(front_script)
        .args(["--site", &SITE_PORT.to_string()])
        .args(["--port", &FRONT_PORT.to_string()])
        .args(["--listener", &BROWSER_PORT.to_string()])
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .current_dir(root)
        .spawn()
        .or_fail("Starting front.ts reverse proxy failed")?;

    Ok(vec![central_child, web_child, front_child])
}

fn wait_for_service(child: &mut Child) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        if let Some(status) = child
            .try_wait()
            .or_fail("Inspecting central-service process failed")?
        {
            return Err(Fail::new(format!(
                "central-service exited during startup with {status}"
            )));
        }
        if tcp_open(ENDPOINT_PORT) && tcp_open(BROWSER_PORT) {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(50));
    }
    Err(Fail::new(
        "central-service listeners did not become ready within 15s",
    ))
}

fn tcp_open(port: u16) -> bool {
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    TcpStream::connect_timeout(&addr, Duration::from_millis(200)).is_ok()
}

fn wait_for_ports(children: &mut [Child]) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < deadline {
        for (idx, child) in children.iter_mut().enumerate() {
            if let Some(status) = child
                .try_wait()
                .or_fail("Inspecting dev-site process failed")?
            {
                return Err(Fail::new(format!(
                    "Dev-site process #{idx} exited during startup with {status}"
                )));
            }
        }
        if tcp_open(ENDPOINT_PORT)
            && tcp_open(BROWSER_PORT)
            && tcp_open(SITE_PORT)
            && tcp_open(FRONT_PORT)
            && tcp_open(MAIL_PORT)
        {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(200));
    }
    Err(Fail::new(
        "Dev-site listeners did not become ready within 30s",
    ))
}

fn supervise(children: &mut [Child], migrator_pw: &str) -> Result<()> {
    let mut last_pulse = Instant::now();
    loop {
        for (idx, child) in children.iter_mut().enumerate() {
            if let Some(status) = child
                .try_wait()
                .or_fail("Inspecting dev-site process failed")?
            {
                return Err(Fail::new(format!(
                    "Dev-site process #{idx} exited unexpectedly with {status}"
                )));
            }
        }
        if last_pulse.elapsed() >= DEV_GATEWAY_PULSE_INTERVAL {
            let _ = refresh_dev_gateway_heartbeats(migrator_pw);
            last_pulse = Instant::now();
        }
        thread::sleep(Duration::from_millis(250));
    }
}

struct DevMailerGuard {
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl Drop for DevMailerGuard {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (&x, &y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

fn start_dev_mailer(token: &str, log_path: &Path) -> Result<DevMailerGuard> {
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, MAIL_PORT));
    let listener =
        TcpListener::bind(addr).or_fail("Binding loopback dev-mailer on 127.0.0.1:28795 failed")?;
    listener
        .set_nonblocking(true)
        .or_fail("Configuring dev-mailer listener failed")?;
    let expected_auth = Arc::new(format!("Bearer {token}"));
    let log_path = Arc::new(log_path.to_path_buf());
    let log_lock = Arc::new(Mutex::new(()));
    let stop = Arc::new(AtomicBool::new(false));
    let stop_worker = Arc::clone(&stop);

    let handle = thread::Builder::new()
        .name("dev-mailer".into())
        .spawn(move || {
            while !stop_worker.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let _ = stream.set_nonblocking(false);
                        let auth = Arc::clone(&expected_auth);
                        let path = Arc::clone(&log_path);
                        let lock = Arc::clone(&log_lock);
                        let _ = thread::Builder::new().name("dev-mailer-conn".into()).spawn(
                            move || {
                                handle_mail_connection(stream, &auth, &path, &lock);
                            },
                        );
                    }
                    Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(25));
                    }
                    Err(_) => {
                        thread::sleep(Duration::from_millis(50));
                    }
                }
            }
        })
        .or_fail("Spawning dev-mailer thread failed")?;

    Ok(DevMailerGuard {
        stop,
        handle: Some(handle),
    })
}

fn find_header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|pos| pos + 4)
}

fn handle_mail_connection(
    mut stream: TcpStream,
    expected_auth: &str,
    log_path: &Path,
    log_lock: &Mutex<()>,
) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));
    let mut buf = Vec::with_capacity(2048);
    let mut chunk = [0u8; 1024];
    let header_end = loop {
        if let Some(end) = find_header_end(&buf) {
            break end;
        }
        if buf.len() >= MAX_MAIL_HEADER_BYTES {
            write_http_reply(
                &mut stream,
                400,
                "Bad Request",
                "{\"error\":\"headers_too_large\"}\n",
            );
            return;
        }
        let Ok(read) = stream.read(&mut chunk) else {
            return;
        };
        if read == 0 {
            return;
        }
        buf.extend_from_slice(&chunk[..read]);
    };
    let head = &buf[..header_end - 4];
    let remainder = &buf[header_end..];
    let (status, reason, reply_body) = process_mail_request(
        head,
        remainder,
        &mut stream,
        expected_auth,
        log_path,
        log_lock,
    );
    write_http_reply(&mut stream, status, reason, reply_body);
}

fn write_http_reply(writer: &mut impl std::io::Write, status: u16, reason: &str, body: &str) {
    let header = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = writer.write_all(header.as_bytes());
    let _ = writer.write_all(body.as_bytes());
    let _ = writer.flush();
}

fn parse_mail_head(
    head: &[u8],
    expected_auth: &str,
) -> std::result::Result<usize, (u16, &'static str, &'static str)> {
    let head_text = std::str::from_utf8(head)
        .map_err(|_| (400, "Bad Request", "{\"error\":\"invalid_utf8\"}\n"))?;
    let mut lines = head_text.split("\r\n");
    let request_line =
        lines
            .next()
            .ok_or((400, "Bad Request", "{\"error\":\"empty_request\"}\n"))?;
    let mut parts = request_line.split_ascii_whitespace();
    let (Some(method), Some(target), Some(_version)) = (parts.next(), parts.next(), parts.next())
    else {
        return Err((
            400,
            "Bad Request",
            "{\"error\":\"malformed_request_line\"}\n",
        ));
    };
    if method != "POST" {
        return Err((
            405,
            "Method Not Allowed",
            "{\"error\":\"method_not_allowed\"}\n",
        ));
    }
    if target != "/v1/send" {
        return Err((404, "Not Found", "{\"error\":\"not_found\"}\n"));
    }

    let mut auth_header: Option<&str> = None;
    let mut content_length: Option<usize> = None;
    for line in lines.filter(|line| !line.is_empty()) {
        let (name, value) = line.split_once(':').ok_or((
            400,
            "Bad Request",
            "{\"error\":\"malformed_header\"}\n",
        ))?;
        let key = name.trim();
        let val = value.trim();
        if key.eq_ignore_ascii_case("authorization") {
            auth_header = Some(val);
        } else if key.eq_ignore_ascii_case("content-length") {
            content_length = Some(val.parse::<usize>().map_err(|_| {
                (
                    400,
                    "Bad Request",
                    "{\"error\":\"invalid_content_length\"}\n",
                )
            })?);
        } else if key.eq_ignore_ascii_case("transfer-encoding") {
            return Err((400, "Bad Request", "{\"error\":\"unsupported_encoding\"}\n"));
        }
    }

    if !auth_header.is_some_and(|val| ct_eq(val.as_bytes(), expected_auth.as_bytes())) {
        return Err((401, "Unauthorized", "{\"error\":\"unauthorized\"}\n"));
    }
    content_length.ok_or((411, "Length Required", "{\"error\":\"length_required\"}\n"))
}

fn process_mail_request(
    head: &[u8],
    remainder: &[u8],
    reader: &mut impl std::io::Read,
    expected_auth: &str,
    log_path: &Path,
    log_lock: &Mutex<()>,
) -> (u16, &'static str, &'static str) {
    let len = match parse_mail_head(head, expected_auth) {
        Ok(len) => len,
        Err(err) => return err,
    };
    if len > MAX_MAIL_BODY_BYTES || remainder.len() > len {
        return (400, "Bad Request", "{\"error\":\"invalid_body_length\"}\n");
    }

    let mut body = vec![0u8; len];
    let pre = remainder.len();
    body[..pre].copy_from_slice(remainder);
    if pre < len && reader.read_exact(&mut body[pre..]).is_err() {
        return (400, "Bad Request", "{\"error\":\"truncated_body\"}\n");
    }

    let Ok(parsed) = serde_json::from_slice::<Value>(&body) else {
        return (400, "Bad Request", "{\"error\":\"invalid_json\"}\n");
    };
    let Some(to) = parsed.get("to").and_then(Value::as_str) else {
        return (400, "Bad Request", "{\"error\":\"missing_to\"}\n");
    };
    let Some(subject) = parsed.get("subject").and_then(Value::as_str) else {
        return (400, "Bad Request", "{\"error\":\"missing_subject\"}\n");
    };
    let text = parsed.get("text").and_then(Value::as_str);
    let html = parsed.get("html").and_then(Value::as_str);
    if text.is_none() && html.is_none() {
        return (400, "Bad Request", "{\"error\":\"missing_body\"}\n");
    }

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let entry = json!({
        "to": to,
        "subject": subject,
        "text": text,
        "html": html,
        "timestamp": timestamp,
    });
    let Ok(line) = serde_json::to_string(&entry) else {
        return (
            500,
            "Internal Server Error",
            "{\"error\":\"encode_failed\"}\n",
        );
    };

    if append_mail_jsonl(log_path, log_lock, &line).is_err() {
        return (
            500,
            "Internal Server Error",
            "{\"error\":\"spool_write_failed\"}\n",
        );
    }
    (202, "Accepted", "{\"ok\":true}\n")
}

fn append_mail_jsonl(log_path: &Path, log_lock: &Mutex<()>, line: &str) -> std::io::Result<()> {
    let _guard = log_lock
        .lock()
        .map_err(|_| std::io::Error::other("lock poisoned"))?;
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;
    writeln!(file, "{line}")?;
    file.flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn dev_mailer_parses_send_requests_and_writes_jsonl() {
        let dir = env::temp_dir().join(format!(
            "baylee-xtask-dev-mail-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let log_path = dir.join("dev-mail.jsonl");
        let lock = Mutex::new(());
        let expected_auth = "Bearer 0123456789abcdef0123456789abcdef";

        let payload = br#"{"to":"friend@example.org","subject":"Baylee friend invitation","text":"Hello","html":"<p>Hello</p>"}"#;
        let head = format!(
            "POST /v1/send HTTP/1.1\r\nHost: 127.0.0.1:28795\r\nAuthorization: {expected_auth}\r\nContent-Length: {}",
            payload.len()
        );
        let mut reader = Cursor::new(&payload[4..]);
        let (status, _, _) = process_mail_request(
            head.as_bytes(),
            &payload[..4],
            &mut reader,
            expected_auth,
            &log_path,
            &lock,
        );
        assert_eq!(status, 202);

        // Wrong token is refused with 401.
        let bad_head = format!(
            "POST /v1/send HTTP/1.1\r\nAuthorization: Bearer wrong\r\nContent-Length: {}",
            payload.len()
        );
        let mut reader2 = Cursor::new(payload);
        let (bad_status, _, _) = process_mail_request(
            bad_head.as_bytes(),
            &[],
            &mut reader2,
            expected_auth,
            &log_path,
            &lock,
        );
        assert_eq!(bad_status, 401);

        let content = fs::read_to_string(&log_path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 1);
        let stored: Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(stored["to"], "friend@example.org");
        assert_eq!(stored["subject"], "Baylee friend invitation");
        assert_eq!(stored["text"], "Hello");
        assert_eq!(stored["html"], "<p>Hello</p>");
        assert!(stored["timestamp"].as_u64().is_some());

        let _ = fs::remove_dir_all(&dir);
    }
}
