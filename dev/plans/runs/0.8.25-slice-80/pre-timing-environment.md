# Slice 80 authorized-campaign environment diagnosis

Observed read-only at 2026-09-11T08:17:33-05:00 before any authorized final
campaign.

- The host has 62 GiB RAM, 45 GiB available, and 8 GiB swap with 2.3 GiB
  already allocated. Allocation alone is not swap I/O.
- Memory PSI averaged 0.00 across the 10-, 60-, and 300-second windows.
- A 30-sample `vmstat 1` observation showed zero `si` and `so` in all 29
  interval samples after its cumulative first line. This is a quiet preflight,
  not a waiver of each cell's zero-delta requirement.
- No FathomDB, Cargo, Rust, perf-gate, or AC-013 process was active. The largest
  resident workload was the running Windows VM QEMU process at about 17 GiB;
  its presence explains why host-wide counters cannot be attributed to FathomDB.
- The host's visible cgroup hierarchy did not expose root `memory.current` or
  `memory.events`; that missing optional observation does not replace the
  per-cell `/proc/vmstat` controls.

Conclusion: no evidence attributes prior swap activity to FathomDB. The host is
quiet enough to attempt the owner-authorized final campaigns, but every cell
still requires zero start/end machine-wide swap deltas and an empty corrected
competitor census. No process or host setting was changed.
