//! Page rules the website enforces, and the worst pages fit the wire limit.
use super::*;
use crate::ranked::{DeckResult, League};

fn id(n: u8) -> Id {
    let mut id = [0xee; 16];
    id[6] = 0x7e;
    id[8] = 0xbe;
    id[15] = n;
    id
}

fn rank(league: League, division: Option<u8>, champion_place: Option<u8>) -> Rank {
    Rank {
        league,
        division,
        champion_place,
    }
}

fn ticket(n: u8) -> Entry {
    Entry::Ticket {
        ticket: Ticket {
            key: id(n),
            season: id(0),
            format: Format::Standard,
            deck: [255; 32],
            queue_elo: -1_000_000,
            replicable: true,
            state: TicketState::Closed,
            match_id: Some(id(n)),
            created_at: i64::MIN,
            expires_at: i64::MIN,
        },
    }
}

fn record() -> Record {
    Record {
        games: u32::MAX,
        wins: u32::MAX,
        losses: u32::MAX,
        draws: u32::MAX,
    }
}

/// The largest deck a page can carry: 250 distinct one-copy cards whose
/// digests print as three-digit bytes, and ten years of monthly seasons.
fn deck(n: u8) -> Entry {
    let cards = (0..250u16)
        .map(|i| {
            let mut card = [255; 32];
            card[30] = 100 + u8::try_from(i / 100).unwrap();
            card[31] = 100 + u8::try_from(i % 100).unwrap();
            PoolEntry { card, copies: 1 }
        })
        .collect();
    let ratings = (0..120)
        .map(|_| DeckRating {
            season: id(1),
            career: record(),
            season_games: u32::MAX,
            rank: rank(League::Champion, None, Some(100)),
            elo: i32::MIN,
            first_at: i64::MIN,
            last_at: i64::MIN,
        })
        .collect();
    let mut deck = [255; 32];
    deck[31] = n;
    Entry::Deck {
        deck: Deck {
            deck,
            format: Format::Highlander,
            cards,
            copies: 250,
            elo: i32::MIN,
            anchor: Some(i32::MIN),
            initial: false,
            first_season: id(1),
            ratings,
        },
    }
}

fn season(n: u8) -> Entry {
    let rating = Rating {
        format: Format::Highlander,
        career: record(),
        rank: rank(League::Champion, None, Some(100)),
        elo: i32::MIN,
        queue_elo: i32::MIN,
        score_thousandths: i64::MIN,
        placement_thousandths: i64::MIN,
        allowance_thousandths: i64::MIN,
        first_at: i64::MIN,
        last_at: i64::MIN,
    };
    let peak = Peak {
        track: Track::Player,
        format: Format::Highlander,
        rank: rank(League::Grandmaster, Some(4), Some(100)),
    };
    Entry::Season {
        season: SeasonEntry {
            season: Season {
                id: id(n),
                name: "N".repeat(96),
                starts: i64::MIN,
                ends: Some(i64::MIN),
            },
            ratings: vec![rating; 3],
            peaks: vec![peak; MAX_SEASON_PEAKS],
            champion: Some(Champion {
                place: 100,
                player_format: Some(Format::Highlander),
                deck_format: Some(Format::Highlander),
                deck: Some([255; 32]),
            }),
        },
    }
}

fn seat(n: u8) -> Entry {
    let operator = Operator {
        host_id: id(n),
        origin: "o".repeat(256),
        epoch: u64::MAX,
        allowed: true,
        terms: Some(TermsVersion {
            version: "v".repeat(64),
            digest: [255; 32],
        }),
    };
    Entry::Match {
        seat: Match {
            match_id: id(n),
            season: id(1),
            format: Format::Highlander,
            seat: 1,
            deck: [255; 32],
            claimed: true,
            host_id: id(n),
            origin: "o".repeat(256),
            state: MatchState::Finished,
            game: Some(id(n)),
            verdict: Some(Verdict::Drawn),
            created_at: i64::MIN,
            finished_at: Some(i64::MIN),
            operators: vec![operator; MAX_MATCH_OPERATORS],
        },
    }
}

fn receipt(n: u8) -> Entry {
    let worst = rank(League::Champion, None, Some(100));
    Entry::Receipt {
        receipt: Receipt {
            match_id: id(n),
            format: Format::Highlander,
            settled_at: i64::MIN,
            verdict: Verdict::Drawn,
            opponent: id(n),
            elo: i32::MIN,
            delta: i32::MIN,
            before: worst,
            after: worst,
            deck: DeckResult {
                before: worst,
                after: worst,
                elo: i32::MIN,
                delta: i32::MIN,
                placement_games: 10,
            },
        },
    }
}

fn unlock(league: League, division: Option<u8>) -> Entry {
    Entry::Cosmetic {
        unlock: Unlock {
            rank: rank(league, division, None),
            earned_at: i64::MIN,
        },
    }
}

