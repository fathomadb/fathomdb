---
title: FathomDB 0.8.27 Slice 114 — configuration audit status
status: COMPLETE_ON_RELEASE_BRANCH
target_release: 0.8.27
product_and_audit_sha: 25115902db8b9b5648c5ceca1823d3cc66fec8e8
---

# Slice 114 configuration audit status

Slice 114 is complete on `release/0.8.27` under the HITL's 2026-10-04
direction to proceed as if Slice 110 were complete. At this slice's close,
Slice 110's intermittent Jetson forced-CUDA acceptance row was open and was
**not** qualified by this disposition. The source/audit commit is `25115902db8b9b5648c5ceca1823d3cc66fec8e8`, based on
`eefc2e3d8`.

## Post-close review of Slice 110 Tegra changes (2026-10-06)

The Slice 110 Tegra allocator and early-`cuInit` changes merged at
`a25d063cd`; Slice 110 subsequently closed at `33102cfc2`. Comparing the
Slice 114 closeout `acc04b8b1` with that release HEAD shows no change under
`src/rust/crates/fathomdb-engine/` or to the engine configuration reference.
The new `FATHOMDB_CUDA_EARLY_INIT=off` control is a Linux AArch64 Node-addon
registration option, documented in the TypeScript interface and Jetson guide;
it is not an engine-consumed setting. The vendored cudarc synchronous
allocation fallback changes internal CUDA allocation on AArch64 Linux but
does not change an `EngineConfig` field, default, range, SQLite mode, or
engine operational limit. The focused `runtime_configuration` test remains
green (8/8), and `check-release-state-views.sh` passes. **Slice 114 stays
closed; no configuration-audit rework is needed.**

## Outcome against requirements

| Acceptance | Result |
| --- | --- |
| AC27-114A | The [census](inventory.md) traces 152 unindented engine declarations, function-local production constants/statics, environment reads, public controls, and 32 dead-code suppressions plus 17 underscore bindings. Each declaration has a source location, classification, code-use witness, and disposition. Default and `operator,test-hooks,tc5-benchmark` engine feature checks produced no unused-binding warning. |
| AC27-114B | All five `EngineConfig` fields are traced from Rust/Python/TypeScript spelling and validation to effective engine consumers. Defaults `2/5/30,000/1,000,000/100`, ranges, zero behavior, precedence, mutability, and failure/fallback outcomes agree with accepted Slice 90 contracts. Process-wide SQLite mode and live profiling/telemetry controls are separately classified. |
| AC27-114C | The accepted SQLite ADR and code permit same-mode repetition after open, while public guidance said any post-open request failed. A real-Engine subprocess witness now pins repetition and conflict; config, Python, TypeScript, Rust interface, and API guidance was corrected. Stale rusqlite-version, experiment-gate, and expired Wave 5 comments were corrected or deleted. No unused constant was silently removed or promoted to a new knob. |
| AC27-114D | Runtime implementation, settings, defaults, ranges, exports, and public API are unchanged. The new test covers existing behavior; all other source changes are comments. Focused verification and independent reviews pass. No full regression was run because the HITL directed scoped verification for this specific finding. |

The discrepancy was in documentation: the contract-correct new test passed
against the original runtime. No production-behavior RED/GREEN cycle was
needed. The plan retains failing-test-first RED/GREEN for any future behavior
correction; no failing test was manufactured to justify this prose fix.

## Review and verification

- First design review: `gpt-6.1-sol` high approved after removal of an
  incorrect proposed RED test. See [design review](design-review.md).
- Independent code review: `gpt-6-sol` high found and closed inventory
  misclassifications, import-as-consumer references, missing live controls,
  and stale comments. Its narrow rereview found no open issue.
- Independent Terra high verification passed: Rust
  `runtime_configuration` 8/8; engine feature check; `cargo fmt --all --check`;
  `git diff --check`; `agent-lint-md.sh`; and `agent-lint-docs.sh`. The
  initial markdown-table style finding was fixed and the gate rerun green.
  Ruff checked the touched Python module, and the default engine check passed.

These are affected-path checks, not a full-workspace green claim. No
performance measurement, published package, or platform GPU qualification is
claimed by Slice 114.

## Handoff

Slice 115 may use the exact settings and internal-limit inventory to select
measurement cells. The unused `_material` construction in graph evidence
resolution creates clones without a consumed value; investigate its cost in
Slice 115 before any performance edit. Slice 140 owns narrowing broad
`allow(dead_code)` and moving retained diagnostic/test seams. Neither handoff
changes a user setting here. After Slice 110 closes, Slice 115 is the next
unblocked release slice.
