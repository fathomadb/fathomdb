---
title: 0.8.29 — final 0.8.28 forward integration
status: IN_PROGRESS
target_release: 0.8.29
---

# Final 0.8.28 forward integration

Owner-directed handoff under the [release plan](../plan-0.8.29.md), completing
0.8.28 Slice 95's existing handover obligation. Inputs: .29 `d29dcced95` and
initial .28 source C `5f668f33056d0a550c51810f6ebeeb1ef7d60695`; merge base
`59079d5c`. Mechanical checkpoint `73eb608eb` first integrates .28 `16234813`;
the source C merge adds the final documentation and CI9/CI10 fixture corrections.
Source D `bef17bfab777e3ea3ad90b66ed122795dfbc0969` merged without conflict
at `6674cf79d`, adding CI11 discovery diagnostics, mandatory UTF-8 coverage and
the exact deferred timing disposition. The current forward input is source E
`86fb2924c3f10a6b0d4ff04fbd00b8ac7c67bf3d`, merging CI12's accepted bounded
logger-admission repair and its reviewed documentation without conflict.
`logging_subscriber.rs` is byte-identical to E. Existing .29 signatures, MCP,
filter/snippet contracts and frozen oracles remain intact. Source C/D/E remain
immutable. Astra approved the bounded integration and the explicit superseded
MCP expectation adjustment before resolution.

## Combined contracts

- Preserve .28 format36, built-in-provider failed-open semantics, historical
  visibility/filtering and maintenance ownership, including the CI8 worker fix.
  An actual vector-check refusal returns no Engine; never restore removed
  dense-disabled methods or add a constant-false compatibility facade.
- Preserve .29 snippets, degradation sidecars, hints and existing MCP tools.
  A requested snippet does not change ranked hits or eligibility. Filters run
  on the reader snapshot before candidate bounds/deduplication/ranking/limit.
- Python public text search: `search_text_only(query, view=None, *, limit=10,
  filter=None, snippet=None)`. Native filter fields and snippet are keyword-only
  after the existing query/view/limit arguments. Type stub and wrapper agree.
- TS public text search keeps its options object with both filter and snippet;
  native order is `searchTextOnly(query, view?, limit?, filter?, snippet?)`.
- Rust `TextSearchOptions` combines filter/view/limit/snippet with defaults
  None/default/10/None and remains non-exhaustive. Existing named filtered and
  snippet core methods keep their signatures and delegate to
  `search_text_only_filtered_with_snippets(query, filter, view, limit, snippet)`.
- Retain full operation guards through snippet/frozen search responses and final
  observability. Pure snippets/hints remain stateless. Keep actual FTS rowids
  associated with their hit cursors, and preserve both degradation/snippet
  response sidecars. Historical visibility axes compose with snippets.

The 17 marked conflicts span interfaces/ADR/root records, Python boundary and
stubs, engine target registration/filter/search paths, Rust SDK and native/TS
bindings. Resolve by composing those contracts. Review clean merges in reader
requests/results, embedding availability, root composition/close hooks, manifests,
patches, parity expectations and generated inventories as well as marked hunks.
Frozen surface, acceptance, snippet and hints baseline bytes remain unchanged;
only explicit already-approved release deltas are combined in their comparators.

## Necessary MCP policy alignment

The existing MCP code calls removed SDK dense-disabled methods and one old test
expects an attached refused Engine. That expectation is superseded by owner
seq-359. Remove live attached-dense-state orientation/search assumptions and its
orient field, rather than silently substituting a no-provider Engine. Existing
pure hints facts and their controls remain independent of live Engine admission.

The accepted [seq-359 admission ADR](../../adr/ADR-0.8.28-vector-compatibility-open-policy.md)
and the existing partial-supersession notice in the
[vector-equivalence ADR](../../adr/ADR-0.8.18-vector-equivalence-self-check.md)
replace its former R-VEQ-4/6 degraded-open contract. The accepted
[.29 hints ADR](../../adr/ADR-0.8.29-agent-hints-sidecar.md) does not require an
attached dense-disabled Engine or orient field; its pure facts remain valid.
No new decision or provider compatibility shim is needed for this alignment.

Reconcile the non-frozen test using its real corrupted-reference fixture: the
configured-provider orient and search calls must fail unavailable/open_failed
with typed vector-mismatch detail. After shutdown, a separate deliberate
None-provider configuration proves text access. Update maintained MCP interface
and design claims alongside this change. No new error code is introduced.

Filtered MCP text search remains assigned to [Slice 60](features/slice-60/requirements.md).
Its existing runtime rejection stays until that slice implements the allocated
scope. This handoff adds no maintenance MCP surface or other adapter feature.

## Verification and current limits

Behavioral RED at `363fc0744` compiled both real-database SDK composition
controls, then failed both because the requested snippet sidecars were missing.
GREEN `021fd7287` composes one filtered reader request with optional snippets;
plain and snippet-only entry points delegate. Additional attribute/edge-rowid
coverage in `a5bea8f23` passed without another production behavior change.

Mechanical repairs are separate: shared MCP SQLite test dependency `0.41`,
projected-text result off-state sidecars, and existing private provider fixture
seams. No production Caller-provider API or adapter fallback is restored.
MCP reconciliation `115853bbe` consumes the accepted upstream failed-open
contract; its corrected real-corruption regression was already GREEN, so no
new MCP RED is claimed. Its configured-provider orient/hybrid/text failures and
separate deliberate None-provider text success all passed.

Focused verification passed:

- Engine filter/snippet/history/provider/maintenance/overload targets: 104 passes,
  two existing child-process helper ignores; maintenance units: six passes.
- SDK behavior/filter/surface: 46 passes; Python/NAPI private overload units:
  four passes; MCP default suite, frozen hints-off bytes, protocol/schema/stdio
  and real built-in-provider refusal control passed.
- Fresh noneditable Python wheel and NAPI module built from `a5bea8f23`, with
  checkout-owned tools/target and CPU-only features. Python focused run had 40
  passes plus one unchanged cancellation control timing out in the sandbox;
  that exact control passed unconfined in 0.38 seconds. TS focused suites:
  30 passes. Native artifact hashes and exact commands are in the local receipt.
- Strict production-library Clippy, TS typecheck, Python Ruff/typecheck,
  384-target coverage, generated documentation freshness and existing real
  installed-type/missing-export controls passed. Python typecheck reported two
  native-module source warnings before wheel installation, with no errors.

The concise local receipt is `/tmp/fathomdb-s95-forward-output.json`; it records
raw logs/hashes and the executor failure without relabeling it as a passing run.
Frozen Python baseline, MCP hints-off fixture and EP-03 byte oracle are unchanged;
the imported approved Python additions match source C. The existing .29 overflow
chunking analysis is only cataloged; its parked feature scope is unchanged.

Astra review of the actual resolutions and parent-owned promotion remain pending.
No full workspace gate, .29 platform/performance campaign, version cut,
publication, default-release activation or relabeling of .28 receipts is claimed.
The source D and E forwards passed the focused 23-check test-target suite,
384-target coverage and diff validation. No Rust compilation or broad gate ran
for either forward. The original canonical .29 full gate is running on frozen
`6674cf79d`; its receipt retains that identity. After E promotion, qualification
must assemble that original result with affected CI12 rechecks on the promoted
candidate. Neither an assembled pass nor a single full run of the E-forwarded
.29 source is claimed while those checks remain pending. Parent owns review,
promotion and qualification. The .28 source candidates and both canonical
checkouts remain untouched by this isolated work.
