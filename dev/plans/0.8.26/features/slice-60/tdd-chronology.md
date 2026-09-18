---
title: FathomDB 0.8.26 Slice 60 — TDD chronology
status: COMPLETE
target_release: 0.8.26
---

# Slice 60 TDD chronology

Slice 60 began as maintained-owner reconciliation, whose test-first oracle was
a reproducible set of source-comparison presence/absence probes. Independent
review then exposed a product reachability defect; after explicit repository-
owner authorization, the recovery addendum used executable Rust RED/GREEN.

## RED — stale owners at `b770de01`

Before changing the owners, focused `rg -q` probes produced these failures:

```text
RED retrieval-current-profile
RED retrieval-cross-encoder
RED recovery-current-profile
RED recovery-full-inventory
RED engine-current-profile
RED engine-projection-gate
RED recovery-no-stream-fiction
RED engine-global-logical-id
```

Direct witnesses included:

```text
dev/design/retrieval.md:45: `rerank` is deferred
dev/design/engine.md:144: per kind
dev/design/recovery.md:74: progress stream plus terminal summary
```

The accepted successor ADR was then owner-authorized, and the reviewed planning
bundle was committed as `3f23ca59` before implementation.

## GREEN — reconciled current owners

The same claim families were rerun after the rewrite, with positive probes for
the current profile and inverse probes for stale claims:

```bash
check_present() { rg -q "$2" "$3"; }
check_absent() { ! rg -q "$2" "$3"; }

check_present retrieval-current-profile '^target_release: 0\.8\.26$' dev/design/retrieval.md
check_present retrieval-cross-encoder 'Cross-encoder reranking|cross-encoder reranking' dev/design/retrieval.md
check_present recovery-current-profile '^target_release: 0\.8\.26$' dev/design/recovery.md
check_present recovery-full-inventory 'orphan-provenance' dev/design/recovery.md
check_present engine-current-profile '^target_release: 0\.8\.26$' dev/design/engine.md
check_present engine-projection-gate 'commit_gate' dev/design/engine.md
check_present engine-shared-vector 'vector_default' dev/design/engine.md
check_absent retrieval-no-deferred-rerank '`rerank` is deferred' dev/design/retrieval.md
check_absent recovery-no-stream-fiction 'progress stream plus terminal summary' dev/design/recovery.md
check_absent engine-no-kind-identity 'per kind\.' dev/design/engine.md
check_absent engine-no-restore 'restore_logical_id' dev/design/engine.md
check_absent engine-no-single-batch-cursor \
  'One write cursor `c_w` is allocated for the committed batch as a whole' \
  dev/design/engine.md
```

```text
GREEN present retrieval-current-profile
GREEN present retrieval-cross-encoder
GREEN present recovery-current-profile
GREEN present recovery-full-inventory
GREEN present engine-current-profile
GREEN present engine-projection-gate
GREEN present engine-shared-vector
GREEN absent retrieval-no-deferred-rerank
GREEN absent recovery-no-stream-fiction
GREEN absent engine-no-kind-identity
GREEN absent engine-no-restore
GREEN absent engine-no-single-batch-cursor
```

The implementation-side guard also remained GREEN:

```console
git diff --name-only b770de01 -- src/rust src/python src/ts
```

It emitted no paths. The only non-document implementation file changed is the
mandatory monotone status-budget ratchet in `scripts/lint-design-status.sh`,
lowered from 46 to 43 because three maintained owners moved from legacy
free-form `locked` status to governed `ACTIVE` status.

## Focused validation

The first Markdown run intentionally exposed the ratchet reduction:

```text
FAIL lint-design-status: legacy count is now 43, ceiling is 46 — lower
`LEGACY_BUDGET` to 43 in scripts/lint-design-status.sh in this same change.
```

After that required one-line guard adjustment, `agent-lint-md.sh` passed.
Design lifecycle, its regression fixture, release-state views, and product-code
diff checks also passed before independent content review.

## Independent-review RED

The first independent implementation review returned CHANGES REQUESTED. Its
focused source comparison found:

- fresh bootstrap reports schema construction steps, while only a current
  reopen reports an empty `migration_steps` list;
