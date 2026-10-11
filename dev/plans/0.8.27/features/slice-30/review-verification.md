---
title: FathomDB 0.8.27 Slice 30 - independent verification
status: PASS
target_release: 0.8.27
candidate: 2967593cc77af82acc6a3e1e50d03970c1a07705
---

# Slice 30 independent verification

The independent verifier returned PASS at clean candidate `b102bceb` after
rejecting the earlier candidate's source-SHA equality defect.

| Gate | Result |
| --- | --- |
| Fresh Node 25.9.0 capture vs immutable baseline | PASS; `equal: true`, distinct validated source SHAs, no metadata or row diff |
| Comparator contract and hardening mutations | PASS |
| SDK governed parity | 10/10 passed |
| Rust surface controls | 4/4 default; 5/5 operator |
| Python surface/stub/parity | 20/20 passed |
| TypeScript focused surface route | 20/20 passed with `RELEASE_SURFACE_TESTS=1` |
| Release-state views, design references, diff hygiene | PASS |

The TypeScript production no-test-hook inspection genuinely executed. The
default-embedder network branch explicitly returned early under
`FATHOMDB_SKIP_NETWORK_TESTS=1`; the status record does not claim otherwise.

The corrected baseline SHA-256 is
`7c76bd0d409cecc5dc073e29baf7806b329379bd9475c7ab138dda12cde5cdf2`
(transcription corrected by the adversarial review; superseded by the
recaptured baseline recorded in `status.md`).
It is 10,206,103 bytes and 212,596 lines; all row counts matched the status
record. Verifier-created capture, cache, and log output was removed and Git
remained clean.

## Adversarial review addendum

This record verifies the 12-row tool at `b102bceb`. The comparator and baseline
were subsequently changed by adversarial-review FIX-1/FIX-2 (see `status.md`);
that review is the reviewer of record for those changes and the 13-row
baseline `b54a01cc486c…` captured at `58bc8eb4`.

## Final whole-work verification

Independent verification passed on clean `2967593c` for the remediated
execution paths:

| Gate | Result |
| --- | --- |
| Python artifact-gate contract | PASS; 26/26 focused checks |
| Candidate receipt | PASS; clean HEAD, exact source-module path, nonce, and SHA-256 validated |
| Surface comparator | PASS; clean capture compared equal with empty metadata and row diffs |
| NAPI artifact hygiene | PASS; debug build followed by production build removed both test-hook declarations |
| TypeScript Slice 55 | PASS; 16/16 and actual temporary directory removed |
| Focused Ruff and TypeScript typecheck | PASS |

The final receipt names candidate
`6ba3be95cd043570da1deafbe4e2f78c878d8a87` and module SHA-256
`2f6d0e2509d1d3b5928723d47a239ae9132ec210b1836fb76518f957a6c450b6`.
The canonical gate at `2967593c` passed lint, typecheck, strict security
0/0/0, Rust, TypeScript, executable NAPI, and 123 of 124 registered suites.
The sole failure was the superseded packaging assertion; after the test-only
`6ba3be95` correction, the full Python suite passed 1,533 tests with 27
documented skips and the receipt validator passed. No unrelated full
regression was repeated for that test-only change.

Final independent verification verdict: **PASS**.
