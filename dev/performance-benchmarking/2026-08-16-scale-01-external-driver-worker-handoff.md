# SCALE-01 external driver worker handoff

**Worker branch:** `experiments/performance-tc5-external-driver-20260816`

**Base:** `8101f1db76e96b6b1f7e7cd422dc23c14bc26d86`

**Scope:** synthetic-only implementation of the external CPU arm-driver ABI
consumed by `experiments.tc5_live_executor`. No corpus payload was read, no
model/device was loaded, and no EU7/smoke/benchmark/external artifact/index
operation was run.

## Owned changes

- `experiments/tc5_external_driver.py`
- `tests/experiments/test_tc5_external_driver.py`
- `src/rust/crates/fathomdb-py/src/lib.rs` only for the dev-only,
  `test-hooks`-gated vector-stage measurement seam
- this contract and handoff

The worker did not change the TC-5 manifest/live executor, historical EU7
test/output, shared receipt/index helper, PROGRAM, or Track Runner state.

## Red/green evidence

1. Red checkpoint `38b3b8a0` (`test(perf): specify tc5 external driver ABI`):
   `python3 -m pytest tests/experiments/test_tc5_external_driver.py -q`
   stopped at collection because `experiments.tc5_external_driver` did not
   exist.
2. The implementation adds strict environment parsing, external-root
   containment, manifest/corpus-input binding, deterministic bootstrap
   projection, exact test-hooked vector-stage runtime, and a content-free
   `tc5-arm-result.v1` writer.
3. Focused verification after implementation:

   ```text
   python3 -m pytest tests/experiments/test_tc5_external_driver.py \
     tests/experiments/test_tc5_live_executor.py \
     tests/experiments/test_tc5_manifest.py \
     tests/experiments/test_tc5_characterization.py -q
   52 passed

   python3 -m ruff check experiments/tc5_external_driver.py \
     tests/experiments/test_tc5_external_driver.py
   All checks passed
   ```

## Review focus

1. Confirm no driver path accepts a CLI argument, unknown `TC5_*` value,
   GPU request, wrong counts/pins, incomplete query/bootstrap result, or
   non-finite statistic before output.
2. Confirm `tc5-corpus-input.v1` binds the selected canonical manifest rows,
   contained payload paths, hashes, and all 100 fixed queries without leaking
   them into the arm-result sidecar.
3. Confirm the new PyO3 method is `test-hooks`-only and that the driver fails
   when the exact pre-fusion vector-stage seam is unavailable; it must never
   substitute fused search.
4. Confirm historical EU7 and all coordinator-owned files remain untouched.

## External prerequisites before any invocation

Independent acceptance and coordinator integration are still required. Then
the coordinator must qualify a real manifest/corpus-input/provenance set,
external copied-driver SHA, CPU test-hook wheel/model cache/host facts, output
root, and action-specific release record. This handoff does not authorize a
release or any live execution.
