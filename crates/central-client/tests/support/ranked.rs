//! Native HTTP and SDK coverage against the fixture's migrated runtime database.

use super::Http;
use central_api::{
    Token,
    ranked::{
        Format, HostRequest, Id, League, ProfileTarget, QueueState, Request, Response, Track,
    },
    wire,
};
use central_client::{Session, Transport, ranked};

pub(super) fn public(http: &Http, account: Id, name: &str, tag: u32) {
    let Response::Seasons { items, .. } =
        ranked::read(http, Request::Seasons { before: None }).unwrap()
    else {
        panic!("season page expected");
    };
    let active = items.first().expect("fresh fixture starts with one season");
    for format in [Format::Standard, Format::Highlander, Format::Commander] {
        for track in [Track::Player, Track::Deck] {
            let query = Request::Board {
                season: Some(active.id),
                format,
                track,
                league: League::Gold,
                division: Some(2),
                after: None,
            };
            let Response::Board {
                season,
                entries,
                next,
            } = ranked::read(http, query).unwrap()
            else {
                panic!("board page expected");
            };
            assert_eq!(season.id, active.id);
            assert!(entries.is_empty() && next.is_none());
        }
    }
    for target in [
        ProfileTarget::Account(account),
        ProfileTarget::Handle(central_api::profile::handle(name, tag)),
    ] {
        let Response::Profile {
            season,
            account: found,
            name: display,
            tag: discriminator,
            ..
        } = ranked::read(
            http,
            Request::Profile {
                target,
                season: None,
            },
        )
        .unwrap()
        else {
            panic!("public profile expected");
        };
        assert_eq!(season.id, active.id);
        assert_eq!(found, account);
        assert_eq!(display.as_deref(), Some(name));
        assert_eq!(discriminator, Some(tag));
    }
}

pub(super) fn private(http: &Http, session: &Session, unverified: &Token) {
    let request = wire::Request::Ranked {
        request: Request::Status,
    };
    assert!(http.exchange(&request, None).is_err());
    assert!(http.exchange(&request, Some(unverified.into())).is_err());
    assert!(ranked::host(http, unverified, HostRequest::Ready { capacity: 1 }).is_err());
    assert!(matches!(
        session.ranked(http, Request::Status).unwrap(),
        Response::Queue {
            state: QueueState::Idle
        }
    ));
}
