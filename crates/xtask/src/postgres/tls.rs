//! Synthetic TLS for the fixture: a test authority, an unrelated "wrong"
//! authority, and a server certificate naming only 127.0.0.1, so the deployed
//! connector's verified TLS is exercised. The authorities' keys are deleted
//! as soon as the certificate is signed.

use std::fs;
use std::process::{Command, Stdio};

use crate::fail::{OrFail, Result};
use crate::owned::OwnedDir;
use crate::process;

const FAILURE: &str = "Synthetic TLS certificate generation failed";
const EXTENSIONS: &str = concat!(
    "subjectAltName=IP:127.0.0.1\n",
    "basicConstraints=critical,CA:FALSE\n",
    "keyUsage=critical,digitalSignature\n",
    "extendedKeyUsage=serverAuth\n",
);

pub(super) fn authority() -> Result<OwnedDir> {
    let dir = OwnedDir::new("central-tls-")?;
    let at = |name: &str| dir.path().join(name).to_string_lossy().into_owned();
    fs::write(dir.path().join("server.ext"), EXTENSIONS).or_fail(FAILURE)?;
    let ec = [
        "-newkey",
        "ec",
        "-pkeyopt",
        "ec_paramgen_curve:P-256",
        "-nodes",
    ];
    let steps: [Vec<String>; 4] = [
        args(
            &["req", "-x509"],
            &ec,
            &[
                "-days",
                "2",
                "-subj",
                "/CN=central-test-ca",
                "-keyout",
                &at("ca.key"),
                "-out",
                &at("ca.pem"),
            ],
        ),
        args(
            &["req", "-x509"],
            &ec,
            &[
                "-days",
                "2",
                "-subj",
                "/CN=central-wrong-ca",
                "-keyout",
                &at("wrong.key"),
                "-out",
                &at("wrong.pem"),
            ],
        ),
        args(
            &["req"],
            &ec,
            &[
                "-subj",
                "/CN=127.0.0.1",
                "-keyout",
                &at("server.key"),
                "-out",
                &at("server.csr"),
            ],
        ),
        args(
            &[
                "x509",
                "-req",
                "-in",
                &at("server.csr"),
                "-CA",
                &at("ca.pem"),
                "-CAkey",
                &at("ca.key"),
            ],
            &["-CAcreateserial", "-days", "2"],
            &["-extfile", &at("server.ext"), "-out", &at("server.crt")],
        ),
    ];
    for step in steps {
        process::succeed(
            Command::new("openssl")
                .args(step)
                .stdout(Stdio::null())
                .stderr(Stdio::null()),
            FAILURE,
        )?;
    }
    for key in ["ca.key", "wrong.key"] {
        fs::remove_file(dir.path().join(key)).or_fail(FAILURE)?;
    }
    Ok(dir)
}

fn args(a: &[&str], b: &[&str], c: &[&str]) -> Vec<String> {
    a.iter()
        .chain(b)
        .chain(c)
        .map(|s| (*s).to_owned())
        .collect()
}
