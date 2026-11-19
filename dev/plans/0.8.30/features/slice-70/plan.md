---
title: Slice 70 integrated qualification and release readiness
status: PROPOSED
slice: 70
target_release: 0.8.30
---

# Slice 70 — Integrated qualification and release readiness

Qualify the integrated candidate for the support and behavior it actually claims.
R30-QUAL joins the selected Slice 20–60 exits. R30-TEST accounts for the selected
test improvements and their discriminating oracles. The [release plan](../../plan.md),
[requirements](../../../../requirements.md) and [acceptance](../../../../acceptance.md)
own the release scope and existing gates.

## Entry and preparation

- Compare the commissioned checkout and current `origin/main` with reviewed draft
  `f7fb1da9a81ee226c70ae347fa2fab3e64ac3c25`. Inventory relevant changes since drafting,
  related source and workflow functions, assigned checks, platform/pin inputs and
  allocated drafts. Keep, adjust or reject each affected qualification claim against
  actual source and the owner-selected Slice 9 scope.
- Require the selected reviewed and integrated Slice 20–60 exits and final consumed
  native/fork pins for combined readiness. Slice 10 owns checkout preparation.
  Slice 20 owns delivery diagnostics, 30 correctness, 40 comparable performance,
  50 lifecycle, and 60 fork qualification and exact consumption. Preparation and
  unaffected checks can proceed while a distinct required row remains open.
