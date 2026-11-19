---
title: Slice 30 correctness implementation status
status: COMPLETE
target_release: 0.8.30
slice: 30
date: 2026-11-19
---

# Slice 30 implementation status

The work commissioned by seq-402 and seq-403 fixes MCP revision attribution and partial-span
dependency tracing. It adds independent oracles for ranking, record provenance
and committed coverage.
[Slice plan](plan.md) · [MCP contract](../../../../interfaces/mcp.md)

## Selected cases

| Case | Disposition and evidence |
| --- | --- |
| S50 G-W sample 860, query 360, operation 9971 | Historical cause unresolved. The original diagnostic remains a mismatch under seq-393. The bounded real-store case reproduced score movement from projection state with exact reference/winner results at each fixed boundary. |
| Ranked record provenance | Expected behavior. The two cursor-distinct role-tagged twins have the same public hash and body. The probe verifies own source, original positions, exact score bits, and UTF-8 highlight/extent coordinates. Existing ownerless, repeat-credit, eligibility and SDK duplicate-context refusal controls remain applicable. |
| MCP revision feedback | Confirmed product defect, repaired. The RED at `ac639b626` recorded the replacement cursor 2 for the earlier response, which returned cursor 1. GREEN `c27b5e3b8` hashes ordered kept hybrid ref/cursor pairs so both handles retain their query and cursor. Identical state and inputs preserve byte determinism. The continuation control distinguishes cursors 1 and 2 under the same public ref on separate pages. Mixed named ambiguity still refuses before any label write. |
| Stored coverage and extents | Expected behavior. The seed `0x50830` tests exact authored counts and shares across eight real-store cases. Its populations include retained history, unmapped data, a passing unattributed head, zero-length known coverage, NULL and known-zero endpoints. A failed refresh cannot replace the last committed counts. Existing pinned/fresh reader, malformed span and bounded-point controls pass. |
| Typed open admission | Expected behavior. Existing profile BLOB/negative-dimension cases discriminate profile admission from coverage token decoding, which reads the same fields. Existing engine controls distinguish malformed span, requested-provider mismatch and token-only failure. Exact checkout-native results below. |
| Partial-span dependency tracing | Confirmed product defect, repaired under seq-403. [#264](https://github.com/fathomadb/fathomdb/issues/264) RED `c8eb1915a` omitted both registered strict-span artifacts and admitted the wrong derived root when the stored hash covered the cited bytes only. GREEN `6874044b5` authenticates the full canonical source after independent locator validation. |
| Query property scaffold | Retired self-equality placeholder. Generated literal-token, quoting and stop-word compiler properties remain in the query crate's in-crate suites. |

## Historical score observation

Both archived native attempts at `09c53495b` exited 0 on the reference route.
In the eager and winner observations respectively, the reader cursor was
108600 and 108610, visibility generation 3392 and 3393, and the bound validity instant
1794962445 and 1794961831. The nonces differ and each dependency generation is
zero. Both recorded 192 raw ranked vector candidates, 10 vector arm
participants and 15,891 lexical rows. No candidate IDs or tie positions were
recorded. The two closed format-37 files retain canonical rows 51292 and 90997
for the changed results and describe final state.

The reported score bits uniquely fit vector/text ranks (8,7) versus (10,7) for
`l:s30doc-51291` and (6,19) versus (7,19) for `l:s30doc-90996` within the recorded
arm lengths and fixed RRF weights. Eager is higher by 1.240362% and 0.850694%.
Asynchronous projection and tie order remain possible contributors. Archived
candidate or validity-state differences cannot be attributed further.

In the CPU fixture, the held projection read at cursor 2 and generation 8
ranked vector rows 2 and 1 at L2 distance 1. The committed read at cursor 3 and
generation 9 ranked row 3 at distance 0 ahead of the tied originals; text
ranks remained rows 1 and 2. Both reads bound validity instant 1800000000.
The fixture recomputes RRF from these original positions, checks complete
reference and optimized output and sidecars per boundary, and captures the
actual arm and visibility inputs on the ranking transaction. For a new
natural-write comparison, group by pinned validity, query/settings and a
matched visible state witness. Preserve the observed raw arm order and compare
exact scores only with state and route evidence. Unmatched observations remain
diagnostic. S50-A3 exactness and the original qualification and charges remain
as recorded.

## Partial-span dependency tracing

The real-store test writes a canonical `aé猫z` source and registers a node and
edge with byte range [1,3). Both directions return exact reciprocal endpoints,
relation records, and work counts under the same frozen context. The positive
fixture also checks a full-length byte range and the persisted locator and
whole-body digest. The negative controls refuse a span-only digest, a
mid-codepoint range, an out-of-bounds range and changed bytes outside the cited
span. Invalid derived roots remain nondisclosing, and the healthy siblings
still appear in the forward trace.

Trace's existing nonempty range requirement remains. The writer and dependency
registration accept empty ranges at valid boundaries. The fixture records the
trace refusal as an adjacent policy discrepancy outside the digest correction.

## Occurrence labels under seq-375

A stable per-result `feedback_ref` could encode a response-specific ordinal.
A structured string field and a quoted compact hit field would identify only
kept hits. Each response would retain ordinal, cursor and query within the
existing store, session, epoch and response LRU. Each continuation creates
its own labels and actual rendering would charge label bytes before retention.
Validate all submitted labels against that response after existing membership
and epoch checks, then record once. A label supplies feedback attribution.
Logical fetch still selects the latest eligible version for its own view.

A deterministic ordinal cannot identify separate telemetry invocations that
share a response ID. If callers require invocation-specific attribution, an
opt-in per-invocation handle would need a distinct determinism contract and
bounded lifetime. Both formats would need tests for repeated refs, same-cursor
occurrences, budget cuts, paging, stale/foreign labels and mixed valid/invalid
input with zero writes. The owner choice remains open.

## Verification

Astra-medium passed the original plan, the MCP design delta and the seq-403
trace plan. Sol-high requested FIX-1 for test route isolation, cross-state tie
assumptions and a Clippy initializer. The feature-enabled probe passed with
both ambient selectors after correction, and Sol confirmed it at `7fb71579e`.
Terra independently passed 112 focused Rust, MCP and snippet cases with no
ignored tests before FIX-1. An existing snippet
proptest persistence warning was recorded. The SDK duplicate-ID context
control also passed. The #264 trace, wire, provenance, dependency and evidence
suites passed 81 tests; one release-only case was ignored in debug. Sol-high
passed the #264 and cumulative code review. The equivalent, lint-clean edge
assertion was confirmed independently at
`aef9b1ba3acb62989be49fb3afb219452952ee3c`.

Terra reproduced the 81 scoped passes on that source. The checkout-owned
production Python 3.12 wheel and Linux x64 Node 26.13.1 addon each passed all
three existing malformed-profile cases. Native process controls verified two
typed refusals with the original class, kind, stage, hint and anchor, unchanged
persisted facts, and healthy Rust-engine reads through each binding in a
fresh process after restoring only the injected field. The Python fixture
and a fresh Node process traced the same stored partial span reciprocally.
Preliminary native fixture and selection failures are retained. The corrected
runs select the installed wheel and all three Node case bodies.

The unchanged strict verification wrapper at clean `aef9b1ba3` exited 1
after 71.75 seconds at module-boundary lint. It reached neither typecheck,
security nor test registration. The two findings were test-module
classification and analysis of the coverage property macro. Test-only
recovery `9a62c4e9b` classifies the ranking module and retains the same seeded
coverage property in a parseable runner. Sol-high passed the recovery and the
targeted boundary and forked property checks passed.

The failed and unrun stages were run separately in their original order.
Workspace lint reached the Markdown link check on `9a62c4e9b`. It found the
prework link to the retired scaffold and exited 2. The doc-only repair
`74247a65d` links to the original tagged source. Scoped Markdown, typecheck
and strict security passed on the corrected subject. AC-036 and both AC-037
layers ran, with zero violations, blockers or downgrades. Workspace Clippy and
the remainder of lint had already passed at the same source.

The collecting test command at `74247a65d` exited 1 with 241 registered,
238 run, 234 passing, four failing, three opt-in private-live skips and zero
exclusions. The serial Rust workspace, release-mode ranking/adapter and MCP
CLI suites, Python with its test-hook receipt, TypeScript and executable N-API
all passed. Two guards found the retired integration entry in the generated
feature matrix. The other guards expect the exact cursor-owner import and the
frozen engine-root file. Recovery `47bd06261` regenerates the feature matrix
with only the retired entry removed, keeps the cursor-owner import and moves the ranking fixture
under the search module to preserve the root blob. All four checked-in guards,
the module boundary, the relocated ranking probe, both issue #264 cases and
30 query unit tests passed in scoped execution. Sol-high passed this recovery.
The historical full-wrapper and collecting-run exits remain recorded.

The production bindings were rebuilt and independently rechecked at final
source `47bd06261867105964bf6a2aa2875eaee4c4adb3`. The selected profile suite
passed three cases per binding. Both runtimes passed the separate-process
typed refusal and persisted reopen check, and reciprocal partial-span tracing.
The original suite logs, scoped recovery and artifact receipts are retained in
`/tmp/fathomdb-0830-slice30-evidence/`.

Slice 70 receives the passing rank/ref/coverage oracles and native scope,
plus the unresolved historical attribution and occurrence-label disposition.
The prior hints timing and subscriber logging outcomes remain with Slices 40
and 50. Integrated qualification and publication remain with their assigned
owners.
