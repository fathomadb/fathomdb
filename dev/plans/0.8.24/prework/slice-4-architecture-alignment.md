---
title: 0.8.24 Slice 4 — architecture and code-alignment review
status: COMPLETE
target_release: 0.8.24
---

# Slice 4 — architecture and code-alignment review

**Observed:** 2026-08-23. This is a high-level, code-grounded review of the
Slice 0–3 proposals. It makes no source, workflow, package, architecture, ADR,
interface, or release-state change.

## Authority and current seams

`dev/architecture.md` delegates release behavior to `dev/design/release.md`.
The latter, the accepted tier/platform/package ADRs, and the current release
workflow are authoritative over a plan's wording. The review inspected current
workflow jobs, release scripts, Python co-install logic, TypeScript loader and
platform package metadata, engine WAL tests, and the retained performance
branch—not labels or test names alone.

## Alignment matrix

| Slice | Current code/design evidence | Classification | Required draft-to-ready outcome |
| ---: | --- | --- | --- |
| 10 CI | `ci.yml` has a mandatory classifier and proportional Rust/Python/TS/Windows/native-artifact routes; `release.yml` remains tag/dispatch-only. Slice 0 recorded that the routing landing is already on main. | **Aligned/existing.** | Begin with no change. Add CI only if the ready plan proves a new Tegra/Windows route has no current selector, transfer, or smoke coverage; serialize any shared workflow edit on current main. |
| 20 performance | The retained branch is 811 lines across engine source plus SCALE-02/correctness tests; it is absent from main. Slice 0 records a selected streamed-boundary-tie decision rule and no confirming benchmark. | **Aligned/extension seam.** | Write a Slice 20 implementation design that reviews the whole branch delta, preserves the recorded top-k/fidelity constraints, and defines targeted RED/GREEN correctness proof. Do not cherry-pick individual commits or rerun a benchmark by default. |
| 30 Tegra | `build-python-cuda-tegra.sh` produces a local `+tegra` proof wheel. `_coinstall.py` expressly says `fathomdb-tegra` is not shipped and warns classic Tegra users about generic builds. | **Decision required / extension seam.** | Owner first chooses a public distribution identity. Then Slice 30 writes a new package-identity/compatibility design or ADR, updates the now-opposed co-install/documentation contract, and defines explicit CPU-versus-CUDA selection plus Jetson proof. |
| 40 Windows CUDA | `release.yml` has `windows-latest` CPU Python/N-API build and publish jobs; `platform.ts` maps Windows to `fathomdb-native-win32-x64-msvc`. No remote Windows CUDA runner, CUDA toolchain, or CUDA platform-loader identity is present. | **Decision required / net-new.** | Owner first chooses Python, npm, or both and names the remote Windows CUDA executor. Slice 40 then writes the artifact/loader/provenance design (and an ADR if the package/platform contract changes); hosted Windows CPU results cannot be repurposed as CUDA proof. |
| 50 Windows WAL | `ci.yml` executes engine checkpoint diagnosis and installed/source attribution jobs. Engine tests include `wal_attribution_owned_reader_typed_refusal_then_post_release_sampler_is_recorded` and close-boundary controls. The external Memex job remains unavailable. | **Aligned/existing evidence boundary.** | Slice 50 compares the actual external client evidence to this installed Python path. It must produce attributed reproduce/not-reproduced/insufficient-evidence findings before proposing a code, interface, or architecture change. |
| 60 publishing and smokes | `release.yml` has idempotent publisher jobs, five platform npm lanes, explicit post-publish smokes, and `next` promotion. Release helpers query exact versions and fail closed. `dev/design/release.md` documents safe partial retry, while REQ-050/architecture still call publication atomic. | **Stale architecture/documentation.** | Update the approved release semantics from impossible atomicity to retry-safe completion; retain CPU lane/publisher no-op/fail-closed proof; extend installed-package smoke to any approved new target distribution. |
| 70 integration | `verify-release-gates.sh` binds the candidate SHA/tag rules; release workflow separates dry run, tag publication, and GitHub release. | **Aligned/existing.** | Assemble only approved feature evidence, revalidate branch/main ownership, and prepare an owner decision. It cannot substitute a release tag, registry mutation, or unapproved external action. |

