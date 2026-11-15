---
title: Slice 160 test and qualification mapping
status: PROPOSED
target_release: 0.8.29
---

# Slice 160 test and qualification mapping

[Active owner dispositions](requirements.md#owner-dispositions) select .29
identity/atomic-feedback/source-seam work, hold real resets to .30 and move new
x86 CUDA13 delivery/quantitative evidence to .31. Seq-381 additionally moves Tegra CUDA13
130A to .31;130D/E and existing CUDA12.6/platform/artifact obligations remain .29; deferral is not qualification. Owner seq-387 excludes non-Tegra ARM+GPU B/C from every .29 gate and known-working environment claim.

[Requirements](requirements.md) join to [qualification rows](qualification.md).
Commands are future execution routes, not evidence that160 ran them. Read exact
current test bodies/feature selection before execution; no fixed historical
count, synthetic stand-in or production helper may become the oracle.

| Acceptance | Matrix coverage | Existing oracle/source and meaningful controls |
| --- | --- | --- |
| A160-1 | Q1; condition register | Actual selected slice requirements/review/exits, Git integration and raw logs; conditional40/140/150 no-run versus unknown required evidence. Failed/partial predecessors remain open. |
| A160-2 | Q2–Q10,Q13 | [Full verifier](../../../../../scripts/agent-verify.sh), [feature runner](../../../../../scripts/test-feature-complete.sh), [matrix](../../../../../scripts/test-feature-matrix.toml), [skip inventory](../../../../../scripts/test-skip-allowlist.toml). [Perf tests](../../../../../src/rust/crates/fathomdb-engine/tests/perf_gates.rs) own exact tier/assert parameters. [30 design](../slice-30/plan.md) owns frozen/current measurement inputs; accepted TC5 authority and legacy EU7 overall failure stay separate. |
| A160-3 | Q11,Q12 | [60 requirements](../slice-60/requirements.md), [60 test plan](../slice-60/test-plan.md), [compact-text contract](../../../../design/agent-access/compact-text-block.md), [remeasure harness](../../../../experiments/agent-remeasure/README.md). Same real store/view SDK plus independent expected IDs/fields/quoted refs/UTF-8 boundaries, both formats, filter-before-cap>100nonmatches/limit1, tight final serialized budget and zero-progress controls. No golden regeneration. Final70/80/90 contracts supersede only their approved fields. |
| A160-4 | Q13–Q16 | [100 plan](../slice-100/plan.md), [Tier1 ADR](../../../../adr/ADR-0.6.0-tier1-ci-platforms.md), [native installed smoke](../../../../../scripts/release/smoke/smoke-local-native-artifacts.sh), [collector](../../../../../scripts/release/native-artifact-receipts.py), [CUDA contract](../../../../../scripts/release/cuda-artifact-contract.sh), [130 acceptance](../slice-130/acceptance.md). Check actual artifact hash/target and process exit, missing-library/CPU nonuse/UUID controls; source-only or hook-only observations retain limits. |
| A160-5 | Q17,Q18 | [Release gates](../../../../../scripts/verify-release-gates.sh), [local rehearsal](../../../../../scripts/release/local-dry-run.sh), [release workflow](../../../../../.github/workflows/release.yml), [.29 route guard](../../../../../scripts/release/check-tag-cuda-route.sh). Actual runtime/package inputs and skipped jobs remain distinct; do not infer permission or invoke publishers. |
| A160-6 | Q19 | [Published Python smoke](../../../../../scripts/release/smoke/smoke-pypi-wheel.sh), [npm smoke](../../../../../scripts/release/smoke/smoke-npm-package.sh), [Rust SDK smoke](../../../../../scripts/release/smoke/smoke-crates-sdk.sh), [CLI smoke](../../../../../scripts/release/smoke/smoke-crates-cli.sh) and accepted separate-process/missing-record controls. Only later registry installs of actual published .29 version count. |

## TDD and qualification distinction

Behavioral defects discovered during qualification get their own independently
specified regression observed failing on baseline, staged/committed before repair,
then separate GREEN and scoped verification; preserve raw assertion diagnostics.
No new product test is required when existing evidence and docs close the scope.
New passing coverage/manual acceptance is never called RED;20's limited seq-369
exception grants160 no exception. Property tests apply if an
actual codec/projection/recovery/round-trip behavior changes. Review authored
oracles and fixture pins before adapting an intentionally changed contract.

No native/full/long/GPU campaign runs during Phase A. Candidate structural/link/
join checks are document validation only. After parent import use the existing
Markdown/contract validators; do not represent a scratch check as full repository
or product qualification. New AC-measurement tools belong in the repo with a
rerunnable command/env description and experiment README index; no such tool is
introduced by these documents.
