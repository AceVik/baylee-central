use super::{BUILD, Node, Registered, Table, Token, api, r};

pub(super) struct Game<'a> {
    pub(super) leader: &'a Node,
    pub(super) witness: &'a Node,
    pub(super) seats: [&'a Token; 2],
    pub(super) fence: r::Fence,
    pub(super) engine: api::Id,
    pub(super) opening: r::Head,
}

pub(super) fn attestation(
    fence: r::Fence,
    head: r::Head,
    state: u8,
    terminal: Option<api::Outcome>,
) -> r::HostRequest {
    let (awaited, decisions) = if terminal.is_some() {
        (0, [None, None])
    } else {
        (1, [Some(7), None])
    };
    r::HostRequest::Attest {
        fence,
        head,
        state: [state; 32],
        accepted: true,
        awaited,
        decisions,
        terminal,
    }
}

async fn inspect(table: &Table<'_>, node: &Node, matched: api::Id, role: r::Role) -> r::Work {
    let r::HostResponse::Assignment { work } = node
        .host(table.store, r::HostRequest::Inspect { match_id: matched })
        .await
        .unwrap()
    else {
        panic!("authenticated assignment")
    };
    assert_eq!(work.role, role);
    assert_eq!(work.phase, r::Phase::Preparing);
    assert_eq!(work.compatibility, BUILD);
    work
}

async fn claim(table: &Table<'_>, work: &r::Work, leader: &Node, seats: [&Token; 2], key: u8) {
    let sid = |index| super::super::id(key + index);
    let mut proofs = Vec::new();
    for (i, seat) in seats.iter().enumerate() {
        let capability = table
            .store
            .gateway_grant_scoped(
                seat,
                &[leader.origin],
                true,
                central_api::wire::GatewayPurpose::Ranked,
                sid(u8::try_from(i + 3).unwrap()),
                Some(work.fence),
            )
            .await
            .unwrap();
        assert_eq!(capability.account, work.accounts[i]);
        let stale = r::Fence {
            generation: work.fence.generation + 1,
            ..work.fence
        };
        assert!(
            table
                .store
                .gateway_grant_scoped(
                    seat,
                    &[leader.origin],
                    true,
                    central_api::wire::GatewayPurpose::Ranked,
                    sid(6),
                    Some(stale)
                )
                .await
                .is_err()
        );
        proofs.push(api::SeatProof::from_bytes(capability.sid));
    }
    let request = |i: usize, proof: api::SeatProof| api::HostRequest::Claim {
        match_id: work.fence.match_id,
        account: work.accounts[i],
        deck: work.decks[i],
        proof,
    };
    assert!(
        table
            .store
            .ranked_host(Registered(&leader.token), request(0, proofs[1].clone()))
            .await
            .is_err()
    );
    for (i, proof) in proofs.into_iter().enumerate() {
        table
            .store
            .ranked_host(Registered(&leader.token), request(i, proof))
            .await
            .unwrap();
    }
}

async fn setup(
    table: &Table<'_>,
    leader: &Node,
    witness: &Node,
    work: &r::Work,
    engine: api::Id,
) -> r::Head {
    let fence = work.fence;
    let pools = [
        vec![api::PoolEntry {
            card: [21; 32],
            copies: 100,
        }],
        vec![api::PoolEntry {
            card: [22; 32],
            copies: 100,
        }],
    ];
    let body = r::SetupBytes::new(vec![0x63, 0x42, 0x89]).unwrap();
    let request = r::HostRequest::Setup {
        fence,
        game: engine,
        body: body.clone(),
        pools: pools.clone(),
    };
    assert!(witness.host(table.store, request.clone()).await.is_err());
    let missing = r::HostRequest::Setup {
        fence,
        game: engine,
        body: body.clone(),
        pools: [vec![], pools[1].clone()],
    };
    assert!(leader.host(table.store, missing).await.is_err());
    assert_eq!(table.receipt_count(fence.match_id).await, 0);
    leader.host(table.store, request.clone()).await.unwrap();
    leader.host(table.store, request).await.unwrap();
    let opening = r::Head::opening(fence.match_id, engine, BUILD, work.seed, &body, &pools);
    let read = r::HostRequest::Read { fence, after: 0 };
    assert!(
        table
            .outsider
            .host(table.store, read.clone())
            .await
            .is_err()
    );
    let r::HostResponse::Page {
        setup,
        opening: found,
        pools: cards,
        ..
    } = witness.host(table.store, read).await.unwrap()
    else {
        panic!("opening page")
    };
    assert_eq!(found, opening);
    assert_eq!(
        setup.as_ref().map(r::SetupBytes::as_slice),
        Some(body.as_slice())
    );
    assert_eq!(cards.as_ref(), Some(&pools));
    opening
}

