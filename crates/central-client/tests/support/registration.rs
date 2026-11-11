//! Registration as one who reads the source and speaks the listener's
//! protocol would attack it (`docs/registration-abuse.md` §"Hardened"),
//! over the real browser listener, the identity service and PostgreSQL:
//! a proof used twice, raced, solved for less than was asked or made up;
//! a way to an account around the proof; and what a request that looks
//! automated, an address that asks often and a block of addresses pay.
//!
//! The fixture's service asks one bit at its base and takes a solution at
//! once (`with_registration_challenge`), so the checks cost no time here;
//! the three seconds and the sixteen bits have the identity service's own
//! tests.
use super::{Http, browser::Browser};
use central_api::{
    Error, Token,
    register::{PARTS, SIGNAL_NO_LANGUAGES, SIGNAL_WEBDRIVER, solve, solved},
    wire::{Request, Response},
};
use central_client::Transport;
use sea_orm::{ConnectionTrait, DatabaseConnection};
use std::net::SocketAddr;
use tokio::runtime::Runtime;

/// The headers a person's browser adds to the page's own.
const PERSON: &str = "User-Agent: Mozilla/5.0 (X11; Linux x86_64; rv:130.0) Gecko/20100101 Firefox/130.0\r\n\
     Accept-Language: de-DE,de;q=0.9,en;q=0.8\r\n";
/// A tool that says what it is, with a browser's other headers.
const TOOL: &str = "User-Agent: curl/8.7.1\r\nAccept-Language: en\r\n";
/// Nothing but what the listener insists on: a script that read the source.
const SCRIPT: &str = "";
/// The fixture's base difficulty and the most over it.
const BASE: u8 = 1;
const MOST: u8 = 6;

fn at(address: SocketAddr, client: &'static str) -> Browser {
    Browser { address, client }
}

/// A challenge asked with `headers`: its nonce and difficulty.
fn challenge(browser: &Browser, request: &Request, headers: &str) -> (String, u8) {
    match browser.answer_as(request, headers, true) {
        Ok(Response::RegisterChallenge {
            nonce,
            parts,
            difficulty,
            ..
        }) => {
            assert_eq!(parts, PARTS);
            (nonce, difficulty)
        }
        other => panic!("a challenge, not {:?}", other.map(|_| "another answer")),
    }
}

fn register(name: &str, nonce: &str, counters: &[u64]) -> Request {
    Request::Register {
        username: name.into(),
        display_name: None,
        nonce: nonce.into(),
        counters: counters.to_vec(),
    }
}

fn reported(signals: u8) -> Request {
    Request::RegisterChallengeFor { signals }
}

pub(crate) fn run(
    address: SocketAddr,
    http: &Http,
    runtime: &Runtime,
    operator: &DatabaseConnection,
) {
    // The flood's rise so far in this run (registrations in the last hour),
    // read off a person's challenge; every expectation below adds to it.
    let (_, plain) = challenge(&at(address, "192.0.2.10"), &reported(0), PERSON);
    let rise = plain - BASE;
    let with = |points: u8| BASE + (rise + points).min(MOST);
    looks(address, &with);
    spent(address);
    raced(address, runtime, operator);
    around(address, http);
    haste(address, &with);
    block(address, &with);
}

/// What a request looks like sets its price, the same for both ways to
/// ask, and nothing is refused for its look.
fn looks(address: SocketAddr, with: &dyn Fn(u8) -> u8) {
    let person = at(address, "192.0.2.11");
    let asked = |request: &Request, headers: &str| challenge(&person, request, headers).1;
    assert_eq!(asked(&reported(0), PERSON), with(0));
    assert_eq!(asked(&Request::RegisterChallenge, PERSON), with(0));
    // What the page reports of its browser: driven by a program, no
    // languages; bits Central does not know say nothing.
    assert_eq!(asked(&reported(SIGNAL_WEBDRIVER), PERSON), with(2));
    assert_eq!(asked(&reported(SIGNAL_NO_LANGUAGES), PERSON), with(1));
    assert_eq!(asked(&reported(0b1111_1100), PERSON), with(0));
    let script = at(address, "192.0.2.12");
    let asked = |request: &Request, headers: &str| challenge(&script, request, headers).1;
    // A tool that names itself.
    assert_eq!(asked(&reported(0), TOOL), with(2));
    // No user agent and no language: the older way to ask is weighed like
    // the newer one, so asking the old way saves nothing.
    assert_eq!(asked(&Request::RegisterChallenge, SCRIPT), with(3));
    assert_eq!(asked(&reported(0), SCRIPT), with(3));
    // Every sign together stops at four bits.
    assert_eq!(asked(&reported(u8::MAX), SCRIPT), with(4));
}

