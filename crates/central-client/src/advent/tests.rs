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
        mat_framing: None,
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

#[test]
fn independent_page_jobs_cannot_switch_art_or_skip_a_range() {
    let scope = api::AssetScope::Owned { item: [7; 16] };
    let meta = api::AssetMeta {
        mat_framing: None,
        size: api::PAGE_BYTES + 1,
        width: 1,
        height: 1,
        mime: api::Mime::Webp,
        tier: api::Finish::Standard,
        digest: [3; 32],
    };
    for changed_digest in [false, true] {
        let mut calls = 0;
        let result = read_asset(scope, None, |query| {
            let api::Request::Asset { offset, .. } = query else {
                panic!("expected an asset range")
            };
            calls += 1;
            let page = if calls == 1 {
                assert_eq!(offset, 0);
                api::AssetPage {
                    asset: meta,
                    offset,
                    bytes: vec![8; api::PAGE_BYTES as usize],
                }
            } else {
                assert_eq!(offset, api::PAGE_BYTES);
                let mut asset = meta;
                if changed_digest {
                    asset.digest = [4; 32];
                }
                api::AssetPage {
                    asset,
                    offset: if changed_digest { offset } else { offset + 1 },
                    bytes: vec![9],
                }
            };
            Ok(api::Response::Asset { page })
        });
        assert_eq!(result, Err(Error::Rejected));
        assert_eq!(calls, 2);
    }
}

fn image(bytes: &[u8]) -> api::AssetMeta {
    api::AssetMeta {
        mat_framing: None,
        size: u32::try_from(bytes.len()).unwrap(),
        width: 1,
        height: 1,
        mime: api::Mime::Webp,
        tier: api::Finish::Standard,
        digest: [3; 32],
    }
}

const SCOPE: api::AssetScope = api::AssetScope::Owned { item: [7; 16] };

/// The art of `bytes` as an honest Central sends it, changed by `alter`.
fn fetched(
    (bytes, digest): (&[u8], [u8; 32]),
    alter: impl Fn(usize, &mut api::AssetPage),
) -> Result<(api::AssetMeta, Option<Vec<u8>>), Error> {
    let mut calls = 0;
    read_asset(SCOPE, None, |query| {
        let api::Request::Asset { offset, .. } = query else {
            panic!("expected an asset range")
        };
        let from = offset as usize;
        let until = bytes.len().min(from + api::PAGE_BYTES as usize);
        let mut page = api::AssetPage {
            asset: api::AssetMeta {
                digest,
                ..image(bytes)
            },
            offset,
            bytes: bytes[from..until].to_vec(),
        };
        alter(calls, &mut page);
        calls += 1;
        assert!(calls < 8, "the transfer does not end");
        Ok(api::Response::Asset { page })
    })
}

#[test]
fn an_image_arrives_whole_or_not_at_all() {
    const PAGE: usize = api::PAGE_BYTES as usize;
    // SHA-256 of that many octets counting up to 252 again and again, computed elsewhere.
    for (length, digest) in [
        (
            1,
            "6e340b9cffb37a989ca544e6bb780a2c78901d3fb33738768511a30617afa01d",
        ),
        (
            PAGE - 1,
            "2127e96020ed53c171ee0348959ac4dd6d88b8ceaf3760c70d9d0481f165ef32",
        ),
        (
            PAGE,
            "6b1ec78bafdb0aa54a02c396a1926ab6d1381b32f403ad9e5cc19a27bfe9f3f9",
        ),
        (
            PAGE + 1,
            "2dd9a4cb746edd4f3ac05644857e8d0af8a72283fc33078220958ab827375062",
        ),
        (
            2 * PAGE,
            "3edfac48ad05b0f3ad7286e8241c5680688d5e33fc9fb7276eee9eb29f5f2cc9",
        ),
    ] {
        let bytes: Vec<u8> = (0..length)
            .map(|i| u8::try_from(i % 253).unwrap())
            .collect();
        let digest =
            std::array::from_fn(|i| u8::from_str_radix(&digest[2 * i..2 * i + 2], 16).unwrap());
        let fetched = |alter: &dyn Fn(usize, &mut api::AssetPage)| fetched((&bytes, digest), alter);
        let (meta, found) = fetched(&|_, _| {}).unwrap();
        assert_eq!((meta.digest, meta.size as usize), (digest, length));
        assert!(found.unwrap() == bytes, "{length}");
        // One changed octet in the last page is found by the digest.
        let last = (length - 1) / PAGE;
        assert!(
            fetched(&|call, page| {
                if call == last {
                    *page.bytes.last_mut().unwrap() ^= 1;
                }
            })
            .is_err(),
            "{length}"
        );
        // A page that is short, long or empty is none.
        assert!(
            fetched(&|_, page| {
                page.bytes.pop();
            })
            .is_err()
        );
        assert!(fetched(&|_, page| page.bytes.push(0)).is_err());
        assert!(fetched(&|_, page| page.bytes.clear()).is_err());
    }
}

