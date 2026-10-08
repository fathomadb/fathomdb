---
title: Slice 135 E01–E12 engine paired comparison at source 224e44c59
status: AUDITED_DIAGNOSTIC_NOT_PHASE1_CHECKPOINT
target_release: 0.8.27
---

# E01–E12 engine paired refresh at source 224e44c59

The [frozen protocol](../../e12-224e-comparison-protocol.json), SHA-256
`90230a1354b28b6cd7851207305a90d26801811e9db3005c6b8cfefe6ab92e44`,
preceded timing. It pairs exact 0.8.26 source
`f99e002f0d2e4002f3694c9f8d4986b56089edaa` with repaired candidate
`224e44c593c13d86ece648adabe445723db04070` through unchanged
release-mode E01–E12 workload binaries, pinned 32-row corpus and local CPU
model. The candidate binary SHA-256 is
`9809af9e7687711d536fc4cc621185f2107854b4604e9d7031915152a29d358e`.

The [schedule](schedule.json) records 20 alternating blocks separated by at
least 20 seconds. Each query cell has 1,000 valid observations per block and
supports p50/p95/p99; each lifecycle cell has 100 and supports p50/p95. The
[independent paired audit](paired-audit.json) accepted all twelve cells and
zero invalid blocks. It recomputed raw observations and checked source,
binary, runner, model, ordered outputs, persisted state, sample counts,
schedule and resources. A second audit of the copied archive produced
byte-identical JSON. The [negative controls](negative-controls.json)
rejected altered schedule order and an altered ordered text ID.

These are medians of five within-pair percentage changes. Positive means
the candidate took longer. The audit retains every pair value and range.

| Cell | p50 change | p95 change | p99 change |
| --- | ---: | ---: | ---: |
| Text | +7.04% | +3.79% | +1.43% |
| Vector stage | +15.67% | +11.45% | +16.25% |
| Hybrid | +14.54% | +10.67% | +13.66% |
| Graph expansion | -1.86% | -0.32% | +1.20% |
| Graph evidence | +2.12% | -2.17% | -3.39% |
| Fresh open | +2.97% | +3.32% | Unsupported |
| Populated open | +12.14% | +11.42% | Unsupported |
| Close | +4.25% | +2.92% | Unsupported |
| Canonical write | +0.79% | +0.73% | Unsupported |
| Projection | +1.06% | +0.35% | Unsupported |
| CPU model call | -1.14% | -0.96% | Unsupported |
| Erasure | -0.44% | +1.76% | Unsupported |

Vector-stage, hybrid and populated-open p50 increased in all five pairs.
Their observed p50 ranges are +10.40% to +24.84%, +6.88% to +23.07%,
and +9.50% to +12.48%. Seven blocks recorded host-only swap-counter drift,
including one 120-page warning; no measured child swapped. The four
warning-free query pairs retain positive vector-stage and hybrid p50
changes, with medians +14.69% and +13.91%. Only one populated-open pair is
warning-free, so its sensitivity is too narrow for a separate verdict.
Close has one +164.73% within-pair p50 outlier; the other four pairs range
from -0.73% to +4.77%. Preserve that valid observation for attribution.

The copied [raw archive](raw-archive/) is local and untracked pending
end-of-phase retention. Its 323 regular-file `SHA256SUMS` manifest has
SHA-256 `db89cf20de42a8313fe1806b0685de7b7690b600e2c043e94aa835715635bd1d`;
model asset symlinks are bound by pinned hashes. The copied archive passed
`sha256sum -c`. The schedule and paired-audit SHA-256 values are
`72331d0ddf2949833bc1d8053768d5ee5968af5de846b5970214acb185a89067`
and `f37f40d390531c1926e80a9bb051bb84037270afafe4ee45fde6ddc1ba745bb1`.

These are fixed 32-row CPU **engine** results, not installed-SDK or
competitor latency. The vector, hybrid and populated-open leads need
call-boundary attribution alongside the near-neutral Python S02 result.
They do not close the broader Phase 1 protocol or checkpoint.
