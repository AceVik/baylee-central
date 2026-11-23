//! Authorized, bounded immutable ranges. No URL, filesystem path, MIME type
//! or owner identity in a request can select content.
use super::{
    DatabaseTransaction, Error, QueryResult, Uuid, api, col, inventory, looks, statement,
    unavailable, uuid,
};
use sea_orm::ConnectionTrait;

pub(super) fn meta(row: &QueryResult) -> Result<api::AssetMeta, Error> {
    let mime = match col::<String>(row, "mime")?.as_str() {
        "image/webp" => api::Mime::Webp,
        _ => return Err(Error::Unavailable),
    };
    let tier = match col::<String>(row, "tier")?.as_str() {
        "standard" => api::Finish::Standard,
        "special" => api::Finish::Special,
        "finale" => api::Finish::Finale,
        _ => return Err(Error::Unavailable),
    };
    let m = api::AssetMeta {
        digest: col::<Vec<u8>>(row, "digest")?
            .try_into()
            .map_err(unavailable)?,
        size: u32::try_from(col::<i32>(row, "size")?).map_err(unavailable)?,
        mime,
        tier,
        width: u16::try_from(col::<i32>(row, "width")?).map_err(unavailable)?,
        height: u16::try_from(col::<i32>(row, "height")?).map_err(unavailable)?,
    };
    m.validate().map_err(unavailable)?;
    Ok(m)
}
pub(super) async fn metadata(db: &DatabaseTransaction, id: Uuid) -> Result<api::AssetMeta, Error> {
    let row=db.query_one_raw(statement("SELECT digest,size,mime,width,height,tier FROM central_identity.advent_art WHERE id=$1",vec![id.into()]))
        .await.map_err(unavailable)?.ok_or(Error::Rejected)?;
    meta(&row)
}
pub(in crate::accounts) async fn page(
    db: &DatabaseTransaction,
    id: Uuid,
    offset: u32,
    cached: Option<api::Digest>,
) -> Result<api::Response, Error> {
    if !offset.is_multiple_of(api::PAGE_BYTES) || (offset != 0 && cached.is_some()) {
        return Err(Error::Rejected);
    }
    let asset = metadata(db, id).await?;
    if offset >= asset.size {
        return Err(Error::Rejected);
    }
    if cached == Some(asset.digest) {
        return Ok(api::Response::Cached { asset });
    }
    let len = api::PAGE_BYTES.min(asset.size - offset);
    let row=db.query_one_raw(statement("SELECT substring(content FROM $2 FOR $3) AS bytes FROM central_identity.advent_art WHERE id=$1",
        vec![id.into(),i32::try_from(offset+1).map_err(unavailable)?.into(),i32::try_from(len).map_err(unavailable)?.into()])).await.map_err(unavailable)?.ok_or(Error::Rejected)?;
    let bytes: Vec<u8> = col(&row, "bytes")?;
    if bytes.len() != usize::try_from(len).map_err(unavailable)? {
        return Err(Error::Unavailable);
    }
    Ok(api::Response::Asset {
        page: api::AssetPage {
            asset,
            offset,
            bytes,
        },
    })
}
pub(super) async fn get(
    db: &DatabaseTransaction,
    me: Uuid,
    scope: api::AssetScope,
    offset: u32,
    cached: Option<api::Digest>,
) -> Result<api::Response, Error> {
    let id = match scope {
        api::AssetScope::Calendar { year } => {
            let r=db.query_one_raw(statement("SELECT hero_id FROM central_identity.advent_campaigns
                WHERE year=$1 AND statement_timestamp()>=starts_at AND statement_timestamp()<closes_at",
                vec![i16::try_from(year).map_err(|_|Error::Rejected)?.into()])).await.map_err(unavailable)?.ok_or(Error::Rejected)?;
            col(&r, "hero_id")?
        }
        api::AssetScope::Owned { item } => {
            let id = uuid(item);
            inventory::owned(db, me, id, None).await?;
            id
        }
        api::AssetScope::Equipped {
            match_id,
            seat,
            kind,
        } => looks::item(db, me, uuid(match_id), seat, kind).await?,
    };
    page(db, id, offset, cached).await
}
