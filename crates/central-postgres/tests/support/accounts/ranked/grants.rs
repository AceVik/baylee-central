//! A seat at the home gateway has a distinct session and cannot rotate the lobby credential.
use super::{AccountStore, Error, ORIGIN, Token, api, id};
use central_api::wire::GatewayPurpose;

fn fence(matched: api::Id) -> api::replica::Fence {
    api::replica::Fence {
        match_id: matched,
        generation: 0,
    }
}

pub(super) async fn alias(
    store: &AccountStore,
    token: &Token,
    matched: api::Id,
) -> (api::Id, [u8; 16]) {
    let grant = store
        .gateway_grant_scoped(
            token,
            &[ORIGIN],
            true,
            GatewayPurpose::Ranked,
            id(89),
            Some(fence(matched)),
        )
        .await
        .unwrap();
    (grant.account, grant.sid)
}

pub(super) async fn assigned(
    store: &AccountStore,
    token: &Token,
    matched: api::Id,
) -> (api::Id, [u8; 16]) {
    let lobby = store
        .gateway_grant(token, &[ORIGIN], true, GatewayPurpose::Admit, id(80))
        .await
        .unwrap();
    let game = store
        .gateway_grant_scoped(
            token,
            &[ORIGIN],
            true,
            GatewayPurpose::Ranked,
            id(81),
            Some(fence(matched)),
        )
        .await
        .unwrap();
    assert_eq!(game.ranked_match, Some(matched));
    assert!(lobby.ranked_match.is_none());
    assert_ne!(game.sid, lobby.sid);
    assert_ne!(game.account, lobby.account);
    let renewed = store
        .gateway_grant_scoped(
            token,
            &[ORIGIN],
            true,
            GatewayPurpose::Ranked,
            id(85),
            Some(fence(matched)),
        )
        .await
        .unwrap();
    assert_eq!(renewed.sid, game.sid);
    assert_eq!(renewed.ranked_match, Some(matched));
    let other = store
        .gateway_grant(
            token,
            &["https://elsewhere.example"],
            true,
            GatewayPurpose::Ranked,
            id(82),
        )
        .await;
    assert!(matches!(other, Err(Error::Rejected)));
    let unchanged = store
        .gateway_grant(token, &[ORIGIN], true, GatewayPurpose::Admit, id(83))
        .await
        .unwrap();
    assert_eq!(lobby.sid, unchanged.sid);
    (game.account, game.sid)
}

pub(super) async fn next_match(store: &AccountStore, token: &Token, matched: api::Id) {
    let grant = store
        .gateway_grant_scoped(
            token,
            &[ORIGIN],
            true,
            GatewayPurpose::Ranked,
            id(86),
            Some(fence(matched)),
        )
        .await
        .unwrap();
    assert_eq!(grant.ranked_match, Some(matched));
    assert_eq!(grant.sid, id(86));
    assert_ne!(grant.sid, id(81));
    let lobby = store
        .gateway_grant(token, &[ORIGIN], true, GatewayPurpose::Admit, id(87))
        .await
        .unwrap();
    assert_eq!(lobby.sid, id(80));
}

pub(super) async fn ended(store: &AccountStore, token: &Token) {
    let grant = store
        .gateway_grant(token, &[ORIGIN], true, GatewayPurpose::Ranked, id(84))
        .await;
    assert!(matches!(grant, Err(Error::Rejected)));
}
