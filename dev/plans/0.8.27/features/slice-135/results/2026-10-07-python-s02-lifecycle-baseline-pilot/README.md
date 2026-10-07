---
title: Slice 135 installed Python S02-L baseline lifecycle noise pilot
---

# S02-L baseline lifecycle noise pilot

Five separate unprofiled blocks ran on the exact 0.8.26 source
`f99e002f0d2e4002f3694c9f8d4986b56089edaa`, using the installed wheel
with SHA-256
`7c11803d9629550be60a76e1549c4c0bb42d23978d9fc901cf5df3c463611282`.
Each block has one warmup and 20 valid measured fresh-process cycles. Each
cycle opens a real SQLite database with the CPU default embedder, materializes
a vector, times `Engine.close()` while retaining the engine handle, measures
memory immediately and after 200 ms, drops the handle, and reopens the
database. The runner verifies installed Python and native bytes against the
wheel. Each block retains commands, raw observations, GNU Time child counters,
host snapshots, all attempts and producer summary.

The [independent audit](independent-audit.json) parsed all 105 per-process raw
JSON files, reparsed the child resource reports, checked source identity fields
and runner/wheel/native hashes,
semantic flags and memory fields, and recomputed every block statistic. The
same audit produced byte-identical output after the blocks were copied here.
Its [negative control](negative-control.json) changed a reopen assertion while
updating the raw receipt hashes; the semantic check still rejected it.

| Block | Close p50 (ms) | Close p95 (ms) | Median opened-to-closed-idle PSS change (KiB) | Warning |
| --- | ---: | ---: | ---: | --- |
| 01 | 5.138 | 6.106 | -33 | None |
| 02 | 5.305 | 6.206 | -33 | One host swap-counter page; child swap zero |
| 03 | 5.301 | 6.110 | -33 | None |
| 04 | 5.201 | 5.955 | -33 | None |
| 05 | 5.320 | 6.071 | -33 | None |

The median spread is 0.181 ms and the p95 spread is 0.251 ms. The first-to-last
median changed by +3.52%, without a monotonic trend. Five paired blocks of 20
fresh-process cycles per version are retained as the descriptive comparison
design in the [S02-L subset protocol](../../s02-python-lifecycle-comparison-protocol.json).
Report each pair delta, its median and complete range; these observations do
not justify significance, equivalence or p99 claims. PSS/RSS changes describe
process memory, not ownership alone. Focused embedder-close tests remain the
ownership and bounded-shutdown contract oracle.

The original failed smoke at `/tmp/slice135-s02l-baseline-smoke-1` recorded a
venv symlink resolution error in the measurement driver. The driver was fixed
before this five-block pilot; the failed smoke was not included as a valid
sample. The source-bound runner is committed at `8d14ec4c7`; the independent
auditor is committed at `aeedd0e18`.
