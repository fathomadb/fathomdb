# Slice 20 test map and validation plan

All commands below are prospective and run from the future owned source tree,
not during PhaseA. Record exact source/host/tool/features, actual listed/reached
names, passed/failed/ignored counts and raw exit. A selector that reaches zero
required tests fails evidence. Preserve compiler diagnostics verbatim.

| Test obligation | Existing selector / proposed addition | Oracle, TDD and limits |
| --- | --- | --- |
| T20-1 / R1,A1; TC57 | `cargo test -p fathomdb-engine --features test-hooks --test tc57_governed_write_race` then same with `-- --test-threads=1`; intact5 tests including two actual-worker arms and3 SQLite mechanism pins | Meaningful nonzero embed calls and original write/error/SQLite codes/busy-handler assertions retained. Current first-open helper already fixed; no preassigned new repair. Deterministic regression staged before any reproduced cause fix. |
| T20-2 / R1/R2,A1/A2; startup paths | `cargo test -p fathomdb-engine --features test-hooks --test slice40_fresh_database_cutover`; isolated control same with `-- --test-threads=1` | Current-schema37/open/refusal/byte identity and subprocess separation retained. Different child raw initialization does not poison parent by itself. No schema/migration work. |
| T20-3 / R2,A2; source proof | `cargo test -p fathomdb-engine --features test-hooks --test slice50_evidence`; isolate exact `evidence_linearizes_at_sidecar_and_resolver_return_seams` and `evidence_search_races_are_snapshot_atomic_or_wholly_refused` | Original cardinality/artifact revision/canonical source bytes/snapshot-atomic-or-unavailable assertions remain. New proposed `evidence_sidecar_hook_belongs_to_its_engine` and `evidence_resolver_hook_belongs_to_its_engine` each exercise A-arm/B-first and dual-arm overwrite with real DBs; bounded channels/cleanup, exact identities. Genuine global-theft RED per seam, separate GREEN. |
| T20-4 / R3,A3; queue gap | `cargo test -p fathomdb-mcp --test admission a_drain_before_enqueue_returns_the_call_to_open_wait -- --exact`; proposed name, currently absent | New explicit reached(epoch1,queued0,in_flight0), actual closed/lock/reopening, stale rejection/no old bridge entry, expected ref/epoch2, zero accounting and later idle/LRU health. No production RED claimed for absent coverage; explicit optionA/B from design required before landing. |
| T20-5 / R3,A3; wait/cancel cleanup | Proposed exact `a_pre_enqueue_drain_keeps_the_original_wait_deadline`, `a_dropped_pre_enqueue_observation_releases_its_hold`, `shutdown_at_pre_enqueue_refuses_without_engine_work`; same MCP admission target | Reopening held past original deadline yields exact current phase error, no reset budget. Test-only aborted pause clears its held token through Drop; shutdown has no engine entry and no retained permit/ticket. Existing client-cancel controls separately retain current waiting/running semantics. Test seam guard Drop/panic cleanup independently bounded. Existing behavior may already be correct; no invented RED. |
| T20-6 / R3,A3; existing queue controls | Whole `cargo test -p fathomdb-mcp --test admission`; `cargo test -p fathomdb-mcp --test lifecycle` | FIFO, queue32, no retry field, queued zero on timeout/cancel/panic, permit-before-guard, late permit starts, started result past wait deadline, run abandonment, cancel-running permit retention, shutdown24, busy-store, idle/LRU/epoch controls all retained. Full target contains elapsed-time controls; new race proof cannot depend only on sleeps. |
| T20-7 / R4,A4 | `cargo build -p fathomdb-mcp --bin fathomdb-mcp --release --no-default-features`; add exact historical default-embedder feature configuration only if required by recovered receipt | Non-test warning item/features/digest/log are evidence. Test dev-dependency unification is separate. Reproduced warning repair needs baseline diagnostic, human-intent non-test configuration check, then same clean build; no cfg-text mirror test or blanket warning policy. Unreproduced means no speculative fix. |
| T20-8 / R5,A5; T2 | Proposed `fusion::tests::overflow_chunking_fusion_tripwire` via `cargo test -p fathomdb-engine --lib overflow_chunking_fusion_tripwire`; proposed `candle_bge::tests::overflow_chunking_embedder_tripwire` via `cargo test -p fathomdb-embedder --features default-embedder --lib overflow_chunking_embedder_tripwire` | Actual30/3/1/name/512; feature-enabled embedder test never instantiates model. Absence under feature-off default gate is not coverage. OptionA explicit exception and disposable actual-constant negative controls or optionB defer; no arbitrary changed production constants called RED. |
| T20-9 / R6,A6; Windows successors | Four exact `--lib` selectors listed below under operator,test-hooks on actual Windows | Preserve current exact typed error/worker/profile/SQLite/admission/lock assertions. Baseline reproduction precedes identity hook fix. Bound each process, retain full logs, kill/reap on timeout. Linux/path alias controls supplement Windows, never replace it. |

## Exact Windows mapping

Prefix each current name with `cargo test -p fathomdb-engine --features
operator,test-hooks --lib`, then the fully qualified name and `-- --exact
--nocapture`. This is a command description, not a shell command broken across
newlines.

