//! Content and shape checks shared by publisher, authority and readers.

use super::{Manifest, NODES, Theme};
use crate::{
    Error,
    challenges::{Kind, Publication},
};

impl Theme {
    /// Bound public copy and require an unambiguous fixed-size upgrade graph.
    ///
    /// # Errors
    /// Non-weekly slot, unsafe/oversized text or impossible replacement counts.
    pub fn validate(&self) -> Result<(), Error> {
        self.period.validate()?;
        if self.period.kind != Kind::Weekly {
            return Err(Error::Rejected);
        }
        self.title.validate(100)?;
        for floor in &self.floors {
            floor.validate(100)?;
        }
        for upgrades in &self.upgrades {
            for upgrade in upgrades {
                upgrade.title.validate(100)?;
                upgrade.summary.validate(400)?;
                if !(1..=4).contains(&upgrade.copies) || upgrade.add == upgrade.remove {
                    return Err(Error::Rejected);
                }
                for name in [&upgrade.add, &upgrade.remove] {
                    if name.trim().is_empty()
                        || name.len() > 200
                        || !name.chars().all(crate::gateway::shown_char)
                    {
                        return Err(Error::Rejected);
                    }
                }
            }
        }
        Ok(())
    }
}

impl Manifest {
    /// Check the full graph's immutable description and digest inventory.
    ///
    /// # Errors
    /// Missing/duplicate/zero nodes or invalid theme.
    pub fn validate(&self) -> Result<(), Error> {
        self.theme.validate()?;
        let mut unique = self.nodes.clone();
        unique.sort_unstable();
        unique.dedup();
        if self.nodes.len() != NODES || unique.len() != NODES || unique.contains(&[0; 32]) {
            return Err(Error::Rejected);
        }
        Ok(())
    }

    /// Bind one certified encounter to its fixed slot. No arbitrary caller edges exist.
    ///
    /// # Errors
    /// Wrong node, content, period or insufficient positive execution evidence.
    pub fn validate_node(&self, node: u8, publication: &Publication) -> Result<(), Error> {
        self.validate()?;
        publication.validate()?;
        if self.nodes.get(usize::from(node)) != Some(&publication.digest)
            || publication.description.period != self.theme.period
        {
            return Err(Error::Rejected);
        }
        Ok(())
    }
}
