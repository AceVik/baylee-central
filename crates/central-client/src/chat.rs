//! Shared browser/native message validation and bounded, encrypted trust state.
//! First contact is TOFU. A changed key set blocks sends until the exact displayed
//! snapshot is confirmed. Public directories cannot add an own-account recipient.
use central_api::{
    Error,
    chat::{AccountKeys, DeviceKey, DeviceRegistration},
};
use central_crypto::chat::{DevicePublic, DeviceSecrets, RestKind, b64::url};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use zeroize::Zeroizing;

mod messages;
pub use messages::Message;

const MAX_PEERS: usize = 1024;
const MAX_TRUST_BYTES: usize = 1_000_000;
const TRUST_NAME: &str = "central-peer-trust/v1";

/// Public review information; the snapshot binds a confirmation to exactly these keys.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Review {
    /// Canonical hexadecimal tag.
    pub tag: String,
    /// Human-comparable 60-digit safety number.
    pub number: String,
    /// Known identity/encryption keys changed; sending is blocked.
    pub changed: bool,
    /// Exact public snapshot to return on explicit confirmation.
    pub snapshot: AccountKeys,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Trust {
    version: u32,
    keys: Vec<AccountKeys>,
    changed: BTreeSet<String>,
}

/// A chat identity and accepted peer keys. No `Debug`, cloning or secret export.
/// Drop this with the authenticated session (terminate its worker in the browser).
pub struct Chat {
    secrets: DeviceSecrets,
    tag: String,
    accepted: BTreeMap<String, AccountKeys>,
    current: BTreeMap<String, AccountKeys>,
    changed: BTreeSet<String>,
}

fn rejected(_: impl Sized) -> Error {
    Error::Rejected
}

fn tag(value: &str) -> Result<u32, Error> {
    let n = u32::from_str_radix(value, 16).map_err(rejected)?;
    if !central_api::profile::valid_tag(n) || format!("{n:04x}") != value {
        return Err(Error::Rejected);
    }
    Ok(n)
}

fn public(device: &DeviceKey, owner: &str) -> Result<DevicePublic, Error> {
    let p = DevicePublic {
        id: device.id.clone(),
        ik: url::decode_array(&device.ik).map_err(rejected)?,
        dk: url::decode_array(&device.dk).map_err(rejected)?,
        dsig: url::decode_array(&device.dsig).map_err(rejected)?,
    };
    p.verify(owner).map_err(rejected)?;
    Ok(p)
}

fn devices(keys: &AccountKeys) -> Result<Vec<DevicePublic>, Error> {
    tag(&keys.tag)?;
    if keys.devices.is_empty() || keys.devices.len() > central_api::chat::MAX_DEVICES {
        return Err(Error::Rejected);
    }
    let mut out = keys
        .devices
        .iter()
        .map(|key| public(key, &keys.tag))
        .collect::<Result<Vec<_>, _>>()?;
    out.sort_by(|a, b| a.id.cmp(&b.id));
    if out.windows(2).any(|pair| pair[0].id == pair[1].id) {
        return Err(Error::Rejected);
    }
    Ok(out)
}

impl Chat {
    /// Bind the derived account identity to its authenticated public tag.
    ///
    /// # Errors
    /// Rejects malformed public tags.
    pub fn new(secrets: DeviceSecrets, owner: u32) -> Result<Self, Error> {
        if !central_api::profile::valid_tag(owner) {
            return Err(Error::Rejected);
        }
        Ok(Self {
            secrets,
            tag: format!("{owner:04x}"),
            accepted: BTreeMap::new(),
            current: BTreeMap::new(),
            changed: BTreeSet::new(),
        })
    }

    /// The stable public device fingerprint, never a session credential.
    #[must_use]
    pub fn device(&self) -> &str {
        self.secrets.device_id()
    }

    /// The account-bound public registration. No machine-specific identifier.
    ///
    /// # Errors
    /// Rejects an invalid account binding.
    pub fn registration(&self) -> Result<DeviceRegistration, Error> {
        let p = self.secrets.public(&self.tag).map_err(rejected)?;
        Ok(DeviceRegistration {
            ik: url::encode(&p.ik),
            dk: url::encode(&p.dk),
            dsig: url::encode(&p.dsig),
            label: "Account".into(),
        })
    }

