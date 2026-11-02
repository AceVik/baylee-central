//! Real database authorization, ciphertext delivery, retries and concurrent senders.
use super::{AccountStore, Error, ServerKeys, Token, enroll, login};
use central_api::{
    chat::*,
    social::{ContactAction, SocialRequest, SocialResponse},
};
use central_crypto::chat::{DeviceSecrets, Entropy, Recipient, b64::url};
use central_postgres::Session;

mod groups;

struct Draw(u8);
impl Entropy for Draw {
    fn fill(&mut self, out: &mut [u8]) {
        out.fill(self.0);
        self.0 = self.0.wrapping_add(1);
    }
}

async fn call(
    store: &AccountStore,
    token: &Token,
    request: ChatRequest,
) -> Result<ChatResponse, Error> {
    match store
        .social(Session::Native(token), SocialRequest::Chat { request })
        .await?
    {
        SocialResponse::Chat { response } => Ok(response),
        _ => panic!("chat response required"),
    }
}

fn registration(key: &DeviceSecrets, tag: u32) -> DeviceRegistration {
    let p = key.public(&format!("{tag:04x}")).unwrap();
    DeviceRegistration {
        ik: url::encode(&p.ik),
        dk: url::encode(&p.dk),
        dsig: url::encode(&p.dsig),
        label: "Test".into(),
    }
}

fn sealed(from: &DeviceSecrets, to: &DeviceSecrets, tag: u32, id: u32) -> Sealed {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let header = Header {
        v: 1,
        id: format!("019a3d70-0000-7000-8000-{id:012x}"),
        conv: "dm:7fffffdb:7fffffdc".into(),
        from: format!("{tag:04x}"),
        dev: from.device_id().into(),
        ts: u64::try_from(now).unwrap(),
        kind: "text".into(),
    };
    let hdr = central_api::encode(&header).unwrap();
    let message = central_crypto::chat::seal(
        from,
        &hdr,
        br#"{"text":"private test message"}"#,
        &[
            Recipient {
                device: from.device_id(),
                dk: from.dk(),
            },
            Recipient {
                device: to.device_id(),
                dk: to.dk(),
            },
        ],
        &mut Draw(41),
    )
    .unwrap();
    Sealed {
        hdr: url::encode(&hdr),
        n: url::encode(&message.n),
        ct: url::encode(&message.ct),
        epk: url::encode(&message.epk),
        sig: url::encode(&message.sig),
        wraps: message
            .wraps
            .into_iter()
            .map(|w| Wrap {
                device: w.device,
                k: url::encode(&w.k),
            })
            .collect(),
    }
}

async fn send(store: &AccountStore, token: &Token, sealed: Sealed) -> Result<ChatResponse, Error> {
    call(
        store,
        token,
        ChatRequest::Send {
            conv: "dm:7fffffdb:7fffffdc".into(),
            sealed,
        },
    )
    .await
}
async fn inbox(
    store: &AccountStore,
    token: &Token,
    device: &DeviceSecrets,
    after: u64,
) -> Vec<Delivery> {
    match call(
        store,
        token,
        ChatRequest::Inbox {
            device: device.device_id().into(),
            after,
        },
    )
    .await
    .unwrap()
    {
        ChatResponse::Inbox { messages, more } => {
            assert!(!more);
            messages
        }
        _ => panic!("inbox required"),
    }
}

