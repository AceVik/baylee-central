//! Project authorized responses onto the original strict JSON wire for older transports.
use super::{AssetMeta, Response, table};

fn omit(asset: &mut Option<AssetMeta>) {
    if let Some(asset) = asset {
        asset.mat_framing = None;
    }
}

impl table::Worn {
    /// Keep both selected surfaces compatible with a client using the original strict schema.
    pub fn omit_framing(&mut self) {
        omit(&mut self.event_sleeve);
        omit(&mut self.event_playmat);
    }
}

impl table::HostResponse {
    /// Preserve the original wire shape when the transport did not negotiate framing.
    pub fn omit_framing(&mut self) {
        match self {
            Self::Confirmed { worn, .. } => worn.omit_framing(),
            Self::Asset { page } => page.asset.mat_framing = None,
            Self::Cached { asset } => asset.mat_framing = None,
            Self::Closed => {}
        }
    }
}

impl Response {
    /// Preserve the original wire shape when the transport did not negotiate framing.
    pub fn omit_framing(&mut self) {
        match self {
            Self::TableLooks { seats } => {
                for seat in seats.iter_mut() {
                    omit(&mut seat.sleeve);
                    omit(&mut seat.playmat);
                }
            }
            Self::Calendar { hero, .. } => omit(hero),
            Self::Reward { reward } => reward.asset.mat_framing = None,
            Self::Inventory { items, .. } => {
                for item in items {
                    item.asset.mat_framing = None;
                }
            }
            Self::Collection { items, .. } => {
                for item in items {
                    item.asset.mat_framing = None;
                }
            }
            Self::Asset { page } => page.asset.mat_framing = None,
            Self::Cached { asset } => asset.mat_framing = None,
            Self::Puzzle { .. }
            | Self::Hint { .. }
            | Self::NoCalendar { .. }
            | Self::Table { .. }
            | Self::Presented
            | Self::Submission { .. } => {}
        }
    }
}