#[test]
fn a_cursor_needs_a_full_strictly_ascending_page_of_its_own_dataset() {
    let table = Table::Tickets;
    let page: Vec<_> = (1..=30).map(ticket).collect();
    let last = Some(Key::Id(id(30)));
    assert!(validate_page(table, None, &page, last).is_ok());
    assert!(validate_page(table, None, &page, None).is_ok());
    assert!(validate_page(table, last, &[ticket(31)], None).is_ok());
    assert!(validate_page(table, last, &[], None).is_ok());
    // short, skipping, empty, backwards, repeated, oversized, foreign
    assert!(validate_page(table, None, &page[..29], Some(Key::Id(id(29)))).is_err());
    assert!(validate_page(table, None, &page, Some(Key::Id(id(31)))).is_err());
    assert!(validate_page(table, None, &[], last).is_err());
    assert!(validate_page(table, last, &[ticket(30)], None).is_err());
    assert!(validate_page(table, None, &[ticket(2), ticket(1)], None).is_err());
    assert!(validate_page(table, None, &[ticket(1), ticket(1)], None).is_err());
    let mut oversized = page.clone();
    oversized.push(ticket(31));
    assert!(validate_page(table, None, &oversized, None).is_err());
    assert!(validate_page(Table::Receipts, None, &page, None).is_err());
    // a key of another kind, or not a UUIDv7
    assert!(validate_page(table, Some(Key::Deck([0; 32])), &[], None).is_err());
    let Entry::Ticket { mut ticket } = ticket(1) else {
        unreachable!()
    };
    ticket.key[6] = 0x40;
    assert!(validate_page(table, None, &[Entry::Ticket { ticket }], None).is_err());
}

#[test]
fn rows_are_checked_before_they_are_kept() {
    let Entry::Deck { mut deck } = deck(1) else {
        unreachable!()
    };
    deck.copies = 249;
    assert!(
        validate_page(
            Table::Decks,
            None,
            &[Entry::Deck { deck: deck.clone() }],
            None
        )
        .is_err()
    );
    deck.copies = 250;
    deck.cards.swap(0, 1);
    assert!(validate_page(Table::Decks, None, &[Entry::Deck { deck }], None).is_err());
    let Entry::Match { mut seat } = seat(1) else {
        unreachable!()
    };
    seat.operators.push(seat.operators[0].clone());
    assert!(validate_page(Table::Matches, None, &[Entry::Match { seat }], None).is_err());
    let Entry::Season { mut season } = season(1) else {
        unreachable!()
    };
    season.peaks.push(season.peaks[0]);
    assert!(validate_page(Table::Seasons, None, &[Entry::Season { season }], None).is_err());
    assert!(
        validate_page(
            Table::Cosmetics,
            None,
            &[unlock(League::Unranked, None)],
            None
        )
        .is_err()
    );
}

#[test]
fn an_unlock_s_key_is_its_stored_step_in_the_database_s_order() {
    let steps = [
        (unlock(League::Paper, None), (0, 0)),
        (unlock(League::Wood, Some(1)), (1, 1)),
        (unlock(League::Wood, Some(3)), (1, 3)),
        (unlock(League::Gold, Some(4)), (5, 4)),
        (unlock(League::Grandmaster, None), (9, 0)),
        (unlock(League::Champion, None), (10, 0)),
    ];
    for (entry, (league, division)) in &steps {
        assert_eq!(
            entry.key(),
            (
                Table::Cosmetics,
                Key::Step {
                    league: *league,
                    division: *division
                }
            )
        );
    }
    let page: Vec<_> = steps.into_iter().map(|(entry, _)| entry).collect();
    assert!(validate_page(Table::Cosmetics, None, &page, None).is_ok());
}

#[test]
fn the_largest_page_of_every_dataset_fits_the_wire() {
    for table in Table::ALL {
        let n = u8::try_from(table.page_size()).unwrap();
        let entries: Vec<_> = (1..=n)
            .map(|i| match table {
                Table::Seasons => season(i),
                Table::Decks => deck(i),
                Table::Receipts => receipt(i),
                Table::Matches => seat(i),
                Table::Tickets => ticket(i),
                Table::Cosmetics => unlock(League::Wood, Some(i)),
            })
            .collect();
        let after = entries.last().map(|entry| entry.key().1);
        if table != Table::Cosmetics {
            assert!(
                validate_page(table, None, &entries, after).is_ok(),
                "{table:?}"
            );
        }
        let response = crate::wire::Response::Ranked {
            response: crate::ranked::Response::Export { entries, after },
        };
        let bytes = crate::encode(&response).unwrap_or_else(|_| panic!("{table:?} too large"));
        assert!(
            bytes.len() < crate::MAX_WIRE_BYTES * 3 / 4,
            "{table:?}: {}",
            bytes.len()
        );
        let back: crate::wire::Response = crate::decode(&bytes).unwrap();
        assert!(matches!(back, crate::wire::Response::Ranked { .. }));
    }
}

#[test]
fn every_dataset_names_itself_and_takes_its_own_key_kind() {
    let names: Vec<_> = Table::ALL.iter().map(|table| table.as_str()).collect();
    assert_eq!(
        names,
        [
            "seasons",
            "decks",
            "receipts",
            "matches",
            "tickets",
            "cosmetics"
        ]
    );
    for table in Table::ALL {
        let encoded = String::from_utf8(crate::encode(&table).unwrap()).unwrap();
        assert_eq!(encoded, format!("\"{}\"", table.as_str()));
        assert!(DOWNLOAD_ROWS.is_multiple_of(table.page_size()));
    }
    assert!(valid_key(Table::Decks, &Key::Deck([0; 32])));
    assert!(!valid_key(Table::Decks, &Key::Id(id(1))));
    assert!(!valid_key(
        Table::Cosmetics,
        &Key::Step {
            league: 11,
            division: 0
        }
    ));
    assert!(!valid_key(
        Table::Cosmetics,
        &Key::Step {
            league: 1,
            division: 5
        }
    ));
    assert!(valid_key(Table::Seasons, &Key::Id(id(1))));
    assert!(!valid_key(Table::Seasons, &Key::Id([0; 16])));
}
