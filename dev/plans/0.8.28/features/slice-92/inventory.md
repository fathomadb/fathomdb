---
title: Slice 92 — documentation audit inventory
status: ACTIVE
target_release: 0.8.28
---

# Documentation audit inventory

Audit baseline: `77b6f0b4d`. Astra independently audited the user, current-owner
and project cohorts. The disposition tables below bind the corrections to actual
authority and source; validation outcomes are in [status](status.md). Historical .26/.27 slice
plans and results are evidence; the Slice 92 plan/design own this correction.

## Confirmed claim corrections

| Claim/cohort | Authority and implementation/test witness | Disposition |
| --- | --- | --- |
| Fresh current format and refusal, formerly 34/35 | Accepted format-35 and explicit-maintenance ADRs; schema SCHEMA_VERSION=36; engine open admission and existing durability/open tests | Update current architecture, engine/recovery/migrations/actuation/bindings, Rust/wire/CLI interfaces and public compatibility/open guidance; preserve historical boundaries. |
| Provider compatibility, obsolete dense_disabled success | Accepted vector-compatibility-open-policy ADR; engine embedding-admission/open verification and Slice 50 tests; current interfaces | Replace attached-disabled/query-time compatibility fiction with failed-open/no-provider behavior, cached verification limits and actual report/errors. |
| Search visibility, properties and direct body FTS | Slices 80/90 approved contracts; retrieval SQL/filter/view paths and focused tests; three SDK signatures | Correct guide/reference/code documentation, pre-cap/ranking/limit semantics and metadata/no-provider limits; MCP remains .29. |
| CUDA/forks, SQLite, context pack and stop words | Accepted .28 ADRs; Slices 30/31/40/41/42 source/contracts and focused tests | Reconcile installation/platform/configuration/search guidance without new claims or dependency changes. |
| Maintenance/recovery | Accepted explicit-maintenance ADR and current contract; maintenance admission/queued-intent/fence code and Slice 85 tests | Explain current opt-in policy, exclusive windows, durable incomplete outcomes and limits; no new SLA or optimization. |
| Public truth checker fixed schema assumptions | Active state, schema constant, accepted format decisions; existing public-doc-truth controls | RED/GREEN existing checker for pending36 vs historical34; preserve receipt/target/platform/no-migration controls. |

## Maintained design owners

The existing catalog contains the following maintained owners at this baseline.
Their accepted ADR/interface authorities remain in current-owner-authority.json;
the source/test column below supplies the code-grounded review witness. Catalog
checks prove coverage/topology, not the truth of prose.

