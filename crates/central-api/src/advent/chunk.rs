//! The existing bounded lowercase-hex wire encoding avoids decimal JSON
//! number arrays while keeping the in-process byte representation unchanged.
use crate::private_bytes::PrivateBytes;
use serde::{Deserialize, Deserializer, Serializer};

type Chunk = PrivateBytes<{ super::PAGE_BYTES as usize }>;

pub(super) fn serialize<S: Serializer>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
    Chunk::serialize_slice(bytes, serializer)
}

pub(super) fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
    Ok(Chunk::deserialize(deserializer)?.into_vec())
}

#[cfg(test)]
mod tests {
    use super::super::{AssetMeta, AssetPage, Finish, Mime, PAGE_BYTES};

    #[test]
    fn maximum_page_round_trips_in_two_characters_per_octet() {
        let bytes: Vec<_> = (0_u8..=255).cycle().take(PAGE_BYTES as usize).collect();
        let decimals = crate::encode(&bytes).unwrap().len();
        let page = AssetPage {
            asset: AssetMeta {
                tier: Finish::Standard,
                mime: Mime::Webp,
                digest: [0; 32],
                size: PAGE_BYTES,
                width: 1,
                height: 1,
            },
            offset: 0,
            bytes,
        };
        let wire = crate::encode(&page).unwrap();
        assert!(wire.len() < 2 * PAGE_BYTES as usize + 512);
        assert!(wire.len() * 10 < decimals * 6);
        assert_eq!(crate::decode::<AssetPage>(&wire).unwrap(), page);
    }
}
