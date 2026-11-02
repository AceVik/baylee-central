//! Native-compatible signed bodies. Received text is data, never markup.
use super::{Chat, Error, devices, public, rejected, tag};
use central_api::chat::{Delivery, Header, Sealed, Wrap};
use central_crypto::chat::{Entropy, Envelope, Recipient, b64::url};
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;
use zeroize::Zeroizing;

/// The bounded plaintext a UI may display after all authentication checks.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Message {
    /// Signed sender-generated correlation ID.
    pub id: String,
    /// Signed conversation ID.
    pub conv: String,
    /// Numeric public sender tag.
    pub sender: u32,
    /// Text only; render through the UI's escaped text API.
    pub text: String,
    /// Whether the native client sent `/me`.
    pub emote: bool,
    /// Server acceptance time in milliseconds.
    pub at: u64,
}

// Unknown body fields are intentionally ignored: native links/mentions remain
// readable text in the website, without creating HTML or following supplied URLs.
#[derive(Serialize, Deserialize)]
struct Body {
    text: String,
    #[serde(default)]
    emote: bool,
}

pub(super) struct Random {
    bytes: Zeroizing<[u8; 128]>,
    used: usize,
}
impl Random {
    pub(super) fn new() -> Result<Self, Error> {
        let mut bytes = Zeroizing::new([0; 128]);
        getrandom::fill(bytes.as_mut()).map_err(|_| Error::Unavailable)?;
        Ok(Self { bytes, used: 0 })
    }
}
impl Entropy for Random {
    fn fill(&mut self, out: &mut [u8]) {
        // The reviewed seal draws 88 bytes, UUID draws 10; at-rest draws 24.
        // Each operation owns a fresh pool. A future overdraw fails closed.
        out.copy_from_slice(&self.bytes[self.used..self.used + out.len()]);
        self.used += out.len();
    }
}

fn valid_text(text: &str) -> bool {
    !text.trim().is_empty()
        && text.len() <= 4000
        && text.chars().count() <= 1000
        && !text.chars().any(|c| {
            (c.is_control() && c != '\n' && c != '\t')
                || matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
        })
}

fn conversation(conv: &str, own: &str, sender: &str) -> Result<(), Error> {
    let parts = conv.split(':').collect::<Vec<_>>();
    match parts.as_slice() {
        ["dm", a, b]
            if tag(a)? < tag(b)? && (*a == own || *b == own) && (*a == sender || *b == sender) =>
        {
            Ok(())
        }
        // Group authorization is server-owned; a signed group UUID also binds the body.
        ["grp", id] if valid_uuid(id) => Ok(()),
        _ => Err(Error::Rejected),
    }
}

fn valid_uuid(id: &str) -> bool {
    let b = id.as_bytes();
    b.len() == 36
        && b[14] == b'7'
        && matches!(b[19], b'8' | b'9' | b'a' | b'b')
        && b.iter().enumerate().all(|(i, c)| {
            if [8, 13, 18, 23].contains(&i) {
                *c == b'-'
            } else {
                c.is_ascii_digit() || (b'a'..=b'f').contains(c)
            }
        })
}

