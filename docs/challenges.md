# Daily and weekly challenges

Implementation work: coordinated with Baylee's `codex/open-beta-retention`
branch. This document tracks the contract and its acceptance evidence as the
feature is implemented; it is not a deployment instruction until the gates pass.

## Authority and invariants

Central owns the UTC calendar, immutable published challenge revisions,
account-bound attempts, leaderboards and permanent badge progress. The official
bridge alone publishes verified packages, claims attempts and submits results.
Directory approval and ordinary gateway credentials grant none of these powers.
Players reach the official challenge host from any gateway using their ordinary
Central sign-in; no Central bearer is forwarded to another gateway.

The Baylee generator proves candidates offline and publishes a private package.
It keeps exact decks, setup, independent opponent randomness, policy generation,
deterministic work limits and the matching executor build together. Central
does not run Magic rules. The official gateway's agent starts one ordinary
engine process per attempt; only that process decides the result.

Public catalogue/detail/leaderboard responses cannot contain packages, shuffle
seeds, hidden cards or solution records. Future challenges require bridge
authority. An account's active attempt is created before any private setup is
sent. Concurrent starts, lost responses and retries cannot buy a fresh hand or
erase failed attempts. Completion and its one-time progress are transactional.
Client clocks and user-supplied results never establish a score.

The official host persists the actual Unix-second finish time with its result
outbox. Central rejects a finish before its claim or more than five seconds
ahead of its clock, and stores receipt time separately. A retry after the UTC
deadline preserves a score completed before that deadline. Live windows remain
half-open, including for late delivery. Host and Central clocks must be synchronized.

Account writes lock the account before rechecking its session, in the same order
as deletion. A claim/result locks the account and shares the publication lock;
withdrawal takes the conflicting publication update lock. Public and personal
read pages use a read-only repeatable-read transaction. The public rankings show
only opt-in names/tags and share ranks for equal scores. A new hint affects active
and future attempts, while previously finished unassisted attempts stay valid.

`Challenges::Export` pages participation/completion metadata and attempts
separately in ascending UUID order, thirty rows at a time. It includes no private
package, witness, token or game record. Account deletion cascades all three
account-owned tables; public immutable challenge descriptions remain.

Publication is idempotent for the same period and digest and conflicts for a
different package. Withdrawal does not silently replace a running challenge or
erase legitimate completion progress. A technical abort is distinct from a loss.
Unfinished or uncertified generation is never published as a fallback.

## Deliverables

- Daily and more demanding weekly candidates, automatic advance generation,
  fresh executions and exact replay verification, three witnesses from at least
  two conservatively recognised solution families.
- Shared native/browser API, PostgreSQL persistence, UTC rollover and archive.
- Account-bound attempts and reconnection, first-win/best-turn/time rankings,
  assistance marking, exactly-once permanent tiered badges.
- Rust/Leptos website and the Bevy client's challenge browser, start/resume,
  result, rankings and badge progress. Original UI assets and translated words.
- Failure injection and real transport/database tests of generation through
  publication, play, evaluation, ranking and awards.

## Required improvement rounds before PR review

1. Functional completeness: walk the whole pipeline and close missing paths;
   test daily/weekly scheduling, multiple solutions, replay and exactly-once awards.
2. Player experience: use website and client, play selected tasks without reading
   witnesses; fix confusing controls, difficulty, error/retry states and layout.
3. Fairness and robustness: attack concurrency, replayed requests, foreign
   accounts/gateways, hint/clock changes, pauses, outage and version boundaries;
   measure and bound expensive paths, fix findings and repeat affected tests.

Each round records findings, actual changes and checks. Independent PR review
starts only after all three rounds and the required local gates. No production
migration or deployment is part of the PR authorization.

## Foundation checks, 13 November 2026

- `cargo test --locked -q -p central-api -p central-identity -p central-http --lib`:
  257 tests passed, including calendar, badge thresholds and publication admission.
- Targeted clippy of API, SDK, store, identity and HTTP with all targets and
  `-D warnings` passed.
- `cargo xtask postgres`: real native/browser HTTP suite and database suite passed
  in isolated PostgreSQL 18.6. Includes ordinary-gateway refusal, immutable and
  future publications, concurrent idempotent starts/results, ownership, claimed
  game/build binding, first/best/sprint scores and equal ranks, hints, consent,
  archive rewards, technical aborts, withdrawal, once-only badges, delayed finish
  delivery and the exact deadline, personal export isolation, prior-schema upgrade,
  UUID/null-version bounds and account-deletion cascades. Owned fixture cleanup passed.

These are synthetic accounting fixtures, not engine-solution certificates. The
actual generator, player/client flow and three-round feature validation were
subsequently exercised as recorded in [challenge assurance](challenge-assurance.md).

## Website and personal export

`/{lang}/challenges` renders the public calendar in its first HTML response.
The browser then reads rankings and, for a remembered session, account-owned
progress. The server-side cache is public only. Every existing site language
has the interface and data-use notice; task descriptions currently use their
English or German publication text. `baylee://challenge/<32 lowercase hex digits>`
hands the task identity to the installed client without an account, seed or token
in the link. Account operations remain same-origin cookie requests.

Listing is an explicit per-task opt-in. A hint requires a second confirmation
before Central records assistance and returns its text. Already-earned results
remain visible, and withdrawing the listing refreshes the displayed ranking.
The account's metadata download includes both paginated challenge datasets, with
strict cursor validation and explicit completeness/cursor fields at its browser
limit of 7,680 rows per dataset. `Session::export_challenges` streams the full
dataset to native callers without holding it all in memory.

Website checks on 13 November:

- 143 web/server library tests passed; shared export cursor regressions passed.
- Production public/admin WebAssembly asset builds and browser type/lint checks passed.
- 10 targeted desktop/phone browser tests passed: server rendering without
  JavaScript, opaque client links, escaped names, shared ranks, hints, persistent
  consent, outage/retry and the combined account export.
- 10 additional checks passed: English/German challenge pages in both colour
  schemes, accessibility, control target sizes and every page at 320–2,560 px in
  portrait/landscape. Browser servers exited and port 28792 was closed.
- Fixed findings: an inert medal element rejected by Trusted Types, duplicate
  landmark names, a table inaccessible to keyboard scrolling, small deck controls,
  and clicks arriving before hydration. The site's security policy was retained.

These checks use synthetic browser responses. They do not yet prove that a real
engine result appears on the website or that the installed game follows the link.

A subsequent local interactive demo on 13 November used the real PostgreSQL
fixture, authenticated Central native/browser listeners, a loopback gate and the
game engine. Its generated Daily and Weekly were solved through the engine; the
native client and website displayed the same published tasks and earned results
(Daily: turn 1; Weekly: turn 11), with the browser showing all five earned Bronze
badge families. No challenge response was mocked. The synthetic solver's very
short Daily time is fixture evidence, not a human difficulty or timing estimate.

The browser signed in normally using a separate, temporary copy of the crypto
module bound to the fixture's development issuer. Production sources/assets,
issuer validation, cookie attributes and origin checks were unchanged. The
loopback-only `e2e-real/front.ts` accepts omission of `--tls` for this setup.
Baylee's `docs/central-login.md` documents the bounded demo fixture, fresh,
completed and resumable accounts, archive task, and owned-service cleanup. The
website-to-installed-client protocol link remains outside this demo's evidence:
the isolated native client opens the identical task through its own task browser.
