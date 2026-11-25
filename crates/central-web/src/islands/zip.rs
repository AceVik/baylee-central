//! Client-side ZIP archive builder (`baylee-recovery-code.txt`) for
//! recovery-code email backups, supporting plain PKZIP STORED archives and
//! `WinZip` AE-2 (AES-256 CTR + HMAC-SHA1-80) password encryption using browser
//! `WebCrypto` (`SubtleCrypto`).

use super::b64;

const ENTRY_NAME: &[u8] = b"baylee-recovery-code.txt";

/// Compute the IEEE CRC-32 checksum over `bytes`.
#[must_use]
pub fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFF_u32;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

/// Build an uncompressed (`STORED`) single-file ZIP archive containing `content`
/// as `baylee-recovery-code.txt`.
#[must_use]
pub fn build_plain_zip(content: &[u8]) -> Vec<u8> {
    let crc = crc32(content);
    assemble_zip(0, 20, 0x0800, crc, &[], content)
}

/// Wrap an already-constructed `WinZip` AE-2 encrypted stream (`salt || pvv ||
/// ciphertext || tag`) into a single-file PKZIP container.
#[cfg_attr(all(not(target_arch = "wasm32"), not(test)), allow(dead_code))]
#[must_use]
pub fn build_ae2_container(payload: &[u8]) -> Vec<u8> {
    // WinZip AE-2 extra field (0x9901):
    // tag(2)=0x9901, len(2)=7, version(2)=2 (AE-2), vendor(2)="AE", strength(1)=3 (AES-256), actual_method(2)=0 (STORED)
    let extra: [u8; 11] = [
        0x01, 0x99, 0x07, 0x00, 0x02, 0x00, 0x41, 0x45, 0x03, 0x00, 0x00,
    ];
    assemble_zip(99, 51, 0x0801, 0, &extra, payload)
}

fn assemble_zip(
    method: u16,
    version_needed: u16,
    flags: u16,
    crc: u32,
    extra: &[u8],
    payload: &[u8],
) -> Vec<u8> {
    let name_len = u16::try_from(ENTRY_NAME.len()).unwrap_or(0);
    let extra_len = u16::try_from(extra.len()).unwrap_or(0);
    let comp_size = u32::try_from(payload.len()).unwrap_or(u32::MAX);
    // WinZip AE-2 mandates CRC=0 and stores total encrypted payload size as comp_size,
    // while uncompressed_size holds plain length (optional; setting 0 or comp_size for AE-2
    // is widely accepted — for STORED AE-2, setting uncomp_size=comp_size when method==99
    // works or plain length if provided; keep deterministic 0 for AE-2? Actually WinZip AE-2
    // specifies storing uncompressed size for AE-1/AE-2 or payload length). We pass caller-friendly
    // STORED size here via comp_size for plain and 0..payload for AE-2 below.
    let uncomp_size = if method == 99 {
        comp_size.saturating_sub(28)
    } else {
        comp_size
    };

    let mut out = Vec::with_capacity(128 + payload.len() + extra.len() * 2);

    // Local File Header (30 bytes + name + extra)
    out.extend_from_slice(&0x0403_4B50_u32.to_le_bytes());
    out.extend_from_slice(&version_needed.to_le_bytes());
    out.extend_from_slice(&flags.to_le_bytes());
    out.extend_from_slice(&method.to_le_bytes());
    out.extend_from_slice(&0_u16.to_le_bytes()); // mod time
    out.extend_from_slice(&0x5421_u16.to_le_bytes()); // mod date (2022-01-01)
    out.extend_from_slice(&crc.to_le_bytes());
    out.extend_from_slice(&comp_size.to_le_bytes());
    out.extend_from_slice(&uncomp_size.to_le_bytes());
    out.extend_from_slice(&name_len.to_le_bytes());
    out.extend_from_slice(&extra_len.to_le_bytes());
    out.extend_from_slice(ENTRY_NAME);
    out.extend_from_slice(extra);
    out.extend_from_slice(payload);

    let cd_offset = u32::try_from(out.len()).unwrap_or(u32::MAX);

    // Central Directory Header (46 bytes + name + extra)
    out.extend_from_slice(&0x0201_4B50_u32.to_le_bytes());
    out.extend_from_slice(&version_needed.to_le_bytes()); // version made by
    out.extend_from_slice(&version_needed.to_le_bytes());
    out.extend_from_slice(&flags.to_le_bytes());
    out.extend_from_slice(&method.to_le_bytes());
    out.extend_from_slice(&0_u16.to_le_bytes());
    out.extend_from_slice(&0x5421_u16.to_le_bytes());
    out.extend_from_slice(&crc.to_le_bytes());
    out.extend_from_slice(&comp_size.to_le_bytes());
    out.extend_from_slice(&uncomp_size.to_le_bytes());
    out.extend_from_slice(&name_len.to_le_bytes());
    out.extend_from_slice(&extra_len.to_le_bytes());
    out.extend_from_slice(&0_u16.to_le_bytes()); // comment len
    out.extend_from_slice(&0_u16.to_le_bytes()); // disk start
    out.extend_from_slice(&0_u16.to_le_bytes()); // internal attrs
    out.extend_from_slice(&0_u32.to_le_bytes()); // external attrs
    out.extend_from_slice(&0_u32.to_le_bytes()); // local header offset
    out.extend_from_slice(ENTRY_NAME);
    out.extend_from_slice(extra);

    let cd_size = u32::try_from(out.len())
        .unwrap_or(u32::MAX)
        .saturating_sub(cd_offset);

    // End of Central Directory (22 bytes)
    out.extend_from_slice(&0x0605_4B50_u32.to_le_bytes());
    out.extend_from_slice(&0_u16.to_le_bytes());
    out.extend_from_slice(&0_u16.to_le_bytes());
    out.extend_from_slice(&1_u16.to_le_bytes());
    out.extend_from_slice(&1_u16.to_le_bytes());
    out.extend_from_slice(&cd_size.to_le_bytes());
    out.extend_from_slice(&cd_offset.to_le_bytes());
    out.extend_from_slice(&0_u16.to_le_bytes());

    out
}