- active source/tests still link six headings removed by the initial rewrite;
- JSON-object wording needed to be scoped to `--json`; and
- CLI recovery opens through the public fail-closed path before invoking its
  action, making the malformed-WAL recovery hint unreachable.

The first three documentation defects were corrected in the follow-up diff.
The last activated the plan stop gate. The repository owner then explicitly
authorized the narrow recovery addendum; the blocked review remains preserved
in `code-review.md` as the reason for the scope change.

## Recovery RED — `a1166237`

The recovery tests and default-facade absence proof were committed before the
product implementation. The focused engine command failed at compile time as
intended because `fathomdb_engine::recover_truncate_wal` did not exist and
`TruncateWalReport` did not carry `discarded_corrupt_wal`:

```console
cargo test -p fathomdb-engine --features operator --test truncate_wal
error[E0432]: unresolved import `fathomdb_engine::recover_truncate_wal`
error[E0609]: no field `discarded_corrupt_wal` on type `TruncateWalReport`
```

The RED suite bound malformed-WAL public-open refusal, acknowledged recovery
and reopen, missing/empty/rollback-journal/live-lock refusal, absent-WAL
disposition, healthy-WAL Busy reporting, CLI exit mapping, acceptance gating,
malformed-header safe-export refusal, and operator-feature facade presence plus
default-feature absence.

## Recovery GREEN

The first focused engine run compiled and passed seven cases, but its healthy
WAL Busy case failed with `IncompatibleSchemaVersion { seen: 0, supported: 34
}`. That exposed a legitimate WAL rule: the current schema cookie can be in a
healthy WAL while the immutable standalone main file remains older. The design
and implementation were corrected so malformed-WAL discard still requires a
standalone schema-34 main file, while healthy WAL is version-checked through
SQLite's effective main-plus-WAL view.

The next engine run passed all eight original recovery cases. The first CLI run
then failed only the new malformed-header contract because the serializer used
generic `CorruptionError`; mapping corruption envelopes to their stable
`RecoveryHint.code` made the second run pass. Two additional acceptance tests
for noncurrent and corrupt main-file byte preservation passed on their first
run. Design rereview then caught that effective-version validation on the
read/write recovery connection could checkpoint a healthy noncurrent WAL while
returning refusal. A deterministic `SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE` fixture
made that concern RED by showing the standalone main change from version 34 to
33. Moving effective validation to a read-only/query-only SQLite connection
made it GREEN; pending-current success and malformed-WAL/noncurrent-main
refusal complete the three-way schema-cookie boundary.

The proportional GREEN matrix was:

```text
fathomdb-engine --features operator --test truncate_wal: 15 passed
fathomdb-engine --test durability_open_path: 13 passed, 1 ignored
fathomdb-cli --test recovery_cli: 16 passed, 7 pre-existing ignored
fathomdb --features operator --test governed_surface: 4 passed
fathomdb --no-default-features --doc: 5 passed
```

## Adversarial-review RED/GREEN

Independent code review then found two additional admission hazards. First, an
unrelated SQLite database could counterfeit currency by setting
`user_version = 34`; with a malformed WAL, the initial implementation reported
successful discard even though normal FathomDB open still rejected the schema.
Second, SQLite's read-only effective-view probe changed transient SHM bytes when
it refused a healthy WAL whose effective schema was noncurrent. Focused tests
made both findings RED while proving database/WAL preservation.

GREEN reuses current open-time dependency, frozen-read, and dependency-closure
schema invariants before any read/write recovery connection. Healthy-WAL
preflight now snapshots SHM and restores it on refusal, matching normal
admission's byte-preserving behavior. The final recovery connection uses
`mode=rw` without create permission, closing the validation-to-open file-
disappearance window. A follow-up design review required explicit coverage for
the healthy-WAL effective-invariant branch; a schema-33 main plus schema-34 WAL
that removes a required frozen-read trigger now proves `SchemaInconsistent`
refusal and exact database/WAL/SHM preservation. The focused engine suite
passes 15/15 with a real
schema-34 database for both pending-current success and effective-noncurrent
refusal.

## Final verification

Independent code review and design rereview returned PASS with no unresolved
P1/P2. An independent verifier reproduced the focused matrix at exact candidate
`173c49cb`. The unchanged canonical `agent-verify` then passed strict security
and all 117 registered suites with none skipped or excluded.
