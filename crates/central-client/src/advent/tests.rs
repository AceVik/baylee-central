use super::*;
use central_api::{Bearer, Endpoint, SignedToken, wire::Identity};
use central_crypto::vault::MasterKey;
use std::{cell::RefCell, collections::VecDeque};

struct Replies {
    endpoint: Endpoint,
    answers: RefCell<VecDeque<api::Response>>,
}
impl Replies {
    fn new(answers: impl IntoIterator<Item = api::Response>) -> Self {
        Self {
            endpoint: Endpoint::live(),
            answers: RefCell::new(answers.into_iter().collect()),
        }
    }
}
impl Transport for Replies {
    fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }
    fn exchange(&self, req: &Request, bearer: Option<Bearer<'_>>) -> Result<Response, Error> {
        assert!(bearer.is_some());
        assert!(matches!(
            req,
            Request::Advent {
                request: api::Request::Asset { .. }
            }
        ));
        Ok(Response::Advent {
            response: self
                .answers
                .borrow_mut()
                .pop_front()
                .ok_or(Error::Unavailable)?,
        })
    }
}
fn session() -> Session {
    Session::new(
        Endpoint::live(),
        Identity {
            account: [8; 16],
            epoch: 1,
        },
        SignedToken::new(zeroize::Zeroizing::new("synthetic.advent.session".into())).unwrap(),
        MasterKey::generate().unwrap(),
    )
}

#[test]
fn verify_content_and_conditional_cache_scope() {
    // Published SHA-256 known-answer test vector for three synthetic octets.
    let meta = api::AssetMeta {
        tier: api::Finish::Standard,
        mime: api::Mime::Webp,
        size: 3,
        width: 1,
        height: 1,
        digest: [
            0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae,
            0x22, 0x23, 0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61,
            0xf2, 0x00, 0x15, 0xad,
        ],
    };
    let scope = api::AssetScope::Owned { item: [1; 16] };
    let ask = |bytes: &[u8]| {
        Replies::new([api::Response::Asset {
            page: api::AssetPage {
                asset: meta,
                offset: 0,
                bytes: bytes.to_vec(),
            },
        }])
    };
    assert_eq!(
        session().advent_asset(&ask(b"abc"), scope, None).unwrap(),
        (meta, Some(b"abc".to_vec()))
    );
    assert_eq!(
        session().advent_asset(&ask(b"abd"), scope, None),
        Err(Error::Rejected)
    );
    assert_eq!(
        session().advent_asset(&ask(b"a"), scope, None),
        Err(Error::Rejected)
    );
    let cache = || Replies::new([api::Response::Cached { asset: meta }]);
    assert_eq!(
        session()
            .advent_asset(&cache(), scope, Some(meta.digest))
            .unwrap(),
        (meta, None)
    );
    assert_eq!(
        session().advent_asset(&cache(), scope, Some([0; 32])),
        Err(Error::Rejected)
    );
}
