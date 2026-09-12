# Slice 85 status

Status: **COMPLETE ON RELEASE BRANCH — READY FOR RELEASE**

Publication is not authorized by this status.

## Outcome

Candidate `18fefc67c914940b0799eb5f6d5d23c9d3c25fb6` closes the
0.8.25 verification slice. Relative to the initially frozen product candidate
`2e52602c`, later changes are limited to planning, evidence validation, release
smoke tooling and private `test-hooks` diagnostics. Ordinary production
native-artifact bytes are unchanged.

- The final manifest accounts for all 31 obligations. AC-034c is the sole
  unavailable cell. The CUDA Engine p95 non-pass uses the owner's
  [explicit disposition](ce-engine-p95-exception.md) and no other oracle is
  relaxed.
- Local full verification passed 109/109 registered suites with no failures,
  exclusions or skips. Final candidate-focused verification passed the 307/
  307 Windows attribution structure/mutation suite, the `test-hooks` feature
  build, the final manifest's 28 tests and the ptrace-capable AC-036 gate.
  The conservative broad Engine-source invalidation also triggered one bounded
  AC-012 rerun: 1,000 queries passed at p50 1 ms and p99 3 ms.
- Runtime configuration passes Rust, statement reuse, Python and TypeScript
  source tests and the six fresh-process installed-runtime cases. Node 25 is
  the sole 0.8.25 Node target; other Node compatibility is deferred.
- AC-073 passes at p99 391 ms against the 730 ms bound. The exact-candidate
  TC-5 GPU bridge completed 100 queries over 7,667 documents at recall 0.958
  with 95% CI [0.938, 0.974], satisfying the governed one-sided AC-075 oracle.
- The six protected-write candidate reruns pass. They satisfy the recorded
  invalidation policy; the intervening Engine edits were debug-only and did
  not independently establish a release-path regression.
- Exact-candidate hosted CI run
  [34697754978](https://github.com/fathomadb/fathomdb/actions/runs/34697754978)
  passes the required inventory and native-artifact runtime rows. Its
  self-hosted Windows job passes a clean Python wheel, Node 25 N-API/TypeScript
  build and installed-runtime validation after runner-only PATH, Python,
  Maturin and PowerShell repairs.
- Exact-candidate direct-SSH Jetson proof passes a clean `0.8.25+tegra` wheel
  build and installed `cpu`, `auto` and `cuda:0` policies on the registered
  Orin host. The CUDA route records the registered GPU UUID and a validated
  witness. GitHub upload was not required and no publication job ran.

Three Rust integration binaries continue to open raw SQLite before `Engine`
only to build compatibility, pre-upgrade tokenizer and legacy/null-provenance
fixtures that the public API cannot create. Their retained 12-test closure set
passes after process runtime configuration. They are known-closed, test-only
fixture builders, not a production dual-SQLite path or ambient test hazard.

Planning, design and implementation reviews are PASS. The final evidence index
is [the Slice 85 manifest](../../../runs/0.8.25-slice-85/final-manifest.json).
The release is ready for a separate publication decision; no tag, registry
upload or Pages publication occurred in this slice.
