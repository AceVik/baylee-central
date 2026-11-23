//! Bounded private authoring study. Online claims use linear replay only.
use super::{Line, Position, Puzzle};

/// Exact counts over the displayed decision tree. A win before the final
/// scripted step is a complete line. Neutralized or interchangeable card
/// copies never create distinct choices.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Study {
    /// All reachable decision and terminal nodes.
    pub nodes: u32,
    /// Winning terminal lines.
    pub wins: u32,
    /// Unsuccessful terminal lines.
    pub losses: u32,
    /// Smallest number of decisions in a winning line.
    pub shortest: Option<usize>,
    /// One real winning line for a private operator's verification.
    pub witness: Option<Line>,
}
impl Puzzle {
    /// Exhaustively explore every legal choice in the bounded public board.
    /// Use during private campaign authoring or import, outside request paths.
    ///
    /// # Errors
    /// Rejects invalid puzzles or a computation exceeding the caller's work
    /// budget. A partial enumeration never counts as a certificate.
    pub fn study(&self, budget: u32) -> Result<Study, crate::Error> {
        self.validate()?;
        let mut out = Study::default();
        explore(
            self,
            &self.opening(),
            &mut Line::default(),
            &mut out,
            budget,
        )?;
        Ok(out)
    }
}
fn explore(
    p: &Puzzle,
    pos: &Position,
    line: &mut Line,
    out: &mut Study,
    budget: u32,
) -> Result<(), crate::Error> {
    if out.nodes >= budget {
        return Err(crate::Error::Busy);
    }
    out.nodes += 1;
    match pos.result {
        Some(true) => {
            out.wins += 1;
            out.shortest = Some(out.shortest.map_or(line.0.len(), |v| v.min(line.0.len())));
            if out.witness.is_none() {
                out.witness = Some(line.clone());
            }
        }
        Some(false) => out.losses += 1,
        None => {
            for choice in pos.board.legal(p.steps[pos.step].phase) {
                let mut next = pos.clone();
                next.next(p, choice)?;
                line.0.push(choice);
                explore(p, &next, line, out, budget)?;
                line.0.pop();
            }
        }
    }
    Ok(())
}
