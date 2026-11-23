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

pub(crate) mod replica;

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
    let own = Request::OwnProfile { season: None };
    assert!(
        central_api::decode::<Request>(br#"{"own_profile":{"season":null,"target":"other"}}"#)
            .is_err()
    );
    assert!(ranked::read(http, own.clone()).is_err());
    assert!(
        http.exchange(
            &wire::Request::Ranked {
                request: own.clone()
            },
            None
        )
        .is_err()
    );
    let Response::Profile {
        account, formats, ..
    } = session.ranked(http, own).unwrap()
    else {
        panic!("owner's authenticated standings expected");
    };
    assert_eq!(account, session.identity().account);
    assert_eq!(formats.len(), 3);
    assert!(formats.iter().all(|s| s.track == Track::Player
        && s.placement_games == 0
        && s.rank.league == League::Unranked));
    let selected = vec![central_api::ranked::PoolEntry {
        card: [7; 32],
        copies: 60,
    }];
    let preview_request = Request::PreviewDeck {
        format: Format::Standard,
        pool: selected.clone(),
    };
    assert!(ranked::read(http, preview_request.clone()).is_err());
    assert!(
        http.exchange(
            &wire::Request::Ranked {
                request: preview_request
            },
            None
        )
        .is_err()
    );
    let preview = session
        .preview_ranked_deck(http, Format::Standard, selected)
        .unwrap();
    assert_eq!(preview.source, central_api::ranked::PreviewSource::New);
    assert_eq!(
        (preview.elo, preview.placement_games, preview.rank.league),
        (1500, 0, League::Unranked)
    );
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
