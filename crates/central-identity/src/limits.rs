use super::Place;
use central_api::{Error, Token, wire::Request};
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};
use subtle::ConstantTimeEq;

/// Requests a second without a session, all clients together: discovery,
/// sign-in, registration and the other public operations.
pub(super) const GLOBAL_PER_SECOND: u32 = 32;
/// Such requests a second beside those, for places that asked little in
/// their minute ([`LIGHT_PER_MINUTE`]), all of them together. Drawn on only
/// when the second everyone shares is spent, so a household still signs in
/// while a few addresses keep up their share. Beside the 32 and not out of
/// them: whatever was served without these is served with them.
pub(super) const RESERVE_PER_SECOND: u32 = 8;
/// Requests in its minute below which a place has asked little: six
/// sign-ins of five requests each. So it is also the most one place draws
/// of the spare requests in a minute, and spending them all takes sixteen
/// places that each ask this much and no more. An IPv6 place is counted
/// here by the /48 around it ([`block`]).
pub(super) const LIGHT_PER_MINUTE: u32 = 30;

/// The place whose requests say how little `place` asked: an IPv4 address
/// itself, and for an IPv6 /64 the /48 around it, written as the place of
/// that block's first /64. A subscriber often holds a /56 or a /48, so
/// hundreds of places; counted one by one, a single subscriber would draw
/// all the spare requests there are. The guesses at passwords count the
/// same block (`guesses.rs`).
fn block(place: Place) -> Place {
    match place {
        Place::V4(_) => place,
        Place::V6(mut prefix) => {
            prefix[6] = 0;
            prefix[7] = 0;
            Place::V6(prefix)
        }
    }
}
/// The most requests a second without a session, both together. Every
/// table sized by what such requests can start is sized by this.
pub(super) const PUBLIC_PER_SECOND: u32 = GLOBAL_PER_SECOND + RESERVE_PER_SECOND;
/// How long an unfinished OPAQUE sign-in, password change or deletion waits
/// for its second message.
pub(super) const PENDING_SECONDS: u64 = 60;
/// Unfinished OPAQUE exchanges held at once. More than the public budget can
/// start within their lifetime ([`PUBLIC_PER_SECOND`] × [`PENDING_SECONDS`] =
/// 2,400), so no sequence of public requests fills the table before it spends
/// the budget everyone shares; a password change or deletion besides needs a
/// signed-in account and its eight units a minute. Each entry is an OPAQUE
/// server state and a credential record, about a kilobyte: a few megabytes
/// at most.
pub(super) const PENDING: usize = 4096;
const _: () = assert!(PENDING as u64 > PUBLIC_PER_SECOND as u64 * PENDING_SECONDS);

/// Legacy sign-ins a minute, all names together: the store holds a handful
/// of accounts, and each sign-in is one Argon2 derivation.
pub(super) const LEGACY_PER_MINUTE: u32 = 30;
/// Legacy sign-ins a minute for one name, as typed and lowercased; the
/// store's lockout counts the failures that pass this.
pub(super) const LEGACY_PER_NAME: u32 = 5;

/// Requests a minute from one place that draw on [`GLOBAL_PER_SECOND`]: a
/// little under a third of that minute's 1,920, so one address alone never
/// spends everyone's sign-ins, and far more than a house or an office
/// needs (a sign-in is three to five such requests).
pub(super) const PLACE_PER_MINUTE: u32 = 600;
/// A client's pairing requests a minute from one place: a pairing polls
/// once a second, so four at once, of the 64 a second all places share.
pub(super) const PLACE_PAIRING_PER_MINUTE: u32 = 240;
/// Places one such budget remembers. Never full of live minutes: fewer
/// requests pass the shared windows between two sweeps and the minute
/// before them (at most 64 a second) than it holds.
pub(super) const PLACES: usize = 8_192;
const _: () = assert!(PLACES > 2 * 60 * 64 && PUBLIC_PER_SECOND <= 64);
const MINUTE: Duration = Duration::from_secs(60);

/// Requests a second that come with a bearer no answer has vouched for
/// ([`Proven`]), all of them together: a made-up token, or a session's
/// first request after a restart or a long pause. Their own window, so
/// they spend neither what sessions share nor what everyone signs in with.
pub(super) const UNPROVEN_PER_SECOND: u32 = 64;
const _: () = assert!(PLACES > 2 * 60 * UNPROVEN_PER_SECOND as usize);
/// Such requests a minute from one place: as many as a place may draw of
/// the sign-ins' budget ([`PLACE_PER_MINUTE`]). A session needs one, since
/// its first answer vouches for it.
pub(super) const PLACE_UNPROVEN_PER_MINUTE: u32 = 600;
/// How long an answer vouches for a bearer: a session's 30 minutes between
/// two renewals. Every further answer that vouches starts it again.
pub(super) const PROVEN_SECONDS: u64 = 1_800;
/// Bearers vouched for at once; one more takes the place of the one whose
/// last such answer is the oldest. About a megabyte.
pub(super) const PROVEN: usize = 16_384;
/// Refusals in a row after which a bearer is no longer vouched for: a
/// session that ended, or was replaced, stops counting as one after this
/// many requests.
pub(super) const REFUSALS: u8 = 4;
/// Control and data requests a second all vouched-for bearers share.
const SHARED_CONTROL: u32 = 512;
const SHARED_DATA: u32 = 256;
/// Control and data requests a second for one bearer, of what they share.
const BEARER_CONTROL: u32 = 128;
const BEARER_DATA: u32 = 64;
/// Control and data requests a second for the bearers of one place
/// together, of what all share: half of each window, so one address alone
/// never spends what every session shares, however many bearers it holds.
/// Half and not a quarter: two that are counted apart at one address each
/// keep their whole own budget (two sessions there, or two accounts'
/// gateways; one account's credentials and tokens are counted as one),
/// and a household's clients ask a small part of it.
const PLACE_CONTROL: u32 = SHARED_CONTROL / 2;
const PLACE_DATA: u32 = SHARED_DATA / 2;
const _: () = assert!(PLACE_CONTROL >= 2 * BEARER_CONTROL && PLACE_DATA >= 2 * BEARER_DATA);
const _: () = assert!(PLACE_CONTROL < SHARED_CONTROL && PLACE_DATA < SHARED_DATA);
/// Bearers' own windows held at once: more than can be opened in the
/// second they live, since each is opened by a request that was served.
const PEERS: usize = 1_024;
/// Places' windows of the shared budgets held at once. Never full: a
/// window is opened by a request that was served and forgotten a second
/// later, and fewer are served in two seconds than this holds.
const SHARING: usize = 2_048;
const _: () = assert!(SHARING > 2 * (SHARED_CONTROL + SHARED_DATA) as usize);

/// How a request is counted.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Class {
    /// A client's pairing: no session, a window of its own.
    Pairing,
    /// Served without a session, whatever token comes along.
    Public,
    /// A session's or a gateway credential's: status, renewal and the
    /// checks gateways make of their players.
    Control,
    /// A session's or a gateway credential's: everything else.
    Data,
}

fn class(request: &Request) -> Class {
    // A client's claim, polls and redeem carry no session; their own
    // window keeps a few pairings' polling from spending the sign-ins'.
    if matches!(
        request,
        Request::PairClaim { .. } | Request::PairPoll { .. } | Request::PairRedeem { .. }
    ) {
        return Class::Pairing;
    }
    if matches!(
        request,
        Request::Discover { .. }
            | Request::LoginStart { .. }
            | Request::LoginFinish { .. }
            // Named, not left to the rule for requests without a
            // session: the table of challenges is sized by this budget
            // (`challenge::HELD`), whatever token comes with the request.
            | Request::RegisterChallenge
            | Request::RegisterChallengeFor { .. }
            | Request::Register { .. }
            | Request::EnrollmentStart { .. }
            | Request::EnrollmentFinish { .. }
            | Request::GatewayDirectory { .. }
            | Request::PublicFigures { .. }
            | Request::Terms
            | Request::GatewayTerms { .. }
            | Request::ResetStart { .. }
            | Request::ResetFinish { .. }
            | Request::LegacyLogin { .. }
    ) {
        return Class::Public;
    }
    // Separate bounded budgets keep a ciphertext transfer from exhausting
    // login or revocation checks.
    if matches!(
        request,
        Request::SessionStatus
            | Request::RenewSession
            | Request::GatewaySession { .. }
            | Request::GatewayProfile { .. }
            | Request::GatewayAuthorize { .. }
            | Request::TermsStatus
            | Request::AccountSummary
            | Request::Logout
            | Request::SetGatewayAdmins { .. }
    ) {
        Class::Control
    } else {
        Class::Data
    }
}

/// What the answer to a request says about the bearer it came with.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Tells {
    /// Answered, it vouches for the bearer: the store refuses each of
    /// these for a token that is no live session or credential, whatever
    /// else was asked, and so does the bridge's comparison.
    Proves,
    /// Answered, the bearer is no session any more, or the answer is the
    /// same for a token nobody issued.
    Ends,
    /// Nothing either way.
    Nothing,
}