/// Build `baylee-recovery-code.zip` encoded as base64 and report whether it was
/// encrypted (`true`) or stored plain (`false`).
#[cfg(target_arch = "wasm32")]
pub async fn recovery_zip_base64(content: &str, password: &str) -> Result<(String, bool), ()> {
    let plain = content.as_bytes();
    if password.is_empty() {
        let zip = build_plain_zip(plain);
        return Ok((b64::encode(&zip), false));
    }
    let payload = ae2_encrypt(plain, password.as_bytes()).await?;
    let zip = build_ae2_container(&payload);
    Ok((b64::encode(&zip), true))
}

/// Non-browser builds only synthesize unencrypted ZIP archives synchronously.
#[cfg(not(target_arch = "wasm32"))]
#[allow(clippy::unused_async)]
pub async fn recovery_zip_base64(content: &str, _password: &str) -> Result<(String, bool), ()> {
    let zip = build_plain_zip(content.as_bytes());
    Ok((b64::encode(&zip), false))
}

#[cfg(target_arch = "wasm32")]
async fn ae2_encrypt(plain: &[u8], password: &[u8]) -> Result<Vec<u8>, ()> {
    ae2::encrypt(plain, password).await
}

#[cfg(target_arch = "wasm32")]
mod ae2 {
    use wasm_bindgen::{JsCast, JsValue};
    use wasm_bindgen_futures::JsFuture;

    fn subtle_api() -> Result<JsValue, ()> {
        let crypto = js_sys::Reflect::get(&js_sys::global(), &JsValue::from_str("crypto"))
            .map_err(|_| ())?;
        js_sys::Reflect::get(&crypto, &JsValue::from_str("subtle")).map_err(|_| ())
    }

    fn subtle_fn(subtle: &JsValue, name: &str) -> Result<js_sys::Function, ()> {
        js_sys::Reflect::get(subtle, &JsValue::from_str(name))
            .map_err(|_| ())?
            .dyn_into()
            .map_err(|_| ())
    }

    fn js_obj(pairs: &[(&str, JsValue)]) -> js_sys::Object {
        let obj = js_sys::Object::new();
        for (k, v) in pairs {
            let _ = js_sys::Reflect::set(&obj, &JsValue::from_str(k), v);
        }
        obj
    }

    async fn await_buf(ret: Result<JsValue, JsValue>) -> Result<Vec<u8>, ()> {
        let promise: js_sys::Promise = ret.map_err(|_| ())?.dyn_into().map_err(|_| ())?;
        let buf = JsFuture::from(promise).await.map_err(|_| ())?;
        Ok(js_sys::Uint8Array::new(&buf).to_vec())
    }

    async fn derive_keys(
        subtle: &JsValue,
        import_key: &js_sys::Function,
        salt: &[u8; 16],
        password: &[u8],
    ) -> Result<Vec<u8>, ()> {
        let derive_bits = subtle_fn(subtle, "deriveBits")?;
        let pw_arr = js_sys::Uint8Array::from(password);
        let usages = js_sys::Array::of1(&JsValue::from_str("deriveBits"));
        let pbkdf2_alg = js_obj(&[("name", JsValue::from_str("PBKDF2"))]);
        let pw_promise: js_sys::Promise = import_key
            .call5(
                subtle,
                &JsValue::from_str("raw"),
                &pw_arr,
                &pbkdf2_alg,
                &JsValue::FALSE,
                &usages,
            )
            .map_err(|_| ())?
            .dyn_into()
            .map_err(|_| ())?;
        let pw_key = JsFuture::from(pw_promise).await.map_err(|_| ())?;

        let salt_arr = js_sys::Uint8Array::from(&salt[..]);
        let derive_params = js_obj(&[
            ("name", JsValue::from_str("PBKDF2")),
            ("salt", salt_arr.into()),
            ("iterations", JsValue::from_f64(1000.0)),
            ("hash", JsValue::from_str("SHA-1")),
        ]);
        let derived = await_buf(derive_bits.call3(
            subtle,
            &derive_params,
            &pw_key,
            &JsValue::from_f64(528.0),
        ))
        .await?;
        if derived.len() != 66 {
            return Err(());
        }
        Ok(derived)
    }

