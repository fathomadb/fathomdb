---
title: 0.8.25 Slice 45 independent verification
status: VERIFIED
verified_commit: 2f48e657
updated: 2026-09-05
---

# Slice 45 independent verification

Independent verification passed the final commit `2f48e657` with no remaining
P0, P1, or P2 finding. The unconfined fast gate passed 103/103 suites with no
skip or exclusion, including the standing-authorized ptrace security route.

## Functional and package evidence

- Focused Rust and schema tests pass, including 14 pagination cases and six
  schema-step-33 cases on Windows.
- Fresh Linux Python wheel and offline packed N-API runtime smokes pass and
  reproduce the same current frozen-context token.
- Native Windows Python and N-API builds pass at `084488d3`. The focused Node
  pagination suite passes 3/3; fresh installed wheel and matched packed N-API
  runtime smokes both pass. The only later product-source change is confined
  to a non-shipping test hook and passed independent review plus default and
  test-hook compilation at `2f48e657`.
- The Windows runner's Git Bash installation was not initially on `PATH`.
  Adding its installed directory allowed the unchanged canonical smoke to
  pass; no source correction or product waiver was required.
- The native-artifact validation helper and release-state view checks pass.
- The complete Engine connection-attribution gate passes 301/301 and the
  focused pagination suite passes 14/14 at `2f48e657`.

## Performance evidence

The authoritative result is
[`result.md`](../../../runs/0.8.25-slice-45-pagination/result.md). Its 190 raw
records bind a clean measured candidate `f6c42bda`, runner, binary, 10k/50k
databases, raw output, and result by SHA-256.

Frozen canonical pages add about 0.116 ms steady p95 and at most 0.218 ms in
the measured cold operation. Minting a context per page adds about
0.104–0.108 ms steady p95. Continuation adds only 0.011 ms steady p95.
Frozen operational-state reads add about 0.079 ms steady p95. The largest
median peak-RSS increase is 1.17 MiB. None crosses the preregistered joint
absolute and relative materiality boundary.

The overhead remains effectively flat from 10k to 50k rows, consistent with
the schema-33 O(1), branch-sensitive binding design. Full ordered walks remain
a separate operational observation: 45.92 ms for 10k items and 208.92 ms for
50k items.