pub(super) async fn run(
    store: &AccountStore,
    operator: &sea_orm::DatabaseConnection,
    keys: &ServerKeys,
) {
    let (ai, bi, ci) = enroll_profiles(store, keys).await;
    let (at, bt) = (0x7fff_ffdb, 0x7fff_ffdc);
    let (a, b, c) = (
        login(store, keys, ai).await,
        login(store, keys, bi).await,
        login(store, keys, ci).await,
    );
    let (ak, bk) = (
        DeviceSecrets::from_bytes(&[71; 96]),
        DeviceSecrets::from_bytes(&[72; 96]),
    );
    registration_and_membership(store, [&a, &b, &c], [&ak, &bk]).await;
    invalid_sends(store, &a, &ak, &bk, at).await;
    let original = sealed(&ak, &bk, at, 2);
    let first = central_api::encode(&send(store, &a, original.clone()).await.unwrap()).unwrap();
    assert_eq!(
        first,
        central_api::encode(&send(store, &a, original).await.unwrap()).unwrap()
    );
    let mut changed = sealed(&ak, &bk, at, 2);
    changed.wraps[0].k = url::encode(&[0; 48]);
    assert!(matches!(
        send(store, &a, changed).await,
        Err(Error::Conflict)
    ));
    let got = inbox(store, &b, &bk, 0).await;
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].sender.tag, at);
    assert_eq!(got[0].sealed.wraps.len(), 1);
    assert_eq!(got[0].sealed.wraps[0].device, bk.device_id());
    let wire = central_api::encode(&got).unwrap();
    assert!(!String::from_utf8_lossy(&wire).contains("private test message"));
    assert!(inbox(store, &b, &bk, got[0].seq).await.is_empty());
    assert_eq!(inbox(store, &a, &ak, 0).await.len(), 1);
    // Two installations can replay the same durable, non-destructive history.
    let browser = Token::generate().unwrap();
    store
        .issue_session(
            &store.credential(bi).await.unwrap().unwrap(),
            Session::Browser(&browser),
        )
        .await
        .unwrap();
    let browser_history = store
        .social(
            Session::Browser(&browser),
            SocialRequest::Chat {
                request: ChatRequest::Inbox {
                    device: bk.device_id().into(),
                    after: 0,
                },
            },
        )
        .await
        .unwrap();
    assert!(
        matches!(browser_history, SocialResponse::Chat { response: ChatResponse::Inbox { messages, .. } } if messages.len() == 1)
    );
    let (left, right) = tokio::join!(
        send(store, &a, sealed(&ak, &bk, at, 3)),
        send(store, &b, sealed(&bk, &ak, bt, 4))
    );
    left.unwrap();
    right.unwrap();
    let got = inbox(store, &a, &ak, 0).await;
    assert_eq!(got.len(), 3);
    assert!(got.windows(2).all(|pair| pair[0].seq < pair[1].seq));
    store
        .social(
            Session::Native(&b),
            SocialRequest::Contact {
                tag: at,
                change: ContactAction::Block,
            },
        )
        .await
        .unwrap();
    assert!(send(store, &a, sealed(&ak, &bk, at, 5)).await.is_err());
    assert!(send(store, &b, sealed(&bk, &ak, bt, 6)).await.is_err());
    store.logout(Session::Browser(&browser)).await.unwrap();
    assert!(
        store
            .social(
                Session::Browser(&browser),
                SocialRequest::Chat {
                    request: ChatRequest::Inbox {
                        device: bk.device_id().into(),
                        after: 0
                    }
                }
            )
            .await
            .is_err()
    );
    assert_eq!(inbox(store, &b, &bk, 0).await.len(), 3);
    groups::run(store, [&a, &b, &c], [&ak, &bk]).await;
    enrolled_tag_is_stable(store, operator, ai).await;
    new_epoch_excludes_old_devices(store, operator, ai, &a, &ak).await;
}

