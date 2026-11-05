# Current/history search POC outcomes

Status: COMPLETE — Astra reviewed; independently verified.
Qualified implementation: `4b6bc10531a28cfc415a5d038f7dc64ee9909090`.
Experiment date: 2026-11-02. Base: Slice 80 at `dd8dbf6f2`.

The POC supports the correctness of an isolated current-search projection,
explicit corrected historical snapshots and prior-belief snapshots. Excluded
superseded history does not alter default scores or ordering. It also disproves
that a current-only partition is sufficient across clock-only validity changes.
The certified fallback restores correctness. Total query latency is still
history-sensitive in this prototype; no production integration or performance
qualification is claimed.

## Hypotheses and assumptions

| ID | Outcome | Evidence and limit |
| --- | --- | --- |
| H1 | Supported within the POC | Current node, edge and property identities and scores are exactly equal at 0/4/16 excluded revisions; direct tests cover top-K and graph seed/result slots |
| H2 | Supported | Late world change preserves earlier truth; partial correction produces A/B/A; prior knowledge preserves A; nested retraction never revives the original error |
| H3 | Supported | Noncoexisting edges cannot form a snapshot path; simultaneous edges can; endpoints resolve under the same knowledge/valid time; archive traversal is refused |
| H4 | Supported for the stated grammar | Three rankers agree across current, corrected/past knowledge and archive views; maximum measured score delta is 2.220446049250313e-16, below the predetermined 1e-12 tolerance |
| H5 | Unconditional partition assumption REJECTED; fallback supported | A clock-only boundary removes an old record and admits a future one without a write; naive current-index search is wrong; exact membership certification routes to selected-corpus statistics |
| H6 | Supported for logical SQLite contents | Fault rolls back canonical/history/current/knowledge changes; admission, source denial, rebuild, erasure and reopen pass; 24 generated interval cases preserve nonoverlap and round trips |
| H7 | Costs observed; zero-overhead inference rejected | Current FTS size stays fixed and preselected ranking stays roughly flat, but selection/certification, storage and total query cost grow with history |
| H8 | Supported for held SQLite snapshots | Real second connection commits replacement and source denial; the held reader completes its prewrite view; the next request observes denial |

Validated boundary assumptions include half-open valid/knowledge intervals,
strictly increasing knowledge ticks, immutable claim source/type/endpoints,
Initial only for a new claim, and structural Revision preserving one accepted
fragment interval. A Revision may be the first ingest of an externally revised
record. A structural revision cannot fill a retracted gap.

Admission and source allowed/active policy are current as of the held read
snapshot, across every historical view. Source validity follows snapshot V;
archive source validity uses the explicit experiment authority instant in
`config.now`. Prior-belief reconstruction does not reconstruct former access
rights. A reader already holding a pre-denial snapshot can finish; subsequent
requests cannot bypass denial by selecting old knowledge.

Unproved assumptions remain explicit: the caller supplies the correct claim
identity, intent, effective interval and source authority. Conflicting sources
retain separate claims; the POC does not infer which source is true. No language
model, contradiction detector or natural-language truth oracle is involved.
The supported query grammar is normalized ASCII words joined by OR, with actual
SQLite Porter normalization. Full production phrase/prefix/NEAR/Unicode query
compiler parity, dense fusion/reranking, engine evidence/cursor integration,
physical WAL erasure, memory zeroization and reader-pool cancellation are not
qualified by this isolated experiment.

## Fixed measurement protocol and observations

[results.json](results.json) retains all 63 cells: 54 current cells
(2 sizes x 3 history counts x 3 projection families x 3 backends), plus nine
corrected-history/prior-belief/clock-fallback cells. Each has 15 measured calls,
a warmup, top-10 identity and reference scores, maximum score delta and route.
The backend order rotates each sample. No sampled cell was discarded.

The axis N means **N node claims plus N edge claims**, or 2N canonical claim
identities. Nodes have two scalar properties, giving 2N property documents.
Current data and queries are held constant as excluded revisions grow. The
history strings are longer than current strings. Source validity has a known
future boundary; the diagnostic clock workload crosses it without a write.

Environment: x86_64 Linux, AMD Ryzen Threadripper PRO 5945WX, Rust 1.95.0,
optimized Cargo release build. Pinned rusqlite 0.41.1 / libsqlite3-sys 0.39.1
match the root lockfile versions and checksums. Runtime SQLite is 3.54.1;
its exact source ID is in the JSON. The synthetic serial workload is warm;
OS caches are not flushed and no CPU/service isolation is claimed.

Every end-to-end time includes selection, certification or statistics scans,
private corpus build where applicable, ranking, result assembly/hydration and
private connection drop. The selected canonical bodies are loaded before
ranking in this prototype; this is not the engine's bounded winner hydration.
Nearest-rank p95 with 15 samples is the **maximum** measured sample.

At N=512, node-body current queries, milliseconds (p50 / p95):

| Excluded revisions per claim | Partitioned, end to end | Custom statistics | Private selected corpus |
| ---: | ---: | ---: | ---: |
| 0 | 1.141 / 1.870 | 1.750 / 1.770 | 1.746 / 1.917 |
| 4 | 2.658 / 3.202 | 14.023 / 15.029 | 3.233 / 4.395 |
| 16 | 7.825 / 8.149 | 55.400 / 58.563 | 8.619 / 8.970 |

