---
title: Built-in provider — user needs, requirements and acceptance criteria
status: proposed; owner contract review required before implementation
target_release: 0.8.28
observed_on: 2026-11-01
---

# Built-in provider requirements

Scope authority: steward `seq-355`. The detailed contract in
[built-in-provider-design.md](built-in-provider-design.md) is proposed, not
implemented or approved. These are feature-local identifiers, not additions to
locked `dev/acceptance.md`. Slice 50 Unit S keeps its existing requirements.
The original Unit D requirements remain historical unselected options; this
proposal replaces their attached-but-refused premise if approved.

## User needs

| Need | User outcome |
| --- | --- |
| N1 Correct vector results | Reject incompatible supported admission under the specified check; report its finite-test limits and accepted first-enrollment residual. |
| N2 Predictable supported configuration | Use the shipped pinned BGE-small/Mean implementation, or explicitly select no provider. No user embedder/plugin selection before 1.x. |
| N3 Responsive repeated use | SDK calls reuse their open engine; MCP idle close and A/B/A switching can release resources without paying repeated full probe inference for unchanged verified state. |
| N4 Useful failure and recovery | A required failed check gives a classified error before returning a handle. Valid text/canonical access and deliberate diagnosis remain possible. |
| N5 Honest durability | Failed checks and rolled-back changes never publish acceptance, replace references or conceal pending projection work. |
| N6 Bounded resource use | Verification reuse retains bounded numerical evidence, without keeping models, workers, database ownership or handles alive. |
| N7 Managed future extension | Preserve only specifically contained internal/test/evaluation seams; future user providers require a separate contract and qualification at 1.x or later. |

## Requirements

| ID | Contract | Needs |
| --- | --- | --- |
| BIP-R1 | Production engine admission exposes only the pinned built-in provider and explicit no-provider mode. Remove caller-provider admission and close every alternate path listed in the design inventory. CPU/CUDA device selection within the built-in implementation remains governed by existing device policy. | N2, N7 |
| BIP-R2 | Keep strict identity/dimension, structural integrity and numeric compatibility checks. Preserve 45 probes, zero P1 binary flips and P2 maximum uncentered L2 `1e-5`. A relevant open means provider-enabled with a registered/durable dense arm or existing probe baseline; it uses a completed check for its exact store/runtime state or performs fresh inference. A persisted success marker alone cannot authorize acceptance. | N1, N5 |
| BIP-R3 | Construct the built-in runtime from the exact verified asset bytes it consumes, with fixed Mean pooling, tokenizer/truncation/preprocessing, unit normalization, F32 policy and effective device identity. All numerical settings that can change BGE execution are immutable for its lifetime. Claimed identity, constructor settings and object addresses are insufficient evidence. | N1, N2 |
| BIP-R4 | Bind reuse to committed store profile, references, mean/centering state, fixture, thresholds and recipe plus the actual runtime descriptor. Missing evidence means a miss. A changed mean misses the old verdict; an admitted engine may validate its proposed mean using retained numerical evidence without fresh inference. Preserve explicit first-enrollment semantics and asynchronous canonical writes. | N1, N3, N5 |
| BIP-R5 | Every supported mean/reference/enrollment transition has a defined invalidation and publication rule. No inference under a cache lock or SQLite write transaction. A rejected candidate mean is not installed. Explicit recompute returns a typed rollback error; automatic pin/recovery retains the previously accepted mean, continues valid projection work and exposes one bounded rejection advisory while suppressing automatic retries under that accepted-state token. Baseline-bearing ordinary no-provider open defers automatic mean recovery; explicit no-provider maintenance creates no verification claim. The explicitly unbaselined first-enrollment residual is not a failed check. | N1, N5 |
| BIP-R6 | Bound the process evidence cache to 16 entries and 2 MiB accounted payload, with 69,120 bytes of evidence plus fixed metadata per active baselined engine; retain no resource-owning objects. Eviction/restart is a safe miss. No background refresh, provider retention or MCP idle/capacity change is required. | N3, N6 |
| BIP-R7 | Concurrent checks may duplicate inference. Lookup/publication holds only a short cache lock. Publish only after successful comparison and matching durable commit; reject obsolete generation, cancelled, timed-out, panicked, malformed and late results. A fresh failed diagnostic cannot leave an applicable reusable positive verdict. Runtime-invalidating device/context loss also revokes affected live runtime admission; context validity is checked at cache/admission, execution and state-renewal boundaries. Cache clearing alone is insufficient. | N1, N5, N6 |
| BIP-R8 | Required mismatch or inability to verify returns a typed open failure and no handle. Preserve database release, dispatch ownership, bounded provider-only cleanup, original error reporting and subscriber notification. Success does not guarantee projection catch-up or absence of later provider errors. | N1, N4 |
| BIP-R9 | Explicit no-provider admission validates stored profiles as data, including well-formed unsupported historic identities; it does not fabricate a BGE provider or bypass corruption. Preserve valid canonical/text/erasure operations and visible deferred dense work. Dense rebuild requires supported compatible provider admission before invalidation. | N4, N5 |
| BIP-R10 | Keep live numeric diagnosis independent of successful engine admission and non-mutating to store references/data. Distinguish numeric mismatch, provider failure, invalid state, unsupported stored provider and legitimate absent baseline. Intentional function changes require reconstruction or a separately designed complete rotation workflow. | N1, N4 |
| BIP-R11 | Every retained extension seam has a named purpose, feature/visibility containment, an annotation naming this contract and a test proving it cannot attach an alternate provider to a production engine. Update affected public interfaces and successor ADRs together; add no compatibility shims. | N2, N7 |
| BIP-R12 | Commit rerunnable tools and raw results for complete open/model reconstruction, fresh/memoized verification, mean validation, A/B/A reopen and memory bounds. Compare against the existing measurements without mislabeling them as cold startup or a fleet failure rate. | N3, N6 |