/// A short list, and a request that is not on it vouches for nothing:
/// what clients, the website and gateways ask on their own, each read in
/// `central-postgres` to refuse a token it does not know.
fn tells(request: &Request) -> Tells {
    match request {
        Request::SessionStatus
        | Request::RenewSession
        | Request::AccountSummary
        | Request::TermsStatus
        | Request::Social { .. }
        | Request::Notifications { .. }
        | Request::CreateObject
        | Request::ReadObject { .. }
        | Request::ListObjects { .. }
        | Request::WriteObject { .. }
        | Request::DeleteObject { .. }
        | Request::ReadDocument
        | Request::WriteDocument { .. }
        | Request::GatewayTicket { .. }
        | Request::GatewaySession { .. }
        | Request::GatewayProfile { .. }
        | Request::RedeemGateway { .. }
        | Request::GatewayHeartbeat { .. }
        | Request::GatewayPulse { .. }
        | Request::GatewayStatus
        | Request::CheckGatewayAdmin { .. } => Tells::Proves,
        // The first two are answered alike whoever asks; after the other
        // two the session that asked is gone.
        Request::Logout
        | Request::PairCancel { .. }
        | Request::ChangePasswordFinish { .. }
        | Request::DeleteAccountFinish { .. } => Tells::Ends,
        _ => Tells::Nothing,
    }
}

/// A request that is counted by the bearer it came with, kept from before
/// it is served until it is answered ([`Limits::answered`]).
pub(super) struct Told {
    bearer: [u8; 32],
    tells: Tells,
    /// Whether a refusal says something about the bearer
    /// ([`refusal_is_the_bearer_s`]).
    refusal_counts: bool,
}

/// Whether a `Rejected` answer to `request` may count against its bearer.
/// Not where the request carries a second secret that is someone else's: a
/// gateway asks these with its own credential about a lease or a ticket a
/// player handed it, and Central refuses a made-up one with the same
/// `Rejected` it has for a credential that ended. Counted, four bad
/// tickets from any player would take a gateway's credential out of the
/// vouched-for at will. Not counted, a credential that did end keeps its
/// place until its last vouching answer is [`PROVEN_SECONDS`] old, and
/// spends its own bearer's budget meanwhile.
fn refusal_is_the_bearer_s(request: &Request) -> bool {
    !matches!(
        request,
        Request::GatewaySession { .. }
            | Request::GatewayProfile { .. }
            | Request::GatewayAuthorize { .. }
            | Request::RedeemGateway { .. }
    )
}

/// What `request`'s answer will say about `bearer`; nothing for a request
/// that is not counted by its bearer (a public one, a pairing's, one
/// without a bearer).
pub(super) fn told(request: &Request, bearer: Option<&Token>) -> Option<Told> {
    let bearer = bearer?;
    matches!(class(request), Class::Control | Class::Data).then(|| Told {
        bearer: bearer.digest(),
        tells: tells(request),
        refusal_counts: refusal_is_the_bearer_s(request),
    })
}

/// The bearers an answer vouched for, by their digests, in memory only: a
/// session a sign-in just issued, or a token that one of the requests in
/// [`tells`] was answered with. Supplying a token vouches for nothing; a
/// bearer that is not here is counted apart ([`UNPROVEN_PER_SECOND`]) and
/// served like any other, and what it may do is the store's to decide
/// either way.
#[derive(Default)]
struct Proven {
    known: BTreeMap<[u8; 32], Vouched>,
    swept: Option<Instant>,
    /// The official gateway's bridge: configured, so never forgotten.
    kept: Option<[u8; 32]>,
}

/// A bearer an answer vouched for.
struct Vouched {
    /// When it was last vouched for.
    at: Instant,
    /// The refusals in a row since.
    refusals: u8,
    /// The account whose gateways it is a credential or a token of, as
    /// the store said when it was first vouched for; none for a session.
    owner: Option<[u8; 16]>,
}

impl Vouched {
    fn live(&self, now: Instant) -> bool {
        now.saturating_duration_since(self.at) < Duration::from_secs(PROVEN_SECONDS)
    }
}

/// Whose budget a vouched-for bearer's requests are counted in
/// ([`BEARER_CONTROL`], [`BEARER_DATA`]).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Unit {
    /// Its own: a session's, or the bridge's.
    Bearer([u8; 32]),
    /// One account's gateways': every credential and token of every
    /// gateway the account owns, together. An account holds up to eight
    /// gateways and eight tokens for each; counted one by one, they
    /// alone would spend what all sessions share.
    Owner([u8; 16]),
}

impl Proven {
    fn kept(&self, bearer: &[u8; 32]) -> bool {
        self.kept.is_some_and(|kept| bool::from(kept.ct_eq(bearer)))
    }

    fn holds(&self, bearer: &[u8; 32], now: Instant) -> bool {
        self.unit(bearer, now).is_some()
    }

    /// Whose budget `bearer` is counted in; none for one that is not
    /// vouched for.
    fn unit(&self, bearer: &[u8; 32], now: Instant) -> Option<Unit> {
        if self.kept(bearer) {
            return Some(Unit::Bearer(*bearer));
        }
        let known = self.known.get(bearer).filter(|known| known.live(now))?;
        Some(known.owner.map_or(Unit::Bearer(*bearer), Unit::Owner))
    }

    /// Vouch for `bearer` from `now` on. `owner` is whose gateways it
    /// belongs to; a bearer vouched for again stays whose it was.
    fn mark(&mut self, bearer: [u8; 32], owner: Option<[u8; 16]>, now: Instant) {
        let due = self
            .swept
            .is_none_or(|swept| now.saturating_duration_since(swept) >= MINUTE);
        if due {
            self.known.retain(|_, known| known.live(now));
            self.swept = Some(now);
        }
        if self.known.len() >= PROVEN && !self.known.contains_key(&bearer) {
            let oldest = self
                .known
                .iter()
                .min_by_key(|(_, known)| known.at)
                .map(|(bearer, _)| *bearer);
            if let Some(oldest) = oldest {
                self.known.remove(&oldest);
            }
        }
        let owner = owner.or_else(|| self.known.get(&bearer).and_then(|known| known.owner));
        let vouched = Vouched {
            at: now,
            refusals: 0,
            owner,
        };
        self.known.insert(bearer, vouched);
    }

    fn refused(&mut self, bearer: &[u8; 32]) {
        let Some(known) = self.known.get_mut(bearer) else {
            return;
        };
        known.refusals = known.refusals.saturating_add(1);
        if known.refusals >= REFUSALS {
            self.known.remove(bearer);
        }
    }
}

/// What each place drew of one shared budget in its current minute, in
/// memory only; a place is forgotten at most a minute after its minute.
#[derive(Default)]
struct Places {
    windows: BTreeMap<Place, (Instant, u32)>,
    swept: Option<Instant>,
}

impl Places {
    /// Whether `place` may draw once more of `limit` a minute. A request
    /// the listener knows no place of meets the shared budget alone.
    fn fits(&self, place: Option<Place>, now: Instant, limit: u32) -> bool {
        place
            .and_then(|place| self.windows.get(&place))
            .is_none_or(|(start, used)| {
                now.saturating_duration_since(*start) >= MINUTE || *used < limit
            })
    }

    /// Count one request that passed the shared budget.
    fn spend(&mut self, place: Option<Place>, now: Instant) {
        let Some(place) = place else {
            return;
        };
        let due = self
            .swept
            .is_none_or(|swept| now.saturating_duration_since(swept) >= MINUTE);
        if due {
            self.windows
                .retain(|_, (start, _)| now.saturating_duration_since(*start) < MINUTE);
            self.swept = Some(now);
        }
        if self.windows.len() >= PLACES && !self.windows.contains_key(&place) {
            let oldest = self
                .windows
                .iter()
                .min_by_key(|(_, (start, _))| *start)
                .map(|(place, _)| *place);
            if let Some(oldest) = oldest {
                self.windows.remove(&oldest);
            }
        }
        let window = self.windows.entry(place).or_insert((now, 0));
        if now.saturating_duration_since(window.0) >= MINUTE {
            *window = (now, 0);
        }
        window.1 = window.1.saturating_add(1);
    }
}

#[derive(Default)]
pub(super) struct Limits {
    global: Option<(Instant, u32)>,
    /// What places that asked little drew beside it ([`RESERVE_PER_SECOND`]).
    spare: Window,
    /// What each block was served of both in its minute ([`block`],
    /// [`LIGHT_PER_MINUTE`]).
    light: Places,
    /// Each place's share of [`GLOBAL_PER_SECOND`] ([`PLACE_PER_MINUTE`]).
    public: Places,
    /// Each place's share of the pairing window ([`PLACE_PAIRING_PER_MINUTE`]).
    paired: Places,
    /// Guesses at passwords: sign-ins, password changes and deletions
    /// (`guesses.rs`).
    pub(super) guesses: super::guesses::Guesses,
    legacy: Option<(Instant, u32)>,
    legacy_names: BTreeMap<String, (Instant, u32)>,
    data: Window,
    control: Window,
    pairing: Window,
    /// What each unit drew of its own budget in the current second.
    peers: BTreeMap<(Unit, bool), Window>,
    /// What each place's bearers drew together of `control` (`true`) and
    /// of `data` in the current second ([`PLACE_CONTROL`], [`PLACE_DATA`]).
    sharing: BTreeMap<(Place, bool), Window>,
    /// The bearers an answer vouched for; only they draw on `control`,
    /// `data`, `peers` and `sharing`.
    proven: Proven,
    /// Every other bearer's requests ([`UNPROVEN_PER_SECOND`]) and each
    /// place's share of them ([`PLACE_UNPROVEN_PER_MINUTE`]).
    unproven: Window,
    strangers: Places,
    /// Reports each gateway filed in the current hour.
    reports: BTreeMap<[u8; 16], (Instant, u32)>,
}

