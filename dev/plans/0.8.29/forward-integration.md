---
title: 0.8.29 — final 0.8.28 forward integration
status: IN_PROGRESS
target_release: 0.8.29
---

# Final 0.8.28 forward integration

Owner-directed handoff under the [release plan](../plan-0.8.29.md), completing
0.8.28 Slice 95's existing handover obligation. Inputs: .29 `d29dcced95` and
final .28 source C `5f668f33056d0a550c51810f6ebeeb1ef7d60695`; merge base
`59079d5c`. Mechanical checkpoint `73eb608eb` first integrates .28 `16234813`;
the source C merge adds the final documentation and CI9/CI10 fixture corrections.
The .28 candidate remains immutable. Astra approved this bounded integration
and the explicit superseded MCP expectation adjustment before resolution.

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

Mechanical resolution and real-database composition tests are prepared first.
Observe behavioral RED for the combined filter/snippet contract before common-route
implementation, with higher-ranked nonmatching distractors, empty-filter outcomes,
correct body-relative snippet spans and independent historical axes. Existing
filter/snippet/provider/admission controls and frozen baseline checks remain.

Astra reviews actual resolutions before canonical promotion. Focused Rust, SDK,
Python/TS installed-binding and MCP protocol/provider checks must bind the merged
source; target/docs/generated views also require their existing validators.
Source C is merged without conflict. Formatting, design-reference and Python lint
hooks passed on the mechanical checkpoint; no runtime RED/GREEN or merged-source
verification result is claimed yet. Parent owns build scheduling and canonical
checkout environments; isolated resolution never borrows another target or venv.
No version cut, publication, default-release activation or relabeling of .28
platform/package receipts is part of this integration.
