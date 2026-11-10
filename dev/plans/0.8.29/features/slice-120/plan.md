---
title: "Slice 120 — CUDA usage policy and second artifact"
status: PROPOSED
target_release: 0.8.29
slice: 120
---

# Slice 120 — CUDA usage policy and second artifact

## Goal, maturity and boundary

Complete the required usage→importance→actions review and deliver a qualified,
unpublished CUDA 13.0 artifact beside 12.6, after the Slice 9/10 decisions in the
[drafting brief](../../planning-brief.md). This is newly proposed placement for
TC-a5afe8b8; the former Slice 120 release close moves to 160, not silently forward.
The [reconciliation](../../reconciliation-plan.md) and
[planning evidence](../../planning-evidence.md) establish format37/current .29:
.28 CUDA 13 GCP library/build qualification exists; second product delivery does
not. November 9 quantitative shares are incomplete; October proxies are historical.
Q5 Tegra CUDA 13 remains unmeasured. No Orin/Thor/generic Arm CUDA 13 claim follows
from an archive listing, compatible driver, fork pass or successful x86 build.
Drafting authorizes no build, install, publication, host change or policy amendment.
Retain accepted 12.6 shipping floor and R580 host constraint pending ratification;
windchill3 primary CUDA stays 12.6, its driver never changes and K620 never runs.

## November10 review input

[Fresh review and saved inputs](../../research/cuda-version-adoption.md) provide
current primary packaging/vendor facts, with comparable download shares still
unavailable. NVIDIA13.2.2 reports compiler issue6111901, present since12.8;
PyTorch cited it when removing13.0 nightlies. This does not establish FathomDB
exposure. Before delivery, assess exact consumed kernels/compiler pipeline and
review mitigation. Any change from accepted13.0 artifact/toolkit/PTX/driver policy
requires owner ratification; no automatic13.2 build or host change follows.
The proposed delivery hold is a checkpoint/design input, not a ratified policy
amendment. Keep importance ratings and R580/primary12.6 constraints unchanged.

## Inspected sources and architecture preparation

Read the [policy ADR](../../../../adr/ADR-0.8.28-cuda-version-support-policy.md),
[review procedure](../../../../design/cuda-version-support-review.md),
[October adoption baseline](../../../0.8.28/features/slice-31/research/cuda-version-adoption.md),
[.28 delivery status](../../../0.8.28/features/slice-95/status.md),
[release mechanics](../../../../design/release.md),
[package rehearsal contract](../../../../design/0.8.23-slice-20-cuda-package-rehearsal.md),
[Tegra distribution ADR](../../../../adr/ADR-0.8.27-jetson-tegra-node-addon-distribution.md)
and [artifact inputs](../../../../../scripts/release/cuda-artifact-contract.sh).
Source inspection covered release.yml's 12.6 lanes, embedder Cargo/library checks,
Python packaging and Node platform/version resolution. Existing Tegra channels
and early-cuInit exception are distinct from generic Arm.
Prepare local `needs.md`, `requirements.md`, `acceptance.md`, `design.md`,
`test-map.md` and `qualification.md`; review actual loaders/build provenance before
revising contracts. No new canonical AC IDs or invented completed reviews.

## Draft needs, requirements and acceptance

All S120 identifiers below are release-local draft criteria.

| Need / requirement | Falsifiable acceptance |
| --- | --- |
| S120-N1 / R1: policy reflects observed use | A1: saved raw inputs and replayable queries compute comparable major/minor shares, dated windows/denominators and trends; each claim is MEASURED/REPORTED/INFERRED. Missing sources stay explicit; no historical/current substitution. |
| S120-N1 / R2: importance/actions are accountable | A2: compare every policy row with October and current evidence; record artifact/test/venue actions and owner ratification of changes in ADR amendment/successor and release plan. Unchanged rows retain their rationale. |
| S120-N2 / R3: consumers identify/select compatible bytes | A3: approved names, versions, channels and selection table cover Python/Node/Rust/CLI, 12-only/13-only/both/no libraries and driverless hosts; installed selected bytes/digests and actual loaded library paths agree. |
| S120-N2 / R4: executable support claims match the build | A4: source/pins, toolchain/image, features, CPU ABI/glibc, architecture, extracted PTX ISA/target, driver floor and required cuBLAS/cuRAND majors/symbols are bound to each artifact; unsupported cells fail plainly or remain UNMEASURED. |
| S120-N3 / R5: delivery preserves operation and fallback | A5: retained installed Python/Node and external Rust consumers perform real model work, persisted write/read, separate-process reopen, close/exit; forced CUDA is typed on failure and auto CPU fallback is truthful, with allocation witnesses where claimed. |
| S120-N3 / R6: package evidence survives promotion | A6: exact-candidate rehearsal inventory/digests, linkage/licence/SBOM checks and installed-consumer receipts verify; substituted/missing package controls refuse. Handoff to 160 is unpublished and source-bound; registry/Pages proof follows separate publication authorization. |

## Research and proposed design to challenge

