---
title: Slice 135 Phase 2 query correctness readiness — 2026-10-08
status: PREPARATION_NOT_SCORED
target_release: 0.8.27
---

# Phase 2 query correctness readiness — 2026-10-08

The [approved plan](plan.md#phase-2-qualification-and-execution-order) and
[Phase 1 checkpoint](phase1-checkpoint-2026-10-08.md) permit Phase 2 work.
This note preserves the first read-only qualification inventory. No Phase 2
gold protocol is frozen, no new Phase 2 runner or auditor has been written,
and no candidate correctness score has been collected.

## Exact identities and available local inputs

| Input | Verified identity or location | Boundary |
| --- | --- | --- |
| 0.8.26 source | `f99e002f0d2e4002f3694c9f8d4986b56089edaa` | Exact baseline source; package version text is insufficient. |
| 0.8.27 candidate product source | `224e44c593c13d86ece648adabe445723db04070` | Runtime product trees still match this commit at the start of Phase 2 preparation. |
| Baseline installed Python wheel | `7c11803d9629550be60a76e1549c4c0bb42d23978d9fc901cf5df3c463611282` | Local `results/2026-10-08-python-s01-224e-paired/raw-archive/baseline.whl`; native module `269fafc1c696244d05f2eabea19b23cfba6de79f5041b1123fe10589c46c51dc`. |
| Candidate installed Python wheel | `ee8b402f76377956034f900ef69f9e3c0296d79ea30d6728dc1283040266f85a` | Same raw archive's `candidate.whl`; native module `1f213a700cf0c96b9997700d6e44ed7019b26fb91df1259bc6514fefb4a8d013`. |
| Isolated Python environments | `/tmp/slice135-python-venv-baseline-f99e002` and `/tmp/slice135-current-venv-224e44` | Their installed native module hashes matched the respective wheel members on 2026-10-08; recheck before running. Do not repoint the shared worktree `.venv`. |
| Existing deterministic search fixture | `src/python/tests/functional_search_fixture.json`, SHA-256 `3f216e6b4fa1ed2aa8ac44eb67cb07ce38d10437b0603ecd3ca9d9b088d045a2` | Four FTS-only rows and three human-authored expected body lists; suitable for a first independent wrong-ID/order negative control, not a complete query correctness corpus. |
| Resolved IR gold | `/home/coreyt/projects/fathomdb/data/corpus-data/eval/ir_gold/all.gold.json`, SHA-256 `4caabddf7ce55f417e639e3c169fe2035b09c231f36d2f39d293a596373de2bb` | Eval-only outside this worktree; `ir-c-reused-v2`, frozen corpus hash `fe973fcd49fbbda083158f69fe720f17858ab8528e171fa2188eec84131c7d4e`, 4,597 queries: 2,888 exact fact, 1,584 exploratory, 125 negative. Its source mapping and scoring fitness still need qualification for this campaign. |
| LongMemEval oracle | `/home/coreyt/projects/fathomdb/data/corpus-data/raw/longmemeval-cleaned/longmemeval_oracle.json`, SHA-256 `821a2034d219ab45846873dd14c14f12cfe7776e73527a483f9dac095d38620c` | Eval-only; class and evidence mapping remain to be qualified. |
| LOCOMO corpus | `/home/coreyt/projects/fathomdb/data/corpus-data/raw/locomo10.json`, SHA-256 `79fa87e90f04081343b8c8debecb80a9a6842b76a7aa537dc9fdf651ea698ff4` | Non-commercial eval-only payload; keep it and derived verbatim gold outside Git. |
| MuSiQue development corpus | `/home/coreyt/projects/fathomdb/data/corpus-data/raw/musique_dev.jsonl`, SHA-256 `3cff37fd7221506a343a125cf7ca20aab7cd09877e376122da9627e1b935b26f` | Eval-only; derive evidence claims only where the supporting-paragraph mapping is validated. |

The existing `scripts/slice135_python_s01.py` verifies installed wheel/native
bytes, and `scripts/slice135_python_capabilities.py` exercises selected
real-database operations. `scripts/slice135_python_s03.py` has a small
fixture-grounded query pilot. The IR gold loader/validator exists in
`src/rust/crates/fathomdb-engine/tests/support/ir_eval.rs` and the Python
EARP gold-basis module. These are candidates for reuse, not proof that the
new paired Phase 2 protocol or independent scorer is qualified.

## Immediate execution path

1. Specify a small installed-Python baseline/candidate deterministic protocol
   using the existing FTS fixture and fresh databases. Keep expected answers
   out of the product runner; make a separate auditor compare raw observations
   with the authored fixture. Test the auditor RED against wrong ID/order,
   missing case and artifact-identity changes before running it on real data.
2. Add current-schema typed error versus empty-success and reopened-state
   cases using independently specified results. Then freeze the broader gold
   protocol with class denominators, corpus/license qualifications, omission
   rules and independent score recomputation before scored runs.
3. Run exact-f32 fidelity separately from judged relevance, evidence and
   memory classes. Qualify checkpoint/resume, backoff, completeness and the
   approved spending ceiling before any priced answer-quality call.

The [verified Phase 1 raw bundle](results/2026-10-08-raw-retention-review/README.md)
remains local. Preserve it through Phase 2 and transfer it before any
worktree cleanup. Missing performance and platform cells remain outside the
query correctness entry gate under the plan's disposition.
