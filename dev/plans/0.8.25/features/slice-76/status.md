---
title: Slice 76 status
status: COMPLETE
date: 2026-09-10
cleanup_candidate: 8027546d
---

# Slice 76 status

Slice 76 is complete as an attribution experiment. It does not ship a product
correction and does not close AC-020.

## Result

Statement reuse reduced matched median sequential time from 564 ms to 175 ms
and concurrent time from 137 ms to 80 ms. Both B2 and C failed AC-020 in all
seven runs because speedup changed from 4.12x to 2.19x against the unchanged
5.33x requirement. Statement reuse is an eligible Slice 77 anchor; the
post-cache residual requires separate attribution.

The final diagnostic observed one prepare-time compilation for each of ten SQL
shapes on every reader, 80 total; automatic-reprepare counters were zero.
SQLite estimated 57,976 bytes of retained statement memory per reader, 463,808
bytes total. Old execution-history-based fresh/reused logs are retained as
superseded and are not treated as equivalent instrumentation.

## Acceptance

| Requirement | Status | Evidence |
| --- | --- | --- |
| R76-1 protect checkpoint and inventory | Pass | Slice 75 cell inventory; final relevant-tree hash `53916442...7b3c` matches protected `5056db9e` |
| R76-2 establish current scaling | Pass | 7 initial B and matched 7 B2 observations with raw logs and dispersion |
| R76-3 bounded attribution | Pass | SQL census, gperftools profile pair, prepare-time compile/reprepare/memory census; off-CPU waiting explicitly unresolved |
| R76-4 statement reuse experiment | Pass | RED `f155af05`; GREEN `9e913517`; matched B2/C series and focused correctness |
| R76-5 Slice 77 decision inputs | Pass | Ranked hypotheses and post-cache residual requirement in `result.md` |

Planning/design review passed. Independent code review passed after correcting
the recorded treatment commands, the diagnostic test feature gate, and the
compile/reprepare qualification. A separate read-only evidence audit verifies
raw retained results without launching another campaign.

## Focused verification

- runner contract: 7 tests passed;
- treatment statement mechanics: 3 passed;
- reader pool: 6 passed;
- feature interactions: 4 passed;
- frozen reads: 5 passed;
- MEMSTATUS behavioral witness: passed for B and C;
- gperftools: explicit search-phase boundaries, 8 registered reader workers,
  `libprofiler.so.0` linked, no tcmalloc;
- no broad regression, package build, hosted workflow, or timing rerun after
  diagnostic corrections.

## Cleanup and handoff

Experimental product/test commits remain identifiable in history. Cleanup
commit `8027546d` restores the four touched engine product/test files to the
protected checkpoint byte-for-byte while keeping plans, the runner, reviews,
and receipts. The audited `/tmp/fathomdb-slice76` build/binary tree was removed;
it is not recoverable there, while its durable profiles, logs, hashes, and exact
commands remain in the release evidence. No separate implementation worktree
was created.

Slice 77 must retain statement reuse as an eligible anchor and first attribute
the post-cache residual. The pre-treatment CPU profile cannot establish that
residual. If bounded diagnostics cannot identify it, Slice 77 should record the
uncertainty and propose one bounded follow-up rather than jump to isolation.