| Historical .28 receipt | Current fully qualified selector | Required semantics |
| --- | --- | --- |
| failed_startup_after_timed_out_probe_preserves_error_and_cleans_sqlite_profiles | `slice90_close_review_tests::failed_startup_after_accepted_probe_preserves_error_and_cleans_sqlite_profiles` | Accepted provider then injected Io startup error; all profiles released after owners/workers, zero SQLite/provider owners, admission lock reacquired. |
| real_post_probe_visibility_error_uses_shared_unwind_and_retains_provider_only | `slice90_post_probe_real_error_tests::verification_timeout_precedes_later_visibility_fault_and_retains_provider_only` | Actual StartedTimeout precedes later visibility error; retained active provider1 then explicit release0; SQLite0 and admission released. |
| Same historical visibility family, now separate accepted path | `slice90_post_probe_real_error_tests::accepted_probe_still_preserves_later_visibility_error_and_releases_database_owners` | Exact visibility Io error after accepted verification; workers0, owners0, admission reacquired. |
| current_opener_holds_lock_before_admission_classification, historical900s hang | `tests::current_opener_holds_lock_before_admission_classification` | Configure-first, reached lock, older competitor refused without durable schema mutation, final fresh37; bounded ready/release and early-error cleanup. |

## Check ordering and receipts

While editing: changed-crate rustfmt/clippy, typecheck, exact RED/GREEN and
relevant target controls in repository latency order. Test-only hook public
contract docs and owned Markdown use scoped validators. Non-test release
configuration stays separate from dev-feature tests. If public test-hook
signatures change, run `scripts/tests/test_slice140_test_seam_boundary.py`
under owned Python; its independent default/dev/release consumer probes must
reject each test path without feature and compile with `test-hooks`. Existing
free export paths stay recorded in its inventory. Parallel target evidence
is explicit: serial workspace gate alone cannot establish parallel hygiene.
Avoid a speculative whole-workspace parallel campaign; targeted ordinary
parallel runs and deterministic overlaps address these reported failures.

Once final source tree/tools/generated views are stable, run required
`scripts/agent-verify.sh` once without changing files mid-run. Its default Rust
selector is serial and may not enable `test-hooks` or default-embedder; therefore
explicit affected targets above remain required. Source/full gate preflight
must fail on missing checkout-owned tools/native identity, never borrow or silently
skip. AC036 ptrace denial requires unchanged strict rerun on a capable executor.
No current full/source gate or product claim follows this design phase.

Per behavior repair: retain test-only RED commit/staging witness, exact assertion
and raw failure at that SHA, distinct GREEN SHA and unchanged test oracle;
parent/reviewer audit actual Git. No oracle refresh, wrong values introduced for
RED, broad sleeps/retries until green, warning suppression or relaxed semantic
assertions. After two same-issue failures reread test, ADR and verified memory
before further correction. New meaningful scope returns to independent review.

## Completed10 command and artifact implications

Completed10 source104dcbcbe92a1eca730d146b99ab17d444df6217 and parent metadata
d7d928f8e49d7af6a792385534f4e991c13232b3 replace the earlier candidate input.
Actual plan/test-mapping/execution and independent final PASS were reread.
All20 Rust selectors above remain valid: no affected Rust source/selector changed.
Inherited214 registrations are207fast/7heavy; no full214 pass is claimed. The
original211-suite failed collector plus actual affected reruns and three new
registrations remain assembled10 evidence. Do not repeat that entire campaign as
phase-zero ceremony or call its serial Rust run parallel20 hygiene evidence.

From eventual20 cwd, before installers/hooks/npm run integrated
`bash scripts/setup-python-owned.sh --check-ownership` (nonmutating). Parent
creates20 source worktree after strict closed10 dependency preflight;20 owns
real venv/root/TS modules/task outputs. Then use --provision for canonical Python
selection/own developer packages plus fresh noneditable native wheel; --build
requires prerequisites and rebuilds. Neither builds from10 env or stale receipt.
Resource measurement/reservation precedes heavy work; same-filesystem source/
task paths require combined accounting. Root32/20GiB, task250/50GiB, total200GiB,
MemAvailable16/8GiB and one native/full phase/jobs2 are reviewed starting limits,
not current availability or a reservation. Preserve other sessions' artifacts.

Current agent-test scopes own absolute source PYTHONPATH to pytest/children only;
use canonical caller so unchanged fresh-wheel/clean-source/owned-venv/nonce/module
path+digest checks qualify the same candidate in parent and children. Do not
export PYTHONPATH globally, change conftest/product test or use hook presence as
identity. Installed production lifecycle remains site-packages with cleared
source path. Preserve hermetic TS/N-API and production distinction. The child-
import and bootstrap ownership defects are already fixed10, not new20 work.

## PhaseA reconciliation checks and source-entry hold

Validate exact six scratch files, six R/A joins, T20-1–9, intended-destination
links and whitespace. Parent-controlled Markdown CLI0.23.3 is available at the
new root for document checks; source10's venv/modules/native are not borrowed.
Explicit-file Markdown validation checks these candidates, not the full imported
repository Markdown gate; parent runs applicable import checks later. No source/
product test or full gate is appropriate for this documentation-only pass.

Actual10 closure is complete and reread. Parent-controlled updated design recheck,
strict closed10/ref/base preflight, fresh20 source worktree, own setup and GO still
precede source implementation. A3/A5 additions retain actual unanswered policy
hold. Review may accept genuine repair scope without granting a policy waiver.
No20 closure/product PASS follows PLAN_READY or10's preparation acceptance.

Earlier PhaseA checks used supplemental0.23.0 and27 links against candidate98fc;
those were historical document checks, never product qualification. Current
reconciliation results are recorded below after the actual bounded invocation.

Reconciled PLAN_READY checks exited0: six exact files,30 resolving destination
links, six R/A joins, nine test obligations, whitespace/newlines and current
policy/source-entry holds. Read-only registry enumeration includes both direct
and conditional suite registrations:207fast+7heavy=214. Parent-owned explicit-
six-file Markdown CLI0.23.3 exited0 with zero issues. No source/product tests,
builds, setup, repo/Git/ledger mutation or20 implementation occurred. This is
updated-design readiness for independent Astra review, not its verdict or GO.
