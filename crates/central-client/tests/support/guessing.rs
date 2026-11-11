//! Guesses at passwords over the real listeners (`docs/native-login.md`
//! §"Guessing a password online"), with a registered account, real OPAQUE
//! and the addresses a proxy would forward. Only what the first seconds of
//! a minute show: the hours and days, the tables' bounds and the refusals
//! that spend nothing are counted with a clock of their own in
//! `central-identity`'s unit tests.
use super::{
    Http,
    browser::Browser,
    password::{self, OLD, Registered},
};
use central_api::{
    Error,
    wire::{Identity, Request, Response},
};
use central_client::Transport;
use central_crypto::opaque::Login;

/// A sign-in's start from `from` with a password that is not the
/// account's, never finished: one guess.
fn guess(from: &Browser, http: &Http, identity: Identity) -> Result<usize, Error> {
    let context = password::context(http, identity);
    let (_, message) = Login::start("synthetic wrong guess".into(), context).unwrap();
    match from.answer(&Request::LoginStart { identity, message }, None)? {
        Response::LoginChallenge { message, .. } => Ok(message.len()),
        _ => panic!("a challenge"),
    }
}

/// The holder's sign-in at `from`, with the right password.
fn holder(from: &Browser, account: &Registered) {
    let cookie = password::browser_sign_in(from, account.identity, account.context, OLD);
    from.answer(&Request::Logout, Some(&cookie)).unwrap();
}

pub(crate) fn run(http: &Http, browser: &Browser) {
    let at = |client: &'static str| Browser {
        address: browser.address,
        client,
    };
    let (home, stranger, other, third) = (
        at("203.0.113.10"),
        at("203.0.113.20"),
        at("203.0.113.30"),
        at("2001:db8:40:1::7"),
    );
    let account = password::register("guessed-at", http, &home);
    // The holder's first sign-in at home: one of the account's eight
    // evaluations this minute, and from now on the place is its own.
    holder(&home, &account);

    // Others guess from two addresses in turn: the seven evaluations left
    // of the minute and no more, whoever asks and from wherever.
    let places = [&stranger, &other];
    let mut challenge = 0;
    for turn in 0..7 {
        challenge = guess(places[turn % 2], http, account.identity).expect("one of the seven");
    }
    for from in [&stranger, &other, &third] {
        assert_eq!(
            guess(from, http, account.identity).err(),
            Some(Error::Busy),
            "the account's minute is spent for every place that is not its own"
        );
    }
    // The native listener counts the same account. (This fixture's native
    // requests come from loopback without a proxy's hop: no place.)
    let context = password::context(http, account.identity);
    assert_eq!(
        http.exchange(
            &Request::LoginStart {
                identity: account.identity,
                message: Login::start(OLD.into(), context).unwrap().1,
            },
            None,
        )
        .err(),
        Some(Error::Busy)
    );
    // Handed on by the proxy from the holder's address, the same start is
    // answered in the same minute: the native listener counts by the
    // address too, and home is the account's own place on both.
    assert!(matches!(
        http.from(
            "203.0.113.10",
            &Request::LoginStart {
                identity: account.identity,
                message: Login::start(OLD.into(), context).unwrap().1,
            },
        ),
        Ok(Response::LoginChallenge { .. })
    ));
    // From a guesser's address it is refused there as well.
    assert_eq!(
        http.from(
            "203.0.113.20",
            &Request::LoginStart {
                identity: account.identity,
                message: Login::start(OLD.into(), context).unwrap().1,
            },
        )
        .err(),
        Some(Error::Busy)
    );

    // The holder is served at home meanwhile, and a proof gives its guess
    // back: more right sign-ins than the account has guesses at one place
    // in an hour (ten).
    for _ in 0..11 {
        holder(&home, &account);
    }
    // A wrong password at home is one guess there, and the next right one
    // is served.
    assert!(guess(&home, http, account.identity).is_ok());
    holder(&home, &account);

    // An id that is nobody's is answered as an account's is, and counted:
    // one place has ten guesses an hour at it, and all made-up ids
    // together eight a minute, so the eleventh start at the latest is
    // refused (earlier steps of this test may have spent some of the
    // eight).
    let mut nobody = account.identity;
    nobody.account[15] ^= 0xff;
    let mut answered = 0;
    let refused = loop {
        match guess(&third, http, nobody) {
            Ok(length) => {
                assert_eq!(length, challenge, "a stand-in answer of the same size");
                answered += 1;
                assert!(answered <= 10, "made-up ids are counted");
            }
            Err(error) => break error,
        }
    };
    assert_eq!(refused, Error::Busy);

    // An account nobody guessed at is served from the guessers' places:
    // their counts are far from spent, and one account's count is not
    // another's.
    let untouched = password::register("not-guessed-at", http, &at("203.0.113.50"));
    for from in [&stranger, &other, &third] {
        assert!(guess(from, http, untouched.identity).is_ok());
    }

    // The native listener's own budgets, by the address the proxy hands
    // on: thirty discoveries a minute from one, then `Busy` for it alone.
    let discover = Request::Discover {
        username: "guessed-at".into(),
    };
    for _ in 0..30 {
        assert!(http.from("203.0.113.60", &discover).is_ok());
    }
    assert_eq!(
        http.from("203.0.113.60", &discover).err(),
        Some(Error::Busy)
    );
    assert!(http.from("203.0.113.61", &discover).is_ok());
    // A forged earlier hop changes nothing: the last one is the proxy's.
    assert_eq!(
        http.from("198.51.100.9, 203.0.113.60", &discover).err(),
        Some(Error::Busy)
    );
    // Without a hop there is no address to count by (a proxy that sent
    // none must not make everyone one address).
    assert!(http.exchange(&discover, None).is_ok());

    password::stale(http, browser);
}