/// A challenge is gone with the first answer to it, whatever the answer
/// was, and only a spent one tells whether a name is taken.
fn spent(address: SocketAddr) {
    let browser = at(address, "192.0.2.13");
    let ask = || challenge(&browser, &Request::RegisterChallenge, SCRIPT);
    let send = |request: &Request| browser.answer_as(request, SCRIPT, true).err();
    // Solved for less than was asked: refused, and the right answer after
    // it finds the challenge gone.
    let (nonce, difficulty) = ask();
    assert!(difficulty >= BASE + 3);
    let easy = solve(&nonce, PARTS, BASE).unwrap();
    assert!(!solved(&nonce, difficulty, &easy), "2^-24 at the least");
    assert_eq!(
        send(&register("spent-less", &nonce, &easy)),
        Some(Error::Rejected)
    );
    let right = solve(&nonce, PARTS, difficulty).unwrap();
    assert_eq!(
        send(&register("spent-less", &nonce, &right)),
        Some(Error::Rejected),
        "spent by the wrong answer"
    );
    // A good proof with a taken name is a conflict, and is spent by it.
    let (nonce, difficulty) = ask();
    let right = solve(&nonce, PARTS, difficulty).unwrap();
    assert_eq!(
        send(&register("web-player", &nonce, &right)),
        Some(Error::Conflict)
    );
    assert_eq!(
        send(&register("spent-free", &nonce, &right)),
        Some(Error::Rejected),
        "spent by the conflict"
    );
    // What is no nonce at all is refused, and with it this address's five
    // registrations of the hour are spent: the sixth is not looked at.
    let malformed = register("spent-free", "not a nonce", &[]);
    assert_eq!(send(&malformed), Some(Error::Rejected));
    assert_eq!(send(&malformed), Some(Error::Busy));
    // A nonce Central never issued, in the issued shape, with counters
    // that solve it: refused the same for a taken and a free name, so a
    // refusal says nothing about the name.
    let elsewhere = at(address, "192.0.2.16");
    let forged = "5a".repeat(32);
    let counters = solve(&forged, PARTS, difficulty).unwrap();
    for name in ["web-player", "spent-forged"] {
        assert_eq!(
            elsewhere
                .answer_as(&register(name, &forged, &counters), SCRIPT, true)
                .err(),
            Some(Error::Rejected),
            "{name}"
        );
    }
}

/// Three registrations with one solved challenge at the same moment: one
/// account, whichever came first.
fn raced(address: SocketAddr, runtime: &Runtime, operator: &DatabaseConnection) {
    let browser = at(address, "192.0.2.14");
    let (nonce, difficulty) = challenge(&browser, &Request::RegisterChallenge, SCRIPT);
    let counters = solve(&nonce, PARTS, difficulty).unwrap();
    let start = std::sync::Barrier::new(3);
    let outcomes: Vec<Result<Response, Error>> = std::thread::scope(|scope| {
        let racers: Vec<_> = ["raced-a", "raced-b", "raced-c"]
            .into_iter()
            .map(|name| {
                let request = register(name, &nonce, &counters);
                let (browser, start) = (&browser, &start);
                scope.spawn(move || {
                    start.wait();
                    browser.answer_as(&request, SCRIPT, false)
                })
            })
            .collect();
        racers.into_iter().map(|r| r.join().unwrap()).collect()
    });
    let won = outcomes
        .iter()
        .filter(|outcome| matches!(outcome, Ok(Response::Enrollment { .. })))
        .count();
    let refused = outcomes
        .iter()
        .filter(|outcome| matches!(outcome, Err(Error::Rejected)))
        .count();
    assert_eq!((won, refused), (1, 2));
    let reserved: i64 = runtime
        .block_on(operator.query_one_raw(sea_orm::Statement::from_string(
            sea_orm::DbBackend::Postgres,
            "SELECT count(*) AS reserved FROM central_identity.accounts WHERE username_key LIKE 'raced-%'",
        )))
        .unwrap()
        .unwrap()
        .try_get("", "reserved")
        .unwrap();
    assert_eq!(reserved, 1);
}

