use super::*;
use crate::{decode, encode};

fn variant(seats: u8, allies: u8, sides: u8, x: u16) -> MatVariant {
    MatVariant {
        seats,
        allies,
        sides,
        framing: Framing {
            focus: [x, 5000],
            safe: None,
        },
    }
}

#[test]
fn duel_is_canonical_and_3v2_depends_on_the_owners_team() {
    let left = variant(5, 3, 2, 2000);
    let right = variant(5, 2, 2, 8000);
    let ffa = variant(5, 1, 5, 4000);
    let four = variant(4, 2, 2, 1000);
    let framing = MatFraming {
        duel: Framing {
            focus: [5000, 6000],
            safe: Some([2500, 3000, 7500, 9000]),
        },
        variants: MatVariants::try_from([left, right, ffa, four].as_slice()).unwrap(),
    };
    assert_eq!(framing.for_layout(2, 1, 2), framing.duel);
    assert_eq!(framing.for_layout(7, 1, 7), framing.duel);
    for v in [left, right, ffa, four] {
        assert_eq!(framing.for_layout(v.seats, v.allies, v.sides), v.framing);
    }
    let reordered = MatFraming {
        variants: MatVariants::try_from([right, four, ffa, left].as_slice()).unwrap(),
        ..framing
    };
    assert_eq!(encode(&reordered).unwrap(), encode(&framing).unwrap());
    assert_eq!(
        decode::<MatFraming>(&encode(&framing).unwrap()).unwrap(),
        framing
    );
    let centered = decode::<MatFraming>(br#"{"duel":{"focus":[5000,5000]}}"#).unwrap();
    assert_eq!(centered, MatFraming::default());
}

#[test]
fn refuse_impossible_teams_and_unbounded_or_ambiguous_layouts() {
    for key in [
        (2, 1, 2),
        (3, 0, 2),
        (3, 2, 3),
        (5, 3, 4),
        (5, 5, 1),
        (9, 2, 2),
    ] {
        let v = variant(key.0, key.1, key.2, 100);
        assert!(MatVariants::try_from([v].as_slice()).is_err());
    }
    let v = variant(3, 1, 3, 100);
    assert!(MatVariants::try_from([v, v].as_slice()).is_err());
    let nine = [
        (3, 1, 3),
        (3, 1, 2),
        (3, 2, 2),
        (4, 1, 4),
        (4, 1, 3),
        (4, 1, 2),
        (4, 2, 2),
        (4, 3, 2),
        (5, 1, 5),
    ]
    .map(|(s, a, t)| variant(s, a, t, 0));
    assert!(MatVariants::try_from(&nine[..8]).is_ok());
    assert!(decode::<MatVariants>(&encode(&nine).unwrap()).is_err());
}

#[test]
fn reject_out_of_bounds_inverted_degenerate_and_malformed_safe_regions_on_decode() {
    for wire in [
        r#"{"focus":[10001,0]}"#,
        r#"{"focus":[1.2,0]}"#,
        r#"{"focus":[0,0],"safe":[0,0,0,4]}"#,
        r#"{"focus":[3,0],"safe":[3,4,1,0]}"#,
        r#"{"focus":[8,8],"safe":[0,0,4,4]}"#,
        r#"{"focus":[8,8],"safe":[0,0,10001,9999]}"#,
        r#"{"focus":[8,8],"url":"unexpected"}"#,
    ] {
        assert!(decode::<Framing>(wire.as_bytes()).is_err());
    }
    assert!(decode::<Framing>(br#"{"focus":[10000,0],"safe":[0,0,10000,10000]}"#).is_ok());
}
