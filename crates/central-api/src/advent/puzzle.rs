//! Small, open-information tactical duels for seasonal doors. The displayed
//! script fixes each draw, opposing attack and blocking policy. Play, hints
//! and validation need no hidden opponent, random shuffle or card database.
use serde::{Deserialize, Serialize};

mod combat;
mod play;
mod solve;
pub use play::{Board, Piece, Position};
pub use solve::Study;
#[cfg(test)]
mod tests;

/// At most three tactical choices and three creatures on either side.
pub const MAX_STEPS: usize = 3;
const SLOTS: usize = 3;

/// The board whose life or creatures are affected.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    /// The puzzle player.
    You,
    /// The scripted opponent.
    Opponent,
}
impl Side {
    const fn index(self) -> usize {
        match self {
            Self::You => 0,
            Self::Opponent => 1,
        }
    }
    const fn other(self) -> Self {
        match self {
            Self::You => Self::Opponent,
            Self::Opponent => Self::You,
        }
    }
}

/// Standard combat abilities used by this focused ruleset.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Keyword {
    /// Only flying or reach can block this attacker.
    Flying,
    /// Can block a flying creature.
    Reach,
    /// Deals combat damage in the first-strike step.
    FirstStrike,
    /// A single point of creature damage is lethal.
    Deathtouch,
    /// Excess combat damage after assigning lethal can reach the defender.
    Trample,
    /// Its controller gains life equal to its damage.
    Lifelink,
    /// Attacking leaves this creature untapped.
    Vigilance,
}

/// A token identified on the board by its side and original slot (A, B, C).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Unit {
    /// Printed power.
    pub power: u8,
    /// Printed toughness.
    pub toughness: u8,
    /// Printed abilities, without duplicates.
    pub keywords: Vec<Keyword>,
    /// Whether this piece enters the displayed starting position tapped.
    pub tapped: bool,
}

/// Face-up instant effects. Each spell can be cast once from the player's
/// hand during an explicitly marked casting window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Spell {
    /// One mana. Target friendly creature gets +2/+2 this turn.
    Might,
    /// One mana. Deal one damage to any creature or either player.
    Spark,
    /// Two mana. Deal three damage to any creature or either player.
    Bolt,
    /// One mana. Tap a creature.
    Frost,
    /// One mana. Target friendly creature gains flying this turn.
    Wings,
    /// One mana. Target friendly creature gains first strike this turn.
    Edge,
    /// Two mana. Friendly creatures get +1/+1 this turn.
    Rally,
    /// One mana. Gain three life.
    Mend,
}
impl Spell {
    /// Generic mana required to cast this card.
    #[must_use]
    pub const fn cost(self) -> u8 {
        match self {
            Self::Bolt | Self::Rally => 2,
            _ => 1,
        }
    }
}

/// What the player is trying to achieve by the last step.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Goal {
    /// Finish above zero life, or win the duel earlier.
    Survive,
    /// Defeat the opponent while surviving.
    Defeat,
}

/// Turn transition before a step. Remove damage and temporary effects,
/// then untap the active side. Mana and any draw are printed in advance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewTurn {
    /// The active player.
    pub active: Side,
    /// The puzzle player's available mana after this transition.
    pub mana: u8,
}

/// The decision being requested, including the opponent's fixed plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Phase {
    /// Cast one instant or pass.
    Cast,
    /// Declare blockers. Bits 0–2 name the opposing creatures that will
    /// attack if alive and untapped.
    Block {
        #[doc = "Opposing attackers in original slot order."]
        attackers: u8,
    },
    /// Declare attackers. Each opposing creature blocks the designated
    /// friendly slot when legal. Damage is assigned to blockers in slot order.
    Attack {
        #[doc = "For each opposing slot, its planned attacking target."]
        guards: [Option<u8>; 3],
    },
}

/// One public step along the fixed encounter.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Step {
    /// Optional cleanup/untap boundary before this choice.
    pub new_turn: Option<NewTurn>,
    /// Fixed face-up draw before this choice.
    pub draw: Option<Spell>,
    /// Requested player choice.
    pub phase: Phase,
}

