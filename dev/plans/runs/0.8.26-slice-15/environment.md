# Slice 15 exploratory measurement environment

- Source: `6235daabd9b949ad436264da7888d9d4504d66cb`
- Date: 2026-09-13
- Host: Linux 7.0.0-30-generic, x86_64
- CPU: AMD Ryzen Threadripper PRO 5945WX, 12 cores / 24 threads
- Rust: `rustc 1.95.0 (59807616e 2026-04-14)`
- Cargo: `cargo 1.95.0 (f2d3ce0bd 2026-03-21)`
- Build: release, `fathomdb-engine/test-hooks`
- Estimator: nearest-rank percentiles over monotonic-clock microseconds
- Order: alternating control-first and treatment-first pairs after 50 warmups

The samples are exploratory only. The prototype at this source did not yet
implement the approved two-statement batched preflight or graph-operation
commitment, so they are not D26-01 decision evidence.
