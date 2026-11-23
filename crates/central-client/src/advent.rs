//! Native seasonal account operations and verified transfer of earned art.
use crate::{Error, Session, Transport};
use central_api::{
    advent as api,
    wire::{Request, Response},
};

fn answers(req: &api::Request, out: &api::Response) -> bool {
    match (req, out) {
        (
            api::Request::Calendar { .. } | api::Request::CurrentCalendar {},
            api::Response::Calendar {
                year: b,
                starts_at,
                closes_at,
                doors,
                hero,
                ..
            },
        ) => {
            !matches!(req,api::Request::Calendar{year} if year!=b)
                && starts_at < closes_at
                && doors.len() == usize::from(api::DOORS)
                && doors
                    .iter()
                    .enumerate()
                    .all(|(i, d)| usize::from(d.door) == i + 1)
                && hero.as_ref().is_none_or(|m| m.validate().is_ok())
        }
        (
            api::Request::Puzzle { year: a, door: x },
            api::Response::Puzzle {
                year: b,
                door: y,
                puzzle,
            },
        ) => a == b && x == y && puzzle.validate().is_ok(),
        (api::Request::Hint { number: n, .. }, api::Response::Hint { number: m, text }) => {
            n == m && text.en.len() <= 512 && text.de.len() <= 512
        }
        (api::Request::CurrentCalendar {}, api::Response::NoCalendar {})
        | (api::Request::Submit { .. }, api::Response::Submission { .. })
        | (api::Request::Table { .. }, api::Response::Table { .. })
        | (api::Request::Present { .. }, api::Response::Presented) => true,
        (api::Request::Claim { year, door }, api::Response::Reward { reward }) => {
            reward.year == *year && reward.door == *door && reward.asset.validate().is_ok()
        }
        (api::Request::Inventory { .. }, api::Response::Inventory { items, .. }) => {
            items.len() <= 32 && items.iter().all(|r| r.asset.validate().is_ok())
        }
        (api::Request::Collection { after }, api::Response::Collection { items, next }) => {
            items.len() <= 32
                && items.iter().all(|item| item.asset.validate().is_ok())
                && items.windows(2).all(|pair| pair[0].id < pair[1].id)
                && after
                    .zip(items.first())
                    .is_none_or(|(a, first)| a < first.id)
                && next.is_none_or(|n| {
                    items.len() == 32 && items.last().is_some_and(|last| last.id == n)
                })
        }
        (api::Request::TableLooks { .. }, api::Response::TableLooks { seats }) => seats
            .iter()
            .flat_map(|s| [s.sleeve, s.playmat])
            .flatten()
            .all(|m| m.validate().is_ok()),
        (api::Request::Asset { offset, .. }, api::Response::Asset { page }) => {
            page.offset == *offset
                && page.asset.validate().is_ok()
                && *offset < page.asset.size
                && page.bytes.len() == (page.asset.size - offset).min(api::PAGE_BYTES) as usize
        }
        (
            api::Request::Asset {
                offset: 0,
                cached: Some(d),
                ..
            },
            api::Response::Cached { asset },
        ) => asset.digest == *d && asset.validate().is_ok(),
        _ => false,
    }
}
impl Session {
    /// Use the authenticated account's configured Central.
    ///
    /// # Errors
    /// Rejects a different endpoint, authorization failure or inconsistent response.
    pub fn advent(
        &self,
        transport: &impl Transport,
        request: api::Request,
    ) -> Result<api::Response, Error> {
        self.same_endpoint(transport)?;
        let wire = Request::Advent { request };
        let reply = transport.exchange(&wire, Some((&self.token).into()))?;
        match (wire, reply) {
            (Request::Advent { request }, Response::Advent { response })
                if answers(&request, &response) =>
            {
                Ok(response)
            }
            _ => Err(Error::Rejected),
        }
    }

    /// Reassemble an authorized immutable image and verify its SHA-256 before use.
    /// A confirmed cache hit returns its metadata without allocating a second image.
    ///
    /// # Errors
    /// Rejects lost authorization, oversized declarations, changed content or broken ranges.
    pub fn advent_asset(
        &self,
        transport: &impl Transport,
        scope: api::AssetScope,
        cached: Option<api::Digest>,
    ) -> Result<(api::AssetMeta, Option<Vec<u8>>), Error> {
        read_asset(scope, cached, |request| self.advent(transport, request))
    }
}

/// Reassemble private art while the caller schedules each authorized page independently.
/// A UI can interleave these requests with time-sensitive game operations.
///
/// # Errors
/// Rejects invalid ranges, cache responses, changing metadata or incomplete content.
pub fn read_asset(
    scope: api::AssetScope,
    cached: Option<api::Digest>,
    mut request: impl FnMut(api::Request) -> Result<api::Response, Error>,
) -> Result<(api::AssetMeta, Option<Vec<u8>>), Error> {
    let mut query = api::Request::Asset {
        scope,
        offset: 0,
        cached,
    };
    let mut metadata = None;
    let mut bytes = Vec::new();
    loop {
        let reply = request(query.clone())?;
        if !answers(&query, &reply) {
            return Err(Error::Rejected);
        }
        let page = match reply {
            api::Response::Asset { page } => page,
            api::Response::Cached { asset } if bytes.is_empty() => return Ok((asset, None)),
            _ => return Err(Error::Rejected),
        };
        if metadata.is_some_and(|m| m != page.asset) {
            return Err(Error::Rejected);
        }
        if metadata.is_none() {
            metadata = Some(page.asset);
            bytes
                .try_reserve_exact(page.asset.size as usize)
                .map_err(|_| Error::Unavailable)?;
        }
        bytes.extend_from_slice(&page.bytes);
        let offset = u32::try_from(bytes.len()).map_err(|_| Error::Rejected)?;
        if offset == page.asset.size {
            page.asset.verify(&bytes)?;
            return Ok((page.asset, Some(bytes)));
        }
        query = api::Request::Asset {
            scope,
            offset,
            cached: None,
        };
    }
}

#[cfg(test)]
mod tests;