#[derive(Default)]
struct Window(Option<(Instant, u32)>);
impl Window {
    fn charge(&mut self, now: Instant, maximum: u32) -> Result<(), Error> {
        let (start, used) = self.0.get_or_insert((now, 0));
        if now.duration_since(*start) >= Duration::from_secs(1) {
            *start = now;
            *used = 0;
        }
        if *used >= maximum {
            return Err(Error::Busy);
        }
        *used += 1;
        Ok(())
    }

    /// Whether one more of `maximum` a second fits; nothing is spent.
    fn fits(&self, now: Instant, maximum: u32) -> bool {
        self.0.is_none_or(|(start, used)| {
            now.duration_since(start) >= Duration::from_secs(1) || used < maximum
        })
    }

    /// Whether its second is still running: one that is over, or never
    /// began, holds nothing worth keeping.
    fn live(&self, now: Instant) -> bool {
        self.0
            .is_some_and(|(start, _)| now.duration_since(start) < Duration::from_secs(1))
    }
}
impl Limits {
    /// Count a request against what all clients share, before anything
    /// else is looked at. `place` is where it came from: a request that
    /// needs no session draws on its place's share first
    /// ([`PLACE_PER_MINUTE`], [`PLACE_PAIRING_PER_MINUTE`]), so one address
    /// cannot spend what everyone signs in with. A request that comes
    /// with a bearer is counted by it only once an answer has vouched for
    /// that bearer ([`Proven`]); until then it shares the strangers'
    /// second ([`UNPROVEN_PER_SECOND`]). A vouched-for bearer has its own
    /// budget, its place's bearers have half a shared window between them
    /// ([`PLACE_CONTROL`], [`PLACE_DATA`]), and all have the shared window.
    /// Nothing is spent when a budget is used up.
    pub(super) fn request(
        &mut self,
        request: &Request,
        bearer: Option<&Token>,
        place: Option<Place>,
        now: Instant,
    ) -> Result<(), Error> {
        let control = match class(request) {
            Class::Pairing => {
                if !self.paired.fits(place, now, PLACE_PAIRING_PER_MINUTE) {
                    return Err(Error::Busy);
                }
                self.pairing.charge(now, 64)?;
                self.paired.spend(place, now);
                return Ok(());
            }
            Class::Public => return self.global(place, now),
            Class::Control => true,
            Class::Data => false,
        };
        let Some(bearer) = bearer else {
            return self.global(place, now);
        };
        let digest = bearer.digest();
        // A bearer no answer has vouched for may be anybody's invention.
        // It is served, and dispatch decides what it may do, but out of a
        // window of its own and its place's share of that: it spends
        // nothing of what sessions share, takes no room among their
        // bearers, and nothing of the sign-ins' budget either.
        let Some(unit) = self.proven.unit(&digest, now) else {
            if !self.strangers.fits(place, now, PLACE_UNPROVEN_PER_MINUTE) {
                return Err(Error::Busy);
            }
            self.unproven.charge(now, UNPROVEN_PER_SECOND)?;
            self.strangers.spend(place, now);
            return Ok(());
        };
        self.peers.retain(|_, window| window.live(now));
        self.sharing.retain(|_, window| window.live(now));
        // A gateway's bearer is counted with every other credential and
        // token of its owner's gateways ([`Unit`]).
        let key = (unit, control);
        let (own, share, shared) = if control {
            (BEARER_CONTROL, PLACE_CONTROL, SHARED_CONTROL)
        } else {
            (BEARER_DATA, PLACE_DATA, SHARED_DATA)
        };
        // The bearer's own budget is looked at before the shared one is
        // drawn on: a bearer asking faster than its own allows is refused
        // here and spends nothing of what every session shares.
        let room = match self.peers.get(&key) {
            Some(window) => window.fits(now, own),
            None => self.peers.len() < PEERS,
        };
        if !room {
            return Err(Error::Busy);
        }
        // Then its place's share, for the same reason: however many
        // bearers one address holds, they draw half a shared window
        // between them and are refused here past it. A request the
        // listener knows no place of (a process on Central's own host)
        // has no share to draw on and meets the other two budgets.
        let spot = place.map(|place| (place, control));
        let room = match spot.and_then(|spot| self.sharing.get(&spot)) {
            Some(window) => window.fits(now, share),
            None => spot.is_none() || self.sharing.len() < SHARING,
        };
        if !room {
            return Err(Error::Busy);
        }
        if control {
            self.control.charge(now, shared)?;
        } else {
            self.data.charge(now, shared)?;
        }
        if let Some(spot) = spot {
            self.sharing.entry(spot).or_default().charge(now, share)?;
        }
        self.peers.entry(key).or_default().charge(now, own)
    }

    /// Limits that count `bridge`, the official gateway's configured
    /// credential, as vouched for from the start and for good.
    pub(super) fn keeping(bridge: [u8; 32]) -> Self {
        let mut limits = Self::default();
        limits.proven.kept = Some(bridge);
        limits
    }

    /// A request counted by its bearer was answered: `refused` is the
    /// error, if it was one. An answer to one of the requests in [`tells`]
    /// vouches for the bearer from now on; [`REFUSALS`] rejections in a
    /// row, or the session's end, take that back. A rejection of a request
    /// that carries someone else's secret is not counted
    /// ([`refusal_is_the_bearer_s`]).
    ///
    /// `true` when the answer vouches for a bearer that was not vouched
    /// for before: it is not yet, and the caller asks the store whose it
    /// is and says so ([`Self::vouch`]). Which request was answered does
    /// not decide that: the store does, for every such bearer alike, so
    /// no list of a gateway's requests can fall behind the wire.
    #[must_use]
    pub(super) fn answered(&mut self, told: &Told, refused: Option<&Error>, now: Instant) -> bool {
        match (refused, told.tells) {
            // The bridge is configured and counted by itself.
            (None, Tells::Proves) if self.proven.kept(&told.bearer) => {}
            (None, Tells::Proves) if self.proven.holds(&told.bearer, now) => {
                self.proven.mark(told.bearer, None, now);
            }
            (None, Tells::Proves) => return true,
            (None, Tells::Ends) => {
                self.proven.known.remove(&told.bearer);
            }
            (Some(Error::Rejected), _) if told.refusal_counts => {
                self.proven.refused(&told.bearer);
            }
            _ => {}
        }
        false
    }

    /// Vouch for the bearer [`Self::answered`] asked about. `owner` is
    /// the account whose gateways it is a credential or a token of, as the
    /// store has it: its requests are then counted with all of that
    /// account's gateways' ([`Unit::Owner`]). None for a session, which is
    /// counted by itself.
    pub(super) fn vouch(&mut self, told: &Told, owner: Option<[u8; 16]>, now: Instant) {
        self.proven.mark(told.bearer, owner, now);
    }

    /// A sign-in, a password change or a pairing just issued `session`: it
    /// is vouched for before its first request.
    pub(super) fn issued(&mut self, session: &Token, now: Instant) {
        self.proven.mark(session.digest(), None, now);
    }

    /// Spend one of the [`GLOBAL_PER_SECOND`] requests everyone shares,
    /// and one of the [`PLACE_PER_MINUTE`] its place may draw of them.
    /// When everyone's second is spent, a place that asked little in its
    /// minute ([`LIGHT_PER_MINUTE`]) draws on the few kept for such places
    /// ([`RESERVE_PER_SECOND`]); a request the listener knows no place of
    /// never does. Nothing is spent when a budget is used up.
    pub(super) fn global(&mut self, place: Option<Place>, now: Instant) -> Result<(), Error> {
        if !self.public.fits(place, now, PLACE_PER_MINUTE) {
            return Err(Error::Busy);
        }
        // How little a place asked is counted by its block ([`block`]):
        // everything the block was served in the minute, so what it
        // draws of the spare requests ends its drawing of them.
        let around = place.map(block);
        if self.everyone(now).is_err() {
            if around.is_none() || !self.light.fits(around, now, LIGHT_PER_MINUTE) {
                return Err(Error::Busy);
            }
            self.spare.charge(now, RESERVE_PER_SECOND)?;
        }
        self.public.spend(place, now);
        self.light.spend(around, now);
        Ok(())
    }

