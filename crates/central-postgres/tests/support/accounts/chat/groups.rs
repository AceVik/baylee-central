//! Real group authorization, private invitations, delivery races and quotas.
use super::*;
use central_api::groups::{Change, GroupRequest, GroupResponse, Groups};

const A: u32 = 0x7fff_ffdb;
const B: u32 = 0x7fff_ffdc;
const C: u32 = 0x7fff_ffdd;

async fn group(
    store: &AccountStore,
    token: &Token,
    request: GroupRequest,
) -> Result<GroupResponse, Error> {
    match store
        .social(Session::Native(token), SocialRequest::Group { request })
        .await?
    {
        SocialResponse::Group { response } => Ok(response),
        _ => panic!("group response required"),
    }
}
async fn change(
    store: &AccountStore,
    token: &Token,
    id: &str,
    change: Change,
) -> Result<GroupResponse, Error> {
    group(
        store,
        token,
        GroupRequest::Change {
            id: id.into(),
            change,
        },
    )
    .await
}
async fn list(store: &AccountStore, token: &Token) -> Groups {
    match group(store, token, GroupRequest::List).await.unwrap() {
        GroupResponse::Groups { snapshot } => snapshot,
        _ => panic!("group snapshot required"),
    }
}
async fn create(store: &AccountStore, token: &Token) -> String {
    match group(
        store,
        token,
        GroupRequest::Create {
            name: "Test Café".into(),
        },
    )
    .await
    .unwrap()
    {
        GroupResponse::Created { id } => id,
        _ => panic!("created group required"),
    }
}
async fn contact(store: &AccountStore, token: &Token, tag: u32, change: ContactAction) {
    store
        .social(
            Session::Native(token),
            SocialRequest::Contact { tag, change },
        )
        .await
        .unwrap();
}
fn message(
    conv: &str,
    from: &DeviceSecrets,
    tag: u32,
    recipients: &[&DeviceSecrets],
    id: u32,
) -> Sealed {
    let header = Header {
        v: 1,
        id: format!("019a3d70-0000-7000-8000-{id:012x}"),
        conv: conv.into(),
        from: format!("{tag:04x}"),
        dev: from.device_id().into(),
        ts: u64::try_from(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis(),
        )
        .unwrap(),
        kind: "text".into(),
    };
    let hdr = central_api::encode(&header).unwrap();
    let recipients: Vec<_> = recipients
        .iter()
        .map(|key| Recipient {
            device: key.device_id(),
            dk: key.dk(),
        })
        .collect();
    let cipher = central_crypto::chat::seal(
        from,
        &hdr,
        br#"{"text":"group test"}"#,
        &recipients,
        &mut Draw(59),
    )
    .unwrap();
    Sealed {
        hdr: url::encode(&hdr),
        n: url::encode(&cipher.n),
        ct: url::encode(&cipher.ct),
        epk: url::encode(&cipher.epk),
        sig: url::encode(&cipher.sig),
        wraps: cipher
            .wraps
            .into_iter()
            .map(|w| Wrap {
                device: w.device,
                k: url::encode(&w.k),
            })
            .collect(),
    }
}
async fn post(
    store: &AccountStore,
    token: &Token,
    conv: &str,
    sealed: Sealed,
) -> Result<ChatResponse, Error> {
    call(
        store,
        token,
        ChatRequest::Send {
            conv: conv.into(),
            sealed,
        },
    )
    .await
}
async fn history(
    store: &AccountStore,
    token: &Token,
    key: &DeviceSecrets,
    conv: &str,
) -> Vec<Delivery> {
    inbox(store, token, key, 0)
        .await
        .into_iter()
        .filter(|delivery| {
            central_api::decode::<Header>(&url::decode(&delivery.sealed.hdr).unwrap())
                .unwrap()
                .conv
                == conv
        })
        .collect()
}

