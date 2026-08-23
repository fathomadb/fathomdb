---
title: 0.8.24 Slice 30 — public Tegra CUDA distribution draft plan
status: DRAFT
target_release: 0.8.24
---

# Slice 30 — public Tegra CUDA distribution

## Planning boundary

This document plans Slice 30. It does not select or reserve a public name,
create a registry project, configure a trusted publisher, change packaging,
build on Jetson, or publish anything. Exact identity, index, import topology,
and publisher route are explicit reviewer/HITL decisions before implementation.

The accepted direction is fixed: Tegra uses a separately identified public
distribution. The local `fathomdb==0.8.24+tegra` form remains a local build
identifier and is never uploaded to PyPI.

## Goal and outcome

Design and, after approval, implement one explicit public Tegra CUDA
distribution that:

- cannot be resolver-confused with generic Linux ARM64 CPU FathomDB;
- is installed by an explicit documented command;
- requires no end-user source compile on the supported Jetson/L4T route;
- preserves the generic `fathomdb` CPU artifact and its installation path;
- binds artifact bytes to source, Jetson build environment, CUDA/toolchain,
  compatibility floor, and trusted publisher; and
- hands Slice 60 a clean Jetson installed-package smoke contract.

## Authority and inputs

- P24-08, R24-1/R24-8, A24-2, draft `REQ-TARGET-TEGRA` and
  `AC-TARGET-TEGRA`, and the Slice 6 owner decision.
- `dev/plans/0.8.24/prework/{publication-topology,executor-inventory}.md`.
- `dev/design/0.8.23-gpu-artifacts.md`, Tegra build/rehearsal designs, and
  `dev/tegra-platform-reference.md`.
- `scripts/release/build-python-cuda-tegra.sh`, `_coinstall.py`, current Python
  package metadata, release workflow, witness schemas, and tests.
- Accepted Tier-1/Linux-ARM64 ADRs. The generic ARM64 CPU route remains valid.
- Primary packaging facts:
  - public PyPI rejects upstream local-version uploads;
  - distribution names and import-package names are distinct concepts;
  - a PyPI pending trusted publisher does not reserve a project name until
    first publication.

Useful primary references:

- <https://packaging.python.org/en/latest/specifications/version-specifiers/>
- <https://packaging.python.org/en/latest/discussions/distribution-package-vs-import-package/>
- <https://docs.pypi.org/trusted-publishers/creating-a-project-through-oidc/>
- <https://docs.pypi.org/trusted-publishers/security-model/>
- <https://docs.nvidia.com/cuda/cuda-installation-guide-linux/>

## Scope

### In scope

- Select the public distribution identity and index/publisher route through an
  explicit owner decision.
- Define the supported Jetson family, L4T/JetPack, Python ABI, CUDA/toolkit,
  driver, glibc, Rust, and architecture matrix.
- Define distribution-versus-import-package behavior and mutual-exclusion or
  plugin semantics relative to generic `fathomdb`.
- Produce an installable artifact on an observed Jetson executor, retain
  provenance, and prepare an OIDC-backed hosted publication handoff.
- Update package/co-install guards, compatibility docs, release workflow, and
  an ADR or successor decision as required by the selected identity.
- Provide Slice 60 with the candidate-installed and post-publication smoke
  commands and evidence schema.

### Non-goals

- Uploading any `+tegra` local version to PyPI.
- Treating a generic hosted ARM64 runner as Jetson evidence.
- Supporting every Tegra/L4T generation, ARM64-SBSA, Windows CUDA, npm Tegra,
  or end-user source builds unless separately approved.
- Claiming CPU/GPU byte identity. Numerical equivalence uses the accepted
  tolerance-based contract where comparison is relevant.
- Mutating registry configuration before the selected design and owner gate.

## Slice prep — planned first phase

Create under this directory:

- `prep.md` — goals, target/executor/index inventory, and decision state;
- `draft-contracts.md` — reviewed slice-local need/requirement/AC drafts;
- `design.md` — package identity, compatibility, build, publish, and smoke design;
- `research.md` — primary-source packaging/CUDA questions and answers; and
- `decision.md` — exact owner-selected identity, index, publisher, and target
  matrix, or an explicit defer outcome.

### Prep tasks

1. Re-query candidate public names and indexes without interpreting
   availability as ownership. Confirm the current project/repository owner is
   `fathomadb/fathomdb` and the intended workflow filename/environment claims.
2. Observe the proposed Jetson executor: selector, online state, OS/L4T,
   JetPack/CUDA, Python/Rust/compiler, glibc, GPU and compute capability,
   storage/cache, trust boundary, and artifact-transfer path.
3. Restate the Slice 3 drafts and propose refinements:
   - **N30-DRAFT:** a supported Jetson user can explicitly select a prebuilt
     CUDA artifact while the generic ARM64 CPU route remains unambiguous;
   - **R30-DRAFT-1:** the public identity, index, installer command, and import
     topology are explicit and cannot silently overwrite a co-installed CPU
     distribution;
   - **R30-DRAFT-2:** each supported target tuple has source/build/toolchain/
     digest provenance and clear unsupported behavior;
   - **AC30-DRAFT:** resolver/selection fixtures distinguish generic CPU and
     Tegra, artifact metadata matches the approved identity, and a clean Jetson
     candidate install proves GPU-selected open/write/search/close/exit.
4. Read release/bindings architecture, target ADRs, Python interface docs, the
   actual build script, and the entire co-install test surface. Write an
   exists-versus-net-new map.
5. Enumerate each prerequisite and assign it: identity/index/publisher choice
   to the owner; executor readiness to the target operator; CI selector gaps to
   Slice 10; shared smoke/publisher logic to Slice 60; final evidence to Slice 70.

## Identity options the design must evaluate

