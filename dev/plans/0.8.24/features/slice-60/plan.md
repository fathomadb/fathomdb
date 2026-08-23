---
title: 0.8.24 Slice 60 — installed smokes and publisher preservation draft plan
status: DRAFT
target_release: 0.8.24
---

# Slice 60 — installed-package smokes and CPU/publisher preservation

## Planning boundary

This document plans Slice 60. It does not publish, mutate registries or trusted
publishers, promote npm tags, create a release tag, or claim post-publication
evidence before publication. It consumes the exact package identities and
artifact contracts approved by Slices 30 and 40.

Slice 60 owns the smoke mechanisms and evidence matrix. Prepublication target
proof installs sealed candidate artifacts in fresh environments. Actual
registry-installed proof can occur only after a separately authorized publish;
Slice 70 consumes that post-publish evidence before declaring the release done.

## Goal and outcome

Preserve the complete existing CPU publication contract while adding the
selected Tegra and Windows CUDA targets:

- every existing CPU package retains its identity, version alignment,
  optional-dependency/loader wiring, ABI floor, and installability;
- each publisher treats an exact existing immutable artifact as success/no-op,
  treats registry uncertainty as failure, and can continue missing targets;
- canonical “atomic publish” wording is corrected to retry-safe completion;
- every new target has a clean candidate-installed target smoke and an exact
  post-publication registry-install smoke; and
- release completion remains false until every required artifact and smoke is
  present and verified.

## Authority and inputs

- P24-14/P24-15, R24-3/R24-4/R24-10, A24-1/A24-4, and Slice 6 approval.
- Draft updates to NEED-016, REQ-050/AC-054, and REQ-052/AC-056 from Slice 3.
- Outputs and approved identities from Slices 30 and 40.
- `dev/design/release.md`, `dev/architecture.md`, Tier-1/platform/package ADRs,
  release workflow, publish helpers, smoke scripts, and contract tests.
- `scripts/release/{cargo,npm,pypi}-publish-if-new.sh` and tests for existing,
  missing, malformed, query-error, and registry-routing behavior.
- Memory `release-publish-gotchas.md` and
  `release-dod-requires-full-workspace-gate.md`.

## Scope

### In scope

- Build a candidate-version matrix of all existing and approved new artifacts,
  registries, package names, build jobs, publishers, tags, and smokes.
- Update canonical release-completion wording to match immutable registry and
  retry-safe mechanics.
- Extend publisher guards/tests only where the new identities are not already
  covered; preserve existing fail-closed behavior.
- Add target-specific candidate-install and registry-install smoke harnesses,
  provenance schema, negative tests, and evidence checks.
- Preserve CPU Python, npm, Cargo/CLI, macOS, Linux x64/ARM64, and Windows CPU
  paths affected by release workflow or metadata changes.
- Define the exact handoff to Slice 70 and the post-publication completion check.

### Non-goals

- Selecting Tegra/Windows identities or executors; those belong to Slices 30/40.
- Rebuilding target artifacts, changing engine behavior, or investigating WAL.
- Claiming a dry run, uploaded artifact, import-only check, or source-tree test
  is a registry-installed lifecycle smoke.
- Requiring an additional full hosted CI cycle for administrative integration.
- Replacing or deleting a valid immutable registry artifact.

## Slice prep — planned first phase

Create under this directory:

- `prep.md` — goals, upstream completion state, current-main SHA, and inventory;
- `draft-contracts.md` — refined need/requirement/AC updates;
- `design.md` — artifact matrix, retry/completion state machine, and smoke design;
- `research.md` — primary registry/OIDC/installer questions and findings; and
- `evidence-matrix.md` — per-target candidate/post-publish proof state.

### Prep tasks

1. Require approved Slice 30/40 handoffs: exact names/versions, registries,
   artifacts/digests, build provenance, target matrix, install commands, and
   unsupported behavior. Missing fields block that target.
2. Enumerate the current release workflow and public package set from code and
   registry queries. Distinguish source-declared, publicly observed, and
   proposed artifacts.
3. Restate and refine drafts:
   - update NEED-016 only for the owner-approved target routes;
   - update REQ-050/AC-054 to immutable-artifact, retry-safe completion;
   - update REQ-052/AC-056 from wheel-only to affected target-native installed
     package evidence;
   - **R60-DRAFT:** no release completes with a failed, missing, or unverified
     required target; retries skip exact valid versions and fail closed on
     uncertainty;
   - **AC60-DRAFT:** publisher fixtures cover exists/absent/query-error/routing,
     and fresh candidate plus post-publication installs run the lifecycle with
     identity/version/digest/target provenance.
4. Read architecture, release design, ADRs, publisher/smoke scripts, workflow,
   and full test bodies. Write an exists-versus-net-new map.
5. Identify prerequisites and route them: artifact defects to Slice 30/40, CI
   selector gap to Slice 10, external registry configuration to owner, and
   final integration/publication authorization to Slice 70.

## Draft design and design review

### Artifact matrix

At minimum, the design records:

| Family | Identities to preserve/prove |
| --- | --- |
| Cargo | Seven tiered workspace crates and published CLI install path |
| Python CPU | `fathomdb` wheels for every retained supported target |
| npm main | Thin `fathomdb` package and exact optional dependencies |
| npm CPU platform | Linux x64/ARM64, macOS x64/ARM64, Windows x64 accepted names |
| Tegra CUDA | Exact Slice 30 distribution/index and supported target tuple |
| Windows CUDA | Exact Slice 40 selected Python/npm identities and target tuple |
| GitHub release | Expected assets, digests, and dependency on successful smokes |

