# Agent instructions — baylee-central

## Binding priorities

1. Security.
2. QA, including adversarial testing and reproducible evidence.
3. Performance within the security and QA constraints.
4. Clean, maintainable code.

These project-specific instructions deliberately differ from sibling Baylee's
performance policy. Do not import its permission to write unsafe or assembly.

## Safety and dependencies

- No first-party `unsafe` Rust, unsafe functions/impls, inline/global assembly,
  assembler sources, intrinsics requiring unsafe, or FFI escape hatches. Do not
  move forbidden operations into a helper crate, build script or native module.
- Every Rust crate root, executable, test and example must enforce
  `#![forbid(unsafe_code)]`. Set workspace `unsafe_code = "forbid"` and make all
  members inherit it. CI must verify coverage, including generated/expanded
  first-party code and build scripts. No lint exemptions.
- Safe callers do not imply a fully safe dependency graph. Rust runtimes, crypto
  and database/network libraries may contain unsafe or assembly internally.
  Inventory transitive/native/build dependencies and their enabled features;
  do not claim the whole executable contains none. Prefer maintained, reviewed
  dependencies and safe implementations. Do not replace vetted crypto with
  custom crypto merely to remove dependency internals. Any required dependency
  containing unsafe/assembly must be disclosed and explicitly accepted by the
  owner before adoption; a strict whole-stack ban is an unresolved feasibility
  decision, not a reason to silently weaken this rule.
- No custom cryptographic primitives, password protocols or token algorithms.
  Pin versions and review advisories, licenses, provenance and build scripts.
- Licences (`LICENSE.md` § 6): `central-api`, `central-client` and
  `central-crypto` are AGPL-3.0-only, because the Baylee client links them,
  and build only on each other; every other crate, the web app and the docs
  are source-available. The policy check enforces both.
- Never log secrets, password proofs, tokens, recovery codes, plaintext private
  content or identifying training payloads. Redaction must have tests.

## Workflow

- Implementation was authorized on 2026-10-19. Use the three-pass method in
  docs/security-assurance.md. Higher model effort is not a security approval.
- Keep reusable libraries separate from the executable and server internals;
  provide isolated dev mode without authorization bypasses.
- The owner accepted the pinned HTTP and crypto graphs via safe APIs (see
  docs/dependency-proposal and docs/crypto-proposal). The SeaORM 2 / SQLx /
  Rustls / ring graph in docs/storage-proposal is also accepted subject to QA.
  First-party unsafe/assembly remains forbidden; pin and review future changes.
- Support native Linux, Windows and macOS, including Apple Silicon and Intel.
  Keep platform integration behind adapters; verify the shared libraries in CI.
- Implement small milestones with tests and a commit per working milestone.
  Use `codex/` branches and rebase feature branches. Never bypass required checks.
- Work does not lie around (owner, 19.11.2026; all agents). After each finished
  milestone, at least once a day, and before a session ends or its quota runs
  low: commit, rebase onto `origin/main` and push the branch, half-done work
  included (as a draft pull request once it is worth reading). A finished
  milestone gets its pull request soon and stays small, and it is merged as
  the rule on merging below allows, not held back for the rest of a stack.
  This runs in the background beside the actual work; a cheaper model may do
  a conflict-free rebase, the check runs and the push, but resolves no
  conflict in code and writes none. A state that lay for long is compared
  with `main` first (`git cherry origin/main <branch>`) and what is missing is
  named to the owner. Parallel work left unpushed or unmerged has been lost
  and has gone stale enough to break `main` before.
- No real users, production credentials or data in tests. Bind local test
  services to loopback; use isolated databases/configuration and synthetic data.
- Always stop agent-started test clients/services after testing, including
  failures. Track owned PIDs, verify exit and port closure, never kill unrelated
  processes. Baylee client tests use its dev-control skill, virtual input and
  the user's additional left monitor; do not capture the user's pointer.
- PostgreSQL 18+ is required. Generate central IDs with DEFAULT uuidv7() and
  INSERT ... RETURNING; reserve account IDs before cryptographic enrollment.
  Explicit offline-client UUIDv7 exceptions are documented in docs/postgresql.md.
  Security tokens use independent randomness; IDs are not authorization/secrets.
- Run `cargo xtask gate`, `cargo xtask postgres`, `cargo xtask smoke` and
  `cargo xtask secrets` (docs/xtask.md), plus cargo audit --deny warnings and
  cargo deny check. Pin checker versions as in CI.
  The gate and dependency CI job also require `cargo xtask crypto-scope`:
  the bigint exception is bounded by reviewed sources and graph, never a
  standalone scanner waiver (`docs/crypto-proposal/bigint-advisory.md`).
- Keep protocol specs, migrations, threat model and operator runbooks in sync.
  Do not assert anonymity, audit completion or legal compliance without evidence.
- An agent may merge a pull request into `main` itself, without asking the
  owner first (owner, 18.11.2026), when the checks named above ran on the pull
  request's tip and every check `main` requires is green on GitHub. It says
  afterwards what it merged. `main`'s protection stays as it is (a pull
  request, the required checks, linear history, for administrators too): an
  agent does not change or bypass it and never pushes to `main` directly.
  This covers the merge alone, not what the next rule names.
- External release, production migration and processing human training replays
  require the explicit gates in the implementation plan. No silent deployment.