## Architecture/documentation discrepancies

| Finding | Current state | Proposed operation | Owner |
| --- | --- | --- | --- |
| Release completion wording conflicts with mechanics | `dev/design/release.md` describes immutable-artifact partial retry; `REQ-050`, AC-054, and the architecture release row describe atomic publish. | Conditional Slice 60 update proposed by A24-1: define completion as all required artifacts present/verified and retry as safe exact-version no-op or fail-closed. | 60 |
| Tegra public identity contradicts current co-install posture | Current Python code intentionally treats `fathomdb-tegra` as unshipped and `+tegra` as local only. | Conditional Slice 30 design/ADR must explicitly supersede the relevant warning/selection assumptions before a public identity is advertised. | 30 |
| Windows CPU package identity is not a CUDA identity | ADR-0.8.22 and `platform.ts` govern the existing CPU npm route only. | Conditional Slice 40 design/ADR must preserve CPU behavior and introduce a separate chosen CUDA shape if npm is selected. | 40 |
| Wheel-only smoke language underspecifies package matrix | Existing Tier-1 ADR requires platform installed smokes; REQ-052/AC-056 name only a PyPI wheel. | Conditional Slice 60 requirement/acceptance and release-design update to use target-native installed artifacts and provenance. | 60 |

## Code-alignment conclusions

1. There is no evidence that a new main CI workflow must be implemented. The
   existing proportional classifier is the correct default boundary.
2. There is no safe public Tegra implementation without an owner-selected
   distribution and a deliberate update to Python's current local-only guard.
3. There is no Windows CUDA implementation seam to activate. The present
   Windows jobs and package are CPU evidence only; a remote GPU executor and
   product matrix are real prerequisites.
4. The performance candidate is substantial, not a one-line optimization. Its
   retained evidence supports a bounded Slice 20 integration review, but not
   an unreviewed cherry-pick.
5. Existing WAL tests are useful FathomDB controls but cannot attribute the
   external client outcome. The correct state remains unknown until Slice 50.
6. The sole verified architecture drift is release-completion terminology,
   already routed to Slice 60. No current runtime architecture defect was
   established by this review.

## Interface and ADR implications

No public runtime SDK change is currently proposed. Packaging-only work must
not alter Python or TypeScript interfaces by accident. If a selected Windows
or Tegra route changes an installer-visible name, loader behavior, or
unsupported-path error, the owning Slice 30/40 plan must draft the matching
public documentation and interface treatment. A material package-identity or
Tier-1 platform-contract change needs its own ADR/successor; a release note is
not sufficient.

## Evidence

- `.github/workflows/ci.yml` classifier, Windows WAL, and local-native-smoke
  jobs; `.github/workflows/release.yml` jobs for CUDA preflight, Windows CPU,
  publisher, smoke, and promotion.
- `scripts/verify-release-gates.sh`, `scripts/release/{pypi,npm,cargo}-publish-if-new.sh`,
  `scripts/release/npm-inject-optional-deps.sh`, and
  `scripts/release/build-python-cuda-tegra.sh`.
- `src/python/fathomdb/_coinstall.py`, `src/ts/src/platform.ts`, and
  `src/ts/package.json`.
- `src/rust/crates/fathomdb-engine/src/lib.rs` Windows WAL attribution tests;
  performance branch `experiments/performance-0.8.23-plan-20260821` at
  `c7e83bfe` and its 811-line engine/test delta.
- `dev/design/release.md`, `dev/architecture.md`, and the ADRs cited in the
  Slice 3 architecture draft.
