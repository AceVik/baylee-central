use central_api::Error;
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

#[derive(Default)]
pub(super) struct Limits {
    global: Option<(Instant, u32)>,
    accounts: BTreeMap<[u8; 16], (Instant, u32)>,
}
impl Limits {
    pub(super) fn global(&mut self, now: Instant) -> Result<(), Error> {
        let (start, used) = self.global.get_or_insert((now, 0));
        if now.duration_since(*start) >= Duration::from_secs(1) {
            *start = now;
            *used = 0;
        }
        if *used >= 32 {
            return Err(Error::Busy);
        }
        *used += 1;
        Ok(())
    }
    pub(super) fn account(&mut self, account: [u8; 16], now: Instant) -> Result<(), Error> {
        self.accounts
            .retain(|_, (start, _)| now.duration_since(*start) < Duration::from_secs(60));
        if !self.accounts.contains_key(&account) && self.accounts.len() >= 1024 {
            return Err(Error::Busy);
        }
        let (_, used) = self.accounts.entry(account).or_insert((now, 0));
        if *used >= 8 {
            return Err(Error::Busy);
        }
        *used += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rate_limits_expire_and_have_bounded_cardinality() {
        let now = Instant::now();
        let mut limits = Limits::default();
        for _ in 0..32 {
            limits.global(now).unwrap();
        }
        assert_eq!(limits.global(now), Err(Error::Busy));
        limits.global(now + Duration::from_secs(1)).unwrap();
        for _ in 0..8 {
            limits.account([0; 16], now).unwrap();
        }
        assert_eq!(limits.account([0; 16], now), Err(Error::Busy));
        for i in 1_u128..1024 {
            limits.account(i.to_be_bytes(), now).unwrap();
        }
        assert_eq!(limits.account([255; 16], now), Err(Error::Busy));
        limits
            .account([255; 16], now + Duration::from_secs(60))
            .unwrap();
        assert_eq!(limits.accounts.len(), 1);
    }
}
