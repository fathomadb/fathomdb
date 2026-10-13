---
title: Cross-SDK benchmark runner implementation plan
status: ACTIVE
observed_on: 2026-10-13
---

# Cross-SDK benchmark runner implementation plan

**Status:** In progress, 2026-10-13. This implements the
[requirements](../performance-benchmarking/competitive-testing-requirements.md)
and [cross-SDK design](../design/sdk-performance-driver.md). The first live
fixture is synthetic, local, and full-text only. This plan does not authorize
external corpora, models, paid services, or comparative claims.

## Sequence and proof

1. **RED: configuration and evidence contracts.** Add failing tests for closed
   `sdk.experiment.v1`, operation inventory/versioning, digest drift, safe
   receipt projection, reruns in one minute, complete cell roster, and answer
   dependency binding. Preserve the existing product runner schemas.
2. **GREEN: shared coordinator.** Implement deterministic matrix expansion,
   fixed driver commands, artifact preflight, fresh database setup, process
   isolation/timeout, repetition aggregation, `sdk.result.v1`, path-free
   receipt/witness, and validated suite manifest. Re-run the RED set.
3. **RED/GREEN: public SDK bindings.** For each language, first fail an explicit
   FTS fixture and a preflight or conversion fixture, then implement the
   versioned driver protocol through public Python, TypeScript, and
   `fathomdb_sdk` Rust methods. Add read/graph and remaining registered
   operations as their typed fixtures and oracles are defined. Verify the
   Python deadline boundary, TypeScript loaded binary, Rust patch closure,
   fixed temporal eligibility, and cold/mutation isolation.
4. **RED/GREEN: scoreboards.** Add failing deterministic latency, retrieval,
   and answer fixtures; implement their separate family payloads, shared
   answer stage, comparable-cell validator, and renderer. Keep product and SDK
   axes separate and historical path-bearing receipts visibly legacy.
5. **Review and qualification.** Run targeted tests from RED to GREEN, then
   lint, typecheck, tests, and the full `agent-verify` gate. Obtain independent
   `gpt-6.1-sol` high-effort adversarial code reviews, remediate every
   material finding, repeat FIX-n review, and verify the final diff.

## Completion evidence

- `BT-AC01`–`BT-AC10` and each Python, TypeScript, and Rust driver AC have
  executable evidence, or the relevant case is explicitly blocked because a
  required external artifact is absent.
- The same experiment file runs twice into distinct external roots and yields
  one safe receipt/index row per expected cell with identical treatment hash.
- A real synthetic database produces the same expected ordered FTS IDs through
  all three public SDKs. No general-search fallback is accepted.
- Results distinguish measured scores from incomplete, blocked, descriptive,
  and unpaired rows. No real competitor comparison is inferred from the smoke.

## Current implementation boundary

- The synthetic local suite runs explicit FTS through all three public SDKs,
  and each driver also exercises typed read and bounded graph calls against a
  real database. The same immutable suite can be rerun with distinct receipt
  IDs. The renderer separates SDK and product axes across latency, retrieval,
  and answer families.
- Latency requires an exact operation oracle. Retrieval relevance gold is
  separate, and unverified retrieval rows omit timing. Product and SDK receipts
  retain safe hash links; invalid evidence stays visible. The local answerer
  is deterministic and binds the same attempt's retrieval evidence.
- Source/build provenance remains explicitly unverified for the local smoke.
  Real paired claims require SDK build attestations and a bootstrap equivalence
  check. The current runner measures three operations; mutation/model isolation,
  resource attribution, and broader typed operation fixtures remain active
  acceptance work under BT-AC04, BT-AC06, and BT-AC07.

## Existing contracts and boundaries

- Use `experiments.fathomdb_test_setup.prepare_test_database` for every
  FathomDB measurement cell and retain its doctor/device evidence externally.
- Keep `experiments.record.v1` and `experiments.index-row.v1` unchanged.
- The [SAFETY-01 charter](../performance-benchmarking/tracks/safety-01-campaign-controls.md)
  governs receipt integrity. A new Program measurement track must be
  registered before non-synthetic data generation; this implementation uses
  synthetic local fixtures only.
