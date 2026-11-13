//! Central's signed tokens: the keys, what a request presents, and what each token says.
//!
//! A Central token's `sid` is the session's random capability, of which the store keeps only
//! a digest, so the signing key alone forges no session. That capability alone must open
//! nothing either: [`credential_operation`] keeps the two kinds of bearer apart, so a
//! capability copied out of a token, presented bare, is refused like any other wrong bearer.
use super::{Error, IdentityService, Place, Request, Response, Token, verified};
use central_api::{SignedToken, wire::GatewayPurpose};
use central_crypto::jwt::{
    self, CENTRAL_TOKEN_SECONDS, CentralClaims, Expected, GATEWAY_TOKEN_SECONDS, GatewayClaims,
    KeySet, Kind, PublicKey, Purpose, REVOCATION_SECONDS, RevocationClaims, SigningKey, TermsClaim,
};
use central_postgres::GatewayGrant;
use std::{
    fmt::Write as _,
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use zeroize::Zeroizing;

/// The key that signs, and every key the JWKS lists: that one, those still verifying what
/// they signed before a rotation, and the next one, listed before it signs.
pub struct TokenKeys {
    signing: SigningKey,
    published: KeySet,
    document: Vec<u8>,
}

impl TokenKeys {
    /// Sign with `signing` and list it beside `others`.
    ///
    /// # Errors
    /// Rejects more keys than a set holds and one key listed twice.
    pub fn new(signing: SigningKey, others: Vec<PublicKey>) -> Result<Self, Error> {
        let mut keys = vec![signing.public().clone()];
        keys.extend(others);
        let published = KeySet::new(keys).map_err(|_| Error::Rejected)?;
        let document = published.to_jwks().map_err(|_| Error::Rejected)?;
        Ok(Self {
            signing,
            published,
            document,
        })
    }

    /// The JWKS document, made once: serving it costs no signing and no parsing.
    #[must_use]
    pub fn jwks(&self) -> &[u8] {
        &self.document
    }
}

/// What a request's `Authorization` header turned out to be.
pub enum Presented {
    /// A gateway's credential or the bridge's: a bare random capability.
    Credential(Token),
    /// A verified Central token's capability.
    Session(Token),
}

impl Presented {
    /// The capability, for the digest lookups that follow.
    pub(super) const fn token(&self) -> &Token {
        match self {
            Self::Credential(token) | Self::Session(token) => token,
        }
    }
}

/// Whether an operation is a gateway's, under its credential or the bridge's, and never a
/// client's. Every other operation refuses a bare credential, and these refuse a signed
/// session. The match names every operation, so a new one does not compile until it is
/// decided here.
#[must_use]
pub const fn credential_operation(request: &Request) -> bool {
    match request {
        Request::ChallengeHost { .. }
        | Request::GatewayHeartbeat { .. }
        | Request::GatewayPulse { .. }
        | Request::GatewayStatus
        | Request::RegisterInvalidate { .. }
        | Request::GatewayRevocations { .. }
        | Request::PublishGatewayTerms { .. }
        | Request::LegacyReserve { .. }
        | Request::LegacyInvalidate { .. }
        | Request::LegacyClose { .. }
        | Request::LegacyProfile { .. }
        | Request::LegacyStatus { .. } => true,
        Request::Challenges { .. }
        | Request::Notifications { .. }
        | Request::Social { .. }
        | Request::SessionStatus
        | Request::RenewSession
        | Request::CreateObject
        | Request::ReadObject { .. }
        | Request::ListObjects { .. }
        | Request::WriteObject { .. }
        | Request::DeleteObject { .. }
        | Request::RegisterGateway { .. }
        | Request::GatewayDirectory { .. }
        | Request::PublicFigures { .. }
        | Request::SetGatewayAdmins { .. }
        | Request::Discover { .. }
        | Request::LoginStart { .. }
        | Request::LoginFinish { .. }
        | Request::LegacyLogin { .. }
        | Request::LegacyExport
        | Request::LegacyLink { .. }
        | Request::RegisterChallenge
        | Request::RegisterChallengeFor { .. }
        | Request::Register { .. }
        | Request::EnrollmentStart { .. }
        | Request::EnrollmentFinish { .. }
        | Request::ReadDocument
        | Request::WriteDocument { .. }
        | Request::ConfirmRestore { .. }
        | Request::GatewayToken { .. }
        | Request::Logout
        | Request::Terms
        | Request::GatewayTerms { .. }
        | Request::TermsStatus
        | Request::AcceptTerms { .. }
        | Request::AccountSummary
        | Request::SetRecovery { .. }
        | Request::SetDisplayName { .. }
        | Request::ChangePasswordStart { .. }
        | Request::ChangePasswordFinish { .. }
        | Request::ResetStart { .. }
        | Request::ResetFinish { .. }
        | Request::ResetMail { .. }
        | Request::Email { .. }
        | Request::Owner { .. }
        | Request::Picture { .. }
        | Request::Admin { .. }
        | Request::AccountExport
        | Request::DeleteAccountStart { .. }
        | Request::DeleteAccountFinish { .. }
        | Request::KeyCheckStatus
        | Request::SetKeyCheck { .. }
        | Request::PairStart
        | Request::PairStatus { .. }
        | Request::PairApprove { .. }
        | Request::PairCancel { .. }
        | Request::PairClaim { .. }
        | Request::PairPoll { .. }
        | Request::PairRedeem { .. } => false,
    }
}

fn now() -> Result<i64, Error> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|elapsed| i64::try_from(elapsed.as_secs()).ok())
        .ok_or(Error::Unavailable)
}

