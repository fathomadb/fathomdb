# Slice 40 implementation status

Status: COMPLETE ON `release/0.8.26` at corrected implementation tip
`bfb2132b`.

## Completed scope

- Reconciled the draft against Slices 9–35, assigned schema/open/binding
  surfaces, and D26-03 through D26-05. The approved scope retained Slice 35's
  changed-in-place V1 derived-edge transaction and limited net-new product work
  to schema 34 and the fresh-database-only public-open boundary.
- Added content-free schema step 34. Missing and empty paths bootstrap all 34
  steps; non-empty schema 33, future, and foreign zero-version databases are
  refused through the existing typed error before product mutation or migration
  events.
- Routed public opens through one persistent product-lock namespace and a
  WAL-aware read-only classifier. Refusal preserves database and SQLite-sidecar
  bytes, restores attempt-created SHM, never rewrites an existing lock, and may
  leave only an empty lock file when establishing the namespace.
- Compiled the custom migration runner seam only under the dedicated,
  non-forwarded `migration-test-hooks` feature. Default engine, facade,
  bindings, and CLI surfaces retain no historical migration route.
- Updated the directly owned Rust, Python, TypeScript, CLI, and wire contracts
  to schema 34 and fresh-only behavior while leaving broad architecture/design
  convergence to Slices 45 and 46 and package/platform verification to Slice
  50.
- Preserved Slice 35's atomic derived-edge grammar, endpoint semantics,
  receipt/replay/integrity, erasure, projection, traversal, and cross-binding
  behavior on fresh schema-34 databases.

## TDD and review chronology

- `c065086b` — reconciled and independently reviewed plan/design approval.
- `943ac4f4` — initial RED schema/open/binding contract.
- `cd7a4abe` — GREEN schema-34 and fresh-only implementation.
- `70e958f9` — code-review RED/GREEN remediation for the persistent lock
  namespace and exact refusal fixtures; independent code review returned PASS.
- `7eadeb94` — Astra-review RED proving fresh-process runtime configuration was
  too late.
- `68152886` — GREEN runtime-before-admission ordering and deterministic
  current-wins race proof.
- `7b1dc0a8` — canonical-gate remediation preserving pre-admission corruption
  events and retiring one superseded v18 public-upgrade oracle.
- `bfb2132b` — independent-verification remediation moving the older-wins
  fixture into a coordinated child process and removing runtime/order coupling.

The GPT-6 Astra medium design review found one P1 and one P2. P1 required
process-global SQLite configuration after product-lock acquisition but before
any admission connection; process-isolated tests now cover clean current
reopen, current WAL with and without SHM, and schema-33 refusal followed by a
fresh open in the same child. P2 required a real current-wins proof; a
path-scoped `cfg(test)` rendezvous now pauses after lock/configuration and
before classification while a competing older opener fails the same lock and
cannot install schema 33. The older-wins case remains. Astra rereview returned
PASS with no remaining P1/P2 finding and confirmed the hook adds no shipped
surface.

## Verification

- Slice 40 fresh-cutover integration: 13/13 PASS in default parallel mode.
- Deterministic current-wins race unit test: 1/1 PASS; the existing older-wins
  case passes in the integration target.
- Fresh-process RED reproduced `RuntimeConfiguration(TooLate)` before the fix;
  all clean/WAL/refusal-then-fresh child cases pass after it.
- Lifecycle corruption-event regression: 2/2 PASS after routing classifier
  corruption to the supplied open subscriber.
- Vector-equivalence target: 22/22 PASS after removing the obsolete v18 public-
  upgrade test; current-schema baseline and fail-safe tests remain.
- Canonical `./scripts/agent-verify.sh`: PASS, including lint, typecheck,
  security, and `agent-test.sh: 110/110 suites passed (skipped=0 excluded=0)`.
- `git diff --check` and the pre-commit Rust/Markdown guards pass.

The independent verification subagent initially returned FAIL because the
older-wins fixture initialized raw SQLite in the tested process. After
remediation it returned PASS at `bfb2132b`: the formerly failing isolated test
passes 1/1, the default-parallel target passes 13/13, synchronization is
bounded, the child is reaped, and no P1/P2 finding remains. Detailed evidence
is in `review-verification.md`.

The first canonical attempt was not hidden: it failed the two pre-admission
corruption-event tests and the superseded v18 public-upgrade test. The focused
fix at `7b1dc0a8` closed those exact failures before the successful canonical
rerun.

## Cleanup and final verdict

No new branch or worktree was created; work remained on the user's existing
`release/0.8.26` worktree. Two untracked SQLite fixtures left by each canonical
run were deleted after inspection. No product database or user artifact was
removed.

R26-40A through R26-40F and AC26-40A through AC26-40G are satisfied at the
release-branch source boundary. Slice 40 is complete and Slice 45 is next.
