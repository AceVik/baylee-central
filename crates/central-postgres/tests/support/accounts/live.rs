//! What the push socket asks of the real store (`docs/push-socket.md`): which
//! sessions stand, who an account's friends are, and whom each social change
//! concerns, so that a socket hears exactly its own account's news.
use super::{AccountStore, Error, ServerKeys, Token, enroll, login};
use central_api::{
    chat::{ChatRequest, DeviceRegistration, Header, Sealed, Wrap},
    groups::{Change, GroupRequest, GroupResponse},
    social::{ContactAction, SocialRequest, SocialResponse},
    wire::Identity,
};
use central_crypto::chat::{DeviceSecrets, Entropy, Recipient, b64::url};
use central_postgres::{Concerns, LIVE_CHECK, Session};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};
use std::collections::BTreeSet;

struct Draw(u8);
impl Entropy for Draw {
    fn fill(&mut self, out: &mut [u8]) {
        out.fill(self.0);
        self.0 = self.0.wrapping_add(1);
    }
}

/// Three synthetic accounts with public profiles, and their tags.
const TAGS: [u32; 3] = [0x7fff_ff10, 0x7fff_ff11, 0x7fff_ff12];

fn set(accounts: &[Identity]) -> BTreeSet<[u8; 16]> {
    accounts.iter().map(|identity| identity.account).collect()
}

async fn concerning(
    store: &AccountStore,
    token: &Token,
    request: SocialRequest,
) -> Result<(SocialResponse, Concerns), Error> {
    store
        .social_concerning(Session::Native(token), request)
        .await
}

async fn contact(store: &AccountStore, token: &Token, tag: u32, change: ContactAction) -> Concerns {
    concerning(store, token, SocialRequest::Contact { tag, change })
        .await
        .unwrap()
        .1
}

async fn group(store: &AccountStore, token: &Token, request: GroupRequest) -> Concerns {
    concerning(store, token, SocialRequest::Group { request })
        .await
        .unwrap()
        .1
}

fn registration(key: &DeviceSecrets, tag: u32) -> DeviceRegistration {
    let p = key.public(&format!("{tag:04x}")).unwrap();
    DeviceRegistration {
        ik: url::encode(&p.ik),
        dk: url::encode(&p.dk),
        dsig: url::encode(&p.dsig),
        label: "Live".into(),
    }
}

/// A message from `from` in `conv`, wrapped for each of `to`.
fn sealed(from: &DeviceSecrets, to: &[&DeviceSecrets], tag: u32, id: u32, conv: &str) -> Sealed {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let header = Header {
        v: 1,
        id: format!("019a3d70-0000-7000-8000-{id:012x}"),
        conv: conv.into(),
        from: format!("{tag:04x}"),
        dev: from.device_id().into(),
        ts: u64::try_from(now).unwrap(),
        kind: "text".into(),
    };
    let hdr = central_api::encode(&header).unwrap();
    let recipients: Vec<Recipient> = to
        .iter()
        .map(|key| Recipient {
            device: key.device_id(),
            dk: key.dk(),
        })
        .collect();
    let message = central_crypto::chat::seal(
        from,
        &hdr,
        br#"{"text":"synthetic push test"}"#,
        &recipients,
        &mut Draw(9),
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

async fn chat(store: &AccountStore, token: &Token, request: ChatRequest) -> Concerns {
    concerning(store, token, SocialRequest::Chat { request })
        .await
        .unwrap()
        .1
}

pub(super) async fn run(store: &AccountStore, operator: &DatabaseConnection, keys: &ServerKeys) {
    let mut identities = Vec::new();
    for (index, (legacy, name)) in [
        ("live-a", "Live_Alice"),
        ("live-b", "Live_Bob"),
        ("live-c", "Live_Carol"),
    ]
    .into_iter()
    .enumerate()
    {
        identities.push(enroll(store, keys, legacy, &name.to_lowercase().replace('_', "")).await);
        store
            .legacy_profile(legacy, Some(name), TAGS[index])
            .await
            .unwrap();
    }
    let [ai, bi, ci] = [identities[0], identities[1], identities[2]];
    let a = login(store, keys, ai).await;
    let b = login(store, keys, bi).await;
    let c = login(store, keys, ci).await;
    sessions(store, operator, keys, [ai, bi, ci], [&a, &b, &c]).await;
    // `sessions` replaced A's and ended B's and C's; sign in again.
    let a = login(store, keys, ai).await;
    let b = login(store, keys, bi).await;
    let c = login(store, keys, ci).await;
    contacts(store, [ai, bi, ci], [&a, &b, &c]).await;
    let keys = messages(store, [ai, bi, ci], [&a, &b, &c]).await;
    groups(store, [ai, bi, ci], [&a, &b, &c], &keys).await;
}

/// Which sessions stand, in one statement for many.
async fn sessions(
    store: &AccountStore,
    operator: &DatabaseConnection,
    keys: &ServerKeys,
    [ai, _, _]: [Identity; 3],
    [a, b, c]: [&Token; 3],
) {
    let all = BTreeSet::from([a.digest(), b.digest(), c.digest()]);
    let named: Vec<[u8; 32]> = all.iter().copied().collect();
    assert_eq!(store.live_sessions(&named).await.unwrap(), all);
    assert!(store.live_sessions(&[]).await.unwrap().is_empty());
    assert_eq!(
        store
            .live_sessions(&vec![[7; 32]; LIVE_CHECK + 1])
            .await
            .unwrap_err(),
        Error::Rejected
    );
    // nobody's digest, and a browser session's token as a native digest
    let browser = Token::generate().unwrap();
    store
        .issue_session(
            &store.credential(ai).await.unwrap().unwrap(),
            Session::Browser(&browser),
        )
        .await
        .unwrap();
    assert!(
        store
            .live_sessions(&[[7; 32], browser.digest()])
            .await
            .unwrap()
            .is_empty()
    );
    // a native sign-in replaces A's native session, not its browser one
    let again = login(store, keys, ai).await;
    assert_eq!(
        store
            .live_sessions(&[a.digest(), again.digest()])
            .await
            .unwrap(),
        BTreeSet::from([again.digest()])
    );
    // a sign-out
    store.logout(Session::Native(b)).await.unwrap();
    // an expired session (the clock moved past it)
    operator
        .execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "UPDATE central_identity.sessions SET created_at = pg_catalog.clock_timestamp() - interval '1 hour',
                 expires_at = pg_catalog.clock_timestamp() - interval '1 second' WHERE digest = $1",
            vec![c.digest().to_vec().into()],
        ))
        .await
        .unwrap();
    assert_eq!(
        store
            .live_sessions(&[again.digest(), b.digest(), c.digest()])
            .await
            .unwrap(),
        BTreeSet::from([again.digest()])
    );
    store.logout(Session::Browser(&browser)).await.unwrap();
}

