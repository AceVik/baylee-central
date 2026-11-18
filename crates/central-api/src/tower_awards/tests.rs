use super::*;
use crate::challenges::Tier;

fn uuid(tail: u8) -> [u8; 16] {
    let mut id = [0; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    id[15] = tail;
    id
}

fn first() -> Fact {
    Fact {
        schema: 2,
        run: uuid(1),
        cohort: uuid(2),
        stage: 1,
        outcome: Outcome::Victory,
        wins: 1,
        defeats: 0,
    }
}

#[test]
fn every_bounded_result_path_reaches_a_consistent_permanent_projection() {
    for decisions in 0_u16..4096 {
        let mut progress = Progress::default();
        let mut previous = None;
        let (mut wins, mut defeats) = (0, 0);
        for stage in 1..=12 {
            let victory = decisions & (1 << (stage - 1)) != 0;
            wins += u8::from(victory);
            defeats += u8::from(!victory);
            let fact = Fact {
                stage,
                wins,
                defeats,
                outcome: if victory {
                    Outcome::Victory
                } else if stage % 2 == 0 {
                    Outcome::Draw
                } else {
                    Outcome::Defeat
                },
                ..first()
            };
            fact.follows(previous.as_ref()).unwrap();
            progress.apply(&fact).unwrap();
            assert_eq!(progress.duels, u64::from(stage));
            assert_eq!(progress.victories, u64::from(wins));
            assert_eq!(progress.clears, u64::from(wins == 8));
            assert_eq!(progress.flawless, u64::from(wins == 8 && defeats == 0));
            if wins == 8 || defeats == 5 {
                break;
            }
            previous = Some(fact);
        }
        assert!(wins == 8 || defeats == 5);
    }
}

#[test]
fn malformed_facts_and_unexpected_delivery_fields_are_rejected() {
    let delivery = Delivery {
        event: uuid(3),
        account: uuid(4),
        fact: first(),
    };
    delivery.validate().unwrap();
    let mut wire = serde_json::to_value(delivery).unwrap();
    wire["gold"] = serde_json::json!(1000);
    assert!(serde_json::from_value::<Delivery>(wire).is_err());
    let mut wire = serde_json::to_value(delivery).unwrap();
    wire["fact"]["badge"] = serde_json::json!("obsidian");
    assert!(serde_json::from_value::<Delivery>(wire).is_err());
    let mut wire = serde_json::to_value(delivery).unwrap();
    wire["fact"]["outcome"] = serde_json::json!("technical_failure");
    assert!(serde_json::from_value::<Delivery>(wire).is_err());
    for fact in [
        Fact {
            schema: 1,
            ..first()
        },
        Fact {
            stage: 0,
            ..first()
        },
        Fact {
            stage: 13,
            wins: 8,
            defeats: 5,
            ..first()
        },
        Fact {
            stage: 2,
            ..first()
        },
        Fact {
            wins: 255,
            ..first()
        },
        Fact {
            run: [0; 16],
            ..first()
        },
        Fact {
            cohort: [0; 16],
            ..first()
        },
        Fact {
            outcome: Outcome::Defeat,
            ..first()
        },
        Fact {
            wins: 0,
            defeats: 1,
            ..first()
        },
        Fact {
            stage: 6,
            defeats: 5,
            ..first()
        },
        Fact {
            stage: 9,
            wins: 8,
            defeats: 1,
            outcome: Outcome::Draw,
            ..first()
        },
    ] {
        assert_eq!(fact.validate(), Err(Error::Rejected));
    }
    for account in [
        [0; 16],
        {
            let mut id = uuid(4);
            id[6] = 0x40;
            id
        },
        {
            let mut id = uuid(4);
            id[8] = 0xc0;
            id
        },
    ] {
        assert_eq!(
            Delivery {
                account,
                ..delivery
            }
            .validate(),
            Err(Error::Rejected)
        );
        assert_eq!(
            Delivery {
                event: account,
                ..delivery
            }
            .validate(),
            Err(Error::Rejected)
        );
    }
}

#[test]
fn receipt_order_prevents_gaps_rebinding_and_results_after_an_ending() {
    let one = first();
    let two = Fact {
        stage: 2,
        wins: 2,
        ..one
    };
    assert_eq!(two.follows(None), Err(Error::Conflict));
    two.follows(Some(&one)).unwrap();
    assert_eq!(one.follows(Some(&one)), Err(Error::Conflict));
    let three = Fact {
        stage: 3,
        wins: 3,
        ..one
    };
    assert_eq!(three.follows(Some(&one)), Err(Error::Conflict));
    for changed in [
        Fact {
            run: uuid(99),
            ..two
        },
        Fact {
            cohort: uuid(99),
            ..two
        },
        Fact {
            wins: 0,
            defeats: 2,
            outcome: Outcome::Defeat,
            ..two
        },
    ] {
        assert_eq!(changed.follows(Some(&one)), Err(Error::Conflict));
    }
    let last_loss = Fact {
        stage: 5,
        wins: 0,
        defeats: 5,
        outcome: Outcome::Defeat,
        ..one
    };
    let after = Fact {
        stage: 6,
        wins: 2,
        defeats: 4,
        ..one
    };
    assert_eq!(after.follows(Some(&last_loss)), Err(Error::Conflict));
    let last_win = Fact {
        stage: 8,
        wins: 8,
        ..one
    };
    let after = Fact {
        stage: 9,
        wins: 7,
        defeats: 2,
        outcome: Outcome::Draw,
        ..one
    };
    assert_eq!(after.follows(Some(&last_win)), Err(Error::Conflict));
}

#[test]
fn progress_overflow_and_corruption_leave_the_original_unchanged() {
    let mut full = Progress {
        duels: u64::MAX,
        victories: u64::MAX,
        clears: 0,
        flawless: 0,
    };
    let original = full;
    assert_eq!(full.apply(&first()), Err(Error::Busy));
    assert_eq!(full, original);
    for mut broken in [
        Progress {
            duels: 1,
            victories: 2,
            ..Progress::default()
        },
        Progress {
            flawless: 1,
            ..Progress::default()
        },
        Progress {
            duels: u64::MAX,
            victories: u64::MAX,
            clears: u64::MAX,
            flawless: 0,
        },
    ] {
        let before = broken;
        assert_eq!(broken.apply(&first()), Err(Error::Rejected));
        assert_eq!(broken, before);
    }
}

#[test]
fn permanent_tiers_cover_exact_thresholds_without_reset_or_wraparound() {
    let tiers = [
        Tier::Bronze,
        Tier::Silver,
        Tier::Gold,
        Tier::Platinum,
        Tier::Obsidian,
    ];
    for kind in [
        BadgeKind::Victories,
        BadgeKind::Ascents,
        BadgeKind::Unbroken,
    ] {
        assert_eq!(kind.progress(0).tier, None);
        for (index, target) in kind.thresholds().into_iter().enumerate() {
            assert_eq!(kind.progress(target - 1).next, Some(target));
            assert_ne!(kind.progress(target - 1).tier, Some(tiers[index]));
            assert_eq!(kind.progress(target).tier, Some(tiers[index]));
        }
        assert_eq!(kind.progress(u64::MAX).tier, Some(Tier::Obsidian));
        assert_eq!(kind.progress(u64::MAX).next, None);
    }
    assert_eq!(
        Progress::default().badges().map(|badge| badge.count),
        [0; 3]
    );
}

#[test]
fn exports_reject_oversized_reordered_or_nonprogressing_pages() {
    let row = ExportedReceipt {
        id: uuid(10),
        event: uuid(11),
        origin: "https://tower.example.invalid".into(),
        fact: first(),
        accepted_at: 1,
    };
    let valid = Export {
        progress: Progress {
            duels: 1,
            victories: 1,
            ..Progress::default()
        },
        receipts: vec![row.clone()],
        next: None,
    };
    valid.validate(None).unwrap();
    assert_eq!(valid.validate(Some(row.id)), Err(Error::Rejected));
    assert_eq!(
        Export {
            next: Some(row.id),
            ..valid.clone()
        }
        .validate(None),
        Err(Error::Rejected)
    );
    assert_eq!(
        Export {
            progress: Progress::default(),
            ..valid.clone()
        }
        .validate(None),
        Err(Error::Rejected)
    );
    assert_eq!(
        Export {
            receipts: vec![row.clone(); 129],
            ..valid.clone()
        }
        .validate(None),
        Err(Error::Rejected)
    );
    for bad in [
        ExportedReceipt {
            event: [0; 16],
            ..row.clone()
        },
        ExportedReceipt {
            accepted_at: 0,
            ..row.clone()
        },
        ExportedReceipt {
            origin: "http://attacker.invalid".into(),
            ..row.clone()
        },
    ] {
        assert_eq!(
            Export {
                receipts: vec![bad],
                ..valid.clone()
            }
            .validate(None),
            Err(Error::Rejected)
        );
    }
    let mut page = Export {
        progress: Progress {
            duels: 128,
            victories: 128,
            ..Progress::default()
        },
        receipts: (1..=128)
            .map(|n| ExportedReceipt {
                id: uuid(n),
                ..row.clone()
            })
            .collect(),
        next: Some(uuid(128)),
    };
    page.validate(None).unwrap();
    page.receipts.swap(0, 1);
    assert_eq!(page.validate(None), Err(Error::Rejected));
}