    fn everyone(&mut self, now: Instant) -> Result<(), Error> {
        let (start, used) = self.global.get_or_insert((now, 0));
        if now.duration_since(*start) >= Duration::from_secs(1) {
            *start = now;
            *used = 0;
        }
        if *used >= GLOBAL_PER_SECOND {
            return Err(Error::Busy);
        }
        *used += 1;
        Ok(())
    }
    /// Spend one legacy sign-in: of [`LEGACY_PER_MINUTE`] for all names and
    /// [`LEGACY_PER_NAME`] for this one (as typed, lowercased, at most 64
    /// bytes of it). Nothing is spent when either is used up. A name that is
    /// nobody's is counted like one that is somebody's.
    pub(super) fn legacy(&mut self, typed: &str, now: Instant) -> Result<(), Error> {
        let minute = Duration::from_secs(60);
        let name: String = typed.chars().take(64).collect::<String>().to_lowercase();
        self.legacy_names
            .retain(|_, (start, _)| now.duration_since(*start) < minute);
        let (start, used) = self.legacy.get_or_insert((now, 0));
        if now.duration_since(*start) >= minute {
            *start = now;
            *used = 0;
        }
        let per_name = self.legacy_names.get(&name).map_or(0, |(_, used)| *used);
        if *used >= LEGACY_PER_MINUTE
            || per_name >= LEGACY_PER_NAME
            || (per_name == 0 && self.legacy_names.len() >= 1024)
        {
            return Err(Error::Busy);
        }
        *used += 1;
        self.legacy_names.entry(name).or_insert((now, 0)).1 += 1;
        Ok(())
    }

    /// A legacy sign-in from `place`: one of the names' budgets
    /// ([`Self::legacy`]) and one guess of the place's, which a proof
    /// gives back (`guesses.rs`). Nothing is spent when either is used up.
    pub(super) fn legacy_from(
        &mut self,
        typed: &str,
        place: Option<Place>,
        now: Instant,
    ) -> Result<super::guesses::Guess, Error> {
        self.guesses.legacy_fits(place, now)?;
        self.legacy(typed, now)?;
        self.guesses.legacy(place, now)
    }

    /// Spend one of the gateway's [`central_api::reports::PER_GATEWAY_HOUR`]
    /// reports an hour. Nothing is spent when it is used up.
    pub(super) fn report(&mut self, gateway: [u8; 16], now: Instant) -> Result<(), Error> {
        let hour = Duration::from_secs(3_600);
        self.reports
            .retain(|_, (start, _)| now.duration_since(*start) < hour);
        if !self.reports.contains_key(&gateway) && self.reports.len() >= 1024 {
            return Err(Error::Busy);
        }
        let (_, used) = self.reports.entry(gateway).or_insert((now, 0));
        if *used >= central_api::reports::PER_GATEWAY_HOUR {
            return Err(Error::Busy);
        }
        *used += 1;
        Ok(())
    }