Run read-only collection later through a research subagent per the review procedure.
Save `research/cuda-version-adoption.md` and `research/data/` at release root:
query ClickPy runtime downloads and cross-check pypistats/pepy; retain query text,
raw responses, URL, retrieval date, package/version mapping and denominator.
Compare matching completed windows; label incomplete windows and download/bot noise.
If inaccessible, find an accessible quantitative source or keep review incomplete;
default package metadata cannot replace adoption shares. Plan primary-source reads
of PyPI torch metadata, llama.cpp/Ollama release assets, NVIDIA JetPack archives,
release notes/PTX/compatibility/driver lifecycle, peer maintainers' supported builds,
and official distro/cloud images. Separate proxies; never invent a combined fleet.
Review each CUDA version's importance, lowest viable per-major toolkit, library
compatibility cells and available venues; submit changed ratings/actions for ruling.

Proposed starting design, requiring fit review: explicitly install one Python
variant, then keep runtime `auto|cpu|cuda:N` within that artifact. Candidate 13 wheel
coordinate is `fathomdb==0.8.29+cu130`, import `fathomdb`, via exact digest URL on an
approved first-party channel; no co-install with 12.6 and no assumption PyPI accepts
local versions. Candidate Node identity `fathomdb-linux-x64-gnu-cuda13` is separate
from the existing generic package; decide explicit pre-load selection versus a
reviewed presence-based selector before implementation. npm os/cpu/libc cannot
choose CUDA major. Specify conflict/version/missing-variant errors before dlopen;
never load both majors to guess compatibility or silently satisfy forced 13 with 12.
Rust remains a source-consumer build with qualified exact pins and explicit toolkit/
CUDARC_CUDA_VERSION identity; decide feature-unification constraints and publishable
consumer instructions. Name any 13 CLI archive distinctly or explicitly defer it.
Review alternative distribution/naming and automatic artifact selection against
these needs; revise local design and governed interfaces before code, then rule.

Keep 12.6 existing artifacts. 13.0 builds must measure PTX/driver/library requirements,
not infer them from the installed toolkit or minor-version compatibility alone.
Preserve static cudart/lazy driver loading, no shipped NVIDIA shared libraries and
no CUDA/NVIDIA NEEDED entries, typed wrong-major/missing-symbol refusal and reports.
Do not widen x86 compute 7.5, Tegra 8.7, generic Arm or architecture claims by naming;
Slice 140 multiarchitecture work is separate and selected only with its own evidence.
Apply [quality-roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md)
boundary/anomaly and installed-package checks; defer exhaustive CUDA/fuzz matrices.
New evidence is dual-major selection/delivery, not a reason to rerun all old cells.

## Dependencies, tasks and safe integration

- Hard: recorded 9/10 authorization, reviewed needs/design, owned environment and
  approved build/qualification venue; settle distribution identities/channel,
  driver/PTX/library contract and any policy change before corresponding build.
- Conditional: selected 140 fork changes qualify in their forks before exact pin
  adoption and 120 builds. Pin chain cudarc→Candle→FathomDB; 130 affected final
  qualification follows 120 integration, not a prerequisite for starting design.
  No 120↔130 cycle. Reset 110 is a dependency only for an affected selected remedy.
- Q5 access/approved venue and130 evidence precede any Tegra 13 support statement;
  no Jetson reflash, driver upgrade or host availability assumption. Slice 7/9/10
  must resolve .29 trusted-route prerequisites; .28 first-tag exception is scoped.
- Serialize release.yml, artifact contracts, Cargo pins, Python packaging, Node
  loader and public docs with 10/110/130/140/160; one writer per isolated checkout.

1. Complete research/review and local requirement→test map; independently review
   needs, architecture fit, selection conflicts and exact policy/delivery decisions.
2. For approved behavior changes stage/commit meaningful failing regressions first:
   package selection/version conflicts, wrong-major/symbol errors, driverless auto,
   forced CUDA and immutable rehearsal inputs; preserve existing fallback controls.
3. Implement per-artifact input axes and approved loaders/package routes; build only
   on authorized venues after owned-tool preflight, pin decisions and review.
4. Install retained bytes outside source imports; execute A3–A6 per selected scope,
   retain failures/raw receipts; hand affected platform/pool/soak cells to 130.
5. Run scoped checks during work, full agent gate on final code phase, affected
   package/native parity and actionlint; docs use Markdown/public-doc validators.
6. Obtain independent code/evidence review, sample RED assertions, resolve findings,
   then recheck affected integrated seams; provide 160 exact artifact/consumer map.

## Open prerequisites and exit

Quantitative sources, names/channels/selection, Rust/CLI scope and permitted venues
remain open. Exit needs A1–A6 evidence or explicit owner deferral, reviewed contracts
and unqualified-platform exclusions. Post-publication consumers remain 160 work;
this draft claims no review approval, policy ruling, execution pass or publication.

## Fork scope boundary (owner seq-367)

Only required repairs from .29 Slice140 may supply affected pins here. The
[broader .30 maintenance](../../../0.8.30/fork-maintenance-plan.md) is not a
prerequisite. A need for that broader work returns to the owner for explicit
scope review; it is not pulled into .29 automatically.
