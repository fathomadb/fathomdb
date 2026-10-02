---
title: Slice 90 stage-2 runtime performance qualification
status: PASS
target_release: 0.8.27
candidate_sha: 2e94aaf4f57399a5ded8fc39b9100e33fe09dbcd
---

# Slice 90 stage-2 runtime performance qualification

The measured engine source is `2e94aaf4f57399a5ded8fc39b9100e33fe09dbcd`.
The merged release engine source is byte-identical. The strict v2 D27 retry
provides a machine-validated PASS. The owner subsequently accepted the third
campaign's small projection-heavy throughput shortfall for this release; its
unchanged verifier failure and the earlier latency failure remain visible.
All other stage-2 matrix, named-selector, installed-binding, review and
verification gates pass.

## Configuration matrix

| Cell | Status | Candidate SHA | Command | Result |
| --- | --- | --- | --- | --- |
| 2/5 | PASS | 2e94aaf4f57399a5ded8fc39b9100e33fe09dbcd | `cargo test --release -p fathomdb-engine --features test-hooks --test slice90_runtime_matrix` | 3 passed; default five embed workers, 20 waiting slots, exact managed SQLite roles and cleanup. |
| 2/1 | PASS | 2e94aaf4f57399a5ded8fc39b9100e33fe09dbcd | `cargo test -p fathomdb-engine --features test-hooks --test slice90_runtime_matrix explicit_two_one_engine_has_exact_managed_roles_and_cleanup -- --exact` | 1 passed on merged test-only correction; exact managed SQLite roles, provider use, close/reopen and cleanup; engine source matches candidate. |
| 1/1 | PASS | 2e94aaf4f57399a5ded8fc39b9100e33fe09dbcd | `cargo test --release -p fathomdb-engine --features test-hooks --test slice90_runtime_matrix` | 3 passed; exact live roles and cleanup. |
| 2/2 | PASS | 2e94aaf4f57399a5ded8fc39b9100e33fe09dbcd | `cargo test --release -p fathomdb-engine --features test-hooks --test slice90_runtime_matrix` | 3 passed; exact live roles and cleanup. |
| 4/4 | PASS | 2e94aaf4f57399a5ded8fc39b9100e33fe09dbcd | `cargo test --release -p fathomdb-engine --features test-hooks --test slice90_runtime_matrix` | 3 passed; exact live roles and cleanup. |
| 64/64 | PASS | 2e94aaf4f57399a5ded8fc39b9100e33fe09dbcd | `cargo test --release -p fathomdb-engine --features test-hooks --test slice90_runtime_matrix` | 3 passed; resource inventory and cleanup only, without throughput claim. |
| 2/no-provider | PASS | 2e94aaf4f57399a5ded8fc39b9100e33fe09dbcd | `cargo test --release -p fathomdb-engine --features test-hooks --test slice90_runtime_matrix` | 3 passed; no embed workers or queue, and clean reopen. |

The checked-capacity property passed all 4,096 scheduler/embed worker pairs.
The merged explicit `2/1` log is retained at
`/home/coreyt/projects/fathomdb-worktrees/qualification-evidence/slice-90/stage2-gates-2e94aaf4f/matrix-explicit-2-1-merged.log`.

## Named release selectors

