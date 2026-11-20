---
title: Slice 55 composed integrity design
status: APPROVED
slice: 55
target_release: 0.8.30
---

# Slice 55 composed integrity design

## Reconciliation and boundaries

Commissioned baseline is `72abe1043b0735ecaf79cde35fda227753c2ca2e`,
not the original draft's `9538272e`. Since drafting,30 repaired record/source
attribution and trace authentication;40 completed retrieval work; partial50
added queue, ownership and logging proof. Their maintained assertions stay.
Prework3's observable readiness,4's separate transaction owners and5's
independent-oracle/positive-progress guidance apply. The six rows of the
[plan](plan.md) remain selected; no canonical acceptance IDs are created.

Goal: independently detect inconsistent object identity/visibility/durability
across the existing format37 data plane and actual boundary conversions.
Non-goals: new algorithms, indexes, formats, migrations, defaults, models,
precision, erasure campaign or broad test framework. Graph uses canonical
traversal. Source proof here does not replace70 integration or75 final artifacts.

The architecture's canonical/source authority, async vector publication,
transaction-owned visibility and SDK/operator separation fit the present code.
The accepted eligibility, record-identity and SDK contracts remain unchanged.
No public API addition is planned. A proven product defect gets its own failing
RED commit followed by a narrow GREEN; already-correct characterization may
pass immediately. Fixture mistakes are setup failures, never product RED.

## Changes and discriminating acceptance

| Local requirement | Implementation and independent expectation |
| --- | --- |
| Object cohesion | One engine integration suite: two sources, Unicode/equal bodies, known8D vectors, distinct revisions, declared group attribute and supports/refutes relations. A derived claim declares a whole-body dependency on source-a revision a-r1; trace/evidence must resolve that revision and its exact Unicode bytes. At instant T and T+1, lexical/vector/canonical/graph observations address these same identities, with expiry excluded at equality without generation change. Freeze the existing scheduler before writes. Assert exact canonical bodies/source/cursors, independent lexical membership, absent vector/terminal during committed lag and truthful pending status; unfreeze/drain and require actual vector publication. Supersede one identity, reject an atomic invalid batch, preserve the unrelated twin and graph. Reopen in a supervised fresh child and inspect exact state. |
| Temporal/graph | Same authored records use explicit instants at start/end with no intervening generation mutation. Retain maintained node-window/current-history and graph direction/kinds/filtered-bridge/cap tests. Assert edge expiry equality excluded and future t_valid traversable as shipped. Exact evidence assertions reuse provenance/graph-evidence suites rather than inventing MCP mutation parity. |
| Bounded operation model |16 deterministic seeded sequences, at most10 operations each, over three logical identities: write/supersede, invalid-batch rollback, drain and close/reopen. Independent map tracks current body/source and retained historical count. Check real reads and lexical membership after every operation and projections after drain. Failure prints seed and shortest failing prefix; stop on first failure. |
| Product-input fuzz | Select two actual SDK boundaries: text query admission/search and frozen canonical-page cursor admission.128 bounded seeded arbitrary Unicode/bytes-derived strings each plus valid/invalid literals, max256bytes for arbitrary strings, max2048bytes for authenticated store-minted envelope mutations, and no recursive payload. Real seeded store, exact survivor/post-state, typed outcome, external timeout60s/target plus completion assertions. First create a real multirow canonical page and consume its valid continuation; then change one byte and require the documented malformed/invalid cursor error. Reject-all therefore fails the positive control. Proptest supplies generation/shrinking and fixed seeds; this is property fuzz, not coverage-guided or FFI/power-loss proof. |
| CPU recovery/WAL | Execute existing slice135 before/after-ack kill, persistent SQLite-full/provider and tc91 stop/reopen selectors. Add one missing process-kill composition with scheduler frozen: committed canonical/provenance/lexical records and pending vector state survive actual delivered kill, unrelated survivor intact; before any normal Engine reopen, inspect raw canonical, pending, terminal, vector and dependency rows on a matching sacrificial main+WAL copy; healthy retry drains to terminal and fresh reopen verifies. Add useful held SQLite reader/writes/WAL-growth/busy-checkpoint/release/successful-checkpoint/reopen fixture. No new faultpoint needed where the scheduler seam suffices. |
| Doctor outcomes | Derive all17 variants from DoctorCommand/run_doctor. Retain one source/effect matrix with current named witnesses, exact outcomes and hardware omissions for70/75. Replace weak full-integrity oracle using an owned witness(v INTEGER) table with partial index witness_gap WHERE v=1 and rows1,2. Rewrite only its sqlite_schema SQL predicate to v>0 under writable_schema, close and reopen; require quick_check ok and ordinary Engine open success. Full must return a physical finding whose detail names witness_gap, code E_CORRUPT_INTEGRITY_CHECK, stage IntegrityCheck and exact design/recovery.md#integrity-check-full-findings anchor; actual CLI exits65. PRAGMA execution error or Clean fails. |
| WAL-only CLI | After healthy base/drain/checkpoint, hold a read transaction and commit selected record plus projections. Capture quiescent main+matching WAL bytes without opening CLI inputs. Sacrificial copies must independently expose control in both, selected record only with WAL. CLI doctor safe-export uses shipped VACUUM INTO and exits0; its manifest_sha256 equals independently computed SHA-256 of exported bytes, manifest sha256 and byte_count checked before opening output. CLI recover --accept-data-loss --truncate-wal exits64 and checks verb, done status and no corrupt-WAL discard. Both run on their own untouched main+WAL copies. Fresh child SDK/facade reads verify exact canonical/profile/lexical/vector facts. Owned temporary DBs only. |
| Thin boundaries/oracle strength | One authored JSON expectation file consumed by Rust/Python/TS plus MCP structured/compact fixture. Thin checks cover Unicode/distinct identity/source, time/filter, cursor/refusal/persistence at actual supported surfaces. Existing MCP wire-budget/epoch/continuation tests remain owning deep cases. Demonstrate a few wrong-state/order variants rejected by the new assertions; retain manual branch-to-test map, report unavailable numerical coverage honestly. |