pub(super) async fn run(store: &AccountStore, tokens: [&Token; 3], keys: [&DeviceSecrets; 2]) {
    let [a, b, c] = tokens;
    let [ak, bk] = keys;
    let ck = DeviceSecrets::from_bytes(&[74; 96]);
    call(
        store,
        c,
        ChatRequest::Register {
            device: registration(&ck, C),
        },
    )
    .await
    .unwrap();
    contact(store, b, A, ContactAction::Unblock).await;
    contact(store, a, B, ContactAction::Request).await;
    contact(store, b, A, ContactAction::Request).await;
    let id = create(store, a).await;
    let conv = format!("grp:{id}");
    assert_eq!(list(store, a).await.groups[0].members.len(), 1);
    assert_eq!(list(store, b).await.groups.len(), 0);
    assert!(change(store, b, &id, Change::Join).await.is_err());
    assert!(change(store, c, &id, Change::Delete).await.is_err());
    assert!(
        change(store, a, &id, Change::Invite { tag: C })
            .await
            .is_err()
    );
    post(store, a, &conv, message(&conv, ak, A, &[ak], 100))
        .await
        .unwrap();
    change(store, a, &id, Change::Invite { tag: B })
        .await
        .unwrap();
    let invited = list(store, b).await;
    assert_eq!(invited.groups.len(), 0);
    assert_eq!(invited.invites[0].inviter.tag, A);
    assert!(
        post(store, b, &conv, message(&conv, bk, B, &[ak, bk], 101))
            .await
            .is_err()
    );
    change(store, b, &id, Change::Join).await.unwrap();
    assert!(
        history(store, b, bk, &conv).await.is_empty(),
        "joining must not disclose earlier ciphertext"
    );
    assert!(
        change(
            store,
            b,
            &id,
            Change::Rename {
                name: "takeover".into()
            }
        )
        .await
        .is_err()
    );
    contact(store, a, C, ContactAction::Request).await;
    contact(store, c, A, ContactAction::Request).await;
    change(store, a, &id, Change::Invite { tag: C })
        .await
        .unwrap();
    // Invitation does not grant directory access to non-friend group members.
    assert!(
        matches!(call(store, c, ChatRequest::Keys { tags: vec![B] }).await.unwrap(), ChatResponse::Keys { keys } if keys.is_empty())
    );
    change(store, c, &id, Change::Join).await.unwrap();
    assert!(
        matches!(call(store, c, ChatRequest::Keys { tags: vec![B] }).await.unwrap(), ChatResponse::Keys { keys } if keys.len() == 1)
    );
    post(store, a, &conv, message(&conv, ak, A, &[ak, bk, &ck], 102))
        .await
        .unwrap();
    assert_eq!(history(store, b, bk, &conv).await.len(), 1);
    assert_eq!(history(store, c, &ck, &conv).await.len(), 1);
    block_and_departure(store, tokens, [ak, bk, &ck], &id).await;
    quotas_and_labels(store, a).await;
}

async fn block_and_departure(
    store: &AccountStore,
    tokens: [&Token; 3],
    keys: [&DeviceSecrets; 3],
    id: &str,
) {
    let [a, b, c] = tokens;
    let [ak, bk, ck] = keys;
    let conv = format!("grp:{id}");
    contact(store, b, A, ContactAction::Block).await;
    assert!(
        matches!(call(store, a, ChatRequest::Keys { tags: vec![B] }).await.unwrap(), ChatResponse::Keys { keys } if keys.len() == 1)
    );
    post(store, a, &conv, message(&conv, ak, A, &[ak, bk, ck], 103))
        .await
        .unwrap();
    assert_eq!(
        history(store, b, bk, &conv).await.len(),
        1,
        "blocking suppresses only new delivery, without revealing the block"
    );
    assert_eq!(history(store, c, ck, &conv).await.len(), 2);
    let (removed, sent) = tokio::join!(
        change(store, a, id, Change::Remove { tag: B }),
        post(store, b, &conv, message(&conv, bk, B, &[ak, bk, ck], 104)),
    );
    removed.unwrap();
    assert!(matches!(
        sent,
        Ok(ChatResponse::Sent { .. }) | Err(Error::Rejected)
    ));
    let before = history(store, b, bk, &conv).await.len();
    assert!(
        post(store, b, &conv, message(&conv, bk, B, &[ak, bk, ck], 105))
            .await
            .is_err()
    );
    assert!(matches!(
        post(store, a, &conv, message(&conv, ak, A, &[ak, bk, ck], 106)).await,
        Err(Error::Conflict)
    ));
    post(store, a, &conv, message(&conv, ak, A, &[ak, ck], 106))
        .await
        .unwrap();
    assert_eq!(history(store, b, bk, &conv).await.len(), before);
    assert!(
        change(store, a, id, Change::Remove { tag: A })
            .await
            .is_err()
    );
    change(store, a, id, Change::Transfer { tag: C })
        .await
        .unwrap();
    assert!(change(store, a, id, Change::Delete).await.is_err());
    change(store, c, id, Change::Delete).await.unwrap();
    assert_eq!(list(store, a).await.groups.len(), 0);
    assert!(
        !history(store, c, ck, &conv).await.is_empty(),
        "deleting group metadata retains previously delivered ciphertext"
    );
    assert!(
        post(store, a, &conv, message(&conv, ak, A, &[ak, ck], 107))
            .await
            .is_err()
    );
}

async fn quotas_and_labels(store: &AccountStore, a: &Token) {
    for name in ["", " ", "bad\u{202e}name", &"é".repeat(49)] {
        assert!(matches!(
            group(store, a, GroupRequest::Create { name: name.into() }).await,
            Err(Error::Rejected)
        ));
    }
    let mut ids = Vec::new();
    for _ in 0..20 {
        ids.push(create(store, a).await);
    }
    assert!(matches!(
        group(
            store,
            a,
            GroupRequest::Create {
                name: "Full".into()
            }
        )
        .await,
        Err(Error::Busy)
    ));
    for id in ids {
        change(store, a, &id, Change::Delete).await.unwrap();
    }
    assert_eq!(list(store, a).await.groups.len(), 0);
}
