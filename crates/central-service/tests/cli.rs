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
fn explicit_dev_flag_is_required_and_extra_arguments_are_rejected() {
    for arguments in [&[][..], &["--dev", "PRIVATE_ARG_CANARY"], &["--production"]] {
        let output = run_invalid(arguments, None);
        assert_eq!(output.stdout, b"");
        assert_eq!(
            String::from_utf8(output.stderr).unwrap().trim(),
            "usage: central-service --dev (local foundation only)"
        );
    }
}
