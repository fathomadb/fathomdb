---
title: 0.8.25 Slice 70 — temporal and associative retrieval qualification design
status: REALLOCATED_EXPERIMENTAL
design_version: 1
target_release: 0.8.25
depends_on: 65
readiness_gate: 0.8.25 Slice 7 completion
---

# Slice 70 — temporal and associative retrieval qualification design

> **Reallocated:** This reviewed design is preserved as experimental evidence,
> not 0.8.25 implementation authority. The temporal profile is reconsidered
> for 0.8.28; associative PPR/graph diffusion is reviewed when planning
> 0.8.31. See the
> [scope adjustment](../../scope-adjustment-2026-09-02.md).

## Authority and scope

This design owns R25/AC25-70 and the temporal/associative half of Memex need 16.
It consumes the Slice 65 registry, qualification states, EARP campaign shape,
and default-promotion guard without redefining them. It evaluates two product
profile families: explicit time-scoped retrieval and associative graph
diffusion. Callers choose the profile and supply semantic intent; FathomDB
executes a deterministic bounded mechanism.

The design is REVIEWED_BLOCKED_ON_SLICE_7 and cannot become READY before Slice 7 activates
architecture v2 or before Slice 65's shared profile contract is READY.

## Requirements-to-design comparison

| Obligation | Existing evidence | Required decision |
| --- | --- | --- |
| Time-scoped retrieval | `ReadView(valid_as_of)` passed synthetic half-open-window probes | Package explicit time scope with a named profile and prove external quality only when a source-derived manifest exists. |
| Changed facts | Validity/supersession mechanisms exist; semantic consolidation failed EXTRACT-01 | Retrieve versioned evidence; never infer contradiction or truth. |
| Associative retrieval | Existing bounded BFS and experimental PPR-fusion design exist | Use Slice 60 constrained paths and bounded deterministic diffusion; do not revive exact-anchor GRAPH-01. |
| Quality without default regression | Temporal external gold is blocked; graph/multi-query treatments were rejected | Require manifest-qualified held-out gates and preserve rejected outcomes. |
| Common safety | Slice 65 defines profile lifecycle | Reuse the same registry, receipts, bounds, wire behavior, and promotion decision. |

## Predecessor disposition

| Design or evidence | Disposition |
| --- | --- |
| Slice 65 design | **Normative dependency.** Owns registry, profile reference, qualification, receipts, and promotion. |
| TEMPORAL-01 plan/result | **Preserve.** Synthetic validity is accepted narrowly; external quality remains blocked without a source-derived validity manifest. |
| `dev/design/0.8.2-m1-multihop-harness.md` | **Experimental successor input.** Reuse PPR/RRF hypotheses and strong comparator discipline, not its release-specific answer harness as product design. |
| `ADR-0.8.0-graph-traversal-scope.md` and Slice 60 | **Reuse.** Supply bounded constrained graph access and exact path evidence. |
| GRAPH-01 and REASON-01 | **Rejected evidence.** Exact-anchor and protected multi-query treatments cannot be renamed or promoted. |
| Planner/router PSD recency ideas | **Experimental only.** No automatic intent detection, lossy rewrite, or model-selected route enters FathomDB. |

## Public profile inputs

Both families use `RetrievalProfileRefV1` and `ProfileExecutionV1` from Slice
65. Their production entries are immutable, installed, and opt-in.

`TemporalScopeV1` is a closed union:

- `valid_at { instant_unix_ms }`; or
- `valid_during { start_unix_ms, end_exclusive_unix_ms }`.

The caller must provide the scope; FathomDB does not extract dates from query
text. The interval must be nonempty. It is intersected with the Slice 35 frozen
snapshot/eligibility envelope and can only narrow visibility.

`AssociativeSeedV1` is exactly one of query-derived Slice 60 seeds or explicit
logical IDs. A profile binds depth, direction, edge/target constraints,
restart mass, iteration ceiling, integer convergence threshold, graph-node cap,
candidate cap, fusion constant, and output limit. Production callers cannot
override them ad hoc.

All types use schema version 1 and Slice 65/A25-05 unknown-field rules. The
SDKs and wire format expose no `auto`, `latest`, natural-language temporal
expression, arbitrary edge-weight callback, or model/provider field.

## Temporal treatment

The first eligible treatment name is `temporal_scoped_v1`:

1. Validate the caller scope, snapshot, declared temporal projections, and
   profile digest.
2. Apply source, lifecycle, hard eligibility, and valid-time constraints before
   lexical/vector/graph candidate truncation.
3. For `valid_at`, return only revisions valid at the instant. For
   `valid_during`, return revisions whose half-open validity windows overlap the
   requested half-open interval.