## Proposed acceptance criteria

| ID | Requirement | Observable evidence required after approval |
| --- | --- | --- |
| BIP-A1 | R1, R11 | Production admission APIs accept only built-in/none regardless of enabled features; fake, ONNX, Nomic, modified-pooling and forged device-report providers cannot attach through them. Distinct test/evaluation openers may inject providers but cannot mint production reuse evidence. Package feature/re-export checks keep those openers out of shipped artifacts. |
| BIP-A2 | R1, R9, R11 | All bindings reject unsupported vector provider selectors before durable changes; default declarations round-trip. Well-formed old unsupported metadata can be inspected without a provider; it never causes substitute BGE dense execution. |
| BIP-A3 | R3 | Construction tests bind digests to consumed bytes; mutation/replacement between loader validation and construction is rejected or irrelevant to the owned snapshot. Changes in each runtime descriptor component cannot inherit an acceptance entry. |
| BIP-A4 | R2, R3 | Real built-in CPU/CUDA tests preserve limits. Concurrent global precision mutation cannot alter strict built-in execution; cover every BGE matmul path. Any other enabled backend needs equivalent stability qualification before reuse. |
| BIP-A5 | R2, R4, R6 | First relevant process open runs probes despite a valid old on-disk marker; unchanged close/reconstruction, idle reopen and A/B/A reuse valid evidence with no new probe inference. Restart, capacity eviction, incompatible runtime or changed store input causes a verdict miss; proposed-mean renewal uses the separate evidence rule, never an old-mean pass. |
| BIP-A6 | R4, R5 | Cover automatic mean pin, explicit recompute, open recovery, projection enrollment, reference/fixture/threshold changes and rollback. Proposed means are checked against current evidence as designed; no stale or pre-commit publication, no dispatcher call in the write transaction, and canonical writes stay asynchronous. Rejected automatic candidates remain uninstalled through close/reopen recovery, while projections, drain and non-embedding mutations progress under the old accepted state; advisory/retry suppression is bounded and observable. |
| BIP-A7 | R5, R7 | Deterministic concurrency tests prove failed diagnostics revoke reuse, generation changes defeat stale publication, duplicate checks are safe, and cancellation/panic/timeout/malformed/late results cannot publish a pass. Context loss revokes affected live evidence and fences results; recoverable allocation errors do not spuriously invalidate it. Property tests cover state/commit/eviction interleavings. |
| BIP-A8 | R6 | Fill beyond both cache bounds; assert strict limits and safe misses. After close, provider/model/dispatcher/database/lock owners release even while evidence remains cached; healthy active-engine evidence remains valid if its LRU entry is evicted; runtime invalidation revokes its admission separately. |
| BIP-A9 | R8 | Separate RED/GREEN cases for mismatch, error, panic, timeout and malformed output: no returned handle, typed cause/subscriber parity, database ownership reacquirable, original error retained through bounded cleanup and no late mutation. |
| BIP-A10 | R9, R10 | Real database tests cover no-provider FTS/canonical/lifecycle/configuration/erasure and corruption refusal; dense rebuild rejects before invalidation without a provider, pure-text rebuild succeeds, explicit built-in CLI rebuild completes truthfully, and live diagnostic preserves database state/WAL correctness. |
| BIP-A11 | R2, R4, R8 | Truly vector-less (no dense arm/declaration or baseline) and no-provider opens do no probes; a retained baseline remains relevant even after all kinds are removed; first-enrollment residual is explicit and tested. Repeated ordinary calls do not rerun 45 probes. Later normal inference failures retain normal typed operation errors rather than changing provider mode. |
| BIP-A12 | R11, R12 | Inventory dispositions have matching code/doc annotations and closure evidence. Behavior-specific RED evidence, focused suites, meaningful measurements, independent code review and final strict `agent-verify.sh` all recorded. No scoped pass is called full-workspace green. |

All criteria are proposed checks, not current pass claims. Existing
[characterization](../../../../experiments/slice50-refusal-baseline/analysis.md)
and [timing data](../../../../experiments/fresh-open-verification/findings.md)
remain the baseline. Detailed execution order and owner gate are in
[built-in-provider-plan.md](built-in-provider-plan.md).
