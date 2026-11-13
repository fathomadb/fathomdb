# Proposed implementation footprint, commands and resources

The coverage-only source checkpoint now contains the listed child/fixture,
operator/guard tests and index. Maintained-plan Astra PASS at c1226d1bf and parent
implementation GO at5f8f12071 precede it.
[Execution](execution.md) records actual source/G2/healthy and assembled gate
results. The native window has returned to G; further Q1 runs remain held.
Q1 source review, exact actor/binary/runtime readiness and parent technical GO
remain before any debugger/inferior/ptrace/fault launch. No source20 artifacts,
venv or target are borrowed. G is unintegrated, not a hard prerequisite.

## Intended changes

| Intended path | Narrow purpose |
| --- | --- |
| src/rust/crates/fathomdb-engine/tests/slice100_interruption.rs | New ignored normal-success q1_child, ignored g2_child and ignored observer/retry entrypoints; one G2 regular parent over the four selected existing-hook cells and healthy rich control. Existing85 files stay byte-frozen. |
| src/rust/crates/fathomdb-engine/tests/support/slice100_fixture.rs | Bounded independent fixture/raw snapshots/assertions adapted from audited existing fixture patterns; no production proof helper as sole oracle. Test-only ready marker; no production fault seam. |
| src/rust/crates/fathomdb-engine/Cargo.toml | Declare only the new target and required operator/test-hooks. No feature/default/API/format/dependency-version change. |
| dev/experiments/0.8.29-slice100-checkpoint/operator.py | Opt-in stdlib MI operator, exact actor/state/witness validation and bounded cleanup; no automatic debugger launch in normal tests/gates. |
| dev/experiments/0.8.29-slice100-checkpoint/test_operator.py and README.md | Explicit refusal/healthy/no-op cleanup controls, documented commands and current limitations. No new generic evidence framework. |
| Existing test-target/measurement index, only if current validators require | Minimum accurate registration of new tests/tool, without rule weakening or new release gate. |

Update maintained100 requirements/design/test plan/status once with the substantive
accepted result; parent owns all release-state/ledger edits. No public Rust API,
maintenance contract, production erasure/checkpoint module, SQLite/VFS, schema37,
migration or published artifact changes are proposed. If a real repair needs
one, return that concrete finding for reviewed scope and genuine RED/GREEN.

## Illustrative future commands — held

Use owned task TMPDIR/AGENT_LOG_DIR/CARGO_TARGET_DIR. Build JSON, not filename
pattern guessing, selects the then-current test binary. Paths/placeholders below
must be substituted with the reviewed actual source and task; they are not GO.

    cargo test --locked --no-run --jobs 1 -p fathomdb-engine --features operator,test-hooks --test slice100_interruption --message-format=json
    python3 -m unittest discover -s dev/experiments/0.8.29-slice100-checkpoint -p test_operator.py
    python3 dev/experiments/0.8.29-slice100-checkpoint/operator.py --mode healthy --binary <own-build-json-executable> --case-root <fresh-owned-case> --source-sha <actual-sha>
    python3 dev/experiments/0.8.29-slice100-checkpoint/operator.py --mode interrupt --binary <same-frozen-executable> --healthy-result <qualified-matched-result> --case-root <new-owned-case> --source-sha <same-sha>
    cargo test --locked --jobs 1 -p fathomdb-engine --features operator,test-hooks --test slice100_interruption -- g2_composed_existing_faults --exact --test-threads=1

The operator would launch exactly `q1_child --exact --ignored --nocapture
--test-threads=1` through GDB; observer/retry entrypoints are separate normal
returning cases. Q1 remains opt-in, never all-ignored execution. It must use its
specific selected fault authority; mode interrupt cannot be inferred from healthy
GO or successful metadata/design review. Future source-phase scoped checks and
required final full verification follow existing rules once on a stable final
tree, preserving failures/scoped resumptions. No full gate is run for this draft.

## Approved caps and proposed operational thresholds

One source owner/heavy window, Cargo jobs1, TOKIO_WORKER_THREADS=2; no model,
provider construction, GPU or data downloads. Q1 has one debugger+one inferior;
observer/retry run afterward, never concurrently. G2 cases are serial. Use
existing resource-monitor logic rather than a new infrastructure subsystem.

Seq-372 caps owned allocation at64GiB and each case at120seconds. The reviewed
operational proposal counts source/tool/runtime allocation, including private target and
case outputs; evidence/case files cap1GiB, logs cap64MiB per case. Start only with
root free>=32GiB, task volume free>=128GiB and MemAvailable>=16GiB; stop at root
free<20GiB, task volume free<64GiB, MemAvailable<8GiB or owned allocation>=64GiB.
Sample existing volume/memory facts every5s during owned phases. Fixture bodies
are64KiB for Q1; G2 uses the small existing8-D fixture pattern, no corpus campaign.

Per-case120s total, READY45s/native-witness30s/healthy-or-retry30s/reap5s as in
PROTOCOL.md. These bounds apply to qualification harnesses only, not product SLA.
No automatic retry after timeout, missing witness or resource stop. Preserve the
failure and reap only owned actors; no cleanup/relocation of unrelated resources.
If canonical full-gate ownership needs a larger build budget, propose its concrete
existing10-compatible budget separately rather than silently exceed this Q1 cap.
