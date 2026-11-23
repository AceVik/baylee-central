//! Simultaneous damage for at most three creatures per side. Creatures assign
//! lethal to blockers in printed slot order, then trample over if able.
use super::{Board, Keyword, Piece, Side};

#[derive(Clone, Copy)]
enum Recipient {
    Player(Side),
    Creature(Side, usize),
}
#[derive(Clone, Copy)]
struct Hit {
    target: Recipient,
    damage: i16,
    deathtouch: bool,
    lifelink: Option<Side>,
}
fn hit(hits: &mut Vec<Hit>, source: &Piece, side: Side, target: Recipient, damage: i16) {
    if damage > 0 {
        hits.push(Hit {
            target,
            damage,
            deathtouch: source.has(Keyword::Deathtouch),
            lifelink: source.has(Keyword::Lifelink).then_some(side),
        });
    }
}
fn attacker_damage(
    hits: &mut Vec<Hit>,
    board: &Board,
    side: Side,
    attacker: usize,
    blocks: [Option<u8>; 3],
) {
    let source = &board.units[side.index()][attacker];
    let defender = side.other();
    let mut power = source.power;
    let blocked = |slot: &Option<u8>| slot.is_some_and(|id| usize::from(id) == attacker);
    let originally_blocked = blocks.iter().any(blocked);
    let mut alive = blocks
        .iter()
        .enumerate()
        .filter(|(slot, target)| blocked(target) && board.units[defender.index()][*slot].alive)
        .peekable();
    while let Some((slot, _)) = alive.next() {
        let guard = &board.units[defender.index()][slot];
        let lethal = if source.has(Keyword::Deathtouch) {
            1
        } else {
            guard.toughness - guard.damage
        };
        let damage = if alive.peek().is_none() && !source.has(Keyword::Trample) {
            power
        } else {
            power.min(lethal)
        };
        hit(
            hits,
            source,
            side,
            Recipient::Creature(defender, slot),
            damage,
        );
        power -= damage;
    }
    if !originally_blocked || source.has(Keyword::Trample) {
        hit(hits, source, side, Recipient::Player(defender), power);
    }
}
fn damage_step(board: &mut Board, side: Side, attacks: u8, blocks: [Option<u8>; 3], first: bool) {
    let mut hits = Vec::with_capacity(8);
    let active = |piece: &Piece| piece.alive && piece.has(Keyword::FirstStrike) == first;
    for (slot, source) in board.units[side.index()].iter().enumerate() {
        if attacks & (1 << slot) != 0 && active(source) {
            attacker_damage(&mut hits, board, side, slot, blocks);
        }
    }
    let defender = side.other();
    for (slot, target) in blocks.iter().enumerate() {
        let Some(target) = target.map(usize::from) else {
            continue;
        };
        let source = &board.units[defender.index()][slot];
        if active(source) && board.units[side.index()][target].alive {
            hit(
                &mut hits,
                source,
                defender,
                Recipient::Creature(side, target),
                source.power,
            );
        }
    }
    for Hit {
        target,
        damage,
        deathtouch,
        lifelink,
    } in hits
    {
        if let Some(controller) = lifelink {
            board.life[controller.index()] += damage;
        }
        match target {
            Recipient::Player(owner) => board.life[owner.index()] -= damage,
            Recipient::Creature(owner, slot) => {
                let p = &mut board.units[owner.index()][slot];
                p.damage += damage;
                if deathtouch || p.damage >= p.toughness {
                    p.alive = false;
                }
            }
        }
    }
}
pub(super) fn resolve(board: &mut Board, side: Side, attacks: u8, blocks: [Option<u8>; 3]) {
    for (slot, unit) in board.units[side.index()].iter_mut().enumerate() {
        if attacks & (1 << slot) != 0 && !unit.has(Keyword::Vigilance) {
            unit.tapped = true;
        }
    }
    damage_step(board, side, attacks, blocks, true);
    if board.life.iter().all(|&life| life > 0) {
        damage_step(board, side, attacks, blocks, false);
    }
}