Each row names producer, immutable artifact digest, publisher authentication,
exact-version query, missing/existing/error behavior, candidate smoke,
post-publish smoke, and completion status.

### Retry-safe completion state machine

The design must replace impossible all-or-nothing publication with:

1. **ABSENT:** publish allowed only after a successful exact-version absence
   query and all build/preflight evidence.
2. **PRESENT_VALID:** immutable exact version and expected artifact are a no-op;
   never overwrite.
3. **UNKNOWN/ERROR:** fail closed; do not publish blindly.
4. **PUBLISHED_UNVERIFIED:** artifact exists but target smoke is missing/failing;
   release remains incomplete.
5. **VERIFIED:** exact registry-installed artifact passed its target smoke.
6. **COMPLETE:** every required matrix row is verified; only then may final
   release completion be recorded.

The design must explain partial retries and index propagation without fixed
sleep assumptions when a resolvability poll already exists.

### Smoke contract

Every affected target smoke records candidate/release SHA, package identity and
version, registry/index or sealed artifact source, artifact digest, OS/arch,
executor label, Python/Node/Rust version, CUDA/driver/GPU facts when applicable,
install command, and open/write/search/close/process-exit output.

- Candidate smoke: fresh environment, install sealed artifact bytes, no source
  or editable install.
- Post-publish smoke: fresh environment, install exact public identity/version
  from the configured public registry/index, then the same lifecycle.
- GPU smoke: also prove selected CUDA through the target design's device/process
  witness.

### Challenging aspects and research plan

- Verify current PyPI, npm, and crates.io immutable-version/query semantics from
  primary registry docs where existing helpers rely on them.
- Verify PyPI/npm OIDC repository/workflow/environment claims, including new
  project and self-hosted-runner constraints.
- Verify package-manager cache/propagation behavior without treating a bounded
  transient miss as proof publication failed; use resolvability polling and
  retain the distinction.
- Verify GitHub artifact digest/attestation use only if it materially strengthens
  the build-to-publish handoff; attestations do not replace target smokes.

### Architectural-fit review and revision

Review the state machine and matrix against actual scripts/workflow, accepted
ADRs, architecture, and locked acceptance policy. Revise canonical documents so
mechanics and contract agree. If a new target changes Tier-1 rather than adding
a distinct target distribution, require an ADR successor before implementation.

## Planned implementation sequence after prep approval

1. Commit approved requirement/acceptance/architecture/release-design wording.
2. Add RED tests for any missing publisher identity, retry state, workflow edge,
   CPU preservation, or smoke provenance behavior.
3. Make the smallest helper/workflow/smoke changes; preserve existing routes.
4. Run local publisher fixture and structural tests, package inspections, and
   candidate smokes on the actual Jetson/Windows target hosts.
5. Fill `evidence-matrix.md` with candidate evidence and mark post-publication
   rows pending rather than falsely complete.
6. Hand Slice 70 the exact remaining publication/post-publish sequence. After
   owner-authorized publication, record actual registry-installed smoke results
   and only then mark Slice 60 evidence complete.

## Verification and evidence

Local/structural proof includes applicable:

- `test_idempotent_republish.sh`, `test_publish_registry_safety.sh`,
  `test_pypi_publish_roundtrip.sh`, and `test_cargo_publish_if_new.sh`;
- native-artifact, Linux ARM64, release-workflow, smoke-script, version-surface,
  and target-specific contract tests;
- package pack/wheel metadata inspection and optional-dependency validation;
- `actionlint` for workflow edits; and
- document/plan lint plus `git diff --check`.

Target/external proof includes:

- clean candidate-installed Jetson and selected Windows CUDA smokes;
- affected CPU candidate install smokes on their actual platform families;
- exact trusted-publisher/readiness observations without secret disclosure;
- after publication, exact-version registry queries and target-native installed
  smokes; and
- full workspace clippy/check and normal release verification before any green
  release claim.

Hosted execution is requested only for a platform or registry fact unavailable
locally. It is not repeated for documentation or administrative integration.

## Risks and recovery

| Risk | Control / recovery |
| --- | --- |
| New CUDA route breaks CPU users | Explicit candidate matrix and CPU install/loader preservation tests. |
| Retry republishes immutable bytes | Exact-version no-op and fail-closed unknown state. |
| Partial publish is mislabeled complete | State machine requires every required smoke before COMPLETE. |
| Registry propagation flake is called publish failure | Separate publish presence from install resolvability; bounded poll and retained status. |
| Target smoke accidentally uses source | Fresh environment and exact artifact/registry provenance are mandatory. |
| Circular Slice 60/70 completion | Slice 60 owns mechanism/candidate proof; Slice 70 sequences authorization and consumes post-publish evidence before final completion. |

## Decisions and prerequisites for the next reviewer

Ready status requires complete Slice 30/40 artifact contracts and owner approval
of the final required-artifact matrix. The reviewer must approve the canonical
REQ-050/AC-054 and REQ-052/AC-056 edits, exact smoke environments, and the
boundary between prepublication candidate proof and post-publication evidence.

Registry/environment/publisher mutations and real publication remain separate
owner-authorized actions. Missing authorization is a pending prerequisite, not
a reason to weaken the smoke.

## Definition of done

Slice 60's implementation is ready for final integration when canonical release
semantics match retry-safe mechanics, every new route has candidate-installed
target proof, every existing CPU path is preserved, and publisher fixtures are
green. Its evidence is complete only after every required public exact-version
install and lifecycle smoke succeeds; Slice 70 cannot declare the release done
before consuming that record.