async fn invalid_sends(
    store: &AccountStore,
    a: &Token,
    ak: &DeviceSecrets,
    bk: &DeviceSecrets,
    at: u32,
) {
    let valid = sealed(ak, bk, at, 10);
    let mut missing = valid.clone();
    missing.wraps.pop();
    assert!(matches!(
        send(store, a, missing).await,
        Err(Error::Conflict)
    ));
    let mut duplicate = valid.clone();
    duplicate.wraps.push(duplicate.wraps[0].clone());
    assert!(send(store, a, duplicate).await.is_err());
    let mut changed = valid.clone();
    changed.ct.push('A');
    assert!(send(store, a, changed).await.is_err());
    let mut changed = valid.clone();
    changed.sig = url::encode(&[0; 64]);
    assert!(send(store, a, changed).await.is_err());
    let mut changed = valid;
    changed.hdr = url::encode(br#"{"v":1,"id":"bad"}"#);
    assert!(send(store, a, changed).await.is_err());
    assert!(inbox(store, a, ak, 0).await.is_empty());
}

async fn registration_and_membership(
    store: &AccountStore,
    tokens: [&Token; 3],
    devices: [&DeviceSecrets; 2],
) {
    let [a, b, c] = tokens;
    let [ak, bk] = devices;
    let (at, bt, ct) = (0x7fff_ffdb, 0x7fff_ffdc, 0x7fff_ffdd);
    // Wrong account binding and malformed device statements cannot claim a device.
    assert!(
        call(
            store,
            a,
            ChatRequest::Register {
                device: registration(ak, bt)
            }
        )
        .await
        .is_err()
    );
    for _ in 0..2 {
        assert!(matches!(
            call(
                store,
                a,
                ChatRequest::Register {
                    device: registration(ak, at)
                }
            )
            .await
            .unwrap(),
            ChatResponse::Registered { .. }
        ));
    }
    call(
        store,
        b,
        ChatRequest::Register {
            device: registration(bk, bt),
        },
    )
    .await
    .unwrap();
    assert!(
        call(
            store,
            c,
            ChatRequest::Register {
                device: registration(ak, ct)
            }
        )
        .await
        .is_err()
    );
    assert!(
        call(
            store,
            c,
            ChatRequest::Inbox {
                device: ak.device_id().into(),
                after: 0
            }
        )
        .await
        .is_err()
    );
    assert!(send(store, a, sealed(ak, bk, at, 1)).await.is_err());
    match call(
        store,
        c,
        ChatRequest::Keys {
            tags: vec![at, bt, ct],
        },
    )
    .await
    .unwrap()
    {
        ChatResponse::Keys { keys } => {
            assert_eq!(keys.len(), 1);
            assert_eq!(keys[0].tag, format!("{ct:04x}"));
        }
        _ => panic!("directory required"),
    }
    for (token, tag) in [(a, bt), (b, at)] {
        store
            .social(
                Session::Native(token),
                SocialRequest::Contact {
                    tag,
                    change: ContactAction::Request,
                },
            )
            .await
            .unwrap();
    }
}

async fn enroll_profiles(
    store: &AccountStore,
    keys: &ServerKeys,
) -> (super::Identity, super::Identity, super::Identity) {
    let ai = enroll(store, keys, "chat-a", "chatalice").await;
    let bi = enroll(store, keys, "chat-b", "chatbob").await;
    let ci = enroll(store, keys, "chat-c", "chatcarol").await;
    let (at, bt, ct) = (0x7fff_ffdb, 0x7fff_ffdc, 0x7fff_ffdd);
    for (legacy, name, tag) in [
        ("chat-a", "Chat_Alice", at),
        ("chat-b", "Chat_Bob", bt),
        ("chat-c", "Chat_Carol", ct),
    ] {
        store.legacy_profile(legacy, Some(name), tag).await.unwrap();
    }
    (ai, bi, ci)
}

async fn new_epoch_excludes_old_devices(
    store: &AccountStore,
    operator: &sea_orm::DatabaseConnection,
    old: super::Identity,
    old_session: &Token,
    old_keys: &DeviceSecrets,
) {
    use sea_orm::ConnectionTrait;
    // Simulate the store's atomic New-vault epoch/revision transition. The
    // complete OPAQUE reset flow is separately exercised over real HTTP.
    operator.execute_unprepared("UPDATE central_identity.accounts SET vault_epoch = vault_epoch + 1, credential_revision = credential_revision + 1 WHERE legacy_id = 'chat-a'").await.unwrap();
    let next = super::Identity {
        epoch: old.epoch + 1,
        ..old
    };
    let token = Token::generate().unwrap();
    store
        .issue_session(
            &store.credential(next).await.unwrap().unwrap(),
            Session::Native(&token),
        )
        .await
        .unwrap();
    assert!(
        call(
            store,
            old_session,
            ChatRequest::Keys {
                tags: vec![0x7fff_ffdb]
            }
        )
        .await
        .is_err()
    );
    assert!(
        call(
            store,
            &token,
            ChatRequest::Inbox {
                device: old_keys.device_id().into(),
                after: 0
            }
        )
        .await
        .is_err()
    );
    assert!(
        call(
            store,
            &token,
            ChatRequest::Register {
                device: registration(old_keys, 0x7fff_ffdb)
            }
        )
        .await
        .is_err()
    );
    let fresh = DeviceSecrets::from_bytes(&[73; 96]);
    call(
        store,
        &token,
        ChatRequest::Register {
            device: registration(&fresh, 0x7fff_ffdb),
        },
    )
    .await
    .unwrap();
    match call(
        store,
        &token,
        ChatRequest::Keys {
            tags: vec![0x7fff_ffdb],
        },
    )
    .await
    .unwrap()
    {
        ChatResponse::Keys { keys } => {
            assert_eq!(keys.len(), 1);
            assert_eq!(keys[0].devices.len(), 1);
            assert_eq!(keys[0].devices[0].id, fresh.device_id());
        }
        _ => panic!("directory required"),
    }
}

async fn enrolled_tag_is_stable(
    store: &AccountStore,
    operator: &sea_orm::DatabaseConnection,
    identity: super::Identity,
) {
    use sea_orm::ConnectionTrait;
    operator.execute_unprepared("INSERT INTO central_identity.account_roles (account_id, role) SELECT id, 'admin' FROM central_identity.accounts WHERE legacy_id = 'chat-a'").await.unwrap();
    let browser = Token::generate().unwrap();
    store
        .issue_session(
            &store.credential(identity).await.unwrap().unwrap(),
            Session::Browser(&browser),
        )
        .await
        .unwrap();
    store
        .set_tag(&browser, identity.account, 0x7fff_ffdb)
        .await
        .unwrap();
    assert_eq!(
        store
            .set_tag(&browser, identity.account, 0x7fff_ffda)
            .await
            .err(),
        Some(Error::Conflict)
    );
    store.logout(Session::Browser(&browser)).await.unwrap();
    operator.execute_unprepared("DELETE FROM central_identity.account_roles WHERE account_id = (SELECT id FROM central_identity.accounts WHERE legacy_id = 'chat-a')").await.unwrap();
}
