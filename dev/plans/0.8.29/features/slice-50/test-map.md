---
title: Slice 50 source-grounded test map and runtime TDD proposal
status: PROPOSED
target_release: 0.8.29
slice: 50
---

# Slice 50 test map and runtime TDD proposal

No test was executed in this documentation phase. Existing tests are inspection
evidence, not new REDs or qualification. Expected outcomes below now follow
owner seq-374 identity/packing and seq-375 atomic-feedback decisions; a passing-on-arrival invariant is retained
coverage, not a fabricated behavior change.

## Existing tests inspected in full

Paths are under `src/rust/crates/fathomdb-engine/tests/` unless qualified.

| Test file | Actual assertions and relevance | Limit / required refinement |
| --- | --- | --- |
| `pr_g9_rrf_fusion.rs` | Manual RRF constants/ranks, agreement, tie preference, deterministic real-DB result/fallback, three-arm arithmetic and graph growth. | Bodies are intentional identity today. Graph test accepts Carol OR result-count growth and does not assert carol_without=false. New identity oracle must name the exact record/path; do not pretend old test proves it. |
| `pr_g1_search_hits.rs` | Hit fields, vector representative, public logical/content identities and cross-DB hash stability, default superseded exclusion. Body dedup test uses distinct bodies and same-record multiarm overlap. | Its shape and IdSpace assertions remain; the fixture alone does not distinguish same-record dedup from distinct equal-body records. Header's old id==cursor wording is stale relative to actual assertions. |
| `slice25_registration_identity_inert.rs` | Registration/boot/drop do not change stored logical IDs or derived spaces; real mutation negative control; different-body anonymous/governed twins both surface; identical-body twins collapse with earlier anonymous representative; supersession keeps logical ID. | Identical-body collapse is deliberate human intent. Change only its retrieval corollary after accepted successor; retain no-retro-mint, both-at-rest, lifecycle and negative-control assertions. |
| `slice71_search_statement_trace.rs` | No per-hit text post-filter source statements; >200 text-rank overlap is verified by SQL; hybrid deferred identity equals forced full-sort at limit100, includes duplicate-body rows and ownerless h:/None source, witnesses both routes. | Already passes baseline by design; cannot be claimed as RED for deferred-body work. It does not count hydrated body bytes or prove per-record duplicate semantics. |
| `slice90_filtered_body_fts.rs` | >100 stronger nonmatching node candidates; selected target survives filter pre-cap; AND attributes exclude edges; node/edge metadata source rules; empty-filter parity; fixed-view prefix stability; each relax flag; validation on empty lexical result; inclusive projected time/empty status sentinel and missing metadata. | Preserve current engine contract. New equal-body identity assertions need exact cursors/source, not body sets. No MCP forwarding/default-format proof lives here. |
| `exp_obs_explain.rs` | Explain on/off leaves results/cursor/fallback identical; per-hit cursor/arm/score/CE fidelity and arm presence; trace fields; depth>0 and graph configurations. | Does not give an adversarial equal-body distinct-record rank oracle. Extend after ruling with manual ranks tied to each record. |
| `snippet_selector.rs` | All selector rules, clamping/rejection/defaults, exact example output, invalid spans, NFD widening, head/merge/word/sentence behavior, emoji/ZWJ/RI safety, Unicode proptest invariants and retained regressions. | General properties verify invariants, not optimized/full differential work bounds. Random options cover40..700, so new differential must also exercise maximum4000 and boundaries explicitly. |
| `snippet_search.rs` | Real DB text/hybrid/frozen sidecars, invalid options, far matches/Porter/NFC/NFD, exact table/byte spans, marker fallback, Vector branch lexical anchors, off invariance, token unchanged, read-only SQL positive control, cap20 and all-or-nothing per-term pool (38 queries in3-term case). | Preserve same run-window semantics. Add record-keyed lookup in new duplicate fixtures; its body-based snippet_of helper is ambiguous for those fixtures. |
| SDK `tests/slice41_context_pack.rs` | Full file read: helper keeps prefixed l: ID, decodes tagged/untagged parts, preserves metadata, malformed bodies refuse; fixture pack_errors test pins repeated-ID InvalidArgument, including context[0].id across lists. | No guarantee that a list of helper results has unique IDs. New record fusion exposes h: collisions; historical l: collisions already exist. Preserve helper/pack contracts unless separately ruled. |
| MCP `tests/feedback.rs` | Correct unique-ref cursor labels, concurrent query attribution, unknown/wrong-store/expired/reopened handling, text/telemetry-off omission and no effect on later output. | No distinct-cursor repeated-ref oracle. Actual guard/ref/fetch trace is in feedback-decision.md. |

