//! Central's outbound HTTPS. Every destination comes from outside, so every call is held
//! to one rule: HTTPS only, to public addresses only, with no proxy, no redirect and short
//! time limits.
//!
//! The address is checked and pinned in one step. A [`Resolver`]'s answer is exactly what
//! the connector dials, so there is no second lookup a name could answer differently, and
//! one private address among a name's answers refuses the whole name.
//!
//! There is one exception, and it is a named one: a [`Pin`] gives one exact host one address
//! on this machine or its network, set by the operator. The URL, the name sent in TLS and the
//! certificate check stay what they were, so it is the same verified HTTPS to another address.
//! No other name reaches a private address, with or without a pin.
#![forbid(unsafe_code)]
use std::{
    net::{IpAddr, SocketAddr},
    time::Duration,
};
pub use ureq::{Agent, Error};
use ureq::{
    config::Config,
    http::Uri,
    unversioned::{
        resolver::{DefaultResolver, ResolvedSocketAddrs, Resolver},
        transport::{DefaultConnector, NextTimeout},
    },
};

/// One exact host, and the one address it is reached at.
#[derive(Clone, Debug)]
pub struct Pin {
    host: String,
    address: SocketAddr,
}

impl Pin {
    /// Reach `host`, and no name like it, at `address`.
    ///
    /// # Errors
    /// Refuses an address that is not loopback or private: a public address needs no pin, and
    /// a link-local one is where a cloud host answers about itself. Refuses an empty host.
    pub fn new(host: &str, address: SocketAddr) -> Result<Self, PinRefused> {
        if host.is_empty() || !local(address.ip()) {
            return Err(PinRefused);
        }
        Ok(Self {
            host: host.to_owned(),
            address,
        })
    }
}

/// The address cannot be pinned.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PinRefused;

/// Loopback, or private to this network, however it is spelled.
fn local(ip: IpAddr) -> bool {
    match ip.to_canonical() {
        IpAddr::V4(ip) => ip.is_loopback() || ip.is_private(),
        IpAddr::V6(ip) => ip.is_loopback() || ip.segments()[0] & 0xfe00 == 0xfc00,
    }
}

/// The limits of one kind of call.
#[derive(Clone)]
pub struct Limits {
    /// The only port allowed, or `None` for the port the URL names.
    pub port: Option<u16>,
    /// The whole call's limit.
    pub total: Duration,
    /// The connection's limit.
    pub connect: Duration,
    /// The one host reached at a set address, if any.
    pub pin: Option<Pin>,
}

#[derive(Debug)]
struct PublicResolver<R> {
    inner: R,
    port: Option<u16>,
    pin: Option<Pin>,
}

impl<R: Resolver> Resolver for PublicResolver<R> {
    fn resolve(
        &self,
        uri: &Uri,
        config: &Config,
        timeout: NextTimeout,
    ) -> Result<ResolvedSocketAddrs, Error> {
        if let Some(pin) = &self.pin
            && uri.host() == Some(pin.host.as_str())
        {
            let mut addresses = self.inner.empty();
            addresses.push(pin.address);
            return Ok(addresses);
        }
        let addresses = self.inner.resolve(uri, config, timeout)?;
        if addresses.is_empty()
            || addresses
                .iter()
                .any(|a| !public(a.ip()) || self.port.is_some_and(|port| a.port() != port))
        {
            return Err(Error::HostNotFound);
        }
        Ok(addresses)
    }
}

/// Whether an address is reachable on the public internet: never a private, loopback,
/// link-local, multicast, reserved or documentation one, however it is spelled.
#[must_use]
pub fn public(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            let [a, b, c, _] = ip.octets();
            !(a == 0
                || a == 10
                || a == 127
                || a >= 224
                || (a == 100 && (64..=127).contains(&b))
                || (a == 169 && b == 254)
                || (a == 172 && (16..=31).contains(&b))
                || (a == 192
                    && (b == 168 || (b == 0 && (c == 0 || c == 2)) || (b == 88 && c == 99)))
                || (a == 198 && (b == 18 || b == 19 || (b == 51 && c == 100)))
                || (a == 203 && b == 0 && c == 113))
        }
        IpAddr::V6(ip) => {
            if let Some(v4) = ip.to_ipv4_mapped() {
                return public(IpAddr::V4(v4));
            }
            let s = ip.segments();
            s[0] & 0xe000 == 0x2000
                && s[0] != 0x2002
                && !(s[0] == 0x2001 && (s[1] < 0x200 || s[1] == 0xdb8))
        }
    }
}

