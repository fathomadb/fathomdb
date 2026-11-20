---
title: Slice 55 composed integrity verification
status: ACTIVE
slice: 55
target_release: 0.8.30
---

# Slice 55 composed integrity verification

Entry `72abe1043b0735ecaf79cde35fda227753c2ca2e` under seq409. Direct
Astra-medium passed the reconciled plan/design after one consolidated FIX-1;
planning commit `03ad944c6a98e8cabb44f2e9914fb9b85759bb65`. Direct
Sol6.2-high passed the corrected implementation after independently finding
and resolving weak ownership/filter/recovery/fuzz assertions and a fixture
pipe-backpressure hazard. No product defect was demonstrated. New tests are
passing characterization of shipped behavior, not fabricated RED/GREEN.
The old full-integrity test's early-refusal/Clean escape paths are replaced
by a deterministic, readable index-only findings fixture.

## Selected implementation and independent runtime

Direct Terra independently executed the final affected scopes with real stores
and the owned native modules. All commands exited0; supervised helper entries
are ignored in discovery but actually invoked by their owning parent tests.

| Requirement | Actual proof |
| --- | --- |
| Object cohesion/model | Engine composed suite4 passed plus one supervised helper. Two authored sources, equal Unicode bodies, exact revision/source links, filterable group, supports relation and claim dependency. Exact canonical/FTS/terminal/mapping/f32 ownership is checked during committed vector lag, after drain, supersession, refused batch and supervised fresh-process reopen.16 seeded sequences×10 operations independently model current/history state and positive vector publication. |
| Temporal/graph | Same identities prove before-start/start/end boundaries at fixed generation, future edge t_valid traversal, and exclusive edge expiry with node windows relaxed so they cannot mask the edge boundary. Exact graph origin/cursor and source evidence are checked. Retained node validity12, graph expansion19 and graph evidence34 passed. History/flags, direction/kinds, filtered bridges and cap placement remain with those maintained oracles. |
| Bounded product fuzz | SDK3 passed, including two real product targets, each under an external60s selector. Seed0x550830,128 generated inputs+6 literals per target,≤256 arbitrary UTF-8 bytes. Query positive/invalid outcomes and unchanged canonical rows are exact. Cursor134 malformed/NUL cases plus134 mutations of an authenticated minted envelope,≤2048bytes, require exact typed refusal; real continuation and one-byte MAC tamper are positive/negative controls.51ms cursor/91ms query in Terra's selected run. Proptest generation/shrinking, not coverage-guided; cargo-fuzz/toolchain route evaluated in design, unexecuted. No failing product seed was found. |
| CPU failure/WAL | New delivered SIGKILL leaves exact canonical/lexical/provenance/dependency and pending doc projections, observed before normal worker reopen on a sacrificial matching main+WAL copy; healthy retry publishes exact vectors. Useful held-reader reads continue as8 writes grow WAL, checkpoint returns busy1, explicit reader release/close permits busy0 and exact reopen. Existing before/after-ack kill1, persistent SQLite-full1, persistent provider1 and tc91 stop/reopen1 passed. Process-crash scope only. |
|17 doctors | New CLI6 passed plus supervised helper. Live clap inventory is17. Full-integrity findings65/code/stage/anchor and repeated findings are distinct from malformed-header early refusal70. WAL-only witnesses have the control in both readable copies, selected record only with WAL; actual safe-export0 independently checks manifest digest/bytes before fresh SDK inspection. Actual truncate-wal64 leaves absent/empty WAL and a post-command main-only copy retains exact canonical/profile/lexical/distinct cursor-bound vectors. Exact dump/profile/census/schema/repeats, output failure66, mismatch/missing source and native CPU platform/no-feature warm-cache outcomes fill selected gaps. The design matrix distinguishes permitted effects and shape-only/controlled/gated witnesses. |
| Thin shared boundaries | One authored JSON fixture drives SDK Rust3, installed Python1, Node1 and MCP both formats. Exact identity/Unicode/source revisions, time/filter, continuation/errors and persistence; compact MCP ordered ref/source/body and structured source/snippet; wire budgets tested. MCP formats20, continuation/epoch4 and fetch/expand11 passed. MCP remains read-only and exposes no general mutation/evidence loop. |

Retained doctor controls: CLI recovery16 passed/7 historical placeholders ignored;
operator28, offline integrity14, bounded data-plane integrity3 and orphan
provenance5 passed. Engine dump-profile2, dump-schema1, dump-row-counts2,
verify-embedder4 and mean recomputation5 passed. The seven placeholders are
mapped in design; they are not counted as runtime passes. Default-reranker,
live CUDA/allocator/driver/ARM and successful cache/network branches retain
their actual feature/venue requirements for60/70/75. No new live fault,
GPU/reset/session, model download or publication ran.

## Oracle sensitivity and measurement limits

Three temporary production mutations were rejected by actual behavioral
assertions: omit full-integrity findings, admit an edge at exact expiry in
bounded graph execution, and return successful recovery JSON without doing
WAL work. All production bytes were restored and affected controls passed.
An earlier mutation of the legacy temporal SQL helper survived this selected
bounded-graph test because that route uses its own in-memory predicate; that
surviving result is retained, not relabelled as a killed mutant.

Manual branch map: object/operation fixture→canonical supersession/rollback,
projection pending/publication and dependency fallback; graph probes→node/edge
bounds and relationship origin; CLI witnesses→findings versus early refusal,
export output failure, WAL checkpoint effect and native inspection; SDK
properties→query validation and cursor malformed/authentication branches;
MCP fixture→both renderers, ordered source ownership and wire budgets.
Numerical line/branch coverage was not measured (`cargo llvm-cov` unavailable).
No global percentage, sanitizer, FFI-fuzz or power-loss proof is claimed.

The live-checkpoint investigation remains conditional on a changed selected
boundary and concrete actor/venue/fault authority. This slice changes no
production checkpoint boundary and ran only owned temporary-store CPU faults.
50's Orin/default-Node and60's exact-pin safe-drop remain with their owners.

## Verification and handoff

Scoped Clippy for engine/CLI/SDK/MCP targets, Rust formatting, Python Ruff,
full-package Pyright0 errors and TypeScript compilation passed. Markdown lint
and release-state views passed. The single clean-source full gate and final
native preparation remain pending at this source commit. This is not a
full-workspace-green or release qualification claim.

Raw affected logs and the independent verifier's selected logs are retained
under `/tmp/fathomdb-0830-slice55-evidence` and
`/tmp/fathomdb-0830-slice55-terra`. Final full-gate/native results follow here.
Parent owns canonical state, integration and worktree/branch cleanup.
