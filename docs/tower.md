# The Verdant Tower

Design milestone, 13 November 2026. Implementation is in progress; the contracts
below are acceptance criteria until the implementation evidence is recorded.

## The player loop

An expedition has five manually played duels. The third floor is a sentinel and
the fifth is the summit guardian. Every victory before the summit offers three
concrete deck improvements. The player sees exactly which cards change before
confirming; a chosen improvement persists through all remaining floors. The
opening and opponent policy are seeded and immutable for the chosen route.

Three lives make learning possible. A lost duel or a deliberate concession
costs one life, retains the current deck and offers a retry of that floor. A
technical void costs no life. Disconnect alone neither rerolls a floor nor
costs a life; the ordinary host's reconnection and abandonment rules apply.
There is no energy timer, purchase, daily entry limit or pay-for-power reward.
Starting another expedition requires explicitly ending the current one.

The expedition rotates weekly at the same UTC boundary everywhere. All players
receive the same base deck, encounters and upgrade offers. An existing run can
continue as practice after the week ends, but only victories completed during
the live week qualify for its ranking. No time pressure applies between floors.

Ranking compares cleared floors first, then fewer defeats, then total winning
turns. Equal values share a rank; no opaque weighted score mixes seconds and
turns. Continuous measured human time has its own optional sprint board. Only
the best run per player appears, and publication by display name is opt-in and
reversible. Paused/interrupted play still earns permanent accomplishments.

Permanent rewards count distinct expedition/floor milestones, distinct summits
and flawless summits. Replaying an easy floor cannot farm those tracks. They
have five increasingly ornate tiers, an explicit next target, and no streak
reset. The journal retains the player's best cleared floor and chosen route.

## Authority and bounded execution

Central owns publications, runs, upgrade selection, reservations, results,
rankings, consent and rewards. A gateway supplies neither a deck nor a claimed
win. Existing separately authenticated official execution, immutable private
packages, engine results and durable result delivery are reused.

The publisher constructs a bounded directed graph of certified encounter
variants. Upgrade counts commute: with three upgrade tracks and four choices,
five layers need 1 + 3 + 6 + 10 + 15 = 35 nodes, rather than an unbounded tree.
Every edge advances exactly one floor. Each node fixes its accumulated upgrades,
public player deck, enemy description and private execution package. The
generator validates every reachable node using real execution and replay before
publication. Central verifies graph shape, bounds and content binding; it does
not interpret Magic rules or claim to independently solve an encounter.

Publication is all-or-nothing and immutable. The official publisher stages one
certified node per request against the same immutable manifest. Activation
checks all 35 indexed digests in one transaction. A crash between uploads leaves
an invisible resumable draft, never an incomplete player activity. The existing
1,056,768-byte wire limit and 196,608-byte package limit remain unchanged.
No future private package, seed, opening order or proof record appears on public
or account response types. Withdrawal prevents new starts and treats in-flight
outcomes as technical voids without deleting already earned accomplishments.

A run is owned by one account. The account lock serializes start, choice,
concession, result accounting and account deletion. Each floor reservation has
an independent database UUIDv7. Requests retain their idempotency key across
ambiguous responses; choices use the expected run revision. An exact retry is
idempotent and a competing different choice conflicts. A client never chooses
the next node, executor binary, host origin or execution seed.

Run phases are ready, playing, choosing, conquered, exhausted and abandoned.
Only a verified terminal host result may clear a floor. Repeated delivery cannot
spend a second life, advance twice, replace the chosen upgrade or mint a reward.
A technical void returns to ready at the same node. The existing durable host
outbox continues to reconcile results after restarts. Missing execution capacity
does not count a failed duel or disclose a new opening.

## Interface

The Lobby holds one compact expedition entrance beside the calendar activities.
The tower screen presents a five-floor ascent, current encounter and supplied
deck, lives, run status and one primary next action. Cleared floors have stable
milestone marks; the summit is visually distinct. Upgrade offers are equal-sized
cards with a precise before/after summary and an explicit confirmation. Rankings
and permanent medals remain separate from gameplay instructions.

Native and web interfaces share the same authority and terminology. The web
page shows the current expedition, public rankings and signed-in progress,
with an explicit client launch link. Loading, unavailable capacity, empty scores,
withdrawal, expired session, interrupted duel and delayed result delivery each
have an understandable state. Keyboard focus, large German text, narrow layouts
and reduced motion are acceptance conditions, not follow-up work.

## Assurance and improvement passes

