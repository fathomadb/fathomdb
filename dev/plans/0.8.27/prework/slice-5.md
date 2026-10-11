---
title: FathomDB 0.8.27 prework Slice 5 - verification adequacy
status: COMPLETE
target_release: 0.8.27
observed_on: 2026-09-21
---

# Slice 5 - verification adequacy

## Plan and delta reconciliation

Outcome: trace the accepted draft requirements to real existing tests, identify
the smallest missing oracles, and allocate them without writing tests. Claims
about test behavior were checked against the complete relevant suites.

## Trace and current coverage

| Requirement | Existing deep evidence | Gap and allocation |
| --- | --- | --- |
| R27-01/R27-02 erasure | `slice20_dependency_lifecycle.rs`, `slice30_dependency_closure.rs`, `erasure_completeness.rs`, `erasure_drain_ordering.rs` cover ordinary erasure, supersession, physical purge, pre-commit rollback, post-commit `ErasureIncomplete`, reopen recovery, fencing, durable retry/proof, properties, WAL/telemetry, and ordering. | No native test composes correction/supersession -> erase. Slice 20 owns one table-driven RED matrix, both blocker phases, retry/reopen, exact survivors, and retained-proof allowlist. |
| R27-03 public correction path | Memex exact characterization reproduces `storage_failed`; Python `test_erase_source.py` and TS `erase-source.test.ts` cover basic success/idempotency/validation. | Slice 20 flips the unchanged Memex setup and adds one corrected-success plus one incomplete-mapping smoke per binding. Do not copy the engine matrix. |
| R27-04/R27-05 surface stability | Governed operation parity covers 69 signed tokens/44 operations; release-surface tests prevent several test-hook leaks. | Full Rust export/re-export, registration, signature, declaration, and package-root comparison is net-new. Slice 30 extends existing parity/release introspection and proves its own sensitivity. |
| Structural equivalence | Many focused subsystem integration suites already exist. | Slices 40-90 map each moved batch to the owning suite and affected feature route; Slices 100-130 own artifact/declaration/stub/shared-wire checks; Slice 150 owns fresh installed artifacts and platform loading. |

## Acceptance and anti-vacuity rules

- `ExciseReport` exact counts are derived from the requested bucket's
  pre-erasure canonical/projection inventory. Dependent physical deletions do
  not silently change that public count contract.
- Unique sentinels are used in content-bearing stores. Content-free metadata
  uses keyed identity and exact-count oracles.
- At-rest absence means no erased payload, logical/artifact/source-revision
  identity, or live authority remains in the enumerated canonical,
  provenance, projection, vector, telemetry, or WAL planes. The raw non-PII
  `source_id` is permitted only as one new committed-deletion
  `excise_source_audit` row (`record_key` and `payload_json.source_id`) and,
  when affected physical dependents exist, one `source_bucket` closure
  `root_value` (otherwise zero). Tests assert exact rows and fields, exact
  report/proof content, no audit growth on scrub-only retry, and reject every
  other raw identity occurrence.
- The TypeScript binding comment that implies reopen is not evidence; the
  process must actually close and reopen independently.
- Surface comparison consumes real compiler/module/declaration/package
  introspection and rejects synthetic extra, missing, registration, re-export,
  declaration, and package-export defects.
- Existing behavioral assertions remain stable through structural moves.
  Import/path-only edits are isolated; new semantic logic requires genuine RED.

## Implementation, review, verification, and status

This slice writes only the coverage trace and allocations. TDD and code review
are not applicable. A read-only verifier reviewed the full owning tests and
current parity tooling. Independent package design review and closeout
verification passed.

Status is `COMPLETE`. No test, fixture, gate, or product file was changed.
Next: Slice 6 documentation evidence.
