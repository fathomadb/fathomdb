---
title: 0.8.30 Slice 40 performance and end-to-end latency
status: ACTIVE
slice: 40
target_release: 0.8.30
---

# Slice 40 — Performance and end-to-end latency

Commissioned at seq-405 to complete the whole slice using the supplied execution
template.

## Entry and ownership

Reconcile [master § C](../../plan.md#c-slice-planning-and-execution-contract)
against the commissioned release checkout. Record the diff from draft anchor
`f7fb1da9a`, related work in flight, and the actual source, fixture, feature,
provider and native inputs. Inventory assigned Slice 3/4/5 contract,
architecture and test drafts and selected Slice 9/10 preparation. Give each
applicable item an adopt, revise, decline or defer disposition and owner.

Slice 10 owns approved baseline preparation. Slice 30 owns correctness fixes
and dispositions that affect selected routes or fixtures, including the .29
natural-write rank observation. Slice 60 owns final consumed fork and native
inputs. CPU baseline and research can proceed after 10 without waiting for the
whole of 30 or 60. Before promotion, reconcile relevant 30 fixes and repeat
affected proof on final consumed inputs. Slice 50 owns lifecycle and resource
defects found outside the selected latency treatment.

Also reconcile the inherited hints-overhead qualification assigned to40 in
[Slice10 status](../slice-10/status.md). Keep the unchanged200 µs criterion and
the original failed observations. Select a bounded diagnosis in the reviewed
measurement plan; this obligation does not replace the100k baseline or grant
an unrelated second optimization.

## Goals, local requirements and acceptance

Users need faster useful search responses under ordinary contention with correct
results and observable failure outcomes. Profile one current 100k hybrid Rust
SDK route and one direct-text SDK control with a close/reopen check. Limit any
treatment to one demonstrated bottleneck and its affected schedules. Keep
format 37, existing numerical gates and accepted retrieval behavior. Leave
ANN, chunking, schema, precision, backend, defaults and new cache adoption
outside this commission. See the [findings](../../testing-robustness-findings.md)
and [recommendations](../../quality-performance-robustness-recommendations.md).

| Local requirement | Observable exit |
| --- | --- |
| R30-LAT baseline and attribution | Both routes complete on current identified inputs. Report caller p50/p95/p99, useful completions per elapsed second, attempts, refusals, timeouts, errors and resource observations. Correlate observed queue, service, eligibility, retrieval, hydration, rerank and serialization boundaries with their actual request or mark the phase inapplicable or unobserved with cause. |
| R30-LAT treatment comparison, if selected | Interleaved pairs use matched starting state, requests, load, configuration and build inputs. Show the expected work reduction and a useful end-to-end benefit within prospectively selected tail, useful-throughput, error and resource bounds. Retain every attempt and uncertainty. |
| R30-LAT correctness and selection | Same-snapshot results satisfy independent expected behavior and reference equality. Natural committed writes preserve expected visibility and progress. Report adoption, no justified treatment, or measured neutral/negative/inconclusive treatment. Missing required baseline evidence stays open. |

## Architecture, research and draft design

The [SDK](../../../../../src/rust/crates/fathomdb-sdk/src/engine.rs) delegates
hybrid to the engine query/embedding dispatch and reader path.
[Engine API](../../../../../src/rust/crates/fathomdb-engine/src/search_api.rs)
direct text dispatches without query embedding. In
[`read_search_in_tx`](../../../../../src/rust/crates/fathomdb-engine/src/search.rs),
dependency presence is checked in the reader snapshot and
`all_nodes_directly_eligible` is guarded. Start with the .29 48.336–49.223 ms
scan lead, then check its current frequency and cost before selecting work.
Use the [.29 text](../../../0.8.29/features/slice-30/results-p.md),
[eligibility](../../../0.8.29/features/slice-40/measurement-plan.md) and
[winner-hydration](../../../0.8.29/features/slice-50/design.md) evidence to choose
probes. AC-083 remains tracked for its direct-text workload.

- Can the guarded eligibility question do less work on the same snapshot?
  Inspect actual plans and runtime counters, including all-eligible and
  early-ineligible shapes. Judge any simplification against accepted
  [pre-cap eligibility](../../../../adr/ADR-0.8.25-eligibility-and-frozen-reads.md)
  and [visibility publication](../../../../adr/ADR-0.8.28-transactional-visibility-coalescing.md).
  [SQLite query plans](https://www.sqlite.org/eqp.html),
  [statement counters](https://www.sqlite.org/c3ref/stmt_status.html).
- Can the comparison distinguish the treatment from shared-host variation?
  Block paired runs by nearby time and fixture, balance arm order, and keep
  ordinary CPU load as context. Use the same measurement profile within each
  comparison. [NIST randomized blocks](https://www.itl.nist.gov/div898/handbook/pri/section3/pri332.htm).
- Where could the two routes share a wrong oracle? Use a fixed-state control
  plus hand-expected eligibility, identity and validity-boundary cases. Apply
  the [quality roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md)
  on this path. Defer broad mutation and fault campaigns.
  [SQLite disabled-optimization tests](https://www.sqlite.org/testing.html#disabled_optimization_tests).

Revise the design from those checks. Prefer eliminating measured work in the
existing route. A cache proposal requires separate authority and a current
matched read-mostly and natural-write comparison covering key, hit, miss,
invalidation and retained-memory costs. Prove snapshot and time-boundary
correctness, including expiry without generation change, rollback and reopen
as applicable.

## Measurement and implementation plan

1. Reuse [`slice30_measure`](../../../../../src/rust/crates/fathomdb-sdk/examples/slice30_measure.rs)
   `G-R`/`G-O` for the SDK route and diagnostic subset, and `G-W` for one
   relevant natural-write schedule. Use the adapter and
   [measurement tooling](../../../../experiments/0.8.29-slice30-measurements/README.md)
   without inheriting historical GPU admission or the full campaign. Confirm
   the 100k fixture and effective CPU provider before timing. Use a bounded
   adapter change only for a demonstrated gap and retain its rerunnable selector.
2. Before treatment, fix the ordered queries, warmups, sample and repeat counts,
   offered schedule, paired statistic and uncertainty method, useful benefit
   and guardrails, and stop and missing-data rules. Use fresh equivalent
   copies of one closed store per matched block, with identical options,
   projection readiness and write schedule. Match artifact bytes where one
   reviewed binary can select both arms. Otherwise record both exact artifact
   identities and match all build inputs except the treatment.
3. Keep `slice30-observer` diagnostics separate from ordinary caller timings.
   Correlate submit/start/finish stages where available. `reader_wait` spans
   the response wait and cannot measure pure queue delay. Report embedding,
   reader service, eligibility, text/vector retrieval, fusion, hydration and
   rerank from actual observed work. Time the selected encoding boundary
   separately from Rust SDK return. Label harness JSONL encoding and flush
   costs separately from product wire work. Report CPU time, peak RSS and
   applicable I/O/WAL and response bytes. Keep first-call and warm populations
   distinct and use actual successful completions for useful throughput.
4. Reuse `cargo test -p fathomdb-engine --features test-hooks --test slice71_search_statement_trace`
   for fixed-state route/work controls and `slice35_eligibility_pretruncation`
   when pre-cap work changes. Compare ordered IDs, cursors, scores, snippets
   and provenance where present, projection authority and typed outcomes.
   Give matching reads fixed snapshot/time authority. Under natural writes,
   check each read against its own observed committed state and cursor bounds.
5. Direct Astra-medium reviews design and necessary contract deltas. Direct
   Astra-medium critically reviews the updated plan with at most one FIX-1
   and re-review. Commit after PASS, then implement when commissioned. Give each
   behavior fix or optimization an actual failing regression with recorded RED,
   then GREEN and refactor. Use deterministic work counts for performance-only
   changes where available. Keep passing characterization as baseline evidence.
   Direct Sol6.2-high reviews code and direct Terra independently verifies the
   affected tests, measurements and artifacts.

## Closure

Retain commands, inputs, original outcomes, matched summaries and one concise
disposition. Apply scoped checks during work and the required full gate once
for a coherent source phase. Hand Slice 70 the exact reusable proof and any
remaining numerical, platform or installed-artifact obligations. Update
affected interfaces, changelog, binding parity and authorized release state once.
Integrate into the owning release branch and clean only owned inactive
worktrees and scratch after retaining evidence. Use direct agents throughout.

## Commissioned entry reconciliation

Reviewed on `8972d4daab8cf1e27756ce8c19f80cb39bb2c91c`. This is the
[execution contract](#measurement-and-implementation-plan) applied to the current
release. [Study design and prospective rules](design.md) define the selected
cells and decision criteria.

| Allocation and current evidence | Disposition |
| --- | --- |
| [Prework 3](../../prework/slice-3.md), [P9-12](../../prework/slice-9.md), [10 allocation](../slice-10/design.md) | Adopt R30-LAT and the one demonstrated treatment limit. Add a distinct hints qualification exit because all four preserved 10 observations exceeded the unchanged gate. |
| [Prework 4](../../prework/slice-4.md) and [architecture v2](../../../../design/fathomdb-data-plane-architecture-v2.md) | Adopt existing SDK/search/reader ownership and opt-in observer. Reconcile the MCP response handle since the draft: Slice 30 binds ordered kept ref/cursor pairs and keeps the public contract. No MCP behavior delta is selected here. |
| [Prework 5](../../prework/slice-5.md), [roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md) and [findings](../../testing-robustness-findings.md) | Adopt same-state disabled-optimization comparison plus hand-expected lifecycle and validity boundaries, useful completions, typed outcomes and affected-route work accounting. Keep AC-072/076 and AC-083 gates. Defer broader injection, fuzzing, lifecycle ownership and installed platform qualification to their existing owners. |
| From `f7fb1da9a` to commission | 10 closed with preparation and inherited hints failure. 20 landed delivery provenance and installed-experiment changes. Lock resolves Tokio 1.54.1 and webpki-roots 1.0.12. 30 landed the response binding and whole-source digest fixes, independent committed coverage and stationary rank controls, and a test-only isolated rank selector. The SDK adapter source and historical tooling have unchanged bytes in this window. |
| [30 status](../slice-30/status.md) | Adopt its independent expected-value controls and current integrated source. Historical moving-write S50 rank cause stays unresolved. Compare stationary outcomes exactly, and judge moving observations against their own committed cursor/visibility. Do not grant a score tolerance from prior work. |
| [Prior G evidence](../../../0.8.29/features/slice-30/continuation-gd.md) and [P evidence](../../../0.8.29/features/slice-30/results-p.md) | Revise to the released closed G dense seed and pinned CPU model on current source. P is useful historical direct-text evidence. Its provider-none zero-vector store cannot qualify the dense route. Decline the historical GPU campaign, budgets, full G-W matrix and archived timings as current results. |
| [60](../slice-60/plan.md), [70](../slice-70/plan.md), [50](../slice-50/plan.md) | Defer consumed-fork/native pin qualification to 60 and exact integrated artifact proof to 70. 50 owns general runtime/resource/lifecycle defects. Inspector and 0.9.0 research stay outside this slice. |

A40-1 requires the two current 100k SDK paths with initial provider/readiness
and demonstrated close/reopen, attributed workload and denominators. A40-2
requires either one prospectively qualified treatment or a reasoned
non-adoption, with the applicability of every guard and omitted phase
accounted for. A40-3 requires stationary exactness and natural committed
progress on the selected routes, scoped source verification, and final pin
reconciliation for promotion. A40-4 requires a prospective hints diagnosis
with current and original evidence and an explicit qualification disposition.
The prior failures remain part of qualification. Any unsatisfied
requirements carry their exact missing evidence and owning follow-up.
