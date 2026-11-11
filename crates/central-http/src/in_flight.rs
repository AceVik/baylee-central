//! Places in flight per client address: how many requests one address may
//! have waiting at once, in a bounded table in memory. The browser listener
//! holds a place for a whole request (`browser.rs`), the native boundary for
//! a body that is late or large (`boundary.rs`). A place is fairness only:
//! it refuses a request beyond its address's share and never grants anything.
use axum::http::HeaderMap;
use std::{
    collections::BTreeMap,
    net::IpAddr,
    sync::{Mutex, PoisonError},
};

/// A client address as places and budgets count it. IPv6 counts by its /64,
/// the block one subscriber usually holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Client {
    V4([u8; 4]),
    V6([u8; 8]),
}

impl From<IpAddr> for Client {
    fn from(ip: IpAddr) -> Self {
        match ip.to_canonical() {
            IpAddr::V4(ip) => Self::V4(ip.octets()),
            IpAddr::V6(ip) => {
                let mut prefix = [0; 8];
                prefix.copy_from_slice(&ip.octets()[..8]);
                Self::V6(prefix)
            }
        }
    }
}

/// The last `X-Forwarded-For` hop, which the local proxy appended; `None`
/// without one or when it is not a bare address.
pub(crate) fn forwarded(headers: &HeaderMap) -> Option<Client> {
    let last = headers.get_all("x-forwarded-for").iter().next_back()?;
    let hop = last
        .to_str()
        .ok()?
        .rsplit(',')
        .next()?
        .trim_matches([' ', '\t']);
    Some(Client::from(hop.parse::<IpAddr>().ok()?))
}

/// The places held: each address's, and their sum.
#[derive(Default)]
struct Held {
    by_client: BTreeMap<Client, u8>,
    total: usize,
}

/// The requests in flight per client address, in memory only: at most
/// `clients` addresses and `places` places at once. An address leaves the
/// table with its last request.
pub(crate) struct InFlight {
    held: Mutex<Held>,
    clients: usize,
    places: usize,
}

/// One request's place among its address's requests in flight; dropping it,
/// however the request ends (answered, refused, timed out, its connection
/// gone), gives the place back.
pub(crate) struct Permit<'a> {
    table: &'a InFlight,
    pub(crate) client: Client,
}

impl InFlight {
    /// An empty table for at most `clients` addresses and `places` places.
    pub(crate) fn new(clients: usize, places: usize) -> Self {
        Self {
            held: Mutex::default(),
            clients,
            places,
        }
    }

    /// A place for one more request of `client`, unless it has `share`
    /// already, every place is taken or the table is full. A place held is
    /// never taken away: a full table refuses newcomers rather than evict.
    pub(crate) fn admit(&self, client: Client, share: u8) -> Option<Permit<'_>> {
        // Nothing panics while the table is held; a poisoned one is still whole.
        let mut held = self.held.lock().unwrap_or_else(PoisonError::into_inner);
        if held.total >= self.places {
            return None;
        }
        if let Some(count) = held.by_client.get_mut(&client) {
            if *count >= share {
                return None;
            }
            *count += 1;
        } else {
            if share == 0 || held.by_client.len() >= self.clients {
                return None;
            }
            held.by_client.insert(client, 1);
        }
        held.total += 1;
        Some(Permit {
            table: self,
            client,
        })
    }

    /// The requests `client` has in flight.
    #[cfg(test)]
    pub(crate) fn count(&self, client: Client) -> u8 {
        let held = self.held.lock().unwrap_or_else(PoisonError::into_inner);
        held.by_client.get(&client).copied().unwrap_or(0)
    }

    /// The addresses with a request in flight.
    #[cfg(test)]
    pub(crate) fn addresses(&self) -> usize {
        let held = self.held.lock().unwrap_or_else(PoisonError::into_inner);
        held.by_client.len()
    }

    /// The places held by all addresses together.
    #[cfg(test)]
    pub(crate) fn places(&self) -> usize {
        self.held
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .total
    }
}

impl Drop for Permit<'_> {
    fn drop(&mut self) {
        let mut held = self
            .table
            .held
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if let Some(count) = held.by_client.get_mut(&self.client) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                held.by_client.remove(&self.client);
            }
            held.total = held.total.saturating_sub(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn client(last: u8) -> Client {
        Client::from(IpAddr::from([198, 51, 100, last]))
    }

    #[test]
    fn an_address_has_its_share_and_all_together_the_places() {
        let table = InFlight::new(8, 5);
        let mut held: Vec<_> = (0..2).map(|_| table.admit(client(1), 2).unwrap()).collect();
        // its share is spent; another address still finds a place
        assert!(table.admit(client(1), 2).is_none());
        held.extend((0..2).map(|_| table.admit(client(2), 2).unwrap()));
        held.push(table.admit(client(3), 2).unwrap());
        // every place is taken: nobody gets one, an address in the table neither
        assert_eq!(table.places(), 5);
        assert!(table.admit(client(3), 2).is_none());
        assert!(table.admit(client(4), 2).is_none());
        // a place given back is there for the next
        held.pop();
        assert_eq!((table.places(), table.count(client(3))), (4, 0));
        assert!(table.admit(client(4), 2).is_some());
        drop(held);
        assert_eq!((table.places(), table.addresses()), (0, 0));
        // a share of none is no place, and leaves nothing behind
        assert!(table.admit(client(1), 0).is_none());
        assert_eq!(table.addresses(), 0);
    }

    #[test]
    fn a_full_table_of_addresses_refuses_a_newcomer_only() {
        let table = InFlight::new(2, usize::MAX);
        let _held = [table.admit(client(1), 2), table.admit(client(2), 2)];
        assert!(table.admit(client(3), 2).is_none());
        assert!(table.admit(client(1), 2).is_some());
    }

    #[test]
    fn the_last_forwarded_hop_is_the_address() {
        let headers = |values: &[&'static str]| {
            let mut headers = HeaderMap::new();
            for value in values {
                headers.append("x-forwarded-for", HeaderValue::from_static(value));
            }
            headers
        };
        for (values, expected) in [
            (&["198.51.100.1"][..], Some(client(1))),
            (&["203.0.113.9, 198.51.100.1"], Some(client(1))),
            (
                &["203.0.113.9", "10.0.0.1,\t198.51.100.2 "],
                Some(client(2)),
            ),
            (&[], None),
            (&["198.51.100.1", "unknown"], None),
            (&["198.51.100.1:443"], None),
            (&[""], None),
        ] {
            assert_eq!(forwarded(&headers(values)), expected, "{values:?}");
        }
        // an IPv6 address counts by its /64, a mapped IPv4 one as IPv4
        let a = forwarded(&headers(&["2001:db8:1:2:3:4:5:6"])).unwrap();
        let b = forwarded(&headers(&["2001:db8:1:2::9"])).unwrap();
        let c = forwarded(&headers(&["2001:db8:1:3::9"])).unwrap();
        assert!(a == b && a != c);
        assert_eq!(
            forwarded(&headers(&["::ffff:198.51.100.1"])),
            Some(client(1))
        );
    }
}