fn hex(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(text, "{byte:02x}");
    }
    text
}

/// 16 bytes from the operating system, as a token's `jti` or a new `sid`.
pub(super) fn random_id() -> Result<[u8; 16], Error> {
    let token = Token::generate()?;
    let bytes = token.to_secret_bytes();
    let mut id = [0; 16];
    id.copy_from_slice(&bytes[..16]);
    Ok(id)
}

/// An account's UUID as a token's `sub` writes it.
fn subject(account: [u8; 16]) -> String {
    let text = hex(&account);
    format!(
        "{}-{}-{}-{}-{}",
        &text[..8],
        &text[8..12],
        &text[12..16],
        &text[16..20],
        &text[20..]
    )
}

fn signed(token: Zeroizing<String>) -> Result<SignedToken, Error> {
    SignedToken::new(token)
}

impl IdentityService {
    /// Read a request's `Authorization` value, sent from `place`. A signed value must be a
    /// Central token that verifies, or one this process issued or verified before and whose
    /// `exp` is still ahead (`verified.rs`); everything else about it, and whether it may do
    /// what it asks, comes later.
    ///
    /// # Errors
    /// Rejects any other spelling, a token longer than a Central token may be, a token that
    /// does not verify, and `Busy` past the budget of signature checks, all places' or
    /// `place`'s share of it, which keeps made-up tokens from costing more than other
    /// requests.
    pub fn present(&self, header: &str, place: Option<Place>) -> Result<Presented, Error> {
        let value = header.strip_prefix("Bearer ").ok_or(Error::Rejected)?;
        if !value.contains('.') {
            return Ok(Presented::Credential(Token::from_hex(value)?));
        }
        // Bounded before anything is computed of it: the digest below comes before every
        // budget.
        if value.len() > Kind::Central.max_bytes() {
            return Err(Error::Rejected);
        }
        let key = verified::key(value);
        let time = now()?;
        {
            let mut limits = self.limits.lock().map_err(|_| Error::Unavailable)?;
            let at = Instant::now();
            if let Some(capability) = limits.verified.find(&key, time, at) {
                return Ok(Presented::Session(capability));
            }
            limits.signature(place, at)?;
        }
        let claims: CentralClaims = jwt::verify(
            value,
            &self.tokens.published,
            Expected {
                issuer: &self.origin,
                audience: &self.origin,
                now: time,
            },
        )
        .map_err(|_| Error::Rejected)?;
        let capability = Token::from_hex(&claims.sid)?;
        // Remembered only while its session is vouched for: a token that verifies may be
        // one whose session was replaced or ended, which must cost a sign-in to enter, not
        // a check (`limits.rs` `may_remember`). A lock that cannot be had only costs the
        // token its next check.
        if let Ok(mut limits) = self.limits.lock() {
            let at = Instant::now();
            if limits.may_remember(&capability.digest(), place, at) {
                limits
                    .verified
                    .remember(key, &capability, claims.exp, time, at);
            }
        }
        Ok(Presented::Session(capability))
    }

    /// The bearer of a native session. It lasts to the session's cap, and the session's row
    /// decides whether it is live, so a renewal changes nothing here.
    pub(super) fn central_token(
        &self,
        capability: &Token,
        account: [u8; 16],
    ) -> Result<SignedToken, Error> {
        let time = now()?;
        let claims = CentralClaims {
            iss: self.origin.clone(),
            aud: self.origin.clone(),
            sub: subject(account),
            sid: capability.hex().to_string(),
            jti: hex(&random_id()?),
            iat: time,
            nbf: time,
            exp: time + CENTRAL_TOKEN_SECONDS,
        };
        let token = self
            .tokens
            .signing
            .sign(&claims)
            .map_err(|_| Error::Rejected)?;
        // Central signed it for a session it just issued and vouched for (`issued`): its
        // first request needs no check either.
        if let Ok(mut limits) = self.limits.lock() {
            limits.verified.remember(
                verified::key(&token),
                capability,
                claims.exp,
                time,
                Instant::now(),
            );
        }
        signed(token)
    }

