---
title: FathomDB 0.8.27 Slice 70 - independent verification
status: PASS
target_release: 0.8.27
candidate: 36fc2352cf243e022315ea302368d9424096aebd
---

# Slice 70 independent verification

The independent read-only verifier (Sonnet) returned **PASS with
unavailable evidence** at clean candidate `36fc2352`. The single unavailable
item, AC-037's live network-namespace layer, was then run and passed on
2026-09-26 (see "AC-037 live layer" below). With it, no Slice 70 evidence is
unavailable except Metal.

| Gate | Result |
| --- | --- |
| `scripts/agent-verify.sh` at `6d13f548` | Lint and typecheck pass. 127 registered, 126 passed, 1 failed, 0 skipped, 0 excluded. The failure is resolved below. |
| Security (verifier run) | 0 violations, 0 blockers, 1 downgrade. AC-036, AC-038 (2/2), AC-050a (Rust, Python, TypeScript), and AC-050c (29 documented) pass. AC-037's offline catch and policy self-test pass; its live layer was unavailable in that run. |
| Security, strict, live AC-037 (main thread, `cacfce45`) | 0 violations, 0 blockers, 0 downgrades. `AC-037 OK` (every `connect()` loopback, `AF_UNIX`, or `AF_NETLINK` under `unshare -rUn` and `strace`) and `AC-037 catch OK (live netns)`. |
| `test-windows-wal-attribution-ci-job` after `36fc2352` | 313 passed, 0 failed, including the load-bearing timeout mutation |
| Candidate-bound Python receipt | Candidate `6d13f548`; native module SHA-256 `2e8e834900f819c615ec6e86b7e260128a4f5e5cd6633ad785785e4f92b19fed` |
| Workspace Clippy, warnings denied | PASS |
| Workspace Cargo all-target check | PASS |
| Focused Slice 70 routes | Default 186, `test-hooks,migration-test-hooks` 31, `operator` 20, `default-reranker` 19, `default-embedder` 1, lib 78, slice35 audit 6. All equal to the pre-move baseline. |
| C1 gate and self-test | 26/26 |
| Test-target coverage | 262 targets; 53 feature-complete-only |
| Recorded receipts | The public (`c38153c9…`), hidden (`3ffedba2…`), and feature-complete (`6a95d578…`) SHA-256 values match. The feature-complete summary shows 21 runs and 357 planned tests: 349 passed, 0 failed, 8 ignored, at candidate `b0289e04`. |

## Canonical-gate failure and fix

`test-windows-wal-attribution-ci-job` failed three probes.

- **Cause:** a Slice 70 move side effect. The guard's awk `function_body`
  matches only indented methods. The `impl ProjectionRuntime` methods
  `report_runtime_connection_inventory_for_test` and
  `report_runtime_native_state_inventory_for_test` moved to
  `projection_runtime.rs`. Only same-named root free functions (column 0)
  remained in `lib.rs`, so the extracted bodies became empty.
- **Fix:** `36fc2352` adds an injectable `RUNTIME_SOURCE` that names
  `projection_runtime.rs`. It retargets the two probes and the timeout
  mutation fixture. Assertion strings are unchanged.
- **Verification:** the verifier confirmed the diagnosis and that no probe is
  weakened. The guard passes 313/313.
- **Scope of the rerun:** the change touches one test script, so the full
  gate was not re-run.

Neither the design review nor the per-batch evidence had exercised this
guard. Its marker-level checks read `lib.rs` as a whole and still passed;
only its function-body probes read the moved methods.

## AC-037 live layer

In the verifier's run, AC-037's live layer could not create an unprivileged
user namespace (`kernel.apparmor_restrict_unprivileged_userns=1` on Ubuntu
24.04). The gate classified this as an environmental downgrade, and it was
not counted as a pass.

The repository owner then installed a temporary per-binary AppArmor profile
granting `userns` to `/usr/bin/unshare` only. The global restriction stayed
enabled. `unshare -rUn true` succeeded. At `cacfce45`, whose `src/` and
`scripts/` are identical to candidate `36fc2352`, `STRICT=1
scripts/agent-security.sh` reported:

- **AC-037:** `AC-037 OK: all connect() syscalls were loopback / AF_UNIX /
  AF_NETLINK.`
- **Live catch:** `AC-037 catch OK (live netns)`.
- **Summary:** 0 violations, 0 blockers, 0 downgrades.

The repository owner then reverted the profile (`apparmor_parser -R` and
file removal). Afterwards, `unshare -rUn true` again failed with
`write failed /proc/self/uid_map: Operation not permitted`, confirming the
default restriction was restored. The procedure is recorded in
`dev/release/ac-037-live-netns-hitl-runbook.md`. Slice 150 must still run the
live layer on its qualification executor, following that runbook on Ubuntu
24.04 hosts.

## Feature-complete and CUDA evidence (main thread)

`scripts/test-feature-complete.sh` ran on this host's two RTX 3090s at
candidate `b0289e04`. It covered 21 runs and 357 planned tests: 349 passed,
0 failed, and 8 ignored. The summary SHA-256 is
`6a95d5781398e0dcd2e965908e15ed30565f69387e38c8110b5043660a727c93`.

- **Ignored tests:** all eight are documented opt-in measurements or stress
  entrypoints:
  - `eu7_real_corpus_ac_validation`;
  - `generate_cross_backend_pinned_mean_fixture`;
  - the Slice 40 50k and Slice 45 pagination measurements;
  - the Slice 55 release ceiling;
  - the Slice 72 watchdog child and the shared-CUDA stress test;
  - `write_cursor_join_index_measurement`.
- **CUDA selection:** R-CAL-4 recorded the candle-CUDA leg as measured on
  `cuda:0`. The GPU reranker scored on `cuda:0`.
- **Numerical equivalence:** measured with tolerance, not byte identity.
  - CPU↔CUDA: `cosine_min=1.000000000`, `l2_max=1.402e-6`, 0 raw or
    mean-centered sign flips.
  - CUDA↔ONNX-CPU: `l2_max=1.272e-6`.
- **Metal:** unavailable; there is no Metal host.
- **Executor inventory:** the Slice 0 inventory's "no usable NVIDIA driver"
  line is stale for this host.
- **Later changes:** they touch only one doc comment, two test files, and one
  test script. The focused suites above cover them.

## Environment and cleanup

The verifier used one disposable checkout-owned `.venv`: Python 3.12, a
non-editable `src/python[dev]` install, and a `.pth` entry naming
`src/python`. It removed the `.venv`, the receipt, the built native module,
and its caches. The tracked tree was clean at `36fc2352`.
