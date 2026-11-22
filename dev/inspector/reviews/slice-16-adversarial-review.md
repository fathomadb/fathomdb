---
title: Inspector Slice 16 postcompletion Astra review
date: 2026-10-31
product: FathomDB Inspector
target_release: 0.1
status: ACTIVE
desc: Independent postcompletion adversarial finding and correction disposition
---

# Slice 16 postcompletion Astra review

Fresh direct **gpt-6-astra / medium** reviewed clean completed Slice16 commit
`4e495db75dba9ab7ec86dcb1402192746c670da1`, canonical needs/req/AC/HLD,
M01–M39, five handoffs and unchanged dependency source. Read-only source/design
review, no application/runtime qualification. The owner explicitly requested
this review after completion and required actual remediation plus Astra agreement
before a fresh Sol-high agent can complete Slice20.

| Priority / finding | Source evidence and consequence | Actual design correction / verification ownership |
| --- | --- | --- |
| NEED — N16-A01: inherited environment can override qualified Engine effects/query semantics | `connection_runtime.rs` `apply_perf_experiment_writer_pragmas` / `apply_perf_experiment_reader_pragmas` accept environment PRAGMAs; `open.rs` invokes writer hook before admission work; `search.rs` reads `FATHOMDB_PERF_SEARCH_LIMIT` for hybrid candidates. Selected features/EngineConfig alone did not neutralize these production hooks. | Canonical HLD now requires env_clear before preflight/session/helper dependency initialization, closed worker allowlist, validated paths/IPC and separate cleared browser GUI environment/private profile. Requirements/AC, F-05/F-07/M15/M22/M29/M30/M33/M35/M36 and all five handoffs include hostile-parent invariants; 20 spawn/header/effects, 30/40 queries/models, 60 installed. Runtime methods remain PLANNED. |

Astra reported no additional substantive Should or nice-to-have finding. It
confirmed source-feasible cache-only CPU, exact readers, separate FTS observations,
stored-f32 L2, full edges and typed observations; .9/1.x evolution and all native
views remain mandatory. B16-01 stays resolved by the owner's “only if it exists”
ruling; physical deletion is correct, with no retention/intent/API/schema changes.
The former historical Markdown-gate limitation remains separate from this finding.

Astra re-reviewed the actual correction against `4e495db75`: **PASS**, N16-A01
remediated at design level, no residual substantive finding. It explicitly agreed
Slice20 may proceed after the correction commit and clean-check sequence. Fresh
Sol-high focused design recheck also passed, no material findings. Runtime
hostile-parent cases remain planned implementation obligations, not executed
security/query qualification. Final narrow direct Terra source/document verification also passed with no
material residual; checks and limitations are recorded in Slice16 status. The known unchanged historical full-Markdown gate failure remains.

Frozen review citations at `4e495db75`: HLD selected features/options lines
128–138 omitted environment isolation;
[writer hook](../../../src/rust/crates/fathomdb-engine/src/connection_runtime.rs)
line 313 and reader hook line 354 accept ambient PRAGMAs;
[open](../../../src/rust/crates/fathomdb-engine/src/open.rs) line 1090 invokes the
writer hook; [query](../../../src/rust/crates/fathomdb-engine/src/search.rs) lines
1730/1752 use the inherited hybrid lexical cap. The correction is in canonical
HLD's process-environment section, not a dependency edit.

This record preserves independent evidence/disposition, not a competing
architecture. Canonical HLD and the single verification matrix remain authoritative.
