//! Git variables that point Git at another repository stop the secret scan
//! before Git runs, in any case of their names.
#![forbid(unsafe_code)]

use xtask::secrets::{REROUTED, ROUTING, reject_rerouting};

#[test]
fn every_routing_variable_is_refused() {
    for name in ROUTING {
        for spelling in [name.to_owned(), name.to_ascii_lowercase()] {
            let refused =
                reject_rerouting([spelling.clone().into()]).map_err(|f| f.message().to_owned());
            assert_eq!(refused, Err(REROUTED.to_owned()), "{spelling}");
        }
    }
}

#[test]
fn other_git_variables_pass() {
    assert_eq!(
        reject_rerouting(["GIT_AUTHOR_NAME".into(), "PATH".into(), "HOME".into()]),
        Ok(())
    );
}