- Inspect the [architecture](../../../../design/fathomdb-data-plane-architecture-v2.md)
  and [release routes](../../../../design/release.md). Trace engine → SDK/bindings/MCP
  → configured build → actual installed process. Compare
  [0.8.29's matrix and limits](../../../0.8.29/features/slice-160/qualification.md) and
  [retained results](../../../0.8.29/features/slice-160/status.md) with current source.
- Check the .29 supplemental Tegra CUDA wheel/addon and Pages result under seq-398
  in the [.29 release state](../../../release-state-0.8.29.json) before applicable .30
  Tegra qualification. That delivery remains .29-owned. Generic ARM64 CPU and
  retained .28 downloads cannot supply version-matched Tegra proof.
- This slice verifies release behavior and closes scoped documentation. Send product
  or executable-check defects to their owning slice for repair. This proposal grants
  no hosted dispatch, new GPU session, reset, paid campaign or publication.

## Local needs, requirements and acceptance

These are release-local acceptance checks. Retain the canonical AC identifiers
and thresholds. Confirm each command's current registration and actual inputs.

| Need and requirement | Falsifiable acceptance and maintained proof |
| --- | --- |
| R30-QUAL / integrated source | Run `./scripts/agent-verify.sh` in the stable, clean candidate checkout with its owned tools and venv. Actual workspace-wide Clippy and type check run on all targets, with registered Rust, Python and affected TypeScript tests and lint in the gate's prescribed order. Retain exits and counts. [Verifier](../../../../../scripts/agent-verify.sh). |
| R30-QUAL / feature execution | Run `python3 scripts/check-test-target-coverage.py` and `./scripts/test-feature-complete.sh` for the applicable maintained registrations. Check executable test IDs, actual feature sets, model/runtime assets, ignored/benign-marker/excluded classifications and stale entries. A matrix/list command, zero selected tests, an early return or a host exclusion cannot supply runtime proof. [Matrix](../../../../../scripts/test-feature-matrix.toml), [skip allowlist](../../../../../scripts/test-skip-allowlist.toml). |
| R30-QUAL / strict security | `STRICT=1 AC037_LIVE_OPTIONAL=0 ./scripts/agent-security.sh` on a capable executor produces 0 violations, 0 blockers and 0 downgrades. Verify AC-036 actual ptrace and AC-037 live deny-egress and catch controls, plus existing FTS5/AST/removal checks. A downgraded verifier copy needs the authoritative live result. Run `./scripts/security/gitleaks-current.sh` at its existing scope. [Security runner](../../../../../scripts/agent-security.sh), [CI contract](../../../../../.github/workflows/ci.yml). |
| R30-QUAL / long concurrent state | Run the exact unchanged long selectors below with actual positive progress. AC-021 exercises the 60-second reader/admin DDL window with successful projection add/drop and rebuild, zero SQLITE_SCHEMA warnings. The AC-059b race exercises 1,000 searches, committed overlapping writes, advancing visible hits and zero row-count/cursor and hit-cursor violations. [DDL test](../../../../../src/rust/crates/fathomdb-engine/tests/lifecycle_reliability.rs), [cursor test](../../../../../src/rust/crates/fathomdb-engine/tests/cursor_read_after_write.rs). |
| R30-QUAL / binding performance | Preserve the canonical release profiles and gates. `./scripts/run-ac011-write-throughput.sh` requires the registered Linux x64/NVMe fixture and 5-second warmup/60-second measurements (1 KB ≥1,000/s and 100 KB ≥100/s). Run the AC-012/013 test functions under AC-076/072 budgets via isolated release `perf_gates`, N=10,000, with the 20/150 ms text and 80/300 ms dim-768 vector p50/p99 bounds. Run AC-081a/b's existing seven-process absolute batch protocol (≤500 ms sequential, ≤100 ms eight-reader), retain warnings, and require AC-081c reader progress. [Performance gates](../../../../design/perf-gates.md), [AC-081 driver](../../../../../scripts/perf-experiments/run-slice80-ac081-cell.sh). |
| R30-QUAL / real-corpus fidelity and stress | Keep AC-073's real-corpus stress limit `max(10 × baseline p99, 150 ms)`, and AC-075's real-embedder vector top-10 exact-f32 oracle and one-sided CI-high ≥0.90. Reconcile the accepted .29 TC5 F/Q8 bridge and frozen 7,667-document/100-query/K192 input with current authority and final artifacts. Synthetic recall/stress remain report-only. Retain legacy EU7 overall exits, the retired ratio scope and 100k tracked results separately. [Canonical ACs](../../../../acceptance.md), [.29 routes](../../../0.8.29/features/slice-160/qualification.md). |
| R30-QUAL / composed behavior and latency | Verify the affected engine, SDK/Python/TS and both MCP formats against independent expected records, provenance, refusal, UTF-8/budget, cursor and persistence facts. Use the 30/50 regressions and 40's matched end-to-end tail/throughput/resource results and adoption decision. Preserve original S50 nonpass and fixed-state exactness. Rendered examples and production-path comparisons without an independent fact cannot prove shared predicates. [MCP contract](../../../../interfaces/mcp.md). |
| R30-QUAL / delivered candidate behavior | Require installed Python wheel and npm main/platform packages and CLI product checks on Linux x64/ARM64, macOS x64/ARM64 and Windows x64. Keep Tier-1 Rust build/test, distinct Windows WAL and a downstream source-archive SDK/CLI consumer. Use actual resolved native path/digest, declared floors, strict TS consumer, persist/read/missing/separate-process reopen/close/exit. [Tier-1 ADR](../../../../adr/ADR-0.6.0-tier1-ci-platforms.md), [POSIX smoke](../../../../../scripts/release/smoke/smoke-local-native-artifacts.sh), [Windows smoke](../../../../../scripts/release/smoke/smoke-local-native-artifacts.ps1), [collector](../../../../../scripts/release/native-artifact-receipts.py). |
| R30-QUAL / declared CUDA candidates | On applicable final bytes and pins, retain exact artifact floor/linkage, driverless auto/CPU and forced typed CUDA refusal, plus required positive selected device/PID/UUID and explicit CPU nonuse under existing approved controls. Cover relevant lifecycle and safe drop only to the extent actually qualified. Keep generic ARM CPU, Tegra and x64 CUDA evidence distinct. New CUDA 13 delivery belongs to .31. [CUDA artifact checks](../../../../../scripts/release/cuda-artifact-contract.sh), [.29 native control routes](../../../0.8.29/features/slice-160/qualification.md). |
| R30-QUAL / contract and docs convergence | Reconcile [Rust](../../../../interfaces/rust.md), [SDK](../../../../interfaces/rust-sdk.md), [Python](../../../../interfaces/python.md), [TypeScript](../../../../interfaces/typescript.md), MCP/affected CLI and public claims with actual behavior. Run SDK surface parity, applicable Markdown/state checks and `./scripts/agent-lint-docs.sh` plus `mkdocs build --strict` for changed public docs/navigation. Check changelog, Axis W/E and exact-candidate nonpublishing release route. [SDK parity](../../../../../scripts/check-sdk-surface-parity.py), [release gates](../../../../../scripts/verify-release-gates.sh). |
| R30-TEST / defensible coverage | Account for every test improvement selected in 9. Give its implementing owner (20/30/50), exact actual test or authorized disposition, independent or real-store oracle, and one concrete wrong result the oracle rejects. For proposed consolidation show overlap and observed cost/misses, then verify the retained oracle covers the removed semantic/platform claim. [Quality roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md). |

## Coverage closure selected under seq-406

Use the [coverage audit](../../coverage-audit.md) and Slice50's added R30-TEST
obligations to close the following explicit proof rows. Existing tests are
reusable inputs; a source assertion, ignored placeholder, registered name or
passing wrapper alone is not evidence that the selected body ran.

| Required row | Exact-candidate proof |
| --- | --- |
| Canonical object, indexes, temporal and graph cohesion | Run the authored cross-index lifecycle scenario and small operation model from50. Record exact expected revisions, ownership, current/history visibility, node-time boundaries, shipped edge-recency semantics, relationship direction/kinds, filter-before-cap and source evidence. Check lag/readiness and converged post-drain state, rollback and reopened state; do not require equal ranking across different modalities. Record creation/enrollment/population/update/removal, generation and cursor controls from existing index suites with actual feature selection. |
| Crash, WAL and persistent failure recovery | Execute existing selected CPU child-kill and first/persistent SQLite-full/provider-fault cases on relevant final source, plus only the missing composed projection/dependency seam from50. Require delivered fault, exact surviving state, pending work/recovery, unrelated positive work and resource release. Include held-reader WAL/busy-checkpoint/release/checkpoint/reopen behavior and the existing Windows WAL regression on its actual supported target. Declare process-crash versus power-loss scope. |
| Product fuzz and regression replay | Run the implemented50 boundary fuzz smoke with exact target, method, byte/depth/time bounds, nonzero input count and seed/corpus. Replay minimized regression inputs in ordinary suites. Distinguish Proptest/Hypothesis, random simulation and coverage-guided fuzzing; a queue simulation or list-only invocation does not close product parser fuzz coverage. Report remaining native/FFI/sanitizer gaps without claiming a broad campaign. |
| All doctor command outcomes and effects | Derive the current command inventory from the CLI and account for all17 existing variants. For each, identify selected success, findings/refusal, busy, malformed and unsupported outcomes as applicable; assert semantic fields and exact exit class, not JSON parseability alone. Verify declared effects: immutable `data-plane-integrity` preserves database/companions; ordinary doctors may open an Engine; `safe-export` writes verified artifacts, `warm-cache` touches approved cache/network and `recompute-mean` mutates transactionally. Include actual full-integrity findings and WAL-only export/recover composition. Require the pre-action paired-copy witness: selected committed record present with WAL, absent from readable main-only copy, independent control present in both; a checkpointed record does not supply WAL-only proof. Use one thin installed final CLI route plus target-specific tests; selected GPU positives require actual approved final artifact/venue proof, while unsupported branches remain labeled. Do not silently run downloads, hardware faults or a platform Cartesian product. |
| Behavioral SDK/MCP parity | Match one independently authored fixture across Rust/Python/TS and structured/compact MCP, within each surface's actual capabilities. Compare expected data, Unicode, identity/provenance, temporal/filter behavior, typed errors, budget and store/epoch/continuation facts. MCP has no general mutation or exact SDK evidence-resolution tools; exercise those through SDKs rather than inventing parity. Retain actual installed binary/module identity, exact source/version, separate-process reopen and natural exit. Return confirmed executable defects to50; no deep matrix duplication in every language. |
| Semantic oracle sensitivity | For selected critical Fathom-owned predicates, retain the wrong-condition/wrong-order/missing-cleanup variant and the test that rejects it, or another concrete independent oracle already demonstrating that sensitivity. Collect a focused line/branch overlay for changed high-risk paths where existing tooling supports it, identify missed conditions and their consequences, and keep instrumentation separate from optimized latency measurements. If measurement is unavailable, label the manual branch/oracle map as such and retain the measurement gap explicitly. Preserve defensive checks. This is a targeted assessment of tests, not a whole-workspace percentage, MC/DC requirement or general mutation framework. |

For each row record owner, real command/features, selected and executed cases,
passed/failed/skipped counts, original failures, focused recovery and input
applicability. Resolve overlap with existing suites rather than adding duplicate
campaigns. Every selected requirement needs passing proof or explicit owner
disposition; missing execution remains open. Erasure stays a lower-priority
critical-regression check. Correct stale graph-seeding documentation to match
existing behavior during contract/doc convergence; new fused seeding, namespace
or entity-resolution behavior is not selected by this coverage work.

Use the current [long-gate route](../../../../../scripts/check.sh) or the
following reviewed, focused selectors to avoid repeating unaffected work.
For latency, the existing test names are AC-012/013; their accepted budgets are
AC-076/072. Select the actual functions below, never nonexistent `ac_076` or
`ac_072` filters. Require actual nonzero execution and numerical output with
1,000 measured samples. AC012 defaults to100k (tracked/nonbinding), so the10k
setting is explicit; absent `AGENT_LONG` can early-return. Unset `AC_FULL_SCALE`,
`AC013_SCALE_TREATMENT` and `AC013_DRAIN_TIMEOUT_MS` for the canonical protocol.
Run on the selected capable release executor, separate from compilation.

```bash
AGENT_LONG=1 AC012_CORPUS_N=10000 cargo test --release -p fathomdb-engine \
  --features test-hooks --test perf_gates ac_012_text_query_latency_on_fts5_path \
  -- --exact --nocapture
AGENT_LONG=1 AC013_CORPUS_N=10000 AC013_VECTOR_DIM=768 \
  cargo test --release -p fathomdb-engine --features test-hooks \
  --test perf_gates ac_013_vector_retrieval_latency -- --exact --nocapture
AGENT_LONG=1 cargo test -p fathomdb-engine --features operator \
  --test lifecycle_reliability ac_021_zero_sqlite_schema_warnings_under_concurrent_reads_and_ddl \
  -- --exact --nocapture
AGENT_LONG=1 cargo test -p fathomdb-engine --test cursor_read_after_write \
  projection_cursor_bounds_observed_row_count -- --exact --nocapture
```

## Design and execution sequence

1. Draft the evidence applicability design before execution. Identify which changes invalidate runtime,
   feature, performance, target or distribution proof. Bind retained evidence to the actual producer,
   relevant source, features, lock/pins, toolchain, artifact bytes, fixture/model, protocol and target.
   Reuse only the matching claim and document the material match. Rebuild or rerun affected claims
   when the inputs differ or cannot be established. Preserve existing independent oracles.
2. Resolve the hard questions on source, host and artifact equivalence, test selection and zero-run
   detection, and why any unaffected result still applies. Consult pinned Cargo/libtest and target
   loader/packaging primary documentation when local contract or implementation leaves uncertainty,
   including test selection and feature unification, wheel/ABI floor and native addon resolution.
   Check conclusions against actual current commands, logs and manifests. Return setup and pin gaps
   to their owners. Record needed design/interface/ADR changes and get direct Astra-medium design review.
3. Have direct Astra-medium review the updated plan, with at most one FIX-1 and re-review. Commit
   after PASS. Preserve findings and hold the affected transition on continued failure.
4. After commissioning, freeze stable source, generated views and owned environment. Run the
   full source gate once for the coherent phase and execute the required omitted rows. Continue
   failed, affected and unrun registrations with exact scoped routes. Record any assembled
   completion as assembled, preserving original wrapper exits, cancellations, failures and skips.
   Scoped Markdown, contract and release-state checks cover later metadata edits. Broad source
   changes can justify another coherent source phase.
5. If verification finds a product or harness defect, return it to 20/30/40/50/60 for genuine
   failed assertion RED, separate GREEN and focused follow-up. Direct Sol6.2-high reviews new
   code changes. Have direct Terra independently verify actual affected test executions and
   installed artifacts, including their exits, scope and input reuse. Listing, compilation,
   symbol presence and receipt text alone cannot satisfy an execution requirement.
6. Close with one concise status and release-state update containing actual exits, skips, reused scopes,
   blocked gaps and owner dispositions. Every mandatory row needs applicable passing proof or an
   explicit owner disposition. Resolve critical wrong-result/data-loss issues or obtain the owner's
   explicit disposition. Unknown or unavailable required evidence remains open.
7. Merge any temporary branch into the owning release branch and clean only owned inactive scratch
   and worktrees after preserving evidence. Serialize state/source integration and isolate concurrent
   writers. Hand Slice 80 the qualified immutable candidate, routing and applicable limits.
   Registry installs and actual publication remain Slice 80 work with separate authority.

## Open choices

Confirm approved venues and current fixture/model availability, which retained scopes survive the
final inputs, and any required execution/location authorization. Record any outstanding .29 Tegra
delivery limit before an affected .30 verdict. This draft records no runtime or artifact result.
