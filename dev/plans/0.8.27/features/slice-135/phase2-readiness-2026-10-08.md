---
title: Slice 135 Phase 2 query correctness readiness — 2026-10-08
status: VECTOR_FIDELITY_AUDITED_JUDGED_GOLD_OPEN
target_release: 0.8.27
---

# Phase 2 query correctness readiness — 2026-10-08

The [approved plan](plan.md#phase-2-qualification-and-execution-order) and
[Phase 1 checkpoint](phase1-checkpoint-2026-10-08.md) permit Phase 2 work.
This note preserves the initial read-only qualification inventory. The first
small [paired deterministic contract pilot](results/2026-10-08-phase2-python-x1-pilot/README.md)
now has a frozen subset protocol, independently audited raw results and
negative controls. The broader Phase 2 gold protocol and retrieval,
evidence, memory and answer-quality scores remain open.
The [IR gold input qualification](phase2-ir-gold-qualification-2026-10-08.md)
checks the pinned local source mapping and class denominators without
promoting an unscored relevance result.
The next [paired query matrix](results/2026-10-08-phase2-python-matrix/README.md)
now adds exact filter, empty-success, pagination, typed-refusal and reopen
oracles on the same installed Python identities.
The [evidence and memory input qualification](phase2-evidence-memory-gold-qualification-2026-10-08.md)
records eligible source mappings and class limits without promoting an
unscored product result.
The [paired graph, evidence and erasure contract](results/2026-10-08-phase2-graph-evidence/README.md)
adds raw-table erasure and reopened graph witnesses for both installed Python
identities.
Its changed runner and tests passed the [full workspace gate](results/2026-10-08-phase2-graph-evidence/verification.stdout):
186/186 suites, no skipped or excluded suite, and zero security blockers.

## Vector fidelity feasibility finding (unscored)

A candidate-only local probe used the pinned default BGE model to embed the
canonical stored JSON bodies and compute brute-force squared-distance neighbors
over the returned vectors. The exploratory distance sums used Python floats;
the scored oracle still needs explicit f32 arithmetic and a tie rule.
On the 32- and 256-row S01 corpus, the probe's zero-text-hit queries returned
only `vector` branch hits and matched exact top-10 IDs. A second probe used 64
nonempty rows each from the locally cached daily-log, to-do, Enron and
CNN/DailyMail corpora (256 total). Of 12 exploratory natural-language queries,
11 had text-search hits and therefore exercised a hybrid path that cannot be
judged against a vector-only oracle. The remaining zero-text-hit query
returned only `vector` hits and matched exact top-10 IDs. These observations
establish route feasibility, not paired recall or a general fidelity score.
The probes used temporary databases and are not frozen scored evidence.

The scored fidelity protocol must bind the same-model body and query
embeddings, exact-f32 ranking and tie rule, top-10 denominator, real-corpus
selection and source hashes, and per-query proof that `search_text_only`
returns no hits and every observed hit is from `vector`. If that public-SDK
qualification cannot support a representative query set, use a separate
pre-fusion vector-stage measurement seam and label any public hybrid output
as a distinct diagnostic. Do not compare a mixed text/vector result to the
exact vector oracle.

The [revised frozen vector fidelity protocol](phase2-vector-fidelity-protocol-v2.json)
uses 1,000 locally cached real documents and 100 title-or-lead queries,
25 from each of four source classes. Its installed-wheel measurement copy
removes the lexical index rows after projection, verifies that canonical and
vector storage bytes are preserved, and requires zero text-only hits after
reopen. It selects globally unique canonical bodies and requires exact-f32
rank monotonicity, so one-arm fusion preserves vector-stage ordering while
retaining the pinned
product wheel and embedder. Source text, queries, vectors and database copies
remain outside Git. A separate 32-row paired pilot exercised the complete
runner and independent audit before freezing; malformed query, branch, ID,
wheel, pairing and persisted-body copies were rejected, while plausible
wrong vector hits lowered the recomputed score. That pilot is qualification
only and is not the 100-query fidelity result. The indexed SQLite copy is
intentionally modified for measurement, so this cell must be reported as a
vector-stage fidelity test seam, not ordinary hybrid search behavior.
The [first frozen protocol](phase2-vector-fidelity-protocol.json) was executed
but its result is an invalid fidelity attempt: two duplicate bodies received
extra same-arm RRF weight and caused three rank inversions. Its raw pair and
diagnostic audit are retained locally; the observed `0.945` means are not
promoted as a Phase 2 fidelity verdict. The v2 protocol was frozen after
that diagnosis and before any corrected scored run.

The corrected [paired vector-stage result](results/2026-10-08-phase2-vector-fidelity-v2/README.md)
has now passed independent audit over 1,000 unique bodies and 100 queries:
baseline/candidate exact-neighbor recall@10 is 0.933/0.931. Candidate scores
worse on 14 queries, better on 11 and equally on 75; the paired bootstrap
interval includes zero. The [per-case diagnosis](results/2026-10-08-phase2-vector-fidelity-v2/diagnosis.json)
records all missed exact ranks. Same-model f32 document/query vectors match
between versions, but pinned mean vectors and vec0 layouts differ, so this
pair does not isolate a version-specific ANN loss. The measurement is a
vector-stage seam; public hybrid relevance remains unscored.

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
| MuSiQue development corpus | `/home/coreyt/projects/fathomdb/data/corpus-data/raw/musique_dev.jsonl`, SHA-256 `3cff37fd7221506a343a125cf7ca20aab7cd09877e376122da9627e1b935b26f` | Eval-only; the acquisition manifest has a different historical hash. Pin these local bytes and use only qualified supporting-paragraph sets. |

The existing `scripts/slice135_python_s01.py` verifies installed wheel/native
bytes, and `scripts/slice135_python_capabilities.py` exercises selected
real-database operations. `scripts/slice135_python_s03.py` has a small
fixture-grounded query pilot. The IR gold loader/validator exists in
`src/rust/crates/fathomdb-engine/tests/support/ir_eval.rs` and the Python
EARP gold-basis module. These are candidates for reuse, not proof that the
new paired Phase 2 protocol or independent scorer is qualified.

## Immediate execution path

1. Freeze and negatively qualify the judged relevance, evidence and memory
   protocols using the already qualified local source mappings. Score each
   class separately with explicit denominators and omissions.
2. Qualify checkpoint/resume, backoff, completeness and the approved spending
   ceiling before any priced paired answer-quality call. Add TypeScript, Rust
   or fault cells when a specific scored claim depends on them.
3. Treat the paired vector-stage difference as a diagnostic until a
   controlled mean-state or repeated-pair study separates runtime index
   variation from a version effect.

The [verified Phase 1 raw bundle](results/2026-10-08-raw-retention-review/README.md)
remains local. Preserve it through Phase 2 and transfer it before any
worktree cleanup. Missing performance and platform cells remain outside the
query correctness entry gate under the plan's disposition.
