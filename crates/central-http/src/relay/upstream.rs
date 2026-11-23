//! Four fixed HTTP exchanges. Neither a caller's headers nor an upstream's arbitrary
//! fields/error messages cross the relay. Private buffers never implement Display.
use super::game_id;
use central_api::{
    Error, SignedToken,
    ranked::{
        Format, Id,
        replica::{
            Fence,
            relay::{Call, HttpBytes, Reply, Secret},
        },
    },
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use zeroize::Zeroizing;

pub(super) enum ResultBody {
    Status(u16),
    Admitted {
        token: Secret,
        renew_in: u64,
    },
    Seat {
        token: Secret,
        game: String,
        seat: u8,
    },
    Socket {
        ticket: Secret,
        ttl: Duration,
        game: Id,
    },
}
#[derive(Clone, Copy)]
enum Operation {
    Admit,
    Play,
    Socket(Id),
}
pub(super) struct Post {
    path: String,
    bearer: Option<Secret>,
    body: Zeroizing<Vec<u8>>,
    op: Operation,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Deck {
    format: Format,
    cards: Vec<String>,
    sideboard: Vec<String>,
    commanders: Vec<String>,
}

fn playable(deck: &Deck) -> bool {
    let rows = deck.cards.iter().chain(&deck.sideboard);
    let clean = |s: &str| {
        !s.is_empty()
            && s.len() <= 2048
            && s.trim() == s
            && !s.chars().any(char::is_control)
            && !s.contains(" #")
    };
    !deck.cards.is_empty()
        && rows.clone().all(|s| {
            clean(s)
                && s.split_once(' ').is_some_and(|(n, name)| {
                    !name.is_empty() && n.parse::<u32>().is_ok_and(|n| n > 0)
                })
        })
        && deck.commanders.iter().all(|s| clean(s))
}

pub(super) fn prepare(
    call: Call,
    fence: Fence,
    bound: Option<Id>,
    signed: Option<&SignedToken>,
) -> Result<Post, Error> {
    #[derive(Serialize)]
    struct Admission<'a> {
        token: &'a SignedToken,
    }
    #[derive(Serialize)]
    struct Resume {
        game: String,
    }
    #[derive(Serialize)]
    struct Ticket<'a> {
        socket: &'a str,
        game: String,
    }
    let play = || format!("/ranked/{}/play", game_id(&fence.match_id));
    let (path, bearer, body, op) = match call {
        Call::Admit {} => (
            "/auth/ranked/login".to_owned(),
            None,
            central_api::encode(&Admission {
                token: signed.ok_or(Error::Rejected)?,
            })?,
            Operation::Admit,
        ),
        Call::Play { bearer, deck } if bearer.is_capability() => {
            let deck: Deck = central_api::decode(deck.as_slice())?;
            if !playable(&deck) {
                return Err(Error::Rejected);
            }
            (
                play(),
                Some(bearer),
                central_api::encode(&deck)?,
                Operation::Play,
            )
        }
        Call::Reclaim { bearer, game } if bearer.is_capability() && bound == Some(game) => (
            play(),
            Some(bearer),
            central_api::encode(&Resume {
                game: game_id(&game),
            })?,
            Operation::Play,
        ),
        Call::SocketTicket { seat_token, game }
            if seat_token.is_capability() && bound == Some(game) =>
        {
            (
                "/ws-ticket".into(),
                Some(seat_token),
                central_api::encode(&Ticket {
                    socket: "seat",
                    game: game_id(&game),
                })?,
                Operation::Socket(game),
            )
        }
        _ => return Err(Error::Rejected),
    };
    Ok(Post {
        path,
        bearer,
        body: Zeroizing::new(body),
        op,
    })
}

fn agent(origin: &str, development: bool) -> Result<central_outbound::Agent, Error> {
    central_api::gateway::validate_origin(origin, development)?;
    let total = Duration::from_secs(8);
    let connect = Duration::from_secs(3);
    if development {
        // validate_origin(true) requires a canonical numeric loopback address and port.
        return Ok(central_outbound::Agent::config_builder()
            .proxy(None)
            .max_redirects(0)
            .timeout_global(Some(total))
            .timeout_connect(Some(connect))
            .max_response_header_size(8192)
            .http_status_as_error(false)
            .build()
            .new_agent());
    }
    Ok(central_outbound::agent(central_outbound::Limits {
        port: None,
        total,
        connect,
        pin: None,
    }))
}

pub(super) fn send(origin: &str, development: bool, post: &Post) -> Result<ResultBody, Error> {
    let agent = agent(origin, development)?;
    let mut request = agent
        .post(format!("{origin}{}", post.path))
        .header("Content-Type", "application/json");
    if let Some(bearer) = &post.bearer {
        let header = Zeroizing::new(format!("Bearer {}", bearer.expose()));
        request = request.header("Authorization", header.as_str());
    }
    let mut response = request
        .send(post.body.as_slice())
        .map_err(|_| Error::Unavailable)?;
    let status = response.status().as_u16();
    if status != 200 {
        let status = match (status, post.op) {
            (202, Operation::Play) => 202,
            (400 | 401 | 403 | 404 | 409 | 413 | 426 | 429 | 503, _) => status,
            _ => return Err(Error::Unavailable),
        };
        return Ok(ResultBody::Status(status));
    }
    let bytes = Zeroizing::new(
        response
            .body_mut()
            .with_config()
            .limit(4096)
            .read_to_vec()
            .map_err(|_| Error::Unavailable)?,
    );
    parse(post.op, &bytes)
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Admission {
    token: Secret,
    renew_in: u64,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Chair {
    game_id: String,
    seat: u8,
    seat_token: Secret,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Ticket {
    ticket: Secret,
    expires_in: u64,
}

pub(super) fn admitted(handle: Secret, renew_in: u64) -> Result<Reply, Error> {
    Ok(Reply::Http {
        status: 200,
        body: Some(HttpBytes::new(central_api::encode(&Admission {
            token: handle,
            renew_in,
        })?)?),
    })
}
pub(super) fn seated(handle: Secret, game: Id, seat: u8) -> Result<Reply, Error> {
    Ok(Reply::Http {
        status: 200,
        body: Some(HttpBytes::new(central_api::encode(&Chair {
            seat_token: handle,
            game_id: game_id(&game),
            seat,
        })?)?),
    })
}

fn parse(op: Operation, bytes: &[u8]) -> Result<ResultBody, Error> {
    match op {
        Operation::Admit => {
            let value: Admission = central_api::decode(bytes)?;
            if !value.token.is_capability() || !(1..=1800).contains(&value.renew_in) {
                return Err(Error::Rejected);
            }
            Ok(ResultBody::Admitted {
                token: value.token,
                renew_in: value.renew_in,
            })
        }
        Operation::Play => {
            let value: Chair = central_api::decode(bytes)?;
            if !value.seat_token.is_capability() || value.seat > 1 {
                return Err(Error::Rejected);
            }
            Ok(ResultBody::Seat {
                token: value.seat_token,
                game: value.game_id,
                seat: value.seat,
            })
        }
        Operation::Socket(game) => {
            let value: Ticket = central_api::decode(bytes)?;
            if !value.ticket.is_capability() || value.expires_in == 0 {
                return Err(Error::Rejected);
            }
            Ok(ResultBody::Socket {
                ticket: value.ticket,
                ttl: Duration::from_secs(value.expires_in),
                game,
            })
        }
    }
}
