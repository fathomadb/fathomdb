---
title: FathomDB 0.8.27 Slice 30 - independent verification
status: PASS
target_release: 0.8.27
candidate: b102bceb
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
`7c76bd0d35e039ff9562c1f52897c21b9854ceeb12a56e7525ccf13938e92a8d`.
It is 10,206,103 bytes and 212,596 lines; all row counts matched the status
record. Verifier-created capture, cache, and log output was removed and Git
remained clean.
