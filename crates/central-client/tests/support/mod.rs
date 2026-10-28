use central_api::{
    Endpoint, Error, Token,
    wire::{Request, Response},
};
use central_client::Transport;
use central_postgres::DevConfig;
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
pub(super) mod admin;
pub(super) mod browser;
pub(super) mod gateways;
pub(super) mod password;
pub(super) mod terms;

pub(super) fn config(user: &str, password: &str) -> DevConfig {
    assert_eq!(
        std::env::var("CENTRAL_TEST_FIXTURE").unwrap(),
        "isolated-v1"
    );
    DevConfig::new(
        std::env::var("CENTRAL_TEST_PORT").unwrap().parse().unwrap(),
        user,
        std::env::var(password).unwrap(),
        "central_native_test",
    )
    .unwrap()
}

pub(super) struct Http {
    pub(super) endpoint: Endpoint,
    pub(super) address: SocketAddr,
    pub(super) lose_write_reply: AtomicBool,
    /// Lose the answer to the next committed `ChangePasswordFinish`.
    pub(super) lose_password_reply: AtomicBool,
}

impl Transport for Http {
    fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }
    fn exchange(&self, request: &Request, bearer: Option<&Token>) -> Result<Response, Error> {
        let bytes = central_api::encode(request)?;
        let mut stream = TcpStream::connect_timeout(&self.address, Duration::from_secs(3))
            .map_err(|_| Error::Unavailable)?;
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        write!(stream, "POST /v1/native HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n", self.address, bytes.len()).unwrap();
        if let Some(token) = bearer {
            write!(stream, "Authorization: {}\r\n", token.bearer().as_str()).unwrap();
        }
        stream.write_all(b"\r\n").unwrap();
        stream.write_all(&bytes).unwrap();
        let mut response = Vec::new();
        stream
            .take((central_api::MAX_WIRE_BYTES + 4096) as u64)
            .read_to_end(&mut response)
            .map_err(|_| Error::Unavailable)?;
        let split = response
            .windows(4)
            .position(|w| w == b"\r\n\r\n")
            .ok_or(Error::Rejected)?;
        let head = std::str::from_utf8(&response[..split]).map_err(|_| Error::Rejected)?;
        assert!(head.to_lowercase().contains("cache-control: no-store"));
        let status = head.split_whitespace().nth(1).ok_or(Error::Rejected)?;
        match status {
            "200" => {}
            "403" => return Err(Error::TermsRequired),
            "409" => return Err(Error::Conflict),
            "429" => return Err(Error::Busy),
            "503" => return Err(Error::Unavailable),
            _ => return Err(Error::Rejected),
        }
        if matches!(request, Request::WriteDocument { .. })
            && self.lose_write_reply.swap(false, Ordering::AcqRel)
        {
            return Err(Error::Unavailable);
        }
        if matches!(request, Request::ChangePasswordFinish { .. })
            && self.lose_password_reply.swap(false, Ordering::AcqRel)
        {
            return Err(Error::Unavailable);
        }
        central_api::decode(&response[split + 4..])
    }
}