| Selector | Status | Candidate SHA | Command | Result |
| --- | --- | --- | --- | --- |
| AC-011a | PASS | 2e94aaf4f57399a5ded8fc39b9100e33fe09dbcd | `cargo test --release -p fathomdb-engine --test perf_gates ac_011a_write_throughput_1kb` | 1 passed; 1,239.384 commits/s against 1,000. |
| AC-011b | PASS | 2e94aaf4f57399a5ded8fc39b9100e33fe09dbcd | `cargo test --release -p fathomdb-engine --test perf_gates ac_011b_write_throughput_100kb` | 1 passed; 297.223 commits/s against 100. |
| AC-017 | PASS | 2e94aaf4f57399a5ded8fc39b9100e33fe09dbcd | `cargo test -p fathomdb-engine --test perf_gates ac_017_vector_projection_freshness_p99_le_five_seconds` | 1 passed unchanged. |
| AC-018 | PASS | 2e94aaf4f57399a5ded8fc39b9100e33fe09dbcd | `cargo test -p fathomdb-engine --test perf_gates ac_018_drain_of_100_vectors_le_two_seconds` | 1 passed; drain 64 ms within two seconds. |
| AC-029 | PASS | 2e94aaf4f57399a5ded8fc39b9100e33fe09dbcd | `cargo test -p fathomdb-engine --test projection_runtime ac_029_canonical_writes_complete_under_projection_stall` | 1 passed unchanged. |
| AC-072 | PASS | 2e94aaf4f57399a5ded8fc39b9100e33fe09dbcd | `cargo test --release -p fathomdb-engine --test perf_gates ac_072` | 3 passed official repetitions; p50 70/69/70 ms and p99 76/77/76 ms. |
| AC-073 | PASS | 2e94aaf4f57399a5ded8fc39b9100e33fe09dbcd | `cargo test --release -p fathomdb-engine --features operator,embed-cuda --test eu7_real_corpus_ac eu7_real_corpus_ac_validation` | stress receipt=dev/plans/0.8.27/features/slice-90/ac073-stress-receipt.json sha256=0751a56c48ce7f0b393971a2a9684271f800cc5c8b33922ca327677d1c39a8f0 combined-exit=101 AC-075=superseded-by-tc5 |
| AC-076 | PASS | 2e94aaf4f57399a5ded8fc39b9100e33fe09dbcd | `cargo test --release -p fathomdb-engine --test perf_gates ac_012_text_query_latency_on_fts5_path` | 1 passed; named release text-query selector, 1,000 samples at n=10,000, p50 1 ms and p99 2 ms. |
| AC-081a | PASS | 2e94aaf4f57399a5ded8fc39b9100e33fe09dbcd | `cargo test --release -p fathomdb-engine --test perf_gates --no-run; sealed perf-gates binary` | 7 passed official fresh-process observations; no warnings, sequential median 187.741 ms. |
| AC-081b | PASS | 2e94aaf4f57399a5ded8fc39b9100e33fe09dbcd | `cargo test --release -p fathomdb-engine --test perf_gates --no-run; sealed perf-gates binary` | 7 passed official fresh-process observations; no warnings, concurrent median 65.277 ms. |
| AC-081c | PASS | 2e94aaf4f57399a5ded8fc39b9100e33fe09dbcd | `cargo test -p fathomdb-engine --test reader_pool one_reader_progresses_while_another_reader_holds_a_snapshot -- --exact` | 1 passed unchanged. |

The combined CUDA EU7 selector exited 101 on AC-075 vector-stage recall
0.773, 95% CI high 0.799 below its old 0.90 floor. AC-073 mixed-tail stress
passed at p99 493 ms within its same-run 513 ms bound. The sealed bundle is
`/home/coreyt/projects/fathomdb-worktrees/qualification-evidence/slice-90/ac073-2e94aaf4f`.

## Installed bindings

| Binding | Status | Candidate SHA | Command | Result |
| --- | --- | --- | --- | --- |
| Python | PASS | 2e94aaf4f57399a5ded8fc39b9100e33fe09dbcd | `bash scripts/verify-slice90-python-wheel.sh 2e94aaf4f; python3 installed tests` | 31 passed isolated Python 3.12 cases; installed wheel real open/close on Python 3.10.20; wheel SHA-256 `af5b99daf877f81549d31064158c40afaea2fca7356c1593a4859d9d1232cd00`. |
| Node | PASS | 2e94aaf4f57399a5ded8fc39b9100e33fe09dbcd | `FATHOMDB_QUALIFICATION_NODE_BIN=/home/coreyt/.nvm/versions/node/v25.9.0/bin/node bash scripts/slice90-node-installed-qualification.sh` | 2 passed installed production and witness consumers; receipt SHA-256 `50744f5fab64bc0504ab955c913a8771a4a38cbf7899ef598d2eff1295bc7f35`. |

The exact-source evidence manifest with 26 hashed logs is
`/home/coreyt/projects/fathomdb-worktrees/qualification-evidence/slice-90/stage2-gates-2e94aaf4f/stage2-evidence-manifest.json`
(SHA-256 `a675ddc41a1b2eef44b95006efb234f35aed413c8cdb130e78d5779fda2c7255`).
The installed Python and Node receipts are retained in adjacent evidence
bundles under `qualification-evidence/slice-90/`.

## D27 candidate qualification

| Receipt | SHA-256 | Status |
| --- | --- | --- |
| dev/plans/0.8.27/features/slice-90/d27-candidate-receipt.json | 52a20f29cb3bbb19a0ec159dace4a3cfbd74307c2dd3234cf59dbbfc820ad9b1 | PASS |

