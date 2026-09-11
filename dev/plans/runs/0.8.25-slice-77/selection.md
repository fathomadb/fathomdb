# Slice 77 treatment selection

Decision: **the diagnostic is inconclusive and authorizes no treatment**.

The direct post-cache statement-reuse profile reproduced the registered gate's 1,600-search sequential warm state before profiling the 1,600 concurrent searches and covered all eight reader workers. It captured 52 concurrent CPU samples, but five are engine/reader teardown because the temporary harness stopped profiling after the concurrent function consumed and dropped the engine. Only 47 samples belong to the search phase, below the sealed minimum of 50. The profile is therefore inconclusive for treatment selection.

| Observed concurrent search stack | Samples | Share of 47 | Selection consequence |
| --- | ---: | ---: | --- |
| SQLite mutex/locking paths | 23 | 48.9% | Active CPU evidence only; it cannot distinguish sleeping on SQLite locks from channel or scheduler wait. |
| Allocation paths | 16 | 34.0% | Overlaps other categories and does not identify a bounded product treatment. |
| Page-cache paths | 11 | 23.4% | Suggests a possible lookaside miss-size/full question, but the required quantitative miss counters are absent. |
| Vector conversion | 1 | 2.1% | Descriptive only; the failed sample gate prevents threshold comparison. |
| Parser/preparation | 2 | 4.3% | Descriptive only; the failed sample gate prevents threshold comparison. |

The categories overlap and must not be summed. The CPU profile does not measure elapsed blocked time. None of these percentages selects or excludes a treatment because the profile missed its sample gate.

Candidate disposition:

- **V:** unresolved and not selected; the search-only profile has too few samples.
- **Q:** not eligible; V was not selected and the required post-V profile does not exist.
- **D:** not selected; no queue-wait plus reader-idle overlap measurement exists.
- **L:** not selected; the profile makes it plausible, but no `LOOKASIDE_HIT`, `LOOKASIDE_MISS_SIZE`, and `LOOKASIDE_MISS_FULL` census establishes a quantitative mechanism.
- **S:** unresolved and not selected; the search-only profile has too few samples.
- **Private-runtime isolation or shared-runtime `MEMSTATUS` changes:** outside Slice 77 authority.

The bounded follow-up for Slice 80 owner consultation is whether to allocate reserved Slice 78 for one corrected search-only profile plus a per-reader lookaside counter/high-water census on the exact statement-reuse anchor, followed by at most one evidence-directed setting. This record proposes that option; it does not allocate the slice or authorize isolation.
