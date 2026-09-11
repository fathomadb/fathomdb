# Slice 80.n independent plan review

Reviewer: direct read-only `gpt-5.6-sol`, high reasoning effort, requested by
the owner on 2026-09-11. No timing or broad verification was launched.

Initial verdict: **NEEDS AMENDMENT before execution**. The completion approach
is sound, but avoidable execution traps needed explicit prevention. The main
agent incorporated the findings below into
[the plan](80n-acceptance-completion.md); implementation remains unproved and
requires its own focused review. This record is not execution authorization.

| Finding | Plan disposition |
| --- | --- |
| Rebuild can unnecessarily expose campaign to disk/build/swap activity | Prefer verified retained sealed executable; fresh copied artifact; offline locked fallback only if needed. |
| Test-only edits can invalidate accepted AC-081 with no rerun budget | Protect exact 80.m input hash with executable pre-dispatch guard; no product/Cargo/fixture edits. |
| AC-081 parser is not a three-cell AC-072 dispatcher | Specify dedicated validator, strict child exit, complete sample/count records and fail-closed dispatch. |
| Printed rounded timings cannot establish full-precision pass | Preserve strict Rust oracle and cross-check retained samples without overriding failure. |
| Receipt metadata may not match actual invocation | Controlled child test proves exact selector/environment delivery. |
| New wrapper/binary names can evade census | Extend scanner recognition and live positive controls for actual new route. |
| Historical reviews could be rewritten | Keep old review verdicts immutable and add current review/summary references. |
| Informal readiness may not exercise production path | Use exact collector-only route and qualifier; retain one bounded 30-second window. |
| Existing paths, partial logs or timeouts can corrupt/reuse evidence | Exclusive output creation; no resume; bounded child termination/reaping and INCOMPLETE stop. |
| Commit/seal sequencing can create identity/self-reference traps | Separate build/campaign identities, freeze between cells, two-commit evidence/state closure. |
| Handoff can expand into Slice 85's full matrix | Detailed Slice 80 applicability map; reference other retained inventories for Slice 85 reconciliation. |

Reviewer verified that the retained executable at
`target/release/deps/perf_gates-e6869802f14984e9` matched the sealed
`ff4b78f…` digest and listed AC-072/AC-081 exactly once. HEAD and `ad422346`
matched protected input hash `95e15e…`. These are point-in-time findings, not
permission to skip identity checks during execution.

Unavoidable boundaries remain: execution approval, a new three-cell AC-072
allowance, and an owner disposition if environmental preparation or an actual
measurement fails. A passing preflight cannot guarantee zero machine-wide
swap during the workload. Repeated polling, ad hoc replacement cells, unnecessary
builds, product changes and AC-081 reruns are not authorized substitutes.

## Second review — completion-first execution

The owner requested a second review emphasizing code/runner correctness, a very
short real-path test before longer execution, and actually unblocking Slice 85.
The same direct Sol-high reviewer inspected the fixture and returned:
**add the real-path smoke, then execute; do not grow another validation framework**.

The plan now includes a 30-second-timeout ten-record/384-dimensional warm smoke
using the same sealed executable, runner, collector and parser. The compiled
sample count is 1,000; `AC013_SAMPLES` does not change it. The smoke tests real
open/write/drain/vector/search and receipt handling without changing protected
inputs, and is explicitly NON_ACCEPTANCE. Only the full three qualified cells
can satisfy AC-072. No actual smoke was run during this plan review.

The smoke replaces the extra idle readiness window. One consolidated approval
covers smoke and conditional full campaign; no permission prompt is required
between them or for routine reads, copies and focused repairs once commissioned.
An identified harness failure returns to a short focused fix/test; an invalid
acceptance run stops timing, not other useful work toward Slice 80 closeout.

The reviewer confirmed this smoke is feasible from source inspection. Runtime
duration and implementation correctness still require the actual smoke and
focused implementation review; this verdict does not claim either has run.