The prep review must compare, not silently choose, at least:

1. **Distinct PyPI distribution, same `fathomdb` import package** (for example
   `fathomdb-tegra`): simple explicit install, but pip does not enforce file
   ownership conflicts; the design must make replacement/mutual exclusivity
   fail loudly and document upgrade/uninstall behavior.
2. **Distinct distribution with a separate plugin/native import topology:**
   avoids two distributions owning `fathomdb/`, but requires a reviewed runtime
   loading seam and may be materially larger product work.
3. **Distinct public alternate index or artifact distribution:** useful if the
   host-bound Linux wheel cannot satisfy PyPI platform-tag/audit constraints;
   installer commands must be explicit and dependency-confusion-safe.

Every viable option keeps a distinct public identity. “Same `fathomdb` project
with another ARM64 wheel” and public `+tegra` are rejected before scoring.

## Draft design and design review

### Required design content

- exact distribution and import names;
- registry/index, project initialization, trusted-publisher claims, and GitHub
  environment name;
- supported target matrix and wheel/platform tag rationale;
- relationship to generic ARM64 CPU installation, upgrades, uninstall, and
  co-install refusal;
- staged source/package metadata and version alignment;
- Jetson-native build, dynamic dependency, glibc/ABI, CUDA, GPU allocation,
  and artifact-digest witnesses;
- artifact transfer from target builder to a hosted OIDC publisher where
  required;
- explicit install and unsupported-route errors/warnings;
- candidate-installed and registry-installed smoke contracts; and
- docs, interface, ADR, release-note, and DOC-INDEX consequences.

### Challenging aspects and research plan

1. **Wheel acceptance:** verify from PyPI/PyPA primary docs and a non-publishing
   validation route whether the chosen Linux tag and external CUDA dependencies
   are accepted. Do not relabel a host-bound wheel as manylinux without proof.
2. **Shared import ownership:** verify pip/metadata behavior for two
   distributions providing one import package and design a real enforcement
   mechanism or choose a separate topology.
3. **JetPack compatibility:** use NVIDIA's JetPack/CUDA compatibility sources
   to pin the supported matrix; do not infer cross-generation compatibility
   from one successful host.
4. **New-project publisher:** verify pending-publisher claims, environment
   matching, and first-publication behavior. A pending publisher is not a name
   reservation.
5. **Provenance:** decide whether the existing witness schemas can be extended
   honestly or require a target-specific schema; never reuse x86_64 fields by
   renaming them.

### Architectural-fit review and revision

The reviewer checks the design against current Python co-install assumptions,
the bindings/release architecture, generic ARM64 ADRs, actual code, and public
documentation. The revised design must explicitly supersede the 0.8.23
“unshipped/local-only” posture without rewriting history. A material package-
identity/platform-contract change receives an ADR or successor before code.

## Planned implementation sequence after decisions

1. Commit the approved ADR/design and contract tests before packaging code.
2. Add meaningful RED tests for identity/metadata, CPU-versus-Tegra selection,
   collision/unsupported behavior, workflow/artifact contract, and publisher
   routing as applicable.
3. Implement the selected staged package/build path and compatibility guards.
4. Build only on the approved Jetson executor; retain source/toolchain/digest
   and GPU evidence. A local Linux x64 build is not a substitute.
5. Transfer immutable artifacts to the approved hosted publisher route without
   executing target-controlled publication logic.
6. Update public install/compatibility docs and indexes.
7. Hand Slice 60 the exact identities, digests, candidate-install smoke, and
   post-publish install command. Publication itself remains separately gated.

## Verification and evidence

- Local structural/TDD tests for packaging metadata, co-install/selection,
  workflow graph, witness verification, and negative cases.
- `actionlint` and applicable release-contract tests for workflow edits.
- Jetson-native build/link/ABI witness and candidate-installed lifecycle smoke.
- GPU evidence names selected device/UUID and process/allocation evidence; a
  successful build or import alone is insufficient.
- Generic ARM64 CPU artifact metadata/install path remains unchanged and is
  handed to Slice 60 for candidate-version preservation proof.
- Registry queries/publisher configuration are evidence only after separately
  authorized external actions; no draft-plan claim marks them complete.

## Risks and recovery

| Risk | Control / recovery |
| --- | --- |
| Public name is taken before first publish | Re-query immediately before owner authorization; pending publisher is not a reservation. |
| Two distributions overwrite one import tree | Require enforceable mutual exclusion or select a separate plugin topology. |
| Wheel tag overclaims portability | Bind to tested L4T/JetPack tuple or choose an honest alternate index/distribution. |
| Generic ARM64 CPU users receive CUDA bytes | Separate identities and explicit install; preserve generic package tests. |
| Self-hosted builder leaks publication authority | Build/attest remotely, publish from a separately trusted hosted job. |
| Published artifact is wrong and immutable | Fail before publish; after publish, yank/deprecate and issue a corrected version—never replace bytes. |

## Decisions and prerequisites for the next reviewer

Ready status requires explicit owner choices for:

1. public distribution name and index;
2. distribution/import-package topology;
3. supported Jetson/L4T/JetPack/Python matrix;
4. executor and trust/artifact-transfer route; and
5. trusted-publisher repository/workflow/environment claims.

Any unresolved choice blocks implementation. A newly discovered CI route gap
returns to Slice 10; shared idempotency/smoke work enters Slice 60; publication
authority remains with the final owner gate.

## Definition of done

Slice 30 closes only after the approved separate identity and architecture are
implemented, local contract tests pass, a real Jetson build and clean
candidate-installed CUDA lifecycle smoke are retained with provenance, generic
ARM64 CPU selection remains distinct, and Slice 60 receives a complete target
artifact/smoke handoff. Public upload is not implied by slice closure.
