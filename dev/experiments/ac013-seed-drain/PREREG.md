# AC-013 drain backlog cost: pre-registration

Written 2026-10-30 on `exp/0.8.29-ac013-scale-drain`, cut from
`exp/0.8.29-ac013-scale` `1fd4bd2bc`, before any timed run. Tools are listed
in [README.md](README.md). Todo TC-68169770 (seq 351).

## Question

At n = 100,000 (768-d, held), a 4 MB worker cache cut seed time from about
43 s to about 33 s. A second cost grows with the pending backlog and does not
respond to cache size: 89–116 dispatcher misses per vector (12 at n = 10k)
and 6–11 M writer misses during the drain. Where does that cost come from,
how does it scale, and what removes it under CS64 (`chunk_size=64`, the lead
fix)?

## What the diagnostic runs showed (untimed, before this file)

These are traced smoke runs (`FATHOMDB_PERF_PAGE_TRACE=all`, CS64, held). No
timing below decides anything.

- `EXPLAIN QUERY PLAN` of the scheduler scan (`scheduler_scan_sql`) and of
  the candidate-page query used by `drain`'s probe (`candidate_page_sql`):
  the `UNION ALL` subquery runs as a `CO-ROUTINE` that feeds a
  `USE TEMP B-TREE FOR ORDER BY`. `LIMIT 256` applies only after every
  pending row past the cursor has been read and sorted.
- The scheduler scan's page requests per run: 5,429 at n = 10k and 16,940 at
  n = 30k, about one per pending row.
- At n = 30k the writer ran `drain`'s probe 299 times, at 22,438 page
  requests per run. Each probe ran while the workers waited for a slow
  dispatcher scan, which left `active_jobs + queued_jobs == 0`.
- The prototype `FATHOMDB_PERF_DRAIN_SCAN=armlimit` gives each arm its own
  `ORDER BY write_cursor LIMIT 256`. With it, scan requests per run were 290
  at 10k and 300 at 30k, the writer ran 2 probes, and the projected-store
  digests matched CS64's.

## Hypotheses

- **H1 (cause).** The backlog cost comes from two queries that read the whole
  pending set on every run. The dispatcher's scheduler scan runs about once
  per 64-row batch. `drain`'s probe (`connection_has_pending_projection_work`)
  runs on the writer connection, once each time the runtime goes idle. Both
  total O(N²) over a drain.
- **H2 (fix).** `armlimit` makes both O(page), so the drain becomes O(N),
  with byte-identical results.

## Cells (`cells.json`)

All runs are 768-d. "Held" means the scheduler is frozen while the corpus is
written, then the drain runs alone.

| Cell | Layout | Runtime `cache_size` | Drain scan | Mode | Workers |
| --- | --- | --- | --- | --- | --- |
| BASE | 1,024-row chunks | default (-2000) | shipped | held | 2 |
| BASE-c4000 | 1,024 | -4000 | shipped | held | 2 |
| BASE-c4000-AL | 1,024 | -4000 | armlimit | held | 2 |
| CS64 (primary) | 64 | default | shipped | held | 2 |
| CS64-AL | 64 | default | armlimit | held | 2 |
| CS64-o | 64 | default | shipped | overlap (gate shape) | 2 |
| CS64-AL-o | 64 | default | armlimit | overlap | 2 |
| CS64-t1 | 64 | default | shipped | held | 1 |
| CS64-AL-t1 | 64 | default | armlimit | held | 1 |

## Runs

1. **Timed sweep**, untraced, 2 repeats, interleaved (the cell order reverses
   on odd repeats):
   - all held two-worker cells at n = 10k, 30k and 100k;
   - CS64-o and CS64-AL-o at 10k and 100k;
   - CS64-t1 and CS64-AL-t1 at 10k.
2. **n = 300k**, 2 repeats, CS64 and CS64-AL. Run only if CS64's 100k held
   drain median is ≤ 90 s. The projection from the 10k–100k fit then puts a
   run under 15 minutes. If CS64 exceeds that, run CS64-AL alone at 300k.
3. **Traced runs**, 1 per cell, for attribution only:
   - BASE-c4000, CS64 and CS64-AL at 10k and 100k;
   - CS64-t1 and CS64-AL-t1 at 10k.
4. **Equivalence**:
   - every run records the projected-store digest (`_fathomdb_vector_rows`,
     `vector_default` without `created_at`, `_fathomdb_projection_terminal`,
     and the `projection_cursor`);
   - `exp_ac013_vector_reads` (AC-013 searches) runs once each with CS64 and
     CS64-AL at n = 10k, comparing `results_digest` and `ids_digest`;
   - the engine's vector, projection and drain test targets run with
     `armlimit` on.

Every timed run holds the shared measurement lock and starts at 1-minute
load ≤ 2. Load is recorded before, at its peak during the run, and after. A
run whose peak or after-load exceeds 2.5 is flagged. A flagged run that
decides rule 2, 3 or 5 is repeated once.

## Decision rules

1. **Cause (H1).** Confirmed if both hold:
   - in the traced runs, scheduler-scan page requests per run grow at least
     5× from 10k to 100k in BASE-c4000 and in CS64;
   - in CS64-AL they stay within 2× of the 10k value.

   The writer-side part is confirmed if CS64's writer drain page requests at
   100k come mostly (> 50%) from the candidate-page statement.
2. **Scaling.** For each held cell, take per-vector drain time
   t(N) = median drain ms / N.
   - **Linear:** t(100k) / t(10k) ≤ 1.3, and t(300k) / t(10k) ≤ 1.5 where
     300k was run.
   - **Super-linear:** t(100k) / t(10k) ≥ 1.5.

   Also fit drain = a·N + b·N² to the medians, and report the N² term's share
   at 100k.
3. **Backlog share under CS64.** At 100k, report:
   - (CS64 drain − CS64-AL drain) / CS64 seed, held;
   - (CS64-o seed − CS64-AL-o seed) / CS64-o seed, overlap.
4. **Equivalence.** All three must hold, or the prototype is rejected:
   - every CS64-AL store digest equals CS64's at the same N;
   - the vector-reads digests are equal;
   - no engine test fails under `armlimit`.
5. **Fix worth a TDD slice.** All three must hold:
   - rule 4 holds;
   - the CS64-AL held drain at 100k is ≤ 0.8× CS64's in both repeats;
   - at 10k, the CS64-AL median is no more than 5% slower than CS64's.