    /// Validate and observe one directory snapshot. First-use keys are accepted;
    /// changed keys remain pending. Persist `save_trust` before sending to a new peer.
    ///
    /// # Errors
    /// Refuses ambiguous/invalid keys, foreign own-account devices and capacity overflow.
    pub fn observe(&mut self, keys: AccountKeys) -> Result<Review, Error> {
        let checked = devices(&keys)?;
        if keys.tag == self.tag {
            if checked != vec![self.secrets.public(&self.tag).map_err(rejected)?] {
                return Err(Error::Rejected);
            }
        } else if !self.accepted.contains_key(&keys.tag) {
            if self.accepted.len() >= MAX_PEERS {
                return Err(Error::Busy);
            }
            self.accepted.insert(keys.tag.clone(), keys.clone());
        }
        if self
            .accepted
            .get(&keys.tag)
            .is_some_and(|old| devices(old).ok().as_ref() != Some(&checked))
        {
            self.changed.insert(keys.tag.clone());
        }
        let changed = self.changed.contains(&keys.tag);
        let number = central_crypto::chat::safety_number(
            (&self.tag, &[self.secrets.ik()]),
            (&keys.tag, &checked.iter().map(|p| p.ik).collect::<Vec<_>>()),
        );
        self.current.insert(keys.tag.clone(), keys.clone());
        Ok(Review {
            tag: keys.tag.clone(),
            number,
            changed,
            snapshot: keys,
        })
    }

    /// Confirm only the exact pending public snapshot displayed to the person.
    ///
    /// # Errors
    /// Refuses stale confirmations, own-account substitutions and unknown peers.
    pub fn confirm(&mut self, snapshot: AccountKeys) -> Result<(), Error> {
        if snapshot.tag == self.tag
            || self
                .current
                .get(&snapshot.tag)
                .is_none_or(|current| devices(current).ok() != devices(&snapshot).ok())
        {
            return Err(Error::Conflict);
        }
        devices(&snapshot)?;
        self.changed.remove(&snapshot.tag);
        self.accepted.insert(snapshot.tag.clone(), snapshot);
        Ok(())
    }

    /// Encrypt the accepted trust state for persistent storage; never plaintext history.
    ///
    /// # Errors
    /// Fails without randomness or if the bounded state cannot be encoded/sealed.
    pub fn save_trust(&self) -> Result<Vec<u8>, Error> {
        let bytes = Zeroizing::new(central_api::encode(&Trust {
            version: 1,
            keys: self.accepted.values().cloned().collect(),
            changed: self.changed.clone(),
        })?);
        if bytes.len() > MAX_TRUST_BYTES {
            return Err(Error::Busy);
        }
        central_crypto::chat::at_rest_seal(
            self.secrets.vault_key(),
            RestKind::State,
            TRUST_NAME,
            &bytes,
            &mut messages::Random::new()?,
        )
        .map_err(rejected)
    }

    /// Authenticate and restore a complete saved trust state, without partial mutation.
    /// Encrypted local state has authenticity, not rollback protection; deletion loses TOFU pins.
    ///
    /// # Errors
    /// Rejects damaged, foreign-account, oversized or duplicate trust records.
    pub fn restore_trust(&mut self, record: &[u8]) -> Result<(), Error> {
        if record.len() > MAX_TRUST_BYTES + 40 {
            return Err(Error::Rejected);
        }
        let bytes = central_crypto::chat::at_rest_open(
            self.secrets.vault_key(),
            RestKind::State,
            TRUST_NAME,
            record,
        )
        .map_err(rejected)?;
        let trust: Trust = central_api::decode(&bytes)?;
        if trust.version != 1
            || trust.keys.len() > MAX_PEERS
            || trust.changed.len() > trust.keys.len()
        {
            return Err(Error::Rejected);
        }
        let mut restored = BTreeMap::new();
        for key in trust.keys {
            devices(&key)?;
            if key.tag == self.tag || restored.insert(key.tag.clone(), key).is_some() {
                return Err(Error::Rejected);
            }
        }
        if trust.changed.iter().any(|tag| !restored.contains_key(tag)) {
            return Err(Error::Rejected);
        }
        self.changed = trust.changed;
        self.accepted = restored;
        self.current.clear();
        Ok(())
    }
}
