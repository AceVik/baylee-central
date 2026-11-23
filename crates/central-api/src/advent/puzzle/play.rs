use super::{Goal, Keyword, Line, Move, Phase, Puzzle, Side, Spell, Step, Target, Unit, combat};

/// A visible battlefield slot. A dead token retains its slot for subsequent
/// choices and the printed opposing plan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Piece {
    /// Current power with temporary effects.
    pub power: i16,
    /// Current toughness with temporary effects.
    pub toughness: i16,
    /// Damage marked this turn.
    pub damage: i16,
    /// Currently effective printed and temporary keywords.
    pub keywords: Vec<Keyword>,
    /// Tapped creatures cannot attack or block.
    pub tapped: bool,
    /// Whether the token is still on the battlefield.
    pub alive: bool,
}
impl From<&Unit> for Piece {
    fn from(unit: &Unit) -> Self {
        Self {
            power: i16::from(unit.power),
            toughness: i16::from(unit.toughness),
            damage: 0,
            keywords: unit.keywords.clone(),
            tapped: unit.tapped,
            alive: true,
        }
    }
}
impl Piece {
    pub(super) fn has(&self, k: Keyword) -> bool {
        self.keywords.contains(&k)
    }
    fn grant(&mut self, k: Keyword) {
        if !self.has(k) {
            self.keywords.push(k);
        }
    }
    fn pump(&mut self, power: i16, toughness: i16) {
        self.power += power;
        self.toughness += toughness;
    }
    pub(super) fn can_block(&self, attacker: &Self) -> bool {
        self.alive
            && !self.tapped
            && (!attacker.has(Keyword::Flying)
                || self.has(Keyword::Flying)
                || self.has(Keyword::Reach))
    }
}

/// Visible life, mana, creatures and remaining hand after a line of decisions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Board {
    /// Life totals in player, opponent order.
    pub life: [i16; 2],
    /// Generic mana still available.
    pub mana: u8,
    /// Spell cards currently available.
    pub hand: Vec<Spell>,
    /// Friendly and opposing slots respectively.
    pub units: [Vec<Piece>; 2],
}
impl Board {
    fn initial(p: &Puzzle) -> Self {
        Self {
            life: p.life.map(i16::from),
            mana: p.mana,
            hand: p.hand.clone(),
            units: p
                .units
                .each_ref()
                .map(|side| side.iter().map(Piece::from).collect()),
        }
    }
    fn prepare(&mut self, p: &Puzzle, step: Step) {
        if let Some(turn) = step.new_turn {
            self.mana = turn.mana;
            for (i, side) in self.units.iter_mut().enumerate() {
                for (piece, unit) in side.iter_mut().zip(&p.units[i]) {
                    if piece.alive {
                        piece.damage = 0;
                        piece.power = i16::from(unit.power);
                        piece.toughness = i16::from(unit.toughness);
                        piece.keywords.clone_from(&unit.keywords);
                        if i == turn.active.index() {
                            piece.tapped = false;
                        }
                    }
                }
            }
        }
        if let Some(draw) = step.draw {
            self.hand.push(draw);
        }
    }