/// Contact changes name exactly the side whose view changed; friends are the
/// accepted contacts, a block removes one.
async fn contacts(store: &AccountStore, [ai, bi, ci]: [Identity; 3], [a, b, c]: [&Token; 3]) {
    let [_, bt, ct] = TAGS;
    let friends = |identity: Identity| async move {
        store
            .live_friends(identity.account)
            .await
            .unwrap()
            .into_iter()
            .collect::<BTreeSet<_>>()
    };
    // A asks B: both see the request; not friends yet.
    let asked = contact(store, a, bt, ContactAction::Request).await;
    assert_eq!(asked.social, set(&[ai, bi]));
    assert_eq!(asked.pair, Some((ai.account, bi.account)));
    assert!(asked.chat.is_empty());
    assert!(friends(ai).await.is_empty());
    // asking again changes nothing, and nobody is told
    assert_eq!(
        contact(store, a, bt, ContactAction::Request).await,
        Concerns::default()
    );
    // B accepts: friends both ways
    let accepted = contact(store, b, TAGS[0], ContactAction::Accept).await;
    assert_eq!(accepted.social, set(&[ai, bi]));
    assert_eq!(friends(ai).await, set(&[bi]));
    assert_eq!(friends(bi).await, set(&[ai]));
    // A blocks C, who is no contact: C is told nothing
    let blocked = contact(store, a, ct, ContactAction::Block).await;
    assert_eq!(blocked.social, set(&[ai]));
    assert!(!blocked.social.contains(&ci.account));
    let unblocked = contact(store, a, ct, ContactAction::Unblock).await;
    assert_eq!(unblocked.social, set(&[ai]));
    // lifting a block that is not there, removing a contact that is not there
    assert_eq!(
        contact(store, a, ct, ContactAction::Unblock).await,
        Concerns::default()
    );
    assert_eq!(
        contact(store, b, ct, ContactAction::Remove).await,
        Concerns::default()
    );
    // C asks A and A accepts by asking back: a request can accept
    contact(store, c, TAGS[0], ContactAction::Request).await;
    let crossed = contact(store, a, ct, ContactAction::Request).await;
    assert_eq!(crossed.social, set(&[ai, ci]));
    assert_eq!(friends(ai).await, set(&[bi, ci]));
    // reads concern nobody
    for request in [
        SocialRequest::Contacts,
        SocialRequest::Lookup { tag: bt },
        SocialRequest::Group {
            request: GroupRequest::List,
        },
    ] {
        assert_eq!(
            concerning(store, a, request).await.unwrap().1,
            Concerns::default()
        );
    }
}