| Owner under dev/design | Disposition | Reviewed current claim | Source/test or bounded enforcement witness |
| --- | --- | --- | --- |
| [README.md](../../../../design/README.md) | Retain | Catalog-backed navigation and authority hierarchy remain accurate. | `scripts/check-design-lifecycle.py` |
| [actuation.md](../../../../design/actuation.md) | Update | Current fresh format36 and maintenance ownership; five-operation V1 unchanged. | `src/rust/crates/fathomdb-engine/tests/slice25_actuation.rs` |
| [bindings.md](../../../../design/bindings.md) | Update | Three application SDK peers, current admission and .28 filter/context/maintenance links. | `src/rust/crates/fathomdb-sdk/src/options.rs` |
| [embedder.md](../../../../design/embedder.md) | Update | Failed-open provider policy retained; obsolete default-compiled test seam and migration framing corrected. | `src/rust/crates/fathomdb-engine/tests/provider_admission.rs` |
| [engine.md](../../../../design/engine.md) | Update | Format36, runtime vec0 ownership/head spans, current config and explicit window admission. | `src/rust/crates/fathomdb-engine/src/open.rs` |
| [errors.md](../../../../design/errors.md) | Update | Runtime MaintenanceError distinct from open configuration mismatch/corruption. | `src/rust/crates/fathomdb-engine/src/errors.rs` |
| [explicit-maintenance-erasure/contract.md](../../../../design/explicit-maintenance-erasure/contract.md) | Update wording | Delivered APIs/format36 and v1 immutable policy; approval history and guarantee unchanged. | `src/rust/crates/fathomdb-engine/tests/slice85_policy.rs` |
| [explicit-maintenance-erasure/production-design.md](../../../../design/explicit-maintenance-erasure/production-design.md) | Retain | Implemented format36 reconciliation and no-SLA/no-immediate-revocation limits accurate. | `src/rust/crates/fathomdb-engine/tests/slice85_physical.rs` |
| [fathomdb-data-plane-architecture-v2.md](../../../../design/fathomdb-data-plane-architecture-v2.md) | Update | Current .28 profile, storage/provider/retrieval/maintenance owners; .26 history retained. | `src/rust/crates/fathomdb-engine/src/open.rs` |
| [gpu-eval-activities-policy.md](../../../../design/gpu-eval-activities-policy.md) | Retain | Role-bounded GPU evaluation mandate; no new release campaign. | `scripts/lib/cuda-gpu-selection.sh` |
| [lifecycle.md](../../../../design/lifecycle.md) | Retain | Observability owner with accepted subscriber exceptions; no transfer of physical cleanup ownership. | `src/rust/crates/fathomdb-engine/tests/lifecycle_observability.rs` |
| [migrations.md](../../../../design/migrations.md) | Update | Current format36 fresh bootstrap separated from historical migration-loop rationale. | `src/rust/crates/fathomdb-schema/src/lib.rs` |
| [nested-source-projections.md](../../../../design/nested-source-projections.md) | Clarify | Shipped grammar and canonical historical-view attributes, not a new grammar. | `src/rust/crates/fathomdb-engine/tests/slice45_nested_source_projections.rs` |
| [op-store.md](../../../../design/op-store.md) | Update | Mutex-serialized caller-write transaction, not dedicated writer thread. | `src/rust/crates/fathomdb-engine/tests/op_store.rs` |
| [orchestration.md](../../../../design/orchestration.md) | Retain | Cross-release role runbook; this release remains direct/Astra by explicit owner instruction. | `scripts/preflight.sh` |
| [perf-gates.md](../../../../design/perf-gates.md) | Clarify | Historical threshold intent is distinct from current feature/path/tier registration and accepted replacements. | `scripts/agent-test.sh` |
| [perf-regression-detection.md](../../../../design/perf-regression-detection.md) | Retain; correct witness | Detector semantics unchanged; authority catalog points to actual CLI detector. | `src/rust/crates/fathomdb-cli/src/bin/perf-regression-check.rs` |
| [pinned-override-rot-guard.md](../../../../design/pinned-override-rot-guard.md) | Retain | Already records .28 registry fork pins and SQLite-patch retirement. | `scripts/check-pinned-override-rot.py` |
| [projections.md](../../../../design/projections.md) | Update | Generation/terminal rules retained; bounded pending selection includes stranded work below watermark. | `src/rust/crates/fathomdb-engine/tests/slice50_drain_scan_bound.rs` |
| [recovery-0.8.25.md](../../../../design/recovery-0.8.25.md) | Retain | Bounded generation/stored-state recovery owner, not old-store upgrade support. | `src/rust/crates/fathomdb-engine/src/projection_generation.rs` |
| [recovery.md](../../../../design/recovery.md) | Update | Current schema admission, maintenance/offline owner links and no-provider rebuild limit. | `src/rust/crates/fathomdb-cli/src/lib.rs` |
| [release.md](../../../../design/release.md) | Update | Exact registry-version polling, live qualification owner, historical bootstrap retained. | `.github/workflows/release.yml` |
| [retrieval-result-limits.md](../../../../design/retrieval-result-limits.md) | Clarify | Same-filter prefix premise and eligibility before fixed direct FTS caps. | `src/rust/crates/fathomdb-engine/tests/slice90_filtered_body_fts.rs` |
| [retrieval.md](../../../../design/retrieval.md) | Update | Filtered body FTS, independent views, metadata limits/native eligibility and failed-open recovery. | `src/rust/crates/fathomdb-engine/src/search_api.rs` |
| [scheduler.md](../../../../design/scheduler.md) | Clarify | Bounded pending scan/no-provider pending state; queue/retry/deadline limits unchanged. | `src/rust/crates/fathomdb-engine/src/projection_worker.rs` |
| [temporary-serial-rust-workspace-release-gate.md](../../../../design/temporary-serial-rust-workspace-release-gate.md) | Retain | Serial verdict and distinct parallel diagnostic remain current. | `scripts/test-rust-workspace.sh` |
| [vector.md](../../../../design/vector.md) | Update | 64-row vec0 chunks, lifecycle/validity, record-to-vector spans and maintenance semantics. | `src/rust/crates/fathomdb-engine/src/vector_storage.rs` |

