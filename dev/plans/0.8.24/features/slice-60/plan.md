---
title: 0.8.24 Slice 60 — installed smokes and publisher preservation draft plan
status: IMPLEMENTING
target_release: 0.8.24
---

# Slice 60 — installed-package smokes and CPU/publisher preservation

## Planning boundary

This document plans Slice 60. It does not publish, mutate registries or trusted
publishers, promote npm tags, create a release tag, or claim post-publication
evidence before publication. It consumes the exact package identities and
artifact contract approved by Slice 30.

Slice 60 owns the smoke mechanisms and evidence matrix for CPU and Tegra only.
Prepublication target proof installs sealed candidate artifacts in fresh
environments. Actual registry-installed proof can occur only after a separately
authorized publish; Slice 70 consumes that post-publish evidence before
declaring the release done. Windows scope is postponed to 0.8.26 by `seq-258`.

## Goal and outcome

Preserve the complete existing CPU publication contract while adding the
selected Tegra CUDA target:

- every existing CPU package retains its identity, version alignment,
  optional-dependency/loader wiring, ABI floor, and installability;
- each publisher treats an exact existing immutable artifact as success/no-op,
  treats registry uncertainty as failure, and can continue missing targets;
- canonical “atomic publish” wording is corrected to retry-safe completion;
- the Tegra target has a clean candidate-installed target smoke and an exact
  post-publication registry-install smoke; and
- release completion remains false until every required artifact and smoke is
  present and verified.

## Authority and inputs

- P24-14/P24-15, R24-3/R24-4/R24-10, A24-1/A24-4, Slice 6 approval, and the
  Windows-scope deferral at `seq-258`.
- Draft updates to NEED-016, REQ-050/AC-054, and REQ-052/AC-056 from Slice 3.
- Outputs and approved identities from Slice 30.
- `dev/design/release.md`, `dev/architecture.md`, Tier-1/platform/package ADRs,
  release workflow, publish helpers, smoke scripts, and contract tests.
- `scripts/release/{cargo,npm,pypi}-publish-if-new.sh` and tests for existing,
  missing, malformed, query-error, and registry-routing behavior.
- Memory `release-publish-gotchas.md` and
  `release-dod-requires-full-workspace-gate.md`.

## Scope

### In scope

- Build a candidate-version matrix of all existing CPU and approved Tegra artifacts,
  registries, package names, build jobs, publishers, tags, and smokes.
- Update canonical release-completion wording to match immutable registry and
  retry-safe mechanics.
- Extend publisher guards/tests only where the new identities are not already
  covered; preserve existing fail-closed behavior.
- Add a Tegra public-PEP-503 installed-package smoke harness. It must use one
  declared index, an exact local-version requirement, an independently retained
  wheel SHA-256, a fresh venv, and an offline model cache for the CUDA witness.
  provenance schema, negative tests, and evidence checks.
- Preserve CPU Python, npm, Cargo/CLI, macOS, Linux x64/ARM64, and Windows CPU
  paths affected by release workflow or metadata changes.
- Define the exact handoff to Slice 70 and the post-publication completion check.

### Non-goals

- Selecting Tegra identities or executors; that belongs to Slice 30. Windows
  identities, executors, and smokes are postponed to 0.8.26.
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

1. Require the approved Slice 30 handoff: exact names/versions, registries,
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
5. Identify prerequisites and route them: artifact defects to Slice 30, CI
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

## Approved implementation sequence

1. Record the reviewed target matrix and the revised requirement/acceptance
   wording. Correct the impossible all-or-nothing wording to retry-safe
   completion without weakening the requirement that every target is verified.
2. Add RED structural tests for a new `smoke-tegra-pages-wheel.sh`: malformed
   version/index/digest fail before networking; no extra index, source install,
   editable install, or cache is allowed; the exact digest is checked before
   install; and the lifecycle and CUDA witness remain mandatory.
3. Implement the smallest script, public Python/CLI guidance, and generic-build
   warning changes. Preserve all existing CPU and legacy Windows CPU release
   lanes; no new Windows work belongs to this release.
4. Recover the SHA-256 from the retained Slice 30 evidence artifact, compare it
   to the Pages index link, then run the new script once on the actual Jetson.
5. Fill `evidence-matrix.md` with that installed public-index proof. Registry
   publication for CPU packages, npm promotion, tag creation, and post-tag
   smokes remain Slice 70 owner actions.
6. Hand Slice 70 the precise evidence, remaining release sequence, and the
   explicit no-tag/no-push boundary of this slice.

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

- clean candidate-installed Jetson smoke;
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
