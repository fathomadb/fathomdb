---
title: FathomDB 0.8.27 Slice 115 — TDD and measurement chronology
status: IMPLEMENTATION_RECEIPT
---

# Slice 115 TDD chronology

The engine source candidate is `4ca09d443d7bcc24072e39fb3c683905a9134f27`.
All new code is measurement-only under `scripts/`. The first test file was
staged before the validator implementation; subsequent contract tests were
run RED before their corresponding parser or workload changes. Test assertions
were not weakened to make a measurement pass.

| Step | RED observation | GREEN change and check |
| --- | --- | --- |
| Receipt schema and percentile contract | `test_slice115_receipt.py` failed to import the absent validator. | Added `slice115_receipt.py`; four checks passed for nearest-rank tails, bindings, invalid attempts, semantic failure and nontrivial attribution. |
| Mutation prestate and profile contract | Three `test_slice115_profiles.py` checks failed: absent profile alternative and unvalidated erasure/prestate. | Added strict fresh-state and operation-profile checks; seven cumulative checks passed. A test-fixture shallow-copy alias was corrected before the final GREEN, without changing its expected result. |
| GDB stack classification | `test_slice115_profile_sampler.py` failed to import the absent profiler, then its natural-completion count test failed. | Added the MI sampler, operation-frame classifier and completion parser; ten cumulative checks passed. A first graph-evidence profile timed out because buffered MI lines were hidden from `select`; a byte-buffer reader corrected it. |
| Real default-model projection | `test_slice115_model_probe.py` failed because inference-only samples passed without a projected vector. | The validator now requires a real projected row and projection time; the workload writes and drains through `Engine::open_with_choice(Default)`. Eleven cumulative checks passed. |
| Real database smoke | The first complete workload attempt failed on an absent graph seed used as a null control; `GraphSeedUnavailable` is retained. | A present seed plus absent edge kind yields the intended empty graph control. Attempt 6 ran twelve path cells, one warm-up and seven semantic samples each, on a real engine. |
| Review repair: write-to-ready | Added a test requiring projection latency to include both write and drain; it failed because the receipt accepted a shorter interval. | Timed before write through drain, retained separate write/drain timings, and required their sum within latency. The real-model projection interval also begins before write. |
| Review repair: unconditional controls and lock binding | Deleting raw `strict_*` flags let missing model projection and bad mutation prestate pass; a missing protocol lock digest raised `KeyError`. Five RED outcomes were recorded across 13 focused checks. | Made both semantic controls unconditional and froze `Cargo.lock` SHA-256 in the protocol before building. All 13 focused checks passed; corrected attempt 7 ran all twelve real-engine cells and was reprofiled against its exact binary. |
| Evidence scan false positive | The staged Gitleaks hook flagged the pinned tokenizer SHA-256 in eleven Slice 115 JSON artifact paths. A focused test failed without an exception and proved a different high-entropy digest would still be flagged. | Added an exact-digest, exact-path exception to the current-tree policy and its equality guard. The focused scanner test and policy guard pass; no hook is skipped. |

The final attempt 7 raw receipt, profile artifacts, protocol, runner bundle, environment
observations and recalculated summary are in [evidence/final](evidence/final).
The [invalid-attempt register](evidence/invalid/attempts.md) retains failed
and superseded trials. The full repository gate and independent reviews are
recorded in [status](status.md) after execution.
