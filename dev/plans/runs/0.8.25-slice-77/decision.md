# Slice 77 decision dossier

## Decision

Carry statement reuse forward as the eligible optimization anchor, but do not ship it alone: the registered AC-020 gate still passed 0 of 7 runs. Do not select another implementation from the current evidence.

## Why

- Statement reuse preserves the large absolute improvement established in Slice 76 and reconfirmed here.
- The post-cache CPU profile has only 47 search samples and cannot rule vector conversion or parser/preparation in or out under the sealed threshold.
- The remaining active stacks point toward SQLite locking, allocation, and page-cache work, but CPU sampling cannot identify elapsed sleeping time and the experiment did not collect the counters required to select a lookaside correction.
- Jumping to private-runtime isolation or a shared-runtime `MEMSTATUS` change would exceed the slice's evidence and authority.

## Next consultation

Slice 80 should choose between:

1. allocating reserved Slice 78 for a corrected search-only profile, a bounded per-reader lookaside hit/miss/high-water census, and at most one evidence-directed setting; or
2. producing a separately authorized private-runtime feasibility plan if the owner decides the packaging and platform cost is justified.

The first option is the smallest evidence-producing follow-up. Neither option is authorized by this dossier. AC-020 remains unresolved, and release verification must not treat Slice 77 as a passing performance gate.