4. Preserve separate revisions and supersession evidence. Do not collapse
   semantically competing logical records, choose a true fact, or rewrite old
   content. Exact-duplicate revisions may be suppressed only through a Slice 65
   qualified stage.
5. Rank with the profile's accepted retrieval stages and emit exact revision,
   validity, supersession, source, and profile explanation through opt-in
   evidence references.

Missing validity metadata is not interpreted as current. The profile entry must
choose one declared policy—`exclude_unbounded` or `include_unbounded`—and the
qualification corpus must contain explicit gold for that choice. The initial
`temporal_scoped_v1` binds `exclude_unbounded`, the fail-safe option.

## Associative diffusion treatment

The first experimental treatment is `associative_ppr_v1`:

1. Resolve eligible seeds and the complete constrained depth-3 subgraph through
   Slice 60 under one frozen snapshot. Canonical BFS frontier order is
   `(depth, seed_ordinal, node_revision_id)`; oriented edges are ordered by
   `(from_revision_id, to_revision_id, edge_kind, edge_revision_id)`. The
   ceiling is 500 unique nodes, 2,000 eligible oriented edges, 10,000 path
   states, and 1,000 initial text/dense candidates. Detecting the first item
   beyond any ceiling returns `AssociativeGraphBoundExceeded` before diffusion,
   with no ranking or partial-subgraph digest.
2. Represent total probability mass as the integer `M = 10^15`. A ranked seed
   at one-based rank `r` has integer weight `floor(10^12/r)`; explicit unranked
   seeds have weight `10^12`. Convert these weights into a seed vector summing
   exactly to `M` with the apportionment function below.
3. `apportion(total, [(weight, canonical_key)])` is normative. Using checked
   `u128` intermediates, assign each recipient
   `floor(total*weight/sum_weights)`. Distribute the remaining units to
   recipients by descending exact remainder `total*weight mod sum_weights`,
   then ascending canonical key. Zero total returns all zeros; zero aggregate
   weight is invalid. The returned allocation sums exactly to `total`.
4. Run serial fixed-point Personalized PageRank for at most 50 full iterations.
   At the start of every iteration, set `R = 15*M/100` and `F = M-R`.
   `apportion(R, seed_vector)` allocates restart units across seeds.
   `apportion(F, current_mass_by_node)` allocates the complete follow budget
   across all nodes, resolving every `/100` remainder globally rather than
   dropping it independently per node. For each non-dangling node, apportion
   its entire follow allocation equally across its canonically ordered outgoing
   edge revisions. Sum the complete follow allocations of all dangling nodes
   into `D`, then `apportion(D, seed_vector)` across seeds. The next vector is
   exactly restart allocations plus incoming edge transfers plus dangling
   allocations, so its sum is exactly `R + F = M` before convergence testing.
5. Outgoing direction uses `from->to`; incoming reverses it; both is their
   ordered union, with a self-loop edge included once and parallel immutable
   edge revisions retained separately. Nodes and arcs accumulate strictly in
   canonical order. Convergence is evaluated only after the complete conserved
   iteration as integer L1 distance `<= 10,000,000`, equivalent to `1e-8*M`.
6. Rank by the final integer mass quantized into `mass/1000` (`1e-12` buckets),
   descending, then immutable revision ID. This defines the cross-platform
   near-tie policy and ordered-result digest.
   Fuse the top 100 graph candidates with A0 candidate ranks using RRF `k=60`,
   then apply only Slice 65-qualified post-selection stages.
7. Return at most 100 compact hits with optional exact **witness paths** and
   profile explanation. A witness is the canonically first shortest visible
   seed-to-hit path and is not represented as complete PPR provenance. Optional
   `PprContributionV1` reports the top eight final-iteration incoming transfers
   ordered by `(mass desc, predecessor_revision_id, edge_revision_id)`, plus
   total omitted incoming mass, restart mass, dangling redistribution mass,
   iteration count, convergence state, and complete-subgraph digest. Slice 55
   degraded/material-omission signals remain mandatory. No generated summary,
   evidence joining, or answer reasoning occurs in the engine.

Ceiling exhaustion fails closed before scoring. A future deterministic
truncated-graph treatment requires a new profile revision and preregistered
quality/no-regression gate; `associative_ppr_v1` cannot qualify or execute it.
Snapshot/projection failure also fails closed. There is no silent BFS-only
fallback.

## Qualification and durable rejection

Each family uses a separate Slice 65 EARP comparison against A0 on an untouched
held-out split. A temporal external-quality campaign cannot execute until a
reviewed, source-derived manifest declares record identity, validity windows,
questions, gold evidence, exclusions, and digests. Until then, only synthetic
validity/lifecycle diagnostics are eligible and no temporal product-quality
claim or registry entry may be created.

