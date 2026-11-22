//! A seat at the home gateway has a distinct session and cannot rotate the lobby credential.
use super::{AccountStore, Error, ORIGIN, Token, api, id};
use central_api::wire::GatewayPurpose;

pub(super) async fn assigned(store: &AccountStore, token: &Token, matched: api::Id) {
    let lobby = store
        .gateway_grant(token, &[ORIGIN], true, GatewayPurpose::Admit, id(80))
        .await
        .unwrap();
    let game = store
        .gateway_grant(token, &[ORIGIN], true, GatewayPurpose::Ranked, id(81))
        .await
        .unwrap();
    assert_eq!(game.ranked_match, Some(matched));
    assert!(lobby.ranked_match.is_none());
    assert_ne!(game.sid, lobby.sid);
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
}

pub(super) async fn ended(store: &AccountStore, token: &Token) {
    let grant = store
        .gateway_grant(token, &[ORIGIN], true, GatewayPurpose::Ranked, id(84))
        .await;
    assert!(matches!(grant, Err(Error::Rejected)));
}