/// Complete public board. Names, hints, reward and day-to-puzzle mappings
/// belong to the seasonal authority and its private campaign.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Puzzle {
    /// Win condition printed before play.
    pub goal: Goal,
    /// Life totals in player, opponent order.
    pub life: [u8; 2],
    /// Available generic mana.
    pub mana: u8,
    /// Friendly and opposing battlefield slots, respectively.
    pub units: [Vec<Unit>; 2],
    /// Fixed face-up starting hand. Equal cards are interchangeable.
    pub hand: Vec<Spell>,
    /// Fixed public encounter (one to three decisions).
    pub steps: Vec<Step>,
}

/// A target for a spell.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Target {
    /// One of the two players.
    Player(Side),
    /// An original battlefield slot.
    Unit {
        #[doc = "Controller."]
        side: Side,
        #[doc = "Zero-based original slot."]
        slot: u8,
    },
}

/// A player decision, independent of localized labels and UI ordering.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Move {
    /// Keep all cards in hand during this casting window.
    Pass,
    /// Use one named card currently available in hand.
    Cast {
        #[doc = "Spell to consume."]
        spell: Spell,
        #[doc = "Legal target, or none for global effects."]
        target: Option<Target>,
    },
    /// Bits 0–2 name the chosen friendly attackers.
    Attack {
        #[doc = "Attacking slots."]
        attackers: u8,
    },
    /// Each own slot blocks one attacking enemy slot, or stays out.
    Block {
        #[doc = "One assignment per friendly slot."]
        assignments: [Option<u8>; 3],
    },
}

/// Ordered decisions from a fresh opening. Server validation enforces the
/// three-step cap before performing any simulation.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct Line(pub Vec<Move>);

impl<'de> Deserialize<'de> for Line {
    fn deserialize<D: serde::Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        struct Bounded;
        impl<'de> serde::de::Visitor<'de> for Bounded {
            type Value = Line;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("up to three tactical moves")
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<Line, A::Error> {
                let mut moves = Vec::with_capacity(MAX_STEPS);
                while let Some(choice) = seq.next_element()? {
                    if moves.len() == MAX_STEPS {
                        return Err(serde::de::Error::custom("too many tactical moves"));
                    }
                    moves.push(choice);
                }
                Ok(Line(moves))
            }
        }
        decoder.deserialize_seq(Bounded)
    }
}

impl Puzzle {
    /// Check the private operator-authored package and all references.
    ///
    /// # Errors
    /// Rejects unsupported dimensions, impossible turn ownership and invalid
    /// script targets. Rejected packages cannot create a playable board.
    pub fn validate(&self) -> Result<(), crate::Error> {
        let valid_side = |units: &Vec<Unit>| {
            units.len() <= SLOTS
                && units.iter().all(|unit| {
                    unit.power <= 12
                        && (1..=12).contains(&unit.toughness)
                        && unit.keywords.len() <= 7
                        && unit
                            .keywords
                            .iter()
                            .enumerate()
                            .all(|(i, k)| !unit.keywords[..i].contains(k))
                })
        };
        if !(1..=MAX_STEPS).contains(&self.steps.len())
            || self.life.iter().any(|n| !(1..=40).contains(n))
            || self.mana > 6
            || self.hand.len() + self.steps.iter().filter(|s| s.draw.is_some()).count() > 5
            || !self.units.iter().all(valid_side)
        {
            return Err(crate::Error::Rejected);
        }
        let mut active = None;
        let mut fought = false;
        for step in &self.steps {
            if let Some(turn) = step.new_turn {
                if turn.mana > 6 || active == Some(turn.active) {
                    return Err(crate::Error::Rejected);
                }
                active = Some(turn.active);
                fought = false;
            }
            if step.phase != Phase::Cast {
                if fought {
                    return Err(crate::Error::Rejected);
                }
                fought = true;
            }
            let legal = match step.phase {
                Phase::Cast => true,
                Phase::Block { attackers } => {
                    let ok = active != Some(Side::You)
                        && attackers > 0
                        && attackers < (1 << self.units[1].len());
                    active = Some(Side::Opponent);
                    ok
                }
                Phase::Attack { guards } => {
                    let ok = active != Some(Side::Opponent)
                        && guards.iter().enumerate().all(|(i, guard)| {
                            guard.is_none_or(|slot| {
                                i < self.units[1].len() && usize::from(slot) < self.units[0].len()
                            })
                        });
                    active = Some(Side::You);
                    ok
                }
            };
            if !legal {
                return Err(crate::Error::Rejected);
            }
        }
        Ok(())
    }
}
