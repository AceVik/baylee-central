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
- Never log secrets, password proofs, tokens, recovery codes, plaintext private
  content or identifying training payloads. Redaction must have tests.

## Workflow

- Current state is planning only. Do not start implementation until the owner
  authorizes that stage. Higher model effort is not a security approval.
- Implement small milestones with tests and a commit per working milestone.
  Use `codex/` branches and rebase feature branches. Never bypass required checks.
- No real users, production credentials or data in tests. Bind local test
  services to loopback; use isolated databases/configuration and synthetic data.
- Always stop agent-started test clients/services after testing, including
  failures. Track owned PIDs, verify exit and port closure, never kill unrelated
  processes. Baylee client tests use its dev-control skill, virtual input and
  the user's additional left monitor; do not capture the user's pointer.
- Use UUIDv7 for generated object IDs; security tokens require independent
  cryptographic randomness. IDs are not authorization or secrets.
- Establish format, lint, test, dependency and security checks in milestone M1;
  this documentation repository currently has no build/test command.
- Keep protocol specs, migrations, threat model and operator runbooks in sync.
  Do not assert anonymity, audit completion or legal compliance without evidence.
- External release, production migration and processing human training replays
  require the explicit gates in the implementation plan. No silent deployment.
