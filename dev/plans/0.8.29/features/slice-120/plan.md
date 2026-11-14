---
title: "Slice 120 — explicit CUDA artifact selection and delivery"
status: DESIGN_REVIEWED — owner contract ruling and technical prerequisites held
target_release: 0.8.29
slice: 120
---

# Slice 120 — explicit CUDA artifact selection and delivery

Deliver a qualified, unpublished Linux x86_64 CUDA-13-library artifact beside
unchanged CUDA 12.6 delivery. This draft proposes a contract; it records no
owner acceptance, compiler clearance, build, runtime qualification or publication.
Format 37 stays fixed; no migration or production operation/API policy change.
[Requirements](requirements.md), [design](design.md) and [test map](test-map.md)
are the complete local design set; no additional needs/status/review packet.

## Decisions ready for the owner

**Recommended artifact contract:** retain normal 12.6 artifacts; deliver Python
`fathomdb==0.8.29+cu130` (same import, one installed variant) and separately named
Node `fathomdb-linux-x64-gnu-cuda13@0.8.29`, both by exact digest-bearing URL in
an approved first-party CUDA channel. Node selects 13 only through
`FATHOMDB_CUDA_VARIANT=13`, set before importing `fathomdb`; unset means the
existing 12 route. Explicit `12` also selects the published 12 package.
No library-presence selection or loading both native modules. Do not add the
13 package to optionalDependencies or publish it to npm/PyPI. Proposed channel
layout is `/cuda/0.8.29/cu130/` on the project's existing first-party Pages host;
this is a proposed namespace, not authorization to modify or publish that site.

Approve public Rust **source-consumer instructions and external qualification**
for each single-major build, using exact qualified pins and a consumer-root
`candle-kernels` patch; defer a new prebuilt CUDA-13 CLI archive. Still qualify a
source-built 13 CLI diagnostic companion and existing 12 CLI; this does not
promise a new public binary. These are explicit scope choices, not inferred
exclusions. A separately named Node 13 package needs a narrow amendment to
ADR-0.8.25-driverless-cuda-runtime-linkage's rejected alternative; retain every
loadability requirement. No change to the 12 default or Tegra selector follows.

**Recommended compiler disposition:** retain the accepted 13.0 target policy and
hold its artifact build/delivery until the concrete compiler-safety criterion in
design.md is met. Source examination neither proves miscompilation nor clears
6111901. Do not substitute 13.2.2, undocumented compiler switches, or a host-driver
change. If the criterion cannot be met, return a specific alternative to the
owner; the second-artifact outcome remains incomplete rather than waived.

**Recommended quantitative disposition:** accept the November review as bounded,
with September/October comparable shares UNKNOWN and importance ratings unchanged;
carry only the exact missing query in design.md. Alternatively authorize one
bounded read-only collection of that query and release-asset counts. Neither
option permits invented shares or treating packaging defaults as adoption. The
owner must explicitly dispose of this limitation before A1 is claimed complete.

## Work and dependencies

1. Independent Astra-medium design review is complete (PASS after native-identity
   symmetry and exact-candidate licence/SBOM corrections). Obtain the decisions
   above; unchanged design is not reviewed again. Update the
   affected ADR/interface docs in the same implementation change.
2. Genuine per-behavior RED, visible to the reviewer, then GREEN: Python variant
   validation; Node pre-load selection; release-version/CUDA identity on BOTH
   installed x86 variants; major-correct errors; artifact axes and installed
   selection. Include opposite-major and stale same-major native displacement
   controls. Preserve frozen RED tests and the explicit development exception.
3. Resolve compiler safety and select exact toolchain/image/pins/venue before
   building any 13 artifact. The current 12.6 route remains unchanged.
4. Only a demonstrated required fork repair enters 140: fork-local RED/GREEN,
   review and qualification, then cudarc→Candle→FathomDB pin adoption. No repair
   has been established by this design. Broad rebase/marker/multiarch work stays
   in .30. A selected 110 remedy gates only an artifact it changes.
5. Build once in the approved owned environment; inspect and install exact bytes;
   retain licence/SBOM acceptance on the actual local-version wheel and separately
   named Node tarball using existing machinery, then run local consumer
   qualification. Source licence proxies do not qualify the wheel. Integrate selected source,
   then 130 runs its affected platform/library cells on final artifacts. There
   is no 120↔130 cycle and no blanket 110/140 prerequisite.
6. One final canonical source gate per completed code phase, scoped checks while
   working, independent Astra implementation review. Hand exact unpublished
   candidates and residual cells to 130/160. Publication remains separate.

Apply the quality roadmap's package-boundary, failure, close/reopen and selected
reference comparisons. Defer exhaustive compiler/driver/fuzz matrices; the new
risk is dual-major selection, not a reason to repeat all historical CUDA cells.
G and M candidate source acceptance is not final runtime qualification and does
not discharge any artifact or installed-package requirement here.

## Resource boundary

Design uses source reads only. No build, provisioning, native install, GPU use,
fault, CI, cloud spend, ledger write or publication is authorized here. windchill3
stays R580/580.178.04 permanently and primary `/usr/local/cuda` stays 12.6; K620
is never a test device. A separately authorized 13.0 toolkit is user-local only.
Tegra 13, Thor and generic Arm are unmeasured/outside this x86 artifact scope.