Other candidate suites (graph history, evidence/frozen authentication, installed
binding snippet parity and performance) were located but not exhaustively read
in this phase. Re-read their entire files at actual source entry before making
assertion claims or changing them. `tests/fetch_expand.rs` was inspected through
its view-parity case, not fully; no whole-file gap claim is made.

## Genuine runtime RED → narrow GREEN units

Each changed behavior gets its own committed/staged test, observed failure at
that SHA, and separate minimal implementation commit. Record the actual failing
assertion/command in the RED commit body; use actual focused results, without a
separate RED-SHA audit phase. Missing symbols, syntax/import failures, env/setup errors, mocked SQL and
manufactured wrong goldens do not establish the intended runtime failure. Stage
I1/I2 selected-record assertions, including equal stored tagged-body twins, before
the shared identity GREEN; a case already fixed by that GREEN remains coverage
and must not be presented later as an independently observed RED.

| Unit → requirement | Runtime RED proposed using already callable behavior | Minimal GREEN and validation |
| --- | --- | --- |
| I1/I2 → R1/R2 | Baseline-callable real Engine fixtures in `tests/slice50_record_identity.rs` keep equal-body records distinct and explained ranks/provenance attached to exact cursors. Baseline collapses eligible twins; the graph fixture independently names the reached node suppressed by an unrelated vector body twin. | Core carrier/reference GREEN and explicit helper signature migration form one identity behavior unit. Existing pure arithmetic/repeat/tie tests migrate mechanically without changing their numeric oracles; new keyed pure tests are controls for this same unit, never claimed as a later independent RED. Same-record repeat credit preserves the first original positional rank. Missing-symbol failures do not qualify. |
| I2 → R1/R2 | Real SQLite write anonymous/governed and distinct-governed equal-body twins, node/edge same body, two historical same-body revisions and byte-identical stored role-tagged twins from distinct sources. Stage these semantic assertions before the identity fix. Assert selected records/source and unchanged IdSpace. Baseline collapses eligible twins. | Carry source-derived ownership through node/edge/vector/historical fusion. Retain default visibility and at-rest identities. Ownerless rows use exact FTS locator; malformed ownership has explicit oracle after ADR choice. |
| I3 → R2 | If selected-key fusion lands before consumers, equal-body records have different per-arm ranks and sources; explained output must attach each manual rank/score to its own cursor. Graph fixture has an unrelated equal-body node in fused results; exact reachable node/path must remain. Baseline consumer maps/exclusions produce wrong rank or suppress node. | Migrate key maps/exclusion and provenance carrier one seam at a time. Preserve logical BFS, first winning edge, nested fusion, recency/importance/CE and evidence authority. If earlier GREEN already fixes a case, keep it as coverage; do not label it a new RED. |
| F1 → R2/R7 | Under selected option1, telemetry-enabled real hybrid response contains repeated ref with distinct cursors; named feedback must be typed bad_argument with no label. Current code records the last mapped cursor. | Bounded Unique(cursor)/Ambiguous response map; ambiguity check only after successful acquisition and final epoch recheck, before cursor-label resolution and bridge/feedback writes. Unique/same-cursor, unknown, epoch, disabled telemetry/text, mixed named labels and both formats are controls. Ambiguous named refs with concurrent unready/close or reopen after ready_epoch must retain acquisition-time/post-acquisition not_recorded and zero writes, not early bad_argument. Richer per-result labels require .30 contract review; the atomic fix remains .29. |
| T1 → R3/R6 | Already-public hybrid query over many long matches, fixed deterministic embedder, no graph/filter/CE/explain/dependencies. Freeze setup, then a serial test allocator counts large Rust allocations during the real search. Assert a derived bound proportional to vector hydrated pool+final hits plus lightweight rows, not matched body bytes. Baseline hydrates all M bodies and fails the work bound; selected hits still match the complete control. | Complete lightweight rank census and final hydration. Do not claim old route-witness equality as new RED. Test allocator observes actual allocations across worker threads, not an implementation-populated boolean; validate measurement scope with a deliberate large allocation/control outside product code. |
| T2 → R3/R4 | Only if widening admission changes behavior: construct the newly admitted combination with a real DB and assert route/work bound plus hand-selected filtered/frozen results. Current reference route violates that intentional work requirement. | Widen one combination only when reviewed. Otherwise these combinations remain reference controls, with no invented RED for unchanged fallback. |
| P1 → R5/R6 | Existing `snippet_for` on a preallocated multi-MiB ASCII alnum/space body with empty spans; isolate allocator interval around invocation. Assert output against the full reference outside measured interval and bounded allocation independent of suffix length. Current whole-body boundary vectors violate the derived allocation bound. | Restricted-prefix specialization; count only actual call allocations. Repeat with suffix lengths spanning orders of magnitude; direct work counter/allocator bounds, not flaky timing. |
| P2 → R5 | Conditional on any broader specialization: a minimized Unicode/context counterexample demonstrates real output difference or unbounded work. | Extend proof/algorithm only for that class; otherwise retain reference fallback. An already-correct Unicode equality test is regression coverage, not RED. |