/// No way to an account around a spent challenge: the native listener has
/// neither operation, an enrollment needs a reservation's ticket, and the
/// bridge's reservation needs the bridge.
fn around(address: SocketAddr, http: &Http) {
    let nonce = "00".repeat(32);
    for request in [
        Request::RegisterChallenge,
        reported(0),
        register("around-native", &nonce, &[0; 8]),
        Request::LegacyReserve {
            legacy_id: "around".into(),
            username: "around-bridge".into(),
            display_name: None,
            tag: None,
        },
    ] {
        assert_eq!(http.exchange(&request, None).err(), Some(Error::Rejected));
        // a made-up token is no session and no bridge
        let made_up = Token::generate().unwrap();
        assert_eq!(
            http.exchange(&request, Some(&made_up)).err(),
            Some(Error::Rejected)
        );
    }
    let browser = at(address, "192.0.2.15");
    let ticket = || Token::generate().unwrap();
    for request in [
        Request::EnrollmentStart {
            ticket: ticket(),
            message: vec![0; 32],
        },
        Request::EnrollmentFinish {
            ticket: ticket(),
            record: vec![0; 192],
            wrapped_master: vec![0; 73],
            document: vec![0; 64],
            recovery: Some(vec![0; 73]),
        },
    ] {
        assert_eq!(
            browser.answer_as(&request, PERSON, true).err(),
            Some(Error::Rejected)
        );
    }
}

/// An address that asks often pays more with every five challenges, and
/// is refused only where its budget of twenty an hour ends.
fn haste(address: SocketAddr, with: &dyn Fn(u8) -> u8) {
    let browser = at(address, "203.0.113.50");
    for index in 0..20u8 {
        let request = if index % 2 == 0 {
            reported(0)
        } else {
            Request::RegisterChallenge
        };
        let (_, difficulty) = challenge(&browser, &request, PERSON);
        assert_eq!(difficulty, with(index / 5), "challenge {}", index + 1);
    }
    for request in [reported(0), Request::RegisterChallenge] {
        assert_eq!(
            browser.answer_as(&request, PERSON, true).err(),
            Some(Error::Busy)
        );
    }
}

/// One who holds a block of addresses is a new address with each request
/// and stays within every address's budget: the block is counted too, and
/// from its twenty-first challenge of the hour each costs more.
fn block(address: SocketAddr, with: &dyn Fn(u8) -> u8) {
    const SUBNETS: [&str; 21] = [
        "2001:db8:5:1::1",
        "2001:db8:5:2::1",
        "2001:db8:5:3::1",
        "2001:db8:5:4::1",
        "2001:db8:5:5::1",
        "2001:db8:5:6::1",
        "2001:db8:5:7::1",
        "2001:db8:5:8::1",
        "2001:db8:5:9::1",
        "2001:db8:5:a::1",
        "2001:db8:5:b::1",
        "2001:db8:5:c::1",
        "2001:db8:5:d::1",
        "2001:db8:5:e::1",
        "2001:db8:5:f::1",
        "2001:db8:5:10::1",
        "2001:db8:5:11::1",
        "2001:db8:5:12::1",
        "2001:db8:5:13::1",
        "2001:db8:5:14::1",
        "2001:db8:5:15::1",
    ];
    let asked: Vec<u8> = SUBNETS
        .iter()
        .map(|client| challenge(&at(address, client), &reported(0), PERSON).1)
        .collect();
    assert!(asked[..20].iter().all(|difficulty| *difficulty == with(0)));
    assert_eq!(asked[20], with(1));
    // The block next to it has asked for nothing yet.
    let next = at(address, "2001:db8:6:1::1");
    assert_eq!(challenge(&next, &reported(0), PERSON).1, with(0));
}