## Mechanisms, risks and research

SQLite's [WAL documentation](https://sqlite.org/wal.html) explains that a reader
pins an end mark and prevents checkpoint completion while later committed
frames remain in WAL. A main-only copy is therefore insufficient proof by
itself: both sacrificial witnesses must open and expose the independent control.
The [backup API](https://sqlite.org/backup.html) is an alternative logical snapshot
mechanism, but shipped safe-export actually uses VACUUM INTO; test that route.
A checksum-consistent unrelated index inconsistency reaches integrity_check
without damaging admission tables; setup must assert ordinary open succeeds.

The [Rust Fuzz Book](https://rust-fuzz.github.io/book/cargo-fuzz.html) uses
cargo-fuzz/libFuzzer via libfuzzer-sys. Entry environment has nightly2026-04-24
but `cargo fuzz --version` reports no command. A real coverage-guided route
would install cargo-fuzz, add a separate fuzz manifest with libfuzzer-sys and a
production SDK/query target, then run `cargo +nightly fuzz run <target> --
-max_total_time=60 -max_len=256` with an explicit corpus. This route is evaluated
but unexecuted: existing Proptest avoids new build/dependency infrastructure for
the selected bounded smoke and supplies shrinking. No coverage-guided claim.
SQLite's [anomaly methods](https://www.sqlite.org/testing.html#anomaly_testing)
apply to Fathom-owned composition; process kill is not torn/reordered-write proof.

Cancellation-safe test ownership is essential: bounded channel rendezvous,
child kill/wait on drop, explicit reader release/join before assertions, and no
unbounded barriers while a runtime owns a transaction. Commands use external
watchdogs as hang detection. Keep exact original failures and selected counts.

## Prerequisites, review and execution

Independent CPU/test implementation is commissioned by seq409. No live
checkpoint venue/session/actor/fault authority exists in this commission;
that conditional investigation remains unexecuted with its existing owner.
50's Orin default Node and60's exact-pin safe-drop do not block these CPU rows.
Parent owns release-state/board/ledgers/integration and worktree cleanup.

Direct Astra-medium reviews plan and design together with distinct verdicts;
one consolidated FIX-1 maximum, re-review before commit. Direct Sol6.2-high
reviews the final code; direct Terra independently executes affected real/native
scopes. Own .venv/root+TS modules and native subjects, never borrowed installs.
Scoped checks during work; full agent-verify once after final coherent commit,
unconfined strict security where ptrace requires it. Fix only failed/affected
or previously unrun scope afterward. Record actual coverage/limits in status.

Direct Astra-medium: plan PASS; design PASS after the single consolidated FIX-1.
Runtime results remain to be established.

## Doctor outcome and effect map

Inventory is derived from current `DoctorCommand`/`run_doctor`; the new CLI
inventory test introspects clap and detects a newly unaccounted variant.
Existing suite names below are maintained source witnesses, with actual55
execution and omissions recorded in status.70 owns source qualification and75
thin installed scope. Ordinary engine-open commands may bootstrap/start workers;
logical census preservation does not imply byte-preserving entry.

| Command | Outcome/effect oracle and owning witness |
| --- | --- |
| gpu | DB-free provider inspection,0/65/70; lib doctor_gpu_process_matrix_has_exact_outputs_and_no_side_effects has controlled-provider exact output and no-file assertions. operator_cli only allows0/65/70 and checks shape; it supplies no live/no-file proof. Live CUDA remains60/70. |
| platform | DB-free classification,0/70; lib platform_classifier_covers_all_two_tier_outcomes_and_json_contract covers controlled outcomes. operator_cli is shape-only. New platform_native_non_aarch64_is_exact_database_free_success asserts actual host JSON/0/empty canary; ARM venue remains60/75. |
| reranker-gpu | DB-free provider policy,CPU0/refusal65/error70; operator_cli exact CPU subprocess, default-reranker selector. |
| cuda-allocator | DB-free,may initialize context/pool;0/70,malformed flags2; slice31_cuda_pool_cli and lib policy controls. Live allocation60. |
| cuda-libraries | DB-free dynamic loads/device probe,0/70; operator_cli exact record/refusal fixtures. Live resolver60/75. |
| check-integrity | Ordinary open,0 clean/65 findings/70 refusal/71 busy. check_integrity plus composed CLI exact named full finding/code/stage/help and repeat; full_integrity_early_header_refusal_is_distinct_from_findings asserts early-open70/E_CORRUPT_HEADER separately. |
| data-plane-integrity | Offline immutable files/bytes,0/65/70/71; slice30_operator_integrity_cli and slice55_data_plane_integrity_cli own clean/findings/busy/missing/malformed checks. |
| safe-export | Ordinary open plus output/manifest writes,0/66/70/71. New paired WAL witnesses, independent digest/bytes and fresh SDK inspection; occupied output66 preserves sentinel and creates no manifest. |
| verify-embedder | Ordinary open diagnostic,0 even mismatch; new repeated both_mismatch with exact supplied fields and surviving canonical row; operator_cli matching controls and engine mismatch enums. |
| trace | Ordinary open/read,0/70/71; recovery_cli exact2 ordered source events and new missing source0/empty events/repeat. Not governed dependency tracing. |
| dump-schema | Ordinary open/read,0/70/71; operator_cli existing envelope is shape-only; engine dump_schema::ac_040a_dump_schema_reports_current_user_version_and_canonical_tables owns semantics. New WAL fixture requires CLI37/canonical tables/no sqlite_* plus repeat. |
| dump-row-counts | Ordinary open/read,0/70/71; operator_cli existing envelope is shape-only. dump_row_counts::ac_040a_dump_row_counts_reflects_canonical_writes owns exact census; new WAL CLI requires nodes2/edges0/repeat. |
| dump-profile | Ordinary open/read,0/70/71; dump_profile::ac_040a_dump_profile_returns_stored_identity_and_dimension and ac_040a_dump_profile_lists_registered_vectorized_kinds own exact semantics. New WAL CLI asserts cli-wal:authored-v1/dim8/[doc]/repeat; old CLI envelope is shape-only. |
| warm-cache | DB-free,cache/network writes;0 successful verified artifacts/70 unsupported or load failure. new warm_cache_no_feature_is_exact_database_free_refusal asserts exact JSON70 and empty canary. cli_warm_cache success remains cache/model-gated; network-disabled early returns are not runtime proof. No download/live claim here. |
| recompute-mean | Ordinary open then transactional mean/sign-bit mutation,0/70/71; pr2b_mean_recompute independently closed-form mean/all bits/rollback/reopen; operator_cli JSON/lock witness. |
| dump-mutations | Ordinary open/read,0/70/71; operator_cli exact ordered rows/exclusive pagination/empty and malformed collection. |
| orphan-provenance | Ordinary open/read,0 clean/65 unerasable/70/71; doctor_orphan_provenance exact per-source census and logical no-mutation. |

The actual WAL recovery selection is `--accept-data-loss --truncate-wal` on
its own untouched main+WAL copy:64,verb truncate-wal,status done and
`discarded_corrupt_wal=false`. This preserves authored positive vectors without
requiring an unavailable production custom provider to regenerate them.
`safe-export` and recovery both retain exact profile and f32 bytes verified
in a fresh SDK process. Existing rebuild_projections/tc91 suites separately own
regeneration/failure behavior. No new recovery command or manifest verifier.

The seven ignored recovery_cli placeholders map to: WAL export→new composed
CLI witness; canonical/FTS/profile/vector parity→same witness plus maintained
rebuild_projections; tamper→safe_export independent digest mismatch (there is no
CLI manifest-verifier); full findings→new exact CLI case. Keep placeholders
historically labelled rather than implying zero existing coverage. No oracle
or canonical AC has been removed.

The implemented pending/recovery oracle joins each authored doc cursor to its
canonical body/source/revision/source-link, lexical row, terminal state and
actual vec0 f32 bytes. It checks exact dependency ownership, not table counts
alone. Fresh child helpers require completion markers written after all reads
and close, with bounded kill/wait cleanup. Required test-hooks registration
replaces debug-only exclusions for the new engine/CLI suites.
