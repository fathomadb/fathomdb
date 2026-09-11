---
title: Slice 77 — post-cache residual and treatment design
status: REVIEW_READY
---

# Slice 77 design

## Boundary

Slice 77 is an experimental selection slice. It does not ship a product change.
It restores the exact reviewed Slice 76 statement-reuse prototype in a temporary
worktree, measures its residual, and tests no more than two evidence-selected
treatments. The registered AC-020 fixture and oracle remain unchanged. The
release checkout ends with no product/test prototype diff.

## Phase 0 — direct residual attribution

Restore the four-file experimental tree removed by cleanup commit `8027546d`;
verify it is byte-identical to the reviewed post-census tree at `b432d24d` for
the affected engine inputs. Build an optimized symbolized diagnostic binary
with `slice76-statement-reuse` and `slice76-gperftools-profile` enabled.

Profile exactly two search intervals:

- sequential: the registered 1,600-search sequential arm;
- concurrent: the same 1,600-search sequential warmup followed by the
  registered 1,600-search concurrent arm.

Fixture setup, seed and teardown are outside `ProfilerStart`/`ProfilerStop`.
All eight reader workers must register. Dynamic linkage must include
`libprofiler` and exclude tcmalloc. The timing campaign uses separately sealed,
uninstrumented binaries.

The profile may select a CPU mechanism only with at least 50 concurrent samples
and a named stack covering at least 10% of concurrent samples. Overlapping
cumulative percentages are not summed. The sequential profile must corroborate
the same work unless the difference itself is concurrency-specific. CPU samples
cannot establish sleep or queue/lock wait; those remain unknown.

## Treatment decision

The post-cache profile determines the next step:

- vector JSON conversion/parsing or SQLite quantization above threshold makes V
  eligible; use f32 little-endian BLOB parameters while preserving SQLite-side
  binary quantization and full-float reranking. Direct post-cache CPU evidence
  is sufficient when V removes the named stack; allocation effects remain
  explicitly unknown unless separately counted;
- remaining prepare/materialization work above threshold makes one narrow S
  correction eligible;
- V may lead to Q only when V is correct, materially faster, still misses
  AC-020, and the residual profile identifies SQLite quantization above the same
  threshold. Q requires a new prospective treatment seal and review after that
  bounded post-V profile; it cannot be preselected;
- D requires measured queue wait plus idle-worker overlap, and L requires
  quantitative lookaside misses. The current profile does not supply either;
- otherwise stop with uncertainty and propose one bounded follow-up. Do not
  infer a need for private SQLite isolation.

`selection.md` freezes the chosen patch and verification before treatment code.

## TDD and semantics

For V, RED property/differential tests compare the proposed byte encoding and
SQLite results with the existing JSON/`vec_f32` path across supported
dimensions, finite edge values and signed zero. Human-authored search tests
continue to own ordering, eligibility, frozen/current views and error behavior.
GREEN changes only query-vector transport; stored vectors, schema, candidate
counts, quantization, reranking and public APIs do not change.

For any S treatment, RED names the observed redundant work and its semantic
boundary. GREEN changes only that operation. No test oracle, fixture, gate,
candidate count, lifecycle/dependency predicate or transaction boundary may be
weakened.

Focused verification consists of the treatment tests plus existing reader-pool,
Slice 75 interaction and frozen-read suites in the measured blast radius. A
selected implementation candidate additionally runs the retained AC-072 and two
Slice 71B write guards once each using their exact prior protocols.

## Evidence and cleanup

Every binary is bound to source, relevant-tree and file hashes. Raw profiles,
symbol reports, run logs and summaries are retained under
`dev/plans/runs/0.8.25-slice-77/`. The code reviewer checks the treatment diff;
the evidence reviewer independently checks retained receipts without rerunning
the campaign. Experimental code and temporary artifacts are removed after
review, and release state advances according to the decision dossier.
