# Slice 15 decision-measurement environment

- Source prototype: `06a54027`
- Date: 2026-09-14
- Host: Linux `7.0.0-30-generic`, x86_64
- CPU: AMD Ryzen Threadripper PRO 5945WX, 12 cores / 24 threads
- Rust: `rustc 1.95.0 (59807616e 2026-04-14)`
- Cargo: `cargo 1.95.0 (f2d3ce0bd 2026-03-21)`
- Build: release, `fathomdb-engine` features `test-hooks,operator`
- Timing: monotonic microseconds, warmup outside samples, alternating paired
  arm order
- Percentiles: nearest-rank
- Across-campaign estimate: median and IQR over five paired campaigns
- Interval: all 3,125 five-of-five resamples, deterministic 2.5th and 97.5th
  nearest-rank percentiles

The graph evidence treatment ran inside the same graph reader transaction as
selection. The earlier endpoint-loader shortcut and its pre-correction
primary-connection serialization remain separately labeled exploratory
evidence and are excluded from the decision analysis.

The process-isolated RSS arm records Linux `VmHWM` growth after fixture setup.
Its values have page/allocation granularity and are comparative, not an object-
allocation census. The 100 KiB arms use exactly 102,400 canonical-source bytes.
The maximum-work arm visits exactly 10,000 edges and returns 50 targets.

The point-resolution 1 KiB arms use exactly 1,024 canonical-source bytes. The
timed treatment executes two class-specific data statements total after
selection. Plan inspection is outside the timed path; minting consumes the
validated, source-deduplicated material without further SQL or hashing. Writer
arms rotate order across isolated fixtures. Each writer reports readiness
before timing. All arms issue foreground writes independently for the same
fixed one-second window with common two-millisecond pacing and no
load-dependent wait, poll, or quota inside the timed writer loop. Loaded arms
must naturally complete at least 20 background operations; recorded background
counts exclude setup and shutdown. Erasure arms use a before-primary-lock
rendezvous and alternate idle/held order.