impl<'a> Game<'a> {
    pub(super) async fn open(table: &'a Table<'_>, key: u8) -> Self {
        let assigned = table.start_pair(key).await;
        let matched = assigned.match_id;
        let leader = table.host(assigned.host_id);
        let witness = if leader.id == table.first.id {
            &table.second
        } else {
            &table.first
        };
        super::participants::run(table, matched, assigned.seat).await;
        super::privacy::protect(table, matched, leader, witness).await;
        let work = inspect(table, leader, matched, r::Role::Primary).await;
        let peer = inspect(table, witness, matched, r::Role::Standby).await;
        assert_eq!(work.fence, peer.fence);
        for alias in work.accounts.into_iter().chain(peer.accounts) {
            assert!(table.seats.iter().all(|(id, _)| *id != alias));
        }
        assert!(work.accounts.iter().all(|id| !peer.accounts.contains(id)));
        assert!(
            table
                .outsider
                .host(table.store, r::HostRequest::Inspect { match_id: matched })
                .await
                .is_err()
        );
        let r::HostResponse::Work { items, pending } = table
            .outsider
            .host(table.store, r::HostRequest::Work)
            .await
            .unwrap()
        else {
            panic!("bounded private work")
        };
        assert!(
            items.iter().all(|r| r.fence.match_id != matched)
                && pending.iter().all(|r| r.fence.match_id != matched)
        );
        let [(_, first), (_, second)] = table.seats;
        let seats = if assigned.seat == 0 {
            [second, first]
        } else {
            [first, second]
        };
        claim(table, &work, leader, seats, key).await;
        let engine = super::super::id(key + 2);
        let opening = setup(table, leader, witness, &work, engine).await;
        let live = Self {
            leader,
            witness,
            seats,
            fence: work.fence,
            engine,
            opening,
        };
        live.certify_opening(table, key).await;
        live
    }

    async fn certify_opening(&self, table: &Table<'_>, key: u8) {
        let fence = self.fence;
        let report = attestation(fence, self.opening, 5, None);
        self.leader.host(table.store, report.clone()).await.unwrap();
        assert_ne!(table.status(fence.match_id).await.phase, r::Phase::Playing);
        self.witness.host(table.store, report).await.unwrap();
        let status = table.status(fence.match_id).await;
        assert_eq!(
            (status.phase, status.head),
            (r::Phase::Playing, self.opening)
        );
        let early = self.submit(super::super::id(key + 2), &[0x13]);
        let r::SeatResponse::Status { status } = table.seat(self.seats[0], early).await.unwrap()
        else {
            panic!("admission is still pending")
        };
        assert!(status.clock.is_none());
        assert_eq!(status.head, self.opening);
        for (index, player) in self.seats.iter().enumerate() {
            table
                .seat(
                    player,
                    r::SeatRequest::Presence {
                        fence,
                        connected: true,
                    },
                )
                .await
                .unwrap();
            let route = table
                .seat(player, r::SeatRequest::RelayRoute { fence })
                .await
                .unwrap();
            assert_eq!(
                route,
                r::SeatResponse::RelayRoute {
                    origin: self.leader.origin.into(),
                    seat: u8::try_from(index).unwrap(),
                    game: Some(self.engine)
                }
            );
        }
        self.leader
            .host(table.store, r::HostRequest::Presence { fence, attached: 3 })
            .await
            .unwrap();
        assert!(
            table
                .seat(table.observer, r::SeatRequest::RelayRoute { fence })
                .await
                .is_err()
        );
    }

    pub(super) fn submit(&self, command_id: api::Id, bytes: &[u8]) -> r::SeatRequest {
        r::SeatRequest::Submit {
            fence: self.fence,
            expected: self.opening,
            command_id,
            body: r::CommandBytes::new(bytes.to_vec()).unwrap(),
        }
    }
}
