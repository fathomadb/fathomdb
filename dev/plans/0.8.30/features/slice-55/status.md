---
title: Slice 55 composed integrity verification
status: COMPLETE
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
and release-state views passed. The final clean-source native preparation and the single full gate completed
as recorded below. This completes55 implementation and its default source gate;
70/75 retain their integrated, long, hardware and final-artifact qualification.

Raw affected logs and the independent verifier's selected logs are retained
under `/tmp/fathomdb-0830-slice55-evidence` and
`/tmp/fathomdb-0830-slice55-terra`. Final full-gate/native results are recorded below.
Parent owns canonical state, integration and worktree/branch cleanup.

## Final clean-source gate

Direct Terra ran unchanged `./scripts/agent-verify.sh` once on clean
`866dcad1650f9b9efade23959e0258b33cbe7197`. The actual verifier exited **0**
after3825.632s.241 suites registered,238 ran and passed,0 failed,3 skipped,
0 excluded. The three skips are the explicitly unselected Slice30 G/M/D live
private-cgroup controls. Lint, default/release/test-hooks typechecks and strict
security passed; AC-036 executed real strace and AC-037 the live netns catch.
Security reports0 violations,0 blockers and0 downgrades. Tegra pool lint and
unconfigured ESLint retain their declared skips. The inherited200µs hints
criterion was unchanged; earlier40/50 records are not relabelled.

Captured Python reports1787 passed/36 skipped; TypeScript638 passed/6 skipped,
0 failed,644 total. Rust's424 result lines sum2852 passed,0 failed,74 ignored
and432 filtered, including39 zero-result lines; this is the raw reported sum,
possibly including subprocess results, **not** a deduplicated test count.
Ignored, filtered, network-disabled/early-return and unselected long branches
are not additional runtime proof. The independently selected nonzero bodies
above remain the discriminating55 witnesses.

The owned outer2400s timeout monitor was suspended individually while the
canonical gate continued making progress after its lengthy initial compile.
Before monitor resumption/reaping, the actual verifier's zombie kernel wait
status0 and PID/start-time topology were captured in `verifier-exit-stat.txt`
and companion evidence. No live test children remained in that process group.
The resumed outer monitor then reported124 from its pending alarm; that
separate wrapper status remains unmodified in `gate-result.txt`. It is not the
verifier exit. The raw log watcher retained316 spill FDs with zero missed files.
No gate restart, source edit, security downgrade or numerical change occurred.

Free space fell to9.4GiB during Rust build/test work. After proving that no
compiler, open FD or mapping owned the main incremental cache, and verifying
and retaining native modules/wheel/receipts, only this checkout's inactive
`target/debug/incremental` cache was removed. Deps, executables, active
trybuild files, native subjects and unrelated data were kept;25GiB was
restored and the gate remained uninterrupted.

Clean-head production Python wheel SHA-256:
`83574b979204f55b5e5925ea712080da984ef04c6a64d03ab54872e883b2ca01`;
installed production module:
`8469f797f0bb7604afdec5a7de839963a86b9a790c2a4bcc0cdff929519226bd`.
Gate source test-hooks module:
`e041eced43fe0c382fa821fb955e572e7f5692ae4c3fcbae9a319a19ef64d735`.
Initial independently checked Node test-hooks addon:
`32dedb1fb7b79397e52f697adc31b5fa866f15743a7e1a8db5071ac4cf4b1e64`;
the gate's Node build produced and tested:
`6b9575d60b399c2fd4b1a55d25f40321bb9fbecfee1cf3fc253ba4642211f521`.
Both Node subjects, Python subjects, wheel and source-bound receipts are kept
under the evidence `native/` directory. Manifests are0.8.29 on this source
candidate; developer Node26.13.1/npm11.20.0 do not promote public support.

Direct Astra-medium also passed the final truncate-WAL design refinement and
truthful doctor matrix at866dcad. Sol6.2-high and Terra results remain as above.
All owned gate/monitor/test processes ended. Final closure changes only these
local plan/status records; the native/gate subjects remain their actual866dcad
source. Parent owns integration and temporary branch/worktree removal.

## Release integration

Parent Codex accepted the final critical recovery, identity/vector ownership,
temporal/graph, input and integrity oracles and integrated
`3595192b0a3d4587556aa00b24768a06469af3ba`. The verifier exit0 is supported by
the captured kernel status, with outer124 retained separately. Owner seq-409
now activates60;50 remains partial and will resume after its prerequisite.