The historical six-run entry at `7a2f9bf9` remains valid under the frozen v1
protocol (receipt SHA-256
`74c5ee0b43188dd2f2af138eb84dd67aa281034017693e095c82327c6dadc129`).
The v2 protocol changes only host-wide swap invalidation to report-only and
requires present, nonnegative, monotonic counters and raw-linked deltas. Its
corpus and median/MAD comparison rule are unchanged. The exact source
`2e94aaf4f` has two complete, environment-valid six-repetition campaigns:

| Campaign | Projection-heavy canonical-commit p95 median | Ceiling | Verdict |
| --- | ---: | ---: | --- |
| First v2 campaign | 7.1435 ms | 5.5604 ms | FAIL, 1.5831 ms above ceiling. |
| One v2 retry | 5.0637 ms | 5.5604 ms | Strict PASS, 0.4967 ms below ceiling. |
| Third v2 campaign, affinity 2-11,14-23 | 4.8958 ms | 5.5604 ms | Latency PASS; strict throughput FAIL, accepted by owner below. |

The third six-repetition campaign was environment-valid. Its projection-heavy
canonical throughput median was 33.1802 commits/s against the frozen 33.5926
commits/s floor, a 0.4125 commits/s (1.23%) shortfall. The unchanged strict
verifier rejected that campaign on throughput and produced no PASS receipt.
The repository owner's 2026-10-02 HITL ruling accepts that absolute shortfall
as PASS for the release decision. Its raw evidence and separate outcome report
are retained at
`/home/coreyt/projects/fathomdb-worktrees/qualification-evidence/slice-90/d27-v2-candidate-2e94aaf4f-third-affinity`.
The 20-logical-CPU affinity mask was verified on 19 sampled workload threads;
it constrained the benchmark but could not exclude other host activity from
those cores. The historical entry used all 24 logical CPUs. The earlier valid
latency FAIL remains retained. The copied strict PASS receipt is byte-identical
to the second campaign's external receipt and remains the machine-checkable
checkpoint artifact.

## D27 artifact bundles

| Phase | Absolute bundle path | Receipt SHA-256 |
| --- | --- | --- |
| entry | /home/coreyt/projects/fathomdb-worktrees/qualification-evidence/slice-90/d27-entry-post-reboot | 74c5ee0b43188dd2f2af138eb84dd67aa281034017693e095c82327c6dadc129 |
| candidate | /home/coreyt/projects/fathomdb-worktrees/qualification-evidence/slice-90/d27-v2-candidate-2e94aaf4f-retry | 52a20f29cb3bbb19a0ec159dace4a3cfbd74307c2dd3234cf59dbbfc820ad9b1 |

The first valid failed campaign is retained separately at
`/home/coreyt/projects/fathomdb-worktrees/qualification-evidence/slice-90/d27-v2-candidate-2e94aaf4f`.

## Published 0.8.26 comparison

A six-repetition equivalent workload ran against published crates.io
`fathomdb-engine` 0.8.26, with its registry checksum fixed in Cargo.lock.
It used the same 10,000-record corpus (SHA-256
`a4eeb2a7c714d0d450e7ecd41bf7bb45a83f97c30d3adc513629eeca836a685d`),
deterministic 2 ms provider, 10 s warm-up, 60 s measurement, interleaved
directions and operation ratios, and 20-logical-CPU affinity mask as the third
candidate campaign. Projection-heavy median canonical throughput was 36.6958
commits/s and canonical-commit p95 was 4.5476 ms; the third candidate campaign
measured 33.1802 commits/s and 4.8958 ms, respectively. The absolute
differences were 3.5156 commits/s and 0.3482 ms. Published 0.8.26 lacks D27
dispatch-owner and exact managed SQLite-role telemetry, so this is a
comparison rather than a strict D27 receipt or checkpoint gate. The PyPI
0.8.26 wheel and npm 0.8.26 main/native packages were also downloaded from
their registries and passed isolated open/close smokes. All package hashes,
six raw runs, and limits are retained at
`/home/coreyt/projects/fathomdb-worktrees/qualification-evidence/slice-90/registry-0.8.26-equivalent/registry-artifacts.json`
(SHA-256 `23464d1e2d36c1a412d0732a51c3c1115bf6d460094b790c889d385fdd904d0f`).