fn uuid(now: u64, rng: &mut Random) -> String {
    let mut bytes = [0; 16];
    bytes[..6].copy_from_slice(&now.to_be_bytes()[2..]);
    rng.fill(&mut bytes[6..]);
    bytes[6] = 0x70 | (bytes[6] & 0xf);
    bytes[8] = 0x80 | (bytes[8] & 0x3f);
    let hex = bytes.iter().fold(String::with_capacity(32), |mut out, b| {
        let _ = write!(out, "{b:02x}");
        out
    });
    format!(
        "{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    )
}

impl Chat {
    /// Seal text for exactly the stated members, including the real own key.
    /// Reuse the returned envelope on uncertain delivery; never generate a retry.
    ///
    /// # Errors
    /// Refuses invalid bodies, missing/changed keys, ambiguous recipients or randomness failure.
    pub fn seal(
        &self,
        conv: &str,
        text: String,
        emote: bool,
        members: &[u32],
        now: u64,
    ) -> Result<Sealed, Error> {
        conversation(conv, &self.tag, &self.tag)?;
        if !valid_text(&text) || members.is_empty() || members.len() > 50 || now > 0xffff_ffff_ffff
        {
            return Err(Error::Rejected);
        }
        let body = Zeroizing::new(central_api::encode(&Body { text, emote })?);
        if body.len() > 4096 {
            return Err(Error::Rejected);
        }
        let mut peers = std::collections::BTreeSet::new();
        let mut keys = Vec::new();
        for member in members {
            let peer = format!("{member:04x}");
            tag(&peer)?;
            if !peers.insert(peer.clone()) {
                return Err(Error::Rejected);
            }
            if peer == self.tag {
                continue;
            }
            let current = self.current.get(&peer).ok_or(Error::Rejected)?;
            let accepted = self.accepted.get(&peer).ok_or(Error::Rejected)?;
            let checked = devices(current)?;
            if self.changed.contains(&peer) || devices(accepted)? != checked {
                return Err(Error::Conflict);
            }
            keys.extend(checked);
        }
        if conv.starts_with("dm:") {
            let expected = conv
                .split(':')
                .skip(1)
                .map(str::to_owned)
                .collect::<std::collections::BTreeSet<_>>();
            peers.insert(self.tag.clone());
            if peers != expected {
                return Err(Error::Rejected);
            }
        }
        keys.push(self.secrets.public(&self.tag).map_err(rejected)?);
        let recipients = keys
            .iter()
            .map(|key| Recipient {
                device: &key.id,
                dk: key.dk,
            })
            .collect::<Vec<_>>();
        let mut random = Random::new()?;
        let hdr = central_api::encode(&Header {
            v: 1,
            id: uuid(now, &mut random),
            conv: conv.into(),
            from: self.tag.clone(),
            dev: self.device().into(),
            ts: now,
            kind: "text".into(),
        })?;
        let sealed =
            central_crypto::chat::seal(&self.secrets, &hdr, &body, &recipients, &mut random)
                .map_err(rejected)?;
        Ok(Sealed {
            hdr: url::encode(&hdr),
            n: url::encode(&sealed.n),
            ct: url::encode(&sealed.ct),
            epk: url::encode(&sealed.epk),
            sig: url::encode(&sealed.sig),
            wraps: sealed
                .wraps
                .into_iter()
                .map(|wrap| Wrap {
                    device: wrap.device,
                    k: url::encode(&wrap.k),
                })
                .collect(),
        })
    }

    /// Authenticate a delivery against previously accepted keys before exposing text.
    /// Its embedded public key cannot authorize itself.
    ///
    /// # Errors
    /// Rejects forgery, substitution, malformed fields and unconfirmed sender identities.
    pub fn open(&self, delivery: &Delivery) -> Result<Message, Error> {
        let sealed = &delivery.sealed;
        if sealed.hdr.len() > 684 || sealed.ct.len() > 5526 || sealed.wraps.len() != 1 {
            return Err(Error::Rejected);
        }
        let hdr = url::decode(&sealed.hdr).map_err(rejected)?;
        let header: Header = central_api::decode(&hdr)?;
        if header.v != 1
            || header.kind != "text"
            || !valid_uuid(&header.id)
            || header.ts.abs_diff(delivery.at) > 600_000
            || tag(&header.from)? != delivery.sender.tag
            || header.dev != delivery.device.id
            || sealed.wraps[0].device != self.device()
        {
            return Err(Error::Rejected);
        }
        conversation(&header.conv, &self.tag, &header.from)?;
        let key = public(&delivery.device, &header.from)?;
        if header.from == self.tag {
            if key != self.secrets.public(&self.tag).map_err(rejected)? {
                return Err(Error::Rejected);
            }
        } else if !self
            .accepted
            .get(&header.from)
            .map(devices)
            .transpose()?
            .is_some_and(|keys| keys.contains(&key))
        {
            return Err(Error::Conflict);
        }
        let n = url::decode_array::<24>(&sealed.n).map_err(rejected)?;
        let ct = url::decode(&sealed.ct).map_err(rejected)?;
        let epk = url::decode_array::<32>(&sealed.epk).map_err(rejected)?;
        let sig = url::decode_array::<64>(&sealed.sig).map_err(rejected)?;
        let k = url::decode_array::<48>(&sealed.wraps[0].k).map_err(rejected)?;
        let opened = central_crypto::chat::open(
            &self.secrets,
            &key.ik,
            &Envelope {
                hdr: &hdr,
                n: &n,
                ct: &ct,
                epk: &epk,
                sig: &sig,
            },
            &k,
        )
        .map_err(rejected)?;
        let body: Body = central_api::decode(&opened.body)?;
        if !valid_text(&body.text) {
            return Err(Error::Rejected);
        }
        Ok(Message {
            id: header.id,
            conv: header.conv,
            sender: delivery.sender.tag,
            text: body.text,
            emote: body.emote,
            at: delivery.at,
        })
    }
}