For T1, choose long bodies large enough to distinguish body allocations from
lightweight rank vectors, e.g.64KiB and M far larger than the fixed vector pool.
Derive the bound from actual permitted live body copies/candidate constants
before running; do not tune it to a measured pass or assert an arbitrary new
latency budget. Quiesce projection work and scope allocation accounting across
reader workers in a dedicated serial integration process. The allocator counts
are an allocation witness, not an exact SQLite body-read or I/O count. If that
witness cannot separate body copies, design the minimal opt-in observer and its
own genuine tests first;20's limited seq-369 exception is no waiver here.

## Concrete O3-09 tagged and context-pack fixtures

Stage these alongside I2 **before** changing identity. Their fixed role encoding
and byte expectations come from the existing human-approved convention; no codec
or fixture/golden regeneration is proposed.

- **TWIN-TAG (identity RED):** two anonymous active node writes from sources A/B
  contain exactly `\u001e\u0011café pear` (escape notation; stored bytes begin
  0x1e,0x11). Search `cafe`, limit sufficient for both, highlights=true,
  max_bytes=40. Under selected record identity assert two exact cursors/sources
  and unchanged equal h: IDs, with original text rank1/2 manual RRF scores and
  tie order. Baseline body fusion returns one, a genuine selected-record-count
  failure. Both stored bodies remain the exact12 UTF-8 bytes; decoded role is
  user. Each whole snippet text is those stored bytes, window [(0,12)], body
  span [(2,7)] and snippet highlight [(2,7)], Whole and cut=false. This proves
  marker bytes are included in coordinates, not stripped before selection.
- **TAG-ROLE (unchanged control):** anonymous user-tagged `\u001e\u0011café pear`
  and assistant-tagged `\u001e\u0012café pear` have equal decoded plain text,
  differing roles/stored bytes and distinct h: hashes. Both already survive body
  fusion; preserve two selected records/sources and each exact stored body and
  [(0,12)]/[(2,7)] coordinates. Do not claim a fresh RED merely because the
  successor changes its key; never merge by decoded text or normalized embedding.
- Add long stored tagged bodies with a multibyte prefix and a query match in a
  later part; independently calculate expected term byte offsets from the
  handwritten stored prefix (including newline+marker+role code). Compare full
  reference snippet fields, verify windows slice the stored bytes and highlights
  map the shown spans. These are unchanged Unicode/coordinate controls unless a
  real new specialization/work assertion is demonstrated to fail.
- **HELPER-PACK (preserved contract):** after the selected two-result TWIN-TAG
  search, the unchanged helper returns two identical item IDs and matching user
  parts. pack_context must give typed InvalidArgument naming the repeated item
  field; do not weaken it to make search→pack universally succeed. Historical
  different-body revisions sharing l: separately pin the already-existing case.
  A direct duplicate-item fixture and duplicate across hits/context can be
  staged and pass before identity GREEN; these are regression controls. The
  TWIN-TAG selected-count assertion, not unchanged pack rejection, is the RED.
- **CALLER-LABEL (existing supported control):** construct equivalent ContextItems
  with distinct caller labels A/B and a separate label→selected-record map;
  packing accepts both under ample budget, retains roles/order/provenance and
  full_ids resolves those labels. This asserts caller-owned packing labels,
  not automatic helper labels, new IdSpace or a stable occurrence ABI. Preserve
  character-based pack counts separately from byte-based snippet coordinates.

