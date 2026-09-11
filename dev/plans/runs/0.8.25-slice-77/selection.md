# Slice 77 treatment selection

Decision: **no treatment met the sealed selection rule**.

The direct post-cache statement-reuse profile reproduced the registered gate's 1,600-search sequential warm state before profiling the 1,600 concurrent searches. It covered all eight reader workers and produced 52 concurrent CPU samples. Five teardown samples were excluded from mechanism attribution, leaving 47 search-phase samples.

| Observed concurrent search stack | Samples | Share of 47 | Selection consequence |
| --- | ---: | ---: | --- |
| SQLite mutex/locking paths | 23 | 48.9% | Active CPU evidence only; it cannot distinguish sleeping on SQLite locks from channel or scheduler wait. |
| Allocation paths | 16 | 34.0% | Overlaps other categories and does not identify a bounded product treatment. |
| Page-cache paths | 11 | 23.4% | Suggests a possible lookaside miss-size/full question, but the required quantitative miss counters are absent. |
| Vector conversion | 1 | 2.1% | Treatment V is below the 10% selection threshold. |
| Parser/preparation | 2 | 4.3% | Treatment S is below the 10% selection threshold; the retained statement-reuse anchor already addresses preparation. |

The categories overlap and must not be summed. The CPU profile does not measure elapsed blocked time.

Candidate disposition:

- **V:** not selected; vector conversion is 2.1% of search samples.
- **Q:** not eligible; V was not selected and the required post-V profile does not exist.
- **D:** not selected; no queue-wait plus reader-idle overlap measurement exists.
- **L:** not selected; the profile makes it plausible, but no `LOOKASIDE_HIT`, `LOOKASIDE_MISS_SIZE`, and `LOOKASIDE_MISS_FULL` census establishes a quantitative mechanism.
- **S:** not selected; parser/preparation is 4.3% of search samples.
- **Private-runtime isolation or shared-runtime `MEMSTATUS` changes:** outside Slice 77 authority.

The bounded follow-up for Slice 80 owner consultation is whether to allocate reserved Slice 78 for one per-reader lookaside counter/high-water census on the exact statement-reuse anchor, followed by at most one counter-directed setting. This record proposes that option; it does not allocate the slice or authorize isolation.