/// An agent held to the rule, within `limits`.
#[must_use]
pub fn agent(limits: Limits) -> Agent {
    let config = Agent::config_builder()
        .https_only(true)
        .proxy(None)
        .max_redirects(0)
        .timeout_global(Some(limits.total))
        .timeout_connect(Some(limits.connect))
        .max_response_header_size(8192)
        .http_status_as_error(false)
        .build();
    Agent::with_parts(
        config,
        DefaultConnector::default(),
        PublicResolver {
            inner: DefaultResolver::default(),
            port: limits.port,
            pin: limits.pin,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;
    use ureq::unversioned::transport::NextTimeout;

    /// A name server that answers what a hostile name would.
    #[derive(Debug)]
    struct Answers(Vec<SocketAddr>);

    impl Resolver for Answers {
        fn resolve(
            &self,
            _: &Uri,
            _: &Config,
            _: NextTimeout,
        ) -> Result<ResolvedSocketAddrs, Error> {
            let mut addresses = self.empty();
            for address in &self.0 {
                addresses.push(*address);
            }
            Ok(addresses)
        }
    }

    fn answered(
        url: &str,
        answers: &[&str],
        port: Option<u16>,
        pin: Option<Pin>,
    ) -> Result<Vec<SocketAddr>, Error> {
        let resolver = PublicResolver {
            inner: Answers(answers.iter().map(|a| a.parse().unwrap()).collect()),
            port,
            pin,
        };
        let uri: Uri = url.parse().unwrap();
        let config = Agent::config_builder().build();
        resolver
            .resolve(&uri, &config, NextTimeout::default())
            .map(|addresses| addresses.iter().copied().collect())
    }

    fn resolve(answers: &[&str], port: Option<u16>) -> Result<usize, Error> {
        answered("https://gateway.example/x", answers, port, None).map(|found| found.len())
    }

    fn pin() -> Pin {
        Pin::new("gateway.example", "127.0.0.1:443".parse().unwrap()).unwrap()
    }

    #[test]
    fn refuses_private_mapped_link_local_and_documentation_addresses() {
        for ip in [
            "127.0.0.1",
            "10.0.0.1",
            "172.16.0.1",
            "192.168.1.1",
            "169.254.169.254",
            "100.64.0.1",
            "0.0.0.0",
            "224.0.0.1",
            "255.255.255.255",
            "192.0.2.1",
            "198.51.100.1",
            "203.0.113.1",
            "::",
            "::1",
            "::ffff:127.0.0.1",
            "::ffff:10.0.0.1",
            "::ffff:169.254.169.254",
            "fe80::1",
            "fc00::1",
            "fd00::1",
            "ff02::1",
            "2001:db8::1",
            "2002:7f00:1::",
            "64:ff9b::7f00:1",
        ] {
            assert!(!public(ip.parse().unwrap()), "{ip}");
        }
        for ip in ["8.8.8.8", "1.1.1.1", "2606:4700:4700::1111"] {
            assert!(public(ip.parse().unwrap()), "{ip}");
        }
    }

    #[test]
    fn a_name_with_one_private_answer_is_refused_whole() {
        assert_eq!(resolve(&["8.8.8.8:443"], None).unwrap(), 1);
        assert!(resolve(&["10.0.0.1:443"], None).is_err());
        // The rebinding shape: a public answer beside the private one it hopes to reach.
        assert!(resolve(&["8.8.8.8:443", "127.0.0.1:443"], None).is_err());
        assert!(resolve(&["[::ffff:192.168.0.1]:443", "8.8.8.8:443"], None).is_err());
    }

    #[test]
    fn no_answer_is_a_refusal() {
        assert!(resolve(&[], None).is_err());
    }

    #[test]
    fn a_fixed_port_refuses_any_other() {
        assert_eq!(resolve(&["8.8.8.8:443"], Some(443)).unwrap(), 1);
        assert!(resolve(&["8.8.8.8:8443"], Some(443)).is_err());
        assert_eq!(resolve(&["8.8.8.8:8443"], None).unwrap(), 1);
    }

    #[test]
    fn the_pinned_host_goes_to_its_address_and_asks_no_name_server() {
        // Whatever a name server would say about it, the operator's address is what is dialled.
        let found = answered(
            "https://gateway.example/auth/central/invalidate",
            &["8.8.8.8:443", "10.9.9.9:443"],
            None,
            Some(pin()),
        )
        .unwrap();
        assert_eq!(found, vec!["127.0.0.1:443".parse().unwrap()]);
    }

    #[test]
    fn a_pin_opens_no_other_name() {
        for url in [
            "https://evil.example/x",
            "https://gateway.example.evil.example/x",
            "https://evil-gateway.example/x",
            "https://sub.gateway.example/x",
            "https://gateway.exampl/x",
        ] {
            // Each still answers to the rule, so a private answer still refuses the name.
            assert!(
                answered(url, &["127.0.0.1:443"], None, Some(pin())).is_err(),
                "{url}"
            );
            assert!(
                answered(url, &["10.0.0.1:443"], None, Some(pin())).is_err(),
                "{url}"
            );
        }
    }

    #[test]
    fn only_loopback_and_private_addresses_can_be_pinned() {
        for address in [
            "127.0.0.1:443",
            "127.8.8.8:443",
            "10.0.0.5:443",
            "172.16.0.5:443",
            "192.168.1.5:8443",
            "[::1]:443",
            "[fd00::5]:443",
            "[::ffff:127.0.0.1]:443",
        ] {
            assert!(
                Pin::new("gateway.example", address.parse().unwrap()).is_ok(),
                "{address}"
            );
        }
        for address in [
            "8.8.8.8:443",
            "169.254.169.254:80",
            "0.0.0.0:443",
            "224.0.0.1:443",
            "100.64.0.1:443",
            "[::]:443",
            "[fe80::1]:443",
            "[ff02::1]:443",
            "[2606:4700:4700::1111]:443",
            "[::ffff:169.254.169.254]:80",
            "[::ffff:8.8.8.8]:443",
        ] {
            assert_eq!(
                Pin::new("gateway.example", address.parse().unwrap()).err(),
                Some(PinRefused),
                "{address}"
            );
        }
        assert!(Pin::new("", "127.0.0.1:443".parse().unwrap()).is_err());
    }

    #[test]
    fn an_agent_refuses_plain_http_and_a_literal_private_address_without_connecting() {
        let agent = agent(Limits {
            port: None,
            total: Duration::from_secs(2),
            connect: Duration::from_secs(1),
            pin: None,
        });
        for url in ["http://8.8.8.8/x", "https://127.0.0.1/x", "https://[::1]/x"] {
            assert!(agent.post(url).send("x").is_err(), "{url}");
        }
    }
}
