//! Loopback checks: whether a port this task owned is closed again.

use std::io::ErrorKind;
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::time::Duration;

/// Whether 127.0.0.1:`port` refuses a connection now. A connection that
/// succeeds, times out or fails any other way is not proof of closure.
pub(crate) fn refused(port: u16) -> bool {
    let address = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    matches!(
        TcpStream::connect_timeout(&address, Duration::from_secs(5)),
        Err(error) if error.kind() == ErrorKind::ConnectionRefused
    )
}
