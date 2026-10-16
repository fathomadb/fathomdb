# COMP-01 / COMP-02 execution side plan

**Status:** complete, 2026-10-15; COMP-03 not run. A side plan, not part of any release. Owner
directive: run COMP-01 first, then COMP-02a and COMP-02b; add adapters only
as needed; no Steward or Orchestrator. Design source: the
[0.8.27 campaign plan](2026-10-13-fathomdb-0.8.27-performance-testing.md).
Branch `llm/comp-01`, worktree `fathomdb-worktrees/comp-01`.

## Deliberate simplifications

- One lean harness, `experiments/comp_fts.py`, instead of threading cells
  through the receipt-heavy `competitive_runner`. One cell is one product arm
  and one fresh-database repetition, run in its own subprocess. Its JSON
  output holds ordered IDs, per-query timings, build/ready times, RSS, and
  disk use.
- Tests cover measurement correctness only: converter counts and
  determinism, metric math against hand-computed values, roster order and
  resume, and one real-database smoke test per arm. No tamper, digest-drift,
  or receipt-hardening tests (owner, 2026-10-15).
- Environment: a dedicated venv with the repo's 0.8.27 build and pinned
  competitor packages. Versions are recorded in every cell output.

## Steps

1. [x] NFCorpus converter (reused from the 2026-10-14 draft) and its tests.
2. [x] Harness: FathomDB and LanceDB FTS arms, `AB/BA/BA/AB` roster,
   `--cell` / `--max-cells` / resume, scorer (nDCG@10, Recall@10, MRR@10;
   duplicate-body-affected queries reported separately).
3. [x] COMP-01 smoke, then 8 measured cells, then the [result note](../2026-10-15-comp-01-nfcorpus-fts-result.md).
4. [x] COMP-02a ([result](../2026-10-15-comp-02a-ingest-ready-scale-result.md)): inspect crud-bench and build the SurrealDB embedded arm. Use
   SCALE-02 10k, then 17,272 documents. Record ingest-ack, query-ready with
   sentinel and spread probes, reopen, and steady latency.
5. [x] COMP-02b ([result](../2026-10-15-comp-02b-filtered-fts-result.md)): a `bench_cohort` attribute at 1/10/33% selectivity, with
   filtered FTS through FathomDB `search_projected_text` + `SearchFilter`
   versus a SurrealDB filtered FTS query, and a membership check on every hit.
6. [x] Result notes for COMP-02a/b, then scoped tests and lint.

Raw artifacts live outside Git under `~/.cache/fathomdb-comp/`; result notes
cite their paths.
