---
title: 0.8.24 Slice 3 — product-document draft CRUD
status: COMPLETE
target_release: 0.8.24
---

# Slice 3 — product-document draft CRUD

**Observed:** 2026-08-23. This is a draft disposition of the 0.8.24 planning
labels. It does not edit `dev/needs.md`, `dev/requirements.md`, or
`dev/acceptance.md`, mint authoritative identifiers, or authorize a feature.

## Authority and review result

The existing product documents already own the broad outcomes: cross-platform
delivery (`NEED-016`), no-footgun embedded deployment (`NEED-003`), concurrent
Windows-capable operation (`NEED-020a`), performance (`REQ-010` / `AC-076`),
and release safety (`REQ-049`–`REQ-052`). The 0.8.24 drafts should therefore
add only the missing target-artifact and retry semantics; they should not turn
internal planning, documentation hygiene, or an unexamined external incident
into product requirements.

## Need disposition

| Input | Disposition | Draft canonical operation | Primary allocation |
| --- | --- | --- | --- |
| N24-1 explicit Jetson CUDA selection | **Merge and adjust** with N24-2. | Update `NEED-016` to include an explicitly selected, documented prebuilt artifact for each supported target/accelerator route; generic CPU installation must remain clear. | 30 |
| N24-2 Windows CUDA without local compile | **Merge and adjust** with N24-1. | Same `NEED-016` update, conditional on an owner-approved Windows CUDA SDK matrix. It promises no local compile only for a declared supported route. | 40 |
| N24-3 retry a partial publication safely | **No separate need.** | Maintainer release behavior, not a user outcome. Strengthen `REQ-050` and its acceptance criterion instead. | 60 |
| N24-4 installed-package proof on the target | **No separate need.** | Release evidence, not a new user need. Strengthen `REQ-052` and its acceptance criterion instead. | 60 |
| N24-5 evidence-led dependency maintenance | **Reject as canonical product need.** | Retain as Slice 7 operating constraint; no `dev/needs.md` change. | 7 |
| N24-6 current repository/release links | **Reject as canonical product need.** | Reader-facing documentation maintenance; no product contract change. | 7 |
| N24-7 unambiguous engineering navigation | **Reject as canonical product need.** | Internal repository navigation; no product contract change. | 7 |
| N24-8 safe cleanup provenance | **Reject as canonical product need.** | Repository-maintenance safeguard; no product contract change. | 7 or later |

## Requirement and acceptance-criterion draft CRUD

The labels below are temporary. Slice 6 must obtain owner approval before the
owning feature edits a canonical document or assigns a real `REQ-*` / `AC-*`
identifier.

