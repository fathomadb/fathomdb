---
title: FathomDB 0.8.26 Slice 15 — graph-evidence impact spike
status: COMPLETE — AWAITING INDEPENDENT REVIEW AND HITL D26-01
execution_authorized: repository owner 2026-09-13
---

# Slice 15 plan — graph-evidence impact spike

## Outcome and boundary

Produce the measured performance, response-shape, security, concurrency, and
erasure-linearization evidence needed for HITL to rule D26-01. This is a
decision-support spike, not the graph-evidence product implementation.

The spike compares one internal treatment with its disabled control:

- option A maps to treatment only when an evidence sidecar is requested; and
- option B maps to the same treatment on every graph expansion.

No provisional request flag, public carrier, resolver route, codec, binding,
schema, or interface contract may remain when Slice 15 closes. Slice 20 owns
the selected public V1 implementation after HITL rules D26-01.

## Reconciliation since the draft

1. Slice 10 closed at reviewed product commit `c2795caf`; it repaired frozen
   explanation finalization but changed no graph traversal, evidence storage,
   erasure, schema, or graph binding shape.
2. D26-01 remains the only open release decision. `seq-283` still requires one
   changed-in-place V1 family and prohibits parallel V1/V2 functional pairs.
3. Graph traversal already holds target and terminal-edge write cursors while
   selecting candidates, then discards the terminal-edge cursor. Artifact
   revisions already have indexed `(artifact_class, write_cursor)` lookup.
4. Existing evidence resolution supplies primary-connection serialization,
   one-transaction materialization, frozen revalidation, nondisclosure, and a
   before-return erasure rendezvous. Point graph evidence still needs a
   distinct intrinsic result because ranked evidence requires contribution and
   projection-origin fields that graph point evidence cannot truthfully fill.
5. Frozen contexts have no elapsed-time expiry. The draft's “expired” case is
   rejected in favor of state drift, out-of-window state, erasure,
   supersession, closure fencing, ineligibility, foreign context, and absence.
6. A raw artifact revision ID is identity, not authority. The provisional
   option-A sidecar must pair stable target/terminal-edge revision identities
   with authenticated opaque references bound to the frozen graph disclosure.
7. Target eligibility and terminal-edge selection are class-sensitive:
   `SearchFilter.kind` selects target nodes, while `edge_kinds` selects edges.
   A resolver must not reinterpret the target kind filter as an edge-kind rule.
8. Ordinary writes can have `migrated_incomplete` provenance. Evidence-bearing
   graph expansion must refuse atomically when any selected target or terminal
   edge lacks complete evidence provenance.
9. Direct SQL-count, mutex-wait, and byte-copy instrumentation does not exist.
   Adding production instrumentation would contaminate the spike. Indexed
   plans, known bounded statement structure, fixture byte counts, end-to-end
   latency, response bytes, isolated RSS, and erasure outcomes are accepted
   proxies.

The earlier proposal to build two public A/B surfaces is rejected as overbuilt
and measurement-distorting. The earlier raw-by-ID resolver is rejected as a
broader enumeration surface. The locked global `dev/acceptance.md` remains
unchanged.

## Release-local need, requirements, and acceptance

**N26-15:** FathomDB needs enough code-grounded evidence to choose the lowest-
risk graph-evidence V1 shape without changing the shipping public surface
before the choice is authorized.

- **R26-15A:** Model A and B through one test-only post-selection hydration
  treatment, preserving a literal pre-spike ordinary-response byte oracle when
  treatment is disabled.
- **R26-15B:** Preflight at most 50 node and 50 terminal-edge evidence records
  through two class-specific bounded indexed statements that fetch and validate
  revision identity plus complete source provenance, independent of the 10,000
  traversal-work bound.
- **R26-15C:** Prototype frozen-only, authenticated graph references and an
  intrinsic primary-connection resolver with class-aware target/edge
  eligibility, complete-provenance refusal, nondisclosure, restart stability,
  and no fabricated ranking facts.
- **R26-15D:** Prove resolver/erasure linearization and preserve the existing
  typed `ErasureIncomplete` outcome for a separately held WAL reader.
- **R26-15E:** Measure response size, graph and point-read latency, concurrency,
  writer interference, isolated RSS, query plans, and erasure outcomes with
  balanced repeatable campaigns and existing ordinary-search sentinels. Model
  A/B carrier size, affected client surfaces, and workload cost across multiple
  evidence-request fractions.
- **R26-15F:** Remove provisional public/runtime product machinery before
  close, retain durable raw evidence and useful characterization/benchmark
  fixtures only, and return a specific recommendation to HITL.

Acceptance criteria:

- **AC26-15A:** Treatment-off graph bytes equal a checked-in literal baseline;
  treatment-on returns distinct target and winning terminal-edge identities,
  bounded after final selection, with indexed joined-preflight query plans,
  validated hash/locator/source facts, and atomic refusal for incomplete
  provenance.
- **AC26-15B:** An authenticated graph reference resolves exact intrinsic node
  and edge evidence after unchanged-state restart; all unavailable/context
  cases collapse to one refusal, and a target `kind` filter does not
  incorrectly reject its authenticated terminal edge.
