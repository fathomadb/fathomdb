# Slice 135 populated-open stack sample — 2026-10-07 UTC

**Status:** a bounded attribution probe, separate from unprofiled E01–E12
latency. Sixteen interrupts per version are insufficient to localize the
observed populated-open difference to a specific function.

The exact 0.8.26 source was
`f99e002f0d2e4002f3694c9f8d4986b56089edaa`; the repaired candidate was
`0245733e42ad1c8b061bb257be793201d3106715`. The baseline measurement
binary SHA-256 was
`401d2375c66c0cd72c81ba8767f777cf79c9ce609c52eb4ceccf2405b4bcb251`;
the candidate binary was
`791574b743939d14188df3d47b70a48d3a8f48286e1af49fa5b32643282798f2`.
The sampler was `scripts/slice115_profile.py` at SHA-256
`fe848c2efc6e3bbd98c2d985b3d893d2daadb000e27f1121116ef3508f2f7045`.
The host blocked `perf` (`perf_event_paranoid=4`), so the existing GDB/MI
fallback was used. It ran the same `slice115_profile_loop` workload in each
binary for 10 seconds, collecting 16 interrupt/backtrace samples at 80 ms
intervals and a separate 10-second unprofiled control. Source symbols were
present, but full Rust debugging information was absent in these release
binaries. The profile loop repeatedly opens a populated real database and
includes per-iteration setup and cleanup; it is a path sample, not a precise
breakdown of the timed `open_populated` cell.

| Version | Operation stack samples / all samples | Unprofiled control operations | Profiled operations | Profiled/control ratio |
| --- | ---: | ---: | ---: | ---: |
| 0.8.26 | 7 / 16 | 152 | 139 | 0.914 |
| 0.8.27 candidate | 10 / 16 | 148 | 140 | 0.946 |

The [baseline MI transcript](baseline.mi), [candidate MI transcript](candidate.mi),
their [baseline](baseline.json) and [candidate](candidate.json) metadata, and
[baseline](baseline-control.log) and [candidate](candidate-control.log)
control logs are retained. The sample stacks include SQLite schema/pragma
work, writer I/O, reader-pool creation and projection-runtime startup. These
are *observed stack locations*, not a quantified attribution of the roughly
12% paired populated-open p50 loss. A stage-timed open diagnostic or a larger
sampling run with symbols is the next attribution step before optimizing it.

The first candidate GDB attempt inside the restricted executor exited invalid:
`ptrace: Operation not permitted`. Its
[transcript](sandbox-denied.mi) and [successful unprofiled control](sandbox-denied-control.log)
are retained. The same profiler command then ran unconfined with no test or
source modification; the baseline profile used the same privilege mode.
Profiled throughput is lower than its control on both versions, so none of
these profile-loop throughputs substitute for unprofiled paired latency.
