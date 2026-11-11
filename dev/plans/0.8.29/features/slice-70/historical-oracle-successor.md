---
title: Slice 70 — narrow historical MCP oracle successor
status: PROPOSED
target_release: 0.8.29
slice: 70
---

# Historical MCP oracle successor

Source-only forward integration finding at clean parent
`2f6913c1d59ac8ff3a30de835b57290a8635998c`. Parent reports independent Astra PASS for this bounded historical amendment
and the actual60 fit packet. This canonical copy is linked from plan/design/test
map and consolidated for consistency recheck, with no source GO or hard60 closure
gate. The prior reviewed originals remain immutable outside this candidate tree.
See [complete oracle inventory](oracle-successors.md), [pinned diff](oracle-successors.proposed.diff)
and [runtime ordering](execution-map.md).

## Confirmed conflict

Immutable `src/rust/crates/fathomdb-mcp/tests/fixtures/hints_off_baseline.jsonl`
SHA256 `0cdca8a0b906b9cd0b559c0e870e898fd4637da4b9d6176b99047a93c73070b4`
has one explicit hybrid search: zero-based row4, JSON-RPC id7,
query story/mode hybrid/explain true/as_of1900000000/limit6. Its six text hits
carry exactly `[{"arm":"vector","reason":"embedder_unavailable"}]`.
The fixture store opens through SDK OpenOptions::default and MCP session config
omits use_default_embedder; both defaults are false. This is an actual supported
None-provider query attempt, not a manufactured degradation fixture.

Accepted60 comparator witness is the immutable
`/tmp/fathomdb-0829-direct/slice-60/comparator-review/hints_baseline.rs.proposed.v2`,
SHA256 `cd8a68aa40973c81bee062ab72705d4ac8e7c2c9c3a8c848389e68728a47abca`.
The actual formatted tests/hints_baseline.rs at307c, unchanged at7038, retains
this same reviewed comparator contract; its exact blob/SHA256 is in
[source pins](source-pins.json). Its compare_baseline clones historical desired payload; only rows5/8 get exact
budget-driven payload adaptations, with all rows retaining text/budget accounting
rules. It grants no row4 degradation change.

Current tools.rs::search_degradations emits the sampled unavailable label, then
copies engine per-call reasons for hybrid. shape.rs::canonical_degradation sorts
and deduplicates. Approved70 absence therefore adds embedder_absent before the
retained unavailable label on this call. Existing comparator would reject that
array via its final exact comparison. This conclusion is source reasoning; no
comparator/product test ran.

## Exact successor mapping

Extend the historical comparator only for row4 of this fixed session. Require
the authored original's hybrid mode, store a, epoch1, id7, expected six refs and
exact one-entry legacy degradation to match before applying the amendment.
Do not discover applicability from current output or from any arbitrary hybrid
row; future fixture/session changes need explicit reconciliation.

Set desired.degradation to exactly this independently authored ordered array:

```json
[
  {"arm":"vector","reason":"embedder_absent"},
  {"arm":"vector","reason":"embedder_unavailable"}
]
```

Keep all remaining row4 semantic fields and envelope exact: six result values/
order/scores/source/snippets, full explanations/trace, state/verdict, request/
response handles, validity instant, completeness limit_reached, budget max4000
and cut=false. Both hints-off modes still omit hints. The accepted60 comparison
already requires structured text to equal the final compact JSON serialization,
independent truthful used_tokens and both-copy fit; retain those rules rather
than accepting stale historical count/text or calling a production shaper.

A static independent compact-JSON calculation over the authored amended payload
converges to2735 bytes, used_tokens1353 and two-copy fit1368 tokens, below4000.
This is an authored reference calculation, not an observed server result. No
extra hit-cut/continuation/handle/completeness adaptation is justified for row4.
Rows5/8 keep only their existing accepted60 adaptations. Other rows' degradation
arrays remain exact historical values; the fixture bytes are never regenerated.

## Runtime RED placement and negative controls

Include the comparator successor in the absence behavior's T70-2 RED/GREEN pair,
not as late passing coverage. At RED, construct current_wire from authored row4
with the exact amended pair and correct reference counts; existing comparator
must reject it at runtime. Also stage the real None-provider MCP both-format
absence assertion before engine GREEN. In that same absence GREEN, add the
approved engine reason and narrowly amend desired row4 comparison. Do not add a
transitional acceptance of both old and new arrays; it would hide missing70
behavior. No missing enum/helper compile error supplies this runtime RED.

The following authored negative controls stay in that same failing test group;
run them after GREEN and preserve existing60 semantic/accounting negatives:

- Original row4 legacy-only array must now fail; missing absent is a regression.
- Absent-only replacement, wrong arm, timeout/overload substitution, duplicate,
  reversed pair or any third entry must fail. Retaining the epoch label and its
  different provenance is part of the exact successor contract.
- Adding either new word to any historical text-mode row must fail. No blanket
  degradation-path ignore or global new-label normalization is permitted.
- Correct pair plus changed row4 hit/score/source/order/explanation/handle/state/
  verdict/completeness/max/cut or envelope must still fail exact comparison.
- Correct pair with false used_tokens, mismatched text/structured payload or
  over-budget copies must fail existing accounting/equality rules.

Config-off versus call-off byte equality remains per current format. Live
hints-on tests separately require the first canonical cause embedder_absent;
this hints-off historical amendment does not authorize a new hint/default/ABI.
Actual integrated60 source/comparator is reread before70 coding; any divergence
from this immutable proposal is reported, not normalized away. No tests, source,
fixture, Git, environments or sibling checkout were mutated in this check.