An associative campaign reports supporting-evidence recall and rank metrics,
multi-hop answer EM/F1 when an identical answerer is explicitly configured,
groundedness, attribution, duplicates, path support, latency, resources, and
all failures. The preregistration supplies exact thresholds and paired
uncertainty; acceptance requires every registered primary and no-regression
predicate. A retrieval gain cannot offset failed answer usability when the
profile changes answer context.

A treatment passing only its retrieval gate may install as
`qualified_retrieval_only`. Because both treatments can change ordered hit or
evidence context, they may install as `qualified_answer` only after the
registered correctness, groundedness, and attribution no-regression predicates
also pass. `intended_use=answer_context` rejects a retrieval-only temporal or
associative profile. In 0.8.25 there is no default-promotion path: omitted
profile remains immutable compiled A0, and the registry rejects
`qualified_opt_in`, default entries, A0 aliases, or default shadowing.

Underpowered, manifest-invalid, bound-dominated, lifecycle-invalid, or
regressive outcomes are written as `rejected` EARP receipts with the immutable
profile/config digest. Their names and revisions enter a rejection ledger
consumed by the production-registry builder, which refuses to install or alias
them. A rejected treatment may be reconsidered only under a new name/revision,
new preregistration, and untouched held-out evidence; its predecessor remains
rejected.

## Persistence, lifecycle, and compatibility

No semantic-memory or graph-schema migration is introduced. Registry entries
are installed assets; database state remains canonical revisions, dependencies,
validity, graph data, and projections owned by earlier slices. Profile requests
and execution receipts record snapshot/generation/profile digests but do not
become canonical memory. Rejection receipts and the derived rejection ledger
live in the evaluation/installed-registry build inputs, not user databases.

Every source, revision, edge, and path obeys supersession, erasure, dependency
liveness, snapshot visibility, and eligibility. Erasure between pages produces
the original snapshot result or a typed snapshot/cursor failure, never mixed
evidence. Existing A0 and graph calls remain unchanged. Rust/Python/TypeScript/
wire and applicable Windows CPU/native parity are mandatory. CUDA equivalence
is required when accepted candidate arms use dense/rerank execution; PPR itself
is deterministic CPU work.

Typed outcomes extend the Slice 65 set with invalid temporal scope, temporal
metadata unavailable, external manifest unavailable,
`AssociativeGraphBoundExceeded`, witness path unavailable, and diffusion
nonconvergence. Bound exhaustion never scores. Nonconvergence after iteration
50 returns no ranked result in v1; neither outcome may be qualified as degraded
success under this profile revision.

## Mapped RED/GREEN and verification

| Acceptance boundary | Required proof |
| --- | --- |
| Temporal boundaries | Lower/interior/upper/overlap fixtures, missing-validity policy, supersession, erasure, and concurrent mutation under one snapshot. |
| No time leakage | An adversarial high-rank out-of-window record never displaces the eligible record before truncation. |
| PPR correctness | Hand-computable integer-mass graphs prove seed-weight apportionment, exact `R+F=M`, uneven out-degree, multiple dangling nodes, non-dividing seed weights, canonical remainder domains, exact per-iteration mass conservation, full-iteration L1 convergence, `1e-12` near-tie buckets, and RRF k=60. |
| Bounds/failures | Each depth/node/edge/path-state cap is exceeded independently under insertion permutations and reopen; every cell fails before diffusion with no partial digest. Missing witnesses, nonconvergence, stale cursor, and projection degradation are typed. |
| Determinism | Shuffled insertion, reopen, required CPU platforms, and applicable CUDA candidate arms preserve graph, mass-vector, contribution, and ordered-result digests. |
| Explanation | Witness paths are labelled non-causal; top-eight contributions and omitted/restart/dangling mass reconcile exactly to the final-iteration mass. |
| Qualification/rejection | Invalid manifest, reused consumed set, recall-only gain with grounding loss, LOCOMO regression, underpowered cells, rejected alias, and same-revision retry reject installation. Dependent-contract fixtures reject `qualified_opt_in`, default/A0 aliases, and answer use of retrieval-only temporal/associative profiles. |
| Default compatibility | Omitted profile remains A0; rejected profiles and renamed historical treatments cannot resolve. |

Run fast mechanism fixtures, heavy temporal/multi-hop held-out campaigns only
when eligible, all/all-feature, Windows CPU/native, CUDA candidate-equivalence,
registry-installed profile/rejection smokes, and any preregistered live-model
usability route. Operator routes are not applicable.

An independent review may require at most five FIX-n cycles. Unresolved P1/P2
findings, inferred time intent, semantic truth selection, a second profile
registry, use of unavailable external gold, or silent fallback blocks READY.
