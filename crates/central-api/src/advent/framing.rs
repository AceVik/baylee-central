//! Optional composition of an authorized playmat, in ten-thousandths of its original image.
use crate::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use std::fmt;

/// Native HTTP clients that support artwork composition send this header with value `1`.
pub const CAPABILITY_HEADER: &str = "baylee-mat-framing";
/// Maximum number of exceptional table layouts attached to one image.
pub const MAX_VARIANTS: usize = 8;
/// Inclusive right and bottom edges in source image coordinates.
pub const EDGE: u16 = 10_000;

/// Top-left image origin. A renderer preserves the entire safe rectangle when framing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Coordinates")]
pub struct Framing {
    /// Point of interest in source coordinates.
    pub focus: [u16; 2],
    /// Optional left, top, right and bottom content bounds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub safe: Option<[u16; 4]>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Coordinates {
    focus: [u16; 2],
    safe: Option<[u16; 4]>,
}

impl TryFrom<Coordinates> for Framing {
    type Error = &'static str;
    fn try_from(raw: Coordinates) -> Result<Self, Self::Error> {
        let value = Self {
            focus: raw.focus,
            safe: raw.safe,
        };
        value
            .valid()
            .then_some(value)
            .ok_or("invalid playmat coordinates")
    }
}

impl Default for Framing {
    fn default() -> Self {
        Self {
            focus: [EDGE / 2; 2],
            safe: None,
        }
    }
}

impl Framing {
    fn valid(self) -> bool {
        let [x, y] = self.focus;
        x <= EDGE
            && y <= EDGE
            && self.safe.is_none_or(|[l, t, r, b]| {
                l < r
                    && t < b
                    && r <= EDGE
                    && b <= EDGE
                    && (l..=r).contains(&x)
                    && (t..=b).contains(&y)
            })
    }
}

/// An exception for a table from the point of view of the mat's owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatVariant {
    /// Number of participating seats.
    pub seats: u8,
    /// Size of the owner's team, including the owner.
    pub allies: u8,
    /// Total number of teams, counting each unallied player separately.
    pub sides: u8,
    /// Composition for this exact layout.
    pub framing: Framing,
}

impl MatVariant {
    fn key(self) -> (u8, u8, u8) {
        (self.seats, self.allies, self.sides)
    }
    fn valid(self) -> bool {
        (3..=8).contains(&self.seats)
            && (2..=self.seats).contains(&self.sides)
            && (1..=self.seats - self.sides + 1).contains(&self.allies)
            && self.framing.valid()
    }
}

/// A canonical, bounded set. JSON is a compact array; source order has no meaning.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MatVariants([Option<MatVariant>; MAX_VARIANTS]);

impl MatVariants {
    /// Yield populated layouts in stable key order.
    pub fn iter(&self) -> impl Iterator<Item = &MatVariant> {
        self.0.iter().flatten()
    }

    fn insert(&mut self, variant: MatVariant) -> Result<(), Error> {
        if !variant.valid() || self.0[MAX_VARIANTS - 1].is_some() {
            return Err(Error::Rejected);
        }
        let i = self
            .0
            .partition_point(|v| v.is_some_and(|v| v.key() < variant.key()));
        if self.0[i].is_some_and(|v| v.key() == variant.key()) {
            return Err(Error::Rejected);
        }
        self.0[i..].rotate_right(1);
        self.0[i] = Some(variant);
        Ok(())
    }

    fn empty(&self) -> bool {
        self.0[0].is_none()
    }
}

impl TryFrom<&[MatVariant]> for MatVariants {
    type Error = Error;
    fn try_from(items: &[MatVariant]) -> Result<Self, Error> {
        let mut result = Self::default();
        for &item in items {
            result.insert(item)?;
        }
        Ok(result)
    }
}

impl Serialize for MatVariants {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_seq(self.iter())
    }
}

impl<'de> Deserialize<'de> for MatVariants {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Bounded;
        impl<'de> de::Visitor<'de> for Bounded {
            type Value = MatVariants;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("up to eight distinct table layouts")
            }
            fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut result = MatVariants::default();
                while let Some(item) = seq.next_element()? {
                    result
                        .insert(item)
                        .map_err(|_| de::Error::custom("invalid table layout"))?;
                }
                Ok(result)
            }
        }
        d.deserialize_seq(Bounded)
    }
}

/// Original duel composition, inherited by any unlisted layout.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MatFraming {
    /// Composition of the original one-on-one image.
    pub duel: Framing,
    /// Optional exact layout overrides.
    #[serde(default, skip_serializing_if = "MatVariants::empty")]
    pub variants: MatVariants,
}

impl MatFraming {
    /// Resolve a table relative to this mat's owner, falling back to its original composition.
    #[must_use]
    pub fn for_layout(&self, seats: u8, allies: u8, sides: u8) -> Framing {
        self.variants
            .iter()
            .find(|v| v.key() == (seats, allies, sides))
            .map_or(self.duel, |v| v.framing)
    }
    /// Validate before using image coordinates or provisioning private art.
    ///
    /// # Errors
    /// Returns a fixed rejection for malformed coordinates.
    pub fn validate(&self) -> Result<(), Error> {
        if self.duel.valid() {
            Ok(())
        } else {
            Err(Error::Rejected)
        }
    }
}

#[cfg(test)]
mod tests;
