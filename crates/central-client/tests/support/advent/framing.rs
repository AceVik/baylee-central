//! Old and current clients request the same private image through the native route.
use super::{A, Http, Q, Session, Transport, YEAR, api, material};
use central_api::{Bearer, Endpoint, Error, wire};

struct Legacy<'a>(&'a Http);

impl Transport for Legacy<'_> {
    fn endpoint(&self) -> &Endpoint {
        self.0.endpoint()
    }

    fn exchange(
        &self,
        request: &wire::Request,
        bearer: Option<Bearer<'_>>,
    ) -> Result<wire::Response, Error> {
        let raw = self
            .0
            .posted(&central_api::encode(request)?, bearer, None, false)?;
        assert!(
            !raw.windows(b"mat_framing".len())
                .any(|s| s == b"mat_framing")
        );
        central_api::decode(&raw)
    }
}

pub(super) fn run(http: &Http, wearer: &Session, outsider: &Session) {
    let A::Calendar {
        hero: Some(hero), ..
    } = wearer.advent(http, Q::Calendar { year: YEAR }).unwrap()
    else {
        panic!("visible calendar")
    };
    assert!(hero.mat_framing.is_none());
    let day = 2;
    assert!(matches!(
        wearer
            .advent(
                http,
                Q::Submit {
                    year: YEAR,
                    door: day,
                    line: material::line(2),
                }
            )
            .unwrap(),
        A::Submission { won: true }
    ));
    let A::Reward { reward } = wearer
        .advent(
            http,
            Q::Claim {
                year: YEAR,
                door: day,
            },
        )
        .unwrap()
    else {
        panic!("earned playmat")
    };
    let expected = Some(material::mat_framing(day));
    assert_eq!(reward.asset.mat_framing, expected);
    let scope = api::AssetScope::Owned { item: reward.id };
    let read = wearer.advent_asset(http, scope, None).unwrap();
    assert_eq!(read, (reward.asset, Some(material::bytes(day))));
    let digest = Some(reward.asset.digest);
    let modern = wearer.advent_asset(http, scope, digest).unwrap();
    assert_eq!(modern, (reward.asset, None));
    assert!(outsider.advent_asset(http, scope, digest).is_err());
    let legacy = Legacy(http);
    let (old, bytes) = wearer.advent_asset(&legacy, scope, digest).unwrap();
    assert!(old.mat_framing.is_none());
    assert_eq!(old.digest, reward.asset.digest);
    assert!(bytes.is_none());
    assert!(outsider.advent_asset(&legacy, scope, digest).is_err());
}
