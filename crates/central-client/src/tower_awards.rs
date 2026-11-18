use crate::{Error, Request, Response, Session, Transport};
use central_api::{
    Token,
    tower_awards::{Delivery, Export, Progress, Receipt},
};

/// Deliver an immutable verified fact to the configured Central transport.
/// The credential is a separate approved issuer secret, never a player session.
/// # Errors
/// Wrong scope, malformed/mismatched reply, revoked issuer or transport failure.
pub fn deliver_tower_award(
    transport: &impl Transport,
    credential: &Token,
    delivery: Delivery,
) -> Result<Receipt, Error> {
    delivery.validate()?;
    let Response::TowerAward { receipt } =
        transport.exchange(&Request::TowerAward { delivery }, Some(credential.into()))?
    else {
        return Err(Error::Rejected);
    };
    if receipt.event != delivery.event
        || receipt.account != delivery.account
        || receipt.run != delivery.fact.run
        || receipt.stage != delivery.fact.stage
    {
        return Err(Error::Rejected);
    }
    Ok(receipt)
}

impl Session {
    /// Read permanent Tower totals from this session's fixed Central endpoint.
    /// # Errors
    /// Wrong endpoint, expired session, invalid projection or transport failure.
    pub fn tower_award_progress(&self, transport: &impl Transport) -> Result<Progress, Error> {
        self.same_endpoint(transport)?;
        let Response::TowerProgress { progress } =
            transport.exchange(&Request::TowerProgress {}, Some((&self.token).into()))?
        else {
            return Err(Error::Rejected);
        };
        progress.validate()?;
        Ok(progress)
    }

    /// Read one bounded own-account receipt page without accepting arbitrary cursors.
    /// # Errors
    /// Wrong endpoint/session, invalid page or transport failure.
    pub fn tower_award_export(
        &self,
        transport: &impl Transport,
        after: Option<[u8; 16]>,
    ) -> Result<Export, Error> {
        self.same_endpoint(transport)?;
        if after.is_some_and(|id| id[6] >> 4 != 7 || id[8] >> 6 != 2) {
            return Err(Error::Rejected);
        }
        let Response::TowerAwardExport { export } = transport.exchange(
            &Request::TowerAwardExport { after },
            Some((&self.token).into()),
        )?
        else {
            return Err(Error::Rejected);
        };
        export.validate(after)?;
        Ok(export)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use central_api::{
        Bearer, Endpoint,
        tower_awards::{Disposition, Fact, Outcome},
    };

    struct Reply {
        endpoint: Endpoint,
        receipt: Receipt,
        wrong_type: bool,
    }
    impl Transport for Reply {
        fn endpoint(&self) -> &Endpoint {
            &self.endpoint
        }
        fn exchange(&self, _: &Request, _: Option<Bearer<'_>>) -> Result<Response, Error> {
            Ok(if self.wrong_type {
                Response::TowerProgress {
                    progress: Progress::default(),
                }
            } else {
                Response::TowerAward {
                    receipt: self.receipt,
                }
            })
        }
    }
    fn id(tail: u8) -> [u8; 16] {
        let mut id = [0; 16];
        id[6] = 0x70;
        id[8] = 0x80;
        id[15] = tail;
        id
    }
    #[test]
    fn a_reply_for_another_event_account_run_or_ordinal_cannot_acknowledge_delivery() {
        let value = Delivery {
            event: id(1),
            account: id(2),
            fact: Fact {
                schema: 2,
                run: id(3),
                cohort: id(4),
                stage: 1,
                outcome: Outcome::Victory,
                wins: 1,
                defeats: 0,
            },
        };
        let token = Token::generate().unwrap();
        let good = Receipt::new(&value, Disposition::Recorded);
        let mut reply = Reply {
            endpoint: Endpoint::development("http://127.0.0.1:28780").unwrap(),
            receipt: good,
            wrong_type: false,
        };
        assert!(deliver_tower_award(&reply, &token, value).unwrap() == good);
        for bad in [
            Receipt {
                event: id(9),
                ..good
            },
            Receipt {
                account: id(9),
                ..good
            },
            Receipt { run: id(9), ..good },
            Receipt { stage: 2, ..good },
        ] {
            reply.receipt = bad;
            assert!(matches!(
                deliver_tower_award(&reply, &token, value),
                Err(Error::Rejected)
            ));
        }
        reply.receipt = good;
        reply.wrong_type = true;
        assert!(matches!(
            deliver_tower_award(&reply, &token, value),
            Err(Error::Rejected)
        ));
    }
}