    /// Spend `units` of the account's eight OPAQUE evaluations a minute
    /// and nothing of its longer windows: a reset's start, which only a
    /// valid reset code reaches, so that guessers who spent the account's
    /// hour or day do not keep its holder from a reset (`reset.rs`). The
    /// starts that test a guessed password (a sign-in's, a password
    /// change's, a deletion's) are counted by [`Self::guesses`]. Nothing
    /// is spent when the units do not fit.
    pub(super) fn account(
        &mut self,
        account: [u8; 16],
        now: Instant,
        units: u32,
    ) -> Result<(), Error> {
        self.guesses.evaluate(account, now, units)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn uploads_cannot_starve_authentication_or_revocation_checks() {
        let now = Instant::now();
        let mut limits = Limits::default();
        let token = Token::generate().unwrap();
        limits.issued(&token, now);
        for _ in 0..64 {
            limits
                .request(&Request::CreateObject, Some(&token), None, now)
                .unwrap();
        }
        assert_eq!(
            limits.request(&Request::CreateObject, Some(&token), None, now),
            Err(Error::Busy)
        );
        limits
            .request(
                &Request::Discover {
                    username: "synthetic".into(),
                },
                None,
                None,
                now,
            )
            .unwrap();
        limits
            .request(&Request::SessionStatus, Some(&token), None, now)
            .unwrap();
        limits
            .request(&Request::Logout, Some(&token), None, now)
            .unwrap();
        limits
            .request(
                &Request::CreateObject,
                Some(&token),
                None,
                now + Duration::from_secs(1),
            )
            .unwrap();
    }

    /// A bearer that asks faster than its own budget allows is refused out
    /// of that budget alone: what its refused requests would have drawn of
    /// the window every session shares stays there for the others.
    #[test]
    fn one_bearer_past_its_own_budget_spends_no_one_else_s() {
        let now = Instant::now();
        for (request, own, shared) in [
            (Request::SessionStatus, 128_u32, 512_u32),
            (Request::CreateObject, 64, 256),
        ] {
            let mut limits = Limits::default();
            let greedy = Token::generate().unwrap();
            limits.issued(&greedy, now);
            // Everything the shared window holds, from one bearer.
            let mut served = 0;
            for _ in 0..shared {
                if limits.request(&request, Some(&greedy), None, now).is_ok() {
                    served += 1;
                }
            }
            assert_eq!(served, own);
            // The rest of the second is still everyone else's: three more
            // bearers are served their whole budgets beside it.
            for _ in 0..3 {
                let other = Token::generate().unwrap();
                limits.issued(&other, now);
                for _ in 0..own {
                    limits
                        .request(&request, Some(&other), None, now)
                        .expect("another session is served");
                }
            }
            // Then the shared window is spent, by requests that were served.
            let last = Token::generate().unwrap();
            limits.issued(&last, now);
            assert_eq!(
                limits.request(&request, Some(&last), None, now),
                Err(Error::Busy)
            );
        }
    }

    /// Four bearers at one address filled a shared window between them
    /// (4 x 128, 4 x 64): two accounts' sessions, or one account's and
    /// its gateways' tokens. One place now draws half a window at most,
    /// however many bearers it holds, what it is refused spends nothing,
    /// and sessions elsewhere are served their whole budgets beside it.
    #[test]
    fn four_bearers_at_one_address_no_longer_fill_what_sessions_share() {
        let now = Instant::now();
        for (request, own, shared) in [
            (Request::SessionStatus, BEARER_CONTROL, SHARED_CONTROL),
            (Request::CreateObject, BEARER_DATA, SHARED_DATA),
        ] {
            let mut limits = Limits::default();
            let mut served = 0;
            for _ in 0..4 {
                let bearer = Token::generate().unwrap();
                limits.issued(&bearer, now);
                for _ in 0..own {
                    if limits
                        .request(&request, Some(&bearer), Some(place(1)), now)
                        .is_ok()
                    {
                        served += 1;
                    }
                }
            }
            assert_eq!(served, shared / 2);
            // The other half is still there, for two sessions at two
            // other addresses and all they may ask.
            for elsewhere in 2..4 {
                let session = Token::generate().unwrap();
                limits.issued(&session, now);
                for _ in 0..own {
                    limits
                        .request(&request, Some(&session), Some(place(elsewhere)), now)
                        .expect("a session elsewhere is served");
                }
            }
            // Then the window is spent, and by served requests only.
            let last = Token::generate().unwrap();
            limits.issued(&last, now);
            assert_eq!(
                limits.request(&request, Some(&last), Some(place(9)), now),
                Err(Error::Busy)
            );
            // A second later the place has its half again.
            let later = now + Duration::from_secs(1);
            let again = Token::generate().unwrap();
            limits.issued(&again, later);
            limits
                .request(&request, Some(&again), Some(place(1)), later)
                .expect("the place's next second");
        }
    }

    /// The share keeps out nobody it should not: a gateway alone at its
    /// address asks all its own budget allows, second after second, and so
    /// do two gateways behind one address, which is why the share is half
    /// a window and not a quarter.
    #[test]
    fn a_gateway_alone_and_two_behind_one_address_keep_their_whole_budgets() {
        let start = Instant::now();
        for gateways in [1, 2] {
            let mut limits = Limits::default();
            let credentials: Vec<Token> =
                (0..gateways).map(|_| Token::generate().unwrap()).collect();
            for second in 0..5 {
                let now = start + Duration::from_secs(second);
                for credential in &credentials {
                    limits.issued(credential, now);
                    for _ in 0..BEARER_CONTROL {
                        limits
                            .request(&check(), Some(credential), Some(place(7)), now)
                            .expect("a gateway's check of a player");
                    }
                    for _ in 0..BEARER_DATA {
                        let asked = Request::GatewayStatus;
                        limits
                            .request(&asked, Some(credential), Some(place(7)), now)
                            .expect("a gateway's own request");
                    }
                }
            }
        }
    }

    /// A household: six signed-in clients behind one address, each asking
    /// eight control and eight data requests every second, far more than a
    /// client's polling. None is refused, in any second.
    #[test]
    fn a_household_s_clients_are_served_within_their_address_s_share() {
        let start = Instant::now();
        let mut limits = Limits::default();
        let home = Some(place(3));
        let sessions: Vec<Token> = (0..6).map(|_| Token::generate().unwrap()).collect();
        for session in &sessions {
            limits.issued(session, start);
        }
        for second in 0..10 {
            let now = start + Duration::from_secs(second);
            for _ in 0..8 {
                for session in &sessions {
                    limits
                        .request(&Request::SessionStatus, Some(session), home, now)
                        .expect("a client's status");
                    limits
                        .request(&Request::ReadDocument, Some(session), home, now)
                        .expect("a client's read");
                }
            }
        }
    }

    /// The official gateway runs on Central's host. Its bridge asks its
    /// whole budget whether its requests come with the host's own address
    /// or with none, and also while another address spends its half of
    /// the window: a request the listener knows no place of has no share
    /// to draw on, and the bridge's address holds nobody else.
    #[test]
    fn the_official_gateway_on_central_s_host_is_served_beside_a_full_address() {
        let now = Instant::now();
        for host in [None, Some(Place::V4([127, 0, 0, 1]))] {
            let bridge = Token::generate().unwrap();
            let mut limits = Limits::keeping(bridge.digest());
            for _ in 0..2 {
                let bearer = Token::generate().unwrap();
                limits.issued(&bearer, now);
                for _ in 0..BEARER_CONTROL {
                    limits
                        .request(&Request::SessionStatus, Some(&bearer), Some(place(1)), now)
                        .expect("within the address's half");
                }
            }
            let third = Token::generate().unwrap();
            limits.issued(&third, now);
            assert_eq!(
                limits.request(&Request::SessionStatus, Some(&third), Some(place(1)), now),
                Err(Error::Busy)
            );
            for _ in 0..BEARER_CONTROL {
                limits
                    .request(&check(), Some(&bridge), host, now)
                    .expect("the bridge's check of a player");
            }
        }
    }

    /// A gateway's check of one of its players' leases.
    fn check() -> Request {
        Request::GatewaySession {
            lease: Token::generate().unwrap(),
            purpose: central_api::wire::TicketPurpose::Admit,
        }
    }

    /// The places' windows of the shared budgets live a second and are
    /// bounded: a served request opens one, the next second forgets it.
    #[test]
    fn a_place_s_share_of_the_shared_windows_is_forgotten_after_its_second() {
        let start = Instant::now();
        let mut limits = Limits::default();
        let session = Token::generate().unwrap();
        limits.issued(&session, start);
        let mut refused = 0;
        for n in 0..SHARED_DATA {
            let from = Some(place(n));
            if limits
                .request(&Request::ReadDocument, Some(&session), from, start)
                .is_err()
            {
                refused += 1;
            }
        }
        // One bearer's own budget was served, each request from another
        // address; a refused request opened no window.
        assert_eq!(refused, SHARED_DATA - BEARER_DATA);
        assert_eq!(limits.sharing.len(), BEARER_DATA as usize);
        limits
            .request(
                &Request::ReadDocument,
                Some(&session),
                Some(place(0)),
                start + Duration::from_secs(1),
            )
            .unwrap();
        assert_eq!(limits.sharing.len(), 1);
        // An IPv6 subscriber's /64 is a place like any other.
        let later = start + Duration::from_secs(2);
        let subscriber = Some(Place::V6([0x20, 0x01, 0x0d, 0xb8, 0, 1, 0, 2]));
        let mut served = 0;
        for _ in 0..4 {
            let bearer = Token::generate().unwrap();
            limits.issued(&bearer, later);
            for _ in 0..BEARER_DATA {
                if limits
                    .request(&Request::ReadDocument, Some(&bearer), subscriber, later)
                    .is_ok()
                {
                    served += 1;
                }
            }
        }
        assert_eq!(served, PLACE_DATA);
    }

    /// Supplying a token is not being signed in. Requests with bearers
    /// nobody issued have a second of their own, however many addresses
    /// they come from: the budgets sessions share, the room among their
    /// bearers and the sign-ins' budget stay whole.
    #[test]
    fn bearers_nobody_issued_spend_nothing_of_what_sessions_share() {
        let now = Instant::now();
        let mut limits = Limits::default();
        let session = Token::generate().unwrap();
        limits.issued(&session, now);
        // One address, 2,000 made-up bearers in one second.
        let mut passed = 0;
        for index in 0..2_000 {
            let request = if index % 2 == 0 {
                Request::SessionStatus
            } else {
                Request::CreateObject
            };
            let invented = Token::generate().unwrap();
            if limits
                .request(&request, Some(&invented), Some(place(1)), now)
                .is_ok()
            {
                passed += 1;
            }
        }
        assert_eq!(passed, UNPROVEN_PER_SECOND);
        // Nothing more this second from other addresses, or from none.
        for from in (2..200).map(|n| Some(place(n))).chain([None]) {
            let invented = Token::generate().unwrap();
            assert_eq!(
                limits.request(&Request::SessionStatus, Some(&invented), from, now),
                Err(Error::Busy)
            );
        }
        // The session beside them is served its whole budget of each kind,
        // from the flood's own address too.
        for _ in 0..BEARER_CONTROL {
            limits
                .request(&Request::SessionStatus, Some(&session), Some(place(1)), now)
                .unwrap();
        }
        for _ in 0..BEARER_DATA {
            limits
                .request(&Request::CreateObject, Some(&session), Some(place(1)), now)
                .unwrap();
        }
        // No made-up bearer took room among the sessions' bearers.
        assert_eq!(limits.peers.len(), 2);
        // And those who are signing in have their whole second.
        let discover = Request::Discover {
            username: "synthetic".into(),
        };
        assert_eq!(
            served(&mut limits, &discover, Some(place(1)), now, 40, 0),
            GLOBAL_PER_SECOND
        );
        // A refusal spent nothing: the next second is whole again.
        let next = now + Duration::from_secs(1);
        let invented = Token::generate().unwrap();
        limits
            .request(
                &Request::SessionStatus,
                Some(&invented),
                Some(place(1)),
                next,
            )
            .unwrap();
    }

    /// One address has its share of the strangers' requests in a minute
    /// and no more; another address, and a request the listener knows no
    /// place of, are served meanwhile.
    #[test]
    fn one_place_has_its_share_of_the_strangers_requests() {
        let start = Instant::now();
        let mut limits = Limits::default();
        let mut passed = 0;
        // 25 a second, well within the second all strangers share.
        for index in 0..700_u64 {
            let now = start + Duration::from_millis(index * 40);
            let invented = Token::generate().unwrap();
            if limits
                .request(
                    &Request::SessionStatus,
                    Some(&invented),
                    Some(place(1)),
                    now,
                )
                .is_ok()
            {
                passed += 1;
            }
        }
        assert_eq!(passed, PLACE_UNPROVEN_PER_MINUTE);
        let later = start + Duration::from_secs(30);
        let invented = Token::generate().unwrap();
        assert_eq!(
            limits.request(
                &Request::ReadDocument,
                Some(&invented),
                Some(place(1)),
                later
            ),
            Err(Error::Busy)
        );
        for from in [Some(place(2)), None] {
            limits
                .request(&Request::ReadDocument, Some(&invented), from, later)
                .unwrap();
        }
        // The share is the strangers' alone: the place signs in meanwhile,
        // and a session there is served.
        limits
            .request(&Request::Terms, None, Some(place(1)), later)
            .unwrap();
        let session = Token::generate().unwrap();
        limits.issued(&session, later);
        limits
            .request(
                &Request::SessionStatus,
                Some(&session),
                Some(place(1)),
                later,
            )
            .unwrap();
        // Its minute over, the place is served again.
        limits
            .request(
                &Request::ReadDocument,
                Some(&invented),
                Some(place(1)),
                start + Duration::from_secs(60),
            )
            .unwrap();
    }

    /// Whether `token` counts as a session's: its control requests then
    /// have the bearer's own budget, a stranger's the strangers' second.
    fn vouched(limits: &Limits, token: &Token, now: Instant) -> bool {
        limits.proven.holds(&token.digest(), now)
    }

    /// An answer, and the store saying the bearer is nobody's gateway's.
    fn answer(limits: &mut Limits, request: &Request, token: &Token, refused: Option<Error>) {
        answer_of(limits, request, token, refused, None);
    }

    /// An answer, and the store saying whose gateways the bearer is of.
    fn answer_of(
        limits: &mut Limits,
        request: &Request,
        token: &Token,
        refused: Option<Error>,
        owner: Option<[u8; 16]>,
    ) {
        let told = told(request, Some(token)).expect("counted by its bearer");
        let now = Instant::now();
        if limits.answered(&told, refused.as_ref(), now) {
            limits.vouch(&told, owner, now);
        }
    }

    /// One account holds eight gateways and eight tokens for each, and its
    /// own two sessions. Counted one by one, four of the tokens filled
    /// what all sessions share. Counted together they have one bearer's
    /// budget, from however many addresses, and with the account's two
    /// sessions that is three budgets of the four a window holds.
    #[test]
    fn one_account_s_gateway_bearers_are_counted_together() {
        let now = Instant::now();
        for (request, own, shared) in [
            (check(), BEARER_CONTROL, SHARED_CONTROL),
            (Request::GatewayStatus, BEARER_DATA, SHARED_DATA),
        ] {
            let mut limits = Limits::default();
            let owner = [7; 16];
            let mut served = 0;
            for gateway in 0..8 {
                let token = Token::generate().unwrap();
                answer_of(
                    &mut limits,
                    &Request::GatewayStatus,
                    &token,
                    None,
                    Some(owner),
                );
                assert!(vouched(&limits, &token, now));
                for _ in 0..own {
                    if limits
                        .request(&request, Some(&token), Some(place(gateway)), now)
                        .is_ok()
                    {
                        served += 1;
                    }
                }
            }
            assert_eq!(served, own);
            // The owner's two sessions beside them, at two more addresses.
            for session in 0..2 {
                let token = Token::generate().unwrap();
                limits.issued(&token, now);
                assert_eq!(
                    served_with(&mut limits, &request, &token, place(20 + session), now, own),
                    own
                );
            }
            // A quarter of the window is left for someone else's session.
            let other = Token::generate().unwrap();
            limits.issued(&other, now);
            assert_eq!(
                served_with(&mut limits, &request, &other, place(30), now, shared),
                own
            );
            // Another account's gateway has a budget of its own.
            let later = now + Duration::from_secs(1);
            let theirs = Token::generate().unwrap();
            answer_of(
                &mut limits,
                &Request::GatewayStatus,
                &theirs,
                None,
                Some([8; 16]),
            );
            let mine = Token::generate().unwrap();
            answer_of(
                &mut limits,
                &Request::GatewayStatus,
                &mine,
                None,
                Some(owner),
            );
            assert_eq!(
                served_with(&mut limits, &request, &mine, place(1), later, own),
                own
            );
            assert_eq!(
                served_with(&mut limits, &request, &theirs, place(2), later, own),
                own
            );
        }
    }

    /// `count` requests with `bearer` at one instant; how many were served.
    fn served_with(
        limits: &mut Limits,
        request: &Request,
        bearer: &Token,
        from: Place,
        now: Instant,
        count: u32,
    ) -> u32 {
        let mut served = 0;
        for _ in 0..count {
            if limits
                .request(request, Some(bearer), Some(from), now)
                .is_ok()
            {
                served += 1;
            }
        }
        served
    }

    /// An answer vouches for a bearer only once the store has said whose
    /// it is. Until then, and when the store could not be asked, the
    /// bearer waits with the strangers; one that is vouched for is never
    /// asked about again, and keeps its owner when it is vouched for anew.
    #[test]
    fn a_bearer_is_vouched_for_once_the_store_said_whose_it_is() {
        let now = Instant::now();
        let bridge = Token::generate().unwrap();
        let mut limits = Limits::keeping(bridge.digest());
        let token = Token::generate().unwrap();
        let status = told(&Request::GatewayStatus, Some(&token)).unwrap();
        assert!(limits.answered(&status, None, now), "nobody knows whose");
        assert!(!vouched(&limits, &token, now), "and nobody was asked yet");
        // The store did not answer: asked again with the next answer.
        assert!(limits.answered(&status, None, now));
        limits.vouch(&status, Some([7; 16]), now);
        assert!(vouched(&limits, &token, now));
        assert!(!limits.answered(&status, None, now), "known from here on");
        assert!(limits.proven.unit(&token.digest(), now) == Some(Unit::Owner([7; 16])));
        // A refusal, or an answer that tells nothing, asks nobody.
        let stranger = Token::generate().unwrap();
        let refused = told(&Request::GatewayStatus, Some(&stranger)).unwrap();
        assert!(!limits.answered(&refused, Some(&Error::Rejected), now));
        let silent = told(&Request::KeyCheckStatus, Some(&stranger));
        assert!(silent.is_none_or(|silent| !limits.answered(&silent, None, now)));
        // The bridge is configured: nobody is asked whose it is.
        let beat = told(&Request::GatewayStatus, Some(&bridge)).unwrap();
        assert!(!limits.answered(&beat, None, now));
        assert!(limits.proven.unit(&bridge.digest(), now) == Some(Unit::Bearer(bridge.digest())));
        // Four refusals take the vouching back, and the owner with it:
        // the next answer asks the store again.
        for _ in 0..REFUSALS {
            assert!(!limits.answered(&status, Some(&Error::Rejected), now));
        }
        assert!(!vouched(&limits, &token, now));
        assert!(limits.answered(&status, None, now));
    }

    /// A gateway asks about its players with its own credential, and a
    /// player's made-up lease or ticket is refused with the `Rejected` a
    /// dead credential gets. Such refusals do not count against the
    /// gateway: no player takes its credential out of the vouched-for.
    #[test]
    fn a_player_s_bad_ticket_is_not_held_against_the_gateway_that_asked() {
        let now = Instant::now();
        let mut limits = Limits::default();
        let gateway = Token::generate().unwrap();
        let of_a_player = || Token::generate().unwrap();
        answer(&mut limits, &Request::GatewayStatus, &gateway, None);
        assert!(vouched(&limits, &gateway, now));
        for _ in 0..4 * REFUSALS {
            for asked in [
                Request::GatewaySession {
                    lease: of_a_player(),
                    purpose: central_api::wire::TicketPurpose::Admit,
                },
                Request::GatewayProfile {
                    lease: of_a_player(),
                    purpose: central_api::wire::TicketPurpose::Admit,
                },
                Request::GatewayAuthorize {
                    lease: of_a_player(),
                    purpose: central_api::wire::TicketPurpose::Admit,
                    terms: None,
                },
                Request::RedeemGateway {
                    ticket: of_a_player(),
                    purpose: central_api::wire::TicketPurpose::Admit,
                },
            ] {
                answer(&mut limits, &asked, &gateway, Some(Error::Rejected));
            }
        }
        assert!(vouched(&limits, &gateway, now));
        // What is the gateway's own still counts: its status, refused four
        // times in a row, says the credential ended.
        for _ in 0..REFUSALS {
            answer(
                &mut limits,
                &Request::GatewayStatus,
                &gateway,
                Some(Error::Rejected),
            );
        }
        assert!(!vouched(&limits, &gateway, now));
    }

    #[test]
    fn an_answer_vouches_for_a_bearer_and_refusals_take_it_back() {
        let now = Instant::now();
        let mut limits = Limits::default();
        let token = Token::generate().unwrap();
        // Asking vouches for nothing, and neither does an answer to a
        // request that is not on the list, nor one that anyone is given.
        limits
            .request(&Request::SessionStatus, Some(&token), None, now)
            .unwrap();
        answer(&mut limits, &Request::AccountExport, &token, None);
        answer(&mut limits, &Request::Logout, &token, None);
        assert!(!vouched(&limits, &token, now));
        // A refusal vouches for nothing either.
        answer(
            &mut limits,
            &Request::SessionStatus,
            &token,
            Some(Error::Rejected),
        );
        assert!(!vouched(&limits, &token, now));
        // A session's status, answered, does.
        answer(&mut limits, &Request::SessionStatus, &token, None);
        assert!(vouched(&limits, &token, now));
        // Three refusals in a row leave it, whatever else was not served.
        for refused in [
            Error::Rejected,
            Error::Busy,
            Error::Rejected,
            Error::Unavailable,
            Error::Conflict,
            Error::TermsRequired,
            Error::Rejected,
        ] {
            answer(&mut limits, &Request::ReadDocument, &token, Some(refused));
        }
        assert!(vouched(&limits, &token, now));
        // An answer that vouches starts the count again.
        answer(&mut limits, &Request::ReadDocument, &token, None);
        for _ in 0..REFUSALS - 1 {
            answer(
                &mut limits,
                &Request::SetDisplayName {
                    display_name: "Synthetic".into(),
                },
                &token,
                Some(Error::Rejected),
            );
        }
        assert!(vouched(&limits, &token, now));
        // The fourth in a row takes it back.
        answer(
            &mut limits,
            &Request::SessionStatus,
            &token,
            Some(Error::Rejected),
        );
        assert!(!vouched(&limits, &token, now));
        // So does the session's end, at once: its logout, a pairing's
        // cancelling (answered alike for any token), a password change's
        // finish and a deletion's.
        for ending in [
            Request::Logout,
            Request::PairCancel {
                handle: Token::generate().unwrap(),
            },
            Request::ChangePasswordFinish {
                attempt: Token::generate().unwrap(),
                proof: Vec::new(),
                record: Vec::new(),
                wrapped_master: Vec::new(),
            },
            Request::DeleteAccountFinish {
                attempt: Token::generate().unwrap(),
                proof: Vec::new(),
            },
        ] {
            limits.issued(&token, now);
            assert!(vouched(&limits, &token, now));
            answer(&mut limits, &ending, &token, None);
            assert!(!vouched(&limits, &token, now));
        }
        // A request that is not counted by its bearer says nothing of it.
        for request in [
            Request::Terms,
            Request::Discover {
                username: "synthetic".into(),
            },
            Request::PairPoll {
                handle: Token::generate().unwrap(),
            },
        ] {
            assert!(told(&request, Some(&token)).is_none());
        }
        assert!(told(&Request::SessionStatus, None).is_none());
    }

    #[test]
    fn a_bearer_is_vouched_for_for_half_an_hour_after_its_last_answer() {
        let start = Instant::now();
        let at = |seconds| start + Duration::from_secs(seconds);
        let mut limits = Limits::default();
        let token = Token::generate().unwrap();
        limits.issued(&token, start);
        assert!(vouched(&limits, &token, at(PROVEN_SECONDS - 1)));
        assert!(!vouched(&limits, &token, at(PROVEN_SECONDS)));
        // Every answer that vouches starts the half hour again.
        let told = told(&Request::RenewSession, Some(&token)).unwrap();
        assert!(
            !limits.answered(&told, None, at(600)),
            "held: nobody is asked"
        );
        assert!(vouched(&limits, &token, at(600 + PROVEN_SECONDS - 1)));
        assert!(!vouched(&limits, &token, at(600 + PROVEN_SECONDS)));
        // After it the session's next request is a stranger's, is served
        // as one, and its answer vouches for it again.
        let later = at(600 + PROVEN_SECONDS);
        limits
            .request(&Request::RenewSession, Some(&token), Some(place(1)), later)
            .unwrap();
        assert!(limits.peers.is_empty());
        assert!(limits.answered(&told, None, later), "whose it is, is asked");
        limits.vouch(&told, None, later);
        limits
            .request(&Request::RenewSession, Some(&token), Some(place(1)), later)
            .unwrap();
        assert_eq!(limits.peers.len(), 1);
    }

    #[test]
    fn the_bridge_is_vouched_for_from_the_start_and_for_good() {
        let now = Instant::now();
        let bridge = Token::generate().unwrap();
        let mut limits = Limits::keeping(bridge.digest());
        assert!(vouched(&limits, &bridge, now));
        for _ in 0..2 * REFUSALS {
            answer(
                &mut limits,
                &Request::RedeemGateway {
                    ticket: Token::generate().unwrap(),
                    purpose: central_api::wire::TicketPurpose::Admit,
                },
                &bridge,
                Some(Error::Rejected),
            );
        }
        answer(&mut limits, &Request::Logout, &bridge, None);
        let later = now + Duration::from_secs(10 * PROVEN_SECONDS);
        assert!(vouched(&limits, &bridge, later));
        // No other token is.
        assert!(!vouched(&limits, &Token::generate().unwrap(), now));
    }

    #[test]
    fn the_vouched_for_bearers_are_bounded_and_forgotten() {
        let start = Instant::now();
        let digest = |n: usize| {
            let mut digest = [0; 32];
            digest[..8].copy_from_slice(&u64::try_from(n).unwrap().to_be_bytes());
            digest
        };
        let mut proven = Proven::default();
        for n in 0..PROVEN {
            // The first is the oldest; the rest came a second later.
            proven.mark(
                digest(n),
                None,
                start + Duration::from_secs(u64::from(n > 0)),
            );
        }
        assert_eq!(proven.known.len(), PROVEN);
        let soon = start + Duration::from_secs(2);
        // Vouched for again, a bearer needs no new room.
        proven.mark(digest(1), None, soon);
        assert_eq!(proven.known.len(), PROVEN);
        assert!(proven.holds(&digest(0), soon));
        // One more takes the place of the one last vouched for longest ago.
        proven.mark(digest(PROVEN), None, soon);
        assert_eq!(proven.known.len(), PROVEN);
        assert!(!proven.holds(&digest(0), soon));
        assert!(proven.holds(&digest(1), soon) && proven.holds(&digest(PROVEN), soon));
        // Half an hour on, the next one sweeps everything that passed.
        let later = start + Duration::from_secs(PROVEN_SECONDS + 60);
        proven.mark(digest(7), None, later);
        assert_eq!(proven.known.len(), 1);
    }

    fn place(n: u32) -> Place {
        Place::V4(n.to_be_bytes())
    }

    /// `count` requests of `place`, one every `step` milliseconds from
    /// `from` on; how many were served.
    fn served(
        limits: &mut Limits,
        request: &Request,
        from: Option<Place>,
        start: Instant,
        count: u32,
        step: u64,
    ) -> u32 {
        let mut served = 0;
        for index in 0..u64::from(count) {
            let now = start + Duration::from_millis(index * step);
            if limits.request(request, None, from, now).is_ok() {
                served += 1;
            }
        }
        served
    }

    #[test]
    fn one_place_draws_its_share_of_what_everyone_signs_in_with_and_no_more() {
        let start = Instant::now();
        let mut limits = Limits::default();
        let discover = Request::Discover {
            username: "synthetic".into(),
        };
        // As fast as the shared budget lets one address go (32 a second):
        // 600 in the minute, whatever it asks, and nothing after.
        assert_eq!(
            served(&mut limits, &discover, Some(place(1)), start, 700, 40),
            PLACE_PER_MINUTE
        );
        let later = start + Duration::from_secs(30);
        for request in [
            &discover,
            &Request::Terms,
            &Request::SessionStatus,
            &Request::LoginFinish {
                attempt: Token::generate().unwrap(),
                message: Vec::new(),
            },
        ] {
            assert_eq!(
                limits.request(request, None, Some(place(1)), later),
                Err(Error::Busy)
            );
            // Everyone else is served meanwhile, and so is a request the
            // listener knows no place of.
            limits
                .request(request, None, Some(place(2)), later)
                .unwrap();
            limits.request(request, None, None, later).unwrap();
        }
        // A request with a session draws on its session's budgets instead.
        let token = Token::generate().unwrap();
        limits.issued(&token, later);
        limits
            .request(&Request::SessionStatus, Some(&token), Some(place(1)), later)
            .unwrap();
        // Its minute over, the place is served again.
        let next = start + Duration::from_secs(60);
        limits
            .request(&discover, None, Some(place(1)), next)
            .unwrap();
        // Refusals spent nothing of anyone's: the second is still whole.
        assert_eq!(
            served(&mut limits, &discover, Some(place(3)), next, 40, 0),
            31
        );
    }

    #[test]
    fn a_report_s_routes_draw_on_the_same_share() {
        let start = Instant::now();
        let mut limits = Limits::default();
        let mut passed = 0;
        for index in 0..700_u64 {
            let now = start + Duration::from_millis(index * 40);
            if limits.global(Some(place(1)), now).is_ok() {
                passed += 1;
            }
        }
        assert_eq!(passed, PLACE_PER_MINUTE);
        let later = start + Duration::from_secs(30);
        assert_eq!(limits.global(Some(place(1)), later), Err(Error::Busy));
        assert_eq!(
            limits.request(&Request::Terms, None, Some(place(1)), later),
            Err(Error::Busy)
        );
        limits.global(Some(place(2)), later).unwrap();
    }

    /// Four addresses that keep up their share spend the second everyone
    /// signs in with. A place that asked little in its minute still has a
    /// few requests a second beside them: a household signs in meanwhile.
    /// The four get nothing of it, nor does a request without a place.
    #[test]
    fn a_place_that_asked_little_still_signs_in_while_four_spend_the_second() {
        let start = Instant::now();
        let mut limits = Limits::default();
        let discover = Request::Discover {
            username: "synthetic".into(),
        };
        // Six seconds of eight a second each: 32 a second between them,
        // and each of them past what counts as little.
        for second in 0..=5 {
            let now = start + Duration::from_secs(second);
            for flooding in 1..=4 {
                assert_eq!(
                    served(&mut limits, &discover, Some(place(flooding)), now, 8, 0),
                    8
                );
            }
        }
        let now = start + Duration::from_secs(5);
        // The second is spent: nothing for the four, nothing without a place.
        assert_eq!(
            limits.request(&discover, None, Some(place(1)), now),
            Err(Error::Busy)
        );
        assert_eq!(limits.request(&discover, None, None, now), Err(Error::Busy));
        // A sign-in from somewhere else is three to five such requests.
        for _ in 0..5 {
            limits
                .request(&discover, None, Some(place(20)), now)
                .expect("a place that asked little is served");
        }
        // The spare requests are a few a second, for all such places.
        assert_eq!(
            served(&mut limits, &discover, Some(place(21)), now, 9, 0),
            RESERVE_PER_SECOND - 5
        );
        assert_eq!(
            limits.request(&discover, None, Some(place(22)), now),
            Err(Error::Busy)
        );
    }

    /// The spare requests are for a place only while it has asked little:
    /// past [`LIGHT_PER_MINUTE`] in its minute it shares the second with
    /// everyone again, so no address draws more of them than that.
    #[test]
    fn a_place_draws_the_spare_requests_only_while_it_asked_little() {
        let start = Instant::now();
        let mut limits = Limits::default();
        let mut through = 0;
        for tenth in 0..100_u64 {
            let now = start + Duration::from_millis(tenth * 100);
            // Everyone's second is spent at once, by requests without a place.
            while limits.global(None, now).is_ok() {}
            if limits.global(Some(place(5)), now).is_ok() {
                through += 1;
            }
        }
        assert_eq!(through, LIGHT_PER_MINUTE);
        // Its minute over, it counts as a place that asked little again.
        let next = start + MINUTE;
        while limits.global(None, next).is_ok() {}
        limits.global(Some(place(5)), next).unwrap();
    }

    /// An IPv6 subscriber often holds a /56 or a /48: hundreds or
    /// thousands of /64s, each a place of its own. For the spare requests
    /// they are one: how little a place asked is counted by the /48
    /// around it, so one block draws what one address does.
    #[test]
    fn an_ipv6_block_draws_the_spare_requests_as_one_place() {
        let start = Instant::now();
        let mut limits = Limits::default();
        let subscriber =
            |block: u8, part: u8| Some(Place::V6([0x20, 0x01, 0x0d, 0xb8, 0, block, 0, part]));
        let mut through = 0;
        for tenth in 0..100_u8 {
            let now = start + Duration::from_millis(u64::from(tenth) * 100);
            while limits.global(None, now).is_ok() {}
            // Another /64 of the same /48 each time.
            if limits.global(subscriber(1, tenth), now).is_ok() {
                through += 1;
            }
        }
        assert_eq!(through, LIGHT_PER_MINUTE);
        // Another block, and an IPv4 address, have asked nothing yet.
        let now = start + Duration::from_secs(10);
        while limits.global(None, now).is_ok() {}
        limits.global(subscriber(2, 0), now).unwrap();
        limits.global(Some(place(6)), now).unwrap();
    }

    #[test]
    fn one_place_polls_four_pairings_and_spends_no_sign_in() {
        let start = Instant::now();
        let mut limits = Limits::default();
        let poll = Request::PairPoll {
            handle: Token::generate().unwrap(),
        };
        assert_eq!(
            served(&mut limits, &poll, Some(place(1)), start, 300, 20),
            PLACE_PAIRING_PER_MINUTE
        );
        let later = start + Duration::from_secs(30);
        assert_eq!(
            limits.request(&poll, None, Some(place(1)), later),
            Err(Error::Busy)
        );
        limits.request(&poll, None, Some(place(2)), later).unwrap();
        // The polls spent nothing of the place's other requests.
        limits
            .request(&Request::Terms, None, Some(place(1)), later)
            .unwrap();
    }

    #[test]
    fn places_are_forgotten_after_their_minute_and_none_is_refused_for_room() {
        let start = Instant::now();
        let mut places = Places::default();
        for n in 0..u32::try_from(PLACES).unwrap() {
            places.spend(Some(place(n)), start);
        }
        assert_eq!(places.windows.len(), PLACES);
        // Full of live minutes (more than the shared budget lets in): a
        // newcomer takes the oldest one's room.
        let soon = start + Duration::from_secs(1);
        places.spend(Some(place(u32::MAX)), soon);
        assert_eq!(places.windows.len(), PLACES);
        assert!(!places.fits(Some(place(u32::MAX)), soon, 1));
        // A minute on, the next request sweeps everything that passed.
        let later = start + Duration::from_secs(61);
        places.spend(Some(place(7)), later);
        assert_eq!(places.windows.len(), 1);
        // No place: nothing is kept, and nothing refuses.
        places.spend(None, later);
        assert!(places.fits(None, later, 0));
        assert_eq!(places.windows.len(), 1);
    }
    #[test]
    fn supplying_a_bearer_never_bypasses_the_authentication_budget() {
        let now = Instant::now();
        let mut limits = Limits::default();
        let token = Token::generate().unwrap();
        let request = Request::Discover {
            username: "synthetic".into(),
        };
        for _ in 0..32 {
            limits.request(&request, Some(&token), None, now).unwrap();
        }
        assert_eq!(limits.request(&request, None, None, now), Err(Error::Busy));
        assert_eq!(
            limits.request(&request, Some(&Token::generate().unwrap()), None, now),
            Err(Error::Busy)
        );
    }
    /// A registration challenge spends the budget all callers share,
    /// whichever way it is asked and whatever token comes with it: the
    /// table of challenges is sized by that budget.
    #[test]
    fn a_registration_challenge_spends_the_global_budget_with_or_without_a_token() {
        let now = Instant::now();
        for request in [
            Request::RegisterChallenge,
            Request::RegisterChallengeFor { signals: 0 },
        ] {
            let mut limits = Limits::default();
            for _ in 0..GLOBAL_PER_SECOND / 2 {
                limits.request(&request, None, None, now).unwrap();
                limits
                    .request(&request, Some(&Token::generate().unwrap()), None, now)
                    .unwrap();
            }
            assert_eq!(limits.request(&request, None, None, now), Err(Error::Busy));
            assert_eq!(
                limits.request(&request, Some(&Token::generate().unwrap()), None, now),
                Err(Error::Busy)
            );
            limits
                .request(&request, None, None, now + Duration::from_secs(1))
                .unwrap();
        }
    }

    #[test]
    fn the_figures_spend_the_public_budget_and_a_pulse_a_heartbeat_s() {
        let now = Instant::now();
        let mut limits = Limits::default();
        let token = Token::generate().unwrap();
        let figures = Request::PublicFigures { after: None };
        // Public: a bearer sent along buys no budget of its own.
        for _ in 0..GLOBAL_PER_SECOND {
            limits.request(&figures, Some(&token), None, now).unwrap();
        }
        for request in [&figures, &Request::GatewayDirectory { after: None }] {
            assert_eq!(limits.request(request, None, None, now), Err(Error::Busy));
            assert_eq!(
                limits.request(request, Some(&token), None, now),
                Err(Error::Busy)
            );
        }
        // A pulse and a heartbeat are one credential's same 64 a second.
        limits.issued(&token, now);
        let metadata = || central_api::gateway::GatewayMetadata {
            name: "Synthetic".into(),
            version: "1".into(),
            website: "https://gateway.invalid/".into(),
        };
        let pulse = Request::GatewayPulse {
            metadata: metadata(),
            activity: central_api::gateway::GatewayActivity {
                players: 1,
                playing: 1,
                tables: 1,
                games: 1,
            },
        };
        let heartbeat = Request::GatewayHeartbeat {
            metadata: metadata(),
        };
        for sent in 0..64 {
            let request = if sent % 2 == 0 { &pulse } else { &heartbeat };
            limits.request(request, Some(&token), None, now).unwrap();
        }
        for request in [&pulse, &heartbeat] {
            assert_eq!(
                limits.request(request, Some(&token), None, now),
                Err(Error::Busy)
            );
        }
        // Without a credential a pulse is one more public request, refused
        // here with the budget and by dispatch otherwise.
        assert_eq!(limits.request(&pulse, None, None, now), Err(Error::Busy));
        // Neither took anything from a session's checks.
        limits
            .request(&Request::SessionStatus, Some(&token), None, now)
            .unwrap();
        limits
            .request(&figures, None, None, now + Duration::from_secs(1))
            .unwrap();
    }

    #[test]
    fn rate_limits_expire_and_have_bounded_cardinality() {
        let now = Instant::now();
        let mut limits = Limits::default();
        for _ in 0..32 {
            limits.global(None, now).unwrap();
        }
        assert_eq!(limits.global(None, now), Err(Error::Busy));
        limits.global(None, now + Duration::from_secs(1)).unwrap();
        for _ in 0..8 {
            limits.account([0; 16], now, 1).unwrap();
        }
        assert_eq!(limits.account([0; 16], now, 1), Err(Error::Busy));
        // However many accounts there are, none keeps another out
        // (`guesses.rs` bounds the table and what a full one forgets).
        for i in 1_u128..2048 {
            limits.account(i.to_be_bytes(), now, 1).unwrap();
        }
        assert_eq!(limits.account([0; 16], now, 1), Err(Error::Busy));
        limits
            .account([0; 16], now + Duration::from_secs(60), 1)
            .unwrap();
    }
    #[test]
    fn legacy_sign_ins_are_bounded_per_name_and_altogether() {
        let now = Instant::now();
        let mut limits = Limits::default();
        for _ in 0..LEGACY_PER_NAME {
            limits.legacy("Synthetic", now).unwrap();
        }
        // The same name in another case is the same name.
        assert_eq!(limits.legacy("SYNTHETIC", now), Err(Error::Busy));
        for i in 1..(LEGACY_PER_MINUTE / LEGACY_PER_NAME) {
            for _ in 0..LEGACY_PER_NAME {
                limits.legacy(&format!("name{i}"), now).unwrap();
            }
        }
        assert_eq!(
            limits.legacy("fresh", now),
            Err(Error::Busy),
            "all names together are bounded too"
        );
        let later = now + Duration::from_secs(60);
        limits.legacy("synthetic", later).unwrap();
        assert_eq!(limits.legacy_names.len(), 1, "a passed minute is forgotten");
    }

    #[test]
    fn a_password_change_costs_two_sign_ins_and_nothing_when_refused() {
        let now = Instant::now();
        let mut limits = Limits::default();
        for _ in 0..3 {
            limits.account([1; 16], now, 2).unwrap();
        }
        limits.account([1; 16], now, 1).unwrap();
        // Seven spent: a change's two do not fit, and spend nothing.
        assert_eq!(limits.account([1; 16], now, 2), Err(Error::Busy));
        limits.account([1; 16], now, 1).unwrap();
        assert_eq!(limits.account([1; 16], now, 1), Err(Error::Busy));
        limits
            .account([1; 16], now + Duration::from_secs(60), 2)
            .unwrap();
    }
}