    pub(super) fn can_attack(&self, side: Side) -> u8 {
        self.units[side.index()]
            .iter()
            .enumerate()
            .fold(0, |mask, (slot, piece)| {
                if piece.alive && !piece.tapped {
                    mask | (1 << slot)
                } else {
                    mask
                }
            })
    }
    pub(super) fn legal_block(
        &self,
        attacker: Side,
        attacks: u8,
        blocker: usize,
        target: u8,
    ) -> bool {
        self.units[attacker.index()]
            .get(usize::from(target))
            .is_some_and(|attack| {
                attacks & (1 << target) != 0
                    && self.units[attacker.other().index()]
                        .get(blocker)
                        .is_some_and(|p| p.can_block(attack))
            })
    }
    fn valid_target(&self, spell: Spell, target: Option<Target>) -> bool {
        match target {
            Some(Target::Unit { side, slot }) => {
                self.units[side.index()]
                    .get(usize::from(slot))
                    .is_some_and(|p| p.alive)
                    && match spell {
                        Spell::Might | Spell::Wings | Spell::Edge => side == Side::You,
                        Spell::Frost | Spell::Bolt | Spell::Spark => true,
                        _ => false,
                    }
            }
            Some(Target::Player(_)) => matches!(spell, Spell::Bolt | Spell::Spark),
            None => matches!(spell, Spell::Mend | Spell::Rally),
        }
    }
    fn cast(&mut self, spell: Spell, target: Option<Target>) -> Result<(), crate::Error> {
        let index = self
            .hand
            .iter()
            .position(|s| *s == spell)
            .ok_or(crate::Error::Rejected)?;
        if self.mana < spell.cost() || !self.valid_target(spell, target) {
            return Err(crate::Error::Rejected);
        }
        self.mana -= spell.cost();
        self.hand.remove(index);
        match (spell, target) {
            (Spell::Mend, None) => self.life[0] += 3,
            (Spell::Rally, None) => self.units[0]
                .iter_mut()
                .filter(|p| p.alive)
                .for_each(|p| p.pump(1, 1)),
            (Spell::Bolt | Spell::Spark, Some(Target::Player(side))) => {
                self.life[side.index()] -= if spell == Spell::Bolt { 3 } else { 1 }
            }
            (_, Some(Target::Unit { side, slot })) => {
                let p = &mut self.units[side.index()][usize::from(slot)];
                match spell {
                    Spell::Might => p.pump(2, 2),
                    Spell::Wings => p.grant(Keyword::Flying),
                    Spell::Edge => p.grant(Keyword::FirstStrike),
                    Spell::Frost => p.tapped = true,
                    Spell::Spark => p.damage += 1,
                    Spell::Bolt => p.damage += 3,
                    Spell::Mend | Spell::Rally => return Err(crate::Error::Rejected),
                }
                if p.damage >= p.toughness {
                    p.alive = false;
                }
            }
            _ => return Err(crate::Error::Rejected),
        }
        Ok(())
    }
    pub(super) fn apply(&mut self, stage: Step, choice: Move) -> Result<(), crate::Error> {
        match (stage.phase, choice) {
            (Phase::Cast, Move::Pass) => Ok(()),
            (Phase::Cast, Move::Cast { spell, target }) => self.cast(spell, target),
            (Phase::Attack { guards }, Move::Attack { attackers })
                if attackers & !self.can_attack(Side::You) == 0 =>
            {
                let mut blocks = guards;
                for (i, slot) in blocks.iter_mut().enumerate() {
                    if slot.is_some_and(|id| !self.legal_block(Side::You, attackers, i, id)) {
                        *slot = None;
                    }
                }
                combat::resolve(self, Side::You, attackers, blocks);
                Ok(())
            }
            (Phase::Block { attackers }, Move::Block { assignments }) => {
                let active = attackers & self.can_attack(Side::Opponent);
                if assignments.iter().enumerate().any(|(i, slot)| {
                    slot.is_some_and(|id| !self.legal_block(Side::Opponent, active, i, id))
                }) {
                    return Err(crate::Error::Rejected);
                }
                combat::resolve(self, Side::Opponent, active, assignments);
                Ok(())
            }
            _ => Err(crate::Error::Rejected),
        }
    }
    fn cast_choices(&self) -> Vec<Move> {
        let mut out = vec![Move::Pass];
        let targets = [Side::You, Side::Opponent]
            .into_iter()
            .flat_map(|side| {
                std::iter::once(Target::Player(side))
                    .chain((0..3).map(move |slot| Target::Unit { side, slot }))
            })
            .map(Some)
            .chain([None]);
        for (i, &spell) in self.hand.iter().enumerate() {
            if self.hand[..i].contains(&spell) || self.mana < spell.cost() {
                continue;
            }
            for target in targets.clone().filter(|&t| self.valid_target(spell, t)) {
                out.push(Move::Cast { spell, target });
            }
        }
        out
    }
    pub(super) fn legal(&self, phase: Phase) -> Vec<Move> {
        match phase {
            Phase::Cast => self.cast_choices(),
            Phase::Attack { .. } => {
                let allowed = self.can_attack(Side::You);
                (0..=allowed)
                    .filter(|mask| mask & !allowed == 0)
                    .map(|attackers| Move::Attack { attackers })
                    .collect()
            }
            Phase::Block { attackers } => {
                let active = attackers & self.can_attack(Side::Opponent);
                let mut boards = vec![[None; 3]];
                for blocker in 0..self.units[0].len() {
                    let count = boards.len();
                    for slot in
                        (0..3).filter(|&id| self.legal_block(Side::Opponent, active, blocker, id))
                    {
                        for i in 0..count {
                            let mut b = boards[i];
                            b[blocker] = Some(slot);
                            boards.push(b);
                        }
                    }
                }
                boards
                    .into_iter()
                    .map(|assignments| Move::Block { assignments })
                    .collect()
            }
        }
    }
}

/// Current step and an optional terminal result for the submitted prefix.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Position {
    /// Public board ready for the next choice.
    pub board: Board,
    /// Index of the next step (or number completed when terminal).
    pub step: usize,
    /// True on a win, false on a finished unsuccessful attempt.
    pub result: Option<bool>,
}
impl Position {
    pub(super) fn next(&mut self, p: &Puzzle, choice: Move) -> Result<(), crate::Error> {
        if self.result.is_some() {
            return Err(crate::Error::Rejected);
        }
        let stage = *p.steps.get(self.step).ok_or(crate::Error::Rejected)?;
        self.board.apply(stage, choice)?;
        self.step += 1;
        self.result = if self.board.life[0] <= 0 {
            Some(false)
        } else if self.board.life[1] <= 0 {
            Some(true)
        } else if let Some(stage) = p.steps.get(self.step) {
            self.board.prepare(p, *stage);
            None
        } else {
            Some(p.goal == Goal::Survive)
        };
        Ok(())
    }
}
impl Puzzle {
    pub(super) fn opening(&self) -> Position {
        let mut board = Board::initial(self);
        board.prepare(self, self.steps[0]);
        Position {
            board,
            step: 0,
            result: None,
        }
    }
    /// Replay an ordered prefix with all ordinary cost and combat checks.
    ///
    /// # Errors
    /// Rejects invalid boards, illegal moves or input after termination.
    pub fn position(&self, line: &Line) -> Result<Position, crate::Error> {
        self.validate()?;
        if line.0.len() > self.steps.len() {
            return Err(crate::Error::Rejected);
        }
        let mut pos = self.opening();
        for &action in &line.0 {
            pos.next(self, action)?;
        }
        Ok(pos)
    }
    /// Every legal decision in this public position. The menu contains no
    /// scoring advice and never reveals the outcome of an unplayed move.
    ///
    /// # Errors
    /// Rejects an illegal prefix or invalid puzzle.
    pub fn choices(&self, line: &Line) -> Result<Vec<Move>, crate::Error> {
        let pos = self.position(line)?;
        Ok(if pos.result.is_some() {
            Vec::new()
        } else {
            pos.board.legal(self.steps[pos.step].phase)
        })
    }
    /// Check whether this exact legal line reached the stated goal.
    ///
    /// # Errors
    /// Rejects illegal or oversized lines; an unfinished legal line simply
    /// has not won.
    pub fn won(&self, line: &Line) -> Result<bool, crate::Error> {
        Ok(self.position(line)?.result == Some(true))
    }
}