#[test]
fn a_cached_answer_is_taken_only_for_the_digest_asked_and_only_first() {
    let meta = image(b"x");
    let ask = |cached, offset| api::Request::Asset {
        scope: SCOPE,
        offset,
        cached,
    };
    let cached = api::Response::Cached { asset: meta };
    assert!(answers(&ask(Some(meta.digest), 0), &cached));
    assert!(!answers(&ask(None, 0), &cached));
    assert!(!answers(&ask(Some([0; 32]), 0), &cached));
    assert!(!answers(&ask(Some(meta.digest), api::PAGE_BYTES), &cached));
    let oversized = api::Response::Cached {
        asset: api::AssetMeta { width: 0, ..meta },
    };
    assert!(!answers(&ask(Some(meta.digest), 0), &oversized));
    // Unasked, it is no answer to a transfer either.
    assert_eq!(
        read_asset(SCOPE, None, |_| Ok(cached.clone())),
        Err(Error::Rejected)
    );
    assert_eq!(
        read_asset(SCOPE, Some(meta.digest), |_| Ok(cached.clone())),
        Ok((meta, None))
    );
}

#[test]
fn a_calendar_has_its_year_and_its_doors_in_order() {
    let calendar = |year, starts_at, closes_at, doors: Vec<u8>, hero| api::Response::Calendar {
        year,
        hero,
        starts_at,
        closes_at,
        now: 5,
        doors: doors
            .into_iter()
            .map(|door| api::Door {
                door,
                state: api::DoorState::Locked,
                enables_at: 0,
            })
            .collect(),
    };
    let all = || (1..=api::DOORS).collect::<Vec<_>>();
    let ask = api::Request::Calendar { year: 2026 };
    let current = api::Request::CurrentCalendar {};
    let good = calendar(2026, 1, 2, all(), Some(image(b"x")));
    assert!(answers(&ask, &good) && answers(&current, &good));
    let other = calendar(2027, 1, 2, all(), None);
    assert!(!answers(&ask, &other));
    assert!(answers(&current, &other));
    let mut swapped = all();
    swapped.swap(3, 4);
    let mut twice = all();
    twice[23] = 23;
    let mut more = all();
    more.push(25);
    for doors in [
        vec![],
        (1..api::DOORS).collect(),
        (0..api::DOORS).collect(),
        swapped,
        twice,
        more,
    ] {
        let reply = calendar(2026, 1, 2, doors, None);
        assert!(
            !answers(&ask, &reply) && !answers(&current, &reply),
            "{reply:?}"
        );
    }
    assert!(!answers(&ask, &calendar(2026, 2, 2, all(), None)));
    let broken = api::AssetMeta {
        size: 0,
        ..image(b"x")
    };
    assert!(!answers(&ask, &calendar(2026, 1, 2, all(), Some(broken))));
    // "No calendar" answers only the question which one is open.
    assert!(answers(&current, &api::Response::NoCalendar {}));
    assert!(!answers(&ask, &api::Response::NoCalendar {}));
}

#[test]
fn a_reward_belongs_to_the_door_that_was_claimed() {
    let reward = |year, door, asset| api::Response::Reward {
        reward: api::Reward {
            id: [1; 16],
            kind: api::Kind::Sleeve,
            title: api::Text {
                de: "a".into(),
                en: "a".into(),
            },
            asset,
            year,
            door,
            earned_at: 1,
        },
    };
    let ask = api::Request::Claim {
        year: 2026,
        door: 3,
    };
    let good = image(b"x");
    assert!(answers(&ask, &reward(2026, 3, good)));
    for (year, door, asset) in [
        (2025, 3, good),
        (2026, 4, good),
        (2026, 3, api::AssetMeta { height: 0, ..good }),
    ] {
        assert!(!answers(&ask, &reward(year, door, asset)));
    }
    let hint = |number, length: usize| api::Response::Hint {
        number,
        text: api::Text {
            de: "x".repeat(length),
            en: "x".into(),
        },
    };
    let ask = api::Request::Hint {
        year: 2026,
        door: 3,
        number: 1,
    };
    assert!(answers(&ask, &hint(1, 512)));
    assert!(!answers(&ask, &hint(1, 513)));
    assert!(!answers(&ask, &hint(2, 1)));
}

#[test]
fn a_collection_page_moves_forward_and_ends_where_it_says() {
    let item = |n: u8| api::collection::Item {
        id: [n; 16],
        kind: api::Kind::Playmat,
        title: api::Text {
            de: "a".into(),
            en: "a".into(),
        },
        asset: image(b"x"),
        earned_at: 1,
        source: api::collection::Source::Unknown,
    };
    let page = |ids: &[u8], next: Option<u8>| api::Response::Collection {
        items: ids.iter().copied().map(item).collect(),
        next: next.map(|n| [n; 16]),
    };
    let ask = |after: Option<u8>| api::Request::Collection {
        after: after.map(|n| [n; 16]),
    };
    let full: Vec<u8> = (1..=32).collect();
    assert!(answers(&ask(None), &page(&full, Some(32))));
    assert!(answers(&ask(None), &page(&full, None)));
    assert!(answers(&ask(Some(32)), &page(&[], None)));
    assert!(answers(&ask(Some(4)), &page(&[5], None)));
    for (after, ids, next) in [
        // The page before, or the item itself, again.
        (Some(4), vec![4], None),
        (Some(4), vec![3, 5], None),
        (None, vec![2, 1], None),
        (None, vec![1, 1], None),
        // A cursor behind a page that is not full, or elsewhere than at its end.
        (None, vec![1], Some(1)),
        (None, vec![], Some(1)),
        (None, full.clone(), Some(31)),
        (None, full.clone(), Some(33)),
        (None, (1..=33).collect(), None),
    ] {
        assert!(
            !answers(&ask(after), &page(&ids, next)),
            "{after:?} {ids:?}"
        );
    }
}