/// A new device is its account's news; a delivery names who took it. The
/// three accounts' devices, for the groups.
async fn messages(
    store: &AccountStore,
    [ai, bi, ci]: [Identity; 3],
    [a, b, c]: [&Token; 3],
) -> [DeviceSecrets; 3] {
    let [at, bt, ct] = TAGS;
    let keys = [
        DeviceSecrets::from_bytes(&[81; 96]),
        DeviceSecrets::from_bytes(&[82; 96]),
        DeviceSecrets::from_bytes(&[83; 96]),
    ];
    for ((token, key), (identity, tag)) in
        [a, b, c]
            .into_iter()
            .zip(&keys)
            .zip([(ai, at), (bi, bt), (ci, ct)])
    {
        let register = || ChatRequest::Register {
            device: registration(key, tag),
        };
        // a new device is its own account's news, and nobody else's
        assert_eq!(
            chat(store, token, register()).await.social,
            set(&[identity])
        );
        assert_eq!(chat(store, token, register()).await, Concerns::default());
    }
    let [ak, bk, _] = &keys;
    // a direct message: the sender and the receiver
    let conv = format!("dm:{at:04x}:{bt:04x}");
    let message = sealed(ak, &[ak, bk], at, 1, &conv);
    let sent = chat(
        store,
        a,
        ChatRequest::Send {
            conv: conv.clone(),
            sealed: message.clone(),
        },
    )
    .await;
    assert_eq!(sent.chat, set(&[ai, bi]));
    assert!(sent.social.is_empty() && sent.pair.is_none());
    // its retry delivers nothing new: nobody is told
    assert_eq!(
        chat(
            store,
            a,
            ChatRequest::Send {
                conv,
                sealed: message
            }
        )
        .await,
        Concerns::default()
    );
    // reading an inbox concerns nobody
    assert_eq!(
        chat(
            store,
            b,
            ChatRequest::Inbox {
                device: bk.device_id().into(),
                after: 0
            }
        )
        .await,
        Concerns::default()
    );
    keys
}

/// A group change names who sees it; a group message who took it.
async fn groups(
    store: &AccountStore,
    [ai, bi, ci]: [Identity; 3],
    [a, b, c]: [&Token; 3],
    [ak, bk, ck]: &[DeviceSecrets; 3],
) {
    let [at, bt, ct] = TAGS;
    // A's group with B and C
    let created = concerning(
        store,
        a,
        SocialRequest::Group {
            request: GroupRequest::Create {
                name: "Live push".into(),
            },
        },
    )
    .await
    .unwrap();
    assert_eq!(created.1.social, set(&[ai]));
    let SocialResponse::Group {
        response: GroupResponse::Created { id },
    } = created.0
    else {
        panic!("a group");
    };
    let change = |change: Change| GroupRequest::Change {
        id: id.clone(),
        change,
    };
    // invited: the joined members and the invitee
    assert_eq!(
        group(store, a, change(Change::Invite { tag: bt }))
            .await
            .social,
        set(&[ai, bi])
    );
    // inviting again changes nothing
    assert_eq!(
        group(store, a, change(Change::Invite { tag: bt })).await,
        Concerns::default()
    );
    assert_eq!(
        group(store, a, change(Change::Invite { tag: ct }))
            .await
            .social,
        set(&[ai, ci])
    );
    // B joins: the joined members and B; C, invited still, sees nothing of it
    assert_eq!(
        group(store, b, change(Change::Join)).await.social,
        set(&[ai, bi])
    );
    // a rename reaches the invitee too, who sees the name
    assert_eq!(
        group(
            store,
            a,
            change(Change::Rename {
                name: "Live push 2".into()
            })
        )
        .await
        .social,
        set(&[ai, bi, ci])
    );
    assert_eq!(
        group(store, c, change(Change::Join)).await.social,
        set(&[ai, bi, ci])
    );
    assert_eq!(
        group(store, c, change(Change::Join)).await,
        Concerns::default(),
        "joined already"
    );
    // C blocks A: their contact ends, C's membership stays
    let blocked = contact(store, c, at, ContactAction::Block).await;
    assert_eq!(blocked.social, set(&[ai, ci]));
    // A writes to the group: C's devices take none of A's messages, so C is
    // not told; B is
    let conv = format!("grp:{id}");
    let sent = chat(
        store,
        a,
        ChatRequest::Send {
            conv: conv.clone(),
            sealed: sealed(ak, &[ak, bk, ck], at, 2, &conv),
        },
    )
    .await;
    assert_eq!(sent.chat, set(&[ai, bi]));
    // B writes: C is told, A too
    let sent = chat(
        store,
        b,
        ChatRequest::Send {
            conv: conv.clone(),
            sealed: sealed(bk, &[ak, bk, ck], bt, 3, &conv),
        },
    )
    .await;
    assert_eq!(sent.chat, set(&[ai, bi, ci]));
    // C leaves: the joined members, C among them
    assert_eq!(
        group(store, c, change(Change::Remove { tag: ct }))
            .await
            .social,
        set(&[ai, bi, ci])
    );
    // the group ends: everyone in it
    assert_eq!(
        group(store, a, change(Change::Delete)).await.social,
        set(&[ai, bi])
    );
}