Run these through Rust SDK, then fresh installed Python/N-API equivalents on
the integrated candidate. Preserve codecs and their current rejection/property
fixtures. Owner seq-374 selects unchanged helper/pack behavior, including typed
refusal for repeated IDs; no automatic relabeling or duplicate acceptance follows.

## Required GREEN/regression coverage

- Independently specified eligible record tuples for flags individually/together,
  validity endpoints, node/edge predicates, source-dependency fallback, undeclared
  attributes on no-match and more excluded records than each cap. No top-K
  post-filter or new metadata hydration to make status/time tests succeed.
- Full versus optimized ordered fields **and score bits**, deep rank>200,
  raw score ties crossing boundaries, distinct same-body sources, empty arms,
  same-record repeats and fixed direct-text100 node inputs followed by edges.
  Use hand-computed overlap arithmetic as well as a shared-code differential.
- Freeze one time/view and same read snapshot; exercise ordinary/explained/
  historical/frozen and reference-forced routes. Concurrent commit/rollback,
  external visibility mutation and equal-generation divergent DB authority use
  existing frozen contracts, never generation-only validation.
- Retain full selector and property-test exact SnippetV1 equality for arbitrary
  Unicode bodies, raw invalid/valid spans, every option boundary, ASCII-prefix
  arbitrary Unicode suffix, very long words/spaces, combining/Format/ZWJ/RI,
  punctuation, CRLF and whitespace. Fast admission and fallback are witnessed;
  no full-body scan is hidden in an admission predicate.
- Real node/edge snippets retain actual FTS rowid, match anchor independent of
  winning branch, two query pools, highlights and byte budgets. Stored vector
  spans/hints remain unchanged;80/90 own public coverage composition.
- Fresh installed Rust/Python/N-API controls compare same-build selected IDs,
  source/score/explanation and UTF-8 sidecars. MCP both formats follow approved60
  rounding/whole-hit budget/paging; do not recut snippets. Addressing policy is
  separately ruled; logical fetch still returns latest eligible per its view.

## Remaining measurement-support TDD

The accepted50 correctness/identity/prefix suites remain completed evidence.
Only the genuine [paired support gaps](design.md#current-paired-measurement-support)
need new RED/GREEN, with independent Astra design review before implementation:

| Unit | Falsifying behavior and narrow GREEN |
| --- | --- |
| Pair selector and diagnostic work | On a tiny real store with independently specified lexical/vector bodies, the new private selector must choose retained eager-body/deferred-identity rather than winner hydration, preserving exact ordered hits/score bits and direct-text routing. Stage runtime assertions before implementation; use a callable scaffold if needed, never a missing symbol as RED. Separately assert literal actual lightweight/body counts and UTF-8 byte sums for both modes, including a multibyte body and repeated materialization, under the existing bounded engine/operation observer. Absent/lost facts cannot appear as zero. General test-hooks are excluded from ordinary pair builds. |
| Existing runner pair binding/output | Existing tiny real-DB/harmless-child controls assert fixed cell populations and balanced mode order, distinct fresh write subjects, same binary/features/provider inputs, no collector in ordinary calls, separate diagnostics/RSS and retained failed/refused members. Extend existing runner tests rather than add a framework. Pin cursor/source/score-bit output with literal expected records and hand-computed metric math; do not regenerate snapshots or repeat previously passing50 tests as new REDs. |

Pure source/harmless-child controls can proceed in the isolated checkout after
design acceptance. Any Rust build/test, current-native/default/model/GPU run and
natural measurement requires the parent's native window. Final independent
review covers the coherent support change; actual candidate-bound paired results
remain a separate qualification obligation. Run required scoped checks and the
final source gate once when the native window permits, preserving genuine
failures and affected resumes.

Only after actual30 is available, repeat its fixed natural-text and write-
interleaved cells with the qualified50 candidate/reference; untraced latency
and separate work/RSS diagnostics identify50's contribution. No performance
run, full gate, model, install, source change or native execution occurred here.

No private Memex snapshot is needed for these correctness tests. Use synthetic
real-SQLite populations covering the selected identity cases and properties;
source/consumer census is reconciled at actual entry. Population frequency and
benefit remain unknown; actual30 baseline prerequisites and same-input performance
comparison remain required. No migration fixture is added: format37 is unchanged.
