---
title: Slice 76 independent evidence review
status: PASS
---

# Slice 76 evidence review

An independent read-only verifier recomputed the retained results without
editing files or launching tests, builds, benchmarks, or another campaign.

Final verdict: **PASS** after two record-only closeout corrections:

1. The manifest now names the actual initial and matched observation/summary
   files rather than nonexistent generic paths.
2. Release-state generated views are regenerated after Slice 76 advances the
   next slice to 77.

The verifier confirmed:

- all initial and matched observation counts, order, binary identities and
  summaries;
- B2 medians 564/137 ms, C medians 175/80 ms, speedups 4.12x/2.19x, 0/7 passes
  in both arms, C's 32.8125 ms bound, and the approximately 59% residual;
- positive B/C MEMSTATUS witnesses;
- gperftools profile hashes, `libprofiler`-only linkage, no tcmalloc, and the
  CPU-only attribution limit;
- 8 readers by 10 prepare-time compilations, zero automatic reprepares,
  57,976 bytes retained per reader and 463,808 bytes total;
- retained treatment-enabled focused logs with 3/6/4/5 passing tests;
- all four Slice 75 retained receipt hashes;
- protected final relevant-tree hash `53916442...7b3c` and no product/test diff
  from `5056db9e` after cleanup;
- superseded fresh/reused logs are labeled non-equivalent and no diagnostic
  correction is represented as a timing rerun.

No evidence claim requires a further Slice 76 run.