    async fn encrypt_blocks(
        subtle: &JsValue,
        import_key: &js_sys::Function,
        aes_bytes: &[u8],
        plain: &[u8],
    ) -> Result<Vec<u8>, ()> {
        let encrypt_fn = subtle_fn(subtle, "encrypt")?;
        let aes_raw = js_sys::Uint8Array::from(aes_bytes);
        let aes_alg = js_obj(&[("name", JsValue::from_str("AES-CTR"))]);
        let enc_usages = js_sys::Array::of1(&JsValue::from_str("encrypt"));
        let aes_promise: js_sys::Promise = import_key
            .call5(
                subtle,
                &JsValue::from_str("raw"),
                &aes_raw,
                &aes_alg,
                &JsValue::FALSE,
                &enc_usages,
            )
            .map_err(|_| ())?
            .dyn_into()
            .map_err(|_| ())?;
        let aes_key = JsFuture::from(aes_promise).await.map_err(|_| ())?;

        let mut ciphertext = Vec::with_capacity(plain.len());
        for (idx, chunk) in plain.chunks(16).enumerate() {
            let mut counter = [0_u8; 16];
            let block_num = (idx as u128) + 1;
            counter.copy_from_slice(&block_num.to_le_bytes());
            let ctr_arr = js_sys::Uint8Array::from(&counter[..]);
            let ctr_params = js_obj(&[
                ("name", JsValue::from_str("AES-CTR")),
                ("counter", ctr_arr.into()),
                ("length", JsValue::from_f64(64.0)),
            ]);
            let chunk_arr = js_sys::Uint8Array::from(chunk);
            let block =
                await_buf(encrypt_fn.call3(subtle, &ctr_params, &aes_key, &chunk_arr)).await?;
            ciphertext.extend_from_slice(&block);
        }
        Ok(ciphertext)
    }

    async fn hmac_tag(
        subtle: &JsValue,
        import_key: &js_sys::Function,
        hmac_bytes: &[u8],
        ciphertext: &[u8],
    ) -> Result<Vec<u8>, ()> {
        let sign_fn = subtle_fn(subtle, "sign")?;
        let hmac_raw = js_sys::Uint8Array::from(hmac_bytes);
        let hmac_alg = js_obj(&[
            ("name", JsValue::from_str("HMAC")),
            ("hash", JsValue::from_str("SHA-1")),
        ]);
        let sign_usages = js_sys::Array::of1(&JsValue::from_str("sign"));
        let hmac_promise: js_sys::Promise = import_key
            .call5(
                subtle,
                &JsValue::from_str("raw"),
                &hmac_raw,
                &hmac_alg,
                &JsValue::FALSE,
                &sign_usages,
            )
            .map_err(|_| ())?
            .dyn_into()
            .map_err(|_| ())?;
        let hmac_key = JsFuture::from(hmac_promise).await.map_err(|_| ())?;
        let cipher_arr = js_sys::Uint8Array::from(ciphertext);
        let mac = await_buf(sign_fn.call3(subtle, &hmac_alg, &hmac_key, &cipher_arr)).await?;
        if mac.len() < 10 {
            return Err(());
        }
        Ok(mac)
    }

    pub(super) async fn encrypt(plain: &[u8], password: &[u8]) -> Result<Vec<u8>, ()> {
        let mut salt = [0_u8; 16];
        getrandom::fill(&mut salt).map_err(|_| ())?;
        let subtle = subtle_api()?;
        let import_key = subtle_fn(&subtle, "importKey")?;
        let derived = derive_keys(&subtle, &import_key, &salt, password).await?;
        let ciphertext = encrypt_blocks(&subtle, &import_key, &derived[0..32], plain).await?;
        let mac = hmac_tag(&subtle, &import_key, &derived[32..64], &ciphertext).await?;

        let mut payload = Vec::with_capacity(16 + 2 + ciphertext.len() + 10);
        payload.extend_from_slice(&salt);
        payload.extend_from_slice(&derived[64..66]);
        payload.extend_from_slice(&ciphertext);
        payload.extend_from_slice(&mac[..10]);
        Ok(payload)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_matches_standard_vector() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn plain_zip_frames_and_validates() {
        let zip = build_plain_zip(b"BAYLEE-RECOVERY-TEST-CODE");
        assert!(zip.starts_with(b"PK\x03\x04"));
        let encoded = b64::encode(&zip);
        assert!(central_api::email::validate_backup_zip_base64(&encoded).is_ok());
    }

    #[test]
    fn ae2_container_includes_winzip_extra_header() {
        let fake_payload = vec![0xAA; 64];
        let zip = build_ae2_container(&fake_payload);
        assert!(zip.starts_with(b"PK\x03\x04"));
        assert!(zip.windows(4).any(|w| w == [0x01, 0x99, 0x07, 0x00]));
        assert!(zip.windows(2).any(|w| w == b"AE"));
    }
}