The full website CI run passed 718 cases and exposed three navigation-overflow
cases at the first unfolded desktop widths after adding Tower. The follow-up
keeps the full text size and target height while tightening horizontal link
padding from 1081 to 1279 px. The affected language and Japanese strip checks
then passed on desktop and phone (four cases, 20.6 seconds), with 1099/1100 px
added to the existing language breakpoint matrix. This correction was separately
reviewed; the other passing website cases were not repeated locally.

1. Rules and authority: contract, graph validation, ownership and migration.
2. Execution: real generated encounters, launch, recovery and durable outcomes.
3. Progression: upgrade choices, lives, score eligibility and permanent rewards.
4. Presentation: ascent, choices, result states and website/native parity.
5. Adversarial refinement: stale/duplicate requests, cross-account access,
   concurrent choices, replayed results, boundary rollover and actual play.

Keep changes small and record targeted evidence per pass. No new dependency,
first-party unsafe Rust, custom cryptographic primitive or production deployment
is part of this work. Account deletion cascades through private run metadata;
exports include owned run/choice/reward records. Logs contain no private package,
proof, session capability or player action trace.

### Pass 1 — Central authority (14 November 2026)

Implemented `central_api::tower`, migration 000029, bounded private staging and
activation, account-locked runs, revisioned choices, exact retry keys, floor
reservations, official result accounting, two score projections, reversible
season consent and three permanent reward tracks. A run from the preceding
week remains discoverable separately from the new week's landing page.

The existing challenge transport carries `Tower` player operations; only its
`Read` is anonymous. Begin/start/choose and affirmative publication consent need
accepted terms. Export, cancellation and withdrawal of consent remain possible
without accepting changed terms. `HostRequest::Tower` stays behind the native
official bridge boundary. `HostRequest::Executor` resolves a reservation's
immutable build without publishing its encounter as a calendar task.

Scope zero retains Daily/Weekly calendar uniqueness. Scopes 1–35 hold private
weekly expedition nodes. They cannot be read, started or hinted through the
calendar endpoints and never award calendar badges. The separate `tower` active
attempt kind prevents concurrent expedition hands without consuming the Weekly
slot. Ordinary attempt export explicitly distinguishes this activity.

Evidence: 15 targeted API/calendar tests passed, including all 81 routes through
the 35-node graph. `cargo xtask postgres --tower-only` passed against owned
PostgreSQL 18.6: staged draft invisibility, atomic activation, immutable content,
foreign-account rejection, concurrent idempotent starts/claims/results, competing
choices, lives, voids, best-per-player rankings, privacy, exports, duplicate
reward prevention, withdrawal and account deletion cascades. The test itself
took 2.60 seconds; the fixture verified container/listener removal afterwards.
Clippy with warnings denied passed for all targets of the API, store, identity,
SDK and xtask. No new dependency was introduced. Actual engine certification,
native/web presentation and end-to-end transport evidence remain subsequent
passes; this is not yet a playable or release-complete feature.

Operators apply migration 000029 and the matching `ops/grants.sql` together.
The runtime can insert/read immutable manifests and nodes, update only publication
and withdrawal flags on a season, and mutate only account-owned progression.
After an interrupted upload, rerun the identical staged manifest; do not replace
a conflicting weekly draft. A withdrawn season retains earned accomplishments.

### Recovery and withdrawal refinement

An authenticated `Locate { game }` follows the account-owned attempt's request
identity to its expedition. It restores the exact run after restart even when the
duel has finished or the week has changed; no public game-to-account lookup is
available. Individual tower nodes reject the calendar withdrawal operation, so
operators explicitly withdraw the complete season. The focused PostgreSQL 18.6
fixture passed these additions in 2.71 seconds and verified cleanup.

### Deadline and recovery adversarial pass

The extended PostgreSQL fixture passed in 4.61 seconds on 14 November. It
reconstructs a historical publication and claims using only the isolated
operator role. Production has no clock override. A victory delivered after the
week closes still qualifies when its authenticated finish precedes the deadline;
a finish at the exact boundary does not. Later practice preserves the prior
ranked prefix while granting distinct-floor rewards. The current-week response
still returns the previous week's active run. Foreign-account game lookup is
refused. Two complete timed summits receive the same rank on both boards.
The owned container and listener were removed. The complete SDK PostgreSQL
suite then passed in 107.66 seconds. Its following store suite found an obsolete
migration-count expectation in the search-path regression. After correction,
`cargo xtask postgres --store-only` passed in 78.23 seconds, including the new
populated-calendar upgrade preservation assertions. That owned container and
listener were also removed. The default PostgreSQL gate still runs both suites;
the narrow option avoids repeating successful transport work during a store fix.

