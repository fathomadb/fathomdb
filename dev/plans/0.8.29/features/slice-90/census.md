---
title: "Slice 90: source and authority census"
status: PROPOSED
target_release: 0.8.29
slice: 90
---

# Slice 90 source and authority census

Refreshed2026-11-16 against clean integrated source
`3b9e02c151dd7308def6cf199c71902106edd837` in the exclusive90 worktree.
This is source/test inspection and design work; no product build, native import,
database, GPU, model or scientific campaign was run.

## Accepted inputs and actual holds

- [Full80](../slice-80/design.md) has independent Astra-medium PASS and explicit
  owner approval seq384 at `f2f4831554ae165420a31890cf102642ce28b21d`.
  Its actual50 source-fit completion and artifact dependency amendment are
  accepted. Earlier aggregate-v2 drafts do not compete with that full contract.
- Actual50 source is integrated at `9036507bc98ef31c05ae334aa22ec5101323c7e7`:
  record-keyed fusion/FTS/snippets/explanation/evidence, complete-rank winner
  hydration, bounded record-start selector and atomic feedback ambiguity refusal
  are implemented. Current paired50 qualification remains separate.
- 60 local source/acceptance closes at `4005fc07cc5f1f5560910c2603dac88a9e08df21`;
  compact default, direct filters, flags, both-format final accounting and
  discovery defaults are built.70 closes at
  `e875f36042f89c1d84756f78135db0e8926c1ed4` with three reason words,
  Rust facade exports and native parity. These are controls, not90 deltas.
- 20, including all four Windows A6 criteria and assembled238-suite common-source
  qualification, is closed. TC5 selected static input identity is resolved;
  applicable110/140 selected source/healthy/OOM/consumer and100 are closed.
  No missing Windows, TC5-input or private Memex snapshot gate remains for90.
- Current both-format MCP conformance is independently accepted within its
  documented capability/fixture scope; the raw malformed tools/call protocol
  discrepancy remains visible in [conformance](../../../../design/mcp-adapter/conformance-0.8.29.md).
  This component does not close Block1 or claim full JSON-RPC compliance.
- Actual Block1 remains open:30 G strict14 with active-window cases still running,
  M4/46 and D12/24;50 paired ordinary6/136 and diagnostic0/24; conditional40
  actual owner choice; and72 approved native-format model episodes, NOT RUN.
  These are the root snapshot's progress counts, not completed90 evidence or
  a reason to reapprove source contracts. Recheck actual exits at source entry.
- Owner seq388 already approves the72episode smoke/item4 models and bounded
  spend: Haiku5.5 answerer/GPT-4o-mini judges. Fixed dataset/IDs are resolved;
  candidate stores must be rebuilt and technical predecessors must complete.
  Model/prompt/budget availability is not an unanswered policy question.

The [master](../../../plan-0.8.29.md), current release JSON and accepted ADRs
retain authority where historical prose lags. Parent alone updates release
JSON/generated views/ledgers.130E and160 qualification proceed independently;
only their affected final assembled-source rows join after90.

## Code-grounded residual

Rust paths below start `src/rust/crates/fathomdb-`; Python paths start
`src/python/`; TS paths start `src/ts/`. Symbols were inspected on the source
above, not inferred from earlier design or passing test counts.