    /// A token for the gateway a client asked for. `audience` is that origin as asked for.
    fn signed_grant(
        &self,
        audience: &str,
        purpose: GatewayPurpose,
        grant: GatewayGrant,
    ) -> Result<Response, Error> {
        let time = now()?;
        let exp = (time + GATEWAY_TOKEN_SECONDS).min(grant.cap);
        if exp <= time {
            return Err(Error::Rejected);
        }
        let claims = GatewayClaims {
            iss: self.origin.clone(),
            aud: audience.to_owned(),
            sub: subject(grant.account),
            sid: hex(&grant.sid),
            jti: hex(&random_id()?),
            iat: time,
            nbf: time,
            exp,
            purpose: match purpose {
                GatewayPurpose::Admit => Purpose::Admit,
                GatewayPurpose::Admin => Purpose::Admin,
            },
            name: grant.display_name,
            tag: grant.tag,
            legacy_id: grant.legacy_id,
            terms: grant.terms.map(|terms| TermsClaim {
                version: terms.version,
                digest: hex(&terms.digest),
            }),
        };
        let token = signed(
            self.tokens
                .signing
                .sign(&claims)
                .map_err(|_| Error::Rejected)?,
        )?;
        Ok(Response::GatewayToken {
            token,
            expires_at: exp,
        })
    }

    /// Issue or refresh a client's token for one gateway.
    pub(super) async fn gateway_token(
        &self,
        gateway: &str,
        purpose: GatewayPurpose,
        bearer: Option<&Token>,
    ) -> Result<Response, Error> {
        let session = bearer.ok_or(Error::Rejected)?;
        let official = self.is_official(gateway);
        let own = [gateway.to_owned()];
        let set: &[String] = if official { &self.official } else { &own };
        let grant = self
            .store
            .gateway_grant(session, set, !official, purpose, random_id()?)
            .await?;
        self.signed_grant(gateway, purpose, grant)
    }

    /// A message telling the gateway at `origin` that the sessions numbered from `from` ended.
    /// Signed fresh for every send, so that its short life also suits a late catch-up.
    ///
    /// # Errors
    /// Rejects an empty or oversized page.
    pub fn revocation_message(
        &self,
        origin: &str,
        from: u64,
        sids: &[[u8; 16]],
    ) -> Result<SignedToken, Error> {
        let time = now()?;
        let count = u64::try_from(sids.len()).map_err(|_| Error::Rejected)?;
        let claims = RevocationClaims {
            iss: self.origin.clone(),
            aud: origin.to_owned(),
            jti: hex(&random_id()?),
            iat: time,
            nbf: time,
            exp: time + REVOCATION_SECONDS,
            from,
            seq: from
                .checked_add(count)
                .and_then(|end| end.checked_sub(1))
                .ok_or(Error::Rejected)?,
            sids: sids.iter().map(|sid| hex(sid)).collect(),
        };
        signed(
            self.tokens
                .signing
                .sign(&claims)
                .map_err(|_| Error::Rejected)?,
        )
    }

    /// The JWKS document, past a budget that keeps this public route bounded.
    ///
    /// # Errors
    /// `Busy` past the budget; a caller keeps the keys it holds and tries again.
    pub fn jwks(&self) -> Result<&[u8], Error> {
        self.limits
            .lock()
            .map_err(|_| Error::Unavailable)?
            .jwks(Instant::now())?;
        Ok(self.tokens.jwks())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use central_crypto::jwt::SigningKey;

    fn key(byte: u8) -> SigningKey {
        SigningKey::from_seed(&Zeroizing::new([byte; 32]))
    }

    #[test]
    fn a_key_set_lists_the_signing_key_and_the_others() {
        let keys = TokenKeys::new(key(1), vec![key(2).public().clone()]).unwrap();
        let read = KeySet::from_jwks(keys.jwks()).unwrap();
        assert_eq!(read.keys().len(), 2);
        assert!(read.find(key(1).public().kid()).is_some());
        assert!(read.find(key(2).public().kid()).is_some());
    }

    #[test]
    fn a_key_set_refuses_one_key_twice_and_too_many() {
        assert!(TokenKeys::new(key(1), vec![key(1).public().clone()]).is_err());
        let many = (2..=9).map(|byte| key(byte).public().clone()).collect();
        assert!(TokenKeys::new(key(1), many).is_err());
    }

    #[test]
    fn a_subject_is_a_hyphenated_lowercase_uuid() {
        let mut account = [0xab; 16];
        account[0] = 0x01;
        assert_eq!(subject(account), "01ababab-abab-abab-abab-abababababab");
    }

    #[test]
    fn only_a_gateways_operations_take_a_bare_credential() {
        assert!(credential_operation(&Request::GatewayRevocations {
            after: 0
        }));
        assert!(credential_operation(&Request::GatewayStatus));
        // A client's own requests: a capability copied out of a token opens none of them.
        for request in [
            Request::ReadDocument,
            Request::SessionStatus,
            Request::RenewSession,
            Request::Logout,
            Request::AccountExport,
            Request::GatewayToken {
                gateway: "https://gateway.invalid".into(),
                purpose: GatewayPurpose::Admit,
            },
        ] {
            assert!(!credential_operation(&request));
        }
    }
}
