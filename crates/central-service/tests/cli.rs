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

#[test]
fn an_explicit_mode_is_required_and_extra_arguments_are_rejected() {
    for arguments in [
        &[][..],
        &["--dev", "PRIVATE_ARG_CANARY"],
        &["--production"],
        &["--accounts-stdin", "PRIVATE_ARG_CANARY"],
        &["--migrate-stdin", "--dev"],
        &["--issue-beta-keys-stdin", "PRIVATE_ARG_CANARY"],
        &["--appoint-admin-stdin", "PRIVATE_ARG_CANARY"],
        &["--sweep-stdin", "--dev"],
        &["--legacy-import-stdin", "PRIVATE_ARG_CANARY"],
        &["--legacy-deadline-stdin", "--dev"],
        &["--generate-secrets", "PRIVATE_ARG_CANARY"],
        &["--DEV"],
    ] {
        let output = run_invalid(arguments, None);
        assert_eq!(output.stdout, b"");
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(!stderr.contains("PRIVATE_ARG_CANARY"));
        assert_eq!(
            stderr.trim(),
            "usage: central-service --dev | --dev-accounts-stdin | --accounts-stdin | --migrate-stdin | --issue-beta-keys-stdin | --appoint-admin-stdin | --sweep-stdin | --legacy-import-stdin | --legacy-deadline-stdin | --generate-secrets"
        );
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
            "--issue-beta-keys-stdin",
            "central: beta keys rejected configuration or failed; none were issued",
        ),
        (
            "--appoint-admin-stdin",
            "central: appointment rejected configuration, found no enrolled account of that name, or failed",
        ),
        (
            "--sweep-stdin",
            "central: sweep rejected configuration or failed",
        ),
    ] {
        let output = run_invalid(&[mode], None);
        assert_eq!(output.stdout, b"");
        assert_eq!(String::from_utf8(output.stderr).unwrap().trim(), message);
        let output = run_with_stdin(mode, br#"{"database":{"password":"PRIVATE_DB_CANARY"}}"#);
        assert!(!output.status.success());
        assert_eq!(output.stdout, b"");
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(!stderr.contains("PRIVATE_DB_CANARY"));
        assert_eq!(stderr.trim(), message);
    }
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
