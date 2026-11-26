//! Process-level launch and redaction checks; owns and reaps every child.
#![forbid(unsafe_code)]

use central_policy::config::is_reserved_setting;
use std::{
    net::{SocketAddr, TcpStream},
    process::{Child, Command, Output, Stdio},
    time::Duration,
};

struct OwnedChild(Option<Child>);

impl Drop for OwnedChild {
    fn drop(&mut self) {
        if let Some(child) = &mut self.0 {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn run_invalid(arguments: &[&str], setting: Option<(&str, &str)>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_central-service"));
    command
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW: owned test processes must not open desktop windows.
        command.creation_flags(0x0800_0000);
    }
    for (key, _) in std::env::vars_os() {
        if is_reserved_setting(&key.to_string_lossy()) {
            command.env_remove(key);
        }
    }
    command.env("BAYLEE_CENTRAL_LISTEN", "127.0.0.1:0");
    if let Some((key, value)) = setting {
        command.env(key, value);
    }
    let mut owned = OwnedChild(Some(command.spawn().unwrap()));
    let mut completed = false;
    for _ in 0..150 {
        if owned.0.as_mut().unwrap().try_wait().unwrap().is_some() {
            completed = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    if !completed {
        owned.0.as_mut().unwrap().kill().unwrap();
        owned.0.as_mut().unwrap().wait().unwrap();
    }
    let output = owned.0.take().unwrap().wait_with_output().unwrap();
    // Even a regression that accidentally starts the server must leave no listener.
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        if let Some(address) = line.strip_prefix("central: development listener at http://") {
            let address: SocketAddr = address.parse().unwrap();
            assert!(TcpStream::connect_timeout(&address, Duration::from_millis(200)).is_err());
        }
    }
    assert!(
        completed,
        "invalid configuration accidentally started a server"
    );
    assert!(!output.status.success());
    output
}

#[test]
fn bad_reserved_environment_is_rejected_without_echoing_values() {
    for key in [
        "BAYLEE_CENTRAL_REQUEST_BYTES",
        "baylee_central_request_bytes",
        "Baylee_Central_UNKNOWN",
    ] {
        let output = run_invalid(&["--dev"], Some((key, "PRIVATE_CONFIG_CANARY")));
        assert_eq!(output.stdout, b"");
        assert_eq!(
            String::from_utf8(output.stderr).unwrap().trim(),
            "central: invalid configuration"
        );
    }
}

/// The usage: every mode, in the order `--modes` names them.
const USAGE: &str = "usage: central-service --dev | --dev-accounts-stdin | --accounts-stdin | --check-accounts-stdin | --migrate-stdin | --appoint-admin-stdin | --sweep-stdin | --after-restore-stdin | --erase-accounts-stdin | --legacy-import-stdin | --legacy-deadline-stdin | --import-reports-stdin | --advent-import-stdin | --ranked-cosmetics-import-stdin | --cards-ingest-stdin | --generate-secrets | --generate-token-key | --modes";

#[test]
fn an_explicit_mode_is_required_and_extra_arguments_are_rejected() {
    for arguments in [
        &[][..],
        &["--dev", "PRIVATE_ARG_CANARY"],
        &["--production"],
        &["--accounts-stdin", "PRIVATE_ARG_CANARY"],
        &["--migrate-stdin", "--dev"],
        &["--issue-beta-keys-stdin", "PRIVATE_ARG_CANARY"],
        // the closed beta's keys went on 11.11.2026, and their mode with them
        &["--issue-beta-keys-stdin"],
        &["--appoint-admin-stdin", "PRIVATE_ARG_CANARY"],
        &["--sweep-stdin", "--dev"],
        &["--legacy-import-stdin", "PRIVATE_ARG_CANARY"],
        &["--legacy-deadline-stdin", "--dev"],
        &["--import-reports-stdin", "PRIVATE_ARG_CANARY"],
        &["--advent-import-stdin", "PRIVATE_ARG_CANARY"],
        &["--ranked-cosmetics-import-stdin", "PRIVATE_ARG_CANARY"],
        &["--cards-ingest-stdin", "PRIVATE_ARG_CANARY"],
        &["--generate-secrets", "PRIVATE_ARG_CANARY"],
        &["--generate-token-key", "PRIVATE_ARG_CANARY"],
        &["--check-accounts-stdin", "PRIVATE_ARG_CANARY"],
        &["--after-restore-stdin", "PRIVATE_ARG_CANARY"],
        &["--erase-accounts-stdin", "--dev"],
        &["--modes", "PRIVATE_ARG_CANARY"],
        &["--DEV"],
        &["--erase-accounts-stdin=PRIVATE_ARG_CANARY"],
    ] {
        let output = run_invalid(arguments, None);
        assert_eq!(output.status.code(), Some(2), "{arguments:?}");
        assert_eq!(output.stdout, b"");
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(!stderr.contains("PRIVATE_ARG_CANARY"));
        assert_eq!(stderr.trim(), USAGE);
    }
}

#[test]
fn production_modes_reject_missing_or_invalid_input_without_echoing_it() {
    for (mode, message) in [
        (
            "--accounts-stdin",
            "central: account service rejected configuration or failed",
        ),
        (
            "--migrate-stdin",
            "central: migration rejected configuration or failed",
        ),
        (
            "--appoint-admin-stdin",
            "central: appointment rejected configuration, found no enrolled account of that name, or failed",
        ),
        (
            "--sweep-stdin",
            "central: sweep rejected configuration or failed",
        ),
        (
            "--after-restore-stdin",
            "central: the revocation streams were not moved: configuration rejected or failed",
        ),
        (
            "--erase-accounts-stdin",
            "central: erasure rejected its input or failed after 0 accounts erased and 0 not found; run the same list again",
        ),
        (
            "--advent-import-stdin",
            "Advent import refused. Verify the private package, worker and staging role.",
        ),
        (
            "--ranked-cosmetics-import-stdin",
            "Ranked cosmetic import refused. Verify the private package, worker and staging role.",
        ),
    ] {
        let output = run_invalid(&[mode], None);
        assert_eq!(output.status.code(), Some(1), "{mode}");
        assert_eq!(output.stdout, b"");
        assert_eq!(String::from_utf8(output.stderr).unwrap().trim(), message);
        let output = run_with_stdin(mode, br#"{"database":{"password":"PRIVATE_DB_CANARY"}}"#);
        assert_eq!(output.status.code(), Some(1), "{mode}");
        assert_eq!(output.stdout, b"");
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(!stderr.contains("PRIVATE_DB_CANARY"));
        assert_eq!(stderr.trim(), message);
    }
}

/// `--modes` names every mode, reads nothing and connects to nothing; each
/// mode it names is one the launcher runs, never the usage.
#[test]
fn the_modes_are_named_and_each_one_runs() {
    let output = run_with_stdin("--modes", b"PRIVATE_STDIN_CANARY");
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stderr, b"");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(stdout.lines().count(), 1);
    let named: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let modes: Vec<&str> = named["modes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|mode| mode.as_str().unwrap())
        .collect();
    assert_eq!(named.as_object().unwrap().len(), 1);
    assert_eq!(
        format!("usage: central-service {}", modes.join(" | ")),
        USAGE
    );
    // `--dev` would listen; the others refuse empty input or need none.
    for mode in modes.iter().filter(|&&mode| mode != "--dev") {
        let output = run_invalid_or_done(mode);
        assert_ne!(output.status.code(), Some(2), "{mode}");
        assert!(
            !String::from_utf8_lossy(&output.stderr).contains("usage:"),
            "{mode}"
        );
    }
}

/// A mode run with empty standard input, which every mode with input refuses.
fn run_invalid_or_done(mode: &str) -> Output {
    run_with_stdin(mode, b"")
}

/// Erasure checks every id before it connects: one that is no UUID, an empty
/// list or one past the batch refuses all, and the input is never echoed.
#[test]
fn erasure_refuses_a_bad_list_before_it_connects() {
    let database = r#""database":{"host":"127.0.0.1","port":1,"name":"central","user":"central_migrator","password":"PRIVATE_DB_CANARY","ca_certificate":"x"}"#;
    let message = "central: erasure rejected its input or failed after 0 accounts erased and 0 not found; run the same list again";
    let good = "0193a5b2-7c41-7d3e-9f00-0123456789ab";
    let past: Vec<String> = (0..=1024)
        .map(|n| format!("\"00000000-0000-7000-8000-{n:012x}\""))
        .collect();
    for accounts in [
        "[]".to_owned(),
        format!(r#"["{good}","PRIVATE_ID_CANARY"]"#),
        format!(r#"["{good}",7]"#),
        format!("[{}]", past.join(",")),
        format!(r#""{good}""#),
    ] {
        let output = run_with_stdin(
            "--erase-accounts-stdin",
            format!(r#"{{{database},"accounts":{accounts}}}"#).as_bytes(),
        );
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(output.stdout, b"");
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(!stderr.contains("CANARY"), "{stderr}");
        assert_eq!(stderr.trim(), message);
    }
    // An unknown field refuses too, whatever it holds.
    let output = run_with_stdin(
        "--erase-accounts-stdin",
        format!(r#"{{{database},"accounts":["{good}"],"force":"PRIVATE_FIELD_CANARY"}}"#)
            .as_bytes(),
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(!String::from_utf8(output.stderr).unwrap().contains("CANARY"));
    let output = run_with_stdin(
        "--after-restore-stdin",
        format!(r#"{{{database},"accounts":["PRIVATE_FIELD_CANARY"]}}"#).as_bytes(),
    );
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(output.stdout, b"");
    assert!(!String::from_utf8(output.stderr).unwrap().contains("CANARY"));
}

/// The reports' copy refuses what is not its migration document, and says
/// why without echoing what it was given.
#[test]
fn the_reports_import_refuses_without_echoing_its_input() {
    let message = "central: reports import refused, nothing was stored: the first document is not the operator's {\"database\": …}";
    let output = run_invalid(&["--import-reports-stdin"], None);
    assert_eq!(output.stdout, b"");
    assert_eq!(String::from_utf8(output.stderr).unwrap().trim(), message);
    let output = run_with_stdin(
        "--import-reports-stdin",
        br#"{"database":{"password":"PRIVATE_DB_CANARY"}}"#,
    );
    assert!(!output.status.success());
    assert_eq!(output.stdout, b"");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(!stderr.contains("PRIVATE_DB_CANARY"));
    assert_eq!(stderr.trim(), message);
}

#[test]
fn generated_secrets_go_to_a_pipe_and_never_to_the_terminal_check_output() {
    let output = Command::new(env!("CARGO_BIN_EXE_central-service"))
        .arg("--generate-secrets")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(output.stderr, b"");
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.starts_with("{\"opaque_setup\":[1,"));
    assert!(text.contains(",\"bridge\":["));
}

fn run_with_stdin(mode: &str, input: &[u8]) -> Output {
    use std::io::Write;
    let mut child = OwnedChild(Some(
        Command::new(env!("CARGO_BIN_EXE_central-service"))
            .arg(mode)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    ));
    let process = child.0.as_mut().unwrap();
    let mut stdin = process.stdin.take().unwrap();
    stdin.write_all(input).unwrap();
    drop(stdin);
    child.0.take().unwrap().wait_with_output().unwrap()
}

#[test]
fn account_service_rejects_missing_secret_input_and_extra_arguments() {
    let output = run_invalid(&["--dev-accounts-stdin"], None);
    assert_eq!(output.stdout, b"");
    assert_eq!(
        String::from_utf8(output.stderr).unwrap().trim(),
        "central: development account service rejected configuration or failed"
    );
    let output = run_invalid(&["--dev-accounts-stdin", "PRIVATE_ARG_CANARY"], None);
    assert!(
        !String::from_utf8(output.stderr)
            .unwrap()
            .contains("PRIVATE_ARG_CANARY")
    );
}