The JSON's `native_rank_only_attribution` is **preselected native ranking and
result assembly**, including by-ID map construction, matching row iteration,
Hit cloning and Rust sorting/truncation. It excludes selection/certification;
it is not pure SQLite function timing. Its node-body p50 is 0.283, 0.279 and
0.286 ms respectively. This isolates the current FTS corpus benefit, but does
not remove the prototype's history-dependent selection work.

At N=512 and 16 revisions, property end-to-end p50 is 9.418 ms partitioned,
73.351 ms custom and 10.639 ms private materialization. All projection families
have exact current reference-score/order invariance across the history axis.
The all-vocabulary custom scorer is a correctness comparator; it scans excluded
history to recover selected frequencies. It is not an optimized production
recommendation.

Diagnostic node-body p50 at N=128 / four excluded revisions:

| View | Partitioned (custom fallback) | Custom | Private selected corpus |
| --- | ---: | ---: | ---: |
| Corrected history | 3.633 ms | 3.614 ms | 0.863 ms |
| Prior belief | 3.635 ms | 3.615 ms | 0.835 ms |
| Clock boundary | 3.505 ms | 3.494 ms | 0.696 ms |

Private materialization is faster than this prototype's full-vocabulary custom
fallback on these workloads. This supports retaining it as an explicit opt-in
mechanism/reference, with its documented memory and reader-lifetime costs;
it does not establish production-scale capacity or latency guarantees.

At N=512, combined database bytes are 765,952 (no history), 6,471,680 (four
revisions) and 24,731,648 (16 revisions). Seed time is 0.893, 4.991 and 18.127 s.
These totals include canonical data and all coexisting current/history indexes.
They are **not per-backend storage/write overhead comparisons**. Sixteen
revisions entail 17 times as many writes, and historical payloads are longer.
Current row counts remain 512 nodes, 512 edges and 1,024 properties in all cells.

## Verification and RED/GREEN evidence

Independent execution passed `cargo test --locked` (21 tests, including the
24 generated interval cases), `cargo fmt --check` and clippy all targets with
warnings denied. Independent JSON validation confirmed unique cells, counts,
all routes, sample counts and numerical parity. Astra approved the design,
resolved findings, implementation and measured interpretation.

| Increment | Behavioral RED | GREEN |
| --- | --- | --- |
| Current corpus isolation | `f3ddf8c68` | `0e103b970` |
| Correction/world-change/prior belief | `cb23f881e` | `128cd153c` |
| Rankers and clock fallback | `6e1abc6a7` | `c8b2a18ea` |
| Coherent graph and seed bounds | `2dd8e4b4e` | `17ea63f03` |
| Lifecycle and atomicity | `6cf4d5db8` | `4d8fc7e95` |
| Four Astra semantic findings | `5a65ad54c` | `a83b014e8` |
| Measurement correctness | `789210968` | `4b6bc1053` |

An independent agent reconstructed three RED commits outside the checkout and
reproduced their behavioral failures: initial matches absent, partial overlay
unsupported, and the four concrete review findings. All compiled successfully.
No test oracle was weakened to make a fix pass. Scoped POC checks are not a
claim of a new full production-workspace regression pass.

Logs: `/tmp/history-poc-red-*.log`, `/tmp/history-poc-green-*.log`,
`/tmp/history-poc-clippy.log`, `/tmp/history-poc-sweep.log`,
`/tmp/current-history-red-*.log`, `/tmp/current-history-independent-results.log`.
The sources, tests, lockfile and measurement driver are retained in this folder.

## Production design and remaining qualification

The [revised production design](../../design/current-history-search.md) replaces
the earlier implementation outline. Its five-change table records each POC
finding, the applied design decision and its missing evidence. Three functional
aspects are demonstrated only in the isolated POC: default ranking isolation,
update/correction and prior-belief semantics, and coherent historical graphs.
None is yet integrated into the production engine.

| Existing gate | Required measurement | POC status |
| --- | --- | --- |
| AC-076 / T-012 | Prescribed binding 10,000-row fixture, >=1,000 samples; p50 <=20 ms and p99 <=150 ms | UNQUALIFIED; small 15-sample POC distributions are not a pass |
| AC-081a | Unchanged 1,600-search sequential batch <=500 ms | NOT TESTED for the candidate |
| AC-081b | Unchanged eight-reader 1,600-search batch <=100 ms | NOT TESTED for the candidate |
| AC-081c | Distinct real engine reader progresses while its peer holds a snapshot | NOT TESTED for the candidate; H8 checks a different property |
| AC-072/073/075 | Existing affected vector latency, real-corpus stress and vector fidelity verdicts | Not qualified by this lexical POC |

Thresholds and prescribed fixtures remain owned by
[acceptance.md](../../acceptance.md). The 7.83-ms small-corpus node p50 is below
the AC-076 p50 threshold numerically, but the corpus, path and sample protocol
do not establish acceptance. The preselected 0.28-ms attribution is not an
end-to-end AC measurement. Latency acceptability is UNQUALIFIED, not PASS or an
inferred FAIL from an unlike fixture.

Before main production implementation, [three bounded follow-ups are proposed](next-poc-plan.md):
P1 maps temporal assertions to real canonical/identity/lifecycle constraints;
P2 proves current-index-first certification and writer refresh;
P3 selects and bounds the historical backend. They have not been executed.
After candidate integration, the actual prescribed engine acceptance runs are
required. No existing AC is weakened, replaced or marked passed here.