| Inputs | Draft operation | Draft requirement / criterion | Allocation |
| --- | --- | --- | --- |
| R24-1, R24-8 | **Create, conditional.** | **REQ-TARGET-TEGRA:** an approved Tegra CUDA distribution has an explicit public identity that cannot be resolver-confused with generic Linux ARM64 CPU. **AC-TARGET-TEGRA:** a clean Jetson install follows the documented explicit identity, while generic CPU install is separately demonstrated; installed open/write/search/close/exit and GPU-selection evidence identify the candidate version. | 30 (identity/contract); 60 (installed smoke) |
| R24-2, R24-9 | **Create, conditional.** | **REQ-TARGET-WINDOWS-CUDA:** Windows CUDA support exists only for an owner-declared Python/npm matrix and a named remote CUDA executor; unsupported routes fail/document clearly and require no local Windows compilation. **AC-TARGET-WINDOWS-CUDA:** retained evidence names executor, GPU/toolchain, artifact provenance, and SDK matrix; the installed supported artifact completes the Windows smoke. | 40 (contract/artifact); 60 (installed smoke) |
| R24-3, R24-4, R24-10 | **Update `REQ-050`; supersede/adjust `AC-054`.** | Replace the impossible “partial publishes are forbidden” wording with a completion invariant: no release is marked complete while a required artifact is failed, missing, or unverified; retry uses exact-version immutable-existing no-ops and fails closed on registry uncertainty. **AC-PUBLISH-RETRY:** inject or simulate each publisher’s existing-version and query-error paths; existing valid artifacts are skipped, uncertain state blocks completion, and a missing target remains publishable on retry. | 60 |
| R24-3, R24-10 | **Update `REQ-052`; supersede/adjust `AC-056`.** | Expand the wheel-only release gate to every affected published binding artifact. **AC-INSTALLED-TARGET-SMOKE:** a fresh target-native install, not a source-tree build or artifact upload, performs the documented end-to-end lifecycle; the target, version, package identity, and artifact provenance are retained. | 60, with target inputs from 30/40 |
| R24-5 | **No canonical CRUD now.** | `REQ-010` / `AC-076` already own text-path performance. The owner-approved SCALE-02 rule is a Slice 20 release decision and targeted correctness evidence, not a new generally applicable performance threshold. | 20 |
| R24-6, R24-12 | **Defer canonical CRUD.** | `NEED-020a` and existing lock/reliability requirements provide the current contract. Do not create a Windows WAL requirement until the external-client evidence is attributed; Slice 50 may propose one only if a real contract gap or defect is established. | 50 |
| R24-7 | **No canonical CRUD now.** | CI topology is release design/process, not a user requirement. Preserve the Slice 10 no-change presumption. | 10 |
| R24-11 | **No canonical CRUD.** | Evidence-led dependency remediation is Slice 7 maintenance discipline, not a product promise. | 7 |
| R24-13, R24-14, R24-15 | **No canonical CRUD.** | Public-link currency, engineering navigation, and safe cleanup are bounded documentation/repository maintenance. | 7 or later |

## Trace and proposed document operations

| Draft | Need → requirement → criterion trace | Canonical target | Why the current text is insufficient |
| --- | --- | --- | --- |
| Tegra target distribution | Updated `NEED-016` → `REQ-TARGET-TEGRA` → `AC-TARGET-TEGRA` | `dev/needs.md`, `dev/requirements.md`, `dev/acceptance.md` | `NEED-016` promises platforms but not unambiguous CPU/CUDA package selection; `REQ-052` is wheel-only. |
| Windows CUDA target distribution | Updated `NEED-016` → `REQ-TARGET-WINDOWS-CUDA` → `AC-TARGET-WINDOWS-CUDA` | Same three documents | Existing Tier-1 Windows support is CPU/build coverage and does not declare a CUDA SDK or remote-executor contract. |
| Retry-safe release completion | Existing release-quality need → updated `REQ-050` → adjusted `AC-054` | `dev/requirements.md`, `dev/acceptance.md` | `REQ-050` says partial publish is forbidden even though the accepted design uses safe retry after a partial immutable publish. |
| Target-installed proof | Existing release-quality/deployment need → updated `REQ-052` → adjusted `AC-056` | `dev/requirements.md`, `dev/acceptance.md` | The current wording names only a PyPI wheel; npm and new target-specific artifacts need the same product-level proof rule. |

## Allocation and readiness boundary

The four retained canonical CRUD groups have been inserted into the overall
plan's prework-to-feature register. Slice 30 owns only Tegra product shape;
Slice 40 owns only Windows CUDA product shape; Slice 60 owns publish-retry and
installed-target proof. Slice 7 must not implement any of them. A feature can
promote its row from draft to ready only after its prerequisite owner decision
and its own design review.

## Evidence

- `dev/needs.md`: NEED-003, NEED-006, NEED-016, NEED-020a.
- `dev/requirements.md`: REQ-010, REQ-049–REQ-052.
- `dev/acceptance.md`: AC-054 and AC-056.
- [Slice 0 decision brief](slice-0-decision-brief.md),
  [publication topology](publication-topology.md), and
  [CI/release controls](ci-and-release-controls.md).
- [Slice 1 design review](slice-1-design-review.md) and
  [Slice 2 design review](slice-2-design-review.md).