## User and contributor routes

| Cohort | Disposition and scope | Verification route |
| --- | --- | --- |
| README and Python/TS/Rust facade/SDK/CLI package READMEs | Update current install target to pending .28; preserve exact-artifact verification and distinguish source consumers from registry proof. Embedder API/runtime READMEs explain standalone traits vs production admission. | Public-doc truth; package examples below; source docs/comments reviewed by Astra. |
| Site home, install/getting-started/compatibility/concepts and operation/reference landings | Update format36/refusal, bundled SQLite/toolchains and current configuration. Preserve platform floors and retained artifact digests; pending install instructions are conditional on publication. | MkDocs/links; existing public-truth/platform controls; generated freshness. |
| Python/TS/Rust references and filter guide | Update signatures/options, pre-cap filtering, metadata sentinel/projection-time and nonempty attribute exclusion of edge hits, independent views, failed-open/report/readiness and typed maintenance/errors. | Slice90/provider/slice80/slice85 source tests unchanged; representative external consumers below. |
| Context, frozen evidence, point-read, structured-hit and integrity guides | Retain reviewed contracts; context budget caveat and bounded close wording clarified where needed. Existing erasure guide remains explanatory maintenance owner. | Existing focused source witnesses plus runnable context/maintenance examples and CLI immutable inspection. |
| Positions/contributor/generated maps | Retain applicable parity/tokenizer/identity position boundaries; source-derived operation/contributor maps regenerated, manual detailed refs retained. | Existing generator --check, contributor navigation and code documentation review. |
| Release notes/changelog | Add pending .28 notes, current format36/provider/view/maintenance/removal entries and final fork dependency facts. Preserve prior release notes/measurements as historical, without treating them as current qualification. | Source/ADR comparison and MkDocs navigation. |

Selected executable examples: the complete README/Python quickstart; TypeScript
quickstart blocks compiled as an external consumer; Rust SDK README/reference
program in an external consumer with candidate workspace/active patches; CLI
`doctor data-plane-integrity` on a closed disposable example store. Additional
small provider-free examples exercise the published direct-filter, context-pack
and explicit-maintenance guidance. Run exact documented code where complete;
record fixture/assembly details for fragments. Registry installation and hosted
platform qualification remain Slice95/publication evidence, not source proof.

Independent execution found and corrected one retained guide claim: direct
writes accept caller-supplied opaque logical IDs; only extractor-ingestion paths
use the internal lowercase kind/name hash (`identity.rs`, `ingest.rs`). The
privacy caveat now applies specifically to those derived IDs. Native Linux npm
package READMEs also shed the stale .20 label and old repository URL; these
packaging-prose changes require a targeted repack/recheck, not native recompilation.

## Project and historical hygiene

| Cohort | Disposition and authority |
| --- | --- |
| dev/README, DOC-INDEX/detail maps, active architecture/design/interface navigation | Update current .28 ownership/target and canonical release-current selection; preserve one active architecture and catalog owner set. |
| needs/requirements/traceability/test plan | Keep stable .6 IDs and frozen acceptance unchanged. Add applicability pointers to accepted current decisions/interfaces; correct logical_id-only identity and implemented AC081 invocation. Historical migrations are fixtures, not public upgrade support. |
| Accepted ADR/index and current interfaces | Preserve decision text and history; correct contradictory current Rust/wire/CLI format statements to36, backed by schema/open source and accepted maintenance ADR. |
| Release plan/state/board and Slice95 handoff | Select pending public_docs_target_release .28 without publication mutation; register design/inventory and hand reviewed baseline to95 after verification. |
| Contributor/agent methods and tooling | Retain TDD/scoped/full gate/worktree/CI boundaries; checker regression fixes use RED/GREEN. No new lifecycle system, size gate, deployment or scope reallocation. |
| Historical corpus | Retain prior release notes, closed slice/progress/experiment/proposal records and frozen public baselines in place. They are evidence, not current semantic authority. No bulk deletion/move/archive or ledger rewrite. |