- **AC26-15C:** A deterministic before-return rendezvous proves erasure cannot
  report success before materialized bytes are released; later resolution is
  unavailable, and held-WAL-reader behavior remains typed and fail-closed.
- **AC26-15D:** The controlled matrix records raw samples and derived
  p50/p95/p99/throughput/response/RSS results. A repeatable writer-throughput
  loss above 10 percent or writer-p99 increase above 25 percent triggers design
  reconsideration. Existing hard-limit, byte, schema, query-plan, or RSS
  failures stop the spike.
- **AC26-15E:** Final Git state exposes no new shipping graph-evidence method,
  field, codec, binding, schema, migration, token format, table, cache, reader-
  pool route, or batch resolver; D26-01 receives a quantified A/B comparison.

## TDD, prototype, and measurement plan

1. Obtain independent review of the reconciled design and resolve at most two
   focused review cycles before implementation.
2. Commit transient RED Rust prototype tests for winning parallel-edge identity, target/edge
   distinction, post-selection bounds, literal ordinary bytes, frozen-only
   evidence, complete-provenance atomicity, restart, nondisclosure,
   class-aware edge eligibility, erasure ordering, and held-reader behavior.
3. GREEN the smallest `test-hooks`-only prototype: retain the winning terminal
   edge cursor internally, run two bounded indexed joined preflight statements
   that fetch complete evidence provenance and canonical source material,
   validate hash/locator facts with source-revision deduplication, mint graph-
   bound authenticated references, and resolve intrinsic evidence on the
   primary connection. Do not add binding or public production routes.
4. Add an ignored release-mode measurement harness and run a balanced matrix:

   | Comparison | Workload | Repetitions |
   | --- | --- | --- |
   | graph control vs hydrated | 1 and 50 results | 1,000 calls per arm |
   | concurrent graph control vs hydrated | 1 and 50 results, eight callers | 8×200 per arm |
   | maximum traversal | 10,000 work units, 50 results | at least 30 per arm |
   | point node/edge, 1 KiB | sequential and eight callers | 1,000; 8×200 |
   | point node/edge, 100 KiB | sequential | 100 per class |
   | Memex-shaped | 50 mixed resolutions | at least 30 batches |
   | writers | alone, with hydrated graph, with point resolution | at least five isolated balanced campaigns |
   | RSS | control and treatment | at least five isolated processes |
   | erasure latency | erase/excise idle baseline and resolver held before return | at least 20 paired observations |

5. Record raw samples, environment/toolchain/CPU, query plans, response bytes,
   fixture copied/hashed bytes, WAL/erasure outcomes, and derived statistics in
   `dev/plans/runs/0.8.26-slice-15/`. Run the identity-bound AC-081a/b
   observation and AC-081c sentinel once; use their established multi-process
   campaign only if the observation enters its warning band.
   Use paired arm order and per-campaign ratios. Report median and interquartile
   range across campaigns plus a deterministic-bootstrap 95 percent interval
   for treatment/control ratios. Use nearest-rank percentiles; treat p99 as
   descriptive where an arm has fewer than 1,000 samples. A performance trigger
   is repeatable only when its median paired ratio crosses the threshold and at
   least four of five writer campaigns agree in direction. Keep fixture setup
   outside every timed erasure arm.
6. Produce transient canonical encodings for A sidecar and B inline carriers,
   measure both response sizes, and count affected maintained Rust/Python/
   TypeScript types, codecs, fixtures, and docs from a checked-in surface
   manifest. Report `A = control + p × incremental treatment` and
   `B = control + incremental treatment` at `p = 0, 0.01, 0.1, 0.5, 1.0`.
   State explicitly that a frozen-only evidence treatment makes B either reject
   existing current-context graph calls or require mixed evidence semantics.
7. Separate transient prototype tests from retained characterization tests.
   Convert only generally useful baseline byte, selection, index, erasure, and
   measurement-fixture oracles into behavior-neutral retained tests. Then
   remove the unchanged prototype-only test files together with the provisional carrier,
   resolver, reference format, and runtime path. Preserve their exact commits
   and measurements as spike evidence; final public-surface and schema
   inventories must match baseline. This planned teardown is not an oracle edit.
   Fast-forward the full RED → GREEN → measurement → teardown chain onto the
   release branch before deleting its temporary branch so every spike commit
   remains reachable.
8. After prototype removal, run focused graph/evidence/erasure tests, retained
   characterization/measurement-fixture tests, `agent-verify`, and full-
   workspace clippy/check. Obtain independent code
   review and independent verification; allow at most two focused FIX cycles.
9. Write `status.md`, return D26-01 to HITL with a recommendation, close Slice
   15 without ruling D26-01, and clean any temporary worktree/branch.

## Stop gates

Stop on a V2 surface/router, raw-ID authorization, current-context evidence,
unindexed or work-unit-scaled hydration, ordinary-response byte drift, partial
evidence success, target-filter misuse for terminal edges, erasure overtaking,
false nondisclosure, schema/persistence change, production-only measurement
instrumentation, or product-scale refactoring.