| Area | Existing implementation | Actual90 work |
| --- | --- | --- |
| MCP omission | mcp/src/output.rs Envelope.state and SearchOutput.verdict are required placeholders; tools.rs constructors fill them. render_orient/search already omit unavailable text rows. | Optional absent unavailable-only fields/schema plus exact orient.not_in_this_release; preserve real readiness/degradation and structured kinds/counts. |
| Arguments | tools.rs parse uses serde deny_unknown_fields; edge.rs discovery owns the schemars input schemas. | Prebuilt schema-derived accepted names and deterministic bounded unknown_argument details before typed parsing/store work. |
| Feedback | session.rs ResponseCursor folds Unique/Ambiguous; tools.rs feedback checks membership before not_recorded branches and ambiguity after acquisition/final epoch. Tests cover atomic mixed labels and compact mode. | Ordered bounded foreign occurrence details at membership only. Preserve all existing later ambiguity/not_recorded behavior. |
| Default | list_stores samples session.default_store once for row flags/hint; routing/use_store/session isolation work. | Required nullable top-level value, quoted first-line default even hints=false and guide sentence. |
| Stored spans | engine/src/vector_spans.rs validate preserves COALESCE invalid predicate/orphan check. projection_commit publishes physical tuple transactionally; format37 retains head storage. | No storage/schema work. Add aggregate arithmetic to the existing full-validation pass, preserving caller-specific errors. |
| Capture/cache | open_locked invokes full span validation before frozen validation. projection_generation.rs physical_completion/cached_status_in_snapshot and its committing generation/mutation callers are real refresh seams. read_projection_status currently has no coverage. | Private staged summary, commit-only writer seed, conservative token invalidation and fixed-work status reuse. Never seed from explained reader connections; no aggregate vector_default access. |
| Record selection | fusion.rs FusionHit/FusionOwnership/key/RecordKey exist. search.rs vector_results precedes vector-only/deferred/ordinary/nested graph branches; explanation maps use RecordKey but are explain-only. | Retain original vector keys only for requested snippets before first consumption; no new ownership carrier, ranking or identity change. |
| Sidecar | snippets_for_hits takes final FusionHits/FtsRows keyed by RecordKey; search.rs attaches snippets before capture.finish/tx.commit. NoEvidenceCapture strips carriers; evidence API stays snippets=None. | Validate/read exact participant extent on that tx and attach scored_bytes after lexical selection. Never infer participation from nested fusion, branch/body/ref or explanation. |
| Rust/native | engine SnippetV1 and ProjectionRuntimeStatus lack new members. py/napi projection.rs and snippet.rs explicitly copy fields.70 facade roots are present. | New aggregate types/reexports and explicit Rust/PyO3/NAPI converters plus scored scalar. No automatic propagation assumption. |
| Python | fathomdb/types.py, _sdk_search._map_native_snippet and read.projection_status explicitly map current fields. | Exact-int frozen aggregate types/decoder and scored_bytes mapping preserving None/0. |
| TypeScript | projection.ts/binding.ts/read.ts and search.ts mapNativeSnippet explicitly map current fields. | Discriminated coverage union/decoder; canonical decimal strings for counts/bytes/fractions only, numeric dimension/scoredBytes with checked bounds. |
| MCP final output | tools.rs search_page/re-snippeting and output.rs Hit/ProjectionStatusOut own positional rendering;60 finish/shaping measures final output. | Lift aggregate once to orient, compact coverage JSON line, optional scored_bytes before pipe; restore trusted metadata after pure re-snippeting and extend schema/census/accounting. |

No source-fit contradiction was found. Per-hit missing/misplaced/extra heads,
ambiguous owner, body mismatch, physical absence, malformed/UTF-8 end and u32
overflow must refuse under approved80; aggregate attribution exclusions must
not introduce those broader physical/open checks. See [design](design.md).

## Affected tests inspected

Full-file reads: MCP schema.rs, stores.rs, feedback.rs and formats.rs; engine
snippet_search.rs, slice60_store_format.rs and snippet_cost_receipt.rs; Python
test_ep17_snippets.py and test_slice22_projection_status.py; TS
ep17-snippets.test.ts and slice22-projection-status.test.ts; PyO3/NAPI snippet modules.
These verify existing selector/UTF-8/point-query/sidecar/routing/schema/ambiguity/
format/budget controls. They lack the new aggregate/scored/diagnostic/default
fields; preserved controls are not claimed as new REDs. The shared pure-selector
parity fixture is a maintained human oracle, not a golden to regenerate.

Other suites (hints/admission, generation/frozen/status and binding error/parity)
remain affected controls located for implementation. Reread any complete file
before prescribing changes to its assertions. [Test map](test-map.md) sequences
semantic failing assertions before production/converter changes. No runtime
pass is claimed from inspection.

Memory read in the real main-repo store includes dont-be-distracted-by-ceremony.md,
agent-access-block1-design-state.md, tdd-red-must-fail-per-finding.md,
slice-prompt-verify-test-claims.md, verify-design-against-code-not-just-architecture.md
and acceptance-md-locked-no-feature-acs.md. Current accepted release/owner
rulings take precedence over those memories' historical status summaries.