### Web and native presentation

The native Lobby entrance, five-floor route, lives, exact three-way replacement
preview, explicit confirmation, recovery link and three original relic motifs
are implemented. Thirty-six focused client/core checks passed, including
keyboard navigation, deep links, account changes and uncertain mutation retries.
The subsequent local-state refactor passed all four Tower core tests again.

The website exposes only public season metadata in its server-side snapshot.
Owned progress and consent are loaded separately in the browser. Account export
includes three independently bounded Tower datasets with strict ascending UUIDv7
cursors and per-dataset completeness markers: runs (including the frozen ranked
prefix and creation metadata), accepted retry-safe actions, and participation
(including consent without a run and exact earned floors). The export and Trusted Types/inert-element
checks passed. Localized Tower copy and the privacy description cover all eleven
site languages.

The first real desktop/phone pass ran fourteen focused browser checks: eleven
passed, and three found duplicate ranking landmark names or an unfocusable
horizontal table. The correction gives the Tower ranking a distinct accessible
name and keyboard access, and hides empty table scaffolding. The inspected
desktop screenshot also prompted a full-width tier gallery, replacing the tall
single-card expansion, a smaller expedition title, and corrected pagination
labels. The fourth rendered pass then passed all four focused checks in 5.8
seconds. The fifth pass adds compact horizontal relic cards on phones, explicit
current-floor and summit markers, and an immediate cross-tab session generation
guard. All six desktop/phone checks passed in 14.2 seconds: SSR without WASM,
opaque links, private-state clearing with a deliberately late old response,
reversible consent, equal ranks, safe name rendering, stable focus and preview
through refresh, dark/light contrast, 44-pixel targets, 320-pixel layout and
reduced motion. Desktop and phone captures were inspected. The fixture's listener
was removed. TypeScript/lint and affected Rust clippy also passed.

The native fifth refinement passed nine targeted controls/core/publisher checks.
Subsequent rendered acceptance used the owner's explicitly approved passive main
display, virtual input and synthetic local accounts. At 1440 × 900 and
1280 × 720 logical pixels, inspection prompted a more prominent upgrade decision,
reachable confirmation and a corrected completion glyph. Native selection,
cancellation and Central-confirmed deck improvement, the completed route, ranking
and earned relics were verified. Five-tier animation and the reduced-motion
setting were checked using bounded recordings and identical-frame controls; the
full evidence and sampling limits are in Baylee's `docs/tower.md`. This is not
representative human difficulty evidence. All owned test services were stopped.
Independent PR review found and corrected season pagination, failed-consent
rollback, native upgrade retry visibility, ranking-week labeling, complete
personal exports and the season/reward foreign-key lock order. Host mutations
use `FOR NO KEY UPDATE`: season identities are immutable, and results already
holding a node share lock can still acquire the reward foreign-key key-share
lock. The controlled concurrent withdrawal/result regression and complete export
regressions passed in PostgreSQL 18.6 in 11.76 seconds; the fixture verified owned
container/listener removal. Static re-review closed both authority findings.

The full 35-node generated expedition now has at least three contributing wins
in at least two terminal families per node, each checked through fresh execution,
exact replay and reconstructed opponent policy. Observed witness turns range
from 9 to 33; these are existence witnesses, not optimal or human difficulty
measurements. The sibling Baylee's full challenge integration passed in 91.46
seconds against real isolated PostgreSQL/Central/gateway/agent/game servers.
It covered Daily/Weekly plus five Tower victories, four upgrades, exact retries,
owned recovery, disconnect/resume Sprint exclusion and permanent rewards. The
actual stock executable resumed a deliberately interrupted upload and its second
invocation published nothing. The fixture removed its owned services, database
and listener. Its first attempt exposed an outdated copy of runtime grants in
Baylee's fixture; that copy was aligned with migration 000029 before the pass.

Final browser regressions after the review corrections passed all twelve
desktop/phone cases in 11.1 seconds. This includes an acknowledged opt-in followed
by a failed refresh and failed opt-out: the checkbox retains the last confirmed
server state. New-season pagination returns to the first page while preserving
the old active run, and the downloadable account file includes all three Tower
datasets. All eight focused native/core cases passed as well (0.80/0.01 seconds).
All five independent review areas have no remaining reported findings after
their correction reviews; this is a code review, not an external security audit.
