---
title: Slice 135 integrated-candidate installed Python functional qualification
status: AUDITED_FUNCTIONAL_NOT_LATENCY
target_release: 0.8.27
---

# Integrated-candidate installed Python exercise — 2026-10-07

The candidate source `cdf253cd223a82e954591db532397a3d78a2027a` includes
the post-Slice-132 release merge `b65283317`, the corrected embedder-close
landing `96796fe04`, and the published `0.8.26+tegra` install-route fix
`c23e2d23f`. The source-bound [wheel](fathomdb-0.8.26-cp310-abi3-manylinux_2_39_x86_64.whl)
has SHA-256 `c33987023754fee2d85f887bfbbd17c68889f287603d3df06844d06a240a60a0`;
its native module has SHA-256
`79fd863733d1a97de308b445a5c45b2434e27a1317048376dbfdb76502ff0ae9`.
The filename still carries the workspace's 0.8.26 package version; the
source and artifact hashes identify the actual 0.8.27 candidate. The
[build log](wheel-build.log), installed [provenance](install-provenance.txt),
graph/evidence smoke and exact wheel bytes are retained.

The first [build attempt](dirty-wheel-build.log) accidentally packaged 17
ignored `__pycache__` members from the source checkout. The
[capability runner rejected it](dirty-wheel-rejection.log) because installed
bytecode differed from the archive. A red test preceded the packaging fix:
Maturin now excludes cached bytecode, and the release-wheel script checks the
archive before installation. The clean rebuild from the same cache-bearing
tree contained 24 members and no bytecode, then passed the installed frozen
evidence and Slice 50 graph/evidence smoke. The rejected wheel is not a
candidate measurement.

The [44-operation raw result](capabilities.json) ran 47 selected cases through
real SQLite databases in the installed wheel:

| Outcome | Operations |
| --- | ---: |
| Executed with selected positive, negative and reopen assertions | 40 |
| Failed | 0 |
| Committed dependency-closure positive-path gap | 1 |
| Unavailable provider/model conditions | 3 |
| **Governed live operations** | **44** |

The [independent audit](capabilities-audit.json) checked the exact wheel and
native bytes, source trees and selected test/fixture hashes, recomputed the
operation partition, and checked all 47 retained reopened SQLite snapshots:
table counts, persisted-row digest and active-body projection. It rejected a
changed persisted count and a false executed count. The temporary databases
were deleted by the runner, so this retained audit recomputes snapshots; it
does not rerun every original operation. The unresolved closure and
provider/model cases remain explicit gaps.

The same installed wheel completed one fresh real-database
[S02 sequence](s02-functional.json): open, 36 governed writes, projection
drain, text/vector/hybrid and graph/evidence reads, source erasure and
idempotent retry, close/reopen and final state checks. Its
[independent state audit](s02-audit.json) verified wheel/native and helper
hashes, the expected materialized observations and a tampered reopened
graph-count rejection. The **5,371.100 ms** whole duration includes
validation checks, so it is functional evidence only; it cannot replace the
frozen sampled S02 timing comparison or contention cell.

To rerun the capability receipt audit from the Slice 135 checkout root:

```sh
python3 scripts/slice135_python_capabilities_audit.py \
  --raw dev/plans/0.8.27/features/slice-135/results/2026-10-07-python-integrated-candidate/capabilities.json \
  --wheel dev/plans/0.8.27/features/slice-135/results/2026-10-07-python-integrated-candidate/fathomdb-0.8.26-cp310-abi3-manylinux_2_39_x86_64.whl \
  --repo . --output /tmp/slice135-python-integrated-audit-recomputed.json
```

[SHA256SUMS](SHA256SUMS) covers the retained receipt. This one Linux x86_64
wheel does not establish installed TypeScript/Rust qualification, the full
contract matrix, other platforms, concurrency or final Phase 1 latency.
